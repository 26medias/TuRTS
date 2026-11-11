---
type: decision
date: 2026-11-11
status: open
tags: [turts, mc, architecture, ceo]
---

# MC architecture audit (RUN126) and two CEO questions

The technical-director audited the whole MC change (main bb611bd to codex/mc-integration 74055cb) on 2026-11-11, read-only, on the CEO concern "no deep architectural issue". Verdict: SOUND-WITH-DEBT. Root verified: the file sizes (economylog.rs 19,082 lines, `cfg(test)` at line 8084; orders.rs 7,036; movement.rs 3,873) and the version table content (version_table.rs:21-52). The other claims are the auditor's, with its path:line evidence in the Root session.

## Fine as it is

Server authority; crate boundaries (`scripts/boundary.sh` exit 0, 15 rows); no Bevy type in turts_mod_api; the order envelope is open (the order is a content id); the HUD is engine primitives with mod layout; no private path for forge or muster outside tests; determinism; fog of war has a seam (the server already builds each delta for each slot, export.rs:25,106; no client code needs every unit).

## Open: two CEO questions

1. The economy log validator is a second model of orders and movement (crates/turts_server/src/economylog.rs, 8,084 production lines; `validate_pending_movement_dispatch` :4282). Each new behaviour needs a validator branch, and this file had the most edits in MC. MP1 lanes will queue on it.
    - Option a: keep it.
    - Option b: the validator checks ledger conservation only; orders and movement are proved by the acceptance rows and the order log.
    - Option c: move it to a `turts_audit` crate with one file for each domain.
    - Auditor and Root recommend b and c, before MP1.
2. Two movement models exist: plain (PlainMoveProgress, apply_legacy_move_to; orders.rs:2848,2882) and pooled (MoveProgress, MovementPools). MC kept both so that the frozen M2 stream did not change (design :1271, K19 a). M4 routes and the M4b push must go into both.
    - Option a: keep both.
    - Option b: one model (a plain move is a pool of one) and one re-freeze of the M2 stream. A frozen value changes, so only the CEO can approve it.
    - Auditor and Root recommend b, before MP1 adds more tests that pin plain moves; at the latest before M4.

## Debt with a deadline (Root schedules these; no CEO decision needed)

| Item | Evidence | Deadline |
| --- | --- | --- |
| Version table has no row for an armed mobile unit (MC removed "weapons must be static") | version_table.rs:21-52 | Before the MC merge: RUN127 repairs it |
| Neighbour grid cell depends on the largest unit: one static unit of radius 4 raises 5,000 movers from 16.7 ms to 56.2 ms a tick (auditor probe) | movement.rs:1402-1403; combat.rs:111-117 | Before MP1 content adds a command centre or a long-range tank |
| `WorldDelta.unit_orders` sends the full owner order table every tick | export.rs:190-195; replica.rs:523-525 | Next protocol change (MP1 or M5) |
| Order validation is O(units x world); no UnitId-to-Entity index; 78 `iter_entities()` calls | orders.rs:2634, 2734, 3056, 3085 | MP2 |
| Sim cost is a constant factor, not O(n^2), but the step is one 1,020-line function with sequential updates | movement.rs:1388-2408 | Split before M4; constant factors before M7 |
| Fault builds use the product profile (fat LTO, opt z, wasm-opt z) | Cargo.toml:37-43; crates/turts_client_web/index.html:6 | Now: RUN125 measures a fault-only profile |
| CLAUDE.md says ENGINE_API_VERSION 0.2.2 (code: 0.2.3) and has no turts_interaction row | CLAUDE.md:125, :174 | The document updates after the merge |
| VIS2 p50 133.3 ms at 5,000 units; the client clones the replica for each delta | docs/measurements/mc.md:11; replica.rs:451 | Profile before M7 |

The audit log (341.6 bytes for each unit, 6.46 GB) is the EL1 economy audit log. It costs nothing when it is off and it is not the replay log of M11.
