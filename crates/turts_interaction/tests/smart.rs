mod common;
use common::*;
use turts_interaction::{Core, InputEvent};
use turts_mod_api::{Button, Mod, SmartOrder, SmartTarget, TargetKind};
use turts_net_protocol::{OrderTarget, UnitId};

fn select_two(core: &mut Core, world: &TestWorld, first: UnitId, second: UnitId) {
    select_world_point(core, world, first, 10);
    core.step(
        &[InputEvent::KeyDown {
            code: "ShiftLeft".into(),
            mods: vec![Mod::Shift],
        }],
        world,
        &VIEW,
        20,
    );
    select_world_point(core, world, second, 30);
}

#[test]
fn smart_attack_precedes_movement_fallback_for_each_weapon_subject() {
    let world = TestWorld::default();
    let mut core = Core::default();
    select_two(&mut core, &world, UnitId(1), UnitId(5));
    let foreign = core.project_world_to_screen(world.units[3].position, &VIEW);
    let effects = click(&mut core, &world, Button::Right, foreign, vec![], 100);
    let envelopes = sent(&effects);
    let attack = envelopes
        .iter()
        .find(|envelope| envelope.order == "forge:order_attack")
        .expect("the weapon subject must resolve the first matching attack rule");
    let move_order = envelopes
        .iter()
        .find(|envelope| envelope.order == "forge:order_move")
        .expect("the non-weapon gatherer must resolve the movement fallback");
    assert_eq!(
        attack.units,
        vec![UnitId(1)],
        "each selected subject must resolve its own first applicable smart rule"
    );
    assert_eq!(
        move_order.units,
        vec![UnitId(5)],
        "the fallback applies only to the subject without a weapon"
    );
    assert_eq!(attack.target, OrderTarget::Unit(UnitId(4)));
    assert_eq!(move_order.target, OrderTarget::Unit(UnitId(4)));
    assert!(
        attack.group > 0 && attack.group == move_order.group,
        "envelopes from one smart gesture must share one non-zero group id"
    );
}

#[test]
fn world_point_smart_helper_matches_pointer_command_rules() {
    let world = TestWorld::default();
    let target = world.units[3].position;
    let mut pointer_core = Core::default();
    let mut world_core = Core::default();
    for core in [&mut pointer_core, &mut world_core] {
        core.step(
            &[InputEvent::UiAction {
                name: "ui_select:1".into(),
            }],
            &world,
            &VIEW,
            10,
        );
    }
    let pointer_px = pointer_core.project_world_to_screen(target, &VIEW);
    let pointer_effects = click(
        &mut pointer_core,
        &world,
        Button::Right,
        pointer_px,
        vec![],
        20,
    );
    let mut world_effects = Vec::new();
    world_core.smart_command_at_world_point(target, &world, &mut world_effects);
    assert_eq!(sent(&world_effects), sent(&pointer_effects));
}

#[test]
fn distant_authored_weapon_target_still_uses_attack_rule() {
    let mut world = TestWorld::default();
    world.units[3].position = glam::vec2(24.0, 0.0);
    assert!(world.units[0].position.distance(world.units[3].position) > 4.0);
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    let foreign = core.project_world_to_screen(world.units[3].position, &VIEW);
    let effects = click(&mut core, &world, Button::Right, foreign, vec![], 100);
    let envelopes = sent(&effects);
    assert!(
        envelopes
            .iter()
            .any(|envelope| envelope.order == "forge:order_attack"),
        "smart targeting uses authored weapon eligibility and leaves range resolution to the simulation"
    );
    assert!(
        !envelopes
            .iter()
            .any(|envelope| envelope.order == "forge:order_move"),
        "distance alone must not downgrade an eligible foreign weapon target to movement"
    );
}

#[test]
fn unit_only_move_keeps_unit_policy_and_skips_ground_or_node_targets() {
    let mut world = TestWorld::default();
    world.orders[0].target = TargetKind::Unit;
    world.rules.smart = [
        SmartTarget::Ground,
        SmartTarget::Node,
        SmartTarget::ForeignUnit,
    ]
    .into_iter()
    .map(|target| SmartOrder {
        target,
        needs: turts_mod_api::Capability::Movement,
        order: "forge:order_move".into(),
    })
    .collect();
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);

    let ground = core.project_world_to_screen(glam::vec2(30.0, 30.0), &VIEW);
    assert!(sent(&click(&mut core, &world, Button::Right, ground, vec![], 20)).is_empty());

    let node = core.project_world_to_screen(world.nodes[0].position, &VIEW);
    assert!(sent(&click(&mut core, &world, Button::Right, node, vec![], 30)).is_empty());

    let foreign = core.project_world_to_screen(world.units[3].position, &VIEW);
    let effects = click(&mut core, &world, Button::Right, foreign, vec![], 40);
    let orders = sent(&effects);
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].order, "forge:order_move");
    assert_eq!(orders[0].target, OrderTarget::Unit(UnitId(4)));
}

#[test]
fn smart_gather_precedes_movement_fallback_for_compatible_node() {
    let world = TestWorld::default();
    let mut core = Core::default();
    select_two(&mut core, &world, UnitId(1), UnitId(5));
    let node = core.project_world_to_screen(world.nodes[0].position, &VIEW);
    let effects = click(&mut core, &world, Button::Right, node, vec![], 100);
    let envelopes = sent(&effects);
    let gather = envelopes
        .iter()
        .find(|envelope| envelope.order == "forge:order_gather")
        .expect("the accepting gatherer must match the first compatible gather rule");
    let movement = envelopes
        .iter()
        .find(|envelope| envelope.order == "forge:order_move")
        .expect("the non-gatherer must use the movement fallback");
    assert_eq!(gather.units, vec![UnitId(5)]);
    assert_eq!(
        gather.target,
        OrderTarget::Node(turts_net_protocol::NodeId(70))
    );
    assert_eq!(movement.units, vec![UnitId(1)]);
    assert_eq!(
        movement.target,
        OrderTarget::Point(turts_net_protocol::Pos { x: 24.0, y: 0.0 }),
        "movement falls back to the node position; only Gather carries a Node target"
    );
    assert_eq!(
        gather.group, movement.group,
        "separate smart-rule batches from one gesture must share one group id"
    );
}

#[test]
fn smart_command_omits_selected_subject_when_no_rule_applies() {
    let mut world = TestWorld::default();
    world.units[2].owner_slot = OWN;
    let mut core = Core::default();
    select_two(&mut core, &world, UnitId(1), UnitId(3));
    let foreign = core.project_world_to_screen(world.units[3].position, &VIEW);
    let effects = click(&mut core, &world, Button::Right, foreign, vec![], 100);
    let envelopes = sent(&effects);
    assert_eq!(
        envelopes
            .iter()
            .flat_map(|envelope| envelope.units.iter().copied())
            .collect::<Vec<_>>(),
        vec![UnitId(1)],
        "a static subject with no applicable smart rule must not be sent"
    );
    assert!(effects.iter().any(|effect| matches!(effect, turts_interaction::Effect::Notice { text_key, units } if text_key == "no_applicable_order" && units == &[UnitId(3)])), "the omitted unit must receive the local no_applicable_order notice");
}

#[test]
fn authored_generic_foreign_rule_precedes_specialized_weapon_rule() {
    let mut world = TestWorld::default();
    let generic = world
        .rules
        .smart
        .iter()
        .find(|rule| rule.target == turts_mod_api::SmartTarget::ForeignUnit)
        .unwrap()
        .clone();
    world
        .rules
        .smart
        .retain(|rule| rule.target != turts_mod_api::SmartTarget::ForeignUnit);
    world.rules.smart.insert(0, generic);
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    let foreign = core.project_world_to_screen(world.units[3].position, &VIEW);
    let effects = click(&mut core, &world, Button::Right, foreign, vec![], 100);
    let envelopes = sent(&effects);
    assert_eq!(envelopes.len(), 1);
    assert_eq!(
        envelopes[0].order, "forge:order_move",
        "matching predicates must be evaluated in authored order across generic and specialized rules"
    );
}

#[test]
fn point_only_move_fallback_under_node_uses_node_position() {
    let mut world = TestWorld::default();
    world
        .orders
        .iter_mut()
        .find(|order| order.id == "forge:order_move")
        .unwrap()
        .target = turts_mod_api::TargetKind::Point;
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    let node = core.project_world_to_screen(world.nodes[0].position, &VIEW);
    let effects = click(&mut core, &world, Button::Right, node, vec![], 100);
    let envelopes = sent(&effects);
    assert_eq!(envelopes.len(), 1);
    assert_eq!(envelopes[0].order, "forge:order_move");
    assert_eq!(
        envelopes[0].target,
        OrderTarget::Point(turts_net_protocol::Pos { x: 24.0, y: 0.0 })
    );
}

#[test]
fn lone_unqueued_plain_move_keeps_group_zero() {
    let mut world = TestWorld::default();
    world
        .rules
        .smart
        .retain(|rule| rule.target == turts_mod_api::SmartTarget::Ground);
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    let ground = core.project_world_to_screen(glam::vec2(20.0, 8.0), &VIEW);
    let effects = click(&mut core, &world, Button::Right, ground, vec![], 100);
    let envelopes = sent(&effects);
    assert_eq!(envelopes.len(), 1);
    assert_eq!(envelopes[0].order, "forge:order_move");
    assert_eq!(
        envelopes[0].group, 0,
        "a single plain point Move retains the legacy server fast path"
    );
}
