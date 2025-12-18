# MCP Integration for axum-chat

This document describes the Model Context Protocol (MCP) integration added to the axum-chat project.

## Overview

The MCP integration allows axum-chat to connect to external MCP servers and expose their tools as callable functions within the chat interface. This enables AI assistants to perform actions like file operations, API calls, web browsing, and more using real MCP protocol communication.

## Architecture

### Core Components

1. **MCP Configuration (`src/mcp/config.rs`)**
   - Handles loading and saving `mcp.json` configuration files
   - Supports multiple transport types (stdio, SSE, HTTP)
   - Provides builder methods for common MCP servers

2. **Real MCP Client (`src/mcp/client.rs`)**
   - Process-based MCP client implementation using JSON-RPC over stdio
   - Direct execution of MCP server processes with proper protocol communication
   - Implements full MCP protocol: initialize, tools/list, tools/call, resources, prompts
   - Error handling, timeout management, and process lifecycle management

3. **MCP Manager (`src/mcp/manager.rs`)**
   - Centralized management of multiple MCP servers
   - Tool registration with server-prefixed naming (e.g., `filesystem__read_file`)
   - Connection lifecycle management and graceful shutdown
   - Tool execution with timeouts and error propagation

4. **Tool Integration (`src/mcp/tools.rs`)**
   - Bridge between MCP tools and OpenAI function calling format
   - Security validation and permission checking
   - Streaming support for tool execution
   - Built-in tool support for basic operations

### Integration Points

- **AI Stream Processing**: Modified `src/ai/stream.rs` to include MCP tools in chat requests
- **Settings API**: Added endpoints for MCP server management
- **Data Models**: Extended with tool-related structures
- **Main Application**: MCP server initialization and graceful shutdown

## Real Implementation Features

### Process-Based Communication

- **Direct JSON-RPC**: Sends properly formatted MCP JSON-RPC requests to stdin
- **Response Parsing**: Reads and parses JSON responses from stdout
- **Error Handling**: Captures and processes stderr for debugging
- **Timeout Management**: Configurable timeouts for all MCP operations
- **Process Lifecycle**: Proper process cleanup and resource management

### Protocol Implementation

- **Initialize**: MCP client initialization with capabilities negotiation
- **Tools Discovery**: Dynamic tool discovery from MCP servers
- **Tool Execution**: Real tool calls with argument marshaling
- **Resource Access**: File and resource management (when supported by servers)
- **Prompt Management**: Prompt template support (when supported)

### Security Features

- **Path Traversal Protection**: Basic filesystem security validation
- **Tool Isolation**: Each tool call runs in separate process
- **Argument Validation**: Input sanitization and validation
- **Error Boundaries**: Error isolation between tools and main application

## Configuration

### mcp.json Format

```json
{
  "mcp_servers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/allowed/files"],
      "description": "Filesystem access and management",
      "timeout": 300,
      "transport": "stdio"
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": {
        "GITHUB_PERSONAL_ACCESS_TOKEN": "your-token"
      },
      "description": "GitHub integration",
      "timeout": 300,
      "transport": "stdio"
    }
  }
}
```

### Server Configuration Options

- `command`: Command to run for stdio transport
- `args`: Command line arguments
- `env`: Environment variables
- `disabled`: Whether the server is disabled
- `timeout`: Request timeout in seconds
- `description`: Human-readable description
- `transport`: Transport type ("stdio", "sse", "http")
- `url`: URL for SSE/HTTP transports
- `headers`: HTTP headers for SSE/HTTP transports

## Available Endpoints

### MCP Settings API

- `GET /settings/mcp` - List MCP servers and tools
- `POST /settings/mcp/update` - Add/update MCP server
- `POST /settings/mcp/delete` - Remove MCP server
- `POST /settings/mcp/restart` - Restart MCP server

## Usage

### For Users

1. Configure MCP servers in `mcp.json` or through the settings UI
2. Chat with the AI and ask it to use available tools
3. Tool calls will be executed automatically and results shown in the conversation

### For Developers

1. Add new MCP server configurations in `mcp.json`
2. The system will automatically discover and register tools
3. Tools are available with server prefix (e.g., `filesystem__read_file`)

## Security Features

- Path traversal protection for filesystem tools
- Tool execution timeouts
- Configurable allowed tools per server
- Environment variable filtering

## Mock Implementation

Current implementation uses mock clients that simulate tool execution. To enable real MCP functionality:

1. Replace mock client with actual rmcp implementation
2. Update imports to use rmcp types
3. Configure real MCP servers with proper authentication

## Example Usage

### File Operations

```
User: Can you read the contents of src/main.rs?
AI: [Calls filesystem__read_file tool]
AI: Here's what's in src/main.rs: [file contents]
```

### GitHub Integration

```
User: Create an issue in the repo
AI: [Calls github__create_issue tool]
AI: I've created issue #123: [issue details]
```

## Dependencies Added

```toml
# MCP dependencies
rmcp = { version = "0.9", features = ["schemars", "auth", "transport-io", "client"] }
async-stream = "0.3"
schemars = { version = "0.8", features = ["derive"] }
dirs = "5"
async-trait = "0.1"
thiserror = "1"
```

## Future Enhancements

1. Real rmcp client implementation
2. UI components for MCP server configuration
3. Tool execution history and logging
4. Advanced permission management
5. Streaming tool responses
6. Tool composition and workflows

## Testing the MCP Integration

### Setup Instructions

1. **Install Node.js**: MCP servers typically run on Node.js
   ```bash
   # Verify Node.js is installed
   node --version
   npm --version
   ```

2. **Configure MCP Servers**: Copy the example configuration
   ```bash
   cp mcp.json.example mcp.json
   ```

3. **Update Configuration**: Edit `mcp.json` with your settings
   ```json
   {
     "mcp_servers": {
       "filesystem": {
         "command": "npx",
         "args": ["-y", "@modelcontextprotocol/server-filesystem", "/your/path"],
         "description": "Filesystem access for your project",
         "timeout": 300,
         "transport": "stdio"
       }
     }
   }
   ```

4. **Start the Application**:
   ```bash
   just dev
   ```

### Testing with Real MCP Servers

**Filesystem Server**:
- Configure a filesystem server in `mcp.json`
- Try: "Can you read the contents of src/main.rs?"
- Try: "Create a new file called test.txt with hello world content"

**Memory Server**:
- Add memory server to configuration
- Try: "Remember that I like cats"
- Try: "What did I tell you I like earlier?"

### Debugging Tips

1. **Check Server Initialization**: Look for "Initializing MCP client: ..." messages in console
2. **Check Tool Discovery**: Verify tools are loaded in the chat interface
3. **Monitor stderr**: MCP server errors are logged to console
4. **Timeout Issues**: Increase timeout values in configuration if needed
5. **Command Path**: Ensure Node.js and npm commands are in your PATH

### Troubleshooting Common Issues

1. **"Failed to spawn process"**: Check command paths and Node.js installation
2. **"No valid JSON response found"**: MCP server not responding properly
3. **"Process exited with status"**: Check server configuration and arguments
4. **Tools not appearing**: Verify MCP server is compatible and supports tools/list

## Future Enhancements

1. Add real SSE and HTTP transport support
2. Add UI components for MCP server configuration
3. Tool execution history and logging
4. Advanced permission management
5. Streaming tool responses
6. Tool composition and workflows
7. Resource and prompt support integration
8. Visual tool execution interface