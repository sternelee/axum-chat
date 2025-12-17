-- Add extended response data fields to messages table
ALTER TABLE messages ADD COLUMN thinking TEXT;
ALTER TABLE messages ADD COLUMN tool_calls TEXT; -- JSON array of tool calls
ALTER TABLE messages ADD COLUMN images TEXT; -- JSON array of image URLs
ALTER TABLE messages ADD COLUMN reasoning TEXT; -- Structured reasoning content
ALTER TABLE messages ADD COLUMN usage_prompt_tokens INTEGER;
ALTER TABLE messages ADD COLUMN usage_completion_tokens INTEGER;
ALTER TABLE messages ADD COLUMN usage_total_tokens INTEGER;
ALTER TABLE messages ADD COLUMN sources TEXT; -- JSON array of source citations
ALTER TABLE messages ADD COLUMN metadata TEXT; -- Additional JSON metadata
