# MP1 recorded measurements

Recorded, not judged. MP1 keeps its numeric limits in the design (section 6.7.8 and row LOB1); this file records what a run printed.

## Disconnect notice L, native release

Task `L-ROOM0`, 2026-11-12. L is the time from the end of a headless client process to the record `session_authority_removed` in the private session observer file (`<stats>.sessions.jsonl`). The record is written by the observer `on_session_removed` (`crates/turts_server/src/session_diagnostics.rs:383`), which fires when the `Session` component leaves the connection entity (the entity is despawned). Both clocks are the host unix clock of one machine.

- Machine: Intel Core i9-14900K, 28 logical CPUs, Linux 7.0.0-42-generic x86_64, `rustc 1.99.1`. Local machine, not the 4-vCPU target of D58. One server and one client, no other load from this test.
- Build: `cargo test --release --locked -p turts_server --test connection_end -- --ignored --test-threads=1 --nocapture` at the lane head `d7d7e6d` plus the new files. Server and client are release binaries.
- Tool: the process test `crates/turts_server/tests/connection_end.rs`. It prints `L_sigkill_ms` and `L_clean_exit_ms`.

| Case | Run | L (ms) |
| --- | --- | --- |
| SIGKILL of the client | `mp1-lroom0-conn-end-run1` | 9998.9 |
| SIGKILL of the client | `mp1-lroom0-conn-end-probe2` | 9996.8 |
| SIGKILL of the client | `mp1-lroom0-conn-end-probe3` | 9998.8 |
| Clean exit of the client (`--run-secs 4`) | `mp1-lroom0-conn-end-run1` | 9996.0 |
| Clean exit of the client (`--run-secs 4`) | `mp1-lroom0-conn-end-probe2` | 9996.9 |
| Clean exit of the client (`--run-secs 4`) | `mp1-lroom0-conn-end-probe3` | 9995.7 |
| SIGSTOP of the client (probe, not in the committed test) | `mp1-lroom0-conn-end-probe-sigstop` | 10000.3 |

The runs with `probe` in the label carried a temporary logging patch that is not in the commit (see `docs/research/mp1/trace-connection-end.md`). The patch only logs; the L values do not differ from the run without it.

What the figures show: L is the 10 s netcode timeout in every case, including a clean process exit. The transport signal on the same connection entity comes about 2 to 3 ms after the end (see the trace note). The record of this table is the despawn, not the transport signal.

Not measured: the browser (a tab close or reload), a release build on the 4-vCPU target, a server under load (the gate 1 probe ran at load 13 in a debug build, design 6.7.8).

## Transport signal on the connection entity, native release

Source: the network-engineer review probe of `L-ROOM0` (a temporary logging patch and a driver script, not a committed test): `target/mc-evidence/mp1-lroom0-review-probe-drive/command.log` (in the main worktree; the same log) (`probe-drive.log`). Time from the end of the client to `Add<Disconnected>` (lightyear) on the connection entity. The author probe of the first version (`docs/research/mp1/trace-connection-end.md`) agrees within about 1 ms.

| Case | Time to `Add<Disconnected>` | Reason text |
| --- | --- | --- |
| SIGKILL of the headless client | +4.3 ms | `ResetWithoutClosingHandshake` |
| Clean exit of the headless client | -3.9 ms (before the exit was read) | `ResetWithoutClosingHandshake` |
| SIGSTOP of the headless client | +9998.7 ms | `Disconnected by peer` (netcode timeout) |
| Raw WebSocket: close frame, then FIN | +0.4 ms | `Disconnected by peer` |
| Raw WebSocket: close frame, TCP held open | +0.9 ms | `Disconnected by peer` |
| Raw WebSocket: FIN, no close frame | +1.0 ms | `ResetWithoutClosingHandshake` |
| Raw WebSocket: RST | +0.9 ms | `ConnectionReset` |

A reload with the same id 500 ms after a SIGKILL is refused with "This game is open in another window" (today's code). Not measured: a real browser.

## UnitId lookup, scan and map

Task `L0-SIM`, 2026-11-12. Recorded, not judged. One lookup of a unit by its id: a scan of all entities (`world.iter_entities().find(..)`, the form that the sites used) against `UnitIndex::get`. The scan looks for the last unit, the worst case; the map takes the same id.

- Machine: Intel Core i9-14900K, 28 logical CPUs, Linux 7.0.0-42-generic x86_64. Local machine, not the 4-vCPU target of D58. Release build, `bevy_ecs` 0.20.0.
- Tool: the ignored test `unit_index::tests::lookup_cost` (`crates/turts_sim/src/unit_index.rs`): `cargo test -p turts_sim --release --locked --lib unit_index::tests::lookup_cost -- --ignored --nocapture` (run `mp1-l0sim-lookup-cost`). It times 200 scans and 200,000 map lookups of one id in a world of N units.

| Units | Scan (ns for each lookup) | `UnitIndex::get` (ns for each lookup) |
| --- | --- | --- |
| 1,000 | 8,062 | 12 |
| 5,000 | 29,937 | 13 |

The performance lens of the design measured 5.0 and 25.9 us for the scan and 0.026 us for the map (D section 9); this run is the same order. The map cost does not depend on the unit count in this range.

Sites that read the index after `L0-SIM`: `economy/production.rs` (`queue_production`, `cancel_production`, `run_production` for each active producer in each tick, `try_output`), `economy/transaction_m3.rs` `find_unit_entity`, `economy/transaction.rs` `inventory_owner`, and `validate_target` in `orders/apply.rs` (the two unit-target checks). The test of each site points the index entry at another entity and shows that the site follows the entry and does not scan.

The scan of the ordered attackers of a removed unit (`economy/transaction_m3.rs`, the `iter_entities().filter_map(..)` that builds `ordered_attackers`) is kept (design 9, site 2, second case: "if those writers are not in one place, lane 0 keeps the scan and records its cost"). The writers of `UnitOrders` are not in one place today (about 62 write lines, plan task `L0-WIRE`), so a reverse set has no one place to stay in step. It runs once for each removed unit and does one full pass of the world: the cost of the scan row at the unit count of the match (about 8 us at 1,000 units, about 30 us at 5,000; the pass gets two components in each entity, so it can cost more than the `find` of the table: NOT TRACED, not timed on its own). `L0-WIRE` writes every `UnitOrders` through one function; the reverse set belongs there.

## Two headed clients on one GPU, MC build

Task `E-GPU`, 2026-11-12. Recorded, not judged: MP1 sets no threshold for this figure (spec 15). The only rule is the fail condition of the plan (task E-GPU, "Fail condition and decision rule (qa M4)"): a window under 1 frame each second for 5 s, or a client disconnect.

VERDICT=PASS

### Machine, GPU and build

| Item | Value | Source |
| --- | --- | --- |
| Machine | Intel Core i9-14900K, 32 logical CPUs in `lscpu` and 28 usable (`nproc`, `os.cpus()`), 125 GB RAM, Ubuntu 24.04.5, Linux 7.0.0-42-generic. Local machine, not the 4-vCPU target of D58. Other lanes ran cargo and rustc on the same machine at the start of the run (2 `cargo` and 8 `rustc` processes in `receipt.json` `other_compilers_at_start`); their load was not measured. | `egpu-result.json` `host`; `lscpu` |
| GPU and driver | NVIDIA GeForce RTX 3080 (0x2206), driver 580.178.04; Chrome shows ANGLE on Vulkan 1.4.312, `NVIDIA-580.178.4.0`, Skia `GaneshVulkan` | `chrome-gpu.txt` lines 102, 111, 142 (text of `chrome://gpu` after the run); `nvidia-smi.csv` |
| `render_adapter` of `turts_scene` | `backend: BrowserWebGpu`, `device_type: Other`, `name: ""` (empty): the browser hides the adapter name. WebGPU `adapter.info` of each window: `vendor: nvidia`, `architecture: ampere` | `egpu-result.json` `windows[].render_adapter`, `webgpu_adapter_info` |
| Browser | Google Chrome 157.0.8103.38, headed, `DISPLAY=:99` (the Xvfb of `mc_headed_v6.sh`), 2 windows of 1280x720, device scale factor 1 | `egpu-result.json` `chrome_version`, `headless`, `display` |
| Chrome arguments | `--enable-gpu --use-angle=vulkan --enable-features=Vulkan --enable-unsafe-webgpu --disable-vulkan-surface --alsa-output-device=null` (the wrapper profile; the ALSA null device is the sound profile; no mute flag) plus `--force-device-scale-factor=1`. Playwright adds `--disable-background-timer-throttling` and `--disable-backgrounding-occluded-windows` (`chrome-gpu.txt` line 91), so a covered window is not throttled by Chrome. | `mc_headed_v6.sh:749`; `tests/browser/egpu-run.js` |
| Package | `mods/muster` 0.1.0, map `field` (64 x 48): 41 slot-0 troopers, 2 workers, a depot and a post; slot 1 owns 10 dummies and 1 post; 515 presented units in each window. Server lockfile `sha256:48f7e9096da29529cf51b3c1a3278d68e12c6724c5701f789eaecaadecaaa3ca` (`server.log`). Server release build, 20 Hz, `--tick-log`. | `mods/muster/maps/field.ron`; `server.log` |
| Toolchain | `rustc 1.99.1`, `trunk` build of the prepared folder `prep-egpu` (HEAD `132c828`, sealed, not stale) | `egpu-prep-5` receipt; `docs/qa/prepared-runs.md` |
| Commit of the run | `132c828` (the measurement code). The verdict function moved into `egpu-stats.js` afterwards (see "What the instrument can fail"); it gives the same verdict on this result. | `git log` |

### Which package gives two armed slots

None does. `grep -rl weapon mods` finds two files: `mods/forge/units/turret.ron` and `mods/muster/units/trooper.ron`. In `forge`, only the slot-0 turret has a weapon (range 8, ammo from the shell chain, `targets: (slots: Other)`); the two slot-1 drones have none (`mods/forge/units/drone.ron`; `mods/forge/maps/proving_ground.ron:5-14` gives the owners). In `muster`, only the slot-0 trooper has a weapon (range 4, damage 5, cooldown 1,000 ms; `mods/muster/units/trooper.ron`); the slot-1 dummies and post have none (`mods/muster/maps/field.ron`: 10 dummies and 1 post with `owner_slot: 1`). `acme`, `base`, `cedar` and `haul` have no weapon. The lead "forge" is true for one armed slot only. This run uses `muster`: the trooper needs no ammo chain, the scene holds 515 units, and the hosted MM1 row names the same package.

### Result

Run `egpu-run-1`, one server, two windows on one GPU, 600 s measured. Frame figures are the intervals between `requestAnimationFrame` callbacks, in ms. The wasm client runs one Bevy frame for each callback (`tests/browser/measure.js` header). The instrument is `tests/browser/egpu-run.js`; the statistics are `tests/browser/egpu-stats.js`.

| Figure | Window 1 (slot 0) | Window 2 (slot 1) |
| --- | --- | --- |
| Frames in 600 s | 17,320 | 16,107 |
| Mean frame rate | 28.9 | 26.8 |
| Frame time p50 | 33.3 ms | 33.3 ms |
| Frame time p95 | 50.1 ms | 50.1 ms |
| Frame time p99 | 83.3 ms | 83.3 ms |
| Frame time max | 183.3 ms | 200.1 ms |
| Lowest frame rate in any 5 s (sliding, 1 s step) | 15.8 (79 frames) | 14.0 (70 frames) |
| Seconds with no frame | 0 | 0 |
| Fail rule (under 1 frame each second for 5 s) | not met | not met |
| Page visibility in the samples | `visible` in all 60 | `visible` in all 60 |

| Figure | Value | Source |
| --- | --- | --- |
| Client disconnects | 0 (no `websocket_close`, `page_close`, `page_crash` or leave of the `Joined` phase during the 600 s; both windows ended in `Joined`; `server.log` shows 2 joins, slots 0 and 1, and no other line) | `egpu-result.json` `events: []`, `disconnect_events: 0` |
| Wait for the GPU lock | 260.3 s (the other lock waits of this task: 267.4, 28.2, 0.008 and 87.5 s for the four smoke runs) | `egpu-run-1/lock-receipt.json` `wait_s` |
| Lock hold | 615.1 s of the 1,800 s limit of `pre-merge` mode; status `ok`, no survivors, 0 yields. The 600 s measured period is inside the hold. | `lock-receipt.json` `hold_s`, `limit_s` |
| Frame rate by minute, window 1 | 27.6, 25.0, 24.4, 26.4, 28.4, 32.4, 26.9, 33.0, 32.1, 32.4 | `window1-frame-times-ms.json` |
| Frame rate by minute, window 2 | 29.6, 24.9, 24.2, 26.4, 26.7, 29.2, 25.7, 29.4, 27.8, 24.5 | `window2-frame-times-ms.json` |
| Frames in one second | window 1: 12 to 43; window 2: 11 to 39 | same |
| App frames against callbacks | the app frame counter (`turts_debug.frames.recent`) grew by 17,338 and 16,129; the callbacks were 17,320 and 16,107 (0.1 % apart) | `egpu-result.json` `app_frame_index_delta`, `raf_callbacks` |
| GPU during the run (nvidia-smi, 1 sample each second, 614 samples; the sampler also ran during the browser setup) | utilization mean 32.7 %, max 53 %; memory used mean 3,758 MiB, max 4,679 MiB of 10,240 MiB; power mean 65.1 W, max 128.7 W | `nvidia-smi.csv` |

The frame times sit on multiples of 16.7 ms (p50 33.3 ms, p95 50.1 ms): about 30 frames each second. The cause is NOT TRACED (Chrome's frame cadence under Xvfb with `--disable-vulkan-surface`, or the client). The GPU was at 33 % mean utilization, so the GPU was not the visible limit; this does not show what limits the frame rate.

### Combat

The package has one armed slot, so the combat is one side: the slot-0 window walks one selected trooper next to one target and orders it to attack (a round), about every 50 s; the slot-1 window moves one of its dummies about every 40 s. An attack order from out of range does not fire in this build: the trooper stops at 4.000025 from the target, outside the range of 4.0 (`egpu-smoke-3`: the tick log shows the trooper standing at (50.818394, 31.575668) from about second 20 to the end, and the target unit 45 still alive at second 80 (the health records were not read in that run); the sim code is `crates/turts_sim/src/orders/behaviours/attack.rs:30-34` and `economy/combat.rs:321`; root cause NOT TRACED). So each round first walks the trooper to 1.8 from the target, then orders the attack from inside the range.

- 10 rounds, one dummy each, at seconds 1, 51, 101, ... 451; each kill took 4.8 to 8.3 s (71.5 s of fire in total; `egpu-result.json` `combat`).
- The 11th round (the post, 200 health) did not get its walk point (`trooper did not arrive` after 30 s), yet the post lost its health by second 570. All 11 slot-1 units had 0 health at the last sample: the replicated health of the slot-1 units fell from 600 to 0 (`windows[].samples[].foreign_damage`).
- No combat after second 570. The package holds 600 health on the slot-1 side, so the content ends the fight in this run; the combat is not 10 minutes of continuous fire. About 110 s of the 600 s had fire, in 11 bursts.
- The slot-1 window sent 10 move orders for its dummies. In 2 of its steps it only moved its camera (no target was on screen). In its last 3 steps, from second 501, no dummy was left to move.

### How the figures were measured, and the measurement effect

- Frames: an init script wraps `window.requestAnimationFrame` in each window and pushes the timestamp of each callback while a flag is set (`tests/browser/egpu-run.js`, `FRAME_RECORDER`). It adds no frame loop. The arrays are read every 10 s. The period starts after both windows joined and a priming click closed the "Click to play" audio gate (a click on the page, not a sound setting). The 600 s period is by the host clock.
- Statistics: `egpu-stats.js` `summarize`. p50 and p95 are the sorted intervals at index `floor(p * (n - 1))`. The lowest 5 s is the smallest frame count in a sliding window of 5 whole seconds, divided by 5.
- Measurement effect: the instrument also loads and drives the page. It reads `turts_debug` and `turts_scene` every 10 s, and about 15 to 30 s of each 50 s while a round runs; it sends mouse and wheel input; both windows are raised before their actions (`bringToFront`, as `io.use` does in the pixel rows). It takes screenshots only before and after the period. In the 167 s that hold a round, window 1 ran at 27.0 frames each second on average, and in the other 433 s at 29.6; window 2, which has no round, ran at 26.8 and 26.9. The two causes (combat and the extra reads) are not separated: NOT TRACED how many frames the reads cost.
- A first run with the same code had no combat orders that worked (`egpu-smoke-2`, the audio gate took the first click); four short headed runs (`egpu-smoke-2` to `-5`, 70 to 130 s) built the combat round. They are not results.

### The hosted probe (plan step 2; no new hosted run was triggered)

Run 43195887161, branch `codex/mc-integration`, commit `214289106210` (`2142891`), event `pull_request`, started 2026-11-12T09:56:25Z, conclusion `failure` at 2026-11-12T11:16:46Z. Runner: `ubuntu-latest`, image `ubuntu-24.04`, image version `20261101.360.1`, runner image provisioner `20261028.624` (`gh api repos/26medias/TuRTS/actions/jobs/131106617235/logs`, lines 2 to 19). The Chrome profile in the `env` of the refutation job 131106616607 (log line 1912) is `["--enable-gpu","--use-angle=gl","--enable-unsafe-webgpu","--ignore-gpu-blocklist"]`; the CI comment at `ci.yml` lines 108 to 121 of that commit says the hosted path is software WebGPU. The `env` of the `t7-m3` job was not read (NOT TRACED).

- MM1 did not run in any hosted job. The workflow of that commit has no step for `scripts/mc-pixels.sh`, `scripts/mc-browser.sh` or the row MM1 (grep of the file at `2142891`: 0 hits); the MM1 row is `"runs": "headed"` in `tests/browser/rows/MM1.json:7`, and `scripts/mc-pixels.sh` skips a headed row without `MC_PIXELS_HEADED=1` and `DISPLAY` (the `SKIP $row headed` line). So plan step 2 as written (run MM1 on the hosted runner) needs a new CI step and a new hosted run; it was not done here, by the instruction of this task.
- Nearest two-window job: `browser t7-m3`, job 131106617244. Its step "M3 browser controls (F10, private Xvfb, software WebGPU)" runs `scripts/m3-browser.sh` (package `forge`, `"windows": 2` at `scripts/m3-browser.sh:97`). Result: success, 2026-11-12T10:30:36Z to 10:48:11Z (17 min 35 s); the log prints `PASS m3-oracle.js` (job log line 3363). The job also passed T7 (one window; 25 frames). The two windows ran, but the log has no frame rate: the frame figure of the hosted software path is the recorded text of the CI comment, "about 1 frame per second for each of two windows" (a local measure, `ci-run134-m3-sw-gl-3`; not a hosted figure).
- The other two-window row, `browser m2` (job 131106617235), failed before a browser started: `error: no matching package named 'ron' found ... required by package 'turts_checks'` (offline cargo, log lines 1957 to 1962). The 9 jobs `browser refutation ...` also failed; the one that was read says `M2_CASE_INFRA_FAIL for E9-browser-base-economy: runner exited before a browser case summary` (job 131106616607). This is a CI set-up cause, not a GPU or frame cause.

### What the instrument can fail, and how it was shown

- `egpu-stats.test.js` has 16 cases (`node --test tests/browser/egpu-stats.test.js`). Run in the scratchpad against changed copies of `egpu-stats.js`: 5 changes of `summarize` and 10 changes of `judge` each gave at least 1 red case (never-starved, 5 s average rule weakened, empty window not starved, p95 read from p50, negative timestamp counted, frame rule dropped from the verdict, disconnects ignored, `during_measure` ignored, each of the 4 event types removed one at a time, setup error ignored, 1 window judged, `pageerror` counted). Two changes survived at first (the p95 field was not asserted; an "empty run of 5 s" clause was dead code); the first was fixed with a test and the second by removing the clause.
- `judge` replayed on `egpu-result.json` gives PASS (the recorded verdict) and FAIL with count 1 when one `websocket_close` with `during_measure: true` is added.
- Not shown red on a real run: the disconnect path of `egpu-run.js` (no run killed a server or a page), and a real window that is starved. The headed runs were all green.
- `tests/browser/egpu-stats.test.js` is not in the CI step list of `ci.yml` (the browser job lists its node tests by name). The driver rule (`python3 -I -B scripts/qa/driver_rule.py tests/browser`) passes with the 3 new files (roles `instrument`, `pure`, `unit`).

### Not measured

From the review (performance-architect, 2026-11-12, `docs/research/mp1/review-e-gpu.md` on branch `mp1-design`):

- The instrument counts `requestAnimationFrame` callbacks. A callback is not proof of a presented frame.
- No one-window control run at the same scene exists, so the cost of the second window is not known.
- The run covers a scene of 515 units. `docs/measurements/mc.md` (VIS2) records p50 133 ms with 5,000 units in one window. A two-window pixel row on a scene of that size is not covered by this result.
- Each window has 2 timestamps outside the 600 s period; the counts inside it are 17,318 and 16,105. No percentile changes.
- `max` and `mean_fps` have no test that goes red on a wrong value. The verdict does not use them.

A run on a second GPU or on the hosted runner; a run with a person in a window; a run with more than 2 windows; the frame cost of the instrument itself; the cause of the 30 frames each second; any frame figure with a browser that is not Chrome 157; fog, audio output (the null device took it); the 4-vCPU target.

### Commands

Prepare (outside the lock), the run, and the evidence folder: `target/mc-evidence/egpu-run-1/` in the main worktree.

```
MC_JOBS=4 python3 -I -B .../gate-runner/exec_in_lane_v4.py --lane .../TuRTS-mp1-e-gpu --target mc-mp1-e-gpu --head 132c828 --label egpu-prep-5 --audio -- bash scripts/mp1-egpu.sh --prepare .../target/mc-mp1-e-gpu/prep-egpu
MC_JOBS=4 bash .../gate-runner/mc_headed_v6.sh --mode pre-merge --lane .../TuRTS-mp1-e-gpu --target mc-mp1-e-gpu --head 132c828 --label egpu-run-1 --audio -- bash scripts/mp1-egpu.sh --prepared .../target/mc-mp1-e-gpu/prep-egpu --secs 600
```

Wrapper `mc_headed_v6.sh` sha256 `1d3a0000e37cd7d001827cb32843807c45c5824a71640cccc323c2d090ebb675` (checked). The run exited 0 (`VERDICT=PASS`), `source_unchanged: true`.

## Order and weapon records on the wire, protocol 5 (L0-WIRE)

Task `L0-WIRE`, 2026-11-12. Recorded, not judged: MP1 sets no byte limit here (spec 15). The figures
come from the real sim (`headless_app`, package `muster`, 20 Hz) projected by `export::delta_for`,
the function that the server calls for each slot. The sizes are postcard bytes, the encoding of the
wire (`postcard` 1.1.3). They are the bytes of the named list inside one `WorldDelta`, not of the
whole message. Tool: `cargo test --locked -p turts_server --test ordw1 -- --nocapture` (the
`ORDW1-BYTES` lines). Machine: the local lane host (Intel Core i9-14900K), debug build; the byte
counts do not depend on the build. Not the 4-vCPU target of D58.

| Scenario | What the run printed |
| --- | --- |
| Idle: 702 idle units of slot 0 (200 spawned, the rest from the map), 100 ticks | `unit_orders`, `orders_cleared` and weapon records: 0 B in every delta (the first delta carries one weapon record for each unit, the status goes Ready to NoTarget once, and it is not counted). The full table is 702 records and 4,802 B: that is the `Snapshot` size, and the size of each delta before protocol 5. |
| One `MoveTo` on one unit | 1 changed record at the accept tick (tick 1, 6 B) and 1 at the arrival tick (tick 41, 6 B). 0 records in the other ticks, 0 records for any other unit. |
| 12 troopers that fire at the dummies for 100 ticks (3 kills) | 81 weapon records in 100 ticks, 627 B in all, 6 B for each tick on average; 95 of the 100 ticks carry 0 weapon bytes. No record repeats the previous record of its unit. |
| `Fired{unit, target, weapon}` | 6 B with two unit ids below 16,384 and weapon index 200 (limit of ORDW1 case C: 8 B). The writer is empty in lane 0 (the sim has no `Fired` rows yet): this is the size of the shape. |

What the figures do not show: the site, job, supply and match records (their sim rows are empty
in lane 0, so they cost 0 B); a fight of 200 units in a long run (the 12-trooper case has 3
kills only; ORDW1 case C with 200 riflemen is a row of `I-HEAD`); the bytes of the whole
`WorldDelta` at the medium workload of D57 (NOT TRACED here: no run of the medium workload).
