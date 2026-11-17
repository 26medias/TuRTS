# MS measurements: scripting candidates (native and Chrome wasm)

Date: 2026-11-16 (local time; the run ended at 2026-11-17T02:00:41Z).
Spec: `docs/superpowers/specs/2026-11-16-ms-scripting-selection.md` (budgets in section 4,
amendments in section 9). Decision: `docs/reports/2026-11-16-scripting-selection.md`.
Generated tables: `target/mc-evidence/ms-scripting/final/summary.md`; raw data:
`target/mc-evidence/ms-scripting/final/` (all paths are under `/home/julien/Projects/TuRTS/`).

## Method

| Item | Value |
| --- | --- |
| Hardware | Intel Core i9-14900K, 28 logical CPUs, 125 GiB RAM; a shared workstation |
| OS | Ubuntu 24.04.5 LTS, kernel 7.0.0-42-generic |
| Toolchain | rustc 1.99.1 (58cf8f9c3 2026-10-13); wasm-opt version 116; node v24.10.0 |
| Browser | Google Chrome 157.0.8103.38, headless, fresh profile for each launch, `--alsa-output-device=null`, `--mute-audio` |
| Crates | `rhai =1.27.0`, `rune =0.14.2`, `piccolo =0.3.3`; lockfile `experiments/ms-scripting/Cargo.lock` |
| Builds | Native: release, `opt-level 3`, fat LTO, one codegen unit, one binary for each candidate. Wasm: the root `wasm-release` settings, `panic = "abort"`, then `wasm-opt -O3`; one module for each candidate; no wasm-bindgen |
| Pinning | Native: `taskset -c 4`. Chrome: `taskset -c 4-7` |
| Lock | All timing and memory runs held `flock -x /tmp/turts-quiet-performance.lock` (2026-11-16 20:58:43 to 21:00:41 -0500). 1-minute load average 3.86 to 5.03 (`meta/load.txt`) |
| Samples | 5 runs for each entry and target (5 processes; 5 Chrome launches). W1 to W3: 200 warmup and 2,000 measured ticks; W4: 50 and 500. Pooled: 10,000 samples (W4: 2,500) |
| Timing | Native `std::time::Instant` around each phase. Chrome `performance.now()` around each exported call; cross-origin isolated (COOP and COEP); measured timer resolution 5 µs. Chrome values are therefore multiples of 5 µs |
| Memory | A counting global allocator in the harness on both targets. B6 = the largest instance peak over W1 and W2 (per-instance call-delta accounting, spec 3.5) minus that of `rust_ref` (240 bytes). B7 = `memory.buffer.byteLength` after W2 minus that of `rust_ref` (1,507,328 bytes) |
| Sizes | `gzip -9 -n` and `brotli -q 11` of the module after `wasm-opt -O3`; the zero is the baseline module (same harness, `none` and `rust_ref` only) |
| Budgets | Instruction budget for one call = 4 x the largest call over W1 to W3 in a calibration run, rounded up to two significant digits: Rhai 92,000 operations (22,772), Rune 51,000 instructions (12,561), piccolo 130,000 fuel (31,014) |

Workloads (spec 3.3): W1 = 1,000 units, mean 16 events for each tick; W2 = 5,000 units,
256 events for each tick; W3 = one call with 1,000 `count_owned` host calls; W4 = one call for
each of 1,000 units (anti-pattern, recorded only). "Script" is the script phase only: event
input, the call, its host calls, and request collection.

## Script cost for each tick (µs)

| Workload | Target | `rust_ref` p99 | Rhai p50 / p99 / max | Rune p50 / p99 / max | piccolo p50 / p99 / max |
| --- | --- | --- | --- | --- | --- |
| W1 | native | 3.56 | 25.91 / 37.41 / 67.46 | 7.70 / 13.63 / 420.79 | 8.96 / 14.79 / 38.47 |
| W1 | Chrome | 10 | 35 / 130 / 335 | 20 / 60 / 805 | 15 / 45 / 170 |
| W2 | native | 125.97 | 445.57 / 644.49 / 1,038.94 | 169.84 / 229.09 / 392.41 | 196.27 / 315.98 / 428.35 |
| W2 | Chrome | 140 | 585 / 790 / 1,210 | 225 / 295 / 485 | 270 / 345 / 515 |
| W3 (1,000 host calls) | native | 1.21 | 263.27 / 343.27 / 1,382.90 | 52.25 / 61.15 / 461.98 | 64.05 / 124.20 / 719.27 |
| W3 (1,000 host calls) | Chrome | 5 | 360 / 635 / 845 | 80 / 95 / 460 | 120 / 165 / 285 |
| W4 (1,000 calls, recorded only) | native | 7.04 | 367.79 / 493.08 / 643.12 | 106.70 / 189.93 / 208.15 | 87.75 / 93.50 / 183.66 |
| W4 (1,000 calls, recorded only) | Chrome | 15 | 375 / 660 / 790 | 150 / 170 / 300 | 120 / 135 / 225 |

The p99 of each run is in `summary.md`. The largest spread of the per-run p99 is Rhai W2
native (541.9 to 791.5 µs). No measured run had a script error.

Host-call cost (W3 median / 1,000): native Rhai 0.263 µs, Rune 0.052 µs, piccolo 0.064 µs;
Chrome Rhai 0.360 µs, Rune 0.080 µs, piccolo 0.120 µs.

No-script tick (W0, `none`): native `sim` p99 5.88 µs (W1) and 24.98 µs (W2); Chrome 15 µs and
55 µs. The `none` script phase has p99 0.026 µs native and 5 µs (one timer step) on Chrome, so
the script timer does not include `sim`.

## Download size added (bytes)

| Module | Raw after `wasm-opt` | Gzip | Brotli |
| --- | --- | --- | --- |
| Baseline (zero) | 96,098 | 33,855 | 27,648 |
| Rhai | +1,993,982 | +501,599 | +340,049 |
| Rune (C2, core modules) | +3,685,760 | +1,003,799 | +695,818 |
| piccolo | +518,075 | +183,187 | +142,235 |
| Rune, broad module set (`rune-full`, recorded only) | +3,982,575 | +1,052,498 | +719,695 |

Each module has no imports (`tools/wasm_imports.py`). SHA-256 values are in
`wasm/sizes.json`.

## Memory added (bytes)

| | Rhai | Rune | piccolo |
| --- | --- | --- | --- |
| B6 instance peak, native | 266,666 | 1,614,030 | 198,013 |
| B6 instance peak, Chrome | 196,724 | 1,132,942 | 183,661 |
| B7 wasm linear memory after W2 | 327,680 | 1,376,256 | 262,144 |
| Runtime creation time, native (µs) | 433 to 478 | 2,081 to 2,145 | 225 to 238 |
| Runtime creation time, Chrome (µs) | 15,700 to 19,400 | 46,705 to 50,875 | 8,445 to 10,545 |

## Limits and determinism

| Probe | Rhai | Rune | piccolo |
| --- | --- | --- | --- |
| L1 runaway: stop time max (native / Chrome, ms) | 0.115 / 1.240 | 0.098 / 1.340 | 0.452 / 1.075 |
| L2 recursion | depth error | budget error (no depth limit) | budget error (no depth limit) |
| L3 (a) to (e) memory, both targets | pass; L3 (c) map peak 34,130,863 | pass | fail: (a) native abort "memory allocation of 4294967296 bytes failed", Chrome trap `unreachable`; (c) peaks 221,302,781 and 199,796,487; (e) peak 1,073,750,867 |
| L4 ambient access | pass | pass | pass |
| L5 (a) W1 / W2 hashes | `fee8b2feb61fc457` / `7996aa16996e51b5` on every run and target, equal to `rust_ref` | same | same |
| L5 (b) map order, 5 processes | one hash, equal on both targets | one hash for each target (native `007c98d5182d93a5`, Chrome `1fe4cfc97381a4c5`) | native: 5 different hashes; Chrome: one |
| L6 recovery | pass | pass | pass |
| L7 errors with line | pass | pass | (a) pass; (b) fail: "runtime error: cannot add values", no line |
| L8 deferred requests | pass | pass | pass |
| L9 request cap and group cap | pass | pass | pass |

Red checks (each instrument goes red on a defective build; `target/mc-evidence/ms-scripting/red-checks/`):
`--mutant apply-now` makes L8 fail on all three; `--mutant no-request-cap` makes L9a fail on
all three; `--mutant no-accounting` makes every L3 variant fail on Rune;
`wasm_imports.py --self-test` finds one import in a hand-built module; the equivalence test
fails when one bounty value changes.

## Limits of these numbers

- The world, the events and the requests are mocks (spec 6). The numbers are the cost of the
  script layer at the stated counts, not of the engine.
- Chrome timing has a 5 µs resolution. Small Chrome values (W1 p50) are only accurate to that
  step.
- The machine is a shared workstation. The exclusive lock stops other measured work, but the
  load average was 3.86 to 5.03.
- Whether Chrome finished its compiler tier-up within 200 warmup ticks was not checked.
- The memory accounting needs one instance at a time on one thread.
- The first run (before amendments 1 to 5) is kept in `target/mc-evidence/ms-scripting/first-run/`.
