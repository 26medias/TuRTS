mod common;
use common::*;
use glam::vec2;
use turts_interaction::{
    Core, Effect, InputEvent,
    placement::{Input, Recipe},
};
use turts_mod_api::{Behaviour, Button, TargetKind};
use turts_net_protocol::{OrderTarget, UnitId};
use turts_rules::placement::{BuildSource, Circle, Obstacle, ObstacleKind, Point, QueueFill};

fn world() -> TestWorld {
    let mut world = TestWorld::default();
    world.orders.push(order(
        "forge:build",
        Behaviour::Construct,
        TargetKind::ContentAtPoint,
    ));
    world.placement.recipes.insert(
        "forge:tower_recipe".into(),
        Recipe {
            radius: 1.0,
            gap: 1.0,
            node_clearance: 1.0,
            in_build_area: true,
        },
    );
    world
        .placement
        .constructors
        .insert(1, vec!["forge:tower_recipe".into()]);
    world.placement.build_sources.insert(
        0,
        vec![BuildSource {
            footprint: Circle {
                centre: Point { x: 0.0, y: 0.0 },
                radius: 1.0,
            },
            build_area: 20.0,
        }],
    );
    world.placement.queues.insert(
        1,
        QueueFill {
            held: 0,
            capacity: 2,
        },
    );
    world.placement.obstacles.push(Obstacle {
        id: 1,
        footprint: Circle {
            centre: Point { x: 0.0, y: 0.0 },
            radius: 1.0,
        },
        kind: ObstacleKind::Static { gap: 1.0 },
    });
    world
}

fn enter(core: &mut Core, world: &TestWorld) {
    select_world_point(core, world, UnitId(1), 10);
    core.step(
        &[InputEvent::Placement(Input {
            constructor: 1,
            content: "forge:tower_recipe".into(),
        })],
        world,
        &VIEW,
        20,
    );
    assert!(core.placement_state().taking_input);
}

#[test]
fn preview_and_placement_use_the_same_point_and_do_not_change_selection() {
    let world = world();
    let mut core = Core::default();
    enter(&mut core, &world);
    let before = core.selected_units().to_vec();
    let px = core.project_world_to_screen(vec2(8.0, 8.0), &VIEW);
    let effects = core.step(
        &[InputEvent::PointerMove {
            px,
            over_widget: false,
        }],
        &world,
        &VIEW,
        30,
    );
    let preview = effects
        .iter()
        .find_map(|effect| match effect {
            Effect::Placement(output) => output.ghost.as_ref(),
            _ => None,
        })
        .unwrap();
    assert!(preview.valid);
    let point = preview.point;
    let effects = click(&mut core, &world, Button::Left, px, vec![], 40);
    let orders = sent(&effects);
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].units, vec![UnitId(1)]);
    let OrderTarget::ContentAtPoint {
        content,
        point: sent_point,
    } = &orders[0].target
    else {
        panic!("construction lost its target");
    };
    assert_eq!(content, "forge:tower_recipe");
    assert!((sent_point.x - point.x).abs() < 0.001 && (sent_point.y - point.y).abs() < 0.001);
    assert_eq!(core.selected_units(), before);
    assert!(!core.placement_state().taking_input);
}

#[test]
fn blocked_placement_stays_armed_and_right_click_cancels_without_an_order() {
    let world = world();
    let mut core = Core::default();
    enter(&mut core, &world);
    let before = core.selected_units().to_vec();
    let blocked = core.project_world_to_screen(vec2(1.1, 1.1), &VIEW);
    let effects = click(&mut core, &world, Button::Left, blocked, vec![], 40);
    assert!(sent(&effects).is_empty());
    assert!(core.placement_state().taking_input);
    assert!(effects.iter().any(
        |effect| matches!(effect, Effect::Notice { text_key, .. } if text_key == "site_blocked")
    ));
    let effects = click(&mut core, &world, Button::Right, blocked, vec![], 60);
    assert!(sent(&effects).is_empty());
    assert!(!core.placement_state().taking_input);
    assert_eq!(core.selected_units(), before);
}

#[test]
fn losing_focus_or_the_constructor_cancels_placement() {
    for focus_lost in [true, false] {
        let mut world = world();
        let mut core = Core::default();
        enter(&mut core, &world);
        let events = if focus_lost {
            vec![InputEvent::FocusLost]
        } else {
            world.placement.constructors.clear();
            vec![]
        };
        let effects = core.step(&events, &world, &VIEW, 40);
        assert!(sent(&effects).is_empty());
        assert!(!core.placement_state().taking_input);
        assert!(
            effects.iter().any(
                |effect| matches!(effect, Effect::Placement(output) if output.ghost.is_none())
            )
        );
    }
}
