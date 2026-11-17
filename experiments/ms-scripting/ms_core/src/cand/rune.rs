//! C2: Rune 0.14.2. A context with selected modules only (see `context`).

use std::sync::Arc;

use rune::runtime::{budget, RuntimeContext, Value, VmError};
use rune::{Context, Diagnostics, Module, Source, Sources, Unit, Vm};

use crate::cand::{self as c, Limits, Runtime, ScriptError, ScriptErrorKind};
use crate::host;

pub struct RuneRt {
    vm: Vm,
    sources: Sources,
    state: Value,
    limits: Limits,
    name: String,
    last: u64,
}

/// A large budget that the calibration counts down from.
const CALIBRATE: usize = 50_000_000;

fn ctx_err(e: impl std::fmt::Display) -> ScriptError {
    ScriptError::new(ScriptErrorKind::Runtime, format!("rune context: {e}"), None)
}

fn host_module() -> Result<Module, rune::ContextError> {
    let mut m = Module::with_crate("host")?;
    m.function("count_owned", host::count_owned).build()?;
    m.function("credits", host::credits).build()?;
    m.function(
        "units_near",
        |p: i64, x: i64, y: i64, r: i64, n: i64| -> Vec<i64> { host::units_near(p, x, y, r, n) },
    )
    .build()?;
    m.function("grant", host::grant).build()?;
    m.function("move_to", host::move_to).build()?;
    m.function("move_group", |units: Vec<i64>, x: i64, y: i64| {
        host::move_group(units, x, y)
    })
    .build()?;
    m.function("set_stance", host::set_stance).build()?;
    m.function("alert", host::alert).build()?;
    Ok(m)
}

/// C2 context (spec 9, amendment 5): the core language modules only. No `std::io` (print,
/// dbg), no `std::test`, no futures or streams, no `collections`, `bytes`, `char`, `tuple`,
/// `hash`, `any`, `mem`, `u64`, generators, or macros.
#[cfg(not(feature = "rune_full"))]
fn context() -> Result<Context, rune::ContextError> {
    use rune::modules as m;
    let mut cx = Context::new();
    cx.install(m::iter::module()?)?;
    cx.install(m::core::module()?)?;
    cx.install(m::cmp::module()?)?;
    cx.install(m::clone::module()?)?;
    cx.install(m::num::module()?)?;
    cx.install(m::string::module()?)?;
    cx.install(m::f64::module()?)?;
    cx.install(m::fmt::module()?)?;
    cx.install(m::i64::module()?)?;
    cx.install(m::object::module()?)?;
    cx.install(m::ops::module()?)?;
    cx.install(m::option::module()?)?;
    cx.install(m::result::module()?)?;
    cx.install(m::vec::module()?)?;
    cx.install(m::slice::module()?)?;
    cx.install(host_module()?)?;
    Ok(cx)
}

/// Recorded-only variant `rune-full`: the broad module set of the first run.
#[cfg(feature = "rune_full")]
fn context() -> Result<Context, rune::ContextError> {
    use rune::modules as m;
    let mut cx = Context::new();
    cx.install(m::iter::module()?)?;
    cx.install(m::core::module()?)?;
    cx.install(m::cmp::module()?)?;
    cx.install(m::any::module()?)?;
    cx.install(m::clone::module()?)?;
    cx.install(m::num::module()?)?;
    cx.install(m::hash::module()?)?;
    cx.install(m::string::module()?)?;
    cx.install(m::bytes::module()?)?;
    cx.install(m::collections::module()?)?;
    cx.install(m::char::module()?)?;
    cx.install(m::f64::module()?)?;
    cx.install(m::tuple::module()?)?;
    cx.install(m::fmt::module()?)?;
    cx.install(m::i64::module()?)?;
    cx.install(m::u64::module()?)?;
    cx.install(m::macros::module()?)?;
    cx.install(m::macros::builtin::module()?)?;
    cx.install(m::mem::module()?)?;
    cx.install(m::object::module()?)?;
    cx.install(m::ops::module()?)?;
    cx.install(m::ops::generator::module()?)?;
    cx.install(m::option::module()?)?;
    cx.install(m::result::module()?)?;
    cx.install(m::vec::module()?)?;
    cx.install(m::slice::module()?)?;
    cx.install(host_module()?)?;
    Ok(cx)
}

impl RuneRt {
    pub fn new(name: &str, source: &str, limits: Limits) -> Result<Self, ScriptError> {
        let cx = context().map_err(ctx_err)?;
        let runtime: Arc<RuntimeContext> = Arc::new(cx.runtime().map_err(ctx_err)?);
        let mut sources = Sources::new();
        sources
            .insert(Source::new(name, source).map_err(ctx_err)?)
            .map_err(ctx_err)?;
        let mut diagnostics = Diagnostics::new();
        let unit = rune::prepare(&mut sources)
            .with_context(&cx)
            .with_diagnostics(&mut diagnostics)
            .build();
        let unit: Unit = match unit {
            Ok(u) => u,
            Err(_) => {
                let mut line = None;
                let mut msg = String::new();
                for d in diagnostics.diagnostics() {
                    if let rune::diagnostics::Diagnostic::Fatal(f) = d {
                        msg = format!("{name}: {f}");
                        if let Some(src) = sources.get(f.source_id()) {
                            if let Some(span) = fatal_span(f) {
                                let (l, _) = src.pos_to_utf8_linecol(span);
                                line = Some(l as u32 + 1);
                            }
                        }
                        break;
                    }
                }
                return Err(ScriptError::new(ScriptErrorKind::Syntax, msg, line));
            }
        };
        let vm = Vm::new(runtime, Arc::new(unit));
        let mut rt = RuneRt {
            vm,
            sources,
            state: Value::empty(),
            limits,
            name: name.to_string(),
            last: 0,
        };
        let state = rt.call("init", ())?;
        rt.state = state;
        Ok(rt)
    }

    fn map_err(&self, e: VmError) -> ScriptError {
        let text = e.to_string();
        let kind = if text.contains("limited") {
            ScriptErrorKind::Budget
        } else if text.to_lowercase().contains("alloc") || text.contains("memory") {
            ScriptErrorKind::Memory
        } else if text.contains("overflow") && text.contains("stack") {
            ScriptErrorKind::Depth
        } else {
            ScriptErrorKind::Runtime
        };
        let mut line = None;
        if let Some(loc) = e.first_location() {
            if let Some(inst) = loc.unit.debug_info().and_then(|d| d.instruction_at(loc.ip)) {
                if let Some(src) = self.sources.get(inst.source_id) {
                    let (l, _) = src.pos_to_utf8_linecol(inst.span.start.into_usize());
                    line = Some(l as u32 + 1);
                }
            }
        }
        ScriptError::new(kind, format!("{}: {text}", self.name), line)
    }

    fn call(
        &mut self,
        f: &'static str,
        args: impl rune::runtime::GuardedArgs,
    ) -> Result<Value, ScriptError> {
        let calibrating = self.limits.budget == u64::MAX;
        let b = if calibrating {
            CALIBRATE
        } else {
            self.limits.budget.min(usize::MAX as u64) as usize
        };
        let room = c::call_room();
        let vm = &mut self.vm;
        let mut left = 0u64;
        let r = rune::alloc::limit::with(
            room,
            budget::with(b, || {
                let r = vm.call([f], args);
                if calibrating {
                    let mut g = budget::acquire();
                    while g.take() {
                        left += 1;
                    }
                }
                r
            }),
        )
        .call();
        if calibrating {
            self.last = CALIBRATE as u64 - left;
        }
        r.map_err(|e| self.map_err(e))
    }
}

fn fatal_span(f: &rune::diagnostics::FatalDiagnostic) -> Option<usize> {
    use rune::ast::Spanned;
    match f.kind() {
        rune::diagnostics::FatalDiagnosticKind::CompileError(e) => {
            Some(e.span().start.into_usize())
        }
        _ => None,
    }
}

impl Runtime for RuneRt {
    fn on_tick(&mut self, events: &[i64]) -> Result<(), ScriptError> {
        let state = self.state.clone();
        self.call("on_tick", (state, events.to_vec())).map(|_| ())
    }

    fn bench_calls(&mut self, n: i64) -> Result<i64, ScriptError> {
        let state = self.state.clone();
        let v = self.call("bench_calls", (state, n))?;
        Ok(rune::from_value::<i64>(v).unwrap_or(-1))
    }

    fn on_unit(&mut self, id: i64, x: i64, y: i64, owner: i64) -> Result<(), ScriptError> {
        let state = self.state.clone();
        self.call("on_unit", (state, id, x, y, owner)).map(|_| ())
    }

    fn last_units(&mut self) -> Option<u64> {
        Some(self.last)
    }
}
