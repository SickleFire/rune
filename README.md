# Rune

Rune is an expert AI coding assistant integrated into a software development workspace and Unity game development environment, built in Rust. It provides an interactive REPL/CLI interface, tool execution (file operations, command execution, git operations, code searching, Unity Editor integration, web utilities), and seamless API integration with LLM providers.

## Features

- **Interactive REPL & CLI**: Powered by `rustyline` with colorized output and prompt support.
- **Tool Execution Framework**: Rich set of built-in tools for workspace inspection, file reading/writing/patching, executing shell commands, and git operations.
- **Web Utility Tools**: Built-in HTTP request execution (`http_request`), web page scraping/fetching (`fetch_web_page`), and TCP port verification (`check_tcp_port`).
- **Docker Container Tools**: Optional Docker integration (`--docker` flag) supporting container listing (`docker_list_containers`), container logs (`docker_container_logs`), container starting/stopping (`docker_start_container`, `docker_stop_container`), image listing (`docker_list_images`), and container inspection (`docker_inspect_container`).
- **MySQL Database Tools**: Optional MySQL integration (`--mysql`) supporting listing tables (`mysql_list_tables`), describing table schemas (`mysql_describe_table`), and executing SQL queries (`mysql_execute_query`).
- **GitHub Integration**: Optional GitHub integration (`--github` flag or `GITHUB_TOKEN`) supporting reading issues & PR diffs, creating issues, posting comments, and opening pull requests.
- **Unity Editor Integration**: Optional Unity Bridge (`--unity` flag) supporting scene inspection, game object/component management, asset searching, reference validation, console log reading, and play mode control.
- **Godot Editor Integration**: Optional Godot Bridge (`--godot` flag) supporting active scene inspection (`godot_inspect_scene`), node property auditing (`godot_inspect_node_properties`), and node creation (`godot_create_node`).
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
- `src/docker_tools.rs`: Docker container tools for listing containers, viewing logs, starting/stopping containers, listing images, and inspecting container state.
- `src/mysql_tools.rs`: MySQL database tools for listing tables, describing table schemas, and executing queries against MySQL databases (such as XAMPP MySQL).
- `src/unity_tools.rs`: Unity Editor bridge tools for inspecting scenes, managing GameObjects/components, assets, and console logs.
- `src/godot_tools.rs`: Godot Editor bridge tools for inspecting active scene hierarchies, auditing node properties, and creating nodes.
- `unity/RuneBridge.cs`: C# Unity Editor integration script providing HTTP bridge endpoints for Rune's Unity tools.
- `godot/`: Godot C# plugin integration (`RuneBridge.cs`, `RuneBridgePlugin.cs`, `plugin.cfg`) providing HTTP bridge endpoints for Rune's Godot tools.

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

To run with Godot Editor bridge integration enabled:

```bash
cargo run -- --godot
```

To run with MySQL database tools enabled:

```bash
cargo run -- --mysql
```

To run with Docker container tools enabled:

```bash
cargo run -- --docker
```

Or pass prompts directly via CLI arguments for quick tasks and automation scripts.

## Contributing

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to contribute to this project.

## License

MIT License. See `LICENSE` for details.
