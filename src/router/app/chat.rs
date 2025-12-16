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
                            // Return the accumulated data as part of the SSE event
                            let html = markdown_to_html(&accumulated);

                            Some((Ok(Event::default().data(html)), (rc, accumulated)))
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
                        } // ... handle other event types if necessary ...
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
