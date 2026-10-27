//! Fixed-tick harness and movement integrator.

use turts_mod_api::{
    Footprint, MapDef, MapSize, PackageId, Placement, UnitDef, UnitRegistry, WorldPos,
};
use turts_sim::{
    ChangedThisTick, Command, CommandKind, CommandQueue, MoveTarget, SlotId, Tick, UnitTypes, Vec2,
    headless_app, load_map, snapshot,
};

const ORIGIN: &str = "fixture_pkg";

/// A 64 by 64 map with one unit of speed 3.0 per placement, all owned by slot 0.
fn app_with_units(positions: &[(f32, f32)]) -> bevy_app::App {
    let mut app = headless_app(20);
    let origin = PackageId(ORIGIN.to_string());
    let def = UnitDef {
        id: "walker".to_string(),
        name: "Walker".to_string(),
        movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
        inventory: None,
        gatherer: None,
        drop_off: None,
        producer: None,
        footprint: Footprint::Circle { radius: 0.5 },
    };
    app.world_mut()
        .resource_mut::<UnitTypes>()
        .register_unit_type(&origin, def)
        .unwrap();
    let map = MapDef {
        id: "field".to_string(),
        size: MapSize { x: 64.0, y: 64.0 },
        placements: positions
            .iter()
            .map(|&(x, y)| Placement {
                unit: format!("{ORIGIN}:walker"),
                owner_slot: 0,
                pos: WorldPos { x, y },
                contents: Vec::new(),
            })
            .collect(),
        nodes: Vec::new(),
        slot_economy: Vec::new(),
    };
    load_map(app.world_mut(), &origin, &map).unwrap();
    app
}

fn move_to(app: &mut bevy_app::App, unit_ids: Vec<u32>, target: Vec2) {
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::MoveTo { unit_ids, target },
        });
}

fn moving_count(app: &mut bevy_app::App) -> usize {
    app.world_mut()
        .query::<&MoveTarget>()
        .iter(app.world())
        .count()
}

#[test]
fn harness_runs_one_tick_per_update() {
    let mut app = headless_app(20);
    assert_eq!(app.world().resource::<Tick>().0, 0, "warm-up runs no tick");
    for expected in 1..=3 {
        app.update();
        assert_eq!(app.world().resource::<Tick>().0, expected);
    }
}

#[test]
fn unit_moves_speed_per_second_and_arrives() {
    let mut app = app_with_units(&[(8.0, 8.0)]);
    let target = Vec2::new(11.0, 8.0);
    move_to(&mut app, vec![1], target);
    for _ in 0..10 {
        app.update();
    }
    let pos = snapshot(app.world_mut())[0].pos;
    assert!((pos.x - 9.5).abs() < 1e-5, "x after 10 ticks was {}", pos.x);
    assert!((pos.y - 8.0).abs() < 1e-5);
    assert_eq!(moving_count(&mut app), 1);
    for _ in 0..11 {
        app.update();
    }
    assert_eq!(snapshot(app.world_mut())[0].pos, target);
    assert_eq!(
        moving_count(&mut app),
        0,
        "MoveTarget is removed on arrival"
    );
}

#[test]
fn changed_this_tick_lists_only_moved_units() {
    let mut app = app_with_units(&[(8.0, 8.0), (20.0, 20.0)]);
    move_to(&mut app, vec![1], Vec2::new(30.0, 8.0));
    app.update();
    let changed = &app.world().resource::<ChangedThisTick>().0;
    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].0, 1);
    assert!((changed[0].1.x - 8.15).abs() < 1e-5);
    // The unit stands still from now on: it arrives on the next tick, then nothing changes.
    let mut app = app_with_units(&[(8.0, 8.0), (20.0, 20.0)]);
    move_to(&mut app, vec![1], Vec2::new(8.1, 8.0));
    app.update();
    assert_eq!(app.world().resource::<ChangedThisTick>().0.len(), 1);
    assert_eq!(moving_count(&mut app), 0);
    app.update();
    assert_eq!(app.world().resource::<ChangedThisTick>().0.len(), 0);
}

#[test]
fn spawned_units_do_not_count_as_changed_after_the_first_tick() {
    // Units are spawned outside a tick. A tick that moves nobody reports no change.
    let mut app = app_with_units(&[(8.0, 8.0), (20.0, 20.0)]);
    app.update();
    assert_eq!(app.world().resource::<ChangedThisTick>().0.len(), 0);
}

#[test]
fn move_to_current_position_changes_nothing() {
    let mut app = app_with_units(&[(8.0, 8.0)]);
    app.update(); // settle the first run of the change detection
    move_to(&mut app, vec![1], Vec2::new(8.0, 8.0));
    app.update();
    assert_eq!(app.world().resource::<ChangedThisTick>().0.len(), 0);
    assert_eq!(moving_count(&mut app), 0);
}

#[test]
fn second_load_map_continues_unit_ids() {
    // Task 7c spawns synthetic units through a second load_map call. Ids must stay unique.
    let mut app = app_with_units(&[(8.0, 8.0), (20.0, 20.0)]);
    let origin = PackageId(ORIGIN.to_string());
    let map = MapDef {
        id: "extra".to_string(),
        size: MapSize { x: 64.0, y: 64.0 },
        placements: vec![Placement {
            unit: format!("{ORIGIN}:walker"),
            owner_slot: 1,
            pos: WorldPos { x: 30.0, y: 30.0 },
            contents: Vec::new(),
        }],
        nodes: Vec::new(),
        slot_economy: Vec::new(),
    };
    load_map(app.world_mut(), &origin, &map).unwrap();
    let ids: Vec<u32> = snapshot(app.world_mut())
        .iter()
        .map(|u| u.unit_id)
        .collect();
    assert_eq!(ids, vec![1, 2, 3]);
}
