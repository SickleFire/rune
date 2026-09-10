# Rune

Rune is an expert AI coding assistant integrated into a software development workspace and Unity game development environment, built in Rust. It provides an interactive REPL/CLI interface, tool execution (file operations, command execution, git operations, code searching, Unity Editor integration, web utilities), and seamless API integration with LLM providers.

## Features

- **Interactive REPL & CLI**: Powered by `rustyline` with colorized output and prompt support.
- **Tool Execution Framework**: Rich set of built-in tools for workspace inspection, file reading/writing/patching, executing shell commands, and git operations.
- **Web Utility Tools**: Built-in HTTP request execution (`http_request`), web page scraping/fetching (`fetch_web_page`), and TCP port verification (`check_tcp_port`).
- **Unity Editor Integration**: Optional Unity Bridge (`--unity` flag) supporting scene inspection, game object/component management, asset searching, reference validation, console log reading, and play mode control.
- **Async API Client**: Robust integration with LLM APIs supporting tool calling and streaming responses.
- **Smart Context**: Global awareness of repository architecture and file systems.
- **Code Search Engine (cix)**: Fast symbol and pattern searching across the repository.
- **Checkpointing & Version Control**: Git integration for safety checks, stashing, and rolling back mutations.

## Project Structure

- `src/main.rs`: Entry point for the CLI and REPL loop, handling user input, history, and command dispatch.
- `src/lib.rs`: Core library initialization, agent orchestrator, and shared modules.
- `src/api.rs`: API communication layer with LLM endpoints, request payload formatting, and response parsing.
- `src/tools.rs`: Tool definitions, execution logic, workspace security validations, and utility functions.
- `src/web_tools.rs`: Web utility tools for HTTP REST requests, webpage fetching & HTML text extraction, and TCP port checks.
- `src/unity_tools.rs`: Unity Editor bridge tools for inspecting scenes, managing GameObjects/components, assets, and console logs.
- `unity/RuneBridge.cs`: C# Unity Editor integration script providing HTTP bridge endpoints for Rune's Unity tools.

## Getting Started

### Prerequisites

- Rust (edition 2024 / stable toolchain)
- Cargo
- Unity 2022+ / Unity Editor (optional, for Unity Bridge integration)

### Building

```bash
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Usage

Run Rune in your workspace:

```bash
cargo run
```

To run with Unity Editor bridge integration enabled:

```bash
cargo run -- --unity
```

Or pass prompts directly via CLI arguments for quick tasks and automation scripts.

## Contributing

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to contribute to this project.

## License

MIT License. See `LICENSE` for details.
