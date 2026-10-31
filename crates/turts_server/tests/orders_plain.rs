//! A plain Move envelope must preserve the frozen M2 MoveTo audit boundary.

#[path = "../src/orders.rs"]
#[allow(dead_code)]
mod orders;

use bevy_math::Vec2;
use std::collections::BTreeMap;
use turts_mod_api::{
    Behaviour, ContentCatalog, ContentId, Footprint, MapDef, MapSize, OrderDef, PackageId,
    Placement, TargetKind, UnitDef, UnitRegistry, WorldPos,
};
use turts_net_protocol::{
    OrderEnvelope, OrderTarget, PlayerCommand, Pos, QueueMode as WireQueueMode, UnitId,
};
use turts_sim::{
    Applied, AuditTrail, Command, CommandKind, CommandResult, JobId, RejectReason, SlotId,
    UnitOutcomes, UnitTypes, apply_command, audit_state, headless_app, load_map,
};

fn app_with_walker() -> bevy_app::App {
    let mut app = headless_app(20);
    let package = PackageId("fixture".into());
    app.world_mut()
        .resource_mut::<UnitTypes>()
        .register_unit_type(
            &package,
            UnitDef {
                id: "walker".into(),
                name: "Walker".into(),
                movement: Some(turts_mod_api::MovementDef { speed: 3.0 }),
                inventory: None,
                gatherer: None,
                drop_off: None,
                producer: None,
                footprint: Footprint::Circle { radius: 0.5 },
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
            },
        )
        .unwrap();
    load_map(
        app.world_mut(),
        &package,
        &MapDef {
            id: "field".into(),
            size: MapSize { x: 64.0, y: 64.0 },
            placements: vec![Placement {
                unit: "fixture:walker".into(),
                owner_slot: 0,
                pos: WorldPos { x: 8.0, y: 8.0 },
                contents: Vec::new(),
                facing: None,
            }],
            nodes: Vec::new(),
            slot_economy: Vec::new(),
            belts: Vec::new(),
        },
    )
    .unwrap();
    app
}

fn catalog() -> ContentCatalog {
    let mut catalog = ContentCatalog {
        resources: BTreeMap::new(),
        units: BTreeMap::new(),
        nodes: BTreeMap::new(),
        production: BTreeMap::new(),
        recipes: BTreeMap::new(),
        technologies: BTreeMap::new(),
        orders: BTreeMap::new(),
        order_queue_depths: BTreeMap::from([(PackageId("fixture".into()), 16)]),
        source_files: BTreeMap::new(),
    };
    let id = ContentId::parse("fixture:move").unwrap();
    catalog.orders.insert(
        id.clone(),
        OrderDef {
            id: id.local.clone(),
            name: "Move".into(),
            behaviour: Behaviour::Move,
            target: TargetKind::Point,
            fire: None,
            engage: None,
            icon: String::new(),
            hotkey: String::new(),
            cursor: None,
            card: None,
            units: Vec::new(),
            description: String::new(),
        },
    );
    catalog
}

fn envelope() -> PlayerCommand {
    PlayerCommand {
        seq: 7,
        order: OrderEnvelope {
            order: "fixture:move".into(),
            units: vec![UnitId(1)],
            target: OrderTarget::Point(Pos { x: 20.0, y: 20.0 }),
            queue: WireQueueMode::Replace,
            group: 0,
        },
    }
}

#[test]
fn plain_move_envelope_has_the_moveto_operation_count() {
    let input = envelope();
    let catalog = catalog();
    let converted = orders::to_sim(SlotId(0), &input, &catalog).unwrap();
    assert_eq!(converted.len(), 1);
    assert!(matches!(converted[0].kind, CommandKind::Order(_)));

    let mut direct_app = app_with_walker();
    direct_app.world_mut().resource_mut::<AuditTrail>().enable();
    let direct_command = Command {
        slot: SlotId(0),
        seq: input.seq,
        kind: CommandKind::MoveTo {
            unit_ids: vec![1],
            target: Vec2::new(20.0, 20.0),
        },
    };
    apply_command(direct_app.world_mut(), &direct_command);
    let direct = direct_app
        .world()
        .resource::<AuditTrail>()
        .operations
        .clone();

    let mut order_app = app_with_walker();
    order_app.world_mut().resource_mut::<AuditTrail>().enable();
    apply_command(order_app.world_mut(), &converted[0]);
    let ordered = order_app
        .world()
        .resource::<AuditTrail>()
        .operations
        .clone();
    assert_eq!(direct.len(), 1);
    assert_eq!(ordered.len(), 1);
    assert_eq!(ordered[0].before, direct[0].before);
    assert_eq!(ordered[0].after, direct[0].after);
    assert!(audit_state(order_app.world_mut()).unit_orders.is_empty());
}

#[test]
fn wire_result_keeps_the_first_successful_production_job() {
    let mut command = envelope();
    command.order.units.push(UnitId(2));
    let result = CommandResult {
        slot: SlotId(0),
        seq: command.seq,
        outcome: Ok(Applied {
            job_id: Some(JobId(42)),
        }),
        units: UnitOutcomes {
            accepted: vec![1],
            refused: vec![(2, RejectReason::NotOwner)],
        },
    };
    let wire = orders::to_wire(&[result], &command);
    assert_eq!(wire.accepted, vec![UnitId(1)]);
    assert_eq!(wire.refused.len(), 1);
    assert_eq!(wire.job_id, Some(turts_net_protocol::JobId(42)));
}

#[test]
fn wrong_target_kind_is_checked_after_unit_owner_and_capability() {
    let mut command = envelope();
    command.order.units = vec![UnitId(1), UnitId(2), UnitId(999)];
    command.order.target = OrderTarget::Node(turts_net_protocol::NodeId(1));
    let converted = orders::to_sim(SlotId(0), &command, &catalog()).unwrap();
    let mut app = app_with_walker();
    let kind = app
        .world()
        .iter_entities()
        .find_map(|entity| entity.get::<turts_sim::UnitType>().copied())
        .unwrap();
    app.world_mut().spawn((
        turts_sim::UnitId(2),
        kind,
        turts_sim::Owner(SlotId(1)),
        turts_sim::Position(Vec2::new(10.0, 10.0)),
        turts_sim::UnitOrders::default(),
    ));
    let result = apply_command(app.world_mut(), &converted[0]);
    assert_eq!(
        result.units.refused,
        vec![
            (1, RejectReason::TargetInvalid),
            (2, RejectReason::NotOwner),
            (999, RejectReason::UnknownUnit),
        ]
    );
}

#[test]
fn move_target_unit_policy_rejects_point_after_subject_priority() {
    let mut command = envelope();
    command.order.units = vec![UnitId(2), UnitId(1)];
    let mut catalog = catalog();
    catalog
        .orders
        .get_mut(&ContentId::parse("fixture:move").unwrap())
        .unwrap()
        .target = TargetKind::Unit;
    let converted = orders::to_sim(SlotId(0), &command, &catalog).unwrap();
    let OrderTarget::Point(_) = command.order.target else {
        panic!("fixture must use Point")
    };
    let mut app = app_with_walker();
    let kind = app
        .world()
        .iter_entities()
        .find_map(|entity| entity.get::<turts_sim::UnitType>().copied())
        .unwrap();
    app.world_mut().spawn((
        turts_sim::UnitId(2),
        kind,
        turts_sim::Owner(SlotId(1)),
        turts_sim::Position(Vec2::new(10.0, 10.0)),
        turts_sim::UnitOrders::default(),
    ));
    let result = apply_command(app.world_mut(), &converted[0]);
    assert_eq!(result.units.accepted, Vec::<u32>::new());
    assert_eq!(
        result.units.refused,
        vec![
            (1, RejectReason::TargetInvalid),
            (2, RejectReason::NotOwner),
        ]
    );
}
