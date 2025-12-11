use serde::{Deserialize, Serialize};
use reqwest::Client;
use std::time::Duration;
use tokio::time::timeout as tokio_timeout;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAgentRequest {
    pub messages: Vec<ChatMessage>,
    pub model: String,
    pub temperature: f64,
    pub max_tokens: u32,
    pub stream: bool,
    pub system_prompt: Option<String>,
    pub tools: Option<Vec<Tool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalAgentResponse {
    pub content: String,
    pub model: String,
    pub usage: Option<TokenUsage>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    pub content: Option<String>,
    pub delta: Option<String>,
    pub finish_reason: Option<String>,
    pub usage: Option<TokenUsage>,
}

pub struct LocalAgentClient {
    client: Client,
    base_url: String,
    request_timeout: Duration,
}

impl LocalAgentClient {
    pub fn new(base_url: String, request_timeout: u64) -> Self {
        Self {
            client: Client::new(),
            base_url,
            request_timeout: Duration::from_secs(request_timeout),
        }
    }

    pub async fn health_check(&self) -> Result<bool, String> {
        let url = format!("{}/health", self.base_url);

        match tokio_timeout(
            Duration::from_secs(5),
            self.client.get(&url).send()
        ).await {
            Ok(Ok(response)) => Ok(response.status().is_success()),
            Ok(Err(e)) => Err(format!("Health check request failed: {}", e)),
            Err(_) => Err("Health check timeout".to_string()),
        }
    }

    pub async fn send_request(&self, request: LocalAgentRequest) -> Result<LocalAgentResponse, String> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let response = tokio_timeout(
            self.request_timeout,
            self.client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
        ).await
        .map_err(|_| "Request timeout".to_string())?
        .map_err(|e| format!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Request failed with status: {}", response.status()));
        }

        response
            .json::<LocalAgentResponse>()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    pub async fn send_stream_request<F>(
        &self,
        request: LocalAgentRequest,
        mut on_chunk: F,
    ) -> Result<(), String>
    where
        F: FnMut(StreamChunk) -> Result<(), String>,
    {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let response = tokio_timeout(
            self.request_timeout,
            self.client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
        ).await
        .map_err(|_| "Request timeout".to_string())?
        .map_err(|e| format!("Failed to send request: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Stream request failed with status: {}", response.status()));
        }

        let mut stream = response.bytes_stream();
        use futures_util::StreamExt;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| format!("Stream error: {}", e))?;

            // Parse SSE format (data: {...})
            let chunk_str = String::from_utf8_lossy(&chunk);
            for line in chunk_str.lines() {
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data.trim() == "[DONE]" {
                        return Ok(());
                    }

                    match serde_json::from_str::<StreamChunk>(data) {
                        Ok(stream_chunk) => {
                            on_chunk(stream_chunk)?;
                        }
                        Err(e) => {
                            // Log error but continue processing
                            eprintln!("Failed to parse stream chunk: {}", e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn get_capabilities(&self) -> Result<Vec<String>, String> {
        let url = format!("{}/v1/capabilities", self.base_url);

        let response = tokio_timeout(
            Duration::from_secs(10),
            self.client.get(&url).send()
        ).await
        .map_err(|_| "Capabilities request timeout".to_string())?
        .map_err(|e| format!("Failed to get capabilities: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Capabilities request failed with status: {}", response.status()));
        }

        #[derive(Deserialize)]
        struct CapabilitiesResponse {
            capabilities: Vec<String>,
        }

        let caps_response: CapabilitiesResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse capabilities: {}", e))?;

        Ok(caps_response.capabilities)
    }
}