use super::types::*;
use super::server::{AcpAgent, AcpServerError};
use super::transport::StdioTransport;
use crate::data::model::ProviderType;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A concrete ACP agent implementation that wraps various AI coding assistants
pub struct LocalCodingAgent {
    agent_type: ProviderType,
    command: String,
    args: Vec<String>,
    capabilities: AgentCapabilities,
    sessions: Arc<RwLock<HashMap<SessionId, SessionInfo>>>,
}

/// Session information
#[derive(Debug, Clone)]
struct SessionInfo {
    id: SessionId,
    cwd: String,
    mcp_servers: Vec<McpServer>,
    created_at: std::time::Instant,
}

impl LocalCodingAgent {
    /// Create a new local coding agent
    pub fn new(agent_type: ProviderType, command: String, args: Vec<String>) -> Self {
        let capabilities = Self::get_capabilities_for_agent_type(&agent_type);

        Self {
            agent_type,
            command,
            args,
            capabilities,
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the agent configuration for each provider type
    fn get_agent_config(provider_type: &ProviderType) -> (String, Vec<String>) {
        match provider_type {
            ProviderType::ClaudeCode => {
                ("claude-code".to_string(), vec![])
            }
            ProviderType::GeminiCLI => {
                ("gemini".to_string(), vec!["chat".to_string()])
            }
            ProviderType::CodexCLI => {
                ("codex".to_string(), vec![])
            }
            ProviderType::CursorCLI => {
                ("cursor".to_string(), vec!["agent".to_string()])
            }
            ProviderType::QwenCode => {
                ("qwen-code".to_string(), vec![])
            }
            ProviderType::ZAIGLM => {
                ("zaiglm".to_string(), vec![])
            }
            ProviderType::Aider => {
                ("aider".to_string(), vec![])
            }
            ProviderType::CodeiumChat => {
                ("codeium".to_string(), vec!["chat".to_string()])
            }
            ProviderType::CopilotCLI => {
                ("github-copilot".to_string(), vec![])
            }
            ProviderType::Tabnine => {
                ("tabnine".to_string(), vec![])
            }
            // Fallback for other provider types
            _ => ("echo".to_string(), vec!["ACP agent not configured".to_string()]),
        }
    }

    /// Get capabilities for a specific agent type
    fn get_capabilities_for_agent_type(agent_type: &ProviderType) -> AgentCapabilities {
        let base_capabilities = AgentCapabilities {
            load_session: false,
            mcp_capabilities: McpCapabilities {
                http: false,
                sse: false,
                _meta: None,
            },
            prompt_capabilities: PromptCapabilities {
                audio: false,
                embedded_context: false,
                image: matches!(agent_type, ProviderType::ClaudeCode | ProviderType::GeminiCLI),
                _meta: None,
            },
            session_capabilities: SessionCapabilities {
                capabilities: HashMap::new(),
                _meta: None,
            },
            _meta: None,
        };

        // Customize capabilities based on agent type
        match agent_type {
            ProviderType::ClaudeCode => AgentCapabilities {
                load_session: true,
                mcp_capabilities: McpCapabilities {
                    http: true,
                    sse: true,
                    _meta: None,
                },
                prompt_capabilities: PromptCapabilities {
                    audio: true,
                    embedded_context: true,
                    image: true,
                    _meta: None,
                },
                ..base_capabilities
            },
            ProviderType::Aider => AgentCapabilities {
                prompt_capabilities: PromptCapabilities {
                    embedded_context: true,
                    image: false,
                    audio: false,
                    _meta: None,
                },
                ..base_capabilities
            },
            _ => base_capabilities,
        }
    }

    /// Create a subprocess transport for the agent
    async fn create_agent_transport(&self, cwd: Option<String>) -> Result<StdioTransport, AcpServerError> {
        let mut command = tokio::process::Command::new(&self.command);
        command.args(&self.args);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::inherit());

        if let Some(working_dir) = cwd {
            command.current_dir(working_dir);
        }

        let transport = StdioTransport::new(self.command.clone(), self.args.clone()).await
            .map_err(|e| AcpServerError::TransportError(e.to_string()))?;

        Ok(transport)
    }
}

#[async_trait]
impl AcpAgent for LocalCodingAgent {
    async fn initialize(&self, request: InitializeRequest) -> Result<InitializeResponse, AcpServerError> {
        let agent_info = Implementation {
            name: match self.agent_type {
                ProviderType::ClaudeCode => "Claude Code",
                ProviderType::GeminiCLI => "Gemini CLI",
                ProviderType::CodexCLI => "Codex CLI",
                ProviderType::CursorCLI => "Cursor CLI",
                ProviderType::QwenCode => "Qwen Code",
                ProviderType::ZAIGLM => "ZAI GLM",
                ProviderType::Aider => "Aider",
                ProviderType::CodeiumChat => "Codeium Chat",
                ProviderType::CopilotCLI => "GitHub Copilot CLI",
                ProviderType::Tabnine => "Tabnine",
                _ => "Unknown Agent",
            }.to_string(),
            version: "1.0.0".to_string(),
            title: Some(format!("{} (ACP)", match self.agent_type {
                ProviderType::ClaudeCode => "Claude Code",
                ProviderType::GeminiCLI => "Gemini CLI",
                ProviderType::CodexCLI => "Codex CLI",
                ProviderType::CursorCLI => "Cursor CLI",
                ProviderType::QwenCode => "Qwen Code",
                ProviderType::ZAIGLM => "ZAI GLM",
                ProviderType::Aider => "Aider",
                ProviderType::CodeiumChat => "Codeium Chat",
                ProviderType::CopilotCLI => "GitHub Copilot CLI",
                ProviderType::Tabnine => "Tabnine",
                _ => "Unknown Agent",
            })),
        };

        Ok(InitializeResponse {
            protocol_version: ProtocolVersion(1),
            agent_info: Some(agent_info),
            agent_capabilities: self.capabilities.clone(),
            auth_methods: vec![],
            _meta: None,
        })
    }

    async fn authenticate(&self, _request: AuthenticateRequest) -> Result<AuthenticateResponse, AcpServerError> {
        // Most local agents don't require authentication
        Ok(AuthenticateResponse { _meta: None })
    }

    async fn new_session(&self, request: NewSessionRequest) -> Result<NewSessionResponse, AcpServerError> {
        let session_id = format!("session_{}", uuid::Uuid::new_v4());
        let cwd = request.cwd.unwrap_or_else(|| std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "/".to_string()));

        let session_info = SessionInfo {
            id: session_id.clone(),
            cwd: cwd.clone(),
            mcp_servers: request.mcp_servers,
            created_at: std::time::Instant::now(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id.clone(), session_info);

        // For now, we don't implement complex mode management
        let modes = None;

        Ok(NewSessionResponse {
            session_id,
            modes,
            _meta: None,
        })
    }

    async fn load_session(&self, request: LoadSessionRequest) -> Result<LoadSessionResponse, AcpServerError> {
        // Check if we support session loading
        if !self.capabilities.load_session {
            return Err(AcpServerError::MethodNotFound("session/load".to_string()));
        }

        let session_info = SessionInfo {
            id: request.session_id.clone(),
            cwd: request.cwd.clone(),
            mcp_servers: request.mcp_servers,
            created_at: std::time::Instant::now(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(request.session_id.clone(), session_info);

        Ok(LoadSessionResponse { modes: None, _meta: None })
    }

    async fn prompt(&self, request: PromptRequest) -> Result<PromptResponse, AcpServerError> {
        // Get session info
        let session_info = {
            let sessions = self.sessions.read().await;
            sessions.get(&request.session_id).cloned()
                .ok_or_else(|| AcpServerError::InvalidParams("Invalid session ID".to_string()))?
        };

        // Create transport for this request
        let _transport = self.create_agent_transport(Some(session_info.cwd)).await?;

        // Convert ACP content blocks to agent-specific format
        let mut prompt_text = String::new();
        for block in request.prompt {
            match block {
                ContentBlock::Text(text) => {
                    prompt_text.push_str(&text.text);
                }
                ContentBlock::ResourceLink(resource) => {
                    prompt_text.push_str(&format!("\n\nResource: {} ({})", resource.name, resource.uri));
                }
                ContentBlock::Image(image) => {
                    // For agents that support images, we'd handle this differently
                    prompt_text.push_str(&format!("\n\n[Image: {}]", image.mime_type));
                }
                _ => {
                    // Handle other content types as needed
                }
            }
        }

        // This is a simplified implementation
        // In a real implementation, we'd:
        // 1. Start the agent subprocess
        // 2. Send the prompt
        // 3. Stream back responses via session/update notifications
        // 4. Return the final response

        Ok(PromptResponse {
            stop_reason: StopReason::EndTurn,
            _meta: None,
        })
    }

    async fn cancel(&self, _request: CancelNotification) -> Result<(), AcpServerError> {
        // Implementation would cancel any ongoing operations
        Ok(())
    }

    async fn set_mode(&self, _request: super::client::SetSessionModeRequest) -> Result<super::client::SetSessionModeResponse, AcpServerError> {
        // Most local agents don't have sophisticated mode management
        Ok(super::client::SetSessionModeResponse { _meta: None })
    }

    async fn session_update(&self, _notification: SessionNotification) -> Result<(), AcpServerError> {
        // Handle session updates from the client
        Ok(())
    }

    async fn read_text_file(&self, request: ReadTextFileRequest) -> Result<ReadTextFileResponse, AcpServerError> {
        // Simple file reading implementation
        let content = tokio::fs::read_to_string(&request.path)
            .await
            .map_err(|e| AcpServerError::InternalError(format!("Failed to read file: {}", e)))?;

        let lines: Vec<&str> = content.lines().collect();

        let start_line = request.line.unwrap_or(1) as usize;
        let limit = request.limit.unwrap_or_else(|| lines.len().try_into().unwrap_or(0)) as usize;

        let end_line = (start_line + limit - 1).min(lines.len());
        let selected_lines = if start_line <= lines.len() {
            &lines[start_line - 1..end_line]
        } else {
            &[]
        };

        let result = selected_lines.join("\n");

        Ok(ReadTextFileResponse {
            content: result,
            _meta: None,
        })
    }

    async fn write_text_file(&self, request: WriteTextFileRequest) -> Result<WriteTextFileResponse, AcpServerError> {
        // Simple file writing implementation
        tokio::fs::write(&request.path, request.content)
            .await
            .map_err(|e| AcpServerError::InternalError(format!("Failed to write file: {}", e)))?;

        Ok(WriteTextFileResponse { _meta: None })
    }

    async fn request_permission(&self, request: RequestPermissionRequest) -> Result<RequestPermissionResponse, AcpServerError> {
        // For now, automatically approve all requests
        // In a real implementation, this would be handled by the client
        let outcome = RequestPermissionOutcome::Selected(SelectedPermissionOutcome {
            option_id: request.options.first()
                .ok_or_else(|| AcpServerError::InvalidParams("No permission options provided".to_string()))?
                .option_id.clone(),
            _meta: None,
        });

        Ok(RequestPermissionResponse {
            outcome,
            _meta: None,
        })
    }

    async fn create_terminal(&self, request: CreateTerminalRequest) -> Result<CreateTerminalResponse, AcpServerError> {
        // Create a terminal subprocess
        let mut cmd = tokio::process::Command::new(&request.command);
        if let Some(args) = request.args {
            cmd.args(args);
        }
        if let Some(cwd) = request.cwd {
            cmd.current_dir(cwd);
        }
        if let Some(env_vars) = request.env {
            for env_var in env_vars {
                cmd.env(&env_var.name, &env_var.value);
            }
        }

        let child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AcpServerError::InternalError(format!("Failed to start terminal: {}", e)))?;

        let terminal_id = format!("term_{}", child.id().unwrap_or(0));

        // In a real implementation, we'd store the child process and manage its lifecycle
        // For now, we just return a terminal ID

        Ok(CreateTerminalResponse {
            terminal_id,
            _meta: None,
        })
    }

    async fn terminal_output(&self, _request: TerminalOutputRequest) -> Result<TerminalOutputResponse, AcpServerError> {
        // This would return the current output from a terminal process
        // For now, return empty output
        Ok(TerminalOutputResponse {
            output: String::new(),
            truncated: false,
            exit_status: None,
            _meta: None,
        })
    }

    async fn release_terminal(&self, _request: ReleaseTerminalRequest) -> Result<ReleaseTerminalResponse, AcpServerError> {
        // This would kill and clean up a terminal process
        Ok(ReleaseTerminalResponse { _meta: None })
    }

    async fn kill_terminal(&self, _request: KillTerminalCommandRequest) -> Result<KillTerminalCommandResponse, AcpServerError> {
        // This would kill a terminal command without releasing the terminal
        Ok(KillTerminalCommandResponse { _meta: None })
    }

    async fn wait_for_terminal_exit(&self, _request: WaitForTerminalExitRequest) -> Result<WaitForTerminalExitResponse, AcpServerError> {
        // This would wait for a terminal command to exit
        Ok(WaitForTerminalExitResponse {
            exit_code: None,
            signal: None,
            _meta: None,
        })
    }
}

/// Factory function to create agents based on provider type
pub fn create_agent_for_provider(
    provider_type: ProviderType,
    custom_command: Option<String>,
    custom_args: Option<Vec<String>>,
) -> Result<LocalCodingAgent, AcpServerError> {
    let (command, args) = if let (Some(cmd), Some(custom_args)) = (custom_command, custom_args) {
        (cmd, custom_args)
    } else {
        LocalCodingAgent::get_agent_config(&provider_type)
    };

    Ok(LocalCodingAgent::new(provider_type, command, args))
}