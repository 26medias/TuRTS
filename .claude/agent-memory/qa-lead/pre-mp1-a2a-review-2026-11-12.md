---
name: pre-mp1-a2a-review-2026-11-12
description: A2a review at ab37cea (lane TuRTS-pre-a2a-review): copy proof, branch reach counters, late-arrival gap, harness gate scope, timings
metadata:
  type: project
---

Reviewed 2026-11-12, candidate ab37cea on pre-mp1/a2a-step (base 8722f8e). Verdict FAIL HIGH=1 (instrument only).

- Copy proof: old frame.rs lines 61-1104 vs reference.rs 33-1077 differ only in the name/doc and `..Default::default()`.
- Reach (64-seed release, counters planted): late arrival (frame.rs:798) 0 hits pooled and no-pool in 5,683,800
  unit frames; same-point projection direction (:764-770) 0; the 0.025 clause alone 0. Fault "drop Resting at late
  arrival" and builder plant (h) stay GREEN on kernel_reference.
- Two-unit probe reaches late arrival in both kernels: mover r0.5 speed 4 at (0.5,0.9) target (0.5,0.5); static r2 at
  (0.5,3.0); projection + clamp land exactly on target. Probe text kept at the scratch path in the report.
- Seen red: push switch inverted (seed 4 t295), `0..7` (seed 8 t260), cap `(v*s)/l` (seed 10 t97; debug CI form seed 14).
- `cargo test -p turts_sim --locked` runs 0 tests of kernel_reference and probe_flags (feature off). Only
  `cargo test --workspace` turns test-probes on (via turts_pack dev-dep) and runs the debug form (16 seeds, <=30
  movers, 145,200 frames, 9.19 s). Release form runs in no gate.
- `git checkout <commit> -- path` stages the file; restore with `git checkout <HEAD> -- path` (clean index).
- mc_cost.rs provenance prints HEAD even when the tree is dirty; host load 19-21 made mc_cost medians noisy.
- In-binary A/B (live vs reference, alternate order) is a cleaner timing instrument: seed 64 (5,000) 7.548 vs 7.506 s.

Re-check 2026-11-12 at 1bff266: PASS. Anchors reach late arrival, same point, slow clause (counters 1-2 each); F4, F5,
(h), slow-clause fault red in release, F4 red in debug CI form. Open: `angle` (mod.rs:123, called by spread.rs:236 on
both sides) unpinned; negating it is GREEN on kernel_reference and group_movement. A live `step` fault is red
(frame.rs:714 calls live, reference has a local copy). Report rereview-slice-a2a.md.

Report: /home/julien/Projects/TuRTS-mp1/docs/research/mp1/review-slice-a2a.md. Related: [[pre-mp1-q1b-followup-2026-11-12]].
