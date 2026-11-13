RESULT=BLOCKED

Reason for BLOCKED: one rule of the task is not met as written. The four trace sha256 values changed on the merged head. The cause is not the repair and not a behaviour change (see "Hash checks"). I did not re-freeze anything. The orchestrator must rule. All other items are done and green. The branch is pushed (665a6e7..c0ab357).

# Repair 2 of L0-SIM (build mode, simulation-core-engineer)

Worktree `/home/julien/Projects/TuRTS-mp1-l0-sim`, branch `mp1/l0-sim`, cache `mc-mp1-l0-sim`. Logs: `/home/julien/Projects/TuRTS/target/mc-evidence/l0sim-fix2-*/command.log`. Scratch folder: `.../scratchpad/l0sim-fix2/`. Integration is now at 379cce4 (it moved after f372c77); I merged f372c77 as ordered.

## Commits (on 665a6e7)

- 28693b8 fix(sim): N1 (job total, loud rate, views in ticks)
- 6cedd47 fix(server): N2
- bfea57c refactor(rules): `base_ticks_done`, the one conversion for the views (needed by N1; see N1 notes)
- 0e860dc merge of origin/mp1/integration f372c77 (two conflicts)
- 67c8a98 docs: plant record of L0-SIM (5 new plants, 4 re-pinned)
- c0ab357 docs: two L0-RULES plant records re-pinned (test renamed)

## N1 (verified in code first: `power.rs:53-64`, `production.rs`, `machine.rs:92-97`, `views.rs:754/839`: yes, as the re-review says)

- **What changed.** `ProductionJob` has a new field `total` (design 6.4: `duration_ticks x Q` with the Q of queue time). `queue_production` stores it (`production.rs`, `total = duration_ticks.checked_mul(power::rate_q(world))`). Machine and research jobs set `total = duration_ticks` (they do not read the rate). `run_production` compares with `job.total`, not with `duration x world Q`. The Q of a job is `total / duration_ticks`, so the world `RateQ` is read only at queue time.
- **(i) Loud, not silent.** `job_step` panics with a message ("the rate 1/2 does not divide Q = 1 of job N (design 6.4 ...)"). Choice and reason: design 6.4 (`2026-11-12-mp1-playable-skirmish-design.md`, "Progress in integers") makes Q the least common multiple of every `Rate` denominator and says the packer refuses a bad Q, so a rate that Q does not divide is a broken invariant of the catalog install, not player or mod input. The sim has no integrity stop reason (`StopReason` has only `TickExhausted`, `events.rs:183`), and a new one would change the wire, so a panic is the smallest loud form. NEEDS DECISION if you prefer an integrity stop with a diagnostic (it needs a new `StopReason` variant and its wire text).
- **(ii) Views.** Both views show base ticks: `elapsed_ticks = floor(done x duration / total)` (`turts_rules::progress::base_ticks_done`, `views.rs` `job_view` and `raw_queue_view`), with `duration_ticks` unchanged. So the pair is in one unit for every job, and no view shows 1/Q units. At Q = 1 (`total == duration`) the value is `done`, so every M2/M3 wire and log byte is the same (the sim record lines of the four traces are byte-equal, below). The function sits in `turts_rules` and not in `production.rs` because a local `u128 x / u128` would raise the formula ratchet count of `production.rs` (1 to 2); the ratchet baseline is not weakened. What the views do not show: the sub-tick fraction (floor). L-PAY and L-WIRE own the `done`/`total`/`per_tick` records (design 6.8).
- **Tests, red first** (log `l0sim-fix2-red-n1`): `a_rate_whose_denominator_does_not_divide_q_is_loud_and_not_a_silent_stall` (red: no panic), `a_job_queued_at_q_2_at_rate_1_2_takes_two_times_its_duration` (queued total = 2 x duration; 1 start tick, then 2 x duration ticks), `the_views_show_ticks_and_not_units_of_one_over_q` (Q 2, 4 ticks at 1/2: raw 4 units, views show 2 of 4 ticks, both views), `base_ticks_done_is_the_progress_in_ticks_of_a_job_with_a_total` (rules). Three existing rate tests now build the job at its Q (`producer_app_q`). Note: in the first red run the second test failed at a fixture step (`ProductionDurations` missing), not at the intended assert; I fixed the fixture and then proved each new test red through a plant (below).
- Plants: (n1a) silent step 0, (n1b) total ignores Q, (n1c) views show raw units, (n1d) `base_ticks_done` returns raw: all red and restored.

## N2

Verified: `admit_player_commands` (`session.rs:382`, exclusive) and `apply_script` (`command_script.rs`) had no order; the new test panicked with "1 pairs of systems with conflicting data access ... conflict on: World" (`l0sim-fix2-red-n2`). Chosen order: the script runs AFTER admission (`.after(crate::session::admit_player_commands)`). Reason (comment in `command_script.rs`): a scripted entry has no session, `seq` 0 and no rate limit, so it lands behind the commands of real clients in `CommandQueue`; a live command has the first place, as in a match with no script, and the order of the two writes is fixed. Test: `command_script::tests::the_script_and_the_admission_system_have_one_order_when_both_are_installed` (installs `session::install` and `command_script::install`, ambiguity check at error level). Plants (n2), (o5) red and restored.

## Merge (0e860dc)

Conflicts and resolution:
1. `crates/turts_checks/tests/formula_ratchet.rs` BASELINE: the shape is the 5-count form of this branch. I applied the six exact changes of L0-API1, all `relation_path`: `core_adapter.rs` 3 to 0 (row removed), `widgets/cursor.rs` 6 to 0 (row removed), `economy/combat.rs` [0,3,4,0,0] to [0,0,4,0,0], `orders/run.rs` 3 to 0 (row removed), new rows `turts_client/src/relation.rs` and `turts_sim/src/rules_adapter.rs`, each `[0,5,0,0,0]`. Reason: the five copies of the relation match are gone (the integration comment says so), and the two new rows are the one mapping in each of client and sim. `rtk proxy git diff origin/mp1/integration` of the file shows only the shape change plus the `gather.rs` `wide_let_div` 1 record of this branch. The ratchet test is exact (red on any difference up or down), and it is green on the merged tree (`l0sim-fix2-merge-ratchet`, exit 0), so the table equals the true counts. No count was raised. No fingerprint changed in `wide_let_div` or `same_name_fn`.
2. `docs/measurements/mp1.md`: add-add. Both sections are kept (UnitId lookup, E-GPU).

Other plant records: two records in `l0-rules.json` named the test `formula_copies_do_not_grow`, which this branch renamed to `..._and_the_record_is_exact`. The runner found it ("baseline: test ... did not report ok"). I re-pinned both records (name only). `l0-rules.json` all 26 plants are red and restored (`l0sim-fix2-plants-rules`).

## Hash checks on the merged head (`l0sim-fix2-hashes`): NOT EQUAL for the four traces

| Item | Recorded | Merged head |
| --- | --- | --- |
| base | d4c73ebe... | bf2a93edb049fb243391f277f95336c62c21e4731da4115cfb14f79177884158 |
| haul | 805309b2... | 0fd2e8eb6af2c8410ee2d8b3bddc774cbb6ba269abfebf77d223e868d64e9c31 |
| acme | 19dcf9a3... | fb13c9695a4705878030903ceef07b5cc153647672b6b73ae91b0445b408db78 |
| cedar | 0db10e01... | 0383a11b2a24bdb739f495858fd8f0d96ab8107e3ca8c2e0d367077dc12e0c32 |
| stream `tests/m3/m2_operation_stream.json` | bcbf72c2... | bcbf72c2... equal |
| `probe_neutral` | frame_chains=d0d1b1c572163424, muster_world=aa9db740578ed5c1 | both equal |

Diagnosis (I stopped and did not re-freeze):
- At bfea57c (my branch before the merge) the four hashes ARE equal to the recorded values (`l0sim-fix2-trace-pre`, in a scratch worktree, removed after). So N1, N2 and the rules function change no trace.
- Between that head and the merged head, the only difference in the four files is line 1, the header: `emitter_sha256` (base: e5152ece... to 04caf579...). Every other line of every file is byte-equal: `tail -n +2 | sha256sum` gives base d78dc09392e5b969, haul 887ce95f69801702, acme ba1c849aaee46dd4, cedar 09f637325c511972 on both sides, with equal line counts (2,898 / 4,820 / 802 / 4,712).
- `emitter_sha256` is the sha256 of `crates/turts_pack/tests/m2_trace.rs` (`m2_trace.rs:14`, `:67`, `:500`). Integration commit 967a786 (L0-API1) added five match arms to that file (`git diff bfea57c 0e860dc -- crates/turts_pack/tests/m2_trace.rs`: `Behaviour::Construct`, `CancelConstruction`, `SetRally`, `Repair`, `Surrender`). So the merge itself changes the header, and any lane that merges integration gets the same four values. NOT TRACED: whether integration alone (f372c77) gives the same four values (I did not build it); by the diff above it must.
- Needs decision: take the four new file hashes as the reference for the merged tree (the content below the header is unchanged), or keep the header comparison by another rule. I changed no recorded value and no file.

## Plants (`mp1_plants.py`, from the tree, self-test 51 of 51 ok)

- `--files <all files changed against integration>` (`l0sim-fix2-plants`): "39 plants in 34 command groups, 38 red-restored, 1 failed". The one failure was `L0-RULES/(s)`, the renamed test (above). After the re-pin, the full `l0-rules.json` run is "26 plants, 26 red-restored, 0 failed". All 35 L0-SIM plants were red and restored. The tree was clean after each run (`source_unchanged` true).
- New or re-pinned L0-SIM plants: re-pinned (d), (m3a), (m3b), (o5); new (n1a) to (n1d), (n2). Total L0-SIM records: 35.

## Gates on the merged head (all through `exec_in_lane_v4.py`, no headed run)

| Gate | Label | Exit |
| --- | --- | --- |
| fmt --check | l0sim-fix2-gates | 0 |
| clippy workspace all-targets `-D warnings` | same | 0 |
| test turts_sim, turts_server, turts_pack, turts_audit, turts_checks | same | 0 each |
| `cargo test --workspace --locked` (2,896 passed, 0 failed, 70 ignored over the whole gate log) | same | 0 |
| wasm32 check | same | 0 |
| `scripts/boundary.sh` | same | 0 |
| `mp1_plants.py --self-test` | same | 0 |
| `scripts/acceptance.sh`, `MC_JOBS=4` | `l0sim-fix2-acceptance4` | 0 (30 passed, 0 failed) |

Acceptance needed 4 runs on a loaded host (load average 10 to 14 on 28 CPUs; other lanes compile and run browsers). Run 1 stopped at `t5_convergence`: "lag_p99 1.552 > 1.5". Run 2 stopped at the hd1 native reference: "accepted set for sequence 12 left [] right [43]"; the same step ran green alone at once after (`l0sim-fix2-hd1a`, exit 0). Run 3 stopped at `f11_private_research...`: "timed out waiting for live client report ... owner-live.json" (230 s). Run 4 passed all stages. Each failed test passed in another run on the same tree. I did not find a code cause; the failures are timing limits under load. NOT TRACED: I did not run a pre-merge head under the same load, so I cannot exclude that the merge made one of these tests slower. A calm host would settle it.

## NOT TRACED

- Integration f372c77 alone gives the four new trace hashes (by the diff, yes; not built).
- The three acceptance flakes: cause not found beyond host load.
- `T7`/headed: not run (as ordered).
- The panic of `job_step` has no recovery path in a running server. That is the intent (invariant), but a lobby server may prefer an integrity stop: needs decision.
- Integration moved to 379cce4 (L0-CORE) after f372c77; this branch does not contain it.
