-- Create table for tool call confirmations
CREATE TABLE IF NOT EXISTS tool_call_confirmations (
    id TEXT PRIMARY KEY,
    chat_id INTEGER NOT NULL,
    message_pair_id INTEGER NOT NULL,
    tool_call TEXT NOT NULL, -- JSON string
    status TEXT NOT NULL DEFAULT 'Pending',
    created_at TEXT NOT NULL,
    user_response TEXT,
    result TEXT,
    FOREIGN KEY (chat_id) REFERENCES chats (id),
    FOREIGN KEY (message_pair_id) REFERENCES message_pairs (id)
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_tool_call_confirmations_chat_id ON tool_call_confirmations(chat_id);
CREATE INDEX IF NOT EXISTS idx_tool_call_confirmations_status ON tool_call_confirmations(status);