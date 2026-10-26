---
title: "Skill: Release Build & Verification"
type: "skill"
tags: ["skill", "release", "ci"]
description: "Builds the release binary, runs workspace tests, and checks installation script syntax."
---

# Skill: Release Build & Verification

When invoked for release preparation or deep verification:
1. Run `cargo check --workspace` to ensure clean compilation.
2. Run `cargo test --workspace` to ensure all tests pass.
3. Check `install.ps1` and `install.sh` for syntax correctness or missing environment references.
4. Verify workspace version consistency in `Cargo.toml`.
