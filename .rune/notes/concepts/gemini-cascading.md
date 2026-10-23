--- Rune Note: concepts/gemini-cascading.md ---

---
title: "Gemini Model Cascading & Fallback"
type: "concept-node"
tags: ["gemini", "llm", "fallback", "cascading", "resilience"]
last_updated: "2025-02-20"
---

# Gemini Model Cascading & Fallback

Rune supports automatic cascading and fallback across multiple Gemini models to maximize reliability and uptime against rate limits (429), service unavailability (5xx), and quota limits.

## Overview
- **`GeminiCascadingProvider`**: Implements `LLMProvider` by maintaining an ordered sequence of Gemini models (e.g. `gemini-3.6-flash` -> `gemini-3.8-flash` -> `gemini-3.1-flash-lite`).
- **Automatic Fallback**: When a request fails with a rate limit, server error, or recoverable API exception, the provider catches the error, logs a warning, and attempts the completion with the next model in the cascade chain.
- **Configuration**: Configured via `rune.toml` or environment variables, allowing users to specify a primary model and a sequence of fallback models.

---
## Related Notes
- [[testing.md|Testing & Quality Assurance]]
- [[../src/overview.md|Source Modules Overview]]
