use async_trait::async_trait;
use serde_json::Value;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Transport layer for ACP communication
#[async_trait]
pub trait AcpTransport: Send + Sync {
    /// Send a message
    async fn send(&self, message: Value) -> Result<(), TransportError>;

    /// Start the message loop with a handler callback
    async fn start_message_loop(&self, handler: Arc<dyn MessageHandlerTrait>) -> Result<(), TransportError>;
}

/// Type alias for message handler callback
pub type MessageHandler = Box<dyn Fn(Value) -> Pin<Box<dyn std::future::Future<Output = Result<Value, super::server::AcpServerError>> + Send>> + Send + Sync>;

/// Simplified message handler trait for easier cloning
#[async_trait]
pub trait MessageHandlerTrait: Send + Sync {
    async fn handle(&self, message: Value) -> Result<Value, super::server::AcpServerError>;
}

/// Transport error types
#[derive(Debug, Clone)]
pub enum TransportError {
    ConnectionError(String),
    SendError(String),
    ReceiveError(String),
    SerializationError(String),
    IoError(String),
}

impl std::fmt::Display for TransportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransportError::ConnectionError(msg) => write!(f, "Connection error: {}", msg),
            TransportError::SendError(msg) => write!(f, "Send error: {}", msg),
            TransportError::ReceiveError(msg) => write!(f, "Receive error: {}", msg),
            TransportError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            TransportError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for TransportError {}

/// Stdio transport for communicating with subprocess agents
pub struct StdioTransport {
    command: String,
    args: Vec<String>,
    #[allow(dead_code)]
    child: Arc<tokio::sync::Mutex<tokio::process::Child>>,
    stdin: Arc<tokio::sync::Mutex<tokio::process::ChildStdin>>,
    stdout: Arc<tokio::sync::Mutex<tokio::process::ChildStdout>>,
}

impl StdioTransport {
    /// Create a new stdio transport by spawning a subprocess
    pub async fn new(command: String, args: Vec<String>) -> Result<Self, TransportError> {
        let mut child = tokio::process::Command::new(&command)
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| TransportError::ConnectionError(format!("Failed to spawn {}: {}", command, e)))?;

        let stdin = child.stdin.take().ok_or_else(|| {
            TransportError::ConnectionError("Failed to open stdin".to_string())
        })?;

        let stdout = child.stdout.take().ok_or_else(|| {
            TransportError::ConnectionError("Failed to open stdout".to_string())
        })?;

        Ok(Self {
            command,
            args,
            child: Arc::new(tokio::sync::Mutex::new(child)),
            stdin: Arc::new(tokio::sync::Mutex::new(stdin)),
            stdout: Arc::new(tokio::sync::Mutex::new(stdout)),
        })
    }
}

#[async_trait]
impl AcpTransport for StdioTransport {
    async fn send(&self, message: Value) -> Result<(), TransportError> {
        let json_str = serde_json::to_string(&message)
            .map_err(|e| TransportError::SerializationError(e.to_string()))?;

        // We need to restructure this to avoid the borrow checker issue
        // For now, let's implement a simplified version
        // In a real implementation, you'd need to handle stdin properly
        use tokio::io::{AsyncWriteExt};

        let mut stdin = self.stdin.lock().await;
        let stdin: &mut tokio::process::ChildStdin = &mut *stdin;

        stdin
            .write_all(json_str.as_bytes())
            .await
            .map_err(|e: std::io::Error| TransportError::SendError(e.to_string()))?;

        stdin
            .write_all(b"\n")
            .await
            .map_err(|e: std::io::Error| TransportError::SendError(e.to_string()))?;

        stdin.flush().await.map_err(|e: std::io::Error| TransportError::SendError(e.to_string()))?;

        Ok(())
    }

    async fn start_message_loop(&self, handler: Arc<dyn MessageHandlerTrait>) -> Result<(), TransportError> {
        let stdout = self.stdout.clone();
        tokio::spawn(async move {
            let mut line_buffer = String::new();

            use tokio::io::{AsyncBufReadExt, BufReader};
            let mut stdout_guard = stdout.lock().await;
            let mut reader = BufReader::new(&mut *stdout_guard);

            loop {
                match reader.read_line(&mut line_buffer).await {
                    Ok(0) => {
                        // EOF reached
                        break;
                    }
                    Ok(_) => {
                        let line = line_buffer.trim();
                        if !line.is_empty() {
                            match serde_json::from_str::<Value>(line) {
                                Ok(message) => {
                                    let handler = handler.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = handler.handle(message).await {
                                            eprintln!("Error handling message: {:?}", e);
                                        }
                                    });
                                }
                                Err(e) => {
                                    eprintln!("Failed to parse JSON message: {}", e);
                                }
                            }
                        }
                        line_buffer.clear();
                    }
                    Err(e) => {
                        eprintln!("Error reading from stdout: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }
}

/// HTTP transport for communicating with HTTP-based agents
pub struct HttpTransport {
    client: reqwest::Client,
    base_url: String,
    message_receiver: Arc<tokio::sync::Mutex<mpsc::UnboundedReceiver<Value>>>,
}

impl HttpTransport {
    /// Create a new HTTP transport
    pub fn new(base_url: String) -> (Self, mpsc::UnboundedSender<Value>) {
        let (sender, receiver) = mpsc::unbounded_channel();

        let transport = Self {
            client: reqwest::Client::new(),
            base_url,
            message_receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
        };

        (transport, sender)
    }
}

#[async_trait]
impl AcpTransport for HttpTransport {
    async fn send(&self, message: Value) -> Result<(), TransportError> {
        let response = self.client
            .post(&self.base_url)
            .json(&message)
            .send()
            .await
            .map_err(|e| TransportError::SendError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(TransportError::SendError(
                format!("HTTP error: {}", response.status())
            ));
        }

        Ok(())
    }

    async fn start_message_loop(&self, handler: Arc<dyn MessageHandlerTrait>) -> Result<(), TransportError> {
        // For HTTP transport, we'd typically use WebSockets for bidirectional communication
        // This is a simplified implementation that receives from the channel
        let receiver = self.message_receiver.clone();

        tokio::spawn(async move {
            let mut receiver = receiver.lock().await;
            // Note: This is simplified - you'd need to redesign the channel for proper async use
        });

        Ok(())
    }
}

/// WebSocket transport for real-time bidirectional communication
pub struct WebSocketTransport {
    base_url: String,
}

impl WebSocketTransport {
    /// Create a new WebSocket transport
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }
}

#[async_trait]
impl AcpTransport for WebSocketTransport {
    async fn send(&self, message: Value) -> Result<(), TransportError> {
        // WebSocket implementation would go here
        // This is a placeholder for the actual WebSocket logic
        Err(TransportError::ConnectionError("WebSocket transport not implemented yet".to_string()))
    }

    async fn start_message_loop(&self, _handler: Arc<dyn MessageHandlerTrait>) -> Result<(), TransportError> {
        // WebSocket message loop implementation would go here
        Err(TransportError::ConnectionError("WebSocket transport not implemented yet".to_string()))
    }
}