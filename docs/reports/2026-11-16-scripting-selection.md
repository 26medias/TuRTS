# MS scripting selection: report

Date: 2026-11-16. Branch: `codex/ms-scripting-selection`. Milestone: spec 14 row 7 (MS), G14, D34.
Spec: `docs/superpowers/specs/2026-11-16-ms-scripting-selection.md`.
Plan: `docs/superpowers/plans/2026-11-16-ms-scripting-selection.md`.
Numbers and method: `docs/measurements/scripting.md`. Sources: `docs/research/scripting-selection.md`.
Code: `experiments/ms-scripting/`. Evidence: `/home/julien/Projects/TuRTS/target/mc-evidence/ms-scripting/`.

## Decision

**Rune 0.14.2 is selected** by spec rule 4.1. It is the only candidate that passes every row
B1 to B11 on both targets. Its score (smallest margin) is 0.043, on B4: it adds 1,003,799 gzip
bytes against a budget of 1,048,576.

MS selects the language only. It does not build the production runtime, does not change any
engine crate, and does not make Rune the engine default in code.

**CEO decision, 2026-11-16:** do not use the MPL-covered `smartstring` dependency.
Retain Rune and the permissive-license selection requirement for the scripting runtime.
No license exception is granted for Rhai.

The measured core-module configuration is accepted for this selection spike.
It is not a promise that the full production script API is implemented.
Adding modules or integrating Rune with the engine dependency graph requires validation
against the recorded size, sandbox and deterministic-behavior requirements.
The post-run configuration choice and its small margin remain explicit below.

## Budget results (budgets fixed before the runs; none changed)

| Row | Budget (native / Chrome) | Rhai | Rune | piccolo |
| --- | --- | --- | --- | --- |
| B1 W1 script p99 (µs) | 500 / 2,000 | 37.4 / 130 | 13.6 / 60 | 14.8 / 45 |
| B2 W2 script p99 (µs) | 5,000 / 15,000 | 644.5 / 790 | 229.1 / 295 | 316.0 / 345 |
| B3 host call, median (µs) | 1.0 / 3.0 | 0.263 / 0.360 | 0.052 / 0.080 | 0.064 / 0.120 |
| B4 added gzip (bytes) | 1,048,576 | 501,599 | 1,003,799 | 183,187 |
| B5 added raw (bytes) | 4,194,304 | 1,993,982 | 3,685,760 | 518,075 |
| B6 added instance peak (bytes) | 8 MiB | 266,666 / 196,724 | 1,614,030 / 1,132,942 | 198,013 / 183,661 |
| B7 added wasm memory (bytes) | 16 MiB (Chrome) | 327,680 | 1,376,256 | 262,144 |
| B8 limit probes | all pass | pass | pass | **FAIL** (L3 a, c, e; L7 b) |
| B9 runaway stop (ms) | 50 / 100 | 0.115 / 1.240 | 0.098 / 1.340 | 0.452 / 1.075 |
| B10 determinism | equal | pass | pass | **FAIL** native (map order changes with each process) |
| B11 build and license | permissive | **FAIL** (smartstring MPL-2.0+) | pass | pass |

Tradeoffs measured:
- **Rune** is the fastest scripted candidate (2.7x to 2.8x faster than Rhai at W2 p99) and
  has a real per-call memory limit (`rune_alloc::limit`). Its download margin is small (4.3 %).
  It uses a reduced module set (amendment 5). It is a 0.x crate. Runtime creation takes about
  2.1 ms native and 47 to 51 ms in Chrome.
- **Rhai** has the largest margins on size and memory, the only call-depth limit, a 1.x
  version, and map order that is equal on both targets. It is the slowest of the three. It has
  the license issue.
- **piccolo** is small and fast, but its sandbox fails real probes. One built-in call (`..`,
  `table.unpack`) can allocate gigabytes before the host can check. Runtime errors have no
  line. Table order changes with each native process, because piccolo enables ahash
  `runtime-rng`. The crate describes itself as "very experimental".
- **All three** stay far inside the time budgets. The W4 anti-pattern (one call for each of
  1,000 units) costs 88 to 368 µs native at p50, against 5.3 µs in Rust. This confirms spec 5.1:
  batched event calls, with the heavy work in Rust.

## Changes after the first run (spec section 9)

The first full run (`first-run/`) gave **MS RED**. That run had two instrument defects
(amendments 1 and 2) and one harness defect (amendment 3). The final review found two more
items (amendments 4 and 5). No budget changed. Each change is in spec section 9:

1. The host counted request payloads as instance memory. Repaired; B6 run again.
2. L9a could not go red. Repaired (4,200 requests); the red check now goes red.
3. One native binary held all three candidates, so ahash features unified across them. Rune's
   native map order failure came from this. Repaired: one binary for each candidate.
4. The Rhai memory check ran every 256 operations; a probe overshot the cap. Now it runs on
   every operation. Rhai L3 (c) now passes.
5. **A configuration decision, not a repair.** The Rune module list was not fixed in the spec.
   The broad set failed B4 by 4,167 bytes. C2 now installs the core language modules only. The
   broad set stays recorded as `rune-full`. This choice came after the first run, and it
   changes the result from RED to PASS. Without it, no candidate is eligible.

The selection uses amendment 5. The CEO rejected the MPL alternative. Production module
expansion must measure B4 again: `rune-full`
adds 1,052,498 gzip bytes and fails B4.

## Validation

| Check | Result | Evidence |
| --- | --- | --- |
| Equivalence: each script gives the `rust_ref` hash; a changed bounty makes it fail | pass (2 tests) | `cargo test --release -p ms_core --features rhai,rune,lua -- --test-threads=1` |
| Final measurement, 5 runs, both targets, exclusive lock | 30 native and 30 Chrome suites, exit 0, 0 script errors | `final/meta/*-exit.txt`, `final/summary.md` |
| Probes L1 to L9, 5 processes and 5 launches for each candidate | as in the table | `final/probes/` |
| Red checks: `apply-now` (L8), `no-request-cap` (L9a), `no-accounting` (L3 on Rune), import parser self-test | each goes red | `red-checks/` |
| License check over all transitive dependencies, both targets | Rune 49 and piccolo 28 packages permissive; Rhai 1 MPL-2.0+ | `final/meta/licenses.json` |
| Gate 1 (spec), gate 2 (plan), final code-running review, recheck of repairs | gate 1 FAIL then repaired; gate 2 PASS_WITH_FINDINGS; final PASS_WITH_FINDINGS (material findings fixed: amendments 4 and 5); recheck PASS_WITH_FINDINGS (amendment 5 recorded as a decision) | reviewer: `mod-runtime-engineer` (5 invocations, including two repair rechecks); the recheck reran the aggregator with one L1 run removed (RED) and with Rune gzip +44,778 bytes (RED) |
| Chrome hygiene | fresh profile for each launch, `--alsa-output-device=null`, no console output, 0 import calls, profiles deleted | `tools/chrome_run.mjs`, `final/summary.json` |

Commands: `experiments/ms-scripting/README.md`. The single full command is
`tools/run_all.sh out/final2 5`, run from `experiments/ms-scripting/`.

## Not checked, and limits

- The world, events and requests are mocks; no Bevy, lightyear, package loading, or mod
  archive was involved. Scripts were embedded with `include_str!`.
- The production runtime is not built. Open for it: the thread model (is a Rune `Vm` `Send`;
  one VM for each match), per-instance accounting when matches share a thread with interleaved
  calls, script loading from packages, the public script API, and error delivery to modders.
- Rune map iteration order is stable within one target but differs between native and wasm
  (recorded only; spec 6.2 does not require cross-platform determinism). Mod code that sends
  requests in map order gives different requests on the two targets.
- Rune's reduced module set has no `HashMap`, `HashSet`, `VecDeque`, `bytes`, `char`, `tuple`
  methods or macros. A later API task decides which modules mods get, and B4 must be measured
  again then: the margin is 44,777 bytes.
- Chrome timing has 5 µs resolution; compiler tier-up after 200 warmup ticks was not checked.
- The timing ran on a shared workstation (load average 3.86 to 5.03), with the exclusive lock.
- Rune has no call-depth limit; recursion is stopped by the instruction budget (L2).

Integration note: five clippy style warnings remain in the isolated spike. No production engine code changes in MS. The orchestrator rechecks the recorded acceptance aggregation; the native/Chrome measurement matrix is not repeated.
