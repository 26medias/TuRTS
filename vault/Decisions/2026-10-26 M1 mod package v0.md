---
type: decision
date: 2026-10-26
tags:
  - turts
  - decision
  - mods
ai-first: true
source: mcp
status: proposed
---

## For future Claude
M1 mod package: RON manifest with the nine 5.5 fields (placeholder formats), one declarative unit, loaded by the server through the same public `register_unit_type` path a third-party package uses. Proposed by mod-runtime-engineer (Opus) with a passing probe; studio call pending gate 1.

**Format:** RON for manifest and unit definitions (bevy_asset 0.20 already depends on ron 0.12; enums and error messages are good). JSON for the lockfile, a protocol document.
**Manifest (placeholders marked):** `id "m1test"`, `version "0.1.0"`, `kind Game`, `engine_api "^0.1"`, `dependencies {}`, `exports (units: [...])`, `context Shared`, `capabilities []`, `license "LicenseRef-TBD"`.
**Unit:** `m1test:rifleman`, speed 3.0 world units per second (engine converts per tick), `Circle(radius: 0.4)`, visual `Primitive(Capsule(0.3, 1.8))`. Engine owns navigation, collision, integrator, registry; mod owns speed, footprint, name, visual (spec 4.3). No script, no per-tick hook.
**Server load path:** read archive, hash it (sha256 placeholder), check `engine_api`, reject non-empty dependencies at M1, write the lockfile entry (seven fields), parse and validate each unit, call `register_unit_type`.
**Probe:** eight tests pass on native; three go red under a mutant; serde, ron, toml, json, sha2, semver, tar all build for wasm32.
**Deviation DEV-M1-01 (studio call):** the client build embeds the same package archive; the join handshake carries the lockfile entry; the client hashes the embedded bytes and refuses the join on mismatch. No download, cache, or browser storage at M1. Retire at M8. Keeps invariant 10 testable from M1.
**Open:** license of the default mod (CEO; TBD like the repo license).
