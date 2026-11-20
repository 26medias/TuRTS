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
replaces A3 and the second, third and fourth items of section 5. Status: Baseline
(implemented at `591a9acd`; B6, B7 and the details of B1 and B2 were added during the
build, see the commit).

### 7.1 Three classes of work

- **K (known).** A read of a fact that the owner knows: a view cell or key, a contact, a
  claim, a pool slot. Metered by the row allowance. Never above `OWNER_WORK` for one owner
  in one pass (A1).
- **P (physical).** A *neighbourhood read*: one `UniformGrid::visit_within` on the base
  grid of the frame at one point, with reach `radius + max radius + CONTACT_GAP` at most
  (a slot read of B4: `2 * max radius` round the slot). It reads hidden bodies. It is
  never charged to an owner and never refused.
- **V (view).** The build of the owner view: one membership check for each row and one
  insert for each member, one time for each owner that has a served row, in each pass.
  Then two units (one remove, one insert, each with its digest update) for each move of a
  member of a view that is built.

### 7.2 Rules

B1. **Retained-target checks are K.** The check of a retained escape target (physical
escape `target_occupied`, group `is_free` with its claim check, plain `body_occupied` of the
target) is a metered point query on the known set of the row (`KnownSet::probe`), with the
same predicate and tolerance as before: it is `EscapeScan::verify_point` (a slot: the claim
check with the own claim allowed, tolerance 0; another point: no claim, tolerance 1e-5).
`target_is_blocked_by_stationary` is removed: it is implied by `is_free` (test in 7.6).
The plain check of the own position (`body_occupied(unit.position)`) is the same kind of
check. The part of the plain test that reads reservations
(`conflicts_with_reserved_filtered`) is not changed.

- *Pause.* When the meter refuses, the row holds its position and its target for this
  tick. With no stored scan, the row stores the query as a scan in stage `Verify` with
  ring 0 (`RecoveryScan::is_check`; the cursor and the local proof of A5) and resumes it.
  A stored check of a point that the row left is dropped.
- *With a stored scan.* With a stored search scan, or the stored check of the own
  position, the check runs first with at most half the allowance and is not stored: when
  the slice cannot pay it, the stored job continues without it (the check counts as
  failed, never as clear). A failed check starts the search in the same slice.
- *Reached.* The test that ends a blocked order at its escape point reads no base grid.
  Group: the retained check proves it; for a search result (a ring point has the ring
  tolerance) one metered query with tolerance 0 and no cursor. When the allowance cannot
  pay it, the row keeps its target and ends at its next exit read. Plain: the retained
  check or the search proves the body part, with the same tolerance.
- *No schedule read.* A plain row that becomes blocked in this tick is no schedule row. It
  has no allowance and no contacts, so it runs no K query: its checks prove nothing and
  the row waits for a slice, as before this amendment.
- *Idle rows.* A blocked plain row with no target, no job and no overlap at tick start
  that gets no allowance keeps no job (the check of the own position cannot start).

B2. **The stall spare-slot search is K.** A stalled group mover runs it as a metered,
resumable job of kind `RecoveryKind::Spare(pool)`: the pool stage of `EscapeScan` with no
ring stage. A search that ends in one slice gives the result of
`MovementPoolIndex::nearest_free` with the same predicate (the slot that the mover leaves
is never the answer; no claimed slot is). The origin is the next position of the mover, as
before. A search that continues keeps its origin while the mover stays within its radius
of it, so a stalled mover that creeps does not start again in each tick. In claim exchange
mode a body of another owner blocks a slot only when it was at rest at tick start and the
owner has a body in contact range of it (the rule of the old predicate); the view marks
the other known bodies of other owners as *soft*, and a spare query ignores soft copies
and soft contacts. A soft copy has its own digest fact, so a body that changes class
changes the proof. In claim exchange mode each owner has its own view, with or without a
visibility basis. A row that may reach the spare search in this tick (group progress, not
blocked, a target, stall clock at tick start at least `tick_hz - 1` or a spare job, fewer
than 3 spare claims) is a schedule row of class *spare*. Blocked rows are served before
spare rows in both tiers. A spare row with no allowance or an unfinished search keeps its
order, its stall state and its job; the job ages, so service rotates.

B3. **One schedule read, and contacts from it.** Each schedule row does one neighbourhood
read at tick start, before any row moves. It gives the tick-start overlap key and the
contacts of the row (bodies in contact range that the owner view does not hold; 4 bytes
for each). The start grid and the sort by unit id are removed: before the first move, the
base grid holds the tick-start positions (`arrival::prepare_with_kept` changes no
position; a debug assertion in `step_frame` checks each solver row against the lookup by
unit id that `known` uses). A row turn builds its contact keys from these rows at their
positions then.

B4. **Count of neighbourhood reads.** In one pass a schedule row does at most:

- *Fixed reads.* One schedule read (B3); one exit read at its turn (the physical overlap
  of a blocked group row or of a physical-escape row); one read at its new position (a
  physical-escape row that moved). So: physical escape 3, blocked group 2, blocked plain
  1, spare 1.
- *Slot reads.* A turn that does not hold the row ends with the slot clearance refresh
  (`refresh_nearby_pool_slot_clearances`; a blocked plain row does it in each turn). It
  does one read for each pool slot within `R = radius + max radius` of the old or the new
  position of the row. For a pool whose slots are at least `d` apart, that is at most
  `2 * (2R/d + 1)^2` slots: the discs of radius `d/2` round the slots near one position
  do not overlap and lie in a disc of radius `R + d/2`. The kernel does not check the
  slot distance of a pool; with no such distance the bound is the slot count of the
  pools. The count does not read a body, so a hidden body changes it only through the
  largest radius of the frame.

No read for each candidate or claim. This count is for recovery. A row that leaves
recovery in its turn, and a spare row, also do the reads of an ordinary mover (separation,
projection, the slot refresh, and the overlap read of an order that fails); those are not
recovery reads and no recovery counter holds them.

B5. **Counters.** `RecoveryCounters` holds `physical_reads` (the fixed reads and the slot
reads of B4), `physical_work` (cells and bodies of these reads, and the contacts that the
row turns read) and `view_work` (membership checks, inserts and two units for each
relocation). `frame_work` is their sum. `checks` counts the metered point checks with no
search. `spare_slot_work` is the metered work of spare jobs; it is inside `owner_work`.
The navigation diagnostics and the terrain benchmark report
`recovery_physical_reads_this_tick`, `recovery_physical_work_this_tick` and
`recovery_view_work_this_tick`.

B6. **The floor of the schedule (addition to A7; accepted 2026-11-19).** B1 makes each row
on the way to a retained target need a slice in each tick, and A7 serves 16 fresh rows of
an owner at most. So a row that gets no slice from A7 gets `MIN_SLICE` when it has a retained target
(for a group row: a target that no other mover claims at tick start, because a claimed
target gets no check) or is a spare row, in service order, from at most half of the rest of the owner budget
(the oldest scan batch, or the fresh rows, get that much less). A batch that does not fit
takes turns by tick (A7). For a row with a retained target the floor pays point checks
only: a failed check waits for a search slice, as an unserved row did before. For a spare
row the floor is a search slice. The allowances of one owner still sum to `OWNER_WORK` at
most.

The floor does not remove every wait of a row with a retained target. The row moves in a
tick only when its check ends clear in that tick. It waits in two cases. (1) More rows
wait than the floor serves (`F` = at least `OWNER_WORK / 4 / MIN_SLICE` = 128 rows of an
owner in a tick): a row gets a floor slice within `ceil(W / F)` ticks, `W` = the rows of
the owner that wait for the floor. (2) Its check costs more than one floor slice: the
check keeps its cursor and continues in each slice that the row gets. A slice that
resumes gives the query at least 8 units (64, less the local proof and the reserve of the
next pause, at most 27 each, and two repeated steps; derived from A5 and A6, not
measured), so a check that reads `C` cells and keys in a stable box ends within
`ceil(C / 8)` slices. Together: at most `ceil(W / F) * ceil(C / 8)` ticks in a stable
box. Measured by the review probe: 120 rows, each with 100 known bodies in the query box
of its target, all end by tick 135; with no such bodies 200 rows end by tick 51.

B7. **Reserve of the pool stage (repair of A6; accepted 2026-11-19).** The pool stage reads slot-grid cells
with no slot in them before it starts the next slot. A slice that resumed inside such a
run had no reserve, so its pause could not pay its proof (a debug assertion; in release
the scan became relaxed). A slice now restores the reserve of its examined region before
its first unit.

### 7.3 Bound

*Replaced by `2026-11-20-bounded-contact.md` section 4.* The claim below that a read visits
at most 16 cells was false for the schedule read: its reach had `CONTACT_GAP`, which does
not scale with the base cell. The text stays as the record of this amendment.

For one owner in one pass:

- K work is at most `OWNER_WORK` = 32,768. It now includes B1 and B2. Fixed overshoot: 0.
- P reads are at most 3 fixed reads for each schedule row of the owner, plus the slot
  reads of B4 (at most `2 * (2R/d + 1)^2` for each pool near the row). One read costs the
  cells of its box (at most 16 with the reaches of 7.1) plus the bodies in them.
- V work is at most one membership check for each row of the frame plus one insert for each
  body that the owner knows, plus two units for each mover of the frame that the owner
  knows (a mover moves one time in a pass).

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

Memory: as A9, less the start grid; plus the schedule reads (24 bytes for each schedule
row and 4 bytes for each contact). The soft flag of a view shares the member byte of each
row. `RecoverySearch` stays 200 bytes.

### 7.4 Privacy

A hidden body out of contact changes no K work, no allowance, no slice end, no result and
no owner counter. It changes `physical_work` and the membership part of `view_work` only.
A hidden body in contact range at tick start is a known body of that actor (A2), for the
retained check and the spare search as for the search: it gives the result of the same
body when visible. Its K work can differ by a few units, because a contact is read from
the contact list and a visible body from a view cell.

### 7.5 Liveness

As section 3. In addition: a retained check with a cursor ends when its query box is
stable for one check; without a cursor it cannot stop the stored search (it uses half the
slice at most). A spare row is served by the floor (B6) within
`ceil(waiting rows / floor rows)` ticks; the floor serves `OWNER_WORK / 4 / MIN_SLICE`
= 128 rows of an owner in a tick when blocked rows take all fresh slices, less the rows
with a retained target that wait. A spare search that needs more than one slice continues
only while the mover stays within its radius of the origin of the search (B2); a mover
that goes farther in each slice starts again.

### 7.6 Tests

`recovery_actual_tests.rs`, R15 to R24 (the test names are in the file):

- R15: a retained target in a cell larger than a slice (group, physical, plain) resumes
  and ends, clear and blocked; in the frame the row holds its position and its target
  while the check is paused; rows with a retained target get the floor, and rows whose
  check fits in a floor slice move in each tick.
- R16: a paused retained check sees a new body or claim on its target.
- R17: the spare search gives the slot of `nearest_free` for each class of body on the
  nearest slot (none, own, another owner at rest with and without a witness, a mover of
  another owner, hidden with and without contact, visible), with the claim exchange on
  and off.
- R18: a spare search larger than a slice resumes (this is the test of B7 too); the slot
  that the mover leaves is never the answer.
- R19: more spare rows than slices: blocked rows first, each spare row is served; in the
  frame, 200 stalled movers wait with their order, stall state and job, and all get an
  answer.
- R20: hidden-world pairs with rows of each class (physical, group, plain, spare), hidden
  bodies near and far, never in contact: no owner output and no owner counter changes in
  60 ticks, exchange on and off.
- R21: a hidden body in contact gives the result of the same body when visible, for the
  retained check and the spare search; out of contact it gives the result of no body.
- R22: the K work of an owner is at most `OWNER_WORK` in a scene with rows of each class,
  and the reads of each tick are the fixed count of B4; the check and the search of one
  row share one allowance.
- R23: a stop order, a new order and a death drop the job (a paused retained check, a
  waiting spare job).
- R24: reversed unit ids, row order and owner slots give the mirrored result and the same
  owner work; the public replay kernel, from jobs that went through the log encoding,
  gives the outputs and the jobs of the live frame.
- B1: a clear retained group target has no resting body on it (3,000 random scenes with
  bodies at the tolerance edges).

`recovery_oracle.rs` (test builds only) holds the base physical queries. The frame calls it
at each metered check, each reached test and each spare search that ends in one slice, so
every movement test compares the metered result with the base result.

Each test went red on a one-line mutant (19 mutants; log in
`target/mc-evidence/recovery-gm7-unblock/bound/mutants.log`).

One test of the base changed: `step_frame_rejects_fully_occupied_spares_without_pool_wide_stalled_scans`
(`spread_tests.rs`) counted slot candidates with a counter inside `nearest_free`. That
function is now the test oracle only, so the test reads the recovery counter `candidates`.

Repair tests after the review of `f740fe2e`. Each went red on a one-line mutant (same
log), but the fog column test: see its item.

- The slot reads of a recovery row are counted: `physical_reads` equals the fixed reads
  plus a count of the near slots that the test makes from the scene (21, 25, 25 in three
  ticks; the counter was 2). Ordinary movers and a hidden body out of contact change no
  count.
- View relocations are counted: `view_work` equals the build plus twice the relocations
  that the view counts itself.
- B1 with a stored search scan: an unpaid check is never clear, uses half the slice at
  most, and the stored search ends.
- A spare search keeps its origin while the mover creeps; a waiting row drops the check
  of a point that it left; a paused `Spare` job continues the same after a JSON round
  trip.
- `sweep.rs`: two bodies that stop each other do not wait for each other; a second turn
  continues the pair count of the first turn.
- `sweep.rs`: random hidden bodies beside a fog column, and one hidden body that is a
  pair candidate of the front body, change the column only on its chords. A control (a
  hidden body on a chord) must change it, so the test is not empty. Open: five one-line
  mutants of `certify_guided` left it green (list in the log). With a mutant that makes
  a pair candidate of another owner never clear, the front body still makes its full
  step; the stage that restores the step is not traced.

### 7.7 Limits

- A row with a retained target can wait (B6): when more rows wait than the floor serves,
  and while a check larger than one floor slice continues.
- Target claims are counted in two buckets, owner 0 and all other owners
  (`spread.rs` `update_target_claims`, `frame.rs` `target_claimed_by_other`;
  pre-existing at `e306a51c`). Under fog with `SLOT_COUNT > 2`, a far hidden mover of a
  third owner with the same target would change the row of another owner that is not
  owner 0. `SLOT_COUNT` is 2, so no live frame has this case. A change of `SLOT_COUNT`
  must give each owner its own bucket first.

- P and V work have no constant bound (7.3).
- The spare search is not immediate any more. With more than 16 fresh rows of one owner, a
  stalled mover waits for the floor (B6), and a mass failure of N movers takes
  `ceil(N / 144)` ticks at best.
- Blocked rows are served before spare rows by state. A blocked row that needs no search
  (it ends at its exit read) still takes a fresh slice before a spare row
  (pre-existing rule of A7: admission is by state).
- The proof of a spare scan counts soft bodies, so a mover of another owner near the
  examined region restarts it (then it runs relaxed, section 3).
- In claim exchange mode with no visibility basis each owner with a served row builds its
  own view (it was one shared view).
- The sweep guard restores the old target of a physical escape when it clips the step
  (`sweep.rs`, pre-existing). So a retained target that failed its check can stay in the
  output for that tick; the next tick checks it again.
- A row with a retained target whose step the sweep guard refuses in a dense cell holds
  its position (pre-existing; test R15 in the frame notes it).
