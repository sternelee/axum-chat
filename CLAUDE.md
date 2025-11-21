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

### Building
```bash
just build-server            # Build release version of Rust server
just build-tailwind          # Build minified Tailwind CSS
cargo build --release        # Alternative release build
```

### Database Operations
```bash
sqlx database create         # Create SQLite database
sqlx migrate info            # Show migration status
sqlite3 db/db.db             # Direct database access
```

## Architecture Overview

This is a Rust-based ChatGPT clone using Axum + HTMX with a focus on server-side rendering and real-time streaming.

### Core Architecture

**Application State**: The `AppState` struct contains shared state across the application:
- `Arc<Pool<Sqlite>>` - Database connection pool
- `Tera` - Template engine instance
- `ChatRepository` - Data access layer for chat operations

**Module Structure**:
- `src/main.rs` - Application entry point with server setup and middleware stack
- `src/router/` - HTTP routing and handlers organized by feature (auth, chat, settings, etc.)
- `src/data/` - Database models and repository pattern implementation
- `src/ai/` - OpenAI-compatible API integration and streaming logic
- `src/middleware/` - Authentication, error handling, and user extraction

### Key Patterns

**Repository Pattern**: Database operations are abstracted through `ChatRepository` which handles all SQLite interactions using sqlx for type-safe queries.

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

Uses SQLite with migrations in `db/migrations/`. Key entities:
- `users` - Authentication and user management
- `chats` - Chat sessions per user
- `message_blocks` and `message_pairs` - Hierarchical message storage
- `settings` - User-specific configuration (API keys)

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

### Prerequisites and Setup

**External Dependencies**:
- Install [Just](https://github.com/casey/just): `cargo install just`
- Install [TailwindCSS Standalone CLI](https://tailwindcss.com/blog/standalone-cli) - required for CSS compilation
- SQLite development tools (for direct database access)

**Quick Start**:
1. Clone repository
2. Create `.env` file (see Environment Configuration section)
3. `just init` - installs cargo-watch, sqlx-cli, creates database, runs migrations
4. `just dev` - starts development server with concurrent Tailwind watch

### Important Implementation Notes

- **Axum 0.8**: Uses new routing syntax (`{param}` instead of `:param`)
- **Authentication**: Session-based using HTTP cookies via `tower_cookies`
- **Security**: Passwords are currently stored as plain text (TODO: implement hashing)
- **Frontend Dependencies**:
  - Tailwind CSS standalone CLI must be installed separately
  - HTMX 2.0.8 for dynamic interactions
  - DaisyUI + Tailwind Browser for styling
- **Database**: SQLite with WAL mode for better concurrent access
- **Streaming**: Native JavaScript EventSource API (HTMX SSE extension removed due to `api.selectAndSwap` compatibility issues)
- **Server**: Runs on port 3000 by default
- **Release Build**: Optimized with LTO and symbol stripping for production