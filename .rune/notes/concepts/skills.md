---
title: "Rune Skills System"
type: "concept-node"
tags: ["skills", "architecture", "ai", "notes"]
last_updated: "2025-02-21"
---

# Rune Skills System

Rune supports a lightweight, zero-bloat **Skills System** inspired by custom agent skills (such as those in Claude Code).

## Architecture & Design

1. **Storage**: Skills are stored as Markdown files with YAML frontmatter inside `.rune/notes/skills/` (e.g., `.rune/notes/skills/release-verification.md`).
2. **Context Efficiency (Zero Bloat)**:
   - At startup, Rune dynamically scans `.rune/notes/skills/` and extracts only the titles and descriptions from the frontmatter.
   - A compact summary index is injected into the system prompt (~15-30 tokens total).
3. **On-Demand Loading**:
   - When the agent or user identifies a task matching a skill, the agent lazy-loads the full skill instructions using existing tools like `read_rune_note` or `search_notes`.
4. **Examples**:
   - `skills/release-verification.md`
   - `skills/engine-audit.md`
