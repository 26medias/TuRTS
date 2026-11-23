//! Compare memoized certificates with fresh geometry, including public-initial
//! fallback and real movement outputs. No private height is read by the planner.
use super::*;
use std::cell::Cell as Counter;
use turts_rules::terrain::{
    Ratio,
    geometry::{Cell, Grid, Surface},
};

struct Terrain {
    heights: [Option<i32>; 64],
    private: [i32; 64],
    revisions: [u64; 64],
    epoch: u64,
    reads: Counter<usize>,
    missing: Option<Cell>,
}
impl Terrain {
    fn flat() -> Self {
        Self {
            heights: [None; 64],
            private: [0; 64],
            revisions: [0; 64],
            epoch: 0,
            reads: Counter::new(0),
            missing: None,
        }
    }
    fn index(cell: Cell) -> Option<usize> {
        (cell.x < 8 && cell.y < 8).then_some((cell.y * 8 + cell.x) as usize)
    }
    fn observe(&mut self, cell: Cell, height: i32) {
        let i = Self::index(cell).unwrap();
        if self.heights[i].unwrap_or(0) != height {
            self.revisions[i] += 1;
            self.epoch += 1;
        }
        self.heights[i] = Some(height);
    }
}
impl Surface for Terrain {
    fn elevation(&self, cell: Cell) -> Option<i32> {
        self.reads.set(self.reads.get() + 1);
        if self.missing == Some(cell) {
            return None;
        }
        Some(self.heights[Self::index(cell)?].unwrap_or(0))
    }
}
impl TerrainRead for Terrain {
    fn grid(&self) -> Grid {
        Grid::checked(8, 8, 1, 1, 1, -8, 8).unwrap()
    }
    fn epoch(&self) -> u64 {
        self.epoch
    }
    fn lease_matches(&self, key: usize, revision: u64) -> bool {
        self.revisions.get(key) == Some(&revision)
    }
    fn valid_lease(&self, lease: &BTreeMap<usize, u64>) -> bool {
        lease
            .iter()
            .all(|(&key, &revision)| self.lease_matches(key, revision))
    }
    fn rectangle(&self, min: Cell, max: Cell, lease: &mut BTreeMap<usize, u64>) -> bool {
        let mut height = None;
        let mut flat = true;
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                let cell = Cell { x, y };
                let Some(i) = Self::index(cell) else {
                    return false;
                };
                lease.insert(i, self.revisions[i]);
                let Some(value) = self.elevation(cell) else {
                    return false;
                };
                flat &= *height.get_or_insert(value) == value;
            }
        }
        flat
    }
}
fn member() -> Member {
    let mut m = Member::stamp(0, 1, 1, 0);
    m.cancelled = false;
    m.radius = 0.2f32.to_bits();
    m.endpoint = Vec2::new(2.1, 1.5);
    m
}
fn query(n: &mut ViewerNavigation, m: &mut Member, t: &Terrain) -> Certificate {
    let endpoint = m.endpoint;
    n.direct_query(m, t, Vec2::new(1.9, 1.5), endpoint)
}

#[test]
fn direct_memo_reuses_clear_and_blocked_with_unchanged_edge_charge() {
    let mut t = Terrain::flat();
    let mut n = ViewerNavigation::default();
    let mut m = member();
    assert_eq!(query(&mut n, &mut m, &t), Certificate::Clear);
    let reads = t.reads.get();
    assert_eq!(query(&mut n, &mut m, &t), Certificate::Clear);
    assert_eq!(t.reads.get(), reads, "hit reran geometry");
    assert_eq!(n.diagnostics.edge_checks_this_tick, 2);
    assert_eq!(n.diagnostics.direct_query_cache_hits, 1);
    assert_eq!(n.diagnostics.direct_query_cache_misses, 1);
    t.observe(Cell { x: 1, y: 1 }, 2);
    assert_eq!(query(&mut n, &mut m, &t), Certificate::Blocked);
    let reads = t.reads.get();
    assert_eq!(query(&mut n, &mut m, &t), Certificate::Blocked);
    assert_eq!(t.reads.get(), reads);
    assert_eq!(n.diagnostics.edge_checks_this_tick, 4);
}

#[test]
fn direct_memo_uses_only_permitted_local_height_leases() {
    let mut t = Terrain::flat();
    let mut n = ViewerNavigation::default();
    let mut m = member();
    assert_eq!(query(&mut n, &mut m, &t), Certificate::Clear);
    // An actual hidden edit must not enter the planning identity or geometry.
    t.private[Terrain::index(Cell { x: 1, y: 1 }).unwrap()] = 3;
    assert_eq!(query(&mut n, &mut m, &t), Certificate::Clear);
    // Observing public-initial height changes no planning revision.
    t.observe(Cell { x: 2, y: 1 }, 0);
    assert_eq!(query(&mut n, &mut m, &t), Certificate::Clear);
    // A distant permitted edit validates existing local leases without rebuilding.
    t.observe(Cell { x: 7, y: 7 }, 2);
    let reads = t.reads.get();
    assert_eq!(query(&mut n, &mut m, &t), Certificate::Clear);
    assert_eq!(t.reads.get(), reads);
    assert_eq!(n.diagnostics.direct_query_cache_hits, 3);
    // Once the local height is observed, the old clear result cannot activate.
    t.observe(Cell { x: 1, y: 1 }, 3);
    assert_eq!(query(&mut n, &mut m, &t), Certificate::Blocked);
    assert_eq!(n.diagnostics.direct_query_cache_misses, 2);
}

#[test]
fn direct_memo_does_not_bypass_suspended_validation_or_edge_quota() {
    let mut t = Terrain::flat();
    let mut n = ViewerNavigation::default();
    let mut m = member();
    assert_eq!(query(&mut n, &mut m, &t), Certificate::Clear);
    t.observe(Cell { x: 7, y: 7 }, 1);
    n.diagnostics.invalidation_entries_this_tick = MAX_ENTRIES;
    assert_eq!(query(&mut n, &mut m, &t), Certificate::Pending);
    assert_eq!(n.diagnostics.direct_query_cache_hits, 0);
    n.diagnostics.invalidation_entries_this_tick = 0;
    n.diagnostics.edge_checks_this_tick = MAX_EDGES;
    assert_eq!(query(&mut n, &mut m, &t), Certificate::Pending);
    assert_eq!(n.diagnostics.edge_checks_this_tick, MAX_EDGES);
    n.diagnostics.edge_checks_this_tick = 0;
    assert_eq!(query(&mut n, &mut m, &t), Certificate::Clear);
    assert_eq!(n.diagnostics.edge_checks_this_tick, 1);
    assert_eq!(n.diagnostics.direct_query_cache_hits, 1);
}

#[test]
fn direct_memo_never_retains_uncertain_or_unadmitted_results() {
    let mut t = Terrain::flat();
    t.missing = Some(Cell { x: 1, y: 1 });
    let mut n = ViewerNavigation::default();
    let mut m = member();
    for _ in 0..2 {
        assert_eq!(query(&mut n, &mut m, &t), Certificate::Pending);
        assert!(m.direct_query.is_none());
    }
    assert_eq!(n.diagnostics.direct_query_cache_misses, 2);
    t.missing = None;
    m.direct = Vec::new();
    n.member_buffer_bytes = MAX_BYTES;
    assert_eq!(query(&mut n, &mut m, &t), Certificate::Pending);
    assert!(
        m.direct_query.is_none(),
        "unadmitted clear lease was cached"
    );
    n.member_buffer_bytes = 0;
    assert_eq!(query(&mut n, &mut m, &t), Certificate::Clear);
}

#[test]
fn direct_memo_numeric_and_order_identity_is_exact() {
    let t = Terrain::flat();
    let mut n = ViewerNavigation::default();
    let mut m = member();
    assert_eq!(query(&mut n, &mut m, &t), Certificate::Clear);
    for change in 0..5 {
        match change {
            0 => m.epoch += 1,
            1 => m.viewer = 1,
            2 => m.radius = 0.21f32.to_bits(),
            3 => {
                m.profile = Traversal {
                    max_step: 1,
                    max_grade: Ratio {
                        numerator: 1,
                        denominator: 1,
                    },
                }
            }
            _ => m.endpoint.x = f32::from_bits(m.endpoint.x.to_bits() + 1),
        }
        assert_eq!(query(&mut n, &mut m, &t), Certificate::Clear);
    }
    assert_eq!(n.diagnostics.direct_query_cache_hits, 0);
    assert_eq!(n.diagnostics.direct_query_cache_misses, 6);
    for x in [0.0, -0.0] {
        assert_eq!(
            n.direct_query(&mut m, &t, Vec2::new(x, 1.5), Vec2::new(2.1, 1.5)),
            Certificate::Blocked
        );
    }
    assert_eq!(
        n.diagnostics.direct_query_cache_hits, 0,
        "signed zero was rounded in key"
    );
    assert_eq!(n.diagnostics.direct_query_cache_misses, 8);
    n.members.insert(41, m);
    n.cancel(41, 2, Some(0));
    assert!(n.members.get(&41).unwrap().direct_query.is_none());
    assert!(size_of::<DirectQueryMemo>() <= 128);
    assert_eq!(MAX_MEMBERS * VIEWERS, 4096);
}

#[test]
fn direct_memo_matches_fresh_certificates_on_a_real_movement_tape() {
    use crate::movement::{MovementFrame, MovementFrameUnit};
    let mut t = Terrain::flat();
    let mut cached = ViewerNavigation::default();
    let mut reference = ViewerNavigation::default();
    let mut a = member();
    let mut b = member();
    a.endpoint = Vec2::new(6.9, 1.5);
    b.endpoint = a.endpoint;
    let initial = MovementFrame {
        recovery: Default::default(),
        bounds: Vec2::splat(8.0),
        tick_hz: 20,
        pools: BTreeMap::new(),
        units: vec![MovementFrameUnit {
            unit_id: 991,
            owner_slot: 0,
            unit_type: 1,
            position: Vec2::new(1.5, 1.5),
            radius: 0.2,
            speed: Some(2.0),
            target: Some(a.endpoint),
            progress: None,
            plain_progress: None,
            physical_escape: None,
        }],
    };
    let mut frames = [initial.clone(), initial];
    for tick in 0..100 {
        if tick == 10 {
            t.observe(Cell { x: 6, y: 1 }, 2);
        }
        if tick == 30 {
            t.private[14] = 0;
        }
        if tick == 45 {
            t.observe(Cell { x: 6, y: 1 }, 0);
        }
        cached.begin(tick);
        reference.begin(tick);
        let from = frames[0].units[0].position;
        let endpoint = a.endpoint;
        let result = cached.direct_query(&mut a, &t, from, endpoint);
        assert!(reference.validate_member(&mut b, &t));
        let fresh = reference.member_query(&mut b, &t, from, endpoint);
        assert_eq!(result, fresh, "certificate at tick {tick}");
        for frame in &mut frames {
            frame.units[0].target = (result == Certificate::Clear).then_some(endpoint);
            let output = crate::movement::step_frame(frame, 0.05).remove(0);
            frame.units[0].position = output.position;
            frame.units[0].target = output.target;
            frame.units[0].progress = output.progress;
            frame.units[0].plain_progress = output.plain_progress;
            frame.units[0].physical_escape = output.physical_escape;
        }
        assert_eq!(frames[0], frames[1], "movement outputs at tick {tick}");
    }
    assert!(cached.diagnostics.direct_query_cache_hits > 20);
    assert_eq!(frames[0].units[0].position, a.endpoint);
}

/// A broken band in columns 2..=4. A sweep from column 1 to column 6 is
/// refused by the geometry (partial sweep); columns 5..=7 stay flat.
fn refused_band() -> (Terrain, Vec2) {
    let mut t = Terrain::flat();
    for y in 0..8 {
        for x in 2..=4 {
            if (x + y) % 2 == 0 {
                t.observe(Cell { x, y }, 1);
            }
        }
    }
    (t, Vec2::new(6.5, 1.5))
}

#[test]
fn direct_refusal_is_swept_once_for_each_member_while_it_stays_in_one_cell() {
    const MEMBERS: usize = 3;
    const TICKS: usize = 5;
    let (t, endpoint) = refused_band();
    let mut n = ViewerNavigation::default();
    let mut members: Vec<Member> = (0..MEMBERS)
        .map(|_| {
            let mut m = member();
            m.endpoint = endpoint;
            m
        })
        .collect();
    let mut reads_after_first_tick = 0;
    for tick in 0..TICKS {
        n.diagnostics.edge_checks_this_tick = 0;
        for (index, m) in members.iter_mut().enumerate() {
            // Each tick has a new exact pose inside cell (1, 1).
            let from = Vec2::new(1.2 + 0.05 * tick as f32, 1.2 + 0.1 * index as f32);
            assert_eq!(n.direct_query(m, &t, from, endpoint), Certificate::Pending);
            assert!(m.direct_query.is_none(), "a refusal is not an exact result");
        }
        // A hit takes the same edge quota as a fresh query.
        assert_eq!(n.diagnostics.edge_checks_this_tick, MEMBERS);
        if tick == 0 {
            reads_after_first_tick = t.reads.get();
        }
    }
    assert_eq!(
        n.diagnostics.direct_query_cache_misses, MEMBERS as u64,
        "one sweep for each member, not one for each member and tick"
    );
    assert_eq!(
        n.diagnostics.direct_query_cache_hits,
        (MEMBERS * (TICKS - 1)) as u64
    );
    assert_eq!(
        t.reads.get(),
        reads_after_first_tick,
        "a hit reran geometry"
    );
}

#[test]
fn direct_refusal_ends_at_the_first_query_from_another_cell() {
    let (t, endpoint) = refused_band();
    let mut n = ViewerNavigation::default();
    let mut m = member();
    m.endpoint = endpoint;
    for x in [1.2, 1.6] {
        assert_eq!(
            n.direct_query(&mut m, &t, Vec2::new(x, 1.5), endpoint),
            Certificate::Pending
        );
    }
    assert_eq!(n.diagnostics.direct_query_cache_misses, 1);
    assert_eq!(n.diagnostics.direct_query_cache_hits, 1);
    // The first pose in a new cell asks the geometry again: the line is clear there.
    assert_eq!(
        n.direct_query(&mut m, &t, Vec2::new(5.5, 1.5), endpoint),
        Certificate::Clear
    );
    assert_eq!(n.diagnostics.direct_query_cache_misses, 2);
    assert!(m.direct_refusal.is_none());
}

#[test]
fn direct_refusal_ends_with_each_other_input() {
    let (mut t, endpoint) = refused_band();
    let mut n = ViewerNavigation::default();
    let mut m = member();
    m.endpoint = endpoint;
    let from = Vec2::new(1.5, 1.5);
    let mut misses = 0;
    let mut fresh = |n: &mut ViewerNavigation, m: &mut Member, t: &Terrain, to: Vec2| {
        assert_eq!(n.direct_query(m, t, from, to), Certificate::Pending);
        misses += 1;
        assert_eq!(n.diagnostics.direct_query_cache_misses, misses);
    };
    fresh(&mut n, &mut m, &t, endpoint);
    fresh(&mut n, &mut m, &t, endpoint + Vec2::new(0.0, 0.1));
    m.radius = 0.25f32.to_bits();
    fresh(&mut n, &mut m, &t, endpoint + Vec2::new(0.0, 0.1));
    m.epoch += 1;
    fresh(&mut n, &mut m, &t, endpoint + Vec2::new(0.0, 0.1));
    // A terrain change outside the lease still ends the refusal (terrain epoch).
    t.observe(Cell { x: 7, y: 7 }, 1);
    fresh(&mut n, &mut m, &t, endpoint + Vec2::new(0.0, 0.1));
}

#[test]
fn a_resource_refusal_is_asked_again_on_the_next_tick() {
    let (t, endpoint) = refused_band();
    let mut n = ViewerNavigation::default();
    let mut m = member();
    m.endpoint = endpoint;
    let from = Vec2::new(1.5, 1.5);
    // No memory for the lease rows: a transient refusal, never retained.
    m.direct = Vec::new();
    n.member_buffer_bytes = MAX_BYTES;
    for expected in 1..=2 {
        assert_eq!(
            n.direct_query(&mut m, &t, from, endpoint),
            Certificate::Pending
        );
        assert!(
            m.direct_refusal.is_none(),
            "a resource refusal was retained"
        );
        assert_eq!(n.diagnostics.direct_query_cache_misses, expected);
    }
    // The edge quota is transient too.
    n.member_buffer_bytes = 0;
    n.diagnostics.edge_checks_this_tick = MAX_EDGES;
    assert_eq!(
        n.direct_query(&mut m, &t, from, endpoint),
        Certificate::Pending
    );
    assert!(m.direct_refusal.is_none());
    n.diagnostics.edge_checks_this_tick = 0;
    assert_eq!(
        n.direct_query(&mut m, &t, from, endpoint),
        Certificate::Pending
    );
    assert_eq!(n.diagnostics.direct_query_cache_misses, 3);
}

#[test]
fn two_orders_take_turns_at_the_work_limit_and_do_not_delay_each_other() {
    const MEMBERS: usize = 4;
    let (t, _) = refused_band();
    let ends = [Vec2::new(6.5, 1.5), Vec2::new(6.5, 5.5)];
    let mut n = ViewerNavigation::default();
    // Two orders: each has its own endpoint and no destination region.
    let mut orders: Vec<Vec<Member>> = ends
        .iter()
        .map(|&endpoint| {
            (0..MEMBERS)
                .map(|_| {
                    let mut m = member();
                    m.endpoint = endpoint;
                    m
                })
                .collect()
        })
        .collect();
    let mut served_tick = [[None; MEMBERS]; 2];
    for tick in 1..=MEMBERS as u32 {
        n.begin(tick);
        // Order 0 is always asked first, with all its members.
        for (order, members) in orders.iter_mut().enumerate() {
            for (index, m) in members.iter_mut().enumerate() {
                let from = Vec2::new(1.5, 1.5 + 4. * order as f32);
                let before = n.diagnostics.direct_query_cache_misses;
                assert_eq!(
                    n.direct_query(m, &t, from, ends[order]),
                    Certificate::Pending
                );
                if n.diagnostics.direct_query_cache_misses > before {
                    served_tick[order][index].get_or_insert(tick);
                    // Make each real sweep as costly as the whole limit of its order.
                    for row in &mut n.direct_order_work {
                        if row.used {
                            row.work = row.work.max(DIRECT_QUERY_WORK);
                        }
                    }
                }
            }
        }
    }
    // One member of each order is served on each tick: the large first order does
    // not delay the second order, and every member is served in MEMBERS ticks.
    for order in 0..2 {
        let mut ticks: Vec<u32> = served_tick[order]
            .iter()
            .map(|tick| tick.expect("a member was put off without end"))
            .collect();
        ticks.sort_unstable();
        assert_eq!(
            ticks,
            (1..=MEMBERS as u32).collect::<Vec<_>>(),
            "order {order}"
        );
    }
    assert_eq!(
        n.diagnostics.direct_query_deferrals,
        (2 * (MEMBERS * (MEMBERS - 1) / 2)) as u64
    );
}
