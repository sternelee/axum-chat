use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{sse::Event, Html, IntoResponse, Response, Sse},
    Form, Json,
};
use tokio::sync::mpsc;

use futures::stream::{self};
use serde::{Deserialize, Serialize};
use tera::Context;
use tokio_stream::wrappers::ReceiverStream; // This brings the necessary stream combinators into scope

use std::sync::Arc;

use crate::{
    ai::stream::{generate_sse_stream, list_engines, GenerationEvent},
    data::model::ChatMessagePair,
    utils::markdown_to_html,
    AppState, User,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_markdown_features() {
        let test_markdown = r#"| Name | Age |
|------|-----|
| John | 25   |

~~strikethrough~~

- [x] task done
- [ ] task pending

https://example.com

```
code block
```

# Heading 1

> This is a quote

[Link text](https://example.com)

```"#;

        let html = markdown_to_html(test_markdown);

        // 打印实际的 HTML 输出以便调试
        println!("Generated HTML:\n{}", html);

        // 验证 DaisyUI 表格样式
        assert!(
            html.contains(r#"class="table table-zebra w-full""#),
            "Should add DaisyUI table classes: {}",
            html
        );

        // 验证 DaisyUI 删除线样式
        assert!(
            html.contains(r#"class="line-through text-base-content/60""#),
            "Should add DaisyUI strikethrough classes: {}",
            html
        );

        // 验证 DaisyUI 任务列表样式
        assert!(
            html.contains(r#"class="checkbox checkbox-primary""#),
            "Should add DaisyUI checkbox classes: {}",
            html
        );

        // 验证 DaisyUI 链接样式
        assert!(
            html.contains(r#"class="link link-primary hover:underline""#),
            "Should add DaisyUI link classes: {}",
            html
        );

        // 验证 DaisyUI 代码块样式
        assert!(
            html.contains(r#"class="mockup-code""#),
            "Should add DaisyUI code block classes: {}",
            html
        );

        // 验证 DaisyUI 标题样式
        assert!(
            html.contains(r#"class="text-5xl font-bold mb-4""#),
            "Should add DaisyUI heading classes: {}",
            html
        );

        // 验证 DaisyUI 引用样式
        assert!(
            html.contains(
                r#"class="border-l-4 border-primary pl-4 italic my-4 bg-base-100 p-4 rounded""#
            ),
            "Should add DaisyUI blockquote classes: {}",
            html
        );

        // 验证代码块样式正确
        assert!(
            html.contains("mockup-code"),
            "Should add DaisyUI code block classes: {}",
            html
        );

        // 检查数学公式支持（如果不支持，不应该失败测试）
        let math_test = markdown_to_html("$E = mc^2$");
        println!("Math test output: {}", math_test);

        // 检查脚注支持
        let footnote_test = markdown_to_html("[^1]: footnote");
        println!("Footnote test output: {}", footnote_test);

        println!("✅ Enhanced markdown features with DaisyUI styling are working!");
    }
}

use tokio_stream::StreamExt as TokioStreamExt;

// Accumulator structure for all message types
#[derive(Clone)]
struct MessageAccumulator {
    text: String,
    thinking: String,
    reasoning: String,
    tool_calls: Vec<crate::data::model::ToolCall>,
    images: Vec<String>,
    usage: Option<crate::data::model::UsageInfo>,
    sources: Vec<crate::data::model::Source>,
}

fn render_message_html(acc: &MessageAccumulator) -> String {
    let mut html = String::new();

    // Render thinking section (collapsible)
    if !acc.thinking.is_empty() {
        html.push_str(r#"<div class="collapse collapse-arrow bg-base-200 mb-4">"#);
        html.push_str(r#"<input type="checkbox" id="thinking-collapse" />"#);
        html.push_str(r#"<div class="collapse-title text-sm font-medium flex items-center gap-2">"#);
        html.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" /></svg>"#);
        html.push_str("Thinking Process");
        html.push_str("</div>");
        html.push_str(r#"<div class="collapse-content"><div class="text-sm opacity-75 whitespace-pre-wrap">"#);
        html.push_str(&html_escape::encode_text(&acc.thinking));
        html.push_str("</div></div></div>");
    }

    // Render reasoning section (collapsible)
    if !acc.reasoning.is_empty() {
        html.push_str(r#"<div class="collapse collapse-arrow bg-base-200 mb-4">"#);
        html.push_str(r#"<input type="checkbox" id="reasoning-collapse" />"#);
        html.push_str(r#"<div class="collapse-title text-sm font-medium flex items-center gap-2">"#);
        html.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>"#);
        html.push_str("Reasoning");
        html.push_str("</div>");
        html.push_str(r#"<div class="collapse-content"><div class="text-sm opacity-75 whitespace-pre-wrap">"#);
        html.push_str(&html_escape::encode_text(&acc.reasoning));
        html.push_str("</div></div></div>");
    }
    
    // Render tool calls
    for tool_call in &acc.tool_calls {
        html.push_str(r#"<div class="card bg-accent/10 mb-4 border border-accent/20">"#);
        html.push_str(r#"<div class="card-body p-4">"#);
        html.push_str(r#"<div class="flex items-center gap-2 mb-2">"#);
        html.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /></svg>"#);
        html.push_str(r#"<span class="font-semibold text-accent">Tool Call: </span>"#);
        html.push_str(&html_escape::encode_text(&tool_call.function.name));
        html.push_str("</div>");
        html.push_str(r#"<div class="mockup-code text-xs"><pre><code>"#);
        // Pretty print JSON arguments if possible
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&tool_call.function.arguments) {
            if let Ok(pretty) = serde_json::to_string_pretty(&parsed) {
                html.push_str(&html_escape::encode_text(&pretty));
            } else {
                html.push_str(&html_escape::encode_text(&tool_call.function.arguments));
            }
        } else {
            html.push_str(&html_escape::encode_text(&tool_call.function.arguments));
        }
        html.push_str("</code></pre></div>");
        html.push_str("</div></div>");
    }
    
    // Render images
    for image_url in &acc.images {
        html.push_str(r#"<div class="mb-4"><img src=""#);
        html.push_str(&html_escape::encode_quoted_attribute(image_url));
        html.push_str(r#"" alt="Generated image" class="rounded-lg max-w-md shadow-lg" /></div>"#);
    }
    
    // Render main text content
    if !acc.text.is_empty() {
        html.push_str(&markdown_to_html(&acc.text));
    }
    
    // Render sources
    if !acc.sources.is_empty() {
        html.push_str(r#"<div class="divider mt-4">Sources</div>"#);
        html.push_str(r#"<div class="flex flex-col gap-2">"#);
        for (idx, source) in acc.sources.iter().enumerate() {
            html.push_str(r#"<div class="card bg-base-200 compact">"#);
            html.push_str(r#"<div class="card-body p-3">"#);
            html.push_str(r#"<div class="flex items-start gap-2">"#);
            html.push_str(&format!(r#"<span class="badge badge-primary badge-sm">{}</span>"#, idx + 1));
            html.push_str(r#"<div class="flex-1">"#);
            if let Some(title) = &source.title {
                html.push_str(r#"<h4 class="font-semibold text-sm">"#);
                html.push_str(&html_escape::encode_text(title));
                html.push_str("</h4>");
            }
            if let Some(snippet) = &source.snippet {
                html.push_str(r#"<p class="text-xs opacity-75 mt-1">"#);
                html.push_str(&html_escape::encode_text(snippet));
                html.push_str("</p>");
            }
            if let Some(url) = &source.url {
                html.push_str(r#"<a href=""#);
                html.push_str(&html_escape::encode_quoted_attribute(url));
                html.push_str(r#"" target="_blank" class="link link-primary text-xs mt-1">View source →</a>"#);
            }
            html.push_str("</div></div></div></div>");
        }
        html.push_str("</div>");
    }
    
    // Render usage statistics
    if let Some(usage) = &acc.usage {
        html.push_str(r#"<div class="stats stats-horizontal shadow mt-4 text-xs">"#);
        html.push_str(r#"<div class="stat py-2 px-4"><div class="stat-title text-xs">Prompt</div><div class="stat-value text-sm">"#);
        html.push_str(&usage.prompt_tokens.to_string());
        html.push_str(r#"</div><div class="stat-desc">tokens</div></div>"#);
        html.push_str(r#"<div class="stat py-2 px-4"><div class="stat-title text-xs">Completion</div><div class="stat-value text-sm">"#);
        html.push_str(&usage.completion_tokens.to_string());
        html.push_str(r#"</div><div class="stat-desc">tokens</div></div>"#);
        html.push_str(r#"<div class="stat py-2 px-4"><div class="stat-title text-xs">Total</div><div class="stat-value text-sm">"#);
        html.push_str(&usage.total_tokens.to_string());
        html.push_str(r#"</div><div class="stat-desc">tokens</div></div>"#);
        html.push_str("</div>");
    }
  
    html
}

// Helper function to render just thinking content
fn render_thinking_section(thinking: &str) -> String {
    if thinking.is_empty() {
        return String::new();
    }

    let mut html = String::new();
    html.push_str(r#"<div id="thinking-container" class="collapse collapse-arrow bg-base-200 mb-4">"#);
    html.push_str(r#"<input type="checkbox" id="thinking-collapse" />"#);
    html.push_str(r#"<div class="collapse-title text-sm font-medium flex items-center gap-2 cursor-pointer">"#);
    html.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z" /></svg>"#);
    html.push_str("Thinking Process");
    html.push_str("</div>");
    html.push_str(r#"<div class="collapse-content"><div class="text-sm opacity-75 whitespace-pre-wrap">"#);
    html.push_str(&html_escape::encode_text(thinking));
    html.push_str("</div></div></div>");
    html
}

// Helper function to render just reasoning content
fn render_reasoning_section(reasoning: &str) -> String {
    if reasoning.is_empty() {
        return String::new();
    }

    let mut html = String::new();
    html.push_str(r#"<div id="reasoning-container" class="collapse collapse-arrow bg-base-200 mb-4">"#);
    html.push_str(r#"<input type="checkbox" id="reasoning-collapse" />"#);
    html.push_str(r#"<div class="collapse-title text-sm font-medium flex items-center gap-2 cursor-pointer">"#);
    html.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" /></svg>"#);
    html.push_str("Reasoning");
    html.push_str("</div>");
    html.push_str(r#"<div class="collapse-content"><div class="text-sm opacity-75 whitespace-pre-wrap">"#);
    html.push_str(&html_escape::encode_text(reasoning));
    html.push_str("</div></div></div>");
    html
}

pub enum ChatError {
    DatabaseError(String),
    InvalidAPIKey,
    EmptyAPIKey,
    ChatNotFound,
    MissingUser,
    InvalidMessage,
    NetworkError(String),
    ServerError(String),
}

impl std::fmt::Display for ChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            ChatError::InvalidAPIKey => write!(f, "Invalid API key"),
            ChatError::EmptyAPIKey => write!(f, "API key is required"),
            ChatError::ChatNotFound => write!(f, "Chat not found"),
            ChatError::MissingUser => write!(f, "User not authenticated"),
            ChatError::InvalidMessage => write!(f, "Invalid message format"),
            ChatError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ChatError::ServerError(msg) => write!(f, "Server error: {}", msg),
        }
    }
}

impl IntoResponse for ChatError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ChatError::DatabaseError(msg) => {
                tracing::error!("Database error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal database error")
            }
            ChatError::InvalidAPIKey => {
                (StatusCode::UNAUTHORIZED, "Invalid API key. Please check your settings.")
            }
            ChatError::EmptyAPIKey => {
                (StatusCode::BAD_REQUEST, "API key is required. Please configure it in settings.")
            }
            ChatError::ChatNotFound => (StatusCode::NOT_FOUND, "Chat not found"),
            ChatError::MissingUser => (StatusCode::UNAUTHORIZED, "User not authenticated"),
            ChatError::InvalidMessage => (StatusCode::BAD_REQUEST, "Message cannot be empty"),
            ChatError::NetworkError(msg) => {
                tracing::error!("Network error: {}", msg);
                (StatusCode::BAD_GATEWAY, "Failed to connect to AI service")
            }
            ChatError::ServerError(msg) => {
                tracing::error!("Server error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        let body = Json(serde_json::json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}


#[axum::debug_handler]
pub async fn chat(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<Option<User>>,
) -> Html<String> {
    let user_chats = state
        .chat_repo
        .get_all_chats(current_user.as_ref().unwrap().id)
        .await
        .unwrap();

    let mut context = Context::new();
    context.insert("user_chats", &user_chats);
    let home = state.tera.render("views/chat.html", &context).unwrap();

    let mut context = Context::new();
    context.insert("view", &home);
    context.insert("current_user", &current_user);
    let rendered = state.tera.render("views/main.html", &context).unwrap();

    Html(rendered)
}

#[derive(Deserialize, Debug)]
pub struct NewChat {
    message: String,
}

#[axum::debug_handler]
pub async fn new_chat(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<Option<User>>,
    Form(new_chat): Form<NewChat>,
) -> Result<Response<String>, ChatError> {
    // Validate message
    if new_chat.message.trim().is_empty() {
        return Err(ChatError::InvalidMessage);
    }

    let current_user = current_user.ok_or_else(|| ChatError::MissingUser)?;

    // Use model from user settings, fallback to default if not set
    let model = current_user.model.as_deref().unwrap_or("Qwen/Qwen2.5-7B-Instruct");

    let chat_id = state
        .chat_repo
        .create_chat(current_user.id, &new_chat.message, model)
        .await
        .map_err(|e| ChatError::DatabaseError(format!("Failed to create chat: {}", e)))?;

    state
        .chat_repo
        .add_message_block(chat_id, &new_chat.message)
        .await
        .map_err(|e| ChatError::DatabaseError(format!("Failed to add message: {}", e)))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("HX-Redirect", format!("/chat/{}", chat_id).as_str())
        .body("".to_string())
        .map_err(|e| ChatError::ServerError(format!("Failed to build response: {}", e)))?)
}

#[derive(Serialize, Deserialize, Debug)]
struct ParsedMessagePair {
    pair: ChatMessagePair,
    human_message_html: String,
    ai_message_html: String,
}

#[axum::debug_handler]
pub async fn chat_by_id(
    Path(chat_id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<Option<User>>,
) -> Result<Html<String>, ChatError> {
    let chat_message_pairs = state
        .chat_repo
        .retrieve_chat(chat_id)
        .await
        .map_err(|e| ChatError::DatabaseError(format!("Failed to retrieve chat: {}", e)))?;

    let current_user = current_user.ok_or_else(|| ChatError::MissingUser)?;
    let user_chats = state
        .chat_repo
        .get_all_chats(current_user.id)
        .await
        .map_err(|e| ChatError::DatabaseError(format!("Failed to retrieve user chats: {}", e)))?;

    let parsed_pairs = chat_message_pairs
        .iter()
        .map(|pair| {
            let human_message_html = markdown_to_html(&pair.human_message);

            // Reconstruct extended message data if AI message exists
            let ai_message_html = if let Some(ai_message) = &pair.ai_message {
                let mut acc = MessageAccumulator {
                    text: ai_message.clone(),
                    thinking: pair.thinking.clone().unwrap_or_default(),
                    reasoning: pair.reasoning.clone().unwrap_or_default(),
                    tool_calls: Vec::new(),
                    images: Vec::new(),
                    usage: None,
                    sources: Vec::new(),
                };

                // Parse tool calls
                if let Some(tool_calls_json) = &pair.tool_calls {
                    if let Ok(parsed) = serde_json::from_str::<Vec<crate::data::model::ToolCall>>(tool_calls_json) {
                        acc.tool_calls = parsed;
                    }
                }

                // Parse images
                if let Some(images_json) = &pair.images {
                    if let Ok(parsed) = serde_json::from_str::<Vec<String>>(images_json) {
                        acc.images = parsed;
                    }
                }

                // Parse sources
                if let Some(sources_json) = &pair.sources {
                    if let Ok(parsed) = serde_json::from_str::<Vec<crate::data::model::Source>>(sources_json) {
                        acc.sources = parsed;
                    }
                }

                // Parse usage
                if pair.usage_prompt_tokens.is_some() || pair.usage_completion_tokens.is_some() || pair.usage_total_tokens.is_some() {
                    acc.usage = Some(crate::data::model::UsageInfo {
                        prompt_tokens: pair.usage_prompt_tokens.unwrap_or(0),
                        completion_tokens: pair.usage_completion_tokens.unwrap_or(0),
                        total_tokens: pair.usage_total_tokens.unwrap_or(0),
                    });
                }

                render_message_html(&acc)
            } else {
                String::new()
            };

            ParsedMessagePair {
                pair: pair.clone(),
                human_message_html,
                ai_message_html,
            }
        })
        .collect::<Vec<_>>();

    let mut context = Context::new();
    context.insert("name", "World");
    context.insert("chat_message_pairs", &parsed_pairs);
    context.insert("chat_id", &chat_id);
    context.insert("user_chats", &user_chats);

    let home = state.tera.render("views/chat.html", &context).unwrap();

    let mut context = Context::new();
    context.insert("view", &home);
    context.insert("current_user", &current_user);
    let rendered = state.tera.render("views/main.html", &context).unwrap();

    Ok(Html(rendered))
}

#[derive(Deserialize, Debug)]
pub struct ChatAddMessage {
    message: String,
}

use axum::extract::Multipart;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use std::path::PathBuf;

#[axum::debug_handler]
pub async fn chat_add_message(
    Path(chat_id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Extension(_current_user): Extension<Option<User>>,
    mut multipart: Multipart,
) -> Result<Html<String>, ChatError> {
    let mut message = String::new();
    let mut file_attachments = Vec::new();
    
    // Create uploads directory if it doesn't exist
    tokio::fs::create_dir_all("uploads").await
        .map_err(|e| ChatError::ServerError(format!("Failed to create uploads directory: {}", e)))?;
    
    // Process multipart form data
    while let Some(field) = multipart.next_field().await
        .map_err(|e| ChatError::ServerError(format!("Failed to read multipart field: {}", e)))? {
        let name = field.name().unwrap_or("").to_string();

        if name == "message" {
            message = field.text().await
                .map_err(|e| ChatError::ServerError(format!("Failed to read message text: {}", e)))?;
        } else if name == "files" {
            let filename = field.file_name().unwrap_or("unknown").to_string();
            let data = field.bytes().await
                .map_err(|e| ChatError::ServerError(format!("Failed to read file data: {}", e)))?;
            
            // Generate unique filename
            let timestamp = chrono::Utc::now().timestamp();
            let path_buf = PathBuf::from(&filename);
            let extension = path_buf
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("bin")
                .to_string();
            let unique_filename = format!("{}-{}.{}", timestamp, uuid::Uuid::new_v4(), extension);
            let file_path = format!("uploads/{}", unique_filename);
            
            // Save file
            let mut file = File::create(&file_path).await
                .map_err(|e| ChatError::ServerError(format!("Failed to create file: {}", e)))?;
            file.write_all(&data).await
                .map_err(|e| ChatError::ServerError(format!("Failed to write file: {}", e)))?;
            
            // Determine if it's an image
            let is_image = filename.ends_with(".jpg") || filename.ends_with(".jpeg") 
                || filename.ends_with(".png") || filename.ends_with(".gif") 
                || filename.ends_with(".webp");
            
            file_attachments.push((filename, file_path, is_image));
        }
    }
    
    // Add file references to message
    if !file_attachments.is_empty() {
        let attachments_text = file_attachments.iter()
            .map(|(name, path, is_image)| {
                if *is_image {
                    format!("\n\n![{}](/{})  ", name, path)
                } else {
                    format!("\n\n[📎 {}](/{})  ", name, path)
                }
            })
            .collect::<String>();
        message.push_str(&attachments_text);
    }
    
    // Validate message
    if message.trim().is_empty() {
        return Err(ChatError::InvalidMessage);
    }

    state
        .chat_repo
        .add_message_block(chat_id, &message)
        .await
        .map_err(|e| ChatError::DatabaseError(format!("Failed to add message: {}", e)))?;

    let human_message_html = markdown_to_html(&message);
    
    let mut context = Context::new();
    context.insert("human_message_html", &human_message_html);
    context.insert("chat_id", &chat_id);
    let update = state
        .tera
        .render("htmx_updates/add_message.html", &context)
        .unwrap();

    Ok(Html(update))
}

pub async fn chat_generate(
    Extension(current_user): Extension<Option<User>>,
    Path(chat_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> Result<Sse<impl tokio_stream::Stream<Item = Result<Event, axum::Error>>>, ChatError> {
    let user = current_user.ok_or_else(|| ChatError::MissingUser)?;

    // Check if user has API key configured
    let key = user.openai_api_key.ok_or_else(|| ChatError::EmptyAPIKey)?;

    if key.trim().is_empty() {
        return Err(ChatError::EmptyAPIKey);
    }

    // Retrieve chat messages
    let chat_message_pairs = state.chat_repo.retrieve_chat(chat_id).await
        .map_err(|e| ChatError::DatabaseError(format!("Failed to retrieve chat: {}", e)))?;

    if chat_message_pairs.is_empty() {
        return Err(ChatError::ChatNotFound);
    }

    // Use model from user settings, fallback to default if not set
    let model = user.model.clone().unwrap_or_else(|| "Qwen/Qwen2.5-7B-Instruct".to_string());

    // Validate API key
    match list_engines(&key).await {
        Ok(_res) => {}
        Err(e) => {
            tracing::error!("API key validation failed: {:?}", e);
            return Err(ChatError::InvalidAPIKey);
        }
    };

    let lat_message_id = chat_message_pairs.last().unwrap().id;

    // Create a channel for sending SSE events
    let (sender, receiver) = mpsc::channel::<Result<GenerationEvent, axum::Error>>(10);

    // Spawn a task that generates SSE events and sends them into the channel
    tokio::spawn(async move {
        // Call your existing function to start generating events
        if let Err(e) = generate_sse_stream(
            &key,
            &model,
            chat_message_pairs,
            sender,
        )
        .await
        {
            eprintln!("Error generating SSE stream: {:?}", e);
        }
    });

    // Convert the receiver into a Stream that can be used by Sse
    // let event_stream = ReceiverStream::new(receiver);
    let state_clone = Arc::clone(&state);

    let receiver_stream = ReceiverStream::new(receiver);
    
    let initial_accumulator = MessageAccumulator {
        text: String::new(),
        thinking: String::new(),
        reasoning: String::new(),
        tool_calls: Vec::new(),
        images: Vec::new(),
        usage: None,
        sources: Vec::new(),
    };
    
    let initial_state = (receiver_stream, initial_accumulator);
    let event_stream = stream::unfold(initial_state, move |(mut rc, mut acc)| {
        let state_clone = Arc::clone(&state_clone);
        async move {
            match rc.next().await {
                Some(Ok(event)) => {
                    match event {
                        GenerationEvent::Text(text) => {
                            acc.text.push_str(&text);
                            let html = render_message_html(&acc);
                            Some((Ok(Event::default().data(html)), (rc, acc)))
                        }
                        GenerationEvent::Thinking(thinking) => {
                            acc.thinking.push_str(&thinking);
                            // Send thinking update as JSON
                            let thinking_html = render_thinking_section(&acc.thinking);
                            let json_data = serde_json::json!({
                                "type": "thinking_update",
                                "html": thinking_html
                            });
                            Some((Ok(Event::default().data(json_data.to_string())), (rc, acc)))
                        }
                        GenerationEvent::Reasoning(reasoning) => {
                            acc.reasoning.push_str(&reasoning);
                            // Send reasoning update as JSON
                            let reasoning_html = render_reasoning_section(&acc.reasoning);
                            let json_data = serde_json::json!({
                                "type": "reasoning_update",
                                "html": reasoning_html
                            });
                            Some((Ok(Event::default().data(json_data.to_string())), (rc, acc)))
                        }
                        GenerationEvent::ThinkingUpdate(_) => {
                            // This shouldn't happen in the current implementation
                            // as we handle Thinking events directly
                            Some((Ok(Event::default().data("")), (rc, acc)))
                        }
                        GenerationEvent::ReasoningUpdate(_) => {
                            // This shouldn't happen in the current implementation
                            // as we handle Reasoning events directly
                            Some((Ok(Event::default().data("")), (rc, acc)))
                        }
                        GenerationEvent::ToolCall(tool_call) => {
                            acc.tool_calls.push(tool_call);
                            let html = render_message_html(&acc);
                            Some((Ok(Event::default().data(html)), (rc, acc)))
                        }
                        GenerationEvent::Image(image_url) => {
                            acc.images.push(image_url);
                            let html = render_message_html(&acc);
                            Some((Ok(Event::default().data(html)), (rc, acc)))
                        }
                        GenerationEvent::Usage(usage) => {
                            acc.usage = Some(usage);
                            let html = render_message_html(&acc);
                            Some((Ok(Event::default().data(html)), (rc, acc)))
                        }
                        GenerationEvent::Sources(sources) => {
                            acc.sources = sources;
                            let html = render_message_html(&acc);
                            Some((Ok(Event::default().data(html)), (rc, acc)))
                        }
                        GenerationEvent::End(_text) => {
                            // Save to database with extended data
                            let tool_calls_json = if !acc.tool_calls.is_empty() {
                                serde_json::to_string(&acc.tool_calls).ok()
                            } else {
                                None
                            };

                            let images_json = if !acc.images.is_empty() {
                                serde_json::to_string(&acc.images).ok()
                            } else {
                                None
                            };

                            let sources_json = if !acc.sources.is_empty() {
                                serde_json::to_string(&acc.sources).ok()
                            } else {
                                None
                            };

                            state_clone
                                .chat_repo
                                .add_ai_message_with_extended_data(
                                    lat_message_id,
                                    &acc.text,
                                    if !acc.thinking.is_empty() { Some(&acc.thinking) } else { None },
                                    tool_calls_json.as_deref(),
                                    images_json.as_deref(),
                                    if !acc.reasoning.is_empty() { Some(&acc.reasoning) } else { None },
                                    acc.usage.as_ref().map(|u| u.prompt_tokens),
                                    acc.usage.as_ref().map(|u| u.completion_tokens),
                                    acc.usage.as_ref().map(|u| u.total_tokens),
                                    sources_json.as_deref(),
                                )
                                .await
                                .unwrap();

                            let html = render_message_html(&acc);
                            let close_event = Event::default().data(html).event("close");
                            Some((Ok(close_event), (rc, acc)))
                        }
                    }
                }
                Some(Err(e)) => {
                    Some((Err(axum::Error::new(e)), (rc, acc)))
                }
                None => None,
            }
        }
    });

    Ok(Sse::new(event_stream))
}

pub async fn delete_chat(
    Path(chat_id): Path<i64>,
    State(state): State<Arc<AppState>>,
) -> Result<Html<String>, ChatError> {
    let rows_affected = state.chat_repo.delete_chat(chat_id).await
        .map_err(|e| ChatError::DatabaseError(format!("Failed to delete chat: {}", e)))?;

    if rows_affected == 0 {
        return Err(ChatError::ChatNotFound);
    }

    let html = r#"<div class="hidden"></div>"#;

    Ok(Html(html.to_string()))
}
