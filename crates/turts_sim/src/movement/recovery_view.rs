//! The owner-known body index of metered, resumable recovery queries (design
//! `2026-11-19-recovery-actual-work`).
//!
//! - A `KnownView` holds the bodies that an owner may know: its own bodies and the bodies
//!   that its visibility basis permits (all bodies with no basis). Hidden bodies are not in
//!   it, so they cannot change a query, a charge or a proof. The frame keeps it live: each
//!   move of a member updates it. A member can be *soft* (design section 7, B2): a body
//!   of another owner that the claim exchange does not treat as an obstacle. A query that
//!   sets `ignore_soft` (the stall spare-slot search) does not count soft copies.
//! - Cells are keyed by `(cy, cx)` in a `BTreeMap`; a cell holds a multiset of body keys
//!   `(ordered x, ordered y, radius bits)`. That order is the canonical physical order of a
//!   query; no unit id, row index or hash order decides it. Equal keys are one entry with a
//!   count: they are the same fact.
//! - Each cell keeps an order-free digest of its multiset, updated in O(log n) for each
//!   change. A proof over a region costs one operation for each cell row and each cell, not
//!   one for each body.
//! - `Contacts` holds the bodies that the actor touches but the view does not hold (the
//!   contact rule of `known`). The frame finds them with one physical query for each row;
//!   that query is frame work and is never charged to an owner.
//! - `ClaimSet` keeps the pool claim keys and an order-free digest for each pool and claim
//!   cell, so a proof reads the own-pool claims of a region in O(cells).

use bevy_math::Vec2;
use std::collections::{BTreeMap, HashSet};
use std::ops::Bound;

use super::RecoveryQuery;

pub(crate) type Digest = [u64; 3];
/// `(ordered x bits, ordered y bits, radius bits)`.
pub(crate) type BodyKey = [u32; 3];

const DIGEST_SEEDS: [u64; 2] = [0x243f_6a88_85a3_08d3, 0x1319_8a2e_0370_7344];
pub(super) const BODY_TAG: u64 = 1 << 63;
/// Marks the fact of a soft copy: a body that changes class changes the digest.
const SOFT_TAG: u64 = 1 << 62;
/// The claim cell size of `ClaimSet`, in world units. Fixed: it depends on no body.
const CLAIM_CELL: f32 = 4.0;
/// The smallest half cell of a view: a view of tiny bodies keeps a sane cell count.
const MIN_HALF: f32 = 1.0 / 64.0;
/// Relative margin on a query box, as in `grid::UniformGrid`.
const EDGE_MARGIN: f32 = 1e-4;

fn mix(a: u64, b: u64, seed: u64) -> u64 {
    let mut z = a
        .wrapping_mul(0x9e37_79b9_7f4a_7c15)
        .rotate_left(31)
        .wrapping_add(b ^ seed);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Adds `count` copies (or removes them, with `negative`) of the fact `(a, b)`.
fn change_facts(digest: &mut Digest, a: u64, b: u64, count: u64, negative: bool) {
    let terms = [
        count,
        count.wrapping_mul(mix(a, b, DIGEST_SEEDS[0])),
        count.wrapping_mul(mix(a, b, DIGEST_SEEDS[1])),
    ];
    for (slot, term) in digest.iter_mut().zip(terms) {
        *slot = if negative {
            slot.wrapping_sub(term)
        } else {
            slot.wrapping_add(term)
        };
    }
}

pub(crate) fn add_digest(digest: &mut Digest, other: &Digest) {
    for (slot, term) in digest.iter_mut().zip(other) {
        *slot = slot.wrapping_add(*term);
    }
}

fn ordered(value: f32) -> u32 {
    let bits = value.to_bits();
    if bits & 0x8000_0000 != 0 {
        !bits
    } else {
        bits ^ 0x8000_0000
    }
}

fn unordered(bits: u32) -> f32 {
    f32::from_bits(if bits & 0x8000_0000 != 0 {
        bits ^ 0x8000_0000
    } else {
        !bits
    })
}

pub(crate) fn body_key(position: Vec2, radius: f32) -> BodyKey {
    [ordered(position.x), ordered(position.y), radius.to_bits()]
}

fn key_position(key: &BodyKey) -> Vec2 {
    Vec2::new(unordered(key[0]), unordered(key[1]))
}

fn key_radius(key: &BodyKey) -> f32 {
    f32::from_bits(key[2])
}

fn key_fact(key: &BodyKey) -> (u64, u64) {
    (
        BODY_TAG | u64::from(key[2]),
        (u64::from(key[0]) << 32) | u64::from(key[1]),
    )
}

fn copy_fact(key: &BodyKey, soft: bool) -> (u64, u64) {
    let (a, b) = key_fact(key);
    (if soft { a | SOFT_TAG } else { a }, b)
}

fn grow(reach: f32) -> f32 {
    reach.abs() + EDGE_MARGIN * reach.abs().max(1.0)
}

/// A work allowance: one unit for each cell row, cell, body key, candidate, claim or
/// contact read. `step` refuses to go below `floor`, so a caller can keep a reserve for
/// the proof digest of a pause.
#[derive(Debug)]
pub(crate) struct Meter {
    pub(crate) left: usize,
    pub(crate) used: usize,
    pub(crate) floor: usize,
    pub(crate) cells: u64,
    pub(crate) bodies: u64,
    pub(crate) digests: u64,
    pub(crate) claims: u64,
}

impl Meter {
    pub(crate) fn new(allowance: usize) -> Self {
        Self {
            left: allowance,
            used: 0,
            floor: 0,
            cells: 0,
            bodies: 0,
            digests: 0,
            claims: 0,
        }
    }

    pub(crate) fn step(&mut self) -> bool {
        if self.left > self.floor {
            self.left -= 1;
            self.used += 1;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Default)]
struct ViewCell {
    /// The copies of each key: `[hard, soft]`.
    bodies: BTreeMap<BodyKey, [u32; 2]>,
    digest: Digest,
}

/// The class of a frame row in a view.
const NOT_MEMBER: u8 = 0;
const HARD: u8 = 1;
const SOFT: u8 = 2;

/// The bodies that one owner may know, with live positions.
#[derive(Debug)]
pub(crate) struct KnownView {
    half: f32,
    cell: f32,
    cells: BTreeMap<(i32, i32), ViewCell>,
    /// One byte for each frame row: not a member, a hard member or a soft member.
    members: Vec<u8>,
    entries: usize,
}

/// An inclusive cell box `(low y, high y, low x, high x)`.
type CellBox = (i32, i32, i32, i32);

impl KnownView {
    /// Builds the view from `(row, position, radius)` members of a frame of `rows` rows.
    #[cfg(test)]
    pub(crate) fn build(members: &[(usize, Vec2, f32)], rows: usize) -> Self {
        let members: Vec<_> = members
            .iter()
            .map(|&(row, position, radius)| (row, position, radius, false))
            .collect();
        Self::build_soft(&members, rows)
    }

    /// Builds the view from `(row, position, radius, soft)` members of a frame of `rows`
    /// rows.
    pub(crate) fn build_soft(members: &[(usize, Vec2, f32, bool)], rows: usize) -> Self {
        let max_radius = members.iter().map(|m| m.2).fold(0.0_f32, f32::max);
        // A power of two: the cell size changes only when the largest known radius crosses
        // one, so a proof survives ordinary visibility changes.
        let half = 2.0_f32.powi(max_radius.max(MIN_HALF).log2().ceil() as i32);
        let mut view = Self {
            half,
            cell: 2.0 * half,
            cells: BTreeMap::new(),
            members: vec![NOT_MEMBER; rows],
            entries: 0,
        };
        for &(row, position, radius, soft) in members {
            view.members[row] = if soft { SOFT } else { HARD };
            view.insert(position, radius, soft);
        }
        view
    }

    /// The largest reach of a known body: a power of two at least as large as every
    /// member radius.
    pub(crate) fn half(&self) -> f32 {
        self.half
    }

    pub(crate) fn contains(&self, row: usize) -> bool {
        self.members
            .get(row)
            .is_some_and(|&class| class != NOT_MEMBER)
    }

    fn cell_of(&self, position: Vec2) -> (i32, i32) {
        (
            (position.y / self.cell).floor() as i32,
            (position.x / self.cell).floor() as i32,
        )
    }

    fn cell_box(&self, center: Vec2, reach: f32) -> CellBox {
        let half = grow(reach);
        let (low_y, low_x) = self.cell_of(center - Vec2::splat(half));
        let (high_y, high_x) = self.cell_of(center + Vec2::splat(half));
        (low_y, high_y, low_x, high_x)
    }

    fn insert(&mut self, position: Vec2, radius: f32, soft: bool) {
        let key = body_key(position, radius);
        let cell = self.cells.entry(self.cell_of(position)).or_default();
        let count = cell.bodies.entry(key).or_default();
        if *count == [0, 0] {
            self.entries += 1;
        }
        count[usize::from(soft)] += 1;
        let (a, b) = copy_fact(&key, soft);
        change_facts(&mut cell.digest, a, b, 1, false);
    }

    fn remove(&mut self, position: Vec2, radius: f32, soft: bool) {
        let key = body_key(position, radius);
        let at = self.cell_of(position);
        let Some(cell) = self.cells.get_mut(&at) else {
            debug_assert!(false, "a view member must be in its cell");
            return;
        };
        let Some(count) = cell.bodies.get_mut(&key) else {
            debug_assert!(false, "a view member must have its key");
            return;
        };
        if count[usize::from(soft)] == 0 {
            debug_assert!(false, "a view member must have a copy of its class");
            return;
        }
        count[usize::from(soft)] -= 1;
        if *count == [0, 0] {
            cell.bodies.remove(&key);
            self.entries -= 1;
        }
        let (a, b) = copy_fact(&key, soft);
        change_facts(&mut cell.digest, a, b, 1, true);
        if cell.bodies.is_empty() {
            self.cells.remove(&at);
        }
    }

    /// Follows the move of frame row `row` (no change when it is not a member). True
    /// when the view changed: one remove and one insert, each with its digest update.
    pub(crate) fn relocate(&mut self, row: usize, from: Vec2, to: Vec2, radius: f32) -> bool {
        if !self.contains(row)
            || from.to_array().map(f32::to_bits) == to.to_array().map(f32::to_bits)
        {
            return false;
        }
        #[cfg(test)]
        RELOCATIONS.with(|count| count.set(count.get() + 1));
        let soft = self.members[row] == SOFT;
        self.remove(from, radius, soft);
        self.insert(to, radius, soft);
        true
    }

    /// An upper bound of the bytes the view holds (B-tree nodes at half fill).
    pub(crate) fn bytes(&self) -> usize {
        let entry = std::mem::size_of::<(BodyKey, [u32; 2])>();
        let cell = std::mem::size_of::<((i32, i32), ViewCell)>();
        2 * (self.entries * entry + self.cells.len() * cell) + self.members.capacity()
    }

    /// The cost bound and the digest of the view cells in a region: one unit for each cell
    /// row and each cell, or one unit for each occupied cell when that is fewer.
    fn region_cost(&self, center: Vec2, reach: f32) -> usize {
        let (low_y, high_y, low_x, high_x) = self.cell_box(center, reach);
        let rows = (i64::from(high_y) - i64::from(low_y) + 1) as u128;
        let columns = (i64::from(high_x) - i64::from(low_x) + 1) as u128;
        let occupied = self.cells.len() as u128;
        if rows <= occupied {
            (rows + (rows * columns).min(occupied)) as usize
        } else {
            occupied as usize
        }
    }

    fn region_digest(&self, center: Vec2, reach: f32, meter: &mut Meter) -> Option<Digest> {
        let (low_y, high_y, low_x, high_x) = self.cell_box(center, reach);
        let rows = (i64::from(high_y) - i64::from(low_y) + 1) as u128;
        let mut digest = [0; 3];
        if rows <= self.cells.len() as u128 {
            for y in low_y..=high_y {
                if !meter.step() {
                    return None;
                }
                for (_, cell) in self.cells.range((y, low_x)..=(y, high_x)) {
                    if !meter.step() {
                        return None;
                    }
                    meter.cells += 1;
                    add_digest(&mut digest, &cell.digest);
                }
            }
        } else {
            for (&(y, x), cell) in &self.cells {
                if !meter.step() {
                    return None;
                }
                meter.cells += 1;
                if (low_y..=high_y).contains(&y) && (low_x..=high_x).contains(&x) {
                    add_digest(&mut digest, &cell.digest);
                }
            }
        }
        Some(digest)
    }
}

/// The bodies that an actor touches at tick start and its view does not hold, with live
/// positions, in canonical key order.
#[derive(Debug, Default)]
pub(crate) struct Contacts {
    entries: Vec<(BodyKey, u32)>,
    digest: Digest,
    max_radius: f32,
}

impl Contacts {
    pub(crate) fn new(bodies: impl IntoIterator<Item = (Vec2, f32)>) -> Self {
        let mut keys: Vec<BodyKey> = bodies
            .into_iter()
            .map(|(position, radius)| body_key(position, radius))
            .collect();
        keys.sort_unstable();
        let mut contacts = Self::default();
        for key in keys {
            contacts.max_radius = contacts.max_radius.max(key_radius(&key));
            let (a, b) = key_fact(&key);
            change_facts(&mut contacts.digest, a, b, 1, false);
            match contacts.entries.last_mut() {
                Some((last, count)) if *last == key => *count += 1,
                _ => contacts.entries.push((key, 1)),
            }
        }
        contacts
    }

    pub(crate) fn bytes(&self) -> usize {
        self.entries.capacity() * std::mem::size_of::<(BodyKey, u32)>()
    }
}

/// The pool claim keys `(pool id, x bits, y bits)`, with an order-free digest for each pool
/// and claim cell. Reads go through `Deref` to the set.
#[derive(Debug, Default)]
pub(crate) struct ClaimSet {
    set: HashSet<(u64, u32, u32)>,
    cells: BTreeMap<(u64, i32, i32), Digest>,
    pool_cells: BTreeMap<u64, usize>,
}

fn claim_cell(x: u32, y: u32) -> (i32, i32) {
    (
        (f32::from_bits(y) / CLAIM_CELL).floor() as i32,
        (f32::from_bits(x) / CLAIM_CELL).floor() as i32,
    )
}

fn claim_fact(key: &(u64, u32, u32)) -> (u64, u64) {
    (
        key.0 & !BODY_TAG,
        (u64::from(key.1) << 32) | u64::from(key.2),
    )
}

impl ClaimSet {
    pub(crate) fn new(keys: impl IntoIterator<Item = (u64, u32, u32)>) -> Self {
        let mut claims = Self::default();
        for key in keys {
            claims.insert(key);
        }
        claims
    }

    pub(crate) fn insert(&mut self, key: (u64, u32, u32)) -> bool {
        if !self.set.insert(key) {
            return false;
        }
        let (y, x) = claim_cell(key.1, key.2);
        let (a, b) = claim_fact(&key);
        let cell = self.cells.entry((key.0, y, x)).or_default();
        if cell[0] == 0 {
            *self.pool_cells.entry(key.0).or_default() += 1;
        }
        change_facts(cell, a, b, 1, false);
        true
    }

    pub(crate) fn remove(&mut self, key: &(u64, u32, u32)) -> bool {
        if !self.set.remove(key) {
            return false;
        }
        let (y, x) = claim_cell(key.1, key.2);
        let at = (key.0, y, x);
        let (a, b) = claim_fact(key);
        if let Some(digest) = self.cells.get_mut(&at) {
            change_facts(digest, a, b, 1, true);
            if digest[0] == 0 {
                self.cells.remove(&at);
                if let Some(count) = self.pool_cells.get_mut(&key.0) {
                    *count -= 1;
                    if *count == 0 {
                        self.pool_cells.remove(&key.0);
                    }
                }
            }
        }
        true
    }

    fn pool_box(center: Vec2, reach: f32) -> CellBox {
        let half = grow(reach);
        let low = center - Vec2::splat(half);
        let high = center + Vec2::splat(half);
        (
            (low.y / CLAIM_CELL).floor() as i32,
            (high.y / CLAIM_CELL).floor() as i32,
            (low.x / CLAIM_CELL).floor() as i32,
            (high.x / CLAIM_CELL).floor() as i32,
        )
    }

    /// The cost bound of `region_digest`: one unit for each claim cell row (the claim cells
    /// are coarse, so a region has few rows).
    fn region_cost(&self, pool: u64, center: Vec2, reach: f32) -> usize {
        let (low_y, high_y, low_x, high_x) = Self::pool_box(center, reach);
        let rows = (i64::from(high_y) - i64::from(low_y) + 1) as usize;
        let columns = (i64::from(high_x) - i64::from(low_x) + 1) as usize;
        let occupied = self.pool_cells.get(&pool).copied().unwrap_or(0);
        rows + (rows.saturating_mul(columns)).min(occupied)
    }

    fn region_digest(
        &self,
        pool: u64,
        center: Vec2,
        reach: f32,
        meter: &mut Meter,
    ) -> Option<Digest> {
        let (low_y, high_y, low_x, high_x) = Self::pool_box(center, reach);
        let mut digest = [0; 3];
        for y in low_y..=high_y {
            if !meter.step() {
                return None;
            }
            for (_, cell) in self.cells.range((pool, y, low_x)..=(pool, y, high_x)) {
                if !meter.step() {
                    return None;
                }
                meter.claims += 1;
                add_digest(&mut digest, cell);
            }
        }
        Some(digest)
    }

    pub(crate) fn bytes(&self) -> usize {
        self.set.capacity() * std::mem::size_of::<(u64, u32, u32)>()
            + 2 * self.cells.len() * std::mem::size_of::<((u64, i32, i32), Digest)>()
            + 2 * self.pool_cells.len() * std::mem::size_of::<(u64, usize)>()
    }
}

impl std::ops::Deref for ClaimSet {
    type Target = HashSet<(u64, u32, u32)>;

    fn deref(&self) -> &Self::Target {
        &self.set
    }
}

#[cfg(test)]
thread_local! {
    /// Test probe: the view relocations that changed a view.
    pub(crate) static RELOCATIONS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// The result of a metered point query.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Probe {
    Blocked,
    Clear,
    /// The meter ran out; continue from this cursor.
    Pending(RecoveryQuery),
}

/// The bodies that one actor knows: its owner view (without the actor) and its contacts.
pub(crate) struct KnownSet<'a> {
    pub(crate) view: &'a KnownView,
    pub(crate) contacts: &'a Contacts,
    /// The key of the actor in its view: one copy of it is not an obstacle.
    pub(crate) own_key: Option<BodyKey>,
    /// True for the stall spare-slot search: a soft copy is not an obstacle. The actor is
    /// a hard member of its own view.
    pub(crate) ignore_soft: bool,
}

impl KnownSet<'_> {
    /// The cell size of the view, as bits: a proof holds only for the same cells.
    pub(crate) fn cell_bits(&self) -> u32 {
        self.view.cell.to_bits()
    }

    pub(crate) fn fresh_query(point: Vec2) -> RecoveryQuery {
        RecoveryQuery {
            point,
            stage: 0,
            cy: 0,
            cx: 0,
            key: None,
            contact: 0,
            started: false,
            digest: [0; 3],
        }
    }

    /// True when a known body blocks a disc of `radius` at `query.point`:
    /// `distance < radius + other radius - epsilon`. It stops at the first blocker. The
    /// visit order is canonical, so the work does not depend on ids or insertion order.
    pub(crate) fn probe(
        &self,
        mut query: RecoveryQuery,
        radius: f32,
        epsilon: f32,
        meter: &mut Meter,
    ) -> Probe {
        let point = query.point;
        let blocks =
            |key: &BodyKey| point.distance(key_position(key)) < radius + key_radius(key) - epsilon;
        if query.stage == 0 {
            let reach = radius + self.view.half;
            let (low_y, high_y, low_x, high_x) = self.view.cell_box(point, reach);
            let low_key = [ordered(point.x - grow(reach)), 0, 0];
            let high_key = [ordered(point.x + grow(reach)), u32::MAX, u32::MAX];
            if !query.started {
                query.started = true;
                query.cy = low_y;
                query.cx = low_x;
                query.key = None;
            }
            while query.cy <= high_y {
                if !meter.step() {
                    return Probe::Pending(query);
                }
                let y = query.cy;
                for (&(_, x), cell) in self.view.cells.range((y, query.cx)..=(y, high_x)) {
                    if x != query.cx {
                        query.cx = x;
                        query.key = None;
                    }
                    if !meter.step() {
                        return Probe::Pending(query);
                    }
                    meter.cells += 1;
                    let low = match query.key {
                        Some(key) => Bound::Excluded(key),
                        None => Bound::Included(low_key),
                    };
                    for (key, &[hard, soft]) in cell.bodies.range((low, Bound::Included(high_key)))
                    {
                        if !meter.step() {
                            return Probe::Pending(query);
                        }
                        meter.bodies += 1;
                        // The actor is a hard copy of its own key.
                        let own = u32::from(self.own_key == Some(*key)).min(hard);
                        let others = hard - own + if self.ignore_soft { 0 } else { soft };
                        if others > 0 && blocks(key) {
                            return Probe::Blocked;
                        }
                        query.key = Some(*key);
                    }
                    // The cell is done: resume at the next cell of this row.
                    query.cx = x.saturating_add(1);
                    query.key = None;
                }
                query.cy += 1;
                query.cx = low_x;
                query.key = None;
            }
            query.stage = 1;
            query.contact = u32::MAX;
        }
        if query.stage == 1 {
            let reach = radius + self.contacts.max_radius;
            let high = ordered(point.x + grow(reach));
            if query.contact == u32::MAX {
                if !meter.step() {
                    return Probe::Pending(query);
                }
                let low = ordered(point.x - grow(reach));
                query.contact =
                    self.contacts
                        .entries
                        .partition_point(|entry| entry.0[0] < low) as u32;
            }
            while let Some((key, _)) = self.contacts.entries.get(query.contact as usize) {
                if key[0] > high {
                    break;
                }
                if !meter.step() {
                    return Probe::Pending(query);
                }
                meter.bodies += 1;
                if blocks(key) {
                    return Probe::Blocked;
                }
                query.contact += 1;
            }
        }
        Probe::Clear
    }

    /// The cost bound of `region_digest`.
    pub(crate) fn region_cost(
        &self,
        center: Vec2,
        reach: f32,
        claims: Option<(u64, &ClaimSet, f32)>,
    ) -> usize {
        1 + self.view.region_cost(center, reach)
            + claims.map_or(0, |(pool, set, claim_reach)| {
                set.region_cost(pool, center, claim_reach)
            })
    }

    /// The order-free digest of the known facts in a region: the view cells within `reach`
    /// of `center` (cell box), every contact, and the claim cells of the own pool within
    /// `claim_reach`. `None` when the meter runs out first.
    pub(crate) fn region_digest(
        &self,
        center: Vec2,
        reach: f32,
        claims: Option<(u64, &ClaimSet, f32)>,
        meter: &mut Meter,
    ) -> Option<Digest> {
        if !meter.step() {
            return None;
        }
        meter.digests += 1;
        let mut digest = self.contacts.digest;
        let cells = self.view.region_digest(center, reach, meter)?;
        add_digest(&mut digest, &cells);
        if let Some((pool, set, claim_reach)) = claims {
            let claims = set.region_digest(pool, center, claim_reach, meter)?;
            // Keep claim facts apart from body facts.
            add_digest(&mut digest, &claims.map(|term| term.rotate_left(17)));
        }
        Some(digest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(bodies: &[(Vec2, f32)]) -> KnownView {
        let members: Vec<_> = bodies
            .iter()
            .enumerate()
            .map(|(row, &(position, radius))| (row, position, radius))
            .collect();
        KnownView::build(&members, bodies.len())
    }

    #[test]
    fn ordered_bits_round_trip_and_keep_order() {
        let values = [-3.5_f32, -0.0, 0.0, 1.0e-7, 2.25, 1.0e9];
        for pair in values.windows(2) {
            assert!(ordered(pair[0]) <= ordered(pair[1]));
        }
        for value in values {
            assert_eq!(unordered(ordered(value)).to_bits(), value.to_bits());
        }
    }

    #[test]
    fn cell_digests_follow_moves_and_ignore_insertion_order() {
        let bodies = [
            (Vec2::new(1.0, 1.0), 0.5),
            (Vec2::new(1.2, 1.1), 0.5),
            (Vec2::new(3.0, 1.0), 0.5),
        ];
        let mut reversed = bodies;
        reversed.reverse();
        let set = Contacts::default();
        let digest = |view: &KnownView| {
            KnownSet {
                view,
                contacts: &set,
                own_key: None,
                ignore_soft: false,
            }
            .region_digest(Vec2::splat(2.0), 3.0, None, &mut Meter::new(usize::MAX))
            .unwrap()
        };
        let mut moved = view(&bodies);
        assert_eq!(digest(&moved), digest(&view(&reversed)));
        let before = digest(&moved);
        moved.relocate(0, bodies[0].0, Vec2::new(1.5, 1.0), 0.5);
        assert_ne!(digest(&moved), before);
        moved.relocate(0, Vec2::new(1.5, 1.0), bodies[0].0, 0.5);
        assert_eq!(digest(&moved), before);
    }

    #[test]
    fn a_probe_resumes_inside_one_dense_cell() {
        let mut bodies = vec![(Vec2::new(5.0, 5.0), 0.5)];
        // 500 tiny bodies in the query box that block nothing.
        for n in 0..500 {
            bodies.push((
                Vec2::new(5.2 + (n % 20) as f32 * 0.01, 4.8 + (n / 20) as f32 * 0.01),
                0.001,
            ));
        }
        // The blocker is the last key in canonical order.
        bodies.push((Vec2::new(5.9, 5.2), 0.2));
        let view = view(&bodies);
        let contacts = Contacts::default();
        let known = KnownSet {
            view: &view,
            contacts: &contacts,
            own_key: Some(body_key(bodies[0].0, 0.5)),
            ignore_soft: false,
        };
        let point = Vec2::new(6.1, 5.2);
        let whole = known.probe(
            KnownSet::fresh_query(point),
            0.5,
            1.0e-5,
            &mut Meter::new(usize::MAX),
        );
        assert_eq!(whole, Probe::Blocked);
        let mut query = KnownSet::fresh_query(point);
        let mut slices = 0;
        let mut used = 0;
        let result = loop {
            let mut meter = Meter::new(7);
            let result = known.probe(query, 0.5, 1.0e-5, &mut meter);
            assert!(meter.used <= 7);
            used += meter.used;
            slices += 1;
            match result {
                Probe::Pending(next) => query = next,
                done => break done,
            }
        };
        assert_eq!(result, Probe::Blocked);
        assert!(slices > 50, "the cell needs many slices: {slices}");
        assert!(used < 1_200, "a resumed probe repeats little work: {used}");
    }
}
