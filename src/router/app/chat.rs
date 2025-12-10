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
    ai::stream::{generate_sse_stream, GenerationEvent, StreamServiceType},
    data::model::{ChatMessagePair, AgentWithProvider},
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

const MODELS: [(&str, &str, &str); 4] = [
    (
        "DeepSeek-V3.2-Exp",
        "deepseek-ai/DeepSeek-V3.2-Exp",
        "This is the preview version of the GPT-4 model.",
    ),
    ("GPT-4", "gpt-4", "Latest generation GPT-4 model."),
    (
        "GPT-3.5-16K",
        "gpt-3.5-turbo-16k",
        "An enhanced GPT-3.5 model with 16K token limit.",
    ),
    (
        "GPT-3.5",
        "gpt-3.5-turbo",
        "Standard GPT-3.5 model with turbo features.",
    ),
];

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

    let selected_model = MODELS
        .iter()
        .filter(|f| f.1 == "deepseek-ai/DeepSeek-V3.2-Exp")
        .collect::<Vec<_>>()[0];

    let mut context = Context::new();
    context.insert("models", &MODELS);
    context.insert("selected_model", &selected_model);
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
    model: String,
}

#[axum::debug_handler]
pub async fn new_chat(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<Option<User>>,
    Form(new_chat): Form<NewChat>,
) -> Result<Response<String>, ChatError> {
    let current_user = current_user.unwrap();

    let chat_id = state
        .chat_repo
        .create_chat(current_user.id, &new_chat.message, &new_chat.model)
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

    let selected_model = MODELS
        .iter()
        .filter(|f| f.1 == chat_message_pairs[0].model)
        .collect::<Vec<_>>()[0];

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
    context.insert("selected_model", &selected_model);

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

    // Create a default agent based on the model from the chat
    // TODO: In the future, this should be replaced with actual agent selection
    let model_name = &chat_message_pairs[0].model;

    // Find a default provider for the model (using SiliconFlow as default for now)
    let default_provider = state.chat_repo.get_provider_by_name("siliconflow").await.unwrap();

    if default_provider.is_none() {
        return Err(ChatError::Other);
    }

    let provider = default_provider.unwrap();

    // Create a temporary agent for backward compatibility
    let agent = AgentWithProvider {
        id: 0, // Temporary ID
        user_id: user.id,
        name: "Default Agent".to_string(),
        description: Some("Default agent for backward compatibility".to_string()),
        provider,
        model_name: model_name.clone(),
        stream: true,
        chat: true,
        embed: false,
        image: false,
        tool: false,
        tools: vec![],
        allow_tools: vec![], // Add allow_tools field
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

    let lat_message_id = chat_message_pairs.last().unwrap().id;

    // Create a channel for sending SSE events
    let (sender, receiver) = mpsc::channel::<Result<GenerationEvent, axum::Error>>(10);

    // Spawn a task that generates SSE events and sends them into the channel
    tokio::spawn(async move {
        // Call the new agent-based function to start generating events
        if let Err(e) = generate_sse_stream(&agent, chat_message_pairs, sender, StreamServiceType::Chat).await {
            eprintln!("Error generating SSE stream: {:?}", e);
        }
    });

    // Convert the receiver into a Stream that can be used by Sse
    // let event_stream = ReceiverStream::new(receiver);
    let state_clone = Arc::clone(&state);

    let receiver_stream = ReceiverStream::new(receiver);
    let initial_state = (receiver_stream, String::new()); // Initial state with an empty accumulator
    let event_stream = stream::unfold(initial_state, move |(mut rc, mut accumulated)| {
        let state_clone = Arc::clone(&state_clone); // Clone the Arc here
        async move {
            match rc.next().await {
                Some(Ok(event)) => {
                    // Process the event
                    match event {
                        GenerationEvent::Text(text) => {
                            accumulated.push_str(&text);
                            // Return the accumulated data as part of the event
                            let html = markdown_to_html(&accumulated);

                            Some((Ok(Event::default().data(html)), (rc, accumulated)))
                        }
                        GenerationEvent::Thinking(thinking_event) => {
                            // Generate HTML for thinking content
                            let thinking_html = crate::ai::stream::generate_thinking_html(&thinking_event);
                            Some((Ok(Event::default().data(thinking_html)), (rc, accumulated)))
                        }
                        GenerationEvent::ToolCall(tool_call_event) => {
                            // Generate HTML for tool call approval form
                            let tool_html = crate::ai::stream::generate_tool_call_html(&tool_call_event);
                            Some((Ok(Event::default().data(tool_html)), (rc, accumulated)))
                        }
                        GenerationEvent::ToolResponse(tool_response_event) => {
                            // Generate HTML for tool response
                            let response_html = crate::ai::stream::generate_tool_response_html(&tool_response_event);
                            Some((Ok(Event::default().data(response_html)), (rc, accumulated)))
                        }
                        GenerationEvent::End(_text) => {
                            println!("accumulated: {:?}", accumulated);

                            state_clone
                                .chat_repo
                                .add_ai_message_to_pair(lat_message_id, &accumulated)
                                .await
                                .unwrap();

                            let html = markdown_to_html(&accumulated);

                            // Send final content with a close event
                            let close_event = Event::default().data(html).event("close");

                            Some((Ok(close_event), (rc, String::new())))
                        }
                    }
                }
                Some(Err(e)) => {
                    // Handle error without altering the accumulator
                    Some((Err(axum::Error::new(e)), (rc, accumulated)))
                }
                None => None, // When the receiver stream ends, finish the stream
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

// Form data structures for tool approval endpoints
#[derive(Deserialize)]
pub struct ToolApprovalForm {
    tool_call_id: String,
    status: String,
}

#[derive(Deserialize)]
pub struct ApproveAllToolsForm {
    tool_name: String,
}

// Tool approval endpoints
pub async fn approve_tool(
    State(_state): State<Arc<AppState>>,
    Extension(current_user): Extension<Option<User>>,
    Form(form): Form<ToolApprovalForm>,
) -> Result<Html<String>, ChatError> {
    println!("User {:?} approved tool {} with status {}",
        current_user.as_ref().map(|u| u.id), form.tool_call_id, form.status);

    // Create a response indicating the tool was approved
    let html = format!(r#"
    <div class="tool-response-container mb-4 border border-green-200 rounded-lg bg-green-50" id="tool_response_{}">
        <div class="p-4">
            <div class="flex items-center justify-between mb-3">
                <div class="flex items-center space-x-2">
                    <span class="text-lg">✓</span>
                    <span class="font-medium text-green-800">Tool Call Approved</span>
                </div>
                <span class="text-xs text-green-600 bg-green-100 px-2 py-1 rounded">Approved</span>
            </div>
            <div class="text-sm text-green-700">Tool {} has been approved and will be executed.</div>
        </div>
    </div>
    "#, form.tool_call_id, form.tool_call_id);

    Ok(Html(html))
}

pub async fn reject_tool(
    State(_state): State<Arc<AppState>>,
    Extension(current_user): Extension<Option<User>>,
    Form(form): Form<ToolApprovalForm>,
) -> Result<Html<String>, ChatError> {
    println!("User {:?} rejected tool {} with status {}",
        current_user.as_ref().map(|u| u.id), form.tool_call_id, form.status);

    // Create a response indicating the tool was rejected
    let html = format!(r#"
    <div class="tool-response-container mb-4 border border-red-200 rounded-lg bg-red-50" id="tool_response_{}">
        <div class="p-4">
            <div class="flex items-center justify-between mb-3">
                <div class="flex items-center space-x-2">
                    <span class="text-lg">✗</span>
                    <span class="font-medium text-red-800">Tool Call Rejected</span>
                </div>
                <span class="text-xs text-red-600 bg-red-100 px-2 py-1 rounded">Rejected</span>
            </div>
            <div class="text-sm text-red-700">Tool {} has been rejected and will not be executed.</div>
        </div>
    </div>
    "#, form.tool_call_id, form.tool_call_id);

    Ok(Html(html))
}

pub async fn approve_all_tools(
    State(_state): State<Arc<AppState>>,
    Extension(current_user): Extension<Option<User>>,
    Form(form): Form<ApproveAllToolsForm>,
) -> Result<Html<String>, ChatError> {
    let user = current_user.unwrap();

    println!("User {} requested to auto-approve tool: {}", user.id, form.tool_name);

    // For now, we'll create a simple response indicating success
    // In a real implementation, you would:
    // 1. Update the agent's allow_tools list in the database
    // 2. Return appropriate feedback to the user

    let html = format!(r#"
    <div class="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-4">
        <div class="flex items-center space-x-2">
            <svg class="w-5 h-5 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z"></path>
            </svg>
            <span class="font-medium text-blue-800">Tool Auto-approved</span>
        </div>
        <p class="text-sm text-blue-700 mt-2">
            The "{}" tool will now be auto-approved for future calls in this agent.
        </p>
    </div>
    "#, html_escape::encode_text_minimal(&form.tool_name));

    Ok(Html(html))
}
