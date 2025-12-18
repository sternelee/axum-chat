use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct McpConfig {
    pub mcp_servers: HashMap<String, McpServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct McpServerConfig {
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub disabled: Option<bool>,
    pub timeout: Option<u64>,
    pub description: Option<String>,
    pub transport: Option<TransportType>,
    pub url: Option<String>,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TransportType {
    Stdio,
    Sse,
    Http,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl McpConfig {
    pub fn new() -> Self {
        Self {
            mcp_servers: HashMap::new(),
        }
    }

    pub fn load_from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(path)?;
        let config: McpConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn add_server(&mut self, name: String, config: McpServerConfig) {
        self.mcp_servers.insert(name, config);
    }

    pub fn remove_server(&mut self, name: &str) -> Option<McpServerConfig> {
        self.mcp_servers.remove(name)
    }

    pub fn get_enabled_servers(&self) -> HashMap<String, &McpServerConfig> {
        self.mcp_servers
            .iter()
            .filter(|(_, config)| config.disabled != Some(true))
            .map(|(name, config)| (name.clone(), config))
            .collect()
    }

    pub fn get_default_mcp_path() -> PathBuf {
        // Try to find mcp.json in current directory first, then home directory
        let current_dir = std::env::current_dir().unwrap_or_default();
        let local_path = current_dir.join("mcp.json");

        if local_path.exists() {
            return local_path;
        }

        // Fall back to home directory
        if let Some(home) = dirs::home_dir() {
            return home.join(".config").join("axum-chat").join("mcp.json");
        }

        // Final fallback to current directory
        local_path
    }
}

// Common MCP server configurations
impl McpServerConfig {
    pub fn filesystem_command(path: &str) -> Self {
        Self {
            command: Some("npx".to_string()),
            args: Some(vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
                path.to_string(),
            ]),
            env: None,
            disabled: None,
            timeout: Some(300),
            description: Some("Filesystem access and management".to_string()),
            transport: Some(TransportType::Stdio),
            url: None,
            headers: None,
        }
    }

    pub fn github_command() -> Self {
        Self {
            command: Some("npx".to_string()),
            args: Some(vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-github".to_string(),
            ]),
            env: None,
            disabled: None,
            timeout: Some(300),
            description: Some("GitHub integration and repository management".to_string()),
            transport: Some(TransportType::Stdio),
            url: None,
            headers: None,
        }
    }

    pub fn brave_search_command(api_key: &str) -> Self {
        Self {
            command: Some("npx".to_string()),
            args: Some(vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-brave-search".to_string(),
            ]),
            env: Some(HashMap::from([(
                "BRAVE_API_KEY".to_string(),
                api_key.to_string(),
            )])),
            disabled: None,
            timeout: Some(300),
            description: Some("Brave search engine integration".to_string()),
            transport: Some(TransportType::Stdio),
            url: None,
            headers: None,
        }
    }

    pub fn memory_command() -> Self {
        Self {
            command: Some("npx".to_string()),
            args: Some(vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-memory".to_string(),
            ]),
            env: None,
            disabled: None,
            timeout: Some(300),
            description: Some("Persistent memory for conversations".to_string()),
            transport: Some(TransportType::Stdio),
            url: None,
            headers: None,
        }
    }

    pub fn puppeteer_command() -> Self {
        Self {
            command: Some("npx".to_string()),
            args: Some(vec![
                "-y".to_string(),
                "@modelcontextprotocol/server-puppeteer".to_string(),
            ]),
            env: None,
            disabled: None,
            timeout: Some(300),
            description: Some("Web browser automation and screenshots".to_string()),
            transport: Some(TransportType::Stdio),
            url: None,
            headers: None,
        }
    }

    pub fn sse_transport(url: &str, headers: Option<HashMap<String, String>>) -> Self {
        Self {
            command: None,
            args: None,
            env: None,
            disabled: None,
            timeout: Some(300),
            description: Some(format!("SSE transport to {}", url)),
            transport: Some(TransportType::Sse),
            url: Some(url.to_string()),
            headers,
        }
    }

    pub fn http_transport(url: &str, headers: Option<HashMap<String, String>>) -> Self {
        Self {
            command: None,
            args: None,
            env: None,
            disabled: None,
            timeout: Some(300),
            description: Some(format!("HTTP transport to {}", url)),
            transport: Some(TransportType::Http),
            url: Some(url.to_string()),
            headers,
        }
    }
}

