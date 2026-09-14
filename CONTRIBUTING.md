# Contributing to Rune

We love your input! We want to make contributing to Rune as easy and transparent as possible. Whether it's submitting bug reports, proposing features, improving documentation, or contributing code, your help is greatly appreciated.

Please take a moment to review the guidelines below.

---

## Table of Contents
1. [Code of Conduct](#code-of-conduct)
2. [Prerequisites & Getting Started](#prerequisites--getting-started)
3. [Development Setup & Local Running](#development-setup--local-running)
4. [Testing & Quality Assurance](#testing--quality-assurance)
5. [Code Style & Linting](#code-style--linting)
6. [Commit Guidelines](#commit-guidelines)
7. [Pull Request Process](#pull_request_process)
8. [Reporting Issues & Feature Requests](#reporting-issues--feature-requests)
9. [Getting Help](#getting-help)

---

## Code of Conduct

By participating in this project, you agree to abide by our standards of openness, respect, and consideration. Please report unacceptable behavior to the repository maintainers.

---

## Prerequisites & Getting Started

### Required Tools
- **Rust**: Stable toolchain (Edition 2024 / latest stable Rust recommended). Install via [rustup](https://rustup.rs/).
- **Cargo**: Included with Rust.
- **Git**: For version control.
- *(Optional)* **Unity 2022+** or **Godot 4.x** if working on engine integration bridges (`unity/` or `godot/`).

### Cloning the Repository
```bash
git clone https://github.com/SickleFire/rune.git
cd rune
```

---

## Development Setup & Local Running

1. **Build the Project**
   ```bash
   cargo build
   ```
   For release builds (optimized performance):
   ```bash
   cargo build --release
   ```

2. **Run Rune Locally**
   ```bash
   cargo run
   ```

3. **Running with Integration Flags**
   - Unity Bridge enabled:
     ```bash
     cargo run -- --unity
     ```
   - Godot Bridge enabled:
     ```bash
     cargo run -- --godot
     ```
   - GitHub integration enabled:
     ```bash
     cargo run -- --github
     ```

---

## Testing & Quality Assurance

Before submitting any code changes, ensure that all tests pass successfully and that new functionality is accompanied by appropriate unit or integration tests.

1. **Run Test Suite**
   ```bash
   cargo test
   ```

2. **Run Specific Tests**
   ```bash
   cargo test test_name_or_module
   ```

---

## Code Style & Linting

We strive to keep the codebase clean, idiomatic, and robust.

1. **Code Formatting**
   Always format your code using `rustfmt` before committing:
   ```bash
   cargo fmt
   ```

2. **Linting & Warnings**
   Run `clippy` to catch common mistakes and improve your Rust code:
   ```bash
   cargo clippy --all-targets --all-features
   ```
   Ensure your code compiles without warnings.

---

## Commit Guidelines

We follow clear, descriptive commit messages to maintain a clean git history.

- Use present tense ("Add feature" not "Added feature").
- Use imperative mood ("Move cursor to..." not "Moves cursor to...").
- Keep the first line concise (under 50 characters) followed by an empty line and a more detailed explanatory body if necessary.

---

## Pull Request Process

1. **Create a Feature Branch**
   ```bash
   git checkout -b feature/my-new-feature
   ```

2. **Implement Changes & Test**
   - Write clean, documented code.
   - Add/update tests where applicable.
   - Run `cargo fmt`, `cargo clippy`, and `cargo test`.

3. **Open a Pull Request**
   - Push your branch to GitHub:
     ```bash
     git push origin feature/my-new-feature
     ```
   - Open a Pull Request against the `main` branch.
   - Fill out the [Pull Request Template](.github/pull_request_template.md) detailing your changes, motivation, and testing steps.

---

## Reporting Issues & Feature Requests

If you encounter a bug or have a feature idea, please open an issue using one of our templates:
- **Bug Reports**: Provide steps to reproduce, expected vs actual behavior, and environment details.
- **Feature Requests**: Describe the proposed feature, use case, and potential implementation approach.

---

## Getting Help

If you have questions or need assistance, feel free to:
- Open an Issue or Discussion on GitHub.
- Reach out to repository maintainers (`SickleFire`).

Thank you for contributing to Rune!
