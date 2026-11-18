#![allow(
    clippy::excessive_precision,
    reason = "Captures retain their original decimal form for reproducible geometry."
)]
//! Differential exact-kernel cases for the sufficient flat rectangle proof.
use super::*;
use turts_rules::terrain::geometry::DualSupportContact;
struct Fixture {
    grid: Grid,
    heights: Vec<Option<i32>>,
    revision: u64,
}
impl Fixture {
    fn flat(height: i32) -> Self {
        Self {
            grid: Grid {
                columns: 16,
                rows: 16,
                length_scale: 1000,
                cell_width: 1000,
                elevation_step: 250,
                floor: -8,
                ceiling: 8,
            },
            heights: vec![Some(height); 256],
            revision: 1,
        }
    }
}
impl Surface for Fixture {
    fn elevation(&self, cell: Cell) -> Option<i32> {
        if cell.x >= self.grid.columns || cell.y >= self.grid.rows {
            return None;
        }
        self.heights[(cell.y * self.grid.columns + cell.x) as usize]
    }
}
impl TerrainRead for Fixture {
    fn grid(&self) -> Grid {
        self.grid
    }
    fn rectangle(&self, min: Cell, max: Cell, lease: &mut BTreeMap<usize, u64>) -> bool {
        let mut height = None;
        for y in min.y..=max.y {
            for x in min.x..=max.x {
                let cell = Cell { x, y };
                let Some(value) = self.elevation(cell) else {
                    return false;
                };
                lease.insert((y * self.grid.columns + x) as usize, self.revision);
                if height.is_some_and(|old| old != value) {
                    return false;
                }
                height = Some(value);
            }
        }
        true
    }
    fn valid_lease(&self, lease: &BTreeMap<usize, u64>) -> bool {
        lease.values().all(|&r| r == self.revision)
    }
}
fn raw(
    terrain: &Fixture,
    from: Vec2,
    to: Vec2,
    axis: Vec2,
    radius: f32,
    profile: Traversal,
) -> DualSupportContact {
    if certify(terrain, from, to, radius, profile, &mut BTreeMap::new()) != Certificate::Clear {
        return DualSupportContact::Unresolved;
    }
    let reach = 2.0 * f64::from(radius);
    let offset = Point {
        x: -f64::from(axis.y) * reach,
        y: f64::from(axis.x) * reach,
    };
    terrain.grid.first_dual_support_contact(
        terrain,
        point(from),
        point(to),
        [
            offset,
            Point {
                x: -offset.x,
                y: -offset.y,
            },
        ],
        f64::from(radius),
        profile.max_step,
        &mut Budget::new(16_384),
    )
}
fn profile() -> Traversal {
    Traversal {
        max_step: 0,
        max_grade: Ratio {
            numerator: 0,
            denominator: 1,
        },
    }
}
#[test]
fn generated_flat_and_nonflat_contacts_match_the_existing_exact_kernel() {
    let mut random = 0x91c5_ba27_u64;
    let mut shortcuts = 0;
    for n in 0..256 {
        random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut terrain = Fixture::flat((n % 5) - 2);
        if n % 3 == 1 {
            terrain.heights[((random >> 32) as usize) % 256] = Some(4);
        }
        if n % 3 == 2 {
            terrain.heights[((random >> 32) as usize) % 256] = None;
        }
        let from = Vec2::new(
            4. + ((random >> 8) % 4) as f32 * 0.5,
            4. + ((random >> 12) % 4) as f32 * 0.5,
        );
        let to = from
            + if n % 2 == 0 {
                Vec2::new(1.5, 0.)
            } else {
                Vec2::new(0., 1.5)
            };
        let axis = if n % 2 == 0 { Vec2::X } else { Vec2::Y };
        let radius = if n % 4 < 2 { 0.25 } else { 0.5 };
        let offset = Point {
            x: -f64::from(axis.y) * 2. * f64::from(radius),
            y: f64::from(axis.x) * 2. * f64::from(radius),
        };
        let mut lease = BTreeMap::new();
        if flat_probe_rectangle(
            &terrain,
            point(from),
            point(to),
            offset,
            f64::from(radius),
            &mut lease,
        ) {
            shortcuts += 1;
            assert!(!lease.is_empty());
        }
        assert_eq!(
            entrance_contact(&terrain, from, to, axis, radius, profile()),
            raw(&terrain, from, to, axis, radius, profile()),
            "case{n}"
        );
    }
    assert!(
        shortcuts > 100,
        "generated cases must exercise the real fast certificate"
    );
}
#[test]
fn unknown_edges_tangency_and_nonflat_spread_keep_the_original_fallback() {
    for radius in [0.25f32, 0.5, 1.125] {
        for from in [
            Vec2::new(radius, 4.),
            Vec2::new(4., 3. * radius),
            Vec2::new(8., 8.),
        ] {
            for delta in [Vec2::ZERO, Vec2::X * 0.25, Vec2::Y * 0.25] {
                for mode in 0..3 {
                    let mut terrain = Fixture::flat(0);
                    if mode == 1 {
                        for y in 0..16 {
                            terrain.heights[(y * 16 + 5) as usize] = Some(4);
                        }
                    }
                    if mode == 2 {
                        for y in 0..16 {
                            terrain.heights[(y * 16 + 5) as usize] = None;
                        }
                    }
                    assert_eq!(
                        entrance_contact(&terrain, from, from + delta, Vec2::X, radius, profile()),
                        raw(&terrain, from, from + delta, Vec2::X, radius, profile())
                    );
                }
            }
        }
    }
}
#[test]
fn full_probe_rectangle_dependencies_are_read_again_after_a_local_change() {
    let mut terrain = Fixture::flat(0);
    let from = Point { x: 5., y: 5. };
    let to = Point { x: 6., y: 5. };
    let offset = Point { x: 0., y: 1. };
    let mut lease = BTreeMap::new();
    assert!(flat_probe_rectangle(
        &terrain, from, to, offset, 0.5, &mut lease
    ));
    for y in 3..=6 {
        for x in 4..=6 {
            assert!(lease.contains_key(&(y * 16 + x)));
        }
    }
    terrain.revision += 1;
    terrain.heights[4 * 16 + 5] = Some(4);
    terrain.heights[6 * 16 + 5] = Some(4);
    assert!(!terrain.valid_lease(&lease));
    assert!(!flat_probe_rectangle(
        &terrain,
        from,
        to,
        offset,
        0.5,
        &mut BTreeMap::new()
    ));
    assert_eq!(
        entrance_contact(
            &terrain,
            Vec2::new(5., 5.),
            Vec2::new(6., 5.),
            Vec2::X,
            0.5,
            profile()
        ),
        raw(
            &terrain,
            Vec2::new(5., 5.),
            Vec2::new(6., 5.),
            Vec2::X,
            0.5,
            profile()
        )
    );
}
#[test]
fn nonflat_other_flank_preserves_the_original_kernel_fallback() {
    let mut terrain = Fixture::flat(0);
    for x in 0..16 {
        terrain.heights[3 * 16 + x] = Some(4);
    }
    let from = Vec2::new(5., 5.);
    let to = Vec2::new(6., 5.);
    let offset = Point { x: 0., y: 1. };
    assert!(!flat_probe_rectangle(
        &terrain,
        point(from),
        point(to),
        offset,
        0.5,
        &mut BTreeMap::new()
    ));

    assert_eq!(
        raw(&terrain, from, to, Vec2::X, 0.5, profile()),
        DualSupportContact::Outside
    );
    assert_eq!(
        entrance_contact(&terrain, from, to, Vec2::X, 0.5, profile()),
        DualSupportContact::Outside
    );
}
#[test]
fn unknown_other_flank_stays_unresolved_even_when_one_flank_is_clear() {
    let mut terrain = Fixture::flat(0);
    for x in 0..16 {
        terrain.heights[3 * 16 + x] = None;
    }
    let from = Vec2::new(5., 5.);
    let to = Vec2::new(6., 5.);
    assert_eq!(
        raw(&terrain, from, to, Vec2::X, 0.5, profile()),
        DualSupportContact::Unresolved
    );
    assert_eq!(
        entrance_contact(&terrain, from, to, Vec2::X, 0.5, profile()),
        DualSupportContact::Unresolved
    );
}

#[test]
fn public_rectangular_throat_requires_complete_floor_flanks_and_both_mouths() {
    let mut terrain = Fixture::flat(20);
    for x in 6..10 {
        for y in 0..16 {
            if !(6..9).contains(&y) {
                terrain.heights[y * 16 + x] = Some(24);
            }
        }
    }
    let from = Vec2::new(4.75, 7.5);
    let goal = Vec2::new(14., 7.5);
    let mut count = 0;
    let found = public_throat(
        &terrain,
        from,
        goal,
        1.125,
        profile(),
        &mut BTreeMap::new(),
        &mut || {
            count += 1;
            true
        },
    )
    .expect("current exact rectangular resource");
    assert_eq!(
        (found.min, found.max),
        (Vec2::new(6., 6.), Vec2::new(10., 9.))
    );
    assert!(count <= 128, "bounded canonical resource reads: {count}");
    terrain.heights[7 * 16 + 5] = None;
    assert!(
        public_throat(
            &terrain,
            from,
            goal,
            1.125,
            profile(),
            &mut BTreeMap::new(),
            &mut || true
        )
        .is_none(),
        "unknown mouth cannot authorize passage"
    );
}

#[test]
fn public_passage_does_not_authorize_a_throat_the_body_cannot_enter() {
    let mut terrain = Fixture::flat(20);
    for x in 6..10 {
        for y in 0..16 {
            if y != 7 {
                terrain.heights[y * 16 + x] = Some(24);
            }
        }
    }
    assert!(
        public_throat(
            &terrain,
            Vec2::new(4.75, 7.5),
            Vec2::new(14., 7.5),
            1.125,
            profile(),
            &mut BTreeMap::new(),
            &mut || true
        )
        .is_none()
    );
}

#[test]
fn captured_rounded_mouth_bodies_are_outside_the_exact_public_contact() {
    let mut terrain = Fixture::flat(0);
    terrain.grid.columns = 64;
    terrain.grid.rows = 48;
    terrain.heights = vec![Some(0); 64 * 48];
    for x in 30..34 {
        for y in 0..48 {
            if !(22..25).contains(&y) {
                terrain.heights[y * 64 + x] = Some(4);
            }
        }
    }
    for p in [
        Vec2::new(35.030757904052734, 22.38410758972168),
        Vec2::new(35.030757904052734, 24.615888595581055),
    ] {
        assert_eq!(
            entrance_contact_with_probe(&terrain, p, p, Vec2::X, 1.1, 1.1, profile()),
            DualSupportContact::Outside
        );
    }
    let small = Vec2::new(32., 23.5);
    assert_eq!(
        entrance_contact_with_probe(&terrain, small, small, Vec2::X, 0.4, 1.1, profile()),
        DualSupportContact::Outside,
        "entrance support can end inside the certified floor; it is not a departure witness"
    );
}
