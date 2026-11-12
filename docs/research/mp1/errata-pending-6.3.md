# Design errata that wait for revision 6.3 (from the lane-0 reviews, 2026-11-12)

The orchestrator applies these in one pass when lane 0 exits. Each comes from an independent review of a lane-0 task.

## From the L-ROOM0 review (network-engineer)

- 6.7.8, replace "The server notices a lost client at the netcode timeout" with: "The server sees the end of a connection in
  two ways. A closed transport (a close frame, FIN or RST) adds `Disconnected` to the `ClientOf` entity within 5 ms (native
  release, L-ROOM0 review). A silent loss is seen at the netcode timeout, 10 s. L is the second value."
- 6.7.5, the reload bullet, add: "A page that unloads starts the closing handshake (WHATWG WebSockets, section 7), so the
  release normally comes before the first retry." (A real browser run is NOT RUN.)
- 6.7.8, add: "Under `Immediate` the room has no seat; the hook reads the slot from the `Session` of the ended entity."
- 6.7.5, add one line for the cells that the table calls "not possible" and for Boot: the refusal text of
  `JoinVerdict::Impossible` (the text that the repair of L-ROOM0 chose).
- 6.7.14 item 4 (CEO item, keep or lower the 10 s timeout): add the fact: the 10 s applies only to a silent loss; a closed
  transport is seen in under 5 ms. The recommendation "keep 10 s" costs less than the text assumed.
- Hook ruling for the wiring task (plan task L-MATCH-SRV1; record in the plan block): an observer on the lightyear
  `Disconnected` component with `With<ClientOf>` that reads the slot inside the observer; a `PreUpdate` system before
  `receive_hello` that calls `Room::connection_ended`; `Query<&Session, Without<Disconnected>>` for liveness; `Session` is
  not removed. The task asserts a notice under 1 s after SIGKILL.

## From the L0-RAW review (mod-builder)

- 6.10 "Version-table detection", add: "Exception: `dependencies` is required since API 0.1; its 0.2.2 feature is a declared
  dependency, so its row reads the parsed value (a non-empty map)."
- 6.10, add: "A path looks through a list." and "A field inside an enum variant is detected by a key marker on the typed
  value, so a key written with `None` counts."
- 6.10, add: "The version rows that read the raw document run before the content cross-checks."

## From the L0-RULES review (technical-director)

- 6.6 (near D:774), old: "gives the relation." New: "gives the set of relations that hold (they overlap: the own slot is
  Same, Ally and Any); `matches(filter, a, b, teams)` tests one filter. A slot that is not in the team table is allied
  with no other slot."
- 6.1 check (near D:474), old: "overlaps a footprint, a unit included". New: "overlaps the footprint of a unit or a site, a
  mobile unit included; an overlap with a resource node fails check 5".
- 6.4 (near D:631), add: "A `total` of 0 is complete: the charge is the full cost; `done` above `total` counts as `total`."
- 6.2: the combine rule for several short supply resources is not defined (MP1 has one). Open point, owner
  game-designer, before M6.
- Plan note: `UI-PLACE` builds the placement `View` from replica positions, and one shared-scenario test compares the
  server `View` and the client `View` (the conversion cannot make the two sides disagree; the view assembly can).

## From the Q-PLANTS review (performance-architect)

- Plan section 4: the slow gate runs the plants through the parallel runner set `mp1` (one worker for each command group),
  not the serial runner; a headed record never runs without the headed wrapper.

## Binding notes for later tasks (from the lane-0 re-checks, 2026-11-12)

- `L0-SIM` (or the next task that edits `crates/turts_checks/tests/formula_ratchet.rs`): make the count comparison exact
  (a count that goes down is red too, with the "record N" message, because the pass output is captured and the headroom
  would let a later copy in); add two fingerprints: a `/` whose operands are bound by a `let` with a `u128` type; and no
  function outside `turts_rules` whose name equals a public `turts_rules` function. Today a formula copy written in u64
  with `checked_mul`, or with a `let w: u128 = …` binding, is not seen.
- `L-REP`: `turts_rules::repair_charge_difference` returns `Option<u64>` (None = cannot pay), or the packer refuses
  `max x amount / per_health` above u64. Today a total above u64 saturates and the tick is undercharged with no signal.
- `L-PAY`: remove the refund formula copy at `crates/turts_sim/src/economy/production.rs:226` (the ratchet table counts it)
  and lower the count.
- `L-MATCH-SRV1`: the hook ruling of `docs/research/mp1/trace-connection-end.md`; assert a notice under 1 s after SIGKILL
  (the process test `connection_end.rs` has no lower bound and cannot tell the hook path from the timeout path).
- Stage 2 (`L-LOBBY2-SRV`): a test for the `CountdownToLobby` edge of `room.rs` (a plant that removed it stayed green).

## Note for every later visuals and effects task (from L0-SPLIT and its review, 2026-11-12)

- A new visuals concern adds one file under `crates/turts_client/src/visuals/` and one call line in `draw_world_visuals`
  after the last present concern call and before `update_health_fills` (the block `draw_queued_targets`,
  `update_health_fills`, then `reconcile_visual_entities`), and writes into `desired` through `&mut DesiredVisuals`.
  Several tasks add a call line in that same region: expect a textual conflict at the merge and keep the order.
- A new retained entity type also adds: a despawn loop in the early-exit block of the caller, a `Query` parameter on the
  system, a field in `VisualState` if it keeps state, and a reset in `sync_session`. `visuals.active = false;` stays
  before the let-else.
- Call pairs that must not swap: `sync_session` before the let-else; `resolve_hover` before `draw_hover_ring` and the
  tooltip; `intake_markers` before `track_blocked`; `draw_order_markers` before `draw_blocked_marks`;
  `collect_health_bars` before `update_health_fills`; every draw call before `reconcile_visual_entities`.
- The scene record `visuals/scene_oracle.txt` is a record of behaviour. A task that changes behaviour re-records it with
  the ignored test `bless_scene_fixture` and shows the diff of the record in its review. A task that adds a concern adds
  a scene for it.
- A new effect family adds a function in `core_adapter/effects.rs` and one match arm in `drive_core_adapter`.
  `Effect::Marker` and `Effect::Sound` have no producer today, so no test reaches those two arms.
- The fault registry case `E9-browser-debug-only-ui` now plants `visuals/rings.rs`. Its browser run is owed before the
  merge of the first later task that edits `rings.rs`.
