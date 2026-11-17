//! Candidate runtimes behind one trait, and the host policy that wraps them.

#[cfg(feature = "lua")]
pub mod lua;
#[cfg(feature = "rhai")]
pub mod rhai;
#[cfg(feature = "rune")]
pub mod rune;

use std::cell::Cell;

use crate::alloc;
use crate::host;
use crate::rules::{NoScript, RustRef};

thread_local! {
    /// Live bytes when the current call started.
    static CALL_BASE: Cell<usize> = const { Cell::new(0) };
    /// Bytes the current call may still grow (cap minus the retained bytes of the instance).
    static CALL_ROOM: Cell<usize> = const { Cell::new(Limits::MEMORY) };
}

/// Growth that the current call may still make, in bytes.
pub fn call_room() -> usize {
    CALL_ROOM.with(|c| c.get())
}

/// True when the current call grew the live heap over its room.
pub fn call_over_cap() -> bool {
    alloc::live().saturating_sub(CALL_BASE.with(|c| c.get())) > call_room()
}

fn begin_call(room: usize) -> usize {
    let base = alloc::live();
    CALL_BASE.with(|c| c.set(base));
    CALL_ROOM.with(|c| c.set(room));
    host::begin_call();
    base
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    None,
    Rust,
    Rhai,
    Rune,
    Lua,
}

impl Kind {
    pub fn parse(s: &str) -> Option<Kind> {
        Some(match s {
            "none" => Kind::None,
            "rust" => Kind::Rust,
            "rhai" => Kind::Rhai,
            "rune" => Kind::Rune,
            "lua" => Kind::Lua,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::None => "none",
            Kind::Rust => "rust",
            Kind::Rhai => "rhai",
            Kind::Rune => "rune",
            Kind::Lua => "lua",
        }
    }

    pub fn from_code(c: u32) -> Option<Kind> {
        Some(match c {
            0 => Kind::None,
            1 => Kind::Rust,
            2 => Kind::Rhai,
            3 => Kind::Rune,
            4 => Kind::Lua,
            _ => return None,
        })
    }

    pub fn is_script(self) -> bool {
        matches!(self, Kind::Rhai | Kind::Rune | Kind::Lua)
    }
}

/// Per-call limits that the host sets. Same values on both targets (plan, Task 3).
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Instruction budget for one call, in the candidate's own unit
    /// (Rhai operations, Rune instructions, piccolo fuel). `u64::MAX`: no budget.
    pub budget: u64,
    /// Cap on script heap growth during one call, in bytes.
    pub memory: usize,
    /// Call depth cap, where the runtime has one.
    pub depth: usize,
}

impl Limits {
    pub const MEMORY: usize = 32 * 1024 * 1024;
    pub const DEPTH: usize = 32;

    pub fn unlimited() -> Self {
        Limits {
            budget: u64::MAX,
            memory: Self::MEMORY,
            depth: Self::DEPTH,
        }
    }

    /// The calibrated budget for each candidate (plan, Task 3: 4 x the largest W2 call,
    /// rounded up to two significant digits). `u64::MAX` for the Rust kinds.
    pub fn for_kind(kind: Kind) -> Self {
        let budget = match kind {
            Kind::Rhai => BUDGET_RHAI,
            Kind::Rune => BUDGET_RUNE,
            Kind::Lua => BUDGET_LUA,
            _ => u64::MAX,
        };
        Limits {
            budget,
            memory: Self::MEMORY,
            depth: Self::DEPTH,
        }
    }
}

// Calibrated by `ms_native calibrate` (plan Task 3). Recorded in docs/measurements/scripting.md.
// Calibration 2026-11-16 (out/calibrate/*.json): largest call over W1 to W3 was Rhai 22,480
// operations (W2), Rune 12,565 instructions (W2), piccolo 31,014 fuel (W3).
pub const BUDGET_RHAI: u64 = 90_000;
pub const BUDGET_RUNE: u64 = 51_000;
pub const BUDGET_LUA: u64 = 130_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptErrorKind {
    Budget,
    Memory,
    Depth,
    Syntax,
    Runtime,
    Disabled,
}

impl ScriptErrorKind {
    pub fn name(self) -> &'static str {
        match self {
            ScriptErrorKind::Budget => "budget",
            ScriptErrorKind::Memory => "memory",
            ScriptErrorKind::Depth => "depth",
            ScriptErrorKind::Syntax => "syntax",
            ScriptErrorKind::Runtime => "runtime",
            ScriptErrorKind::Disabled => "disabled",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScriptError {
    pub kind: ScriptErrorKind,
    pub message: String,
    pub line: Option<u32>,
}

impl ScriptError {
    pub fn new(kind: ScriptErrorKind, message: impl Into<String>, line: Option<u32>) -> Self {
        ScriptError {
            kind,
            message: message.into(),
            line,
        }
    }
}

/// The calls that the host makes into a script. All candidates implement the same three.
pub trait Runtime {
    /// Handle one batch of events (`EVENT_INTS` ints for each event).
    fn on_tick(&mut self, events: &[i64]) -> Result<(), ScriptError>;
    /// W3: make `n` `count_owned` calls from the script; return their sum.
    fn bench_calls(&mut self, n: i64) -> Result<i64, ScriptError>;
    /// W4: one call for one unit (the anti-pattern, recorded only).
    fn on_unit(&mut self, id: i64, x: i64, y: i64, owner: i64) -> Result<(), ScriptError>;
    /// Budget units that the last call used, when the runtime can tell (calibration only).
    fn last_units(&mut self) -> Option<u64> {
        None
    }
    /// Run a full garbage collection, where the runtime has one (piccolo).
    fn collect(&mut self) {}
}

/// The source of the mod rules for a candidate (spec 3.2).
pub fn rules_source(kind: Kind) -> &'static str {
    match kind {
        Kind::Rhai => include_str!("../../../scripts/rules.rhai"),
        Kind::Rune => include_str!("../../../scripts/rules.rn"),
        Kind::Lua => include_str!("../../../scripts/rules.lua"),
        _ => "",
    }
}

/// Create a runtime of `kind` from `source`. `name` is the script name in errors.
pub fn create(
    kind: Kind,
    name: &str,
    source: &str,
    limits: Limits,
) -> Result<Box<dyn Runtime>, ScriptError> {
    let _ = (name, source, limits);
    match kind {
        Kind::None => Ok(Box::new(NoScript)),
        Kind::Rust => Ok(Box::new(RustRef::default())),
        #[cfg(feature = "rhai")]
        Kind::Rhai => Ok(Box::new(rhai::RhaiRt::new(name, source, limits)?)),
        #[cfg(feature = "rune")]
        Kind::Rune => Ok(Box::new(rune::RuneRt::new(name, source, limits)?)),
        #[cfg(feature = "lua")]
        Kind::Lua => Ok(Box::new(lua::LuaRt::new(name, source, limits)?)),
        #[allow(unreachable_patterns)]
        _ => Err(ScriptError::new(
            ScriptErrorKind::Disabled,
            format!("candidate {} is not in this build", kind.name()),
            None,
        )),
    }
}

/// Host policy around a runtime (spec 3.5): per-instance memory accounting, the request cap,
/// a failed call leaves no requests, and a budget, memory, or depth failure disables the
/// script for the rest of the match with an error that names it.
pub struct Guarded {
    pub name: String,
    pub rt: Box<dyn Runtime>,
    pub disabled: Option<ScriptError>,
    pub limits: Limits,
    /// Retained live bytes of this instance: the sum of (live after - live before) of its calls.
    pub retained: isize,
    /// The highest `retained + in-call growth` seen, in bytes.
    pub peak: usize,
}

impl Guarded {
    /// Create the runtime under the same accounting as a call.
    pub fn create(kind: Kind, name: &str, source: &str, limits: Limits) -> Result<Self, ScriptError> {
        alloc::reset_peak();
        let base = begin_call(limits.memory);
        let rt = create(kind, name, source, limits);
        let after = alloc::live();
        let peak = alloc::peak().saturating_sub(base);
        let rt = rt?;
        let mut g = Guarded {
            name: name.to_string(),
            rt,
            disabled: None,
            limits,
            retained: after as isize - base as isize,
            peak,
        };
        if g.retained > limits.memory as isize {
            let e = g.over_cap();
            return Err(e);
        }
        g.peak = g.peak.max(g.retained.max(0) as usize);
        Ok(g)
    }

    fn over_cap(&self) -> ScriptError {
        ScriptError::new(
            ScriptErrorKind::Memory,
            format!(
                "{}: instance memory {} bytes is over the cap of {} bytes",
                self.name, self.retained, self.limits.memory
            ),
            None,
        )
    }

    fn run<T>(
        &mut self,
        f: impl FnOnce(&mut dyn Runtime) -> Result<T, ScriptError>,
    ) -> Result<T, ScriptError> {
        if let Some(e) = &self.disabled {
            return Err(ScriptError::new(
                ScriptErrorKind::Disabled,
                format!(
                    "script {} is disabled after a {} error: {}",
                    self.name,
                    e.kind.name(),
                    e.message
                ),
                e.line,
            ));
        }
        let mark = host::request_len();
        let retained = self.retained.max(0) as usize;
        let room = if host::accounting() {
            self.limits.memory.saturating_sub(retained)
        } else {
            usize::MAX
        };
        let host_before = host::request_heap_bytes();
        alloc::reset_peak();
        let base = begin_call(room);
        let mut r = f(self.rt.as_mut());
        let after = alloc::live();
        // Request payloads belong to the host (freed in `apply`): take them out of the delta.
        let host_growth = host::request_heap_bytes() as isize - host_before as isize;
        self.peak = self
            .peak
            .max(retained + alloc::peak().saturating_sub(base).saturating_sub(host_growth.max(0) as usize));
        self.retained += after as isize - base as isize - host_growth;
        if r.is_ok() && host::over_request_cap() {
            r = Err(ScriptError::new(
                ScriptErrorKind::Budget,
                format!(
                    "{}: more than {} requests in one call",
                    self.name,
                    host::REQUEST_CAP
                ),
                None,
            ));
        }
        if r.is_ok() && self.retained > self.limits.memory as isize && host::accounting() {
            let before = alloc::live();
            self.rt.collect();
            self.retained += alloc::live() as isize - before as isize;
            if self.retained > self.limits.memory as isize {
                r = Err(self.over_cap());
            }
        }
        if let Err(e) = &r {
            host::truncate_requests(mark);
            if matches!(
                e.kind,
                ScriptErrorKind::Budget | ScriptErrorKind::Memory | ScriptErrorKind::Depth
            ) {
                self.disabled = Some(e.clone());
            }
        }
        r
    }

    pub fn on_tick(&mut self, events: &[i64]) -> Result<(), ScriptError> {
        self.run(|rt| rt.on_tick(events))
    }

    pub fn bench_calls(&mut self, n: i64) -> Result<i64, ScriptError> {
        self.run(|rt| rt.bench_calls(n))
    }

    pub fn on_unit(&mut self, id: i64, x: i64, y: i64, owner: i64) -> Result<(), ScriptError> {
        self.run(|rt| rt.on_unit(id, x, y, owner))
    }
}
