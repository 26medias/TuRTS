//! Protocol 4 behavior envelopes must resolve through package definitions into simulation orders.

#[path = "../src/economylog.rs"]
#[allow(dead_code)]
mod economylog;
#[path = "../src/orders.rs"]
#[allow(dead_code)]
mod orders;

use std::{collections::BTreeMap, path::PathBuf};

use turts_mod_api::{
    Behaviour, ContentCatalog, ContentId, EngagePolicy, Footprint, HealthDef, MapDef, MapSize,
    MovementDef, OrderDef, Placement, RemovalDef, ResolvedAmmo, ResolvedInventory, ResolvedUnit,
    ResolvedWeapon, ResourceDef, ResourceKind, SlotRelation, SourceDef, TargetFilter, TargetKind,
    WorldPos,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{OrderEnvelope, OrderTarget, PlayerCommand, Pos, QueueMode, UnitId};
use turts_pack::load_package;
use turts_sim::{
    AuditTrail, CommandKind, CommandQueue, MoveTarget, OrderTargetSim, Owner, Position, SlotId,
    UnitOrders, UnitTypes, apply_command, audit_state, headless_app, install_package,
    stage_package,
};

fn muster() -> PackageArchive {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
    PackageArchive::parse(&pack_dir(&path).expect("pack muster package"))
        .expect("parse muster archive")
}

fn target_for(kind: TargetKind) -> OrderTarget {
    match kind {
        TargetKind::None => OrderTarget::None,
        TargetKind::Point => OrderTarget::Point(Pos { x: 40.0, y: 30.0 }),
        TargetKind::Unit | TargetKind::PointOrUnit => OrderTarget::Unit(UnitId(1)),
        TargetKind::Node => OrderTarget::Node(turts_net_protocol::NodeId(1)),
        TargetKind::Job => OrderTarget::Job(turts_net_protocol::JobId(1)),
        TargetKind::Content => OrderTarget::Content("muster:trooper".into()),
        // API 0.2.4: no 0.2.3 package has an order with this target.
        TargetKind::ContentAtPoint => unreachable!("no API 0.2.3 order targets a content point"),
    }
}

fn sim_target_for(kind: TargetKind) -> OrderTargetSim {
    match kind {
        TargetKind::None => OrderTargetSim::None,
        TargetKind::Point => OrderTargetSim::Point(bevy_math::Vec2::new(40.0, 30.0)),
        TargetKind::Unit | TargetKind::PointOrUnit => OrderTargetSim::Unit(1),
        TargetKind::Node => OrderTargetSim::Node(turts_sim::NodeId(1)),
        TargetKind::Job => OrderTargetSim::Job(turts_sim::JobId(1)),
        TargetKind::Content => {
            OrderTargetSim::Content(turts_mod_api::ContentId::parse("muster:trooper").unwrap())
        }
        // API 0.2.4: no 0.2.3 package has an order with this target.
        TargetKind::ContentAtPoint => unreachable!("no API 0.2.3 order targets a content point"),
    }
}

fn audit_catalog() -> ContentCatalog {
    let units = ["audit:shooter", "audit:enemy"]
        .into_iter()
        .map(|id| {
            let content_id = ContentId::parse(id).unwrap();
            let shooter = id == "audit:shooter";
            let unit = ResolvedUnit {
                constructor: None,
                placement: None,
                build_area: None,
                id: content_id.clone(),
                name: id.into(),
                footprint: Footprint::Circle { radius: 0.25 },
                movement: shooter.then_some(MovementDef { speed: 10.0 }),
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: None,
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: Some(HealthDef { max: 60 }),
                weapon: shooter.then_some(ResolvedWeapon {
                    range: 3.0,
                    cooldown_ms: 0,
                    damage: 1,
                    ammo: None,
                    targets: TargetFilter {
                        slots: SlotRelation::Other,
                    },
                }),
                removal: Some(RemovalDef {
                    by_command: true,
                    contents: turts_mod_api::ContentsPolicy::Destroy,
                }),
            };
            (content_id, unit)
        })
        .collect();
    let order = |id: &str, behaviour, target, engage| {
        let definition = OrderDef {
            id: id.into(),
            name: id.into(),
            behaviour,
            target,
            fire: None,
            engage,
            icon: String::new(),
            hotkey: String::new(),
            cursor: None,
            card: None,
            units: vec!["audit:shooter".into()],
            description: id.into(),
        };
        (
            ContentId::parse(&format!("audit:{id}")).unwrap(),
            definition,
        )
    };
    ContentCatalog {
        resources: BTreeMap::new(),
        units,
        nodes: BTreeMap::new(),
        production: BTreeMap::new(),
        recipes: BTreeMap::new(),
        technologies: BTreeMap::new(),
        orders: [
            order("move", Behaviour::Move, TargetKind::Point, None),
            order(
                "rush",
                Behaviour::Attack,
                TargetKind::Point,
                Some(EngagePolicy::StopAndFire),
            ),
        ]
        .into_iter()
        .collect(),
        order_queue_depths: BTreeMap::new(),
        source_files: BTreeMap::new(),
    }
}

fn audit_fixture() -> (bevy_app::App, ContentCatalog) {
    audit_fixture_at_enemy(audit_catalog(), WorldPos { x: 35.0, y: 5.0 })
}

fn audit_fixture_at_enemy(
    catalog: ContentCatalog,
    enemy_position: WorldPos,
) -> (bevy_app::App, ContentCatalog) {
    let map = MapDef {
        id: "audit:field".into(),
        size: MapSize { x: 40.0, y: 40.0 },
        placements: vec![
            Placement {
                unit: "audit:shooter".into(),
                owner_slot: 0,
                pos: WorldPos { x: 5.0, y: 5.0 },
                contents: vec![],
                facing: None,
                start_order: None,
            },
            Placement {
                unit: "audit:shooter".into(),
                owner_slot: 0,
                pos: WorldPos { x: 5.0, y: 7.0 },
                contents: vec![],
                facing: None,
                start_order: None,
            },
            Placement {
                unit: "audit:enemy".into(),
                owner_slot: 1,
                pos: enemy_position,
                contents: vec![],
                facing: None,
                start_order: None,
            },
        ],
        nodes: vec![],
        slot_economy: vec![],
        belts: vec![],
    };
    let mut app = headless_app(20);
    let staged = stage_package(
        app.world(),
        catalog.clone(),
        SourceDef {
            file: "maps/field.ron".into(),
            value: map,
        },
        20,
    )
    .expect("stage audit fixture");
    install_package(app.world_mut(), staged);
    (app, catalog)
}

fn drain_audit(app: &mut bevy_app::App, writer: &mut economylog::EconomyLogWriter<Vec<u8>>) {
    let mut trail = app.world_mut().resource_mut::<AuditTrail>();
    for command in trail.drain_commands() {
        writer
            .write_command(&trail, command)
            .expect("write command audit");
    }
    for operation in trail.drain_operations() {
        writer
            .write_operation(&trail, operation)
            .expect("write operation audit");
    }
    for tick in trail.drain_ticks() {
        writer.write_tick(&trail, tick).expect("write tick audit");
    }
}

#[test]
fn every_behaviour_has_an_envelope() {
    let archive = muster();
    let catalog = archive.content().expect("resolve muster catalog");
    let definitions = archive.orders().expect("read muster orders");
    let mut behaviours = Vec::new();
    for (_, definition) in &definitions {
        let category = match definition.behaviour {
            Behaviour::Move => 0,
            Behaviour::Stop => 1,
            Behaviour::Hold => 2,
            Behaviour::Attack => 3,
            Behaviour::Gather => 4,
            Behaviour::Produce => 5,
            Behaviour::CancelProduction => 6,
            Behaviour::Remove => 7,
            Behaviour::SetEnabled(_) => 8,
            // API 0.2.4: `muster` has no order of the new behaviours.
            Behaviour::Construct
            | Behaviour::CancelConstruction
            | Behaviour::SetRally
            | Behaviour::Repair
            | Behaviour::Surrender => unreachable!("muster has no API 0.2.4 order"),
        };
        if !behaviours.contains(&category) {
            behaviours.push(category);
        }
    }
    assert_eq!(behaviours.len(), 9, "muster must cover all nine behaviors");

    for (_, definition) in definitions {
        let expected_target = sim_target_for(definition.target);
        let order = format!("{}:{}", archive.manifest.id, definition.id);
        let command = PlayerCommand {
            seq: 7,
            order: OrderEnvelope {
                order: order.clone(),
                units: vec![UnitId(1)],
                target: target_for(definition.target),
                queue: QueueMode::Replace,
                group: 0,
            },
        };
        let converted = orders::to_sim(SlotId(0), &command, &catalog)
            .unwrap_or_else(|error| panic!("{order} must resolve: {error:?}"));
        assert_eq!(converted.len(), 1, "{order} must produce one sim envelope");
        let CommandKind::Order(resolved) = &converted[0].kind else {
            panic!("{order} must remain an order command");
        };
        assert_eq!(resolved.behaviour, definition.behaviour, "{order} behavior");
        assert_eq!(
            resolved.target, expected_target,
            "{order} target adapter must preserve {:?}",
            definition.target,
        );
    }
}

#[test]
fn rush_order_attacks() {
    let archive = muster();
    let catalog = archive.content().expect("resolve muster catalog");
    let mut app = headless_app(20);
    load_package(app.world_mut(), &archive).expect("load muster in simulation");

    let unit_types = app.world().resource::<UnitTypes>().clone();
    let mut attacker = None;
    for entity in app.world().iter_entities() {
        let (Some(id), Some(owner), Some(kind)) = (
            entity.get::<turts_sim::UnitId>(),
            entity.get::<Owner>(),
            entity.get::<turts_sim::UnitType>(),
        ) else {
            continue;
        };
        let content = &unit_types
            .get(kind.0)
            .expect("registered unit type")
            .content_id;
        if owner.0 == SlotId(0) && content == "muster:trooper" {
            attacker.get_or_insert(id.0);
        }
    }
    let attacker = attacker.expect("muster trooper");
    let command = PlayerCommand {
        seq: 1,
        order: OrderEnvelope {
            order: "muster:rush".into(),
            units: vec![UnitId(attacker)],
            target: OrderTarget::Point(Pos { x: 54.0, y: 34.0 }),
            queue: QueueMode::Replace,
            group: 0,
        },
    };
    let converted = orders::to_sim(SlotId(0), &command, &catalog)
        .expect("server resolves rush through the actual package definition");
    let CommandKind::Order(resolved) = &converted[0].kind else {
        panic!("rush must resolve as an Order command");
    };
    assert_eq!(
        resolved.policy.as_ref().and_then(|policy| policy.engage),
        Some(turts_mod_api::EngagePolicy::StopAndFire),
        "the accepted order must retain its authored engage policy",
    );
    let result = apply_command(app.world_mut(), &converted[0]);
    assert_eq!(
        result.units.accepted,
        vec![attacker],
        "muster:rush must be accepted by simulation as an Attack order; got {:?}",
        result.units.refused,
    );
}

#[test]
fn captured_catalog_keeps_authored_fire_and_engage_policy() {
    let archive = muster();
    let mut app = headless_app(20);
    load_package(app.world_mut(), &archive).expect("load muster in simulation");
    let audit = turts_sim::audit_state(app.world_mut());
    let definitions = &audit.catalog.expect("catalog snapshot").order_definitions;
    let move_order = definitions
        .iter()
        .find(|definition| definition.id.to_string() == "muster:order_move")
        .expect("move order policy snapshot");
    assert_eq!(move_order.fire, Some(turts_mod_api::FirePolicy::Never));
    let rush = definitions
        .iter()
        .find(|definition| definition.id.to_string() == "muster:rush")
        .expect("rush order policy snapshot");
    assert_eq!(rush.engage, Some(turts_mod_api::EngagePolicy::StopAndFire));
}

#[test]
fn queued_attack_activates_against_its_captured_policy() {
    let archive = muster();
    let catalog = archive.content().expect("resolve muster catalog");
    let mut app = headless_app(20);
    load_package(app.world_mut(), &archive).expect("load muster in simulation");
    let unit_types = app.world().resource::<UnitTypes>().clone();
    let attacker = app
        .world()
        .iter_entities()
        .find_map(|entity| {
            let (Some(id), Some(owner), Some(kind)) = (
                entity.get::<turts_sim::UnitId>(),
                entity.get::<Owner>(),
                entity.get::<turts_sim::UnitType>(),
            ) else {
                return None;
            };
            (owner.0 == SlotId(0)
                && unit_types
                    .get(kind.0)
                    .is_some_and(|unit| unit.content_id == "muster:trooper"))
            .then_some(id.0)
        })
        .expect("muster trooper");
    let actor = app
        .world()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<turts_sim::UnitId>()
                .is_some_and(|id| id.0 == attacker)
        })
        .unwrap()
        .id();

    let move_command = PlayerCommand {
        seq: 1,
        order: OrderEnvelope {
            order: "muster:order_move".into(),
            units: vec![UnitId(attacker)],
            target: OrderTarget::Point(Pos { x: 29.0, y: 18.0 }),
            queue: QueueMode::Replace,
            group: 1,
        },
    };
    let resolved_move = orders::to_sim(SlotId(0), &move_command, &catalog).unwrap();
    assert!(
        apply_command(app.world_mut(), &resolved_move[0])
            .units
            .refused
            .is_empty()
    );
    app.update();

    let rush_command = PlayerCommand {
        seq: 2,
        order: OrderEnvelope {
            order: "muster:rush".into(),
            units: vec![UnitId(attacker)],
            target: OrderTarget::Point(Pos { x: 54.0, y: 34.0 }),
            queue: QueueMode::Append,
            group: 1,
        },
    };
    let resolved_rush = orders::to_sim(SlotId(0), &rush_command, &catalog).unwrap();
    assert!(
        apply_command(app.world_mut(), &resolved_rush[0])
            .units
            .refused
            .is_empty()
    );
    let before = app.world().entity(actor).get::<UnitOrders>().unwrap();
    assert_eq!(before.queue.len(), 1, "rush waits behind the active move");
    assert_eq!(before.queue[0].order.to_string(), "muster:rush");

    let movement_slot = app.world().entity(actor).get::<MoveTarget>().unwrap().0;
    let mut actor_mut = app.world_mut().entity_mut(actor);
    actor_mut.insert(Position(movement_slot));
    actor_mut.remove::<MoveTarget>();
    app.update();

    let after = app.world().entity(actor).get::<UnitOrders>().unwrap();
    assert_eq!(
        after.current.as_ref().unwrap().order.to_string(),
        "muster:rush"
    );
    assert!(after.queue.is_empty());
    assert!(app.world().entity(actor).get::<MoveTarget>().is_some());
    let audit = turts_sim::audit_state(app.world_mut());
    let rush = audit
        .catalog
        .unwrap()
        .order_definitions
        .into_iter()
        .find(|definition| definition.id.to_string() == "muster:rush")
        .unwrap();
    assert_eq!(rush.engage, Some(turts_mod_api::EngagePolicy::StopAndFire));

    let replacement = PlayerCommand {
        seq: 3,
        order: OrderEnvelope {
            order: "muster:order_move".into(),
            units: vec![UnitId(attacker)],
            target: OrderTarget::Point(Pos { x: 56.0, y: 35.0 }),
            queue: QueueMode::Replace,
            group: 0,
        },
    };
    let resolved_replacement = orders::to_sim(SlotId(0), &replacement, &catalog).unwrap();
    assert!(
        apply_command(app.world_mut(), &resolved_replacement[0])
            .units
            .refused
            .is_empty()
    );
    let after_replace = app.world().entity(actor).get::<UnitOrders>().unwrap();
    assert!(after_replace.current.is_none());
    assert!(
        after_replace.queue.is_empty(),
        "replacement interrupts queued pursuit"
    );
    assert_eq!(
        app.world()
            .resource::<turts_sim::orders::PlainMoveOrderIdentities>()
            .0[&attacker]
            .order
            .to_string(),
        "muster:order_move",
        "the legacy plain-move adapter retains the authored fire policy identity",
    );
}

#[test]
fn queued_group_attack_spreads_and_strictly_replays() {
    let (mut app, catalog) = audit_fixture();
    let mut members = app
        .world()
        .iter_entities()
        .filter_map(|entity| {
            let (Some(id), Some(owner), Some(kind)) = (
                entity.get::<turts_sim::UnitId>(),
                entity.get::<Owner>(),
                entity.get::<turts_sim::UnitType>(),
            ) else {
                return None;
            };
            (owner.0 == SlotId(0)
                && catalog
                    .units
                    .contains_key(&ContentId::parse("audit:shooter").unwrap())
                && app
                    .world()
                    .resource::<UnitTypes>()
                    .get(kind.0)
                    .is_some_and(|unit| unit.content_id == "audit:shooter"))
            .then_some(id.0)
        })
        .collect::<Vec<_>>();
    members.sort_unstable();
    members.truncate(2);
    assert_eq!(members.len(), 2);

    app.world_mut().resource_mut::<AuditTrail>().enable();
    let initial = audit_state(app.world_mut());
    let mut writer = economylog::EconomyLogWriter::new(Vec::new());
    {
        let trail = app.world().resource::<AuditTrail>();
        writer
            .write_initial(trail, initial)
            .expect("write initial audit snapshot");
    }

    for (seq, id, target, queue, group) in [
        (
            1,
            "audit:move",
            OrderTarget::Point(Pos { x: 6.0, y: 6.0 }),
            QueueMode::Replace,
            11,
        ),
        (
            2,
            "audit:rush",
            OrderTarget::Point(Pos { x: 20.0, y: 5.0 }),
            QueueMode::Append,
            12,
        ),
    ] {
        let command = PlayerCommand {
            seq,
            order: OrderEnvelope {
                order: id.into(),
                units: members.iter().copied().map(UnitId).collect(),
                target,
                queue,
                group,
            },
        };
        let sim = orders::to_sim(SlotId(0), &command, &catalog)
            .unwrap_or_else(|error| panic!("{id} resolves: {error:?}"));
        assert_eq!(sim.len(), 1);
        app.world_mut()
            .resource_mut::<CommandQueue>()
            .0
            .push_back(sim.into_iter().next().unwrap());
    }

    for _ in 0..80 {
        app.update();
        let mut trail = app.world_mut().resource_mut::<AuditTrail>();
        for command in trail.drain_commands() {
            writer
                .write_command(&trail, command)
                .expect("write command audit");
        }
        for operation in trail.drain_operations() {
            writer
                .write_operation(&trail, operation)
                .expect("write operation audit");
        }
        for tick in trail.drain_ticks() {
            writer.write_tick(&trail, tick).expect("write tick audit");
        }
    }
    let records = writer
        .into_inner()
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).expect("parse audit log record"))
        .collect::<Vec<turts_audit::EconomyLogRecord>>();
    assert!(
        turts_audit::validate_records(&records).is_ok(),
        "queued Point Attack shared-spread operations strictly replay: {:?}",
        turts_audit::validate_records(&records)
    );

    // The outcome from the order log, with no help from the replay validator. Each member
    // first has the order as its current order at some completed tick. At that tick the unit
    // has a destination (`move_target`). The Move goes to the order point (6,6) and the queued
    // Attack goes to its order point (20,5). The group spread puts each member on its own
    // slot of a hex pool around the order point. The pool holds `members + max(6,
    // ceil(members/4))` slots, which is 8 slots for 2 members. So every slot is within two
    // rings of the order point (`crates/turts_sim/src/movement.rs:274-276`: spacing is
    // `2 * radius + 0.1`, and the slot count; `:293`: slot zero is the order point).
    let Footprint::Circle { radius } =
        catalog.units[&ContentId::parse("audit:shooter").unwrap()].footprint;
    let spread_reach = 2.0 * (2.0 * radius + 0.1);
    for (order, point) in [
        ("audit:move", bevy_math::Vec2::new(6.0, 6.0)),
        ("audit:rush", bevy_math::Vec2::new(20.0, 5.0)),
    ] {
        let destinations = first_logged_destinations(&records, &members, order);
        assert_eq!(
            destinations.len(),
            members.len(),
            "{order} gives every member a destination: {destinations:?}"
        );
        for (unit, destination) in &destinations {
            assert!(
                destination.distance(point) <= spread_reach,
                "{order} sends unit {unit} to {destination:?}, more than {spread_reach} from \
                 the order point {point:?}"
            );
        }
        // Queued members can activate on different ticks. Their plans may share a free
        // point; physical arrivals decide who keeps it. Check the actual final bodies.
    }
    let final_units = &turts_sim::completed_world(app.world()).units;
    let positions = members
        .iter()
        .map(|id| {
            final_units
                .iter()
                .find(|unit| unit.unit_id == *id)
                .unwrap()
                .pos
        })
        .collect::<Vec<_>>();
    assert!(
        positions[0].distance(positions[1]) >= 2.0 * radius - 1e-5,
        "queued arrivals overlap: {positions:?}"
    );
}

/// For each unit, the destination (`move_target`) at the first completed tick of the order log
/// where the unit has the current order `order` and a destination. A unit with no such tick is
/// left out.
fn first_logged_destinations(
    records: &[turts_audit::EconomyLogRecord],
    units: &[u32],
    order: &str,
) -> Vec<(u32, bevy_math::Vec2)> {
    units
        .iter()
        .filter_map(|unit_id| {
            records.iter().find_map(|record| {
                let turts_audit::EconomyLogRecord::Tick { state, .. } = record else {
                    return None;
                };
                let current = state
                    .unit_orders
                    .iter()
                    .find(|row| row.unit_id == *unit_id)?
                    .orders
                    .current
                    .as_ref()?;
                if current.order.to_string() != order {
                    return None;
                }
                let target = state
                    .raw_units
                    .iter()
                    .find(|row| row.unit_id == *unit_id)?
                    .move_target
                    .as_ref()?;
                Some((*unit_id, bevy_math::Vec2::new(target.x, target.y)))
            })
        })
        .collect()
}

#[test]
fn queued_unit_attack_activates_and_strictly_replays() {
    let mut catalog = audit_catalog();
    let mut focus = catalog.orders[&ContentId::parse("audit:rush").unwrap()].clone();
    focus.id = "focus".into();
    focus.name = "Focus".into();
    focus.behaviour = Behaviour::Attack;
    focus.target = TargetKind::Unit;
    focus.engage = Some(EngagePolicy::StopAndFire);
    catalog
        .orders
        .insert(ContentId::parse("audit:focus").unwrap(), focus);
    let (mut app, catalog) = audit_fixture_at_enemy(catalog, WorldPos { x: 35.0, y: 5.0 });
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let initial = audit_state(app.world_mut());
    let mut writer = economylog::EconomyLogWriter::new(Vec::new());
    {
        let trail = app.world().resource::<AuditTrail>();
        writer
            .write_initial(trail, initial)
            .expect("write initial audit snapshot");
    }

    let move_command = PlayerCommand {
        seq: 1,
        order: OrderEnvelope {
            order: "audit:move".into(),
            units: vec![UnitId(1)],
            target: OrderTarget::Point(Pos { x: 6.0, y: 5.0 }),
            queue: QueueMode::Replace,
            group: 0,
        },
    };
    let sim_move = orders::to_sim(SlotId(0), &move_command, &catalog).unwrap();
    assert_eq!(
        apply_command(app.world_mut(), &sim_move[0]).units.accepted,
        vec![1]
    );
    drain_audit(&mut app, &mut writer);

    let attack_command = PlayerCommand {
        seq: 2,
        order: OrderEnvelope {
            order: "audit:focus".into(),
            units: vec![UnitId(1)],
            target: OrderTarget::Unit(UnitId(3)),
            queue: QueueMode::Append,
            group: 0,
        },
    };
    let sim_attack = orders::to_sim(SlotId(0), &attack_command, &catalog).unwrap();
    assert_eq!(
        apply_command(app.world_mut(), &sim_attack[0])
            .units
            .accepted,
        vec![1]
    );
    drain_audit(&mut app, &mut writer);
    assert_eq!(
        app.world()
            .entity(
                app.world()
                    .iter_entities()
                    .find(|entity| entity
                        .get::<turts_sim::UnitId>()
                        .is_some_and(|id| id.0 == 1))
                    .unwrap()
                    .id()
            )
            .get::<UnitOrders>()
            .unwrap()
            .queue
            .len(),
        1
    );

    let mut attack_activated = false;
    for _ in 0..160 {
        app.update();
        let active = app
            .world()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|id| id.0 == 1)
            })
            .and_then(|entity| entity.get::<UnitOrders>())
            .and_then(|orders| orders.current.as_ref())
            .is_some_and(|order| order.order.to_string() == "audit:focus");
        attack_activated |= active;
        drain_audit(&mut app, &mut writer);
        if !active && attack_activated {
            break;
        }
    }
    assert!(
        attack_activated,
        "queued Unit Attack becomes active after Move"
    );
    assert!(
        app.world()
            .iter_entities()
            .all(|entity| entity.get::<turts_sim::UnitId>().is_none_or(|id| id.0 != 3)),
        "attacker pursues and removes its assigned target",
    );

    let records = writer
        .into_inner()
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).expect("parse audit log record"))
        .collect::<Vec<turts_audit::EconomyLogRecord>>();
    assert!(
        turts_audit::validate_records(&records).is_ok(),
        "queued Unit Attack activation and target completion strictly replay: {:?}",
        turts_audit::validate_records(&records)
    );
}

#[test]
fn point_attack_switches_to_another_eligible_target_and_strictly_replays() {
    let mut catalog = audit_catalog();
    catalog
        .units
        .get_mut(&ContentId::parse("audit:enemy").unwrap())
        .unwrap()
        .health
        .as_mut()
        .unwrap()
        .max = 1;
    let map = MapDef {
        id: "audit:target-switch-field".into(),
        size: MapSize { x: 40.0, y: 40.0 },
        placements: vec![
            Placement {
                unit: "audit:shooter".into(),
                owner_slot: 0,
                pos: WorldPos { x: 5.0, y: 5.0 },
                contents: vec![],
                facing: None,
                start_order: None,
            },
            Placement {
                unit: "audit:enemy".into(),
                owner_slot: 1,
                pos: WorldPos { x: 6.0, y: 5.0 },
                contents: vec![],
                facing: None,
                start_order: None,
            },
            Placement {
                unit: "audit:enemy".into(),
                owner_slot: 1,
                pos: WorldPos { x: 7.0, y: 5.0 },
                contents: vec![],
                facing: None,
                start_order: None,
            },
        ],
        nodes: vec![],
        slot_economy: vec![],
        belts: vec![],
    };
    let mut app = headless_app(20);
    let staged = stage_package(
        app.world(),
        catalog.clone(),
        SourceDef {
            file: "maps/target-switch.ron".into(),
            value: map,
        },
        20,
    )
    .expect("stage target switch fixture");
    install_package(app.world_mut(), staged);
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let initial = audit_state(app.world_mut());
    let mut writer = economylog::EconomyLogWriter::new(Vec::new());
    {
        let trail = app.world().resource::<AuditTrail>();
        writer
            .write_initial(trail, initial)
            .expect("write initial audit snapshot");
    }

    let command = PlayerCommand {
        seq: 1,
        order: OrderEnvelope {
            order: "audit:rush".into(),
            units: vec![UnitId(1)],
            target: OrderTarget::Point(Pos { x: 20.0, y: 5.0 }),
            queue: QueueMode::Replace,
            group: 0,
        },
    };
    let sim = orders::to_sim(SlotId(0), &command, &catalog).expect("resolve rush");
    assert_eq!(
        apply_command(app.world_mut(), &sim[0]).units.accepted,
        vec![1]
    );
    drain_audit(&mut app, &mut writer);

    let mut paused_position = None;
    for tick in 0..3 {
        app.update();
        let actor = app
            .world()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|id| id.0 == 1)
            })
            .unwrap();
        let position = actor.get::<Position>().unwrap().0;
        if tick == 0 {
            paused_position = Some(position);
            assert!(actor.get::<MoveTarget>().is_none());
            assert!(
                app.world()
                    .iter_entities()
                    .all(|entity| entity.get::<turts_sim::UnitId>().is_none_or(|id| id.0 != 2)),
                "first in-range target is destroyed"
            );
        } else if tick == 1 {
            assert_eq!(position, paused_position.unwrap());
            assert!(actor.get::<MoveTarget>().is_none());
            assert!(
                app.world()
                    .iter_entities()
                    .all(|entity| entity.get::<turts_sim::UnitId>().is_none_or(|id| id.0 != 3)),
                "the second in-range target is destroyed without an intervening move"
            );
        } else {
            assert!(actor.get::<MoveTarget>().is_some());
        }
        drain_audit(&mut app, &mut writer);
    }

    let records = writer
        .into_inner()
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).expect("parse audit log record"))
        .collect::<Vec<turts_audit::EconomyLogRecord>>();
    assert!(
        turts_audit::validate_records(&records).is_ok(),
        "target switch remains paused and strictly replays: {:?}",
        turts_audit::validate_records(&records)
    );
}

#[test]
fn suppressed_order_cooldowns_advance_and_strictly_replay() {
    let mut catalog = audit_catalog();
    catalog
        .units
        .get_mut(&ContentId::parse("audit:shooter").unwrap())
        .unwrap()
        .weapon
        .as_mut()
        .unwrap()
        .cooldown_ms = 1_000;
    for (id, policy) in [
        ("hold_never", turts_mod_api::FirePolicy::Never),
        ("hold_fire", turts_mod_api::FirePolicy::InRange),
    ] {
        let content_id = ContentId::parse(&format!("audit:{id}")).unwrap();
        catalog.orders.insert(
            content_id,
            OrderDef {
                id: id.into(),
                name: id.into(),
                behaviour: Behaviour::Hold,
                target: TargetKind::None,
                fire: Some(policy),
                engage: None,
                icon: String::new(),
                hotkey: String::new(),
                cursor: None,
                card: None,
                units: vec!["audit:shooter".into()],
                description: id.into(),
            },
        );
    }
    let (mut app, catalog) = audit_fixture_at_enemy(catalog, WorldPos { x: 8.0, y: 5.0 });
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let initial = audit_state(app.world_mut());
    let mut writer = economylog::EconomyLogWriter::new(Vec::new());
    {
        let trail = app.world().resource::<AuditTrail>();
        writer
            .write_initial(trail, initial)
            .expect("write initial audit snapshot");
    }

    app.update();
    drain_audit(&mut app, &mut writer);
    assert_eq!(
        audit_state(app.world_mut())
            .raw_healths
            .iter()
            .find(|health| health.unit == 3)
            .unwrap()
            .current,
        59,
        "the enabled weapon fires once before the cooldown test",
    );

    for (seq, id) in [(1, "audit:hold_never"), (2, "audit:hold_fire")] {
        let command = PlayerCommand {
            seq,
            order: OrderEnvelope {
                order: id.into(),
                units: vec![UnitId(1)],
                target: OrderTarget::None,
                queue: QueueMode::Replace,
                group: 0,
            },
        };
        let sim = orders::to_sim(SlotId(0), &command, &catalog)
            .unwrap_or_else(|error| panic!("{id} resolves: {error:?}"));
        assert_eq!(
            apply_command(app.world_mut(), &sim[0]).units.accepted,
            vec![1]
        );
        drain_audit(&mut app, &mut writer);
        let ticks = if id == "audit:hold_never" { 20 } else { 1 };
        for _ in 0..ticks {
            app.update();
            drain_audit(&mut app, &mut writer);
        }
        let remaining_health = audit_state(app.world_mut())
            .raw_healths
            .iter()
            .find(|health| health.unit == 3)
            .unwrap()
            .current;
        if id == "audit:hold_never" {
            assert_eq!(remaining_health, 59, "Never suppresses all cooldown shots");
            assert_eq!(
                audit_state(app.world_mut())
                    .raw_weapons
                    .iter()
                    .find(|weapon| weapon.unit == 1)
                    .unwrap()
                    .cooldown,
                0,
                "Never still advances the enabled weapon timer",
            );
        } else {
            assert_eq!(remaining_health, 58, "InRange fires on the next ready tick");
        }
    }

    let records = writer
        .into_inner()
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).expect("parse audit log record"))
        .collect::<Vec<turts_audit::EconomyLogRecord>>();
    assert!(
        turts_audit::validate_records(&records).is_ok(),
        "suppressed authored cooldowns and prompt re-enable strictly replay: {:?}",
        turts_audit::validate_records(&records)
    );
}

fn empty_ammo_attack_move_pause_replays(group: u32) {
    let mut catalog = audit_catalog();
    let ammo = ContentId::parse("audit:ammo").unwrap();
    catalog.resources.insert(
        ammo.clone(),
        ResourceDef {
            id: "ammo".into(),
            name: "Ammo".into(),
            unit_name: "shot".into(),
            kind: ResourceKind::Item,
        },
    );
    let shooter = catalog
        .units
        .get_mut(&ContentId::parse("audit:shooter").unwrap())
        .unwrap();
    shooter.inventory = Some(ResolvedInventory {
        capacity: 5,
        accepts: vec![ammo.clone()],
    });
    shooter.weapon.as_mut().unwrap().ammo = Some(ResolvedAmmo {
        item: ammo,
        per_shot: 1,
    });
    let (mut app, catalog) = audit_fixture_at_enemy(catalog, WorldPos { x: 6.0, y: 5.0 });
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let initial = audit_state(app.world_mut());
    let mut writer = economylog::EconomyLogWriter::new(Vec::new());
    {
        let trail = app.world().resource::<AuditTrail>();
        writer
            .write_initial(trail, initial)
            .expect("write initial audit snapshot");
    }

    let actor = app
        .world()
        .iter_entities()
        .find_map(|entity| {
            let (Some(id), Some(owner), Some(kind)) = (
                entity.get::<turts_sim::UnitId>(),
                entity.get::<Owner>(),
                entity.get::<turts_sim::UnitType>(),
            ) else {
                return None;
            };
            (owner.0 == SlotId(0)
                && app
                    .world()
                    .resource::<UnitTypes>()
                    .get(kind.0)
                    .is_some_and(|unit| unit.content_id == "audit:shooter"))
            .then_some(id.0)
        })
        .expect("audit shooter");
    let command = PlayerCommand {
        seq: 1,
        order: OrderEnvelope {
            order: "audit:rush".into(),
            units: vec![UnitId(actor)],
            target: OrderTarget::Point(Pos { x: 20.0, y: 5.0 }),
            queue: QueueMode::Replace,
            group,
        },
    };
    let sim = orders::to_sim(SlotId(0), &command, &catalog).expect("resolve rush");
    let result = apply_command(app.world_mut(), &sim[0]);
    assert_eq!(result.units.accepted, vec![actor]);
    assert!(result.units.refused.is_empty());
    {
        let mut trail = app.world_mut().resource_mut::<AuditTrail>();
        for command in trail.drain_commands() {
            writer
                .write_command(&trail, command)
                .expect("write command audit");
        }
        for operation in trail.drain_operations() {
            writer
                .write_operation(&trail, operation)
                .expect("write operation audit");
        }
    }

    let mut position_after_acquisition = None;
    for tick in 0..4 {
        app.update();
        let entity = app
            .world()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<turts_sim::UnitId>()
                    .is_some_and(|id| id.0 == actor)
            })
            .unwrap();
        let position = entity.get::<Position>().unwrap().0;
        if tick == 0 {
            position_after_acquisition = Some(position);
            assert_eq!(
                entity
                    .get::<UnitOrders>()
                    .and_then(|orders| orders.current.as_ref())
                    .map(|order| order.order.to_string()),
                Some("audit:rush".into()),
                "the actual server-resolved Point Attack is active",
            );
        } else {
            assert_eq!(position, position_after_acquisition.unwrap());
        }
        assert_eq!(
            audit_state(app.world_mut())
                .raw_healths
                .iter()
                .find(|health| health.unit == 3)
                .unwrap()
                .current,
            60,
        );

        let mut trail = app.world_mut().resource_mut::<AuditTrail>();
        for command in trail.drain_commands() {
            writer
                .write_command(&trail, command)
                .expect("write command audit");
        }
        for operation in trail.drain_operations() {
            writer
                .write_operation(&trail, operation)
                .expect("write operation audit");
        }
        for tick in trail.drain_ticks() {
            writer.write_tick(&trail, tick).expect("write tick audit");
        }
    }

    let records = writer
        .into_inner()
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).expect("parse audit log record"))
        .collect::<Vec<turts_audit::EconomyLogRecord>>();
    assert!(
        turts_audit::validate_records(&records).is_ok(),
        "no-ammo acquisition and attack-move pause strictly replay: {:?}",
        turts_audit::validate_records(&records)
    );
}

#[test]
fn empty_ammo_singleton_attack_move_pause_replays_before_first_shot() {
    empty_ammo_attack_move_pause_replays(0);
}

#[test]
fn empty_ammo_group_attack_move_pause_replays_before_first_shot() {
    empty_ammo_attack_move_pause_replays(1);
}
