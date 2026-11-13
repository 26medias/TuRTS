---
name: pre-s1-audit-crate-facts
description: Verified 2026-11-11 facts from the S1 review (turts_audit pure move, base a29ce67, head 5ce21d2): token-check result, read_jsonl embeds validation, shadowed boundary rows, bevy facade edge, A1 traps
metadata:
  type: project
---

Verified 2026-11-11 in /home/julien/Projects/TuRTS-pre-s1-review (S1 lane, head 5ce21d2):
- Own token check (scratch tokcheck.py: lexer, strip pub/use, difflib old vs format+replay+lib,
  deleted runs vs server economylog.rs): residual = from_tick impl reordered into format.rs, one
  trailing comma of read_jsonl_file, `#[cfg(test)] mod tests {` + 4-line `fn initial()` copy.
  Planted `!=`->`==` at replay.rs:4214: red in tokcheck and in builder partition.py.
- partition.py normalises `,)` to `)` (hides a 1-tuple change) and only prints the server
  economylog.rs leftovers; item_equal.py needs unwrap_tests.py to see single tests.
- read_jsonl (replay.rs:73-87) calls validate_records: the decoder IS a validator call. Users:
  el1.rs, economy.rs, session.rs:1053, server economylog.rs tests. A1 step 3 must move the
  decoder to format.rs before replay.rs is deleted.
- Server writer test validator_accepts_an_orders_operation_before_movement
  (economylog.rs:247-334) pins the replay-only text "movement transition changed nonmovement
  state" (:332). The writer tests compile in 3 binaries via #[path] economy.rs:9, mc_orders.rs:3
  plus main.rs:7.
- Boundary: the 7 "X no turts_audit" rows are shadowed: sim->audit is a cargo cycle error;
  client/client_web/interaction -> audit trips the older turts_sim rows first (probe in clone).
  turts_audit -> turts_mod_runtime is red by name on the real row (probe).
- turts_audit uses the `bevy` facade (no default features) for bevy::math::Vec2; it adds bevy,
  bevy_internal, bevy_diagnostic, bevy_input, bevy_shape, bevy_transform beyond turts_sim's tree;
  no render/winit/pbr on host or wasm32. turts_sim depends on bevy_app/ecs/time/math only.

**Why:** A1 steps 2 and 3 and the S2/S3 pure moves reuse these instruments and traps.
**How to apply:** Re-check line numbers after A1 lands. See [[mp1-architecture-input-facts]].
