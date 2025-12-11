use super::types::*;
use super::transport::AcpTransport;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// ACP client for communicating with ACP-compliant agents
#[derive(Clone)]
pub struct AcpClient {
    transport: Arc<dyn AcpTransport>,
    pending_requests: Arc<RwLock<HashMap<String, mpsc::Sender<Value>>>>,
    next_request_id: Arc<RwLock<u64>>,
}

impl AcpClient {
    /// Create a new ACP client with the given transport
    pub fn new<T: AcpTransport + 'static>(transport: T) -> Self {
        Self {
            transport: Arc::new(transport),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            next_request_id: Arc::new(RwLock::new(0)),
        }
    }

    /// Initialize the connection with the agent
    pub async fn initialize(&self, request: InitializeRequest) -> Result<InitializeResponse, AcpError> {
        let response = self
            .send_request("initialize", Some(serde_json::to_value(request)?))
            .await?;

        Ok(serde_json::from_value(response)?)
    }

    /// Authenticate with the agent if required
    pub async fn authenticate(&self, request: AuthenticateRequest) -> Result<AuthenticateResponse, AcpError> {
        let response = self
            .send_request("authenticate", Some(serde_json::to_value(request)?))
            .await?;

        Ok(serde_json::from_value(response)?)
    }

    /// Create a new session
    pub async fn new_session(&self, request: NewSessionRequest) -> Result<NewSessionResponse, AcpError> {
        let response = self
            .send_request("session/new", Some(serde_json::to_value(request)?))
            .await?;

        Ok(serde_json::from_value(response)?)
    }

    /// Load an existing session
    pub async fn load_session(&self, request: LoadSessionRequest) -> Result<LoadSessionResponse, AcpError> {
        let response = self
            .send_request("session/load", Some(serde_json::to_value(request)?))
            .await?;

        Ok(serde_json::from_value(response)?)
    }

    /// Send a prompt to the agent
    pub async fn prompt(&self, request: PromptRequest) -> Result<PromptResponse, AcpError> {
        let response = self
            .send_request("session/prompt", Some(serde_json::to_value(request)?))
            .await?;

        Ok(serde_json::from_value(response)?)
    }

    /// Cancel an ongoing operation
    pub async fn cancel(&self, request: CancelNotification) -> Result<(), AcpError> {
        self.send_notification("session/cancel", Some(serde_json::to_value(request)?))
            .await?;
        Ok(())
    }

    /// Set the session mode
    pub async fn set_mode(&self, request: SetSessionModeRequest) -> Result<SetSessionModeResponse, AcpError> {
        let response = self
            .send_request("session/set_mode", Some(serde_json::to_value(request)?))
            .await?;

        Ok(serde_json::from_value(response)?)
    }

    /// Read a text file
    pub async fn read_text_file(&self, request: ReadTextFileRequest) -> Result<ReadTextFileResponse, AcpError> {
        let response = self
            .send_request("fs/read_text_file", Some(serde_json::to_value(request)?))
            .await?;

        Ok(serde_json::from_value(response)?)
    }

    /// Write a text file
    pub async fn write_text_file(&self, request: WriteTextFileRequest) -> Result<WriteTextFileResponse, AcpError> {
        let response = self
            .send_request("fs/write_text_file", Some(serde_json::to_value(request)?))
            .await?;

        Ok(serde_json::from_value(response)?)
    }

    /// Request permission for a tool call
    pub async fn request_permission(&self, request: RequestPermissionRequest) -> Result<RequestPermissionResponse, AcpError> {
        let response = self
            .send_request("session/request_permission", Some(serde_json::to_value(request)?))
            .await?;

        Ok(serde_json::from_value(response)?)
    }

    /// Create a terminal
    pub async fn create_terminal(&self, request: CreateTerminalRequest) -> Result<CreateTerminalResponse, AcpError> {
        let response = self
            .send_request("terminal/create", Some(serde_json::to_value(request)?))
            .await?;

        Ok(serde_json::from_value(response)?)
    }

    /// Get terminal output
    pub async fn terminal_output(&self, request: TerminalOutputRequest) -> Result<TerminalOutputResponse, AcpError> {
        let response = self
            .send_request("terminal/output", Some(serde_json::to_value(request)?))
            .await?;

        Ok(serde_json::from_value(response)?)
    }

    /// Release a terminal
    pub async fn release_terminal(&self, request: ReleaseTerminalRequest) -> Result<ReleaseTerminalResponse, AcpError> {
        let response = self
            .send_request("terminal/release", Some(serde_json::to_value(request)?))
            .await?;

        Ok(serde_json::from_value(response)?)
    }

    /// Kill a terminal command
    pub async fn kill_terminal(&self, request: KillTerminalCommandRequest) -> Result<KillTerminalCommandResponse, AcpError> {
        let response = self
            .send_request("terminal/kill", Some(serde_json::to_value(request)?))
            .await?;

        Ok(serde_json::from_value(response)?)
    }

    /// Wait for terminal exit
    pub async fn wait_for_terminal_exit(&self, request: WaitForTerminalExitRequest) -> Result<WaitForTerminalExitResponse, AcpError> {
        let response = self
            .send_request("terminal/wait_for_exit", Some(serde_json::to_value(request)?))
            .await?;

        Ok(serde_json::from_value(response)?)
    }

    /// Send a generic request and wait for response
    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<Value, AcpError> {
        let request_id = self.generate_request_id();

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(RequestId::String(request_id.clone())),
            method: method.to_string(),
            params,
        };

        // Create a response channel
        let (tx, mut rx) = mpsc::channel(1);

        // Register the pending request
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request_id.clone(), tx);
        }

        // Send the request
        self.transport.send(serde_json::to_value(&request)?).await?;

        // Wait for response
        let response_value = rx.recv().await.ok_or(AcpError::RequestTimeout)?;

        // Parse the response
        let response: JsonRpcResponse = serde_json::from_value(response_value)?;

        match response.result {
            Some(result) => Ok(result),
            None => match response.error {
                Some(error) => Err(AcpError::RpcError(error.code, error.message)),
                None => Err(AcpError::InvalidResponse("No result or error in response".to_string())),
            },
        }
    }

    /// Send a notification (no response expected)
    async fn send_notification(&self, method: &str, params: Option<Value>) -> Result<(), AcpError> {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None, // Notifications don't have IDs
            method: method.to_string(),
            params,
        };

        self.transport.send(serde_json::to_value(&request)?).await?;
        Ok(())
    }

    /// Generate a unique request ID
    fn generate_request_id(&self) -> String {
        // Use a simple counter combined with UUID for uniqueness
        let mut counter = self.next_request_id.blocking_write();
        *counter += 1;
        format!("{}-{}", *counter, Uuid::new_v4().to_string())
    }

    /// Handle incoming messages from the transport
    pub async fn handle_message(&self, message: Value) -> Result<(), AcpError> {
        let response: JsonRpcResponse = serde_json::from_value(message)?;

        if let Some(ref id) = response.id {
            let id_str = match id {
                RequestId::String(s) => s.clone(),
                RequestId::Number(n) => n.to_string(),
                RequestId::Null => "null".to_string(),
            };

            // Find the pending request and send the response
            let mut pending = self.pending_requests.write().await;
            if let Some(tx) = pending.remove(&id_str) {
                let _ = tx.send(serde_json::to_value(response)?).await;
            }
        } else {
            // This is a notification, handle it appropriately
            // For now, we just ignore notifications in the client
        }

        Ok(())
    }
}

/// Set session mode request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetSessionModeRequest {
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    #[serde(rename = "modeId")]
    pub mode_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Set session mode response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetSessionModeResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// ACP error types
#[derive(Debug, Clone)]
pub enum AcpError {
    SerializationError(String),
    TransportError(String),
    RpcError(i32, String),
    RequestTimeout,
    InvalidResponse(String),
    InternalError(String),
}

impl From<super::transport::TransportError> for AcpError {
    fn from(err: super::transport::TransportError) -> Self {
        AcpError::TransportError(err.to_string())
    }
}

impl std::fmt::Display for AcpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AcpError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            AcpError::TransportError(msg) => write!(f, "Transport error: {}", msg),
            AcpError::RpcError(code, msg) => write!(f, "RPC error ({}): {}", code, msg),
            AcpError::RequestTimeout => write!(f, "Request timeout"),
            AcpError::InvalidResponse(msg) => write!(f, "Invalid response: {}", msg),
            AcpError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AcpError {}

impl From<serde_json::Error> for AcpError {
    fn from(err: serde_json::Error) -> Self {
        AcpError::SerializationError(err.to_string())
    }
}