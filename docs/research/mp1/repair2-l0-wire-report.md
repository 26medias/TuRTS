RESULT=PASS

# Repair 2 of L0-WIRE after the re-check (network-engineer, build mode)

Branch `mp1/l0-wire`, head 67a3f388 (pushed, not merged). `origin/mp1/integration` 54c39fdd is in the branch (merge commit 78068855, no conflict). Labels `l0wire-fix2-*`, receipts in `/home/julien/Projects/TuRTS/target/mc-evidence/`.

## Item 1 (High): the job rate while a producer is disabled — done

- **Cause confirmed.** The sim freezes a disabled producer, machine or research station and keeps the stage `Running`. The export wrote `per_tick = (stage == Running)`, and a `SetEnabled` change resent only `QueueState`.
- **One function.** `economy::production::job_step_per_tick(stage, frozen, rate, q)` now gives the progress of a job in one tick. `run_production`, `run_machines` and `run_research` call it for their step. The wire calls it through `production::view_job_per_tick`, which reads the disabled set of the view. No shape change.
- **Job records at a toggle.** The delta writes the job records of each producer in `batch.enabled` (`export::job_record_queues`). The Snapshot reads the same function.
- **Stalled conditions** (table in `docs/research/mp1/errata-l0-wire.md`, section 8):

| Condition | Sim | Wire `per_tick` |
| --- | --- | --- |
| `Running`, enabled | advances | 1 |
| disabled producer, machine or station | skipped, stage stays `Running` | 0 |
| `Queued` (behind the head, or the start tick) | no progress | 0 |
| `OutputBlocked` | no progress | 0 |
| machine with a missing input | no job | no record |
| no power (`L-PWR`), held job (`L-PAY`) | not written yet | the function reads them when they come |

  - Limit for `L-PWR`: the wire `done`, `total` and `per_tick` are in base ticks (Q = 1). At Q above 1 the view needs the raw progress. A machine or station reads no rate, and the view-level function does not know the producer kind: stated in the doc comment.
  - `OutputBlocked` is a unit test (`production::rate_tests`), because no package can block an output.
- **Test (the reviewer's probe, committed).** `crates/turts_client/tests/job_progress_clients.rs`. It runs the real sim and the real export, and real `Replica` values for a continuous, a late-join and a lossy client; the client value equals the sim value at every tick, and the job-record table holds exactly the jobs of the sim.
  - `base`: 4 jobs, disable at 30, enable at 90, cancel at 120, quick toggles at 200 to 207: 3,123 comparisons, 498 of them with a frozen `Running` job, 0 differences.
  - `forge`: machines and research stations disabled and enabled: 778 comparisons, 120 with a frozen job, 0 differences.
  - Plants `JOBRATE1/(a)` (the old stage-only rule), `(b)` (no records at a toggle), `(c)` (no Snapshot form: the re-check's fault), `(d)` (the view ignores the disabled set), `(e)` (the one function ignores `frozen`) and `(f)` (forge) are red and restored. The L0-SIM plants (m3a, m3b, n1a) on `production.rs` needed a re-pin (the call `job_step(job, frozen, rate)`) and are red and restored.
- **Where the test lives, and a finding.** It first sat in `turts_server/tests` with a dev-dependency of `turts_server` on `turts_client`. With that edge the process test `mc_kill_replica` failed in 2 of 12 runs (and 3 of 9 earlier), each with a delta gap in the client log (`TMPDBG gap: ... base`). Without the edge it passed 12 of 12, and integration 54c39fdd passed 8 of 8 in a scratch worktree (removed). I moved the test into the client crate (dev-dependencies on `turts_sim`, `turts_pack`, `postcard`). The boundary rules read the normal and build edges only. Rule for the next author (errata section 10): add no dev-dependency to `turts_server`; run `mc_kill_replica` 12 times after a change of its dependency graph. The cause is a feature unification that I inferred from the 12-run comparison; I did not read the cargo feature graph (NOT TRACED).
- **Also at high load** (above 15) the process test still failed once in the final chain, at load 15.6; at load 8 to 9 it passed, and plant 1d was red and restored. Run the process test and plant 1d on a quiet host.

## Item 2: the independent Snapshot oracle — done

`crates/turts_server/tests/economy.rs`: the rate of each job comes from the audit of the sim: `measured_job_rates` takes the change of `elapsed_ticks` from the audit tick to the next audit tick, and does not read the stage or the disabled set. A job that is gone at the next tick completed (rate 1) or was cancelled (0), by audit arithmetic. Without a next tick (the end of a log) the rates are masked on both sides. No plant shows it red: the raw Snapshots of the process scenarios hold no running job, so a wrong rate cannot be seen there (I tried a plant `wire rate + 1` and it stayed green; I removed it). The check that finds a wrong rate is the new client test.

## Item 3 (Medium): join-at-N with a running job — done

The job test above compares the job tables of the late and the lossy clients, with the Snapshot of a tick in the middle of a run. The re-check's fault (the jobs Snapshot form removed) is `JOBRATE1/(c)`: red. The record kinds with a writer today are: order records and `orders_cleared`, weapon records (`a_snapshot_at_tick_n_equals_the_tables_built_from_the_deltas`, plant `JOINN1/(12)`), job records (above), and the Snapshot `match_state` (`every_snapshot_carries_a_match_state`, plant `MATCH1/(a)`).

## Item 4 (Low) — done

- The writer scan sees `modify_component` (fixture; plant `SCAN1/(c)` in the Stop path is red).
- The net-id test pins the whole list of protocol messages in net-id order, and that they are contiguous (plant `NETID1/(a)`: `Hello` and `Welcome` swapped, red).
- The clamp in `production_queue`: the unit test gives a record of 120 of 200 for a job of 80 ticks, so the ratio is above 1 and the clamp holds (plant `CLAMP1/(a)`, red).

## Notes for the errata file (written, `errata-l0-wire.md` sections 8 to 10)

`JobState.charged` stays (changes at a charge event only); `L-PAY` moves the two client readers (`replica.rs` job validation, `ingest.rs` charge feedback) to `RateRecord.charged`; `job_progress` stays in the replica candidate and `L0-STORE` moves it into the side store and counts it in the store-clone counter.

## Bytes (`docs/measurements/mp1.md`)

`base`, 4 jobs, disable at tick 30, enable at tick 90: the disable tick carries 205 B (the queue and 4 job records), the enable tick 205 B, ticks 31 to 89 (frozen job) 0 B, ticks 125 to 158 (a running job) 0 B. Before the repair the frozen job cost 0 B and was wrong: the client ran on (re-check: 58 in the sim, 100 in the client at tick 119).

## Gates on the head 67a3f388

- fmt; clippy `-D warnings`; `cargo test --workspace --locked`; `cargo test -p turts_sim --locked --features test-probes`; wasm32 check; `boundary.sh`: all exit 0. The ratchet is green, no count raised.
- Frozen stream bcbf72c2... equal; trace bodies base d78dc09392e5b969 (2,898), haul 887ce95f69801702 (4,820), acme ba1c849aaee46dd4 (802), cedar 09f637325c511972 (4,712) equal; `probe_neutral`: `frame_chains=d0d1b1c572163424`, `muster_world=aa9db740578ed5c1`.
- `mp1_plants.py --self-test` exit 0; `--check --all`: 324 apply, 0 stale.
- Changed plants (my records and the re-pinned L0-SIM ones, by file): 27 of 27 red and restored (`l0wire-fix2b-plants`). Plant 1d (`combat-fixes.json`) is red and restored on a quiet host (`l0wire-fix2c-1d`: 39 resyncs across the kill).
- `scripts/acceptance.sh`, `MC_JOBS=4`, load 8 to 10: exit 0 on the first run (`l0wire-fix2-accept`).

## NOT TRACED

- Headed browser rows. The debug state keeps `economy.queues[].jobs[].elapsed_ticks` and `weapons[].cooldown` as derived values; no browser run here.
- The feature-graph cause of the `mc_kill_replica` flake (only the 12-run comparison).
- The raw-unit form of the wire rate for Q above 1 (`L-PWR`).
