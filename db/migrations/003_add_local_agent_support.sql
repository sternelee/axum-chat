-- Add local_agent_config column to providers table for local AI coding agent configuration
ALTER TABLE providers ADD COLUMN local_agent_config TEXT;

-- Create index for local agent providers to optimize queries
CREATE INDEX idx_providers_local_agent ON providers(provider_type) WHERE provider_type IN (
    'claude-code', 'gemini-cli', 'codex-cli', 'cursor-cli', 'qwen-code',
    'zai-glm', 'aider', 'codeium-chat', 'copilot-cli', 'tabnine'
);

-- Update provider type check constraint to include local agents
-- Note: SQLite doesn't support ALTER CONSTRAINT directly, so we need to recreate the table
CREATE TABLE providers_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    provider_type TEXT NOT NULL CHECK (
        provider_type IN (
            -- Cloud-based providers
            'openai', 'openrouter', 'deepseek', 'azure', 'anthropic', 'cohere',
            'groq', 'mistral', 'gemini', 'huggingface', 'xai',
            -- Local AI coding agents
            'claude-code', 'gemini-cli', 'codex-cli', 'cursor-cli', 'qwen-code',
            'zai-glm', 'aider', 'codeium-chat', 'copilot-cli', 'tabnine'
        )
    ),
    base_url TEXT NOT NULL,
    chat_endpoint TEXT,
    embed_endpoint TEXT,
    image_endpoint TEXT,
    api_key_encrypted TEXT NOT NULL,
    supports_chat BOOLEAN DEFAULT TRUE,
    supports_embed BOOLEAN DEFAULT FALSE,
    supports_image BOOLEAN DEFAULT FALSE,
    supports_streaming BOOLEAN DEFAULT TRUE,
    supports_tools BOOLEAN DEFAULT TRUE,
    supports_images BOOLEAN DEFAULT FALSE,
    local_agent_config TEXT, -- JSON configuration for local agents
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Copy data from old table to new table
INSERT INTO providers_new (
    id, name, provider_type, base_url, chat_endpoint, embed_endpoint, image_endpoint,
    api_key_encrypted, supports_chat, supports_embed, supports_image, supports_streaming,
    supports_tools, supports_images, is_active, created_at, updated_at
)
SELECT
    id, name, provider_type, base_url, chat_endpoint, embed_endpoint, image_endpoint,
    api_key_encrypted, supports_chat, supports_embed, supports_image, supports_streaming,
    supports_tools, supports_images, is_active, created_at, updated_at
FROM providers;

-- Drop old table and rename new table
DROP TABLE providers;
ALTER TABLE providers_new RENAME TO providers;

-- Recreate indexes
CREATE INDEX idx_providers_name ON providers(name);
CREATE INDEX idx_providers_type ON providers(provider_type);
CREATE INDEX idx_providers_supports_chat ON providers(supports_chat);
CREATE INDEX idx_providers_supports_embed ON providers(supports_embed);
CREATE INDEX idx_providers_supports_image ON providers(supports_image);
CREATE INDEX idx_providers_is_active ON providers(is_active, provider_type);
CREATE INDEX idx_providers_local_agent ON providers(provider_type) WHERE provider_type IN (
    'claude-code', 'gemini-cli', 'codex-cli', 'cursor-cli', 'qwen-code',
    'zai-glm', 'aider', 'codeium-chat', 'copilot-cli', 'tabnine'
);

-- Insert default configurations for common local agents if they don't exist
INSERT OR IGNORE INTO providers (
    name, provider_type, base_url, chat_endpoint, api_key_encrypted,
    supports_chat, supports_streaming, supports_tools, is_active,
    local_agent_config
) VALUES
(
    'Claude Code Local',
    'claude-code',
    'http://localhost:3000',
    '/api/v1/chat',
    'local-agent',
    TRUE, TRUE, TRUE, FALSE,
    '{"executable_path": "claude", "working_directory": null, "environment_variables": {}, "startup_command": "claude serve --port 3000", "shutdown_command": null, "health_check_endpoint": "/health", "auto_restart": true, "max_restarts": 3, "startup_timeout": 30, "request_timeout": 60}'
),
(
    'Gemini CLI Local',
    'gemini-cli',
    'http://localhost:8080',
    '/v1/chat',
    'local-agent',
    TRUE, TRUE, TRUE, FALSE,
    '{"executable_path": "gemini", "working_directory": null, "environment_variables": {"GEMINI_API_KEY": ""}, "startup_command": "gemini serve --port 8080", "shutdown_command": null, "health_check_endpoint": "/health", "auto_restart": true, "max_restarts": 3, "startup_timeout": 30, "request_timeout": 60}'
),
(
    'Aider Local',
    'aider',
    'http://localhost:8084',
    '/v1/chat',
    'local-agent',
    TRUE, TRUE, TRUE, FALSE,
    '{"executable_path": "aider", "working_directory": null, "environment_variables": {}, "startup_command": "aider --serve --port 8084", "shutdown_command": null, "health_check_endpoint": "/health", "auto_restart": true, "max_restarts": 3, "startup_timeout": 30, "request_timeout": 60}'
);