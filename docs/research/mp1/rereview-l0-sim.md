VERDICT=PASS

# Re-review of L0-SIM repair at 665a6e7 (performance-architect)

HIGH=0. The repair closes H1, H2 and M1 to M4. I found two new Medium defects and one Low. None of them changes behaviour today.

## 1. Old findings
- H1: closed. `plugin.rs:118-128`: FixedPreUpdate is Advance, Supply, Construction, Commands. FixedUpdate (`:129-147`) is Income, Orders, Movement, Gather, Extract, Transport, Inserters, Production, Acquire, Repair, Combat, Outcome. FixedPostUpdate is Collect (`:148`). This equals ruling section 5: 17 sets, in the right schedules. Supply (`:160-164`) and Construction (`:172-176`) are in FixedPreUpdate.
- H2: closed as the ruling orders. Construction does placement only, before Commands. Production is the one pay pass. The site walk belongs to L-PAY.
- M1: closed. Scratch test (a) now panics: "second holder of UnitId 4" (`unit_index.rs:53-58`).
- M2: closed. Scratch test (b) does not compile: E0271, `Mutability == Mutable` (`world.rs:19`).
- M3: closed. A model of `rate_step(Q, rate)`: 1/2 with duration 3 and Q 2 gives 6 ticks; 1/3 with duration 10 and Q 3 gives 30; 1/1 then 1/2 gives 15. The rule does not read the tick, so the start phase cannot change the count.
- M4: closed. Plants m4a to m4e are red.
- Admission order: closed against Supply and Construction (`session.rs:118-122`, `command_script.rs:43-47`). It is not total: see N2.

## 2. New defects of the repair
- N1 (Medium) `power.rs:53-64`, `production.rs:301-345`: RateQ is a trap for L-PWR.
    - (i) The default Q is 1. With any Rate whose denominator does not divide Q, `rate_step` gives None. Then `steps.ok_or(..)?` (`:330`) aborts the transact, and `let _ =` (`:303`) discards the error. The job stalls with no diagnostic. Scratch test (e): rate 1/2, no RateQ, 10 ticks gave elapsed 0 (5 expected).
    - (ii) `elapsed_ticks` is now in 1/Q ticks for producer jobs. Machine jobs in the same `ProductionQueues` count ticks (`machine.rs:92-97`). Both go to views as `elapsed_ticks` beside `duration_ticks` (`views.rs:754`, `:839`). At Q above 1, the units mix. L-PWR "edits power.rs only", so setting Q breaks other files.
    - Repair: store `total = duration x Q` on the job at queue time and compare against it. Make a None from `rate_step` loud (panic, or an integrity diagnostic), not silent.
- N2 (Medium; it exists at the base too, but the repair says the order is total): `admit_player_commands` and `apply_script` are two exclusive systems in the same gap, with no order between them. A scratch test that installs both, as `main.rs` does with `--command-script`, panics: "1 pairs of systems with conflicting data access have indeterminate execution order … conflict on: World". Repair: `apply_script.after(admit_player_commands)` (or the reverse, chosen on purpose), and install both in the test.
- N3 (Low): the second-holder panic can be reached only by duplicate ids. The allocator never reuses an id (`ids.rs:33-36`). The boot paths take staged ids (`bootstrap.rs:456`, `:470`, `map.rs:160`). Scratch (d) (spawn, then despawn of the same id in one batch) panics. That needs id reuse, which no production path does. Scratch (c) (bare World batch) passes.
- Immutable UnitId: one replace path exists (`entity_mut(..).insert(UnitId(new))`, tests only, `unit_index.rs:188`, `:245`). The long-scenario test proves that on_discard and on_insert keep the index equal to a scan there.

## 3. No behaviour change (`mp1-l0sim-rereview-item3`, exit 0, source_unchanged true)
- The four traces are equal: base d4c73ebe…, haul 805309b2…, acme 19dcf9a3…, cedar 0db10e01….
- The stream is equal: bcbf72c2…. `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`: equal.
- turts_sim 365 passed, 2 ignored. turts_server 216 passed, 30 ignored. turts_audit 67 passed. turts_rules passed.
- fmt, clippy -D warnings and turts_checks: exit 0.

## 4. Plants
- My plant: the job completion compares with `duration_ticks` in place of `total` (`production.rs:340`).
    - Red: `a_job_at_a_low_rate_takes…` and `a_job_that_changes_rate…`.
    - Restored with `cmp` against `git show 665a6e7`.
    - Green: 8 passed.
- `mp1_plants.py` (from integration), run on a scratch git copy of 665a6e7 with only `l0-sim.json`: "30 plant(s) in 25 command group(s), 30 red-restored, 0 failed". The copy was clean after the run.

## 5. M5 ruling
I see no cost reason against it. The combat parse (`combat.rs:270`) costs about 0.1 µs for each weapon in each tick (a model, NOT MEASURED), about 0.5 ms at 5,000 units. L-PWR must sum supply by index, not by a string key, in the per-tick computation.

## 6. Headed T7 (`mp1-l0sim-rereview-t7`)
- Exit 0, `PASS t7-oracle.js`, 40 frames, lag_p99 1.560 ticks.
- lock-receipt: wait_s 1083.9, hold_s 95.8, yields 0, survivors none.
- I built outside the lock first, so the in-lock builds were no-change rebuilds.

## 7. Merge (`git merge-tree --write-tree origin/mp1/integration 665a6e7`, integration 9044820)
- One conflict: `crates/turts_checks/tests/formula_ratchet.rs`, BASELINE (integration has a 3-tuple, l0-sim has 5 counts).
- Cargo.toml, Cargo.lock, production.rs, research.rs, combat.rs, machine.rs, session.rs, orders/apply.rs and lib.rs merge with no conflict.
- The conflict is not textual only. With the l0-sim table, the trial merge compiles (sim, server, pack, audit, rules, mod_api, all targets) and turts_sim passes 368 tests. The ratchet then names six changes: relation_path for core_adapter.rs 3→0, cursor.rs 6→0, combat.rs 3→0, run.rs 3→0, and two new rows, client relation.rs [0,5,0,0,0] and sim rules_adapter.rs [0,5,0,0,0]. No wide_let_div or same_name_fn count changes.

## Not checked
- acceptance.sh at 665a6e7: NOT RUN (the builder reports 30 passed).
- The 5,000-unit movement cost (not this task).
