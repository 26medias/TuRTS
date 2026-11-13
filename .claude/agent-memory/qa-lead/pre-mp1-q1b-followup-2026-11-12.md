---
name: pre-mp1-q1b-followup-2026-11-12
description: Q1b review follow-up build (lane TuRTS-pre-q1b-trace, branch pre-mp1/q1b-trace): real was_projected in 3 loops, gather_stage /4, probe_neutral.sh, feature-leak row; plants and survivors
metadata:
  type: project
---

Built 2026-11-12 on top of d08dc24 (one commit, amended before push; hash in the reply).

- Real `was_projected` per branch: probe field is `Option<bool>`; each loop stores `Some(any pass moved)` after its
  clamp (frame.rs no-pool, frame.rs pool, plain.rs). Pooled acme proxy (group 7 Order) then shows NO was_projected
  difference vs the base trace (false ticks 1-122, null from arrival 123, both runs): the old "false -> null" allowed
  entry is removed (3 unit entries left: state, activity, order; world pools; 2 added ops; 1 route field).
  Muster plain unit: proj true on ticks 1-11, false from 12. Muster pool group (ids 1-8): tick 1 all false, tick 2
  ids 6-8 true. Those two spans are what probe_flags asserts, so a constant store is red in each branch.
- gather_stage (trace /4): read through `order_view` (EconomyOrders), not UnitOrders; base/haul/cedar show Idle,
  MovingToNode, Gathering, MovingToReceiver, Unloading; acme none only.
- Plant survivors found by mutating the classifier: "gather_stage not allowed-listable" and "decode field order"
  survived the plants; killed by two unittest methods, not by plants (plants stay 74).
- probe_neutral.sh: frame_hash lines frame_chains=d0d1b1c572163424 muster_world=aa9db740578ed5c1 (this machine); NOT
  pinned (cos/sin libm in fixture, atan2 in movement; one machine only).
- boundary.sh `no-probes` rule via `cargo tree -p C -e normal,build,features -i turts_sim --locked`; unreachable crate
  gives "did not match any packages" (rc 101) and counts as absent. Self-check = dev edge of turts_pack must show it.
- Runner needs `--allow-dirty` whenever a plant dirties the tree; without it the call exits silently (no output).
- `git checkout -- path` restores a plant only after the work is committed: commit locally first, amend at the end.
