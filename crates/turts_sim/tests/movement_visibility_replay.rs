//! The public replay API accepts frames in any input-row order.
use bevy_math::Vec2;
use std::collections::{BTreeMap, BTreeSet};
use turts_sim::movement::{
    MovementFrame, MovementFrameUnit, MovementVisibility,
    step_frame_preserving_resting_at_tick_with_visibility,
};

struct AllVisible;
impl MovementVisibility for AllVisible {
    fn unit_permitted(&self, _: u8, _: u32) -> bool {
        true
    }
}

#[test]
fn replay_visibility_does_not_depend_on_frame_row_order() {
    let row = |id, owner, pos, speed, target| MovementFrameUnit {
        unit_id: id,
        owner_slot: owner,
        unit_type: 1,
        position: pos,
        radius: 0.5,
        speed,
        target,
        progress: None,
        plain_progress: None,
        physical_escape: None,
    };
    let mut frame = MovementFrame {
        recovery: Default::default(),
        bounds: Vec2::splat(64.0),
        tick_hz: 20,
        units: vec![
            row(
                1,
                0,
                Vec2::new(10.0, 10.0),
                Some(3.0),
                Some(Vec2::new(30.0, 30.0)),
            ),
            row(2, 1, Vec2::new(30.0, 30.0), None, None),
        ],
        pools: BTreeMap::new(),
    };
    let solve = |frame: &MovementFrame| {
        let outputs = step_frame_preserving_resting_at_tick_with_visibility(
            frame,
            0.05,
            &BTreeSet::new(),
            1,
            &AllVisible,
        );
        let mover = outputs.into_iter().find(|unit| unit.unit_id == 1).unwrap();
        (mover.target, mover.position)
    };
    let expected = solve(&frame);
    frame.units.reverse();
    assert_eq!(solve(&frame), expected);
}
