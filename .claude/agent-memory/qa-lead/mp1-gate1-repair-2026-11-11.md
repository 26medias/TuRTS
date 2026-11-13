---
name: mp1-gate1-repair-2026-11-11
description: MP1 design gate-1 repair (qa-lead part) - completeness gate form-only proof and R7b proposal, scan misses interface kinds, float damage cells, CA13 mutation, static writer probe
metadata:
  type: project
---

Verified 2026-11-11 on TuRTS-mp1 (code of d729a97 = ba4e689), scratch copies via `git archive`.

- Completeness gate checks form only: `done:Cargo.toml` passes (exit 0). R7b proposal (2 lines after
  completeness.py:123: the `test` cell token must occur in the done file) is red on it. R7b needs a fill of 15
  MC done cells (all `-`) and 14 fixture cells (`acquire.rs`), plus case 22; then self-test 22/22, enforce 0.
  Diff: scratchpad qarep/r7-proposal.diff.
- R4 checks only `in` rows; a `done:` row's deps are never checked. MF07 (dep MP07) goes R4 red when MP07 is
  later. MP07-MP01 edge removal needs the CEO (QA0 K6), like K20 for MP09.
- No-content scan derives a fixed kind list (support/mod.rs:86-267): an interface alert id literal
  (`muster_unit_damaged`) gives 0 hits; `muster:trooper` gives 48 hits, exit 101.
- Wrong MC evidence found: CA13 (cam1 has no bound assert; real: turts_interaction camera.rs `bounds_hold`,
  red when clamp_to_map returns early), PE04 (no timing), PC17 (ORD2 is a selection filter; c17 is the gate),
  PC23 (ring is in sel1-oracle), UI12 (vis-full-oracle assertHealthBar), AN06 (ORD7 marker), CA16
  (sh1-settings-oracle:79).
- Float damage: f64 `100*0.29` floors to 28 (int 29); f32 differs elsewhere (50*106/100 -> 52); no cell
  fails both, so a row needs two cells.
- Production writers in turts_sim: Healths only boot_m3.rs:110, transaction_m3.rs:218; despawn only
  transaction.rs:747. A grep that cuts at the first `#[cfg(test)]` works today but is fragile.
- Repair passes ran in parallel: other seats' commits landed during my run (parts 1-3); re-read the design
  before assembling a section that holds other seats' rows.

See [[mp1-coverage-2026-11-11]], [[skill-review-2026-10-26]] (rtk diff hid a difference again here).
