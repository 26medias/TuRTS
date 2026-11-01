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

## C162: reject fire policy on Attack definitions

Conflict: content_mc_validation.rs:1328 permits fire on Move, Hold and Attack, but design14 section5.2:488-498 lists fire only for Move/Hold and engage for Attack. combat.rs:151-160 correctly uses fire only for Move/Hold, so a formerly accepted Attack.fire value would be ignored.

Choice: restrict authored fire to Move/Hold. Reject Attack.fire Some(Never) and Some(InRange) with package/file/field fire; keep Attack.fire None and its required approach/fire mechanism unchanged. Add compiled red-then-green negative validation regression plus positive controls. Shipped Forge/Muster Attack definitions already have fire None; no content, frozen value, design or acceptance criterion changes. This repairs an instrument/schema mismatch under brief4.

Evidence: design5.2:488-498, crates/turts_mod_runtime/src/content_mc_validation.rs:1328-1334, crates/turts_sim/src/economy/combat.rs:151-160, mods/muster/orders/order_attack.ron:1. Reviewer RUN65 raised the edge; builder RUN64 owns the repair and all final gates. Alternative not taken: invent Attack.fire behaviour, silently ignore accepted policy, or weaken audit/validation. Actual new regression is NOT TRACED until builder runs it.14/25complete;65/170runs, no new dispatch.
