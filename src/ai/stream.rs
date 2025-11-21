use axum::Error;
use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest_eventsource::{Event as ReqwestEvent, EventSource as ReqwestEventSource};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

use crate::data::model::{ChatMessagePair, AgentWithProvider, ProviderType};

// Define a struct to represent a model.
#[derive(Serialize, Deserialize, Debug)]
pub struct Model {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub owned_by: String,
}

// Define a struct to represent the list of models.
#[derive(Serialize, Deserialize, Debug)]
struct ModelList {
    object: String,
    data: Vec<Model>,
}

// Gemini API response structures
#[derive(Serialize, Deserialize, Debug)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
    role: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiPart {
    text: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiGenerateContentRequest {
    contents: Vec<GeminiContent>,
    generation_config: Option<GeminiGenerationConfig>,
    safety_settings: Option<Vec<GeminiSafetySetting>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiGenerationConfig {
    temperature: Option<f64>,
    top_p: Option<f64>,
    top_k: Option<i32>,
    max_output_tokens: Option<i32>,
    stop_sequences: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiSafetySetting {
    category: String,
    threshold: String,
}

#[derive(Deserialize, Debug)]
struct GeminiStreamResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize, Debug)]
struct GeminiCandidate {
    content: GeminiContent,
    finish_reason: Option<String>,
}

#[derive(Debug)]
pub enum GenerationEvent {
    Text(String),
    End(String),
}

/// List available models for a provider
pub async fn list_engines(
    provider_type: &ProviderType,
    api_key: &str,
    base_url: &str,
) -> Result<Vec<Model>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = match provider_type {
        ProviderType::OpenAI => format!("{}/models", base_url),
        ProviderType::Gemini => format!("{}models?key={}", base_url, api_key),
    };

    let mut request = client.get(&url);

    // Only add Authorization header for OpenAI-compatible providers
    if matches!(provider_type, ProviderType::OpenAI) {
        let auth_header = HeaderValue::from_str(&format!("Bearer {}", api_key))?;
        request = request.header(AUTHORIZATION, auth_header);
    }

    let response = request.send().await?;

    match provider_type {
        ProviderType::OpenAI => {
            let res: ModelList = response.json().await?;
            Ok(res.data)
        }
        ProviderType::Gemini => {
            // Gemini models endpoint returns different structure
            let gemini_response: Value = response.json().await?;
            let mut models = Vec::new();

            if let Some(models_array) = gemini_response["models"].as_array() {
                for model in models_array {
                    if let Some(name) = model["name"].as_str() {
                        let model_id = name.split('/').last().unwrap_or(name);
                        models.push(Model {
                            id: model_id.to_string(),
                            object: "model".to_string(),
                            created: chrono::Utc::now().timestamp(),
                            owned_by: "google".to_string(),
                        });
                    }
                }
            }
            Ok(models)
        }
    }
}

/// Generate streaming response using the specified agent and provider
pub async fn generate_sse_stream(
    agent: &AgentWithProvider,
    messages: Vec<ChatMessagePair>,
    sender: mpsc::Sender<Result<GenerationEvent, Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    match agent.provider.provider_type {
        ProviderType::OpenAI => {
            generate_openai_stream(agent, messages, sender).await
        }
        ProviderType::Gemini => {
            generate_gemini_stream(agent, messages, sender).await
        }
    }
}

/// Generate streaming response for OpenAI-compatible providers
async fn generate_openai_stream(
    agent: &AgentWithProvider,
    messages: Vec<ChatMessagePair>,
    sender: mpsc::Sender<Result<GenerationEvent, Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let api_key = &agent.provider.api_key_encrypted;
    let base_url = &agent.provider.base_url;
    let model = &agent.model_name;

    let url = format!("{}/chat/completions", base_url);

    // Create messages array with system prompt if provided
    let mut body_messages = Vec::new();

    // Add system message if provided in agent
    if let Some(system_prompt) = &agent.system_prompt {
        body_messages.push(json!({
            "role": "system",
            "content": system_prompt
        }));
    }

    // Add conversation history
    for msg in &messages {
        body_messages.push(json!({
            "role": "user",
            "content": msg.human_message
        }));

        if let Some(ai_message) = &msg.ai_message {
            body_messages.push(json!({
                "role": "assistant",
                "content": ai_message
            }));
        }
    }

    // Build request body with agent parameters
    let mut request_body = json!({
        "model": model,
        "messages": body_messages,
        "stream": true
    });

    // Add agent-specific parameters
    if agent.temperature != 0.7 {
        request_body["temperature"] = json!(agent.temperature);
    }
    if agent.top_p != 1.0 {
        request_body["top_p"] = json!(agent.top_p);
    }
    if agent.max_tokens != 2048 {
        request_body["max_tokens"] = json!(agent.max_tokens);
    }
    if agent.presence_penalty != 0.0 {
        request_body["presence_penalty"] = json!(agent.presence_penalty);
    }
    if agent.frequency_penalty != 0.0 {
        request_body["frequency_penalty"] = json!(agent.frequency_penalty);
    }

    println!("OpenAI Request: {}", request_body);

    let client = reqwest::Client::new();
    let request = client
        .post(&url)
        .header(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))?,
        )
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .body(request_body.to_string());

    let mut stream = ReqwestEventSource::new(request)?;

    while let Some(event) = stream.next().await {
        match event {
            Ok(ReqwestEvent::Open) => println!("OpenAI Connection Open!"),
            Ok(ReqwestEvent::Message(message)) => {
                if message.data.trim() == "[DONE]" {
                    println!("OpenAI Stream completed.");
                    stream.close();
                    send_end_event(&sender).await;
                    break;
                } else {
                    if let Ok(parsed) = serde_json::from_str::<Value>(&message.data) {
                        if let Some(text) = parsed["choices"][0]["delta"]["content"].as_str() {
                            if sender.send(Ok(GenerationEvent::Text(text.to_string()))).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            }
            Err(err) => {
                println!("OpenAI Error: {}", err);
                stream.close();
                if sender.send(Err(axum::Error::new(err))).await.is_err() {
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Generate streaming response for Gemini API
async fn generate_gemini_stream(
    agent: &AgentWithProvider,
    messages: Vec<ChatMessagePair>,
    sender: mpsc::Sender<Result<GenerationEvent, Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let api_key = &agent.provider.api_key_encrypted;
    let base_url = &agent.provider.base_url;
    let model = &agent.model_name;

    let url = format!("{}/models/{}:streamGenerateContent?key={}", base_url, model, api_key);

    // Convert messages to Gemini format
    let mut contents = Vec::new();

    // Add system prompt as the first content with "user" role (Gemini doesn't have system role)
    if let Some(system_prompt) = &agent.system_prompt {
        contents.push(GeminiContent {
            parts: vec![GeminiPart {
                text: format!("System: {}", system_prompt),
            }],
            role: "user".to_string(),
        });

        // Add a model response to acknowledge the system instruction
        contents.push(GeminiContent {
            parts: vec![GeminiPart {
                text: "Understood. I will follow these instructions.".to_string(),
            }],
            role: "model".to_string(),
        });
    }

    // Add conversation history
    for msg in &messages {
        contents.push(GeminiContent {
            parts: vec![GeminiPart {
                text: msg.human_message.clone(),
            }],
            role: "user".to_string(),
        });

        if let Some(ai_message) = &msg.ai_message {
            contents.push(GeminiContent {
                parts: vec![GeminiPart {
                    text: ai_message.clone(),
                }],
                role: "model".to_string(),
            });
        }
    }

    // Build generation config from agent parameters
    let mut generation_config = GeminiGenerationConfig {
        temperature: None,
        top_p: None,
        top_k: None,
        max_output_tokens: None,
        stop_sequences: None,
    };

    if agent.temperature != 0.7 {
        generation_config.temperature = Some(agent.temperature);
    }
    if agent.top_p != 1.0 {
        generation_config.top_p = Some(agent.top_p);
    }
    if agent.max_tokens != 2048 {
        generation_config.max_output_tokens = Some(agent.max_tokens as i32);
    }

    let request_body = GeminiGenerateContentRequest {
        contents,
        generation_config: Some(generation_config),
        safety_settings: Some(vec![
            GeminiSafetySetting {
                category: "HARM_CATEGORY_HARASSMENT".to_string(),
                threshold: "BLOCK_NONE".to_string(),
            },
            GeminiSafetySetting {
                category: "HARM_CATEGORY_HATE_SPEECH".to_string(),
                threshold: "BLOCK_NONE".to_string(),
            },
            GeminiSafetySetting {
                category: "HARM_CATEGORY_SEXUALLY_EXPLICIT".to_string(),
                threshold: "BLOCK_NONE".to_string(),
            },
            GeminiSafetySetting {
                category: "HARM_CATEGORY_DANGEROUS_CONTENT".to_string(),
                threshold: "BLOCK_NONE".to_string(),
            },
        ]),
    };

    let gemini_request_json = serde_json::to_string(&request_body)?;
        println!("Gemini Request: {}", gemini_request_json);

    let client = reqwest::Client::new();
    let request = client
        .post(&url)
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .body(serde_json::to_string(&request_body)?);

    let mut stream = ReqwestEventSource::new(request)?;

    while let Some(event) = stream.next().await {
        match event {
            Ok(ReqwestEvent::Open) => println!("Gemini Connection Open!"),
            Ok(ReqwestEvent::Message(message)) => {
                if let Ok(response) = serde_json::from_str::<GeminiStreamResponse>(&message.data) {
                    if let Some(candidate) = response.candidates.first() {
                        if let Some(text) = candidate.content.parts.first() {
                            if sender.send(Ok(GenerationEvent::Text(text.text.clone()))).await.is_err() {
                                break;
                            }
                        }

                        // Check if generation is complete
                        if candidate.finish_reason.is_some() {
                            println!("Gemini Stream completed.");
                            stream.close();
                            send_end_event(&sender).await;
                            break;
                        }
                    }
                }
            }
            Err(err) => {
                println!("Gemini Error: {}", err);
                stream.close();
                if sender.send(Err(axum::Error::new(err))).await.is_err() {
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Send end event to close the stream
async fn send_end_event(sender: &mpsc::Sender<Result<GenerationEvent, Error>>) {
    let _ = sender
        .send(Ok(GenerationEvent::End(
            r#"<div id="sse-listener" hx-swap-oob="true"></div>"#.to_string(),
        )))
        .await;
}

#[cfg(test)]
mod tests {
    use tokio_stream::wrappers::ReceiverStream;

    use super::*;

    #[tokio::test]
    async fn test_openai_stream() {
        // Create a test agent
        let test_agent = AgentWithProvider {
            id: 1,
            user_id: 1,
            name: "Test Agent".to_string(),
            description: None,
            provider: crate::data::model::Provider {
                id: 1,
                name: "test_provider".to_string(),
                provider_type: ProviderType::OpenAI,
                base_url: "https://api.openai.com/v1".to_string(),
                api_key_encrypted: "test_key".to_string(),
                is_active: true,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            },
            model_name: "gpt-4".to_string(),
            stream: true,
            chat: true,
            embed: false,
            image: false,
            tool: false,
            tools: vec![],
            system_prompt: Some("You are a helpful assistant.".to_string()),
            top_p: 1.0,
            max_context: 4096,
            file: false,
            file_types: vec![],
            temperature: 0.7,
            max_tokens: 2048,
            presence_penalty: 0.0,
            frequency_penalty: 0.0,
            icon: "🤖".to_string(),
            category: "general".to_string(),
            public: false,
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let (_sender, receiver) = mpsc::channel::<Result<GenerationEvent, axum::Error>>(10);
        let mut stream = ReceiverStream::new(receiver);

        let pairs = vec![ChatMessagePair {
            id: 1,
            chat_id: 1,
            message_block_id: 1,
            model: "gpt-4".to_string(),
            human_message: "Hello".to_string(),
            ai_message: None,
            block_rank: 1,
            block_size: 1,
        }];

        tokio::spawn(async move {
            let _ = generate_openai_stream(&test_agent, pairs, _sender).await;
        });

        while let Some(event) = stream.next().await {
            match event {
                Ok(sse_event) => {
                    println!("Received event: {:?}", sse_event)
                }
                Err(_e) => {}
            }
        }
    }

    #[tokio::test]
    async fn test_gemini_stream() {
        // Create a test agent for Gemini
        let test_agent = AgentWithProvider {
            id: 2,
            user_id: 1,
            name: "Gemini Agent".to_string(),
            description: None,
            provider: crate::data::model::Provider {
                id: 2,
                name: "gemini_provider".to_string(),
                provider_type: ProviderType::Gemini,
                base_url: "https://generativelanguage.googleapis.com/v1beta/".to_string(),
                api_key_encrypted: "test_gemini_key".to_string(),
                is_active: true,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            },
            model_name: "gemini-1.5-pro".to_string(),
            stream: true,
            chat: true,
            embed: false,
            image: false,
            tool: false,
            tools: vec![],
            system_prompt: Some("You are a helpful assistant.".to_string()),
            top_p: 1.0,
            max_context: 4096,
            file: false,
            file_types: vec![],
            temperature: 0.7,
            max_tokens: 2048,
            presence_penalty: 0.0,
            frequency_penalty: 0.0,
            icon: "🔮".to_string(),
            category: "general".to_string(),
            public: false,
            is_active: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let (_sender, receiver) = mpsc::channel::<Result<GenerationEvent, axum::Error>>(10);
        let mut stream = ReceiverStream::new(receiver);

        let pairs = vec![ChatMessagePair {
            id: 1,
            chat_id: 1,
            message_block_id: 1,
            model: "gemini-1.5-pro".to_string(),
            human_message: "Hello".to_string(),
            ai_message: None,
            block_rank: 1,
            block_size: 1,
        }];

        tokio::spawn(async move {
            let _ = generate_gemini_stream(&test_agent, pairs, _sender).await;
        });

        while let Some(event) = stream.next().await {
            match event {
                Ok(sse_event) => {
                    println!("Received event: {:?}", sse_event)
                }
                Err(_e) => {}
            }
        }
    }
}