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
        html.push_str(r#"<input type="checkbox" />"#);
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
        html.push_str(r#"<input type="checkbox" />"#);
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

pub enum ChatError {
    Other,
    InvalidAPIKey,
}
// Implement Display for ChatError to provide user-facing error messages.

impl IntoResponse for ChatError {
    fn into_response(self) -> Response {
        match self {
            ChatError::Other => (StatusCode::BAD_REQUEST, Json("Chat Errror")).into_response(),
            ChatError::InvalidAPIKey => {
                (StatusCode::UNAUTHORIZED, Json("Chat Errror")).into_response()
            }
        }
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
    let current_user = current_user.unwrap();

    // Use model from user settings, fallback to default if not set
    let model = current_user.model.as_deref().unwrap_or("Qwen/Qwen2.5-7B-Instruct");

    let chat_id = state
        .chat_repo
        .create_chat(current_user.id, &new_chat.message, model)
        .await
        .map_err(|_| ChatError::Other)?;

    state
        .chat_repo
        .add_message_block(chat_id, &new_chat.message)
        .await
        .map_err(|_| ChatError::Other)?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("HX-Redirect", format!("/chat/{}", chat_id).as_str())
        .body("".to_string())
        .unwrap())
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
        .map_err(|_| ChatError::Other)?;

    let user_chats = state
        .chat_repo
        .get_all_chats(current_user.as_ref().unwrap().id)
        .await
        .unwrap();

    let parsed_pairs = chat_message_pairs
        .iter()
        .map(|pair| {
            let human_message_html = markdown_to_html(&pair.human_message);
            let ai_message_html =
                markdown_to_html(&pair.clone().ai_message.unwrap_or("".to_string()));
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

#[axum::debug_handler]
pub async fn chat_add_message(
    Path(chat_id): Path<i64>,
    State(state): State<Arc<AppState>>,
    Extension(_current_user): Extension<Option<User>>,
    Form(chat_add_message): Form<ChatAddMessage>,
) -> Result<Html<String>, ChatError> {
    let message = chat_add_message.message;
    state
        .chat_repo
        .add_message_block(chat_id, &message)
        .await
        .map_err(|_| ChatError::Other)?;

    let mut context = Context::new();
    context.insert("human_message", &message);
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
    let chat_message_pairs = state.chat_repo.retrieve_chat(chat_id).await.unwrap();
    let user = current_user.unwrap();
    let key = user.openai_api_key.unwrap_or(String::new());

    // Use model from user settings, fallback to default if not set
    let model = user.model.clone().unwrap_or_else(|| "Qwen/Qwen2.5-7B-Instruct".to_string());

    match list_engines(&key).await {
        Ok(_res) => {}
        Err(_) => {
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
                            let html = render_message_html(&acc);
                            Some((Ok(Event::default().data(html)), (rc, acc)))
                        }
                        GenerationEvent::Reasoning(reasoning) => {
                            acc.reasoning.push_str(&reasoning);
                            let html = render_message_html(&acc);
                            Some((Ok(Event::default().data(html)), (rc, acc)))
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
                            // Save to database - we'll store extended data as JSON in metadata
                            // For now, just save the main text content
                            state_clone
                                .chat_repo
                                .add_ai_message_to_pair(lat_message_id, &acc.text)
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
    state.chat_repo.delete_chat(chat_id).await.unwrap();

    let html = r#"<div class="hidden"></div>"#;

    Ok(Html(html.to_string()))
}
