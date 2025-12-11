use super::types::*;
use super::transport::AcpTransport;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// ACP server trait for handling agent requests
#[async_trait]
pub trait AcpAgent: Send + Sync {
    /// Initialize the agent
    async fn initialize(&self, request: InitializeRequest) -> Result<InitializeResponse, AcpServerError>;

    /// Authenticate with the agent
    async fn authenticate(&self, request: AuthenticateRequest) -> Result<AuthenticateResponse, AcpServerError>;

    /// Create a new session
    async fn new_session(&self, request: NewSessionRequest) -> Result<NewSessionResponse, AcpServerError>;

    /// Load an existing session
    async fn load_session(&self, request: LoadSessionRequest) -> Result<LoadSessionResponse, AcpServerError>;

    /// Send a prompt to the agent
    async fn prompt(&self, request: PromptRequest) -> Result<PromptResponse, AcpServerError>;

    /// Cancel an ongoing operation
    async fn cancel(&self, request: CancelNotification) -> Result<(), AcpServerError>;

    /// Set the session mode
    async fn set_mode(&self, request: SetSessionModeRequest) -> Result<SetSessionModeResponse, AcpServerError>;

    /// Handle session update notification
    async fn session_update(&self, notification: SessionNotification) -> Result<(), AcpServerError>;

    /// Read a text file
    async fn read_text_file(&self, request: ReadTextFileRequest) -> Result<ReadTextFileResponse, AcpServerError>;

    /// Write a text file
    async fn write_text_file(&self, request: WriteTextFileRequest) -> Result<WriteTextFileResponse, AcpServerError>;

    /// Request permission for a tool call
    async fn request_permission(&self, request: RequestPermissionRequest) -> Result<RequestPermissionResponse, AcpServerError>;

    /// Create a terminal
    async fn create_terminal(&self, request: CreateTerminalRequest) -> Result<CreateTerminalResponse, AcpServerError>;

    /// Get terminal output
    async fn terminal_output(&self, request: TerminalOutputRequest) -> Result<TerminalOutputResponse, AcpServerError>;

    /// Release a terminal
    async fn release_terminal(&self, request: ReleaseTerminalRequest) -> Result<ReleaseTerminalResponse, AcpServerError>;

    /// Kill a terminal command
    async fn kill_terminal(&self, request: KillTerminalCommandRequest) -> Result<KillTerminalCommandResponse, AcpServerError>;

    /// Wait for terminal exit
    async fn wait_for_terminal_exit(&self, request: WaitForTerminalExitRequest) -> Result<WaitForTerminalExitResponse, AcpServerError>;
}

/// ACP server that handles client requests and routes them to the agent
pub struct AcpServer {
    agent: Arc<dyn AcpAgent>,
    transport: Arc<dyn AcpTransport>,
}

impl AcpServer {
    /// Create a new ACP server with the given agent and transport
    pub fn new<T: AcpTransport + 'static>(agent: Arc<dyn AcpAgent>, transport: T) -> Self {
        Self {
            agent,
            transport: Arc::new(transport),
        }
    }

    /// Start the server and begin processing requests
    pub async fn start(&self) -> Result<(), AcpServerError> {
        // Set up message handler
        let agent = self.agent.clone();

        struct Handler {
            agent: Arc<dyn AcpAgent>,
        }

        #[async_trait]
        impl super::transport::MessageHandlerTrait for Handler {
            async fn handle(&self, message: Value) -> Result<Value, AcpServerError> {
                AcpServer::handle_message(self.agent.clone(), message).await
            }
        }

        let handler = Arc::new(Handler { agent });

        // Start listening for messages
        self.transport.start_message_loop(handler).await?;

        Ok(())
    }

    /// Send a notification to the client
    pub async fn send_notification(&self, notification: SessionNotification) -> Result<(), AcpServerError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "session/update".to_string(),
            params: Some(serde_json::to_value(notification)?),
        };

        self.transport.send(serde_json::to_value(request)?).await?;
        Ok(())
    }

    /// Handle incoming messages and route them to the appropriate agent method
    async fn handle_message(agent: Arc<dyn AcpAgent>, message: Value) -> Result<Value, AcpServerError> {
        let request: JsonRpcRequest = serde_json::from_value(message)?;

        let result = match request.method.as_str() {
            "initialize" => {
                let init_req: InitializeRequest = serde_json::from_value(request.params.unwrap_or_default())?;
                let response = agent.initialize(init_req).await?;
                serde_json::to_value(response)?
            }
            "authenticate" => {
                let auth_req: AuthenticateRequest = serde_json::from_value(request.params.unwrap_or_default())?;
                let response = agent.authenticate(auth_req).await?;
                serde_json::to_value(response)?
            }
            "session/new" => {
                let new_session_req: NewSessionRequest = serde_json::from_value(request.params.unwrap_or_default())?;
                let response = agent.new_session(new_session_req).await?;
                serde_json::to_value(response)?
            }
            "session/load" => {
                let load_session_req: LoadSessionRequest = serde_json::from_value(request.params.unwrap_or_default())?;
                let response = agent.load_session(load_session_req).await?;
                serde_json::to_value(response)?
            }
            "session/prompt" => {
                let prompt_req: PromptRequest = serde_json::from_value(request.params.unwrap_or_default())?;
                let response = agent.prompt(prompt_req).await?;
                serde_json::to_value(response)?
            }
            "session/cancel" => {
                let cancel_req: CancelNotification = serde_json::from_value(request.params.unwrap_or_default())?;
                agent.cancel(cancel_req).await?;
                Value::Null
            }
            "session/set_mode" => {
                let set_mode_req: SetSessionModeRequest = serde_json::from_value(request.params.unwrap_or_default())?;
                let response = agent.set_mode(set_mode_req).await?;
                serde_json::to_value(response)?
            }
            "fs/read_text_file" => {
                let read_req: ReadTextFileRequest = serde_json::from_value(request.params.unwrap_or_default())?;
                let response = agent.read_text_file(read_req).await?;
                serde_json::to_value(response)?
            }
            "fs/write_text_file" => {
                let write_req: WriteTextFileRequest = serde_json::from_value(request.params.unwrap_or_default())?;
                let response = agent.write_text_file(write_req).await?;
                serde_json::to_value(response)?
            }
            "session/request_permission" => {
                let perm_req: RequestPermissionRequest = serde_json::from_value(request.params.unwrap_or_default())?;
                let response = agent.request_permission(perm_req).await?;
                serde_json::to_value(response)?
            }
            "terminal/create" => {
                let term_req: CreateTerminalRequest = serde_json::from_value(request.params.unwrap_or_default())?;
                let response = agent.create_terminal(term_req).await?;
                serde_json::to_value(response)?
            }
            "terminal/output" => {
                let output_req: TerminalOutputRequest = serde_json::from_value(request.params.unwrap_or_default())?;
                let response = agent.terminal_output(output_req).await?;
                serde_json::to_value(response)?
            }
            "terminal/release" => {
                let release_req: ReleaseTerminalRequest = serde_json::from_value(request.params.unwrap_or_default())?;
                let response = agent.release_terminal(release_req).await?;
                serde_json::to_value(response)?
            }
            "terminal/kill" => {
                let kill_req: KillTerminalCommandRequest = serde_json::from_value(request.params.unwrap_or_default())?;
                let response = agent.kill_terminal(kill_req).await?;
                serde_json::to_value(response)?
            }
            "terminal/wait_for_exit" => {
                let wait_req: WaitForTerminalExitRequest = serde_json::from_value(request.params.unwrap_or_default())?;
                let response = agent.wait_for_terminal_exit(wait_req).await?;
                serde_json::to_value(response)?
            }
            "session/update" => {
                // This is a notification, not a request
                let notification: SessionNotification = serde_json::from_value(request.params.unwrap_or_default())?;
                agent.session_update(notification).await?;
                return Ok(Value::Null); // No response for notifications
            }
            _ => {
                return Err(AcpServerError::MethodNotFound(request.method));
            }
        };

        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(result),
            error: None,
        };

        Ok(serde_json::to_value(response)?)
    }
}

/// Set session mode request (re-exported from client module for server use)
pub use super::client::SetSessionModeRequest;

/// Set session mode response (re-exported from client module for server use)
pub use super::client::SetSessionModeResponse;

/// ACP server error types
#[derive(Debug, Clone)]
pub enum AcpServerError {
    SerializationError(String),
    MethodNotFound(String),
    InvalidParams(String),
    InternalError(String),
    TransportError(String),
}

impl std::fmt::Display for AcpServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AcpServerError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            AcpServerError::MethodNotFound(method) => write!(f, "Method not found: {}", method),
            AcpServerError::InvalidParams(msg) => write!(f, "Invalid parameters: {}", msg),
            AcpServerError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            AcpServerError::TransportError(msg) => write!(f, "Transport error: {}", msg),
        }
    }
}

impl std::error::Error for AcpServerError {}

impl From<serde_json::Error> for AcpServerError {
    fn from(err: serde_json::Error) -> Self {
        AcpServerError::SerializationError(err.to_string())
    }
}

/// Convert server error to JSON-RPC error code
impl From<AcpServerError> for JsonRpcError {
    fn from(err: AcpServerError) -> Self {
        let (code, message) = match err {
            AcpServerError::SerializationError(msg) => (-32700, msg),
            AcpServerError::MethodNotFound(method) => (-32601, format!("Method not found: {}", method)),
            AcpServerError::InvalidParams(msg) => (-32602, msg),
            AcpServerError::InternalError(msg) => (-32603, msg),
            AcpServerError::TransportError(msg) => (-32000, msg),
        };

        Self {
            code,
            message,
            data: None,
        }
    }
}

impl From<super::transport::TransportError> for AcpServerError {
    fn from(err: super::transport::TransportError) -> Self {
        AcpServerError::TransportError(err.to_string())
    }
}