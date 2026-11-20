# Actual-work bound for blocked recovery

Status: Baseline (implemented; one independent review, repairs in section 6). Date: 2026-11-19. Track: Foundation
(it changes audited movement state). Source: `target/mc-evidence/handoffs/2026-11-19-recovery-actual-work.md`.
Base: `4db1211a` on `codex/dense-acceptance-integration`. Branch: `codex/recovery-actual-fable`.
This note revises `2026-11-18-bounded-recovery.md` sections 2 (R1, R5, R8), 3.3 and 4. The
other sections stay in force.

## 1. Problem

The 2026-11-18 allowance limited a public charge, not the real work. One 32-unit slice of a
scene with 40,000 tiny bodies that do not overlap read 219,852 operations (probe commit
`5178aa1c`). Each candidate ran a full grid query over every body in reach, hidden bodies
included, before the charge was taken. The digest, the pool slot search, the claim digest
and the tick-start overlap query had the same defect.

## 2. Contract

A1. **Metered work.** Every read of a known fact in a recovery slice costs one unit: a cell
row, a cell, a body key, a candidate or pool slot, a claim check, a claim cell, a contact,
a digest start. The meter refuses a unit when the allowance is spent. The metered work of a
slice is never above its allowance. The metered work of one owner in one solver pass is
never above `OWNER_WORK` (32,768). The pool precheck (`has_potential_free_slot`) is
metered too: two units for its constant-time part, then one unit for each slot-grid cell
and each slot of its local part. It resumes from the pool cursor (`at`, `item`, `local`).

A2. **Known bodies only.** A slice reads the owner view (own bodies and bodies that the
visibility basis permits; all bodies without a basis) and the contacts of its actor (bodies
in contact range at tick start that the view does not hold). This is the same set as
`known` in `frame.rs`. Hidden bodies out of contact are not in either set, so they change no
result, charge, slice end, proof or schedule. One input is outside these sets: the slot
clearances that the pool precheck reads (section 5).

A3. **Frame work.** Three costs are charged to no owner and are counted in
`RecoveryCounters::frame_work`: the owner view build (one membership check for each row and
owner that has a recovery row), the contact query of each searching row (one grid query of
reach `radius + max radius + CONTACT_GAP`), and the tick-start overlap query of each
scheduled row. Each is one query for each row and pass, not one for each candidate. This is
the same class of cost as one physical step of a mover. Limit: this work is not bounded by
`OWNER_WORK`; it grows with the bodies near a searching row.

A4. **Resumable queries.** A point query visits the view cells of its box in `(cy, cx)`
order and the keys of a cell in `(ordered x, ordered y, radius bits)` order, then the
contacts in key order. Equal keys are one entry with a count. A paused query stores the
cell row, the cell column, the last key and the next contact (`RecoveryQuery`). No unit id,
row index or hash order decides an order.

A5. **Proof.** Each view cell keeps an order-free digest of its multiset of keys. The
frame updates it on each move of a member. The proof of a paused scan is the sum of the cell
digests in the box `origin +- (examined + radius + view half)`, the contact digest, and the
claim-cell digests of the own pool in `origin +- examined` (claim cells are 4 units wide).
It costs one unit for each cell row and each cell. A different proof restarts the scan.
`RELAX_AFTER` (2) failed proofs, or a proof that costs more than half a slice, make the scan
relaxed. A relaxed scan resumes paused probes as they are, and checks the chosen point
against the current state before it commits (stage `Verify`). A paused check resumes only
with a local proof: the cell digests of its query box and the own-pool claim cell of the
point. A partial query never reports a point as clear.

A6. **Reserve.** Before a scan starts a candidate, it keeps a reserve that pays the proof
digest of a pause there. A slice that cannot keep the reserve and do one more unit pauses.

A7. **Schedule.** As 2026-11-18 R4, with two changes. A batch of rows with an equal service
key shares its credit only when each share is at least `MIN_SLICE` (64). Otherwise
`credit / MIN_SLICE` rows of the batch get `MIN_SLICE`, from a start that moves by that
count each tick; every row of the batch is served within `ceil(rows / served)` ticks. The
allowances of one owner never sum above `OWNER_WORK`. The tick-start overlap key counts only
a body that `known` admits.

A8. **Cell size.** The view half is the smallest power of two at least as large as every
member radius (at least 1/64); the cell is twice the half. A scan stores the cell bits of
its proof. The cell size changes only when the largest known radius crosses a power of two.
A different cell size voids a paused query cursor and counts as an invalidation. A proving
scan restarts. A relaxed scan keeps its place and restarts only its paused query.

A9. **Memory.** Retained: `RecoverySearch` is 200 bytes (was 88), plus the unit id. Phase
(`phase_bytes`, an upper bound, not a heap measurement): the schedule, each owner view
(twice the entries and cells, for B-tree nodes at half fill), the contact list of a row,
the start grid, the claim set with its cell digests.

A10. **Replay.** The new `RecoveryScan` fields have serde defaults. The live server and the
auditor run the same kernel on the same frame, so they rebuild the same views, contacts and
slices. A log written before this change does not replay to the same result: the slices
differ.

## 3. Liveness

- Stable world: a served job progresses in each slice. Each unit of work moves a cursor
  forward, and no step is repeated except one cell row and one cell entry for each resume.
- Equal keys: see A7.
- Churn in the examined region: the job restarts at most `RELAX_AFTER` times, then runs
  relaxed. A relaxed job ends when its chosen point stays free during the check. The check
  resumes only when its own query box and the claim cell of its point are unchanged. If
  that box changes in every slice and the check needs more than one slice, the job does
  not end. This is the liveness condition: the query box of the chosen point must be
  stable for one check.
- Cell size changes: each change counts as an invalidation (A8), so after `RELAX_AFTER`
  changes the job is relaxed and keeps its place. Only a query that needs more than one
  slice and sees a new cell size in each slice does not end (the same liveness condition).
- A row whose allowance cannot pay the reserve of a proof runs relaxed (A5, A6).

## 4. Tests

`crates/turts_sim/src/movement/tests/recovery_actual_tests.rs`: R1 counterexample with
known bodies (each slice within its allowance, stateless result) and with hidden bodies
(same work as an empty scene); R2 a cell larger than a slice; R3 stale proof rejection and
end after stabilization; R4 equal-key batches; R5 hidden non-contact bodies change no owner
output or counter for 60 frame ticks under fog (red on the base: owner actual work 21,618 to
550,761 at tick 0); R6 contact and visibility; R7 reversed ids, rows and owner slots; R8 a
hundred blocked group movers. Repair tests (section 6): R9 a paused check sees a new claim
or body on its point; R10 a cell size that changes in every slice; R11 a pool precheck
larger than a slice; R12 a move earlier in the tick is in the view; R13 the mover does not
block a pool slot under itself; R14 a new claim in the examined region invalidates the
proof. R6 also asserts that a contacted hidden body gives the escape of the same body when
visible. `recovery_view.rs` tests: key order, cell digests, a probe
that resumes inside one dense cell. The 2026-11-18 tests in `recovery_tests.rs` stay, with
these changes: one-candidate slices use the test hook `SLICE_CANDIDATES` (a slice must pay
its reserve, so an allowance of 1 cannot progress); allowances below `MIN_SLICE` are raised
to it; the stress bounds use A1 and A9.

## 5. Limits

- Frame work (A3) is not bounded by the owner allowance.
- The pool precheck reads clearances that `refresh_slot_clearance` builds. On this branch,
  under fog without claim exchange, they include bodies that `known` does not admit, so a
  hidden body on the only free slot can change a group escape (pre-existing; an exception
  to A2). Integration commit `f98dc856` ("Keep hidden bodies out of fog slot prechecks")
  restricts these clearances under fog. The two branches merge with no text conflict, and
  the merged tree passes the `turts_sim` tests (section 6). This branch does not repeat
  that change, so it does not conflict with it.
- The stall spare-slot search (`nearest_free` after a stall) is counted but not metered
  (pre-existing, 2026-11-18 section 5).
- Retained-target checks and the exit overlap checks stay single physical queries for each
  row and pass (not metered).
- The region proof uses whole cells, so a move in a cell next to the examined region also
  restarts a scan.
- The digest is a sum of two 64-bit mixes and a count; a collision is possible (near
  2^-64 for each check).

## 6. Review and repairs

One review (`simulation-core-engineer`, runs and mutants) of `e2e5d8e5` gave
`VERDICT=FAIL FINDINGS=6 HIGH=1`. The same reviewer checked the repairs at `0ffe97b4`:
`VERDICT=PASS REPAIRED=5/5 NEW_FINDINGS=1` (Low, closed in item 2). Repairs:

1. High. A paused check of a relaxed point did not see a new claim of the point: the
   local proof read bodies only. The local proof now adds the own-pool claim cell of the
   point (A5). Test R9 (red on `e2e5d8e5`).
2. A cell size change restarted the scan with zero invalidations, so a far body that joins
   and leaves the view in turn stopped the search. It now counts as an invalidation (A8,
   section 3). Test R10 (red on `e2e5d8e5`: no end in 2,000 slices). The re-check found
   that R10 stayed green when the cell change was not counted, because its proof became
   relaxed by cost. R10 now also runs a cheap proof (allowance 1,024, one candidate in each
   slice); that case is red without the count.
3. The pool precheck was forced work and could use the proof reserve
   (`debug_assert` in the pause proof). It is now metered and resumable (A1). Test R11
   (red on `e2e5d8e5`: the assertion fires). `Meter::force` is removed.
4. Five mutants survived the tests. Each one now turns a test red: no `relocate` (R12),
   no local proof (R9), own key counted (R13), no contacts (R6), no claim digest (R14,
   R9).
5. The A2 and section 5 contradiction: A2 now names the exception; section 5 names the
   integration repair.
6. A 6-tick output cycle of 40 stacked escapers (review probe P5): the base `4db1211a`
   gives the same trace, hash for hash. It is pre-existing, not caused by this change.

## 7. Amendment: the work outside the owner allowance (2026-11-19, branch `codex/recovery-gm7-unblock`)

Source: `target/mc-evidence/handoffs/2026-11-19-recovery-gm7-unblock.md`. This section
replaces A3 and the second, third and fourth items of section 5.

### 7.1 Three classes of work

- **K (known).** A read of a fact that the owner knows: a view cell or key, a contact, a
  claim, a pool slot. Metered by the row allowance. Never above `OWNER_WORK` for one owner
  in one pass (A1).
- **P (physical).** A *neighbourhood read*: one `UniformGrid::visit_within` on the base
  grid of the frame at one point, with reach `radius + max radius + CONTACT_GAP` at most.
  It reads hidden bodies. It is never charged to an owner and never refused.
- **V (view).** The build of the owner view: one membership check for each row and one
  insert for each member, one time for each owner that has a served row, in each pass.

### 7.2 Rules

B1. **Retained-target checks are K.** The check of a retained escape target (physical
escape `target_occupied`, group `is_free` with its claim check, plain `body_occupied` of the
target) is a metered point query on the known set of the row (`KnownSet::probe`), with the
same predicate and tolerance as before. `target_is_blocked_by_stationary` is removed: it is
implied by `is_free`. When the meter refuses, the row holds its position for this tick.
With no stored scan, the row stores the query as a scan in stage `Verify` (the cursor and
the local proof of A5) and resumes it. With a stored search scan, the check runs first and
is not stored: when the slice cannot pay it, the stored search continues without it. A
failed check starts the search in the same slice.

B2. **The stall spare-slot search is K.** A stalled group mover runs it as a metered,
resumable job of kind `RecoveryKind::Spare(pool)`: the pool stage of `EscapeScan` with no
ring stage. The result is the result of `MovementPoolIndex::nearest_free` with the same
predicate. In claim exchange mode a body of another owner blocks a slot only when it was
at rest at tick start and the owner has a body in contact range of it (the rule of the
old predicate); the view marks the other known bodies of other owners as *soft*, and a
spare query ignores soft copies. A row that may reach the spare search in this tick (group
progress, not blocked, stall clock at tick start at least `tick_hz - 1`, fewer than 3
spare claims) is a schedule row of class *spare*. Blocked rows are served before spare
rows in both tiers. A spare row with no allowance or an unfinished search keeps its stall
state and its job; the job ages, so service rotates.

B3. **One schedule read, and contacts from it.** Each schedule row does one neighbourhood
read at tick start, before any row moves. It gives the tick-start overlap key and the
contacts of the row (bodies in contact range that the owner view does not hold). The
start grid and the sort by unit id are removed: before the first move, the base grid holds
the tick-start positions (`arrival::prepare_with_kept` changes no position).

B4. **Fixed count of neighbourhood reads.** In one pass a schedule row does at most: one
schedule read (B3); one exit read at its turn (the physical overlap of a blocked group row
or of a physical-escape row); one read at its new position (a physical-escape row that
moved). So: physical escape 3, blocked group 2, blocked plain 1, spare 1. No read for each
candidate, slot or claim.

B5. **Counters.** `RecoveryCounters` holds `physical_reads` (count), `physical_work`
(cells and bodies of the reads) and `view_work` (membership checks and inserts).
`frame_work` is their sum. `spare_slot_work` is the metered work of spare jobs; it is
inside `owner_work`.

### 7.3 Bound

For one owner in one pass:

- K work is at most `OWNER_WORK` = 32,768. It now includes B1 and B2. Fixed overshoot: 0.
- P reads are at most 3 for each schedule row of the owner. One read costs the cells of
  its box (at most 16 with the reach of 7.1) plus the bodies in them.
- V work is at most one membership check for each row of the frame plus one insert for each
  body that the owner knows.

P and V are not bounded by a constant. This is a limit of the design, not an omission:

1. *An exact physical read cannot have a cap.* The exit of a blocked row and its contacts
   are physical facts. To find them, the frame must test each body near the row. A cap
   has two results only. The row waits: then a hidden body that is near the row, and not
   in contact, changes its schedule. Or the read stops: then a contact or an overlap is
   lost, and a body can stop on a hidden body. Both break a contract of this spec. One
   neighbourhood read is the cost of the physical step of any mover of the base frame
   (which does three or more for each mover and tick), so a blocked row does no more
   physical work than a row that moves.
2. *A stateless kernel cannot have a known-body index for less than one read for each
   known body.* `step_frame` gets the bodies of one tick and keeps nothing. A lazy index
   of the cells that a slice reads must filter the base grid cells, and then each hidden
   body in the search region costs a read (this was the 2026-11-18 defect, and a meter on
   it stops the slice of an owner because of hidden bodies). The view build reads no
   position of a hidden body.

Memory: as A9, less the start grid; plus the contact rows of the schedule rows (4 bytes
for each contact) and the soft flags of each view (1 byte for each row).

### 7.4 Privacy

A hidden body out of contact changes no K work, no allowance, no slice end, no result and
no owner counter. It changes `physical_work` and the membership part of `view_work` only.
A hidden body in contact range at tick start is a known body of that actor (A2), for the
retained check and the spare search as for the search.

### 7.5 Liveness

As section 3. In addition: a retained check with a cursor ends when its query box is
stable for one check; without a cursor it cannot stop the stored search. A spare row is
served within `ceil(waiting rows / served rows)` ticks after the blocked rows of its owner
have no fresh job.

### 7.6 Tests

`recovery_actual_tests.rs`, R15 to R24: R15 a retained target in a cell larger than a
slice (group, physical, plain) resumes and ends; R16 a retained check sees a new body or
claim on its target while paused; R17 the spare search gives the slot of `nearest_free`
(visible, soft, contacted and hidden bodies on slots; exchange mode on and off); R18 a
spare search larger than a slice resumes; R19 more spare rows than slices: each is served,
and blocked rows first; R20 hidden-world pairs: hidden bodies near and far, never in
contact, change no owner output and no owner counter for 60 ticks, for blocked group,
plain, physical and spare rows; R21 a hidden body in contact gives the same result as the
same body when visible, for the retained check and the spare search; R22 the K work of an
owner is at most `OWNER_WORK` in a scene with blocked and spare rows, and the reads of
each row are within B4; R23 a stop order, a new order and a death drop the job (spare and
retained); R24 reversed unit ids, row order and owner slots give the mirrored result, and
the audit replay kernel gives the same jobs and outputs as the live frame.
