---
type: decision
date: 2026-10-28
status: adopted
tags: [turts, m3, views, interface]
---

# M3 facing retention

Orchestrator ruling before T13a: retain the optional facing declared by map placements and belt lines during authoritative boot, then expose it in the sim snapshot DTO before T13b's UnitSnapshot.facing consumer is frozen. Root traced boot_m3 using facing only for endpoint/geometry resolution; no facing survives in world.rs UnitState.

This supplies data already required by the approved protocol/client plan. Do not infer it from neighboring pieces or surviving endpoints, and do not invent a default orientation. Synthetic/production paths with no facing remain None. Preserve M2 None serialization compatibility and update internal struct literals explicitly.

T13a may propose narrow world/boot/snapshot patches in addition to its view files; root composes them. Mod schema is already sufficient, and protocol changes remain T13b. If this placement is wrong, rework is the private storage and internal snapshot constructors; the eventual wire facing field is required either way.
