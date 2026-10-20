---
title: "Rune Notes Conventions"
type: "concept-node"
tags: ["convention", "rune-notes", "guidelines"]
last_updated: "2025-02-20"
---

# Rune Notes Conventions

Rune Notes serve as a persistent, token-efficient knowledge base in `.rune/notes/` that allows Rune to retain context across sessions.

## 1. Directory & File Conventions
- **Root Store:** `.rune/notes/`
- **File Naming Conventions:**
  - File-associated notes mirror the repository structure: e.g., `.rune/notes/src/api.rs.md`.
  - Standalone concept/idea/architecture nodes: e.g., `.rune/notes/concepts/godot-integration.md` or `.rune/notes/architecture.md`.

## 2. Best Practices for Rune
- **Priority Rule:** When starting any task, Rune should **first check and read relevant rune-notes** in `.rune/notes/` via `list_rune_notes` and `read_rune_note` before diving into raw source code files. This provides high-level architectural context and historical decisions instantly in a token-efficient manner.
- **Cross-Linking:** Use Wiki-style markdown links (`[[relative/path/to/note.md|Display Name]]`) to connect related architectural modules, concepts, and file references together into a navigable knowledge graph (e.g., linking [[architecture.md|Architecture]] to [[game_engines.md|Game Engines]]).
- Create or update notes whenever architectural decisions, recurring bug patterns, or complex subsystem APIs are established or modified.
