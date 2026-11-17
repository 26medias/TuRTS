//! C1: Rhai 1.27.0. `Engine::new_raw` with only the packages that the rules need.

use std::cell::Cell;

use rhai::packages::{Package, StandardPackage};
use rhai::{Array, CallFnOptions, Dynamic, Engine, EvalAltResult, Position, Scope, AST};

use crate::cand::{self as c, Limits, Runtime, ScriptError, ScriptErrorKind};
use crate::host;

thread_local! {
    static LAST_OPS: Cell<u64> = const { Cell::new(0) };
}

const MEMORY_TOKEN: i64 = 0x6d656d;

pub struct RhaiRt {
    engine: Engine,
    ast: AST,
    scope: Scope<'static>,
    state: Dynamic,
    name: String,
}

fn line_of(p: Position) -> Option<u32> {
    p.line().map(|l| l as u32)
}

/// Map a Rhai error to the host error. Function-call wrappers are unwrapped to the cause.
fn map_err(name: &str, e: Box<EvalAltResult>) -> ScriptError {
    let mut e = *e;
    let mut line = line_of(e.position());
    loop {
        match e {
            EvalAltResult::ErrorInFunctionCall(_, _, inner, _) => {
                e = *inner;
                if let Some(l) = line_of(e.position()) {
                    line = Some(l);
                }
            }
            _ => break,
        }
    }
    let kind = match &e {
        EvalAltResult::ErrorTooManyOperations(_) => ScriptErrorKind::Budget,
        EvalAltResult::ErrorStackOverflow(_) => ScriptErrorKind::Depth,
        EvalAltResult::ErrorDataTooLarge(..) => ScriptErrorKind::Memory,
        EvalAltResult::ErrorTerminated(token, _) if token.as_int().ok() == Some(MEMORY_TOKEN) => {
            ScriptErrorKind::Memory
        }
        EvalAltResult::ErrorParsing(..) => ScriptErrorKind::Syntax,
        _ => ScriptErrorKind::Runtime,
    };
    ScriptError::new(kind, format!("{name}: {e}"), line)
}

impl RhaiRt {
    pub fn new(name: &str, source: &str, limits: Limits) -> Result<Self, ScriptError> {
        let mut engine = Engine::new_raw();
        // The standard package that a modder expects (strings, arrays, maps, math). With
        // `no_time` it has no clock; Rhai has no file, network, or random functions.
        engine.register_global_module(StandardPackage::new().as_shared_module());
        // No ambient output and no new source at run time (L4).
        engine.disable_symbol("print");
        engine.disable_symbol("debug");
        engine.disable_symbol("eval");
        engine.on_print(|_| {});
        engine.on_debug(|_, _, _| {});

        engine.set_max_operations(if limits.budget == u64::MAX {
            0
        } else {
            limits.budget
        });
        engine.set_max_call_levels(limits.depth);
        engine.set_max_expr_depths(64, 32);
        engine.set_max_string_size(1024 * 1024);
        engine.set_max_array_size(65_536);
        engine.set_max_map_size(4_096);
        // Whole-heap guard: Rhai limits each value, not the sum. The host checks the counting
        // allocator every 256 operations and stops the call over the room of the instance.
        engine.on_progress(|ops| {
            LAST_OPS.with(|c| c.set(ops));
            let mask = if host::mutant() == host::VARIANT_RHAI_GUARD_EVERY_OP { 0 } else { 255 };
            if ops & mask == 0 && c::call_over_cap() {
                return Some(Dynamic::from_int(MEMORY_TOKEN));
            }
            None
        });

        engine.register_fn("count_owned", host::count_owned);
        engine.register_fn("credits", host::credits);
        engine.register_fn("units_near", |p: i64, x: i64, y: i64, r: i64, m: i64| -> Array {
            host::units_near(p, x, y, r, m)
                .into_iter()
                .map(Dynamic::from_int)
                .collect()
        });
        engine.register_fn("grant", host::grant);
        engine.register_fn("move_to", host::move_to);
        engine.register_fn("move_group", |units: Array, x: i64, y: i64| {
            let v = units.iter().map(|d| d.as_int().unwrap_or(-1)).collect();
            host::move_group(v, x, y)
        });
        engine.register_fn("set_stance", host::set_stance);
        engine.register_fn("alert", host::alert);

        let ast = engine
            .compile(source)
            .map_err(|e| map_err(name, Box::new(e.into())))?;
        let mut rt = RhaiRt {
            engine,
            ast,
            scope: Scope::new(),
            state: Dynamic::UNIT,
            name: name.to_string(),
        };
        let state = rt.call("init", ())?;
        rt.state = state;
        Ok(rt)
    }

    fn call(&mut self, f: &str, args: impl rhai::FuncArgs) -> Result<Dynamic, ScriptError> {
        LAST_OPS.with(|c| c.set(0));
        let opts = CallFnOptions::new()
            .eval_ast(false)
            .rewind_scope(true)
            .bind_this_ptr(&mut self.state);
        let r = self
            .engine
            .call_fn_with_options::<Dynamic>(opts, &mut self.scope, &self.ast, f, args);
        r.map_err(|e| map_err(&self.name, e))
    }
}

impl Runtime for RhaiRt {
    fn on_tick(&mut self, events: &[i64]) -> Result<(), ScriptError> {
        let arr: Array = events.iter().map(|v| Dynamic::from_int(*v)).collect();
        self.call("on_tick", (arr,)).map(|_| ())
    }

    fn bench_calls(&mut self, n: i64) -> Result<i64, ScriptError> {
        let v = self.call("bench_calls", (n,))?;
        Ok(v.as_int().unwrap_or(-1))
    }

    fn on_unit(&mut self, id: i64, x: i64, y: i64, owner: i64) -> Result<(), ScriptError> {
        self.call("on_unit", (id, x, y, owner)).map(|_| ())
    }

    fn last_units(&mut self) -> Option<u64> {
        Some(LAST_OPS.with(|c| c.get()))
    }
}
