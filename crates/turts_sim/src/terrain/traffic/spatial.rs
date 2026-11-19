//! Exact rectangle reporting with layout-independent, nonexhaustible metadata work.
use super::envelope_index::WORDS;
#[cfg(test)]
use super::envelope_index::{query_rank_bound, query_word_bound};
use super::*;
use super::{EnvelopeIndex, MetadataWork};
const EXTRA: f64 = 1.0 + 1.0 / 262_144.0;
const TINY: f64 = 256.0 * f32::from_bits(1) as f64;
fn radius(value: f64) -> f64 {
    (value * EXTRA + TINY).next_up()
}
fn envelope(point: Vec2, reach: f64) -> [f64; 4] {
    let r = radius(reach);
    [
        (f64::from(point.x) - r).next_down(),
        (f64::from(point.x) + r).next_up(),
        (f64::from(point.y) - r).next_down(),
        (f64::from(point.y) + r).next_up(),
    ]
}
pub(super) struct Spatial {
    index: Option<EnvelopeIndex>,
    owners: [Vec<u64>; SLOTS],
    pub(super) work: MetadataWork,
}
impl Spatial {
    pub(super) const fn capacity_bound(count: usize) -> usize {
        std::mem::size_of::<Self>() + EnvelopeIndex::capacity_bound(count)
            - std::mem::size_of::<EnvelopeIndex>()
            + SLOTS * WORDS * 8
    }
    pub(super) fn new(frame: &MovementFrame) -> Self {
        let index = EnvelopeIndex::new(frame.units.len(), |i| {
            let unit = &frame.units[i];
            (unit.position.is_finite() && unit.radius.is_finite() && unit.radius > 0.)
                .then(|| envelope(unit.position, 4. * f64::from(unit.radius) * 1.000_001))
        });
        let mut owners: [Vec<u64>; SLOTS] = std::array::from_fn(|_| vec![0; WORDS]);
        let mut work = index.as_ref().map_or_default(|i| i.work);
        work.build_mask_words += (SLOTS * WORDS) as u64;
        for (i, u) in frame.units.iter().enumerate() {
            if let Some(mask) = owners.get_mut(usize::from(u.owner_slot)) {
                mask[i / 64] |= 1u64 << (i % 64);
                work.build_mask_words += 2;
            }
        }
        Self {
            index,
            owners,
            work,
        }
    }
    pub(super) fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self
                .index
                .as_ref()
                .map_or(0, |i| i.bytes() - std::mem::size_of::<EnvelopeIndex>())
            + self.owners.iter().map(|m| m.capacity() * 8).sum::<usize>()
    }
    /// Complete owner-local body candidates for one actual proposed chord.
    /// Each admitted query starts at the actor, so its own indexed disc pays
    /// at least one existing body visit. No-credit requests do no metadata work.
    pub(super) fn chord(
        &mut self,
        frame: &MovementFrame,
        i: usize,
        to: Vec2,
        visits: &mut usize,
        quota: usize,
    ) -> Option<Vec<usize>> {
        if *visits >= quota || !to.is_finite() {
            return None;
        }
        let a = &frame.units[i];
        let r = radius(f64::from(a.radius));
        let b = [
            (f64::from(a.position.x.min(to.x)) - r).next_down(),
            (f64::from(a.position.x.max(to.x)) + r).next_up(),
            (f64::from(a.position.y.min(to.y)) - r).next_down(),
            (f64::from(a.position.y.max(to.y)) + r).next_up(),
        ];
        self.owner_rectangle(a.owner_slot, b, visits, quota)
    }
    /// Certificate witnesses can surround a private point beyond its direct
    /// collision chord. Search the retained authored destination region too.
    /// This broadens discovery only; candidate legs still use exact chord guards.
    pub(super) fn destination_region(
        &mut self,
        frame: &MovementFrame,
        i: usize,
        to: Vec2,
        region: DestinationRegion,
        visits: &mut usize,
        quota: usize,
    ) -> Option<Vec<usize>> {
        if *visits >= quota
            || !to.is_finite()
            || !region.min.is_finite()
            || !region.max.is_finite()
            || region.min.x > region.max.x
            || region.min.y > region.max.y
            || !region.cohort_radius.is_finite()
            || region.cohort_radius <= 0.
        {
            return None;
        }
        let a = &frame.units[i];
        let r = radius(f64::from(a.radius));
        let layer = f64::from(region.cohort_radius) + f64::from(a.radius);
        let b = [
            (f64::from(a.position.x.min(to.x)).min(f64::from(region.min.x) - layer) - r)
                .next_down(),
            (f64::from(a.position.x.max(to.x)).max(f64::from(region.max.x) + layer) + r).next_up(),
            (f64::from(a.position.y.min(to.y)).min(f64::from(region.min.y) - layer) - r)
                .next_down(),
            (f64::from(a.position.y.max(to.y)).max(f64::from(region.max.y) + layer) + r).next_up(),
        ];
        self.owner_rectangle(a.owner_slot, b, visits, quota)
    }
    fn owner_rectangle(
        &mut self,
        owner: u8,
        b: [f64; 4],
        visits: &mut usize,
        quota: usize,
    ) -> Option<Vec<usize>> {
        let owner = self.owners.get(usize::from(owner))?;
        let index = self.index.as_mut()?;
        let rows = index.query(b, Some(owner))?;
        let result = (|| {
            let mut found = Vec::with_capacity(NEIGHBORS);
            for other in rows {
                if *visits == quota {
                    return None;
                }
                *visits += 1;
                if found.len() == NEIGHBORS {
                    return None;
                }
                found.push(other);
            }
            Some(found)
        })();
        self.work.queries = index.work.queries;
        self.work.rank_comparisons = index.work.rank_comparisons;
        self.work.tail_entries = index.work.tail_entries;
        self.work.word_ops = index.work.word_ops;
        result
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "Retain explicit existing owner work ports"
    )]
    pub(super) fn neighbors(
        &mut self,
        frame: &MovementFrame,
        i: usize,
        dt: f32,
        visits: &mut usize,
        _buckets: &mut usize,
        quota: usize,
    ) -> Option<Vec<usize>> {
        let a = &frame.units[i];
        let step = f64::from(a.speed.unwrap_or(0.) * dt);
        if !step.is_finite() || step < 0. {
            return None;
        }
        let owner = self.owners.get(usize::from(a.owner_slot))?;
        let b = envelope(a.position, 4. * f64::from(a.radius) * 1.000_001);
        let index = self.index.as_mut()?;
        let rows = index.query(b, Some(owner))?;
        let result = (|| {
            let mut found = Vec::with_capacity(NEIGHBORS);
            for other in rows {
                if *visits == quota {
                    return None;
                }
                *visits += 1;
                let p = &frame.units[other];
                let sum = f64::from(a.radius) + f64::from(p.radius);
                let reach = 4. * sum * 1.000_001;
                let dx = f64::from(p.position.x) - f64::from(a.position.x);
                let dy = f64::from(p.position.y) - f64::from(a.position.y);
                if dx * dx + dy * dy <= reach * reach {
                    if found.len() == NEIGHBORS {
                        return None;
                    }
                    found.push(other);
                }
            }
            Some(found)
        })();
        // Preserve build-owner-mask accounting while copying actual query deltas.
        self.work.queries = index.work.queries;
        self.work.rank_comparisons = index.work.rank_comparisons;
        self.work.tail_entries = index.work.tail_entries;
        self.work.word_ops = index.work.word_ops;
        result
    }
}
/// One bounded contiguous successful-result buffer. No per-query tree nodes or
/// geometric-size-dependent capacities survive extraction.
pub(super) struct NeighborRows {
    ranges: Vec<Option<(usize, usize)>>,
    rows: Vec<usize>,
}
impl NeighborRows {
    pub(super) const fn capacity_bound(count: usize) -> usize {
        std::mem::size_of::<Self>()
            + count * std::mem::size_of::<Option<(usize, usize)>>()
            + (VISITS / 2) * std::mem::size_of::<usize>()
    }
    pub(super) fn new(count: usize) -> Self {
        Self {
            ranges: vec![None; count],
            rows: Vec::with_capacity(VISITS / 2),
        }
    }
    pub(super) fn has(&self, index: usize) -> bool {
        self.ranges.get(index).is_some_and(Option::is_some)
    }
    pub(super) fn insert(&mut self, index: usize, rows: Vec<usize>) -> bool {
        if rows.len() > VISITS / 2 - self.rows.len() {
            return false;
        }
        let start = self.rows.len();
        self.rows.extend_from_slice(&rows);
        self.ranges[index] = Some((start, self.rows.len()));
        true
    }
    pub(super) fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.ranges.capacity() * std::mem::size_of::<Option<(usize, usize)>>()
            + self.rows.capacity() * std::mem::size_of::<usize>()
    }
}
impl std::ops::Index<&usize> for NeighborRows {
    type Output = [usize];
    fn index(&self, index: &usize) -> &Self::Output {
        let (start, end) =
            self.ranges[*index].expect("only complete admitted neighbor queries guide movement");
        &self.rows[start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn body(id: u32, p: Vec2, r: f32) -> MovementFrameUnit {
        MovementFrameUnit {
            unit_id: id,
            owner_slot: 0,
            unit_type: 1,
            position: p,
            radius: r,
            speed: Some(3.),
            target: Some(Vec2::new(200., 10.)),
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }
    }
    fn scene(units: Vec<MovementFrameUnit>) -> MovementFrame {
        MovementFrame {
            recovery: Default::default(),
            bounds: Vec2::splat(500_000.),
            tick_hz: 20,
            units,
            pools: BTreeMap::new(),
        }
    }
    fn expected(frame: &MovementFrame, i: usize, _dt: f32) -> Vec<usize> {
        let a = &frame.units[i];
        frame
            .units
            .iter()
            .enumerate()
            .filter_map(|(j, b)| {
                let sum = f64::from(a.radius) + f64::from(b.radius);
                let reach = 4. * sum * 1.000_001;
                let dx = f64::from(a.position.x) - f64::from(b.position.x);
                let dy = f64::from(a.position.y) - f64::from(b.position.y);
                (dx * dx + dy * dy <= reach * reach).then_some(j)
            })
            .collect()
    }
    fn bounds(p: Vec2, r: f64) -> [f64; 4] {
        let grown = (r * (1. + 2f64.powi(-18)) + 256. * f64::from(f32::from_bits(1))).next_up();
        [
            (f64::from(p.x) - grown).next_down(),
            (f64::from(p.x) + grown).next_up(),
            (f64::from(p.y) - grown).next_down(),
            (f64::from(p.y) + grown).next_up(),
        ]
    }
    #[test]
    fn destination_region_query_keeps_owner_row_cap_and_empty_credit() {
        let region = DestinationRegion {
            goal: Vec2::new(50., 24.),
            min: Vec2::new(40., 20.),
            max: Vec2::new(60., 30.),
            cohort_radius: 1.1,
        };
        let mut frame = scene(
            (0..=NEIGHBORS)
                .map(|i| body(i as u32, Vec2::new(45., 24.), 0.5))
                .collect(),
        );
        let mut spatial = Spatial::new(&frame);
        let mut visits = 0;
        assert!(
            spatial
                .destination_region(&frame, 0, region.goal, region, &mut visits, 8192)
                .is_none()
        );
        assert_eq!(
            visits,
            NEIGHBORS + 1,
            "every examined owner row is charged before overflow refusal"
        );
        for u in &mut frame.units[1..] {
            u.owner_slot = 1;
        }
        let mut spatial = Spatial::new(&frame);
        let mut visits = 0;
        assert_eq!(
            spatial.destination_region(&frame, 0, region.goal, region, &mut visits, 8192),
            Some(vec![0])
        );
        assert_eq!(
            visits, 1,
            "foreign rows do not select certificate witnesses"
        );
        let before = spatial.work;
        let mut visits = 0;
        assert!(
            spatial
                .destination_region(&frame, 0, region.goal, region, &mut visits, 0)
                .is_none()
        );
        assert_eq!(visits, 0);
        assert_eq!(spatial.work.queries, before.queries);
    }
    #[test]
    fn exact_masks_and_old_circle_results_match_exhaustive_seeded_and_reversed_oracles() {
        let mut seed = 0xb348_aec1_729d_00d1u64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for scene_id in 0..12 {
            let mut units = (0..(8 + scene_id * 7))
                .map(|i| {
                    let mut b = body(
                        i as u32,
                        Vec2::new(
                            (next() % 15000) as f32 / 100. + 10.,
                            (next() % 15000) as f32 / 100. + 10.,
                        ),
                        (next() % 300 + 5) as f32 / 100.,
                    );
                    b.speed = Some((next() % 10000) as f32 / 100.);
                    b
                })
                .collect::<Vec<_>>();
            for reverse in [false, true] {
                if reverse {
                    units.reverse();
                    for b in &mut units {
                        b.unit_id = 9000 - b.unit_id;
                    }
                }
                let frame = scene(units.clone());
                let mut index = Spatial::new(&frame);
                for i in 0..frame.units.len() {
                    let before = index.work;
                    let mut visits = 0;
                    let mut buckets = 0;
                    let mut got = index
                        .neighbors(&frame, i, 0.05, &mut visits, &mut buckets, OWNER_VISITS)
                        .unwrap();
                    got.sort_unstable();
                    assert_eq!(got, expected(&frame, i, 0.05));
                    let a = &frame.units[i];
                    let q = bounds(a.position, 4. * f64::from(a.radius) * 1.000_001);
                    for (j, b) in frame.units.iter().enumerate() {
                        let p = bounds(b.position, 4. * f64::from(b.radius) * 1.000_001);
                        let overlap = p[0] <= q[1] && p[1] >= q[0] && p[2] <= q[3] && p[3] >= q[2];
                        assert_eq!(
                            index.index.as_ref().unwrap().mask_bits()[j / 64] & (1u64 << (j % 64))
                                != 0,
                            overlap,
                            "scene{scene_id} actor{i} peer{j}"
                        );
                    }
                    assert!(
                        index.work.rank_comparisons - before.rank_comparisons
                            <= query_rank_bound() as u64
                    );
                    assert!(index.work.tail_entries - before.tail_entries <= 252);
                    assert!(index.work.word_ops - before.word_ops <= query_word_bound() as u64);
                    assert_eq!(buckets, 0, "grid buckets do not measure rank metadata");
                }
            }
        }
    }
    #[test]
    fn inclusive_tangency_equal_endpoints_and_f32_rounding_boundary_are_not_omitted() {
        let r = 1f32;
        let peer = 3. * 2f32.powi(-24);
        let old32 = (4f32 * (r + peer)) * 1.000_001f32;
        let frame = scene(vec![
            body(1, Vec2::ZERO, r),
            body(2, Vec2::new(old32, 0.), peer),
        ]);
        let mut index = Spatial::new(&frame);
        let mut visits = 0;
        let mut buckets = 0;
        let mut got = index
            .neighbors(&frame, 0, 0., &mut visits, &mut buckets, OWNER_VISITS)
            .unwrap();
        got.sort_unstable();
        assert_eq!(got, expected(&frame, 0, 0.));
        assert_ne!(
            index.index.as_ref().unwrap().mask_bits()[0] & 2,
            0,
            "envelopes also contain the nearest-rounded f32 reference"
        );
        for x in [4f32.next_down(), 4., 4f32.next_up(), f32::from_bits(1)] {
            let frame = scene(vec![
                body(1, Vec2::new(10., 10.), 0.5),
                body(2, Vec2::new(10. + x, 10.), 0.5),
            ]);
            let mut index = Spatial::new(&frame);
            let mut v = 0;
            let mut b = 0;
            let mut got = index
                .neighbors(&frame, 0, 0., &mut v, &mut b, OWNER_VISITS)
                .unwrap();
            got.sort_unstable();
            assert_eq!(got, expected(&frame, 0, 0.));
        }
        let frame = scene((0..64).map(|i| body(i, Vec2::new(10., 10.), 0.5)).collect());
        let mut index = Spatial::new(&frame);
        let mut v = 0;
        let mut b = 0;
        assert_eq!(
            index
                .neighbors(&frame, 0, 0., &mut v, &mut b, OWNER_VISITS)
                .unwrap()
                .len(),
            64
        );
    }
    #[test]
    fn distant_hidden_large_radius_and_remote_pressure_do_not_spend_protected_quota() {
        let run = |large: bool, reverse: bool| {
            let mut units = vec![
                body(1, Vec2::new(10., 10.), 0.4),
                body(2, Vec2::new(12., 10.), 0.4),
            ];
            units[1].target = Some(Vec2::new(2., 10.));
            for i in 0..400 {
                let mut b = body(
                    100 + i,
                    Vec2::new(150. + (i % 20) as f32, 10. + (i / 20) as f32),
                    0.4,
                );
                b.owner_slot = 1;
                b.speed = None;
                b.target = None;
                units.push(b);
            }
            if large {
                let mut b = body(900, Vec2::new(1000., 10.), 50.);
                b.owner_slot = 1;
                b.speed = None;
                b.target = None;
                units.push(b);
            }
            if reverse {
                units.reverse();
                for b in &mut units {
                    b.unit_id = 5000 - b.unit_id;
                }
            }
            let frame = scene(units);
            let a = frame
                .units
                .iter()
                .position(|u| u.position == Vec2::new(10., 10.))
                .unwrap();
            let mut index = Spatial::new(&frame);
            let mut visits = 0;
            let mut buckets = 0;
            let got = index
                .neighbors(&frame, a, 0.05, &mut visits, &mut buckets, 2)
                .unwrap();
            assert_eq!(got.len(), 2);
            assert_eq!(visits, 2);
            let mut proposed = frame.clone();
            let mut state = TrafficState::default();
            let waiting = apply(&frame, &mut proposed, 0.05, 1, &mut state, |_, _, _| true);
            let mut facts = proposed
                .units
                .iter()
                .filter(|u| u.owner_slot == 0)
                .map(|u| {
                    (
                        u.position.to_array().map(f32::to_bits),
                        u.target.map(|v| v.to_array().map(f32::to_bits)),
                        u.speed.map(f32::to_bits),
                        waiting.contains(&u.unit_id),
                    )
                })
                .collect::<Vec<_>>();
            facts.sort();
            (facts, state.tick_owner_neighbor_work(0).unwrap())
        };
        let reference = run(false, false);
        for large in [false, true] {
            for reverse in [false, true] {
                assert_eq!(run(large, reverse), reference);
            }
        }
    }
    #[test]
    fn relevant_dense_overflow_discards_the_query_and_public_cap_fits_eight_mib() {
        let frame = scene(
            (0..129)
                .map(|i| body(i, Vec2::new(10., 10.), 0.5))
                .collect(),
        );
        let mut index = Spatial::new(&frame);
        let mut v = 0;
        let mut b = 0;
        assert!(
            index
                .neighbors(&frame, 0, 0.05, &mut v, &mut b, OWNER_VISITS)
                .is_none()
        );
        assert_eq!(v, 129);
        let frame = scene(
            (0..INDEX_BODIES)
                .map(|i| body(i as u32, Vec2::new(10. + i as f32 * 20., 10.), 0.5))
                .collect(),
        );
        let mut index = Spatial::new(&frame);
        let mut visits = 0;
        let mut buckets = 0;
        assert_eq!(
            index
                .neighbors(
                    &frame,
                    INDEX_BODIES - 1,
                    0.05,
                    &mut visits,
                    &mut buckets,
                    OWNER_VISITS
                )
                .unwrap(),
            vec![INDEX_BODIES - 1]
        );
        let rows = NeighborRows::new(frame.units.len());
        let peak = RETAINED_ALLOWANCE
            + INDEX_BODIES * PER_BODY_SCRATCH
            + FIXED_SCRATCH
            + index.bytes()
            + rows.bytes();
        println!(
            "EXACT_INDEX_ALLOCATION capacity={INDEX_BODIES} peak={peak} index={} neighbor_rows={} limit={SCRATCH_BYTES}",
            index.bytes(),
            rows.bytes()
        );
        assert!(peak <= SCRATCH_BYTES);
        assert_eq!(
            INDEX_BODIES, 12_870,
            "preserve the measured prior physical-row cap"
        );
        assert!(index.work.build_prefix_words <= 4 * ((INDEX_BODIES / 64 + 1) * WORDS) as u64);
    }
}

#[cfg(test)]
mod lazy_tests {
    use super::*;
    fn body(id: u32, owner: u8, p: Vec2, target: Option<Vec2>) -> MovementFrameUnit {
        MovementFrameUnit {
            unit_id: id,
            owner_slot: owner,
            unit_type: 1,
            position: p,
            radius: 0.3,
            speed: Some(100.),
            target,
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }
    }
    #[test]
    fn zero_length_chords_pay_self_and_no_credit_does_no_metadata() {
        let frame = MovementFrame {
            recovery: Default::default(),
            bounds: Vec2::splat(128.),
            tick_hz: 20,
            pools: BTreeMap::new(),
            units: vec![body(1, 0, Vec2::new(10., 10.), Some(Vec2::new(20., 10.)))],
        };
        let mut index = Spatial::new(&frame);
        let mut visits = 0;
        assert_eq!(
            index.chord(&frame, 0, frame.units[0].position, &mut visits, 1),
            Some(vec![0])
        );
        assert_eq!(visits, 1);
        let work = index.work;
        assert!(
            index
                .chord(&frame, 0, frame.units[0].position, &mut visits, 1)
                .is_none()
        );
        assert_eq!(index.work, work);
        assert!(EnvelopeIndex::new(1, |_| None).is_none());
        assert!(EnvelopeIndex::new(1, |_| Some([f64::NAN, 1., 0., 1.])).is_none());
    }
    #[test]
    fn fast_parallel_cohort_and_remote_spotters_keep_discovery_local() {
        let mut units = Vec::new();
        for owner in 0..2 {
            let lane = 32. + 64. * f32::from(owner);
            for n in 0..250 {
                let p = Vec2::new(
                    26.75 + (n % 16) as f32 * 0.7,
                    lane - 5.25 + (n / 16) as f32 * 0.7,
                );
                units.push(body(
                    1 + u32::from(owner) * 500 + n,
                    owner,
                    p,
                    Some(p + Vec2::X * 48.),
                ));
            }
            for n in 0..250 {
                let p = Vec2::new(
                    6. + (n % 16) as f32 * 0.7,
                    lane - 5.25 + (n / 16) as f32 * 0.7,
                );
                units.push(body(251 + u32::from(owner) * 500 + n, owner, p, None));
            }
            units.push(body(
                1035 + u32::from(owner),
                owner,
                Vec2::new(118.5, 54.5 + 10. * f32::from(owner)),
                Some(Vec2::new(118.5, 52.5 + 10. * f32::from(owner))),
            ));
        }
        let frame = MovementFrame {
            recovery: Default::default(),
            bounds: Vec2::splat(128.),
            tick_hz: 20,
            pools: BTreeMap::new(),
            units,
        };
        let mut state = TrafficState::default();
        let mut proposed = frame.clone();
        let waiting = apply(&frame, &mut proposed, 0.05, 1, &mut state, |_, _, _| true);
        assert!(
            waiting.is_empty(),
            "parallel requests need no long-range cooperative search"
        );
        assert_eq!(state.neighbor_overflows, 0);
        for (before, after) in frame.units.iter().zip(proposed.units.iter()) {
            if before.target.is_some() {
                assert_eq!(after.target, before.target);
                assert_eq!(after.speed, before.speed);
            }
        }
        assert!(state.tick_neighbors <= VISITS / 2);
        assert_eq!(state.index_work.queries, 502);
    }
}
