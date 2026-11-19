//! Owner-isolated accounting and exact chord-envelope candidate storage.
//! Queries filter earlier certified chords and later original discs.
//! Public size bounds never depend on a global radius or motion maximum.
use super::*;
const SLOTS: usize = turts_mod_api::SLOT_COUNT as usize;
pub(crate) const WORK: usize = 65_536;
const OWNER_WORK: usize = WORK / SLOTS;
pub(super) const PEERS: usize = 128;

pub(super) const PAIRS_PER_ACTOR: usize = 4096;
pub(crate) const SCRATCH_BYTES: usize = 5 * 1024 * 1024;

#[derive(Clone, Debug)]
pub(crate) struct SweepBudget {
    pub(crate) epoch: u32,
    pub(crate) bodies: [usize; SLOTS],
    pub(crate) buckets: [usize; SLOTS],
    pub(crate) pairs: [usize; SLOTS],
    pub(crate) overflows: usize,
    pub(crate) peak_bytes: usize,
    pub(crate) index_work: crate::terrain::traffic::MetadataWork,
    pub(crate) envelope_filters: u64,
    pub(crate) index_rebuilds: u64,
    pub(crate) index_updates: u64,
    pub(crate) index_update_entries: u64,
    pub(crate) coherence_members: u64,
    pub(crate) coherence_attempts: u64,
    pub(crate) coherence_operations: u64,
    pub(crate) coherence_advice: u64,
    pub(crate) model_reserved: [usize; SLOTS],
    pub(crate) model_attempts: u64,
    pub(crate) model_operations: u64,
    pub(crate) model_fallbacks: u64,
    pub(crate) model_view_refusals: u64,
    pub(crate) support_queries: u64,
    pub(crate) support_pairs: u64,
    pub(crate) support_failures: u64,
    pub(crate) support_accepted_members: u64,
    pub(crate) recovery: crate::movement::recovery::RecoveryCounters,
}
impl SweepBudget {
    pub(crate) fn new(epoch: u32) -> Self {
        Self {
            epoch,
            bodies: [0; SLOTS],
            buckets: [0; SLOTS],
            pairs: [0; SLOTS],
            overflows: 0,
            peak_bytes: 0,
            index_work: Default::default(),
            envelope_filters: 0,
            index_rebuilds: 0,
            index_updates: 0,
            index_update_entries: 0,
            coherence_members: 0,
            coherence_attempts: 0,
            coherence_operations: 0,
            coherence_advice: 0,
            model_reserved: [0; SLOTS],
            model_attempts: 0,
            model_operations: 0,
            model_fallbacks: 0,
            model_view_refusals: 0,
            support_queries: 0,
            support_pairs: 0,
            support_failures: 0,
            support_accepted_members: 0,
            recovery: Default::default(),
        }
    }
    pub(crate) fn can_charge_pair(&self, owner: u8, spent: usize, allowance: usize) -> bool {
        spent < allowance
            && self
                .pairs
                .get(usize::from(owner))
                .is_some_and(|&n| n < OWNER_WORK)
    }
    pub(crate) fn charge_pair(&mut self, owner: u8, actor: &mut usize, allowance: usize) -> bool {
        let Some(count) = self.pairs.get_mut(usize::from(owner)) else {
            return false;
        };
        if *count == OWNER_WORK || *actor == allowance {
            return false;
        }
        *count += 1;
        *actor += 1;
        true
    }
    pub(crate) fn model_available(&self, owner: u8) -> usize {
        let i = usize::from(owner);
        if i >= SLOTS {
            0
        } else {
            (OWNER_WORK / 2).saturating_sub(self.pairs[i] + self.model_reserved[i])
        }
    }
    pub(crate) fn reserve_model_amount(&mut self, owner: u8, amount: usize) -> bool {
        let i = usize::from(owner);
        if amount > self.model_available(owner) {
            return false;
        }
        let Some(reserved) = self.model_reserved.get_mut(i) else {
            return false;
        };
        *reserved += amount;
        true
    }
    pub(crate) fn release_model(&mut self, owner: u8, n: usize) {
        self.model_reserved[usize::from(owner)] -= n;
    }
    /// Optional local proposals share the pair pool and leave at least half
    /// of each owner's original credit for final physical certification.
    pub(crate) fn charge_guidance_pair(&mut self, owner: u8, actor: &mut usize) -> bool {
        if self
            .pairs
            .get(usize::from(owner))
            .is_none_or(|&used| used + self.model_reserved[usize::from(owner)] >= OWNER_WORK / 2)
        {
            return false;
        }
        self.charge_pair(owner, actor, PAIRS_PER_ACTOR)
    }
    pub(crate) fn body(&mut self, owner: u8) -> bool {
        charge(&mut self.bodies, owner)
    }
}
fn charge(rows: &mut [usize; SLOTS], owner: u8) -> bool {
    let Some(count) = rows.get_mut(usize::from(owner)) else {
        return false;
    };
    // Half of the shared neighbor envelope is reserved for traffic guidance.
    if *count >= OWNER_WORK / 2 {
        return false;
    }
    *count += 1;
    true
}
/// Outward-rounded envelope of the actual closed chord and its disk.
pub(super) fn envelope(from: Vec2, to: Vec2, radius: f32) -> Option<[f64; 4]> {
    if !from.is_finite() || !to.is_finite() || !radius.is_finite() || radius <= 0.0 {
        return None;
    }
    let r = f64::from(radius);
    Some([
        (f64::from(from.x.min(to.x)) - r).next_down(),
        (f64::from(from.x.max(to.x)) + r).next_up(),
        (f64::from(from.y.min(to.y)) - r).next_down(),
        (f64::from(from.y.max(to.y)) + r).next_up(),
    ])
}
pub(super) fn intersects(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] <= b[1] && b[0] <= a[1] && a[2] <= b[3] && b[2] <= a[3]
}
fn contains(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0] <= b[0] && a[1] >= b[1] && a[2] <= b[2] && a[3] >= b[3]
}
pub(super) struct Candidates {
    rows: Vec<usize>,
    ranges: Vec<Option<(usize, usize)>>,
    allowances: Vec<usize>,
    envelopes: Vec<[f64; 4]>,
    index: Option<crate::terrain::traffic::EnvelopeIndex>,
    committed_mask: Vec<u64>,
    query_mask: Vec<u64>,
}
impl Candidates {
    pub(super) fn index_work(&self) -> Option<crate::terrain::traffic::MetadataWork> {
        self.index.as_ref().map(|i| i.work)
    }
    pub(super) fn get(&self, i: usize) -> Option<(&[usize], usize)> {
        let (a, b) = self.ranges.get(i).copied().flatten()?;
        Some((&self.rows[a..b], self.allowances[i]))
    }
    /// Queries use certified chords for earlier actors and original disks for
    /// later actors. Cheap planning may reuse fully charged original rows.
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(super) fn query(
        &mut self,
        actor: usize,
        to: Vec2,
        units: &[MovementFrameUnit],
        origins: &[Vec2],
        outputs: &[MovementFrameOutput],
        seen: &mut Vec<usize>,
        committed: &[bool],
        budget: &mut SweepBudget,
    ) -> Option<Vec<usize>> {
        self.query_guided(
            actor,
            to,
            units,
            origins,
            outputs,
            seen,
            committed,
            budget,
            &crate::movement::motion::NoMotionGuide,
        )
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "Keep captured geometry and shared accounting explicit"
    )]
    pub(super) fn query_guided(
        &mut self,
        actor: usize,
        to: Vec2,
        units: &[MovementFrameUnit],
        origins: &[Vec2],
        outputs: &[MovementFrameOutput],
        seen: &mut Vec<usize>,
        committed: &[bool],
        budget: &mut SweepBudget,
        guide: &dyn crate::movement::motion::MotionGuide,
    ) -> Option<Vec<usize>> {
        self.get(actor)?;
        let bounds = envelope(origins[actor], to, units[actor].radius)?;
        let mut result = Vec::with_capacity(PEERS);
        if !guide.charge_certificate_visit(
            units[actor].unit_id,
            units[actor].owner_slot,
            false,
            budget,
        ) {
            return None;
        }
        let cached = !guide.certificate_search_enabled()
            && !committed.get(actor).copied().unwrap_or(false)
            && contains(*self.envelopes.get(actor)?, bounds);
        let mut accept = |other: usize, budget: &mut SweepBudget| -> Option<()> {
            if other == actor {
                return Some(());
            }
            budget.envelope_filters = budget.envelope_filters.saturating_add(1);
            let end = if committed.get(other).copied().unwrap_or(false) {
                outputs[other].position
            } else {
                origins[other]
            };
            if !intersects(bounds, envelope(origins[other], end, units[other].radius)?) {
                return Some(());
            }
            if result.len() == PEERS {
                return None;
            }
            let unseen = !seen.contains(&other);
            if unseen && seen.len() == PEERS {
                return None;
            }
            if !guide.admit_certificate_row(
                units[actor].unit_id,
                units[actor].owner_slot,
                unseen,
                budget,
            ) {
                return None;
            }
            if unseen {
                seen.push(other);
            }
            result.push(other);
            Some(())
        };
        if cached {
            self.index.as_mut()?.work.word_ops += (2 * self.query_mask.len()) as u64;
            self.query_mask.copy_from_slice(&self.committed_mask);
            let (start, end) = self.ranges[actor]?;
            for &other in &self.rows[start..end] {
                // Pay every retained row before looking at its commit bit or geometry.
                if !guide.examine_certificate_row(
                    units[actor].unit_id,
                    units[actor].owner_slot,
                    budget,
                ) {
                    return None;
                }
                self.query_mask[other / 64] &= !(1u64 << (other % 64));
                self.index.as_mut()?.work.word_ops += 2;
                accept(other, budget)?;
            }
        }
        let index = self.index.as_mut()?;
        for other in index.query(bounds, cached.then_some(self.query_mask.as_slice()))? {
            if !guide.examine_certificate_row(units[actor].unit_id, units[actor].owner_slot, budget)
            {
                return None;
            }
            accept(other, budget)?;
        }
        if cached {
            sort_physical(
                &mut result,
                units,
                origins,
                outputs,
                committed,
                &mut index.work.build_comparisons,
            );
        }
        Some(result)
    }
    pub(super) fn mark_committed(&mut self, actor: usize) {
        if let Some(word) = self.committed_mask.get_mut(actor / 64) {
            *word |= 1u64 << (actor % 64);
            if let Some(index) = self.index.as_mut() {
                index.work.word_ops = index.work.word_ops.saturating_add(2);
            }
        }
    }
    pub(super) fn committed(
        &mut self,
        actor: usize,
        from: Vec2,
        to: Vec2,
        radius: f32,
        budget: &mut SweepBudget,
    ) {
        self.mark_committed(actor);
        let Some(actual) = envelope(from, to, radius) else {
            self.index = None;
            return;
        };
        let Some(stored) = self.envelopes.get(actor).copied() else {
            return;
        };
        if contains(stored, actual) {
            return;
        }
        self.envelopes[actor] = actual;
        if let Some(index) = self.index.as_mut()
            && let Some(entries) = index.replace_bounds(actor, actual)
        {
            budget.index_updates = budget.index_updates.saturating_add(1);
            budget.index_update_entries =
                budget.index_update_entries.saturating_add(entries as u64);
            return;
        }
        if let Some(old) = self.index.take() {
            budget.index_work.add(old.work);
            drop(old);
        }
        budget.index_rebuilds = budget.index_rebuilds.saturating_add(1);
        self.index = crate::terrain::traffic::EnvelopeIndex::new(self.envelopes.len(), |i| {
            Some(self.envelopes[i])
        });
    }
}
fn sort_physical(
    rows: &mut [usize],
    units: &[MovementFrameUnit],
    origins: &[Vec2],
    outputs: &[MovementFrameOutput],
    committed: &[bool],
    comparisons: &mut u64,
) {
    let mut scratch = [0usize; PEERS];
    let end = |i: usize| {
        if committed[i] {
            outputs[i].position
        } else {
            origins[i]
        }
    };
    let cmp = |a: usize, b: usize| {
        origins[a]
            .x
            .total_cmp(&origins[b].x)
            .then(origins[a].y.total_cmp(&origins[b].y))
            .then(units[a].radius.total_cmp(&units[b].radius))
            .then(end(a).x.total_cmp(&end(b).x))
            .then(end(a).y.total_cmp(&end(b).y))
    };
    let mut width = 1;
    while width < rows.len() {
        let mut start = 0;
        while start < rows.len() {
            let middle = (start + width).min(rows.len());
            let stop = (middle + width).min(rows.len());
            let (mut a, mut b) = (start, middle);
            for slot in &mut scratch[start..stop] {
                let left = if a == middle {
                    false
                } else if b == stop {
                    true
                } else {
                    *comparisons = comparisons.saturating_add(1);
                    cmp(rows[a], rows[b]) != std::cmp::Ordering::Greater
                };
                if left {
                    *slot = rows[a];
                    a += 1;
                } else {
                    *slot = rows[b];
                    b += 1;
                }
            }
            rows[start..stop].copy_from_slice(&scratch[start..stop]);
            start = stop;
        }
        width *= 2;
    }
}
fn service(units: &[MovementFrameUnit], epoch: u32) -> Vec<usize> {
    let mut order = Vec::with_capacity(units.len());
    for owner in 0..SLOTS {
        let begin = order.len();
        order.extend(
            units
                .iter()
                .enumerate()
                .filter_map(|(i, u)| (usize::from(u.owner_slot) == owner).then_some(i)),
        );
        let count = order.len() - begin;
        if count > 0 {
            order[begin..].rotate_left(epoch as usize % count);
        }
    }
    order
}
#[cfg(test)]
pub(super) fn collect(
    units: &[MovementFrameUnit],
    origins: &[Vec2],
    outputs: &mut [MovementFrameOutput],
    bounds: Vec2,
    budget: &mut SweepBudget,
) -> Candidates {
    collect_guided(
        units,
        origins,
        outputs,
        bounds,
        budget,
        &crate::movement::motion::NoMotionGuide,
    )
}

#[cfg(test)]
pub(super) fn collect_guided(
    units: &[MovementFrameUnit],
    origins: &[Vec2],
    outputs: &mut [MovementFrameOutput],
    bounds: Vec2,
    budget: &mut SweepBudget,
    guide: &dyn crate::movement::motion::MotionGuide,
) -> Candidates {
    collect_guided_deferred(units, origins, outputs, bounds, budget, guide, &[])
}
#[allow(
    clippy::too_many_arguments,
    reason = "Deferred permission is not an ordinary admission range"
)]
pub(super) fn collect_guided_deferred(
    units: &[MovementFrameUnit],
    origins: &[Vec2],
    outputs: &mut [MovementFrameOutput],
    bounds: Vec2,
    budget: &mut SweepBudget,
    guide: &dyn crate::movement::motion::MotionGuide,
    deferred: &[bool],
) -> Candidates {
    let n = units.len();
    let mut result = Candidates {
        rows: Vec::new(),
        ranges: Vec::new(),
        allowances: Vec::new(),
        envelopes: Vec::new(),
        index: None,
        committed_mask: Vec::new(),
        query_mask: Vec::new(),
    };
    if n > crate::terrain::traffic::index_capacity()
        || !bounds.is_finite()
        || bounds.min_element() <= 0.0
    {
        budget.overflows += n;
        return result;
    }
    let bytes = crate::terrain::traffic::EnvelopeIndex::capacity_bound(n)
        + n * (32 + std::mem::size_of::<Option<(usize, usize)>>() + 34)
        + WORK / 2 * std::mem::size_of::<usize>()
        + 2 * n.div_ceil(64) * std::mem::size_of::<u64>()
        + PEERS * std::mem::size_of::<usize>()
        + 4096;
    if bytes > SCRATCH_BYTES {
        budget.overflows += n;
        return result;
    }
    budget.peak_bytes = budget.peak_bytes.max(bytes);
    result.committed_mask = vec![0; n.div_ceil(64)];
    result.query_mask = vec![0; n.div_ceil(64)];
    budget.index_work.word_ops += (2 * n.div_ceil(64)) as u64;
    // Explicit capacity matches the envelope bytes reserved above.
    let mut envelopes = Vec::with_capacity(n);
    for (i, unit) in units.iter().enumerate() {
        let Some(bounds) = envelope(origins[i], outputs[i].position, unit.radius) else {
            budget.overflows += n;
            return result;
        };
        envelopes.push(bounds);
    }
    result.envelopes = envelopes;
    // Admission only needs original discs. Future raw chords are queried later,
    // after certification starts. Indexing them here wastes charged extraction
    // credit on bodies which cannot participate in this admission predicate.
    result.index = crate::terrain::traffic::EnvelopeIndex::new(n, |i| {
        envelope(origins[i], origins[i], units[i].radius)
    });
    if result.index.is_none() {
        budget.overflows += n;
        return result;
    }
    result.rows = Vec::with_capacity(WORK / 2);
    result.ranges = vec![None; n];
    result.allowances = vec![0; n];
    let order = service(units, budget.epoch);
    for &i in &order {
        if deferred.get(i).copied().unwrap_or(false) || outputs[i].position == origins[i] {
            continue;
        }
        let begin = result.rows.len();
        let mut complete =
            guide.charge_certificate_visit(units[i].unit_id, units[i].owner_slot, false, budget);
        if !complete {
            budget.overflows += 1;
            continue;
        }
        for j in result
            .index
            .as_mut()
            .unwrap()
            .query(result.envelopes[i], None)
            .unwrap()
        {
            if !guide.examine_certificate_row(units[i].unit_id, units[i].owner_slot, budget) {
                complete = false;
                break;
            }
            if i == j {
                continue;
            }
            budget.envelope_filters = budget.envelope_filters.saturating_add(1);
            // Admission charges only original discs. Earlier raw proposals
            // may later stop far away; their motion cannot consume this
            // actor's body quota before it is actually certified.
            let end = origins[j];
            if !intersects(
                result.envelopes[i],
                envelope(origins[j], end, units[j].radius).unwrap(),
            ) {
                continue;
            }
            if result.rows.len() - begin == PEERS
                || !guide.admit_certificate_row(units[i].unit_id, units[i].owner_slot, true, budget)
            {
                complete = false;
                break;
            }
            result.rows.push(j);
        }
        if complete {
            result.ranges[i] = Some((begin, result.rows.len()));
        } else {
            result.rows.truncate(begin);
            budget.overflows += 1;
        }
    }
    // Only one index is live at a time. Preserve all admission metadata before
    // building the complete raw-chord index used by later certification.
    if let Some(originals) = result.index.take() {
        budget.index_work.add(originals.work);
        drop(originals);
    }
    result.index = crate::terrain::traffic::EnvelopeIndex::new(n, |i| Some(result.envelopes[i]));
    for owner in 0..SLOTS {
        let count = order
            .iter()
            .filter(|&&i| usize::from(units[i].owner_slot) == owner && result.ranges[i].is_some())
            .count();
        if count == 0 {
            continue;
        }
        // Original-disc checks are already bounded by this owner's body credit.
        // Share optional credit across every admitted actor: spending it all on
        // the first few actors starves later lazy queries whose certified peers
        // were not present in original-disc admission.
        let minimum = order
            .iter()
            .filter(|&&i| usize::from(units[i].owner_slot) == owner)
            .filter_map(|&i| result.get(i).map(|(rows, _)| rows.len()))
            .sum::<usize>();
        let extra = OWNER_WORK.saturating_sub(budget.pairs[owner] + minimum);
        let (share, remainder) = (extra / count, extra % count);
        let mut rank = 0;
        for &i in &order {
            if usize::from(units[i].owner_slot) != owner {
                continue;
            }
            if let Some((a, b)) = result.ranges[i] {
                result.allowances[i] =
                    (b - a + share + usize::from(rank < remainder)).min(PAIRS_PER_ACTOR);
                rank += 1;
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(super) fn body(id: u32, owner: u8, position: Vec2, radius: f32) -> MovementFrameUnit {
        MovementFrameUnit {
            unit_id: id,
            owner_slot: owner,
            unit_type: 0,
            position,
            radius,
            speed: Some(1.0),
            target: Some(position + Vec2::X),
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }
    }
    pub(super) fn outputs(units: &[MovementFrameUnit]) -> Vec<MovementFrameOutput> {
        units
            .iter()
            .map(|u| MovementFrameOutput {
                recovery: Default::default(),
                unit_id: u.unit_id,
                position: u.position + Vec2::X * 0.1,
                target: u.target,
                progress: None,
                plain_progress: None,
                physical_escape: None,
                failed_blocked: false,
                arrival_slot: None,
                #[cfg(feature = "test-probes")]
                probe: Default::default(),
            })
            .collect()
    }
    #[test]
    fn malformed_whole_index_keeps_commit_path_fail_closed() {
        let units = vec![body(1, 0, Vec2::ZERO, f32::NAN)];
        let origins = vec![Vec2::ZERO];
        let mut ends = outputs(&units);
        let mut budget = SweepBudget::new(0);
        let mut candidates = collect(&units, &origins, &mut ends, Vec2::splat(32.), &mut budget);
        assert!(candidates.get(0).is_none());
        candidates.committed(0, Vec2::ZERO, Vec2::ZERO, 0.2, &mut budget);
        assert!(
            candidates
                .query(
                    0,
                    Vec2::X,
                    &units,
                    &origins,
                    &ends,
                    &mut Vec::new(),
                    &vec![true; units.len()],
                    &mut budget
                )
                .is_none()
        );
    }
    #[test]
    fn update_refusal_uses_the_existing_complete_rebuild_fallback() {
        let units = vec![
            body(9, 0, Vec2::ZERO, 0.2),
            body(2, 0, Vec2::new(0., 2.), 0.2),
        ];
        let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
        let mut ends = outputs(&units);
        let mut budget = SweepBudget::new(0);
        let mut candidates = collect(&units, &origins, &mut ends, Vec2::splat(32.), &mut budget);
        // A mismatched index cannot accept row1; the complete frame remains available.
        candidates.index =
            crate::terrain::traffic::EnvelopeIndex::new(1, |i| Some(candidates.envelopes[i]));
        ends[1].position = Vec2::new(0., 0.6);
        candidates.committed(
            1,
            origins[1],
            ends[1].position,
            units[1].radius,
            &mut budget,
        );
        assert_eq!(budget.index_updates, 0);
        assert_eq!(budget.index_rebuilds, 1);
        let mut seen = Vec::new();
        assert_eq!(
            candidates
                .query(
                    0,
                    Vec2::Y,
                    &units,
                    &origins,
                    &ends,
                    &mut seen,
                    &[false, true],
                    &mut budget
                )
                .unwrap(),
            vec![1]
        );
    }
    #[test]
    fn actual_alternative_requeries_and_escaped_accepted_chord_updates() {
        let units = vec![
            body(9, 0, Vec2::ZERO, 0.2),
            body(2, 0, Vec2::new(0., 2.), 0.2),
        ];
        let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
        let mut ends = outputs(&units);
        let mut budget = SweepBudget::new(0);
        let mut candidates = collect(&units, &origins, &mut ends, Vec2::splat(32.), &mut budget);
        assert!(candidates.get(0).unwrap().0.is_empty());
        let mut seen = Vec::new();
        assert_eq!(
            candidates
                .query(
                    0,
                    Vec2::new(0., 2.),
                    &units,
                    &origins,
                    &ends,
                    &mut seen,
                    &vec![true; units.len()],
                    &mut budget
                )
                .unwrap(),
            vec![1]
        );
        ends[0].position = Vec2::new(0., 1.7);
        candidates.committed(
            0,
            origins[0],
            ends[0].position,
            units[0].radius,
            &mut budget,
        );
        assert_eq!(budget.index_rebuilds, 0);
        assert_eq!(budget.index_updates, 1);
        assert!(budget.index_update_entries > 0);
        assert_eq!(
            candidates
                .query(
                    1,
                    ends[1].position,
                    &units,
                    &origins,
                    &ends,
                    &mut Vec::new(),
                    &vec![true; units.len()],
                    &mut budget
                )
                .unwrap(),
            vec![0]
        );
        assert!(budget.index_work.build_comparisons > 0);
    }
    #[test]
    fn later_raw_crossing_does_not_spend_original_disc_service_credit() {
        let units = vec![
            body(7, 0, Vec2::ZERO, 0.2),
            body(1, 1, Vec2::new(10., 0.), 0.2),
        ];
        let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
        let mut ends = outputs(&units);
        ends[1].position = Vec2::new(-10., 0.);
        let mut budget = SweepBudget::new(0);
        let mut candidates = collect(&units, &origins, &mut ends, Vec2::splat(32.), &mut budget);
        assert_eq!(budget.bodies[0], 0);
        assert!(
            candidates
                .query(
                    0,
                    ends[0].position,
                    &units,
                    &origins,
                    &ends,
                    &mut Vec::new(),
                    &vec![false; units.len()],
                    &mut budget
                )
                .unwrap()
                .is_empty()
        );
        assert_eq!(budget.bodies[0], 0);
    }
    #[test]
    fn distant_fast_foreign_motion_does_not_expand_local_queries_or_spend_own_quota() {
        let local = vec![
            body(8, 0, Vec2::new(2., 2.), 0.4),
            body(3, 0, Vec2::new(3., 2.), 0.4),
        ];
        let origins = local.iter().map(|u| u.position).collect::<Vec<_>>();
        let mut a = SweepBudget::new(7);
        let first = collect(
            &local,
            &origins,
            &mut outputs(&local),
            Vec2::splat(20_000.),
            &mut a,
        );
        let mut units = local.clone();
        units.push(body(77, 1, Vec2::splat(10_000.), 0.4));
        let mut ends = outputs(&units);
        ends[2].position += Vec2::X * 1000.;
        let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
        let mut b = SweepBudget::new(7);
        let second = collect(&units, &origins, &mut ends, Vec2::splat(20_000.), &mut b);
        assert_eq!(first.get(0).unwrap().0, second.get(0).unwrap().0);
        assert_eq!((a.bodies[0], a.buckets[0]), (b.bodies[0], b.buckets[0]));
        assert!(
            second.get(2).is_some(),
            "distant motion uses its own fixed public index level"
        );
        // Synthetic index-pressure unit case: all far foreign circles remain
        // separated by their100-unit diameters. Their large radii change only
        // their own public level, never the local grid or its query allowance.
        let public_bounds = Vec2::splat(1_000_000.);
        let mut baseline = SweepBudget::new(7);
        let baseline_rows = collect(
            &local,
            &local.iter().map(|u| u.position).collect::<Vec<_>>(),
            &mut outputs(&local),
            public_bounds,
            &mut baseline,
        );
        let mut crowded = local.clone();
        for n in 0..crate::terrain::traffic::index_capacity() - local.len() - 1 {
            crowded.push(body(
                n as u32 + 100,
                1,
                Vec2::new(
                    10_000. + (n % 100) as f32 * 100.,
                    20_000. + (n / 100) as f32 * 100.,
                ),
                50.,
            ));
        }
        let crowded_origins = crowded.iter().map(|u| u.position).collect::<Vec<_>>();
        let mut ends = outputs(&crowded);
        for i in local.len()..ends.len() {
            ends[i].position = crowded[i].position;
        }
        let mut pressure = SweepBudget::new(7);
        let pressure_rows = collect(
            &crowded,
            &crowded_origins,
            &mut ends,
            public_bounds,
            &mut pressure,
        );
        assert_eq!(
            baseline_rows.get(0).unwrap().0,
            pressure_rows.get(0).unwrap().0
        );
        assert_eq!(
            (baseline.bodies[0], baseline.buckets[0]),
            (pressure.bodies[0], pressure.buckets[0])
        );
        assert!(pressure.peak_bytes <= SCRATCH_BYTES);
    }
    #[test]
    fn true_dense_query_fails_closed_without_an_unbounded_neighbor_vector() {
        let mut units = vec![body(1, 0, Vec2::new(-1.5, 0.), 0.05)];
        for n in 0..140 {
            units.push(body(
                n + 2,
                1,
                Vec2::new((n % 14) as f32 * 0.08, (n / 14) as f32 * 0.004 - 0.018),
                0.002,
            ));
        }
        let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
        let mut ends = outputs(&units);
        for (u, e) in units.iter().zip(&mut ends) {
            e.position = u.position;
        }
        ends[0].position = Vec2::new(1.5, 0.);
        let mut budget = SweepBudget::new(0);
        let result = collect(
            &units,
            &origins,
            &mut ends,
            Vec2::splat(20_000.),
            &mut budget,
        );
        assert!(result.get(0).is_none());
        assert!(budget.overflows > 0);
        assert!(result.rows.len() <= WORK / 2);
        assert!(budget.peak_bytes <= SCRATCH_BYTES);
    }
    #[test]
    fn exhausted_service_rotates_physically_and_is_independent_of_handle_values() {
        let units = (0..8)
            .map(|n| body(100 - n, 0, Vec2::new(n as f32 * 10., 2.), 0.4))
            .collect::<Vec<_>>();
        let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
        let mut served = std::collections::BTreeSet::new();
        for epoch in 0..8 {
            let mut budget = SweepBudget::new(epoch);
            budget.bodies[0] = OWNER_WORK / 2 - 20;
            let result = collect(
                &units,
                &origins,
                &mut outputs(&units),
                Vec2::splat(20_000.),
                &mut budget,
            );
            for i in 0..units.len() {
                if result.get(i).is_some() {
                    served.insert(i);
                }
            }
            assert!(budget.bodies[0] <= OWNER_WORK / 2);
            assert!(budget.buckets.iter().sum::<usize>() <= WORK / 2);
        }
        assert_eq!(served.len(), units.len());
        let mut swapped = units.clone();
        for (i, u) in swapped.iter_mut().enumerate() {
            u.unit_id = i as u32 + 1;
        }
        let mut a = SweepBudget::new(3);
        let mut b = a.clone();
        a.bodies[0] = OWNER_WORK / 2 - 20;
        b.bodies[0] = a.bodies[0];
        let first = collect(
            &units,
            &origins,
            &mut outputs(&units),
            Vec2::splat(20_000.),
            &mut a,
        );
        let second = collect(
            &swapped,
            &origins,
            &mut outputs(&swapped),
            Vec2::splat(20_000.),
            &mut b,
        );
        assert_eq!(first.ranges, second.ranges);
    }
    #[test]
    fn repeated_guard_passes_share_pair_limits_and_preserve_other_owner_credit() {
        let mut budget = SweepBudget::new(0);
        let mut actor = 0;
        for _ in 0..OWNER_WORK {
            assert!(budget.charge_pair(0, &mut actor, OWNER_WORK));
        }
        assert!(!budget.charge_pair(0, &mut 0, OWNER_WORK));
        assert!(budget.charge_pair(1, &mut 0, OWNER_WORK));
        assert!(budget.pairs.iter().sum::<usize>() <= WORK);
    }
}

#[cfg(test)]
mod extraction_tests {
    use super::*;
    use crate::movement::motion::{MotionGuide, MotionKind, MotionRequest};
    use std::cell::Cell;
    struct Metered {
        spent: Cell<usize>,
        rows: Cell<usize>,
    }
    impl MotionGuide for Metered {
        fn certificate_search_enabled(&self) -> bool {
            false
        }
        fn kind(&self, _: u32) -> MotionKind {
            MotionKind::Waypoint
        }
        fn next_position(&mut self, r: MotionRequest, _: &mut SweepBudget) -> Option<Vec2> {
            Some(r.preferred_position())
        }
        fn charge_certificate_visit(
            &self,
            _: u32,
            owner: u8,
            _: bool,
            b: &mut SweepBudget,
        ) -> bool {
            let mut n = self.spent.get();
            if !b.can_charge_pair(owner, n, 128) || !b.body(owner) {
                return false;
            }
            if !b.charge_pair(owner, &mut n, 128) {
                return false;
            }
            self.spent.set(n);
            true
        }
        fn examine_certificate_row(&self, id: u32, owner: u8, b: &mut SweepBudget) -> bool {
            if !self.charge_certificate_visit(id, owner, false, b) {
                return false;
            }
            self.rows.set(self.rows.get() + 1);
            true
        }
        fn admit_certificate_row(&self, _: u32, _: u8, _: bool, _: &mut SweepBudget) -> bool {
            true
        }
    }
    #[test]
    fn original_disc_admission_does_not_extract_distant_raw_chords() {
        let units = (0..201)
            .map(|i| MovementFrameUnit {
                unit_id: i + 1,
                owner_slot: u8::from(i != 0),
                unit_type: 0,
                position: Vec2::new(if i == 0 { 5. } else { 1. }, 5.),
                radius: 0.1,
                speed: None,
                target: None,
                progress: None,
                plain_progress: None,
                physical_escape: None,
            })
            .collect::<Vec<_>>();
        let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
        let mut outputs = units
            .iter()
            .map(|u| MovementFrameOutput {
                recovery: Default::default(),
                unit_id: u.unit_id,
                position: Vec2::new(if u.unit_id == 1 { 6. } else { 9. }, 5.),
                target: None,
                progress: None,
                plain_progress: None,
                physical_escape: None,
                failed_blocked: false,
                arrival_slot: None,
                #[cfg(feature = "test-probes")]
                probe: Default::default(),
            })
            .collect::<Vec<_>>();
        let mut budget = SweepBudget::new(0);
        let guide = Metered {
            spent: Cell::new(0),
            rows: Cell::new(0),
        };
        let mut candidates = collect_guided(
            &units,
            &origins,
            &mut outputs,
            Vec2::splat(32.),
            &mut budget,
            &guide,
        );
        assert!(
            candidates.get(0).is_some_and(|(rows, _)| rows.is_empty()),
            "Distant original discs must not exhaust this actor's admission"
        );
        assert_eq!(
            budget.bodies[0], 2,
            "Charge one query and its one extracted self row"
        );
        // Later certification still needs every committed crossing chord.
        assert!(
            candidates
                .query(
                    0,
                    outputs[0].position,
                    &units,
                    &origins,
                    &outputs,
                    &mut Vec::new(),
                    &vec![true; units.len()],
                    &mut SweepBudget::new(0),
                )
                .is_none(),
            "The complete swept index must retain all 200 crossing peers"
        );
    }
    #[test]
    fn deferred_permission_does_not_fabricate_paid_ranges() {
        let units = vec![
            super::tests::body(1, 0, Vec2::new(2., 2.), 0.2),
            super::tests::body(2, 0, Vec2::new(10., 2.), 0.2),
        ];
        let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
        let mut outputs = super::tests::outputs(&units);
        let guide = Metered {
            spent: Cell::new(0),
            rows: Cell::new(0),
        };
        let mut budget = SweepBudget::new(0);
        let c = collect_guided_deferred(
            &units,
            &origins,
            &mut outputs,
            Vec2::splat(32.),
            &mut budget,
            &guide,
            &[true, false],
        );
        assert!(c.get(0).is_none());
        assert!(c.get(1).is_some());
        assert_eq!(
            guide.spent.get(),
            2,
            "Only the ordinary row paid query+self extraction"
        );
    }
    #[test]
    fn cheap_baseline_reuses_original_candidates_without_future_raw_extraction() {
        let units = (0..201)
            .map(|i| MovementFrameUnit {
                unit_id: i + 1,
                owner_slot: u8::from(i != 0),
                unit_type: 0,
                position: Vec2::new(if i == 0 { 5. } else { 1. }, 5.),
                radius: 0.1,
                speed: None,
                target: None,
                progress: None,
                plain_progress: None,
                physical_escape: None,
            })
            .collect::<Vec<_>>();
        let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
        let mut outputs = units
            .iter()
            .map(|u| MovementFrameOutput {
                recovery: Default::default(),
                unit_id: u.unit_id,
                position: Vec2::new(if u.unit_id == 1 { 6. } else { 9. }, 5.),
                target: None,
                progress: None,
                plain_progress: None,
                physical_escape: None,
                failed_blocked: false,
                arrival_slot: None,
                #[cfg(feature = "test-probes")]
                probe: Default::default(),
            })
            .collect::<Vec<_>>();
        let mut budget = SweepBudget::new(0);
        let guide = Metered {
            spent: Cell::new(0),
            rows: Cell::new(0),
        };
        let mut candidates = collect_guided(
            &units,
            &origins,
            &mut outputs,
            Vec2::splat(32.),
            &mut budget,
            &guide,
        );
        assert!(
            candidates.get(0).is_some_and(|(rows, _)| rows.is_empty()),
            "Distant original discs must not exhaust this actor's admission"
        );
        assert_eq!(
            budget.bodies[0], 2,
            "Charge one query and its one extracted self row"
        );
        let guide = Metered {
            spent: Cell::new(0),
            rows: Cell::new(0),
        };
        let mut budget = SweepBudget::new(0);
        assert_eq!(
            candidates.query_guided(
                0,
                outputs[0].position,
                &units,
                &origins,
                &outputs,
                &mut Vec::new(),
                &vec![false; units.len()],
                &mut budget,
                &guide
            ),
            Some(Vec::new()),
            "Cheap baseline must not re-extract uncommitted distant raw envelopes"
        );
        assert_eq!(guide.rows.get(), 0);
        assert_eq!(guide.spent.get(), 1, "Only query admission is charged");
    }
    #[test]
    fn cached_baseline_merges_new_committed_crossings_once_and_meters_refusal() {
        for reverse in [false, true] {
            let mut units = vec![
                super::tests::body(1, 0, Vec2::new(5., 5.), 0.1),
                super::tests::body(2, 0, Vec2::new(5.5, 5.), 0.1),
                super::tests::body(3, 0, Vec2::new(3., 5.), 0.1),
                super::tests::body(4, 0, Vec2::new(1., 5.), 0.1),
            ];
            if reverse {
                units.reverse();
                for u in &mut units {
                    u.unit_id = 100 - u.unit_id;
                }
            }
            let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
            let mut outputs = super::tests::outputs(&units);
            for (i, u) in units.iter().enumerate() {
                outputs[i].position = if u.position.x == 5. {
                    Vec2::new(6., 5.)
                } else if u.position.x == 5.5 {
                    Vec2::new(5.5, 6.)
                } else if u.position.x == 3. {
                    Vec2::new(5.8, 5.)
                } else {
                    Vec2::new(9., 5.)
                };
            }
            let actor = origins.iter().position(|p| p.x == 5.).unwrap();
            let guide = Metered {
                spent: Cell::new(0),
                rows: Cell::new(0),
            };
            let mut budget = SweepBudget::new(0);
            let mut c = collect_guided(
                &units,
                &origins,
                &mut outputs,
                Vec2::splat(32.),
                &mut budget,
                &guide,
            );
            let mut flags = vec![false; units.len()];
            for i in 0..units.len() {
                if origins[i].x == 5.5 || origins[i].x == 3. {
                    flags[i] = true;
                    c.committed(
                        i,
                        origins[i],
                        outputs[i].position,
                        units[i].radius,
                        &mut budget,
                    );
                }
            }
            for (to, expected_rows, expected_peers) in [
                (Vec2::new(6., 5.), 2, 2),
                (Vec2::new(5.2, 5.), 2, 1),
                (Vec2::new(7., 5.), 4, 2),
            ] {
                guide.spent.set(0);
                guide.rows.set(0);
                let result = c
                    .query_guided(
                        actor,
                        to,
                        &units,
                        &origins,
                        &outputs,
                        &mut Vec::new(),
                        &flags,
                        &mut budget,
                        &guide,
                    )
                    .unwrap();
                assert_eq!(guide.rows.get(), expected_rows, "to={to:?}");
                assert_eq!(result.len(), expected_peers, "to={to:?}");
                if to.x <= 6. {
                    assert!(
                        result
                            .windows(2)
                            .all(|p| origins[p[0]].x <= origins[p[1]].x)
                    );
                }
            }
            // Every cached read is charged; a refused second row publishes no prefix.
            guide.spent.set(126);
            guide.rows.set(0);
            assert!(
                c.query_guided(
                    actor,
                    Vec2::new(6., 5.),
                    &units,
                    &origins,
                    &outputs,
                    &mut Vec::new(),
                    &flags,
                    &mut budget,
                    &guide
                )
                .is_none()
            );
            assert_eq!(guide.spent.get(), 128);
            assert_eq!(guide.rows.get(), 1);
            // An already-committed actor uses the original full-query path.
            c.mark_committed(actor);
            flags[actor] = true;
            guide.spent.set(0);
            guide.rows.set(0);
            assert!(
                c.query_guided(
                    actor,
                    Vec2::new(6., 5.),
                    &units,
                    &origins,
                    &outputs,
                    &mut Vec::new(),
                    &flags,
                    &mut budget,
                    &guide
                )
                .is_some()
            );
            assert_eq!(guide.rows.get(), 4);
        }
    }
    #[test]
    fn rejected_envelope_aliases_exhaust_planning_before_unpaid_examination() {
        let n = 201;
        let units = (0..n)
            .map(|i| MovementFrameUnit {
                unit_id: i as u32 + 1,
                owner_slot: if i == 0 { 0 } else { 1 },
                unit_type: 0,
                position: Vec2::new(if i == 0 { 5. } else { 1. }, 5.),
                radius: 0.1,
                speed: None,
                target: None,
                progress: None,
                plain_progress: None,
                physical_escape: None,
            })
            .collect::<Vec<_>>();
        let origins = units.iter().map(|u| u.position).collect::<Vec<_>>();
        let outputs = units
            .iter()
            .map(|u| MovementFrameOutput {
                recovery: Default::default(),
                unit_id: u.unit_id,
                position: u.position,
                target: None,
                progress: None,
                plain_progress: None,
                physical_escape: None,
                failed_blocked: false,
                arrival_slot: None,
                #[cfg(feature = "test-probes")]
                probe: crate::movement::probe::FrameFlags::default(),
            })
            .collect::<Vec<_>>();
        let envelopes = vec![[0., 10., 4., 6.]; n];
        let mut ranges = vec![None; n];
        ranges[0] = Some((0, 0));
        let mut candidates = Candidates {
            index: crate::terrain::traffic::EnvelopeIndex::new(n, |i| Some(envelopes[i])),
            envelopes,
            rows: Vec::new(),
            ranges,
            allowances: vec![4096; n],
            committed_mask: vec![u64::MAX; n.div_ceil(64)],
            query_mask: vec![0; n.div_ceil(64)],
        };
        let mut seen = Vec::new();
        let committed = vec![true; n];
        let mut budget = SweepBudget::new(1);
        let guide = Metered {
            spent: Cell::new(0),
            rows: Cell::new(0),
        };
        assert!(
            candidates
                .query_guided(
                    0,
                    Vec2::new(6., 5.),
                    &units,
                    &origins,
                    &outputs,
                    &mut seen,
                    &committed,
                    &mut budget,
                    &guide
                )
                .is_none()
        );
        assert_eq!(guide.spent.get(), 128);
        assert_eq!(guide.rows.get(), 127);
        assert_eq!(budget.pairs[0], 128);
        assert_eq!(budget.bodies[0], 128);
        assert_eq!(budget.envelope_filters as usize, guide.rows.get() - 1);
        let mut legacy = SweepBudget::new(1);
        assert!(
            candidates
                .query_guided(
                    0,
                    Vec2::new(6., 5.),
                    &units,
                    &origins,
                    &outputs,
                    &mut Vec::new(),
                    &committed,
                    &mut legacy,
                    &crate::movement::motion::NoMotionGuide
                )
                .unwrap()
                .is_empty()
        );
        assert_eq!(legacy.pairs[0], 0);
        assert_eq!(legacy.bodies[0], 0);
    }
}
