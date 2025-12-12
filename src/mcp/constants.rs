// Default MCP runtime settings
pub const DEFAULT_MCP_TOOL_CALL_TIMEOUT_SECS: u64 = 30;
pub const DEFAULT_MCP_BASE_RESTART_DELAY_MS: u64 = 1000; // Start with 1 second
pub const DEFAULT_MCP_MAX_RESTART_DELAY_MS: u64 = 30000; // Cap at 30 seconds
pub const DEFAULT_MCP_BACKOFF_MULTIPLIER: f64 = 2.0; // Double the delay each time
pub const DEFAULT_MCP_HEALTH_CHECK_INTERVAL_SECS: u64 = 5;

pub const DEFAULT_MCP_CONFIG: &str = r#"{
  "services": [
    {
      "id": "filesystem",
      "name": "Local Filesystem",
      "description": "Access local files and directories",
      "enabled": true,
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
      "env": {},
      "timeout": 30000,
      "max_restarts": 3,
      "auto_restart": true,
      "tools": ["read_file", "write_file", "list_directory", "create_directory", "delete_file"]
    },
    {
      "id": "github",
      "name": "GitHub Repository Access",
      "description": "Access GitHub repositories",
      "enabled": false,
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_PERSONAL_ACCESS_TOKEN": "YOUR_TOKEN_HERE"
      },
      "timeout": 30000,
      "max_restarts": 3,
      "auto_restart": true,
      "tools": ["create_or_update_file", "create_repository", "get_file_contents", "list_issues", "list_pull_requests", "push_files"]
    },
    {
      "id": "brave-search",
      "name": "Brave Search Engine",
      "description": "Search the web using Brave Search API",
      "enabled": true,
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-brave-search"],
      "env": {
        "BRAVE_API_KEY": "YOUR_BRAVE_API_KEY_HERE"
      },
      "timeout": 30000,
      "max_restarts": 3,
      "auto_restart": true,
      "tools": ["brave_search", "brave_web_search"]
    },
    {
      "id": "postgres",
      "name": "PostgreSQL Database",
      "description": "Query PostgreSQL databases",
      "enabled": false,
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-postgres"],
      "env": {
        "POSTGRES_CONNECTION_STRING": "postgresql://user:password@localhost:5432/database"
      },
      "timeout": 30000,
      "max_restarts": 3,
      "auto_restart": true,
      "tools": ["read_query", "write_query", "create_table", "list_tables", "describe_table", "database_schema"]
    },
    {
      "id": "puppeteer",
      "name": "Web Automation",
      "description": "Automate web browser interactions",
      "enabled": false,
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-puppeteer"],
      "env": {},
      "timeout": 60000,
      "max_restarts": 3,
      "auto_restart": true,
      "tools": ["puppeteer_navigate", "puppeteer_screenshot", "puppeteer_click", "puppeteer_fill", "puppeteer_get_content"]
    }
  ]
}"#;

