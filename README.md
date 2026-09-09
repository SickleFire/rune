# Rune

Rune is an expert AI coding assistant integrated into a software development workspace, built in Rust. It provides an interactive REPL/CLI interface, tool execution (file operations, command execution, git operations, code searching), and seamless API integration with LLM providers.

## Features

- **Interactive REPL & CLI**: Powered by `rustyline` with colorized output and prompt support.
- **Tool Execution Framework**: Rich set of built-in tools for workspace inspection, file reading/writing/patching, executing shell commands, and git operations.
- **Async API Client**: Robust integration with LLM APIs supporting tool calling and streaming responses.
- **Smart Context**: Global awareness of repository architecture and file systems.
- **Code Search Engine (cix)**: Fast symbol and pattern searching across the repository.
- **Checkpointing & Version Control**: Git integration for safety checks, stashing, and rolling back mutations.

## Project Structure

- `src/main.rs`: Entry point for the CLI and REPL loop, handling user input, history, and command dispatch.
- `src/lib.rs`: Core library initialization, agent orchestrator, and shared modules.
- `src/api.rs`: API communication layer with LLM endpoints, request payload formatting, and response parsing.
- `src/tools.rs`: Tool definitions, execution logic, workspace security validations, and utility functions.

## Getting Started

### Prerequisites

- Rust (edition 2024 / stable toolchain)
- Cargo

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

Or pass prompts directly via CLI arguments for quick tasks and automation scripts.

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

MIT License. See `LICENSE` for details.
