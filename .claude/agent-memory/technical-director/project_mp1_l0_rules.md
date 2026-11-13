---
name: mp1-l0-rules-review-facts
description: Verified 2026-11-12 facts from the L0-RULES review (turts_rules at 51bd515): oracle method, saturation defects, team fallback collision, seam gaps, plant traps
metadata:
  type: project
---

Verified 2026-11-12, worktree TuRTS-mp1-l0-rules-review at 51bd515 (crate `crates/turts_rules`):
- Oracle method that worked: Python exact-integer oracle from design text -> cases.tsv; tiny scratch
  crate with `[workspace]` and a path dep on turts_rules reads the TSV (builds in 2 s, no lockfile issue).
- Defects found: heal.rs `heal_difference` and `repair_charge_difference` saturate both terms to u64::MAX,
  so the difference is 0 when rate*t/hz or H*amount/per_health > u64 (R1 `rate_per_second` is u64, D:1559).
  relation.rs `Teams::team` fallback "missing slot = team number = slot number" collides with real team ids
  (teams [2,2]: slot 0 and unlisted slot 2 are Ally).
- Seam gaps: no "build time at rate" function (H9 `BuildTimeNow`, HUD6); `refund_at` takes a ratio not
  the CancelCase; no "no damage table" form; hidden static in gap gives site_near_building (D:493-495 says site_blocked).
- Facts: wire `Pos` f32, `UnitId(u32)`, `SlotId(u8)`, ticks u32, `tick_hz` u16 (turts_net_protocol messages.rs:11-31);
  `MatchStart.teams: Vec<u8>` index = slot (D:1271); refund ratios n<=d already validated
  (content_m3_validation.rs:72-75); `try_ms_to_ticks` rounds up and gives 0 for 0 ms (boot_m3.rs:591-599).
- Trap: boundary.sh `tree` runs cargo tree without --locked, so a Cargo.toml plant rewrites Cargo.lock;
  the runner then reports wrapper_exit 2 (source changed). Restore Cargo.lock with the Cargo.toml.

- Re-check 2026-11-12 at 1b60318: all 8 closed; oracle 0 unexplained. Ratchet `formula_ratchet.rs` catches u128::from and
  `as u128` copies, misses a u64 checked_mul copy and a `let w: u128` copy (no u128 ident in the `/` expr); a lowered
  count passes with a println hidden by test capture. Trial merge with mp1/integration 8ba981d: no conflict, ratchet green.

**Why:** later MP1 tasks (L0-SIM, L-PAY, UI-HUD, UI-PLACE) consume this crate; the re-review cites these.
**How to apply:** re-check the repair of each defect at the next L0-RULES head. See [[mp1-gate2-plan-facts]].
