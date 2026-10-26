//! ReplicaState.
//!
//! The client copy of the units, built from `Snapshot` and `UnitDeltas` (design section 5).
//! The replica starts in resync, because the client enters resync at `Welcome` (G2). While in
//! resync it buffers deltas. A `Snapshot` ends the resync: the replica drops buffered deltas at
//! or below the snapshot tick and replays the rest while their `base_tick` chain holds.

use std::collections::{HashMap, VecDeque};

use bevy::platform::time::Instant;
use turts_net_protocol::{Pos, Snapshot, UnitDeltas};

use crate::interp;

/// A replica keeps the samples of the last `PRUNE_TICKS` ticks before the latest tick.
const PRUNE_TICKS: u32 = 16;

/// One unit on the client.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitReplica {
    pub content_id: String,
    pub owner_slot: u8,
    /// Ascending, consecutive ticks.
    pub samples: VecDeque<(u32, Pos)>,
}

/// The position of one unit at a presentation time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Presented {
    pub pos: Pos,
    pub lo_tick: u32,
    pub hi_tick: u32,
    pub fraction: f32,
}

#[derive(Debug, PartialEq)]
pub enum DeltaOutcome {
    Applied,
    Discarded,
    Buffered,
    ResyncRequested,
}

#[derive(Debug, PartialEq)]
pub enum SnapshotOutcome {
    Applied { replayed: u32 },
    ResyncRequested { replayed: u32 },
}

#[derive(Debug, Clone)]
pub struct Replica {
    pub tick_hz: u16,
    pub units: HashMap<u32, UnitReplica>,
    pub snapshot_tick: Option<u32>,
    pub last_applied_tick: Option<u32>,
    pub latest_receipt: Option<Instant>,
    /// `Some` while in resync. It holds the buffered deltas in arrival order.
    pub resync: Option<Vec<UnitDeltas>>,
}

impl Replica {
    /// A new replica is in resync with an empty buffer: deltas before the first `Snapshot`
    /// are buffered (G2).
    pub fn new(tick_hz: u16) -> Self {
        Self {
            tick_hz,
            units: HashMap::new(),
            snapshot_tick: None,
            last_applied_tick: None,
            latest_receipt: None,
            resync: Some(Vec::new()),
        }
    }

    pub fn in_resync(&self) -> bool {
        self.resync.is_some()
    }

    pub fn apply_snapshot(&mut self, s: &Snapshot, now: Instant) -> SnapshotOutcome {
        self.units = s
            .units
            .iter()
            .map(|u| {
                let unit = UnitReplica {
                    content_id: u.content_id.clone(),
                    owner_slot: u.owner_slot.0,
                    samples: VecDeque::from([(s.tick, u.pos)]),
                };
                (u.unit_id.0, unit)
            })
            .collect();
        self.snapshot_tick = Some(s.tick);
        self.last_applied_tick = Some(s.tick);
        self.latest_receipt = Some(now);

        let buffer = self.resync.take().unwrap_or_default();
        let mut pending = buffer.into_iter().filter(|d| d.tick > s.tick);
        let mut replayed = 0;
        while let Some(d) = pending.next() {
            if !self.extends_chain(&d) {
                let mut rest = vec![d];
                rest.extend(pending);
                self.resync = Some(rest);
                return SnapshotOutcome::ResyncRequested { replayed };
            }
            self.apply_in_chain(&d, now);
            replayed += 1;
        }
        SnapshotOutcome::Applied { replayed }
    }

    pub fn apply_delta(&mut self, d: &UnitDeltas, now: Instant) -> DeltaOutcome {
        if let Some(buffer) = self.resync.as_mut() {
            buffer.push(d.clone());
            return DeltaOutcome::Buffered;
        }
        if self.snapshot_tick.is_some_and(|s| d.tick <= s) {
            return DeltaOutcome::Discarded;
        }
        if !self.extends_chain(d) {
            self.resync = Some(vec![d.clone()]);
            return DeltaOutcome::ResyncRequested;
        }
        self.apply_in_chain(d, now);
        DeltaOutcome::Applied
    }

    /// True when `d` follows the last applied tick and names only known units. A change for an
    /// unknown unit is a chain break, so the replica never invents a unit.
    fn extends_chain(&self, d: &UnitDeltas) -> bool {
        Some(d.base_tick) == self.last_applied_tick
            && d.changes
                .iter()
                .all(|c| self.units.contains_key(&c.unit_id.0))
    }

    /// Applies a delta that `extends_chain` accepted: every unit gets a sample at `d.tick`.
    fn apply_in_chain(&mut self, d: &UnitDeltas, now: Instant) {
        let changes: HashMap<u32, Pos> = d.changes.iter().map(|c| (c.unit_id.0, c.pos)).collect();
        let keep_from = d.tick.saturating_sub(PRUNE_TICKS);
        for (id, unit) in self.units.iter_mut() {
            let pos = match changes.get(id) {
                Some(&p) => p,
                None => unit
                    .samples
                    .back()
                    .map(|&(_, p)| p)
                    .expect("a unit has at least one sample"),
            };
            unit.samples.push_back((d.tick, pos));
            while unit.samples.front().is_some_and(|&(t, _)| t < keep_from) {
                unit.samples.pop_front();
            }
        }
        self.last_applied_tick = Some(d.tick);
        self.latest_receipt = Some(now);
    }

    /// `None` before the first snapshot.
    pub fn presentation_time(&self, now: Instant) -> Option<f64> {
        let latest = self.last_applied_tick?;
        let receipt = self.latest_receipt?;
        let since = now.saturating_duration_since(receipt).as_secs_f64();
        Some(interp::presentation_time(latest, since, self.tick_hz))
    }

    /// The position of `unit_id` at presentation time `t`. It interpolates only between the
    /// samples at two consecutive ticks and never reads above the latest sample.
    pub fn sample(&self, unit_id: u32, t: f64) -> Option<Presented> {
        let samples = &self.units.get(&unit_id)?.samples;
        let &(first, first_pos) = samples.front()?;
        let &(last, last_pos) = samples.back()?;
        let floor = t.floor();
        if floor.is_nan() || floor < f64::from(first) {
            return Some(Presented {
                pos: first_pos,
                lo_tick: first,
                hi_tick: first,
                fraction: 0.0,
            });
        }
        if floor >= f64::from(last) {
            return Some(Presented {
                pos: last_pos,
                lo_tick: last,
                hi_tick: last,
                fraction: 0.0,
            });
        }
        let lo = floor as u32;
        let hi = lo + 1;
        let fraction = (t - floor) as f32;
        let a = samples[(lo - first) as usize].1;
        let b = samples[(hi - first) as usize].1;
        if fraction == 0.0 {
            return Some(Presented {
                pos: a,
                lo_tick: lo,
                hi_tick: lo,
                fraction: 0.0,
            });
        }
        Some(Presented {
            pos: interp::lerp(a, b, fraction),
            lo_tick: lo,
            hi_tick: hi,
            fraction,
        })
    }
}
