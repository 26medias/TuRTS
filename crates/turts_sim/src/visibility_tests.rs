use super::*;
use crate::{
    Command, CommandKind, SlotId, Vec2, apply_command, headless_app, install_package, stage_package,
};
use turts_mod_api::{
    HealthDef, Placement, ResolvedWeapon, SourceDef, TargetFilter, VisionDef, WeaponRangeMode,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};

fn id(local: &str) -> ContentId {
    ContentId::parse(&format!("cedar:{local}")).unwrap()
}
fn p(x: f32, y: f32) -> WorldPos {
    WorldPos { x, y }
}
fn prepared(sharing: VisionSharing, limit: u32) -> VisibilityState {
    prepare(
        1,
        id("clearing"),
        &VisibilityDef {
            cell_size: 1.0,
            sharing,
            memory_limit: Some(limit),
        },
        VisibilityGrid {
            size: p(64.0, 64.0),
            cell_size: 1.0,
            columns: 64,
            rows: 64,
        },
        &[0, 0],
    )
    .unwrap()
}
fn visible(unit: u32, owner: u8, position: WorldPos) -> VisibleUnit {
    VisibleUnit {
        unit,
        content: Arc::new(id("worker")),
        owner,
        position,
        facing: None,
        site: false,
    }
}

#[test]
fn prepared_policy_is_fail_closed_until_context_is_finalized() {
    let mut world = World::new();
    world.insert_resource(crate::Tick(0));
    world.insert_resource(prepared(VisionSharing::Team, 10));
    assert!(!context_ready(&world));
    initialize(&mut world);
    for phase in [
        ObservationPhase::Start,
        ObservationPhase::Tactical,
        ObservationPhase::Completed,
    ] {
        let basis = basis(&world, phase).unwrap();
        assert!(!basis.unit_permitted(0, 1));
        assert!(!basis.area_observed(0, p(0.0, 0.0), p(1.0, 1.0)));
    }
    initialize_context(&mut world, 7, &[0, 1]).unwrap();
    assert!(context_ready(&world));
    assert_eq!(
        basis(&world, ObservationPhase::Completed)
            .unwrap()
            .stamp()
            .match_id,
        7
    );
    initialize_context(&mut world, 7, &[0, 1]).unwrap();
    assert!(initialize_context(&mut world, 8, &[0, 0]).is_err());
    world.resource_mut::<crate::Tick>().0 = 1;
    assert!(initialize_context(&mut world, 7, &[0, 1]).is_err());
}

#[test]
fn overlapping_sources_and_team_union_preserve_exact_coverage() {
    for sharing in [VisionSharing::OwnSlot, VisionSharing::Team] {
        let mut state = prepared(sharing, 10);
        let units = vec![
            visible(1, 0, p(2.0, 2.0)),
            visible(2, 0, p(3.0, 2.0)),
            visible(3, 1, p(4.0, 2.0)),
        ];
        state.update(
            1,
            ObservationPhase::Completed,
            units.clone(),
            vec![],
            vec![(1, 0, p(2.0, 2.0), 2.0), (2, 0, p(3.0, 2.0), 2.0)],
        );
        let first = state.phase(ObservationPhase::Completed);
        assert!(first.unit_permitted(0, 3));
        assert_eq!(
            first.point_observed(1, p(4.0, 2.0)),
            sharing == VisionSharing::Team
        );
        state.update(
            2,
            ObservationPhase::Completed,
            units.clone(),
            vec![],
            vec![(2, 0, p(3.0, 2.0), 2.0)],
        );
        assert!(
            state
                .phase(ObservationPhase::Completed)
                .point_observed(0, p(4.0, 2.0))
        );
        state.update(3, ObservationPhase::Completed, units, vec![], vec![]);
        let empty = state
            .phase(ObservationPhase::Completed)
            .knowledge(0)
            .unwrap();
        assert!(empty.observed.iter().all(|byte| *byte == 0));
        assert!(empty.explored.iter().any(|byte| *byte != 0));
        assert!(
            first.unit_permitted(0, 3),
            "old phase basis remains immutable"
        );
    }
}

#[test]
fn hidden_movement_and_hidden_death_produce_identical_stale_knowledge() {
    let initial = vec![visible(1, 0, p(2.0, 2.0)), visible(2, 1, p(4.0, 2.0))];
    let mut first = prepared(VisionSharing::OwnSlot, 10);
    first.update(
        1,
        ObservationPhase::Completed,
        initial,
        vec![],
        vec![(1, 0, p(2.0, 2.0), 4.0)],
    );
    let mut absent = first.clone();
    first.update(
        2,
        ObservationPhase::Completed,
        vec![visible(1, 0, p(20.0, 20.0)), visible(2, 1, p(40.0, 40.0))],
        vec![],
        vec![(1, 0, p(20.0, 20.0), 4.0)],
    );
    absent.update(
        2,
        ObservationPhase::Completed,
        vec![visible(1, 0, p(20.0, 20.0))],
        vec![],
        vec![(1, 0, p(20.0, 20.0), 4.0)],
    );
    let stale = first
        .phase(ObservationPhase::Completed)
        .knowledge(0)
        .unwrap();
    assert_eq!(
        stale,
        absent
            .phase(ObservationPhase::Completed)
            .knowledge(0)
            .unwrap()
    );
    assert_eq!(stale.remembered_units.len(), 1);
    assert_eq!(stale.remembered_units[0].position, p(4.0, 2.0));
    assert_eq!(stale.remembered_units[0].last_seen_tick, 1);
    first.update(
        3,
        ObservationPhase::Completed,
        vec![visible(1, 0, p(2.0, 2.0)), visible(2, 1, p(40.0, 40.0))],
        vec![],
        vec![(1, 0, p(2.0, 2.0), 4.0)],
    );
    assert!(
        first
            .phase(ObservationPhase::Completed)
            .knowledge(0)
            .unwrap()
            .remembered_units
            .is_empty()
    );
}

#[test]
fn retention_discards_a_whole_oldest_tick_group_and_the_flag_stays_set() {
    let mut state = prepared(VisionSharing::OwnSlot, 1);
    state.update(
        1,
        ObservationPhase::Completed,
        vec![
            visible(1, 0, p(2.0, 2.0)),
            visible(80, 1, p(4.0, 2.0)),
            visible(2, 1, p(4.0, 3.0)),
        ],
        vec![],
        vec![(1, 0, p(2.0, 2.0), 4.0)],
    );
    state.update(
        2,
        ObservationPhase::Completed,
        vec![visible(1, 0, p(20.0, 20.0))],
        vec![],
        vec![(1, 0, p(20.0, 20.0), 4.0)],
    );
    let memory = state
        .phase(ObservationPhase::Completed)
        .knowledge(0)
        .unwrap();
    assert!(
        memory.remembered_units.is_empty(),
        "an ID cannot choose which tied sighting survives"
    );
    assert!(memory.retention_truncated);
    state.update(
        3,
        ObservationPhase::Completed,
        vec![visible(1, 0, p(20.0, 20.0))],
        vec![],
        vec![(1, 0, p(20.0, 20.0), 4.0)],
    );
    assert!(
        state
            .phase(ObservationPhase::Completed)
            .knowledge(0)
            .unwrap()
            .retention_truncated
    );
}

fn scene(armed: bool) -> bevy_app::App {
    let bytes =
        pack_dir(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar"))
            .unwrap();
    let archive = PackageArchive::parse(&bytes).unwrap();
    let mut catalog = archive.content().unwrap();
    let mut enemy = catalog.units[&id("worker")].clone();
    enemy.id = id("other");
    enemy.vision = None;
    enemy.inventory = None;
    enemy.gatherer = None;
    enemy.health = Some(HealthDef { max: 100 });
    catalog.units.insert(enemy.id.clone(), enemy);
    let source = catalog.units.get_mut(&id("worker")).unwrap();
    source.vision = Some(VisionDef { radius: 4.0 });
    source.health = Some(HealthDef { max: 100 });
    if armed {
        source.weapon = Some(ResolvedWeapon {
            damage_type: None,
            bonus: vec![],
            range: 30.0,
            range_mode: WeaponRangeMode::Centre,
            cooldown_ms: 100,
            damage: 1,
            ammo: None,
            targets: TargetFilter {
                slots: turts_mod_api::SlotRelation::Enemy,
            },
            when_short: vec![],
        });
    }
    let (file, mut map) = archive.maps().unwrap().remove(0);
    map.placements = vec![
        Placement {
            unit: "cedar:worker".into(),
            owner_slot: 0,
            pos: p(8.0, 8.0),
            contents: vec![],
            facing: None,
            start_order: None,
        },
        Placement {
            unit: "cedar:other".into(),
            owner_slot: 1,
            pos: p(24.0, 8.0),
            contents: vec![],
            facing: None,
            start_order: None,
        },
    ];
    map.nodes.clear();
    map.belts.clear();
    let mut app = headless_app(20);
    let staged = stage_package(app.world(), catalog, SourceDef { file, value: map }, 20).unwrap();
    install_package(app.world_mut(), staged);
    app.world_mut()
        .insert_resource(prepared(VisionSharing::OwnSlot, 10));
    initialize_context(app.world_mut(), 1, &[0, 1]).unwrap();
    app
}
fn move_to(app: &mut bevy_app::App, slot: u8, unit: u32, x: f32, seq: u32) {
    apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(slot),
            seq,
            kind: CommandKind::MoveTo {
                unit_ids: vec![unit],
                target: Vec2::new(x, 8.0),
            },
        },
    )
    .outcome
    .unwrap();
}
fn run(app: &mut bevy_app::App, ticks: usize) {
    for _ in 0..ticks {
        app.update();
    }
}

#[test]
fn ordinary_movement_reveals_then_preserves_and_disproves_a_stale_sighting() {
    let mut app = scene(false);
    assert!(
        !basis(app.world(), ObservationPhase::Completed)
            .unwrap()
            .unit_permitted(0, 2)
    );
    move_to(&mut app, 0, 1, 22.0, 1);
    run(&mut app, 160);
    assert!(
        basis(app.world(), ObservationPhase::Completed)
            .unwrap()
            .unit_permitted(0, 2)
    );
    move_to(&mut app, 0, 1, 8.0, 2);
    run(&mut app, 160);
    let stale = basis(app.world(), ObservationPhase::Completed)
        .unwrap()
        .knowledge(0)
        .unwrap();
    assert_eq!(stale.remembered_units.len(), 1);
    let remembered = stale.remembered_units[0].clone();
    move_to(&mut app, 1, 2, 40.0, 1);
    run(&mut app, 160);
    assert_eq!(
        basis(app.world(), ObservationPhase::Completed)
            .unwrap()
            .knowledge(0)
            .unwrap()
            .remembered_units,
        [remembered]
    );
    move_to(&mut app, 0, 1, 22.0, 3);
    run(&mut app, 160);
    assert!(
        basis(app.world(), ObservationPhase::Completed)
            .unwrap()
            .knowledge(0)
            .unwrap()
            .remembered_units
            .is_empty()
    );
}

#[test]
fn ordinary_hidden_attack_and_auto_fire_do_not_read_a_hidden_target() {
    use crate::orders::{OrderCommand, OrderTargetSim, QueueMode};
    let mut app = scene(true);
    for target in [2, 999] {
        let result = apply_command(
            app.world_mut(),
            &Command {
                slot: SlotId(0),
                seq: target,
                kind: CommandKind::Order(Box::new(OrderCommand {
                    order: id("attack"),
                    behaviour: turts_mod_api::Behaviour::Attack,
                    target_kind: turts_mod_api::TargetKind::Unit,
                    policy: None,
                    units: vec![1],
                    target: OrderTargetSim::Unit(target),
                    queue: QueueMode::Replace,
                    group: 0,
                    allowed_units: vec![],
                    queue_depth: 16,
                })),
            },
        );
        assert_eq!(result.outcome, Err(crate::RejectReason::TargetInvalid));
    }
    run(&mut app, 40);
    assert_eq!(
        app.world().resource::<crate::economy::combat::Healths>().0[&2],
        100
    );
    move_to(&mut app, 0, 1, 22.0, 1000);
    run(&mut app, 160);
    assert!(app.world().resource::<crate::economy::combat::Healths>().0[&2] < 100);
}
