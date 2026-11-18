//! Public physical passage state. Foreign intent never enters these inputs.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Throat {
    pub min: Vec2,
    pub max: Vec2,
    pub axis: Vec2,
    pub level: i32,
}
impl Throat {
    fn axial(self, p: Vec2) -> f32 {
        p.dot(self.axis)
    }
    fn ends(self) -> (f32, f32) {
        (self.axial(self.min), self.axial(self.max))
    }
    fn middle(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }
    fn half_width(self) -> f32 {
        let d = self.max - self.min;
        if self.axis.x != 0. {
            d.y * 0.5
        } else {
            d.x * 0.5
        }
    }
    fn side(self, p: Vec2) -> i8 {
        if self.axial(p) < self.axial(self.middle()) {
            -1
        } else {
            1
        }
    }
    fn inside(self, p: Vec2, r: f32) -> bool {
        let (a, b) = self.ends();
        let x = self.axial(p);
        x + r > a
            && x - r < b
            && (p - self.middle()).perp_dot(self.axis).abs() < self.half_width() + r
    }
    fn near(self, p: Vec2, r: f32) -> bool {
        let (a, b) = self.ends();
        let x = self.axial(p);
        x >= a - 8. * r
            && x <= b + 8. * r
            && (p - self.middle()).perp_dot(self.axis).abs() <= 8. * r + self.half_width()
    }
    fn mouth(self, side: i8, r: f32) -> Vec2 {
        let (a, b) = self.ends();
        let endpoint = if side < 0 {
            f64::from(a) - f64::from(r)
        } else {
            f64::from(b) + f64::from(r)
        };
        let mut x = endpoint as f32;
        if side < 0 && f64::from(x) + f64::from(r) > f64::from(a) {
            x = x.next_down();
        }
        if side > 0 && f64::from(x) - f64::from(r) < f64::from(b) {
            x = x.next_up();
        }
        if self.axis.x != 0. {
            Vec2::new(x, self.middle().y)
        } else {
            Vec2::new(self.middle().x, x)
        }
    }
    fn connector(self, side: i8, r: f32, goal: Vec2) -> Option<Vec2> {
        let (lo, hi, want) = if self.axis.x != 0. {
            (self.min.y, self.max.y, goal.y)
        } else {
            (self.min.x, self.max.x, goal.x)
        };
        let low = f64::from(lo) + f64::from(r);
        let high = f64::from(hi) - f64::from(r);
        if low > high {
            return None;
        }
        let mut normal = f64::from(want).clamp(low, high) as f32;
        if f64::from(normal) < low {
            normal = normal.next_up();
        }
        if f64::from(normal) > high {
            normal = normal.next_down();
        }
        if f64::from(normal) < low || f64::from(normal) > high {
            return None;
        }
        let mut point = self.mouth(side, r);
        if self.axis.x != 0. {
            point.y = normal;
        } else {
            point.x = normal;
        }
        Some(point)
    }
    fn entry(self, side: i8, r: f32, goal: Vec2, depth: u8) -> Option<Vec2> {
        // Keep the owned goal lane within the certified aperture. A centreline
        // stage can be blocked while a compatible body leaves a clear side leg.
        let mut point = self.connector(side, r, goal)?;
        let (a, b) = self.ends();
        let x = match depth {
            0 => {
                if side < 0 {
                    f64::from(a) + f64::from(r).min(f64::from(b - a) * 0.5)
                } else {
                    f64::from(b) - f64::from(r).min(f64::from(b - a) * 0.5)
                }
            }
            1 => {
                if side < 0 {
                    f64::from(a)
                } else {
                    f64::from(b)
                }
            }
            _ => {
                let m = self.axial(point);
                f64::from(if side < 0 { m.next_up() } else { m.next_down() })
            }
        } as f32;
        if self.axis.x != 0. {
            point.x = x;
        } else {
            point.y = x;
        }
        Some(point)
    }
    fn waiting_bay(
        self,
        side: i8,
        radius: f32,
        common: f32,
        axial: u8,
        lateral: u8,
        normal_side: i8,
    ) -> Vec2 {
        let (a, b) = self.ends();
        let outward = |value: f64, sign: i8| {
            let rounded = value as f32;
            if sign < 0 && f64::from(rounded) > value {
                rounded.next_down()
            } else if sign > 0 && f64::from(rounded) < value {
                rounded.next_up()
            } else {
                rounded
            }
        };
        let axial_clearance = f64::from(radius) + f64::from(common) * f64::from(axial);
        let x = outward(
            if side < 0 {
                f64::from(a) - axial_clearance
            } else {
                f64::from(b) + axial_clearance
            },
            side,
        );
        let (low, high) = if self.axis.x != 0. {
            (self.min.y, self.max.y)
        } else {
            (self.min.x, self.max.x)
        };
        let clearance = f64::from(radius) + f64::from(common) * f64::from(lateral);
        let y = outward(
            if normal_side < 0 {
                f64::from(low) - clearance
            } else {
                f64::from(high) + clearance
            },
            normal_side,
        );
        if self.axis.x != 0. {
            Vec2::new(x, y)
        } else {
            Vec2::new(y, x)
        }
    }
    fn outside_waiting_layer(self, point: Vec2, radius: f32, common: f32) -> bool {
        let (p, low, high) = if self.axis.x != 0. {
            (point.y, self.min.y, self.max.y)
        } else {
            (point.x, self.min.x, self.max.x)
        };
        f64::from(p) + f64::from(radius) <= f64::from(low) - f64::from(common)
            || f64::from(p) - f64::from(radius) >= f64::from(high) + f64::from(common)
    }
    fn away_leg(self, from: Vec2, to: Vec2, r: f32) -> bool {
        if self.away(from, to, r) {
            return true;
        }
        let (a, b, x, g) = if self.axis.x != 0. {
            (self.min.y, self.max.y, from.y, to.y)
        } else {
            (self.min.x, self.max.x, from.x, to.x)
        };
        let r = f64::from(r);
        (f64::from(x) + r <= f64::from(a) && f64::from(g) + r <= f64::from(a))
            || (f64::from(x) - r >= f64::from(b) && f64::from(g) - r >= f64::from(b))
    }
    fn away(self, from: Vec2, goal: Vec2, r: f32) -> bool {
        let (a, b) = self.ends();
        let x = f64::from(self.axial(from));
        let g = f64::from(self.axial(goal));
        let r = f64::from(r);
        (x + r <= f64::from(a) && g + r <= f64::from(a))
            || (x - r >= f64::from(b) && g - r >= f64::from(b))
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Open,
    Drain,
    Retry,
    Transit,
}
#[derive(Clone, Copy, Debug)]
struct Episode {
    mode: Mode,
    side: i8,
    stalled: u8,
    admitted: u8,
    tick: u32,
    level: i32,
    probe_radius: u32,
}
impl Episode {
    fn new(tick: u32, level: i32) -> Self {
        Self {
            mode: Mode::Open,
            side: -1,
            stalled: 0,
            admitted: 0,
            tick: tick.saturating_sub(1),
            level,
            probe_radius: 0,
        }
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "Passage witness fields stay explicit."
    )]
    fn update(
        &mut self,
        tick: u32,
        complete: bool,
        inside: [bool; 2],
        drained: bool,
        present: [bool; 2],
        progress: [bool; 2],
        new_entries: [u8; 2],
    ) {
        if !complete {
            self.mode = Mode::Drain;
            self.tick = tick;
            return;
        }
        if self.tick.checked_add(1) != Some(tick) && !(self.mode == Mode::Open && self.tick == tick)
        {
            self.mode = Mode::Drain;
        }
        self.tick = tick;
        let at = usize::from(self.side > 0);
        match self.mode {
            Mode::Open => {
                if inside == [true, true] {
                    self.mode = Mode::Drain;
                } else if inside[0] != inside[1] {
                    self.mode = Mode::Transit;
                    self.side = if inside[0] { -1 } else { 1 };
                    self.admitted = new_entries[usize::from(self.side > 0)].max(1);
                    self.stalled = 0;
                }
            }
            Mode::Drain => {
                if inside == [false, false] && drained {
                    self.mode = Mode::Retry;
                    self.side = -1;
                    self.stalled = 0;
                    self.admitted = 0;
                }
            }
            Mode::Retry => {
                if inside[at] {
                    self.mode = Mode::Transit;
                    self.admitted = self.admitted.saturating_add(new_entries[at]);
                    self.stalled = 0;
                } else if inside[1 - at] {
                    self.mode = Mode::Transit;
                    self.side = -self.side;
                    self.stalled = 0;
                } else {
                    // Empty is a witness, not a new episode. Preserve its side,
                    // progress reference and turn until an explicit transition.
                    self.stalled = if progress[at] {
                        0
                    } else {
                        self.stalled.saturating_add(1)
                    };
                    if self.stalled >= BATCH as u8 || (!present[at] && present[1 - at]) {
                        self.side = -self.side;
                        self.stalled = 0;
                        self.admitted = 0;
                    }
                }
            }
            Mode::Transit => {
                self.admitted = self.admitted.saturating_add(new_entries[at]);
                if !inside[at] {
                    if inside[1 - at] {
                        self.mode = Mode::Drain;
                        self.admitted = 0;
                    } else {
                        self.mode = Mode::Retry;
                        if self.admitted >= BATCH as u8 || (!present[at] && present[1 - at]) {
                            self.side = -self.side;
                            self.admitted = 0;
                        }
                        self.stalled = 0;
                    }
                }
            }
        }
    }
    fn yields(self, side: i8) -> bool {
        self.mode == Mode::Drain || (self.mode != Mode::Open && side != self.side)
    }
    fn yields_new_entry(self, side: i8, inside: bool) -> bool {
        self.yields(side)
            || (self.mode == Mode::Transit
                && side == self.side
                && !inside
                && usize::from(self.admitted) >= BATCH)
    }
}
// A current witnessed exit is departure through either mouth. Keep that fact
// through outside clipping/jitter; only a new physical entry consumes it.
fn departed(previous: Option<(bool, bool)>, inside: bool) -> bool {
    !inside && previous.is_some_and(|(was_inside, was_departed)| was_inside || was_departed)
}

#[derive(Clone, Copy, Debug)]
struct BodyStamp {
    pose: Vec2,
    radius: f32,
    tick: u32,
    side: i8,
    inside: bool,
    departed: bool,
    bay: Option<Vec2>,
    reference: f32,
    profile: [u32; 3],
    accepted: Option<Vec2>,
    entry: Option<EntranceStamp>,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Episode,
    Body(u32),
}
#[derive(Clone, Copy, Debug)]
enum Value {
    Owned,
    Body(BodyStamp),
    Episode(Episode),
}
#[derive(Debug, Default)]
pub(super) struct Rows(BTreeMap<Key, Value>);
impl Rows {
    pub(super) fn new() -> Self {
        Self::default()
    }
    pub(super) fn contains(&self, id: &u32) -> bool {
        matches!(self.0.get(&Key::Body(*id)), Some(Value::Owned))
    }
    pub(super) fn insert(&mut self, id: u32) -> bool {
        self.0.insert(Key::Body(id), Value::Owned).is_none()
    }
    pub(super) fn remove(&mut self, id: &u32) -> bool {
        self.contains(id) && self.0.remove(&Key::Body(*id)).is_some()
    }
    pub(super) fn len(&self) -> usize {
        self.0.len()
    }
    pub(super) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub(super) fn retain(&mut self, mut keep: impl FnMut(&u32) -> bool) {
        self.0.retain(|key, value| match (key, value) {
            (Key::Body(id), Value::Owned) => keep(id),
            _ => true,
        });
    }
    pub(super) fn public(&self) -> bool {
        self.0.contains_key(&Key::Episode)
    }
    pub(super) fn suspend(&mut self) {
        for value in self.0.values_mut() {
            match value {
                Value::Episode(episode) => episode.mode = Mode::Drain,
                Value::Body(row) => {
                    row.entry = None;
                    row.accepted = None;
                }
                Value::Owned => {}
            }
        }
    }
}
impl<const N: usize> From<[u32; N]> for Rows {
    fn from(ids: [u32; N]) -> Self {
        let mut rows = Self::new();
        for id in ids {
            rows.insert(id);
        }
        rows
    }
}
const _: () =
    assert!(11 * (std::mem::size_of::<Key>() + std::mem::size_of::<Value>()) + 128 <= 5 * 256);
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_age_exit_and_tiny_outside_motion_cannot_pin_the_old_passage() {
        assert!(
            departed(Some((true, false)), false),
            "unknown-age occupant exits through either mouth"
        );
        assert!(
            departed(Some((false, true)), false),
            "tiny clipped outside motion is not a new entry"
        );
        assert!(
            !departed(Some((false, true)), true),
            "actual committed reentry starts a new occupancy"
        );
        assert!(
            !departed(None, false),
            "missing or stale continuity cannot fabricate departure"
        );
    }
    #[test]
    fn fresh_tick_zero_does_not_fabricate_a_backout_episode() {
        let mut e = Episode::new(0, 0);
        e.update(0, true, [false; 2], false, [true; 2], [false; 2], [0; 2]);
        assert_eq!(e.mode, Mode::Open);
        assert!(!e.yields(-1) && !e.yields(1));
        e.update(1, true, [true, false], false, [true; 2], [false; 2], [1, 0]);
        assert_eq!(e.mode, Mode::Transit);
        assert!(!e.yields(-1) && e.yields(1));
        // A later opposed entrant cannot steal a still valid incumbent.
        e.update(2, true, [true, true], false, [true; 2], [false; 2], [0, 1]);
        assert_eq!((e.mode, e.side), (Mode::Transit, -1));
    }
    #[test]
    fn drain_does_not_regrant_when_only_one_backing_body_remains_inside() {
        let mut e = Episode::new(4, 0);
        e.mode = Mode::Drain;
        e.update(4, true, [false, true], false, [true; 2], [false; 2], [0; 2]);
        assert_eq!(e.mode, Mode::Drain);
        e.update(5, true, [false; 2], false, [true; 2], [false; 2], [0; 2]);
        assert_eq!(e.mode, Mode::Drain, "mouths are still occupied");
        e.update(6, true, [false; 2], true, [true; 2], [false; 2], [0; 2]);
        assert_eq!((e.mode, e.side), (Mode::Retry, -1));
    }
    #[test]
    fn repeated_empty_witnesses_do_not_pin_idle_first_side() {
        let mut e = Episode::new(0, 0);
        e.mode = Mode::Drain;
        e.update(0, true, [false; 2], true, [true; 2], [false; 2], [0; 2]);
        assert_eq!(e.side, -1);
        for tick in 1..4 {
            e.update(tick, true, [false; 2], true, [true; 2], [false; 2], [0; 2]);
            assert_eq!(e.side, -1);
        }
        e.update(4, true, [false; 2], true, [true; 2], [false; 2], [0; 2]);
        assert_eq!(e.side, 1);
        e.update(5, true, [false, true], false, [true; 2], [false; 2], [0, 1]);
        assert_eq!(e.mode, Mode::Transit);
        assert_eq!(e.side, 1);
    }
    #[test]
    fn unresolved_opposing_occupants_request_backout_until_empty() {
        let mut e = Episode::new(0, 0);
        e.update(0, true, [true, true], false, [true; 2], [false; 2], [0; 2]);
        assert!(e.yields(-1) && e.yields(1));
        e.update(1, true, [false; 2], true, [true; 2], [false; 2], [0; 2]);
        assert_eq!(e.mode, Mode::Retry);
        assert!(!e.yields(-1) && e.yields(1));
    }
    #[test]
    fn missing_current_witness_cannot_keep_historical_priority() {
        let mut e = Episode::new(0, 0);
        e.update(0, true, [true, false], false, [true; 2], [false; 2], [1, 0]);
        assert_eq!(e.side, -1);
        e.update(
            1,
            false,
            [true, false],
            false,
            [true; 2],
            [false; 2],
            [0; 2],
        );
        assert_eq!(e.mode, Mode::Drain);
        e.update(2, true, [true, true], false, [true; 2], [false; 2], [0; 2]);
        assert_eq!(e.mode, Mode::Drain);
    }
}

#[derive(Clone, Copy)]
struct PublicRow {
    id: u32,
    body: ObservedBody,
    profile: [u32; 3],
}
// The cooperative indexes have been retired. This complete small observation
// batch replaces the observed-detour result buffers; it is not another index.
const _: () = assert!(
    RETAINED_ALLOWANCE
        + INDEX_BODIES * PER_BODY_SCRATCH
        + FIXED_SCRATCH
        + EnvelopeIndex::capacity_bound(INDEX_BODIES)
        + (INDEX_BODIES + 2 * NEIGHBORS) * 20
        <= SCRATCH_BYTES
);
const _: () = assert!(
    NEIGHBORS
        * (std::mem::size_of::<PublicRow>()
            + 2 * std::mem::size_of::<Option<BodyStamp>>()
            + std::mem::size_of::<usize>()
            + std::mem::size_of::<bool>()
            + std::mem::size_of::<Retreat>()
            + std::mem::size_of::<u32>())
        + OWNER_GATES * std::mem::size_of::<(Throat, usize)>()
        <= FIXED_SCRATCH
);
/// Ephemeral authority for this unchanged captured frame only. The full public
/// body batch is already capped at NEIGHBORS; no retained per-unit map is added.
pub(crate) type CertifiedLegs = [bool; NEIGHBORS];
pub(crate) const fn no_certified_legs() -> CertifiedLegs {
    [false; NEIGHBORS]
}

fn same_resource(gate: &Gate, throat: Throat, owner: usize) -> bool {
    gate.admitted.public()
        && gate.reserved_by[owner]
        && gate.centre == throat.middle()
        && gate.axis == throat.axis
        && gate.half_length == (throat.ends().1 - throat.ends().0) * 0.5
        && gate.radius == throat.half_width()
        && matches!(gate.admitted.0.get(&Key::Episode), Some(Value::Episode(e)) if e.level == throat.level)
}
// Admission is complete-batch only. Retired metadata never wins capacity by ID.
fn room_for(gates: &[Gate], resource: Throat, owner: usize, rows: &[PublicRow]) -> bool {
    let gate = gates.iter().find(|g| same_resource(g, resource, owner));
    let extra = gate.map_or(rows.len() + 1, |gate| {
        rows.iter()
            .filter(|row| !gate.admitted.0.contains_key(&Key::Body(row.id)))
            .count()
    });
    let retained: usize = gates.iter().map(|g| g.admitted.len()).sum();
    let owned: usize = gates
        .iter()
        .filter(|g| g.reserved_by[owner])
        .map(|g| g.admitted.len())
        .sum();
    (gate.is_some()
        || gates.len() < GATES
            && gates.iter().filter(|g| g.reserved_by[owner]).count() < OWNER_GATES)
        && retained + extra <= GATES * BATCH
        && owned + extra <= OWNER_GATES * BATCH
}
// Called only after capacity refuses a complete CURRENT AllVisible batch.
// Reserve the entire bounded scan before touching history. A nonconsecutive
// observation has no active dated authority; live episode timers remain intact.
fn retire_inert(
    gates: &mut Vec<Gate>,
    owner: usize,
    tick: u32,
    rows: &[PublicRow],
    charge: &mut impl FnMut() -> bool,
) -> bool {
    if rows.len() > NEIGHBORS {
        return false;
    }
    let work = rows.len() + gates.len() + gates.iter().map(|g| g.admitted.len()).sum::<usize>();
    for _ in 0..work {
        if !charge() {
            return false;
        }
    }
    let mut ids = [0_u32; NEIGHBORS];
    for (to, row) in ids.iter_mut().zip(rows) {
        *to = row.id;
    }
    let ids = &mut ids[..rows.len()];
    ids.sort_unstable(); // Lookup handles only; all absent rows retire together.
    gates.retain_mut(|gate| {
        if !gate.reserved_by[owner] || !gate.admitted.public() {
            return true;
        }
        let Some(Value::Episode(episode)) = gate.admitted.0.get(&Key::Episode) else {
            return true;
        };
        if episode.tick != tick && episode.tick.checked_add(1) != Some(tick) {
            return false;
        }
        gate.admitted.0.retain(|key, _| match key {
            Key::Episode => true,
            Key::Body(id) => ids.binary_search(id).is_ok(),
        });
        true
    });
    true
}

fn owned_goal(unit: &MovementFrameUnit) -> Option<Vec2> {
    unit.target.map(|goal| {
        unit.plain_progress
            .as_ref()
            .map_or(goal, |p| p.original_point)
    })
}
fn carrier_order(
    a: &MovementFrameUnit,
    sa: BodyStamp,
    b: &MovementFrameUnit,
    sb: BodyStamp,
    resource: Throat,
) -> Ordering {
    sb.inside
        .cmp(&sa.inside)
        .then_with(|| {
            if sa.inside && sb.inside {
                if let Some(order) = sa
                    .entry
                    .zip(sb.entry)
                    .and_then(|(x, y)| {
                        if x.tick != y.tick {
                            Some(x.tick.cmp(&y.tick))
                        } else {
                            contact_order(x.contact(), y.contact())
                        }
                    })
                    .filter(|o| *o != Ordering::Equal)
                {
                    return order;
                }
                (resource.axial(a.position) * f32::from(sa.side))
                    .total_cmp(&(resource.axial(b.position) * f32::from(sb.side)))
            } else {
                a.position
                    .as_dvec2()
                    .distance_squared(resource.mouth(sa.side, a.radius).as_dvec2())
                    .total_cmp(
                        &b.position
                            .as_dvec2()
                            .distance_squared(resource.mouth(sb.side, b.radius).as_dvec2()),
                    )
            }
        })
        .then_with(|| point_cmp(a.position, b.position))
        .then(a.radius.total_cmp(&b.radius))
}
fn incompatible(a: f32, b: f32, width: f64) -> bool {
    2. * (f64::from(a) + f64::from(b)) > width
}

fn exact_leg(
    from: Vec2,
    to: Vec2,
    unit: &MovementFrameUnit,
    rows: &[PublicRow],
    charge: &mut impl FnMut() -> bool,
    checks: &mut usize,
) -> Option<bool> {
    use turts_rules::terrain::geometry::{Budget, Point, swept_disks_clear};
    let p = |v: Vec2| Point {
        x: f64::from(v.x),
        y: f64::from(v.y),
    };
    for row in rows {
        if !charge() {
            return None;
        }
        if row.id == unit.unit_id {
            continue;
        }
        *checks += 1;
        if !swept_disks_clear(
            p(from),
            p(to),
            f64::from(unit.radius),
            p(row.body.position),
            p(row.body.position),
            f64::from(row.body.radius),
            &mut Budget::new(1),
        )
        .unwrap_or(false)
        {
            return Some(false);
        }
    }
    Some(true)
}
/// Owned retreat endpoints must not cut another selected retreat leg. These
/// temporary proposals never assume that a current body has already moved.
#[derive(Clone, Copy)]
struct Retreat {
    from: Vec2,
    to: Vec2,
    radius: f32,
}
fn retreats_clear(
    candidate: Retreat,
    prior: &[Retreat],
    charge: &mut impl FnMut() -> bool,
    checks: &mut usize,
) -> Option<bool> {
    use turts_rules::terrain::geometry::{Budget, Point, swept_disks_clear};
    let p = |v: Vec2| Point {
        x: f64::from(v.x),
        y: f64::from(v.y),
    };
    for old in prior {
        for (from, to, radius, centre, other_radius) in [
            (
                candidate.from,
                candidate.to,
                candidate.radius,
                old.to,
                old.radius,
            ),
            (old.from, old.to, old.radius, candidate.to, candidate.radius),
        ] {
            if !charge() {
                return None;
            }
            *checks += 1;
            if !swept_disks_clear(
                p(from),
                p(to),
                f64::from(radius),
                p(centre),
                p(centre),
                f64::from(other_radius),
                &mut Budget::new(1),
            )
            .unwrap_or(false)
            {
                return Some(false);
            }
        }
    }
    Some(true)
}

/// CURRENT physical observation adapter. A complete public body port is
/// mandatory for an empty-throat certificate. Restricted/incomplete captures
/// cannot infer departure from absence. No foreign movement-frame row is read.
#[allow(
    clippy::too_many_arguments,
    reason = "Explicit public observation and proof ports"
)]
pub(crate) fn apply(
    frame: &MovementFrame,
    proposed: &mut MovementFrame,
    certified: &mut CertifiedLegs,
    state: &mut TrafficState,
    waiting: &mut BTreeSet<u32>,
    tick: u32,
    dt: f32,
    budget: &mut crate::movement::sweep::SweepBudget,
    complete: bool,
    mut active: impl FnMut(usize) -> bool,
    mut may_enter: impl FnMut(usize) -> bool,
    mut foreign: impl FnMut(u8, &mut dyn FnMut(u32, ObservedBody) -> bool) -> bool,
    mut public_profile: impl FnMut(u8, u32) -> Option<[u32; 3]>,
    mut throat: impl FnMut(usize, f32, &mut dyn FnMut() -> bool) -> Option<Throat>,
    mut contact: impl FnMut(usize, u32, Vec2, Vec2, f32, f32, Vec2) -> EntranceContact,
    mut clear: impl FnMut(usize, Vec2, Vec2) -> bool,
) {
    *certified = no_certified_legs();
    // A complete AllVisible body capture cannot fit the existing 128-row
    // lane when its own physical frame is larger. Refuse the whole service;
    // no eligible suffix can silently lose its certified marker.
    if !complete || !dt.is_finite() || dt <= 0. || frame.units.len() > NEIGHBORS {
        return;
    }
    for owner in 0..SLOTS {
        let viewer = owner as u8;
        let mut rows = Vec::with_capacity(NEIGHBORS);
        let mut capture = |id, body: ObservedBody| {
            if state.tick_owner_neighbors[owner] >= OWNER_VISITS - GUIDED_RESERVE {
                return false;
            }
            state.tick_owner_neighbors[owner] += 1;
            state.tick_neighbors += 1;
            state.neighbor_visits += 1;
            if rows.len() == NEIGHBORS
                || !body.position.is_finite()
                || !body.radius.is_finite()
                || body.radius <= 0.
            {
                return false;
            }
            let Some(profile) = public_profile(viewer, id) else {
                return false;
            };
            rows.push(PublicRow { id, body, profile });
            true
        };
        let mut captured = true;
        for unit in frame
            .units
            .iter()
            .filter(|u| usize::from(u.owner_slot) == owner)
        {
            if !capture(
                unit.unit_id,
                ObservedBody {
                    position: unit.position,
                    radius: unit.radius,
                },
            ) {
                captured = false;
                break;
            }
        }
        if !captured || !foreign(viewer, &mut capture) {
            continue;
        }
        rows.sort_by(|a, b| {
            point_cmp(a.body.position, b.body.position)
                .then_with(|| a.body.radius.total_cmp(&b.body.radius))
        });
        let mut own_order: Vec<_> = frame
            .units
            .iter()
            .enumerate()
            .filter(|(i, u)| {
                usize::from(u.owner_slot) == owner
                    && moving(u)
                    && active(*i)
                    && state.parking_slot(u.unit_id) != Some(u.position)
            })
            .map(|(i, _)| i)
            .collect();
        own_order.sort_by(|&a, &b| physical(&frame.units[a], &frame.units[b]));
        let mut resources = Vec::<(Throat, usize)>::with_capacity(OWNER_GATES);
        for &index in &own_order {
            let unit = &frame.units[index];
            let Some(member) = state.members.get_mut(&unit.unit_id) else {
                continue;
            };
            let mut charge = || budget.charge_guidance_pair(viewer, &mut member.guidance_pairs);
            let mut physical_radius = unit.radius;
            let mut complete_radius = true;
            for row in &rows {
                if !charge() {
                    complete_radius = false;
                    break;
                }
                let reach = 8. * (f64::from(unit.radius) + f64::from(row.body.radius));
                if f32::from_bits(row.profile[1]) > 0.
                    && unit
                        .position
                        .as_dvec2()
                        .distance_squared(row.body.position.as_dvec2())
                        <= reach * reach
                {
                    physical_radius = physical_radius.max(row.body.radius);
                }
            }
            if !complete_radius {
                continue;
            }
            let Some(resource) = throat(index, physical_radius, &mut charge) else {
                continue;
            };
            if resources.iter().any(|(r, _)| *r == resource) {
                continue;
            }
            if resources.len() == OWNER_GATES {
                break;
            }
            resources.push((resource, index));
        }
        resources.sort_by(|(a, _), (b, _)| {
            point_cmp(a.min, b.min).then_with(|| point_cmp(a.max, b.max))
        });
        // This is a complete current observation. Retiring a resource requires
        // current positive geometry; an unknown/disappeared witness suspends it.
        for (resource, witness_index) in resources {
            // The original positive discovery supplies its charged owned
            // witness, including after a smaller body reaches a common bay.
            // Probe size below still comes only from CURRENT public bodies.
            let Some(witness) = state.members.get_mut(&frame.units[witness_index].unit_id) else {
                continue;
            };
            let mut probe_radius = 0_f32;
            let mut admitted_work = true;
            for row in &rows {
                if !budget.charge_guidance_pair(viewer, &mut witness.guidance_pairs) {
                    admitted_work = false;
                    break;
                }
                if f32::from_bits(row.profile[1]) > 0.
                    && resource.near(row.body.position, row.body.radius)
                {
                    probe_radius = probe_radius.max(row.body.radius);
                }
            }
            let width = 2. * f64::from(resource.half_width());
            if !admitted_work
                || probe_radius <= 0.
                || width < 2. * f64::from(probe_radius)
                || width >= 4. * f64::from(probe_radius)
            {
                continue;
            }
            if !room_for(&state.gates, resource, owner, &rows) {
                let mut charge =
                    || budget.charge_guidance_pair(viewer, &mut witness.guidance_pairs);
                if !retire_inert(&mut state.gates, owner, tick, &rows, &mut charge)
                    || !room_for(&state.gates, resource, owner, &rows)
                {
                    continue;
                }
            }
            let mut gate_index = state
                .gates
                .iter()
                .position(|g| same_resource(g, resource, owner));
            if gate_index.is_none() {
                let mut admitted = Rows::new();
                admitted.0.insert(
                    Key::Episode,
                    Value::Episode(Episode::new(tick, resource.level)),
                );
                let mut reserved_by = [false; SLOTS];
                reserved_by[owner] = true;
                state.gates.push(Gate {
                    centre: resource.middle(),
                    axis: resource.axis,
                    half_length: (resource.ends().1 - resource.ends().0) * 0.5,
                    radius: resource.half_width(),
                    direction: 0.,
                    admitted,
                    reserved_by,
                    provisional_for: None,
                    issued: 0,
                    blocked_contact: None,
                });
                gate_index = Some(state.gates.len() - 1);
            }
            let gate_index = gate_index.unwrap();
            // Capacity was admitted before creating or replacing any live rows.
            let mut inside = [false; 2];
            let mut present = [false; 2];
            let mut progress = [false; 2];
            let mut entries = [0u8; 2];
            let gate = &mut state.gates[gate_index];
            let Some(Value::Episode(mut episode)) = gate.admitted.0.get(&Key::Episode).copied()
            else {
                continue;
            };
            let before = episode;
            let same_probe =
                episode.probe_radius == 0 || episode.probe_radius == probe_radius.to_bits();
            if !same_probe {
                episode.mode = Mode::Drain;
            }
            episode.probe_radius = probe_radius.to_bits();
            let mut discontinuity = false;
            let mut role_complete = true;
            let mut pending = Vec::with_capacity(rows.len());
            let mut inside_radius = [0_f32; 2];
            for row in &rows {
                let prior = match gate.admitted.0.get(&Key::Body(row.id)) {
                    Some(Value::Body(body)) => Some(*body),
                    _ => None,
                };
                let consecutive = prior.filter(|p| {
                    same_probe
                        && p.tick.checked_add(1) == Some(tick)
                        && p.radius.to_bits() == row.body.radius.to_bits()
                        && p.profile == row.profile
                        && p.accepted == Some(row.body.position)
                        && f64::from(p.pose.distance(row.body.position))
                            <= f64::from(f32::from_bits(row.profile[1])) * f64::from(dt)
                                + 4. * [
                                    p.pose.x,
                                    p.pose.y,
                                    row.body.position.x,
                                    row.body.position.y,
                                ]
                                .into_iter()
                                .map(|x| f64::from(x.abs().next_up()) - f64::from(x.abs()))
                                .sum::<f64>()
                });
                discontinuity |= prior.is_some_and(|p| p.inside) && consecutive.is_none();
                let is_inside = if resource.inside(row.body.position, row.body.radius) {
                    if !budget.charge_guidance_pair(viewer, &mut witness.guidance_pairs) {
                        role_complete = false;
                        break;
                    }
                    state.detour_work.terrain_calls += 1;
                    match contact(
                        witness_index,
                        row.id,
                        row.body.position,
                        row.body.position,
                        row.body.radius,
                        probe_radius,
                        resource.axis,
                    ) {
                        EntranceContact::Inside => true,
                        EntranceContact::Outside => {
                            // Dual support describes the entrance collar, not
                            // occupancy throughout a certified rectangular floor.
                            // Losing one flank probe inside must not publish a
                            // departure or admit an incompatible follower.
                            row.body.position.x > resource.min.x
                                && row.body.position.x < resource.max.x
                                && row.body.position.y > resource.min.y
                                && row.body.position.y < resource.max.y
                        }
                        _ => {
                            role_complete = false;
                            break;
                        }
                    }
                } else {
                    false
                };
                // A missing continuity-generation port never creates a dated
                // entry. Prior outside-side is only geometric backout advice.
                let side = if is_inside {
                    consecutive.map_or(resource.side(row.body.position), |p| p.side)
                } else {
                    resource.side(row.body.position)
                };
                let at = usize::from(side > 0);
                let departed = departed(consecutive.map(|p| (p.inside, p.departed)), is_inside);
                inside[at] |= is_inside;
                if is_inside {
                    inside_radius[at] = inside_radius[at].max(row.body.radius);
                }
                // Outside bodies remain possible contenders. Four-stall retry
                // turns resolve an idle side without reading its private intent.
                present[at] |= resource.near(row.body.position, row.body.radius.max(probe_radius));
                let distance = row
                    .body
                    .position
                    .distance(resource.mouth(side, row.body.radius));
                let mut reference = consecutive.map_or(distance, |p| p.reference);
                // This is geometric progress toward a fixed public mouth. The
                // authored public speed supplies the ordinary progress threshold;
                // private desired velocity/activity never participates. This does
                // not create a dated entry or external-continuity witness.
                let threshold = (f32::from_bits(row.profile[1]) * dt * 0.01).max(f32::EPSILON);
                if consecutive.is_some() && reference - distance >= threshold {
                    progress[at] = true;
                    reference = distance;
                }
                if is_inside && consecutive.is_some_and(|p| !p.inside) {
                    entries[at] = entries[at].saturating_add(1);
                }
                let entry = if is_inside {
                    consecutive.and_then(|p| {
                        p.entry.or_else(|| {
                            if p.inside
                                || !budget.charge_guidance_pair(viewer, &mut witness.guidance_pairs)
                            {
                                return None;
                            }
                            state.detour_work.terrain_calls += 1;
                            let observed = contact(
                                witness_index,
                                row.id,
                                p.pose,
                                row.body.position,
                                row.body.radius,
                                probe_radius,
                                resource.axis,
                            );
                            let mut stamp = EntranceStamp::new(p.tick, p.pose, observed)?;
                            if stamp.inside_at_start {
                                return None;
                            }
                            if stamp.lower == stamp.upper {
                                let fraction =
                                    f64::from(stamp.lower) / f64::from(stamp.denominator);
                                stamp.point = (p.pose.as_dvec2()
                                    + (row.body.position.as_dvec2() - p.pose.as_dvec2())
                                        * fraction)
                                    .as_vec2();
                            }
                            Some(stamp)
                        })
                    })
                } else {
                    None
                };
                pending.push((
                    row.id,
                    BodyStamp {
                        pose: row.body.position,
                        radius: row.body.radius,
                        tick,
                        side,
                        inside: is_inside,
                        departed,
                        bay: consecutive.and_then(|p| p.bay),
                        reference,
                        profile: row.profile,
                        accepted: None,
                        entry,
                    },
                ));
            }
            if !role_complete {
                continue;
            }
            for &(id, body) in &pending {
                gate.admitted.0.insert(Key::Body(id), Value::Body(body));
            }
            if discontinuity {
                episode.mode = Mode::Drain;
            }
            if episode.mode == Mode::Open && inside == [true, true] {
                let stamps = rows
                    .iter()
                    .map(|row| match gate.admitted.0.get(&Key::Body(row.id)) {
                        Some(Value::Body(body)) => Some(*body),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let cross = stamps
                    .iter()
                    .flatten()
                    .filter(|p| p.inside && p.side < 0)
                    .count()
                    * stamps
                        .iter()
                        .flatten()
                        .filter(|p| p.inside && p.side > 0)
                        .count();
                let mut finished = true;
                for _ in 0..cross {
                    if !budget.charge_guidance_pair(viewer, &mut witness.guidance_pairs) {
                        finished = false;
                        break;
                    }
                }
                if !finished {
                    continue;
                }
                let mut possible = [false; NEIGHBORS];
                for (i, row) in stamps.iter().enumerate() {
                    possible[i] = row.is_some_and(|r| r.inside && r.entry.is_some());
                }
                // Admit the complete comparison program before selecting any
                // winner. Every opposing pair is evaluated, even after a veto.
                for (i, a) in stamps.iter().enumerate() {
                    let Some(a) = a.filter(|p| p.inside) else {
                        continue;
                    };
                    for (j, b) in stamps.iter().enumerate().skip(i + 1) {
                        let Some(b) = b.filter(|p| p.inside && p.side != a.side) else {
                            continue;
                        };
                        let order = a.entry.zip(b.entry).and_then(|(ae, be)| {
                            if ae.tick != be.tick {
                                return Some(ae.tick.cmp(&be.tick));
                            }
                            contact_order(ae.contact(), be.contact()).map(|o| {
                                if o == Ordering::Equal {
                                    point_cmp(ae.point, be.point)
                                        .then(a.radius.total_cmp(&b.radius))
                                } else {
                                    o
                                }
                            })
                        });
                        match order {
                            Some(Ordering::Less) => possible[j] = false,
                            Some(Ordering::Greater) => possible[i] = false,
                            _ => {
                                possible[i] = false;
                                possible[j] = false;
                            }
                        }
                    }
                }
                let mut sides = [false; 2];
                for (i, row) in stamps.iter().enumerate() {
                    if possible[i] {
                        sides[usize::from(row.unwrap().side > 0)] = true;
                    }
                }
                if sides[0] != sides[1] {
                    episode.mode = Mode::Transit;
                    episode.side = if sides[0] { -1 } else { 1 };
                }
            }
            // An empty throat alone cannot release a mutual backout while a
            // body still occupies either straight exit strip. These current
            // public positions also prove the outside waiting space was reached.
            let drained = !rows.iter().any(|row| {
                let p = row.body.position;
                let r = row.body.radius;
                let (a, b) = resource.ends();
                let x = resource.axial(p);
                x > a - 3. * r
                    && x < b + 3. * r
                    && (p - resource.middle()).perp_dot(resource.axis).abs()
                        < resource.half_width() + r
            });
            episode.update(tick, true, inside, drained, present, progress, entries);
            gate.admitted.0.retain(|key, _| match key {
                Key::Episode => true,
                Key::Body(id) => rows.iter().any(|row| row.id == *id),
            });
            if before.side != episode.side
                || (before.mode == Mode::Drain && episode.mode == Mode::Retry)
            {
                for value in gate.admitted.0.values_mut() {
                    if let Value::Body(stamp) = value {
                        stamp.reference = stamp
                            .pose
                            .distance(resource.mouth(stamp.side, stamp.radius));
                    }
                }
            }
            gate.admitted
                .0
                .insert(Key::Episode, Value::Episode(episode));
            let mut entry_permitted = vec![false; frame.units.len().min(INDEX_BODIES)];
            let mut carriers = [None::<usize>; 2];
            let mut complete_carriers = true;
            for &index in &own_order {
                if !budget.charge_guidance_pair(viewer, &mut witness.guidance_pairs) {
                    complete_carriers = false;
                    break;
                }
                let unit = &frame.units[index];
                if !resource.near(unit.position, unit.radius.max(probe_radius))
                    || owned_goal(unit)
                        .is_none_or(|goal| resource.away(unit.position, goal, unit.radius))
                {
                    // A private outside leg does not remove a body from the
                    // service order. Otherwise two bodies at the same mouth
                    // distance exchange the lead each tick and send each
                    // other back to a waiting bay.
                    continue;
                }
                let Some(Value::Body(stamp)) = state.gates[gate_index]
                    .admitted
                    .0
                    .get(&Key::Body(unit.unit_id))
                    .copied()
                else {
                    continue;
                };
                let permitted = may_enter(index);
                entry_permitted[index] = permitted;
                if !stamp.inside && !permitted {
                    continue;
                }
                let at = usize::from(stamp.side > 0);
                if carriers[at].is_none_or(|old| {
                    let Some(Value::Body(prior)) = state.gates[gate_index]
                        .admitted
                        .0
                        .get(&Key::Body(frame.units[old].unit_id))
                        .copied()
                    else {
                        return true;
                    };
                    carrier_order(unit, stamp, &frame.units[old], prior, resource) == Ordering::Less
                }) {
                    carriers[at] = Some(index);
                }
            }
            if !complete_carriers {
                continue;
            }
            // Geometric service order is already fixed above. Reserve only
            // this owner's selected legs, never a foreign private destination.
            let mut retreats = Vec::<Retreat>::with_capacity(own_order.len());
            for &index in &own_order {
                let unit = &frame.units[index];
                if !resource.near(unit.position, unit.radius.max(probe_radius)) {
                    continue;
                }
                let Some(Value::Body(stamp)) = state.gates[gate_index]
                    .admitted
                    .0
                    .get(&Key::Body(unit.unit_id))
                    .copied()
                else {
                    continue;
                };
                let side = stamp.side;
                let Some(goal) = owned_goal(unit) else {
                    continue;
                };
                let at = usize::from(side > 0);
                let own_follower = !stamp.inside
                    && carriers[at].is_some_and(|lead| {
                        lead != index && incompatible(unit.radius, frame.units[lead].radius, width)
                    });
                let occupied = !stamp.inside
                    && inside_radius[at] > 0.
                    && incompatible(unit.radius, inside_radius[at], width);
                let yielding =
                    episode.yields_new_entry(side, stamp.inside) || own_follower || occupied;
                // Original exterior destinations keep their own controller.
                // A temporary outside leg cannot overrule an owed public
                // backout: its endpoint can still block the winner's exit.
                if resource.away(unit.position, goal, unit.radius)
                    || (state.gates[gate_index]
                        .admitted
                        .0
                        .get(&Key::Body(unit.unit_id))
                        .is_some_and(|row| {
                            matches!(row, Value::Body(body)
                            if body.bay.is_none() && !yielding)
                        })
                        && proposed.units[index].target.is_some_and(|point| {
                            resource.away_leg(unit.position, point, unit.radius)
                        }))
                {
                    if let Some(Value::Body(row)) = state.gates[gate_index]
                        .admitted
                        .0
                        .get_mut(&Key::Body(unit.unit_id))
                    {
                        row.bay = None;
                    }
                    continue;
                }
                let Some(member) = state.members.get_mut(&unit.unit_id) else {
                    continue;
                };
                let mut charge = || budget.charge_guidance_pair(viewer, &mut member.guidance_pairs);
                let mut selected = None;
                let mut body_checks = 0;
                let mut terrain_calls = 0;
                if exact_leg(
                    unit.position,
                    unit.position,
                    unit,
                    &rows,
                    &mut charge,
                    &mut body_checks,
                ) != Some(true)
                {
                    state.detour_work.body_checks += body_checks;
                    continue;
                }
                member.parking = None;
                member.parking_retained = false;
                if !yielding {
                    if !entry_permitted[index] {
                        continue;
                    }
                    let exit = resource.side(goal);
                    let (a, b) = resource.ends();
                    let goal_inside = resource.axial(goal) >= a && resource.axial(goal) <= b;
                    let candidates = [
                        goal_inside.then_some(goal),
                        (!goal_inside)
                            .then(|| resource.connector(exit, unit.radius, goal))
                            .flatten(),
                        // A collar occupant may need to align at the near
                        // mouth before its full exit chord clears a corner.
                        // Every leg still needs complete current certificates.
                        resource.entry(side, unit.radius, goal, 0),
                        resource.entry(side, unit.radius, goal, 1),
                        resource.entry(side, unit.radius, goal, 2),
                    ];
                    for connector in candidates.into_iter().flatten() {
                        state.detour_work.candidates += 1;
                        if connector.x < unit.radius
                            || connector.y < unit.radius
                            || connector.x > frame.bounds.x - unit.radius
                            || connector.y > frame.bounds.y - unit.radius
                        {
                            continue;
                        }
                        match exact_leg(
                            unit.position,
                            connector,
                            unit,
                            &rows,
                            &mut charge,
                            &mut body_checks,
                        ) {
                            None => break,
                            Some(false) => continue,
                            Some(true) => {}
                        }
                        if !charge() {
                            break;
                        }
                        terrain_calls += 1;
                        if clear(index, unit.position, connector) {
                            selected = Some(connector);
                            break;
                        }
                    }
                    state.detour_work.body_checks += body_checks;
                    state.detour_work.terrain_calls += terrain_calls;
                    if let Some(Value::Body(row)) = state.gates[gate_index]
                        .admitted
                        .0
                        .get_mut(&Key::Body(unit.unit_id))
                    {
                        row.bay = None;
                    }
                    if let Some(connector) = selected {
                        // The original destination keeps its existing arrival
                        // path. Only a private connector gets nonterminal pinning.
                        certified[index] = connector != goal;
                        proposed.units[index].target = Some(connector);
                        proposed.units[index].speed = unit.speed;
                        waiting.remove(&unit.unit_id);
                    } else {
                        certified[index] = false;
                        stopped(proposed, index);
                        waiting.insert(unit.unit_id);
                    }
                    continue;
                }
                // Retained geometry-derived endpoint first. Every tick recertifies
                // its complete leg; tiny clipping cannot silently replace it.
                let retained = stamp
                    .bay
                    .filter(|bay| resource.outside_waiting_layer(*bay, unit.radius, probe_radius));
                let candidates = std::iter::once(retained).chain((0..32).map(|n| {
                    // Outside bodies may retreat along the wall before
                    // crossing another waiting row. Normal clearance alone
                    // keeps this endpoint outside the common collision layer.
                    let axial = n / 8 + u8::from(resource.inside(unit.position, unit.radius));
                    let lateral = 1 + (n % 8) / 2;
                    let normal_side = if n % 2 == 0 { -1 } else { 1 };
                    let mut point = resource.waiting_bay(
                        side,
                        unit.radius,
                        probe_radius,
                        axial,
                        lateral,
                        normal_side,
                    );
                    if resource.inside(unit.position, unit.radius) {
                        // Only four distinct axial exits. Repeating each eight
                        // times spends proof credit without adding a candidate.
                        if n % 8 != 0 {
                            return None;
                        }
                        if resource.axis.x != 0. {
                            point.y = unit.position.y;
                        } else {
                            point.x = unit.position.x;
                        }
                    }
                    Some(point)
                }));
                for bay in candidates.flatten() {
                    state.detour_work.candidates += 1;
                    if !bay.is_finite()
                        || (bay.x < unit.radius
                            || bay.y < unit.radius
                            || bay.x > frame.bounds.x - unit.radius
                            || bay.y > frame.bounds.y - unit.radius)
                    {
                        continue;
                    }
                    match exact_leg(
                        unit.position,
                        bay,
                        unit,
                        &rows,
                        &mut charge,
                        &mut body_checks,
                    ) {
                        None => break,
                        Some(false) => continue,
                        Some(true) => {}
                    }
                    if !charge() {
                        break;
                    }
                    match retreats_clear(
                        Retreat {
                            from: unit.position,
                            to: bay,
                            radius: unit.radius,
                        },
                        &retreats,
                        &mut charge,
                        &mut body_checks,
                    ) {
                        None => break,
                        Some(false) => continue,
                        Some(true) => {}
                    }
                    terrain_calls += 1;
                    if clear(index, unit.position, bay) {
                        selected = Some(bay);
                        break;
                    }
                }
                state.detour_work.body_checks += body_checks;
                state.detour_work.terrain_calls += terrain_calls;
                if let Some(bay) = selected {
                    retreats.push(Retreat {
                        from: unit.position,
                        to: bay,
                        radius: unit.radius,
                    });
                    certified[index] = bay != goal;
                    if let Some(Value::Body(row)) = state.gates[gate_index]
                        .admitted
                        .0
                        .get_mut(&Key::Body(unit.unit_id))
                    {
                        row.bay = Some(bay);
                    }
                    proposed.units[index].target = Some(bay);
                    proposed.units[index].speed = unit.speed;
                    waiting.remove(&unit.unit_id);
                } else {
                    certified[index] = false;
                    stopped(proposed, index);
                    waiting.insert(unit.unit_id);
                }
            }
        }
    }
}

/// Publish only final certified physical chords to rows already observed by the
/// complete public port. This creates no foreign intent or new admission rows.
pub(crate) fn publish(
    state: &mut TrafficState,
    frame: &MovementFrame,
    outputs: &[crate::movement::MovementFrameOutput],
    tick: u32,
) {
    for owner in 0..SLOTS {
        let count = state
            .gates
            .iter()
            .filter(|g| g.admitted.public() && g.reserved_by[owner])
            .map(|g| g.admitted.len().saturating_sub(1))
            .sum::<usize>();
        let work = count
            .checked_mul(frame.units.len())
            .zip(count.checked_mul(outputs.len()))
            .and_then(|(a, b)| a.checked_add(b));
        let admitted = frame.units.len() <= NEIGHBORS
            && outputs.len() <= NEIGHBORS
            && work.is_some_and(|n| {
                state.tick_owner_neighbors[owner].saturating_add(n) <= OWNER_VISITS - GUIDED_RESERVE
            });
        if admitted {
            let n = work.unwrap();
            state.tick_owner_neighbors[owner] += n;
            state.tick_neighbors += n;
            state.neighbor_visits += n as u64;
        }
        for gate in &mut state.gates {
            if !gate.admitted.public() || !gate.reserved_by[owner] {
                continue;
            }
            for (key, value) in &mut gate.admitted.0 {
                let (Key::Body(id), Value::Body(row)) = (key, value) else {
                    continue;
                };
                row.accepted = None;
                if !admitted || row.tick != tick {
                    continue;
                }
                let mut bound = false;
                for unit in &frame.units {
                    if unit.unit_id == *id {
                        bound = row.pose == unit.position
                            && row.radius.to_bits() == unit.radius.to_bits()
                            && row.profile[0] == unit.unit_type
                            && row.profile[2] == u32::from(unit.owner_slot);
                    }
                }
                for output in outputs {
                    if bound && output.unit_id == *id {
                        row.accepted = Some(output.position);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn retry_state(state: &TrafficState, owner: usize) -> Option<(i8, u8, u32)> {
    state
        .gates
        .iter()
        .filter(|g| g.reserved_by[owner])
        .find_map(|gate| match gate.admitted.0.get(&Key::Episode) {
            Some(Value::Episode(e)) if e.mode == Mode::Retry => Some((e.side, e.stalled, e.tick)),
            _ => None,
        })
}

#[cfg(test)]
#[allow(
    clippy::excessive_precision,
    reason = "Captures retain their original decimal form for reproducible geometry."
)]
mod retention_tests {
    use super::*;
    fn unit() -> MovementFrameUnit {
        MovementFrameUnit {
            unit_id: 7,
            owner_slot: 0,
            unit_type: 9,
            position: Vec2::new(31., 23.5),
            radius: 0.5,
            speed: Some(3.),
            target: Some(Vec2::new(48., 23.5)),
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }
    }
    fn state() -> TrafficState {
        let u = unit();
        let mut rows = Rows::new();
        rows.0.insert(
            Key::Episode,
            Value::Episode(Episode {
                mode: Mode::Transit,
                side: -1,
                stalled: 0,
                admitted: 1,
                tick: 10,
                level: 0,
                probe_radius: 0,
            }),
        );
        rows.0.insert(
            Key::Body(u.unit_id),
            Value::Body(BodyStamp {
                pose: u.position,
                radius: u.radius,
                tick: 10,
                side: -1,
                inside: true,
                departed: false,
                bay: None,
                reference: 0.,
                profile: [u.unit_type, 3_f32.to_bits(), 0],
                accepted: Some(u.position),
                entry: EntranceStamp::new(
                    9,
                    Vec2::new(29., 23.5),
                    EntranceContact::Transition {
                        lower: 1,
                        upper: 1,
                        denominator: 2,
                    },
                ),
            }),
        );
        let mut state = TrafficState::default();
        state.gates.push(Gate {
            centre: Vec2::new(32., 23.5),
            axis: Vec2::X,
            half_length: 2.,
            radius: 1.5,
            direction: 0.,
            admitted: rows,
            reserved_by: [true, false],
            provisional_for: None,
            issued: 0,
            blocked_contact: None,
        });
        state
    }
    fn body(state: &TrafficState) -> BodyStamp {
        match state.gates[0].admitted.0[&Key::Body(7)] {
            Value::Body(row) => row,
            _ => panic!("public body"),
        }
    }
    #[test]
    fn own_inside_leader_overrules_a_followers_private_outside_waypoint() {
        // Actual final opposing-world poses. Only the public adapter seam is
        // exercised; no missing Guide or prior dated entry is reconstructed.
        let follower = MovementFrameUnit {
            unit_id: 5,
            unit_type: 7,
            speed: Some(3.5),
            position: Vec2::new(29.599998, 21.75002),
            radius: 0.4,
            target: Some(Vec2::new(48., 23.5)),
            ..unit()
        };
        let leader = MovementFrameUnit {
            unit_id: 8,
            unit_type: 0,
            speed: Some(2.5),
            position: Vec2::new(28.900002, 23.1),
            radius: 1.1,
            target: Some(Vec2::new(44.55, 21.508142)),
            ..unit()
        };
        let resource = Throat {
            min: Vec2::new(30., 22.),
            max: Vec2::new(34., 25.),
            axis: Vec2::X,
            level: 0,
        };
        let frame = MovementFrame {
            bounds: Vec2::new(64., 48.),
            tick_hz: 20,
            pools: BTreeMap::new(),
            units: vec![follower.clone(), leader.clone()],
        };
        let mut proposed = frame.clone();
        let outside = Vec2::new(29.5, 23.0);
        proposed.units[0].target = Some(outside);
        assert!(resource.away_leg(follower.position, outside, follower.radius));
        let mut state = state();
        state.gates[0].admitted.0.clear();
        state.gates[0]
            .admitted
            .0
            .insert(Key::Episode, Value::Episode(Episode::new(2400, 0)));
        for unit in &frame.units {
            state.members.insert(
                unit.unit_id,
                Member {
                    owner: 0,
                    target: unit.target.unwrap(),
                    entrance: None,
                    accepted_tick: 1,
                    passing: None,
                    local_detour: None,
                    lazy_plan: lazy::Plan::None,
                    parking: None,
                    parking_retained: false,
                    guidance_pairs: 0,
                    motion_profile: [0; 3],
                    attempts: Attempts::default(),
                },
            );
        }
        let mut marked = no_certified_legs();
        let mut waiting = BTreeSet::new();
        let mut budget = crate::movement::sweep::SweepBudget::new(2400);
        apply(
            &frame,
            &mut proposed,
            &mut marked,
            &mut state,
            &mut waiting,
            2400,
            0.05,
            &mut budget,
            true,
            |_| true,
            |_| true,
            |_, _| true,
            |_, id| {
                Some(if id == 5 {
                    [7, 3.5_f32.to_bits(), 0]
                } else {
                    [0, 2.5_f32.to_bits(), 0]
                })
            },
            |_, _, _| Some(resource),
            |_, _, point, _, _, _, _| {
                if point == leader.position {
                    EntranceContact::Inside
                } else {
                    EntranceContact::Outside
                }
            },
            |_, _, _| true,
        );
        assert!(
            marked[0],
            "same-owner follower must honor the current inside leader"
        );
        let bay = proposed.units[0].target.unwrap();
        assert_ne!(bay, outside);
        assert!(resource.outside_waiting_layer(bay, follower.radius, leader.radius));
        assert_eq!(frame.units[0].target, follower.target);
    }

    #[test]
    fn equal_distance_leader_keeps_the_lead_and_its_private_outside_leg() {
        // Two same-owner large bodies wait at the two wall-side bays of the
        // real GM3 throat, at the same distance from the mouth. A foreign
        // body waits on the far side. No direct entry leg is certifiable
        // from a wall-side bay, so the leader must walk its private outside
        // leg to the mouth. The follower must not take the lead from it.
        let resource = Throat {
            min: Vec2::new(30., 22.),
            max: Vec2::new(34., 25.),
            axis: Vec2::X,
            level: 0,
        };
        let large = |unit_id, y| MovementFrameUnit {
            unit_id,
            unit_type: 0,
            speed: Some(2.5),
            position: Vec2::new(28.9, y),
            radius: 1.1,
            target: Some(Vec2::new(48., 23.5)),
            ..unit()
        };
        let frame = MovementFrame {
            bounds: Vec2::new(64., 48.),
            tick_hz: 20,
            pools: BTreeMap::new(),
            units: vec![large(7, 19.8), large(9, 27.2)],
        };
        let private = [Vec2::new(28.5, 21.), Vec2::new(28.5, 26.)];
        let mut state = state();
        state.gates[0].admitted.0.clear();
        state.gates[0].admitted.0.insert(
            Key::Episode,
            Value::Episode(Episode {
                mode: Mode::Retry,
                side: 1,
                stalled: 0,
                admitted: 0,
                tick: 99,
                level: 0,
                probe_radius: 0,
            }),
        );
        for unit in &frame.units {
            state.members.insert(
                unit.unit_id,
                Member {
                    owner: 0,
                    target: unit.target.unwrap(),
                    entrance: None,
                    accepted_tick: 1,
                    passing: None,
                    local_detour: None,
                    lazy_plan: lazy::Plan::None,
                    parking: None,
                    parking_retained: false,
                    guidance_pairs: 0,
                    motion_profile: [0; 3],
                    attempts: Attempts::default(),
                },
            );
        }
        let mut leader_legs = 0;
        for tick in 100..107 {
            for member in state.members.values_mut() {
                member.guidance_pairs = 0;
            }
            let mut proposed = frame.clone();
            for (unit, leg) in proposed.units.iter_mut().zip(private) {
                assert!(resource.away_leg(unit.position, leg, unit.radius));
                unit.target = Some(leg);
            }
            let mut marked = no_certified_legs();
            let mut waiting = BTreeSet::new();
            let mut budget = crate::movement::sweep::SweepBudget::new(tick);
            apply(
                &frame,
                &mut proposed,
                &mut marked,
                &mut state,
                &mut waiting,
                tick,
                0.05,
                &mut budget,
                true,
                |_| true,
                |_| true,
                |_, emit| {
                    emit(
                        50,
                        ObservedBody {
                            position: Vec2::new(35.100002, 19.8),
                            radius: 1.1,
                        },
                    )
                },
                |_, id| Some([0, 2.5_f32.to_bits(), u32::from(id == 50)]),
                |_, _, _| Some(resource),
                |_, _, _, _, _, _, _| EntranceContact::Outside,
                // The left plateau and the centre lane are each convex. A
                // leg between them cuts the wall corner.
                |_, from, to| {
                    let plateau = |p: Vec2| f64::from(p.x) + 1.1_f32 as f64 <= 30.;
                    let lane = |p: Vec2| p.y - 1.1 >= 22. && p.y + 1.1 <= 25.;
                    (plateau(from) && plateau(to)) || (lane(from) && lane(to))
                },
            );
            let side = match state.gates[0].admitted.0[&Key::Episode] {
                Value::Episode(e) => e.side,
                _ => panic!("episode"),
            };
            if tick < 103 {
                assert_eq!(
                    side, 1,
                    "tick{tick}: the far side holds the first retry turn"
                );
                for (unit, guide) in frame.units.iter().zip(&proposed.units) {
                    assert_eq!(
                        guide.target,
                        Some(unit.position),
                        "tick{tick} waits in its bay"
                    );
                }
            } else {
                assert_eq!(side, -1, "tick{tick}: the idle far side lost its turn");
                // The other large body is incompatible with the leader.
                assert_eq!(proposed.units[1].target, Some(frame.units[1].position));
                if proposed.units[0].target == Some(private[0]) {
                    leader_legs += 1;
                }
                if tick > 103 {
                    assert_eq!(
                        proposed.units[0].target,
                        Some(private[0]),
                        "tick{tick}: the leader lost its outside approach leg"
                    );
                }
            }
            let outputs = frame
                .units
                .iter()
                .map(|unit| crate::movement::MovementFrameOutput {
                    unit_id: unit.unit_id,
                    position: unit.position,
                    target: unit.target,
                    progress: None,
                    plain_progress: None,
                    physical_escape: None,
                    failed_blocked: false,
                    arrival_slot: None,
                    #[cfg(feature = "test-probes")]
                    probe: Default::default(),
                })
                .collect::<Vec<_>>();
            publish(&mut state, &frame, &outputs, tick);
        }
        assert_eq!(leader_legs, 3);
    }

    #[test]
    fn wall_side_bay_clears_a_current_waiting_body_without_entering_the_throat_layer() {
        let resource = Throat {
            min: Vec2::new(30., 22.),
            max: Vec2::new(34., 25.),
            axis: Vec2::X,
            level: 0,
        };
        let actor = MovementFrameUnit {
            position: Vec2::new(34.92974, 24.431387),
            radius: 0.4,
            ..unit()
        };
        let rows = [PublicRow {
            id: actor.unit_id + 1,
            body: ObservedBody {
                position: Vec2::new(36.2, 19.8),
                radius: 1.1,
            },
            profile: [0; 3],
        }];
        let mut checks = 0;
        let mut clear =
            |to| exact_leg(actor.position, to, &actor, &rows, &mut || true, &mut checks);
        assert_eq!(
            clear(resource.waiting_bay(1, 0.4, 1.1, 1, 1, -1)),
            Some(false)
        );
        let bay = resource.waiting_bay(1, 0.4, 1.1, 0, 1, -1);
        assert_eq!(clear(bay), Some(true));
        assert!(resource.outside_waiting_layer(bay, 0.4, 1.1));
        assert!(f64::from(bay.x) - f64::from(actor.radius) >= f64::from(resource.max.x));
    }

    #[test]
    fn pressure_retirement_preserves_live_retry_and_releases_complete_absent_rows_atomically() {
        for reverse in [false, true] {
            let mut state = state();
            let prototype = body(&state);
            state.gates[0]
                .admitted
                .0
                .retain(|key, _| *key == Key::Episode);
            if let Value::Episode(e) = state.gates[0].admitted.0.get_mut(&Key::Episode).unwrap() {
                e.mode = Mode::Retry;
                e.side = 1;
                e.stalled = 3;
                e.admitted = 2;
            }
            for id in 0..NEIGHBORS as u32 {
                state.gates[0]
                    .admitted
                    .0
                    .insert(Key::Body(id), Value::Body(prototype));
            }
            let resource = Throat {
                min: Vec2::new(30., 22.),
                max: Vec2::new(34., 25.),
                axis: Vec2::X,
                level: 0,
            };
            let mut rows: Vec<_> = (0..NEIGHBORS as u32)
                .map(|id| PublicRow {
                    id: 1000 + id,
                    body: ObservedBody {
                        position: Vec2::new(5., 5.),
                        radius: 0.4,
                    },
                    profile: [0; 3],
                })
                .collect();
            if reverse {
                rows.reverse();
            }
            assert!(!room_for(&state.gates, resource, 0, &rows));
            let before = format!("{:?}", state.gates);
            let mut remaining = 10;
            assert!(!retire_inert(&mut state.gates, 0, 11, &rows, &mut || {
                remaining -= 1;
                remaining > 0
            }));
            assert_eq!(
                format!("{:?}", state.gates),
                before,
                "partial work cannot evict any history"
            );
            assert!(retire_inert(&mut state.gates, 0, 11, &rows, &mut || true));
            assert!(room_for(&state.gates, resource, 0, &rows));
            assert_eq!(state.gates[0].admitted.len(), 1);
            let Value::Episode(e) = state.gates[0].admitted.0[&Key::Episode] else {
                panic!("episode");
            };
            assert_eq!(
                (e.mode, e.side, e.stalled, e.admitted, e.tick),
                (Mode::Retry, 1, 3, 2, 10)
            );
        }
    }

    #[test]
    fn pressure_retirement_keeps_current_body_history_and_other_owners() {
        let mut state = state();
        let old = body(&state);
        let mut foreign_gate = self::state().gates.remove(0);
        foreign_gate.reserved_by = [false, true];
        state.gates.push(foreign_gate);
        let rows = [PublicRow {
            id: 7,
            body: ObservedBody {
                position: old.pose,
                radius: old.radius,
            },
            profile: old.profile,
        }];
        assert!(retire_inert(&mut state.gates, 0, 11, &rows, &mut || true));
        assert_eq!(body(&state).entry, old.entry);
        assert_eq!(body(&state).reference, old.reference);
        assert_eq!(state.gates.len(), 2);
        assert!(retire_inert(&mut state.gates, 0, 12, &rows, &mut || true));
        assert_eq!(
            state.gates.len(),
            1,
            "nonconsecutive own metadata no longer has dated authority"
        );
        assert_eq!(
            state.gates[0].reserved_by,
            [false, true],
            "other owners remain untouched"
        );
    }

    #[test]
    fn later_owned_bay_cannot_cut_an_earlier_retreat_or_share_its_endpoint() {
        let prior = Retreat {
            from: Vec2::new(10., 10.),
            to: Vec2::new(15., 10.),
            radius: 0.4,
        };
        let mut checks = 0;
        // Current body positions are clear, but this final bay would block
        // the first actor before it could reach its already certified bay.
        let crossing = Retreat {
            from: Vec2::new(12., 13.),
            to: Vec2::new(12., 10.),
            radius: 0.4,
        };
        assert_eq!(
            retreats_clear(crossing, &[prior], &mut || true, &mut checks),
            Some(false)
        );
        let duplicate = Retreat {
            from: Vec2::new(18., 13.),
            to: prior.to,
            radius: 0.4,
        };
        assert_eq!(
            retreats_clear(duplicate, &[prior], &mut || true, &mut checks),
            Some(false)
        );
        let separate = Retreat {
            from: Vec2::new(12., 13.),
            to: Vec2::new(15., 13.),
            radius: 0.4,
        };
        assert_eq!(
            retreats_clear(separate, &[prior], &mut || true, &mut checks),
            Some(true)
        );
        assert_eq!(
            retreats_clear(separate, &[prior], &mut || false, &mut checks),
            None
        );
    }

    #[test]
    fn owned_goal_entry_lane_bypasses_a_compatible_current_body() {
        // Captured movable-world positions. This is a local exact-leg check,
        // not a replay of the missing controller history.
        let resource = Throat {
            min: Vec2::new(30., 22.),
            max: Vec2::new(34., 25.),
            axis: Vec2::X,
            level: 0,
        };
        let actor = MovementFrameUnit {
            position: Vec2::new(32., 23.5),
            radius: 0.4,
            target: Some(Vec2::new(42., 30.)),
            ..unit()
        };
        let rows = [PublicRow {
            id: actor.unit_id + 1,
            body: ObservedBody {
                position: Vec2::new(33.599998, 23.5),
                radius: 0.4,
            },
            profile: [0; 3],
        }];
        let mut checks = 0;
        let mut clear =
            |to| exact_leg(actor.position, to, &actor, &rows, &mut || true, &mut checks);
        assert_eq!(
            clear(
                resource
                    .connector(1, actor.radius, actor.target.unwrap())
                    .unwrap()
            ),
            Some(false)
        );
        for depth in 0..3 {
            assert_eq!(
                clear(
                    resource
                        .entry(1, actor.radius, resource.middle(), depth)
                        .unwrap()
                ),
                Some(false)
            );
        }
        let side_leg = resource
            .entry(1, actor.radius, actor.target.unwrap(), 0)
            .unwrap();
        assert_eq!(clear(side_leg), Some(true));
        assert!(f64::from(side_leg.y) + f64::from(actor.radius) <= f64::from(resource.max.y));
        assert!(f64::from(side_leg.y) - f64::from(actor.radius) >= f64::from(resource.min.y));
    }

    #[test]
    fn mixed_radius_bays_clear_the_common_collision_layer_and_keep_their_service_window() {
        for axis in [Vec2::X, Vec2::Y] {
            let resource = if axis == Vec2::X {
                Throat {
                    min: Vec2::new(30., 22.),
                    max: Vec2::new(34., 25.),
                    axis,
                    level: 0,
                }
            } else {
                Throat {
                    min: Vec2::new(22., 30.),
                    max: Vec2::new(25., 34.),
                    axis,
                    level: 0,
                }
            };
            for radius in [0.4, 1.1] {
                for side in [-1, 1] {
                    for axial in 0..=4 {
                        for lateral in 1..=4 {
                            for normal_side in [-1, 1] {
                                let bay = resource.waiting_bay(
                                    side,
                                    radius,
                                    1.1,
                                    axial,
                                    lateral,
                                    normal_side,
                                );
                                assert!(
                                    resource.outside_waiting_layer(bay, radius, 1.1),
                                    "outward f32 rounding must clear the whole collision layer"
                                );
                                assert!(
                                    resource.near(bay, radius.max(1.1)),
                                    "a small body must retain its bay until service releases it"
                                );
                                assert!(!resource.inside(bay, radius));
                            }
                        }
                    }
                }
            }
        }
        let resource = Throat {
            min: Vec2::new(30., 22.),
            max: Vec2::new(34., 25.),
            axis: Vec2::X,
            level: 0,
        };
        assert!(
            !resource.outside_waiting_layer(Vec2::new(28.8, 25.4), 0.4, 1.1),
            "an actor-only bay does not reserve clearance for the current large convoy"
        );
    }

    #[test]
    fn opposite_transit_grant_retires_a_private_outside_leg_and_keeps_the_holder_fixed() {
        // Actual movable-blocker capture positions and private waypoint. This
        // checks the public service seam, not a replay of uncaptured Guide state.
        let left = MovementFrameUnit {
            unit_id: 1,
            unit_type: 7,
            position: Vec2::new(28.075450897216797, 23.324234008789062),
            radius: 0.4,
            speed: Some(3.5),
            target: Some(Vec2::new(49.150001525878906, 21.508142471313477)),
            ..unit()
        };
        let winner = MovementFrameUnit {
            unit_id: 16,
            owner_slot: 1,
            unit_type: 0,
            position: Vec2::new(32.900001525878906, 23.100000381469727),
            radius: 1.1,
            speed: None,
            target: None,
            ..unit()
        };
        let resource = Throat {
            min: Vec2::new(30., 22.),
            max: Vec2::new(34., 25.),
            axis: Vec2::X,
            level: 0,
        };
        let frame = MovementFrame {
            bounds: Vec2::new(64., 48.),
            tick_hz: 20,
            pools: BTreeMap::new(),
            units: vec![left.clone(), winner.clone()],
        };
        let mut proposed = frame.clone();
        proposed.units[0].target = Some(Vec2::new(29., 23.5));
        assert!(resource.away_leg(
            left.position,
            proposed.units[0].target.unwrap(),
            left.radius
        ));
        let mut state = state();
        state.gates[0].admitted.0.clear();
        // The current settled body is already inside while the active mover
        // is outside. Let the real reducer establish occupancy priority; no
        // synthetic dated entry or missing prior body history is supplied.
        state.gates[0]
            .admitted
            .0
            .insert(Key::Episode, Value::Episode(Episode::new(2410, 0)));
        state.members.insert(
            left.unit_id,
            Member {
                owner: 0,
                target: left.target.unwrap(),
                entrance: None,
                accepted_tick: 11,
                passing: None,
                local_detour: None,
                lazy_plan: lazy::Plan::None,
                parking: None,
                parking_retained: false,
                guidance_pairs: 0,
                motion_profile: [0; 3],
                attempts: Attempts::default(),
            },
        );
        let mut marked = no_certified_legs();
        let mut waiting = BTreeSet::new();
        let mut budget = crate::movement::sweep::SweepBudget::new(2410);
        apply(
            &frame,
            &mut proposed,
            &mut marked,
            &mut state,
            &mut waiting,
            2410,
            0.05,
            &mut budget,
            true,
            |i| i == 0,
            |_| true,
            |viewer, emit| {
                let peer = if viewer == 0 { &winner } else { &left };
                emit(
                    peer.unit_id,
                    ObservedBody {
                        position: peer.position,
                        radius: peer.radius,
                    },
                )
            },
            |_, id| {
                Some(if id == 1 {
                    [7, 3.5_f32.to_bits(), 0]
                } else {
                    [0, 2.5_f32.to_bits(), 1]
                })
            },
            |_, _, _| Some(resource),
            |_, _, _, _, _, _, _| EntranceContact::Outside,
            |_, _, _| true,
        );
        assert!(
            marked[0],
            "the active loser must receive a currently certified backout"
        );
        assert_ne!(proposed.units[0].target, Some(Vec2::new(29., 23.5)));
        let bay = proposed.units[0].target.unwrap();
        assert!(resource.away_leg(bay, bay, left.radius));
        assert_eq!(proposed.units[1].position, winner.position);
        assert_eq!(
            proposed.units[1].target, None,
            "settled holder stays immovable"
        );
        assert!(!marked[1]);
        assert_eq!(
            frame.units[0].target, left.target,
            "original owned destination is retained"
        );
    }

    #[test]
    fn complete_marker_admission_includes_last_row_and_refuses_the_entire_overbound_frame() {
        for count in [NEIGHBORS, NEIGHBORS + 1] {
            let mut units = (0..count - 1)
                .map(|i| MovementFrameUnit {
                    unit_id: 1000 + i as u32,
                    owner_slot: 0,
                    unit_type: 2,
                    position: Vec2::new(5. + 2. * (i % 8) as f32, 40. + 2. * (i / 8) as f32),
                    radius: 0.01,
                    speed: None,
                    target: None,
                    progress: None,
                    plain_progress: None,
                    physical_escape: None,
                })
                .collect::<Vec<_>>();
            let mover = MovementFrameUnit {
                position: Vec2::new(28.75, 23.5),
                radius: 1.125,
                target: Some(Vec2::new(48., 23.5)),
                ..unit()
            };
            units.push(mover.clone());
            let frame = MovementFrame {
                bounds: Vec2::new(64., 80.),
                tick_hz: 20,
                pools: BTreeMap::new(),
                units,
            };
            let mut proposed = frame.clone();
            let mut state = TrafficState::default();
            state.members.insert(
                mover.unit_id,
                Member {
                    owner: 0,
                    target: mover.target.unwrap(),
                    entrance: None,
                    accepted_tick: 1,
                    passing: None,
                    local_detour: None,
                    lazy_plan: lazy::Plan::None,
                    parking: None,
                    parking_retained: false,
                    guidance_pairs: 0,
                    motion_profile: [0; 3],
                    attempts: Attempts::default(),
                },
            );
            let mut marked = [true; NEIGHBORS];
            let mut waiting = BTreeSet::new();
            let mut budget = crate::movement::sweep::SweepBudget::new(1);
            apply(
                &frame,
                &mut proposed,
                &mut marked,
                &mut state,
                &mut waiting,
                1,
                0.05,
                &mut budget,
                true,
                |i| i == count - 1,
                |_| true,
                |_, _| true,
                |_, id| {
                    Some([
                        if id == mover.unit_id { 9 } else { 2 },
                        if id == mover.unit_id {
                            3_f32.to_bits()
                        } else {
                            0
                        },
                        0,
                    ])
                },
                |_, _, _| {
                    Some(Throat {
                        min: Vec2::new(30., 22.),
                        max: Vec2::new(34., 25.),
                        axis: Vec2::X,
                        level: 0,
                    })
                },
                |_, _, _, _, _, _, _| EntranceContact::Outside,
                |_, _, _| true,
            );
            if count == NEIGHBORS {
                assert!(
                    marked[count - 1],
                    "last eligible row keeps full leg authority"
                );
                assert_eq!(marked.iter().filter(|&&flag| flag).count(), 1);
                assert_eq!(
                    proposed.units[count - 1].target,
                    Some(Vec2::new(35.125, 23.5))
                );
            } else {
                assert!(!marked.iter().any(|&flag| flag));
                assert_eq!(
                    proposed.units[count - 1].target,
                    mover.target,
                    "overbound refusal retains the complete original order"
                );
            }
        }
    }

    #[test]
    fn four_entry_turn_closes_new_entries_without_backing_out_its_incumbent() {
        let mut e = Episode::new(10, 0);
        e.mode = Mode::Transit;
        e.side = -1;
        e.admitted = 4;
        assert!(e.yields_new_entry(-1, false));
        assert!(!e.yields_new_entry(-1, true));
        e.tick = 10;
        e.update(11, true, [false; 2], true, [true; 2], [false; 2], [0; 2]);
        assert_eq!((e.mode, e.side), (Mode::Retry, 1));
    }
    #[test]
    fn private_forget_keeps_public_physical_history_but_relevant_contact_loss_suspends_it() {
        let mut state = state();
        state.forget(7);
        assert!(body(&state).entry.is_some());
        assert!(body(&state).accepted.is_some());
        let mut foreign = unit();
        foreign.owner_slot = 1;
        state.invalidate_contact(&foreign);
        assert!(
            body(&state).entry.is_some(),
            "another viewer cannot invalidate this contact lease"
        );
        state.invalidate_contact(&unit());
        assert!(body(&state).entry.is_none());
        assert!(body(&state).accepted.is_none());
        assert!(matches!(
            state.gates[0].admitted.0[&Key::Episode],
            Value::Episode(Episode {
                mode: Mode::Drain,
                ..
            })
        ));
    }
    #[test]
    fn certified_publication_ignores_private_intent_and_refuses_pose_shape_or_owner_discontinuity()
    {
        let original = unit();
        let frame = MovementFrame {
            bounds: Vec2::new(64., 48.),
            tick_hz: 20,
            pools: BTreeMap::new(),
            units: vec![original.clone()],
        };
        let output = crate::movement::MovementFrameOutput {
            unit_id: 7,
            position: original.position + Vec2::X * 0.1,
            target: None,
            progress: None,
            plain_progress: None,
            physical_escape: None,
            failed_blocked: false,
            arrival_slot: None,
            #[cfg(feature = "test-probes")]
            probe: Default::default(),
        };
        for mode in 0..6 {
            let mut state = state();
            let mut input = frame.clone();
            match mode {
                0 => {}
                1 => {
                    input.units[0].target = None;
                    input.units[0].speed = None;
                    input.units[0].physical_escape = Some(crate::movement::PhysicalEscape {
                        target: Some(Vec2::ZERO),
                    });
                }
                2 => input.units[0].position.x = 30.,
                3 => input.units[0].radius = 0.6,
                4 => input.units[0].unit_type = 10,
                _ => input.units[0].owner_slot = 1,
            }
            publish(&mut state, &input, std::slice::from_ref(&output), 10);
            assert_eq!(
                body(&state).accepted,
                (mode < 2).then_some(output.position),
                "binding mode{mode}"
            );
        }
        let mut pressured = state();
        pressured.tick_owner_neighbors[0] = OWNER_VISITS - GUIDED_RESERVE - 1;
        publish(&mut pressured, &frame, &[output], 10);
        assert!(
            body(&pressured).accepted.is_none(),
            "complete publication admission refuses without extending work"
        );
        assert_eq!(
            pressured.tick_owner_neighbors[0],
            OWNER_VISITS - GUIDED_RESERVE - 1
        );
    }
}
