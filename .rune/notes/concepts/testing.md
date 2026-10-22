--- Rune Note: concepts/testing.md ---

---
title: "Testing & Quality Assurance"
type: "concept-node"
tags: ["testing", "unit-tests", "rust", "quality"]
last_updated: "2025-02-20"
---

# Testing & Quality Assurance

Rune maintains robust unit test coverage across its core modules and tool integrations to ensure stability, security (path guardrails, command sanitization), and correct LLM payload serialization.

## Test Suite Architecture
- **Memory & Storage Tests**: `src/memory_tests.rs` validates persistence, preference storage, and file relationship maps.
- **Config & Tool Tests**: Unit tests in `src/config.rs`, `src/tools.rs`, and various tool modules (`godot_tools.rs`, `docker_tools.rs`, `mysql_tools.rs`, `web_tools.rs`, `unity_tools.rs`, `github_tools.rs`) verify configuration loading, path sanitization guardrails, file reading ranges, and patching logic.
- **Test Execution**: Run `cargo test` to execute the full unit and integration test suite across the repository.

---
## Related Notes
- Related: [[../architecture.md|Architecture Overview]] -> [[../src/overview.md|Source Modules Overview]]
