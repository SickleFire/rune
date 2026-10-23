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
- **Priority Rule:** When starting a task, Rune should check relevant rune-notes before diving into raw source files. For a specific task, use `search_notes` with targeted keywords first; for a broad or unfamiliar topic, start with `read_rune_note` on `overview.md` when available. This provides architectural context and historical decisions efficiently.
- **Reference-Only Traversal:** Treat `.rune/notes/` as a directed graph. Search first for specific tasks; start from `overview.md` for broad or unfamiliar topics. Use `recall_related_notes` to surface graph- and metadata-related neighbors, and read only the relevant results rather than every note or a full-store listing.
- **Targeted Search:** Use `search_notes` with task-specific keywords to locate relevant notes for specific tasks. Search results identify candidates; follow the graph from a relevant result rather than opening unrelated notes.
- **Related Note Recall:** Use `recall_related_notes` with a note path to get a prioritized list of neighboring notes. Recall combines inbound and outbound wiki links, shared frontmatter tags, keyword overlap, and file relationships recorded in Rune memory. Results include a score and match reasons; use `format: "json"` when structured output is useful.
- **Cross-Linking:** Every note should link to its parent or overview and to relevant child or related notes using Wiki-style markdown links (`[[relative/path/to/note.md|Display Name]]`). Add links when creating or updating notes so the knowledge graph remains navigable; avoid unrelated links. For example: `Related: [[architecture.md|Architecture Overview]] -> [[src/overview.md|Source Structure]]`.
- Create or update notes whenever architectural decisions, recurring bug patterns, or complex subsystem APIs are established or modified.
