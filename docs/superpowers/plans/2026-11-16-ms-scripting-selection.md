# MS scripting selection: implementation plan

Spec: `docs/superpowers/specs/2026-11-16-ms-scripting-selection.md` (gate 1 repaired).
Owner: the orchestrator implements all tasks (the parts share one core; parallel lanes would
conflict). Reviewer: `mod-runtime-engineer` (gate 1, gate 2, final code review).
Root: `experiments/ms-scripting/` (standalone workspace, own `Cargo.lock`, target directory
`experiments/ms-scripting/target`, ignored). Raw evidence:
`/home/julien/Projects/TuRTS/target/mc-evidence/ms-scripting/`.

## Fixed inputs

| Item | Value |
| --- | --- |
| World seeds | W1 `0x5EED0001`, W2 `0x5EED0002`, W3 and W4 `0x5EED0003` |
| Event rate | W1: `12 + below(9)` (mean 16) and `Timer` each 20 ticks; W2: 256 and `Timer` |
| Units | W1, W3, W4: 1,000; W2: 5,000; 8 players, 4 unit types, 256 x 256 cells |
| Ticks | W1 to W3: 200 warmup + 2,000 measured; W4: 50 + 500 |
| Runs | 5 for each candidate and target; each run is a new process or a new Chrome launch |
| Kinds | `none` (W0), `rust` (`rust_ref`), `rhai`, `rune`, `lua` |
| L5 (b) keys | `k0` to `k63`, inserted in the order `(i * 37) mod 64` |
| CPU | native runs pinned with `taskset -c 4`; Chrome launched under `taskset -c 4-7` |

## Tasks

1. **Core and bindings.** `ms_core`: counting allocator, PCG32, world, host boundary (with the
   caps of spec 3.1), `rust_ref`, `none`, the three candidate adapters, the rule scripts
   `scripts/rules.{rhai,rn,lua}`, and the host policy (`Guarded`: per-instance accounting,
   request cap, disable on fatal errors). Check: `cargo test -p ms_core --all-features`
   (release) shows that each candidate gives the `rust_ref` hash for 300 ticks of W1 and W2.
2. **Probes.** `ms_core::probes`: L1 to L9 for each candidate, as script sources in
   `scripts/probes/`. Each probe returns a JSON record: id, pass, error kind, message, line,
   peak instance bytes, stop time (from the driver). Check: `ms_native probes --kind K` runs
   on all three.
3. **Calibration.** `ms_native calibrate --kind K` runs W1, W2 and W3 (2,200 ticks each) with
   no budget and prints the largest count of one call over the three. Write `4 x max`, rounded up to two significant
   digits, into `BUDGET_*` in `ms_core/src/cand/mod.rs`. No timing is recorded. Command and
   output go to the evidence directory.
4. **Wasm builds.** `tools/build_wasm.sh`: for each of `baseline` (no candidate feature),
   `rhai`, `rune`, `lua`: `cargo build -p ms_web --profile wasm-release --target
   wasm32-unknown-unknown`, then `wasm-opt -O3`, then `gzip -9 -k -n` and `brotli -q 11`.
   Print raw, gzip and brotli bytes and the import list (`tools/wasm_imports.py`, a standard
   library parser). `.cargo/config.toml` sets `-zstack-size=1048576` and
   `getrandom_backend="custom"` for wasm32 only.
4a. **Entropy backends (wasm32 only).** `ms_core` has direct optional dependencies on
   getrandom 0.2 (`custom` feature) and getrandom 0.3. It registers
   `register_custom_getrandom!` (0.2) and defines `__getrandom_v03_custom` (0.3). Both fill
   the buffer with a fixed byte pattern (spec 2.1). A non-empty wasm import list: the Chrome
   driver binds each import to a stub that throws and counts calls (spec L4).
5. **Drivers.** Native: `ms_native suite --kind K --run N --out P` (all workloads, raw samples
   in µs) and `ms_native probes --kind K --out P`. Chrome: `web/index.html` + `web/bench.js`
   (same call order as the native suite); `tools/chrome_run.mjs` serves `web/` and the wasm
   files with COOP and COEP headers, launches `/usr/bin/google-chrome` headless with a fresh
   profile and `--alsa-output-device=null`, and writes the same JSON schema. It uses
   `playwright-core` from `/home/julien/Projects/TuRTS/tests/browser/node_modules`. The page
   also records the timer resolution and `WebAssembly.Module.imports`.
6. **Measurements.** `tools/run_all.sh`: build first (shared lock), then all native and Chrome
   runs under one exclusive lock, with the load average before each run. Then
   `tools/aggregate.py` (standard library) writes `out/summary.json` and `out/summary.md`:
   p50, p90, p99, max, count per phase; B1 to B11 results; the margins and the 4.1 selection.
7. **Documents.** `docs/research/scripting-selection.md`, `docs/measurements/scripting.md`,
   `docs/reports/2026-11-16-scripting-selection.md`, `experiments/ms-scripting/README.md`.
   `README.md` progress only when the MS row passes. Copy `out/` to the evidence directory.
8. **Review.** The same reviewer runs the spike (builds, a native suite, the probes, one
   Chrome run) and reviews the code and the instrument. Fix material findings; the reviewer
   rechecks the repaired items only. Then commit with explicit paths.

## Red checks (each instrument can fail)

- `ms_native --mutant apply-now` applies each request inside the host call: L8 must go red.
  `--mutant no-request-cap`: L9a (the 100,000-`alert` case of spec L9; L9b is the 1,000-id
  group) must go red. `--mutant no-accounting` (no host per-instance
  check): L3 (d) must go red on Rune. Each run once, output recorded.
- `tools/wasm_imports.py --self-test` parses a hand-built module with one import and must
  report it.
- The `script` phase of `none` must have a p99 of 5 µs or less on native, and at most the
  measured timer resolution on Chrome, while its `sim` phase is larger: this shows that the
  script timer does not include `sim`.
- The aggregator marks a row FAIL when a measured workload has a script error (for example
  the budget was hit).

- Task 1 test: change one rule in one script (for example the bounty table) and the hash test
  fails. Run once and record.
- L3 (c) is red on Rhai without the host accounting (reviewer probe, gate 1); the run with the
  accounting must show the stop.
- L5 (b) compares hashes from separate processes; a per-process hash seed shows as two values.
- The aggregator marks a row FAIL when a number is missing.

## Disk and processes

Check free disk before builds (about 117 GiB at the start). The target directory stays below
10 GiB. Each Chrome profile is a new temporary directory that the runner deletes. No process
stays alive after a run.
