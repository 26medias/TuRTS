//! Construction through the real simulation, server export, and client replica.
#![allow(dead_code)]
use bevy::app::{App, FixedPostUpdate, FixedPreUpdate, FixedUpdate};
use std::collections::BTreeMap;
use turts_mod_api::*;
use turts_sim::economy::construction::{ConstructionLimit, ConstructionState, construct};
use turts_sim::*;
#[path = "../../turts_server/src/export.rs"]
mod export;
#[path = "../../turts_server/src/permissions.rs"]
mod permissions;
fn cid(local: &str) -> ContentId {
    ContentId::parse(&format!("build:{local}")).unwrap()
}
fn tick(app: &mut App) {
    app.world_mut().run_schedule(FixedPreUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedPostUpdate);
}
fn balance(app: &App) -> u64 {
    balance_view(app.world(), SlotId(0), &cid("credits"))
        .unwrap()
        .on_hand
}
fn fixture(policy: PaymentPolicy, mutate: impl FnOnce(&mut ContentCatalog, &mut MapDef)) -> App {
    let mut app = turts_sim::headless_app(20);
    let tower = ResolvedUnit {
        terrain_class: None,
        terrain_cargo: None,
        terrain_foundation: None,
        terrain_tools: vec![],
        vision: None,
        consumes: Vec::new(),
        income: None,
        provides: Vec::new(),

        acquire: None,
        armour: None,
        id: cid("tower"),
        name: "Tower".into(),
        footprint: Footprint::Circle { radius: 1.0 },
        movement: None,
        inventory: None,
        gatherer: None,
        drop_off: None,
        producer: Some(ResolvedProducer {
            when_short: Vec::new(),

            recipes: vec![cid("train")],
            queue_capacity: 2,
            spawn_offsets: vec![WorldPos { x: 3.0, y: 0.0 }],
        }),
        constructor: None,
        repairer: None,
        placement: Some(SitePlacementDef {
            gap: 1.0,
            node_clearance: 1.0,
            in_build_area: true,
            site_health: Ratio {
                numerator: 1,
                denominator: 2,
            },
        }),
        build_area: Some(8.0),
        output_inventory: None,
        extractor: None,
        conveyor: None,
        inserter: None,
        machine: None,
        research: None,
        health: Some(HealthDef { max: 100 }),
        weapon: None,
        removal: None,
    };
    let mut base = tower.clone();
    base.id = cid("base");
    base.name = "Base".into();
    base.producer = None;
    base.constructor = Some(ResolvedConstructor {
        buildings: vec![cid("build_tower")],
        concurrent_sites: 1,
        queue_capacity: 2,
        progress: ConstructProgress::Autonomous,
        when_short: vec![],
    });
    base.build_area = Some(15.0);
    let mut soldier = tower.clone();
    soldier.id = cid("soldier");
    soldier.movement = Some(MovementDef { speed: 3.0 });
    soldier.producer = None;
    soldier.placement = None;
    soldier.build_area = None;
    let refunds = CancelRefunds {
        queued: RefundRatio::one(),
        running: RefundRatio::one(),
        output_blocked: RefundRatio::one(),
        destroyed: RefundRatio {
            numerator: 0,
            denominator: 1,
        },
    };
    let recipe = |id: &str, output: &str, amount| ResolvedProduction {
        id: cid(id),
        name: id.into(),
        output_unit: cid(output),
        duration_ms: 100,
        costs: vec![ResolvedCost {
            resource: cid("credits"),
            amount,
            source: CostSource::SlotBalance,
        }],
        payment: policy,
        cancel_refunds: refunds,
    };
    let mut catalog = ContentCatalog {
        terrain_materials: Default::default(),
        terrain_movement_classes: Default::default(),
        terrain_tools: Default::default(),
        bots: Default::default(),

        combat: None,
        resources: BTreeMap::from([(
            cid("credits"),
            ResourceDef {
                id: "credits".into(),
                name: "Credits".into(),
                unit_name: "credit".into(),
                kind: ResourceKind::Balance,
            },
        )]),
        units: BTreeMap::from([
            (base.id.clone(), base),
            (tower.id.clone(), tower),
            (soldier.id.clone(), soldier),
        ]),
        production: BTreeMap::from([
            (cid("build_tower"), recipe("build_tower", "tower", 100)),
            (cid("train"), recipe("train", "soldier", 20)),
        ]),
        nodes: BTreeMap::new(),
        recipes: BTreeMap::new(),
        technologies: BTreeMap::new(),
        orders: BTreeMap::new(),
        order_queue_depths: BTreeMap::new(),
        source_files: BTreeMap::new(),
    };
    let mut map = MapDef {
        terrain: None,
        visibility: None,
        environment: None,
        ground: None,
        id: "field".into(),
        size: MapSize { x: 64.0, y: 64.0 },
        placements: vec![Placement {
            unit: "build:base".into(),
            owner_slot: 0,
            pos: WorldPos { x: 10.0, y: 10.0 },
            contents: vec![],
            facing: None,
            start_order: None,
        }],
        nodes: vec![],
        slot_economy: vec![SlotEconomyInit {
            slot: 0,
            balances: vec![BalanceAmount {
                balance: "build:credits".into(),
                amount: 500,
            }],
        }],
        belts: vec![],
    };
    mutate(&mut catalog, &mut map);
    let staged = turts_sim::stage_package(
        app.world(),
        catalog,
        SourceDef {
            file: "maps/field.ron".into(),
            value: map,
        },
        20,
    )
    .unwrap();
    turts_sim::install_package(app.world_mut(), staged);
    app.insert_resource(ConstructionLimit(2));
    app
}

#[test]
fn held_construction_snapshot_is_accepted_by_the_real_replica() {
    use turts_client::{
        content::ClientContent,
        replica::{Replica, ReplicaContext, SnapshotOutcome},
    };
    let mut app = fixture(PaymentPolicy::ChargeOnQueue, |_, _| {});
    construct(
        app.world_mut(),
        SlotId(0),
        1,
        cid("build_tower"),
        Vec2::new(16.0, 10.0),
    )
    .unwrap();
    construct(
        app.world_mut(),
        SlotId(0),
        1,
        cid("build_tower"),
        Vec2::new(20.0, 10.0),
    )
    .unwrap();
    tick(&mut app);
    let snapshot = export::snapshot_for(completed_world(app.world()), SlotId(0));
    assert_eq!(
        snapshot.records.construction_queues[0].orders[0].content,
        "build:build_tower"
    );
    let catalog = app
        .world()
        .resource::<turts_sim::bootstrap::InstalledCatalog>()
        .0
        .clone();
    let mut replica = Replica::new(20);
    replica.set_context(ReplicaContext::new(
        ClientContent {
            catalog,
            ..Default::default()
        },
        0,
    ));
    let outcome = replica.apply_snapshot(&snapshot, bevy::platform::time::Instant::now());
    assert!(
        matches!(outcome, SnapshotOutcome::Applied { .. }),
        "held-queue snapshot was rejected: {outcome:?}"
    );
}

#[test]
fn an_unfinished_convert_dropoff_cannot_receive_cargo() {
    let mut app = fixture(PaymentPolicy::ChargeOnQueue, |catalog, map| {
        catalog
            .production
            .get_mut(&cid("build_tower"))
            .unwrap()
            .duration_ms = 10_000;
        catalog.units.get_mut(&cid("tower")).unwrap().drop_off = Some(ResolvedDropOff {
            rules: vec![ResolvedDropOffRule {
                item: cid("ore"),
                effect: ResolvedDropOffEffect::Convert {
                    balance: cid("credits"),
                    per_item: 1,
                },
            }],
        });
        let mut worker = catalog.units[&cid("soldier")].clone();
        worker.id = cid("worker");
        worker.inventory = Some(ResolvedInventory {
            capacity: 5,
            accepts: vec![cid("ore")],
        });
        worker.gatherer = Some(ResolvedGatherer {
            items: vec![cid("ore")],
            gather_per_second: 20,
            unload_per_second: 20,
            interaction_range: 1.0,
        });
        catalog.units.insert(worker.id.clone(), worker);
        catalog.resources.insert(
            cid("ore"),
            ResourceDef {
                id: "ore".into(),
                name: "Ore".into(),
                unit_name: "ore".into(),
                kind: ResourceKind::Item,
            },
        );
        catalog.nodes.insert(
            cid("node"),
            ResolvedNode {
                id: cid("node"),
                name: "Ore node".into(),
                item: cid("ore"),
                radius: 1.0,
            },
        );
        map.nodes.push(NodePlacement {
            node: "build:node".into(),
            pos: WorldPos { x: 40.0, y: 10.0 },
            remaining: 100,
        });
        map.placements.push(Placement {
            unit: "build:worker".into(),
            owner_slot: 0,
            pos: WorldPos { x: 18.5, y: 10.0 },
            contents: vec![ItemStack {
                item: "build:ore".into(),
                amount: 5,
            }],
            facing: None,
            start_order: None,
        });
    });
    construct(
        app.world_mut(),
        SlotId(0),
        1,
        cid("build_tower"),
        Vec2::new(16.0, 10.0),
    )
    .unwrap();
    assign_gather(app.world_mut(), SlotId(0), 2, NodeId(1)).unwrap();
    for _ in 0..10 {
        tick(&mut app);
    }
    assert_eq!(app.world().resource::<ConstructionState>().sites.len(), 1);
    assert_eq!(
        balance(&app),
        400,
        "unfinished drop-off converted worker cargo"
    );
}

#[test]
fn a_completed_building_with_a_queued_job_can_be_destroyed() {
    for policy in [
        PaymentPolicy::ChargeOnQueue,
        PaymentPolicy::ReserveThenCharge,
    ] {
        let mut app = fixture(policy, |catalog, map| {
            catalog
                .production
                .get_mut(&cid("train"))
                .unwrap()
                .duration_ms = 100_000;
            let mut enemy = catalog.units[&cid("soldier")].clone();
            enemy.id = cid("enemy");
            enemy.weapon = Some(ResolvedWeapon {
                when_short: Vec::new(),

                bonus: Vec::new(),
                damage_type: None,
                range: 10.0,
                range_mode: turts_mod_api::WeaponRangeMode::Centre,
                cooldown_ms: 50,
                damage: 200,
                ammo: None,
                targets: TargetFilter {
                    slots: SlotRelation::Other,
                },
            });
            catalog.units.insert(enemy.id.clone(), enemy);
            // Enemy starts far away. Move it into range only after the building starts training.
            map.placements.push(Placement {
                unit: "build:enemy".into(),
                owner_slot: 1,
                pos: WorldPos { x: 50.0, y: 10.0 },
                contents: vec![],
                facing: None,
                start_order: None,
            });
        });
        construct(
            app.world_mut(),
            SlotId(0),
            1,
            cid("build_tower"),
            Vec2::new(16.0, 10.0),
        )
        .unwrap();
        tick(&mut app);
        tick(&mut app);
        // Map ids 1 and 2 are base and enemy. The constructed tower is 3.
        turts_sim::economy::production::queue_production(
            app.world_mut(),
            SlotId(0),
            3,
            cid("train"),
        )
        .unwrap();
        turts_sim::economy::production::queue_production(
            app.world_mut(),
            SlotId(0),
            3,
            cid("train"),
        )
        .unwrap();
        let entity = turts_sim::unit_index::unit_entity(app.world(), 2).unwrap();
        app.world_mut()
            .get_mut::<turts_sim::world::Position>(entity)
            .unwrap()
            .0 = Vec2::new(20.0, 10.0);
        for _ in 0..5 {
            tick(&mut app);
        }
        assert!(
            !completed_world(app.world())
                .units
                .iter()
                .any(|unit| unit.unit_id == 3),
            "training building survived repeated lethal fire"
        );
        assert_eq!(
            balance(&app),
            if policy == PaymentPolicy::ChargeOnQueue {
                360
            } else {
                380
            }
        );
        assert_eq!(
            balance_view(app.world(), SlotId(0), &cid("credits"))
                .unwrap()
                .reserved,
            0
        );
    }
}

#[test]
fn construction_lifecycle_deltas_and_rejoin_work_for_both_owners() {
    use turts_client::{
        content::ClientContent,
        replica::{DeltaOutcome, Replica, ReplicaContext, SnapshotOutcome},
    };
    for duration in [50, 100, 500] {
        let mut app = fixture(PaymentPolicy::ChargeOnQueue, |catalog, _| {
            catalog
                .production
                .get_mut(&cid("build_tower"))
                .unwrap()
                .duration_ms = duration;
        });
        initialize_completed_world(app.world_mut());
        let catalog = app
            .world()
            .resource::<turts_sim::bootstrap::InstalledCatalog>()
            .0
            .clone();
        let mut replicas = Vec::new();
        for slot in [0, 1] {
            let mut replica = Replica::new(20);
            replica.set_context(ReplicaContext::new(
                ClientContent {
                    catalog: catalog.clone(),
                    ..Default::default()
                },
                slot,
            ));
            let snapshot = export::snapshot_for(completed_world(app.world()), SlotId(slot));
            assert!(matches!(
                replica.apply_snapshot(&snapshot, bevy::platform::time::Instant::now()),
                SnapshotOutcome::Applied { .. }
            ));
            replicas.push(replica);
        }
        construct(
            app.world_mut(),
            SlotId(0),
            1,
            cid("build_tower"),
            Vec2::new(16.0, 10.0),
        )
        .unwrap();
        for step in 0..11 {
            tick(&mut app);
            for slot in [0, 1] {
                let view = completed_world(app.world());
                let delta = export::delta_for(committed_tick(app.world()), view, SlotId(slot));
                let outcome = replicas[slot as usize]
                    .apply_delta(&delta, bevy::platform::time::Instant::now());
                assert!(
                    matches!(outcome, DeltaOutcome::Applied),
                    "duration={duration}, step={step}, slot={slot}, delta rejected: {outcome:?}; {delta:?}"
                );
                let mut rejoined = Replica::new(20);
                rejoined.set_context(ReplicaContext::new(
                    ClientContent {
                        catalog: catalog.clone(),
                        ..Default::default()
                    },
                    slot,
                ));
                let snapshot = export::snapshot_for(view, SlotId(slot));
                assert!(
                    matches!(
                        rejoined.apply_snapshot(&snapshot, bevy::platform::time::Instant::now()),
                        SnapshotOutcome::Applied { .. }
                    ),
                    "rejoin failed duration={duration} step={step} slot={slot}"
                );
            }
        }
    }
}

fn construction_command(
    behaviour: Behaviour,
    target_kind: TargetKind,
    target: OrderTargetSim,
) -> Command {
    Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::Order(Box::new(OrderCommand {
            order: cid(if behaviour == Behaviour::Stop {
                "stop"
            } else {
                "construct"
            }),
            behaviour,
            target_kind,
            policy: None,
            units: vec![1],
            target,
            queue: QueueMode::Replace,
            group: 0,
            allowed_units: vec![],
            queue_depth: 16,
        })),
    }
}

fn two_sites_and_one_held(duration: u64) -> App {
    let mut app = fixture(PaymentPolicy::ChargeOnQueue, |catalog, _| {
        catalog
            .production
            .get_mut(&cid("build_tower"))
            .unwrap()
            .duration_ms = duration;
        let base = catalog.units.get_mut(&cid("base")).unwrap();
        base.constructor.as_mut().unwrap().concurrent_sites = 2;
        base.removal = Some(RemovalDef {
            by_command: true,
            contents: ContentsPolicy::Destroy,
        });
        for (local, behaviour, target) in [
            (
                "construct",
                Behaviour::Construct,
                TargetKind::ContentAtPoint,
            ),
            ("stop", Behaviour::Stop, TargetKind::None),
        ] {
            catalog.orders.insert(
                cid(local),
                OrderDef {
                    id: local.into(),
                    name: local.into(),
                    behaviour,
                    target,
                    fire: None,
                    engage: None,
                    icon: String::new(),
                    hotkey: String::new(),
                    cursor: None,
                    card: None,
                    units: vec![cid("base").to_string()],
                    description: String::new(),
                },
            );
        }
    });
    initialize_completed_world(app.world_mut());
    for x in [16.0, 20.0, 24.0] {
        app.world_mut()
            .resource_mut::<CommandQueue>()
            .0
            .push_back(construction_command(
                Behaviour::Construct,
                TargetKind::ContentAtPoint,
                OrderTargetSim::ContentAtPoint {
                    content: cid("build_tower"),
                    point: Vec2::new(x, 10.0),
                },
            ));
    }
    tick(&mut app);
    assert_eq!(
        app.world().resource::<ConstructionState>().held[&1].len(),
        1
    );
    app
}

fn replica_with_held_snapshot(app: &App) -> turts_client::replica::Replica {
    use turts_client::{
        content::ClientContent,
        replica::{Replica, ReplicaContext, SnapshotOutcome},
    };
    let mut replica = Replica::new(20);
    replica.set_context(ReplicaContext::new(
        ClientContent {
            catalog: app
                .world()
                .resource::<turts_sim::bootstrap::InstalledCatalog>()
                .0
                .clone(),
            ..Default::default()
        },
        0,
    ));
    let snapshot = export::snapshot_for(completed_world(app.world()), SlotId(0));
    assert!(matches!(
        replica.apply_snapshot(&snapshot, bevy::platform::time::Instant::now()),
        SnapshotOutcome::Applied { .. }
    ));
    assert_eq!(replica.construction_queue(1).len(), 1);
    replica
}

fn apply_construction_delta(
    app: &App,
    replica: &mut turts_client::replica::Replica,
) -> turts_net_protocol::messages::WorldDelta {
    use turts_client::replica::DeltaOutcome;
    let delta = export::delta_for(
        committed_tick(app.world()),
        completed_world(app.world()),
        SlotId(0),
    );
    assert_eq!(
        replica.apply_delta(&delta, bevy::platform::time::Instant::now()),
        DeltaOutcome::Applied
    );
    delta
}

#[test]
fn accepted_static_constructor_stop_publishes_empty_held_queue_once_and_keeps_sites() {
    let mut app = two_sites_and_one_held(100_000);
    let mut replica = replica_with_held_snapshot(&app);
    let sites = app
        .world()
        .resource::<ConstructionState>()
        .sites
        .keys()
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(sites.len(), 2);
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(construction_command(
            Behaviour::Stop,
            TargetKind::None,
            OrderTargetSim::None,
        ));
    tick(&mut app);
    let result = &app.world().resource::<CommandResults>().0[0];
    assert!(result.outcome.is_ok(), "{result:?}");
    assert_eq!(result.units.accepted, vec![1]);
    assert!(
        !app.world()
            .resource::<ConstructionState>()
            .held
            .contains_key(&1),
        "authority clears the held request"
    );
    assert_eq!(
        app.world()
            .resource::<ConstructionState>()
            .sites
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        sites
    );
    let delta = apply_construction_delta(&app, &mut replica);
    assert_eq!(delta.records.construction_queues.len(), 1);
    assert_eq!(delta.records.construction_queues[0].constructor.0, 1);
    assert!(delta.records.construction_queues[0].orders.is_empty());
    assert!(
        replica.construction_queue(1).is_empty(),
        "delta clears the old client list"
    );
    assert!(
        export::snapshot_for(completed_world(app.world()), SlotId(0))
            .records
            .construction_queues
            .is_empty(),
        "fresh baseline has no stale request"
    );
    assert!(
        export::delta_for(
            committed_tick(app.world()),
            completed_world(app.world()),
            SlotId(1)
        )
        .records
        .construction_queues
        .is_empty(),
        "foreign slot receives no private queue row"
    );
    tick(&mut app);
    let next = apply_construction_delta(&app, &mut replica);
    assert!(
        next.records.construction_queues.is_empty(),
        "empty replacement is sent once"
    );
}

#[test]
fn automatic_placement_publishes_the_last_held_request_clear_once() {
    let mut app = two_sites_and_one_held(50);
    let mut replica = replica_with_held_snapshot(&app);
    assert!(app.world().resource::<ConstructionState>().sites.is_empty());
    tick(&mut app);
    let delta = apply_construction_delta(&app, &mut replica);
    assert_eq!(delta.records.construction_queues.len(), 1);
    assert!(delta.records.construction_queues[0].orders.is_empty());
    assert!(replica.construction_queue(1).is_empty());
    assert_eq!(
        completed_world(app.world()).units.len(),
        4,
        "all three held/placed requests became buildings"
    );
    tick(&mut app);
    assert!(
        apply_construction_delta(&app, &mut replica)
            .records
            .construction_queues
            .is_empty()
    );
}

#[test]
fn constructor_removal_clears_held_queue_only_through_removed_record() {
    let mut app = two_sites_and_one_held(100_000);
    let mut replica = replica_with_held_snapshot(&app);
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(0),
            seq: 9,
            kind: CommandKind::Remove { unit_id: 1 },
        });
    tick(&mut app);
    assert!(
        app.world().resource::<CommandResults>().0[0]
            .outcome
            .is_ok()
    );
    let delta = apply_construction_delta(&app, &mut replica);
    assert!(delta.removed.iter().any(|id| id.0 == 1));
    assert!(
        delta
            .records
            .construction_queues
            .iter()
            .all(|row| row.constructor.0 != 1)
    );
    assert!(replica.construction_queue(1).is_empty());
    assert_eq!(app.world().resource::<ConstructionState>().sites.len(), 2);
}
