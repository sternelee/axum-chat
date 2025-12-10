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
    pub chat_endpoint: Option<String>,
    pub embed_endpoint: Option<String>,
    pub image_endpoint: Option<String>,
    pub api_key_encrypted: String,
    pub supports_chat: bool,
    pub supports_embed: bool,
    pub supports_image: bool,
    pub supports_streaming: bool,
    pub supports_tools: bool,
    pub supports_images: bool,
    pub is_active: bool,
    pub created_at: String, // SQLite timestamp as string
    pub updated_at: String, // SQLite timestamp as string
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ProviderType {
    #[serde(rename = "openai")]
    OpenAI,
    #[serde(rename = "openrouter")]
    OpenRouter,
    #[serde(rename = "deepseek")]
    DeepSeek,
    #[serde(rename = "azure")]
    AzureOpenAI,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "cohere")]
    Cohere,
    #[serde(rename = "groq")]
    Groq,
    #[serde(rename = "mistral")]
    MistralAI,
    #[serde(rename = "gemini")]
    Gemini,
    #[serde(rename = "huggingface")]
    HuggingFace,
    #[serde(rename = "xai")]
    XAI,
}

impl ProviderType {
    pub fn to_string(&self) -> String {
        match self {
            ProviderType::OpenAI => "openai".to_string(),
            ProviderType::OpenRouter => "openrouter".to_string(),
            ProviderType::DeepSeek => "deepseek".to_string(),
            ProviderType::AzureOpenAI => "azure".to_string(),
            ProviderType::Anthropic => "anthropic".to_string(),
            ProviderType::Cohere => "cohere".to_string(),
            ProviderType::Groq => "groq".to_string(),
            ProviderType::MistralAI => "mistral".to_string(),
            ProviderType::Gemini => "gemini".to_string(),
            ProviderType::HuggingFace => "huggingface".to_string(),
            ProviderType::XAI => "xai".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "openai" => ProviderType::OpenAI,
            "openrouter" => ProviderType::OpenRouter,
            "deepseek" => ProviderType::DeepSeek,
            "azure" => ProviderType::AzureOpenAI,
            "anthropic" => ProviderType::Anthropic,
            "cohere" => ProviderType::Cohere,
            "groq" => ProviderType::Groq,
            "mistral" => ProviderType::MistralAI,
            "gemini" => ProviderType::Gemini,
            "huggingface" => ProviderType::HuggingFace,
            "xai" => ProviderType::XAI,
            _ => ProviderType::OpenAI, // default
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            ProviderType::OpenAI => "OpenAI",
            ProviderType::OpenRouter => "OpenRouter",
            ProviderType::DeepSeek => "DeepSeek",
            ProviderType::AzureOpenAI => "Azure OpenAI",
            ProviderType::Anthropic => "Anthropic Claude",
            ProviderType::Cohere => "Cohere",
            ProviderType::Groq => "Groq",
            ProviderType::MistralAI => "Mistral AI",
            ProviderType::Gemini => "Google Gemini",
            ProviderType::HuggingFace => "Hugging Face",
            ProviderType::XAI => "xAI Grok",
        }
    }

    pub fn default_base_url(&self) -> &'static str {
        match self {
            ProviderType::OpenAI => "https://api.openai.com/v1",
            ProviderType::OpenRouter => "https://openrouter.ai/api/v1",
            ProviderType::DeepSeek => "https://api.deepseek.com/v1",
            ProviderType::AzureOpenAI => "https://your-resource.openai.azure.com/openai/deployments/your-deployment",
            ProviderType::Anthropic => "https://api.anthropic.com/v1",
            ProviderType::Cohere => "https://api.cohere.ai/v1",
            ProviderType::Groq => "https://api.groq.com/openai/v1",
            ProviderType::MistralAI => "https://api.mistral.ai/v1",
            ProviderType::Gemini => "https://generativelanguage.googleapis.com/v1beta/",
            ProviderType::HuggingFace => "https://api-inference.huggingface.co/models",
            ProviderType::XAI => "https://api.x.ai/v1",
        }
    }

    pub fn default_endpoints(&self) -> ProviderEndpoints {
        match self {
            ProviderType::OpenAI => ProviderEndpoints {
                chat: Some("/chat/completions".to_string()),
                embed: Some("/embeddings".to_string()),
                image: Some("/images/generations".to_string()),
            },
            ProviderType::OpenRouter => ProviderEndpoints {
                chat: Some("/chat/completions".to_string()),
                embed: Some("/embeddings".to_string()),
                image: Some("/images/generations".to_string()),
            },
            ProviderType::DeepSeek => ProviderEndpoints {
                chat: Some("/chat/completions".to_string()),
                embed: Some("/embeddings".to_string()),
                image: None,
            },
            ProviderType::AzureOpenAI => ProviderEndpoints {
                chat: Some("/chat/completions?api-version=2024-02-15-preview".to_string()),
                embed: Some("/embeddings?api-version=2024-02-15-preview".to_string()),
                image: Some("/images/generations?api-version=2024-02-15-preview".to_string()),
            },
            ProviderType::Anthropic => ProviderEndpoints {
                chat: Some("/messages".to_string()),
                embed: None,
                image: None,
            },
            ProviderType::Cohere => ProviderEndpoints {
                chat: Some("/chat".to_string()),
                embed: Some("/embed".to_string()),
                image: None,
            },
            ProviderType::Groq => ProviderEndpoints {
                chat: Some("/chat/completions".to_string()),
                embed: None,
                image: None,
            },
            ProviderType::MistralAI => ProviderEndpoints {
                chat: Some("/chat/completions".to_string()),
                embed: Some("/embeddings".to_string()),
                image: None,
            },
            ProviderType::Gemini => ProviderEndpoints {
                chat: Some("/models/{model}:generateContent".to_string()),
                embed: Some("/models/{model}:embedContent".to_string()),
                image: None,
            },
            ProviderType::HuggingFace => ProviderEndpoints {
                chat: Some("/models/{model}/v1/chat/completions".to_string()),
                embed: Some("/pipeline/feature-extraction".to_string()),
                image: None,
            },
            ProviderType::XAI => ProviderEndpoints {
                chat: Some("/chat/completions".to_string()),
                embed: None,
                image: None,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderEndpoints {
    pub chat: Option<String>,
    pub embed: Option<String>,
    pub image: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum ProviderServiceType {
    Chat,
    ImageGeneration,
    Embeddings,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
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
    pub chat_endpoint: Option<String>,
    pub embed_endpoint: Option<String>,
    pub image_endpoint: Option<String>,
    pub supports_chat: Option<bool>,
    pub supports_embed: Option<bool>,
    pub supports_image: Option<bool>,
    pub supports_streaming: Option<bool>,
    pub supports_tools: Option<bool>,
    pub supports_images: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProviderRequest {
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub chat_endpoint: Option<String>,
    pub embed_endpoint: Option<String>,
    pub image_endpoint: Option<String>,
    pub supports_chat: Option<bool>,
    pub supports_embed: Option<bool>,
    pub supports_image: Option<bool>,
    pub supports_streaming: Option<bool>,
    pub supports_tools: Option<bool>,
    pub supports_images: Option<bool>,
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
            chat_endpoint: row["chat_endpoint"].as_str().map(|s| s.to_string()),
            embed_endpoint: row["embed_endpoint"].as_str().map(|s| s.to_string()),
            image_endpoint: row["image_endpoint"].as_str().map(|s| s.to_string()),
            api_key_encrypted: row["api_key_encrypted"].as_str().ok_or("Missing api_key_encrypted")?.to_string(),
            supports_chat: row["supports_chat"].as_bool().unwrap_or(true),
            supports_embed: row["supports_embed"].as_bool().unwrap_or(false),
            supports_image: row["supports_image"].as_bool().unwrap_or(false),
            supports_streaming: row["supports_streaming"].as_bool().unwrap_or(true),
            supports_tools: row["supports_tools"].as_bool().unwrap_or(false),
            supports_images: row["supports_images"].as_bool().unwrap_or(false),
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
