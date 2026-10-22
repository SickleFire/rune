---
title: "CI/CD Workflows"
type: "concept-node"
tags: ["ci", "github", "workflows", "actions"]
last_updated: "2025-02-20"
---

# CI/CD Workflows (`.github/workflows/`)

Rune uses GitHub Actions for continuous integration and release automation:

- **`ci.yml`**: Runs on push and pull request events to `main` and `master` branches across Ubuntu, macOS, and Windows (`matrix.os: [ubuntu-latest, macos-latest, windows-latest]`).
  - Checks formatting with `cargo fmt -- --check`.
  - Runs all unit/integration tests with `cargo test --all-targets --all-features`.
  - Installs and builds release binaries (`cargo build --release`).
- **`release.yml`**: Handles automated release tagging and artifact publishing.

Parent: [[overview.md|Rune Repository Overview]]
Related: [[src/overview.md|Source Structure]] -> [[src/src.md|Source File Reference]]
