use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub email: String,
    pub password: String, // Note: Storing plain-text passwords is not recommended. Use hashed passwords instead.
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Chat {
    pub id: i64,
    pub name: String,
    pub user_id: i64,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ChatMessagePair {
    pub id: i64,
    pub model: String,
    pub message_block_id: i64,
    pub chat_id: i64,
    pub human_message: String,
    pub ai_message: Option<String>,
    pub block_rank: i64,
    pub block_size: i64,
}

// Extended AI response data structures
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ExtendedMessageData {
    pub thinking: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub images: Option<Vec<String>>,
    pub reasoning: Option<String>,
    pub usage: Option<UsageInfo>,
    pub sources: Option<Vec<Source>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String, // "function"
    pub function: FunctionCall,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String, // JSON string of arguments
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UsageInfo {
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Source {
    pub title: Option<String>,
    pub url: Option<String>,
    pub snippet: Option<String>,
}
