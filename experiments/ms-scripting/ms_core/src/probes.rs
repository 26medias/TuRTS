//! Limit probes L1 to L9 (spec 3.4). Each probe makes a fresh instance under the calibrated
//! limits of its kind and returns one JSON record. The drivers run each probe in a fresh
//! process (native) or a fresh wasm instance (Chrome), so a crash or a trap is a fail that the
//! driver records, not a lost run.

use crate::alloc;
use crate::cand::{Guarded, Kind, Limits, ScriptError, ScriptErrorKind};
use crate::host;
use crate::json_str;
use crate::world::{EventRate, Request, World};

pub const IDS: [&str; 19] = [
    "L1",
    "L2",
    "L3a",
    "L3b",
    "L3c-map",
    "L3c-list",
    "L3d",
    "L3e",
    "L4",
    "L5b",
    "L7a",
    "L7b",
    "L8",
    "L9a",
    "L9b",
    "L1-prepare",
    "L1-call",
    "L1-finish",
    "L6",
];

/// Probes that the drivers run, in order. (`L1` runs as prepare, call, finish; `L6` is the
/// recovery field of L1, L2, L3x, L9a.)
pub const RUN_ORDER: [&str; 15] = [
    "L1", "L2", "L3a", "L3b", "L3c-map", "L3c-list", "L3d", "L3e", "L4", "L5b", "L7a", "L7b", "L8",
    "L9a", "L9b",
];

const CAP_TOLERANCE: usize = 1024 * 1024;

/// Wrap an `on_tick` body into a full script with an empty state.
fn script(kind: Kind, prelude: &str, body: &str) -> String {
    match kind {
        Kind::Rhai => format!("fn init() {{ #{{}} }}\n{prelude}\nfn on_tick(ev) {{\n{body}\n}}\n"),
        Kind::Rune => {
            format!("pub fn init() {{ #{{}} }}\n{prelude}\npub fn on_tick(s, ev) {{\n{body}\n}}\n")
        }
        Kind::Lua => format!("{prelude}\nfunction on_tick(ev)\n{body}\nend\n"),
        _ => String::new(),
    }
}

/// The probe sources: (variant name, full source).
pub fn sources(kind: Kind, id: &str) -> Vec<(String, String)> {
    let one = |p: &str, b: &str| vec![(id.to_string(), script(kind, p, b))];
    match (kind, id) {
        // L1: runaway loop.
        (Kind::Rhai, "L1") => one("", "loop { }"),
        (Kind::Rune, "L1") => one("", "loop { }"),
        (Kind::Lua, "L1") => one("", "while true do end"),
        // L2: unbounded recursion.
        (Kind::Rhai, "L2") => one("fn f(n) { f(n + 1) + 1 }", "f(0)"),
        (Kind::Rune, "L2") => one("fn f(n) { f(n + 1) + 1 }", "f(0)"),
        (Kind::Lua, "L2") => one("local function f(n) return f(n + 1) + 1 end", "return f(0)"),
        // L3a: a string that doubles.
        (Kind::Rhai, "L3a") => one("", "let s = \"x\"; loop { s += s; }"),
        (Kind::Rune, "L3a") => one("", "let s = \"x\"; loop { s = s + s; }"),
        (Kind::Lua, "L3a") => one("", "local s = 'x' while true do s = s .. s end"),
        // L3b: a list that doubles.
        (Kind::Rhai, "L3b") => one("", "let a = [1]; loop { a += a; }"),
        (Kind::Rune, "L3b") => one("", "let a = [1]; loop { let b = a.clone(); a.extend(b); }"),
        (Kind::Lua, "L3b") => one(
            "",
            "local a = {1} while true do local n = #a for i = 1, n do a[n + i] = a[i] end end",
        ),
        // L3c: many values, each below any per-value limit: 512 KiB string keys.
        (Kind::Rhai, "L3c-map") => one(
            "",
            "let s = \"x\"; while s.len() < 524288 { s += s; } let m = #{}; let i = 0; loop { m[s + i] = i; i += 1; }",
        ),
        (Kind::Rune, "L3c-map") => one(
            "",
            "let s = \"x\"; while s.len() < 524288 { s = s + s; } let m = #{}; let i = 0; loop { m[`${s}${i}`] = i; i += 1; }",
        ),
        (Kind::Lua, "L3c-map") => one(
            "",
            "local s = 'x' while #s < 524288 do s = s .. s end local m = {} local i = 0 while true do m[s .. i] = i i = i + 1 end",
        ),
        (Kind::Rhai, "L3c-list") => one(
            "",
            "let s = \"x\"; while s.len() < 524288 { s += s; } let l = []; let i = 0; loop { l.push(s + i); i += 1; }",
        ),
        (Kind::Rune, "L3c-list") => one(
            "",
            "let s = \"x\"; while s.len() < 524288 { s = s + s; } let l = []; let i = 0; loop { l.push(`${s}${i}`); i += 1; }",
        ),
        (Kind::Lua, "L3c-list") => one(
            "",
            "local s = 'x' while #s < 524288 do s = s .. s end local l = {} local i = 0 while true do l[#l + 1] = s .. i i = i + 1 end",
        ),
        // L3d: kept state that grows by 64 KiB on each tick.
        (Kind::Rhai, "L3d") => vec![(
            id.to_string(),
            "fn init() { #{ keep: [] } }\nfn on_tick(ev) { let s = \"x\"; while s.len() < 65536 { s += s; } this.keep.push(s + this.keep.len()); }\n".to_string(),
        )],
        (Kind::Rune, "L3d") => vec![(
            id.to_string(),
            "pub fn init() { #{ keep: [] } }\npub fn on_tick(st, ev) { let s = \"x\"; while s.len() < 65536 { s = s + s; } let n = st.keep.len(); st.keep.push(`${s}${n}`); }\n".to_string(),
        )],
        (Kind::Lua, "L3d") => vec![(
            id.to_string(),
            "local keep = {}\nfunction on_tick(ev) local s = 'x' while #s < 65536 do s = s .. s end keep[#keep + 1] = s .. #keep end\n".to_string(),
        )],
        // L3e: one built-in call that asks for about 1 GiB.
        (Kind::Rhai, "L3e") => one("", "let a = []; a.pad(67108864, 0);"),
        (Kind::Rune, "L3e") => one("", "let v = []; v.resize(67108864, 0);"),
        (Kind::Lua, "L3e") => one("", "local t = {} table.unpack(t, 1, 67108864)"),
        // L4: ambient access. Each variant must fail to compile or fail when called.
        (Kind::Rhai, "L4") => [
            ("time", "timestamp()"),
            ("print", "print(\"ambient\")"),
            ("debug", "debug(\"ambient\")"),
            ("eval", "eval(\"1 + 1\")"),
            ("import", "import \"x\" as y;"),
            ("file", "open_file(\"/etc/hostname\")"),
            ("random", "rand()"),
            ("net", "http_get(\"http://127.0.0.1/\")"),
        ]
        .iter()
        .map(|(n, b)| (n.to_string(), script(kind, "", b)))
        .collect(),
        (Kind::Rune, "L4") => [
            ("time", "std::time::Instant::now()"),
            ("print", "std::io::println(\"ambient\")"),
            ("print-macro", "println!(\"ambient\")"),
            ("debug", "dbg!(1)"),
            ("file", "std::fs::read_to_string(\"/etc/hostname\")"),
            ("random", "rand::random()"),
            ("process", "std::process::exit(0)"),
            ("net", "std::net::TcpStream::connect(\"127.0.0.1:80\")"),
        ]
        .iter()
        .map(|(n, b)| (n.to_string(), script(kind, "", b)))
        .collect(),
        (Kind::Lua, "L4") => [
            ("time", "return os.time()"),
            ("clock", "return os.clock()"),
            ("print", "print('ambient')"),
            ("io", "io.write('ambient')"),
            ("random", "return math.random()"),
            ("load", "return load('return 1')()"),
            ("loadstring", "return loadstring('return 1')()"),
            ("require", "return require('x')"),
            ("dofile", "return dofile('/etc/hostname')"),
        ]
        .iter()
        .map(|(n, b)| (n.to_string(), script(kind, "", b)))
        .collect(),
        // L5b: map iteration order with 64 string keys.
        (Kind::Rhai, "L5b") => one(
            "",
            "let m = #{}; for i in 0..64 { let j = (i * 37) % 64; m[\"k\" + j] = j; } for k in m.keys() { alert(0, m[k]); }",
        ),
        (Kind::Rune, "L5b") => one(
            "",
            "let m = #{}; for i in 0..64 { let j = (i * 37) % 64; m[`k${j}`] = j; } for (k, v) in m { host::alert(0, v); }",
        ),
        (Kind::Lua, "L5b") => one(
            "",
            "local m = {} for i = 0, 63 do local j = (i * 37) % 64 m['k' .. j] = j end for k, v in pairs(m) do alert(0, v) end",
        ),
        // L7a: a syntax error on line 3. L7b: a runtime type error on line 4.
        (Kind::Rhai, "L7a") => vec![(id.to_string(), "fn init() { #{} }\n// line 2\nlet = = ;\n".to_string())],
        (Kind::Rune, "L7a") => vec![(id.to_string(), "pub fn init() { #{} }\n// line 2\nlet = = ;\n".to_string())],
        (Kind::Lua, "L7a") => vec![(id.to_string(), "-- line 1\n-- line 2\nlocal = = \n".to_string())],
        (Kind::Rhai, "L7b") => vec![(
            id.to_string(),
            "fn init() { #{} }\n// line 2\nfn on_tick(ev) {\n    let x = 1 + [];\n}\n".to_string(),
        )],
        (Kind::Rune, "L7b") => vec![(
            id.to_string(),
            "pub fn init() { #{} }\n// line 2\npub fn on_tick(s, ev) {\n    let x = 1 + \"a\";\n}\n".to_string(),
        )],
        (Kind::Lua, "L7b") => vec![(
            id.to_string(),
            "-- line 1\n-- line 2\nfunction on_tick(ev)\n  local x = 1 + {}\nend\n".to_string(),
        )],
        // L8: requests are deferred.
        (Kind::Rhai, "L8") => vec![(
            id.to_string(),
            "fn init() { #{ a: 0, n: 0 } }\nfn on_tick(ev) { if this.n == 0 { this.a = credits(0); grant(0, 100); let b = credits(0); alert(0, if this.a == b { 1 } else { 0 }); } else { alert(0, credits(0) - this.a); } this.n += 1; }\n".to_string(),
        )],
        (Kind::Rune, "L8") => vec![(
            id.to_string(),
            "pub fn init() { #{ a: 0, n: 0 } }\npub fn on_tick(s, ev) { if s.n == 0 { s.a = host::credits(0); host::grant(0, 100); let b = host::credits(0); host::alert(0, if s.a == b { 1 } else { 0 }); } else { host::alert(0, host::credits(0) - s.a); } s.n += 1; }\n".to_string(),
        )],
        (Kind::Lua, "L8") => vec![(
            id.to_string(),
            "local a, n = 0, 0\nfunction on_tick(ev) if n == 0 then a = credits(0) grant(0, 100) local b = credits(0) if a == b then alert(0, 1) else alert(0, 0) end else alert(0, credits(0) - a) end n = n + 1 end\n".to_string(),
        )],
        // L9a: request flood (4,200: over the cap of 4,096, under every instruction budget).
        // L9b: one group of 1,000 ids.
        (Kind::Rhai, "L9a") => one("", "let i = 0; while i < 4200 { alert(0, 1); i += 1; }"),
        (Kind::Rune, "L9a") => one("", "let i = 0; while i < 4200 { host::alert(0, 1); i = i + 1; }"),
        (Kind::Lua, "L9a") => one("", "local i = 0 while i < 4200 do alert(0, 1) i = i + 1 end"),
        (Kind::Rhai, "L9b") => one("", "let a = []; let i = 0; while i < 1000 { a.push(i); i += 1; } move_group(a, 1000, 1000);"),
        (Kind::Rune, "L9b") => one("", "let a = []; let i = 0; while i < 1000 { a.push(i); i += 1; } host::move_group(a, 1000, 1000);"),
        (Kind::Lua, "L9b") => one("", "local a = {} for i = 0, 999 do a[#a + 1] = i end move_group(a, 1000, 1000)"),
        _ => Vec::new(),
    }
}

fn probe_world() -> World {
    World::new(1_000, 0x5EED_00F0, EventRate { min: 12, spread: 9 })
}

fn err_json(e: &Option<ScriptError>) -> String {
    match e {
        None => "null".to_string(),
        Some(e) => format!(
            "{{\"kind\":{},\"message\":{},\"line\":{}}}",
            json_str(e.kind.name()),
            json_str(&e.message),
            e.line.map_or("null".to_string(), |l| l.to_string())
        ),
    }
}

/// One tick for a probe: sim, script, apply. Returns the script result.
fn tick(g: &mut Guarded) -> Result<(), ScriptError> {
    host::with(|h| h.world.as_mut().expect("world").sim_step());
    let events = host::with(|h| std::mem::take(&mut h.world.as_mut().expect("world").events));
    let r = g.on_tick(&events);
    host::with(|h| {
        let w = h.world.as_mut().expect("world");
        w.events = events;
        let mut req = std::mem::take(&mut h.requests);
        w.apply(&mut req);
        h.requests = req;
    });
    r
}

/// Recovery check (L6): the next call returns "script disabled" with the script name, or Ok.
fn recovery(g: &mut Guarded) -> (bool, String) {
    match tick(g) {
        Ok(()) => (true, "next call ran".to_string()),
        Err(e) if e.kind == ScriptErrorKind::Disabled && e.message.contains(&g.name) => {
            (true, format!("disabled: {}", e.message))
        }
        Err(e) => (false, format!("next call: {} {}", e.kind.name(), e.message)),
    }
}

fn record(kind: Kind, id: &str, pass: bool, fields: &[(&str, String)]) -> String {
    let mut s = format!(
        "{{\"probe\":{},\"kind\":{},\"pass\":{}",
        json_str(id),
        json_str(kind.name()),
        pass
    );
    for (k, v) in fields {
        s.push_str(&format!(",{}:{}", json_str(k), v));
    }
    s.push('}');
    s
}

fn create(kind: Kind, name: &str, src: &str) -> Result<Guarded, ScriptError> {
    Guarded::create(kind, name, src, Limits::for_kind(kind))
}

/// A probe that must end with an error under the limits (L2, L3x, L9a).
fn bounded(kind: Kind, id: &str, max_ticks: u32, memory: bool) -> String {
    host::install_world(probe_world());
    let (name, src) = sources(kind, id).remove(0);
    let base_live = alloc::live();
    alloc::reset_peak();
    let mut g = match create(kind, &name, &src) {
        Ok(g) => g,
        Err(e) => {
            return record(
                kind,
                id,
                false,
                &[
                    ("detail", json_str("create failed")),
                    ("error", err_json(&Some(e))),
                ],
            );
        }
    };
    let mut err = None;
    let mut ticks = 0;
    // `Guarded::run` resets the allocator peak for each call, so take the maximum over ticks.
    let mut process_peak = alloc::peak().saturating_sub(base_live);
    while ticks < max_ticks {
        ticks += 1;
        let r = tick(&mut g);
        process_peak = process_peak.max(alloc::peak().saturating_sub(base_live));
        if let Err(e) = r {
            err = Some(e);
            break;
        }
    }
    let cap = Limits::MEMORY + CAP_TOLERANCE;
    let requests_left = host::with(|h| h.requests.len());
    let flagged = host::over_request_cap();
    let (rec_ok, rec) = recovery(&mut g);
    let mut pass = err.is_some() && rec_ok;
    if memory {
        pass &= g.peak <= cap && process_peak <= cap;
    }
    if id == "L9a" {
        // The cap must be what stopped the call: 5,000 requests fit every instruction budget.
        pass &= requests_left == 0
            && flagged
            && err
                .as_ref()
                .is_some_and(|e| e.message.contains("requests in one call"));
    }
    let applied = host::with(|h| h.world.as_ref().map_or(0, |w| w.applied));
    record(
        kind,
        id,
        pass,
        &[
            ("error", err_json(&err)),
            ("ticks", ticks.to_string()),
            ("instance_peak_bytes", g.peak.to_string()),
            ("process_peak_bytes", process_peak.to_string()),
            ("cap_bytes", cap.to_string()),
            ("request_cap_hit", flagged.to_string()),
            ("applied", applied.to_string()),
            ("recovery_pass", rec_ok.to_string()),
            ("recovery", json_str(&rec)),
        ],
    )
}

thread_local! {
    static L1: std::cell::RefCell<Option<(Guarded, Option<ScriptError>)>> = const { std::cell::RefCell::new(None) };
}

/// L1 in three steps, so that each driver can time `L1-call` alone (B9).
pub fn l1_prepare(kind: Kind) -> String {
    host::install_world(probe_world());
    host::with(|h| h.world.as_mut().expect("world").sim_step());
    let (name, src) = sources(kind, "L1").remove(0);
    match create(kind, &name, &src) {
        Ok(g) => {
            L1.with(|c| *c.borrow_mut() = Some((g, None)));
            "{\"ok\":true}".to_string()
        }
        Err(e) => format!("{{\"ok\":false,\"error\":{}}}", err_json(&Some(e))),
    }
}

pub fn l1_call() {
    L1.with(|c| {
        if let Some((g, err)) = c.borrow_mut().as_mut() {
            let events = host::with(|h| h.world.as_ref().expect("world").events.clone());
            *err = g.on_tick(&events).err();
        }
    });
}

pub fn l1_finish(kind: Kind) -> String {
    L1.with(|c| {
        let Some((mut g, err)) = c.borrow_mut().take() else {
            return record(kind, "L1", false, &[("detail", json_str("not prepared"))]);
        };
        let budget_err = err
            .as_ref()
            .is_some_and(|e| e.kind == ScriptErrorKind::Budget);
        let (rec_ok, rec) = recovery(&mut g);
        record(
            kind,
            "L1",
            budget_err && rec_ok,
            &[
                ("error", err_json(&err)),
                ("budget", g.limits.budget.to_string()),
                ("recovery_pass", rec_ok.to_string()),
                ("recovery", json_str(&rec)),
            ],
        )
    })
}

/// Run one probe (not L1) and return its JSON record.
pub fn run(kind: Kind, id: &str) -> String {
    match id {
        "L2" => bounded(kind, id, 1, false),
        "L3a" | "L3b" | "L3c-map" | "L3c-list" | "L3e" => bounded(kind, id, 1, true),
        "L3d" => bounded(kind, id, 1_000, true),
        "L9a" => bounded(kind, id, 1, false),
        "L4" => {
            host::install_world(probe_world());
            let mut all = true;
            let mut parts = Vec::new();
            for (name, src) in sources(kind, id) {
                let r = create(kind, &format!("L4-{name}"), &src).and_then(|mut g| tick(&mut g));
                let blocked = r.is_err();
                all &= blocked;
                let e = r.err();
                parts.push(format!(
                    "{{\"variant\":{},\"blocked\":{},\"error\":{}}}",
                    json_str(&name),
                    blocked,
                    err_json(&e)
                ));
            }
            let applied = host::with(|h| h.world.as_ref().map_or(0, |w| w.applied));
            record(
                kind,
                id,
                all && applied == 0,
                &[("variants", format!("[{}]", parts.join(",")))],
            )
        }
        "L5b" => {
            host::install_world(probe_world());
            let (name, src) = sources(kind, id).remove(0);
            let r = create(kind, &name, &src).and_then(|mut g| {
                let r = tick(&mut g);
                r
            });
            let (hash, applied, order) = host::with(|h| {
                let w = h.world.as_ref().expect("world");
                (w.output_hash(), w.applied, w.alerts)
            });
            record(
                kind,
                id,
                r.is_ok() && applied == 64,
                &[
                    ("error", err_json(&r.err())),
                    ("order_hash", json_str(&format!("{hash:016x}"))),
                    ("alerts", order.to_string()),
                ],
            )
        }
        "L7a" | "L7b" => {
            host::install_world(probe_world());
            let (name, src) = sources(kind, id).remove(0);
            let want = if id == "L7a" { 3 } else { 4 };
            let r = create(kind, &name, &src).and_then(|mut g| tick(&mut g));
            let e = r.err();
            let ok = e
                .as_ref()
                .is_some_and(|e| e.line == Some(want) && e.message.contains(&name));
            record(
                kind,
                id,
                ok,
                &[("error", err_json(&e)), ("want_line", want.to_string())],
            )
        }
        "L8" => {
            host::install_world(probe_world());
            let (name, src) = sources(kind, id).remove(0);
            let mut codes = Vec::new();
            let r = create(kind, &name, &src).and_then(|mut g| {
                for _ in 0..2 {
                    host::with(|h| h.world.as_mut().expect("world").sim_step());
                    g.on_tick(&[])?;
                    host::with(|h| {
                        for r in &h.requests {
                            if let Request::Alert { code, .. } = r {
                                codes.push(*code);
                            }
                        }
                        let mut req = std::mem::take(&mut h.requests);
                        h.world.as_mut().expect("world").apply(&mut req);
                        h.requests = req;
                    });
                }
                Ok(())
            });
            let pass = r.is_ok() && codes == vec![1, 100];
            record(
                kind,
                id,
                pass,
                &[
                    ("error", err_json(&r.err())),
                    ("alert_codes", format!("{codes:?}")),
                ],
            )
        }
        "L9b" => {
            host::install_world(probe_world());
            let (name, src) = sources(kind, id).remove(0);
            let r = create(kind, &name, &src).and_then(|mut g| tick(&mut g));
            let (rejected, moved) = host::with(|h| {
                let w = h.world.as_ref().expect("world");
                (
                    w.rejected,
                    w.units
                        .iter()
                        .filter(|u| u.tx == 1000 && u.ty == 1000)
                        .count(),
                )
            });
            record(
                kind,
                id,
                r.is_ok() && rejected == 1 && moved == 0,
                &[
                    ("error", err_json(&r.err())),
                    ("rejected", rejected.to_string()),
                    ("moved", moved.to_string()),
                ],
            )
        }
        _ => record(kind, id, false, &[("detail", json_str("unknown probe"))]),
    }
}
