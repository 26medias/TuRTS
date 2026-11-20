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
