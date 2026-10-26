---
title: "Rune Skills System"
type: "concept-node"
tags: ["skills", "architecture", "ai", "notes"]
last_updated: "2025-02-21"
---

# Rune Skills System

Rune supports a lightweight, zero-bloat **Skills System** inspired by custom agent skills (such as those in Claude Code).

## Architecture & Design

1. **Storage**: Skills are stored as Markdown files with YAML frontmatter inside `.rune/notes/skills/` (e.g., `.rune/notes/skills/release-verification.md`) or as dedicated skill folders containing a `SKILL.md` and optional companion scripts (supporting arbitrary nested subfolders):
   ```text
   .rune/notes/skills/
   ├── release-verification.md
   └── complex-skill/
       ├── SKILL.md
       ├── setup.ps1
       └── scripts/
           ├── benchmark.py
           └── internal/
               └── helpers.py
   ```
2. **Context Efficiency (Zero Bloat)**:
   - At startup, Rune dynamically scans `.rune/notes/skills/` recursively, extracting titles, descriptions, and automatically listing all accompanying companion scripts with their relative paths.
   - A compact summary index is injected into the system prompt (~15-30 tokens total).
3. **On-Demand Loading & Execution**:
   - When the agent or user identifies a task matching a skill, the agent lazy-loads the full skill instructions using `read_rune_note`.
   - The agent can inspect and run any nested companion scripts using `execute_commands`.
4. **Examples**:
   - `skills/release-verification.md`
   - `skills/engine-audit.md`
