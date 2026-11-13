VERDICT=FAIL HIGH=1 FINDINGS=8 RUNS=34 NOT_CHECKED=4

# Review: pre-MP1 slice task A2b1 (fallback pool, gather protocol, guards)

Seat: performance-architect, review mode. Date: 2026-11-12. Candidate f119dc39 (`pre-mp1/a2b1-fallback`), base
afb9c6de. Lane `/home/julien/Projects/TuRTS-pre-a2b1-review`, cache `target/mc-pre-a2b1-review`. Receipts:
`/home/julien/Projects/TuRTS/target/mc-evidence/a2b1-rev-*`. Scratch: this folder. The lane is clean at f119dc39
at the end (`git status --porcelain` empty). The probe test binaries are deleted from the cache.
Standing rule: I write nothing in a repository in review mode, so this file is in the scratch folder, not at
`docs/research/mp1/review-slice-a2b1.md`. What I changed: none.

## Findings

H1. High, cost; needs decision. `crates/turts_sim/src/movement/frame.rs:101-105`, `:173-190`; `movement/pool_index.rs:77-108`;
`movement/ecs.rs:83`. Each fallback pool of one is a full `MovementPool` in `MovementPools`. Each tick, the step clones
every pool, builds a `MovementPoolIndex` for it (Vec, `UniformGrid` with a SipHash `HashMap`, `slot_indices` HashMap,
clearances Vec, BTreeSet, HashSet), refreshes the clearance of each of its 7 slots (one `within` Vec each,
`grid.rs:81-87`), and drops it. Measured (release, i9-14900K, 28 threads, load 12-18, whole `app.update()`, 20 Hz
assumed): 1,000 single `MoveTo`, packed crowd, steady ticks: base 1.81-1.97 ms, head 4.43-5.56 ms (interleaved, 3
pairs); 40-tick runs: p50 1.62-2.08 ms against 7.40-8.34 ms. 5,000: p50 13.4-25.5 ms against 36.2-58.6 ms, p99 14-65
ms against 54-134 ms. LBR call graph of the head steady state: pool index build 10.4 %, drop 8.4 %,
`refresh_slot_clearance` 17.5 % (inclusive) of the tick. Cost model: 1,000 pools x 7 slots x (1 grid query + about
2 allocations) + 1,000 x about 6 allocations = about 20,000 allocations per tick (code reading; no allocation
counter: NOT TRACED). No plan or design text records this cost; A2c gives a pool of one to every single move.
Ten times cheaper: do not build a `MovementPoolIndex` for a pool with `primary_count <= 1`; scan its at most 7 slots
linearly when a spare is needed, or build the index lazily only for a pool with a stalled or `Blocked` member.
Options: (a) fix in A2b1 with `kernel_reference` bit-equal; (b) accept, record the numbers, and add a cost gate on
the 13.2 workloads before A2c.

M1. Medium, instrument. `frame.rs:201-211`. Fault F1: skip the reservation update of a unit that took a fallback in
this tick. Two fallback pools then get the same slot (29.45, 14.047372) (scenario: units 2 and 3, engine trips to
(30, 15) where unit 1 rests). The suite stays green, 357 of 357. Fix: a test with two same-tick fallbacks; the
expected slots come from the ring rule: (29.45, 14.047372) at -120 degrees and (30.55, 14.047372) at -60 degrees
(the clean head gives these).

M2. Medium, instrument. `frame.rs:229-231` (design 9.17 at the escape-clear site). Fault F5: remove
`&& !progress.is_fallback()`. Green 357 of 357. A reach plant (F5c, a panic on a `Blocked` fallback at that site)
fires only in `kernel_reference` (`frame.rs:230`), after its cut-off, so nothing compares that state. The main 9.17
site (`frame.rs:379`, fault F5b) is red in 4 tests.

M3. Medium, instrument. `economy/gather.rs:1429` (design 9.19). Fault F2: never write `gather_goal`. Green 357 of
357. Effect on the crowd of 20 gatherers (1,200 ticks): positions bit-equal (hash d039f9cfa50a4d26 both), depot 596
both, fallback pools 352 both, max 1 per trip; only the goal unit-ticks fall from 19,855 to 0 and the fallback
unit-ticks from 19,856 to 19,833. So no instrument reaches a case where 9.19 changes a result. Fix: a scenario where a
fallback rests on its pool point while its gather stage still approaches, with a read of the result.

L1. Low, cost. `movement/fallback.rs:88-91`. `assign_fallback_pools` does a linear `find` over all outputs for each
request: O(F x N), 12.5 million compares at F = N = 5,000. Outputs and requests are both in unit-id order: use a
merge walk.

L2. Low, instrument. `crates/turts_sim/tests/group_movement.rs:3914`. The hex test leaves one legal ring slot, so
the tie rule is not tested. My own case (P (30.3, 17.7), r 0.7, s 1.5, a post on the -120 degree ring point, a
resting unit on P): five ring slots tie; the rule "ties by angle" (MC design 6.3 step 3) with ascending `atan2`
gives -60 degrees, (31.05, 16.40096), bits 41f86666,4183352c. Head and base give these bits. Add it as a case.

L3. Low, instrument. `crates/turts_sim/tests/probe_flags.rs:105-111` (doc), `:155-187` (loop). The doc says the pool branch
"must" report a projection on a tick in 2..200. The geometry makes contact likely; it does not prove it (steering can
avoid contact). The window is a description of the run, not a derived value. It can go red (the builder's constant
plant). The `separated` read of the old tick 2 has no replacement here; `probe_flags.rs:216` covers `separated` on
the pool branch.

L4. Low, merge. `tests/m2/refutations.json`. `git merge-tree` against `origin/pre-mp1/a1d-oracle` (a921096b): content
conflict, append against append (exit 1). The three A1d anchors in `economy/views.rs` are still present at f119dc39.
Keep both row sets. Against `origin/pre-mp1/integration` (afb9c6de): fast-forward.

## Lens answers

1. Differential (Muster scenario of `frame_hash.rs`, 515 units, 300 ticks; own probe). First fallback tick 1, units
   21 to 40, on both builds; the tick-1 slots are bit-equal for all 20. First position difference tick 38, unit 40.
   At 120 ticks 390 of 61,800 unit-ticks differ (259 fallback units, 131 pooled units); at 300 ticks 2,852 of
   154,500 (2,356 and 496). Classes:
   - C1, 20 fallback units: float drift with the same slot (pool expressions, D 9.8, 9.18). First at tick 38.
   - C2, 4 fallback units (27, 31, 34, 36): another spare slot (ticks 202 to 291; D 9.18 stall and spare logic).
     Units 31 and 36 rest at another free place (3.97 and 1.10 apart). Units 26, 27, 34 (base) and 26, 27, 33, 34
     (head) still move at tick 300.
   - C3, 10 pooled units with no conflict (1-6, 9, 13-15): another path from tick 102. At the tick before its first
     difference, each one is 0.89 to 1.78 from a unit that already differs. Final places bit-equal. Arrival tick
     differs for unit 3 only (+1).
   - No difference before the first fallback; no difference in units above 40; no overlap at tick 300 (minimum
     pair distance 1.0000558 on both).
   All classes are explained by D 9.18. The design rule of the CEO sentence explains none of them. Own scenarios
   (two units to one point: lower id first, higher id first, same tick, as engine trips; one unit to a point with a
   static post; one unit to a point with an idle unit; own hex case): positions bit-equal base against head on all
   ticks.
2. Hex slot: the test computes P + (s, 0) itself (`group_movement.rs:3915-3924`), independent of the code. My case:
   see L2. Tie rule: in MC design 6.3 step 3 ("ties by angle"). The code key is (exact integer hex norm
   i*i + i*j + j*j, `atan2` rounded to 1e-6 rad, insertion index) in a BTreeSet (`spread.rs:233-237`). No float
   compare decides the distance; iteration order is fixed. `atan2` is platform libm (`movement/mod.rs:153-155`), so
   a cross-platform tie flip needs two slots within 1e-6 rad: NOT TRACED on another platform.
3. Determinism: per-tick FNV of the full audit state over 300 Muster ticks: 09b14ec4a8dab29b in two runs in one
   process and in a second process; the dumps are `cmp` equal. No hash-map iteration feeds an output
   (`frame.rs:170` collects keys into a set only); requests sort by unit id (`fallback.rs:82`); the ECS rows sort by
   unit id (`ecs.rs:72`). The DET1 script (`gm40-script.json`), replayed in process, reaches one fallback (unit 41,
   gather trip, tick 161); positions bit-equal to the base over 420 ticks. DET1 itself is `#[ignore]`: not run.
4. Cost: see H1 and L1. The 2,048 cap is in A2c (plan `:1016`), not in A2b1: all 5,000 single moves start in tick 1
   on both builds. Tick 1 (all units fall back in tick 1, also with distinct targets: the packed crowd projects):
   1,000: 46-48 ms against 83-92 ms; 5,000 distinct: 1.65-1.79 s against 2.54-2.72 s; 5,000 to one point: 5.6-8.1 s
   against 9.4-10.3 s. Per fallback the head allocates as the base `make_plain_progress` did (HashSet, spread Vecs,
   grids); no pool clone in the request path.
5. `kernel_reference` (release, 118.5 s, exit 0): 64 of 64 random seeds have a fallback (55 at tick 0). Compared
   no-pool moving unit-frames: 942 in total, against 1,482,121 pooled. It still proves: the pooled branch over 64 x
   300 ticks, the first fallback unit, the frames before it, the anchors. It does not prove anything after a
   fallback (F1, F2, F4, F5 green in it; F5c shows it reaches the F5 state).
   Faults (all restored, `git diff` empty, next run fresh):
   | Fault | Red |
   | --- | --- |
   | F1 reservation not updated after a fallback (`frame.rs:203`) | none (357/357) |
   | F2 gather goal never written (`gather.rs:1429`) | none (357/357) |
   | F3 ring off-by-one: `pool.slots.get(1)` (`fallback.rs:48-50`) | hex test, `post_target_gives_a_ring`, `spread_tests::contact_correction_does_not_add_a_post_step_arrival_snap` |
   | F4 ids in reverse unit-id order (`fallback.rs:82`) | `fallback_pool_ids_come_from_the_order_counter_in_unit_id_order` only |
   | F5 9.17 at escape clear (`frame.rs:229-231`) | none (357/357) |
   | F5b 9.17 main site (`frame.rs:379`) | 3 R13 tests + `probe_flags` item 4 |
   | F6 pushed branch keeps the old target (`frame.rs:928-929`) | none; probably equivalent (slot = point when the goal was free): NOT TRACED |
   | F7 collect without the point check (`frame.rs:1006-1009`) | hex test |
6. Five changed tests: items 1, 2, 3, 5 match the ruling (same property; item 1 `<= 1` as ruled; item 2 injects
   `primary_count 1`, slots [(8, 8)]). No expected value comes from a run. Item 4: see L3.
7. Frozen values: stream bcbf72c2...e370b (lane file and the emitted stream). Traces at head: base d4c73ebe...,
   haul 805309b2..., acme 19dcf9a3..., cedar 0db10e01..., `cmp` equal to the stored base traces. Classifier
   `--self-test` 74/74, `--p0` PASS, `--counts` PASS. `probe_neutral.sh` exit 0: `frame_chains=e6e4417f9cbe5fa5`,
   `muster_world=49fc63ffd1d8c57d`. Emitter e5152ece... unchanged. Item pins: `fn angle` and `pub fn step` text
   hashes equal base to head.
8. Gates: `cargo test -p turts_sim --locked` 347/0; with `test-probes` 357/0; release `kernel_reference` 3/3;
   pack/audit/server/checks 514/0; clippy `-D warnings` exit 0; `boundary.sh` exit 0. Merge: see L4.
9. CEO sentence (plan `:410`, "After the slice, ..."): not true of A2b1. At A2b1 the lower unit id gives the point
   up, whatever unit was sent first (own scenario: unit 1 sent at tick 1 leaves the point at tick 20 when unit 2 is
   sent), bit-equal to the base. "The first unit keeps the point" is A2c (S12, plan `:1013`). The harvester half
   is true on both builds (the base plain progress took the same spread slot). A2b1 in one sentence: "A unit that
   goes to a free place beside a taken point now uses the group-move rules on its way there, so in a crowd its path,
   and sometimes its final free place, differ a little; which unit gets the point does not change."

## NOT TRACED

DET1 two-process run (ignored test, not run); allocation counts (no counter; code reading); cross-platform `atan2`
ties; F6 reachability; CEO touch point 1 answer; T7 and browser cases; D57/D58 4-vCPU host numbers (this machine is
a loaded 28-thread desktop).

## What I ran (claim | result that refutes | observed)

Receipts `a2b1-rev-*` under `/home/julien/Projects/TuRTS/target/mc-evidence/`.
- head-diff, base-diff, head-det2: Muster and own scenarios, base and head, second process | a difference before the
  first fallback, or unexplained | none; table above.
- base-timing, head-timing, steady (3 interleaved pairs), perf LBR | head equal to base | 2.3-5x steady.
- g-sim, g-simtp, g-kref, g-rest, g-clippy, g-boundary, g-probeneutral, g-trace, g-clsself, g-clsp0, g-clscounts |
  builder gate claims | all as claimed.
- fault-f1 .. fault-f7, f5b, f5c, sametick-clean/-f1, f2probe-clean/-plant | each guard can go red | F1, F2, F5 green.
- `git merge-tree` x2 | clean merge | A1d conflict.
Files: `muster_cmp.txt`, `classify120.txt`, `classify300.txt`, `base/`, `head/`, `head2/`, `perf/`, `f*.out`,
`plant.py`, `zz_*.rs` in this folder.
