use super::*;
use crate::grid::{GridPoint, UniformGrid};
use crate::movement::{MovementActivity, PlainMoveProgress};
struct Legacy(bool);
impl Legacy {
    fn set(value: bool) -> Self {
        Self(LEGACY_SEPARATION_QUERY.with(|slot| slot.replace(value)))
    }
}
impl Drop for Legacy {
    fn drop(&mut self) {
        LEGACY_SEPARATION_QUERY.with(|slot| slot.set(self.0));
    }
}
fn step_mode(frame: &MovementFrame, dt: f32, legacy: bool) -> Vec<MovementFrameOutput> {
    let _guard = Legacy::set(legacy);
    step_frame(frame, dt)
}
fn actor(id: u32, point: Vec2, radius: f32, speed: f32, target: Vec2) -> MovementFrameUnit {
    MovementFrameUnit {
        unit_id: id,
        owner_slot: 0,
        unit_type: 1,
        position: point,
        radius,
        speed: Some(speed),
        target: Some(target),
        progress: None,
        plain_progress: None,
        physical_escape: None,
    }
}
fn frame(units: Vec<MovementFrameUnit>) -> MovementFrame {
    MovementFrame {
        recovery: Default::default(),
        bounds: Vec2::splat(128.0),
        tick_hz: 20,
        units,
        pools: BTreeMap::new(),
    }
}
fn compare_trajectory(mut current: MovementFrame, steps: usize) {
    for tick in 0..steps {
        let expected = step_mode(&current, 0.05, true);
        let actual = step_mode(&current, 0.05, false);
        assert_eq!(
            actual, expected,
            "full movement state differs at tick {tick}"
        );
        for output in actual {
            let unit = current
                .units
                .iter_mut()
                .find(|unit| unit.unit_id == output.unit_id)
                .unwrap();
            unit.position = output.position;
            unit.target = output.target;
            unit.progress = output.progress;
            unit.plain_progress = output.plain_progress;
            unit.physical_escape = output.physical_escape;
        }
    }
}
#[test]
fn full_old_and_new_trajectories_match_dense_sparse_mixed_speed_and_radius_frames() {
    for spacing in [0.9, 2.5, 12.0] {
        for inverted in [false, true] {
            let mut units = (0..16)
                .map(|index| {
                    actor(
                        if inverted { 100 - index } else { index },
                        Vec2::new(
                            8.0 + (index % 4) as f32 * spacing,
                            8.0 + (index / 4) as f32 * spacing,
                        ),
                        [0.2, 0.4, 0.5, 1.1][index as usize % 4],
                        [0.25, 3.0, 20.0, 100.0][index as usize % 4],
                        Vec2::new(90.0, 60.0),
                    )
                })
                .collect::<Vec<_>>();
            units.push(MovementFrameUnit {
                speed: None,
                target: None,
                ..actor(999, Vec2::new(40.0, 30.0), 2.0, 0.0, Vec2::ZERO)
            });
            if inverted {
                units.reverse();
            }
            compare_trajectory(frame(units), 40);
        }
    }
}
#[test]
fn group_plain_ties_and_projection_keep_exact_full_outputs_with_legacy_query() {
    for inverted in [false, true] {
        for plain in [false, true] {
            let goal = Vec2::new(40.0, 25.0);
            let mut units = (0..8)
                .map(|index| {
                    let point = Vec2::new(
                        10.0 + (index % 4) as f32 * 0.9,
                        10.0 + (index / 4) as f32 * 0.9,
                    );
                    let mut unit = actor(
                        if inverted { 100 - index } else { index },
                        point,
                        0.5,
                        3.0,
                        goal,
                    );
                    if plain {
                        unit.plain_progress = Some(PlainMoveProgress {
                            arrival_yield: false,
                            original_point: goal,
                            slot: goal,
                            pool_slots: vec![goal, goal + Vec2::X * 1.1, goal + Vec2::Y * 1.1],
                            gather_goal: None,
                            activity: MovementActivity::Moving,
                            stalled_ticks: 0,
                            best_distance: point.distance(goal),
                            claim_count: 0,
                            failure_recorded: false,
                        });
                    } else {
                        unit.progress = Some(super::super::MoveProgress {
                            pool_id: 7,
                            slot: goal,
                            activity: MovementActivity::Moving,
                            pass_mode: false,
                            stalling_on_group: false,
                            stalled_ticks: 0,
                            best_distance: point.distance(goal),
                            claim_count: 0,
                            follow_origin: None,
                        });
                    }
                    unit
                })
                .collect::<Vec<_>>();
            // Exact coincident starts and initial overlap exercise contested and
            // projection recovery. This is a defensive kernel fixture, not authored placement.
            units[1].position = units[0].position;
            if inverted {
                units.reverse();
            }
            let mut input = frame(units);
            if !plain {
                input.pools.insert(
                    7,
                    super::super::MovementPool {
                        pool_id: 7,
                        point: goal,
                        spacing: 1.1,
                        primary_count: 8,
                        slots: (0..24)
                            .map(|i| goal + Vec2::new((i % 6) as f32 * 1.1, (i / 6) as f32 * 1.1))
                            .collect(),
                    },
                );
            }
            compare_trajectory(input, 60);
        }
    }
}
#[test]
fn tightened_query_retains_every_f32_boundary_contributor_in_canonical_order() {
    let _guard = Legacy::set(false);
    for radius in [0.000001_f32, 0.05, 0.4, 0.5, 1.1, 50.0] {
        for other_radius in [0.000001_f32, 0.05, 0.4, 0.5, 1.1, 50.0] {
            let max_radius = radius.max(other_radius);
            let limit = radius + other_radius + 0.05;
            let mut distances = vec![
                0.0,
                1e-8,
                limit.next_down().next_down(),
                limit.next_down(),
                limit,
                limit.next_up(),
            ];
            distances.extend([
                f32::from_bits(limit.to_bits().saturating_sub(4)),
                f32::from_bits(limit.to_bits() + 4),
            ]);
            for distance in distances {
                for direction in [
                    Vec2::X,
                    Vec2::Y,
                    -Vec2::X,
                    Vec2::ONE.normalize(),
                    Vec2::new(-3.0, 4.0).normalize(),
                ] {
                    for origin in [Vec2::ZERO, Vec2::splat(100.0), Vec2::splat(1e6)] {
                        let points = vec![
                            GridPoint {
                                id: 99,
                                position: origin,
                            },
                            GridPoint {
                                id: 1,
                                position: origin + direction * distance,
                            },
                        ];
                        let grid = UniformGrid::new(&points, (2.0 * max_radius).max(0.05));
                        let old = radius + max_radius + 0.05 + 5.0;
                        let tight = separation_reach(radius, max_radius, old);
                        assert!(tight <= old);
                        let contributes =
                            |&index: &usize| points[index].position.distance(origin) < limit;
                        let expected = grid
                            .nearby(&points, origin, old)
                            .into_iter()
                            .filter(contributes)
                            .collect::<Vec<_>>();
                        let actual = grid
                            .nearby(&points, origin, tight)
                            .into_iter()
                            .filter(contributes)
                            .collect::<Vec<_>>();
                        assert_eq!(
                            actual, expected,
                            "f32 boundary lost a contributor: r={radius},other={other_radius},d={distance},origin={origin:?},direction={direction:?}"
                        );
                    }
                }
            }
        }
    }
}
