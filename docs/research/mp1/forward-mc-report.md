RESULT=PASS

# Forward merge of MC into MP1: report

- Branch: `mp1/forward-mc`, pushed. Head: `f4020d1f92316dd1d0fc4fcdaf7e16830b994f0a`.
- Merge commit: `961357e0` (parents `4553acc4` MP1 and `9d2ab86b` MC). Then `785d08c6` (plant re-pin) and `f4020d1f` (two MP1 DBG1 rule repairs and two reorder tests).
- Worktree: `/home/julien/Projects/TuRTS-mp1-forward-mc`. Cache: `target/mc-mp1-forward-mc` only.
- Not pushed and not changed: `main`, `mp1/integration`, `codex/mc-integration`.
- One High finding (sound rule, MC probe files). See "Findings".

## 1. Conflicts and resolutions (13 files from `git merge-tree`, plus 2 that the gates found)

1. `crates/turts_client/src/replica.rs:55-94`. MP1 `HEALTH_TRACE_WINDOW` (`:57`) and MC `ResyncReason`, `MAX_FRAME_DELTA`, `max_catchup_ticks` (`:87`) are both kept. All the other MC and MP1 hunks merged with no conflict.
2. `crates/turts_client/src/client_shell.rs`. MP1 moved the settings and the tests to submodules. MP1 file kept. The MC frame limit fields and the window modes go to `client_shell/settings.rs:36,61,73`. The two MC tests go to `client_shell/tests.rs:2464,2485`.
3. `crates/turts_client/src/core_adapter.rs`. MP1 `mod adapter_tests;` kept. The MC field goes to `core_adapter/adapter_tests.rs:229,344`.
4. `crates/turts_client/src/commands.rs:500`, `tests/support/feedback.rs:232`. Each `ReplicaState` literal has `match_start` (MP1) and `reorder_grace_override` (MC). The compiler found two more MP1 literals: `tests/alerts.rs:731`, `src/visuals/scene_tests.rs:167` (same repair).
5. `crates/turts_net_lightyear/src/bin/spike_server.rs:311`. MC `make_delta` and the burst sender, with the MP1 protocol 5 `..Default::default()` and `Welcome.start` (`:186`).
6. `crates/turts_server/tests/mc_kill_replica.rs` (add/add). The MP1 file is kept. `cmp` of the two sides: the only difference is 9 doc lines in MP1 (`:8-16`). The test bodies are the same, so the file holds every case of both sides.
7. `scripts/acceptance.sh`. MC side kept (a superset): `run_logged` (`:161`), the rows `mc_reorder_stall` (`:159`) and `p8_one_flush_order` (`:160`). MP1 `connection_end` (`:103`) and `mc_kill_replica` stay.
8. `scripts/mc-browser.sh:228`. MC rule (every row starts the server with `--stats`) on the MP1 `$SERVER_BIN` (built or prepared).
9. `scripts/t7.sh:74`. MC `T7_SCOUT_SPEED` block goes into the MP1 build branch, after `cp -r mods/acme`. A `--prepared` run uses the speed of its prepare step. `mc_prepare.py` does not record `T7_SCOUT_SPEED` (Low, see "Findings").
10. `tests/browser/m2-oracle.js:55,207-223`. Two redesigns of one sampler. Both behaviours are kept in one function: the MC judgement (gate and wide profiles, first projection per tick, report, named failure) with the MP1 sampling under the 4 Hz debug cap (both windows asked at once `:223`, an idle longer than the cap with a changing phase `:222`). Gate `poll_ms` is now `DEBUG_CAP_MS + 20` = 270 ms (MC had 100 ms). `now`, `sleep`, `project` are test hooks.
11. `tests/browser/m2-oracle.test.js:171`. All 4 MP1 sampler tests and all MC tests run on the merged sampler (a local helper `sampleSharedTick` calls `waitForSharedTick` with the gate profile). 24 of 24 pass. Three faults in the merged sampler each made one MP1 test red (sequential reads; no phase change; poll 100 ms). Restored, `cmp` equal.
12. `.github/workflows/ci.yml:265`. The node list is the union (53 MP1 + 12 MC-only = 65 files), with the MC `if: matrix.suite == 't7-m3'`. MP1 plant and prepare steps and the MC Python list step (`:342`) are kept. New step `:51`: `tests/m3/refreeze_classify.py --self-test` and `tests/m3/test_refreeze_classify.py` (the MC list check found them in no CI step).
13. Found by the node list (not a git conflict), commit `f4020d1f`. MC code broke two MP1 DBG1 rules. `harness/client-settings-seed.js:40` now asks with `turts_debug_request` (new unit test; it is red when the check does not ask). `harness/driver-io.js:318` now waits with `waitForDebugRecord` and not `waitForFunction`. Without this repair the reload wait reads an empty `turts_debug()` under DBG1 and times out after 20 s.
14. Plant re-pin, commit `785d08c6`: `docs/research/mp1/plants/l0-dbg.json` h8 and h9 name the merged lines. Both ran red and restored.

## 2. Receive path on the MP1 wire (protocol 5)

- One message kind rides the unreliable channel: `WorldDelta`. Server: `crates/turts_server/src/outbound.rs:81` (the only `send::<StateChannel>`). The channel is `UnorderedUnreliable` (`turts_net_lightyear/src/protocol_plugin.rs:20-21`). All the other sends use `ReliableChannel`, which is `OrderedReliable` (`protocol_plugin.rs:15-16`; `session.rs:290,303,380,645,721`, `outbound.rs:45`).
- Every protocol 5 record stream rides inside `WorldDelta.records: DeltaRecords` (`turts_net_protocol/src/records.rs:320-333`). There is no separate stream.
- Client: `ingest.rs:924` receives `WorldDelta`. `on_deltas` (`:469`) calls `Replica::apply_delta` (`:492`). `on_welcome` sets the grace to `max_catchup_ticks(tick_hz)` (`:419-422`).
- The window holds the whole `WorldDelta` (`replica.rs:435`; snapshot path `:338,358`). It applies a held delta only through `apply_chained` -> `stage_delta` when `chain_extends` holds (`:453,465,531`). So the MP1 merge-in-chain-order records (`unit_orders` + `records.orders_cleared` at `:771`, `records.jobs` at `:779`) apply in chain order, never in arrival order.
- The client reads only `orders_cleared` and `jobs` of the records today. `sites_completed`, `sites`, `construction_queues`, `supply`, `effects` and `match_state` have no client reader at this head (grep of `crates/turts_client/src`). They ride in the held delta, so the window covers them when a reader comes.
- New tests on the protocol 5 wire: `crates/turts_client/tests/ordw1_replica.rs:320` (a reversed flush 13, 12, 11 gives no record for unit 1 and no resync) and `:356` (a lost tick 11: ticks 12 to 15 held, tick 16 asks once).
- The grace rests on one lightyear flush per frame and a 250 ms frame clamp. Both are pinned on the MP1 server app (`turts_server/src/main.rs`, `frame_tests`, green in the workspace run).
- Uncovered kind: none found.

## 3. Gates (head `f4020d1f`, label prefix `fwd-mc-g2-`; acceptance on `785d08c6`, `fwd-mc-g1-acceptance`)

The wrapper exit is 0 only when the command exit is 0 and the source stayed the same. Logs: `/home/julien/Projects/TuRTS/target/mc-evidence/<label>/command.log`.

| Gate | Exit |
| --- | --- |
| `cargo fmt --all --check` | 0 |
| clippy `--workspace --all-targets --locked -- -D warnings` (rustc 1.99.1) | 0 |
| `cargo test --workspace --locked` (2106 passed, 0 failed, 40 ignored) | 0 |
| `cargo test -p turts_sim --locked --features test-probes` | 0 |
| wasm32 check | 0 |
| `bash scripts/boundary.sh` | 0 |
| `bash scripts/acceptance.sh` (on `785d08c6`; it ran `a_server_stall_of_0_6_s_makes_no_resync`, `p8_one_flush_reorders_deltas_by_size`, `a_client_updated_each_100_ms_keeps_pace_with_the_server`, `ordered_attack_kill_...`, `t5_convergence`, all ok) | 0 |
| yardstick `--check` | 0 |
| completeness `--self-test`, `--enforce` | 0, 0 |
| driver rule `--self-test`, run | 0, 0 |
| node unit list (65 files, 419 tests, 419 pass; g1 run before the repair: 2 fail) | 0 |
| `tests/qa/test_python_ci_list.py` | 0 |
| every other Python command of `ci.yml` (10 rows) | 0 |
| `mp1_plants.py --check --all` (428 apply, 0 stale) and `--self-test` | 0, 0 |
| `tests/m2/local_only.py --check` | 0 |
| `scripts/qa/probe_neutral.sh` | 0 |
| m2 trace emitter | 0 |

The acceptance run is on `785d08c6`. The commit after it changes only browser harness files and one client test file. NOT TRACED: acceptance on `f4020d1f`.

## 4. Own faults in the merged reorder path (MP1 wire)

1. `replica.rs:506`, `>` to `>=` (the window releases one tick early). Red: `ordw1_replica::a_lost_delta_holds_the_order_records_up_to_the_grace_then_asks_once`, 5 tests of `t8_resync_logic`, 6 of `ingest`. Restored (`git diff` 0 bytes), green: 22 + 9 + 23.
2. `ingest.rs:422`, the Welcome grace ignored (0). Red: 8 tests of `ingest`, among them `reordered_batch_12_then_11_sends_no_request` and `a_catchup_flush_of_five_in_reverse_sends_no_request`. The `ordw1` and `t8` tests set the grace on the replica, so they stay green (as expected). Restored (0 bytes), green.

Logs: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/forward-mc/fault1-red.log`, `fault2-red.log`, `fault-green.log`.

## 5. Frozen values

| Value | Before (4553acc4) | After (f4020d1f) |
| --- | --- | --- |
| `tests/m3/m2_operation_stream.json` sha256 | `bcbf72c2...e370b` | same |
| stream file written by the emitter | `bcbf72c2...e370b` | same |
| trace body `acme` (`tail -n +2`) sha256 | `ba1c849a...19856` (802 lines) | same |
| trace body `base` | `d78dc093...0c822f0c` (2898) | same |
| trace body `cedar` | `09f63732...e540ae` (4712) | same |
| trace body `haul` | `887ce95f...6074f` (4820) | same |
| `probe_neutral` | `frame_chains=d0d1b1c572163424 plain_unit_frames=71543 pooled_unit_frames=34093`, `muster_world=aa9db740578ed5c1` | same |

The body files are compared with `cmp` (`trace-base-bodies.txt`, `trace-after-bodies.txt` in the scratchpad). The trace header lines are also equal.

## 6. Findings

1. High (sound rule). The MC probes `docs/measurements/probes/mc-gpu-process/pacing-probe/probe.js:225-228` and `probe-a/probe-a.js:155-158` start a headed Chrome (`headless: false` in `probe-a.js:157`) with arguments only from `MC_CHROME_ARGS_JSON`, and with no `silentEnv`. The null device is not an argument in their own text. No gate runs them. They are evidence files under `SHA256SUMS`, so I did not edit them. Fix: needs decision. Options: (a) leave them as frozen evidence and mark them "do not run"; (b) add `SILENT_ARGS` and `silentEnv` and record new sums.
2. Note (pre-existing in MP1, not changed by the merge). `tests/browser/harness/run.js:136` has a default Chrome argument list with no `--alsa-output-device=null`. It depends on the headed wrapper. The scan test covers only `*.test.js`.
3. Low. `scripts/mc_prepare.py:49` does not record `T7_SCOUT_SPEED`. A `--prepared` T7 run can use a slow scout from its prepare step with no sign in the seal.
4. Medium (design change in the merge). The gate profile of the M2 convergence now polls at about 270 to 320 ms (was 100 ms) and asks both windows at once. Unit tests of both sides pass. A real GPU run of the convergence cases is not done.

## 7. Headed rows for the orchestrator (I ran none)

- The M2 browser convergence cases (`*-public-convergence`, local-only on hosted): the merged sampler.
- T7 (`scripts/t7.sh`).
- MC rows whose drivers or oracles came in: SLOT1 (stored frame limit through `client-settings-seed.js`), MM1, ORD3, ORD4, ORD5, HUD2, AUD1, ALR1, PER1-binding, SEL3, CAM1, CLICK9, HD1, NOORD.
- One row with a reload and scene checkpoints (for example AUD1): `driver-io.js:318` now uses `waitForDebugRecord`.
- The frame pacing applies to every headed row.

## 8. NOT TRACED

- Acceptance on `f4020d1f` (it ran on `785d08c6`).
- The headed rows of section 7.
- That lightyear puts all fragments of one large delta in one flush (the grace assumes reorder only inside one flush).
- The hosted CI run of this branch.
- That the 270 ms gate poll finds a shared tick in 10 s on a real GPU.

## Follow-up

FOLLOWUP=PASS

- New head: `37c41e70c8b0521af53d70ac82c229ed8fe228c2` (one commit on `f4020d1f`). Branch `mp1/forward-mc` is pushed.
- No browser was started. The probes were not run.

### Finding 1 (ruling b): the two MC probes

- `docs/measurements/probes/mc-gpu-process/pacing-probe/probe.js:14,230-231` and `probe-a/probe-a.js:15,159-160`. Each probe imports `tests/browser/harness/silent-chrome.js`. The launch always has `...SILENT_ARGS` (`--alsa-output-device=null`) as an argument and `env: silentEnv(work)` (`PULSE_SERVER=unix:/nonexistent/mc-tests-no-pulse`, `ALSA_CONFIG_PATH` set to a null configuration).
- New sums: `probe.js` `35a84c07aaaaa9c4642bb2b595625dcd51b0e43db81233c2737c2ac5f9ce1489`, `probe-a.js` `fe16213845c4cfbe5fafc85602ede7c75fb0f86db05516581d59bd240f061f71`. `sha256sum -c` passes in both folders.
- The note `docs/measurements/mc-gpu-process-saturation.md:8` says why the files changed, and gives the old sums of the measured files: `probe.js` `c395df41...a7049868`, `probe-a.js` `4e071efa...9f57036`.

### Finding 2: `harness/run.js` and the scan

- Why the scan did not see `run.js`: the scan test read only `*.test.js` under `tests/browser` (`testFiles`, `silent-chrome-scan.test.js:54-61`). Harness, instrument and probe files were outside its scope, and `docs/measurements/probes` was not read at all.
- The extended scan is `silent-chrome-scan.test.js:66-108`. It reads every `.js`, `.mjs` and `.cjs` file under `tests/browser`, `scripts` and `docs/measurements/probes`, and every `.sh` file under `scripts` and `docs/measurements/probes`. It uses the existing rules (`unsilenced`, `scriptProblems`). It also checks that `harness/run.js`, `probe-a.js` and `scripts/mp1-play.sh` are in its scope.
- The wider scan found three more files with the same defect. I repaired them:
    - `tests/browser/egpu-run.js:35,145-146`
    - `tests/browser/measure.js:22,24-25,47-49`
    - `tests/browser/spike_p8.js:9-12,26-27`
- Lines touched in `run.js`, kept small because of the `mp1/sound-off-default` lane: `:12` (the require), `:137` (the default list holds `SILENT_ARGS`), `:156-158` (the comment, `args` always with the null device, `env: silentEnv(options.outDir)`).
- Scan proof:
    - Old `run.js` in place: red. `[['tests/browser/harness/run.js', '.launch( has no silentEnv and SILENT_ARGS']]`
    - Old `pacing-probe/probe.js` in place: red. `[['docs/measurements/probes/mc-gpu-process/pacing-probe/probe.js', '.launch( has no silentEnv and SILENT_ARGS']]`
    - Both files restored (`cmp` equal): green, 7 of 7.
    - The test at `:102` keeps a planted copy of the old launch as a permanent red case.
    - Logs: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/forward-mc/scan-red-runjs.log`, `scan-red-probe.log`, `scan-green.log`.

### Finding 3: `scripts/mc_prepare.py`

- `scripts/mc_prepare.py:49-50`: `ENV_NAMES` includes `T7_SCOUT_SPEED`. A prepared folder made with another scout speed is refused (`build environment differs`).
- I added one case to `tests/qa/test_mc_prepare.py:492`. With the name removed from `ENV_NAMES`, the case was red (`a stale prepared run started its services`). After restore it was green (37 tests OK).

### Gate table (head `37c41e70`, label prefix `fwd-mc-g3-`, one run of each; every wrapper exit 0)

| Gate | Exit |
| --- | --- |
| fmt | 0 |
| clippy `-D warnings` | 0 |
| `cargo test --workspace --locked` (2106 passed, 0 failed, 40 ignored) | 0 |
| `turts_sim --features test-probes` | 0 |
| wasm32 check | 0 |
| boundary | 0 |
| acceptance (on this head; `a_server_stall_of_0_6_s_makes_no_resync`, `p8_one_flush_reorders_deltas_by_size`, `a_client_updated_each_100_ms_keeps_pace_with_the_server`, `ordered_attack_kill_...`, `t5_convergence` ok) | 0 |
| yardstick; completeness self-test and enforce; driver rule self-test and run | 0 each |
| node unit list (421 tests, 421 pass) | 0 |
| Python list check and the 10 other Python commands of CI | 0 each |
| plants `--check --all` (428, 0 stale), `--self-test` | 0, 0 |
| plants of the follow-up files (`--files`, 21 plants of `mc_prepare.py`: 21 red-restored, 0 failed) | 0 |
| `local_only.py --check` | 0 |
| `probe_neutral.sh` | 0 |
| m2 trace emitter | 0 |

### Frozen values at `37c41e70`

The stream sha256 is `bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b` (unchanged). The four trace bodies are equal to the base (`cmp` of `trace-base-bodies.txt` and `trace-final-bodies.txt`). The `probe_neutral` lines `frame_chains=d0d1b1c572163424 plain_unit_frames=71543 pooled_unit_frames=34093` and `muster_world=aa9db740578ed5c1` are unchanged.

### NOT TRACED (follow-up)

- The two probes and the four repaired instruments were not run with a browser. Only `node --check`, the scan and the node unit list ran.
- The silent environment on a headed `run.js` row: the headed wrapper sets the same values. No headed row ran.
