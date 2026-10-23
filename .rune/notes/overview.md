---
title: "Rune Repository Overview"
type: "concept-node"
tags: ["architecture", "overview", "rune-notes"]
last_updated: "2025-02-20"
---

# Rune Repository Overview

Rune is an expert AI coding assistant integration framework written in Rust.

## Rune Notes Conventions & Initialization
- **Root Store:** `.rune/notes/`
- **File Naming Conventions:**
  - File-associated notes mirror the repo structure: e.g., `.rune/notes/src/api.rs.md`
  - Standalone concept/idea nodes: e.g., `.rune/notes/concepts/godot-integration.md` or `.rune/notes/architecture.md`
- **Agent Workflow:** When exploring unfamiliar codebase sections or tackling complex tasks, Rune checks and reads relevant rune notes in `.rune/notes/` first to maintain context across sessions without bloating the active token window.

## Key Subsystems
- **Agent Orchestration (`src/agent.rs`)**: Multi-agent coordination (Architect + Coder roles).
- **Tool Ecosystem (`src/tools.rs`, `src/*_tools.rs`)**: Extensible tools for GitHub, Docker, MySQL, Unity, Godot, Web search, and now **Rune Notes** (`.rune/notes/`).
- **Memory Subsystem (`src/memory.rs`, `src/memory_tools.rs`)**: Persistent JSON memory store for preferences, past fixes, and file relationships.
- **Rune Notes (`.rune/notes/`)**: Markdown-based knowledge nodes allowing Rune to read, save, and navigate contextual ideas and file-specific insights across sessions without bloating token windows.
- **Related note recall:** `recall_related_notes` ranks neighboring notes from wiki links, frontmatter tags, keyword overlap, and file relationships stored in `.rune/memory.json`. It returns paths, relevance scores, and match reasons, with optional JSON output.
