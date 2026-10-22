---
title: "Source Modules (`src/`)"
type: "architecture-node"
tags: ["src", "modules", "rust", "tools", "agent"]
last_updated: "2025-02-20"
---

# Source Modules (`src/`)

This directory contains the core Rust implementation of Rune. Below is an inventory of the modules and their responsibilities:

## Core Engine & Orchestration
- **`lib.rs`**: Library root, module declarations, and public re-exports.
- **`main.rs`**: Application entry point, CLI parsing, and interactive loop / command dispatch.
- **`config.rs`**: Configuration loading and parsing (`rune.toml`, API keys, model parameters).
- **`agent.rs`**: Core agent logic, conversation state machine, planning, tool execution loops, and prompt construction.
- **`api.rs`**: API client implementations for interacting with LLM providers (e.g. Gemini, OpenAI, etc.) and handling tool call declarations / responses.

## Tool System & Specialized Integrations
- **`tools.rs`**: Central tool registry, definitions, and execution dispatcher trait (`AgentTool`).
- **`memory.rs` & `memory_tools.rs`**: Persistent storage mechanism (`memory.json`) and tools for recalling past fixes, storing preferences, and recording file relationships.
- **`notes_tools.rs`**: Tools enabling the assistant to list, read, and upsert Rune Notes in `.rune/notes/`.
- **`github_tools.rs`**: GitHub integrations (issues, PRs, workflow checks).
- **`docker_tools.rs`**: Docker container management and inspection tools.
- **`mysql_tools.rs`**: MySQL database connectivity and query tools.
- **`godot_tools.rs` & `unity_tools.rs`**: Game engine integration helpers for Godot and Unity projects.
- **`web_tools.rs`**: Web scraping and HTTP request tools.
- **`memory_tests.rs`**: Unit tests for memory and state tracking.
- **Testing & QA**: Comprehensive unit tests covering configuration parsing, path security guardrails, file editing/patching, and integration tools (see [[../concepts/testing.md|Testing & QA]]).
