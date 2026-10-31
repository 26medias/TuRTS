mod common;
use common::*;
use glam::vec2;
use turts_interaction::{Core, InputEvent};
use turts_mod_api::{Binding, EngineOverride, Mod};
use turts_net_protocol::{OrderTarget, QueueMode, UnitId};

fn submit_move(
    core: &mut Core,
    world: &TestWorld,
    queue_modifier: bool,
    now_ms: u64,
) -> Vec<turts_interaction::Effect> {
    let target = core.project_world_to_screen(vec2(20.0, 8.0), &VIEW);
    let mut events = Vec::new();
    if queue_modifier {
        events.push(InputEvent::KeyDown {
            code: "ShiftLeft".into(),
            mods: vec![Mod::Shift],
        });
    }
    events.push(InputEvent::UiAction {
        name: "forge:order_move".into(),
    });
    events.push(InputEvent::PointerDown {
        button: turts_mod_api::Button::Left,
        px: target,
        over_widget: false,
    });
    events.push(InputEvent::PointerUp {
        button: turts_mod_api::Button::Left,
        px: target,
        over_widget: false,
    });
    core.step(&events, world, &VIEW, now_ms)
}

#[test]
fn explicit_order_without_shift_replaces_current_and_queue() {
    let world = TestWorld::default();
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    let effects = submit_move(&mut core, &world, false, 20);
    let envelopes = sent(&effects);
    assert_eq!(
        envelopes[0].queue,
        QueueMode::Replace,
        "an unmodified explicit order replaces each unit's current order and queue"
    );
    assert_eq!(
        envelopes[0].group, 0,
        "one plain point Move has no group and retains the server plain-move path"
    );
}

#[test]
fn shift_explicit_order_appends_to_each_unit_queue() {
    let world = TestWorld::default();
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    let effects = submit_move(&mut core, &world, true, 20);
    let envelopes = sent(&effects);
    assert_eq!(
        envelopes[0].queue,
        QueueMode::Append,
        "Shift must compose the explicit order in append mode"
    );
}

#[test]
fn split_4097_subjects_into_4096_and_one_with_one_shared_group() {
    let world = TestWorld {
        units: (1..=4097)
            .map(|id| {
                unit(
                    id,
                    "forge:drone",
                    OWN,
                    vec2(id as f32 * 0.0001, 0.0),
                    &[turts_mod_api::Capability::Movement],
                )
            })
            .collect(),
        ..Default::default()
    };
    let mut core = Core::default();
    let first = core.project_world_to_screen(world.units[0].position, &VIEW);
    core.step(
        &[
            InputEvent::KeyDown {
                code: "ControlLeft".into(),
                mods: vec![Mod::Ctrl],
            },
            InputEvent::PointerDown {
                button: turts_mod_api::Button::Left,
                px: first,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: turts_mod_api::Button::Left,
                px: first,
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        10,
    );
    assert_eq!(
        core.selected_units().len(),
        4097,
        "selection must not cap subjects before composition"
    );
    let effects = submit_move(&mut core, &world, false, 20);
    let envelopes = sent(&effects);
    assert_eq!(
        envelopes
            .iter()
            .map(|envelope| envelope.units.len())
            .collect::<Vec<_>>(),
        vec![4096, 1],
        "the core must split at the protocol cap of 4,096 units"
    );
    let OrderTarget::Point(target) = &envelopes[0].target else {
        panic!("an explicit point order must retain a point target");
    };
    assert!(
        (target.x - 20.0).abs() <= 0.0001 && (target.y - 8.0).abs() <= 0.0001,
        "inverse projection must recover the submitted world point within float precision"
    );
    assert_eq!(envelopes[0].queue, QueueMode::Replace);
    assert_eq!(
        envelopes[0].group, envelopes[1].group,
        "all pieces of one composed order must share the same group id"
    );
    assert!(envelopes[0].group > 0);
    let mut submitted = envelopes
        .iter()
        .flat_map(|envelope| envelope.units.iter().copied())
        .collect::<Vec<_>>();
    submitted.sort_by_key(|id| id.0);
    assert_eq!(submitted.first(), Some(&UnitId(1)));
    assert_eq!(submitted.last(), Some(&UnitId(4097)));
}

#[test]
fn explicit_move_under_foreign_unit_is_a_point_force_move() {
    let world = TestWorld::default();
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    let destination = world.units[3].position;
    let px = core.project_world_to_screen(destination, &VIEW);
    let effects = core.step(
        &[
            InputEvent::UiAction {
                name: "forge:order_move".into(),
            },
            InputEvent::PointerDown {
                button: turts_mod_api::Button::Left,
                px,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: turts_mod_api::Button::Left,
                px,
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        20,
    );
    let envelopes = sent(&effects);
    let envelope = envelopes
        .first()
        .expect("an explicit Move must compose an order at a foreign unit's ground point");
    assert_eq!(envelope.order, "forge:order_move");
    let OrderTarget::Point(point) = &envelope.target else {
        panic!("explicit point-capable Move is a force move, not an attack or unit-follow target");
    };
    assert!((point.x - destination.x).abs() <= 0.0001 && (point.y - destination.y).abs() <= 0.0001);
}

#[test]
fn one_envelope_with_many_subjects_uses_a_nonzero_group() {
    let world = TestWorld {
        units: (1..=40)
            .map(|id| {
                unit(
                    id,
                    "forge:drone",
                    OWN,
                    vec2(id as f32 * 0.01, 0.0),
                    &[turts_mod_api::Capability::Movement],
                )
            })
            .collect(),
        ..Default::default()
    };
    let mut core = Core::default();
    let first = core.project_world_to_screen(world.units[0].position, &VIEW);
    core.step(
        &[
            InputEvent::KeyDown {
                code: "ControlLeft".into(),
                mods: vec![Mod::Ctrl],
            },
            InputEvent::PointerDown {
                button: turts_mod_api::Button::Left,
                px: first,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: turts_mod_api::Button::Left,
                px: first,
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        10,
    );
    assert_eq!(core.selected_units().len(), 40);
    let effects = core.step(
        &[InputEvent::UiAction {
            name: "forge:order_stop".into(),
        }],
        &world,
        &VIEW,
        20,
    );
    let envelopes = sent(&effects);
    assert_eq!(envelopes.len(), 1);
    assert_eq!(envelopes[0].units.len(), 40);
    assert!(
        envelopes[0].group > 0,
        "a multi-unit gesture needs a group even when the order fits one envelope"
    );
}

#[test]
fn empty_subject_filter_is_unrestricted_and_unit_target_content_is_independent() {
    let mut world = TestWorld::default();
    world.orders[0].target = turts_mod_api::TargetKind::Unit;
    world.orders[0].units = vec!["forge:drone".into()];
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    let target = core.project_world_to_screen(world.units[3].position, &VIEW);
    let effects = core.step(
        &[
            InputEvent::UiAction {
                name: "forge:order_move".into(),
            },
            InputEvent::PointerDown {
                button: turts_mod_api::Button::Left,
                px: target,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: turts_mod_api::Button::Left,
                px: target,
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        20,
    );
    let envelopes = sent(&effects);
    assert_eq!(
        envelopes.len(),
        1,
        "the authored subject filter should allow the selected drone"
    );
    assert_eq!(envelopes[0].units, vec![UnitId(1)]);
    assert_eq!(
        envelopes[0].target,
        OrderTarget::Unit(UnitId(4)),
        "target content is not filtered by the order's subject-type list"
    );

    world.orders[0].units.clear();
    let effects = core.step(
        &[InputEvent::UiAction {
            name: "forge:order_stop".into(),
        }],
        &world,
        &VIEW,
        30,
    );
    assert_eq!(
        sent(&effects)[0].units,
        vec![UnitId(1)],
        "an empty subject list means unrestricted"
    );
}

#[test]
fn add_and_queue_modifier_actions_are_independently_rebindable() {
    let mut world = TestWorld::default();
    world.input.engine_overrides = vec![
        EngineOverride {
            name: "add_modifier".into(),
            bindings: vec![Binding::Key {
                code: "KeyA".into(),
                mods: vec![],
            }],
        },
        EngineOverride {
            name: "queue_modifier".into(),
            bindings: vec![Binding::Key {
                code: "KeyB".into(),
                mods: vec![],
            }],
        },
    ];
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    let second = core.project_world_to_screen(world.units[1].position, &VIEW);
    core.step(
        &[
            InputEvent::KeyDown {
                code: "KeyA".into(),
                mods: vec![],
            },
            InputEvent::PointerDown {
                button: turts_mod_api::Button::Left,
                px: second,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: turts_mod_api::Button::Left,
                px: second,
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        20,
    );
    assert_eq!(
        core.selected_units(),
        &[UnitId(1), UnitId(2)],
        "rebound add modifier appends a selected unit without physical Shift"
    );
    let target = core.project_world_to_screen(vec2(20.0, 8.0), &VIEW);
    let appended = core.step(
        &[
            InputEvent::KeyDown {
                code: "KeyB".into(),
                mods: vec![],
            },
            InputEvent::UiAction {
                name: "forge:order_move".into(),
            },
            InputEvent::PointerDown {
                button: turts_mod_api::Button::Left,
                px: target,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: turts_mod_api::Button::Left,
                px: target,
                over_widget: false,
            },
            InputEvent::KeyUp {
                code: "KeyB".into(),
            },
        ],
        &world,
        &VIEW,
        30,
    );
    assert_eq!(
        sent(&appended)[0].queue,
        QueueMode::Append,
        "rebound queue modifier appends the explicit order"
    );
    let old_shift = core.step(
        &[
            InputEvent::KeyDown {
                code: "ShiftLeft".into(),
                mods: vec![Mod::Shift],
            },
            InputEvent::UiAction {
                name: "forge:order_move".into(),
            },
            InputEvent::PointerDown {
                button: turts_mod_api::Button::Left,
                px: target,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: turts_mod_api::Button::Left,
                px: target,
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        40,
    );
    assert_eq!(
        sent(&old_shift)[0].queue,
        QueueMode::Replace,
        "the old physical Shift binding stops queuing after queue_modifier is rebound"
    );
}
