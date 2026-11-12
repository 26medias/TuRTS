# Pre-MP1 slice: technical-director ruling on the gate findings (one step function, optional pool)

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=RULED

**Short answer:** H1 is real, and my A2 input was wrong. I rule for a fourth option: one per-unit step function moves every unit, and a pool is optional data on the unit. With this option the re-freeze still changes only the `acme` scout. M2 is a change that a player sees, and it is the behaviour the MC design wants, so the CEO must be told. The grid edge miss is a defect and gets its own repair step.

Work tree: /home/julien/Projects/TuRTS-mp1. I did not run cargo. Probe numbers come from the gate's raw files in the scratchpad (`stream_plain.txt`, `all.txt`, `zz_grid_probe*.txt`). I read those files; I did not run the probes again.

**Gate findings I verified:**
- H1: three branches exist.
    - The plain-progress branch is at `movement.rs:1843-1879`.
    - The no-progress branch is at `:1880-2085`. It falls back to `make_plain_progress` at `:1893` and `:2053`.
    - The pool branch starts at `:2086`.
    - The trip writers that the gate names do write `MoveTarget`: `gather.rs:1261`, `orders.rs:1004, 2304, 2459, 2505-2578`, `command.rs:495`, `synthetic.rs:72,92`.
    - Probe file: all four packages run only on the no-progress branch (`acme` 122, `base` 256, `haul` 230, `cedar` 289 unit-ticks).
- M1: the pool of a lone unit is collected in its arrival tick (`movement.rs:2383-2404`, `:2497-2508`). My input line about a resting pool entry is refuted.
- M5: Orders runs before Movement (`plugin.rs:97-98`).
- M4:
    - `grid.rs:63` uses `ceil(reach/cell)`.
    - Two callers have zero margin (`movement.rs:1704-1705`, `:2308-2309`, `>= r + other.r`).
    - The probe file shows 43 misses, 41 results that change with the cell size, and a largest miss of 7.15e-7. With the fix: 0 misses.
    - There are 27 production `within` sites in movement.rs. My count of 21 was wrong.
- A float difference between the branches: the no-progress branch caps speed with `velocity.normalize() * step_limit` (`:2003-2004`); the pool branch uses `velocity *= step_limit / velocity_len` (`:2183-2185`).

## Ruling 1 (H1): option (d), one step function and an optional pool

**What "one model" must mean:**
- One function moves every unit that has a `MoveTarget`. It does arrival, steering, projection, stall and `Blocked`, and the obstacle fallback.
- The M4 route follower and the M4b push go only into this function. This gives one place for every writer: player orders, bot orders, gather, attack approach, follow, resume, queued starts and synthetic units.
- A pool (`MoveProgress` and `MovementPool`) only chooses the destination slot and turns on the group rules.
- An order makes a pool: every envelope move, the plain move of MC design 6.1 (`:568`), and `CommandKind::MoveTo`. This is how "a plain move is a pool of one" holds.
- An engine-made trip has no pool. When its goal is occupied (`:1882-1892`), or when it is pushed and does not arrive (`:2052`), the step function gives it a pool of one. It uses the same spread code. `PlainMoveProgress` is deleted.

**Options I did not take:**
- (a) Keep the branch. M4 routes and the M4b push would then go into two step functions. That is the exact cost the CEO decision removes.
- (b) A pool for each writer. Each pool adds a `MovementPoolIndex` for each frame (`:1404-1408`) and a clearance refresh of 7 slots (`:1484-1491`). That cost would apply to every gather trip and to the 5,000 synthetic units. All four streams would change, and a lone engine trip does not need the group rules.
- (c) Player moves only. This deletes `PlainMoveProgress` but keeps two step functions.

**Re-freeze size:** only `acme` changes. I keep the float expressions of the no-progress branch, because the stream runs on that branch.

**Proof:** a call counter, and the classifier with a new P0 check (section 5 below).

## Ruling 2 (M2): this is the design rule, and the CEO must know

- MC design 6.3 step 2 (`:612-614`) reserves "the live slots of moves in progress".
- GMU (`:1253`) names "slots reused" as a fault.
- So the pooled result is what the design wants: the first unit keeps P, and the later unit takes a hex slot. The plain path today departs from this only because MC kept the plain path to protect the frozen stream (K19 a).
- The slice text "changes no rule that a player sees" becomes false.

Sentence for the CEO: "After the slice, when you send a unit to a point that another moving unit already goes to, the new unit stops in a free place next to the point and the first unit keeps the point (before, the first unit gave the point up); a harvester or other engine trip that finds its point blocked uses the same free-place rule."

Alternative I did not take: keep the plain behaviour through a special case. That would be a second rule.

## Ruling 3 (M4): a defect repair, step G1, before everything in the sim

**Fix:** take the cell range from the bounding box of the query disc, made larger by 1e-4 x max(1, reach). The distance filter does not change.
- Alternative I did not take: `ceil+1`. At reach/cell = 1 it raises the scanned cells from 9 to 25, which works against A3. Use `ceil+1` only if the bounding-box form fails the property test, and record its tick cost.

**Cheap check whether it changes anything:**
1. Run the property test against a brute-force scan: 0 misses and 0 results that change with cell size.
2. Add a temporary counter (not committed) that counts the points the fix adds. Run `m3_m2_stream`, `forge_fixed_point`, and the `turts_sim`, `turts_pack` and `turts_server` tests.
3. If the counter shows 0 hits, no golden test can change. G1 then lands first.
4. If the stream has hits, G1 joins A2c. The trace then gets an `edge_hit` flag, and P0 and P1 accept a difference only on a flagged query.

## Ruling 4: order of the parts, and what the gate checks again

**Order:** the depletion repair is merged (gate L2) → G1 → S1 → S2, S3 → A1 → A2a → A2b → A2c (the one re-freeze) → A4 → A3. S4 and S5 run in parallel.

A4 becomes small. A2a already makes the step for one unit. A4 extracts what remains (index build, escapes, write-back, resting collection) as functions on `&mut FrameState`, called in one loop in unit-id order (gate L1).

**The gate checks again:**
- the writer table, against a grep of `MoveTarget` writers;
- that the A2a reference copy includes no-progress, plain-progress, escape, `Blocked`, follow and Attack-Point units;
- that the call-counter test can go red;
- the classifier and its self-test;
- the G1 property test and the counter result;
- the text of section 1, and that the CEO is told before A2b.

Lenses: `simulation-core-engineer` and `qa-lead`.

## Ruling 5: what changes in my MP1 parts B and C

- E2: change "a second movement path" to "a second step function".
- Rally: an order point, so units sent to it get a pool.
- Repair, construction and attack approach: trips with no pool. They use the same step function.
- C3: `frame.rs` becomes per-unit functions (L1). I wrongly listed `:1885-1960` for deletion; that range is in the branch that is kept and merged.
- A3: the grid change gives results that do not depend on the cell size only after G1.
- The cap: only `CommandKind::MoveTo` skips it today (`orders.rs:1190,1265`).
- Parts B1 to B14, except rally and repair, do not depend on the refuted claims.

## Replacement text for the slice design

**Section 1. Why this slice exists**

The CEO decided on 2026-11-11 (`vault/Decisions/2026-11-11 MC architecture audit.md`):

1. The economy log validator checks ledger conservation only, and moves to a `turts_audit` crate with one file for each domain.
2. One movement model: a plain move is a pool of one. The M2 stream is frozen again one time.
3. Both before MP1.

"One movement model" means this (technical-director ruling, 2026-11-12):

- One step function moves every unit that has a `MoveTarget`. It does arrival, steering, projection, stall and `Blocked`, and the obstacle fallback. M4 routes and the M4b push go into this function only.
- A pool (`MoveProgress` and a `MovementPool`) is optional data on the unit. It chooses the destination slot and turns on the group rules (MC design 6.3 to 6.5).
- An order makes a pool: every move envelope, the plain move of MC design 6.1 (`:568`), and `CommandKind::MoveTo`.
- An engine-made trip has no pool: gather trips, queued starts, the attack approach, follow, resume and `--synthetic-units`. When such a trip finds its goal occupied (`movement.rs:1882-1892`), or is pushed and does not arrive (`:2052`), the step function gives it a pool of one with the same spread code.
- `PlainMoveProgress` is deleted.

The slice also holds three items of the MC architecture audit and its gate:

- the neighbour grid cell must not depend on the largest unit;
- `within` must find every point inside `reach` (`grid.rs:63`);
- the files that every MC task edited are split, so that MP1 lanes do not wait for each other.

The slice changes one behaviour that a player can see, and no other. Before: when a unit was sent to a point that another moving unit went to, the first unit gave the point up. After: the first unit keeps the point, and the later unit takes a free slot next to it (MC design 6.3 step 2, `:612-614`). An engine trip that finds its goal blocked uses the same free-slot rule. Above 2,048 single moves in one tick, the rest start one tick later (6.3 work cap). The CEO is told before step A2b.

The slice adds no mod API. It is not part of MC. The MC frozen stream hash `bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b` stays until step A2c.

**Section 2. Parts and order**

Order: the depletion repair (`codex/mc-fix-deplete`) is merged -> G1 -> S1 -> S2, S3 -> A1 -> A2a -> A2b -> A2c with the one re-freeze -> A4 -> A3. S4 and S5 run in parallel with all other parts.

| Part | What | Kind | Proof |
| --- | --- | --- | --- |
| G1 | `within` scans the cells of the bounding box of the query disc, made larger by 1e-4 x max(1, reach) (`grid.rs:61-70`) | defect repair | Property test against a brute-force scan (24,000 queries, whole-number reach/cell included): 0 misses, 0 cell-size-dependent results. An edge counter on the stream and the golden tests: 0 hits, or G1 moves into A2c (section 5) |
| S1 to S5 | As in revision 1 | pure move | As in revision 1 |
| A1 | As in revision 1 | test oracle | As in revision 1 |
| A2a | One step function for the no-progress branch (`movement.rs:1932-2085`) and the pool branch (`:2086-2380`). Where they compute one value with different float expressions, keep the no-progress expression (for example `:2003-2004`). No writer changes | refactor | Reference copy of the old kernel, bit-equal on seeded frames (no-progress, plain-progress, pooled, escape, `Blocked`, follow, Attack-Point units). Stream unchanged. GM40, GMA, GMU and `group_movement.rs` green; each changed assertion has a reason. The call-counter test |
| A2b | The obstacle fallback (`:1893`, `:2053`) makes a pool of one with `spread_destinations_with_context`. Delete `step_plain_progress` (`:671-905`), `make_plain_progress` (`:925`), `PlainMoveProgress` (`:58`) and the plain reservations in `orders/`. `PlainGatherGoal` (`:63,79-82`) moves to `MoveProgress` | sim behaviour for a blocked trip | Stream unchanged (the streams spend 0 unit-ticks on `PlainMoveProgress`). Guards of section 6 |
| A2c | The plain move (`orders.rs:2822-2900`) and `CommandKind::MoveTo` (`command.rs:488-495`) take the pool path at dispatch | player-visible change of section 1; audit operations for a single move | Section 5 |
| A4 | The rest of `step_frame_preserving_resting` becomes functions on `&mut FrameState`, called in one loop in unit-id order | refactor | Reference copy, bit-equal; stream unchanged |
| A3 | Two neighbour grids (movers, statics); combat grid cell from the smallest weapon range | performance only | Reference copy, bit-equal; property test; RUN126 probe time recorded |

A pure move is checked with `git diff -M --color-moved=dimmed-zebra --color-moved-ws=allow-indentation-change`. No line outside `mod`, `use` and visibility lines may show as added or removed.

**Section 5. The one re-freeze (A2c)**

0. If the G1 edge counter has hits on the stream, G1 lands in A2c. The trace then records `edge_hit` for each query, and P0 and P1 accept a difference only on a flagged query.
1. After A2b, `qa-lead` adds a trace that is not frozen:
    - for each tick and unit: the position bits, the target, the activity, the movement state (no pool or pool) and the `separated` flag;
    - for each operation: the unit and the phase.
   It runs on the build before A2c, and its sha256 is logged.
2. The builder lands A2c and does not touch the fixture.
3. A classifier (Python standard library) checks:
    - P0: for `base`, `haul` and `cedar`, the trace before and after is equal byte for byte.
    - P1: in `acme`, every position is bitwise equal over 400 ticks.
    - P2: in all four packages, every economy event kind falls on the same tick.
    - P3: every added or changed operation is in `acme`, belongs to the scout, has no event kind, and falls on the accept tick or on the tick after arrival (Orders runs before Movement, `plugin.rs:97-98`). Expected: one added operation at tick 1 and one at the tick after arrival (gate probe: arrival 123, operation 124).
    - An independent calculation of the arrival tick, `ceil(|P - p0| / (speed x dt))`, must equal the arrival tick in the trace.
4. Classifier self-test. Each of these must give red: one flipped position bit; one added `Stored` event; one changed operation in `base`; one scout operation on the arrival tick itself.
5. The technical-director reviews the classified difference. The CEO approves the old and the new sha256 in the vault, together with the player-visible change of section 1. Then the fixture and `refutations.py:18` change in one commit.

Any difference outside P3 is a defect until it has a cause. "Regenerate with the current code" is refused. G1, A2a and A2b change no frozen value. If the stream test goes red on one of them, that step stops for a new ruling.

**Section 6. Guards (A2)**

- `one_unit_alone_takes_the_point` (`group_movement.rs:1309`) stays. It gains these cases: through `MoveTo`; through an envelope with group 0; with a static unit of radius 4 far away; to a point off the map (the clamped point, bitwise).
- Second move to a held point (gate case S12). Unit 1 moves to P; unit 2 gets a move to P while unit 1 still moves.
    - After A2c: unit 1 rests on P, bitwise. Unit 2 rests at least s - 0.00001 from P (s = 2 x r_max + 0.1). No footprints overlap.
    - The guard is red on the build before A2c. Both runs are recorded.
- One step function (A2a). A counter only in test builds (the pattern of `movement.rs:13-21`) counts the calls to the step function for each unit and tick.
    - The scenario has: a gather trip, a queued start, an attack approach, a follow, a resume, a single envelope move, a group move and a synthetic unit.
    - It asserts one call for each moving unit in each tick.
    - It is red on a build that keeps a second step branch.
- Blocked engine trip (A2b). Two gatherers deliver to one receiver in the same tick: both deliver, no footprints overlap, and each delivers within 2 x (distance / speed) + 5 s. A delivery after node depletion is also checked (gate L2).
- Pool collection. After a single move arrives: 0 `MoveProgress` and 0 pools (`movement.rs:2383-2404`, `:2497-2508`).
- G1. The property test against a brute-force scan.
- T7 (the unit rests within 0.011 of the ordered point) stays.
- The tests that pin plain moves are listed in the input (part A2). Each one is kept, inverted with a reason, or deleted with a reason, in the plan.

## Not traced
- Whether a stream unit is ever `separated` (P0 decides).
- How many tests pin bitwise positions of pooled units in contact (A2a lists them).
- Whether `queue_unallocated_move_orders` (`orders.rs:1416`) gives a pool to queued group moves.
- Whether the bounding-box form of G1 passes the property test.

## What I changed
No repository file. I updated my memory note `/home/julien/Projects/TuRTS/.claude/agent-memory/technical-director/project_mp1_input.md` with a correction block.

## Open questions for the orchestrator
- The CEO must get the sentence of ruling 2 before A2b.
- The gate reported an extra file in the work tree, `crates/turts_sim/tests/zz_mp1_obstacle_probe.rs`. It is not mine.
