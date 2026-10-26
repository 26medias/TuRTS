---
type: decision
date: 2026-10-26
tags:
  - turts
  - decision
  - mods
  - m1
ai-first: true
source: mcp
status: active
---

## For future Claude
M1 drops deviation DEV-M1-01. The client downloads packages from the lockfile URLs and checks their hash; units are placed by package map files, never by engine code. Studio call after gate 1.

**Who:** orchestrator, on findings from mod-builder and mod-runtime-engineer at M1 gate 1 (both found it independently; mod-runtime-engineer showed with a probe that the old T2 stayed green on a privileged build).
**Why:** the earlier plan let the engine hard-code the default unit's placement and let only the default package ride inside the client build. Both are default-mod privileges that spec invariant 1 and the M1 anti-goals forbid. Downloading from the lockfile URL is a plain HTTP GET; with no cache and no browser storage it costs little and honours the Decided 6.5 user experience.
**Supersedes:** the DEV-M1-01 part of [[Decisions/2026-10-26 M1 mod package v0]].
**Test:** T2a starts the same server binary with the default package and with a third-party fixture; each must spawn its own units. A grep over `crates/` must find no package name or content id.
