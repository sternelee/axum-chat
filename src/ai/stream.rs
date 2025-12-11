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

// OpenAI tool call structures
#[derive(Serialize, Deserialize, Debug)]
struct OpenAIToolCall {
    id: String,
    r#type: String,
    function: OpenAIFunction,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIFunction {
    name: String,
    arguments: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIToolChoice {
    r#type: String,
    function: OpenAIFunctionChoice,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIFunctionChoice {
    name: String,
}

// OpenAI thinking/reasoning structures (for models like o1)
#[derive(Serialize, Deserialize, Debug)]
struct OpenAIReasoningContent {
    r#type: String,
    text: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIReasoningDelta {
    r#type: String,
    text: String,
}

// Gemini API response structures
#[derive(Serialize, Deserialize, Debug)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
    role: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiPart {
    #[serde(flatten)]
    part_type: GeminiPartType,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum GeminiPartType {
    Text { text: String },
    FunctionCall { function_call: GeminiFunctionCall },
    FunctionResponse { function_response: GeminiFunctionResponse },
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiFunctionCall {
    name: String,
    args: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiFunctionResponse {
    name: String,
    response: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiTool {
    function_declarations: Vec<GeminiFunctionDeclaration>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiFunctionDeclaration {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiToolConfig {
    function_calling_config: GeminiFunctionCallingConfig,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiFunctionCallingConfig {
    mode: String,
    allowed_function_names: Option<Vec<String>>,
}

// Gemini thinking structures (for models that support it)
#[derive(Serialize, Deserialize, Debug)]
struct GeminiThoughtContent {
    thought: String,
}

// Anthropic API structures
#[derive(Serialize, Deserialize, Debug)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AnthropicContent {
    r#type: String,
    text: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AnthropicStreamRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: i32,
    temperature: f64,
    stream: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct AnthropicStreamResponse {
    r#type: String,
    delta: Option<AnthropicDelta>,
    message: Option<AnthropicMessage>,
}

#[derive(Serialize, Deserialize, Debug)]
struct AnthropicDelta {
    r#type: String,
    text: String,
    stop_reason: Option<String>,
}

// Cohere API structures
#[derive(Serialize, Deserialize, Debug)]
struct CohereStreamRequest {
    model: String,
    message: String,
    chat_history: Vec<CohereMessage>,
    temperature: f64,
    max_tokens: i32,
    stream: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct CohereMessage {
    role: String,
    message: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CohereStreamResponse {
    text: String,
    finish_reason: String,
    generation_id: String,
}

// Hugging Face API structures
#[derive(Serialize, Deserialize, Debug)]
struct HuggingFaceStreamRequest {
    inputs: String,
    parameters: HuggingFaceParameters,
    stream: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct HuggingFaceParameters {
    max_new_tokens: Option<i32>,
    temperature: Option<f64>,
    top_p: Option<f64>,
    return_full_text: Option<bool>,
}

// Image generation structures
#[derive(Serialize, Deserialize, Debug)]
struct ImageGenerationRequest {
    prompt: String,
    n: Option<i32>,
    size: Option<String>,
    quality: Option<String>,
    model: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ImageGenerationResponse {
    created: i64,
    data: Vec<ImageData>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ImageData {
    url: String,
    revised_prompt: Option<String>,
}

// Embedding structures
#[derive(Serialize, Deserialize, Debug)]
struct EmbeddingRequest {
    model: String,
    input: String,
    encoding_format: Option<String>,
    dimensions: Option<i32>,
}

#[derive(Serialize, Deserialize, Debug)]
struct EmbeddingResponse {
    object: String,
    data: Vec<EmbeddingData>,
    model: String,
    usage: EmbeddingUsage,
}

#[derive(Serialize, Deserialize, Debug)]
struct EmbeddingData {
    embedding: Vec<f64>,
    index: i64,
    object: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct EmbeddingUsage {
    prompt_tokens: i32,
    total_tokens: i32,
}

#[derive(Serialize, Deserialize, Debug)]
struct GeminiGenerateContentRequest {
    contents: Vec<GeminiContent>,
    generation_config: Option<GeminiGenerationConfig>,
    safety_settings: Option<Vec<GeminiSafetySetting>>,
    tools: Option<Vec<GeminiTool>>,
    tool_config: Option<GeminiToolConfig>,
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
    Thinking(ThinkingEvent),
    ToolCall(ToolCallEvent),
    ToolResponse(ToolResponseEvent),
    End(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThinkingEvent {
    pub id: String,
    pub content: String,
    pub is_final: bool,
    pub is_collapsed: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCallEvent {
    pub id: String,
    pub name: String,
    pub arguments: String,
    pub description: Option<String>,
    pub requires_approval: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResponseEvent {
    pub id: String,
    pub call_id: String,
    pub status: String, // "approved" | "rejected" | "executed"
    pub result: Option<String>,
    pub error: Option<String>,
}

/// List available models for a provider
pub async fn list_engines(
    provider_type: &ProviderType,
    api_key: &str,
    base_url: &str,
) -> Result<Vec<Model>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let url = match provider_type {
        // OpenAI-compatible providers
        ProviderType::OpenAI | ProviderType::OpenRouter | ProviderType::DeepSeek |
        ProviderType::AzureOpenAI | ProviderType::Groq | ProviderType::MistralAI | ProviderType::XAI => {
            format!("{}/models", base_url)
        }
        ProviderType::Gemini => format!("{}models?key={}", base_url, api_key),
        ProviderType::Anthropic => format!("{}/messages/batches", base_url), // Anthropic doesn't have models endpoint
        ProviderType::Cohere => format!("{}/models", base_url),
        ProviderType::HuggingFace => format!("{}models", base_url),
        // Local AI coding agents - return default models
        ProviderType::ClaudeCode | ProviderType::GeminiCLI | ProviderType::CodexCLI |
        ProviderType::CursorCLI | ProviderType::QwenCode | ProviderType::ZAIGLM |
        ProviderType::Aider | ProviderType::CodeiumChat | ProviderType::CopilotCLI |
        ProviderType::Tabnine => {
            format!("{}/models", base_url)
        }
    };

    let mut request = client.get(&url);

    // Add Authorization header for providers that need it
    match provider_type {
        ProviderType::OpenAI | ProviderType::OpenRouter | ProviderType::DeepSeek |
        ProviderType::AzureOpenAI | ProviderType::Groq | ProviderType::MistralAI |
        ProviderType::XAI | ProviderType::Cohere => {
            let auth_header = HeaderValue::from_str(&format!("Bearer {}", api_key))?;
            request = request.header(AUTHORIZATION, auth_header);
        }
        ProviderType::Anthropic => {
            let auth_header = HeaderValue::from_str(&format!("Bearer {}", api_key))?;
            request = request.header(AUTHORIZATION, auth_header)
                .header("anthropic-version", "2023-06-01");
        }
        ProviderType::HuggingFace => {
            let auth_header = HeaderValue::from_str(&format!("Bearer {}", api_key))?;
            request = request.header(AUTHORIZATION, auth_header);
        }
        ProviderType::Gemini => {
            // Gemini uses API key as query parameter, handled above
        }
        // Local AI coding agents - no special auth needed
        ProviderType::ClaudeCode | ProviderType::GeminiCLI | ProviderType::CodexCLI |
        ProviderType::CursorCLI | ProviderType::QwenCode | ProviderType::ZAIGLM |
        ProviderType::Aider | ProviderType::CodeiumChat | ProviderType::CopilotCLI |
        ProviderType::Tabnine => {
            // Local agents typically don't require special auth headers
        }
    }

    let response = request.send().await?;

    match provider_type {
        // OpenAI-compatible providers return standard ModelList
        ProviderType::OpenAI | ProviderType::OpenRouter | ProviderType::DeepSeek |
        ProviderType::AzureOpenAI | ProviderType::Groq | ProviderType::MistralAI |
        ProviderType::XAI | ProviderType::Cohere => {
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
        ProviderType::Anthropic => {
            // Anthropic doesn't have a public models endpoint, return known models
            Ok(vec![
                Model {
                    id: "claude-3-5-sonnet-20241022".to_string(),
                    object: "model".to_string(),
                    created: chrono::Utc::now().timestamp(),
                    owned_by: "anthropic".to_string(),
                },
                Model {
                    id: "claude-3-5-haiku-20241022".to_string(),
                    object: "model".to_string(),
                    created: chrono::Utc::now().timestamp(),
                    owned_by: "anthropic".to_string(),
                },
                Model {
                    id: "claude-3-opus-20240229".to_string(),
                    object: "model".to_string(),
                    created: chrono::Utc::now().timestamp(),
                    owned_by: "anthropic".to_string(),
                },
            ])
        }
        ProviderType::HuggingFace => {
            // Hugging Face returns different structure
            let hf_response: Value = response.json().await?;
            let mut models = Vec::new();

            if let Some(models_array) = hf_response.as_array() {
                for model in models_array {
                    if let Some(model_id) = model["id"].as_str() {
                        models.push(Model {
                            id: model_id.to_string(),
                            object: "model".to_string(),
                            created: chrono::Utc::now().timestamp(),
                            owned_by: "huggingface".to_string(),
                        });
                    }
                }
            }
            Ok(models)
        }
        // Local AI coding agents - return default models
        ProviderType::ClaudeCode | ProviderType::GeminiCLI | ProviderType::CodexCLI |
        ProviderType::CursorCLI | ProviderType::QwenCode | ProviderType::ZAIGLM |
        ProviderType::Aider | ProviderType::CodeiumChat | ProviderType::CopilotCLI |
        ProviderType::Tabnine => {
            // Return default models for local agents
            let agent_name = match provider_type {
                ProviderType::ClaudeCode => "claude-code",
                ProviderType::GeminiCLI => "gemini-cli",
                ProviderType::CodexCLI => "codex-cli",
                ProviderType::CursorCLI => "cursor-cli",
                ProviderType::QwenCode => "qwen-code",
                ProviderType::ZAIGLM => "zaiglm",
                ProviderType::Aider => "aider",
                ProviderType::CodeiumChat => "codeium-chat",
                ProviderType::CopilotCLI => "copilot-cli",
                ProviderType::Tabnine => "tabnine",
                _ => "local-agent",
            };
            Ok(vec![Model {
                id: agent_name.to_string(),
                object: "model".to_string(),
                created: chrono::Utc::now().timestamp(),
                owned_by: "local".to_string(),
            }])
        }
    }
}

/// Generate streaming response using the specified agent and provider
pub async fn generate_sse_stream(
    agent: &AgentWithProvider,
    messages: Vec<ChatMessagePair>,
    sender: mpsc::Sender<Result<GenerationEvent, Error>>,
    service_type: StreamServiceType,
) -> Result<(), Box<dyn std::error::Error>> {
    match service_type {
        StreamServiceType::Chat => {
            if !agent.provider.supports_chat {
                return Err("Provider does not support chat services".into());
            }
            generate_chat_stream(agent, messages, sender).await
        }
        StreamServiceType::Embedding => {
            if !agent.provider.supports_embed {
                return Err("Provider does not support embedding services".into());
            }
            generate_embeddings(agent, messages, sender).await
        }
        StreamServiceType::ImageGeneration => {
            if !agent.provider.supports_image {
                return Err("Provider does not support image generation services".into());
            }
            generate_image_generation(agent, messages, sender).await
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StreamServiceType {
    Chat,
    Embedding,
    ImageGeneration,
}

/// Generate streaming response for chat services
async fn generate_chat_stream(
    agent: &AgentWithProvider,
    messages: Vec<ChatMessagePair>,
    sender: mpsc::Sender<Result<GenerationEvent, Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    match agent.provider.provider_type {
        // OpenAI-compatible chat providers
        ProviderType::OpenAI | ProviderType::OpenRouter | ProviderType::DeepSeek |
        ProviderType::AzureOpenAI | ProviderType::Groq | ProviderType::MistralAI | ProviderType::XAI => {
            generate_openai_compatible_stream(agent, messages, sender).await
        }
        // Specialized chat providers
        ProviderType::Gemini => {
            generate_gemini_stream(agent, messages, sender).await
        }
        ProviderType::Anthropic => {
            generate_anthropic_stream(agent, messages, sender).await
        }
        ProviderType::Cohere => {
            generate_cohere_stream(agent, messages, sender).await
        }
        ProviderType::HuggingFace => {
            generate_huggingface_stream(agent, messages, sender).await
        }
        // Local AI coding agents
        ProviderType::ClaudeCode | ProviderType::GeminiCLI | ProviderType::CodexCLI |
        ProviderType::CursorCLI | ProviderType::QwenCode | ProviderType::ZAIGLM |
        ProviderType::Aider | ProviderType::CodeiumChat | ProviderType::CopilotCLI |
        ProviderType::Tabnine => {
            generate_local_agent_stream(agent, messages, sender).await
        }
    }
}

/// Generate streaming response for OpenAI-compatible providers (OpenAI, OpenRouter, DeepSeek, XAI)
async fn generate_openai_compatible_stream(
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

    // Add tools if configured
    if agent.tool && !agent.tools.is_empty() {
        let tools: Vec<Value> = agent.tools.iter().map(|tool_name| {
            json!({
                "type": "function",
                "function": {
                    "name": tool_name,
                    "description": format!("Tool: {}", tool_name),
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }
            })
        }).collect();

        request_body["tools"] = json!(tools);

        // For tool calls that require approval, we use "auto" mode but will handle approval in UI
        request_body["tool_choice"] = json!("auto");
    }

    println!("{} Request: {:?}", agent.provider.provider_type, request_body);

    let client = reqwest::Client::new();
    let mut request = client
        .post(&url)
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    // Add provider-specific headers
    match agent.provider.provider_type {
        ProviderType::OpenAI | ProviderType::DeepSeek | ProviderType::XAI => {
            request = request.header(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key))?,
            );
        }
        ProviderType::OpenRouter => {
            request = request.header(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key))?,
            );
            // OpenRouter-specific header for model routing
            request = request.header(
                "HTTP-Referer",
                HeaderValue::from_str("https://github.com/sternelee/axum-chat")?,
            );
            request = request.header(
                "X-Title",
                HeaderValue::from_str("Axum Chat")?,
            );
        }
        ProviderType::AzureOpenAI => {
            request = request.header(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key))?,
            );
            // Azure-specific headers
            request = request.header(
                "api-key",
                HeaderValue::from_str(api_key)?,
            );
        }
        ProviderType::Groq => {
            request = request.header(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key))?,
            );
        }
        ProviderType::MistralAI => {
            request = request.header(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key))?,
            );
        }
        _ => {}
    }

    request = request.body(request_body.to_string());

    let mut stream = ReqwestEventSource::new(request)?;

    while let Some(event) = stream.next().await {
        match event {
            Ok(ReqwestEvent::Open) => println!("{} Connection Open!", agent.provider.provider_type),
            Ok(ReqwestEvent::Message(message)) => {
                if message.data.trim() == "[DONE]" {
                    println!("{} Stream completed.", agent.provider.provider_type);
                    stream.close();
                    send_end_event(&sender).await;
                    break;
                } else {
                    if let Ok(parsed) = serde_json::from_str::<Value>(&message.data) {
                        let choice = &parsed["choices"][0];
                        let delta = &choice["delta"];

                        // Handle reasoning/thinking content (for models like o1)
                        if let Some(reasoning_content) = choice["delta"]["reasoning_content"].as_str() {
                            let thinking_id = format!("thinking_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
                            if sender.send(Ok(GenerationEvent::Thinking(ThinkingEvent {
                                id: thinking_id.clone(),
                                content: reasoning_content.to_string(),
                                is_final: false,
                                is_collapsed: false,
                            }))).await.is_err() {
                                break;
                            }
                        }

                        // Handle tool calls
                        if let Some(tool_calls) = delta["tool_calls"].as_array() {
                            for tool_call in tool_calls {
                                if let (Some(_index), Some(call_id), Some(function)) = (
                                    tool_call["index"].as_u64(),
                                    tool_call["id"].as_str(),
                                    tool_call["function"].as_object()
                                ) {
                                    let tool_name = function.get("name").and_then(|n| n.as_str()).unwrap_or("");
                                    let tool_args = function.get("arguments").and_then(|a| a.as_str()).unwrap_or("{}");

                                    // Check if this tool is auto-approved
                                    let requires_approval = !agent.allow_tools.contains(&tool_name.to_string());

                                    if sender.send(Ok(GenerationEvent::ToolCall(ToolCallEvent {
                                        id: call_id.to_string(),
                                        name: tool_name.to_string(),
                                        arguments: tool_args.to_string(),
                                        description: Some(format!("Execute tool: {}", tool_name)),
                                        requires_approval,
                                    }))).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }

                        // Handle regular text content
                        if let Some(text) = delta["content"].as_str() {
                            if sender.send(Ok(GenerationEvent::Text(text.to_string()))).await.is_err() {
                                break;
                            }
                        }

                        // Handle finish reason
                        if let Some(finish_reason) = choice["finish_reason"].as_str() {
                            match finish_reason {
                                "tool_calls" => {
                                    // Tool calls completed, wait for user approval
                                    println!("Tool calls completed, waiting for approval");
                                }
                                "stop" => {
                                    // Normal completion
                                    if sender.send(Ok(GenerationEvent::End(
                                        r#"<div id="sse-listener" hx-swap-oob="true"></div>"#.to_string(),
                                    ))).await.is_err() {
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            Err(err) => {
                println!("{} Error: {}", agent.provider.provider_type, err);
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
                part_type: GeminiPartType::Text {
                    text: format!("System: {}", system_prompt),
                },
            }],
            role: "user".to_string(),
        });

        // Add a model response to acknowledge the system instruction
        contents.push(GeminiContent {
            parts: vec![GeminiPart {
                part_type: GeminiPartType::Text {
                    text: "Understood. I will follow these instructions.".to_string(),
                },
            }],
            role: "model".to_string(),
        });
    }

    // Add conversation history
    for msg in &messages {
        contents.push(GeminiContent {
            parts: vec![GeminiPart {
                part_type: GeminiPartType::Text {
                    text: msg.human_message.clone(),
                },
            }],
            role: "user".to_string(),
        });

        if let Some(ai_message) = &msg.ai_message {
            contents.push(GeminiContent {
                parts: vec![GeminiPart {
                    part_type: GeminiPartType::Text {
                        text: ai_message.clone(),
                    },
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

    // Build tools if configured
    let tools = if agent.tool && !agent.tools.is_empty() {
        Some(vec![GeminiTool {
            function_declarations: agent.tools.iter().map(|tool_name| {
                GeminiFunctionDeclaration {
                    name: tool_name.clone(),
                    description: format!("Tool: {}", tool_name),
                    parameters: json!({
                        "type": "object",
                        "properties": {},
                        "required": []
                    }),
                }
            }).collect(),
        }])
    } else {
        None
    };

    // Configure tool calling
    let tool_config = if tools.is_some() {
        Some(GeminiToolConfig {
            function_calling_config: GeminiFunctionCallingConfig {
                mode: "AUTO".to_string(),
                allowed_function_names: None,
            },
        })
    } else {
        None
    };

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
        tools,
        tool_config,
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
                        // Process all parts in the content
                        for part in &candidate.content.parts {
                            match &part.part_type {
                                GeminiPartType::Text { text } => {
                                    if sender.send(Ok(GenerationEvent::Text(text.clone()))).await.is_err() {
                                        break;
                                    }
                                }
                                GeminiPartType::FunctionCall { function_call } => {
                                    let thinking_id = format!("thinking_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
                                    // Send a thinking event first to indicate AI is reasoning about the tool call
                                    if sender.send(Ok(GenerationEvent::Thinking(ThinkingEvent {
                                        id: thinking_id.clone(),
                                        content: format!("I need to call the '{}' function to help with this request.", function_call.name),
                                        is_final: true,
                                        is_collapsed: false,
                                    }))).await.is_err() {
                                        break;
                                    }

                                    // Then send the tool call event
                                    let tool_call_id = format!("tool_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));
                                    let args_json = serde_json::to_string(&function_call.args).unwrap_or_else(|_| "{}".to_string());

                                    // Check if this tool is auto-approved
                                    let requires_approval = !agent.allow_tools.contains(&function_call.name);

                                    if sender.send(Ok(GenerationEvent::ToolCall(ToolCallEvent {
                                        id: tool_call_id,
                                        name: function_call.name.clone(),
                                        arguments: args_json,
                                        description: Some(format!("Execute Gemini function: {}", function_call.name)),
                                        requires_approval,
                                    }))).await.is_err() {
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }

                        // Check if generation is complete
                        if let Some(finish_reason) = &candidate.finish_reason {
                            match finish_reason.as_str() {
                                "STOP" => {
                                    println!("Gemini Stream completed normally.");
                                    stream.close();
                                    send_end_event(&sender).await;
                                    break;
                                }
                                "MAX_TOKENS" => {
                                    println!("Gemini Stream completed: Max tokens reached.");
                                    if sender.send(Ok(GenerationEvent::End(
                                        r#"<div id="sse-listener" hx-swap-oob="true"></div>"#.to_string(),
                                    ))).await.is_err() {
                                        break;
                                    }
                                }
                                _ => {
                                    println!("Gemini Stream completed with reason: {:?}", finish_reason);
                                    stream.close();
                                    send_end_event(&sender).await;
                                    break;
                                }
                            }
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

/// Process tool call approval/rejection
pub async fn process_tool_response(
    tool_call_id: &str,
    status: &str,
    result: Option<String>,
    error: Option<String>,
    sender: &mpsc::Sender<Result<GenerationEvent, Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let response_event = ToolResponseEvent {
        id: format!("tool_response_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)),
        call_id: tool_call_id.to_string(),
        status: status.to_string(),
        result,
        error,
    };

    if sender.send(Ok(GenerationEvent::ToolResponse(response_event))).await.is_err() {
        return Err("Failed to send tool response".into());
    }

    Ok(())
}

/// Generate HTML for thinking content with collapse functionality
pub fn generate_thinking_html(event: &ThinkingEvent) -> String {
    let thinking_target = format!("#thinking_content_{}", event.id);
    format!(r#"
<div class="thinking-container mb-4" id="thinking_{}">
    <div class="thinking-header bg-gray-100 p-3 rounded-t-lg cursor-pointer hover:bg-gray-200 transition-colors"
         hx-toggle="collapse"
         hx-target="{}">
        <div class="flex items-center justify-between ">
            <div class="flex items-center space-x-2">
                <svg class="w-5 h-5 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                          d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z"></path>
                </svg>
                <span class="font-medium text-gray-700">Thinking Process</span>
                <span class="text-sm text-gray-500">{}</span>
            </div>
            <div class="flex items-center space-x-2">
                <span class="thinking-toggle text-xs text-gray-500">
                    <span class="collapse-show ">Show</span>
                    <span class="collapse-hide ">Hide</span>
                </span>
                <svg class="w-4 h-4 transition-transform collapse-icon " fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
                </svg>
            </div>
        </div>
    </div>
    <div class="thinking-content bg-white border border-t-0 border-gray-200 rounded-b-lg p-4 {}"
         id="thinking_content_{}">
        <div class="prose prose-sm max-w-none ">
            <pre class="whitespace-pre-wrap text-sm text-gray-700 font-mono ">{}</pre>
        </div>
    </div>
</div>
    "#,
        event.id,
        thinking_target,
        chrono::Utc::now().format("%H:%M:%S"),
        if event.is_collapsed { "hidden" } else { "" },
        event.id,
        html_escape::encode_text_minimal(&event.content)
    )
}

/// Generate HTML for tool call approval form
pub fn generate_tool_call_html(event: &ToolCallEvent) -> String {
    let tool_target = format!("#tool_{}", event.id);
    format!(r#"
<div class="tool-call-container mb-4 border border-yellow-200 rounded-lg bg-yellow-50" id="tool_{}">
    <div class="p-4">
        <div class="flex items-center justify-between mb-3">
            <div class="flex items-center space-x-2">
                <svg class="w-5 h-5 text-yellow-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                          d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
                </svg>
                <span class="font-medium text-yellow-800">Tool Call Required</span>
            </div>
            <span class="text-xs text-yellow-600 bg-yellow-100 px-2 py-1 rounded">ID: {}</span>
        </div>

        <div class="mb-3">
            <div class="text-sm font-medium text-gray-700 mb-1">Function:</div>
            <div class="text-sm font-mono bg-white px-2 py-1 rounded border">{}</div>
        </div>

        <div class="mb-3">
            <div class="text-sm font-medium text-gray-700 mb-1">Arguments:</div>
            <pre class="text-xs bg-gray-900 text-green-400 p-2 rounded overflow-x-auto">{}</pre>
        </div>

        {}

        <div class="mb-3 pb-3 border-b border-yellow-200">
            <form hx-post="/api/approve-all-tools" hx-target="{}" hx-swap="outerHTML">
                <input type="hidden" name="tool_name" value="{}">
                <button type="submit"
                        class="w-full bg-blue-500 hover:bg-blue-600 text-white px-3 py-2 rounded text-sm font-medium transition-colors">
                    <svg class="w-4 h-4 inline mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                    </svg>
                    Approve All "{}" (Auto-approved in future)
                </button>
            </form>
        </div>

        <div class="flex space-x-2">
            <form hx-post="/api/approve-tool" hx-target="{}" hx-swap="outerHTML" class="flex-1">
                <input type="hidden" name="tool_call_id" value="{}">
                <input type="hidden" name="status" value="approved">
                <button type="submit"
                        class="w-full bg-green-500 hover:bg-green-600 text-white px-3 py-2 rounded text-sm font-medium transition-colors ">
                    <svg class="w-4 h-4 inline mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                    </svg>
                    Approve Once
                </button>
            </form>

            <form hx-post="/api/reject-tool" hx-target="{}" hx-swap="outerHTML" class="flex-1">
                <input type="hidden" name="tool_call_id" value="{}">
                <input type="hidden" name="status" value="rejected">
                <button type="submit"
                        class="w-full bg-red-500 hover:bg-red-600 text-white px-3 py-2 rounded text-sm font-medium transition-colors ">
                    <svg class="w-4 h-4 inline mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
                    </svg>
                    Reject
                </button>
            </form>
        </div>
    </div>
</div>
    "#,
        event.id,
        event.id,
        html_escape::encode_text_minimal(&event.name),
        html_escape::encode_text_minimal(&event.arguments),
        if let Some(desc) = &event.description {
            format!(r#"<div class="mb-3"><div class="text-sm font-medium text-gray-700 mb-1">Description:</div><div class="text-sm text-gray-600">{}</div></div>"#, desc)
        } else {
            String::new()
        },
        tool_target,
        html_escape::encode_text_minimal(&event.name),
        html_escape::encode_text_minimal(&event.name),
        tool_target, event.id,
        tool_target, event.id
    )
}

/// Generate HTML for tool response
pub fn generate_tool_response_html(event: &ToolResponseEvent) -> String {
    let (color, icon, status_text) = match event.status.as_str() {
        "approved" => ("green", "✓", "Approved"),
        "rejected" => ("red", "✗", "Rejected"),
        "executed" => ("blue", "⚡", "Executed"),
        _ => ("gray", "?", "Unknown"),
    };

    format!(r#"
<div class="tool-response-container mb-4 border border-{}-200 rounded-lg bg-{}-50" id="tool_response_{}">
    <div class="p-4">
        <div class="flex items-center justify-between mb-3">
            <div class="flex items-center space-x-2">
                <span class="text-lg">{}</span>
                <span class="font-medium text-{}-800">Tool Call {}</span>
            </div>
            <span class="text-xs text-{}-600 bg-{}-100 px-2 py-1 rounded">Call ID: {}</span>
        </div>

        {}

        {}
    </div>
</div>
    "#,
        color, color, event.id,
        icon, color, status_text,
        color, color, event.call_id,
        if let Some(result) = &event.result {
            format!(r#"<div class="mb-2"><div class="text-sm font-medium text-gray-700 mb-1">Result:</div><pre class="text-sm bg-gray-100 p-2 rounded overflow-x-auto">{}</pre></div>"#, html_escape::encode_text_minimal(result))
        } else {
            String::new()
        },
        if let Some(error) = &event.error {
            format!(r#"<div class="mb-2"><div class="text-sm font-medium text-red-700 mb-1">Error:</div><pre class="text-sm bg-red-100 text-red-800 p-2 rounded overflow-x-auto">{}</pre></div>"#, html_escape::encode_text_minimal(error))
        } else {
            String::new()
        }
    )
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
                chat_endpoint: Some("/chat/completions".to_string()),
                embed_endpoint: Some("/embeddings".to_string()),
                image_endpoint: Some("/images/generations".to_string()),
                api_key_encrypted: "test_key".to_string(),
                supports_chat: true,
                supports_embed: false,
                supports_image: false,
                supports_streaming: true,
                supports_tools: true,
                supports_images: false,
                is_active: true,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
                local_agent_config: None,
            },
            model_name: "gpt-4".to_string(),
            stream: true,
            chat: true,
            embed: false,
            image: false,
            tool: false,
            tools: vec![],
            allow_tools: vec![],
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
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
            let _ = generate_chat_stream(&test_agent, pairs, _sender).await;
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
                chat_endpoint: Some("/models/{model}:generateContent".to_string()),
                embed_endpoint: Some("/models/{model}:embedContent".to_string()),
                image_endpoint: None,
                api_key_encrypted: "test_gemini_key".to_string(),
                supports_chat: true,
                supports_embed: false,
                supports_image: false,
                supports_streaming: true,
                supports_tools: true,
                supports_images: false,
                is_active: true,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
                local_agent_config: None,
            },
            model_name: "gemini-1.5-pro".to_string(),
            stream: true,
            chat: true,
            embed: false,
            image: false,
            tool: false,
            tools: vec![],
            allow_tools: vec![],
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
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
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

/// Generate streaming response for Anthropic Claude
async fn generate_anthropic_stream(
    agent: &AgentWithProvider,
    messages: Vec<ChatMessagePair>,
    sender: mpsc::Sender<Result<GenerationEvent, Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let api_key = &agent.provider.api_key_encrypted;
    let base_url = &agent.provider.base_url;
    let model = &agent.model_name;

    let url = format!("{}/messages", base_url);

    // Convert messages to Anthropic format
    let mut anthropic_messages = Vec::new();

    // Add conversation history
    for msg in &messages {
        anthropic_messages.push(AnthropicMessage {
            role: "user".to_string(),
            content: vec![AnthropicContent {
                r#type: "text".to_string(),
                text: Some(msg.human_message.clone()),
            }],
        });

        if let Some(ai_message) = &msg.ai_message {
            anthropic_messages.push(AnthropicMessage {
                role: "assistant".to_string(),
                content: vec![AnthropicContent {
                    r#type: "text".to_string(),
                    text: Some(ai_message.clone()),
                }],
            });
        }
    }

    // Build request body
    let request_body = AnthropicStreamRequest {
        model: model.clone(),
        messages: anthropic_messages,
        max_tokens: agent.max_tokens as i32,
        temperature: agent.temperature,
        stream: true,
    };

    let client = reqwest::Client::new();
    let request = client
        .post(&url)
        .header(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", api_key))?)
        .header("anthropic-version", "2023-06-01")
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .body(serde_json::to_string(&request_body)?);

    let mut stream = ReqwestEventSource::new(request)?;

    while let Some(event) = stream.next().await {
        match event {
            Ok(ReqwestEvent::Open) => println!("Anthropic Connection Open!"),
            Ok(ReqwestEvent::Message(message)) => {
                if let Ok(response) = serde_json::from_str::<AnthropicStreamResponse>(&message.data) {
                    if let Some(delta) = response.delta {
                        if sender.send(Ok(GenerationEvent::Text(delta.text))).await.is_err() {
                            break;
                        }

                        if let Some(stop_reason) = delta.stop_reason {
                            println!("Anthropic Stream completed with reason: {}", stop_reason);
                            break;
                        }
                    }
                }
            }
            Err(err) => {
                println!("Anthropic Error: {}", err);
                if sender.send(Err(axum::Error::new(err))).await.is_err() {
                    break;
                }
            }
        }
    }

    send_end_event(&sender).await;
    Ok(())
}

/// Generate streaming response for Cohere
async fn generate_cohere_stream(
    agent: &AgentWithProvider,
    messages: Vec<ChatMessagePair>,
    sender: mpsc::Sender<Result<GenerationEvent, Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let api_key = &agent.provider.api_key_encrypted;
    let base_url = &agent.provider.base_url;
    let model = &agent.model_name;

    let url = format!("{}/chat", base_url);

    // Convert messages to Cohere format
    let mut chat_history = Vec::new();
    let mut last_message = String::new();

    for msg in &messages {
        chat_history.push(CohereMessage {
            role: "USER".to_string(),
            message: msg.human_message.clone(),
        });

        if let Some(ai_message) = &msg.ai_message {
            chat_history.push(CohereMessage {
                role: "CHATBOT".to_string(),
                message: ai_message.clone(),
            });
        }

        last_message = msg.human_message.clone();
    }

    // Build request body
    let request_body = CohereStreamRequest {
        model: model.clone(),
        message: last_message,
        chat_history,
        temperature: agent.temperature,
        max_tokens: agent.max_tokens as i32,
        stream: true,
    };

    let client = reqwest::Client::new();
    let request = client
        .post(&url)
        .header(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", api_key))?)
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .body(serde_json::to_string(&request_body)?);

    let mut stream = ReqwestEventSource::new(request)?;

    while let Some(event) = stream.next().await {
        match event {
            Ok(ReqwestEvent::Open) => println!("Cohere Connection Open!"),
            Ok(ReqwestEvent::Message(message)) => {
                if let Ok(response) = serde_json::from_str::<CohereStreamResponse>(&message.data) {
                    if sender.send(Ok(GenerationEvent::Text(response.text))).await.is_err() {
                        break;
                    }

                    if response.finish_reason == "MAX_TOKENS" || response.finish_reason == "COMPLETE" {
                        println!("Cohere Stream completed with reason: {}", response.finish_reason);
                        break;
                    }
                }
            }
            Err(err) => {
                println!("Cohere Error: {}", err);
                if sender.send(Err(axum::Error::new(err))).await.is_err() {
                    break;
                }
            }
        }
    }

    send_end_event(&sender).await;
    Ok(())
}

/// Generate streaming response for Hugging Face
async fn generate_huggingface_stream(
    agent: &AgentWithProvider,
    messages: Vec<ChatMessagePair>,
    sender: mpsc::Sender<Result<GenerationEvent, Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let api_key = &agent.provider.api_key_encrypted;
    let base_url = &agent.provider.base_url;
    let model = &agent.model_name;

    let url = format!("{}/models/{}/generate_stream", base_url, model);

    // Combine messages into a single prompt for Hugging Face
    let mut prompt = String::new();
    for msg in &messages {
        prompt.push_str(&format!("Human: {}\n\nAssistant: ", msg.human_message));
        if let Some(ai_message) = &msg.ai_message {
            prompt.push_str(&format!("{}\n\n", ai_message));
        }
    }
    prompt.push_str("Assistant: ");

    // Build request body
    let request_body = HuggingFaceStreamRequest {
        inputs: prompt,
        parameters: HuggingFaceParameters {
            max_new_tokens: Some(agent.max_tokens as i32),
            temperature: Some(agent.temperature),
            top_p: Some(agent.top_p),
            return_full_text: Some(false),
        },
        stream: true,
    };

    let client = reqwest::Client::new();
    let request = client
        .post(&url)
        .header(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", api_key))?)
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .body(serde_json::to_string(&request_body)?);

    let mut stream = ReqwestEventSource::new(request)?;

    while let Some(event) = stream.next().await {
        match event {
            Ok(ReqwestEvent::Open) => println!("HuggingFace Connection Open!"),
            Ok(ReqwestEvent::Message(message)) => {
                if let Ok(response) = serde_json::from_str::<Value>(&message.data) {
                    if let Some(token) = response["token"]["text"].as_str() {
                        if sender.send(Ok(GenerationEvent::Text(token.to_string()))).await.is_err() {
                            break;
                        }
                    }

                    // Check for completion
                    if response.get("generated_text").is_some() {
                        println!("HuggingFace Stream completed");
                        break;
                    }
                }
            }
            Err(err) => {
                println!("HuggingFace Error: {}", err);
                if sender.send(Err(axum::Error::new(err))).await.is_err() {
                    break;
                }
            }
        }
    }

    send_end_event(&sender).await;
    Ok(())
}

/// Generate images using image generation providers
async fn generate_image_generation(
    agent: &AgentWithProvider,
    messages: Vec<ChatMessagePair>,
    sender: mpsc::Sender<Result<GenerationEvent, Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let api_key = &agent.provider.api_key_encrypted;
    let base_url = &agent.provider.base_url;
    let model = &agent.model_name;

    // Get the last user message as the image prompt
    let prompt = messages.last()
        .map(|msg| msg.human_message.clone())
        .unwrap_or_default();

    let (url, request_body) = match agent.provider.provider_type {
        ProviderType::OpenAI | ProviderType::OpenRouter | ProviderType::AzureOpenAI => {
            let url = format!("{}/images/generations", base_url);
            let body = ImageGenerationRequest {
                prompt: prompt.clone(),
                n: Some(1),
                size: Some("1024x1024".to_string()),
                quality: Some("standard".to_string()),
                model: model.clone(),
            };
            (url, serde_json::to_value(body)?)
        }
        _ => return Err("Unsupported image generation provider".into()),
    };

    let client = reqwest::Client::new();
    let mut request = client
        .post(&url)
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    // Add appropriate authentication
    request = request.header(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );

    let response = request.body(request_body.to_string()).send().await?;
    let image_response: ImageGenerationResponse = response.json().await?;

    // Send generated image URLs as text events
    for image_data in image_response.data {
        let image_html = format!(
            r#"<img src="{}" alt="Generated image" style="max-width: 100%; height: auto; border-radius: 8px;">"#,
            image_data.url
        );

        if sender.send(Ok(GenerationEvent::Text(image_html))).await.is_err() {
            break;
        }
    }

    send_end_event(&sender).await;
    Ok(())
}

/// Generate embeddings using embedding providers
async fn generate_embeddings(
    agent: &AgentWithProvider,
    messages: Vec<ChatMessagePair>,
    sender: mpsc::Sender<Result<GenerationEvent, Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let api_key = &agent.provider.api_key_encrypted;
    let base_url = &agent.provider.base_url;
    let model = &agent.model_name;

    // Get the last user message for embedding
    let text = messages.last()
        .map(|msg| msg.human_message.clone())
        .unwrap_or_default();

    let (url, request_body) = match agent.provider.provider_type {
        ProviderType::OpenAI | ProviderType::OpenRouter | ProviderType::AzureOpenAI => {
            let url = format!("{}/embeddings", base_url);
            let body = EmbeddingRequest {
                model: model.clone(),
                input: text.clone(),
                encoding_format: Some("float".to_string()),
                dimensions: Some(1536),
            };
            (url, serde_json::to_value(body)?)
        }
        ProviderType::Cohere => {
            let url = format!("{}/embed", base_url);
            let body = EmbeddingRequest {
                model: model.clone(),
                input: text.clone(),
                encoding_format: None,
                dimensions: Some(4096),
            };
            (url, serde_json::to_value(body)?)
        }
        ProviderType::HuggingFace => {
            let url = format!("{}/pipeline/feature-extraction", base_url);
            let body = EmbeddingRequest {
                model: model.clone(),
                input: text.clone(),
                encoding_format: None,
                dimensions: None,
            };
            (url, serde_json::to_value(body)?)
        }
        _ => return Err("Unsupported embedding provider".into()),
    };

    let client = reqwest::Client::new();
    let mut request = client
        .post(&url)
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .header(AUTHORIZATION, HeaderValue::from_str(&format!("Bearer {}", api_key))?)
        .body(request_body.to_string());

    let response = request.send().await?;
    let embedding_response: EmbeddingResponse = response.json().await?;

    // Send embedding information as a text event
    let embedding_info = format!(
        "Generated embedding with {} dimensions for model '{}'. Input: '{}'",
        embedding_response.data[0].embedding.len(),
        embedding_response.model,
        text
    );

    if sender.send(Ok(GenerationEvent::Text(embedding_info))).await.is_err() {
        return Err("Failed to send embedding info".into());
    }

    send_end_event(&sender).await;
    Ok(())
}

/// Generate streaming response for local AI coding agents
async fn generate_local_agent_stream(
    agent: &AgentWithProvider,
    messages: Vec<ChatMessagePair>,
    sender: mpsc::Sender<Result<GenerationEvent, Error>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // For now, we'll use a simple implementation that treats local agents like OpenAI-compatible providers
    // This could be enhanced to use the LocalAgentManager and LocalAgentClient
    generate_openai_compatible_stream(agent, messages, sender).await
}