---
name: pre-mp1-q1b-trace-2026-11-12
description: Q1b probe/emitter/allowed list (lane TuRTS-pre-q1b-trace, commit d08dc24) - plan Step 5 plant cannot go red on the stream; counts all zero; leak check; mutants
metadata:
  type: project
---

Verified 2026-11-12, branch pre-mp1/q1b-trace, d08dc24 (base 7b23b37).

- Stream counts at base (emitter 9216741700...): state none 1600/2548/400/2494 (base/haul/acme/cedar),
  pool 0, plain 0, (b) 0, (c) 0; unit-ticks with target and state none 256/230/122/289 = GSIM figures.
- Plan Step 5 plant (probe `separated` always false) stays GREEN on `--p0` and on count (b): the stream
  never sets separated/was_projected. Only crates/turts_sim/tests/probe_flags.rs (muster crowd) goes red.
  Lesson: a plant on a flag that the oracle data never sets proves nothing; add a crowd case.
- A probe write that moves units (feature on) is red in m3_m2_stream (cedar op count), forge_cases c16,
  frame_hash cmp and P0. A plant placed before `outputs[index].position = position` is overwritten: green.
- Feature leak: `cargo test --workspace` builds ONE turts_sim lib with test-probes (json artifact check), so
  turts_server tests link it on. `cargo build -p turts_server` and `cargo tree -p turts_server -e features
  -i turts_sim` show no test-probes; turts_client has no turts_sim edge. Plant (feature in a normal dep) turns
  the grep red.
- `cargo test -p turts_pack` alone needs `cargo build -p turts_checks --example m2_variants` first (pre-existing).
- Real data for 9.24a: pooled single acme move (CommandKind::Order group 7): order Move at ticks 1-123, none at
  124; activity/state/was_projected/pools differ on 1-122; position, target, pending count equal; added op
  at tick 124 has phase Orders (not PostMovementOrders). Op at tick 1 of that proxy is phase Orders, unit 1.
- Classifier: 69 plants (44 + 25), 15 mutants of the new code each red; mutant "order kind not checked" first
  survived (stage check alone caught the content-id plant): add a plant per independent rule.
- grep is rewritten by the rtk hook to a compact form; an empty output file then makes `cmp` pass vacuously.
  Use `rtk proxy grep`.

See [[pre-mp1-q1a-classifier-2026-11-11]].
