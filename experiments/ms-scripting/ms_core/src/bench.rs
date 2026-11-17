//! The measured sequence. Both drivers call `sim`, `script`, `apply` in this order for each tick
//! and time each phase themselves (spec 5).

use crate::cand::{rules_source, Guarded, Kind, Limits, ScriptError};
use crate::host;
use crate::world::{EventRate, World};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Workload {
    W1,
    W2,
    W3,
    W4,
}

impl Workload {
    pub const ALL: [Workload; 4] = [Workload::W1, Workload::W2, Workload::W3, Workload::W4];

    pub fn name(self) -> &'static str {
        match self {
            Workload::W1 => "W1",
            Workload::W2 => "W2",
            Workload::W3 => "W3",
            Workload::W4 => "W4",
        }
    }

    pub fn from_code(c: u32) -> Option<Workload> {
        Workload::ALL.get(c as usize).copied()
    }

    fn slot(self) -> usize {
        match self {
            Workload::W1 => 0,
            Workload::W2 => 1,
            Workload::W3 | Workload::W4 => 2,
        }
    }

    pub fn warmup(self) -> u32 {
        match self {
            Workload::W4 => 50,
            _ => 200,
        }
    }

    pub fn measured(self) -> u32 {
        match self {
            Workload::W4 => 500,
            _ => 2_000,
        }
    }
}

/// W3: host calls in one script call.
pub const W3_CALLS: i64 = 1_000;

fn make_world(slot: usize) -> World {
    match slot {
        0 => World::new(1_000, 0x5EED_0001, EventRate { min: 12, spread: 9 }),
        1 => World::new(5_000, 0x5EED_0002, EventRate { min: 256, spread: 0 }),
        _ => World::new(1_000, 0x5EED_0003, EventRate { min: 12, spread: 9 }),
    }
}

pub struct Bench {
    pub kind: Kind,
    pub guard: Guarded,
    worlds: [Option<World>; 3],
    current: Option<usize>,
    pub errors: u64,
    pub first_error: Option<ScriptError>,
    pub w3_sum: i64,
    units_buf: Vec<[i64; 4]>,
}

impl Bench {
    /// Create the worlds first, then the runtime, so that the runtime accounting holds only
    /// the runtime. `limits`: `None` uses the calibrated limits of the kind.
    pub fn new(kind: Kind, limits: Option<Limits>) -> Result<Bench, ScriptError> {
        Self::with_source(kind, rules_source(kind), limits)
    }

    /// As `new`, with another script source (tests).
    pub fn with_source(kind: Kind, source: &str, limits: Option<Limits>) -> Result<Bench, ScriptError> {
        let worlds = [Some(make_world(0)), Some(make_world(1)), Some(make_world(2))];
        let limits = limits.unwrap_or_else(|| Limits::for_kind(kind));
        let guard = Guarded::create(kind, &format!("rules.{}", kind.name()), source, limits)?;
        Ok(Bench {
            kind,
            guard,
            worlds,
            current: None,
            errors: 0,
            first_error: None,
            w3_sum: 0,
            units_buf: Vec::with_capacity(1_000),
        })
    }

    /// Put the world of `w` into the host.
    pub fn select(&mut self, w: Workload) {
        let slot = w.slot();
        if self.current == Some(slot) {
            return;
        }
        if let Some(old) = self.current {
            self.worlds[old] = host::take_world();
        }
        host::install_world(self.worlds[slot].take().expect("world present"));
        self.current = Some(slot);
    }

    pub fn sim(&mut self) {
        host::with(|h| h.world.as_mut().expect("world").sim_step());
    }

    fn record(&mut self, r: Result<(), ScriptError>) {
        if let Err(e) = r {
            self.errors += 1;
            if self.first_error.is_none() {
                self.first_error = Some(e);
            }
        }
    }

    pub fn script(&mut self, w: Workload) {
        match w {
            Workload::W1 | Workload::W2 => {
                let events = host::with(|h| std::mem::take(&mut h.world.as_mut().expect("world").events));
                let r = self.guard.on_tick(&events);
                host::with(|h| h.world.as_mut().expect("world").events = events);
                self.record(r);
            }
            Workload::W3 => {
                let r = self.guard.bench_calls(W3_CALLS).map(|s| {
                    self.w3_sum = s;
                });
                self.record(r);
            }
            Workload::W4 => {
                let mut buf = std::mem::take(&mut self.units_buf);
                buf.clear();
                host::with(|h| {
                    for (i, u) in h.world.as_ref().expect("world").units.iter().enumerate() {
                        buf.push([i as i64, i64::from(u.x), i64::from(u.y), i64::from(u.owner)]);
                    }
                });
                for u in &buf {
                    let r = self.guard.on_unit(u[0], u[1], u[2], u[3]);
                    self.record(r);
                }
                self.units_buf = buf;
            }
        }
    }

    pub fn apply(&mut self) {
        host::with(|h| {
            let mut req = std::mem::take(&mut h.requests);
            h.world.as_mut().expect("world").apply(&mut req);
            h.requests = req;
        });
    }

    /// Output hash of the world of `w` (requests in order and final credits).
    pub fn hash(&mut self, w: Workload) -> u64 {
        self.select(w);
        host::with(|h| h.world.as_ref().expect("world").output_hash())
    }

    /// Budget units of the last call (calibration).
    pub fn last_units(&mut self) -> Option<u64> {
        self.guard.rt.last_units()
    }
}

impl Bench {
    /// The non-timing part of a suite record, as JSON fields (no braces). Same on both targets.
    pub fn report_fields(&mut self) -> String {
        let h1 = self.hash(Workload::W1);
        let h2 = self.hash(Workload::W2);
        let calls = host::with(|h| h.calls);
        let first = match &self.first_error {
            None => "null".to_string(),
            Some(e) => crate::json_str(&format!("{}: {}", e.kind.name(), e.message)),
        };
        format!(
            "\"kind\":{},\"hash_w1\":\"{h1:016x}\",\"hash_w2\":\"{h2:016x}\",\"errors\":{},\"first_error\":{first},\"w3_sum\":{},\"instance_peak_bytes\":{},\"instance_retained_bytes\":{},\"budget\":{},\"host_calls\":{calls}",
            crate::json_str(self.kind.name()),
            self.errors,
            self.w3_sum,
            self.guard.peak,
            self.guard.retained,
            if self.guard.limits.budget == u64::MAX { "null".to_string() } else { self.guard.limits.budget.to_string() },
        )
    }

    /// The instance peak after W1 and W2 (B6), read by the drivers before W3.
    pub fn instance_peak(&self) -> usize {
        self.guard.peak
    }
}
