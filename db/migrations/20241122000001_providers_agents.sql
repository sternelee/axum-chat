-- Create providers table
CREATE TABLE providers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    provider_type TEXT NOT NULL CHECK (provider_type IN ('openai', 'gemini')),
    base_url TEXT NOT NULL,
    api_key_encrypted TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Create provider_models table
CREATE TABLE provider_models (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    context_length INTEGER NOT NULL,
    input_price REAL DEFAULT 0.0,
    output_price REAL DEFAULT 0.0,
    capabilities TEXT NOT NULL, -- JSON array of capabilities
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (provider_id) REFERENCES providers (id) ON DELETE CASCADE,
    UNIQUE(provider_id, name)
);

-- Create agents table
CREATE TABLE agents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    provider_id INTEGER NOT NULL,
    model_name TEXT NOT NULL,
    stream BOOLEAN NOT NULL DEFAULT TRUE,
    chat BOOLEAN NOT NULL DEFAULT TRUE,
    embed BOOLEAN NOT NULL DEFAULT FALSE,
    image BOOLEAN NOT NULL DEFAULT FALSE,
    tool BOOLEAN NOT NULL DEFAULT FALSE,
    tools TEXT DEFAULT '[]', -- JSON array of tool names
    system_prompt TEXT,
    top_p REAL DEFAULT 1.0,
    max_context INTEGER DEFAULT 4096,
    file BOOLEAN NOT NULL DEFAULT FALSE,
    file_types TEXT DEFAULT '[]', -- JSON array of file extensions
    temperature REAL DEFAULT 0.7,
    max_tokens INTEGER DEFAULT 2048,
    presence_penalty REAL DEFAULT 0.0,
    frequency_penalty REAL DEFAULT 0.0,
    icon TEXT DEFAULT '🤖',
    category TEXT DEFAULT 'general',
    public BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (provider_id) REFERENCES providers (id) ON DELETE RESTRICT
);

-- Create indexes for better performance
CREATE INDEX idx_providers_name ON providers(name);
CREATE INDEX idx_providers_type ON providers(provider_type);
CREATE INDEX idx_provider_models_provider_id ON provider_models(provider_id);
CREATE INDEX idx_agents_user_id ON agents(user_id);
CREATE INDEX idx_agents_provider_id ON agents(provider_id);
CREATE INDEX idx_agents_category ON agents(category);
CREATE INDEX idx_agents_public ON agents(public);

-- Insert default providers
INSERT INTO providers (name, provider_type, base_url, api_key_encrypted) VALUES
('openai_official', 'openai', 'https://api.openai.com/v1', '${OPENAI_API_KEY}'),
('siliconflow', 'openai', 'https://api.siliconflow.cn/v1', '${SILICONFLOW_API_KEY}'),
('google_gemini', 'gemini', 'https://generativelanguage.googleapis.com/v1beta', '${GEMINI_API_KEY}');

-- Insert default models for OpenAI provider
INSERT INTO provider_models (provider_id, name, display_name, context_length, input_price, output_price, capabilities) VALUES
(1, 'gpt-4o', 'GPT-4o', 128000, 5.0, 15.0, '["chat", "vision", "tools", "stream"]'),
(1, 'gpt-4o-mini', 'GPT-4o Mini', 128000, 0.15, 0.6, '["chat", "vision", "tools", "stream"]'),
(1, 'o1-preview', 'OpenAI o1 Preview', 128000, 15.0, 60.0, '["chat", "tools"]');

-- Insert default models for SiliconFlow provider
INSERT INTO provider_models (provider_id, name, display_name, context_length, input_price, output_price, capabilities) VALUES
(2, 'deepseek-chat', 'DeepSeek Chat', 128000, 0.14, 0.28, '["chat", "stream"]'),
(2, 'Qwen/Qwen2.5-7B-Instruct', 'Qwen2.5 7B', 131072, 0.0, 0.0, '["chat", "stream"]'),
(2, 'meta-llama/Llama-3.1-8B-Instruct', 'Llama 3.1 8B', 128000, 0.0, 0.0, '["chat", "stream"]');

-- Insert default models for Gemini provider
INSERT INTO provider_models (provider_id, name, display_name, context_length, input_price, output_price, capabilities) VALUES
(3, 'gemini-1.5-pro', 'Gemini 1.5 Pro', 2097152, 3.5, 10.5, '["chat", "vision", "stream"]'),
(3, 'gemini-1.5-flash', 'Gemini 1.5 Flash', 1048576, 0.075, 0.3, '["chat", "vision", "stream"]');