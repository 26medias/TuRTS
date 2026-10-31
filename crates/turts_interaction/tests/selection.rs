mod common;
use common::*;
use glam::vec2;
use turts_interaction::{Core, InputEvent};
use turts_mod_api::{Binding, Button, EngineOverride, Mod};
use turts_net_protocol::UnitId;

#[test]
fn boxed_own_mobile_units_exclude_own_static_units() {
    let world = TestWorld::default();
    let mut core = Core::default();
    let mobile_a = core.project_world_to_screen(world.units[0].position, &VIEW);
    let mobile_b = core.project_world_to_screen(world.units[1].position, &VIEW);
    let static_unit = core.project_world_to_screen(world.units[2].position, &VIEW);
    let low = mobile_a.min(mobile_b).min(static_unit) - vec2(2.0, 2.0);
    let high = mobile_a.max(mobile_b).max(static_unit) + vec2(2.0, 2.0);
    core.step(
        &[
            InputEvent::PointerDown {
                button: Button::Left,
                px: low,
                over_widget: false,
            },
            InputEvent::PointerMove {
                px: high,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: Button::Left,
                px: high,
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        100,
    );
    assert_eq!(
        ids(&core),
        vec![UnitId(1), UnitId(2)],
        "a box containing mobile and static own units must keep the mobiles and drop the statics"
    );
}

#[test]
fn foreign_units_are_inspectable_but_never_box_selected() {
    let world = TestWorld::default();
    let mut core = Core::default();
    let own = core.project_world_to_screen(world.units[0].position, &VIEW);
    let foreign = core.project_world_to_screen(world.units[3].position, &VIEW);
    let low = own.min(foreign) - vec2(3.0, 3.0);
    let high = own.max(foreign) + vec2(3.0, 3.0);
    core.step(
        &[
            InputEvent::PointerDown {
                button: Button::Left,
                px: low,
                over_widget: false,
            },
            InputEvent::PointerMove {
                px: high,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: Button::Left,
                px: high,
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        100,
    );
    assert_eq!(
        ids(&core),
        vec![UnitId(1), UnitId(2)],
        "foreign units must not enter box selection"
    );
    let foreign_px = core.project_world_to_screen(world.units[3].position, &VIEW);
    click(
        &mut core,
        &world,
        turts_mod_api::Button::Left,
        foreign_px,
        vec![],
        500,
    );
    assert_eq!(
        ids(&core),
        vec![UnitId(4)],
        "a direct click must still select a foreign unit for read-only inspection"
    );
    let ground = core.project_world_to_screen(vec2(40.0, 40.0), &VIEW);
    let effects = click(
        &mut core,
        &world,
        turts_mod_api::Button::Right,
        ground,
        vec![],
        600,
    );
    assert!(
        sent(&effects).is_empty(),
        "an inspected foreign unit must not become a command subject"
    );
}

#[test]
fn selection_has_no_256_unit_cap() {
    let world = TestWorld {
        units: (1..=301)
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
    let last = core.project_world_to_screen(world.units[300].position, &VIEW);
    let low = first.min(last) - vec2(8.0, 8.0);
    let high = first.max(last) + vec2(8.0, 8.0);
    core.step(
        &[
            InputEvent::PointerDown {
                button: Button::Left,
                px: low,
                over_widget: false,
            },
            InputEvent::PointerMove {
                px: high,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: Button::Left,
                px: high,
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        100,
    );
    assert_eq!(
        core.selected_units().len(),
        301,
        "all 301 own units in the box must be retained; the former 256 cap must not return"
    );
}

#[test]
fn box_selection_uses_projected_centres_not_disc_overlap() {
    let mut world = TestWorld {
        units: vec![
            unit(
                1,
                "forge:drone",
                OWN,
                vec2(0.0, 0.0),
                &[turts_mod_api::Capability::Movement],
            ),
            unit(
                2,
                "forge:drone",
                OWN,
                vec2(1.0, 0.0),
                &[turts_mod_api::Capability::Movement],
            ),
        ],
        ..Default::default()
    };
    world.units[0].footprint_radius = 100.0;
    world.units[1].footprint_radius = 100.0;
    let mut core = Core::default();
    let centre_inside = core.project_world_to_screen(world.units[0].position, &VIEW);
    let centre_outside = core.project_world_to_screen(world.units[1].position, &VIEW);
    let edge = (centre_inside + centre_outside) * 0.5;
    let low = centre_inside.min(edge) - vec2(1.0, 1.0);
    let high = centre_inside.max(edge) + vec2(1.0, 1.0);
    core.step(
        &[
            InputEvent::PointerDown {
                button: Button::Left,
                px: low,
                over_widget: false,
            },
            InputEvent::PointerMove {
                px: high,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: Button::Left,
                px: high,
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        100,
    );
    assert_eq!(
        ids(&core),
        vec![UnitId(1)],
        "only the unit whose projected footprint centre lies in the rectangle may be selected"
    );
}

#[test]
fn muster_mobile_box_keeps_all_500_units_and_drops_static_post() {
    let world = TestWorld {
        units: (1..=40)
            .map(|id| {
                unit(
                    id,
                    "muster:trooper",
                    OWN,
                    vec2(id as f32 * 0.001, 0.0),
                    &[turts_mod_api::Capability::Movement],
                )
            })
            .chain((41..=500).map(|id| {
                unit(
                    id,
                    "muster:beacon",
                    OWN,
                    vec2(id as f32 * 0.001, 0.0),
                    &[turts_mod_api::Capability::Movement],
                )
            }))
            .chain([unit(501, "muster:post", OWN, vec2(0.0, 0.0), &[])])
            .collect(),
        selection_priority: vec![
            "muster:trooper".into(),
            "muster:worker".into(),
            "muster:beacon".into(),
            "muster:dummy".into(),
            "muster:depot".into(),
            "muster:post".into(),
        ],
        ..Default::default()
    };
    let mut core = Core::default();
    let a = core.project_world_to_screen(world.units[0].position, &VIEW);
    let b = core.project_world_to_screen(world.units[499].position, &VIEW);
    let low = a.min(b) - vec2(4.0, 4.0);
    let high = a.max(b) + vec2(4.0, 4.0);
    core.step(
        &[
            InputEvent::PointerDown {
                button: Button::Left,
                px: low,
                over_widget: false,
            },
            InputEvent::PointerMove {
                px: high,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: Button::Left,
                px: high,
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        100,
    );
    assert_eq!(
        core.selected_units().len(),
        500,
        "the authored Muster mobile tier must keep all 500 mobile units and suppress the static post"
    );
}

#[test]
fn static_first_selection_priority_selects_static_class() {
    let world = TestWorld {
        units: vec![
            unit(
                1,
                "forge:drone",
                OWN,
                vec2(0.0, 0.0),
                &[turts_mod_api::Capability::Movement],
            ),
            unit(2, "forge:drill", OWN, vec2(1.0, 0.0), &[]),
        ],
        selection_priority: vec!["forge:drill".into(), "forge:drone".into()],
        ..Default::default()
    };
    let mut core = Core::default();
    let a = core.project_world_to_screen(world.units[0].position, &VIEW);
    let b = core.project_world_to_screen(world.units[1].position, &VIEW);
    let low = a.min(b) - vec2(3.0, 3.0);
    let high = a.max(b) + vec2(3.0, 3.0);
    core.step(
        &[
            InputEvent::PointerDown {
                button: Button::Left,
                px: low,
                over_widget: false,
            },
            InputEvent::PointerMove {
                px: high,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: Button::Left,
                px: high,
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        100,
    );
    assert_eq!(
        ids(&core),
        vec![UnitId(2)],
        "the earliest represented authored static type must override the mobile default"
    );
}

#[test]
fn partial_mobile_priority_keeps_unlisted_mobile_types_in_same_class() {
    let world = TestWorld {
        units: vec![
            unit(
                1,
                "forge:drone",
                OWN,
                vec2(0.0, 0.0),
                &[turts_mod_api::Capability::Movement],
            ),
            unit(
                2,
                "forge:scout",
                OWN,
                vec2(1.0, 0.0),
                &[turts_mod_api::Capability::Movement],
            ),
            unit(3, "forge:drill", OWN, vec2(2.0, 0.0), &[]),
        ],
        selection_priority: vec!["forge:drone".into()],
        ..Default::default()
    };
    let mut core = Core::default();
    let a = core.project_world_to_screen(world.units[0].position, &VIEW);
    let c = core.project_world_to_screen(world.units[2].position, &VIEW);
    let low = a.min(c) - vec2(3.0, 3.0);
    let high = a.max(c) + vec2(3.0, 3.0);
    core.step(
        &[
            InputEvent::PointerDown {
                button: Button::Left,
                px: low,
                over_widget: false,
            },
            InputEvent::PointerMove {
                px: high,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: Button::Left,
                px: high,
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        100,
    );
    assert_eq!(
        ids(&core),
        vec![UnitId(1), UnitId(2)],
        "priority ranks mobile class membership and must retain unlisted mobile content types"
    );
}

#[test]
fn drag_threshold_latches_after_pointer_returns_near_start() {
    let world = TestWorld::default();
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    let start = core.project_world_to_screen(world.units[3].position, &VIEW);
    core.step(
        &[
            InputEvent::PointerDown {
                button: turts_mod_api::Button::Left,
                px: start,
                over_widget: false,
            },
            InputEvent::PointerMove {
                px: start + vec2(10.0, 0.0),
                over_widget: false,
            },
            InputEvent::PointerMove {
                px: start,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: turts_mod_api::Button::Left,
                px: start,
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        20,
    );
    assert!(
        core.selected_units().is_empty(),
        "once the drag threshold is crossed, returning to the down point must not turn the gesture into a foreign click"
    );
}

#[test]
fn type_modifier_rebind_removes_physical_ctrl_type_selection() {
    let mut world = TestWorld::default();
    world.input.engine_overrides.push(EngineOverride {
        name: "type_modifier".into(),
        bindings: vec![Binding::Key {
            code: "KeyB".into(),
            mods: vec![],
        }],
    });
    let mut core = Core::default();
    let point = core.project_world_to_screen(world.units[0].position, &VIEW);
    core.step(
        &[
            InputEvent::KeyDown {
                code: "ControlLeft".into(),
                mods: vec![Mod::Ctrl],
            },
            InputEvent::PointerDown {
                button: Button::Left,
                px: point,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: Button::Left,
                px: point,
                over_widget: false,
            },
            InputEvent::KeyUp {
                code: "ControlLeft".into(),
            },
        ],
        &world,
        &VIEW,
        10,
    );
    assert_eq!(
        core.selected_units(),
        &[UnitId(1)],
        "physical Ctrl loses type-selection behavior when type_modifier is rebound"
    );

    let point = core.project_world_to_screen(world.units[0].position, &VIEW);
    core.step(
        &[
            InputEvent::KeyDown {
                code: "KeyB".into(),
                mods: vec![],
            },
            InputEvent::PointerDown {
                button: Button::Left,
                px: point,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: Button::Left,
                px: point,
                over_widget: false,
            },
            InputEvent::KeyUp {
                code: "KeyB".into(),
            },
        ],
        &world,
        &VIEW,
        20,
    );
    assert_eq!(
        core.selected_units(),
        &[UnitId(1), UnitId(2)],
        "the rebound type modifier selects the visible unit type"
    );
}
