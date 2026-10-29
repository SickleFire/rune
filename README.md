# Rune

[![CI](https://github.com/SickleFire/rune/actions/workflows/ci.yml/badge.svg)](https://github.com/SickleFire/rune/actions/workflows/ci.yml)
[![Release](https://github.com/SickleFire/rune/actions/workflows/release.yml/badge.svg)](https://github.com/SickleFire/rune/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE.md)
[![XERJ Corpus Hub contributor](https://hub.xerj.org/contributors/SickleFire/badge.svg)](https://hub.xerj.org/contributors/SickleFire)

Rune is an expert AI coding assistant integrated into a software development workspace, Unity, and Godot game development environments, built in Rust. It provides an interactive REPL/CLI interface, tool execution (file operations, command execution, git operations, code searching, Unity and Godot Editor integration, web utilities, persistent memory, and Rune Notes knowledge graph), and seamless API integration with LLM providers.

## Features

- **Interactive REPL & CLI**: Powered by `rustyline` with colorized output and prompt support.
- **Tool Execution Framework**: Rich set of built-in tools for workspace inspection, file reading/writing/patching, executing shell commands, and git operations.
- **Rune Notes Knowledge Graph**: Persistent Markdown knowledge nodes in `.rune/notes/` featuring Wiki-style cross-linking (`[[note.md]]`) and architectural awareness.
- **Persistent Memory**: Storage for agent preferences, code solutions, and file relationship maps (`src/memory.rs`).
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
- **Python Automations**: Run user-authored Python scripts at startup or on demand, with script approvals, execution timeouts, bounded output, and optional agent context handoff.

## Project Structure

- `src/main.rs`: Entry point for the CLI and REPL loop, handling user input, history, and command dispatch.
- `src/automation.rs`: Python automation configuration, approval, execution, output capture, and context handoff.
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

### Installation

You can install Rune instantly using our automated installer scripts without needing to build from source via Cargo:

#### macOS & Linux
```bash
curl -sSL https://raw.githubusercontent.com/SickleFire/rune/main/install.sh | bash
```

#### Windows (PowerShell)
```powershell
iex (iwr -useb https://raw.githubusercontent.com/SickleFire/rune/main/install.ps1)
```

#### Install from crates.io
If you have Rust and Cargo installed, you can install the `rune-code` package from crates.io. The installed command is still `rune`:

```bash
cargo install rune-code
```

### Configuration & Environment Management

Rune supports configuring API keys, models, and local endpoints via a streamlined `rune.toml` configuration file in your project root or home directory, with full backward compatibility for environment variables (`GEMINI_API_KEY`, `OPENAI_API_KEY`, `GROQ_API_KEY`, `OPENROUTER_API_KEY`, `ANTHROPIC_API_KEY`, `RUNE_*`). Groq and OpenRouter use their OpenAI-compatible chat-completions APIs.
To use OpenRouter, set `provider = "openrouter"` and provide `openrouter_api_key` and `openrouter_model` in `rune.toml`, or set `OPENROUTER_API_KEY` and optionally `OPENROUTER_MODEL` in the environment.

Gemini requests are paced to a minimum interval of 1 second by default. Set
`gemini_min_request_interval_ms` in `rune.toml` (or `RUNE__GEMINI_MIN_REQUEST_INTERVAL_MS`)
to adjust it. Rune estimates input tokens from the serialized request and reports a rolling
60-second estimate; to enforce a TPM budget, set `gemini_max_input_tokens_per_minute` (or
`RUNE__GEMINI_MAX_INPUT_TOKENS_PER_MINUTE`) to a value below your model/account quota. This
estimate is not Google's tokenizer, and TPM quotas vary by model and account tier. The limiter
is shared by requests using the same Gemini provider instance, including retries and tool loops.

To keep sensitive workspace files out of Rune's built-in read-only file tools, configure
`excluded_read_paths` in `rune.toml`. For example:

```toml
excluded_read_paths = [".env", ".env.*", "*.pem", "secrets/**"]
```

Patterns are case-insensitive; `*` and `?` are wildcards, and a pattern without `/` matches
any path component. Exclusions apply to file reads, listings, code search, `@file` context,
the project tree, and Git diffs. They block reads only; write and patch tools remain available.
This is not a sandbox: shell commands and external or custom tools can still access excluded
files.

1. Copy the sample configuration file (or run the setup script):
   ```bash
   cp rune.toml.example rune.toml
   ```
2. Edit `rune.toml` to specify your preferred provider, API keys, models (such as `gpt-5.6-luna`, `gemini-3.5-flash-lite`, or local models via Ollama / LM Studio).

### Usage

Run Rune in your workspace:

```bash
rune
```

To run with Unity Editor bridge integration enabled:

```bash
rune --unity
```

To run with Godot Editor bridge integration enabled:

```bash
rune --godot
```

To run with MySQL database tools enabled:

```bash
rune --mysql
```

To run with Docker container tools enabled:

```bash
rune --docker
```

Or pass prompts directly via CLI arguments for quick tasks and automation scripts.

### Automations

Rune can run optional Python automations without making an LLM request. Add scripts as
editable `.py` files and configure them in `rune.toml`:

```toml
python_interpreter = "python"
automation_timeout_seconds = 30
automation_max_output_bytes = 8192

[[automations]]
name = "morning-brief"
script = ".rune/automations/morning_brief.py"
enabled = true
trigger = "startup" # "manual", "interval", or "scheduled" are also supported
```

An interval automation runs repeatedly while Rune is open. Its first run is after the
configured interval:

```toml
[[automations]]
name = "refresh-status"
script = ".rune/automations/refresh_status.py"
enabled = true
trigger = "interval"
interval_seconds = 1800
```

A scheduled automation runs once per matching local time each day while Rune is open:

```toml
[[automations]]
name = "daily-brief"
script = ".rune/automations/morning_brief.py"
enabled = true
trigger = "scheduled"
schedule_times = ["08:30", "13:00"] # 24-hour local time
```

Set `python_interpreter` to the Python executable on your system (for example, `python3`
on macOS/Linux or `py` on Windows). Set `python_args = ["-3"]` if your Windows Python
launcher requires an explicit version. Rune does not bundle Python or install packages;
automations run with the configured interpreter and its installed dependencies.

On first use, inspect the script and explicitly approve its current contents in Rune:

```text
/automation approve morning-brief
```

Changed scripts require approval again. `/automations` lists configured automations,
`/automation run morning-brief` runs an enabled automation on demand, and
`/automation revoke morning-brief` removes its approval. Startup automations run before
the prompt only when enabled and approved. By default, output is printed in the terminal
and is not sent to the agent, so it adds no tokens to later LLM requests. If you want Rune
to interpret an automation result, run it with `/automation run morning-brief --context`;
Rune will include that output in the next agent request only, where it uses input tokens.
Output is capped per stream and execution is stopped at the configured timeout. Failed
runs are reported and their output is not added to agent context.

By default, interval and scheduled triggers run only while the Rune process is open.
On Windows, install an automation in Task Scheduler to keep it running while Rune is
closed:

```text
/automation install-schedule morning-brief
```

This registers a task for the current Windows user without requiring administrator
privileges or storing a password. It runs only while that user is logged on; Rune itself
does not need to be open. Task Scheduler starts missed runs when available after sleep or
shutdown. Use `/automation uninstall-schedule morning-brief` to remove the OS task and
resume in-process scheduling while Rune is open. `/automations` shows whether an OS
schedule is installed. Reinstall after changing the trigger settings or moving Rune's
executable. The task runs with your logged-on account; if you sign out, it waits until you
log back in. Scheduled-task output is not shown in the REPL, so have unattended scripts
write their results to an appropriate file or notification if you need to review them.

Interval runs wait the configured number of seconds before the first run and after each
completion. Windows Task Scheduler installation requires intervals to be whole minutes
(at least 60 seconds); the in-process scheduler also supports shorter intervals. Scheduled
times use the computer's local clock and 24-hour `HH:MM` format. Task Scheduler prevents
overlapping instances of the same automation. OS-scheduled runs invoke Rune's
`--automation-run` command, which runs the script and exits without initializing an LLM
provider or starting the REPL. Output is not sent to an LLM.

Automations are not sandboxed: Python code has the permissions of the Rune process,
including access to local files, credentials, and network resources. Approval is bound to
the exact script contents, not a guarantee that the script is safe. Review scripts before
approving them, and install any required dependencies yourself.

## Contributing

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines on how to contribute to this project.

## License

MIT License. See `LICENSE` for details.
