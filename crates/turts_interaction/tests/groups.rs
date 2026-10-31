mod common;
use common::*;
use turts_interaction::{Core, InputEvent};
use turts_mod_api::{Binding, EngineOverride, Mod};
use turts_net_protocol::UnitId;

fn assign(core: &mut Core, world: &TestWorld, number: &str, mods: Vec<Mod>, now: u64) {
    core.step(
        &[InputEvent::KeyDown {
            code: number.into(),
            mods,
        }],
        world,
        &VIEW,
        now,
    );
}

#[test]
fn group_assignment_contains_only_own_unit_ids() {
    let mut world = TestWorld::default();
    world.units[0].owner_slot = OTHER;
    world.units[0].content_id = "muster:trooper".into();
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(2), 10);
    core.step(
        &[InputEvent::KeyDown {
            code: "ShiftLeft".into(),
            mods: vec![Mod::Shift],
        }],
        &world,
        &VIEW,
        15,
    );
    let foreign = core.project_world_to_screen(world.units[3].position, &VIEW);
    click(
        &mut core,
        &world,
        turts_mod_api::Button::Left,
        foreign,
        vec![Mod::Shift],
        18,
    );
    assign(&mut core, &world, "Digit1", vec![Mod::Ctrl], 20);
    assert_eq!(
        core.group_members(1),
        &[UnitId(2)],
        "assigned control groups must filter a directly inspected foreign unit id"
    );
}

#[test]
fn shift_group_assignment_adds_without_removing_other_memberships() {
    let world = TestWorld::default();
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    assign(&mut core, &world, "Digit1", vec![Mod::Ctrl], 20);
    assign(&mut core, &world, "Digit2", vec![Mod::Ctrl], 40);
    select_world_point(&mut core, &world, UnitId(2), 50);
    assign(&mut core, &world, "Digit1", vec![Mod::Shift], 60);
    assert_eq!(
        core.group_members(1),
        &[UnitId(1), UnitId(2)],
        "Shift+Digit1 adds the current selection to group 1"
    );
    assert_eq!(
        core.group_members(2),
        &[UnitId(1)],
        "adding to group 1 must preserve unit 1 in its existing group 2"
    );
}

#[test]
fn group_recall_restores_members_and_dead_ids_are_pruned() {
    let mut world = TestWorld::default();
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    assign(&mut core, &world, "Digit1", vec![Mod::Ctrl], 20);
    assert_eq!(
        core.group_members(1),
        &[UnitId(1)],
        "prune baseline must contain an assigned live unit"
    );
    assert_eq!(ids(&core), vec![UnitId(1)]);
    world.units.retain(|unit| unit.id != UnitId(1));
    core.step(&[], &world, &VIEW, 30);
    assert!(
        core.selected_units().is_empty(),
        "an id removed from the replica must leave the active selection in the same frame"
    );
    assert_eq!(
        core.group_members(1),
        &[] as &[UnitId],
        "an id missing from the replica must leave every control group in the same frame"
    );
    assign(&mut core, &world, "Digit1", vec![], 40);
    assert!(
        core.selected_units().is_empty(),
        "recalling a pruned group must not resurrect the removed unit"
    );
}

#[test]
fn a_unit_can_belong_to_more_than_one_group() {
    let world = TestWorld::default();
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    assign(&mut core, &world, "Digit1", vec![Mod::Ctrl], 20);
    assign(&mut core, &world, "Digit2", vec![Mod::Shift], 30);
    assert_eq!(
        core.group_members(1),
        &[UnitId(1)],
        "group 1 must retain its member after another group assignment"
    );
    assert_eq!(
        core.group_members(2),
        &[UnitId(1)],
        "one own unit may be present in multiple groups"
    );
}

#[test]
fn rebound_group_add_action_appends_without_physical_shift() {
    let mut world = TestWorld::default();
    world.input.engine_overrides.push(EngineOverride {
        name: "group_add_1".into(),
        bindings: vec![Binding::Key {
            code: "KeyB".into(),
            mods: vec![],
        }],
    });
    let mut core = Core::default();
    select_world_point(&mut core, &world, UnitId(1), 10);
    assign(&mut core, &world, "Digit1", vec![Mod::Ctrl], 20);
    select_world_point(&mut core, &world, UnitId(2), 30);
    core.step(
        &[InputEvent::KeyDown {
            code: "KeyB".into(),
            mods: vec![],
        }],
        &world,
        &VIEW,
        40,
    );
    assert_eq!(
        core.group_members(1),
        &[UnitId(1), UnitId(2)],
        "the resolved group_add action keeps append semantics after rebinding"
    );
}

#[test]
fn restore_groups_filters_bad_slots_foreign_dead_and_duplicate_ids() {
    let world = TestWorld::default();
    let mut core = Core::default();
    let effects = core.step(
        &[InputEvent::RestoreGroups {
            groups: vec![
                (0, vec![UnitId(1)]),
                (1, vec![UnitId(1), UnitId(1), UnitId(4), UnitId(999)]),
                (9, vec![UnitId(2)]),
                (10, vec![UnitId(2)]),
            ],
        }],
        &world,
        &VIEW,
        10,
    );
    assert_eq!(core.group_members(0), &[]);
    assert_eq!(core.group_members(1), &[UnitId(1)]);
    assert_eq!(core.group_members(9), &[UnitId(2)]);
    assert_eq!(
        effects,
        vec![turts_interaction::Effect::GroupsChanged],
        "restore updates the group feed even when invalid slots are discarded"
    );
    core.step(
        &[InputEvent::KeyDown {
            code: "Digit9".into(),
            mods: vec![],
        }],
        &world,
        &VIEW,
        20,
    );
    assert_eq!(core.selected_units(), &[UnitId(2)]);
}
