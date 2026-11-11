# MC Rendering Measurements

## VIS2

Measured 2026-11-09 from source head `40b9d90effc3f20e3e830f8018cabd9925ad4849` using the headed Chrome profile under an isolated Xvfb display (`:100`). The unchanged VIS2 server row loaded 515 authored Muster units plus 4,485 synthetic units; the browser reported 5,000 replicated world units and 5,000 rendered scene units. Chrome 157.0.8103.38 used Vulkan WebGPU flags with SwiftShader disabled.

The host inventory identifies an Intel Core i9-14900K and NVIDIA GA102 GeForce RTX 3080 (`10de:2206`), driver 580.178.04. Bevy reported `BrowserWebGpu` / `Other` with an empty adapter name (redacted by the browser); WebGPU API metadata for the configured application device reported vendor `nvidia`, architecture `ampere`. The application acquired and configured its device, recorded 89 current-texture acquisitions and 1,529 submitted draw calls, and reported no instrumentation errors. `configuredContexts` is a count of configuration events, not a claim about distinct contexts.

| Profile | Metric | Value | Unit |
| --- | --- | ---: | --- |
| VIS2 | p50 | 133.3 | ms |
| VIS2 | p99 | 199.9 | ms |
| VIS2 | draw calls | 1529 | calls over 10 s |
| VIS2 | memory | 505872384 | bytes |

The capture recorded 86 frame intervals, a 17.375 mean submitted draw calls per RAF interval, and a 5,611 ms first frame. No acceptance threshold is specified here. GCP is pending (D59).

The raw measurement is `target/mc-evidence/t13-vis2-final-40b9d90-20261109/vis2-final-actual.json` (SHA-256 `d58dd8ed430a7a2b39564f137e98c33aac9dec0118103b5474b7807e9f792df0`). Its command receipt binds the headed browser argv, served URL, source hashes, and configured-device proof. The same evidence directory contains the server run, exact grep red/green logs, and raw `lscpu`, `lspci -nn`, and `nvidia-smi` inventories.

### Historical measurement

The earlier 2026-11-09 measurement used source head `9cf23638214e79a2800bbe1bf8a84d9a4f92b45a` on `DISPLAY=:1`. It recorded p50 133.3 ms, p99 183.3 ms, 1,530 total submitted draw calls over 10 seconds (17.191 mean per RAF interval), 558628864 bytes of WebAssembly memory, and an initial frame at 4,983 ms. This is retained as historical evidence and is not the final-source result. Its raw JSON remains at `target/mc-evidence/t13-vis2-9cf2363-20261109/vis2-actual-measurement.json` with the original source-bound receipts.

## Tick cost

Measured 2026-11-11 on commit `0983dc475074ff3d3a06d9c0111cd1978acba9ef` with
`cargo test --release -p turts_sim --test mc_cost --locked -- --ignored --nocapture`
(runner label `t22-run112-records-mc-cost-v3`, raw log SHA-256
`787bc779f9b79d194bbfafc55f1e2d395b85b5feb04635f92094d9025406883b`). These figures are recorded,
not judged. No budget applies (product spec section 15 stays Open).

Machine: Intel Core i9-14900K (32 logical CPUs; the process affinity allows 28), 125 GiB RAM,
Ubuntu 24.04.5 LTS, Linux 7.0.0-42-generic, rustc 1.99.1, Bevy 0.20.0, release profile with no
debug assertions. This is the development host, not the D57 allocation of 4 vCPUs and 8 GB. The
GCP benchmark VM is pending (D59).

Method:

- Fixture: the authored `muster` package (archive `sha256:9b546eb6c5f69a7089fbb34c7bb869a2846e194ff8cf67a3cdea48904fa0b121`), its parsed trooper,
  dummy and `order_move` definitions, on an isolated 128 x 128 copy of the `field` map. The copy
  holds only the test placements and no resource node (the authored node at (7, 9) overlaps the
  unit grid).
- Timed metric: one whole `app.update()` (one fixed tick at 20 Hz), so each figure includes all
  simulation systems. Audit reads (`audit_state`) are outside the timed section.
- Spreading: N troopers in a 1.1 grid, one `order_move` group to (64, 64), sent as parts of at
  most 4,096 units in one tick and one group (5,000 = 4,096 + 904). One warm-up fixture, then one
  timed fixture. The figure is the sum of every tick with spreading work (`movement_dispatch.used`
  above 0). Both sizes spread in one tick: all N units admitted, all N active, 0 pending.
- Avoidance: the same group after spreading, 10 warm-up ticks, then 20 timed ticks. All N units
  are moving in each timed tick.
- Combat: N/2 armed troopers (slot 0) and N/2 dummies (slot 1) 1.5 apart, inside the authored
  weapon range of 4. One warm-up instance, then 20 fresh instances with one timed tick each, so
  each timed tick fires with authored health and cooldown.

| Units | Phase | Samples | Min (ms) | p50 (ms) | Max (ms) | Mean (ms) | Work in each timed tick |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | --- |
| 1,000 | spreading | 1 | 6.069 | 6.069 | 6.069 | 6.069 | 1,000 units spread in tick 1 |
| 1,000 | avoidance | 20 | 3.092 | 3.167 | 3.241 | 3.170 | 1,000 moving units |
| 1,000 | combat | 20 | 2.438 | 2.508 | 2.940 | 2.533 | 500 armed, 500 targets in range, 500 `Fired` events |
| 5,000 | spreading | 1 | 28.783 | 28.783 | 28.783 | 28.783 | 5,000 units spread in tick 1 |
| 5,000 | avoidance | 20 | 18.583 | 19.183 | 20.132 | 19.262 | 5,000 moving units |
| 5,000 | combat | 20 | 12.671 | 13.255 | 15.131 | 13.293 | 2,500 armed, 2,500 targets in range, 2,500 `Fired` events |

With 20 samples, p99 equals the maximum. The raw per-tick nanoseconds are in the raw log.

## EL1 log size

Measured 2026-11-11 on commit `a9d5b2601b209f6ea030a3d64e39519cf6ccda77` by
`el1_muster_movement_and_queued_gather_validate_every_captured_tick`
(`crates/turts_server/tests/el1.rs`), runner label `t22-run112-P1-green`, with
`--nocapture`. The run is `muster` with `--economy-log`, `--run-ticks 420` and the command script
`tests/browser/fixtures/gm40-script.json` (GM40, the GMA crossing and a queued gather). Same
machine as above.

Method: for each captured `Tick.before` and `Tick.state`, the test groups the non-default
movement rows (`movement_progress`, `plain_movement_progress`, `physical_escapes`, `unit_orders`)
by unit id, sums the `serde_json` length of each row, and divides by the number of distinct
(captured state, unit id) pairs. It sums the encoded length of each pool slot (`Vec2`) and
divides by the slot count. Brackets, commas and the shared state, catalog and record bytes are
not attributed.

| Figure | Measured | Design 6.1 estimate |
| --- | ---: | ---: |
| Bytes per non-default unit, per captured state | 341.6 (6,668,125 bytes / 19,519 unit-states) | about 60 |
| Bytes per pool slot, per captured state | 15.4 (385,964 bytes / 25,048 slots) | about 20 |
| Whole economy log for 420 ticks | 6,464,387,050 bytes | - |

The whole log is not the sum of these figures: each record also carries the full prior and
published audit state (catalog, ledger, units and the rest). Recorded, not judged: no budget
applies. Cost item for the performance-architect: the whole log is 6.46 GB for 420 ticks (about
15.4 MB per tick), so a long match with `--economy-log` on needs a size decision.

## GMU and GMA `Blocked` counts

Measured 2026-11-11 on commit `fd057b7ccb18afddf5d52eefbeea04a6cdb6c842` (RUN122) and repeated
on commit `145df1e5a3917e1b678b5fb959cbbe164bfe742c` (label `t22-run122-h2-plan-native`, exit 0)
with the same 13 record lines. These are
counts from a deterministic fixed-tick simulation, not timings, so the parallel Task 21 build
on the same host does not change them.

Rule: design revision 15 (ruling O14). A unit whose slot is behind a resting group of another
envelope can stop `Blocked` with a reachable target (6.5). These counts are recorded and not
gated, except where the table says "gated". M4b (push, chokes) owns this limit, and its exit rows
gate the `muster` reinforcement case at 0 `Blocked`.

Method: `cargo test --release -p turts_server --test mc_group --test det1 --test el1 --locked --
--ignored --nocapture` (runner label `t22-run122-plan-native`, exit 0, raw log SHA-256
`f527ee302a4133df755cc94b0ff31e26c4a586d269f62003429626cf9af325f6`) on `muster` (map `field`,
64 x 48; trooper radius 0.5, speed 3; 20 Hz) with the fixed-tick `--command-script` hook. The
count comes from the `Blocked` lines of `--order-log`; the rest position comes from the tick log
at the end tick. The GMU tests print `GMU record case=...` lines; for GMA the positions come from
the same run's tick log at the end tick (copies and hashes in
`target/mc-evidence/t22-run122/records/`). The native slot check is
`cargo test --release -p turts_sim --test gmu_same_point --locked -- --nocapture` (label
`t22-run122-gmu-slot-release`, exit 0, raw log SHA-256
`67c96473e5c5699a569d5f8335e91cfd16a3858dcadf09baaec7a8f50046ace7`).

Machine: the development host of "Tick cost" above (Intel Core i9-14900K, Ubuntu 24.04.5 LTS,
Linux 7.0.0-42-generic, rustc 1.99.1, Bevy 0.20.0, release profile).

| Case | Points and orders | End tick | `Blocked` units | Rest position of each (unit id, tick, x, y, distance to the point) |
| --- | --- | ---: | ---: | --- |
| GMU same point, first order (gated at 0) | troopers 21 to 40 to (40, 30) at tick 1 | 700 | 0 | - |
| GMU same point, second order | troopers 1 to 20 to (40, 30) at tick 300 | 700 | 1 of 20 | 19, tick 599, (37.837635, 28.188488), 2.821 |
| GMU clamped corner, first order | troopers 1 to 20 to (70, 55), clamped to (63.5, 47.5), at tick 1 | 1300 | 1 of 20 | 18, tick 179, (28.404446, 20.295242), 44.405 |
| GMU clamped corner, second order | troopers 21 to 40 to (70, 55) at tick 300 | 1300 | 3 of 20 | 22, tick 587, (53.183846, 34.621994), 16.500; 40, tick 706, (60.22587, 45.695618), 3.738; 39, tick 745, (62.487587, 45.776867), 1.999 |
| GMA crossing | troopers 1 to 20 to (29, 35) at tick 1; troopers 21 to 40 to (50, 22) at tick 20 | 600 | 0 of 40 | - |
| GMA case I | troopers 21 to 40 to (29, 35) at tick 1; troopers 1 to 20 to (29, 44) at tick 240 | 700 | 3 of 20 (second order) | 18, tick 487, (27.390099, 32.234398), 11.875; 4, tick 525, (28.49244, 32.233013), 11.778; 5, tick 532, (29.586084, 32.23679), 11.778 (distances to (29, 44)) |
| GMU native slot check | the GMU same-point orders, native run | 700 | 1 of 20 (second order) | 19 at (37.837635, 28.188488), 2.8208861. Nearest slot distance 1.0999985 (bound s - 0.00001 = 1.09999; gated) |

The native run gives the same `Blocked` unit and the same rest position as the process run.

GMU same-point deadline (gated): unit 19 starts at (27.2, 20.2), d = 16.12 to (40, 30), so its
deadline is tick 300 + ceil(2 x (16.12 / 3) x 20) + 100 = 615. It stops `Blocked` at tick 599:
a margin of 16 ticks (0.8 s).

Clamped corner (recorded only). The second order starts at tick 300, before the first order
rests: the first order reaches `Done` from tick 288 to tick 412 (unit 18 stops `Blocked` at
tick 179). The cause of each `Blocked` unit, from the RUN122 tick log at its `Blocked` tick
(`target/mc-evidence/t22-run122/records/`):

| Unit | Order | `Blocked` tick | Contact at that tick (distance between centres) | Cause |
| ---: | --- | ---: | --- | --- |
| 18 | first | 179 | idle troopers 21 (1.000) and 28 (1.010) of the second order | idle rows on its route; idle units do not move for others (6.4) |
| 22 | second | 587 | slot-1 dummies at (54, 35.2) (1.000) and (54, 34) (1.026), alive at that tick | idle obstacle on its route (6.4), not a slot behind the resting group |
| 40 | second | 706 | resting first-order troopers 4 (1.000) and 12 (1.028) | a slot behind the resting group (6.5) |
| 39 | second | 745 | resting first-order troopers 14 (1.000) and 8 (1.029) | a slot behind the resting group (6.5) |

During the run the authored trooper weapons destroy slot-1 units in range: the post at
(61, 44) after tick 355, the dummies at (54, 35.2) after tick 727 and at (54, 34) after tick
887. So the obstacles of the corner change during the run. Unit 22 rests 16.5 from the point,
outside R_near; R_near is not gated in the corner case.
