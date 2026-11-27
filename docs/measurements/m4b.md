# M4b dense group movement: results and limits

Recorded on 2026-11-26 for task T-45 on `codex/t7-flock-sim`. Final product code: c1dc1e60 (the E1 fix on
a6d54cee). The browser rows name their commit. The native GM1 to GM6 rows and the GM7 diagnostic are on c1dc1e60;
the GM7 run under B3 is on a6d54cee (with the test layout of 854e01e8). This page records results. It does not record an acceptance: a person accepts M4b.

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
- **Queues at a passage.** Of two own units that come too near, only the one whose step goes
  toward the other yields: the unit behind waits and the unit ahead goes on (c1dc1e60). Before
  it, a fast unit that closed in on a slow one stopped both, and the stop spread through a
  crowd at the mouth of a dug passage (FM2).
- **Cost.** The flock costs about 12 ms of a GM7 tick, down from about 31 ms. A flat grid
  snapshot, active-set push passes, reused buffers and an exact flat-ground shortcut in the
  terrain check do it, and the traces are byte-identical (127fa4e5).
- **Client.** The Spade drag area, the minimap orientation, and work clips that play only while
  a unit works (Marco's branch, merged in the candidate).

## Decisions

- Contested move points: arrive first. The unit id never decides a visible result (CEO,
  2026-11-12).
- One movement path, the flock (design record above). The Game Director's rules V2, V3, V5 and
  V6, and the busy-unit rule (vault, 2026-11-24). V6 refined on 2026-11-26: the place stays the
  first stop until a new order, a push never moves it, and a refused return waits a new quiet
  delay. A spot is free when no body stands nearer than the sum of the two radii.
- A move ends at a standing blocker as before. Keep-order (wait far from the goal) moves to T-59,
  with the local search (Iris and the Game Director, 2026-11-26).
- Ridge demo as standalone packages `mods/quarry_ridge` and `mods/terrain_stockyard_ridge`
  (`vault/Decisions/2026-11-23 M4b ridge demo as standalone packages.md`).

## Results

| Check | Evidence | Result on the candidate |
| --- | --- | --- |
| GM1 wide passage | `cargo test -p turts_server --release --locked --test terrain_group` (`gm1_wide_*`) | PASS on c1dc1e60 |
| GM2 narrow opening | the same suite, `gm2_*` (2 tests) | PASS on c1dc1e60 |
| GM3 shared, opposing, unique, blockers | the same suite, `gm3_*` (8 tests) | PASS on c1dc1e60 |
| GM4 edits during approach | the same suite, `gm4_*` | PASS on c1dc1e60 |
| GM5 memberships and replay | the same suite, `gm5_*` | PASS on c1dc1e60 |
| GM6 hidden terrain privacy | the same suite, `gm6_*` (2 tests); the server unit test driver | PASS on c1dc1e60 |
| FM2 return, server path | the same suite, `fm2_return` (10 seeded cases, order points offset by up to 0.05 m) | PASS on c1dc1e60; red on its parent (up to 32 units not idle) |
| GM7 frozen workload, B3 | `target/mc-evidence/t7-flocking/gm7-b3-candidate/runs/b3-2/` | PASS on a6d54cee, all 4 rows, 1000 intervals each (table below) |
| GM7 pinned diagnostic, no sudo | `target/mc-evidence/t7-flocking/gm7-diag-c1dc1e60/runs/A1/` | PASS on c1dc1e60, all 4 rows, 200 intervals each: p99 27.95 to 28.40 ms, max 32.48 to 38.21 ms, debt 0 |
| GM8 browser, 24 cases | `target/mc-evidence/t7-flocking/cand-c1dc1e60/`; known limits `.swarm/reports/T-42-known-limits-c1dc1e60.md` | c1dc1e60: 16 of 24 PASS, 8 FAIL; no stall, crash, overlap or wall fault recorded. Red: 3 cases where the first arrival ends more than one body diameter from its place (0.81 to 0.92 m; new on c1dc1e60; against the GM3 first-arrival rule); 4 settled-blocker cases where a large unit gives up its move at a standing enemy (T-59); 1 test click on a covered unit. |
| FPS, 400 detailed models | `cand-c1dc1e60/fps/` | c1dc1e60: PASS. Normal 48.6, near 56.2, far 48.0 FPS (floor 30); p99 33.4 ms in each view; at least 7 units in motion in each interval. |
| FM1 first motion | `cand-c1dc1e60/fm/` | c1dc1e60: PASS on both ridge packages (table below). |
| FM2 return crossing | `cand-c1dc1e60/fm/` | c1dc1e60: PASS. All 36 settle at 323 (quarry_ridge) and 324 (terrain_stockyard_ridge) ticks; 0 overlapping pairs at rest; no stall on the return. The earlier PASS line here was from d7ee8c72. With V6 before the E1 fix, FM2 failed (17 of 36 settle): cb031474 and 063bd531. |
| FM3 overlap while moving | `cand-c1dc1e60/fm/*/fm3.json` | c1dc1e60: PASS. Sampled maximum overlap / smaller radius (bound 0.25): 0.200 outbound on both; return 0.182 and 0.200. Sampled overlap at rest at most 0.0098 m (positions are rounded to 0.01 m). |
| Wide units dig (quarry_ridge) | `crawlers_dig_the_ridge_from_the_edge_of_their_reach`; browser check in T-42-cand-8fa203ab | 40 of 40 cells natively; 32 of 32 cells in the browser on 8fa203ab (code a6d54cee), not rerun on c1dc1e60. |

### First motion (FM1, c1dc1e60, 36 mixed units)

All times are in ticks from the applied order, at 20 Hz. Each value is quarry_ridge / terrain_stockyard_ridge.
Terrain seeds: 3113004681313666839 / 11574917217546001924. Host: i9-14900K, RTX 3080, Chrome.

| Row | Bound | Result |
| --- | --- | --- |
| First unit moves, outbound and return | 20 ticks (1 s) | outbound 0 / 0; return 1 / 0 |
| Units that move within 20 ticks, outbound | all | 36 of 36 (last at 11 / 12 ticks) |
| Units that move within 20 ticks, return | all | 36 of 36 (last at 12 / 11 ticks) |
| All 36 idle after the opening | (recorded) | 304 / 304 ticks |

The free-space clause of FM1 is judged by the headless check, not in the browser. On the old path, the first unit moved
after 809 / 810 ticks (T-49).

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

A person's sign-off of the milestone (spec section 14) is not recorded. The operator played the
ridge loop and skirmish on 2026-11-26 ("Flocking looks good"; "movements look and behave well,
the game is playable").

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
