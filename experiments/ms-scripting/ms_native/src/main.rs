//! Native driver for the MS spike. See `experiments/ms-scripting/README.md` for the commands.

use std::fmt::Write as _;
use std::time::Instant;

use ms_core::bench::Workload;
use ms_core::cand::{Kind, Limits};
use ms_core::{host, probes, Bench};

#[global_allocator]
static ALLOC: ms_core::alloc::Counting = ms_core::alloc::Counting;

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn kind_of(args: &[String]) -> Kind {
    let k = arg(args, "--kind").unwrap_or_else(|| die("--kind is required"));
    Kind::parse(&k).unwrap_or_else(|| die(&format!("unknown kind {k}")))
}

fn die(msg: &str) -> ! {
    eprintln!("ms_native: {msg}");
    std::process::exit(2)
}

fn write_out(args: &[String], text: &str) {
    match arg(args, "--out") {
        Some(p) => std::fs::write(&p, text).unwrap_or_else(|e| die(&format!("write {p}: {e}"))),
        None => eprintln!("{text}"),
    }
}

fn set_mutant(args: &[String]) -> &'static str {
    match arg(args, "--mutant").as_deref() {
        None | Some("none") => "none",
        Some("apply-now") => {
            host::set_mutant(host::MUTANT_APPLY_NOW);
            "apply-now"
        }
        Some("no-request-cap") => {
            host::set_mutant(host::MUTANT_NO_REQUEST_CAP);
            "no-request-cap"
        }
        Some("no-accounting") => {
            host::set_mutant(host::MUTANT_NO_ACCOUNTING);
            "no-accounting"
        }
        Some(m) => die(&format!("unknown mutant {m}")),
    }
}

fn samples(v: &[f64]) -> String {
    let mut s = String::with_capacity(v.len() * 8);
    s.push('[');
    for (i, x) in v.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let _ = write!(s, "{x:.3}");
    }
    s.push(']');
    s
}

fn us(t: Instant) -> f64 {
    t.elapsed().as_nanos() as f64 / 1_000.0
}

fn suite(args: &[String]) {
    let kind = kind_of(args);
    let mutant = set_mutant(args);
    let run = arg(args, "--run").unwrap_or_else(|| "0".into());
    let live0 = ms_core::alloc::live();
    let t = Instant::now();
    let mut b = Bench::new(kind, None)
        .unwrap_or_else(|e| die(&format!("create: {} (line {:?})", e.message, e.line)));
    let create_us = us(t);
    let mut parts = Vec::new();
    let mut peak_after_w2 = 0;
    for w in Workload::ALL {
        b.select(w);
        let (warm, meas) = (w.warmup(), w.measured());
        let mut sim = Vec::with_capacity(meas as usize);
        let mut scr = Vec::with_capacity(meas as usize);
        let mut app = Vec::with_capacity(meas as usize);
        for i in 0..warm + meas {
            let t0 = Instant::now();
            b.sim();
            let d_sim = us(t0);
            let t1 = Instant::now();
            b.script(w);
            let d_scr = us(t1);
            let t2 = Instant::now();
            b.apply();
            let d_app = us(t2);
            if i >= warm {
                sim.push(d_sim);
                scr.push(d_scr);
                app.push(d_app);
            }
        }
        if w == Workload::W2 {
            peak_after_w2 = b.instance_peak();
        }
        parts.push(format!(
            "{}:{{\"warmup\":{warm},\"sim_us\":{},\"script_us\":{},\"apply_us\":{}}}",
            ms_core::json_str(w.name()),
            samples(&sim),
            samples(&scr),
            samples(&app)
        ));
    }
    let out = format!(
        "{{\"target\":\"native\",\"mutant\":\"{mutant}\",\"run\":{run},\"create_us\":{create_us:.3},\"instance_peak_w1w2_bytes\":{peak_after_w2},\"process_live_before_bytes\":{live0},{},\"workloads\":{{{}}}}}",
        b.report_fields(),
        parts.join(",")
    );
    write_out(args, &out);
}

fn calibrate(args: &[String]) {
    let kind = kind_of(args);
    let mut b = Bench::new(kind, Some(Limits::unlimited()))
        .unwrap_or_else(|e| die(&format!("create: {} (line {:?})", e.message, e.line)));
    let mut fields = Vec::new();
    let mut overall = 0u64;
    for w in [Workload::W1, Workload::W2, Workload::W3] {
        b.select(w);
        let mut max = 0u64;
        for _ in 0..2_200 {
            b.sim();
            b.script(w);
            b.apply();
            max = max.max(b.last_units().unwrap_or(0));
        }
        overall = overall.max(max);
        fields.push(format!("\"{}\":{max}", w.name()));
    }
    let budget = round_up_2sig(overall * 4);
    let errors = b.errors;
    let h1 = b.hash(Workload::W1);
    write_out(
        args,
        &format!(
            "{{\"kind\":{},\"max_units\":{{{}}},\"max_overall\":{overall},\"budget\":{budget},\"errors\":{},\"hash_w1\":\"{:016x}\"}}",
            ms_core::json_str(kind.name()),
            fields.join(","),
            errors,
            h1
        ),
    );
}

/// Round up to two significant digits.
fn round_up_2sig(v: u64) -> u64 {
    if v < 100 {
        return v;
    }
    let mut p = 1u64;
    while v / p >= 100 {
        p *= 10;
    }
    v.div_ceil(p) * p
}

fn probe(args: &[String]) {
    let kind = kind_of(args);
    let mutant = set_mutant(args);
    let id = arg(args, "--id").unwrap_or_else(|| die("--id is required"));
    let rec = if id == "L1" {
        let prep = probes::l1_prepare(kind);
        let t = Instant::now();
        probes::l1_call();
        let stop_ms = t.elapsed().as_secs_f64() * 1_000.0;
        let fin = probes::l1_finish(kind);
        format!(
            "{{\"target\":\"native\",\"mutant\":\"{mutant}\",\"stop_ms\":{stop_ms:.3},\"prepare\":{prep},\"result\":{fin}}}"
        )
    } else {
        format!(
            "{{\"target\":\"native\",\"mutant\":\"{mutant}\",\"result\":{}}}",
            probes::run(kind, &id)
        )
    };
    write_out(args, &rec);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("suite") => suite(&args),
        Some("calibrate") => calibrate(&args),
        Some("probe") => probe(&args),
        Some("info") => println!("{:?}", ms_core::compiled_candidates()),
        _ => die("usage: ms_native suite|calibrate|probe|info --kind K [--run N] [--id ID] [--mutant M] [--out P]"),
    }
}
