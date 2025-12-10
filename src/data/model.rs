use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ProviderType {
    #[serde(rename = "openai")]
    OpenAI,
    #[serde(rename = "gemini")]
    Gemini,
}

impl ProviderType {
    pub fn to_string(&self) -> String {
        match self {
            ProviderType::OpenAI => "openai".to_string(),
            ProviderType::Gemini => "gemini".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "openai" => ProviderType::OpenAI,
            "gemini" => ProviderType::Gemini,
            _ => ProviderType::OpenAI, // default
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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

// Helper functions to convert from JSON rows to our structs
impl Chat {
    pub fn from_json_row(row: &serde_json::Value) -> Result<Self, String> {
        Ok(Chat {
            id: row["id"].as_i64().ok_or("Missing id")?,
            name: row["name"].as_str().ok_or("Missing name")?.to_string(),
            user_id: row["user_id"].as_i64().ok_or("Missing user_id")?,
        })
    }
}

impl ChatMessagePair {
    pub fn from_json_row(row: &serde_json::Value) -> Result<Self, String> {
        Ok(ChatMessagePair {
            id: row["id"].as_i64().ok_or("Missing id")?,
            model: row["model"].as_str().ok_or("Missing model")?.to_string(),
            message_block_id: row["message_block_id"].as_i64().ok_or("Missing message_block_id")?,
            chat_id: row["chat_id"].as_i64().ok_or("Missing chat_id")?,
            human_message: row["human_message"].as_str().ok_or("Missing human_message")?.to_string(),
            ai_message: row["ai_message"].as_str().map(|s| s.to_string()),
            block_rank: row["block_rank"].as_i64().ok_or("Missing block_rank")?,
            block_size: row["block_size"].as_i64().ok_or("Missing block_size")?,
        })
    }
}

impl Provider {
    pub fn from_json_row(row: &serde_json::Value) -> Result<Self, String> {
        let provider_type_str = row["provider_type"].as_str().ok_or("Missing provider_type")?;
        Ok(Provider {
            id: row["id"].as_i64().ok_or("Missing id")?,
            name: row["name"].as_str().ok_or("Missing name")?.to_string(),
            provider_type: ProviderType::from_string(provider_type_str),
            base_url: row["base_url"].as_str().ok_or("Missing base_url")?.to_string(),
            api_key_encrypted: row["api_key_encrypted"].as_str().ok_or("Missing api_key_encrypted")?.to_string(),
            is_active: row["is_active"].as_bool().ok_or("Missing is_active")?,
            created_at: row["created_at"].as_str().ok_or("Missing created_at")?.to_string(),
            updated_at: row["updated_at"].as_str().ok_or("Missing updated_at")?.to_string(),
        })
    }
}

impl ProviderModel {
    pub fn from_json_row(row: &serde_json::Value) -> Result<Self, String> {
        Ok(ProviderModel {
            id: row["id"].as_i64().ok_or("Missing id")?,
            provider_id: row["provider_id"].as_i64().ok_or("Missing provider_id")?,
            name: row["name"].as_str().ok_or("Missing name")?.to_string(),
            display_name: row["display_name"].as_str().ok_or("Missing display_name")?.to_string(),
            context_length: row["context_length"].as_i64().ok_or("Missing context_length")?,
            input_price: row["input_price"].as_f64(),
            output_price: row["output_price"].as_f64(),
            capabilities: row["capabilities"].as_str().ok_or("Missing capabilities")?.to_string(),
            is_active: row["is_active"].as_bool().ok_or("Missing is_active")?,
            created_at: row["created_at"].as_str().ok_or("Missing created_at")?.to_string(),
        })
    }
}

impl Agent {
    pub fn from_json_row(row: &serde_json::Value) -> Result<Self, String> {
        Ok(Agent {
            id: row["id"].as_i64().ok_or("Missing id")?,
            user_id: row["user_id"].as_i64().ok_or("Missing user_id")?,
            name: row["name"].as_str().ok_or("Missing name")?.to_string(),
            description: row["description"].as_str().map(|s| s.to_string()),
            provider_id: row["provider_id"].as_i64().ok_or("Missing provider_id")?,
            model_name: row["model_name"].as_str().ok_or("Missing model_name")?.to_string(),
            stream: row["stream"].as_bool().ok_or("Missing stream")?,
            chat: row["chat"].as_bool().ok_or("Missing chat")?,
            embed: row["embed"].as_bool().ok_or("Missing embed")?,
            image: row["image"].as_bool().ok_or("Missing image")?,
            tool: row["tool"].as_bool().ok_or("Missing tool")?,
            tools: row["tools"].as_str().ok_or("Missing tools")?.to_string(),
            allow_tools: row["allow_tools"].as_str().ok_or("Missing allow_tools")?.to_string(),
            system_prompt: row["system_prompt"].as_str().map(|s| s.to_string()),
            top_p: row["top_p"].as_f64().ok_or("Missing top_p")?,
            max_context: row["max_context"].as_i64().ok_or("Missing max_context")?,
            file: row["file"].as_bool().ok_or("Missing file")?,
            file_types: row["file_types"].as_str().ok_or("Missing file_types")?.to_string(),
            temperature: row["temperature"].as_f64().ok_or("Missing temperature")?,
            max_tokens: row["max_tokens"].as_i64().ok_or("Missing max_tokens")?,
            presence_penalty: row["presence_penalty"].as_f64().ok_or("Missing presence_penalty")?,
            frequency_penalty: row["frequency_penalty"].as_f64().ok_or("Missing frequency_penalty")?,
            icon: row["icon"].as_str().ok_or("Missing icon")?.to_string(),
            category: row["category"].as_str().ok_or("Missing category")?.to_string(),
            public: row["public"].as_bool().ok_or("Missing public")?,
            is_active: row["is_active"].as_bool().ok_or("Missing is_active")?,
            created_at: row["created_at"].as_str().ok_or("Missing created_at")?.to_string(),
            updated_at: row["updated_at"].as_str().ok_or("Missing updated_at")?.to_string(),
        })
    }
}
