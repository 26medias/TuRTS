# Unit exit and rally point

Date: 2026-11-13. Status: ready to build. Track: Foundation (simulation, one order behaviour).
Process: this spec, the build, one independent review that runs code. No plan document.

## 1. The problem (CEO, after play)

"Once there are 4 soldiers built around the barracks, no others can be built until the space is
vacated."

Cause, traced:

- A producer has a fixed list of exit points. The Barracks has four
  (`mods/skirmish/units/barracks.ron`, `spawn_offsets`).
- `try_output` tries each point in order and takes the first free one. If none is free, the job
  stays in `OutputBlocked` and the queue stops (`crates/turts_sim/src/economy/production.rs:570-589`).
- A new unit does not move after it appears. Thus four idle units block the building.
- The rally order is in the mod API but no unit can do it:
  `Behaviour::SetRally => false` (`crates/turts_sim/src/orders/apply.rs:165-168`) and
  `Err(R::NotApplicable)` (`apply.rs:932-935`). The packer refuses it (`UNBUILT_FEATURES`).

## 2. What the player sees after this change

1. A trained unit always appears while free ground exists near the building. Idle units around
   the building do not stop production.
2. Each producer building has a rally point. A new unit walks to it immediately.
3. The default rally point is a short distance in front of the building, on the side of the
   building that faces the map centre. The player sets no rally point to get this.
4. With the building selected, a right-click on the ground sets the rally point. A small flag
   marker and a thin line from the building show it while the building is selected.
5. Units that arrive at the rally point follow the "arrive first" rule: the first unit keeps the
   point, and later units stop near it with no overlap. Twenty units make a compact group.
6. If no free ground exists in the search area (section 3.1), the job waits as today, and the
   production row of the HUD shows the text `Exit blocked`. The job continues when ground is free.

## 3. Engine rules

These are engine mechanisms. All mods get them. The skirmish package gets no private path.

### 3.1 Exit point search

Replace the candidate selection in `try_output` with this order:

1. The authored `spawn_offsets`, in their order (as today).
2. If none is free: a ring search around the producer. Ring `k` (k = 1, 2, ...) has the radius
   `producer radius + output radius + k * (2 * output radius)`. On each ring, test points at a
   fixed angular step so that adjacent points are `2 * output radius` apart (floor the count,
   minimum 8). Start at the direction of the rally point and alternate left and right of it.
3. Stop at the first free point. Stop with `Blocked` when the ring radius exceeds
   `producer radius + 12.0` map units.

"Free" keeps the meaning of today: inside the map, no overlap with a unit circle, no overlap
with a surface node that has `remaining > 0`.

Requirements:

- The search is deterministic: the same world state gives the same point on every run and every
  platform. Use only the arithmetic that the simulation already uses for positions.
- Cost: the search runs only when the authored points are full. One attempt for each blocked
  producer for each tick, as today. Reuse the unit spatial index if one serves; do not add a
  scan of all units for each ring point when the index can answer.
- A unit id does not decide which unit gets a point that a player can see. Two producers that
  finish in the same tick can keep the producer iteration order of today (a same-tick tie-break).

### 3.2 Rally point

- State: each producer has an optional rally target. For this spec the target is a ground point
  only (`GroundRally`, `crates/turts_mod_api/src/schema.rs:484-498`, already exists). A rally on
  a unit or on a resource node is out of scope.
- Default: when a producer has no rally target, the engine uses the point at the distance
  `producer radius + 4.0` from the producer centre, in the direction of the map centre. If that
  point is not free ground, the unit goes as near as the movement rules permit.
- `Behaviour::SetRally`: build it. A producer that the slot owns accepts it. The target must be
  inside the map. The server validates it; the client only sends the order.
- On spawn: the new unit gets one move order to the rally target, through the same order path
  that a player move uses. No new movement code.
- The rally target is server state. It goes to the owner's client so that the marker can show.
  Choose the smallest wire change; if it changes the protocol version, state it in the report.
- Remove `SetRally` from the `UNBUILT_FEATURES` refusal. `CancelConstruction`, `Repair` and
  `Surrender` stay unbuilt.

### 3.3 Skirmish package

- Add the rally order to `mods/skirmish/orders/` and to the Barracks and the Command centre.
- Smart right-click: with a producer building selected and nothing else, a right-click on the
  ground sets the rally point.
- The computer opponent needs no rally logic; the default serves.

## 4. Out of scope

Rally on a unit or node, waypoint queues for rally, a rally shared by several buildings, a hotkey
for rally, unit formations.

## 5. Acceptance

Each row must be able to fail on the build before the change.

| Id | Check | Level |
| --- | --- | --- |
| EX1 | A Barracks with all four authored points occupied by idle units trains a fifth soldier; it appears at the first ring point. Before the change: the job stays `OutputBlocked`. | sim test |
| EX2 | A Barracks trains 30 soldiers with no player input. All 30 exist. No two unit circles overlap at any tick. | sim test |
| EX3 | A producer enclosed so that no free point exists inside the search limit stays `OutputBlocked`. When one blocker is removed, the unit appears in the next attempt. | sim test |
| EX4 | The same scenario run two times gives the same positions for every unit at every tick. | sim test |
| RL1 | With no rally set, a new unit moves and stops within 1.5 map units of the default rally point (first unit), and the next units stop near it with no overlap. | sim test |
| RL2 | `SetRally` from the owner is accepted; from another slot it is refused; outside the map it is refused. | sim test |
| RL3 | Browser, pixels only: select the Barracks, right-click the ground, train a soldier. The marker shows at the click point. The soldier ends within 1.5 map units of it. | browser |
| RL4 | The scripted player probes of `docs/qa/cold-play.md` still pass, and one probe trains 10 soldiers from one Barracks without a stop. | browser |
| HUD1 | In the EX3 state the production row shows `Exit blocked`. | browser or client test |

The frozen operation stream and the probe hashes of the older packages can change if they
contain a blocked exit. If one changes, record the old value, the new value and the reason in the
report. Do not edit an expected value without that reason.

## 6. Review

One reviewer, after the build. The reviewer runs EX1 to EX4 and RL1 to RL2, and plants two
faults: (a) the ring search removed, (b) the move order at spawn removed. Both must turn a row red.
