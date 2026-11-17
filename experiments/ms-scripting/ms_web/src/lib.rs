//! Wasm32 driver for the MS spike: plain exports, no wasm-bindgen. The page in `web/` calls
//! these in the same order as `ms_native` and times each call with `performance.now()`.

use std::cell::RefCell;

use ms_core::bench::Workload;
use ms_core::cand::Kind;
use ms_core::{host, probes, Bench};

#[global_allocator]
static ALLOC: ms_core::alloc::Counting = ms_core::alloc::Counting;

thread_local! {
    static BENCH: RefCell<Option<Bench>> = const { RefCell::new(None) };
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn put(s: String) -> u32 {
    OUT.with(|o| {
        let mut o = o.borrow_mut();
        o.clear();
        o.extend_from_slice(s.as_bytes());
        o.len() as u32
    })
}

fn kind(code: u32) -> Kind {
    Kind::from_code(code).unwrap_or(Kind::None)
}

fn with_bench(f: impl FnOnce(&mut Bench)) {
    BENCH.with(|b| {
        if let Some(b) = b.borrow_mut().as_mut() {
            f(b)
        }
    })
}

/// Pointer to the last JSON result; its length is the return value of the call that made it.
#[no_mangle]
pub extern "C" fn ms_out_ptr() -> *const u8 {
    OUT.with(|o| o.borrow().as_ptr())
}

/// JSON list of the candidates in this build.
#[no_mangle]
pub extern "C" fn ms_info() -> u32 {
    let c: Vec<String> = ms_core::compiled_candidates()
        .iter()
        .map(|s| ms_core::json_str(s))
        .collect();
    put(format!("[{}]", c.join(",")))
}

/// Create the bench (worlds, then runtime). Returns 0 on success; the error is in the output.
#[no_mangle]
pub extern "C" fn ms_bench_new(k: u32) -> u32 {
    match Bench::new(kind(k), None) {
        Ok(b) => {
            BENCH.with(|c| *c.borrow_mut() = Some(b));
            put(String::new());
            0
        }
        Err(e) => put(format!("{}: {}", e.kind.name(), e.message)).max(1),
    }
}

#[no_mangle]
pub extern "C" fn ms_select(w: u32) {
    let w = Workload::from_code(w).unwrap_or(Workload::W1);
    with_bench(|b| b.select(w))
}

#[no_mangle]
pub extern "C" fn ms_sim() {
    with_bench(|b| b.sim())
}

#[no_mangle]
pub extern "C" fn ms_script(w: u32) {
    let w = Workload::from_code(w).unwrap_or(Workload::W1);
    with_bench(|b| b.script(w))
}

#[no_mangle]
pub extern "C" fn ms_apply() {
    with_bench(|b| b.apply())
}

#[no_mangle]
pub extern "C" fn ms_instance_peak() -> f64 {
    let mut v = 0.0;
    with_bench(|b| v = b.instance_peak() as f64);
    v
}

/// JSON fields of the suite record (hashes, errors, memory).
#[no_mangle]
pub extern "C" fn ms_report() -> u32 {
    let mut s = String::new();
    with_bench(|b| s = b.report_fields());
    put(s)
}

#[no_mangle]
pub extern "C" fn ms_live() -> f64 {
    ms_core::alloc::live() as f64
}

#[no_mangle]
pub extern "C" fn ms_set_mutant(m: u32) {
    host::set_mutant(m as u8)
}

/// Probe `RUN_ORDER[i]` (not L1). Returns the JSON length.
#[no_mangle]
pub extern "C" fn ms_probe(k: u32, i: u32) -> u32 {
    let id = probes::RUN_ORDER.get(i as usize).copied().unwrap_or("?");
    put(probes::run(kind(k), id))
}

#[no_mangle]
pub extern "C" fn ms_l1_prepare(k: u32) -> u32 {
    put(probes::l1_prepare(kind(k)))
}

#[no_mangle]
pub extern "C" fn ms_l1_call() {
    probes::l1_call()
}

#[no_mangle]
pub extern "C" fn ms_l1_finish(k: u32) -> u32 {
    put(probes::l1_finish(kind(k)))
}
