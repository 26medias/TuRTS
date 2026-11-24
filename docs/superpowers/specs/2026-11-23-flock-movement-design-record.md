# T-6: group movement as a flock (design note)

Viktor (Technical Director), 2026-11-23. Owners: Viktor (architecture), Clara (player feel).
Advice: Lena (authority, privacy, wire), Kenji (trace of the current faults; see section 9).
Status: decision note for Iris. Not a spec. No code is built. Iris approves the build plan.

Code references are to `/home/julien/Projects/TuRTS-dense-acceptance-integration` at 554775ac.
S:n = line n of `docs/superpowers/specs/2026-11-15-terrain-group-movement.md` at 554775ac.
[M] = measured by a run. [E] = estimate. NOT TRACED = nobody has a run or a code path for it.

## 0. Summary

The current path makes each step a proof. A unit moves only after a route admission, an exact
swept certificate and several budgets agree. When any of them says "not yet", the unit waits.
Every repair added another rule. The movement and navigation folders alone hold 34,271 lines
[M, `wc -l`; about 11,100 are test files], and `terrain/traffic*` adds 19,206 lines [M].

Proposal: a flock, as D63 and spec 13.6 already allow ("hierarchical ... flow fields for
group destinations, and bounded-neighbour local avoidance", Baseline). Guidance is advice that
is always available. A unit moves on the first tick. The physical step check stays the only
authority against walls, cliffs, buildings and foreign bodies. Between own moving units,
separation is soft. At rest, overlap stays forbidden.

## 1. Decided player rules (operator, 2026-11-23, through Iris)

These are inputs, not proposals. They replace "arrive first keeps the exact point" and the
GM3 rule A where they conflict.

1. A moving unit pushes an idle friendly unit aside. The pushed unit stays where the push
   leaves it. Never a unit on Hold, an enemy or a building.
2. A group order targets an area around the point. The first unit keeps its place in the
   crowd. Settled units shift a short distance to make room. A unit of a later order may push
   a settled idle unit. The unit id never decides.
3. A unit that touches the settled crowd of its own order has arrived and stops.
4. Moving units may touch and overlap a little for a short time in a dense stream. At rest
   each unit stands apart. Walls, cliffs, buildings and enemies stay solid.
5. A unit that cannot pass goes to the obstacle and waits at the side of the flow, visibly
   marked. A reason text needs a wire change and stays out of M4b.
6. Units move at once and correct the route during the move.

Feel targets (Clara, `.swarm/reports/T-6-player-feel-section.md`; proposals, not measured):
the first units move in less than 0.5 s, 1 s at most; the last of 36 units starts in 3 to 5 s;
a gap one unit wide passes 2 to 3 small units each second; a too-wide unit waits beside the gap
and goes 1 to 2 s after the dig opens it; the last unit stops less than 3 s after it touches the
crowd; the crowd fills from the point outward; no red X on a unit that stopped at the edge of its
own group. Rules a player keeps: no unit through a wall, cliff, building or enemy; a too-wide
unit never passes; no stack at rest; Hold stays; no movement that reveals a hidden enemy; the
unit id never decides.

## 2. The model

Each tick has four stages. Every stage reads one frozen snapshot taken at the start of the tick.
All units are updated at the same time from that snapshot. So the order in which units are
processed cannot change a result. This removes the id-order dependency that exists today
("Units resolve in id order", `movement/frame.rs:1717`).

A. Guidance (shared, per owner). For each group destination area and size class, the owner gets
   a flow field: a cost-to-goal grid built from the owner's permitted terrain view only (S:68,
   S:70; Lena). A coarse field (2 m cells) comes first, the fine field (0.5 m) refines it over the
   next ticks. Each tick has a fixed field credit, counted in cells, never in wall time and never
   on a thread (Lena: replay). Until a field cell is ready, a unit steers on the straight line or
   on the coarse field. Rule 6 is this stage. Fields are cached by (owner, size class, goal
   region) and are invalidated by the revision of the terrain chunks that they read. A single
   unit with a unique point uses the same service with a small region.
B. Steering (per unit, advice). Desired velocity = goal direction from the field + separation
   from neighbours (radius-aware) + alignment with moving units of the same order + weak
   cohesion to the order centroid. At most 8 neighbours, chosen by distance, then position
   (Lena: never by id or hash order). Neighbours are own units and CURRENTLY permitted foreign
   bodies. A hidden body exerts no force.
C. Physical resolution (authority). Each unit proposes one step (speed x dt). The step check
   against the TRUE world refuses or shortens any step into a wall, a cliff, a building or a
   foreign body. That check stays exact, as today. Then own units relax overlaps in K passes of
   simultaneous position corrections. A pass moves only own units that are not on Hold, and
   never into terrain. Idle friends move under this push and stay (rule 1). Settled units of the
   order shift a bounded distance to make room (rule 2). A small transient overlap between own
   MOVING units is allowed (rule 4). At rest the overlap must be zero: a unit settles only at a
   non-overlapping pose.
D. Arrival and waiting. A unit settles when it reaches the goal area or when it touches a
   settled unit of its own order (rule 3). A unit whose size class has no path in its field
   walks to the reachable cell nearest the goal, steps to the side of the flow and waits,
   marked by the existing wait display (rule 5). It resumes when the field invalidation finds a
   path (target 1 to 2 s after the dig). The tie rule of today stays: an exact geometric tie
   gives the same result to both units; no id breaks it.

Lena's constraint, adopted as written: "Flock advice is a pure function of the owner's frozen
permitted view and the owner's own controller state at the tick; it has a fixed work credit; the
exact checks stay the only authority."

## 3. What stays (foundation)

Server authority; the client is a presenter, with no client-side flocking (no
`lightyear_prediction`). Same-build replay, and equal trajectories under id, spawn and
connection-order reversal (S:43, S:84). No wall or cliff penetration: the exact step check
against terrain, the M4 plateau, step and grade rules. No overlap at rest. GM6 privacy: fields
and advice read only the permitted view; field work does not depend on hidden things (S:120);
the field, the group heading and the waiting reason stay off the wire. The frozen GM7 workload
and limits. No wire change in M4b: units keep the present state records (Lena; protocol 11,
`turts_net_protocol/src/lib.rs:21`).

## 4. What goes or softens

Row numbers point into the inventory of the current code (554775ac). [P] = a player sees it;
all [P] items are already decided in section 1.

| Current mechanism | What it was for | Why it can go | Risk |
| --- | --- | --- | --- |
| Route admission: a move row starts with no target and no speed until `guide` gives a waypoint (`terrain/navigation/shared.rs:1929-1931`); "A route that exceeds its current work budget remains pending" (S:64) [P] | Bounded route work; no straight-line permission | Rule 6. Guidance is advice; the step check guards walls. The first-motion wait comes from here or from the next row (Kenji's trace pending) | A unit walks a wrong first step into a dead end; the field corrects it |
| Direct-line certificate, memo and `DIRECT_QUERY_WORK` order accounts (`shared.rs:747-758`, `:1369`, `:1422-1462`; S:509-519) | Bound the cost of refused long lines (about 1.7 ms each, S:517) | No long-line proof is needed: a unit checks only its own short step | None known |
| Swept pair certificate against other moving bodies, slide and bisection fallbacks (`movement/sweep.rs:60`, `:269`, `:497`, `:503-608`) | Exact no-overlap while moving | Rule 4 allows a transient overlap between own movers. KEEP the exact step check against terrain, buildings and foreign bodies | Visible overlap if K is too small; a cap on overlap depth is an acceptance row |
| Cooperative certification and joint staging (`sweep/support.rs:207`; S:163-173, S:287-307) | Let a group move together under exact certification | Soft separation makes it unnecessary | None known |
| Pair and owner budgets `PAIRS_PER_ACTOR` 4096, `OWNER_WORK` 32,768, half-pool guidance, `PEERS` 128 fail-closed (`sweep/bounded.rs:6-11`, `:88`, `:123`, `:262-266`) | Bound certification work | A fixed neighbour count (8) bounds the work by construction | None known |
| ORCA steering, coherence, traffic gates and fences, detours, parking (`terrain/traffic*`, 19,206 lines; S:175-286) | Avoid collisions under exact certification | Steering stage B replaces them | Foreign-body avoidance must still route around a disclosed body; stage B treats it as a stationary obstacle (S:150) |
| Stall clocks, pass mode, spare slots, claim exchange, restart budget, queue rule (`movement/frame.rs:406-430`, `:1812-1950`; `movement/exchange.rs:33`, `:154`; `movement/plain.rs:365-409`) | Break jams that exact rules create | Push and soft separation remove the jams at their source | A real jam with no feasible order; then rule 5 marks the unit |
| Bounded recovery (`movement/recovery.rs`, 1,937 lines, `:288`, `:1878`) | Find an escape for a Blocked row | No row is Blocked by a friend; terrain blocks give rule 5 | None known |
| Packed stop, held final leg, order clock (`terrain/navigation/packed.rs:171`; `shared.rs:835-907`; `navigation.rs:40-47`; S:309-490) | GM3 rule A | Rule 3 is simpler: touch the settled crowd of the order and stop | None known |
| Arrival precedence and the ban on displacing settled or idle friends (`movement/arrival.rs:66`, `:126-162`; `exchange.rs:11-12`; S:30-33, S:41) [P] | "Arrive first keeps the exact point" | Replaced by rules 1 to 3 | A pushed idle unit leaves a place that the player chose; decided as acceptable |
| Spread slot pool (`movement/spread.rs:171`, `pool_index.rs`) | Assign exact final slots | The area target and the crowd fill replace slots | The crowd shape differs from today; the feel target is a round blob |
| Solver retries that hold the whole frame (`terrain/navigation.rs:23`, `:986-1013`) | Exact terrain solve | One step check per unit; no frame-wide retry | None known |
| Stale escape repair (`sweep.rs:789-808`) | A defect of the sweep | Goes with the sweep | None |

KEEP and reuse: the terrain geometry and clearance code (`terrain/navigation/geometry.rs`,
`GEOMETRY_BUDGET` 16,384), the per-viewer knowledge and its revisions, the region field cache
idea (`shared.rs:1544-1683`, bounded to 16 fields), the order envelope and per-unit results
(D65), and the wait display.

## 5. Cost

Measured [M] with a C probe, one core (CPU 14), not engine code:
`/tmp/claude-1000/-home-julien-Projects-TuRTS/00d9ddb9-f1f3-422f-8912-5def3b1de55a/scratchpad/flock/flock.c`.
Worst case: all units packed in front of one 4 m gap, all moving, 20 Hz.

| Case | Flock tick (mean) | Per unit | Field, one size class, full Dijkstra |
| --- | --- | --- | --- |
| 1,000 units, 128 m map (256 x 256 cells), K = 2 | 0.70 ms | 0.70 us | 3.24 ms |
| 1,000 units, same map, K = 4 | 0.98 ms | 0.98 us | 3.27 ms |
| 1,000 units, 256 m map (512 x 512 cells), K = 2 | 0.75 ms | 0.75 us | 15.6 ms |
| 5,000 units, 256 m map, K = 2 | 4.53 ms | 0.91 us | 18.1 ms |

Budget [M, full-3]: GM7 server p50 26.4 to 28.9 ms and p99 32.7 to 35.0 ms against the limits
p99 40, p99.9 50, max 100 ms and debt 50 ms. The movement share of that tick is NOT TRACED.
[E] Flocking adds about 1 ms per tick for 1,000 units. A full fine field costs 3.2 ms, so it gets
a fixed credit of about 16,000 cells per tick (about 0.8 ms) and is ready in about 4 ticks
(0.2 s); the coarse field (about 4,000 cells) is ready in the first tick. The exact step check
on short steps replaces the long-line certificates (about 1.7 ms per refused line, S:517). Its
cost per step in Rust is NOT TRACED. GM7 must run again on the new path: the movement code is in
the tick path.

## 6. Acceptance

Keep GM1 to GM8 as player-facing rows. Text changes (S:105-112):

- GM1: keep. Add: "The first unit moves within 1 s of the order."
- GM2: "Large units keep safe positions and a truthful wait state." becomes "Large units go to
  the obstacle and wait at the side of the flow, visibly marked." Add to the resume sentence:
  "within 2 s".
- GM3: "First physical arrival retains its point under ID reversal." becomes "The first arrival
  keeps its place in the crowd. Results are equal under ID reversal."
- GM4: "Check full swept clearance on every committed segment" becomes "Check that no committed
  step crosses a wall, cliff, building or foreign body". "No ID-based winner or overlap is
  permitted during retries." becomes "No ID-based winner. No overlap at rest."
- GM5 to GM8: unchanged.
- New measured rows (ridge package, 36 mixed units): FM1 first motion (first unit within 1 s,
  every unit with free space in front of it within 1 s); FM2 return crossing (Jade's case:
  36 units back through the dug passage; all settled and apart within 60 s (Iris, 2026-11-23); no unit with no
  motion and no settled state for more than 5 s); FM3 overlap while moving (maximum overlap
  depth less than 25 % of the smaller radius, and no overlap at rest).
- Internal checks that go: the swept pair certificate tests, the cooperative support tests,
  the packed-stop and held-clock tests, the recovery and exchange suites. Internal checks that
  stay: Lena's list (paired hidden worlds, contact locality, id and order reversal with a planted
  id-sort fault, same-build replay, viewer check, the 20 Hz server-only replay gate).

Spec lines that change (S at 554775ac):
- S:30 "The first unit to arrive keeps its occupied point." becomes rule 2.
- S:31 "Later units stop at the closest feasible non-overlapping position." becomes rule 3.
- S:33 "Destination planning cannot evict a unit that already arrived." becomes "A unit of a
  later order may push a settled idle unit. A unit on Hold is never pushed."
- S:40-41 "Units in transit can yield ... They cannot overlap, teleport or displace a settled
  arrival to make progress." becomes rules 1 and 4.
- S:46 "units stop safely and report waiting or blocked" becomes rule 5.
- S:64 "A route that exceeds its current work budget remains pending. It does not become
  straight-line permission." becomes rule 6 plus "Guidance is advice. The step check refuses
  any step into a wall."
- S:69 "Execution certifies actual physical segments before committing them." becomes
  "Execution checks each actual step against terrain, buildings and foreign bodies before
  committing it."
- Retire the sections at S:118-145 (traffic-index bound), S:163-173, S:175-307 (steering,
  coherence, staging), S:309-490 (packed stop: replaced by rule 3), S:491-507 and S:509-519.
  Keep S:146-161 only for the rule that a disclosed foreign body is a stationary obstacle.

## 7. Plan and estimate [E]

Engineers: Kenji (simulation), with Lena for the privacy and replay tests and Marco for the wait
display. Estimates in engineer hours, not measured.

1. Field service: coarse and fine fields, size classes, owner view, chunk-revision
   invalidation, fixed credit (6 to 8 h).
2. Flock stages B to D behind a switch, with push, settle and wait (8 to 10 h).
3. Wire it into the navigation entry for terrain moves; headless ridge traces for FM1 and FM2
   (4 h). Shortest path to a build that a person can play: steps 1 (coarse field only), 2 and 3,
   then a browser build of the ridge package: about 16 to 20 h.
4. Tests: Lena's list, GM1 to GM4 with the new texts (6 to 8 h).
5. GM8 browser and GM7 under B3 (4 h plus one operator window).
6. Delete the old code after parity (3 to 5 h).
Total about 31 to 39 h. The old path stays reachable behind the switch until step 5 passes.

## 8. Risks

- Jamming at a choke (the "faster is slower" arch of real crowds). Mitigation: the field funnels
  into lanes; separation yields by distance to the goal (geometric, never id). The probe moved
  383 of 1,000 units through a 4 m gap in 21 s with crude rules, with no deadlock [M]; that is a
  probe, not the engine.
- Float sums: a fixed order is required (Lena). The simultaneous update and the physical-key sort
  carry this; the reversal test with a planted id sort must go red.
- Wire load: all units of a group move from the first tick, so bytes per tick stay high for the
  whole move (Lena: 1,000 moving units fill the send window in one frame). Measure with
  `--synthetic-units` before a group size is fixed. NOT TRACED.

## 9. Not traced and open

- Kenji's trace of the 43 s first-motion delay (854 ticks), of the return knot and of the stranded
  Walker: NOT RECEIVED when this note was written. Section 4 names the likely holders from the
  code; the measured cause is NOT TRACED.
- The movement share of the GM7 tick, the cost of one exact step check in Rust, and the bytes for
  each moving unit on the wire: NOT TRACED.
- Inventory rows 18 and T (ORCA, traffic) are read only at their entry points.

## Decision asked from Iris

Approve the build plan of section 7, with the switch, the new rows FM1 to FM3, and the spec line
changes of section 6. Or name what to change.

## Iris decision, 2026-11-23 (task T-7, owner Tomas)

Plan approved with changes: (1) playable first: steps 1 (coarse field), 2 and 3, then a browser
build of both ridge packages and Jade's outbound-dig-return play, before the full test campaign;
(2) FM2 = all 36 units settled and apart within 60 s; GM1 keeps 120 s; (3) step 3 measures the
wire bytes per tick for 36 and 1,000 moving units, and the work stops and goes to Iris if the send
window fills; (4) the switch is temporary: the old path is deleted in M4b once GM1-GM8 and FM1-FM3
pass; (5) the spec edit uses the line changes of section 6, the operator rules are marked "Decided
2026-11-23", and this note is committed as a design record.

## Design answers for T-7 (Viktor, 2026-11-23)

A. Temporary activation contract.
- One publish point: a `MovementModel { Legacy, Flock }` resource in `turts_sim`, inserted one time
  at world setup and read only at the terrain-movement entry. Nothing else branches on it.
- One setup entry: the host reads `TURTS_MOVEMENT=legacy|flock` one time at server start (and the
  test harness sets the resource directly). Any other value stops the server before the match.
  Default: `legacy` until the removal build; then the resource, the variable and the old path go.
- Not in the mod API, not in `server.ron`, not on the wire, not in the client. The client stays a
  presenter.
- Evidence: the host prints the mode in its start line on stderr, and `--stats` (a new JSON field)
  and the GM7 artifact record it. The tick log format stays unchanged (`tick,wall_ns,unit_id,x,y`,
  `crates/turts_server/src/ticklog.rs:4`; readers such as `tests/browser/gm40-oracle.js:209` need 5
  fields on every line; Lena). A replay takes the mode from the same setting: tests set the
  resource, process runs set the variable. The command script gets no new field.

B. Solid and disclosed bodies (from the decided rules 1 to 6).
- Solid for every step, by the exact step check against the true world: terrain walls and cliffs,
  buildings, every body of another owner, and own units on Hold.
- "Friendly" in rules 1 and 2 means the same owner. A push moves only the pusher's own units, so
  no player moves another player's unit (authority).
- Steering reads only own units and CURRENTLY permitted foreign bodies. A disclosed foreign body
  is a stationary obstacle (S:150). A hidden body exerts no force. Contact with it stops the step;
  the contact is a permitted fact for that unit only (S:74).
- Own idle units (not on Hold) are displaced by the overlap relaxation and stay where it leaves them
  (rule 1). Settled units of an order shift at most a bounded distance (proposal: one radius per
  settle event) to make room (rule 2). Units of a later order push settled idle units of earlier
  orders as idle units (rule 2).
- Overlap between own MOVING units is allowed and capped (FM3). A unit settles only at a pose with
  zero overlap. If the relaxation cannot reach zero, the unit keeps moving; it does not settle.

C. Geometric ties.
- All corrections come from one frozen snapshot and are applied together, so equal geometry gives
  equal and opposite corrections. No order, id, slot or hash decides.
- Two units with the same centre have no geometric separation direction: separation gives them no
  correction; each moves by its own goal velocity. They cannot settle until they are apart.
- Two units that reach the crowd of their order in the same tick at the same distance both arrive.
  An area target has no exclusive point, so no tie winner is needed.

D. Acceptance and the removal build.
- On the switch build (mode `flock`): iterate GM1 to GM6 and FM1 to FM3, the playable browser
  check, and the wire measurement of step 3. Also one diagnostic GM7-class run without sudo after
  step 3, to find budget trouble early. It is not acceptance.
- Then delete the old path, the switch, the resource and the variable. The removal build is the
  only build whose results count: GM1 to GM8 and FM1 to FM3 once, GM7 under B3 in one operator
  window, GM8 in fresh silent browsers.

## History: Kenji's trace of Jade's findings on the old path (554775ac)

Record: `target/mc-evidence/ridge-return-repro/RESULT.md` (15 run logs; private instrument on the
generated GM2-narrow fixture, not the bytes of `mods/quarry_ridge`). Kenji's figures; I did not re-run them.

- First-motion delay (finding 3): CAUSE LOCATED. All 36 members stay `Searching` from tick 1 to about
  600-800; the first unit moves at tick 703. `expansions_this_tick` is at its cap (64) in every sampled
  tick from 3 to 700, and again from 1380 to 1700 after the dig. Caps `MAX_EXPANSIONS = 128 / VIEWERS`
  and `MAX_EDGES = 1024 / VIEWERS` (`terrain/navigation/shared.rs:27-28`); 7 route fields for one order.
  On the return order, 15 members with a direct line start at +4 to +7 ticks, the other 20 at +606 to
  +771. Cause: the search work budget, with no movement before the field is complete. The flock removes
  it by rule 6 and the coarse field.
- Return jam (finding 1): REPRODUCED; it depends on the return point. At 120 s, completed members by
  point: (10,24) 35 of 35, (14,24) 35, (18,24) 34, (22,24) 14, (26,24) 22. At (22,24) after 200 s, 21
  units keep an active order in and east of the passage; two large units ended their order INSIDE the
  passage (packed stops); the digger rests in the passage. Bodies touch and never overlap (minimum gap
  0.000). Controls: the packed stop makes the jam worse but is not the only cause (with it off, 24
  complete; with it off and the digger in the order, all 36 at +1005 ticks).
- One Walker that did not start (finding 2): not reproduced as seen. NOT TRACED.

Design consequence for T-7 (Viktor): a unit never settles, and an idle unit is never left, in a passage.
A field cell whose clearance is less than two diameters of the largest size class that uses the field
is a passage cell. A unit in a passage cell does not settle (it keeps moving with the flow), and an
idle own unit in a passage cell is pushed out by rule 1. This applies decided rules 1 and 2 and Clara's
"the blob does not close the passage exit"; it adds no player rule.
