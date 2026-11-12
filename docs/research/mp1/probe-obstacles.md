# MP1 probe MP1-P1: units and buildings in the way

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=PASSABLE-WITH-LIMITS FINDINGS=4 HIGH=2 RUNS=8 NOT_CHECKED=3

Mode: test. The probe is at `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/mp1-obstacle-probe/zz_mp1_obstacle_probe.rs`. The full output is `run3.txt` in the same directory. Settings: 20 Hz, speed 2.4, 112 x 72 map, nonzero group (the interaction core sends nonzero groups, `turts_interaction/src/lib.rs:1344`). The limit is 3 x d/v + 10 s. "late" means last arrival / straight-line time.

**Scenario table**

| Scenario | Done | Blocked (distance to point) | Not ended | Late |
|---|---|---|---|---|
| S1: 1 unit, r2.5 static, offsets 0, ±0.5, ±1.5, ±2.5, ±3.0 (group 7 and group 0) | 18/18 | 0 | 0 | 1.00–1.10 |
| S2: 20 x r0.4, offsets 0, ±1.5, 3.0 | 80/80 | 0 | 0 | 1.05–1.22 (control 1.03) |
| S3: base, gap 1.0 to CC, 0.78 between satellites: 20 inf / 1 tank / 5 / 20 tanks | 20 / 1 / 4 / 16 | 0 / 0 / 1 / 4 (29.6–33.0, stuck in the base) | 0 | 1.22–1.43 |
| S3: gap 2.0, 1.46 between satellites | 20 / 0 / 1 / 10 | 0 / 1 / 4 / 10 | 0 | 1.04–1.28 |
| S3: gap 0.5 (the proposed minimum) | 13 / 1 / 4 / 17 | 7 / 0 / 1 / 3 | 0 | 1.16–1.44 |
| S3: full ring, gap 1.0 | 20 / 0 / 2 / 11 | 0 / 1 / 3 / 9 | 0 | 1.07–1.26 |
| S3: every gap ≥ 2.0 | 20 / 1 / 5 / 20 | 0 | 0 | 1.06–1.40 |
| S4: real Gather, r0.9 harvester, r1.5 pylon mid-path, offsets 0, ±1, ±2, 2.5 | 20/20 trips each | 0 | 0 | cycle 1.00–1.27 x the free 14.5 s |
| S4: pylon 1.0 or 2.0 from the CC on the line, or 1.0 from the CC at offset ±2 | **0 trips** | Blocked at 9.3–9.9 s; gather cleared | – | – |
| S4: pylon 2.5, 3 or 4 from the CC; or 1.0 from the CC at offset 3 or 6 | 20/20 trips | 0 | 0 | 1.00–1.20 |
| S5: click on the r2.5 centre: 1 unit (group 7 and group 0) / 20 units | Done | 0 | 0 | ring at 3.12 / at most 3.60 |
| S6: waypoints, 1 unit | 3/3 Done | 0 | 0 | 1.03 |
| S6: waypoints, 20 inf (single static) / base 20 inf / base 20 tanks | 17 / 15 / 11 | 3 / 5 / 9 (0.9–5.6, near the point) | 0 | 1.52–1.78 |

**a. What moves a unit round a static today.** A static is a frame unit with no speed (`movement.rs:1412-1419`, `2434`). Four steps act on it:
- Separation, plus a push to the right when the neighbour is ahead (`movement.rs:2158-2169`; plain path `783-789`).
- Hard projection out of the footprint (`2192-2225`).
- The stall rule: 20 ticks with less than 1 % progress per step (`2272-2281`), then up to 3 spare slots (`2286-2335`), then `Blocked` (`2337-2343`).
- For Gather, the drop-off target is the ring point that faces the harvester (`gather.rs:1126-1149`, `660-672`). A harvester that comes to rest out of range is `Blocked`, and its gather is cleared (`orders.rs:785-819`, `865`).

This works for any single circular static. I measured r2.5 with every offset from −3 to +3, for 1 and 20 units. It fails in two cases. First, in a notch between two footprints whose gap is smaller than the unit diameter. Second, when a static is within r_h + r_s of the drop-off ring point.

**b. Is this acceptable for MP1?** Single buildings: yes. Units come late, but they come. Compact bases: no. With gaps below the unit diameter, 20–50 % of tanks stay `Blocked` in the base, and so does 1 lone tank. With the proposed 0.5 gap (`input-game-designer.md:146`), 7 of 20 infantry stay too. A power station 1–2 from the CC on the ore side gives 0 trips. That stops the economy, and a player would call the game broken.

**c. The smallest engine changes (none built; the sizes are NOT TRACED estimates).**
- C1, a free drop-off point. `approach_position` takes the nearest ring point that no static blocks, for the receiver and for the node. About 40 lines, 1 file and 1 test. It is not thrown away at M4, because the goal stays `MoveTarget` (`m4-terrain-design.md:175`).
- A tangent steer buys nothing. Single statics already pass, and a tangent steer cannot get a unit out of a notch.
- C2, a route round statics. Stamp the footprints, grown by the unit radius, onto the M4 navigation grid. Search with A* as in `m4-terrain-design.md:178-191`. The route waypoints feed `MoveTarget`. About 400–700 lines in 5–7 files: a nav module, dispatch, gather, `economylog` replay, and tests. This is the first step of **M4** navigation, not of M4b (M4b is collision, push and chokes, spec line 726). A visibility graph over circles would take about 300 lines, but M4 would throw it away.

**d. Rule and map changes, each with its cost.**
- R1. Every gap between buildings must be at least the largest mobile diameter + 0.2. For an r0.9 tank this is 2.0. Measured: 0 `Blocked`. Cost: bases are wider, and players cannot build walls.
- R2. Keep the drop-off side of the CC clear. No footprint may be within r_h + r_s of the ring point that faces each field. Measured: offset 3 passes and offset 2 fails; gap 2.5 passes and gap 2.0 fails. Cost: a placement clearance in the site check (X4), plus some layout freedom.
- R3. Smaller footprints do not remove notches. They are not enough alone.
- Hand waypoints are a weak mitigation. The group splits into 31–38 pools, so the GMU limit returns at the final point.

**Findings**
- HIGH. `docs/research/mp1/input-game-designer.md:146`: a 0.5 gap traps units. Fix: R1, or C2.
- HIGH. `gather.rs:1126-1149`: a building near the drop-off stops all gathering. Fix: C1, or R2.
- MED. `movement.rs:1587-1601`: a `Blocked` unit in physical escape walks up to 1.154 into a building. Design 6.5:679 permits this, but a player can see it.
- MED. Design 6.9:208-209 names M4b for the route step, but routing belongs to M4. Needs decision.

**What I ran**
- `cargo test --release --no-run` | the probe builds | a build error | exit 0
- run1 | the instrument sees Done for group 0 | rows open with the unit at its target | exit 0. It found an instrument defect: group 0 holds no `current`. I fixed the end test.
- run2 (first try) | exit 101. My fixture overlapped placements and `load_map` refused it. This was a fixture error, not a red.
- run2 | S1 to S6 | – | exit 0
- run3 | the deep overlaps are physical escape | an overlap with `escape=false` | exit 0. All 7 overlaps had `escape=true` and `Blocked`.
- run3 (first try) | compile error, exit 101. A compile error is not a red.
- `cmp r2.rows r3.rows` | the probe gives the same result twice | a difference | exit 0, identical
- `git status --porcelain` | exit 0. It shows one untracked file, `docs/research/mp1/gate-slice-simulation.md` (mtime 21:55). It is not mine, and I did not touch it.

The instrument can show each class: the controls give Done, S3 gives `Blocked`, and the first run1 bug gave open rows.

**Not checked**
- Approach angles other than head-on to the base arc.
- The `turts_server` process path.
- Units of other owners.

**What I changed:** none. I created the probe file, copied it to the scratchpad, and removed it. My own changes leave the tree clean.

**Open questions for the orchestrator**
- Is the route step M4 or M4b?
- Is the drop-off rule R2 a mod placement rule or engine fix C1?
- Who owns the untracked `gate-slice-simulation.md`?
