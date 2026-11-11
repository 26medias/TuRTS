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
published audit state (catalog, ledger, units and the rest). No budget applies.
