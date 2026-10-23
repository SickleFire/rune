---
title: "Architecture & Core Modules"
type: "concept-node"
tags: ["architecture", "modules", "src"]
last_updated: "2025-02-20"
---

# Architecture & Core Modules

This note maps out the source files in `src/` and their roles in the Rune assistant ecosystem:

- **`src/main.rs` & `src/lib.rs`**: Entry point and library exports.
- **`src/agent.rs`**: Multi-agent orchestration (Architect, Coder, planning and review loops).
- **`src/api.rs`**: LLM API client definitions and tool declaration models.
- **`src/config.rs`**: Configuration handling (`rune.toml`, environment variables).
- **`src/tools.rs`**: Core tool registry and trait definitions (`AgentTool`).
- **`src/memory.rs` & `src/memory_tools.rs`**: Persistent storage for agent preferences, code solutions, and file relationship maps.
- **`src/notes_tools.rs`**: Rune Notes management and graph recall (`list_rune_notes`, `read_rune_note`, `upsert_rune_note`, `search_notes`, `recall_related_notes`).
- **Testing**: Comprehensive test suite across configuration, tool guardrails, and integrations (see [[concepts/testing.md|Testing & QA]]).
- **Integration Tools**:
  - `github_tools.rs`
  - `docker_tools.rs`
  - `mysql_tools.rs`
  - `godot_tools.rs` (see [[godot/godot.md|Godot Integration Reference]]) & `unity_tools.rs` (see [[unity/unity.md|Unity Integration Reference]])
  - `web_tools.rs`

---
## Related Notes
- See [[src/overview.md|Source Modules overview]] for detailed module breakdown.
- See [[game_engines.md|Game Engines overview]] for cross-engine bridge architecture.
