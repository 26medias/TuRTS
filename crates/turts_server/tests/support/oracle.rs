//! The segment oracle (design section 7, T5; plan Task 9).
//!
//! It checks the frame log of one client against the server tick log. For each frame of the
//! unit, in file order:
//!
//! 1. bracket: `hi == lo + 1`, or `hi == lo` and `lo == latest_tick`;
//! 2. `hi <= latest_tick`;
//! 3. `latest_tick` is at most the highest tick in the tick log with `wall_ns <= frame_ns`;
//! 4. `0 <= fraction <= 1`;
//! 5. when `hi == lo + 1`: `|fraction - (t - lo)| <= 1e-4`; when `hi == lo`: `fraction == 0`;
//! 6. `pos` is within 1e-3 of `lerp(P(lo), P(hi), fraction)`, where `P` reads the tick log;
//! 7. `t` never decreases from one frame to the next.
//!
//! Lag at a frame is the continuous server tick at `frame_ns` (linear between the wall times
//! of the tick log) minus `t`. The report gives the p99 (the sample at index
//! `floor(0.99 * (n - 1))` of the sorted lags) and the maximum.

use std::collections::BTreeMap;

use super::logs::{FrameRecord, TickRecord};

/// Tolerance of the `fraction == t - lo` rule.
const FRACTION_TOLERANCE: f64 = 1e-4;
/// Tolerance of the segment rule, per axis.
const POS_TOLERANCE: f32 = 1e-3;

#[derive(Debug, Clone)]
pub struct OracleReport {
    /// The frames of the unit that the oracle checked.
    pub frames: usize,
    pub lag_p99: f64,
    pub lag_max: f64,
}

/// The wall time of each tick, ascending by tick.
struct TickClock(Vec<(u32, u128)>);

impl TickClock {
    fn new(ticks: &[TickRecord]) -> Self {
        let mut walls: BTreeMap<u32, u128> = BTreeMap::new();
        for record in ticks {
            walls.entry(record.tick).or_insert(record.wall_ns);
        }
        Self(walls.into_iter().collect())
    }

    /// The highest tick with `wall_ns <= at`, and its index.
    fn logged_by(&self, at: u128) -> Option<(usize, u32)> {
        let index = self
            .0
            .partition_point(|&(_, wall)| wall <= at)
            .checked_sub(1)?;
        Some((index, self.0[index].0))
    }

    /// The continuous server tick at `at`: linear between two logged ticks.
    fn continuous_tick(&self, at: u128) -> Result<f64, String> {
        let (index, tick) = self
            .logged_by(at)
            .ok_or_else(|| format!("frame at {at} ns is before the tick log"))?;
        let Some(&(next_tick, next_wall)) = self.0.get(index + 1) else {
            return Err(format!("frame at {at} ns is after the end of the tick log"));
        };
        let wall = self.0[index].1;
        let part = (at - wall) as f64 / (next_wall - wall) as f64;
        Ok(f64::from(tick) + part * f64::from(next_tick - tick))
    }
}

/// Check every frame of `unit_id` against the tick log. `Err` names the first frame that
/// breaks a rule and the rule.
pub fn check_segments(
    ticks: &[TickRecord],
    frames: &[FrameRecord],
    unit_id: u32,
) -> Result<OracleReport, String> {
    let positions: BTreeMap<u32, (f32, f32)> = ticks
        .iter()
        .filter(|record| record.unit_id == unit_id)
        .map(|record| (record.tick, record.pos))
        .collect();
    let clock = TickClock::new(ticks);
    let position = |tick: u32| {
        positions
            .get(&tick)
            .copied()
            .ok_or_else(|| format!("tick {tick} of unit {unit_id} is not in the tick log"))
    };

    let mut lags = Vec::new();
    let mut previous_t: Option<f64> = None;
    for (index, frame) in frames
        .iter()
        .filter(|frame| frame.unit_id == unit_id)
        .enumerate()
    {
        let FrameRecord {
            frame_ns,
            t,
            lo,
            hi,
            fraction,
            pos,
            latest_tick,
            ..
        } = *frame;
        let at = |rule: String| format!("frame {index} (t={t}, frame_ns={frame_ns}): {rule}");

        if !(hi == lo + 1 || (hi == lo && lo == latest_tick)) {
            return Err(at(format!(
                "bracket lo={lo} hi={hi} latest={latest_tick}: hi == lo only at latest_tick"
            )));
        }
        if hi > latest_tick {
            return Err(at(format!("hi {hi} above latest_tick {latest_tick}")));
        }
        let logged = clock.logged_by(frame_ns).map(|(_, tick)| tick);
        if logged.is_none_or(|logged| latest_tick > logged) {
            return Err(at(format!(
                "latest_tick {latest_tick} ahead of the tick log (logged by then: {logged:?})"
            )));
        }
        if !(0.0..=1.0).contains(&fraction) {
            return Err(at(format!("fraction {fraction} outside [0, 1]")));
        }
        if hi == lo + 1 && (f64::from(fraction) - (t - f64::from(lo))).abs() > FRACTION_TOLERANCE {
            return Err(at(format!(
                "fraction {fraction} != t - lo = {}",
                t - f64::from(lo)
            )));
        }
        if hi == lo && fraction != 0.0 {
            return Err(at(format!("fraction {fraction} != 0 with hi == lo")));
        }
        let (a, b) = (position(lo).map_err(at)?, position(hi).map_err(at)?);
        let expected = (a.0 + (b.0 - a.0) * fraction, a.1 + (b.1 - a.1) * fraction);
        if (pos.0 - expected.0).abs() > POS_TOLERANCE || (pos.1 - expected.1).abs() > POS_TOLERANCE
        {
            return Err(at(format!(
                "off segment: pos {pos:?}, lerp(P({lo}), P({hi}), {fraction}) = {expected:?}"
            )));
        }
        if let Some(previous) = previous_t
            && t < previous
        {
            return Err(at(format!("t decreased from {previous}")));
        }
        previous_t = Some(t);
        lags.push(clock.continuous_tick(frame_ns).map_err(at)? - t);
    }
    if lags.is_empty() {
        return Err(format!("no frames for unit {unit_id}"));
    }
    lags.sort_by(f64::total_cmp);
    let p99_index = (0.99 * (lags.len() - 1) as f64).floor() as usize;
    Ok(OracleReport {
        frames: lags.len(),
        lag_p99: lags[p99_index],
        lag_max: lags[lags.len() - 1],
    })
}

/// The position of `unit_id` at the last tick of the log that holds it. Panics when the log
/// has no line for the unit.
pub fn final_position(ticks: &[TickRecord], unit_id: u32) -> (f32, f32) {
    ticks
        .iter()
        .filter(|record| record.unit_id == unit_id)
        .max_by_key(|record| record.tick)
        .unwrap_or_else(|| panic!("the tick log has no line for unit {unit_id}"))
        .pos
}
