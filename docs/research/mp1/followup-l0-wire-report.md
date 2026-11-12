RESULT=PASS

# Follow-up of L0-WIRE after the second re-check (network-engineer, build mode)

Branch `mp1/l0-wire-fu` from 67a3f388, head dc72fa4d (pushed, not merged). Labels `l0wire-fu-*` and `l0wire-fu2-*`.

## 1. One freeze predicate — done

- `economy::production::FreezeFacts { disabled }` with `frozen()` is the one place of the freeze rule. `frozen` destructures the struct, and the two builders list every field, so a new cause (`L-PWR`: no power; `L-PAY`: a held job) is a new field and a cause that one side leaves out does not compile.
- The three sim skips (`run_production`, `run_machines`, `run_research`) call `freeze_facts_in_world(..).frozen()`, and pass the real `frozen` value into `job_step_per_tick` (the literal `false` is gone). The wire calls `freeze_facts_in_view(..).frozen()` in `view_job_per_tick`. No behaviour change: `job_progress_clients.rs` is green (3 tests), the frozen stream and the trace bodies are equal.
- Plants `FREEZE1/(a)` (a freeze cause in the world builder only) and `(b)` (in the view builder only): both red in `job_progress_clients.rs` ("a disabled producer holds a Running job"), restored.

## 2. Units of `RateRecord` — done

- Ruling in the doc comment of `RateRecord` (`records.rs`) and in `errata-l0-wire.md` section 11: `done`, `total` and `per_tick` count raw units of 1/Q tick; today Q is 1; the client derives ticks with `base_ticks_done(units, duration_ticks, total)`; example Q = 2, 100 ticks, rate 1/2: `total` 200, `per_tick` 1.
- `Replica::job_elapsed_ticks` now does that (it reads `duration_ticks` from the `JobState` of the queue).
- Unit test `job_progress_in_raw_units_of_one_over_q_derives_base_ticks` (`economy_replica.rs`, values built by hand): rate 1/2, 50 ticks later, 25 base ticks; full rate at Q = 2, 50 ticks; the record never passes `total`. Plant `Q2UNITS1/(a)` (units read as ticks) is red and restored.
- Side effect: the derived ticks never pass the job duration, so the display clamp in `production_queue` is dead code. I removed it (and its plant `CLAMP1/(a)`), and the unit test keeps its observations (progress 1.0, 0 s left) with a record of 120 of 80.

## 3. The Snapshot oracle — limit stated, not a scenario change

- Chosen: state the limit in the doc comment of `measured_job_rates` (`economy.rs`) and name `crates/turts_client/tests/job_progress_clients.rs` as the guard.
- Reason: the first raw Snapshot of the process scenarios is taken at the join, before the scripted `Produce`. A change of the scenario costs one full `acceptance.sh` run (about 25 minutes) for each try. The guard that sees a wrong rate exists and is cheap: it compares three real `Replica` clients with the sim at every tick (plants `JOBRATE1/(a)` to `(f)`).

## 4. `view_job_per_tick` — done

Doc comment: "`L-PWR` MUST CHANGE THIS FUNCTION": it passes Q = 1 and the full rate; the view must carry the raw progress, the Q and the rate of each producer; the function calls `job_step_per_tick(stage, frozen, rate, q)` with them and writes `per_tick` in raw units (rule of item 2).

## Gates on dc72fa4d

- fmt; clippy `-D warnings`; `cargo test --workspace --locked`; `cargo test -p turts_sim --locked --features test-probes`; `boundary.sh`; wasm32 check: all exit 0.
- Frozen stream bcbf72c2... equal; trace bodies base d78dc09392e5b969, haul 887ce95f69801702, acme ba1c849aaee46dd4, cedar 09f637325c511972 equal; `probe_neutral` `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`.
- `mp1_plants.py --check --all`: 326 apply, 0 stale (the three L0-SIM plants of `production.rs` still apply). The plants of `production.rs`, `replica.rs` and `sources.rs`: 23 of 23 red and restored (`l0wire-fu2-plants`).
- Not run (not asked): `acceptance.sh`, headed rows.

## Note

I edited a file while the plant runner was still running once; it correctly reported "the command changed files outside the plant" for three plants. I ran them again after the edit, and they are green. The rule for me: do not edit the tree while the plant runner runs.
