---
type: decision
date: 2026-11-01
tags: [turts, mc, codex, rulings]
ai-first: true
source: codex
---

# MC Codex rulings

## C161: share two ring topologies without per-unit assets

Conflict: design9.1:864 says one shared ring mesh, while9.1:840-843 requires a solid own ring and a foreign ring with four gaps. The prepared Mesh3d/StandardMaterial renderer represents gaps in geometry; one fixed topology cannot represent both with the same per-slot material. Candidate visuals.rs:28-31,190-191 holds two handles and creates each once.

Choice: use two global shared ring meshes, one solid and one with four gaps, and one global shared bar mesh. Keep one unlit material per slot, no per-unit mesh/material creation, and identical visible geometry. This is the nearest existing render mechanism with the same player-visible shape under brief4. Add a native asset-sharing assertion; actual browser performance is NOT TRACED until VIS2. The design is not edited.

Alternative not taken: lose the foreign gaps, build per-unit assets, add a custom shader/material system only to encode the gap topology, or claim the candidate uses one topology. O13 palette/thresholds are unchanged.14/25complete;65/170runs, no new dispatch.
