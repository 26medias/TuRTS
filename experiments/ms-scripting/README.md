# MS scripting selection spike

A standalone Cargo workspace (own `Cargo.lock`, own `target/`). It is not a member of the root
workspace and changes no engine crate. Spec: `docs/superpowers/specs/2026-11-16-ms-scripting-selection.md`.
Plan: `docs/superpowers/plans/2026-11-16-ms-scripting-selection.md`. Results:
`docs/measurements/scripting.md` and `docs/reports/2026-11-16-scripting-selection.md`.

## Layout

| Path | What it is |
| --- | --- |
| `ms_core/src/world.rs` | The mock world: units, grid, seeded events, request validation and application |
| `ms_core/src/host.rs` | The host boundary: 3 queries and 5 requests, the request cap, the test mutants |
| `ms_core/src/rules.rs` | `rust_ref` (the rules in Rust, the reference hash) and `none` (W0) |
| `ms_core/src/cand/` | The three adapters (`rhai.rs`, `rune.rs`, `lua.rs`) and the host policy `Guarded` |
| `ms_core/src/probes.rs` | Probes L1 to L9, with the probe scripts for each language |
| `ms_core/src/bench.rs` | The measured sequence (sim, script, apply) and the report fields |
| `ms_core/src/alloc.rs` | The counting global allocator (both targets) |
| `ms_core/src/entropy.rs` | The wasm32 getrandom custom backends (fixed byte pattern) |
| `scripts/rules.{rhai,rn,lua}` | The mod rules of spec 3.2, one for each candidate |
| `ms_native/` | Native driver: `suite`, `calibrate`, `probe` |
| `ms_web/` | Wasm32 `cdylib` with plain exports (no wasm-bindgen) |
| `web/`, `tools/chrome_run.mjs` | The Chrome page and its launcher (fresh profile, `--alsa-output-device=null`) |
| `tools/` | Build, run, probe, license and aggregation scripts |

## Commands

Run from this directory. Builds hold a shared lock; timing holds the exclusive lock.

```bash
# Equivalence test (each script gives the rust_ref hash; a changed rule must change it).
flock -s /tmp/turts-quiet-performance.lock cargo test --release -p ms_core --all-features

# Calibration of the instruction budgets (functional, no timing).
flock -s /tmp/turts-quiet-performance.lock cargo build --release -p ms_native
for k in rhai rune lua; do ./target/release/ms_native calibrate --kind $k; done

# Wasm modules, sizes and import lists (out/wasm/).
flock -s /tmp/turts-quiet-performance.lock tools/build_wasm.sh

# The full measurement: probes (shared lock), then suites and L1 (exclusive lock),
# then the aggregation into OUT/summary.{json,md}.
# (run_all.sh also runs tools/chrome_l1.sh under the exclusive lock and tools/aggregate.py.)
tools/run_all.sh out/final 5

# Red checks of the instruments (plan).
python3 tools/wasm_imports.py --self-test
flock -s /tmp/turts-quiet-performance.lock tools/native_probes.sh out/mutant-apply-now apply-now rhai rune lua
flock -s /tmp/turts-quiet-performance.lock tools/native_probes.sh out/mutant-no-request-cap no-request-cap rhai rune lua
flock -s /tmp/turts-quiet-performance.lock tools/native_probes.sh out/mutant-no-accounting no-accounting rune

# Recorded-only variant V1: Rhai checks the memory room on every operation.
flock -s /tmp/turts-quiet-performance.lock tools/native_probes.sh out/variant-rhai-guard rhai-guard-every-op rhai
```

The Chrome launcher needs `playwright-core` from `/home/julien/Projects/TuRTS/tests/browser`
(`npm ci` done there) and `/usr/bin/google-chrome`.
