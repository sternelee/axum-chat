use std::sync::Arc;

use axum::{
    extract::{Extension, Form, Path, State},
    http::{StatusCode, Response},
    response::{Html, IntoResponse},
};
use axum_extra::extract::cookie::{Key, PrivateCookieJar};
use html_escape;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePool;
use tera::{Context, Tera};

use crate::{
    ai::{stream::generate_sse_stream, GenerationEvent},
    data::{model::ChatMessagePair, repository::ChatRepository},
    middleware::User,
};

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

                render_complete_message_html(&acc)
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
    context.insert("user_chats", &user_chats);

    let html = state
        .tera
        .render("views/chat.html", &context)
        .unwrap();

    Ok(Html(html))
}

fn render_complete_message_html(acc: &MessageAccumulator) -> String {
    let mut html = String::new();

    // Render thinking section
    if !acc.thinking.is_empty() {
        html.push_str(&render_thinking_section(&acc.thinking));
    }

    // Render reasoning section
    if !acc.reasoning.is_empty() {
        html.push_str(&render_reasoning_section(&acc.reasoning));
    }

    // Main content
    if !acc.text.is_empty() {
        html.push_str(&markdown_to_html(&acc.text));
    }

    // Tool calls, images, sources, etc.
    for tool_call in &acc.tool_calls {
        html.push_str(r#"<div class="card bg-accent/10 mb-4 border border-accent/20">"#);
        html.push_str(r#"<div class="card-body p-4">"#);
        html.push_str(r#"<div class="flex items-center gap-2 mb-2">"#);
        html.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5 text-accent" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" /><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" /></svg>"#);
        html.push_str(r#"<span class="font-semibold text-accent">Tool Call: </span>"#);
        html.push_str(&html_escape::encode_text(&tool_call.function.name));
        html.push_str("</div>");
        html.push_str(r#"<div class="mockup-code text-xs"><pre><code>"#);
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

fn render_thinking_section(thinking: &str) -> String {
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

fn render_reasoning_section(reasoning: &str) -> String {
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

#[derive(Serialize, Deserialize, Debug)]
struct ParsedMessagePair {
    pair: ChatMessagePair,
    human_message_html: String,
    ai_message_html: String,
}

#[derive(Debug)]
struct MessageAccumulator {
    text: String,
    thinking: String,
    reasoning: String,
    tool_calls: Vec<crate::data::model::ToolCall>,
    images: Vec<String>,
    usage: Option<crate::data::model::UsageInfo>,
    sources: Vec<crate::data::model::Source>,
}

fn markdown_to_html(text: &str) -> String {
    let options = markdown_it::plugins::cmark::CMARK_OPTIONS;
    let parser = markdown_it::MarkdownIt::new().push(&options);
    parser.parse(text).render()
}

pub struct AppState {
    pub tera: Tera,
    pub chat_repo: ChatRepository,
    pub cookie_key: Key,
}

#[derive(Debug, Clone)]
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

impl IntoResponse for ChatError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            ChatError::DatabaseError(msg) => {
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
                (StatusCode::BAD_GATEWAY, "Failed to connect to AI service")
            }
            ChatError::ServerError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        let body = Json(serde_json::json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}