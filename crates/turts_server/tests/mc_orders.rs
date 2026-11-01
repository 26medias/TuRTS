//! Protocol 4 behavior envelopes must resolve through package definitions into simulation orders.

#[path = "../src/orders.rs"]
#[allow(dead_code)]
mod orders;

use std::path::PathBuf;

use turts_mod_api::{Behaviour, TargetKind};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{OrderEnvelope, OrderTarget, PlayerCommand, Pos, QueueMode, UnitId};
use turts_pack::load_package;
use turts_sim::{
    CommandKind, MoveTarget, OrderTargetSim, Owner, Position, SlotId, UnitOrders, UnitTypes,
    apply_command, headless_app,
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
