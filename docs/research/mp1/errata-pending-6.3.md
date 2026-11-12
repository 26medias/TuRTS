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
