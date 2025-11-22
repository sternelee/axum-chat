use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn, error};

#[derive(Debug, Clone)]
pub struct SecurityManager {
    rate_limiter: RateLimiter,
    tool_whitelist: HashSet<String>,
    tool_blacklist: HashSet<String>,
    category_permissions: HashMap<String, CategoryPermission>,
    session_manager: SessionManager,
}

#[derive(Debug, Clone)]
pub struct RateLimiter {
    global_limit: GlobalRateLimit,
    service_limits: HashMap<String, ServiceRateLimit>,
    user_limits: HashMap<String, UserRateLimit>,
}

#[derive(Debug, Clone)]
pub struct GlobalRateLimit {
    requests_per_minute: u32,
    requests: Vec<Instant>,
}

#[derive(Debug, Clone)]
pub struct ServiceRateLimit {
    service_id: String,
    requests_per_minute: u32,
    requests: Vec<Instant>,
}

#[derive(Debug, Clone)]
pub struct UserRateLimit {
    user_id: String,
    requests_per_minute: u32,
    requests: Vec<Instant>,
}

#[derive(Debug, Clone)]
pub struct CategoryPermission {
    category: String,
    allowed_operations: Vec<String>,
    requires_approval: bool,
    time_restrictions: Option<TimeRestriction>,
    max_execution_time: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct TimeRestriction {
    allowed_hours: Vec<u8>, // 0-23
    allowed_days: Vec<u8>,  // 0-6 (Sunday=0)
}

#[derive(Debug, Clone)]
pub struct SessionManager {
    sessions: HashMap<String, SecuritySession>,
    default_session_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct SecuritySession {
    session_id: String,
    user_id: String,
    created_at: Instant,
    last_activity: Instant,
    approved_tools: HashSet<String>,
    blocked_tools: HashSet<String>,
    risk_score: f32,
    max_risk_score: f32,
}

#[derive(Debug, Clone)]
pub enum SecurityDecision {
    Allow,
    ApproveRequired(String),
    Deny(String),
}

impl SecurityManager {
    pub fn new(
        global_rate_limit: u32,
        service_rate_limits: HashMap<String, u32>,
        tool_whitelist: HashSet<String>,
        tool_blacklist: HashSet<String>,
    ) -> Self {
        Self {
            rate_limiter: RateLimiter {
                global_limit: GlobalRateLimit {
                    requests_per_minute: global_rate_limit,
                    requests: Vec::new(),
                },
                service_limits: service_rate_limits.into_iter()
                    .map(|(service, limit)| (service.clone(), ServiceRateLimit {
                        service_id: service,
                        requests_per_minute: limit,
                        requests: Vec::new(),
                    }))
                    .collect(),
                user_limits: HashMap::new(),
            },
            tool_whitelist,
            tool_blacklist,
            category_permissions: HashMap::new(),
            session_manager: SessionManager {
                sessions: HashMap::new(),
                default_session_timeout: Duration::from_secs(3600), // 1 hour
            },
        }
    }

    pub async fn check_tool_access(
        &mut self,
        user_id: &str,
        session_id: &str,
        service_id: &str,
        tool_name: &str,
        tool_category: &str,
    ) -> SecurityDecision {
        info!("Checking access for tool {}::{} for user {}", service_id, tool_name, user_id);

        // 1. Check if tool is explicitly blocked
        if self.tool_blacklist.contains(tool_name) {
            return SecurityDecision::Deny(format!("Tool {} is blocked", tool_name));
        }

        // 2. Check if tool is whitelisted (if whitelist is not empty)
        if !self.tool_whitelist.is_empty() && !self.tool_whitelist.contains(tool_name) {
            return SecurityDecision::Deny(format!("Tool {} is not whitelisted", tool_name));
        }

        // 3. Check rate limits
        if let Err(reason) = self.check_rate_limits(user_id, service_id).await {
            return SecurityDecision::Deny(reason);
        }

        // 4. Check category permissions
        if let Some(perm) = self.category_permissions.get(tool_category) {
            if !self.is_category_allowed(perm) {
                return SecurityDecision::Deny(format!("Category {} not allowed at this time", tool_category));
            }

            if perm.requires_approval {
                return SecurityDecision::ApproveRequired(format!(
                    "Category {} requires approval", tool_category
                ));
            }
        }

        // 5. Check session-specific permissions
        if let Some(session) = self.session_manager.sessions.get(session_id) {
            if session.blocked_tools.contains(tool_name) {
                return SecurityDecision::Deny(format!("Tool {} is blocked in session", tool_name));
            }

            if !session.approved_tools.contains(tool_name) {
                return SecurityDecision::ApproveRequired(format!("Tool {} needs session approval", tool_name));
            }

            // Check risk score
            if session.risk_score >= session.max_risk_score {
                return SecurityDecision::Deny("Session risk score too high".to_string());
            }
        }

        // 6. Apply risk assessment
        let risk_score = self.calculate_risk_score(user_id, service_id, tool_name, tool_category);
        if risk_score > 0.8 {
            return SecurityDecision::ApproveRequired("High risk tool call".to_string());
        }

        SecurityDecision::Allow
    }

    pub async fn record_tool_usage(
        &mut self,
        user_id: &str,
        service_id: &str,
        tool_name: &str,
        execution_time: Duration,
    ) {
        // Record in rate limiter
        self.rate_limiter.record_usage(user_id, service_id);

        // Update session activity
        for session in self.session_manager.sessions.values_mut() {
            if session.user_id == user_id {
                session.last_activity = Instant::now();
                break;
            }
        }

        info!("Recorded tool usage: {}::{} (executed in {:?})",
              service_id, tool_name, execution_time);
    }

    pub async fn approve_tool_for_session(
        &mut self,
        session_id: &str,
        tool_name: &str,
        duration: Option<Duration>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let session = self.session_manager.sessions
            .get_mut(session_id)
            .ok_or("Session not found")?;

        session.approved_tools.insert(tool_name.to_string());

        if let Some(duration) = duration {
            // Schedule removal of approval after duration
            // TODO: Implement scheduled approval removal
            info!("Approved tool {} for session {} for {:?}", tool_name, session_id, duration);
        } else {
            info!("Approved tool {} for session {} permanently", tool_name, session_id);
        }

        Ok(())
    }

    pub async fn block_tool_for_session(
        &mut self,
        session_id: &str,
        tool_name: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let session = self.session_manager.sessions
            .get_mut(session_id)
            .ok_or("Session not found")?;

        session.blocked_tools.insert(tool_name.to_string());
        info!("Blocked tool {} for session {}", tool_name, session_id);

        Ok(())
    }

    pub async fn create_session(
        &mut self,
        session_id: String,
        user_id: String,
        max_risk_score: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let session = SecuritySession {
            session_id: session_id.clone(),
            user_id,
            created_at: Instant::now(),
            last_activity: Instant::now(),
            approved_tools: HashSet::new(),
            blocked_tools: HashSet::new(),
            risk_score: 0.0,
            max_risk_score,
        };

        self.session_manager.sessions.insert(session_id, session);
        info!("Created new security session");

        Ok(())
    }

    pub async fn cleanup_expired_sessions(&mut self) {
        let now = Instant::now();
        let mut expired_sessions = Vec::new();

        for (session_id, session) in &self.session_manager.sessions {
            if now.duration_since(session.last_activity) > self.session_manager.default_session_timeout {
                expired_sessions.push(session_id.clone());
            }
        }

        for session_id in expired_sessions {
            self.session_manager.sessions.remove(&session_id);
            info!("Cleaned up expired session: {}", session_id);
        }
    }

    // Private methods
    async fn check_rate_limits(&mut self, user_id: &str, service_id: &str) -> Result<(), String> {
        let now = Instant::now();

        // Check global rate limit
        if !self.rate_limiter.global_limit.check_request(now) {
            return Err("Global rate limit exceeded".to_string());
        }

        // Check service rate limit
        if let Some(service_limit) = self.rate_limiter.service_limits.get_mut(service_id) {
            if !service_limit.check_request(now) {
                return Err(format!("Service {} rate limit exceeded", service_id));
            }
        }

        // Check user rate limit
        if !self.rate_limiter.user_limits.contains_key(user_id) {
            self.rate_limiter.user_limits.insert(user_id.to_string(), UserRateLimit {
                user_id: user_id.to_string(),
                requests_per_minute: 50, // Default user limit
                requests: Vec::new(),
            });
        }

        if let Some(user_limit) = self.rate_limiter.user_limits.get_mut(user_id) {
            if !user_limit.check_request(now) {
                return Err(format!("User {} rate limit exceeded", user_id));
            }
        }

        Ok(())
    }

    fn is_category_allowed(&self, permission: &CategoryPermission) -> bool {
        let now = chrono::Local::now();

        // Check time restrictions
        if let Some(time_restriction) = &permission.time_restrictions {
            if !time_restriction.allowed_hours.contains(&(now.hour() as u8)) {
                return false;
            }
            if !time_restriction.allowed_days.contains(&(now.weekday().num_days_from_sunday() as u8)) {
                return false;
            }
        }

        true
    }

    fn calculate_risk_score(
        &self,
        user_id: &str,
        service_id: &str,
        tool_name: &str,
        tool_category: &str,
    ) -> f32 {
        let mut risk_score = 0.0;

        // Base risk by category
        match tool_category {
            "filesystem" => {
                if tool_name.contains("delete") || tool_name.contains("remove") {
                    risk_score += 0.6;
                } else if tool_name.contains("write") {
                    risk_score += 0.4;
                } else {
                    risk_score += 0.1;
                }
            }
            "database" => {
                if tool_name.contains("delete") || tool_name.contains("drop") {
                    risk_score += 0.7;
                } else if tool_name.contains("insert") || tool_name.contains("update") {
                    risk_score += 0.3;
                } else {
                    risk_score += 0.1;
                }
            }
            "web" => risk_score += 0.5,
            "search" => risk_score += 0.1,
            "system" => risk_score += 0.8,
            _ => risk_score += 0.2,
        }

        // Adjust based on user history (simplified)
        // TODO: Implement actual user behavior analysis

        // Adjust based on service trust level
        match service_id {
            id if id.contains("filesystem") => risk_score += 0.1,
            id if id.contains("github") => risk_score += 0.2,
            id if id.contains("postgres") => risk_score += 0.3,
            id if id.contains("puppeteer") => risk_score += 0.4,
            _ => risk_score += 0.2,
        }

        risk_score.min(1.0)
    }
}

impl RateLimiter {
    fn record_usage(&mut self, user_id: &str, service_id: &str) {
        let now = Instant::now();

        // Record global usage
        self.global_limit.requests.push(now);
        self.cleanup_old_requests(&mut self.global_limit.requests);

        // Record service usage
        if let Some(service_limit) = self.service_limits.get_mut(service_id) {
            service_limit.requests.push(now);
            self.cleanup_old_requests(&mut service_limit.requests);
        }

        // Record user usage
        if let Some(user_limit) = self.user_limits.get_mut(user_id) {
            user_limit.requests.push(now);
            self.cleanup_old_requests(&mut user_limit.requests);
        }
    }

    fn cleanup_old_requests(&self, requests: &mut Vec<Instant>) {
        let cutoff = Instant::now() - Duration::from_secs(60);
        requests.retain(|&time| time > cutoff);
    }
}

impl GlobalRateLimit {
    fn check_request(&mut self, now: Instant) -> bool {
        self.requests.push(now);
        self.cleanup_old_requests();

        self.requests.len() <= self.requests_per_minute as usize
    }

    fn cleanup_old_requests(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(60);
        self.requests.retain(|&time| time > cutoff);
    }
}

impl ServiceRateLimit {
    fn check_request(&mut self, now: Instant) -> bool {
        self.requests.push(now);
        self.cleanup_old_requests();

        self.requests.len() <= self.requests_per_minute as usize
    }

    fn cleanup_old_requests(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(60);
        self.requests.retain(|&time| time > cutoff);
    }
}

impl UserRateLimit {
    fn check_request(&mut self, now: Instant) -> bool {
        self.requests.push(now);
        self.cleanup_old_requests();

        self.requests.len() <= self.requests_per_minute as usize
    }

    fn cleanup_old_requests(&mut self) {
        let cutoff = Instant::now() - Duration::from_secs(60);
        self.requests.retain(|&time| time > cutoff);
    }
}

use chrono;