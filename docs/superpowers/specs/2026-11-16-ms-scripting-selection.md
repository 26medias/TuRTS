# MS: scripting language selection spike

Status: gate 1 and gate 2 findings incorporated (B10 decided as option (b)). Date: 2026-11-16. Track: Foundation (the mod API contract, D34).
Spec rows: D34, D18 (scripting open), 5.1, 5.2, 5.3, 5.4, 5.5, 6.2 (G9), 6.9, 13, 14 row 7 (MS),
15 (Scripts row), G14.

## 1. Goal and limits

MS selects the scripting language for mod and library scripts (D34). The selection uses
numbers measured natively and in Chrome WebAssembly (wasm32), against the budgets in section 4.
Section 4 is fixed before any candidate run. Budgets do not change after a run.

MS is a selection spike. It does not build the production script runtime. It does not change
the engine crates, the root workspace and lockfile, the wire protocol, `ENGINE_API_VERSION`,
the mods, or the launchers. It does not make a language the engine default in code. It does not
complete M4b or M5.

Spec 14 row 7 is red when a number is missing or when no candidate meets the budgets.

## 2. Candidates

A candidate must be embeddable in Rust, build for `wasm32-unknown-unknown` with no Emscripten,
no WASI and no threads, and have a permissive license. The research note
`docs/research/scripting-selection.md` records the primary sources.

| Candidate | Crate (exact) | Language | License (crate metadata) | Limit controls in the crate |
| --- | --- | --- | --- | --- |
| C1 | `rhai =1.27.0` | Rhai | MIT OR Apache-2.0 | `set_max_operations`, `set_max_call_levels`, `set_max_expr_depths`, `set_max_string_size`, `set_max_array_size`, `set_max_map_size`, `on_progress` |
| C2 | `rune =0.14.2` | Rune | MIT OR Apache-2.0 | `runtime::budget::with` (instructions), `rune_alloc::limit::with` (bytes) |
| C3 | `piccolo =0.3.3` | Lua 5.x dialect, pure Rust | MIT | `Executor::step` with `Fuel`; `Lua::total_memory` (GC bytes) |

Screened out before the runs (reasons in the research note):

- `mlua` (Lua 5.x, LuaJIT, Luau): C or C++ sources. Its documentation supports
  `wasm32-unknown-unknown` only "with a WASI C runtime", and the host must provide the WASI
  imports. The no-WASI rule excludes it.
- `boa_engine =0.22.0` (JavaScript): `RuntimeLimits` has a stack size, a loop-iteration and a
  recursion limit, but no instruction budget. A script without a loop (for example a long
  chain of calls) is not bounded.
- `wasmi` (WebAssembly modules): spec 5.3 defers independently executable WASM modules.
- `koto`: its execution limit is a wall-clock timeout, which is not deterministic.

No candidate has a total-heap cap for one instance in the runtime itself. Rhai checks the size
of each value (map keys are not counted). Rune limits the bytes of one call
(`rune_alloc::limit::with`). piccolo counts the GC bytes of one instance (`Lua::total_memory`),
but does not cap them. So the host enforces the memory cap of each instance (section 3.5).

### 2.1 Build features and entropy (fixed before the runs)

| Candidate | Features | Entropy on wasm32 |
| --- | --- | --- |
| C1 | `default-features = false`, `std`, `no_time`, `no_module`, `no_custom_syntax` (no `sync`: one instance on one thread) | not needed when `ahash/runtime-rng` is off; the build proves it |
| C2 | `default-features = false`, `std` (no `emit`) | the build shows what it needs |
| C3 | crate defaults (piccolo enables `ahash` `runtime-rng`) | needed: the table hasher seeds from getrandom |

Native builds use the operating-system entropy source, as a production server does (the
server needs real entropy for identity and sessions, 6.13). Wasm builds use `getrandom`
custom backends (0.2 and, if linked, 0.3) that fill a fixed byte pattern. No script can call
them. A candidate whose results depend on entropy fails L5 (b) on native, because each native
run is a new process. Note for C3: with `ahash` `runtime-rng` (enabled by piccolo), the fixed
hasher seeds come from `getrandom::fill` (ahash-0.8.12 `random_state.rs:68-79`), and
`set_random_source` does not replace them. A host can only fix them by replacing the
process-wide getrandom backend, which the server needs for identity (6.13). So MS does not
apply that change; the report records it as a configuration limit of the crate.

## 3. The experiment

The experiment is a standalone Cargo workspace at `experiments/ms-scripting/` with its own
lockfile and its own target directory. One shared core library holds the mock world, the
workloads, the host boundary and the measurement code. A native binary and a wasm32 `cdylib`
call the same core functions. One wasm build exists for each candidate, and one for the
baseline with no script runtime, so the added download size is a difference of two files.

### 3.1 Host boundary (equivalent for every candidate)

A script never sees the world, Bevy, an entity, or a schedule. The host gives each script:

- **Input:** one batch of events for the tick. An event is a record of small integers:
  `Produced{player, unit_type, unit, x, y}`, `Died{player, unit_type, unit, killer, x, y}`,
  `Researched{player, tech}`, `Timer{id}`.
- **Queries** (read-only, executed in Rust): `count_owned(player, unit_type)`,
  `credits(player)`, `units_near(player, x, y, radius, max)` (a list of at most `max` unit ids;
  Rust does the spatial search on a grid).
- **Requests** (appended to a host buffer, never applied inside the script call):
  `grant(player, amount)`, `move_to(unit, x, y)`, `move_group(units, x, y)` (the batched
  request), `set_stance(unit, stance)`, `alert(player, code)`.

All values are integers (positions in fixed-point milli-cells). The host applies the requests
after the script call, with validation, as the engine would. Each candidate registers the same
eight functions with the same arguments.

Host caps (same for all candidates): `units_near` returns at most 256 ids; the host rejects a
`move_group` with more than 256 ids; one call can make at most 4,096 requests. Over the request
cap, the host drops the request, and the call ends with a budget error (section 3.5). The engine
gives at most 512 events in one tick.

### 3.2 Mod rules (one script for each candidate, same rules)

The script keeps per-match state: kills for each player, a rally point for each player, the
last leader. It handles each event of the batch:

- `Died`: add 1 to the kills of `killer`. Grant the bounty of the unit type from a script
  table; double it on each tenth kill. Query `units_near` around the death point for the
  victim; when fewer than 3 friends are near, send `alert`; else send one `move_group` of the
  returned ids to the rally point.
- `Produced`: query `count_owned`. Above 20, `set_stance` defend; else `move_to` the rally
  point.
- `Researched`: move the rally point of the player by a fixed offset.
- `Timer`: query `credits` for each of the 8 players; send `alert` when the leader changes.

The Rust-only comparison (`rust_ref`) implements the same rules in Rust behind the same host
boundary. It is the reference output.

### 3.3 Workloads

The world moves its units each tick in Rust and rebuilds the grid (the no-script engine work).
The event generator is a seeded PCG; the seed and counts are fixed in the plan.

| Id | Units | Events for each tick | Purpose |
| --- | --- | --- | --- |
| W0 | 1,000 and 5,000 | as W1, W2, no script | No-script baseline: the Rust tick alone |
| W1 | 1,000 | mean 16 (`Produced` and `Died` mixed, `Timer` each 20 ticks) | Nominal match, event-driven |
| W2 | 5,000 | 256 | Battle burst |
| W3 | 1,000 | 1 script call that makes 1,000 `count_owned` calls | Host-call cost |
| W4 | 1,000 | 1 script call for each unit | Anti-pattern, recorded only. Not the engine default (spec 5.1, 13.5) |
| WR | as W1, W2 | as W1, W2 | Rust-only rules (`rust_ref`) |

### 3.4 Limit probes

| Id | Probe | Pass condition |
| --- | --- | --- |
| L1 | Runaway: an endless loop in the tick handler | Stopped by the instruction budget with a budget error; wall time to stop at or below B9 |
| L2 | Unbounded recursion | Stopped with an error. No host stack overflow, no wasm trap, no process abort |
| L3 | Memory, measured as the live heap of the one instance in the process, over all its ticks. Variants: (a) a string that doubles; (b) a list that doubles; (c) many values, each below any per-value limit (a map with 512 KiB string keys, a list of 512 KiB strings); (d) kept state that grows by 64 KiB on each tick, for up to 1,000 ticks; (e) one built-in call that asks for about 1 GiB: Rhai `array.pad(67108864, 0)`, Rune `vec.resize(67108864, 0)`, piccolo `table.unpack(t, 1, 67108864)` (piccolo has no string repeat or pad) | Each variant ends with an error, and the instance heap never exceeds the cap of 32 MiB plus 1 MiB (the peak of the counting allocator). No wasm trap, no process abort |
| L4 | Ambient access: time, random, file and network IO, `print`, module import, `eval` or `load` of new source | Each probe gives "not found" or an error. No output appears on stdout or the console. On wasm: the module has no imports, or the driver binds each import to a stub that throws and counts its calls, and the count stays 0 in every run; the report lists each import |
| L5 | Determinism. (a) W1 and W2 output hash; (b) the script iterates a map with 64 string keys, inserted in a fixed order, and sends one `alert` for each key in iteration order. Each run is a new process (native) or a new Chrome launch (wasm); 5 runs on each target | (a) one hash for each workload over all runs, both targets, and `rust_ref`; (b) one hash over all runs on one target, for each candidate and target. Equality of (b) across the two targets is recorded only |
| L6 | Recovery: a tick after L1, L2, L3 and L9 | The host continues. The next call either runs correctly or returns a "script disabled" error that names the script |
| L7 | Useful errors | A script syntax error and a runtime type error report the script name and the line |
| L8 | Requests are deferred: the script reads `credits(p)`, sends `grant(p, 100)`, and reads `credits(p)` again in the same call | The two reads are equal. The next tick shows the grant |
| L9 | Request flood: one call sends 4,200 `alert` requests (amended from 100,000, see section 9); one `move_group` with 1,000 ids | The call ends with the request-cap error at 4,096 requests, and no request of that call is applied; the large group is rejected |

### 3.5 Limits (fixed before the runs)

- **Instruction budget** for one call, in the unit of the candidate (Rhai operations, Rune
  instructions, piccolo fuel). Rule: a calibration run of W2 (2,200 ticks, no timing, no B
  row) records the largest count of one call. The budget is 4 times that count, rounded up to
  two significant digits. The plan records the calibration command; the measurement document
  records the counts and the budgets. The same budget applies on both targets and in every
  measured run.
- **Call depth:** 32 where the runtime has a call-depth limit (Rhai `set_max_call_levels`).
  Rune and piccolo keep script frames on the heap; their recursion is bounded by the budget
  and the memory cap. Rhai expression depth: 64 (global), 32 (functions).
- **Wasm stack:** 1 MiB, set by the linker argument `-zstack-size=1048576`.
- **Per-value limits** (Rhai): string 1 MiB, array 65,536 items, map 4,096 entries.
- **Memory cap:** 32 MiB for one instance. The host accounts each instance by the counting
  allocator: the retained bytes of the instance are the sum, over its calls, of the live bytes
  after the call minus the live bytes before it. One instance runs at a time on a thread. In a
  call, each candidate checks retained bytes plus the growth of the call against the cap:
  Rhai in `on_progress` every 256 operations; piccolo between fuel steps of 4,096; Rune by
  `rune_alloc::limit::with(cap - retained)`. After each call the host checks the retained
  bytes again. Before the host reports a cap error for piccolo, it runs a full collection
  (`Lua::gc_collect`) and measures again, because `enter` collects only incrementally.
- **Policy:** a failed call applies none of its requests. A budget, memory, or depth error
  disables the script for the rest of the match, and each later call returns "script
  disabled" with the script name.

## 4. Budgets (set before any candidate run)

Context: the tick rate is 20 Hz (`DEFAULT_TICK_HZ`, `docs/measurements/m1.md`), so one tick is
50,000 µs. M1 measured a server tick p99 of 228 µs at 1,000 units. The current browser client
transfers 14,974,676 gzip bytes for its wasm (`docs/reports/2026-11-14-large-army-performance.md`).
Scripts run on the authoritative server (6.2). The wasm budgets cover a local solo server in a
browser wasm module (D41) and the M7 integrated workload on both targets.

"Script cost" is the wall time of the script phase of one tick: build the event input, call
the script, run its host calls, and collect its requests. The value is a p99 over all measured
ticks of all runs.

| Id | Budget | Native | Chrome wasm | Reason |
| --- | --- | --- | --- | --- |
| B1 | W1 script cost p99 | ≤ 500 µs | ≤ 2,000 µs | 1 % of a tick on a shared server; 4 % in a browser that also renders |
| B2 | W2 script cost p99 | ≤ 5,000 µs | ≤ 15,000 µs | A rare burst must still fit in the tick |
| B3 | Host-call cost (W3 script cost / 1,000), median | ≤ 1.0 µs | ≤ 3.0 µs | Batched queries must stay cheaper than the work they replace |
| B4 | Added download, gzip (`gzip -9`) of the wasm after `wasm-opt` | ≤ 1,048,576 bytes | (same) | 7 % of the current client transfer |
| B5 | Added download, raw wasm after `wasm-opt` | ≤ 4,194,304 bytes | (same) | Decoded size bounds parse and compile time |
| B6 | Added peak heap for one runtime instance (W1 then W2) | ≤ 8 MiB | ≤ 8 MiB | Many matches share one 8-GB server (13.2) |
| B7 | Added wasm linear memory after W1 and W2 | n/a | ≤ 16 MiB | Wasm memory never shrinks |
| B8 | L1 to L4 and L6 to L9 | all pass | all pass | Spec 5.4, 6.5: bounded execution, no ambient capability |
| B9 | L1 wall time from start to the budget error | ≤ 50 ms | ≤ 100 ms | One tick native; two ticks in the browser |
| B10 | L5 (a): equal over runs, both targets and `rust_ref`. L5 (b): equal over the runs of one target | equal | equal | G9 (6.2, 6.11) needs one result for each build and each run. Equal W1 and W2 results across targets show that the integer rules give the same requests in both builds; this costs nothing, because the rules do not iterate a hash map. Spec 6.2 does not require cross-platform determinism, so L5 (b) across targets is recorded only |
| B11 | Build and license | builds for both targets; every transitive license is permissive | (same) | Spec 13.1, 13.9 |

The download figures use the baseline wasm (same harness, no script runtime) as the zero. The
memory figures use the same harness with `rust_ref` as the zero. B4 and B5 are measured once
because the file is the same for every browser.

A missing number is a fail. A probe that cannot run on a target is a fail on that target.

### 4.1 Selection rule (fixed before the runs)

1. A candidate is eligible only when every row B1 to B11 passes on both targets.
2. For each eligible candidate, compute the margin of each numeric row:
   `(budget - measured) / budget`. Its score is its smallest margin.
3. Select the eligible candidate with the highest score. When two scores are within 0.10 of
   each other, apply these tie-breaks in this order, and stop at the first one that separates
   them:
   1. **Per-instance accounting in the runtime:** the runtime itself reports the live bytes of
      one instance across calls, so that a server can cap each match when many matches share
      a thread. From the crate sources: piccolo yes (`Lua::total_memory`); Rhai no (per
      value); Rune no (per call).
   2. **Modder familiarity**, fixed order: Lua (the language of many RTS and game mods), then
      Rhai (dynamic, JavaScript-like syntax), then Rune (Rust-like syntax). The research note
      records the size of the standard library of each candidate; piccolo has a small one.
   3. **Crate maturity:** version 1.0 or later first.
4. When no candidate is eligible, MS is red. The report states the failing rows and the
   candidate that is nearest to a pass.

## 5. Measurement protocol

- Release builds. Native: `opt-level 3`, fat LTO, one codegen unit. Wasm: the root
  `wasm-release` settings (`opt-level 3`, fat LTO, one codegen unit, `panic = "abort"`), then
  `wasm-opt -O3`. No `wasm-bindgen`: the `cdylib` exports plain functions, so the wasm import
  list is evidence for L4.
- Timing: the tick has three exported phases, called in the same order on both targets:
  `sim` (Rust movement, grid, events), `script` (the script phase only), `apply` (request
  validation and application). Native: `std::time::Instant` around each phase. Chrome:
  `performance.now()` around each exported call in a cross-origin-isolated page (COOP and COEP
  headers). The report states the measured timer resolution. B1 to B3 use the `script` phase
  only.
- Each workload: 5 runs (5 processes; 5 fresh Chrome launches), 200 warmup ticks, then 2,000
  measured ticks. W4 (recorded only): 50 warmup and 500 measured ticks. The report gives p50,
  p90, p99, max and the sample count.
- Memory: a counting global allocator in the harness (live bytes, peak) on both targets.
  Chrome also reports `memory.buffer.byteLength`.
- CPU quiet: final timing and memory runs hold an exclusive `flock` on
  `/tmp/turts-quiet-performance.lock`; builds hold a shared lock. The report states the load
  average before each run. A run without the exclusive lock is not acceptance evidence.
- Chrome: `/usr/bin/google-chrome` headless, a fresh profile for each launch,
  `--alsa-output-device=null`. The report states the Chrome version.
- The report records the hardware, the OS, `rustc`, the crate versions and the commands.

## 6. What is mocked and what stays open

Open after MS, recorded in the report: the thread model of the production runtime (C1 is
built without `sync`; whether a piccolo `Lua` is `Send` is not checked); per-instance
accounting when many matches share one thread with interleaved calls.


Mocked: the world (a unit array and a grid), the events (seeded, not real combat), the requests
(applied by simple Rust code), and the package path (scripts are embedded with
`include_str!`, not loaded from a mod archive). Bevy, lightyear, the mod runtime crates and the
lockfile are not involved. One thread only.

## 7. Outputs

- `experiments/ms-scripting/` (code, scripts, commands in its `README.md`).
- `docs/research/scripting-selection.md` (sources, screening).
- `docs/measurements/scripting.md` (all numbers, method, hardware).
- `docs/reports/2026-11-16-scripting-selection.md` (decision, budgets table with results,
  evidence paths, unchecked items).
- Raw evidence: `/home/julien/Projects/TuRTS/target/mc-evidence/ms-scripting/`.
- `README.md` progress, only when the MS row passes.

## 8. Acceptance

MS passes when section 7 exists, every candidate has a number for every row on both targets,
one candidate is selected by rule 4.1, and one independent review that runs the spike code is
complete with its material findings fixed.

## 9. Amendments after the first runs (instrument defects only; no budget changed)

1. **Host accounting of request payloads (spec 3.5).** The first full run counted the request
   payloads of a call (for example the id list of `move_group`) as retained bytes of the
   instance, but the host frees them in `apply`, outside the call. `rust_ref` showed
   30,416,976 retained bytes. The host now subtracts the growth of its own request buffer from
   the delta of each call. B6 and all probes were run again.
2. **L9a (spec 3.4).** With 100,000 requests, the instruction budget stopped the call before
   the request cap could act, so the red check `--mutant no-request-cap` stayed green. L9a now
   sends 4,200 requests (over the cap of 4,096, under every instruction budget; 5,000 was over
   the piccolo fuel budget, about 26 fuel for each iteration) and passes only when the
   request-cap error stopped the call.
