# Rune

Rune is an expert AI coding assistant integrated into a software development workspace, built in Rust. It provides an interactive REPL/CLI interface, tool execution (file operations, command execution, git operations, code searching), and seamless API integration with LLM providers.

## Features

- **Interactive REPL & CLI**: Powered by `rustyline` with colorized output and prompt support.
- **Tool Execution Framework**: Rich set of built-in tools for workspace inspection, file reading/writing/patching, executing shell commands, and git operations.
- **Async API Client**: Robust integration with LLM APIs supporting tool calling and streaming responses.
- **Smart Context**: Global awareness of repository architecture and file systems.

## Project Structure

- `src/main.rs`: Entry point for the CLI and REPL loop.
- `src/lib.rs`: Core library initialization and shared modules.
- `src/api.rs`: API communication layer with LLM endpoints.
- `src/tools.rs`: Tool definitions, execution logic, and workspace utilities.

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
