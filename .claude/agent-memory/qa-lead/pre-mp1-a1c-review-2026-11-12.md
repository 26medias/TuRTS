---
name: pre-mp1-a1c-review-2026-11-12
description: A1c review at b552c71 (replay deleted) - server to_sim allowed_units and ECS resting set GREEN everywhere (replay red at base); full_state tautology; JS readers read only `economy`
metadata:
  type: project
---

Verified 2026-11-12, lane TuRTS-pre-a1c-review, cache mc-pre-a1c-review, labels a1c-rev-*. Verdict FAIL HIGH=1.
Report: TuRTS-mp1/docs/research/mp1/review-slice-a1c.md. Scripts: scratchpad a1c-review/.

- Combined set `cargo test -p turts_sim -p turts_audit -p turts_checks -p turts_pack -p turts_server --locked
  --no-fail-fast --features turts_sim/test-probes`: 842 tests, 73 binaries, 200-700 s per plant.
- GREEN at head, red under replay at 8722f8e: N10 server `orders.rs:100` allowed_units -> Vec::new() (replay
  "order command 1 differs from its immutable catalog definition", 8 tests); N3 `movement/ecs.rs:70` Attack->Move
  in the resting set (replay red mc_orders.rs:566 only; replay off green).
- N13 queue depth 255 in to_sim: red only via session.rs:1750/:2760. N6 dispatch cursor: only orders/tests.rs:1724.
  F4 on the NotOwner path (sim): only session.rs:2673 (no ledger fixture has that refusal).
- Old A1b phase check (relabel Other) now ledger Ok; A1c test uses Command instead.
- full_state: bounds/resources/compiled_durations/unit_types from template -> accounting.rs:449-456 is a
  tautology on the log path; N12 (bounds grow) red only in process + full_state_rebuilds_the_audit_state.
- Browser readers (m2-audit, m2-oracle-lib, m3-oracle) read only key `economy` (Proxy probe on real sink log).
  m3 conservation rule is red on ledger_logs base/forge fixtures (domain mismatch, not format).
- Base check method: detach the lane at the base commit, plant, run with and without TURTS_AUDIT_REPLAY=off,
  detach back; plant files equal across commits first (`git diff --stat`).
- Merge check method: `git archive <merge-tree oid> | tar -x` into scratch, `git init` + commit, move the one
  cache into it, run through the runner, move the cache back.
- Removing turts_audit deps needs Cargo.lock lines 8608-8610 or --locked fails.
- Hash typo trap: `git archive` of a mistyped tree oid fails with "not a tree object"; copy oids from output.

See [[pre-mp1-a1b-review-2026-11-12]], [[pre-mp1-a2a-review-2026-11-12]].

Re-check 2026-11-12 at 3c53b96 (labels a1c-rerev-*): PASS, 0 High. N10/N13 red in ledger orders rule + to_sim test;
N3 red in movement_resting.rs; my N14 (point axis swap in to_sim) red at mc_orders.rs:253. Sweep test blind to the
physical-escape branch (S2 owner write green; only group_movement.rs:2230 red by chance). 6 sampled plants equal counts.
- ci-browser: parallel_refutations.py defaults to mc_headed_v4 (:42) and passes no --mode; use a shim
  `exec mc_headed_v6.sh --mode pre-merge "$@"` via --headed, plus --lane-target <cache> --lane-head <sha>.
  4 workers: 8/8 RED-RESTORED, 3,369 s wall, seed 437 s, holds <= 53 s. Lane must stay clean the whole run.
