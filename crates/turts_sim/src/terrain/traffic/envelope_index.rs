//! Complete exact envelope masks with independently bounded metadata work.
use super::INDEX_BODIES;
const BLOCK: usize = 64;
pub(crate) const WORDS: usize = INDEX_BODIES.div_ceil(64);
#[derive(Clone, Copy)]
struct Endpoint {
    value: f64,
    index: usize,
}
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct MetadataWork {
    pub(crate) queries: u64,
    pub(crate) rank_comparisons: u64,
    pub(crate) tail_entries: u64,
    pub(crate) word_ops: u64,
    pub(crate) build_comparisons: u64,
    pub(crate) build_prefix_words: u64,
    pub(crate) build_mask_words: u64,
}
impl MetadataWork {
    pub(crate) fn add(&mut self, other: Self) {
        self.queries = self.queries.saturating_add(other.queries);
        self.rank_comparisons = self.rank_comparisons.saturating_add(other.rank_comparisons);
        self.tail_entries = self.tail_entries.saturating_add(other.tail_entries);
        self.word_ops = self.word_ops.saturating_add(other.word_ops);
        self.build_comparisons = self
            .build_comparisons
            .saturating_add(other.build_comparisons);
        self.build_prefix_words = self
            .build_prefix_words
            .saturating_add(other.build_prefix_words);
        self.build_mask_words = self.build_mask_words.saturating_add(other.build_mask_words);
    }
}
pub(crate) const fn query_word_bound() -> usize {
    21 * WORDS + 504
}
pub(crate) const fn query_rank_bound() -> usize {
    4 * (usize::BITS - INDEX_BODIES.leading_zeros()) as usize
}

pub(crate) struct EnvelopeIndex {
    endpoints: [Vec<Endpoint>; 4],
    prefixes: [Vec<u64>; 4],
    mask: Vec<u64>,
    tail: Vec<u64>,
    count: usize,
    pub(crate) work: MetadataWork,
}
impl EnvelopeIndex {
    pub(crate) const fn capacity_bound(count: usize) -> usize {
        let words = count.div_ceil(64);
        std::mem::size_of::<Self>()
            + 4 * count * std::mem::size_of::<Endpoint>()
            + 4 * (count / BLOCK + 1) * words * 8
            + 2 * words * 8
    }
    pub(crate) fn new(count: usize, bounds: impl FnMut(usize) -> Option<[f64; 4]>) -> Option<Self> {
        Self::new_impl::<false>(count, bounds)
    }
    fn new_impl<const INDEX_TIE: bool>(
        count: usize,
        mut bounds: impl FnMut(usize) -> Option<[f64; 4]>,
    ) -> Option<Self> {
        if count > INDEX_BODIES {
            return None;
        }
        // Prefix masks need only the active rows. The public worst-case work
        // and physical-row limits still use WORDS; unused maximum rows need no storage.
        let words = count.div_ceil(64);
        let mut value = Self {
            endpoints: std::array::from_fn(|_| Vec::with_capacity(count)),
            prefixes: std::array::from_fn(|_| Vec::with_capacity((count / BLOCK + 1) * words)),
            mask: vec![0; words],
            tail: vec![0; words],
            count,
            work: MetadataWork::default(),
        };
        value.work.build_mask_words += (2 * words) as u64;
        for index in 0..count {
            let b = bounds(index)?;
            if !b.iter().all(|v| v.is_finite()) || b[0] > b[1] || b[2] > b[3] {
                return None;
            }
            for (axis, &bound) in b.iter().enumerate() {
                value.endpoints[axis].push(Endpoint {
                    value: bound,
                    index,
                });
            }
        }
        for axis in 0..4 {
            value.endpoints[axis].sort_unstable_by(|a, b| {
                value.work.build_comparisons += 1;
                let order = a.value.total_cmp(&b.value);
                // Queries include/exclude an entire equal-value group. Its
                // internal order cannot affect the completed prefix+tail mask.
                if INDEX_TIE {
                    order.then_with(|| a.index.cmp(&b.index))
                } else {
                    order
                }
            });
            value.tail.fill(0);
            value.work.build_mask_words += words as u64;
            value.prefixes[axis].extend_from_slice(&value.tail);
            value.work.build_prefix_words += words as u64;
            for (i, e) in value.endpoints[axis].iter().enumerate() {
                value.tail[e.index / 64] |= 1u64 << (e.index % 64);
                value.work.build_mask_words += 2;
                if (i + 1) % BLOCK == 0 {
                    value.prefixes[axis].extend_from_slice(&value.tail);
                    value.work.build_prefix_words += words as u64;
                }
            }
        }
        Some(value)
    }
    /// Replace one row without allocation. Refusal preserves structural state.
    /// The result counts logical endpoint positions rewritten, not byte stores.
    pub(crate) fn replace_bounds(&mut self, row: usize, bounds: [f64; 4]) -> Option<usize> {
        if self.count > INDEX_BODIES
            || row >= self.count
            || !bounds.iter().all(|v| v.is_finite())
            || bounds[0] > bounds[1]
            || bounds[2] > bounds[3]
        {
            return None;
        }
        let words = self.count.div_ceil(64);
        let prefix_len = (self.count / BLOCK + 1) * words;
        if self.endpoints.iter().any(|v| v.len() != self.count)
            || self.prefixes.iter().any(|v| v.len() != prefix_len)
            || self.mask.len() != words
            || self.tail.len() != words
        {
            return None;
        }
        let mut positions = [0; 4];
        for (axis, position) in positions.iter_mut().enumerate() {
            let mut found = None;
            for (i, endpoint) in self.endpoints[axis].iter().enumerate() {
                self.work.build_comparisons += 1;
                if endpoint.index == row {
                    found = Some(i);
                    break;
                }
            }
            *position = found?;
        }
        let mut rewritten = 0;
        for axis in 0..4 {
            let old = positions[axis];
            let order = bounds[axis].total_cmp(&self.endpoints[axis][old].value);
            self.work.build_comparisons += 1;
            if order.is_eq() {
                continue;
            }
            let new = if order.is_lt() {
                self.endpoints[axis][..old].partition_point(|e| {
                    self.work.build_comparisons += 1;
                    e.value.total_cmp(&bounds[axis]).is_lt()
                })
            } else {
                old + self.endpoints[axis][old + 1..].partition_point(|e| {
                    self.work.build_comparisons += 1;
                    e.value.total_cmp(&bounds[axis]).is_lt()
                })
            };
            if new < old {
                self.endpoints[axis].copy_within(new..old, new + 1);
            } else if new > old {
                self.endpoints[axis].copy_within(old + 1..new + 1, old);
            }
            self.endpoints[axis][new] = Endpoint {
                value: bounds[axis],
                index: row,
            };
            rewritten += old.abs_diff(new) + 1;
            let first = (old.min(new) / BLOCK + 1) * BLOCK;
            // Only crossed prefix boundaries change membership. After the
            // shift, b-1 is the incoming row moving right; b is outgoing left.
            for boundary in (first..=old.max(new)).step_by(BLOCK) {
                let (remove, add) = if old < new {
                    (row, self.endpoints[axis][boundary - 1].index)
                } else {
                    (self.endpoints[axis][boundary].index, row)
                };
                let offset = boundary / BLOCK * words;
                self.prefixes[axis][offset + remove / 64] &= !(1u64 << (remove % 64));
                self.prefixes[axis][offset + add / 64] |= 1u64 << (add % 64);
                self.work.build_mask_words += 4;
            }
        }
        Some(rewritten)
    }
    pub(crate) fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self
                .endpoints
                .iter()
                .map(|v| v.capacity() * std::mem::size_of::<Endpoint>())
                .sum::<usize>()
            + self
                .prefixes
                .iter()
                .map(|v| v.capacity() * 8)
                .sum::<usize>()
            + (self.mask.capacity() + self.tail.capacity()) * 8
    }
    pub(crate) fn query(
        &mut self,
        bounds: [f64; 4],
        restrict: Option<&[u64]>,
    ) -> Option<MaskRows<'_>> {
        self.query_impl::<true, true>(bounds, restrict)
    }
    fn query_impl<const EARLY: bool, const NEAREST: bool>(
        &mut self,
        bounds: [f64; 4],
        restrict: Option<&[u64]>,
    ) -> Option<MaskRows<'_>> {
        let words = self.count.div_ceil(64);
        if !bounds.iter().all(|v| v.is_finite())
            || bounds[0] > bounds[1]
            || bounds[2] > bounds[3]
            || restrict.is_some_and(|m| m.len() < words)
        {
            return None;
        }
        self.work.queries += 1;
        // min_x<=query_max_x, max_x>=query_min_x, and the two y predicates.
        // Maximum predicates complement STRICT lower prefixes, preserving tangency.
        for axis in 0..4 {
            let upper = axis % 2 == 0;
            let threshold = if upper {
                bounds[axis + 1]
            } else {
                bounds[axis - 1]
            };
            let mut before = |endpoint: &Endpoint| {
                self.work.rank_comparisons += 1;
                if upper {
                    endpoint.value <= threshold
                } else {
                    endpoint.value < threshold
                }
            };
            let rank = if EARLY {
                // The half-open lower bound performs at most ceil(log2(n+1))
                // comparisons, matching the public per-query metadata bound.
                let (mut low, mut high) = (0, self.count);
                while low < high {
                    let middle = low + (high - low) / 2;
                    if before(&self.endpoints[axis][middle]) {
                        low = middle + 1;
                    } else {
                        high = middle;
                    }
                }
                low
            } else {
                self.endpoints[axis].partition_point(before)
            };
            let block = rank / BLOCK;
            let start = block * BLOCK;
            let end = (start + BLOCK).min(self.count);
            let suffix = NEAREST && end - rank < rank - start;
            let full_prefix = suffix && end == self.count && !end.is_multiple_of(BLOCK);
            let prefix_block = if suffix { end / BLOCK } else { block };
            let range = if suffix { rank..end } else { start..rank };
            self.tail[..words].fill(0);
            self.work.word_ops += words as u64;
            for endpoint in &self.endpoints[axis][range] {
                self.tail[endpoint.index / 64] |= 1u64 << (endpoint.index % 64);
                self.work.tail_entries += 1;
                self.work.word_ops += 2;
            }
            let mut any = 0;
            for word in 0..words {
                let active = || {
                    let remaining = self.count.saturating_sub(word * 64);
                    if remaining >= 64 {
                        u64::MAX
                    } else {
                        (1u64 << remaining) - 1
                    }
                };
                let prefix = if full_prefix {
                    active()
                } else {
                    self.prefixes[axis][prefix_block * words + word]
                };
                let selected = if suffix {
                    prefix & !self.tail[word]
                } else {
                    prefix | self.tail[word]
                };
                let selected = if upper { selected } else { !selected };
                let previous = if axis == 0 {
                    if full_prefix { prefix } else { active() }
                } else {
                    self.mask[word]
                };
                let restriction_axis = if EARLY { 0 } else { 3 };
                let restriction = if axis == restriction_axis {
                    restrict.map_or(u64::MAX, |keep| keep[word])
                } else {
                    u64::MAX
                };
                let next = previous & selected & restriction;
                self.mask[word] = next;
                // A synthesized full prefix has no prefix-memory read.
                self.work.word_ops += (if axis == 0 { 3 } else { 4 }) - u64::from(full_prefix);
                if axis == restriction_axis && restrict.is_some() {
                    self.work.word_ops += 1;
                }
                if EARLY {
                    // Reuse the computed register value; no extra word read.
                    any |= next;
                }
            }
            // Every later predicate only removes candidates. The restriction
            // is already included, so an empty complete mask is final.
            if EARLY && any == 0 {
                break;
            }
        }

        Some(MaskRows {
            mask: &self.mask[..words],
            work: &mut self.work,
            word: 0,
            bits: 0,
            base: 0,
        })
    }
    #[cfg(test)]
    pub(crate) fn mask_bits(&self) -> &[u64] {
        &self.mask[..self.count.div_ceil(64)]
    }
}
/// Iteration counts actual output-word reads, including words without a hit.
pub(crate) struct MaskRows<'a> {
    mask: &'a [u64],
    work: &'a mut MetadataWork,
    word: usize,
    bits: u64,
    base: usize,
}
impl Iterator for MaskRows<'_> {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        loop {
            if self.bits != 0 {
                let i = self.base + self.bits.trailing_zeros() as usize;
                self.bits &= self.bits - 1;
                return Some(i);
            }
            let word = *self.mask.get(self.word)?;
            self.work.word_ops += 1;
            self.base = self.word * 64;
            self.word += 1;
            self.bits = word;
        }
    }
}

#[cfg(test)]
mod stride_tests {
    use super::*;

    fn bounds(i: usize) -> [f64; 4] {
        let x = ((i * 17) % 97) as f64;
        let y = ((i * 31) % 89) as f64;
        [x, x + 0.5, y, y + 1.0]
    }

    #[test]
    fn thousand_body_index_uses_less_than_eighty_kib_of_scratch() {
        let index = EnvelopeIndex::new(1000, |i| Some(bounds(i))).unwrap();
        println!("ACTIVE_INDEX_SCRATCH rows=1000 bytes={}", index.bytes());
        assert!(
            index.bytes() < 80 * 1024,
            "1000-row metadata scratch is {} bytes",
            index.bytes()
        );
    }

    #[test]
    fn every_active_word_and_prefix_boundary_matches_direct_envelopes() {
        for count in [0, 1, 63, 64, 65, 127, 128, 129, 500, 1000, INDEX_BODIES] {
            let mut index = EnvelopeIndex::new(count, |i| Some(bounds(i))).unwrap();
            let mut restriction = vec![0u64; count.div_ceil(64)];
            for i in 0..count {
                if i % 3 == 0 {
                    restriction[i / 64] |= 1u64 << (i % 64);
                }
            }
            for query in [
                [-1., 200., -1., 200.],
                [17., 31.5, 19., 54.],
                [97., 98., 91., 92.],
                [0.5, 0.5, 1., 1.],
            ] {
                for restricted in [false, true] {
                    let expected: Vec<_> = (0..count)
                        .filter(|&i| {
                            let b = bounds(i);
                            (!restricted || i % 3 == 0)
                                && b[0] <= query[1]
                                && b[1] >= query[0]
                                && b[2] <= query[3]
                                && b[3] >= query[2]
                        })
                        .collect();
                    let actual: Vec<_> = index
                        .query(query, restricted.then_some(restriction.as_slice()))
                        .unwrap()
                        .collect();
                    assert_eq!(
                        actual, expected,
                        "count={count} query={query:?} restricted={restricted}"
                    );
                }
            }
            assert!(index.bytes() <= EnvelopeIndex::capacity_bound(count));
        }
    }
    #[test]
    fn early_empty_matches_original_and_reuses_masks() {
        for count in [0, 1, 63, 64, 65, 129, 500, 1000, INDEX_BODIES] {
            let mut old = EnvelopeIndex::new(count, |i| Some(bounds(i))).unwrap();
            let mut new = EnvelopeIndex::new(count, |i| Some(bounds(i))).unwrap();
            for mask_value in [0, u64::MAX, 0x123456789abcdef1] {
                let mask = vec![mask_value; count.div_ceil(64)];
                for restricted in [false, true] {
                    for q in [
                        [-5., -4., 0., 100.],
                        [500., 600., 0., 100.],
                        [0., 100., -5., -4.],
                        [17., 31.5, 19., 54.],
                        [0.5, 0.5, 1., 1.],
                        [-1., 200., -1., 200.],
                    ] {
                        let restrict = restricted.then_some(mask.as_slice());
                        let a: Vec<_> = old
                            .query_impl::<false, false>(q, restrict)
                            .unwrap()
                            .collect();
                        let before = new.work;
                        let b: Vec<_> = new.query(q, restrict).unwrap().collect();
                        assert!(new.work.word_ops - before.word_ops <= query_word_bound() as u64);
                        assert!(
                            new.work.rank_comparisons - before.rank_comparisons
                                <= query_rank_bound() as u64
                        );
                        assert_eq!(a, b);
                        assert_eq!(old.mask_bits(), new.mask_bits());
                    }
                }
            }
            for q in [[f64::NAN, 1., 0., 1.], [2., 1., 0., 1.]] {
                assert!(old.query_impl::<false, false>(q, None).is_none());
                assert!(new.query(q, None).is_none());
            }
        }
    }
    #[test]
    #[ignore = "Scoped metadata cost only"]
    fn early_empty_scoped_cost() {
        use std::{hint::black_box, time::Instant};
        let mut checks = Vec::new();
        for (mixed, early) in [(true, false), (true, true), (false, false), (false, true)] {
            let mut index = EnvelopeIndex::new(1000, |i| Some(bounds(i))).unwrap();
            let keep = vec![0x123456789abcdef1; 1000usize.div_ceil(64)];
            let start = Instant::now();
            let mut sum = 0;
            for i in 0..200000 {
                let b = bounds(i % 1000);
                let q = match if mixed { i % 4 } else { 2 } {
                    0 => [-5., -4., b[2], b[3]],
                    1 => [b[0], b[1], -5., -4.],
                    _ => [b[0] - 1., b[1] + 1., b[2] - 1., b[3] + 1.],
                };
                let rows = if early {
                    index.query_impl::<true, true>(black_box(q), Some(&keep))
                } else {
                    index.query_impl::<false, false>(black_box(q), Some(&keep))
                }
                .unwrap();
                sum += rows.count();
            }
            eprintln!(
                "mixed={mixed} early={early} elapsed={:?} checksum={sum} word_ops={}",
                start.elapsed(),
                index.work.word_ops
            );
            checks.push(sum);
        }
        assert_eq!(checks[0], checks[1]);
        assert_eq!(checks[2], checks[3]);
    }
    #[test]
    fn equal_endpoint_groups_match_index_tie_reference_under_permutation() {
        for n in [0, 1, 63, 64, 65, 129, 1000, INDEX_BODIES] {
            for reversed in [false, true] {
                let bounds = |i: usize| {
                    let i = if reversed { n - 1 - i } else { i };
                    let x = match i % 4 {
                        0 => -0.0,
                        1 => 0.0,
                        _ => 1.0,
                    };
                    [x, 2.0, 3.0, 4.0]
                };
                let mut old = EnvelopeIndex::new_impl::<true>(n, |i| Some(bounds(i))).unwrap();
                let mut new = EnvelopeIndex::new(n, |i| Some(bounds(i))).unwrap();
                let keep = vec![0x123456789abcdef1; n.div_ceil(64)];
                for q in [
                    [-0.0, 0.0, 3., 4.],
                    [1., 1., 3., 3.],
                    [2., 2., 4., 4.],
                    [3., 4., 3., 4.],
                    [-1., 3., 2., 5.],
                ] {
                    for restricted in [false, true] {
                        let restrict = restricted.then_some(keep.as_slice());
                        let a: Vec<_> = old.query(q, restrict).unwrap().collect();
                        let b: Vec<_> = new.query(q, restrict).unwrap().collect();
                        assert_eq!(a, b);
                        assert_eq!(old.mask_bits(), new.mask_bits());
                        let direct: Vec<_> = (0..n)
                            .filter(|&i| {
                                let v = bounds(i);
                                (!restricted || keep[i / 64] & (1 << (i % 64)) != 0)
                                    && v[0] <= q[1]
                                    && v[1] >= q[0]
                                    && v[2] <= q[3]
                                    && v[3] >= q[2]
                            })
                            .collect();
                        assert_eq!(b, direct);
                    }
                }
                assert_eq!(new.bytes(), old.bytes());
            }
        }
    }
    #[test]
    fn nearest_prefix_matches_lower_prefix_at_every_block_and_partial_boundary() {
        for n in [
            0,
            1,
            31,
            32,
            33,
            63,
            64,
            65,
            95,
            96,
            97,
            127,
            128,
            129,
            255,
            256,
            257,
            500,
            1000,
            INDEX_BODIES,
        ] {
            let mut old =
                EnvelopeIndex::new(n, |i| Some([i as f64, i as f64 + 0.5, 0., 1.])).unwrap();
            let mut new =
                EnvelopeIndex::new(n, |i| Some([i as f64, i as f64 + 0.5, 0., 1.])).unwrap();
            let keep = vec![0x123456789abcdef1; n.div_ceil(64)];
            for base in (0..=n).step_by(64) {
                for offset in [0, 1, 15, 16, 31, 32, 33, 47, 48, 63, 64] {
                    let rank = (base + offset).min(n);
                    for delta in [-0.25, 0., 0.25, 0.5] {
                        for restricted in [false, true] {
                            let x = rank as f64 + delta;
                            let q = [x, x, 0., 1.];
                            let restrict = restricted.then_some(keep.as_slice());
                            let a0 = old.work;
                            let b0 = new.work;
                            let a: Vec<_> = old
                                .query_impl::<true, false>(q, restrict)
                                .unwrap()
                                .collect();
                            let b: Vec<_> = new.query(q, restrict).unwrap().collect();
                            assert_eq!(a, b, "n={n},rank={rank},delta={delta}");
                            assert_eq!(old.mask_bits(), new.mask_bits());
                            assert_eq!(
                                new.work.rank_comparisons - b0.rank_comparisons,
                                old.work.rank_comparisons - a0.rank_comparisons
                            );
                            assert!(
                                new.work.tail_entries - b0.tail_entries
                                    <= old.work.tail_entries - a0.tail_entries
                            );
                            assert!(new.work.word_ops - b0.word_ops <= query_word_bound() as u64);
                            assert_eq!(new.bytes(), old.bytes());
                        }
                    }
                }
            }
        }
    }
    fn assert_complete_prefixes(index: &EnvelopeIndex) {
        let words = index.count.div_ceil(64);
        for axis in 0..4 {
            let mut expected = vec![0; words];
            assert_eq!(&index.prefixes[axis][..words], expected.as_slice());
            for (i, e) in index.endpoints[axis].iter().enumerate() {
                if i > 0 {
                    assert!(
                        !index.endpoints[axis][i - 1]
                            .value
                            .total_cmp(&e.value)
                            .is_gt()
                    );
                }
                expected[e.index / 64] |= 1 << (e.index % 64);
                if (i + 1) % BLOCK == 0 {
                    let start = (i + 1) / BLOCK * words;
                    assert_eq!(
                        &index.prefixes[axis][start..start + words],
                        expected.as_slice()
                    );
                }
            }
        }
    }
    #[test]
    fn repeated_row_updates_match_full_rebuild_direct_masks_and_reversed_rows() {
        for n in [1, 2, 63, 64, 65, 127, 128, 129, 500, 1000, INDEX_BODIES] {
            for reversed in [false, true] {
                let physical = |i: usize| if reversed { n - 1 - i } else { i };
                let mut boxes: Vec<_> = (0..n).map(|i| bounds(physical(i))).collect();
                let mut index = EnvelopeIndex::new(n, |i| Some(boxes[i])).unwrap();
                let pointers: Vec<_> = index
                    .endpoints
                    .iter()
                    .map(|v| v.as_ptr() as usize)
                    .chain(index.prefixes.iter().map(|v| v.as_ptr() as usize))
                    .chain([index.mask.as_ptr() as usize, index.tail.as_ptr() as usize])
                    .collect();
                let bytes = index.bytes();
                let keep = vec![0x123456789abcdef1; n.div_ceil(64)];
                for step in 0..if n == INDEX_BODIES { 16 } else { 64 } {
                    let row = physical(step * 997 % n);
                    let x = match step % 4 {
                        0 => -0.0,
                        1 => 0.0,
                        _ => (step * 37 % 151) as f64 - 50.,
                    };
                    let y = (step * 19 % 127) as f64 - 40.;
                    boxes[row] = [x, x + f64::from(step as u32 % 3), y, y + 0.5];
                    assert!(index.replace_bounds(row, boxes[row]).is_some());
                    assert_complete_prefixes(&index);
                    let mut rebuilt = EnvelopeIndex::new(n, |i| Some(boxes[i])).unwrap();
                    for q in [
                        [-100., 200., -100., 200.],
                        [x, x, y, y + 0.5],
                        [x - 1., x + 1., y - 1., y + 1.],
                        [500., 501., 500., 501.],
                    ] {
                        for restricted in [false, true] {
                            let restrict = restricted.then_some(keep.as_slice());
                            let actual: Vec<_> = index.query(q, restrict).unwrap().collect();
                            let reference: Vec<_> = rebuilt.query(q, restrict).unwrap().collect();
                            assert_eq!(actual, reference);
                            let direct: Vec<_> = (0..n)
                                .filter(|&i| {
                                    let b = boxes[i];
                                    (!restricted || keep[i / 64] & (1 << (i % 64)) != 0)
                                        && b[0] <= q[1]
                                        && b[1] >= q[0]
                                        && b[2] <= q[3]
                                        && b[3] >= q[2]
                                })
                                .collect();
                            assert_eq!(actual, direct);
                        }
                    }
                    assert_eq!(index.bytes(), bytes);
                    let after: Vec<_> = index
                        .endpoints
                        .iter()
                        .map(|v| v.as_ptr() as usize)
                        .chain(index.prefixes.iter().map(|v| v.as_ptr() as usize))
                        .chain([index.mask.as_ptr() as usize, index.tail.as_ptr() as usize])
                        .collect();
                    assert_eq!(pointers, after);
                }
            }
        }
    }
    #[test]
    fn invalid_row_updates_preserve_all_structural_state() {
        type Snapshot = ([Vec<(u64, usize)>; 4], [Vec<u64>; 4], Vec<u64>, Vec<u64>);
        fn snapshot(index: &EnvelopeIndex) -> Snapshot {
            (
                std::array::from_fn(|axis| {
                    index.endpoints[axis]
                        .iter()
                        .map(|e| (e.value.to_bits(), e.index))
                        .collect()
                }),
                index.prefixes.clone(),
                index.mask.clone(),
                index.tail.clone(),
            )
        }
        let mut index = EnvelopeIndex::new(129, |i| Some(bounds(i))).unwrap();
        for (row, b) in [
            (129, [0., 1., 0., 1.]),
            (0, [f64::NAN, 1., 0., 1.]),
            (0, [0., f64::INFINITY, 0., 1.]),
            (0, [2., 1., 0., 1.]),
            (0, [0., 1., 2., 1.]),
        ] {
            let before = snapshot(&index);
            assert!(index.replace_bounds(row, b).is_none());
            assert_eq!(before, snapshot(&index));
        }
        // A failed last-axis lookup must not change the first three axes.
        for e in &mut index.endpoints[3] {
            if e.index == 7 {
                e.index = 129;
            }
        }
        let before = snapshot(&index);
        assert!(index.replace_bounds(7, [0., 1., 0., 1.]).is_none());
        assert_eq!(before, snapshot(&index));
        let mut empty = EnvelopeIndex::new(0, |_| None).unwrap();
        assert!(empty.replace_bounds(0, [0.; 4]).is_none());
    }
}
