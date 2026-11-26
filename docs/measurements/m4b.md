# M4b dense group movement: results and limits

Recorded on 2026-11-26 for task T-45 on `codex/t7-flock-sim`. Product code: a6d54cee. Test code: up to
854e01e8. This page records results. It does not record an acceptance: a person accepts M4b.

## What M4b delivers

- **One movement path: the flock.** Stages: flow fields per owner, class and goal; steering
  advice; guarded steps; the push between own bodies; the end-pose and swept conflict check (E1);
  settle and wait. The old solver, its audit and its traffic state are deleted (1962b8d3,
  4ad52808). Design: `docs/superpowers/specs/2026-11-23-flock-movement-design-record.md`.
- **The movement rules of the Game Director** (`vault/Decisions/2026-11-24 Movement rules V2 V3
  V5 V6.md`). V2: traffic means a wait, and no timer ends a move. V3: a passage holder, a wait
  at the mouth, and a head-on go-back. V5: the slower unit goes first. The coin is private
  (terrain seed), never the unit id. V6: an idle unit that own units push away goes back to
  the place where it first stopped, when the area is quiet and the place is free. Busy units
  (Hold, terrain work) stand firm.
- **Arrive first.** The first unit that arrives keeps a contested point. The unit id decides
  nothing that a player sees (CEO, 2026-11-12).
- **Terrain work for wide bodies.** The work approach tries the 8 poses that touch the cell,
  then a lattice of poses in the tool's reach. A walk that ends short, or that makes no
  progress for 3 s, takes the next pose. With no pose left, the unit shows Blocked
  (a6d54cee).
- **Cost.** The flock costs about 12 ms of a GM7 tick, down from about 31 ms. A flat grid
  snapshot, active-set push passes, reused buffers and an exact flat-ground shortcut in the
  terrain check do it, and the traces are byte-identical (127fa4e5).
- **Client.** The Spade drag area, the minimap orientation, and work clips that play only while
  a unit works (Marco's branch, merged in the candidate).

## Results

| Check | Evidence | Result on the candidate |
| --- | --- | --- |
| GM1 wide passage | `cargo test -p turts_server --release --locked --test terrain_group` (`gm1_wide_*`) | PASS |
| GM2 narrow opening | the same suite, `gm2_*` (2 tests) | PASS |
| GM3 shared, opposing, unique, blockers | the same suite, `gm3_*` (8 tests) | PASS |
| GM4 edits during approach | the same suite, `gm4_*` | PASS |
| GM5 memberships and replay | the same suite, `gm5_*` | PASS |
| GM6 hidden terrain privacy | the same suite, `gm6_*` (2 tests); the server unit test driver | PASS |
| GM7 frozen workload, B3 | `target/mc-evidence/t7-flocking/gm7-b3-candidate/runs/b3-2/` | PASS, all 4 rows (table below) |
| GM8 browser, 24 cases | `.swarm/reports/T-42-cand-8fa203ab.md`, `target/mc-evidence/t7-flocking/cand-8fa203ab/` | 7 of 24 PASS on 8fa203ab. The return-check and end-pose fails were check faults (repaired in test code, 0582fda5). Marco's rerun is open. |
| FPS, 400 detailed models | the same report | normal view 56.0 FPS (floor 30). The row needs a rerun: no unit moved in its last interval. |
| FM1 first motion | `.swarm/reports/T-49.md`, `.swarm/reports/T-42-d7ee8c72.md` | PASS on d7ee8c72. Not rerun on the candidate. |
| FM2 return crossing | the same reports | PASS on d7ee8c72: settle at 332 to 336 ticks, 0 stall intervals, 0 overlapping pairs at rest. Not rerun on the candidate. |
| FM3 overlap while moving | the same reports | PASS on d7ee8c72. Not rerun on the candidate. |
| Wide units dig (quarry_ridge) | `crawlers_dig_the_ridge_from_the_edge_of_their_reach`; browser check in T-42-cand-8fa203ab | 40 of 40 cells natively; 32 of 32 cells in the browser |

### GM7 under B3 (run b3-2)

The acceptance test `tw7_frozen_1000_warmed_intervals` ran under B3: the bench on CPUs 0, 2, 4
and 6, and the other workloads on CPUs 14 to 30. It used 1,000 measured intervals per row, the
binary sha256 `15ae9a79…`, and ran from 15:50:06 to 16:26:15. The bounds are p99 40 ms, p99.9
50 ms, max 100 ms and debt 50 ms.

| Row | p50 | p95 | p99 | p99.9 | max | max debt |
| --- | --- | --- | --- | --- | --- | --- |
| fog on, RTT 50 ms | 20.94 | 27.28 | 29.24 | 33.59 | 45.36 | 0.00 |
| fog on, RTT 100 ms | 21.05 | 27.37 | 29.51 | 34.53 | 44.68 | 0.00 |
| fog off, RTT 50 ms | 20.57 | 27.26 | 28.97 | 32.53 | 52.19 | 2.19 |
| fog off, RTT 100 ms | 19.98 | 26.42 | 27.73 | 29.14 | 34.13 | 0.00 |

Server tick times are in ms. The fog rows churned at least 69 hides and 73 reveals per interval (the bound is 20). The
first B3 run (b3-1) stopped in 0.6 s: the static enemies of each side stood on the other side's
scouting lane, and the flock does not walk round such a wall. The test layout now puts them
beside the route (854e01e8, test code only).

## Known limits

- **No local path round standing bodies.** A Move unit that a standing body stops ends its leg
  where it stands, also far from its goal, outside a passage. A player's queued legs can then
  end short (battle400 FPS row). The fix (keep-order) needs the local search and a rule for two
  units that wait for each other. Without them, it locks owners in a mutual wait.
- **The return to a place is not exact in a crowd.** A unit whose place a larger own unit took
  waits there. One small unit stays 3.7 m from its place in GM2-narrow. A returning unit can
  slide a standing neighbour by up to 0.23 m.
- Four of the 40 units of `cases_f_and_i` case I would wait at a standing enemy crowd if they
  kept their order.
- `STUCK_TICKS` of the work approach counts approach checks, not ticks.

## After M4b (task T-59)

The local path with keep-order and a rule for two units that wait for each other. The
acceptance cases: case I; the GM6 mirrored edits and `gm3_idle_big_unit_in_a_gap` with
keep-order on; the battle400 queued legs; the old TW7 layout with the enemy block on the lane.
Also: a return step that keeps a gap; a new place after a long wait (the Game Director's
rule); pose reservation for work approaches; tests for the `flat_box` margin and the server's
256-cell area limit. The details are in the T-59 comments of 2026-11-26.
