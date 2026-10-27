//! Command validation (design 7, T4 core; Review Focus 4).

use turts_mod_api::{
    Footprint, MapDef, MapSize, PackageId, Placement, UnitDef, UnitRegistry, WorldPos,
};
use turts_sim::{
    Command, CommandKind, CommandQueue, CommandResult, CommandResults, MoveTarget, RejectReason,
    SlotId, UnitTypes, Vec2, headless_app, load_map, snapshot,
};

const ORIGIN: &str = "fixture_pkg";

/// Two units owned by slot 0 at (8, 8) and (20, 20), on a 64 by 64 map.
fn app_with_two_units() -> bevy_app::App {
    app_with_two_units_on(MapSize { x: 64.0, y: 64.0 })
}

/// The same two units on a map of the given size.
fn app_with_two_units_on(size: MapSize) -> bevy_app::App {
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
    let place = |x: f32, y: f32| Placement {
        unit: format!("{ORIGIN}:walker"),
        owner_slot: 0,
        pos: WorldPos { x, y },
        contents: Vec::new(),
    };
    let map = MapDef {
        id: "field".to_string(),
        size,
        placements: vec![place(8.0, 8.0), place(20.0, 20.0)],
        nodes: Vec::new(),
        slot_economy: Vec::new(),
    };
    load_map(app.world_mut(), &origin, &map).unwrap();
    app
}

fn send(app: &mut bevy_app::App, slot: u8, seq: u32, unit_ids: Vec<u32>, target: Vec2) {
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(slot),
            seq,
            kind: CommandKind::MoveTo { unit_ids, target },
        });
}

fn results(app: &bevy_app::App) -> Vec<CommandResult> {
    app.world().resource::<CommandResults>().0.clone()
}

fn moving_count(app: &mut bevy_app::App) -> usize {
    app.world_mut()
        .query::<&MoveTarget>()
        .iter(app.world())
        .count()
}

#[test]
fn validate_move_to_rejects_malformed_lists() {
    let mut app = app_with_two_units();
    let target = Vec2::new(30.0, 30.0);
    send(&mut app, 0, 1, vec![], target);
    send(&mut app, 0, 2, (1..=257).collect(), target);
    send(&mut app, 0, 3, vec![1, 1], target);
    send(&mut app, 0, 4, vec![1, 99], target);
    app.update();
    let got: Vec<(u32, Result<turts_sim::Applied, RejectReason>)> =
        results(&app).iter().map(|r| (r.seq, r.outcome)).collect();
    assert_eq!(
        got,
        vec![
            (1, Err(RejectReason::EmptyCommand)),
            (2, Err(RejectReason::TooManyUnits)),
            (3, Err(RejectReason::DuplicateUnit)),
            (4, Err(RejectReason::UnknownUnit)),
        ]
    );
    // A rejected command changes nothing.
    assert_eq!(moving_count(&mut app), 0);
    let before = snapshot(app.world_mut());
    app.update();
    assert_eq!(snapshot(app.world_mut()), before);
}

#[test]
fn exactly_256_units_is_not_too_many() {
    let mut app = app_with_two_units();
    // 256 ids pass the length check; the first unknown id then fails the existence check.
    send(&mut app, 0, 1, (1..=256).collect(), Vec2::new(30.0, 30.0));
    app.update();
    assert_eq!(results(&app)[0].outcome, Err(RejectReason::UnknownUnit));
}

#[test]
fn valid_move_to_sets_targets_on_every_listed_unit() {
    let mut app = app_with_two_units();
    send(&mut app, 0, 7, vec![2, 1], Vec2::new(30.0, 30.0));
    app.update();
    let r = results(&app);
    assert_eq!(r.len(), 1);
    assert_eq!(
        (r[0].slot, r[0].seq, r[0].outcome),
        (SlotId(0), 7, Ok(turts_sim::Applied { job_id: None }))
    );
    assert_eq!(moving_count(&mut app), 2);
}

#[test]
fn move_to_for_unowned_unit_is_not_owner() {
    let mut app = app_with_two_units();
    let before = snapshot(app.world_mut());
    send(&mut app, 1, 1, vec![1], Vec2::new(30.0, 30.0));
    for _ in 0..3 {
        app.update();
    }
    // The result is only kept for the tick that drained it, so read it after one update.
    let mut app2 = app_with_two_units();
    send(&mut app2, 1, 1, vec![1], Vec2::new(30.0, 30.0));
    app2.update();
    assert_eq!(results(&app2)[0].outcome, Err(RejectReason::NotOwner));
    assert_eq!(snapshot(app.world_mut()), before, "position is unchanged");
    assert_eq!(moving_count(&mut app), 0);
}

#[test]
fn map_owner_slot_decides_who_may_move_a_unit() {
    // M1 verify: unit 1 is placed for slot 0 and unit 2 for slot 1. A `load_map` that ignores
    // `owner_slot` gives both units to slot 0, and this test goes red.
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
    let place = |owner_slot: u8, x: f32, y: f32| Placement {
        unit: format!("{ORIGIN}:walker"),
        owner_slot,
        pos: WorldPos { x, y },
        contents: Vec::new(),
    };
    let map = MapDef {
        id: "field".to_string(),
        size: MapSize { x: 64.0, y: 64.0 },
        placements: vec![place(0, 8.0, 8.0), place(1, 20.0, 20.0)],
        nodes: Vec::new(),
        slot_economy: Vec::new(),
    };
    load_map(app.world_mut(), &origin, &map).unwrap();
    let owners: Vec<(u32, u8)> = snapshot(app.world_mut())
        .iter()
        .map(|unit| (unit.unit_id, unit.owner_slot))
        .collect();
    assert_eq!(owners, vec![(1, 0), (2, 1)]);

    send(&mut app, 0, 1, vec![2], Vec2::new(30.0, 30.0));
    send(&mut app, 1, 1, vec![1], Vec2::new(30.0, 30.0));
    app.update();
    let got: Vec<(SlotId, Result<turts_sim::Applied, RejectReason>)> =
        results(&app).iter().map(|r| (r.slot, r.outcome)).collect();
    assert_eq!(
        got,
        vec![
            (SlotId(0), Err(RejectReason::NotOwner)),
            (SlotId(1), Err(RejectReason::NotOwner)),
        ]
    );
    assert_eq!(moving_count(&mut app), 0);

    send(&mut app, 1, 2, vec![2], Vec2::new(30.0, 30.0));
    app.update();
    assert_eq!(
        results(&app)[0].outcome,
        Ok(turts_sim::Applied { job_id: None }),
        "slot 1 moves its own unit"
    );
    assert_eq!(moving_count(&mut app), 1);
}

#[test]
fn mixed_ownership_in_one_command_is_not_owner_and_moves_nothing() {
    // Unit 1 and unit 2 are both owned by slot 0. Slot 1 lists both.
    let mut app = app_with_two_units();
    send(&mut app, 1, 1, vec![1, 2], Vec2::new(30.0, 30.0));
    app.update();
    assert_eq!(results(&app)[0].outcome, Err(RejectReason::NotOwner));
    assert_eq!(moving_count(&mut app), 0);
}

#[test]
fn move_to_nan_and_outside_map_rejected() {
    let mut app = app_with_two_units();
    send(&mut app, 0, 1, vec![1], Vec2::new(f32::NAN, 8.0));
    send(&mut app, 0, 2, vec![1], Vec2::new(8.0, f32::INFINITY));
    send(&mut app, 0, 3, vec![1], Vec2::new(65.0, 8.0));
    send(&mut app, 0, 4, vec![1], Vec2::new(-0.5, 8.0));
    send(&mut app, 0, 5, vec![1], Vec2::new(64.0, 64.0));
    send(&mut app, 0, 6, vec![1], Vec2::new(0.0, 0.0));
    app.update();
    let got: Vec<Result<turts_sim::Applied, RejectReason>> =
        results(&app).iter().map(|r| r.outcome).collect();
    assert_eq!(
        got,
        vec![
            Err(RejectReason::TargetNotFinite),
            Err(RejectReason::TargetNotFinite),
            Err(RejectReason::TargetOutsideMap),
            Err(RejectReason::TargetOutsideMap),
            Ok(turts_sim::Applied { job_id: None }),
            Ok(turts_sim::Applied { job_id: None }),
        ]
    );
}

#[test]
fn move_to_bounds_use_the_axis_of_each_coordinate_on_a_non_square_map() {
    // M1 verify: the map is 48 wide and 32 deep. A check that compares `y` with `size.x` (48)
    // accepts (40, 40) and goes red here. A check that compares `x` with `size.y` (32) rejects
    // (40, 20) and goes red here. Every shipped map is square, so only this map tells them apart.
    let mut app = app_with_two_units_on(MapSize { x: 48.0, y: 32.0 });
    send(&mut app, 0, 1, vec![1], Vec2::new(40.0, 40.0));
    send(&mut app, 0, 2, vec![1], Vec2::new(40.0, 20.0));
    send(&mut app, 0, 3, vec![1], Vec2::new(49.0, 20.0));
    send(&mut app, 0, 4, vec![1], Vec2::new(48.0, 32.0));
    app.update();
    let got: Vec<(u32, Result<turts_sim::Applied, RejectReason>)> =
        results(&app).iter().map(|r| (r.seq, r.outcome)).collect();
    assert_eq!(
        got,
        vec![
            (1, Err(RejectReason::TargetOutsideMap)),
            (2, Ok(turts_sim::Applied { job_id: None })),
            (3, Err(RejectReason::TargetOutsideMap)),
            (4, Ok(turts_sim::Applied { job_id: None })),
        ]
    );
}

#[test]
fn results_cleared_each_tick() {
    let mut app = app_with_two_units();
    send(&mut app, 0, 1, vec![1], Vec2::new(30.0, 30.0));
    app.update();
    assert_eq!(results(&app).len(), 1);
    app.update();
    assert_eq!(
        results(&app).len(),
        0,
        "the next tick drains nothing and clears"
    );
}

#[test]
fn reject_reason_strings_match_the_wire_names() {
    let all = [
        (RejectReason::EmptyCommand, "empty_command"),
        (RejectReason::TooManyUnits, "too_many_units"),
        (RejectReason::DuplicateUnit, "duplicate_unit"),
        (RejectReason::UnknownUnit, "unknown_unit"),
        (RejectReason::NotOwner, "not_owner"),
        (RejectReason::TargetNotFinite, "target_not_finite"),
        (RejectReason::TargetOutsideMap, "target_outside_map"),
    ];
    for (reason, text) in all {
        assert_eq!(reason.as_str(), text);
    }
}

#[test]
fn move_to_mixed_mobile_and_static_units_is_atomic() {
    let mut app = headless_app(20);
    let origin = PackageId(ORIGIN.to_string());
    let mobile = UnitDef {
        id: "walker".into(),
        name: "Walker".into(),
        footprint: Footprint::Circle { radius: 0.5 },
        movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
        inventory: None,
        gatherer: None,
        drop_off: None,
        producer: None,
    };
    let static_def = UnitDef {
        id: "yard".into(),
        name: "Yard".into(),
        footprint: Footprint::Circle { radius: 2.0 },
        movement: None,
        inventory: None,
        gatherer: None,
        drop_off: None,
        producer: None,
    };
    let mut types = app.world_mut().resource_mut::<UnitTypes>();
    types.register_unit_type(&origin, mobile).unwrap();
    types.register_unit_type(&origin, static_def).unwrap();
    let map = MapDef {
        id: "field".into(),
        size: MapSize { x: 64.0, y: 64.0 },
        placements: vec![
            Placement {
                unit: format!("{ORIGIN}:walker"),
                owner_slot: 0,
                pos: WorldPos { x: 8.0, y: 8.0 },
                contents: vec![],
            },
            Placement {
                unit: format!("{ORIGIN}:yard"),
                owner_slot: 0,
                pos: WorldPos { x: 20.0, y: 20.0 },
                contents: vec![],
            },
        ],
        nodes: vec![],
        slot_economy: vec![],
    };
    load_map(app.world_mut(), &origin, &map).unwrap();
    send(&mut app, 0, 1, vec![1, 2], Vec2::new(30.0, 30.0));
    app.update();
    assert_eq!(results(&app)[0].outcome, Err(RejectReason::NotMobile));
    assert_eq!(moving_count(&mut app), 0);
}
