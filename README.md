# RustGPT 🦀✨

https://github.com/bitswired/rustgpt/assets/19983429/980a88b9-93df-48c7-a438-b232d2830e00

Welcome to the RustGPT repository! Here, you'll find a web ChatGPT clone entirely crafted using Rust and HTMX, where technology meets simplicity and performance. 🚀

- [Try the RustGPT hosted demo](https://rustgpt.bitswired.com)
- [Read the blog article](https://bitswired.com/blog/posts/rustgpt-journey-rust-htmx-web-dev)

## Introduction

RustGPT is an advanced ChatGPT clone built entirely in Rust, showcasing modern web development with the Axum framework and HTMX. This project has evolved from a simple chat application into a comprehensive AI development platform that supports both cloud-based AI services and local AI coding agents.

With the recent addition of **Agent Client Protocol (ACP)** support, RustGPT now provides a standardized interface for integrating local AI coding assistants like Claude Code, Cursor, and Aider, making it a powerful development environment for AI-assisted programming.

### 🆕 New Features

- **🤖 Local AI Agent Support**: Integration with 10+ local AI coding assistants via ACP protocol
- **📡 Agent Client Protocol**: Full ACP implementation compatible with Zed, VSCode, and other modern editors
- **🔧 MCP Integration**: Model Context Protocol support for extensibility
- **⚡ Provider Management**: Capability-based provider system supporting multiple AI services

## Features 🌟

### 🤖 AI Integration
- **Multiple AI Providers**: Support for OpenAI, Anthropic, Google Gemini, and more
- **Local AI Agents**: Integration with 10+ local AI coding assistants via ACP protocol
- **Capability-Based System**: Dynamic provider capabilities (chat, embed, image, streaming, tools, vision)
- **MCP Support**: Model Context Protocol for extensibility

### 🌐 Web Application
- **Rust with Axum Framework**: High-performance server with async-first design
- **SQLite/libsql**: Lightweight database with async support and WAL mode
- **Server Sent Events (SSE)**: Real-time streaming for AI responses
- **HTMX**: Dynamic interactions without heavy JavaScript frameworks
- **Tailwind CSS**: Modern, utility-first styling

### 📡 Agent Client Protocol (ACP)
- **Standard Compliant**: Full ACP implementation based on [Agent Client Protocol](https://agentclientprotocol.com)
- **Multiple Transports**: Stdio, HTTP, and WebSocket transport layers
- **Session Management**: Advanced session handling with state persistence
- **Tool Integration**: File operations, terminal management, and permission requests
- **Real-time Updates**: Live streaming of agent responses and progress

### 🔧 Development Features
- **Provider Management**: Web UI for configuring AI providers and agents
- **User Authentication**: Session-based user management
- **Responsive Design**: Mobile-friendly interface
- **Error Handling**: Comprehensive error handling and recovery

## Tech Stack 🛠️

### Core Framework
- [`axum`](https://github.com/tokio-rs/axum): Web application framework with async-first design
- [`tokio`](https://github.com/tokio-rs/tokio): Asynchronous runtime for Rust
- [`serde`](https://github.com/serde-rs/serde): Serialization framework for Rust

### Database & Storage
- [`libsql`](https://github.com/libsql/libsql): Modern SQLite database with async support
- [`libsql-client`](https://github.com/libsql/libsql-client-rs): Async SQLite client

### Web Frontend
- [`tera`](https://github.com/Keats/tera): Templating engine for HTML views
- [`htmx`](https://htmx.org/): Dynamic web interactions without JavaScript frameworks
- [`tailwindcss`](https://tailwindcss.com/): Utility-first CSS framework

### AI & Protocol Integration
- **Agent Client Protocol (ACP)**: Standard for AI agent communication
  - [`async-trait`](https://github.com/dtolnay/async-trait): Async trait definitions
  - `JSON-RPC 2.0`: Standard protocol for client-server communication
- **MCP Support**: Model Context Protocol integration
  - [`rmcp`](https://github.com/juziyo/rmcp): Rust MCP client implementation

### HTTP & Networking
- [`reqwest`](https://github.com/seanmonstar/reqwest): HTTP client for API calls
- [`reqwest-eventsource`](https://github.com/joshuaquek/reqwest-eventsource): SSE support
- [`tower-http`](https://github.com/tower-rs/tower-http): HTTP middleware
- [`tower-cookies`](https://github.com/tower-rs/tower-cookies): Cookie management

### Utilities
- [`uuid`](https://github.com/uuid-rs/uuid): UUID generation
- [`chrono`](https://github.com/chronotope/chrono): Date and time handling
- [`tracing`](https://github.com/tokio-rs/tracing): Structured logging

## 🤖 Supported Local AI Agents

RustGPT supports integration with local AI coding assistants through the Agent Client Protocol (ACP). These agents run locally on your machine and provide powerful AI-assisted programming capabilities.

### Available Agents

| Agent | Command | Special Features | ACP Support |
|-------|---------|------------------|-------------|
| **Claude Code** | `claude-code` | 🎯 Full ACP support<br>🖼️ Image & Audio<br>🔧 MCP Integration<br>📋 Session Persistence | ✅ Full |
| **Gemini CLI** | `gemini chat` | 🖼️ Image support<br>🔍 Context-aware | ✅ Standard |
| **Cursor CLI** | `cursor agent` | 💻 Code editor integration<br>🎯 AI-assisted editing | ✅ Standard |
| **Aider** | `aider` | 🔄 Git integration<br>📝 Embedded context<br>🤝 Pair programming | ✅ Enhanced |
| **CodeiumChat** | `codeium chat` | 🧠 Code completion<br>💬 Chat interface | ✅ Standard |
| **GitHub Copilot CLI** | `github-copilot` | 🐙 GitHub integration<br>💡 Code suggestions | ✅ Standard |
| **Tabnine** | `tabnine` | ⚡ Fast completion<br>🧠 Context-aware | ✅ Standard |
| **Qwen Code** | `qwen-code` | 🌏 Alibaba model<br>🔍 Code understanding | ✅ Standard |
| **ZAIGLM** | `zaiglm` | 🇨🇳 Chinese model<br>🔧 Development tools | ✅ Standard |
| **Codex CLI** | `codex` | 🔤 OpenAI Codex<br>📝 Code generation | ✅ Standard |

### Setting Up Local Agents

1. **Install the Agent**: Follow the installation instructions for your preferred agent
2. **Configure in RustGPT**: Use the provider management UI to add the local agent
3. **Customize Command**: Optionally specify custom commands and arguments
4. **Set Capabilities**: Configure which features the agent supports
5. **Start Chatting**: Begin using the agent directly in the RustGPT interface

### ACP Protocol Features

The Agent Client Protocol enables advanced features:

- 🔄 **Session Management**: Persistent conversation history
- 🛠️ **Tool Calls**: File operations, terminal commands, and more
- 📡 **Real-time Updates**: Live streaming of agent responses
- 🔐 **Permission Requests**: Interactive approval for sensitive operations
- 📋 **Execution Plans**: View and understand agent reasoning
- 🎯 **Slash Commands**: Quick access to agent features

## Quickstart 🏁

### Prerequisites

- **Rust**: Latest stable version
- **Just**: Task runner (`cargo install just`)
- **TailwindCSS**: Standalone CLI ([installation guide](https://tailwindcss.com/blog/standalone-cli))

### Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/sternelee/axum-chat.git
   cd axum-chat
   ```

2. **Create environment file**
   ```bash
   cp .env.example .env
   ```

3. **Configure your environment**
   ```bash
   # .env file
   MIGRATIONS_PATH=db/migrations
   TEMPLATES_PATH=templates
   DATABASE_URL=sqlite:db/db.db
   DATABASE_PATH=db/db.db
   SILICONFLOW_API_KEY=your-server-api-key  # Default API key
   ```

4. **Initialize the project**
   ```bash
   just init  # Install tools, create database, run migrations
   ```

5. **Start the development server**
   ```bash
   just dev  # Start server with Tailwind watch (recommended)
   # or
   just dev-server  # Server only
   just dev-tailwind  # Tailwind only
   ```

6. **Open your browser**
   Navigate to `http://localhost:3000` and start chatting!

### Adding AI Providers

1. **Cloud Providers**: Add your API keys in the Settings page
2. **Local Agents**: Configure local AI agents in the Provider Management page
3. **Custom Commands**: Specify custom commands for local agents

### Development Commands

```bash
just init                    # Install tools, create database, run migrations
just dev                     # Start development server with Tailwind watch
just dev-server              # Start cargo watch for Rust code only
just dev-tailwind            # Start Tailwind CSS watch only
cargo run                    # Start server without watch mode
cargo test                   # Run all tests
just build-server            # Build release version
```

## Contributing 🤝

Contributions are what make the open-source community an incredible place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

If you have a suggestion that would make RustGPT better, please fork the repo and create a pull request. You can also simply open an issue. Don't forget to give the project a star! Thank you again!

## Architecture 📐

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Web Frontend   │    │   AI Providers   │    │  Local Agents   │
│                 │    │                 │    │                 │
│ • HTMX + Tailwind │    │ • OpenAI API     │    │ • Claude Code    │
│ • Server Sent    │    │ • Anthropic      │    │ • Cursor CLI     │
│   Events         │    │ • Google Gemini  │    │ • Aider         │
│ • Responsive UI  │    │ • SiliconFlow    │    │ • 10+ Agents     │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │   Rust Backend   │
                    │                 │
                    │ • Axum Framework │
                    │ • SQLite/libsql  │
                    │ • Async/Tokio    │
                    │ • ACP Protocol   │
                    │ • MCP Support    │
                    └─────────────────┘
```

## Project Status 🚧

- ✅ **Core Chat Functionality**: Complete
- ✅ **Multiple AI Providers**: Complete
- ✅ **Agent Client Protocol**: Complete
- ✅ **Local AI Agent Support**: Complete
- ✅ **Provider Management UI**: Complete
- 🚧 **Advanced MCP Integration**: In Progress
- 📋 **User Management Enhancement**: Planned
- 📋 **Plugin System**: Planned

## Contributing 🤝

Contributions are what make the open-source community an incredible place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

### Areas for Contribution

- 🤖 **Additional AI Agents**: Help integrate more local AI assistants
- 📡 **Transport Improvements**: Enhance WebSocket and HTTP transports
- 🎨 **UI/UX Enhancements**: Improve the web interface
- 📚 **Documentation**: Help improve docs and examples
- 🧪 **Testing**: Add more comprehensive test coverage
- 🔧 **Performance**: Optimize for better performance and scalability

### Development Setup

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## Acknowledgments 🎓

Hats off to the wonderful projects and libraries that made RustGPT possible!

### Core Dependencies
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [HTMX](https://htmx.org) - Dynamic web interactions
- [libsql](https://github.com/libsql/libsql) - Modern SQLite
- [Tokio](https://github.com/tokio-rs/tokio) - Async runtime

### AI & Protocol Standards
- [Agent Client Protocol](https://agentclientprotocol.com) - Standard for AI agents
- [Zed Industries](https://zed.dev) - ACP implementation reference
- [Anthropic Claude Code](https://claude.ai/code) - Local AI coding assistant

### Inspiration
- Original RustGPT project by [Bitswired](https://www.bitswired.com)
- Modern AI development tools and standards

## License 📄

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

Created with 💚 by the Rust community! Built with passion for modern web development and AI integration.
