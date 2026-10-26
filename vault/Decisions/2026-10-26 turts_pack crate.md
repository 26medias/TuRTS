---
type: decision
date: 2026-10-26
tags:
  - turts
  - decision
  - architecture
ai-first: true
source: mcp
status: active
---

## For future Claude
`turts-pack` moved into its own crate `turts_pack`, which depends on `turts_sim`, so it can validate a package with the same rules the server applies before writing an archive. The server boot loads packages through `turts_pack::load_package`.

**Who:** the Part B implementer (Sonnet), accepted by the orchestrator.
**Why:** the map rules live in `turts_sim`; `turts_mod_runtime` and the client must not depend on `turts_sim` (boundary rules). A modder now sees every content error at pack time, with file, field, and allowed values, instead of at server start.
**Boundary:** `turts_pack` is a tool and server-side crate; the client never depends on it.
**Open (low):** `server.ron` still accepts unknown fields and its parse error names a Rust type; it is operator-facing, not modder-facing.
