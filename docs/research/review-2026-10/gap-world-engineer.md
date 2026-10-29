VERDICT=fail ITEMS=45 MISSING=22 PARTIAL=22 CONFLICT=0 RUNS=6 NOT_TRACED=4

Seat: world-engineer. Areas: group movement, pathfinding as the player sees it, collision, air and naval units, cliffs and high ground, map format, map editor and generator, map metadata and playable bounds, neutral objects. Second on 6.12.
Statuses: S:624-630 (13.6) is Baseline. D63 (S:86) is Baseline. S:156 (4.3 table) is Decided principle. S:211-215 (5.6) is Decided. S:113 is a deferral. S:720 (16.1) is Decided. M4 draft is DRAFT, not gated.
Term sweep output: `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/review/we/terms.out`.

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A-GM01 | Destination spreading | missing | "spread", "destination slot", "40 units": 0 hits in S. S:630 and S:653 name "shared destinations" as a test scenario only. | M | none | M4b (new movement milestone after M4) | world-engineer-1 | yes: `command.rs:239` gives every unit the same `MoveTarget` | Probe: 40 units end on 1 point, 780 overlapping pairs (run 1). |
| A-GM02 | Formations | partial | S:156 "formation primitives" (Decided principle, no design); S:624 says flow fields "do not solve formations". | B | none | after M4b, before 16.2 | A-GM01, A-GM03 | no | M4:267 "no formations". |
| A-GM03 | Speed matching | missing | "speed match", "slowest": 0 hits. | M | none | after M4b | world-engineer-1 | NOT TRACED | Needs a group identity; none exists. |
| A-GM04 | Local avoidance | partial | S:624 "bounded-neighbour local avoidance" (Baseline, evaluate only); S:86 D63 "bounded local steering". | M | none (M4:22 defers collision) | M4b | A-GM06 | yes: `movement.rs:42-56` straight line, no neighbour query | 13.6 frames it as cost, not as a player outcome. |
| A-GM05 | Flocking | partial | S:86, S:624 "steering"; "flock", "boids": 0 hits. | M | none | M4b or M12 | A-GM04 | no | Implementation candidate inside A-GM04. |
| A-GM06 | Unit collision size | partial | S:156 "collision"; S:626 "actor-radius classes"; S:630 "mixed footprints". | B | boot only | M4b | none | yes: radius exists (`schema.rs:28-29`), used only at boot (`map.rs:120-124`), not in movement | Data is there; movement ignores it. |
| A-GM07 | Push idle friends | missing | "push", "shove": 0 hits in S. | M | none | M4b | A-GM04 | no | |
| A-GM08 | Air separation | missing | "air unit": 0 hits; S:113 "flying units ... require a separate design". | M | none | air-layer milestone after M4b | A-UN02 | no | |
| A-GM09 | Clean arrival | missing | "arriv": 0 hits. | M | none | M4b | A-GM01 | yes: `movement.rs:50-52` drops target per unit, no group stop rule | |
| A-GM10 | Unreachable fallback | missing | "unreachable", "nearest reachable": 0 hits in S. M4 K8 (M4:34) keeps the order and holds; M2:274 "order waits visibly". | M | M4 draft: hold | M4 (rule), M4b (group) | A-PF04 | no | M4 K8 freezes the unit at the start. Goal wants nearest reachable point. |
| A-GM11 | Choke traffic | partial | S:624 "deadlocks"; S:630, S:653 "chokepoints" as tests. | M | M7/M12 benchmark | M4b | A-GM04 | no | Test named, behaviour not. |
| A-GM12 | Formation facing | missing | no input term for drag-facing in S. | M | none | after A-GM02 | A-GM02 | no | Input side belongs to the control seat. |
| A-GM13 | Worker collision exception | missing | M2:274-275 "nodes have no collision body"; S: 0 hits. | B | none | M4b | A-GM06 | no | Needs a mod flag per unit type. |
| A-PF01 | Sensible path | partial | S:624 hierarchical routes (Baseline, evaluate); S:703 routing Open. | M | M4 draft A* | M4 | terrain grid | no | M4:173-176 designs grid A*. |
| A-PF02 | Repath on change | partial | Terrain: S:455-466, S:630 (Baseline). Buildings: not stated. | M | M4 terrain only | M4 terrain; buildings with world-engineer-4 | world-engineer-4 | no | |
| A-PF03 | Movement layers, cliffs | partial | S:156 "movement categories"; S:470 traversal by unit type; S:706 slope rules Open; S:113 air deferred. | B | M4 draft `max_climb` | M4 ground; later air/water | world-engineer-2, world-engineer-3 | no | A cliff is a height step above `max_climb`. D09 allows it. |
| A-PF04 | No stuck units | partial | S:630 "defined temporary movement behavior"; "stuck": 0 hits. | M | M4 draft | M4b | A-GM04 | no | |
| A-PF05 | Instant start | missing | S:243 "immediate input acknowledgement" is network-only. S:630 caps searches with "pending states". M4:178-179 "4 searches per tick", "a unit that waits holds position". | M | none | M4 (rule) | none | no | Probe: 40 units: last unit waits 10 ticks = 500 ms at 20 Hz; 256 units: 3,200 ms (run 2). |
| A-PF06 | Clearance | partial | S:459, S:626; M4:36 K10 defers it. | M | none | M4b | A-GM06, world-engineer-2 | no | |
| A-PF07 | Gate passage | missing | "gate": only S:664 (selection gates). | B | none | with construction | world-engineer-4 | no | |
| A-PF08 | Terrain speed | missing | "terrain speed": 0; "road" hits are noise (S:29 "broad"). S:156 "speeds" is mod policy, not per-terrain. | B | none | after M4 | A-PF03 | no | |
| A-PF09 | Turn radius | missing | "turn rate", "turn radius": 0. S:171 lists heading on the server. | B | none | M4b | A-GM04 | NOT TRACED | |
| A-PF10 | Mass pathing | partial | S:624-630 Baseline; S:653 scenario; S:703 Open. | M | M7, M12 | probe with M4b, measure at M12 | A-GM04 | no | Run 2: per-unit A* for 40 units = 1,093,817 expansions; one shared reverse search = 65,054. |
| A-PF11 | Route display | missing | no hit for path display in S. | M | none | OPTIONAL, late | A-PF01 | no | |
| A-UN02 | Air units | partial | S:113 "possible ... separate design". | B | none | air-layer milestone | world-engineer-2 | no | M4:103 makes `max_climb` mandatory for every mover: blocks air. |
| A-UN03 | Naval units | missing | "naval", "amphib": 0; "water" hits are utilities (S:398, S:522). | B | none | after world-engineer-3 | world-engineer-3 | no | |
| A-UN04 | Amphibious, hover | missing | "hover", "amphib": 0. | B | none | with A-UN03 | A-UN03 | no | |
| A-MP01 | Map format | partial | S:179 "maps" as definitions; S:213 "serializable map state". No field list. | B | M1-M3 `MapDef` | M4 | none | yes: `MapDef` = id, size, placements, nodes, slot_economy, belts (`schema.rs:455-464`) | M4 adds `terrain`. |
| A-MP02 | Map editor | partial | S:447 "instant edits ... map editing"; S:720 (Decided) no large editor precommitment. | M | none; M4:22 defers instant edits | after M4, as agent tools | A-MP01 | no | Tension with D43; see Conflicts. |
| A-MP03 | Random map generator | present | S:211-215 (Decided principle). | B | no milestone; S:694 "proposed" | after M5 | M5 secrecy | no | No milestone validates reproducibility. |
| A-MP05 | Map metadata | missing | "preview", "map metadata": 0. S:288 player count is a room field. | M | none | M8/M9 (map choice) | A-MP01 | yes: `schema.rs:455-464` has no name, author, player count, preview | |
| A-MP06 | Neutral objects | partial | S:213 "vegetation" (generator output). "neutral", "doodad", "destructible": 0. | B | none | after world-engineer-4 | world-engineer-4 | no | |
| A-MP08 | Map validation | partial | S:213 "optional start/accessibility policies" in libraries. | M | none | M4 (reachability query) | A-PF01 | no | |
| A-MP09 | Playable bounds | missing | "playable" hits are other senses; "bounds" are asset bounds (S:225, S:610). | B | none | M4 | A-MP01 | yes: `MapBounds { size }` only (`world.rs:41-43`) | |
| B-7 | Group movement, no clumping | missing | as A-GM01, A-GM04. | M | none | M4b | A-GM01, A-GM04 | yes: as A-GM01 | Player-visible defect, run 1. |
| B-40 | Formation move | partial | S:156. | B | none | after A-GM02 | A-GM02 | no | |
| B-47 | Sight blocked by height | partial | S:461 refresh on terrain change; S:618 "terrain occlusion" named. M4:22 defers LoS. | B | none | M5 or vision milestone | world-engineer-6 | no | High-ground rules are mod policy on an engine query. |
| world-engineer-1 | Group order identity | missing | `MoveTo` has unit ids and one target, no group id (`command.rs:257-294`, `command.rs:239`). | M | none | M4 (carry the id) | none | yes: `command.rs:239` | Spreading, speed match and shared search all need it. |
| world-engineer-2 | Movement class as a traversal key | partial | S:156 "movement categories"; S:626 "actor-radius classes". M4:95 puts `max_climb` per unit type. | B | none | M4 | none | no | Shared caches key on a class, not on a unit type. |
| world-engineer-3 | Water surface on the heightfield | missing | no water level in 9.1 (S:432-436). | M | none | naval milestone | A-MP01 | no | A per-cell water level is not a second ground surface (D09). |
| world-engineer-4 | Static footprints block paths | partial | S:158 "occupancy"; S:628 "dynamic obstacle". M4:173 edge rule uses height only. | M | none | M4 | A-GM06 | no | Under M4, units walk through `survey_post`. |
| world-engineer-5 | Map as content apart from the game package | partial | S:179; D38 (S:61). | B | none | M8 | A-MP01 | yes: one map per package (`turts_pack/src/lib.rs:16-22`) | Map pools and map download need it. |
| world-engineer-6 | Height-aware gameplay queries | partial | S:461; M4:192 lists 2D-only range, gather, spawn. | M | none | after M4 | M4 heightfield | yes: `combat.rs:165-192` NOT TRACED by me (cited from M4:192) | High ground needs an elevation query. |
| world-engineer-7 | Optional map markers and regions | missing | S:332 rejects mandatory start positions as engine rules; map data markers not stated. M4:85 adds `zones`. | B | M4 draft zones | M4 | A-MP01 | no | Markers are data; use is mod policy (D27, D28). |
| world-engineer-8 | Map-keyed terrain and navigation state | missing | S:374 map-local terrain and navigation (Baseline). M4:114 `Heightfield` is one singleton resource. | M | none | M4 (key it) | D38 | yes: `MapBounds` singleton (`world.rs:41-43`) | Cheap now, costly after M4. |

## Conflicts with the goal

- No Decided item blocks the six-game union in my areas. I did not reopen any item.
- D43 / S:720 (Decided): "Do not precommit to a large manual editor suite." A-MP02 is CORE for a moddable engine. This is a tension, not a block: S:720 lets tools emerge from need. Needs decision on when a map-authoring tool becomes a need.
- D10 / S:106, S:113 (Decided): no walk-under bridges, no underground unit movement. A burrowing unit or a tunnel-transport link can still be a hidden state or a network link, not a space. Game facts for burrowing units: NOT TRACED.
- M4 K8 (draft, not Decided) conflicts with A-GM10. M4 validation rule M4:103 (draft) blocks air units.

## Lens answer

Q1. No document plans what the player sees when 40 units go to one point. S:630 and S:653 name "shared destinations" as a cost and congestion test. S:624 says flow fields do not solve formations. S:156 names "formation primitives" with no design. M2:274 and M4:22 exclude avoidance and collision. Today 40 units end on 1 point with 780 overlapping pairs (run 1).

Q2. The M4 draft is an extendable foundation, not a dead end. Reasons: the goal stays a per-unit `MoveTarget` (M4:170), so a group layer can give each unit its own slot. `Route` per unit (M4:171) is a path corridor; steering can sit in Movement after `Navigate` (M4:206). The NAV check is about cells per tick (M4:296), so steering inside traversable cells keeps it green. Radius data exists (`schema.rs:28-29`).
Risks: per-unit A* plus a FIFO cap per unit gives a 500 ms start delay for 40 units and 3,200 ms for 256 (run 2). It also costs 16.8 times the expansions of one shared search on 256 x 256 (run 2).
Smallest changes (direction only): (1) carry a group or order id from `MoveTo` to the search queue, so one request serves the group. (2) Keep the group target and the unit target apart in the text. (3) Key traversal on a movement class, and require `max_climb` for ground movers only. (4) Let a queued unit start to move, and do not make "hold at start" (K8) an expected result in tests. (5) Key `Heightfield` and navigation state by map id. (6) Treat static footprints as blocked cells.

## What I ran

1. `cargo test --release --test stack` in `scratchpad/review/we/stackprobe` (path deps on `turts_sim`, `turts_mod_api`) | claim: M1-M3 group `MoveTo` stacks units (`command.rs:239`, `movement.rs:42-56`) | refute: distinct positions > 1 | observed: `units=40 distinct_positions=1 overlapping_pairs(radius 0.5)=780`.
2. `python3 -I scratchpad/review/we/m4_group.py` | claim: M4:173-179 rules give start delay and repeated search cost | refute: last search tick <= 1, or per-unit expansions near shared | observed: 10 ticks (500 ms) for 40, 64 ticks (3,200 ms) for 256; 1,093,817 vs 65,054 expansions. `distinct_final_points=1` is from M4:176 by construction, not tested. Output: `scratchpad/review/we/m4_group.out`.
3. `python3 -I scratchpad/review/we/terms.py` | claim: S has no text for spreading, arrival, push, unreachable, naval, metadata, neutral | refute: any hit | observed: 37 zero-hit terms; list in `terms.out` line 1.
4. `rtk proxy grep -n -w -i -E "formations?"` on S | claim: "formation" hits are not "information" | observed: S:156, S:624 only.
5. `rtk proxy grep -rn -i -E "radius|footprint|collision|separation|avoid"` on `turts_sim/src`, `turts_mod_api/src` | claim: radius is not used in movement | refute: a hit in `movement.rs` | observed: hits only in `map.rs`, `bootstrap.rs`, `registry.rs`, `boot_m3.rs`.
6. `grep struct MapBounds` and read of `turts_pack/src/lib.rs:14-24` | claim: bounds = size only; one map per package | observed: `world.rs:41-43`; `lib.rs:16-22` error "exactly one map".

## NOT TRACED

1. `combat.rs:165-192`, `gather.rs:438-456`, `production.rs:419-470` 2D-only (taken from M4:192).
2. M4 cost probe numbers (M4:188-191).
3. Turn and heading fields in sim code (A-PF09, A-GM03).
4. Burrowing-unit facts in the named games.
