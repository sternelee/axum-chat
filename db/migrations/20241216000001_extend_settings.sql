-- Add additional AI configuration fields to settings table
ALTER TABLE settings ADD COLUMN base_url TEXT DEFAULT 'https://api.siliconflow.cn/v1';
ALTER TABLE settings ADD COLUMN model TEXT DEFAULT 'Qwen/Qwen2.5-7B-Instruct';
ALTER TABLE settings ADD COLUMN system_prompt TEXT DEFAULT 'You are a helpful assistant.';
ALTER TABLE settings ADD COLUMN temperature REAL DEFAULT 0.7;
ALTER TABLE settings ADD COLUMN top_p REAL DEFAULT 1.0;
ALTER TABLE settings ADD COLUMN max_tokens INTEGER DEFAULT 2000;