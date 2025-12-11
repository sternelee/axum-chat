use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
    Restarting,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalAgent {
    pub id: i64,
    pub name: String,
    pub provider_type: String,
    pub status: AgentStatus,
    pub process_id: Option<u32>,
    pub port: u16,
    pub base_url: String,
    pub config: crate::data::model::LocalAgentConfig,
    #[serde(skip)] // Skip serialization as Instant doesn't implement Serialize
    pub last_health_check: Option<Instant>,
    pub restart_count: u32,
    #[serde(skip)] // Skip serialization as Instant doesn't implement Serialize
    pub start_time: Option<Instant>,
}

impl<'de> Deserialize<'de> for LocalAgent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct LocalAgentData {
            id: i64,
            name: String,
            provider_type: String,
            status: AgentStatus,
            process_id: Option<u32>,
            port: u16,
            base_url: String,
            config: crate::data::model::LocalAgentConfig,
            restart_count: u32,
        }

        let data = LocalAgentData::deserialize(deserializer)?;
        Ok(LocalAgent {
            id: data.id,
            name: data.name,
            provider_type: data.provider_type,
            status: data.status,
            process_id: data.process_id,
            port: data.port,
            base_url: data.base_url,
            config: data.config,
            last_health_check: None, // Will be set at runtime
            restart_count: data.restart_count,
            start_time: None, // Will be set at runtime
        })
    }
}

impl LocalAgent {
    pub fn new(
        id: i64,
        name: String,
        provider_type: String,
        port: u16,
        config: crate::data::model::LocalAgentConfig,
    ) -> Self {
        Self {
            id,
            name,
            provider_type,
            status: AgentStatus::Stopped,
            process_id: None,
            port,
            base_url: format!("http://localhost:{}", port),
            config,
            last_health_check: None,
            restart_count: 0,
            start_time: None,
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.status, AgentStatus::Running)
    }

    pub fn is_healthy(&self) -> bool {
        if let Some(last_check) = self.last_health_check {
            // Consider healthy if last check was within 30 seconds
            last_check.elapsed() < Duration::from_secs(30)
        } else {
            false
        }
    }

    pub fn get_uptime(&self) -> Option<Duration> {
        self.start_time.map(|start| start.elapsed())
    }

    pub fn can_restart(&self) -> bool {
        self.restart_count < self.config.max_restarts
    }

    pub fn increment_restart_count(&mut self) {
        self.restart_count += 1;
    }

    pub fn reset_restart_count(&mut self) {
        self.restart_count = 0;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCommand {
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: Option<String>,
    pub env_vars: HashMap<String, String>,
}

impl AgentCommand {
    pub fn from_config(config: &crate::data::model::LocalAgentConfig) -> Result<Self, String> {
        let parts: Vec<&str> = config.startup_command.split_whitespace().collect();
        if parts.is_empty() {
            return Err("Startup command is empty".to_string());
        }

        let command = parts[0].to_string();
        let args = parts[1..].iter().map(|s| s.to_string()).collect();

        Ok(AgentCommand {
            command,
            args,
            working_dir: config.working_directory.clone(),
            env_vars: config.environment_variables.clone(),
        })
    }

    pub async fn execute(&self) -> Result<Child, String> {
        let mut cmd = Command::new(&self.command);

        cmd.args(&self.args);

        if let Some(working_dir) = &self.working_dir {
            cmd.current_dir(working_dir);
        }

        // Add environment variables
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        // Add additional environment variables for agent context
        cmd.env("RUSTGPT_AGENT_ID", "local");
        cmd.env("RUSTGPT_AGENT_TYPE", "coding");

        match cmd.spawn() {
            Ok(child) => Ok(child),
            Err(e) => Err(format!("Failed to start agent: {}", e)),
        }
    }
}