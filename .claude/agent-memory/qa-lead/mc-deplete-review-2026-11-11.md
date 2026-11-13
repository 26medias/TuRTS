---
name: mc-deplete-review-2026-11-11
description: Review of codex/mc-fix-deplete 43282fe (deliver cargo, then node_depleted) - fault cycles, model-only probe, receiver-lost wait, helper never asserted, E9 delay probe green
metadata:
  type: project
---

Verified 2026-11-11 at 43282fe (worktree TuRTS-mc-deplete-review; scratch copies via git archive).

- Gates at 43282fe: fmt 0, clippy 0, `cargo test --workspace --locked` 0 (1772 passed, 0 failed,
  32 ignored, 853 s), boundary 0, frozen sha bcbf72c2... equal, both frozen tests ok.
- Fault cycles (scratch copy, cmp restore). gather_depletion.rs: old sim (revert 16 lines) red
  a, a', b at :331 (cargo held), c green. Double credit after depletion red at :288 (per-tick
  conservation is real). No-failure plant red at :323. Never-fail plant red all 4.
- GAP: helper (unit 3, second gatherer) is never asserted. Plant "unit 3 never fails" green in
  gather_depletion, mc_order_rows, E9 in-process, validator fixture, economy_accounting.
- economy_accounting (42 tests) stays green for every order-timing plant; only the
  double-credit plant turns it red. Under old-sim and never-fail plants, `-p turts_pack -p
  turts_sim` reds only gather_depletion, mc_order_rows (1), orders unit test (economy_variants
  3 tests failed for env: "build crates/turts_checks example m2_variants first").
- Model-only probe (validator bypassed in scratch: read_live_complete_tick_prefix also calls
  validate_records): the model alone catches old sim, no-failure, never-fail, fail-during-unload.
- e9 in-process: node empty tick 550, fail tick 697, deterministic (headless_app 1 tick/update).
- Receiver removed after depletion: worker Waiting{NoReceiver}, order current, no
  last_failure, cargo kept, for 600 ticks. Gatherer removed: cargo destroyed with a Destroyed
  event (M3 Destroy policy).
- E9 delay probe (builder run134-probe.rs) at 43282fe: 200 s -> 4395 ticks ok, 600 s -> 12406
  ticks ok, extraction 1000 delivery 1000 depletions 1 (old code red at 180 s, op 6744).
- HIGH: full muster order-rows fixture (no prune_units) at 43282fe: worker empties node t32,
  delivery move is blocked (worker 42 at 12,12 near depot), order fails `Blocked` at t50 with
  2 cargo held (orders.rs plain_blocked path + clear_gather). Old sim: NodeDepleted t33.
  ORD3 oracle expects node_depleted. mc_order_rows test prunes units, so it hides this.
- ORD3 headed at 43282fe: both runs FAIL at "no exposed trooper column" (Click-to-play overlay).
- Validator validate_records accepts a log with Store double credit (depot 2 from 1 ore).
- ci-native parallel at 43282fe: 33/33 RED-RESTORED, 2485 s, E9-midtrace baseline 557 s.
- HAZARD: rtk hook rewrites `cargo test` inside background scripts; use `rtk proxy cargo`.
- GPU lock: 5 lanes queued at once; my ORD3 waited behind 3 others.

Related: [[mc-ci-run142-2026-11-11]], [[mc-reg1-parallel-2026-11-11]].
