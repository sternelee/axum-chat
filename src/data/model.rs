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

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Provider {
    pub id: i64,
    pub name: String,
    pub provider_type: ProviderType,
    pub base_url: String,
    pub api_key_encrypted: String,
    pub is_active: bool,
    pub created_at: String, // SQLite timestamp as string
    pub updated_at: String, // SQLite timestamp as string
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type, Clone)]
#[sqlx(type_name = "TEXT")]
pub enum ProviderType {
    #[serde(rename = "openai")]
    OpenAI,
    #[serde(rename = "gemini")]
    Gemini,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct ProviderModel {
    pub id: i64,
    pub provider_id: i64,
    pub name: String,
    pub display_name: String,
    pub context_length: i64,
    pub input_price: Option<f64>,
    pub output_price: Option<f64>,
    pub capabilities: String, // JSON array
    pub is_active: bool,
    pub created_at: String, // SQLite timestamp as string
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Agent {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub provider_id: i64,
    pub model_name: String,
    pub stream: bool,
    pub chat: bool,
    pub embed: bool,
    pub image: bool,
    pub tool: bool,
    pub tools: String, // JSON array
    pub allow_tools: String, // JSON array of auto-approved tool IDs
    pub system_prompt: Option<String>,
    pub top_p: f64,
    pub max_context: i64,
    pub file: bool,
    pub file_types: String, // JSON array
    pub temperature: f64,
    pub max_tokens: i64,
    pub presence_penalty: f64,
    pub frequency_penalty: f64,
    pub icon: String,
    pub category: String,
    pub public: bool,
    pub is_active: bool,
    pub created_at: String, // SQLite timestamp as string
    pub updated_at: String, // SQLite timestamp as string
}

// Request/Response DTOs
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,
    pub provider_type: ProviderType,
    pub base_url: String,
    pub api_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProviderRequest {
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAgentRequest {
    pub name: String,
    pub description: Option<String>,
    pub provider_id: i64,
    pub model_name: String,
    pub stream: Option<bool>,
    pub chat: Option<bool>,
    pub embed: Option<bool>,
    pub image: Option<bool>,
    pub tool: Option<bool>,
    pub tools: Option<Vec<String>>,
    pub allow_tools: Option<Vec<String>>,
    pub system_prompt: Option<String>,
    pub top_p: Option<f64>,
    pub max_context: Option<i64>,
    pub file: Option<bool>,
    pub file_types: Option<Vec<String>>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub presence_penalty: Option<f64>,
    pub frequency_penalty: Option<f64>,
    pub icon: Option<String>,
    pub category: Option<String>,
    pub public: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateAgentRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub provider_id: Option<i64>,
    pub model_name: Option<String>,
    pub stream: Option<bool>,
    pub chat: Option<bool>,
    pub embed: Option<bool>,
    pub image: Option<bool>,
    pub tool: Option<bool>,
    pub tools: Option<Vec<String>>,
    pub allow_tools: Option<Vec<String>>,
    pub system_prompt: Option<String>,
    pub top_p: Option<f64>,
    pub max_context: Option<i64>,
    pub file: Option<bool>,
    pub file_types: Option<Vec<String>>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub presence_penalty: Option<f64>,
    pub frequency_penalty: Option<f64>,
    pub icon: Option<String>,
    pub category: Option<String>,
    pub public: Option<bool>,
    pub is_active: Option<bool>,
}

// Detailed response models with joined data
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentWithProvider {
    pub id: i64,
    pub user_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub provider: Provider,
    pub model_name: String,
    pub stream: bool,
    pub chat: bool,
    pub embed: bool,
    pub image: bool,
    pub tool: bool,
    pub tools: Vec<String>,
    pub allow_tools: Vec<String>,
    pub system_prompt: Option<String>,
    pub top_p: f64,
    pub max_context: i64,
    pub file: bool,
    pub file_types: Vec<String>,
    pub temperature: f64,
    pub max_tokens: i64,
    pub presence_penalty: f64,
    pub frequency_penalty: f64,
    pub icon: String,
    pub category: String,
    pub public: bool,
    pub is_active: bool,
    pub created_at: String, // SQLite timestamp as string
    pub updated_at: String, // SQLite timestamp as string
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderWithModels {
    pub id: i64,
    pub name: String,
    pub provider_type: ProviderType,
    pub base_url: String,
    pub is_active: bool,
    pub created_at: String, // SQLite timestamp as string
    pub updated_at: String, // SQLite timestamp as string
    pub models: Vec<ProviderModel>,
}
