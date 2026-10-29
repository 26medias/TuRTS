---
name: mc-gate1-boundary-facts
description: Verified 2026-10-29 facts from the MC gate-1 review: T1 row gaps for a new no-bevy client crate, T1X copy trap, version table probe at 0.2.3, glam versions, CancelProduction job id, lightyear message order
metadata:
  type: project
---

Verified 2026-10-29 (MC gate 1, design docs/superpowers/specs/2026-10-29-mc-player-control-design.md):
- boundary.sh `nobevy` rows run on $HOST only (scripts/boundary.sh:35-37). A crate with a
  `[target.'cfg(target_family = "wasm")'.dependencies] bevy_ecs` passes the full run; the
  wasm32 `--rule no-bevy-prefix` catches it. A client crate needs a wasm32 nobevy row.
- A plain copy of a member manifest (`version.workspace = true`) under `--manifest` fails with
  "cargo tree failed ... failed to find a workspace root", exit 1 = same code as a real red.
  A fixture copy needs its own `[workspace]`, explicit fields, absolute path deps.
- t1_boundary_script_passes (crates/turts_checks/tests/boundary.rs:92-101) asserts only the
  turts_client_web lines; a new row is not asserted unless added there.
- version_table rows are `fn(&Manifest) -> bool` (version_table.rs:6-10): manifest level only.
  Probe: row since 0.2.3 refuses ^0.2, ^0.2.1, ^0.2.2 ("admits 0.2.2"), accepts ^0.2.3.
- Cargo.lock: glam 0.33.12 (bevy_math 0.20) and 0.34.1. No glam in workspace deps.
- CancelProduction carries JobId(u64) (turts_net_protocol messages.rs:67-70, economy.rs:13).
- lightyear registration order: Hello first (turts_net_lightyear/src/protocol_plugin.rs:26).
  Version guards: client acquire.rs:36 (protocol), :42 (engine equality); netcode protocol_id
  = NET_PROTOCOL_VERSION (client join.rs:275, server net.rs:44); server session.rs:97 (Hello).
- ENGINE_API_VERSION "0.2.2" at turts_mod_api/src/lib.rs:33; NET_PROTOCOL_VERSION 3.
- Spec 4.3 rows: Combat S:179, Abilities S:180, AI S:185; principle S:171.

**Why:** MC plan and later gates will cite these edges and traps.
**How to apply:** Re-check line numbers with grep before citing; MC changes these files.

Re-check 2026-10-29 (revision 2):
- movement.rs has no `capture_operation`; frozen M2 stream test (crates/turts_pack/tests/m3_m2_stream.rs)
  issues an acme MoveTo (:263) and counts every drained operation for 400 ticks (:54-58); the
  stream JSON hash is pinned at tests/m3/refutations.py:18. Any change that captures movement
  as operations changes this stream.
- Slots (turts_sim/src/slots.rs:20-57) is open -> claimed only; liveness is server Session state;
  judge_hello at turts_server/src/session.rs:93-109. Netcode client timeout 10 s (turts_client/src/join.rs:278);
  netcode client id is random per connection (join.rs:270-273).
- CI on main 0f044e9 (run 40943729334): browser job red at "M2 browser proof (base, haul, acme, cedar)".
- version table runs after catalog build (turts_mod_runtime/src/archive.rs:134-137).
- Today's movement (turts_sim/src/movement.rs:36-58): straight line, lands when distance <= step,
  removes MoveTarget with no captured operation. acme scout starts (4,4), speed 6
  (mods/acme/maps, units); the frozen-stream MoveTo goes to (30,30), so it arrives inside 400 ticks.
  Any audited order write on move completion adds an operation to that frozen stream.
- Frozen M2 stream compares, per operation, operation_id, event_kinds and `op.before == op.after`
  (m3_m2_stream.rs:116-120), not only counts. Each gatherer tick is one captured operation
  (turts_sim/src/economy/gather.rs:226, process_gatherer). New AuditState fields written inside
  process_gatherer flip before==after. Stream gather: first slot-0 worker to nearest node (:300-331).
