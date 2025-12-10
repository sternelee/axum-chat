# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Setup and Initialization
```bash
just init                    # Install tools, create database, run migrations
just db-migrate              # Run database migrations
just db-reset                # Drop, recreate database and run migrations + seed data
```

### Development
```bash
just dev                     # Start development server with Tailwind watch (recommended)
just dev-server              # Start cargo watch for Rust code only
just dev-tailwind            # Start Tailwind CSS watch only
cargo run                    # Start server without watch mode
```

### Testing
```bash
cargo test                   # Run all tests
cargo test <test_name>       # Run specific test
cargo test -- <filter>       # Run tests matching filter
```

### Building
```bash
just build-server            # Build release version of Rust server
just build-tailwind          # Build minified Tailwind CSS
cargo build --release        # Alternative release build
```

### Database Operations
```bash
sqlite3 db/db.db             # Direct database access
```

## Architecture Overview

This is a Rust-based ChatGPT clone using Axum + HTMX with a focus on server-side rendering and real-time streaming.

### Core Architecture

**Application State**: The `AppState` struct contains shared state across the application:
- `Arc<Database>` - Database connection using libsql
- `Tera` - Template engine instance
- `ChatRepository` - Data access layer for chat operations
- `Arc<Mutex<Option<PracticalMcpManager>>>` - MCP server manager for tool integration

**Module Structure**:
- `src/main.rs` - Application entry point with server setup and middleware stack
- `src/router/` - HTTP routing and handlers organized by feature (auth, chat, settings, mcp, providers, agents, etc.)
- `src/data/` - Database models and repository pattern implementation
- `src/ai/` - OpenAI-compatible API integration and streaming logic
- `src/middleware/` - Authentication, error handling, and user extraction
- `src/mcp/` - MCP (Model Context Protocol) server management and tools integration
- `src/utils/` - Utility functions and helpers

### Key Patterns

**Repository Pattern**: Database operations are abstracted through `ChatRepository` which handles all SQLite interactions using libsql with JSON-based parameters and custom error handling.

**Middleware Stack**: Ordered middleware layers handle:
1. Cookie management via `tower_cookies::CookieManagerLayer`
2. User extraction/authentication session management
3. Error handling with custom error pages

**Server-Sent Events (SSE)**: Real-time AI responses are streamed using:
- `reqwest-eventsource` for external AI API communication
- Tokio channels (`mpsc`) for internal message passing
- Native JavaScript EventSource API in the browser (HTMX SSE extension removed due to compatibility issues)
- Axum's SSE support for streaming responses

**Frontend Architecture**:
- HTMX handles most interactions via HTML attributes
- Native JavaScript for SSE streaming (in `templates/components/message.html`)
- Tailwind CSS for styling with standalone CLI
- Tera templating engine for server-side rendering

### Database Schema

Uses SQLite with libsql database backend. Key entities:
- `users` - Authentication and user management
- `chats` - Chat sessions per user
- `message_blocks` and `message_pairs` - Hierarchical message storage
- `settings` - User-specific configuration (API keys)
- `mcp_providers` - MCP provider configurations
- `mcp_tools` - Available MCP tools and functions

### MCP Integration

The application includes comprehensive MCP (Model Context Protocol) server management:
- **Configuration Management**: JSON-based provider configuration in `mcp.json`
- **Tool Integration**: Dynamic loading and execution of MCP tools
- **Provider Support**: Multiple AI providers with configurable endpoints
- **Security**: Secure handling of MCP server processes and communications
- **Management Interface**: Web UI for managing MCP providers and tools

### AI Integration

The streaming system uses `reqwest-eventsource` to communicate with external AI APIs (configured for SiliconFlow/OpenAI compatible endpoints). Messages are processed through a tokio channel system that allows real-time streaming to the browser via SSE.

### Environment Configuration

Required `.env` file:
```
MIGRATIONS_PATH=db/migrations
TEMPLATES_PATH=templates
DATABASE_URL=sqlite:db/db.db
DATABASE_PATH=db/db.db
SILICONFLOW_API_KEY=your-key-here
OPENAI_API_KEY=<api-key> (only necessary for tests, users will add their own keys)
```

**Note**: `SILICONFLOW_API_KEY` is the server's default API key, but users can provide their own OpenAI-compatible API keys in settings.

### Key Configuration Files

- `mcp.json` - MCP server and provider configurations
- `tailwind.config.js` - Tailwind CSS configuration with forms and typography plugins
- `justfile` - Development task automation with Just
- `input.css` - Tailwind CSS input file
- `Cargo.toml` - Rust project dependencies and build configuration

### Prerequisites and Setup

**External Dependencies**:
- Install [Just](https://github.com/casey/just): `cargo install just`
- Install [TailwindCSS Standalone CLI](https://tailwindcss.com/blog/standalone-cli) - required for CSS compilation
- SQLite development tools (for direct database access)

**Quick Start**:
1. Clone repository
2. Create `.env` file (see Environment Configuration section)
3. `just init` - installs cargo-watch, creates database, runs migrations
4. `just dev` - starts development server with concurrent Tailwind watch

### Important Implementation Notes

- **Axum 0.8**: Uses new routing syntax (`{param}` instead of `:param`)
- **Authentication**: Session-based using HTTP cookies via `tower_cookies`
- **Security**: Passwords are currently stored as plain text (TODO: implement hashing)
- **Frontend Dependencies**:
  - Tailwind CSS standalone CLI must be installed separately
  - HTMX 2.0.8 for dynamic interactions
  - Tailwind CSS forms and typography plugins
- **Database**: SQLite with libsql backend and WAL mode for better concurrent access
- **Streaming**: Native JavaScript EventSource API (HTMX SSE extension removed due to `api.selectAndSwap` compatibility issues)
- **Server**: Runs on port 3000 by default
- **Release Build**: Optimized with LTO and symbol stripping for production
- **MCP**: Uses rmcp crate for MCP server communication and process management
- **Dependencies**: Key crates include reqwest-eventsource for SSE, libsql for database, tera for templating, and tower-cookies for session management