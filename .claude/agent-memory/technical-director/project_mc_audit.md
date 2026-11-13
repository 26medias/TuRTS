---
name: mc-architecture-audit-facts
description: Verified 2026-11-11 facts from the MC whole-milestone architecture audit (RUN126) at integration 74055cb: grid max-radius probe, dual move paths, economylog second model, unit_orders full table, fault build profile
metadata:
  type: project
---

Verified 2026-11-11 in /home/julien/Projects/TuRTS-mc-integration at 74055cb (RUN126):
- boundary.sh exit 0, includes 4 turts_interaction rows. turts_pack -> turts_interaction is dev-only.
- Grid cell = 2 x max radius of ALL units incl. static (movement.rs:1402-1403); combat and
  engagement cells = max weapon range in the catalog (combat.rs:111-117, orders.rs:722-727).
  Probe (scratch gridprobe, step_frame, plain movers in 1.1 grid): 1000 units 3.0 ms, +1 static
  r=2 3.7, r=4 8.4; 5000 units 16.7, 20.1, 56.2 ms per tick.
- Two move models: plain/legacy (PlainMoveProgress, apply_legacy_move_to reached from
  apply_order orders.rs:2848,2882) and pooled (MoveProgress, MovementPools). Cause: REG1 no
  re-freeze of frozen M2 stream (design :1271), K19 (a) (design :117).
- economylog.rs 19,082 lines, 8,084 before cfg(test); validate_pending_movement_dispatch
  (:4282) re-derives sim dispatch: a second model. Most-edited MC code file (16 commits).
- WorldDelta.unit_orders = full owner table every tick (export.rs:190-195, replica.rs:523-525).
- validate_target scans the whole world per unit for Unit targets (orders.rs:3056,3085), called
  per unit at orders.rs:2734. 78 iter_entities() calls in sim src, no UnitId->Entity index.
- CORRECTION 2026-11-11: version table HAS `weapon with movement` since 0.2.3 (version_table.rs:52-56).
- wasm-release profile: lto fat, codegen-units 1, opt z (Cargo.toml:37-43); index.html:6 wasm-opt z.
- CLAUDE.md (integration) still says ENGINE_API_VERSION 0.2.2 and has no turts_interaction row.

**Why:** MP1, M4, M7 gates will cite these debts.
**How to apply:** Re-check line numbers; a fix may land between MC merge and MP1.
