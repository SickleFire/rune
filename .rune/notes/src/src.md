---
title: "Source File Reference (`src/`)"
type: "reference-node"
tags: ["src", "rust", "files", "reference"]
last_updated: "2025-02-20"
---

# Source File Reference (`src/`)

Detailed breakdown of each source file in `src/`:

- `@src/main.rs`: Entry point (`fn main()`), CLI parsing, and bootstrapping.
- `@src/lib.rs`: Library crate root, exposing modules (`agent`, `api`, `tools`, etc.).
- `@src/agent.rs`: Orchestration of LLM reasoning cycles, multi-turn tool calling, and response synthesis.
- `@src/api.rs`: Structs and methods for API requests to LLMs and tool declaration schemas.
- `@src/config.rs`: Configuration structs (`Config`) loading from environment variables and `rune.toml`.
- `@src/tools.rs`: Tool dispatcher and `AgentTool` trait definitions.
- `@src/memory.rs` & `@src/memory_tools.rs`: Storage logic and tools for persistent cross-session memory (`memory.json`).
- `@src/memory_tests.rs`: Tests covering memory persistence and retrieval.
- `@src/notes_tools.rs`: Tools for programmatic reading, listing, and writing of Rune Notes in `.rune/notes/`.
- `@src/github_tools.rs`: GitHub tool implementations.
- `@src/docker_tools.rs`: Docker container and image management tools.
- `@src/mysql_tools.rs`: MySQL database query and inspection tools.
- `@src/godot_tools.rs` & `@src/unity_tools.rs`: Game engine integration utilities.
- `@src/web_tools.rs`: Web requests and parsing.
