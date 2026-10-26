---
name: rune-skill-creator
description: Create, validate, evaluate, and package custom skills specifically designed for the Rune coding harness and Rune's native tools, commands, and .rune/notes/ knowledge base. Use whenever a user wants to build a new Rune skill from scratch, test its integration with Rune tools, validate frontmatter schemas, or optimize skill triggers for Rune.
compatibility: Requires Rune CLI harness, Python 3.x for validator/eval scripts, and .rune/notes/ structure.
---

# Rune Skill Creator

A specialized skill-creation, validation, and benchmarking harness tailored specifically for **Rune** and its ecosystem.

Use this skill whenever you or a user want to:
- Build a brand new custom skill for Rune.
- Integrate Rune-native tools (`search_code`, `patch_file`, `exec_command`, `upsert_rune_note`, etc.).
- Validate SKILL.md frontmatter, metadata, and directory structure against Rune's conventions using the built-in validator script.
- Evaluate and test skill execution, description trigger quality, and adherence using the evaluation script.

---

## Rune Skill Architecture & Anatomy

A Rune skill lives inside `.rune/notes/skills/<skill-name>/` (or as a flat `.md` file) and follows progressive disclosure principles:

```
.rune/notes/skills/<skill-name>/
├── SKILL.md (Required)
│   ├── YAML Frontmatter (`name`, `description`, `compatibility`, `tags`)
│   └── Markdown Instructions & Workflow steps
└── scripts/ (Optional)
    ├── validate_skill.py - Standalone validator for frontmatter, naming, and structure
    └── eval_skill.py     - Evaluates skill triggers and instruction clarity against test queries
```

---

## Bundled Scripts & Tools

### 1. Skill Validator (`scripts/validate_skill.py`)
Validates any Rune skill folder or file for:
- Valid YAML frontmatter (`name`, `description`).
- Kebab-case naming conventions.
- Presence of required documentation sections and workflow steps.
- Absence of malformed markdown.

**Usage:**
```bash
python .rune/notes/skills/rune-skill-creator/scripts/validate_skill.py .rune/notes/skills/<skill-name>
```

### 2. Skill Evaluator (`scripts/eval_skill.py`)
Evaluates a skill's description quality against test queries to verify whether Rune's dynamic skill loader will trigger the skill appropriately, and tests script executability.

**Usage:**
```bash
python .rune/notes/skills/rune-skill-creator/scripts/eval_skill.py .rune/notes/skills/<skill-name>
```

---

## Step-by-Step Workflow for Creating a Rune Skill

### 1. Capture Intent & Scope
Understand what the skill should enable Rune to do:
- **Trigger context**: What user prompts or workspace states should trigger this skill? (Make descriptions slightly "pushy" to ensure reliable triggering).
- **Tool integration**: Which Rune tools (`search_code`, `patch_file`, `upsert_rune_note`, etc.) will the skill leverage?

### 2. Draft `SKILL.md`
Create the skill directory at `.rune/notes/skills/<skill-name>/SKILL.md` with required frontmatter:

```yaml
---
name: my-rune-skill
description: Brief summary of what the skill does and exact contextual triggers for when Rune should activate it.
compatibility: Required Rune tools or dependencies
---

# Title of Skill

## Workflow
1. Step one...
2. Step two...
```

### 3. Validate & Evaluate
Run validator and evaluator scripts:
```bash
python .rune/notes/skills/rune-skill-creator/scripts/validate_skill.py .rune/notes/skills/my-rune-skill
python .rune/notes/skills/rune-skill-creator/scripts/eval_skill.py .rune/notes/skills/my-rune-skill
```
