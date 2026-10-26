# M2 scripting probes (2026-10-26)

Probes live in the session scratchpad (`scratchpad/m2/script/ws`, `results.txt`); not in the repo.
Measured (wasm-release + wasm-opt -Oz, node v24 V8, native rustc 1.99.1):
- baseline no-runtime wasm 53,072 B; Rust behaviour 10.5 ns/call native.
- rhai 1.27.0 (MIT/Apache): +1,028,518 B; 224-394 ns native, 800-840 ns node; `set_max_operations` stops a runaway loop ("Too many operations").
- piccolo 0.3.3 (MIT, last crates.io release 2024-06-16): +372,164 B; 349-393 ns native, 670-710 ns node; `Fuel` stops the loop. Needs getrandom 0.2 `js` and 0.3 `wasm_js` cfg on wasm32.
- wasmi 2.0.0 (MIT/Apache): +990,072 B; 92-150 ns native, 190-230 ns node; `consume_fuel` stops the loop. Conflicts with spec 5.3 deferral of wasm modules.
- mlua 0.12.2 fails on wasm32-unknown-unknown: "don't know how to build Lua for wasm32-unknown-unknown".
Red check: PROBE_NO_BUDGET=1 makes all three runaway loops hang (timeout exit 124).
Recommendation given: M2 without scripting (5.1 layers 1-2 cover both economies); Rhai stays the candidate.
