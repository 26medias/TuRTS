#![cfg(test)]

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
    scene_with(armed, |_, _| {})
}
fn scene_with(
    armed: bool,
    edit: impl FnOnce(&mut turts_mod_api::ContentCatalog, &mut turts_mod_api::MapDef),
) -> bevy_app::App {
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
    edit(&mut catalog, &mut map);
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

#[test]
fn sight_without_health_and_manually_disabled_sight_sources_remain_valid() {
    let no_health = scene_with(false, |catalog, _| {
        catalog.units.get_mut(&id("worker")).unwrap().health = None;
    });
    assert!(
        basis(no_health.world(), ObservationPhase::Completed)
            .unwrap()
            .point_observed(0, p(9.0, 8.0))
    );
    let mut disabled = scene(true);
    move_to(&mut disabled, 0, 1, 22.0, 1);
    run(&mut disabled, 160);
    apply_command(
        disabled.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 2,
            kind: CommandKind::SetEnabled {
                unit_id: 1,
                enabled: false,
            },
        },
    )
    .outcome
    .unwrap();
    run(&mut disabled, 5);
    assert!(
        basis(disabled.world(), ObservationPhase::Completed)
            .unwrap()
            .unit_permitted(0, 2)
    );
}

#[test]
fn final_source_death_keeps_authorized_shot_handles_and_last_seen_memory() {
    let mut app = scene_with(true, |catalog, map| {
        map.placements[0].pos = p(22.0, 8.0);
        let enemy = catalog.units.get_mut(&id("other")).unwrap();
        enemy.vision = Some(VisionDef { radius: 40.0 });
        enemy.weapon = Some(ResolvedWeapon {
            damage_type: None,
            bonus: vec![],
            range: 30.0,
            range_mode: WeaponRangeMode::Centre,
            cooldown_ms: 100,
            damage: 200,
            ammo: None,
            targets: TargetFilter {
                slots: turts_mod_api::SlotRelation::Enemy,
            },
            when_short: vec![],
        });
    });
    run(&mut app, 1);
    assert!(
        crate::unit_entity(app.world(), 1).is_none(),
        "ordinary enemy fire killed the sight source"
    );
    assert_eq!(
        app.world().resource::<crate::economy::combat::Healths>().0[&2],
        99,
        "the physically earlier source fired before its death"
    );
    let completed = basis(app.world(), ObservationPhase::Completed).unwrap();
    assert!(!completed.unit_permitted(0, 2));
    assert!(
        completed
            .knowledge(0)
            .unwrap()
            .observed
            .iter()
            .all(|byte| *byte == 0)
    );
    let memory = completed.knowledge(0).unwrap();
    assert_eq!(memory.remembered_units.len(), 1);
    assert_eq!(
        (
            memory.remembered_units[0].unit,
            memory.remembered_units[0].last_seen_tick
        ),
        (2, 1)
    );
    let witnesses = event_units(app.world(), 0);
    assert!(witnesses.iter().any(|row| row.unit.unit == 1));
    assert!(witnesses.iter().any(|row| row.unit.unit == 2));
    assert!(
        witnesses
            .iter()
            .all(|row| row.basis.phase == ObservationPhase::Tactical)
    );
}

fn attack(app: &mut bevy_app::App, queue: crate::QueueMode, seq: u32) {
    use crate::orders::{OrderCommand, OrderTargetSim};
    apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: id("attack"),
                behaviour: turts_mod_api::Behaviour::Attack,
                target_kind: turts_mod_api::TargetKind::Unit,
                policy: None,
                units: vec![1],
                target: OrderTargetSim::Unit(2),
                queue,
                group: 0,
                allowed_units: vec![],
                queue_depth: 16,
            })),
        },
    )
    .outcome
    .unwrap();
}
#[test]
fn explicit_attack_finishes_when_the_target_leaves_sight_without_a_death_claim() {
    let mut app = scene_with(true, |_, map| map.placements[1].pos = p(10.0, 8.0));
    attack(&mut app, crate::QueueMode::Replace, 1);
    move_to(&mut app, 1, 2, 40.0, 1);
    run(&mut app, 240);
    let source = crate::unit_entity(app.world(), 1).unwrap();
    let orders = app.world().get::<crate::UnitOrders>(source).unwrap();
    assert!(orders.current.is_none());
    assert!(
        orders.last_failure.is_none(),
        "loss of sight does not assert TargetGone"
    );
    assert_eq!(
        app.world().get::<crate::Position>(source).unwrap().0,
        Vec2::new(8.0, 8.0)
    );
    assert!(crate::unit_entity(app.world(), 2).is_some());
}

#[test]
fn an_unfinished_site_contributes_sight_only_after_actual_completion() {
    use turts_mod_api::{
        CancelRefunds, ConstructProgress, PaymentPolicy, Ratio, RefundRatio, ResolvedConstructor,
        ResolvedProduction, SitePlacementDef,
    };
    let mut app = scene_with(false, |catalog, _| {
        let mut watch = catalog.units[&id("worker")].clone();
        watch.id = id("watch");
        watch.movement = None;
        watch.inventory = None;
        watch.gatherer = None;
        watch.placement = Some(SitePlacementDef {
            gap: 0.0,
            node_clearance: 0.0,
            in_build_area: false,
            site_health: Ratio {
                numerator: 1,
                denominator: 4,
            },
        });
        catalog.units.insert(watch.id.clone(), watch);
        catalog.units.get_mut(&id("worker")).unwrap().constructor = Some(ResolvedConstructor {
            buildings: vec![id("build_watch")],
            concurrent_sites: 2,
            queue_capacity: 2,
            progress: ConstructProgress::Autonomous,
            when_short: vec![],
        });
        catalog.production.insert(
            id("build_watch"),
            ResolvedProduction {
                id: id("build_watch"),
                name: "Watch".into(),
                output_unit: id("watch"),
                duration_ms: 150,
                costs: vec![],
                payment: PaymentPolicy::ChargeOnQueue,
                cancel_refunds: CancelRefunds {
                    queued: RefundRatio::ZERO,
                    running: RefundRatio::ZERO,
                    output_blocked: RefundRatio::ZERO,
                    destroyed: RefundRatio::ZERO,
                },
            },
        );
    });
    app.world_mut()
        .insert_resource(crate::economy::construction::ConstructionLimit(2));
    crate::economy::construction::construct(
        app.world_mut(),
        SlotId(0),
        1,
        id("build_watch"),
        Vec2::new(21.0, 8.0),
    )
    .unwrap();
    run(&mut app, 1);
    assert_eq!(
        app.world()
            .resource::<crate::economy::construction::ConstructionState>()
            .sites
            .len(),
        1
    );
    assert!(
        !basis(app.world(), ObservationPhase::Completed)
            .unwrap()
            .unit_permitted(0, 2)
    );
    run(&mut app, 2);
    assert!(
        app.world()
            .resource::<crate::economy::construction::ConstructionState>()
            .sites
            .is_empty()
    );
    assert!(
        basis(app.world(), ObservationPhase::Completed)
            .unwrap()
            .unit_permitted(0, 2)
    );
}

#[test]
fn legacy_gather_rejects_hidden_and_absent_nodes_with_the_same_reason() {
    let mut app = scene_with(false, |_, map| {
        map.nodes.push(turts_mod_api::NodePlacement {
            node: "cedar:grove".into(),
            pos: p(40.0, 40.0),
            remaining: 100,
        })
    });
    for node in [1, 999] {
        let result = apply_command(
            app.world_mut(),
            &Command {
                slot: SlotId(0),
                seq: node,
                kind: CommandKind::Gather {
                    unit_id: 1,
                    node_id: crate::NodeId(node),
                },
            },
        );
        assert_eq!(result.outcome, Err(crate::RejectReason::TargetInvalid));
    }
    assert_eq!(
        app.world().resource::<crate::SurfaceNodes>().0[0].remaining,
        100
    );
}

#[test]
fn autonomous_extraction_waits_for_permitted_node_knowledge() {
    let mut app = scene_with(false, |catalog, map| {
        let drill = catalog.units.get_mut(&id("worker")).unwrap();
        drill.vision = None;
        drill.movement = None;
        drill.gatherer = None;
        drill.extractor = Some(turts_mod_api::ExtractorDef {
            per_second: 20,
            interaction_range: 3.0,
        });
        catalog.units.get_mut(&id("other")).unwrap().vision = Some(VisionDef { radius: 4.0 });
        map.placements[1].owner_slot = 0;
        map.nodes.push(turts_mod_api::NodePlacement {
            node: "cedar:grove".into(),
            pos: p(8.0, 10.0),
            remaining: 100,
        });
    });
    run(&mut app, 40);
    assert!(
        !basis(app.world(), ObservationPhase::Completed)
            .unwrap()
            .node_permitted(0, 1)
    );
    assert_eq!(
        app.world().resource::<crate::SurfaceNodes>().0[0].remaining,
        100
    );
    move_to(&mut app, 0, 2, 9.0, 1);
    run(&mut app, 200);
    assert!(
        basis(app.world(), ObservationPhase::Completed)
            .unwrap()
            .node_permitted(0, 1)
    );
    assert!(app.world().resource::<crate::SurfaceNodes>().0[0].remaining < 100);
}

#[test]
fn queued_attack_rechecks_visibility_when_the_prior_point_move_finishes() {
    let mut app = scene_with(true, |_, map| map.placements[1].pos = p(10.0, 10.0));
    move_to(&mut app, 0, 1, 22.0, 1);
    attack(&mut app, crate::QueueMode::Append, 2);
    let entity = crate::unit_entity(app.world(), 1).unwrap();
    assert_eq!(
        app.world()
            .get::<crate::UnitOrders>(entity)
            .unwrap()
            .queue
            .len(),
        1
    );
    run(&mut app, 200);
    let orders = app.world().get::<crate::UnitOrders>(entity).unwrap();
    assert!(orders.current.is_none() && orders.queue.is_empty());
    assert_eq!(
        orders.last_failure.as_ref().map(|failure| failure.reason),
        Some(crate::RejectReason::TargetInvalid)
    );
    assert!(
        app.world()
            .get::<crate::Position>(entity)
            .unwrap()
            .0
            .distance(Vec2::new(22.0, 8.0))
            < 0.01
    );
    assert!(crate::unit_entity(app.world(), 2).is_some());
}

#[test]
fn recorded_commands_reproduce_every_phase_of_permitted_knowledge() {
    let commands = [
        (
            0,
            Command {
                slot: SlotId(0),
                seq: 1,
                kind: CommandKind::MoveTo {
                    unit_ids: vec![1],
                    target: Vec2::new(22.0, 8.0),
                },
            },
        ),
        (
            160,
            Command {
                slot: SlotId(0),
                seq: 2,
                kind: CommandKind::MoveTo {
                    unit_ids: vec![1],
                    target: Vec2::new(8.0, 8.0),
                },
            },
        ),
        (
            320,
            Command {
                slot: SlotId(1),
                seq: 1,
                kind: CommandKind::MoveTo {
                    unit_ids: vec![2],
                    target: Vec2::new(40.0, 8.0),
                },
            },
        ),
        (
            480,
            Command {
                slot: SlotId(0),
                seq: 3,
                kind: CommandKind::MoveTo {
                    unit_ids: vec![1],
                    target: Vec2::new(24.0, 8.0),
                },
            },
        ),
    ];
    let replay = || {
        let mut app = scene(false);
        let mut evidence = Vec::new();
        let mut ever_visible = false;
        let mut ever_remembered = false;
        for tick in 0..660 {
            let mut results = Vec::new();
            for (_, command) in commands.iter().filter(|(at, _)| *at == tick) {
                let result = apply_command(app.world_mut(), command);
                assert!(result.outcome.is_ok());
                results.push(serde_json::to_value(result).unwrap());
            }
            app.update();
            let mut phases = Vec::new();
            for phase in [
                ObservationPhase::Start,
                ObservationPhase::Tactical,
                ObservationPhase::Completed,
            ] {
                let basis = basis(app.world(), phase).unwrap();
                ever_visible |= basis.unit_permitted(0, 2);
                let knowledge = basis.knowledge(0).unwrap();
                ever_remembered |= !knowledge.remembered_units.is_empty();
                phases.push(serde_json::to_value(knowledge).unwrap());
            }
            evidence.push((results, phases));
        }
        assert!(
            ever_visible && ever_remembered,
            "the command tape must cross real sight boundaries"
        );
        evidence
    };
    assert_eq!(
        replay(),
        replay(),
        "the same commands must reproduce each phase, mask, memory, and result"
    );
}

#[test]
fn second_install_is_atomic_and_a_fresh_match_has_no_old_memory() {
    let mut app = scene(false);
    move_to(&mut app, 0, 1, 22.0, 1);
    run(&mut app, 160);
    move_to(&mut app, 0, 1, 8.0, 2);
    run(&mut app, 160);
    let before = basis(app.world(), ObservationPhase::Completed)
        .unwrap()
        .knowledge(0)
        .unwrap();
    assert!(!before.remembered_units.is_empty());
    let bytes =
        pack_dir(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar"))
            .unwrap();
    let archive = PackageArchive::parse(&bytes).unwrap();
    let (file, map) = archive.maps().unwrap().remove(0);
    let Err(error) = stage_package(
        app.world(),
        archive.content().unwrap(),
        SourceDef { file, value: map },
        20,
    ) else {
        panic!("a second live install must refuse");
    };
    assert_eq!(error.field_path, "map");
    assert_eq!(
        before,
        basis(app.world(), ObservationPhase::Completed)
            .unwrap()
            .knowledge(0)
            .unwrap()
    );
    let fresh = scene(false);
    let clean = basis(fresh.world(), ObservationPhase::Completed)
        .unwrap()
        .knowledge(0)
        .unwrap();
    assert!(clean.remembered_units.is_empty());
    assert_eq!(clean.basis.tick, 0);
    assert!(!clean.retention_truncated);
}

#[test]
fn stale_only_storage_uses_the_latest_permitted_phase_for_units_and_nodes() {
    let mut state = prepared(VisionSharing::OwnSlot, 10);
    let mut enemy = visible(2, 1, p(4.0, 2.0));
    let mut node = VisibleNode {
        node: 3,
        content: Arc::new(id("wood")),
        position: p(4.0, 3.0),
        remaining: 100,
    };
    state.update(
        6,
        ObservationPhase::Completed,
        vec![enemy.clone()],
        vec![node.clone()],
        vec![(1, 0, p(2.0, 2.0), 6.0)],
    );
    enemy.position = p(5.0, 2.0);
    enemy.facing = Some(Facing::NegY);
    node.remaining = 40;
    state.update(
        7,
        ObservationPhase::Start,
        vec![enemy.clone()],
        vec![node.clone()],
        vec![(1, 0, p(2.0, 2.0), 6.0)],
    );
    let permitted = state.phase(ObservationPhase::Start);
    assert!(state.memories[0].units.is_empty());
    assert!(state.memories[0].nodes.is_empty());
    state.update(
        7,
        ObservationPhase::Tactical,
        vec![visible(2, 1, p(50.0, 50.0))],
        vec![node.clone()],
        vec![],
    );
    let hidden = state.phase(ObservationPhase::Tactical);
    let knowledge = hidden.knowledge(0).unwrap();
    assert_eq!(knowledge.remembered_units, [enemy.remembered(7)]);
    assert_eq!(knowledge.remembered_nodes, [node.remembered(7)]);
    assert!(permitted.knowledge(0).unwrap().remembered_units.is_empty());
    assert_eq!(permitted.permitted_unit(0, 2), Some(&enemy));
    state.update(
        8,
        ObservationPhase::Completed,
        vec![enemy],
        vec![node],
        vec![(1, 0, p(2.0, 2.0), 6.0)],
    );
    assert!(state.memories[0].units.is_empty());
    assert!(state.memories[0].nodes.is_empty());
    assert_eq!(hidden.knowledge(0).unwrap(), knowledge);
}

#[test]
fn observed_removal_cannot_restore_a_previous_phase_row_as_stale_memory() {
    let mut state = prepared(VisionSharing::OwnSlot, 10);
    state.update(
        7,
        ObservationPhase::Start,
        vec![visible(2, 1, p(4.0, 2.0))],
        vec![],
        vec![(1, 0, p(2.0, 2.0), 6.0)],
    );
    state.observed_removals[0].insert(2);
    state.update(7, ObservationPhase::Completed, vec![], vec![], vec![]);
    assert!(state.memories[0].units.is_empty());
    assert!(
        state
            .completed
            .knowledge(0)
            .unwrap()
            .remembered_units
            .is_empty()
    );
}

#[test]
fn typed_capture_tracks_archetype_changes_facing_spawns_and_source_death() {
    let mut app = scene(false);
    let entity = crate::unit_entity(app.world(), 1).unwrap();
    let kind = *app.world().get::<crate::UnitType>(entity).unwrap();
    app.world_mut()
        .entity_mut(entity)
        .remove::<crate::world::UnitFacing>();
    let (units, sources) = capture_units(app.world_mut());
    assert_eq!(units.iter().find(|row| row.unit == 1).unwrap().facing, None);
    assert!(sources.iter().any(|source| source.0 == 1));
    app.world_mut()
        .entity_mut(entity)
        .insert(crate::world::UnitFacing(Facing::NegX));
    let extra = app
        .world_mut()
        .spawn((
            crate::UnitId(99),
            kind,
            crate::Owner(SlotId(0)),
            crate::Position(Vec2::new(9.0, 9.0)),
        ))
        .id();
    app.world_mut()
        .resource_mut::<crate::economy::combat::Healths>()
        .0
        .insert(99, 1);
    let (units, sources) = capture_units(app.world_mut());
    assert_eq!(
        units.iter().find(|row| row.unit == 1).unwrap().facing,
        Some(Facing::NegX)
    );
    assert!(
        units
            .iter()
            .any(|row| row.unit == 99 && row.facing.is_none())
    );
    assert!(sources.iter().any(|source| source.0 == 99));
    app.world_mut()
        .resource_mut::<crate::economy::combat::Healths>()
        .0
        .insert(99, 0);
    let (units, sources) = capture_units(app.world_mut());
    assert!(units.iter().any(|row| row.unit == 99));
    assert!(!sources.iter().any(|source| source.0 == 99));
    app.world_mut().entity_mut(extra).remove::<crate::Owner>();
    let (units, _) = capture_units(app.world_mut());
    assert!(!units.iter().any(|row| row.unit == 99));
}

#[test]
fn cached_circle_deltas_match_full_rebuilds_across_routes_radius_changes_and_teams() {
    for sharing in [VisionSharing::OwnSlot, VisionSharing::Team] {
        let mut incremental = prepared(sharing, 10);
        let mut explored = vec![vec![0u8; 512]; usize::from(SLOT_COUNT)];
        for step in 0..40u32 {
            let mut sources = vec![
                (
                    1,
                    0,
                    p(2.0 + step as f32 * 1.3, 10.0),
                    3.0 + (step % 7) as f32,
                ),
                (2, (step / 10 % 2) as u8, p(30.0, 2.0 + step as f32), 8.0),
            ];
            if step % 3 == 0 {
                sources.pop();
            }
            incremental.update(
                step,
                ObservationPhase::Completed,
                vec![],
                vec![],
                sources.clone(),
            );
            let mut rebuilt = prepared(sharing, 10);
            rebuilt.update(step, ObservationPhase::Completed, vec![], vec![], sources);
            for (viewer, accumulated) in explored.iter_mut().enumerate() {
                let expected = &rebuilt.coverage[viewer];
                for (seen, current) in accumulated.iter_mut().zip(expected.observed.iter()) {
                    *seen |= current;
                }
                assert_eq!(
                    incremental.coverage[viewer].counts, expected.counts,
                    "step {step}, viewer {viewer}"
                );
                assert_eq!(incremental.coverage[viewer].observed, expected.observed);
                assert_eq!(
                    incremental.coverage[viewer].explored.as_ref(),
                    &*accumulated
                );
            }
            assert!(incremental.cached_source_cell_bytes() <= MAX_CACHED_SOURCE_CELL_BYTES);
        }
    }
}

#[test]
fn immutable_permission_index_matches_geometry_for_dense_sparse_and_inverted_ids() {
    for identities in [[1, 2, 3, 4], [u32::MAX, 1_000_000, 7, 0], [4, 3, 2, 1]] {
        for sharing in [VisionSharing::OwnSlot, VisionSharing::Team] {
            let mut state = prepared(sharing, 10);
            let units = vec![
                visible(identities[0], 0, p(60.0, 60.0)),
                visible(identities[1], 1, p(4.0, 2.0)),
                visible(identities[2], 1, p(50.0, 50.0)),
                visible(identities[3], 1, p(0.0, 0.0)),
            ];
            state.update(
                1,
                ObservationPhase::Start,
                units.clone(),
                vec![],
                vec![(9, 0, p(2.0, 2.0), 4.0)],
            );
            let old = state.phase(ObservationPhase::Start);
            for viewer in 0..=SLOT_COUNT {
                for unit in &units {
                    let expected = viewer < SLOT_COUNT
                        && (unit.owner == viewer || old.point_observed(viewer, unit.position));
                    assert_eq!(old.unit_permitted(viewer, unit.unit), expected);
                    assert_eq!(
                        old.permitted_unit(viewer, unit.unit),
                        expected.then_some(unit)
                    );
                }
                assert!(!old.unit_permitted(viewer, 8));
            }
            if identities[0] == u32::MAX {
                assert!(
                    old.unit_rows.is_none(),
                    "sparse IDs must not allocate a dense table"
                );
            } else {
                let rows = old.unit_rows.as_ref().unwrap();
                assert!(rows.len() <= MAX_DENSE_UNIT_ROWS);
                assert!(rows.len() <= units.len() * 4);
            }
            let mut changed = units;
            changed[1].position = p(55.0, 55.0);
            changed[1].facing = Some(Facing::PosY);
            state.update(
                1,
                ObservationPhase::Tactical,
                changed.clone(),
                vec![],
                vec![],
            );
            let next = state.phase(ObservationPhase::Tactical);
            if let Some(rows) = &old.unit_rows {
                assert!(
                    Arc::ptr_eq(rows, next.unit_rows.as_ref().unwrap()),
                    "unchanged IDs reuse the immutable index"
                );
            }
            assert!(old.unit_permitted(0, identities[1]));
            assert!(!next.unit_permitted(0, identities[1]));
            assert_eq!(
                old.permitted_unit(0, identities[1]).unwrap().position,
                p(4.0, 2.0)
            );
            changed.remove(0);
            state.update(2, ObservationPhase::Completed, changed, vec![], vec![]);
            assert!(state.completed.permitted_unit(0, identities[0]).is_none());
            assert!(old.permitted_unit(0, identities[0]).is_some());
        }
    }
}

#[test]
fn dense_permission_index_rejects_unbounded_sparse_storage() {
    for identity in [8, MAX_DENSE_UNIT_ROWS as u32, u32::MAX] {
        let units = [
            visible(0, 0, p(1.0, 1.0)),
            visible(identity, 1, p(2.0, 2.0)),
        ];
        assert!(dense_unit_rows(&units).is_none());
    }
    let units = [visible(0, 0, p(1.0, 1.0)), visible(7, 1, p(2.0, 2.0))];
    assert_eq!(dense_unit_rows(&units).unwrap().len(), units.len() * 4);
}

fn repair_scene(automatic: bool, target: WorldPos) -> bevy_app::App {
    let mut app = scene_with(false, |catalog, map| {
        map.placements[1].pos = target;
        catalog.units.get_mut(&id("worker")).unwrap().repairer = Some(turts_mod_api::RepairerDef {
            rate_per_second: 20,
            reach: 40.0,
            cost: vec![],
            targets: TargetFilter {
                slots: turts_mod_api::SlotRelation::Any,
            },
            classes: vec![],
            auto_range: automatic.then_some(40.0),
        });
    });
    app.world_mut()
        .resource_mut::<crate::economy::combat::Healths>()
        .0
        .insert(2, 10);
    app
}
fn repair_command(target: u32, queue: crate::QueueMode, seq: u32) -> Command {
    Command {
        slot: SlotId(0),
        seq,
        kind: CommandKind::Order(Box::new(crate::orders::OrderCommand {
            order: id("repair"),
            behaviour: turts_mod_api::Behaviour::Repair,
            target_kind: turts_mod_api::TargetKind::Unit,
            policy: None,
            units: vec![1],
            target: crate::orders::OrderTargetSim::Unit(target),
            queue,
            group: 0,
            allowed_units: vec![],
            queue_depth: 16,
        })),
    }
}

#[test]
fn explicit_and_queued_repair_refuse_hidden_and_absent_targets_identically() {
    let mut app = repair_scene(false, p(24.0, 8.0));
    for (index, queue) in [crate::QueueMode::Replace, crate::QueueMode::Append]
        .into_iter()
        .enumerate()
    {
        for target in [2, 999] {
            let result = apply_command(
                app.world_mut(),
                &repair_command(target, queue, target + index as u32 * 1000),
            );
            assert_eq!(result.outcome, Err(crate::RejectReason::TargetInvalid));
        }
    }
    run(&mut app, 40);
    assert_eq!(
        app.world().resource::<crate::economy::combat::Healths>().0[&2],
        10
    );
    let actor = crate::unit_entity(app.world(), 1).unwrap();
    assert_eq!(
        app.world().get::<crate::Position>(actor).unwrap().0,
        Vec2::new(8.0, 8.0)
    );
    assert!(
        app.world()
            .get::<crate::UnitOrders>(actor)
            .unwrap()
            .current
            .is_none()
    );
}

#[test]
fn queued_repair_rechecks_sight_after_real_movement_before_healing() {
    for modern_move in [false, true] {
        for absent in [false, true] {
            let mut app = repair_scene(false, p(10.0, 10.0));
            if modern_move {
                apply_command(
                    app.world_mut(),
                    &Command {
                        slot: SlotId(0),
                        seq: 1,
                        kind: CommandKind::Order(Box::new(crate::orders::OrderCommand {
                            order: id("move"),
                            behaviour: turts_mod_api::Behaviour::Move,
                            target_kind: turts_mod_api::TargetKind::Point,
                            policy: None,
                            units: vec![1],
                            target: crate::orders::OrderTargetSim::Point(Vec2::new(22.0, 8.0)),
                            queue: crate::QueueMode::Replace,
                            group: 0,
                            allowed_units: vec![],
                            queue_depth: 16,
                        })),
                    },
                )
                .outcome
                .unwrap();
            } else {
                move_to(&mut app, 0, 1, 22.0, 1);
            }
            // Activate the ordinary move before appending. A newly admitted legacy
            // move still has a deferred reservation at command acceptance.
            run(&mut app, 1);
            let actor = crate::unit_entity(app.world(), 1).unwrap();
            assert!(app.world().get::<crate::world::MoveTarget>(actor).is_some());
            assert!(
                basis(app.world(), ObservationPhase::Completed)
                    .unwrap()
                    .unit_permitted(0, 2)
            );
            apply_command(
                app.world_mut(),
                &repair_command(2, crate::QueueMode::Append, 2),
            )
            .outcome
            .unwrap();
            let actor = crate::unit_entity(app.world(), 1).unwrap();
            assert_eq!(
                app.world()
                    .get::<crate::UnitOrders>(actor)
                    .unwrap()
                    .queue
                    .len(),
                1
            );
            assert!(
                app.world().get::<crate::world::MoveTarget>(actor).is_some(),
                "appending repair must preserve the admitted player move"
            );
            if absent {
                let target = crate::unit_entity(app.world(), 2).unwrap();
                app.world_mut().despawn(target);
                app.world_mut()
                    .resource_mut::<crate::economy::combat::Healths>()
                    .0
                    .remove(&2);
            }
            let mut refusal = None;
            for _ in 0..200 {
                run(&mut app, 1);
                let orders = app.world().get::<crate::UnitOrders>(actor).unwrap();
                if let Some(failure) = &orders.last_failure {
                    refusal = Some(failure.reason);
                }
                if !absent {
                    assert_eq!(
                        app.world().resource::<crate::economy::combat::Healths>().0[&2],
                        10,
                        "a repair behind a move must never heal before or after hidden activation"
                    );
                }
            }
            let orders = app.world().get::<crate::UnitOrders>(actor).unwrap();
            assert!(orders.current.is_none() && orders.queue.is_empty());
            assert_eq!(
                refusal,
                Some(crate::RejectReason::TargetInvalid),
                "queued activation must record its generic refusal on a real tick"
            );
            if !absent {
                assert_eq!(
                    app.world().resource::<crate::economy::combat::Healths>().0[&2],
                    10
                );
            }
            assert!(
                app.world()
                    .get::<crate::Position>(actor)
                    .unwrap()
                    .0
                    .distance(Vec2::new(22.0, 8.0))
                    < 0.01
            );
        }
    }
}

#[test]
fn autonomous_repair_heals_visible_targets_and_stops_after_target_leaves_sight() {
    let mut hidden = repair_scene(true, p(24.0, 8.0));
    let mut absent = repair_scene(true, p(24.0, 8.0));
    let target = crate::unit_entity(absent.world(), 2).unwrap();
    absent.world_mut().despawn(target);
    absent
        .world_mut()
        .resource_mut::<crate::economy::combat::Healths>()
        .0
        .remove(&2);
    run(&mut absent, 40);
    run(&mut hidden, 40);
    assert_eq!(
        hidden
            .world()
            .resource::<crate::economy::combat::Healths>()
            .0[&2],
        10
    );
    assert!(crate::economy::repair::status_rows(hidden.world()).is_empty());
    assert_eq!(
        crate::economy::repair::status_rows(hidden.world()),
        crate::economy::repair::status_rows(absent.world())
    );
    assert_eq!(
        basis(hidden.world(), ObservationPhase::Completed)
            .unwrap()
            .knowledge(0),
        basis(absent.world(), ObservationPhase::Completed)
            .unwrap()
            .knowledge(0)
    );
    let mut visible = repair_scene(true, p(10.0, 8.0));
    run(&mut visible, 2);
    assert!(
        visible
            .world()
            .resource::<crate::economy::combat::Healths>()
            .0[&2]
            > 10
    );
    move_to(&mut visible, 1, 2, 40.0, 1);
    run(&mut visible, 220);
    assert!(
        !basis(visible.world(), ObservationPhase::Completed)
            .unwrap()
            .unit_permitted(0, 2)
    );
    let health = visible
        .world()
        .resource::<crate::economy::combat::Healths>()
        .0[&2];
    assert!(
        health < 100,
        "the target must leave sight before repair completes"
    );
    run(&mut visible, 40);
    assert_eq!(
        visible
            .world()
            .resource::<crate::economy::combat::Healths>()
            .0[&2],
        health
    );
    assert!(crate::economy::repair::status_rows(visible.world()).is_empty());
}
