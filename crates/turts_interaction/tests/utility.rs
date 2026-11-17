//! M6 utility view and route mode: each transition of the pure state machine.
mod common;
use common::*;
use glam::{Vec2, vec2};
use std::collections::BTreeMap;
use turts_interaction::{
    Core, Effect, InputEvent,
    utility::{
        ACTION_DEEPER, ACTION_SHALLOWER, ACTION_VIEW, Endpoint, Facts, HoverTarget, Input, Output,
        SNAP_RADIUS, UtilityFact, reason,
    },
};
use turts_mod_api::{Behaviour, Button, TargetKind, UtilityEndpoint};
use turts_net_protocol::{
    OrderTarget, Pos, QueueMode, UnitId, UtilityEndRecord, UtilityNetworkRecord, UtilityNodeRecord,
    UtilitySegmentRecord, UtilitySegmentState,
};

const LINE: &str = "forge:line";
const ORDER: &str = "forge:route";

/// A segment from node 10 at (4,2) to node 11 at (8,2), and a port at the drill (0,6)+(0,1).
fn world() -> TestWorld {
    let mut world = TestWorld::default();
    world.orders.push(order(
        ORDER,
        Behaviour::UtilityRoute,
        TargetKind::UtilityRoute,
    ));
    world.orders.last_mut().unwrap().units = vec!["forge:drone".into()];
    world.utility = Facts {
        utilities: BTreeMap::from([(
            LINE.to_owned(),
            UtilityFact {
                name: "Power line".into(),
                depth_default: 2,
                depth_min: 1,
                depth_max: 4,
                reach: 30.0,
                max_segment_length: 16.0,
                cost: vec![("Loam".into(), 1)],
                ports: BTreeMap::from([("forge:drill".to_owned(), vec2(0.0, 1.0))]),
            },
        )]),
        view: view(),
    };
    world
}

fn view() -> turts_net_protocol::UtilityView {
    let node = |id, x, y| UtilityNodeRecord {
        id,
        utility: LINE.into(),
        pos: Pos { x, y },
    };
    turts_net_protocol::UtilityView {
        nodes: vec![node(10, 4.0, 2.0), node(11, 8.0, 2.0)],
        segments: vec![UtilitySegmentRecord {
            id: 1,
            utility: LINE.into(),
            a: UtilityEndRecord::Node(10),
            b: UtilityEndRecord::Node(11),
            elevation: -2,
            depth: 2,
            state: UtilitySegmentState::Conducting,
        }],
        networks: vec![UtilityNetworkRecord {
            utility: LINE.into(),
            units: vec![UnitId(3)],
            nodes: vec![10, 11],
            provided: 10,
            consumed: 6,
            short: false,
        }],
    }
}

fn press(core: &mut Core, world: &TestWorld, code: &str) -> Vec<Effect> {
    core.step(
        &[InputEvent::KeyDown {
            code: code.into(),
            mods: vec![],
        }],
        world,
        &VIEW,
        50,
    )
}

fn at(core: &Core, point: Vec2) -> Vec2 {
    core.project_world_to_screen(point, &VIEW)
}

fn click_at(core: &mut Core, world: &TestWorld, point: Vec2) -> Vec<Effect> {
    let px = at(core, point);
    click(core, world, Button::Left, px, vec![], 100)
}

fn output(effects: &[Effect]) -> Option<&Output> {
    effects.iter().rev().find_map(|effect| match effect {
        Effect::Utility(output) => Some(output),
        _ => None,
    })
}

fn enter(core: &mut Core, world: &TestWorld) -> Vec<Effect> {
    select_world_point(core, world, UnitId(1), 10);
    core.step(
        &[InputEvent::UtilityRoute(Input {
            order: ORDER.into(),
            utility: LINE.into(),
        })],
        world,
        &VIEW,
        20,
    )
}

fn route_target(effect: &turts_net_protocol::OrderEnvelope) -> &turts_mod_api::UtilityRouteTarget {
    match &effect.target {
        OrderTarget::UtilityRoute(target) => target,
        other => panic!("not a utility route: {other:?}"),
    }
}

fn free(endpoint: &UtilityEndpoint) -> Vec2 {
    match endpoint {
        UtilityEndpoint::Free(point) => vec2(point.x, point.y),
        other => panic!("not a free point: {other:?}"),
    }
}

#[test]
fn the_view_key_turns_the_view_on_and_off() {
    let world = world();
    let mut core = Core::default();
    let effects = press(&mut core, &world, "KeyU");
    assert!(output(&effects).is_some_and(|output| output.view_on));
    assert!(core.utility_state().view_on());
    let effects = press(&mut core, &world, "KeyU");
    assert!(output(&effects).is_some_and(|output| !output.view_on));
    assert!(!core.utility_state().view_on());
}

#[test]
fn a_catalog_without_utility_types_has_no_view() {
    let world = TestWorld::default();
    let mut core = Core::default();
    press(&mut core, &world, "KeyU");
    assert!(!core.utility_state().view_on());
    let effects = core.step(
        &[InputEvent::UiAction {
            name: ACTION_VIEW.into(),
        }],
        &world,
        &VIEW,
        60,
    );
    assert!(output(&effects).is_none());
}

#[test]
fn a_hud_action_turns_the_view_on_like_the_key() {
    let world = world();
    let mut core = Core::default();
    core.step(
        &[InputEvent::UiAction {
            name: ACTION_VIEW.into(),
        }],
        &world,
        &VIEW,
        60,
    );
    assert!(core.utility_state().view_on());
}

#[test]
fn the_engine_defaults_bind_the_three_view_actions() {
    let bindings = turts_interaction::input_map::engine_defaults();
    for (name, code) in [
        (ACTION_VIEW, "KeyU"),
        (ACTION_DEEPER, "BracketRight"),
        (ACTION_SHALLOWER, "BracketLeft"),
    ] {
        let action = bindings
            .iter()
            .find(|action| action.name == name)
            .unwrap_or_else(|| panic!("{name} is an engine action"));
        assert!(
            action.default.iter().any(|binding| matches!(
                binding,
                turts_mod_api::Binding::Key { code: key, mods } if key == code && mods.is_empty()
            )),
            "{name} defaults to {code}"
        );
    }
}

#[test]
fn choosing_a_utility_enters_route_mode_and_turns_the_view_on() {
    let world = world();
    let mut core = Core::default();
    let effects = enter(&mut core, &world);
    let route = core.utility_state().route().expect("route mode");
    assert_eq!((route.order.as_str(), route.depth), (ORDER, 2));
    assert!(core.utility_state().view_on());
    assert_eq!(core.pending_order_id(), Some(ORDER));
    let shown = output(&effects).expect("effect");
    assert!(shown.view_on);
    let preview = shown.route.as_ref().expect("preview");
    assert_eq!((preview.name.as_str(), preview.depth), ("Power line", 2));
    assert_eq!(preview.cost, vec![("Loam".to_owned(), 1)]);
}

#[test]
fn route_mode_needs_a_selected_eligible_builder() {
    let world = world();
    let mut core = Core::default();
    // Unit 5 is an own gatherer; the order lists only the drone.
    select_world_point(&mut core, &world, UnitId(5), 10);
    let effects = core.step(
        &[InputEvent::UtilityRoute(Input {
            order: ORDER.into(),
            utility: LINE.into(),
        })],
        &world,
        &VIEW,
        20,
    );
    assert!(core.utility_state().route().is_none());
    assert!(!core.utility_state().view_on());
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::Notice { .. }))
    );
}

#[test]
fn two_clicks_send_one_route_and_the_end_becomes_the_next_start() {
    let world = world();
    let mut core = Core::default();
    enter(&mut core, &world);
    let first = click_at(&mut core, &world, vec2(10.0, 4.0));
    assert!(
        sent(&first).is_empty(),
        "the first click only sets the start"
    );
    let second = click_at(&mut core, &world, vec2(10.0, 12.0));
    let orders = sent(&second);
    assert_eq!(orders.len(), 1);
    let envelope = orders[0];
    assert_eq!(envelope.order, ORDER);
    assert_eq!(envelope.units, vec![UnitId(1)]);
    assert_eq!(envelope.queue, QueueMode::Replace);
    let target = route_target(envelope);
    assert_eq!((target.utility.as_str(), target.depth), (LINE, 2));
    assert!(free(&target.from).distance(vec2(10.0, 4.0)) < 0.05);
    assert!(free(&target.to).distance(vec2(10.0, 12.0)) < 0.05);
    // The end is the next start: the third click goes from (-6, 4).
    let third = click_at(&mut core, &world, vec2(14.0, 12.0));
    let target = route_target(sent(&third)[0]).clone();
    assert!(free(&target.from).distance(vec2(10.0, 12.0)) < 0.05);
    assert!(free(&target.to).distance(vec2(14.0, 12.0)) < 0.05);
}

#[test]
fn the_start_snaps_to_the_junction_that_the_last_end_became() {
    let mut world = world();
    let mut core = Core::default();
    enter(&mut core, &world);
    click_at(&mut core, &world, vec2(10.0, 4.0));
    click_at(&mut core, &world, vec2(10.0, 12.0));
    // The server made a junction there; the next view holds it.
    world.utility.view.nodes.push(UtilityNodeRecord {
        id: 12,
        utility: LINE.into(),
        pos: Pos { x: 10.0, y: 12.0 },
    });
    let third = click_at(&mut core, &world, vec2(14.0, 12.0));
    let target = route_target(sent(&third)[0]).clone();
    assert_eq!(target.from, UtilityEndpoint::Node(12));
}

#[test]
fn a_click_snaps_to_a_port_then_a_junction_then_a_segment() {
    let world = world();
    // A port wins over a node and a segment within the radius.
    let port = vec2(0.0, 7.0);
    assert!(matches!(
        turts_interaction::utility::resolve(port + vec2(0.3, 0.0), LINE, &world),
        Endpoint::Port { unit, point } if unit == UnitId(3) && point.distance(port) < 0.001
    ));
    // A junction.
    assert!(matches!(
        turts_interaction::utility::resolve(vec2(4.2, 2.2), LINE, &world),
        Endpoint::Node { id: 10, .. }
    ));
    // The middle of the segment: a split at the projected point.
    match turts_interaction::utility::resolve(vec2(6.0, 2.4), LINE, &world) {
        Endpoint::Split { segment: 1, point } => assert!(point.distance(vec2(6.0, 2.0)) < 0.001),
        other => panic!("expected a split: {other:?}"),
    }
    // Within the radius of a segment but within 0.5 of its end: free (not a split).
    assert!(matches!(
        turts_interaction::utility::resolve(vec2(4.5, 2.5), LINE, &world),
        Endpoint::Free { .. }
    ));
    // Farther than the radius from everything: free at the point.
    let far = vec2(6.0, 2.0 + SNAP_RADIUS + 0.2);
    assert!(matches!(
        turts_interaction::utility::resolve(far, LINE, &world),
        Endpoint::Free { point } if point == far
    ));
}

#[test]
fn a_foreign_unit_or_a_unit_with_no_network_row_is_never_a_port() {
    let mut world = world();
    // The drill is not listed in a network (it is a site, not completed).
    world.utility.view.networks[0].units.clear();
    assert!(matches!(
        turts_interaction::utility::resolve(vec2(0.0, 7.0), LINE, &world),
        Endpoint::Free { .. }
    ));
    // A foreign unit of the same content never has a port for this player.
    world.utility.view.networks[0].units = vec![UnitId(3)];
    world
        .units
        .iter_mut()
        .find(|unit| unit.id == UnitId(3))
        .unwrap()
        .owner_slot = OTHER;
    assert!(matches!(
        turts_interaction::utility::resolve(vec2(0.0, 7.0), LINE, &world),
        Endpoint::Free { .. }
    ));
}

#[test]
fn a_click_on_a_segment_sends_a_split_and_a_click_on_a_port_sends_the_unit() {
    let world = world();
    let mut core = Core::default();
    enter(&mut core, &world);
    click_at(&mut core, &world, vec2(0.0, 7.0));
    let effects = click_at(&mut core, &world, vec2(6.0, 2.3));
    let target = route_target(sent(&effects)[0]).clone();
    assert_eq!(target.from, UtilityEndpoint::Port(3));
    match target.to {
        UtilityEndpoint::Split { segment, point } => {
            assert_eq!(segment, 1);
            assert!((point.x - 6.0).abs() < 0.01 && (point.y - 2.0).abs() < 0.01);
        }
        other => panic!("expected a split: {other:?}"),
    }
}

#[test]
fn the_depth_keys_stay_between_the_minimum_and_the_maximum() {
    let world = world();
    let mut core = Core::default();
    enter(&mut core, &world);
    for _ in 0..6 {
        press(&mut core, &world, "BracketRight");
    }
    assert_eq!(core.utility_state().route().unwrap().depth, 4);
    for _ in 0..6 {
        press(&mut core, &world, "BracketLeft");
    }
    assert_eq!(core.utility_state().route().unwrap().depth, 1);
    click_at(&mut core, &world, vec2(10.0, 4.0));
    let effects = click_at(&mut core, &world, vec2(10.0, 12.0));
    assert_eq!(
        route_target(sent(&effects)[0]).depth,
        1,
        "the route carries the depth"
    );
}

#[test]
fn the_depth_keys_do_nothing_outside_route_mode() {
    let world = world();
    let mut core = Core::default();
    press(&mut core, &world, "KeyU");
    press(&mut core, &world, "BracketRight");
    assert!(core.utility_state().route().is_none());
}

fn assert_ended(core: &Core) {
    assert!(core.utility_state().route().is_none(), "route mode ended");
    assert!(core.utility_state().view_on(), "the view stays on");
    assert_eq!(core.pending_order_id(), None);
}

#[test]
fn right_click_ends_route_mode_and_sends_nothing() {
    let world = world();
    let mut core = Core::default();
    enter(&mut core, &world);
    click_at(&mut core, &world, vec2(10.0, 4.0));
    let px = at(&core, vec2(0.0, 0.0));
    let effects = click(&mut core, &world, Button::Right, px, vec![], 120);
    assert!(sent(&effects).is_empty());
    assert_ended(&core);
}

#[test]
fn cancel_ends_route_mode_and_a_second_cancel_is_the_normal_cancel() {
    let world = world();
    let mut core = Core::default();
    enter(&mut core, &world);
    let effects = press(&mut core, &world, "Escape");
    assert_ended(&core);
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::ShellAction { .. })),
        "the first cancel only ends the route"
    );
    let effects = press(&mut core, &world, "Escape");
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::ShellAction { action } if action == "menu")),
        "with no route the ordinary cancel is unchanged"
    );
}

#[test]
fn focus_loss_ends_route_mode() {
    let world = world();
    let mut core = Core::default();
    enter(&mut core, &world);
    core.step(&[InputEvent::FocusLost], &world, &VIEW, 90);
    assert_ended(&core);
}

#[test]
fn turning_the_view_off_ends_route_mode() {
    let world = world();
    let mut core = Core::default();
    enter(&mut core, &world);
    press(&mut core, &world, "KeyU");
    assert!(core.utility_state().route().is_none());
    assert!(!core.utility_state().view_on());
}

#[test]
fn an_ordinary_order_ends_route_mode() {
    let mut world = world();
    world
        .orders
        .push(order("forge:order_hold", Behaviour::Hold, TargetKind::None));
    let mut core = Core::default();
    enter(&mut core, &world);
    // A target order sets its own pending order; the route does not survive it.
    world.orders.push(order(
        "forge:order_strike",
        Behaviour::Attack,
        TargetKind::Unit,
    ));
    world
        .orders
        .iter_mut()
        .find(|order| order.id == "forge:order_strike")
        .unwrap()
        .hotkey = "strike_key".into();
    world.input.actions.push(turts_mod_api::ActionDef {
        name: "strike_key".into(),
        default: vec![turts_mod_api::Binding::Key {
            code: "KeyZ".into(),
            mods: vec![],
        }],
    });
    // The route intercepts pointer input only; a hotkey still reaches the core.
    press(&mut core, &world, "KeyZ");
    assert!(core.utility_state().route().is_none());
    assert_eq!(core.pending_order_id(), Some("forge:order_strike"));
}

#[test]
fn a_known_geometry_error_blocks_the_send_and_a_notice_says_why() {
    let world = world();
    let mut core = Core::default();
    enter(&mut core, &world);
    // Longer than 16 world units.
    click_at(&mut core, &world, vec2(1.0, 10.0));
    let effects = click_at(&mut core, &world, vec2(20.0, 10.0));
    assert!(sent(&effects).is_empty());
    assert!(effects.iter().any(|effect| matches!(
        effect,
        Effect::Notice { text_key, .. } if text_key == turts_interaction::utility::reason_text(reason::TOO_LONG)
    )));
    // The start stays: a shorter second click now sends.
    let effects = click_at(&mut core, &world, vec2(10.0, 10.0));
    assert_eq!(sent(&effects).len(), 1);
}

#[test]
fn a_zero_length_route_is_blocked() {
    let world = world();
    let mut core = Core::default();
    enter(&mut core, &world);
    click_at(&mut core, &world, vec2(10.0, 4.0));
    let effects = click_at(&mut core, &world, vec2(10.0, 4.1));
    assert!(sent(&effects).is_empty());
}

#[test]
fn a_duplicate_pair_of_junctions_is_blocked() {
    let world = world();
    let mut core = Core::default();
    enter(&mut core, &world);
    click_at(&mut core, &world, vec2(4.0, 2.0));
    let effects = click_at(&mut core, &world, vec2(8.0, 2.0));
    assert!(
        sent(&effects).is_empty(),
        "node 10 and node 11 already have a segment"
    );
}

#[test]
fn out_of_reach_is_advice_and_the_preview_reports_it() {
    let mut world = world();
    world.utility.utilities.get_mut(LINE).unwrap().reach = 1.0;
    let mut core = Core::default();
    enter(&mut core, &world);
    click_at(&mut core, &world, vec2(10.0, 4.0));
    let px = at(&core, vec2(10.0, 12.0));
    let effects = core.step(
        &[InputEvent::PointerMove {
            px,
            over_widget: false,
        }],
        &world,
        &VIEW,
        110,
    );
    let preview = output(&effects)
        .and_then(|output| output.route.as_ref())
        .expect("preview");
    assert_eq!(preview.reason, Some(reason::OUT_OF_REACH));
    assert!(
        preview
            .length
            .is_some_and(|length| (length - 8.0).abs() < 0.1)
    );
    // The client is not sure about the units' positions: the server decides, so it still sends.
    let effects = click_at(&mut core, &world, vec2(10.0, 12.0));
    assert_eq!(sent(&effects).len(), 1);
}

#[test]
fn the_preview_shows_length_depth_and_cost_without_a_reason_when_all_is_well() {
    let world = world();
    let mut core = Core::default();
    enter(&mut core, &world);
    click_at(&mut core, &world, vec2(10.0, 4.0));
    let px = at(&core, vec2(10.0, 12.0));
    let effects = core.step(
        &[InputEvent::PointerMove {
            px,
            over_widget: false,
        }],
        &world,
        &VIEW,
        110,
    );
    let preview = output(&effects)
        .and_then(|output| output.route.as_ref())
        .expect("preview");
    assert_eq!(preview.reason, None);
    assert_eq!(
        (preview.depth, preview.depth_min, preview.depth_max),
        (2, 1, 4)
    );
    assert_eq!(preview.cost, vec![("Loam".to_owned(), 1)]);
    assert!(preview.start.unwrap().distance(vec2(10.0, 4.0)) < 0.05);
}

#[test]
fn a_click_over_a_widget_does_not_place_a_point() {
    let world = world();
    let mut core = Core::default();
    enter(&mut core, &world);
    let px = at(&core, vec2(10.0, 4.0));
    core.step(
        &[
            InputEvent::PointerDown {
                button: Button::Left,
                px,
                over_widget: true,
            },
            InputEvent::PointerUp {
                button: Button::Left,
                px,
                over_widget: true,
            },
        ],
        &world,
        &VIEW,
        130,
    );
    assert!(core.utility_state().route().unwrap().start.is_none());
}

#[test]
fn a_drag_is_not_a_click() {
    let world = world();
    let mut core = Core::default();
    enter(&mut core, &world);
    let down = at(&core, vec2(10.0, 4.0));
    let up = down + vec2(40.0, 0.0);
    core.step(
        &[
            InputEvent::PointerDown {
                button: Button::Left,
                px: down,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button: Button::Left,
                px: up,
                over_widget: false,
            },
        ],
        &world,
        &VIEW,
        130,
    );
    assert!(core.utility_state().route().unwrap().start.is_none());
}

#[test]
fn the_view_without_a_route_leaves_selection_unchanged() {
    let world = world();
    let mut core = Core::default();
    press(&mut core, &world, "KeyU");
    select_world_point(&mut core, &world, UnitId(2), 70);
    assert_eq!(
        ids(&core),
        vec![UnitId(2)],
        "a click still selects with the view on"
    );
    let px = at(&core, vec2(40.0, 40.0));
    let effects = click(&mut core, &world, Button::Right, px, vec![], 80);
    assert_eq!(
        sent(&effects).len(),
        1,
        "a right click still moves with the view on"
    );
}

#[test]
fn hover_names_the_own_object_under_the_pointer() {
    let world = world();
    let mut core = Core::default();
    press(&mut core, &world, "KeyU");
    let hover_at = |core: &mut Core, point: Vec2| {
        let px = at(core, point);
        let effects = core.step(
            &[InputEvent::PointerMove {
                px,
                over_widget: false,
            }],
            &world,
            &VIEW,
            140,
        );
        output(&effects).and_then(|output| output.hover.clone())
    };
    assert_eq!(
        hover_at(&mut core, vec2(6.0, 2.2)).map(|hover| hover.target),
        Some(HoverTarget::Segment(1))
    );
    assert_eq!(
        hover_at(&mut core, vec2(4.1, 2.1)).map(|hover| hover.target),
        Some(HoverTarget::Node(10))
    );
    assert_eq!(
        hover_at(&mut core, vec2(0.0, 7.1)).map(|hover| hover.target),
        Some(HoverTarget::Port(UnitId(3)))
    );
    assert_eq!(hover_at(&mut core, vec2(-8.0, -6.0)), None);
}

#[test]
fn the_view_off_has_no_preview_and_no_hover() {
    let world = world();
    let mut core = Core::default();
    let px = at(&core, vec2(6.0, 2.2));
    let effects = core.step(
        &[InputEvent::PointerMove {
            px,
            over_widget: false,
        }],
        &world,
        &VIEW,
        140,
    );
    assert!(output(&effects).is_none(), "nothing changed, so no effect");
    assert!(!core.utility_state().view_on());
}
