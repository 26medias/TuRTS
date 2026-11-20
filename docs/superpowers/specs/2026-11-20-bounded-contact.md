# Bounded recovery contact reads

Status: Baseline (implemented; one independent review, section 8). Date: 2026-11-20. Track:
Foundation (it changes the cost of audited movement reads; it changes no result). Source:
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
of `1e-4 * reach` (relative). Each f32 subtraction is correctly rounded, so the f32 filter
`distance_squared <= reach * reach` accepts a point only within `reach * (1 + 2^-21)` on
each axis; the margin is larger. So every accepted point is in the box. A read whose reach
is at most the cell size scans at most 4 cells on each axis, 16 cells, at every finite
coordinate. A coordinate whose cell index is outside the i32 range saturates; those points
share an edge cell, and the exact filter still decides.

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

## 3. Results do not change

The new reads give the same overlap flags and the same contact sets as the old read.

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
membership vector, plus the hidden index (`HiddenIndex::bytes`: 4 bytes of row, 12 bytes
of point and 64 bytes of grid entry for each hidden body). The index is dropped when the
schedule reads end.

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

This branch implements none of them.

## 6. Limits

- The two terms of section 5.
- The movement kernel outside recovery has the same density term and costs more than
  recovery in the tiny-footprint scene (section 7).
- A far hidden body can change the cell count of a contact read (at most 16), because the
  grid uses its sparse scan when the box has more cells than the index. This is P work
  only: no owner counter, result or schedule reads it.
- The f64 grid cells change the cost of every grid read of the frame, not its results.

## 7. Evidence

Filled at delivery (section 8 and `target/mc-evidence/bounded-contact/`).

## 8. Review

Filled after the review.
