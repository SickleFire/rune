---
title: "Skill: Game Engine Bridge Audit"
type: "skill"
tags: ["skill", "godot", "unity", "game-engines"]
description: "Audits Godot and Unity C# bridge scripts and C#/.gd plugin configurations."
---

# Skill: Game Engine Bridge Audit

When tasked with checking game engine integrations:
1. Inspect `godot/RuneBridge.cs`, `godot/RuneBridge.gd`, and `godot/RuneBridgePlugin.cs` for correct lifecycle methods.
2. Verify Unity C# script `unity/RuneBridge.cs` has necessary Editor attributes (`[InitializeOnLoad]` or `[ExecuteAlways]`).
3. Check `plugin.cfg` config metadata for Godot plugin correctness.
