# Providers and Agents Management

This document describes the new providers and agents management system in RustGPT, which allows for flexible configuration of AI service providers and creation of specialized AI agents.

## Overview

The providers and agents system provides:

- **Provider Management**: Add, configure, and manage AI service providers (OpenAI, Gemini, etc.)
- **Agent Creation**: Create specialized AI agents with different capabilities and configurations
- **Flexible Configuration**: JSON-based configuration for both providers and agents
- **User-Specific Agents**: Private agents for individual users and public agents shared across the platform
- **Model Selection**: Support for multiple models per provider with capability detection

## Database Schema

### Providers Table

Stores AI service provider configurations:

```sql
CREATE TABLE providers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    provider_type TEXT NOT NULL CHECK (provider_type IN ('openai', 'gemini')),
    base_url TEXT NOT NULL,
    api_key_encrypted TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

### Provider Models Table

Stores available models for each provider:

```sql
CREATE TABLE provider_models (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    context_length INTEGER NOT NULL,
    input_price REAL DEFAULT 0.0,
    output_price REAL DEFAULT 0.0,
    capabilities TEXT NOT NULL, -- JSON array of capabilities
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (provider_id) REFERENCES providers (id) ON DELETE CASCADE,
    UNIQUE(provider_id, name)
);
```

### Agents Table

Stores user-defined AI agents:

```sql
CREATE TABLE agents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    provider_id INTEGER NOT NULL,
    model_name TEXT NOT NULL,
    stream BOOLEAN NOT NULL DEFAULT TRUE,
    chat BOOLEAN NOT NULL DEFAULT TRUE,
    embed BOOLEAN NOT NULL DEFAULT FALSE,
    image BOOLEAN NOT NULL DEFAULT FALSE,
    tool BOOLEAN NOT NULL DEFAULT FALSE,
    tools TEXT DEFAULT '[]', -- JSON array of tool names
    system_prompt TEXT,
    top_p REAL DEFAULT 1.0,
    max_context INTEGER DEFAULT 4096,
    file BOOLEAN NOT NULL DEFAULT FALSE,
    file_types TEXT DEFAULT '[]', -- JSON array of file extensions
    temperature REAL DEFAULT 0.7,
    max_tokens INTEGER DEFAULT 2048,
    presence_penalty REAL DEFAULT 0.0,
    frequency_penalty REAL DEFAULT 0.0,
    icon TEXT DEFAULT '🤖',
    category TEXT DEFAULT 'general',
    public BOOLEAN NOT NULL DEFAULT FALSE,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (provider_id) REFERENCES providers (id) ON DELETE RESTRICT
);
```

## JSON Configuration Examples

### Providers Configuration

```json
{
  "providers": {
    "openai_official": {
      "provider_type": "openai",
      "base_url": "https://api.openai.com/v1",
      "api_key": "${OPENAI_API_KEY}",
      "models": [
        {
          "name": "gpt-4o",
          "display_name": "GPT-4o",
          "context_length": 128000,
          "input_price": 5.0,
          "output_price": 15.0,
          "capabilities": ["chat", "vision", "tools", "stream"]
        }
      ]
    }
  }
}
```

### Agents Configuration

```json
{
  "agents": {
    "general_assistant": {
      "name": "General Assistant",
      "description": "A helpful AI assistant for general conversations",
      "provider": "openai_official",
      "model": "gpt-4o",
      "stream": true,
      "chat": true,
      "embed": false,
      "image": true,
      "tool": true,
      "tools": ["web_search", "calculator", "file_reader"],
      "system_prompt": "You are a helpful AI assistant...",
      "top_p": 0.9,
      "max_context": 12000,
      "file": true,
      "file_types": [".txt", ".md", ".pdf", ".docx"],
      "temperature": 0.7,
      "max_tokens": 4096,
      "presence_penalty": 0.1,
      "frequency_penalty": 0.1,
      "icon": "💬",
      "category": "general",
      "public": true
    }
  }
}
```

## API Endpoints

### Providers API

- `GET /providers` - List providers management page
- `GET /api/providers` - List all providers (JSON)
- `POST /api/providers` - Create new provider
- `GET /api/providers/{id}` - Get specific provider
- `PUT /api/providers/{id}` - Update provider
- `DELETE /api/providers/{id}` - Delete provider
- `GET /api/providers/{id}/models` - Get provider models

### Agents API

- `GET /agents` - List agents management page
- `GET /api/agents` - List user's agents (JSON)
- `POST /api/agents` - Create new agent
- `GET /api/agents/{id}` - Get specific agent
- `PUT /api/agents/{id}` - Update agent
- `DELETE /api/agents/{id}` - Delete agent

## Provider Types

### OpenAI Compatible

For providers that follow the OpenAI API format:
- OpenAI official API
- SiliconFlow
- Anthropic (via proxy)
- Local models with OpenAI-compatible endpoints

Configuration:
```json
{
  "provider_type": "openai",
  "base_url": "https://api.provider.com/v1",
  "api_key": "your-api-key"
}
```

### Google Gemini

For Google's Gemini API:
```json
{
  "provider_type": "gemini",
  "base_url": "https://generativelanguage.googleapis.com/v1beta",
  "api_key": "your-gemini-key"
}
```

## Agent Capabilities

### Basic Capabilities
- `chat` - Text conversation support
- `stream` - Streaming response support
- `embed` - Text embedding generation
- `image` - Image processing/vision
- `tool` - Function calling/tool use
- `file` - File upload and processing

### Advanced Configuration
- `temperature` (0.0-2.0) - Response randomness
- `top_p` (0.0-1.0) - Nucleus sampling
- `max_tokens` - Response length limit
- `max_context` - Context window size
- `presence_penalty` (-2.0 to 2.0) - Encourage new topics
- `frequency_penalty` (-2.0 to 2.0) - Reduce repetition

### Tool Integration
Agents can be configured with various tools:
- `web_search` - Internet search capabilities
- `calculator` - Mathematical calculations
- `file_reader` - Document processing
- `code_executor` - Code execution
- `git_commands` - Git operations

## File Upload Support

When `file` capability is enabled, agents can process uploaded files. Supported file types are configured per agent:

```json
{
  "file": true,
  "file_types": [".txt", ".md", ".pdf", ".docx", ".csv", ".xlsx"]
}
```

## Agent Categories

- `general` - General purpose assistants
- `development` - Programming and development
- `research` - Academic and research tasks
- `creative` - Creative writing and content creation
- `business` - Business and professional tasks

## Public vs Private Agents

- **Private agents**: Only visible to the creator
- **Public agents**: Available to all users on the platform

Public agents are useful for:
- Providing specialized expertise
- Sharing useful configurations
- Creating platform-wide services

## Security Considerations

- API keys are encrypted in the database
- Provider access can be disabled without deletion
- User authentication is required for agent management
- Public agents cannot access private user data

## Setup and Installation

1. **Run Database Migration**:
   ```bash
   just db-migrate
   ```

2. **Import Default Providers**:
   ```bash
   ./scripts/import_configs.sh
   ```

3. **Configure Environment Variables**:
   ```bash
   # Add to .env file
   OPENAI_API_KEY=your-openai-key
   SILICONFLOW_API_KEY=your-siliconflow-key
   GEMINI_API_KEY=your-gemini-key
   ```

4. **Access Management Pages**:
   - Providers: `http://localhost:3000/providers`
   - Agents: `http://localhost:3000/agents`

## Usage Examples

### Creating a Code Review Agent

```json
{
  "name": "Code Review Assistant",
  "description": "Specialized in code review and best practices",
  "provider_id": 1,
  "model_name": "gpt-4o",
  "stream": true,
  "chat": true,
  "tool": true,
  "tools": ["syntax_checker", "security_scanner"],
  "system_prompt": "You are an expert code reviewer...",
  "temperature": 0.3,
  "max_context": 16000,
  "file": true,
  "file_types": [".rs", ".js", ".py", ".java"],
  "category": "development"
}
```

### Creating a Research Assistant

```json
{
  "name": "Research Assistant",
  "description": "Academic research and literature review",
  "provider_id": 3,
  "model_name": "gemini-1.5-pro",
  "stream": true,
  "chat": true,
  "embed": true,
  "image": true,
  "tool": true,
  "tools": ["academic_search", "citation_formatter"],
  "system_prompt": "You are a research assistant...",
  "max_context": 100000,
  "temperature": 0.5,
  "file": true,
  "file_types": [".pdf", ".docx", ".txt"],
  "category": "research",
  "public": true
}
```

## Troubleshooting

### Common Issues

1. **Provider Not Showing Models**
   - Check if the provider is active
   - Verify API key configuration
   - Ensure models are marked as active

2. **Agent Creation Fails**
   - Verify the selected provider has available models
   - Check required fields are filled
   - Ensure model capabilities match agent requirements

3. **Authentication Errors**
   - Verify user is logged in
   - Check session cookies
   - Ensure auth middleware is properly configured

### Debug Mode

Enable debug logging by setting the environment variable:
```bash
export RUST_LOG=debug
cargo run
```

This will show detailed SQL queries and API request information.