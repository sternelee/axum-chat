use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// JSON-RPC 2.0 base request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 base response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// JSON-RPC 2.0 request ID
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
    Null,
}

/// ACP Protocol Version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolVersion(pub u16);

/// Implementation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Agent capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapabilities {
    #[serde(default)]
    pub load_session: bool,
    #[serde(default)]
    pub mcp_capabilities: McpCapabilities,
    #[serde(default)]
    pub prompt_capabilities: PromptCapabilities,
    #[serde(default)]
    pub session_capabilities: SessionCapabilities,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

impl Default for AgentCapabilities {
    fn default() -> Self {
        Self {
            load_session: false,
            mcp_capabilities: McpCapabilities::default(),
            prompt_capabilities: PromptCapabilities::default(),
            session_capabilities: SessionCapabilities::default(),
            _meta: None,
        }
    }
}

/// MCP capabilities supported by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCapabilities {
    #[serde(default)]
    pub http: bool,
    #[serde(default)]
    pub sse: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

impl Default for McpCapabilities {
    fn default() -> Self {
        Self {
            http: false,
            sse: false,
            _meta: None,
        }
    }
}

/// Prompt capabilities supported by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptCapabilities {
    #[serde(default)]
    pub audio: bool,
    #[serde(default)]
    pub embedded_context: bool,
    #[serde(default)]
    pub image: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

impl Default for PromptCapabilities {
    fn default() -> Self {
        Self {
            audio: false,
            embedded_context: false,
            image: false,
            _meta: None,
        }
    }
}

/// Session capabilities supported by the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCapabilities {
    #[serde(flatten)]
    pub capabilities: HashMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

impl Default for SessionCapabilities {
    fn default() -> Self {
        Self {
            capabilities: HashMap::new(),
            _meta: None,
        }
    }
}

/// Client capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    #[serde(default)]
    pub fs: FileSystemCapability,
    #[serde(default)]
    pub terminal: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

impl Default for ClientCapabilities {
    fn default() -> Self {
        Self {
            fs: FileSystemCapability::default(),
            terminal: false,
            _meta: None,
        }
    }
}

/// File system capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemCapability {
    #[serde(default, rename = "readTextFile")]
    pub read_text_file: bool,
    #[serde(default, rename = "writeTextFile")]
    pub write_text_file: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

impl Default for FileSystemCapability {
    fn default() -> Self {
        Self {
            read_text_file: false,
            write_text_file: false,
            _meta: None,
        }
    }
}

/// Authentication method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthMethod {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Initialize request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: ProtocolVersion,
    #[serde(rename = "clientInfo")]
    pub client_info: Option<Implementation>,
    #[serde(rename = "clientCapabilities")]
    #[serde(default)]
    pub client_capabilities: ClientCapabilities,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Initialize response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: ProtocolVersion,
    #[serde(rename = "agentInfo")]
    pub agent_info: Option<Implementation>,
    #[serde(rename = "agentCapabilities")]
    #[serde(default)]
    pub agent_capabilities: AgentCapabilities,
    #[serde(default)]
    pub auth_methods: Vec<AuthMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Authenticate request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticateRequest {
    #[serde(rename = "methodId")]
    pub method_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Authenticate response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticateResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Session ID
pub type SessionId = String;

/// New session request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSessionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(rename = "mcpServers", default)]
    pub mcp_servers: Vec<McpServer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// New session response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSessionResponse {
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modes: Option<SessionModeState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Load session request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadSessionRequest {
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    pub cwd: String,
    #[serde(rename = "mcpServers")]
    pub mcp_servers: Vec<McpServer>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Load session response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadSessionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modes: Option<SessionModeState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// MCP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpServer {
    Stdio(McpServerStdio),
    Http(McpServerHttp),
    Sse(McpServerSse),
}

/// Stdio transport configuration for MCP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerStdio {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Vec<EnvVariable>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// HTTP transport configuration for MCP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerHttp {
    pub name: String,
    pub url: String,
    pub headers: Vec<HttpHeader>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// SSE transport configuration for MCP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerSse {
    pub name: String,
    pub url: String,
    pub headers: Vec<HttpHeader>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Environment variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVariable {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// HTTP header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Session mode state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionModeState {
    #[serde(rename = "currentModeId")]
    pub current_mode_id: String,
    #[serde(rename = "availableModes")]
    pub available_modes: Vec<SessionMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Session mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMode {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Content block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text(TextContent),
    Image(ImageContent),
    Audio(AudioContent),
    ResourceLink(ResourceLink),
    Resource(EmbeddedResource),
}

/// Text content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Image content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    pub data: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Audio content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioContent {
    pub data: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Resource link
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLink {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Embedded resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedResource {
    pub resource: EmbeddedResourceResource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Embedded resource resource
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "uri", content = "contents", rename_all = "snake_case")]
pub enum EmbeddedResourceResource {
    Text(TextResourceContents),
    Blob(BlobResourceContents),
}

/// Text resource contents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextResourceContents {
    pub uri: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Blob resource contents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobResourceContents {
    pub uri: String,
    pub blob: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Annotations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Prompt request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptRequest {
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    pub prompt: Vec<ContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Prompt response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptResponse {
    #[serde(rename = "stopReason")]
    pub stop_reason: StopReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Stop reason
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    EndTurn,
    MaxTokens,
    MaxTurnRequests,
    Refusal,
    Cancelled,
}

/// Session update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionNotification {
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    pub update: SessionUpdate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Session update types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "sessionUpdate", content = "data", rename_all = "snake_case")]
pub enum SessionUpdate {
    UserMessageChunk(ContentChunk),
    AgentMessageChunk(ContentChunk),
    AgentThoughtChunk(ContentChunk),
    ToolCall(ToolCall),
    ToolCallUpdate(ToolCallUpdate),
    Plan(Plan),
    AvailableCommandsUpdate(AvailableCommandsUpdate),
    CurrentModeUpdate(CurrentModeUpdate),
}

/// Content chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentChunk {
    pub content: ContentBlock,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    #[serde(rename = "toolCallId")]
    pub tool_call_id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ToolKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ToolCallStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<ToolCallContent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<ToolCallLocation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_input: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_output: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Tool call update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallUpdate {
    #[serde(rename = "toolCallId")]
    pub tool_call_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<ToolKind>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ToolCallStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<ToolCallContent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<ToolCallLocation>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_input: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_output: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Tool call content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ToolCallContent {
    Content(Content),
    Diff(Diff),
    Terminal(Terminal),
}

/// Tool kind
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Read,
    Edit,
    Delete,
    Move,
    Search,
    Execute,
    Think,
    Fetch,
    SwitchMode,
    Other,
}

/// Tool call status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCallStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Tool call location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallLocation {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Content {
    pub content: ContentBlock,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// File diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diff {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_text: Option<String>,
    pub new_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Terminal reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Terminal {
    #[serde(rename = "terminalId")]
    pub terminal_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub entries: Vec<PlanEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Plan entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanEntry {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<PlanEntryStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<PlanEntryPriority>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Plan entry status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanEntryStatus {
    Pending,
    InProgress,
    Completed,
}

/// Plan entry priority
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanEntryPriority {
    High,
    Medium,
    Low,
}

/// Available commands update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableCommandsUpdate {
    #[serde(rename = "availableCommands")]
    pub available_commands: Vec<AvailableCommand>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Available command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailableCommand {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<AvailableCommandInput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Available command input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum AvailableCommandInput {
    Unstructured(UnstructuredCommandInput),
}

/// Unstructured command input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnstructuredCommandInput {
    pub hint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Current mode update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentModeUpdate {
    #[serde(rename = "currentModeId")]
    pub current_mode_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Cancel notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelNotification {
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Error codes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ErrorCode {
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    AuthRequired = -32000,
    ResourceNotFound = -32002,
}

// Client methods

/// Read text file request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadTextFileRequest {
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Read text file response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadTextFileResponse {
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Write text file request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteTextFileRequest {
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    pub path: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Write text file response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteTextFileResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Request permission request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPermissionRequest {
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    #[serde(rename = "toolCall")]
    pub tool_call: ToolCall,
    pub options: Vec<PermissionOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Permission option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionOption {
    #[serde(rename = "optionId")]
    pub option_id: String,
    pub name: String,
    pub kind: PermissionOptionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Permission option kind
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionOptionKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
}

/// Request permission response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestPermissionResponse {
    pub outcome: RequestPermissionOutcome,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Request permission outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum RequestPermissionOutcome {
    Selected(SelectedPermissionOutcome),
    Cancelled(CancelledPermissionOutcome),
}

/// Selected permission outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectedPermissionOutcome {
    #[serde(rename = "optionId")]
    pub option_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Cancelled permission outcome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelledPermissionOutcome {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

// Terminal methods

/// Create terminal request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTerminalRequest {
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<Vec<EnvVariable>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "outputByteLimit")]
    pub output_byte_limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Create terminal response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTerminalResponse {
    #[serde(rename = "terminalId")]
    pub terminal_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Terminal output request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalOutputRequest {
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    #[serde(rename = "terminalId")]
    pub terminal_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Terminal output response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalOutputResponse {
    pub output: String,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "exitStatus")]
    pub exit_status: Option<TerminalExitStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Terminal exit status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalExitStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "exitCode")]
    pub exit_code: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Release terminal request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseTerminalRequest {
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    #[serde(rename = "terminalId")]
    pub terminal_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Release terminal response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseTerminalResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Kill terminal command request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillTerminalCommandRequest {
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    #[serde(rename = "terminalId")]
    pub terminal_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Kill terminal command response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KillTerminalCommandResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Wait for terminal exit request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitForTerminalExitRequest {
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,
    #[serde(rename = "terminalId")]
    pub terminal_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}

/// Wait for terminal exit response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitForTerminalExitResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "exitCode")]
    pub exit_code: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<Value>,
}