use axum::Error;
use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest_eventsource::{Event as ReqwestEvent, EventSource as ReqwestEventSource};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::select;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

use crate::data::model::{ChatMessagePair, ToolCallConfirmation};
use crate::mcp::tools::{execute_mcp_tool_streaming, get_available_tools, parse_tool_call_from_ai};

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

pub async fn list_engines(api_key: &str) -> Result<Vec<Model>, reqwest::Error> {
    let client = reqwest::Client::new();
    let res: ModelList = client
        .get("https://api.siliconflow.cn/v1/models")
        .bearer_auth(api_key)
        .send()
        .await?
        .json()
        .await?;

    Ok(res.data)
}

#[derive(Serialize, Deserialize, Debug)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Clone)]
pub enum GenerationEvent {
    Text(String),
    Thinking(String),
    ThinkingUpdate(String),
    ToolCall(crate::data::model::ToolCall),
    ToolCallConfirmation(crate::data::model::ToolCallConfirmation),
    Image(String),
    Reasoning(String),
    ReasoningUpdate(String),
    Usage(crate::data::model::UsageInfo),
    Sources(Vec<crate::data::model::Source>),
    End(String),
}

pub async fn generate_sse_stream(
    api_key: &str,
    model: &str,
    messages: Vec<ChatMessagePair>,
    sender: mpsc::Sender<Result<GenerationEvent, Error>>,
    chat_id: Option<i64>,
    message_pair_id: Option<i64>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Monitor if the sender channel is closed (client disconnected)
    let mut sender_closed = false;

    // Track tool calls being built across streaming chunks
    let mut current_tool_calls: std::collections::HashMap<String, crate::data::model::ToolCall> = std::collections::HashMap::new();
    // Your OpenAI API key

    // The API endpoint for chat completions
    let url = "https://api.siliconflow.cn/v1/chat/completions";

    let system_message = json!({
        "role": "system",
        "content": "You are a helpful assistant. Use the available tools when they are relevant to the user's request. Always call tools to get the most accurate and up-to-date information."
    });
    let system_message_iter = std::iter::once(Some(system_message));

    // Create an iterator over the messages
    let messages_iter = messages.iter().flat_map(|msg| {
        let user_message = Some(json!({
            "role": "user",
            "content": msg.human_message
        }));

        let ai_message = msg.ai_message.as_ref().map(|ai_msg| {
            json!({
                "role": "assistant",
                "content": ai_msg
            })
        });

        std::iter::once(user_message).chain(std::iter::once(ai_message))
    });

    // Chain the system message with the user and AI messages, filter out the Nones, and collect into a Vec<Value>
    let body_messages = system_message_iter
        .chain(messages_iter)
        .flatten() // This removes any None values
        .collect::<Vec<Value>>();

    // Get available MCP tools and add them to the request
    let mcp_tools = match get_available_tools().await {
        Ok(tools) => tools,
        Err(e) => {
            eprintln!("Failed to get MCP tools: {}", e);
            vec![]
        }
    };

    // Prepare the request body with tools
    let mut body = json!({
        "model": model,
        "messages": body_messages,
        "stream": true
    });

    // Add tools to the request if any are available
    if !mcp_tools.is_empty() {
        println!("Found {} MCP tools to send to AI:", mcp_tools.len());
        for tool in &mcp_tools {
            println!("Tool: {} - {}", tool.name, tool.description);
        }

        let openai_tools: Vec<Value> = mcp_tools
            .into_iter()
            .map(|tool| {
                let tool_json = json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameters.unwrap_or(json!({
                            "type": "object",
                            "properties": {},
                            "required": []
                        }))
                    }
                });
                println!("Formatted tool for OpenAI: {}", serde_json::to_string_pretty(&tool_json).unwrap_or_default());
                tool_json
            })
            .collect();
        body["tools"] = serde_json::to_value(openai_tools).unwrap_or(Value::Array(vec![]));
        body["tool_choice"] = json!("auto");
    } else {
        println!("No MCP tools available for AI request");
    }

    println!("body: {}", body);

    // Create a client
    let client = reqwest::Client::new();

    // Create a request
    let request = client
        .post(url)
        .header(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))?,
        )
        .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
        .body(body.to_string());

    // Start streaming
    let mut stream = ReqwestEventSource::new(request)?;

    // Handle streaming events
    while let Some(event) = stream.next().await {
        // Check if sender is closed (client disconnected)
        if sender.is_closed() && !sender_closed {
            println!("Client disconnected, closing reqwest stream...");
            stream.close();
            sender_closed = true;
            break;
        }

        match event {
            Ok(ReqwestEvent::Open) => println!("Connection Open!"),
            Ok(ReqwestEvent::Message(message)) => {
                if message.data.trim() == "[DONE]" {
                    println!("Stream completed.");
                    stream.close();
                    if sender
                        .send(Ok(GenerationEvent::End(
                            r#"<div id="sse-listener" hx-swap-oob="true"></div>"#.to_string(),
                        )))
                        .await
                        .is_err()
                    {
                        break; // Receiver has dropped, stop sending.
                    }
                    break;
                } else {
                    let m: Value = serde_json::from_str(&message.data).unwrap();
                    let delta = &m["choices"][0]["delta"];

                    // Debug: Print the delta to see what AI is responding
                    if !delta.is_null() {
                        println!("AI delta: {}", serde_json::to_string_pretty(delta).unwrap_or_default());
                    }

                    // Handle thinking (for models like o1)
                    if let Some(thinking) = delta["thinking"].as_str() {
                        if sender
                            .send(Ok(GenerationEvent::Thinking(thinking.to_string())))
                            .await
                            .is_err()
                        {
                            println!("Client disconnected during thinking, closing stream...");
                            stream.close();
                            break;
                        }
                    }

                    // Handle reasoning content
                    if let Some(reasoning) = delta["reasoning_content"].as_str() {
                        if sender
                            .send(Ok(GenerationEvent::Reasoning(reasoning.to_string())))
                            .await
                            .is_err()
                        {
                            println!("Client disconnected during reasoning, closing stream...");
                            stream.close();
                            break;
                        }
                    }

                    // Handle tool calls
                    if let Some(tool_calls) = delta["tool_calls"].as_array() {
                        println!("Received {} tool calls from AI", tool_calls.len());
                        for tool_call_delta in tool_calls {
                            println!("Tool call delta: {}", serde_json::to_string_pretty(tool_call_delta).unwrap_or_default());

                            // Extract the tool call index to handle multi-part tool calls
                            let index = tool_call_delta.get("index").and_then(|i| i.as_i64()).unwrap_or(0) as usize;
                            let tool_key = format!("tool_{}", index);

                            // Get or create tool call entry
                            let tool_call = current_tool_calls.entry(tool_key.clone()).or_insert_with(|| {
                                crate::data::model::ToolCall {
                                    id: tool_call_delta.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()).unwrap_or_else(|| format!("call_{}", index)),
                                    r#type: tool_call_delta.get("type").and_then(|t| t.as_str()).unwrap_or("function").to_string(),
                                    function: crate::data::model::FunctionCall {
                                        name: String::new(),
                                        arguments: String::new(),
                                    },
                                }
                            });

                            // Update tool call fields if present in delta
                            if let Some(id) = tool_call_delta.get("id").and_then(|id| id.as_str()) {
                                tool_call.id = id.to_string();
                            }
                            if let Some(t_type) = tool_call_delta.get("type").and_then(|t| t.as_str()) {
                                tool_call.r#type = t_type.to_string();
                            }
                            if let Some(function_delta) = tool_call_delta.get("function") {
                                if let Some(function_obj) = function_delta.as_object() {
                                    if let Some(name) = function_obj.get("name").and_then(|n| n.as_str()) {
                                        tool_call.function.name = name.to_string();
                                    }
                                    if let Some(args) = function_obj.get("arguments").and_then(|a| a.as_str()) {
                                        tool_call.function.arguments.push_str(args);
                                    }
                                }
                            }

                            println!("Current tool call state for {}: {}", tool_key, serde_json::to_string(&tool_call).unwrap_or_default());

                            // Only process complete tool calls (those with both name and arguments)
                            if !tool_call.function.name.is_empty() && !tool_call.function.arguments.is_empty() {
                                println!("Processing complete tool call: {}", tool_call.function.name);

                                // Check if this is an MCP tool
                                let parsed_mcp = parse_tool_call_from_ai(&tool_call);
                                let is_mcp = parsed_mcp.is_some();
                                println!("Tool call '{}' is MCP: {}", tool_call.function.name, is_mcp);
                                if let Some(mcp_tool) = parsed_mcp {
                                    println!("Parsed MCP tool: {} with args: {}", mcp_tool.name, serde_json::to_string(&mcp_tool.arguments).unwrap_or_default());
                                } else {
                                    println!("Failed to parse as MCP tool, arguments: {}", tool_call.function.arguments);
                                }

                                if is_mcp {
                                    // Create tool call confirmation for MCP tools
                                    if let (Some(chat_id_val), Some(message_pair_id_val)) = (chat_id, message_pair_id) {
                                        let confirmation = crate::data::model::ToolCallConfirmation {
                                            id: tool_call.id.clone(),
                                            chat_id: chat_id_val,
                                            message_pair_id: message_pair_id_val,
                                            tool_call: tool_call.clone(),
                                            status: crate::data::model::ToolCallStatus::Pending,
                                            created_at: chrono::Utc::now(),
                                            user_response: None,
                                            result: None,
                                        };

                                        println!("Creating tool call confirmation for: {}", tool_call.function.name);

                                        // Save confirmation to database
                                        if let Err(e) = save_tool_call_confirmation(&confirmation).await {
                                            println!("Error saving tool call confirmation: {}", e);
                                            // Continue anyway and send the confirmation event
                                        }

                                        // Send confirmation request to UI
                                        if sender
                                            .send(Ok(GenerationEvent::ToolCallConfirmation(confirmation)))
                                            .await
                                            .is_err()
                                        {
                                            println!("Client disconnected during tool call confirmation, closing stream...");
                                            stream.close();
                                            break;
                                        }
                                    } else {
                                        // Fallback: Execute directly if no chat/message IDs
                                        if let Some(mcp_tool_call) = parse_tool_call_from_ai(&tool_call) {
                                            if let Err(e) = execute_mcp_tool_streaming(&mcp_tool_call, sender.clone()).await {
                                                println!("Error executing MCP tool: {}", e);
                                                let error_text = format!("Tool execution error: {}", e);
                                                if sender
                                                    .send(Ok(GenerationEvent::Text(error_text)))
                                                    .await
                                                    .is_err()
                                                {
                                                    println!("Client disconnected during tool error, closing stream...");
                                                    stream.close();
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    // Regular OpenAI tool call - just forward it
                                    println!("Forwarding regular tool call: {}", tool_call.function.name);
                                    if sender
                                        .send(Ok(GenerationEvent::ToolCall(tool_call.clone())))
                                        .await
                                        .is_err()
                                    {
                                        println!(
                                            "Client disconnected during tool call, closing stream..."
                                        );
                                        stream.close();
                                        break;
                                    }
                                }

                                // Remove processed tool call from tracking
                                current_tool_calls.remove(&tool_key);
                            }
                        }
                    }

                    // Handle regular text content
                    if let Some(text) = delta["content"].as_str() {
                        if sender
                            .send(Ok(GenerationEvent::Text(text.to_string())))
                            .await
                            .is_err()
                        {
                            println!("Client disconnected during text, closing stream...");
                            stream.close();
                            break;
                        }
                    }

                    // Handle usage information (usually in final message)
                    if let Some(usage_obj) = m["usage"].as_object() {
                        if let (Some(prompt), Some(completion), Some(total)) = (
                            usage_obj.get("prompt_tokens").and_then(|v| v.as_i64()),
                            usage_obj.get("completion_tokens").and_then(|v| v.as_i64()),
                            usage_obj.get("total_tokens").and_then(|v| v.as_i64()),
                        ) {
                            let usage = crate::data::model::UsageInfo {
                                prompt_tokens: prompt,
                                completion_tokens: completion,
                                total_tokens: total,
                            };
                            if sender
                                .send(Ok(GenerationEvent::Usage(usage)))
                                .await
                                .is_err()
                            {
                                println!("Client disconnected during usage, closing stream...");
                                stream.close();
                                break;
                            }
                        }
                    }
                }
            }
            Err(err) => {
                println!("Error: {}", err);
                stream.close();
                if sender.send(Err(axum::Error::new(err))).await.is_err() {
                    break; // Receiver has dropped, stop sending.
                }
            }
        }
    }

    println!("SSE stream generation completed or cancelled.");

    Ok(())
}

// Save tool call confirmation to database
async fn save_tool_call_confirmation(confirmation: &ToolCallConfirmation) -> Result<(), Box<dyn std::error::Error>> {
    let tool_call_json = serde_json::to_string(&confirmation.tool_call)?;
    let status_json = serde_json::to_string(&confirmation.status)?;
    let created_at_str = confirmation.created_at.to_rfc3339();

    sqlx::query!(
        r#"
        INSERT INTO tool_call_confirmations (id, chat_id, message_pair_id, tool_call, status, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(id) DO UPDATE SET
            status = excluded.status,
            user_response = excluded.user_response,
            result = excluded.result
        "#,
        confirmation.id,
        confirmation.chat_id,
        confirmation.message_pair_id,
        tool_call_json,
        status_json,
        created_at_str
    )
    .execute(crate::get_db_pool())
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use tokio_stream::wrappers::ReceiverStream;

    use super::*;

    #[tokio::test]
    async fn test_something_async() {
        // Create a channel for sending SSE events
        let (_sender, receiver) = mpsc::channel::<Result<GenerationEvent, axum::Error>>(10);

        // Convert the receiver end into a Stream
        let mut stream = ReceiverStream::new(receiver);

        // Read api key from .env
        let _api_key = dotenv::var("SILICONFLOW_API_KEY").unwrap();

        let _pairs = vec![ChatMessagePair {
            id: 1,
            chat_id: 1,
            message_block_id: 1,
            model: "gpt-4".to_string(),
            human_message: "Hello".to_string(),
            ai_message: Some("Hi there!".to_string()),
            block_rank: 1,
            block_size: 1,
        }];

        tokio::spawn(async move {
            generate_sse_stream(&_api_key, "gpt-4", _pairs, _sender, None, None)
                .await
                .unwrap();
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
