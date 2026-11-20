# Bounded recovery contact reads

Status: Baseline (implemented; one independent review, section 8). Date: 2026-11-20. Track:
Foundation (it changes the cost of audited movement reads; it adds contacts that the old
read missed at the f32 edge of the contact rule; the metered cost of the pool stage
changes; section 3). Source:
`target/mc-evidence/handoffs/2026-11-20-bounded-contact-design.md`. Base: `5f18ef1c` on
`codex/dense-acceptance-integration`. Branch: `codex/bounded-contact`. This note replaces
the bound of `2026-11-19-recovery-actual-work.md` section 7.3 and changes 7.1 (class P),
B3, B4 and B5 as stated here. The other rules stay in force.

## 1. Problem

Section 7.3 said that one neighbourhood read visits at most 16 cells. That is false.

- The schedule read had the reach `radius + max radius + CONTACT_GAP`. The base cell is
  `2 * max radius`. `CONTACT_GAP` is 0.101 and does not scale with the bodies. With bodies
  of radius 0.001, one read visited 10,201 cells and tested 9,801 bodies
  (`target/mc-evidence/recovery-frame-bound-audit/tiny-radius.log`). A frame with 5,041
  tiny bodies and 100 recovery rows did 826,378 units of physical work and took 3.1 s.
- The schedule read tested every body in that reach: own and permitted bodies too. Those
  bodies are never contacts (a contact is a body that the owner view does not hold).
- The grid computed its cell boxes in f32 and added an absolute margin of 1e-4. At large
  coordinates the f32 box grew (25 cells at x = 8.4e6, `float-cell.log`). For a tiny reach
  the absolute margin made the box many cells wide.
- An overlap read is a yes or no answer, but it tested every body in its box. A stack of
  N escaping bodies did N tests for each read of each row.

## 2. Rules

G1. **Grid boxes.** `UniformGrid` computes cells and query boxes in f64, with a margin
of `1e-4 * reach` (relative), and at least `2^-70` (below `2^-75` a squared f32 distance
underflows to 0, so the filter accepts such points for any reach, also reach 0). Each f32 subtraction is correctly rounded, so the f32 filter
`distance_squared <= reach * reach` accepts a point only within `reach * (1 + 2^-21)` on
each axis; the margin is larger. So every accepted point is in the box. A read whose reach
is at most the cell size scans at most 4 cells on each axis, 16 cells, at every finite
coordinate. A coordinate whose cell index is outside the i32 range saturates; those points
share an edge cell, and the exact filter still decides. Limit (as on the base): a reach
above about 1.8e19 has an infinite square; the filter then accepts points that the box does
not hold. Movement reaches are bounded by the map size.

G2. **Two schedule reads.** Before any row moves, each schedule row does:

- the *overlap read*: one read of the base grid with the reach `radius + max radius` (the
  reach of an overlap). It gives the tick-start overlap. It reads no contact range.
- the *contact read* (fog only): one read of the *hidden index* of the owner with the reach
  `(radius + hidden max radius + CONTACT_GAP) * (1 + 1e-6)`. It gives the bodies in contact
  range (`touch`) that the owner view does not hold. These are the contacts of section 7,
  B3, unchanged.

The hidden index of an owner holds the frame rows that are not members of its view, at
their tick-start positions. Its cell is `(max radius + hidden max radius + CONTACT_GAP) *
1.001`, so a contact read scans at most 16 cells (G1). It holds no own and no permitted
body, so their density costs a contact read nothing. It is built for each owner with a
schedule row, one time in a pass, and dropped when the schedule reads end.

With no visibility basis there is no contact read: every body is in the view.

G3. **Overlap reads stop at the first overlap.** The overlap read, the exit read and the
read at a new position (`overlap_read`) stop at the first body that overlaps the row
(`UniformGrid::any_within`). The answer is the answer of a full scan; only the work
changes. A stack of bodies that overlaps the row costs one test.

G4. **One membership pass.** Under fog, the view membership of each row for an owner is
computed one time in a pass and shared by the hidden index and the owner view.

G5. **Counters.** `contact_reads` counts the contact reads; `contact_work` counts their
cells and hidden bodies, and is inside `physical_work`. `physical_reads` counts the
overlap-class reads and the slot reads, as before (the count of B4 is unchanged: the
contact read is not in it). `view_work` adds one insert for each row of a built hidden
index. `phase_bytes` adds the membership vectors and the hidden indexes.

## 3. What changes and what does not

Each grid query gives the same points as before (every point that the f32 filter accepts
is in the box). The overlap flags are the same.

The contact sets follow the contact rule of `known` and `touch`
(`distance <= radius + other radius + CONTACT_GAP`) exactly. The old read was not exact at
the f32 edge: it first filtered with `distance_squared <= (radius + max radius +
CONTACT_GAP)^2`, and when `other radius == max radius` that filter can reject a body that
`touch` accepts. The new reach has the factor `1 + 1e-6`, so the contact read finds that
body. In such a scene the escape changes: it is now the escape of the same body when
visible, as section 7 (A2) requires. Test:
`a_hidden_body_at_the_f32_edge_of_contact_range_is_a_contact` (review finding 1).

One metered cost changes. The pool stage and the pool precheck of a recovery job walk the
cell box of the slot grid (`EscapeScan::pool_levels` and `precheck`, `UniformGrid::cell_box`)
and pay one unit for each cell. G1 makes that box tighter for a reach below 1 and removes
the f32 growth at large coordinates. So these stages can cost fewer units (K work 132 to
117 in the tiny spare scene of section 7), and a slice can end at another point. The cost
depends on slot positions only, so privacy does not change. The live server and the
auditor run the same kernel, so replay stays deterministic. A log written before this
change does not replay to the same result when a slice end moves, and its paused pool
cursor (`RecoveryPoolCursor::at`) indexes other cells.

- Test oracle (`recovery_oracle::check_schedule_read`, test builds only): at each schedule
  read of a frame of at most 4,096 rows, it tests every row of the frame and compares the
  overlap flag and the contact set. All `turts_sim` tests run it.
- `tests/frame_hash.rs` gives the same hashes on the base and on this branch.

## 4. Bound

For one owner in one solver pass, with `N` frame rows and `S` schedule rows of the owner:

- K (known) work: at most `OWNER_WORK` (32,768). Unchanged.
- V (view) work: at most `N` membership checks, one insert for each member of a built view
  and for each row of a built hidden index (together at most `N`), and two units for each
  move of a member of a built view. So at most `2N + 2M`, where `M` is the moves of members
  in the pass. There is no density term.
- P overlap-class reads: at most 3 for each schedule row (section 7, B4), plus the slot
  reads of B4. Each scans at most 16 cells (G1). It tests the bodies in those cells until
  the first overlap (G3).
- P contact reads (fog only): one for each schedule row. Each scans at most 16 cells (G1)
  and tests the hidden bodies of the owner in those cells only.

Memory of a pass: as section 7.3, plus one byte for each row and each owner with a
membership vector, plus the hidden index (`HiddenIndex::bytes`, from capacities: 4 bytes
of row and 12 bytes of point for each hidden body, plus `UniformGrid::bytes`). The index is
dropped when the schedule reads end. Under fog, the hidden indexes of all owners with a
schedule row exist together during the schedule reads: at most `owners * N` entries.

## 5. What has no constant bound, and why

Two terms of P work are still not bounded by a constant. Both read physical facts that the
contracts need.

1. *Hidden bodies near a row under fog.* A contact read tests each hidden body in its 16
   cells. Adversary: put M hidden bodies just outside the contact range of a row. To give
   the exact contact set, a read must test each of them: if it skips one, the same read
   cannot tell that body from one that is just inside. A read that stops early either
   loses a contact (a body can then stop on a hidden body, collision safety fails) or makes
   the row wait (then a hidden body out of contact changes its schedule, privacy fails).
   An index built for each pass does not remove this cost without an exact structure for
   additively weighted discs (an Apollonius diagram); that is not a practical change.
2. *Bodies in 16 cells that do not overlap the row.* The base cell is `2 * max radius`.
   With one large body and many small ones, 16 cells can hold many small bodies. Every
   neighbourhood read of the movement kernel has this term; the kernel reads have larger
   reaches (`radius + max radius + 0.05 + max step`).

A constant bound needs a rule change that a player can see. Options for the CEO:

- (a) *Touch reveals.* A hidden unit that touches a unit of a player becomes visible to
  that player. The movement kernel then needs no contact read. Cost: the visibility phase
  marks the contact range of each unit (a raster, bounded by radius and cell size).
- (b) *Contact by touch only.* A blocked unit treats a hidden unit as an obstacle only after
  that unit stops one of its steps (the physical guard already reads it). Escapes under fog
  then avoid hidden units one tick later.
- (c) *A minimum footprint.* A radius floor of about `CONTACT_GAP / 2` limits the bodies that
  do not overlap in a contact range. Stacks of overlapping bodies stay unbounded, so this
  alone is not enough.

This branch implements none of them. Note also that the exact contact set of one row can
itself hold M bodies (a stack). Any exact design is bounded by its output size at best; a
constant bound was never possible without a rule change.

## 6. Limits

- The two terms of section 5.
- The movement kernel outside recovery has the same density term and costs more than
  recovery in the tiny-footprint scene (section 7).
- A far hidden body can change the cell count of a contact read (at most 16), because the
  grid uses its sparse scan when the box has more cells than the index. This is P work
  only: no owner counter, result or schedule reads it.
- The f64 grid cells change the cost of every grid read of the frame, not its results.

## 7. Evidence

Files: `target/mc-evidence/bounded-contact/` (README there). Probe
`frame_contact_density_probe`: 5,041 bodies in a packed square, 100 recovery rows of owner
0, one tick. *Spare*: stalled movers; *blocked*: blocked group movers. Fog *visible*: every
other still body is of owner 1 and permitted; *hidden*: the same bodies are hidden, so the
rows touch them. Work is in units of the counters (one cell or one body test).

| Scene | Base `physical_work` | `bd0c46e0` `physical_work` (of it `contact_work`) |
| --- | --- | --- |
| radius 0.4, spare, no fog | 1,649 | 1,583 (0) |
| radius 0.4, spare, hidden | 1,677 | 2,788 (1,177) |
| radius 0.001, spare, no fog | 826,378 | 1,717 (0) |
| radius 0.001, spare, visible | 826,378 | 1,717 (0) |
| radius 0.001, spare, hidden | 851,083 | 239,637 (213,215) |
| radius 0.001, blocked, no fog | 828,287 | 3,434 (0) |
| radius 0.001, blocked, hidden | 828,287 | 216,649 (213,215) |

- K work (`owner`) is the same in the radius-0.4 scenes (759, 0) and in the tiny blocked
  scenes (0). In the tiny spare scenes it changes from 132 to 117 (no fog) and from 231 to
  216 (fog): the pool precheck reads fewer slot-grid cells (section 3).
- V work: 10,220 (radius 0.4) and 10,106 (radius 0.001) with no fog. Under fog a blocked
  scene now has 5,041 or 7,511 units of V work (the membership pass and the hidden index)
  where the base had 0: the base read the same rows in its schedule read as P work.
- Phase memory (`phase_bytes`): 723,633 bytes in the radius-0.4 spare scene (base
  799,257); 707,250 in the tiny hidden spare scene (base 738,809).
- Wall time does not change: the spare tiny scene takes 3.15 s on both builds. The time is
  in the movement kernel. The integration probe with ordinary movers and no recovery row
  takes 4.27 s on the base for the tiny scene (29 ms for radius 0.4). In the base profile
  of the tiny spare scene, 75 % of the cycles are in `UniformGrid::within_unsorted`, the
  general neighbourhood query of the kernel.
- The ordinary-mover tiny scene, base and final head interleaved: 4,479 and 4,351 ms
  (base) against 4,380 and 4,242 ms (head). No regression from the f64 grid.
- `tests/frame_hash.rs`: `frame_chains=c47f80851ee283b8`, `muster_world=980377f842f51f57`
  on `5f18ef1c` and on `bd0c46e0`.
- All 929 `turts_sim` unit tests and the 20 other `turts_sim` test binaries pass in release.
- Six one-line mutants each turn a new test red (`mutants.log`): the overlap read with
  `CONTACT_GAP`, a hidden index with members, a contact reach with no gap, the absolute
  grid margin, f32 cell boxes, and a full scan in `any_within`. The frame-level test at
  large coordinates stays green on the f32-box mutant; the grid test catches it.

## 8. Review

One review (`simulation-core-engineer`, runs and mutants) of `bd0c46e0` gave
`VERDICT=FAIL FINDINGS=6 HIGH=0`. It confirmed: `any_within` matches a full scan; a box of
reach at most one cell has at most 4 cells on each axis; the contact read tests only hidden
bodies; R5, R7 and R20 and three new hidden layouts keep owner outputs and counters;
`frame_hash` is equal; the section 5 argument holds. Repairs:

1. Medium. The contact sets differ from the old read at the f32 edge (the new read is
   exact, the old one missed contacts). Section 3 now says so. New test with the edge body
   (red when the reach factor is `1 - 1e-6`).
2. Medium. Reach 0 or an underflowing reach missed points that the f32 filter accepts.
   The box now has a floor of `2^-70` (G1). New grid test (red with no floor).
3. Medium. No test checked the 16-cell bound of the contact read (the sparse scan hid
   it). New test with 1,600 hidden bodies round one row (red when the index cell is a
   quarter).
4. Medium. The oracle skipped frames above 4,096 rows, so R5 (4,176 rows) was not checked.
   The limit is now 8,192. The edge test of item 1 makes the oracle go red.
5. Low. The view work, the bytes and the shared membership pass had no test, and
   `HiddenIndex::bytes` undercounted. Bytes now come from capacities
   (`UniformGrid::bytes`). New test (red with no view work for the index, zero bytes, or
   no membership cache).
6. Low. A reach above about 1.8e19 overflows (as on the base). Stated in G1 and the code.

The review also asked whether a hidden body just out of contact changes an escape. A probe
at seven angles round the free escape direction (distance one f32 step above the contact
reach) found a change only where `touch` reports contact, because the f32 position falls
inside the reach. Bodies with no contact changed no own output. Each repair mutant turns
its test red (`target/mc-evidence/bounded-contact/mutants-repair.log`).

The same reviewer checked the repairs at `ecc7a9ab`: `VERDICT=FAIL REPAIRED=5/6
NEW_FINDINGS=0`. Its brute-force probe found no miss in 200,000 grid queries (reach 0,
subnormal reaches, coordinates up to 3e38) and no box wider than 4 cells. It refuted its
own question: its comparison held the output of the hidden body itself; with that output
left out, a body out of contact changes no escape. The open item (Low): `UniformGrid::bytes`
counted the cell map about 8 % low (it used `capacity()`, which is 7/8 of the buckets).
Repair: the map is counted as a power-of-two bucket table with one control byte for each
bucket and a 16-byte group; a test checks 270,352 bytes for 5,041 cells, the value that
the reviewer measured with a counting allocator. This item is closed without a further
review (light rule: one check of the repaired items).
