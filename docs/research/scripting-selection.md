# Scripting candidates: sources and screening (MS)

Date: 2026-11-16. Spec: `docs/superpowers/specs/2026-11-16-ms-scripting-selection.md`.
Source of each claim: the crate source as published on crates.io, at the exact version, in
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` (written `REG/` below), the crate
metadata (`cargo info`), or the project documentation named in the row. Claims that come from
reading only are marked "read"; claims that a spike run checked are marked "run".

## Requirements used for the screen

From spec 5.3, 5.4, 13.1 and 13.9 and the MS spec section 2: embeddable in Rust; builds for
`wasm32-unknown-unknown` with no Emscripten, no WASI and no threads; an instruction budget (a
deterministic bound, not a wall clock); controlled host capabilities; a permissive license.

## Candidates measured

| | Rhai | Rune | Lua (piccolo) |
| --- | --- | --- | --- |
| Crate and version | `rhai 1.27.0` | `rune 0.14.2` | `piccolo 0.3.3` |
| Repository | github.com/rhaiscript/rhai | github.com/rune-rs/rune | github.com/kyren/piccolo |
| Crate license | MIT OR Apache-2.0 | MIT OR Apache-2.0 | MIT |
| Transitive licenses (run, `tools/licenses.py`) | one MPL-2.0+ dependency: `smartstring 1.0.1`, required (`REG/rhai-1.27.0/Cargo.toml:269-271`, not optional) | all permissive (49 packages) | all permissive (28 packages) |
| Language | dynamic, JavaScript-like syntax | dynamic, Rust-like syntax | Lua 5.x dialect |
| Execution model | AST walker; Rust recursion for script calls | bytecode VM; frames on the heap | bytecode VM, stackless; frames on the heap |
| Instruction budget | `Engine::set_max_operations` (`api/limits.rs:173`), per call | `runtime::budget::with` (`runtime/budget.rs:86`), thread-local | `Executor::step` with `Fuel` (`thread/executor.rs:147`, `fuel.rs:9`); about one VM instruction for each unit (`fuel.rs:1-4`) |
| Call depth | `set_max_call_levels` (`api/limits.rs:155`) | none; bounded by budget and memory (run: L2 stops by budget) | none; bounded by budget and memory (run: L2 stops by budget) |
| Memory controls | per value: string, array and map sizes (`eval/data_check.rs`); map keys not counted (`data_check.rs:64`); no total | `rune_alloc::limit::with` (`rune-alloc/src/limit/mod.rs:103`), a per-call cap on rune-alloc allocations | none; `Lua::total_memory` counts the GC bytes of one instance (`lua.rs:151`) |
| Progress hook | `on_progress` (`api/events.rs:244`) | none | the host steps the executor |
| Ambient capabilities in the configuration used | `Engine::new_raw` + `StandardPackage`; `no_time`, `no_module`; `print`, `debug`, `eval` disabled with `disable_symbol` | `Context::new` with selected modules; no `std::io`, no `std::test`, no futures, no streams | `base`, `coroutine`, `string`, `table` loaded; no `io` (print and files), no `math` (it seeds `SmallRng::from_entropy`, `stdlib/math.rs:37`) |
| Hash-map order | `BTreeMap`: deterministic | rune-alloc hashbrown (see L5 results) | ahash `RandomState` with `runtime-rng` (`table/raw.rs:64`); fixed seeds from `getrandom::fill` (`ahash-0.8.12/src/random_state.rs:68-79`) |
| Standard library size | large (strings, arrays, maps, math, iterators) | large (strings, vectors, objects, iterators, formatting) | small: `string` has `len`, `sub`, `lower`, `reverse`, `upper`; `table` has `pack`, `unpack` |
| wasm32-unknown-unknown (run) | builds; no imports after `wasm-opt` | builds; no imports | builds with the getrandom custom backends (spec 2.1); no imports |
| Maturity | 1.x | 0.x | 0.x; the README describes it as experimental |

## Screened out before the runs

| Candidate | Reason | Source |
| --- | --- | --- |
| `mlua` (Lua 5.1 to 5.5, LuaJIT, Luau) | C or C++ sources. Its documentation supports `wasm32-unknown-unknown` only "with a WASI C runtime", and "the host must provide the matching WASI imports". The no-WASI rule excludes it. | mlua README and `docs/wasm32.md` (fetched by the gate-1 reviewer, evidence R12) |
| `boa_engine 0.22.0` (JavaScript) | `RuntimeLimits` has `stack_size`, `loop_iteration`, `backtrace_limit` and `recursion`. It has no instruction budget, so a script without a loop is not bounded. | `REG/boa_engine-0.22.0/src/vm/runtime_limits.rs:3-15` |
| `wasmi` (WebAssembly modules) | Spec 5.3 defers independently executable WASM modules. | spec 5.3 |
| `koto` | `KotoVmSettings::execution_limit` is an `Option<Duration>`: a wall-clock limit, not deterministic. | docs.rs `koto_runtime` 0.16.1 (gate-1 reviewer, evidence R11) |

## Notes for the production runtime (not checked by MS)

- Thread model: C1 was built without the Rhai `sync` feature. Whether a piccolo `Lua` is
  `Send` was not checked.
- None of the three runtimes has a total-heap cap for one instance. MS enforces the cap in
  the host with call-delta accounting on a counting allocator (spec 3.5). That method needs
  one instance at a time on a thread.
