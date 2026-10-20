---
title: "Game Engine Bridges (Godot & Unity)"
type: "concept-node"
tags: ["godot", "unity", "integration"]
last_updated: "2025-02-20"
---

# Game Engine Integration Bridges

Rune provides bidirectional bridges and plugins for game development engines:

## Godot (`godot/`)
- **`RuneBridge.cs`**: C# bridge implementation for Godot.
- **`RuneBridge.gd`**: GDScript counterpart for Godot integration.
- **`RuneBridgePlugin.cs`**: Godot editor plugin for Rune.
- **`plugin.cfg`**: Godot plugin configuration metadata.
- *Reference:* See [[godot/godot.md|Godot Integration Reference]].

## Unity (`unity/`)
- **`RuneBridge.cs`**: Unity C# script / bridge component.
- *Reference:* See [[unity/unity.md|Unity Integration Reference]].

---
## Related Notes
- See main [[architecture.md|Architecture & Core Modules]] for backend integration tools (`godot_tools.rs`, `unity_tools.rs`).
