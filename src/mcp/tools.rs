use serde_json::Value;
use std::collections::HashMap;
use tracing::{info, warn, error};

use crate::mcp::manager::McpManager;

pub struct McpToolManager {
    mcp_manager: McpManager,
    tool_registry: HashMap<String, RegisteredTool>,
}

pub struct RegisteredTool {
    pub id: String,
    pub service_id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub requires_approval: bool,
    pub auto_approved: bool,
    pub usage_count: u64,
    pub last_used: Option<std::time::Instant>,
    pub parameters: Option<Value>,
}

impl McpToolManager {
    pub async fn new(config_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mcp_manager = McpManager::new(config_path)?;

        Ok(Self {
            mcp_manager,
            tool_registry: HashMap::new(),
        })
    }

    pub async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Start enabled MCP services
        self.mcp_manager.start_enabled_services().await?;

        // Load available tools from all services
        self.refresh_tools().await?;

        info!("MCP Tool Manager initialized with {} tools", self.tool_registry.len());
        Ok(())
    }

    pub async fn refresh_tools(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let rustgpt_tools = self.mcp_manager.get_rustgpt_tools().await;

        self.tool_registry.clear();

        for tool in rustgpt_tools {
            let registered_tool = RegisteredTool {
                id: tool.id.clone(),
                service_id: tool.service_id,
                name: tool.name,
                description: tool.description,
                category: tool.category,
                requires_approval: tool.requires_approval,
                auto_approved: tool.auto_approved,
                usage_count: tool.usage_count,
                last_used: tool.last_used,
                parameters: None, // Will be populated when needed
            };

            self.tool_registry.insert(tool.id, registered_tool);
        }

        Ok(())
    }

    pub async fn call_tool(
        &mut self,
        tool_id: &str,
        arguments: Option<Value>,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        if let Some(tool) = self.tool_registry.get(tool_id) {
            info!("Calling MCP tool: {}::{}", tool.service_id, tool.name);

            // Check if tool requires approval
            if tool.requires_approval && !tool.auto_approved {
                return Err(format!("Tool {} requires approval", tool_id).into());
            }

            // Call the tool via MCP manager
            let result = self.mcp_manager.call_tool(
                &tool.service_id,
                &tool.name,
                arguments,
            ).await;

            // Update usage stats
            if let Ok(ref mut tool) = self.tool_registry.get_mut(tool_id) {
                tool.usage_count += 1;
                tool.last_used = Some(std::time::Instant::now());
            }

            result
        } else {
            Err(format!("Tool {} not found", tool_id).into())
        }
    }

    pub async fn get_tool_info(&self, tool_id: &str) -> Option<&RegisteredTool> {
        self.tool_registry.get(tool_id)
    }

    pub async fn list_tools(&self) -> Vec<&RegisteredTool> {
        self.tool_registry.values().collect()
    }

    pub async fn list_tools_by_category(&self, category: &str) -> Vec<&RegisteredTool> {
        self.tool_registry
            .values()
            .filter(|tool| tool.category == category)
            .collect()
    }

    pub async fn get_approval_required_tools(&self) -> Vec<&RegisteredTool> {
        self.tool_registry
            .values()
            .filter(|tool| tool.requires_approval && !tool.auto_approved)
            .collect()
    }

    pub async fn approve_tool(&mut self, tool_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(tool) = self.tool_registry.get_mut(tool_id) {
            if tool.requires_approval {
                tool.requires_approval = false;
                info!("Approved tool: {}", tool_id);
                Ok(())
            } else {
                Err(format!("Tool {} does not require approval", tool_id).into())
            }
        } else {
            Err(format!("Tool {} not found", tool_id).into())
        }
    }

    pub async fn revoke_tool_approval(&mut self, tool_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(tool) = self.tool_registry.get_mut(tool_id) {
            tool.requires_approval = true;
            info!("Revoked approval for tool: {}", tool_id);
            Ok(())
        } else {
            Err(format!("Tool {} not found", tool_id).into())
        }
    }

    pub async fn get_usage_stats(&self) -> ToolUsageStats {
        let mut total_calls = 0;
        let mut category_stats = HashMap::new();
        let mut service_stats = HashMap::new();

        for tool in self.tool_registry.values() {
            total_calls += tool.usage_count;

            // Category stats
            *category_stats.entry(tool.category.clone())
                .or_insert(0) += tool.usage_count;

            // Service stats
            *service_stats.entry(tool.service_id.clone())
                .or_insert(0) += tool.usage_count;
        }

        ToolUsageStats {
            total_calls,
            category_stats,
            service_stats,
            most_used_tools: self.get_most_used_tools(5).await,
        }
    }

    pub async fn get_service_status(&self) -> Vec<crate::mcp::manager::ServiceStatusInfo> {
        self.mcp_manager.list_services().await
    }

    pub async fn restart_service(&self, service_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.mcp_manager.restart_service(service_id).await
    }

    async fn get_most_used_tools(&self, limit: usize) -> Vec<(String, u64)> {
        let mut tools: Vec<_> = self.tool_registry
            .values()
            .map(|tool| (tool.id.clone(), tool.usage_count))
            .collect();

        tools.sort_by(|a, b| b.1.cmp(&a.1));
        tools.truncate(limit);
        tools
    }
}

#[derive(Debug, Clone)]
pub struct ToolUsageStats {
    pub total_calls: u64,
    pub category_stats: HashMap<String, u64>,
    pub service_stats: HashMap<String, u64>,
    pub most_used_tools: Vec<(String, u64)>,
}

// RustGPT integration specific functions
impl McpToolManager {
    /// Get tools that are safe to use in RustGPT context
    pub async fn get_safe_tools(&self) -> Vec<&RegisteredTool> {
        self.tool_registry
            .values()
            .filter(|tool| {
                // Filter out potentially dangerous tools
                !tool.name.contains("delete") &&
                !tool.name.contains("remove") &&
                !tool.name.contains("execute") &&
                !tool.name.contains("system") &&
                !tool.requires_approval
            })
            .collect()
    }

    /// Get tools that require user approval
    pub async fn get_approval_needed_tools(&self) -> Vec<&RegisteredTool> {
        self.tool_registry
            .values()
            .filter(|tool| tool.requires_approval && !tool.auto_approved)
            .collect()
    }

    /// Execute a tool in RustGPT context with safety checks
    pub async fn execute_safe_tool(
        &mut self,
        tool_id: &str,
        arguments: Option<Value>,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        if let Some(tool) = self.tool_registry.get(tool_id) {
            // Additional safety checks for RustGPT context
            if self.is_tool_safe_for_rustgpt(tool) {
                self.call_tool(tool_id, arguments).await
            } else {
                Err(format!("Tool {} is not safe for RustGPT execution", tool_id).into())
            }
        } else {
            Err(format!("Tool {} not found", tool_id).into())
        }
    }

    fn is_tool_safe_for_rustgpt(&self, tool: &RegisteredTool) -> bool {
        // Define safety criteria for RustGPT
        match tool.category.as_str() {
            "filesystem" => {
                // Allow read-only filesystem operations
                tool.name.contains("read") ||
                tool.name.contains("list") ||
                tool.name.contains("search")
            }
            "database" => {
                // Allow SELECT queries only
                tool.name.contains("select") ||
                tool.name.contains("describe") ||
                tool.name.contains("list")
            }
            "search" => true, // Search operations are generally safe
            "web" => false,     // Web operations can be risky
            _ => false,        // Unknown categories are unsafe by default
        }
    }
}