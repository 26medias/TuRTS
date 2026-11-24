use super::{SpreadMember, StaticFootprint, angle, spread_destinations};
use bevy_math::Vec2;

#[test]
fn pool_has_primary_slots_and_ceiling_spares() {
    for count in [1_usize, 7, 8, 28] {
        let members: Vec<_> = (0..count)
            .map(|id| SpreadMember {
                id: id as u32 + 1,
                position: Vec2::new(10.0 + id as f32, 10.0),
                radius: 0.5,
            })
            .collect();
        let (pool, assignments) = spread_destinations(
            &members,
            Vec2::new(32.0, 32.0),
            &[],
            &[],
            Vec2::new(64.0, 64.0),
        );
        assert_eq!(pool.primary_count, count as u32);
        assert_eq!(pool.slots.len(), count + 6_usize.max(count.div_ceil(4)));
        assert_eq!(assignments.len(), count);
    }
}

#[test]
fn pool_spacing_uses_the_largest_unit_radius() {
    let members = [
        SpreadMember {
            id: 1,
            position: Vec2::new(10.0, 10.0),
            radius: 0.2,
        },
        SpreadMember {
            id: 2,
            position: Vec2::new(11.0, 10.0),
            radius: 0.7,
        },
    ];
    let (pool, _) = spread_destinations(
        &members,
        Vec2::new(32.0, 32.0),
        &[],
        &[],
        Vec2::new(64.0, 64.0),
    );
    assert_eq!(pool.spacing, 1.5);
    for (index, slot) in pool.slots.iter().enumerate() {
        for other in &pool.slots[index + 1..] {
            assert!(slot.distance(*other) >= pool.spacing - 1.0e-5);
        }
    }
}

#[test]
fn pool_spacing_uses_small_actual_radii_without_a_half_unit_floor() {
    let members = [SpreadMember {
        id: 1,
        position: Vec2::new(4.0, 4.0),
        radius: 0.05,
    }];
    let (pool, _) = spread_destinations(
        &members,
        Vec2::new(32.0, 32.0),
        &[],
        &[],
        Vec2::new(64.0, 64.0),
    );
    assert!((pool.spacing - 0.2).abs() < 1.0e-6);
    assert!(pool.slots[0].distance(pool.slots[1]) >= 0.2 - 1.0e-6);
}

#[test]
fn pool_starts_at_origin_then_orders_equal_radius_by_angle() {
    let member = [SpreadMember {
        id: 1,
        position: Vec2::new(4.0, 4.0),
        radius: 0.5,
    }];
    let (pool, assignments) = spread_destinations(
        &member,
        Vec2::new(32.0, 32.0),
        &[],
        &[],
        Vec2::new(64.0, 64.0),
    );
    assert_eq!(pool.slots.len(), 7);
    assert_eq!(pool.slots[0], Vec2::new(32.0, 32.0));
    let s = 1.1_f32;
    let expected = [
        Vec2::new(32.0 - 0.5 * s, 32.0 - 3.0_f32.sqrt() * 0.5 * s),
        Vec2::new(32.0 + 0.5 * s, 32.0 - 3.0_f32.sqrt() * 0.5 * s),
        Vec2::new(32.0 + s, 32.0),
        Vec2::new(32.0 + 0.5 * s, 32.0 + 3.0_f32.sqrt() * 0.5 * s),
        Vec2::new(32.0 - 0.5 * s, 32.0 + 3.0_f32.sqrt() * 0.5 * s),
        Vec2::new(32.0 - s, 32.0),
    ];
    for (actual, expected) in pool.slots[1..].iter().zip(expected) {
        assert!(
            (actual.x - expected.x).abs() < 1.0e-5 && (actual.y - expected.y).abs() < 1.0e-5,
            "actual {actual:?} expected {expected:?}"
        );
    }
    assert_eq!(assignments, vec![(1, Vec2::new(32.0, 32.0))]);
}

#[test]
fn large_pool_enumeration_is_nearest_first_and_complete() {
    let members: Vec<_> = (0..200)
        .map(|id| SpreadMember {
            id: id + 1,
            position: Vec2::new(10.0 + id as f32 * 0.01, 10.0),
            radius: 0.5,
        })
        .collect();
    let (pool, assignments) = spread_destinations(
        &members,
        Vec2::new(32.0, 32.0),
        &[],
        &[],
        Vec2::new(64.0, 64.0),
    );
    assert_eq!(pool.slots.len(), 250);
    assert_eq!(assignments.len(), 200);
    assert_eq!(pool.slots[0], Vec2::new(32.0, 32.0));
    let keys: Vec<_> = pool
        .slots
        .iter()
        .map(|slot| {
            let delta = *slot - Vec2::new(32.0, 32.0);
            let ring_key = (delta.length_squared() / (pool.spacing * pool.spacing)).round() as i64;
            let angle = angle(*slot, Vec2::new(32.0, 32.0));
            (ring_key, angle)
        })
        .collect();
    assert!(keys.windows(2).all(|pair| {
        pair[0].0 < pair[1].0 || (pair[0].0 == pair[1].0 && pair[0].1 <= pair[1].1 + 1.0e-5)
    }));
}

#[test]
fn pool_order_matches_independent_axial_lattice_reference_at_medium_and_large_sizes() {
    let center = Vec2::new(32.0, 32.0);
    let spacing = 1.1_f32;
    for member_count in [135_u32, 1_000] {
        let members: Vec<_> = (0..member_count)
            .map(|id| SpreadMember {
                id: id + 1,
                position: Vec2::new(10.0 + id as f32 * 0.001, 12.0),
                radius: 0.5,
            })
            .collect();
        let spare_count = 6_usize.max(members.len().div_ceil(4));
        let wanted = members.len() + spare_count;
        let mut reference = vec![(0_i64, 0_i64, center)];
        for i in -64_i32..=64 {
            for j in -64_i32..=64 {
                let ring = i.abs().max(j.abs()).max((i + j).abs());
                if ring == 0 {
                    continue;
                }
                let position = Vec2::new(
                    center.x + spacing * (i as f32 + 0.5 * j as f32),
                    center.y + spacing * (3.0_f32.sqrt() * 0.5 * j as f32),
                );
                if position.x < 0.5 || position.y < 0.5 || position.x > 63.5 || position.y > 63.5 {
                    continue;
                }
                let ring_key = i64::from(i) * i64::from(i)
                    + i64::from(i) * i64::from(j)
                    + i64::from(j) * i64::from(j);
                let angle_key = ((position.y - center.y).atan2(position.x - center.x) * 1_000_000.0)
                    .round() as i64;
                reference.push((ring_key, angle_key, position));
            }
        }
        reference.sort_by_key(|(ring_key, angle_key, _)| (*ring_key, *angle_key));
        let expected: Vec<_> = reference
            .into_iter()
            .take(wanted)
            .map(|(_, _, position)| position)
            .collect();

        let (pool, assignments) =
            spread_destinations(&members, center, &[], &[], Vec2::new(64.0, 64.0));
        assert_eq!(pool.slots.len(), wanted, "member count {member_count}");
        assert_eq!(
            pool.primary_count, member_count,
            "member count {member_count}"
        );
        assert_eq!(assignments.len(), member_count as usize);
        let mismatch = pool
            .slots
            .iter()
            .zip(&expected)
            .position(|(actual, expected)| actual != expected);
        if let Some(index) = mismatch {
            let start = index.saturating_sub(2);
            let end = (index + 3).min(expected.len());
            panic!(
                "member count {member_count}, first mismatch {index}; actual={:?}; expected={:?}",
                &pool.slots[start..end],
                &expected[start..end]
            );
        }
    }
}

#[test]
fn static_footprint_is_skipped_and_assignments_repeat_bit_exactly() {
    let members: Vec<_> = (0..20)
        .map(|id| SpreadMember {
            id: id as u32 + 1,
            position: Vec2::new(10.0 + id as f32 * 0.5, 10.0),
            radius: 0.5,
        })
        .collect();
    let obstacle = [StaticFootprint {
        position: Vec2::new(32.0, 32.0),
        radius: 2.0,
    }];
    let a = spread_destinations(
        &members,
        Vec2::new(32.0, 32.0),
        &[],
        &obstacle,
        Vec2::new(64.0, 64.0),
    );
    let b = spread_destinations(
        &members,
        Vec2::new(32.0, 32.0),
        &[],
        &obstacle,
        Vec2::new(64.0, 64.0),
    );
    assert_eq!(a, b);
    assert!(
        a.0.slots
            .iter()
            .all(|slot| slot.distance(obstacle[0].position) >= 2.5)
    );
}

#[test]
fn singleton_nearest_slot_uses_intersecting_holder_clearance_boundaries() {
    let goal = Vec2::new(20.0, 20.0);
    let radius = 0.85_f32;
    let other = 0.2_f32;
    let holders = [
        StaticFootprint {
            position: Vec2::new(19.7, 20.0),
            radius: other,
        },
        StaticFootprint {
            position: Vec2::new(20.3, 20.0),
            radius: other,
        },
    ];
    let members = [SpreadMember {
        id: 7,
        position: Vec2::new(18.0, 18.0),
        radius,
    }];
    let (pool, _) = spread_destinations(&members, goal, &[], &holders, Vec2::splat(48.0));
    let chosen = pool.slots[0];
    let r = f64::from(radius) + f64::from(other);
    let x = f64::from(goal.x) - f64::from(holders[0].position.x);
    let expected = (r * r - x * x).sqrt();
    assert!(
        (f64::from(chosen.distance(goal)) - expected).abs() < 0.0001,
        "intersection optimum {expected}, got {chosen:?}"
    );
    for holder in holders {
        assert!(chosen.distance(holder.position) >= radius + holder.radius);
    }
}
#[test]
fn singleton_nearest_slot_handles_clearance_circle_at_map_edge() {
    let goal = Vec2::new(0.9, 10.0);
    let radius = 0.85_f32;
    let holder = StaticFootprint {
        position: Vec2::new(1.0, 10.0),
        radius: 0.35,
    };
    let members = [SpreadMember {
        id: 7,
        position: Vec2::new(10.0, 18.0),
        radius,
    }];
    let (pool, _) = spread_destinations(&members, goal, &[], &[holder], Vec2::new(48.0, 32.0));
    let chosen = pool.slots[0];
    let r = f64::from(radius) + f64::from(holder.radius);
    let x = f64::from(holder.position.x) - f64::from(radius);
    let y = (r * r - x * x).sqrt();
    let expected = (f64::from(goal.x) - f64::from(radius)).hypot(y);
    assert!(
        (f64::from(chosen.distance(goal)) - expected).abs() < 0.0001,
        "edge optimum {expected}, got {chosen:?}"
    );
    assert!(chosen.x >= radius);
    assert!(chosen.distance(holder.position) >= radius + holder.radius);
}

/// The six lattice sites round `point`, in angular order from the +x side.
fn ring_one(point: Vec2, spacing: f32) -> [Vec2; 6] {
    [(1, 0), (0, 1), (-1, 1), (-1, 0), (0, -1), (1, -1)].map(|(i, j): (i32, i32)| {
        Vec2::new(
            point.x + spacing * (i as f32 + 0.5 * j as f32),
            point.y + spacing * (3.0_f32.sqrt() * 0.5 * j as f32),
        )
    })
}

fn three_members() -> Vec<SpreadMember> {
    (0..3)
        .map(|index| SpreadMember {
            id: index,
            position: Vec2::new(5.0 + 1.2 * index as f32, 5.0),
            radius: 0.5,
        })
        .collect()
}

/// The gap between two resting neighbours is 0.1: a body cannot pass. A free site that resting
/// bodies seal on every side is not a destination, although it is the nearest free site.
#[test]
fn a_pool_takes_no_slot_in_a_pocket_that_resting_bodies_seal() {
    let point = Vec2::new(20.0, 20.0);
    let reserved: Vec<_> = ring_one(point, 1.1)
        .into_iter()
        .map(|position| super::ReservedFootprint {
            position,
            radius: 0.5,
        })
        .collect();
    let (pool, assignments) = spread_destinations(
        &three_members(),
        point,
        &reserved,
        &[],
        Vec2::new(64.0, 64.0),
    );
    assert_eq!(assignments.len(), 3);
    assert_eq!(pool.primary_count, 3);
    assert!(
        pool.slots.iter().all(|slot| slot.distance(point) > 1.0),
        "the sealed centre site is a slot: {:?}",
        pool.slots
    );
    // The canonical order holds: nearest first.
    assert!(
        pool.slots
            .windows(2)
            .all(|pair| pair[0].distance(point) <= pair[1].distance(point) + 1.0e-4)
    );
    for reserved in &reserved {
        for slot in &pool.slots {
            assert!(slot.distance(reserved.position) >= 1.1 - 1.0e-4);
        }
    }
}

/// The control of the test above: with two adjacent ring sites open the gap is wide enough
/// (2.2 between the bodies at its sides), so the centre site is reachable and stays slot zero.
#[test]
fn a_pool_keeps_a_site_behind_a_gap_that_a_body_can_pass() {
    let point = Vec2::new(20.0, 20.0);
    let reserved: Vec<_> = ring_one(point, 1.1)[2..]
        .iter()
        .map(|&position| super::ReservedFootprint {
            position,
            radius: 0.5,
        })
        .collect();
    let (pool, assignments) = spread_destinations(
        &three_members(),
        point,
        &reserved,
        &[],
        Vec2::new(64.0, 64.0),
    );
    assert_eq!(assignments.len(), 3);
    assert_eq!(pool.slots[0], point, "the open centre site is slot zero");
    // One open ring site alone is not a gate: both bodies beside it are 1.905 apart.
    let reserved: Vec<_> = ring_one(point, 1.1)[1..]
        .iter()
        .map(|&position| super::ReservedFootprint {
            position,
            radius: 0.5,
        })
        .collect();
    let (pool, _) = spread_destinations(
        &three_members(),
        point,
        &reserved,
        &[],
        Vec2::new(64.0, 64.0),
    );
    assert!(
        pool.slots.iter().all(|slot| slot.distance(point) > 1.0),
        "the centre site behind a closed gate is a slot: {:?}",
        pool.slots
    );
}

/// A closed ring of resting bodies round the point. Each free site inside the ring is sealed,
/// although the sites inside connect to each other. The pool lies outside the ring. (The flood
/// runs only when a footprint closes a site that the pool scan examined: a wall far outside
/// the scanned rings is not seen.)
#[test]
fn a_pool_for_members_outside_a_closed_ring_lies_outside_the_ring() {
    let point = Vec2::new(30.0, 30.0);
    let spacing = 1.1_f32;
    // Hex ring 2 of the pool lattice, complete: 12 bodies, each 0.1 from the next.
    let mut reserved = Vec::new();
    let ring = 2_i32;
    for side in 0..6 {
        for step in 0..ring {
            let (i, j) = match side {
                0 => (ring - step, step),
                1 => (-step, ring),
                2 => (-ring, ring - step),
                3 => (-ring + step, -step),
                4 => (step, -ring),
                _ => (ring, -ring + step),
            };
            reserved.push(super::ReservedFootprint {
                position: Vec2::new(
                    point.x + spacing * (i as f32 + 0.5 * j as f32),
                    point.y + spacing * (3.0_f32.sqrt() * 0.5 * j as f32),
                ),
                radius: 0.5,
            });
        }
    }
    let (pool, assignments) = spread_destinations(
        &three_members(),
        point,
        &reserved,
        &[],
        Vec2::new(64.0, 64.0),
    );
    assert_eq!(assignments.len(), 3);
    assert!(
        pool.slots
            .iter()
            .all(|slot| slot.distance(point) > 2.0 * spacing * 0.86),
        "a slot lies inside the closed ring: {:?}",
        pool.slots
    );
}
