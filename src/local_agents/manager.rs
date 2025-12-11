use crate::local_agents::{LocalAgent, AgentStatus, LocalAgentClient};
use crate::local_agents::agent::AgentCommand;
use std::collections::HashMap;
use std::process::{Child, Command};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex};
use tokio::time::{sleep, interval};

#[derive(Debug, Clone)]
pub struct LocalAgentManager {
    agents: Arc<RwLock<HashMap<i64, LocalAgent>>>,
    processes: Arc<Mutex<HashMap<i64, Child>>>,
}

impl LocalAgentManager {
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn add_agent(&self, agent: LocalAgent) -> Result<(), String> {
        let mut agents = self.agents.write().await;
        agents.insert(agent.id, agent);
        Ok(())
    }

    pub async fn get_agent(&self, id: i64) -> Option<LocalAgent> {
        let agents = self.agents.read().await;
        agents.get(&id).cloned()
    }

    pub async fn get_all_agents(&self) -> Vec<LocalAgent> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }

    pub async fn start_agent(&self, id: i64) -> Result<(), String> {
        let mut agents = self.agents.write().await;
        let agent = agents.get_mut(&id).ok_or("Agent not found")?;

        if agent.is_running() {
            return Err("Agent is already running".to_string());
        }

        // Check if we can restart
        if !agent.can_restart() {
            return Err("Maximum restart count exceeded".to_string());
        }

        // Update status to starting
        agent.status = AgentStatus::Starting;

        // Build the startup command
        let agent_command = AgentCommand::from_config(&agent.config)?;

        // Start the process
        let child = agent_command.execute().await?;
        let process_id = child.id();

        // Update agent state
        agent.process_id = Some(process_id);
        agent.status = AgentStatus::Running;
        agent.start_time = Some(Instant::now());
        agent.last_health_check = None;

        // Store the process handle
        let mut processes = self.processes.lock().await;
        processes.insert(id, child);

        // Start health checking for this agent
        self.start_health_check(id).await;

        Ok(())
    }

    pub async fn stop_agent(&self, id: i64) -> Result<(), String> {
        let mut agents = self.agents.write().await;
        let agent = agents.get_mut(&id).ok_or("Agent not found")?;

        if !agent.is_running() {
            return Err("Agent is not running".to_string());
        }

        // Update status
        agent.status = AgentStatus::Stopped;
        agent.start_time = None;

        // Kill the process
        let mut processes = self.processes.lock().await;
        if let Some(mut child) = processes.remove(&id) {
            // Try graceful shutdown first
            if let Some(shutdown_cmd) = &agent.config.shutdown_command {
                match self.execute_shutdown_command(shutdown_cmd) {
                    Ok(_) => {
                        // Give graceful shutdown a moment to work
                        sleep(Duration::from_secs(2)).await;
                    }
                    Err(e) => {
                        eprintln!("Graceful shutdown failed: {}", e);
                    }
                }
            }

            // Force kill if still running
            match child.kill() {
                Ok(_) => {
                    // Wait for process to actually die
                    let _ = child.wait();
                }
                Err(e) => {
                    eprintln!("Failed to kill process {}: {}", agent.process_id.unwrap_or(0), e);
                }
            }
        }

        agent.process_id = None;
        Ok(())
    }

    pub async fn restart_agent(&self, id: i64) -> Result<(), String> {
        // Check if agent exists first
        let agent_exists = {
            let agents = self.agents.read().await;
            agents.contains_key(&id)
        };

        if !agent_exists {
            return Err("Agent not found".to_string());
        }

        // Get agent info to see if it's running
        let is_running = {
            let agents = self.agents.read().await;
            agents.get(&id).map(|agent| agent.is_running()).unwrap_or(false)
        };

        if is_running {
            // Stop first
            self.stop_agent(id).await?;
        }

        // Update status and restart count
        {
            let mut agents = self.agents.write().await;
            if let Some(agent) = agents.get_mut(&id) {
                agent.status = AgentStatus::Restarting;
                agent.increment_restart_count();
            }
        }

        // Start again
        self.start_agent(id).await
    }

    pub async fn health_check_agent(&self, id: i64) -> Result<bool, String> {
        let agent = self.get_agent(id).await.ok_or("Agent not found")?;

        if !agent.is_running() {
            return Ok(false);
        }

        let client = LocalAgentClient::new(
            agent.base_url.clone(),
            agent.config.request_timeout
        );

        let is_healthy = client.health_check().await?;

        // Update the agent's health check status
        let mut agents = self.agents.write().await;
        if let Some(agent) = agents.get_mut(&id) {
            if is_healthy {
                agent.last_health_check = Some(Instant::now());
                agent.status = AgentStatus::Running;
            } else {
                agent.status = AgentStatus::Error("Health check failed".to_string());
            }
        }

        Ok(is_healthy)
    }

    pub async fn get_agent_client(&self, id: i64) -> Result<LocalAgentClient, String> {
        let agent = self.get_agent(id).await.ok_or("Agent not found")?;

        if !agent.is_running() {
            return Err("Agent is not running".to_string());
        }

        Ok(LocalAgentClient::new(
            agent.base_url.clone(),
            agent.config.request_timeout
        ))
    }

    pub async fn start_all_agents(&self) -> Vec<(i64, Result<(), String>)> {
        let agents = self.get_all_agents().await;
        let mut results = Vec::new();

        for agent in agents {
            let result = self.start_agent(agent.id).await;
            results.push((agent.id, result));
        }

        results
    }

    pub async fn stop_all_agents(&self) -> Vec<(i64, Result<(), String>)> {
        let agents = self.get_all_agents().await;
        let mut results = Vec::new();

        for agent in agents {
            if agent.is_running() {
                let result = self.stop_agent(agent.id).await;
                results.push((agent.id, result));
            }
        }

        results
    }

    async fn start_health_check(&self, agent_id: i64) {
        let agents = self.agents.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Check every 30 seconds

            loop {
                interval.tick().await;

                // Check if agent still exists and is running
                {
                    let agents_lock = agents.read().await;
                    if let Some(agent) = agents_lock.get(&agent_id) {
                        if !agent.is_running() {
                            break;
                        }
                    } else {
                        break;
                    }
                }

                // Perform health check
                // Note: This would need to be implemented to avoid circular dependency
                // For now, we'll just update the last check time if the agent is still running
                {
                    let mut agents_lock = agents.write().await;
                    if let Some(agent) = agents_lock.get_mut(&agent_id) {
                        if agent.is_running() {
                            agent.last_health_check = Some(Instant::now());
                        }
                    }
                }
            }
        });
    }

    fn execute_shutdown_command(&self, shutdown_command: &str) -> Result<(), String> {
        let parts: Vec<&str> = shutdown_command.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Empty shutdown command".to_string());
        }

        let mut cmd = Command::new(parts[0]);
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }

        match cmd.output() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Failed to execute shutdown command: {}", e)),
        }
    }

    // Cleanup processes that have died
    pub async fn cleanup_dead_processes(&self) {
        let mut processes = self.processes.lock().await;
        let mut agents = self.agents.write().await;

        let mut dead_processes = Vec::new();

        for (id, child) in processes.iter_mut() {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // Process has died
                    dead_processes.push(*id);
                }
                Ok(None) => {
                    // Process is still running
                }
                Err(_) => {
                    // Error checking status, assume dead
                    dead_processes.push(*id);
                }
            }
        }

        // Remove dead processes and update agent status
        for id in dead_processes {
            processes.remove(&id);
            if let Some(agent) = agents.get_mut(&id) {
                agent.status = AgentStatus::Error("Process died unexpectedly".to_string());
                agent.process_id = None;
                agent.start_time = None;
            }
        }
    }
}

impl Default for LocalAgentManager {
    fn default() -> Self {
        Self::new()
    }
}