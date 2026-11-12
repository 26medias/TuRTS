use super::{
    MoveProgress, MovementActivity, MovementFrame, MovementFrameUnit, MovementInput, MovementPool,
    MovementPoolIndex, PlainMoveProgress, SpreadMember, StaticFootprint, angle,
    spread_destinations, step, step_frame,
};
use bevy_math::Vec2;
use std::collections::BTreeMap;

#[test]
fn progressless_singleton_move_never_spends_two_steps_in_one_tick() {
    let target = Vec2::new(22.0, 10.0);
    let frame = MovementFrame {
        bounds: Vec2::new(64.0, 64.0),
        tick_hz: 20,
        units: vec![MovementFrameUnit {
            unit_id: 1,
            owner_slot: 0,
            unit_type: 1,
            position: Vec2::new(21.2, 10.0),
            radius: 0.5,
            speed: Some(8.0),
            target: Some(target),
            progress: None,

            plain_progress: None,
            physical_escape: None,
        }],
        pools: BTreeMap::new(),
    };
    let output = step_frame(&frame, 0.05).remove(0);
    assert!((output.position.x - 21.6).abs() < 1.0e-5);
    assert_eq!(output.position.y, 10.0);
    assert_eq!(output.target, Some(target));
    assert!((output.position.distance(frame.units[0].position) - 0.4).abs() < 1.0e-5);
}

#[test]
fn progressless_singleton_move_keeps_the_exact_free_step_before_arrival() {
    let target = Vec2::new(22.0, 10.0);
    let frame = MovementFrame {
        bounds: Vec2::new(64.0, 64.0),
        tick_hz: 20,
        units: vec![MovementFrameUnit {
            unit_id: 1,
            owner_slot: 0,
            unit_type: 1,
            position: Vec2::new(21.577, 10.0),
            radius: 0.5,
            speed: Some(8.0),
            target: Some(target),
            progress: None,

            plain_progress: None,
            physical_escape: None,
        }],
        pools: BTreeMap::new(),
    };
    let output = step_frame(&frame, 0.05).remove(0);
    let expected = step(
        &MovementInput {
            position: frame.units[0].position,
            target,
            speed: 8.0,
        },
        0.05,
    );
    assert_eq!(output.position, expected.position);
    assert_eq!(output.target, Some(target));
    assert!(output.position.distance(frame.units[0].position) <= 0.4);
}

#[test]
fn active_group_progress_does_not_add_a_post_step_arrival_snap() {
    let target = Vec2::new(2.0, 1.0);
    let position = Vec2::new(1.83, 1.0);
    let frame = MovementFrame {
        bounds: Vec2::splat(16.0),
        tick_hz: 20,
        units: vec![MovementFrameUnit {
            unit_id: 1,
            owner_slot: 0,
            unit_type: 1,
            position,
            radius: 0.5,
            speed: Some(3.0),
            target: Some(target),
            progress: Some(MoveProgress {
                pool_id: 1,
                slot: target,
                activity: MovementActivity::Moving,
                pass_mode: false,
                stalling_on_group: false,
                stalled_ticks: 0,
                best_distance: position.distance(target),
                claim_count: 0,

                follow_origin: None,
                gather_goal: None,
                fallback: None,
            }),
            plain_progress: None,
            physical_escape: None,
        }],
        pools: BTreeMap::new(),
    };

    let output = step_frame(&frame, 0.05).remove(0);
    let expected = step(
        &MovementInput {
            position,
            target,
            speed: 3.0,
        },
        0.05,
    )
    .position;

    assert_eq!(output.position, expected);
    assert_eq!(output.target, Some(target));
    assert_eq!(output.progress.unwrap().activity, MovementActivity::Moving);
}

#[test]
fn contact_correction_does_not_add_a_post_step_arrival_snap() {
    let target = Vec2::new(5.0, 5.0);
    let position = Vec2::new(4.83, 5.0);
    let modes = [
        (None, None),
        (
            Some(MoveProgress {
                pool_id: 1,
                slot: target,
                activity: MovementActivity::Moving,
                pass_mode: false,
                stalling_on_group: false,
                stalled_ticks: 0,
                best_distance: position.distance(target),
                claim_count: 0,

                follow_origin: None,
                gather_goal: None,
                fallback: None,
            }),
            None,
        ),
        (
            None,
            Some(PlainMoveProgress {
                original_point: target,
                slot: target,
                pool_slots: vec![],
                gather_goal: None,
                activity: MovementActivity::Moving,
                stalled_ticks: 0,
                best_distance: position.distance(target),
                claim_count: 1,
                failure_recorded: false,
            }),
        ),
    ];

    for (progress, plain_progress) in modes {
        let frame = MovementFrame {
            bounds: Vec2::splat(16.0),
            tick_hz: 20,
            units: vec![
                MovementFrameUnit {
                    unit_id: 1,
                    owner_slot: 0,
                    unit_type: 1,
                    position,
                    radius: 0.5,
                    speed: Some(3.0),
                    target: Some(target),
                    progress,
                    plain_progress,

                    physical_escape: None,
                },
                MovementFrameUnit {
                    unit_id: 2,
                    owner_slot: 1,
                    unit_type: 2,
                    position: Vec2::new(3.81, 5.0),
                    radius: 0.5,
                    speed: None,
                    target: None,
                    progress: None,
                    plain_progress: None,
                    physical_escape: None,
                },
            ],
            pools: BTreeMap::new(),
        };

        let output = step_frame(&frame, 0.05).remove(0);
        assert!(
            output.position.distance(position) <= 0.150_001,
            "contact movement exceeded one step: {:?}",
            output.position
        );
        assert_eq!(output.target, Some(target));
        assert_ne!(output.position, target);
    }
}

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
fn blocked_escape_chooses_nearest_free_point_when_old_pool_is_far_and_occupied() {
    let origin = Vec2::new(10.0, 10.0);
    let old_slot = Vec2::new(50.0, 50.0);
    let pool = MovementPool {
        pool_id: 7,
        point: old_slot,
        spacing: 1.1,
        primary_count: 1,
        slots: vec![old_slot],
    };
    let frame = MovementFrame {
        bounds: Vec2::new(64.0, 64.0),
        tick_hz: 20,
        units: vec![
            MovementFrameUnit {
                unit_id: 1,
                owner_slot: 0,
                unit_type: 1,
                position: origin,
                radius: 0.5,
                speed: Some(3.0),
                target: Some(old_slot),
                progress: Some(MoveProgress {
                    pool_id: 7,
                    slot: old_slot,
                    activity: MovementActivity::Blocked,
                    pass_mode: true,
                    stalling_on_group: false,
                    stalled_ticks: 60,
                    best_distance: 1.0,
                    claim_count: 3,

                    follow_origin: None,
                    gather_goal: None,
                    fallback: None,
                }),

                plain_progress: None,
                physical_escape: None,
            },
            MovementFrameUnit {
                unit_id: 2,
                owner_slot: 1,
                unit_type: 1,
                position: origin,
                radius: 0.5,
                speed: None,
                target: None,
                progress: None,

                plain_progress: None,
                physical_escape: None,
            },
            MovementFrameUnit {
                unit_id: 3,
                owner_slot: 1,
                unit_type: 1,
                position: old_slot,
                radius: 0.5,
                speed: None,
                target: None,
                progress: None,

                plain_progress: None,
                physical_escape: None,
            },
        ],
        pools: BTreeMap::from([(7, pool)]),
    };
    let output = step_frame(&frame, 0.05);
    let mover = &output[0];
    let escape = mover.target.expect("the blocked escape remains explicit");
    assert_ne!(escape, old_slot, "the blocked old pool point is occupied");
    assert!(
        escape.distance(origin) <= 1.1 + 1.0e-5,
        "nearest legal free point is local: {escape:?}"
    );
    assert!(
        mover.position.distance(origin) <= 0.15 + 1.0e-5,
        "escape advances at most one speed step"
    );
}

#[test]
fn no_spare_pool_blocks_on_the_first_other_group_stall_deadline() {
    let target = Vec2::new(11.0, 10.0);
    let pool = MovementPool {
        pool_id: 8,
        point: target,
        spacing: 1.1,
        primary_count: 1,
        slots: vec![target],
    };
    let frame = MovementFrame {
        bounds: Vec2::new(64.0, 64.0),
        tick_hz: 20,
        units: vec![
            MovementFrameUnit {
                unit_id: 1,
                owner_slot: 0,
                unit_type: 1,
                position: Vec2::new(10.0, 10.0),
                radius: 0.5,
                speed: Some(3.0),
                target: Some(target),
                progress: Some(MoveProgress {
                    pool_id: 8,
                    slot: target,
                    activity: MovementActivity::Moving,
                    pass_mode: false,
                    stalling_on_group: false,
                    stalled_ticks: 19,
                    best_distance: 1.0,
                    claim_count: 0,

                    follow_origin: None,
                    gather_goal: None,
                    fallback: None,
                }),

                plain_progress: None,
                physical_escape: None,
            },
            MovementFrameUnit {
                unit_id: 2,
                owner_slot: 1,
                unit_type: 1,
                position: Vec2::new(10.2, 10.0),
                radius: 0.5,
                speed: None,
                target: None,
                progress: None,

                plain_progress: None,
                physical_escape: None,
            },
        ],
        pools: BTreeMap::from([(8, pool)]),
    };

    let mut before_deadline = frame.clone();
    before_deadline.units[0]
        .progress
        .as_mut()
        .unwrap()
        .stalled_ticks = 18;
    let before = step_frame(&before_deadline, 0.05).remove(0);
    assert!(
        !before.failed_blocked,
        "the unit has not stalled for one second yet"
    );
    assert_eq!(before.target, Some(target));

    let mover = step_frame(&frame, 0.05).remove(0);

    assert!(mover.failed_blocked);
    assert_eq!(mover.target, None);
    assert_eq!(
        mover.progress.as_ref().map(|progress| progress.activity),
        Some(MovementActivity::Blocked)
    );
}

#[test]
fn exhausted_claims_become_blocked_at_the_one_second_stall_deadline() {
    let point = Vec2::new(10.0, 10.0);
    let pool = MovementPool {
        pool_id: 9,
        point,
        spacing: 1.1,
        primary_count: 1,
        slots: vec![point],
    };
    let frame = MovementFrame {
        bounds: Vec2::new(64.0, 64.0),
        tick_hz: 20,
        units: vec![
            MovementFrameUnit {
                unit_id: 1,
                owner_slot: 0,
                unit_type: 1,
                position: point,
                radius: 0.5,
                speed: Some(3.0),
                target: Some(point),
                progress: Some(MoveProgress {
                    pool_id: 9,
                    slot: point,
                    activity: MovementActivity::Moving,
                    pass_mode: true,
                    stalling_on_group: false,
                    stalled_ticks: 19,
                    best_distance: 0.0,
                    claim_count: 3,

                    follow_origin: None,
                    gather_goal: None,
                    fallback: None,
                }),

                plain_progress: None,
                physical_escape: None,
            },
            MovementFrameUnit {
                unit_id: 2,
                owner_slot: 0,
                unit_type: 1,
                position: point,
                radius: 0.5,
                speed: Some(3.0),
                target: None,
                progress: Some(MoveProgress {
                    pool_id: 9,
                    slot: point,
                    activity: MovementActivity::Resting,
                    pass_mode: false,
                    stalling_on_group: false,
                    stalled_ticks: 0,
                    best_distance: 0.0,
                    claim_count: 0,

                    follow_origin: None,
                    gather_goal: None,
                    fallback: None,
                }),

                plain_progress: None,
                physical_escape: None,
            },
        ],
        pools: BTreeMap::from([(9, pool)]),
    };
    let output = step_frame(&frame, 0.05);
    assert!(output[0].failed_blocked);
    assert_eq!(output[0].target, None);
    assert_eq!(
        output[0]
            .progress
            .as_ref()
            .map(|progress| progress.activity),
        Some(MovementActivity::Blocked)
    );
}

#[test]
fn zero_available_pool_slots_skip_repeated_grid_queries() {
    let pool_id = 9;
    let slots: Vec<_> = (0..5_000_u32)
        .map(|index| Vec2::new(index as f32 + 1.0, 1.0))
        .collect();
    let pool = MovementPool {
        pool_id,
        point: Vec2::ZERO,
        spacing: 1.1,
        primary_count: 5_000,
        slots: slots.clone(),
    };
    let index = MovementPoolIndex::new(&pool, Vec2::new(10_000.0, 10_000.0));
    let assigned: std::collections::HashSet<_> = slots
        .iter()
        .map(|point| (pool_id, point.x.to_bits(), point.y.to_bits()))
        .collect();
    let is_free_calls = std::sync::atomic::AtomicUsize::new(0);
    for _ in 0..5_000 {
        assert_eq!(
            index.nearest_free(1, Vec2::ZERO, pool_id, None, &assigned, 0.5, |_| {
                is_free_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                true
            },),
            None
        );
    }
    assert_eq!(is_free_calls.load(std::sync::atomic::Ordering::Relaxed), 0);
}

#[test]
fn nearest_free_checks_the_final_reach_interval() {
    let pool = MovementPool {
        pool_id: 4,
        point: Vec2::ZERO,
        spacing: 1.0,
        primary_count: 1,
        slots: vec![Vec2::new(300.0, 0.0)],
    };
    let mut index = MovementPoolIndex::new(&pool, Vec2::new(300.0, 300.0));
    index.clearances[0] = super::SlotClearance::empty();
    index.available_by_clearance.insert(super::ClearanceRank {
        clearance: f64::INFINITY,
        slot_index: 0,
    });
    index.available_slot_indices.insert(0);
    assert_eq!(
        index.nearest_free(
            1,
            Vec2::ZERO,
            pool.pool_id,
            None,
            &std::collections::HashSet::new(),
            0.5,
            |_| true,
        ),
        Some(Vec2::new(300.0, 0.0))
    );
}

#[test]
fn unavailable_spares_are_not_hidden_by_an_assigned_free_primary() {
    let pool = MovementPool {
        pool_id: 9,
        point: Vec2::ZERO,
        spacing: 1.1,
        primary_count: 1,
        slots: vec![Vec2::ZERO, Vec2::new(10.0, 0.0), Vec2::new(11.0, 0.0)],
    };
    let mut index = MovementPoolIndex::new(&pool, Vec2::new(20.0, 20.0));
    let units = vec![
        MovementFrameUnit {
            unit_id: 1,
            owner_slot: 0,
            unit_type: 1,
            position: Vec2::new(5.0, 0.0),
            radius: 0.5,
            speed: Some(3.0),
            target: Some(Vec2::new(12.0, 0.0)),
            progress: None,

            plain_progress: None,
            physical_escape: None,
        },
        MovementFrameUnit {
            unit_id: 99,
            owner_slot: 1,
            unit_type: 2,
            position: Vec2::new(10.5, 0.0),
            radius: 1.0,
            speed: None,
            target: None,
            progress: None,

            plain_progress: None,
            physical_escape: None,
        },
    ];
    let points = units
        .iter()
        .map(|unit| crate::grid::GridPoint {
            id: unit.unit_id,
            position: unit.position,
        })
        .collect::<Vec<_>>();
    let grid = crate::grid::UniformGrid::new(&points, 2.0);
    let assigned = std::collections::HashSet::from([(
        pool.pool_id,
        Vec2::ZERO.x.to_bits(),
        Vec2::ZERO.y.to_bits(),
    )]);
    for slot_index in 0..index.points.len() {
        let point = index.points[slot_index].position;
        index.refresh_slot_clearance(
            slot_index,
            &units,
            &points,
            &grid,
            1.0,
            assigned.contains(&(pool.pool_id, point.x.to_bits(), point.y.to_bits())),
        );
    }
    assert!(!index.has_potential_free_slot(1, units[0].position, 0.5, None));
}

#[test]
fn step_frame_rejects_fully_occupied_spares_without_pool_wide_stalled_scans() {
    let count = 64_usize;
    let bounds = Vec2::splat(512.0);
    let center = Vec2::splat(256.0);
    let starts = (0..count)
        .map(|index| {
            Vec2::new(
                10.0 + (index % 8) as f32 * 3.0,
                10.0 + (index / 8) as f32 * 3.0,
            )
        })
        .collect::<Vec<_>>();
    let members = starts
        .iter()
        .enumerate()
        .map(|(index, &position)| SpreadMember {
            id: index as u32 + 1,
            position,
            radius: [0.2, 0.3, 0.5][index % 3],
        })
        .collect::<Vec<_>>();
    let (pool, assignments) = spread_destinations(&members, center, &[], &[], bounds);
    let target_by_id = assignments.iter().copied().collect::<BTreeMap<_, _>>();
    assert_eq!(pool.primary_count, count as u32);
    assert_eq!(pool.slots.len(), count + count.div_ceil(4));
    let assigned_keys = assignments
        .iter()
        .map(|(_, target)| (target.x.to_bits(), target.y.to_bits()))
        .collect::<std::collections::HashSet<_>>();
    let primary_keys = pool.slots[..count]
        .iter()
        .map(|target| (target.x.to_bits(), target.y.to_bits()))
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(assigned_keys, primary_keys);

    let mut units = Vec::new();
    for index in 0..count {
        let unit_id = index as u32 + 1;
        let position = starts[index];
        let radius = [0.2, 0.3, 0.5][index % 3];
        let target = target_by_id[&unit_id];
        let direction = (target - position).normalize();
        let blocker_position = position + direction * (radius + 0.5001);
        assert!((radius + 0.5..radius + 0.502).contains(&position.distance(blocker_position)));
        units.push(MovementFrameUnit {
            unit_id,
            owner_slot: 0,
            unit_type: index as u32,
            position,
            radius,
            speed: Some(0.05),
            target: Some(target),
            progress: Some(MoveProgress {
                pool_id: 9,
                slot: target,
                activity: MovementActivity::Moving,
                pass_mode: true,
                stalling_on_group: false,
                stalled_ticks: 19,
                best_distance: position.distance(target),
                claim_count: 0,

                follow_origin: None,
                gather_goal: None,
                fallback: None,
            }),

            plain_progress: None,
            physical_escape: None,
        });
        units.push(MovementFrameUnit {
            unit_id: 10_000 + unit_id,
            owner_slot: 1,
            unit_type: 4,
            position: blocker_position,
            radius: 0.5,
            speed: None,
            target: None,
            progress: None,

            plain_progress: None,
            physical_escape: None,
        });
    }
    for (index, &point) in pool.slots.iter().skip(count).enumerate() {
        units.push(MovementFrameUnit {
            unit_id: 20_000 + index as u32,
            owner_slot: 1,
            unit_type: 4,
            position: point,
            radius: 0.5,
            speed: Some(3.0),
            target: None,
            progress: Some(MoveProgress {
                pool_id: 77,
                slot: point,
                activity: MovementActivity::Resting,
                pass_mode: false,
                stalling_on_group: false,
                stalled_ticks: 0,
                best_distance: 0.0,
                claim_count: 0,

                follow_origin: None,
                gather_goal: None,
                fallback: None,
            }),

            plain_progress: None,
            physical_escape: None,
        });
    }
    for (left_index, left) in units.iter().enumerate() {
        for right in &units[left_index + 1..] {
            assert!(
                left.position.distance(right.position) + 1.0e-5 >= left.radius + right.radius,
                "invalid overlapping fixture units {} and {}",
                left.unit_id,
                right.unit_id
            );
        }
    }
    assert!(pool.slots[..count].iter().all(|slot| {
        units
            .iter()
            .all(|unit| slot.distance(unit.position) + 1.0e-5 >= 0.5 + unit.radius)
    }));
    let frame = MovementFrame {
        bounds,
        tick_hz: 20,
        units,
        pools: BTreeMap::from([(9, pool)]),
    };
    super::POOL_SLOT_CANDIDATE_CHECKS.with(|checks| checks.set(0));
    let output = step_frame(&frame, 0.05);
    let inspected = super::POOL_SLOT_CANDIDATE_CHECKS.with(std::cell::Cell::get);
    assert!(
        output[..count].iter().all(|row| row.failed_blocked),
        "stalled outputs: {:?}",
        &output[..3]
    );
    assert!(
        inspected < count,
        "{count} stalled units inspected {inspected} pool slot candidates despite every spare being occupied"
    );
    eprintln!("canonical N={count} occupied-spare candidate inspections={inspected}");
}

/// A unit with a pool of one of `pool_id` at `slot`, moving from `position`.
fn pool_of_one_unit(
    unit_id: u32,
    pool_id: u64,
    position: Vec2,
    slot: Vec2,
) -> (MovementFrameUnit, MovementPool) {
    let unit = MovementFrameUnit {
        unit_id,
        owner_slot: 0,
        unit_type: 1,
        position,
        radius: 0.5,
        speed: Some(3.0),
        target: Some(slot),
        progress: Some(MoveProgress {
            pool_id,
            slot,
            activity: MovementActivity::Moving,
            pass_mode: false,
            stalling_on_group: false,
            stalled_ticks: 0,
            best_distance: position.distance(slot),
            claim_count: 0,
            follow_origin: None,
            gather_goal: None,
            fallback: Some(super::MoveFallback {
                failure_recorded: false,
            }),
        }),
        plain_progress: None,
        physical_escape: None,
    };
    let pool = MovementPool {
        pool_id,
        point: slot,
        spacing: 1.1,
        primary_count: 1,
        slots: vec![slot, slot + Vec2::new(1.1, 0.0)],
    };
    (unit, pool)
}

/// The cost guard of the pools of one (the A2b1 review, finding H1). A frame with 40 units, each
/// with its own pool of one and a free path, builds no pool index: the step builds the index of
/// a pool of one only when its member looks for a free slot (a stall at its deadline, or
/// `Blocked`). A count, not a time: the bound is exact on every machine.
#[test]
fn pools_of_one_build_no_index_in_a_free_frame() {
    let mut units = Vec::new();
    let mut pools = BTreeMap::new();
    for index in 0..40_u32 {
        let position = Vec2::new(
            4.0 + 3.0 * (index % 10) as f32,
            4.0 + 3.0 * (index / 10) as f32,
        );
        let (unit, pool) = pool_of_one_unit(
            index + 1,
            u64::from(index) + 1,
            position,
            position + Vec2::new(0.0, 30.0),
        );
        units.push(unit);
        pools.insert(pool.pool_id, pool);
    }
    let frame = MovementFrame {
        bounds: Vec2::splat(64.0),
        tick_hz: 20,
        units,
        pools,
    };
    super::POOL_OF_ONE_INDEX_BUILDS.with(|builds| builds.set(0));
    let outputs = step_frame(&frame, 0.05);
    let builds = super::POOL_OF_ONE_INDEX_BUILDS.with(std::cell::Cell::get);
    assert!(
        outputs.iter().all(|output| output.progress.is_some()),
        "each unit keeps its pool of one"
    );
    assert_eq!(
        builds, 0,
        "the step built {builds} indexes for pools of one"
    );
}

/// Design 9.17 at the site where a physical escape ends with no step (the body of the unit no
/// longer overlaps another body at the start of the frame; the A2b1 review, finding M2): a
/// `Blocked` fallback pool keeps `Blocked`, its slot and its pool, and the escape ends. The pool
/// of an order in the same state clears its progress (the same frame with `fallback: None`).
#[test]
fn a_blocked_fallback_keeps_blocked_when_its_escape_ends_without_a_step() {
    for fallback in [true, false] {
        let position = Vec2::new(10.0, 10.0);
        let (mut unit, pool) = pool_of_one_unit(1, 7, position, position + Vec2::new(1.1, 0.0));
        let progress = unit.progress.as_mut().unwrap();
        progress.activity = MovementActivity::Blocked;
        if !fallback {
            progress.fallback = None;
        }
        unit.target = None;
        unit.physical_escape = Some(super::PhysicalEscape { target: None });
        let expected = unit.progress.clone();
        let frame = MovementFrame {
            bounds: Vec2::splat(64.0),
            tick_hz: 20,
            units: vec![unit],
            pools: BTreeMap::from([(7, pool)]),
        };
        let output = step_frame(&frame, 0.05).remove(0);
        assert_eq!(output.physical_escape, None, "the escape ends: no overlap");
        if fallback {
            assert_eq!(
                output.progress, expected,
                "a Blocked fallback pool keeps Blocked until a new order or a new trip (9.17)"
            );
        } else {
            assert_eq!(
                output.progress, None,
                "the pool of an order clears its progress"
            );
        }
    }
}
