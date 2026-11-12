use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
};

use bevy_app::{App, FixedUpdate};
use bevy_ecs::{prelude::*, schedule::IntoScheduleConfigs, world::World};
use bevy_math::Vec2;
use turts_interaction::{
    Button, CameraState, Core, InputEvent, NodeView, UnitView, Viewport, WorldView,
};
use turts_mod_api::{
    Behaviour, Capability, ContentId, Footprint, InputMapDef, OrderDef, OrderRulesDef, TargetKind,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{
    NodeId as ProtocolNodeId, OrderTarget, Pos, SlotId as ProtocolSlotId, UnitId as ProtocolUnitId,
};
use turts_pack::load_package;
use turts_sim::{
    Command, CommandKind, CommandResult, MoveTarget, OrderCommand, OrderPolicyAudit,
    OrderTargetSim, OrderUnitCapabilitiesAudit, Position, QueueMode, RejectReason, SimSet, Tick,
    UnitId, UnitOrders, apply_command, headless_app,
};

const UI_VIEW: Viewport = Viewport {
    size: Vec2::new(1280.0, 720.0),
    scale_factor: 1.0,
};

struct SmartMusterWorld {
    catalog: turts_mod_api::ContentCatalog,
    units: Vec<UnitView>,
    nodes: Vec<NodeView>,
    orders: Vec<OrderDef>,
    rules: OrderRulesDef,
    input: InputMapDef,
    priority: Vec<String>,
    size: Vec2,
}

impl SmartMusterWorld {
    fn actual() -> Self {
        let archive = muster();
        let catalog = archive.content().expect("actual resolved Muster catalog");
        let (_, map) = archive
            .maps()
            .expect("actual Muster map")
            .into_iter()
            .find(|(path, _)| path == "maps/field.ron")
            .expect("actual field map");
        let units = map
            .placements
            .iter()
            .enumerate()
            .map(|(index, placement)| {
                let content_id = placement.unit.to_string();
                let definition = &catalog.units[&ContentId::parse(&content_id).unwrap()];
                let mut capabilities = Vec::new();
                if definition.movement.is_some() {
                    capabilities.push(Capability::Movement);
                }
                if definition.weapon.is_some() {
                    capabilities.push(Capability::Weapon);
                }
                if definition.gatherer.is_some() {
                    capabilities.push(Capability::Gatherer);
                }
                if definition.producer.is_some() {
                    capabilities.push(Capability::Producer);
                }
                if definition.research.is_some() {
                    capabilities.push(Capability::Research);
                }
                let Footprint::Circle { radius } = definition.footprint;
                UnitView {
                    id: ProtocolUnitId(index as u32 + 1),
                    content_id,
                    owner_slot: ProtocolSlotId(placement.owner_slot),
                    position: Vec2::new(placement.pos.x, placement.pos.y),
                    footprint_radius: radius,
                    capabilities,
                    content_index: index as u32,
                }
            })
            .collect();
        let nodes = map
            .nodes
            .iter()
            .enumerate()
            .map(|(index, placement)| {
                let content_id = placement.node.to_string();
                NodeView {
                    id: ProtocolNodeId(index as u32 + 1),
                    content_id: content_id.clone(),
                    position: Vec2::new(placement.pos.x, placement.pos.y),
                    radius: catalog.nodes[&ContentId::parse(&content_id).unwrap()].radius,
                    content_index: index as u32,
                }
            })
            .collect();
        let orders = catalog
            .orders
            .iter()
            .map(|(id, definition)| {
                let mut definition = definition.clone();
                definition.id = id.to_string();
                definition
            })
            .collect();
        let rules = archive.order_rules().unwrap().into_iter().next().unwrap().1;
        let input = archive.input_maps().unwrap().into_iter().next().unwrap().1;
        let priority = archive
            .interfaces()
            .unwrap()
            .into_iter()
            .map(|(_, interface)| interface.selection_priority)
            .find(|priority| !priority.is_empty())
            .unwrap_or_default();
        Self {
            catalog,
            units,
            nodes,
            orders,
            rules,
            input,
            priority,
            size: Vec2::new(map.size.x, map.size.y),
        }
    }
}

impl WorldView for SmartMusterWorld {
    fn units(&self) -> &[UnitView] {
        &self.units
    }
    fn nodes(&self) -> &[NodeView] {
        &self.nodes
    }
    fn own_slot(&self) -> ProtocolSlotId {
        ProtocolSlotId(0)
    }
    fn orders(&self) -> &[OrderDef] {
        &self.orders
    }
    fn order_rules(&self) -> &OrderRulesDef {
        &self.rules
    }
    fn input_map(&self) -> &InputMapDef {
        &self.input
    }
    fn map_size(&self) -> Option<Vec2> {
        Some(self.size)
    }
    fn selection_priority(&self) -> &[String] {
        &self.priority
    }
    fn can_gather(&self, unit: &UnitView, node: &NodeView) -> bool {
        let Ok(unit_id) = ContentId::parse(&unit.content_id) else {
            return false;
        };
        let Ok(node_id) = ContentId::parse(&node.content_id) else {
            return false;
        };
        self.catalog
            .units
            .get(&unit_id)
            .and_then(|definition| definition.gatherer.as_ref())
            .zip(self.catalog.nodes.get(&node_id))
            .is_some_and(|(gatherer, node)| gatherer.items.contains(&node.item))
    }
    fn can_weapon_target(&self, unit: &UnitView, target: &UnitView) -> bool {
        let Ok(unit_id) = ContentId::parse(&unit.content_id) else {
            return false;
        };
        self.catalog
            .units
            .get(&unit_id)
            .and_then(|definition| definition.weapon.as_ref())
            .is_some_and(|weapon| {
                turts_sim::rules_adapter::holds_own_team(
                    weapon.targets.slots,
                    unit.owner_slot.0,
                    target.owner_slot.0,
                )
            })
    }
}

fn screen_click(
    core: &mut Core,
    world: &SmartMusterWorld,
    button: Button,
    position: Vec2,
) -> Vec<turts_interaction::Effect> {
    let px = core.project_world_to_screen(position, &UI_VIEW);
    assert!(
        px.x >= 0.0 && px.y >= 0.0 && px.x < UI_VIEW.size.x && px.y < UI_VIEW.size.y,
        "physical test point is visible: {position:?} -> {px:?}"
    );
    core.step(
        &[
            InputEvent::PointerDown {
                button,
                px,
                over_widget: false,
            },
            InputEvent::PointerUp {
                button,
                px,
                over_widget: false,
            },
        ],
        world,
        &UI_VIEW,
        1,
    )
}

fn selected_mixed_core(
    world: &SmartMusterWorld,
    worker: ProtocolUnitId,
    trooper: ProtocolUnitId,
) -> Core {
    let mut core = Core::new(CameraState {
        look_at: Vec2::new(28.85, 20.2),
        height: 38.0,
        pitch: 0.7,
        yaw: 0.0,
        vertical_fov: 60.0_f32.to_radians(),
    });
    for id in [worker, trooper] {
        let position = world
            .units
            .iter()
            .find(|unit| unit.id == id)
            .unwrap()
            .position;
        screen_click(&mut core, world, Button::Left, position);
        if id == worker {
            core.step(
                &[InputEvent::KeyDown {
                    code: "ShiftLeft".into(),
                    mods: vec![turts_mod_api::Mod::Shift],
                }],
                world,
                &UI_VIEW,
                2,
            );
        }
    }
    core.step(
        &[InputEvent::KeyUp {
            code: "ShiftLeft".into(),
        }],
        world,
        &UI_VIEW,
        3,
    );
    assert_eq!(core.selected_units(), &[worker, trooper]);
    core
}

fn sim_command_from_envelope(envelope: &turts_net_protocol::OrderEnvelope, seq: u32) -> Command {
    let target = match &envelope.target {
        OrderTarget::None => OrderTargetSim::None,
        OrderTarget::Point(Pos { x, y }) => OrderTargetSim::Point(Vec2::new(*x, *y)),
        OrderTarget::Unit(ProtocolUnitId(id)) => OrderTargetSim::Unit(*id),
        OrderTarget::Node(ProtocolNodeId(id)) => OrderTargetSim::Node(turts_sim::NodeId(*id)),
        OrderTarget::Job(_) | OrderTarget::Content(_) => {
            panic!("smart order fixture has a supported world target")
        }
    };
    let queue = match envelope.queue {
        turts_net_protocol::QueueMode::Replace => QueueMode::Replace,
        turts_net_protocol::QueueMode::Append => QueueMode::Append,
    };
    let mut command = command(
        seq,
        &envelope.order,
        catalog().orders[&ContentId::parse(&envelope.order).unwrap()].behaviour,
        catalog().orders[&ContentId::parse(&envelope.order).unwrap()].target,
        target,
        envelope.units.iter().map(|unit| unit.0).collect(),
        queue,
    );
    if let CommandKind::Order(order) = &mut command.kind {
        order.group = envelope.group;
    }
    command
}

#[test]
fn actual_mixed_core_orders_are_accepted_by_the_authored_simulation_for_ground_node_and_unit() {
    let world = SmartMusterWorld::actual();
    let worker = world
        .units
        .iter()
        .find(|unit| unit.content_id == "muster:worker" && unit.owner_slot == ProtocolSlotId(0))
        .unwrap()
        .id;
    let trooper = world
        .units
        .iter()
        .find(|unit| {
            unit.content_id == "muster:trooper"
                && unit.owner_slot == ProtocolSlotId(0)
                && unit.position == Vec2::new(32.7, 22.4)
        })
        .unwrap()
        .id;
    let node = &world.nodes[0];
    let enemy = world
        .units
        .iter()
        .find(|unit| unit.content_id == "muster:dummy" && unit.owner_slot == ProtocolSlotId(1))
        .unwrap();
    let ground = Vec2::new(18.0, 10.0);

    for target in [ground, node.position, enemy.position] {
        let mut app = app_with_muster();
        let mut core = selected_mixed_core(&world, worker, trooper);
        let effects = screen_click(&mut core, &world, Button::Right, target);
        let envelopes: Vec<_> = effects
            .into_iter()
            .filter_map(|effect| match effect {
                turts_interaction::Effect::Send(envelope) => Some(envelope),
                _ => None,
            })
            .collect();
        assert!(
            !envelopes.is_empty(),
            "one physical right click resolves authored smart orders"
        );
        let mut outcomes = Vec::new();
        for (index, envelope) in envelopes.iter().enumerate() {
            let sim_command = sim_command_from_envelope(envelope, index as u32 + 1);
            outcomes.push((envelope.clone(), apply(&mut app, sim_command)));
        }
        // Core smart groups are accepted into the real deferred spread path; advance its actual Orders phase.
        app.update();
        for unit in [worker, trooper] {
            assert!(
                outcomes.iter().any(|(envelope, outcome)| envelope
                    .units
                    .iter()
                    .any(|id| id.0 == unit.0)
                    && outcome.units.accepted.contains(&unit.0)),
                "the exact physical smart result for selected unit {unit:?} is accepted by CommandKind::Order"
            );
        }
        let expected = if target == ground {
            [
                (worker.0, "muster:order_move", OrderTargetSim::Point(ground)),
                (
                    trooper.0,
                    "muster:order_move",
                    OrderTargetSim::Point(ground),
                ),
            ]
        } else if target == node.position {
            [
                (
                    worker.0,
                    "muster:order_gather",
                    OrderTargetSim::Node(turts_sim::NodeId(node.id.0)),
                ),
                (
                    trooper.0,
                    "muster:order_move",
                    OrderTargetSim::Point(node.position),
                ),
            ]
        } else {
            [
                (
                    worker.0,
                    "muster:order_move",
                    OrderTargetSim::Unit(enemy.id.0),
                ),
                (
                    trooper.0,
                    "muster:order_attack",
                    OrderTargetSim::Unit(enemy.id.0),
                ),
            ]
        };
        for (id, order_id, target) in expected {
            let entity = entity_for(&app, id);
            let state = app.world().entity(entity).get::<UnitOrders>().unwrap();
            let current = state
                .current
                .as_ref()
                .expect("Core emitted typed order dispatches to active UnitOrders.current");
            assert_eq!(current.order, ContentId::parse(order_id).unwrap());
            match (&current.target, &target) {
                (OrderTargetSim::Point(actual), OrderTargetSim::Point(expected)) => assert!(
                    actual.distance(*expected) <= 0.001,
                    "actual point target remains within the authored f32 tolerance: {actual:?} vs {expected:?}"
                ),
                (actual, expected) => assert_eq!(
                    actual, expected,
                    "actual per-unit target survives typed simulation resolution"
                ),
            }
        }
    }
}

fn muster() -> PackageArchive {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
    archive_at(&path)
}

fn archive_at(path: &std::path::Path) -> PackageArchive {
    PackageArchive::parse(&pack_dir(path).expect("pack authored Muster fixture"))
        .expect("parse authored Muster fixture")
}

fn catalog() -> &'static turts_mod_api::ContentCatalog {
    static CATALOG: OnceLock<turts_mod_api::ContentCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| muster().content().expect("resolve actual authored catalog"))
}

fn command(
    seq: u32,
    order: &str,
    behaviour: Behaviour,
    target_kind: TargetKind,
    target: OrderTargetSim,
    units: Vec<u32>,
    queue: QueueMode,
) -> Command {
    let catalog = catalog();
    let id = ContentId::parse(order).expect("authored order id");
    let definition = catalog
        .orders
        .get(&id)
        .expect("actual installed catalog order");
    assert_eq!(
        definition.behaviour, behaviour,
        "fixture names the actual behavior"
    );
    assert_eq!(
        definition.target, target_kind,
        "fixture names the actual target kind"
    );
    let queue_depth = catalog
        .order_queue_depths
        .get(&id.package)
        .copied()
        .unwrap_or(turts_mod_api::DEFAULT_QUEUE_DEPTH);
    let unit_capabilities = catalog
        .units
        .iter()
        .map(|(unit_id, unit)| OrderUnitCapabilitiesAudit {
            unit: unit_id.to_string(),
            movement: unit.movement.is_some(),
            weapon: unit.weapon.is_some(),
            gatherer: unit.gatherer.is_some(),
            producer: unit.producer.is_some(),
            research: unit.research.is_some(),
            removable: unit
                .removal
                .as_ref()
                .is_some_and(|policy| policy.by_command),
            toggleable: unit.extractor.is_some()
                || unit.inserter.is_some()
                || unit.machine.is_some()
                || unit.research.is_some()
                || unit.producer.is_some()
                || unit.weapon.is_some(),
        })
        .collect();
    let policy = OrderPolicyAudit {
        id: id.clone(),
        behaviour: definition.behaviour,
        target: definition.target,
        fire: definition.fire,
        engage: definition.engage,
        units: definition.units.clone(),
        queue_depth,
        unit_capabilities,
    };
    Command {
        slot: turts_sim::SlotId(0),
        seq,
        kind: CommandKind::Order(Box::new(OrderCommand {
            order: id,
            behaviour: definition.behaviour,
            target_kind: definition.target,
            policy: Some(policy),
            units,
            target,
            queue,
            group: 0,
            allowed_units: definition.units.clone(),
            queue_depth,
        })),
    }
}

fn app_with_muster() -> App {
    let archive = muster();
    let mut app = headless_app(20);
    load_package(app.world_mut(), &archive).expect("load authored Muster map and catalog");
    app
}

fn app_with_archive(archive: &PackageArchive) -> App {
    let mut app = headless_app(20);
    load_package(app.world_mut(), archive).expect("load authored Muster map and catalog");
    app
}

fn entity_for(app: &App, id: u32) -> Entity {
    app.world()
        .iter_entities()
        .find(|entity| entity.get::<UnitId>().is_some_and(|unit| unit.0 == id))
        .expect("authored map unit id")
        .id()
}

fn apply(app: &mut App, command: Command) -> CommandResult {
    apply_command(app.world_mut(), &command)
}

fn set_pos(app: &mut App, id: u32, position: Vec2) {
    let entity = entity_for(app, id);
    app.world_mut()
        .entity_mut(entity)
        .insert(Position(position));
}

fn health(app: &mut App, id: u32) -> Option<u64> {
    turts_sim::audit_state(app.world_mut())
        .raw_healths
        .into_iter()
        .find(|record| record.unit == id)
        .map(|record| record.current)
}

fn isolate_order_trial(app: &mut App, actor: u32, keep_enemies: &[u32]) {
    let mut keep = vec![actor];
    keep.extend_from_slice(keep_enemies);
    prune_units(app, &keep);
}

fn prune_units(app: &mut App, keep: &[u32]) {
    let remove: Vec<_> = app
        .world()
        .iter_entities()
        .filter_map(|entity| {
            let id = entity.get::<UnitId>()?.0;
            (!keep.contains(&id)).then_some(entity.id())
        })
        .collect();
    for entity in remove {
        app.world_mut().despawn(entity);
    }
}

fn enable_audit(app: &mut App) {
    app.world_mut()
        .resource_mut::<turts_sim::AuditTrail>()
        .enable();
}

fn drain_tick_events(app: &mut App) -> Vec<turts_sim::EconomyEvent> {
    app.world_mut()
        .resource_mut::<turts_sim::AuditTrail>()
        .drain_ticks()
        .into_iter()
        .flat_map(|tick| tick.events)
        .collect()
}

fn drain_tick_records(app: &mut App) -> Vec<(u32, Vec<turts_sim::EconomyEvent>)> {
    app.world_mut()
        .resource_mut::<turts_sim::AuditTrail>()
        .drain_ticks()
        .into_iter()
        .map(|tick| (tick.tick, tick.events))
        .collect()
}

#[test]
fn authored_muster_append_replace_stop_hold_gather_and_queue_full_contracts() {
    let mut app = app_with_muster();
    let trooper = 40;
    let worker = 41;
    let node = turts_sim::NodeId(1);
    let actor = entity_for(&app, trooper);
    let first = Vec2::new(51.0, 8.0);
    let second = Vec2::new(53.0, 8.0);
    let third = Vec2::new(55.0, 8.0);
    for (seq, target) in [(1, first), (2, second), (3, third)] {
        let result = apply(
            &mut app,
            command(
                seq,
                "muster:order_move",
                Behaviour::Move,
                TargetKind::PointOrUnit,
                OrderTargetSim::Point(target),
                vec![trooper],
                QueueMode::Append,
            ),
        );
        assert_eq!(result.units.accepted, vec![trooper]);
    }
    let orders = app.world().entity(actor).get::<UnitOrders>().unwrap();
    assert_eq!(
        orders.current.as_ref().map(|order| order.target.clone()),
        Some(OrderTargetSim::Point(first))
    );
    assert_eq!(
        orders
            .queue
            .iter()
            .map(|order| order.target.clone())
            .collect::<Vec<_>>(),
        vec![OrderTargetSim::Point(second), OrderTargetSim::Point(third)],
        "Append on idle starts point A; the next two are queued in order"
    );
    assert_eq!(
        app.world().entity(actor).get::<MoveTarget>().unwrap().0,
        first
    );

    let replacement = Vec2::new(57.0, 8.0);
    let result = apply(
        &mut app,
        command(
            4,
            "muster:order_move",
            Behaviour::Move,
            TargetKind::PointOrUnit,
            OrderTargetSim::Point(replacement),
            vec![trooper],
            QueueMode::Replace,
        ),
    );
    assert_eq!(result.units.accepted, vec![trooper]);
    let orders = app.world().entity(actor).get::<UnitOrders>().unwrap();
    assert!(
        orders.queue.is_empty(),
        "plain replacement removes the queued suffix"
    );
    let effective = &app
        .world()
        .resource::<turts_sim::orders::PlainMoveOrderIdentities>()
        .0[&trooper];
    assert_eq!(
        effective.order,
        ContentId::parse("muster:order_move").unwrap()
    );
    assert_eq!(
        effective.target,
        OrderTargetSim::Point(replacement),
        "the optimized singleton move retains the authored replacement identity"
    );

    for (seq, target) in [(5, Vec2::new(58.0, 8.0)), (6, Vec2::new(59.0, 8.0))] {
        assert_eq!(
            apply(
                &mut app,
                command(
                    seq,
                    "muster:order_move",
                    Behaviour::Move,
                    TargetKind::PointOrUnit,
                    OrderTargetSim::Point(target),
                    vec![trooper],
                    QueueMode::Append
                )
            )
            .units
            .accepted,
            vec![trooper]
        );
    }
    assert_eq!(
        app.world()
            .entity(actor)
            .get::<UnitOrders>()
            .unwrap()
            .queue
            .len(),
        2,
        "the Stop witness begins with a real nonempty queued tail"
    );
    let before_stop = app.world().entity(actor).get::<Position>().unwrap().0;
    for _ in 0..4 {
        app.update();
    }
    let moving_before_stop = app.world().entity(actor).get::<Position>().unwrap().0;
    assert!(
        moving_before_stop.distance(before_stop) > 0.001,
        "Stop is issued after the real actor has begun moving toward its current point"
    );
    assert_eq!(
        app.world()
            .entity(actor)
            .get::<UnitOrders>()
            .unwrap()
            .queue
            .len(),
        2,
        "the moving Stop witness still has a nonempty queued tail immediately before S"
    );

    let result = apply(
        &mut app,
        command(
            7,
            "muster:order_stop",
            Behaviour::Stop,
            TargetKind::None,
            OrderTargetSim::None,
            vec![trooper],
            QueueMode::Replace,
        ),
    );
    assert_eq!(result.units.accepted, vec![trooper]);
    assert!(app.world().entity(actor).get::<MoveTarget>().is_none());
    assert!(
        app.world()
            .entity(actor)
            .get::<UnitOrders>()
            .unwrap()
            .current
            .is_none()
    );
    assert!(
        app.world()
            .entity(actor)
            .get::<UnitOrders>()
            .unwrap()
            .queue
            .is_empty()
    );
    let stopped_at = app.world().entity(actor).get::<Position>().unwrap().0;
    for _ in 0..8 {
        app.update();
    }
    assert_eq!(
        app.world().entity(actor).get::<Position>().unwrap().0,
        stopped_at,
        "Stop clears movement and stays stationary through later simulation ticks"
    );

    let hold = apply(
        &mut app,
        command(
            8,
            "muster:order_hold",
            Behaviour::Hold,
            TargetKind::None,
            OrderTargetSim::None,
            vec![trooper],
            QueueMode::Replace,
        ),
    );
    assert_eq!(hold.units.accepted, vec![trooper]);
    let blocked = apply(
        &mut app,
        command(
            9,
            "muster:order_move",
            Behaviour::Move,
            TargetKind::PointOrUnit,
            OrderTargetSim::Point(second),
            vec![trooper],
            QueueMode::Append,
        ),
    );
    assert_eq!(
        blocked.units.refused,
        vec![(trooper, RejectReason::NotQueueable)],
        "Append behind authored Hold is refused for the unit"
    );

    let gather = apply(
        &mut app,
        command(
            10,
            "muster:order_gather",
            Behaviour::Gather,
            TargetKind::Node,
            OrderTargetSim::Node(node),
            vec![worker],
            QueueMode::Replace,
        ),
    );
    assert_eq!(gather.units.accepted, vec![worker]);
    let blocked = apply(
        &mut app,
        command(
            11,
            "muster:order_move",
            Behaviour::Move,
            TargetKind::PointOrUnit,
            OrderTargetSim::Point(second),
            vec![worker],
            QueueMode::Append,
        ),
    );
    assert_eq!(
        blocked.units.refused,
        vec![(worker, RejectReason::NotQueueable)],
        "Append behind authored Gather is refused for the worker"
    );

    // The worker fixture remains independent from the trooper's queue.
    apply(
        &mut app,
        command(
            12,
            "muster:order_stop",
            Behaviour::Stop,
            TargetKind::None,
            OrderTargetSim::None,
            vec![trooper],
            QueueMode::Replace,
        ),
    );
    apply(
        &mut app,
        command(
            13,
            "muster:order_move",
            Behaviour::Move,
            TargetKind::PointOrUnit,
            OrderTargetSim::Point(first),
            vec![trooper],
            QueueMode::Append,
        ),
    );
    for seq in 14..=29 {
        let target = Vec2::new(40.0 + (seq as f32 - 13.0) * 0.4, 10.0);
        let result = apply(
            &mut app,
            command(
                seq,
                "muster:order_move",
                Behaviour::Move,
                TargetKind::PointOrUnit,
                OrderTargetSim::Point(target),
                vec![trooper],
                QueueMode::Append,
            ),
        );
        assert_eq!(result.units.accepted, vec![trooper]);
    }
    let overflow = apply(
        &mut app,
        command(
            30,
            "muster:order_move",
            Behaviour::Move,
            TargetKind::PointOrUnit,
            OrderTargetSim::Point(third),
            vec![trooper],
            QueueMode::Append,
        ),
    );
    assert_eq!(
        overflow.units.refused,
        vec![(trooper, RejectReason::QueueFull)],
        "the actual depth-16 authored queue refuses the seventeenth queued append"
    );
    assert_eq!(
        app.world()
            .entity(actor)
            .get::<UnitOrders>()
            .unwrap()
            .queue
            .len(),
        16
    );
}

type PhaseSample = (
    u32,
    &'static str,
    Vec2,
    Option<Vec2>,
    Option<OrderTargetSim>,
    Vec<OrderTargetSim>,
);

#[derive(Resource, Default)]
struct PhaseSamples {
    unit: u32,
    rows: Mutex<Vec<PhaseSample>>,
}

fn sample_after_orders(world: &mut World) {
    sample_phase(world, "after_orders");
}

fn sample_after_movement(world: &mut World) {
    sample_phase(world, "after_movement");
}

fn sample_phase(world: &mut World, phase: &'static str) {
    let probe = world.resource::<PhaseSamples>();
    let id = probe.unit;
    let Some(entity) = world
        .iter_entities()
        .find(|entity| entity.get::<UnitId>().is_some_and(|unit| unit.0 == id))
    else {
        return;
    };
    let position = entity
        .get::<Position>()
        .map_or(Vec2::ZERO, |position| position.0);
    let movement = entity.get::<MoveTarget>().map(|target| target.0);
    let orders = entity.get::<UnitOrders>();
    let current =
        orders.and_then(|orders| orders.current.as_ref().map(|order| order.target.clone()));
    let queue = orders.map_or_else(Vec::new, |orders| {
        orders
            .queue
            .iter()
            .map(|order| order.target.clone())
            .collect()
    });
    let tick = world.resource::<Tick>().0;
    probe
        .rows
        .lock()
        .unwrap()
        .push((tick, phase, position, movement, current, queue));
}

#[test]
fn completed_move_starts_queued_move_in_the_next_orders_phase_without_an_idle_tick() {
    let mut app = app_with_muster();
    let actor_id = 40;
    prune_units(&mut app, &[actor_id]);
    let actor = entity_for(&app, actor_id);
    let start = Vec2::new(52.0, 5.0);
    let first = Vec2::new(54.0, 5.0);
    let second = Vec2::new(56.0, 5.0);
    let third = Vec2::new(58.0, 5.0);
    app.world_mut().entity_mut(actor).insert(Position(start));
    app.insert_resource(PhaseSamples {
        unit: actor_id,
        ..Default::default()
    });
    app.add_systems(
        FixedUpdate,
        sample_after_orders
            .after(SimSet::Orders)
            .before(SimSet::Movement),
    );
    app.add_systems(
        FixedUpdate,
        sample_after_movement
            .after(SimSet::Movement)
            .before(SimSet::Gather),
    );

    assert_eq!(
        apply(
            &mut app,
            command(
                1,
                "muster:order_move",
                Behaviour::Move,
                TargetKind::PointOrUnit,
                OrderTargetSim::Point(first),
                vec![actor_id],
                QueueMode::Append
            )
        )
        .units
        .accepted,
        vec![actor_id]
    );
    assert_eq!(
        apply(
            &mut app,
            command(
                2,
                "muster:order_move",
                Behaviour::Move,
                TargetKind::PointOrUnit,
                OrderTargetSim::Point(second),
                vec![actor_id],
                QueueMode::Append
            )
        )
        .units
        .accepted,
        vec![actor_id]
    );
    assert_eq!(
        apply(
            &mut app,
            command(
                3,
                "muster:order_move",
                Behaviour::Move,
                TargetKind::PointOrUnit,
                OrderTargetSim::Point(third),
                vec![actor_id],
                QueueMode::Append
            )
        )
        .units
        .accepted,
        vec![actor_id]
    );
    for _ in 0..120 {
        app.update();
    }
    let probe = app.world().resource::<PhaseSamples>();
    let rows = probe.rows.lock().unwrap();
    let landing = rows
        .iter()
        .find(|(_, phase, position, movement, _, queue)| {
            *phase == "after_movement"
                && position.distance(first) <= 0.001
                && movement.is_none()
                && queue == &vec![OrderTargetSim::Point(second), OrderTargetSim::Point(third)]
        })
        .expect("Movement phase records A landing while B remains queued");
    let next_order = rows
        .iter()
        .find(|(tick, phase, _, movement, current, _)| {
            *tick == landing.0 + 1
                && *phase == "after_orders"
                && *movement == Some(second)
                && *current == Some(OrderTargetSim::Point(second))
        })
        .expect("the immediately following Orders phase starts B before its Movement phase");
    assert_eq!(next_order.0, landing.0 + 1);
    let next_movement = rows
        .iter()
        .find(|(tick, phase, position, movement, current, queue)| {
            *tick == landing.0 + 1
                && *phase == "after_movement"
                && *movement == Some(second)
                && *current == Some(OrderTargetSim::Point(second))
                && queue == &vec![OrderTargetSim::Point(third)]
                && position.distance(landing.2) > 0.01
                && (position - landing.2).dot(second - landing.2) > 0.0
        })
        .expect("Movement in that same t+1 tick advances from A toward B");
    assert_eq!(next_movement.0, landing.0 + 1);
    let b_landing = rows
        .iter()
        .find(|(tick, phase, position, movement, _, queue)| {
            *tick > next_movement.0
                && *phase == "after_movement"
                && position.distance(second) <= 0.001
                && movement.is_none()
                && queue == &vec![OrderTargetSim::Point(third)]
        })
        .expect("the real route reaches B before C");
    let c_landing = rows
        .iter()
        .find(|(tick, phase, position, movement, _, queue)| {
            *tick > b_landing.0
                && *phase == "after_movement"
                && position.distance(third) <= 0.001
                && movement.is_none()
                && queue.is_empty()
        })
        .expect("the real route reaches C after B and exhausts its queue");
    assert!(
        landing.0 < b_landing.0 && b_landing.0 < c_landing.0,
        "actual A/B/C landing observations are ordered by tick"
    );
}

#[test]
fn actual_worker_gather_exhausts_the_isolated_small_node_and_records_node_depleted() {
    let (emptied_at, failed_at) = worker_gather_exhausts_the_small_node(Some(&[41, 43]));
    eprintln!("pruned ORD3 scene: node empty at tick {emptied_at}, node_depleted at {failed_at}");
}

/// The ORD3 browser scene keeps every authored unit. Worker 42 stands near the depot
/// approach of worker 41, so the last trip can be blocked. The blocked trip must wait and
/// try again; the order still ends with node_depleted, and the depot converts the ore.
#[test]
fn actual_worker_gather_in_the_full_ord3_scene_delivers_then_records_node_depleted() {
    let (emptied_at, failed_at) = worker_gather_exhausts_the_small_node(None);
    eprintln!("full ORD3 scene: node empty at tick {emptied_at}, node_depleted at {failed_at}");
}

/// Return the tick the node empties and the tick the Gather order of worker 41 ends, both
/// counted from the tick after the order.
fn worker_gather_exhausts_the_small_node(keep: Option<&[u32]>) -> (u32, u32) {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/browser/fixtures/order-rows/muster");
    let archive = archive_at(&fixture);
    let mut app = app_with_archive(&archive);
    let worker = 41;
    if let Some(keep) = keep {
        prune_units(&mut app, keep);
    }
    assert_eq!(
        turts_sim::node_view(app.world(), turts_sim::NodeId(1))
            .unwrap()
            .remaining,
        2,
        "the isolated authored fixture differs only in its bounded positive node quantity"
    );
    let result = apply(
        &mut app,
        command(
            1,
            "muster:order_gather",
            Behaviour::Gather,
            TargetKind::Node,
            OrderTargetSim::Node(turts_sim::NodeId(1)),
            vec![worker],
            QueueMode::Replace,
        ),
    );
    assert_eq!(result.units.accepted, vec![worker]);
    let ore = ContentId::parse("muster:ore").unwrap();
    let cargo = |app: &App| -> u64 {
        turts_sim::inventory_view(app.world(), turts_sim::SlotId(0), worker)
            .unwrap()
            .items
            .iter()
            .map(|item| item.on_hand)
            .sum()
    };
    let credits = |app: &App| {
        turts_sim::balance_view(app.world(), turts_sim::SlotId(0), &ore)
            .unwrap()
            .on_hand
    };
    let credits_at_start = credits(&app);
    let mut emptied_at = None;
    let mut failed_at = None;
    for tick in 1..=400_u32 {
        app.update();
        if emptied_at.is_none()
            && turts_sim::node_view(app.world(), turts_sim::NodeId(1))
                .unwrap()
                .remaining
                == 0
        {
            emptied_at = Some(tick);
            assert_eq!(
                cargo(&app),
                2,
                "the worker holds the last load when the node empties"
            );
        }
        let current = app
            .world()
            .entity(entity_for(&app, worker))
            .get::<UnitOrders>()
            .unwrap()
            .current
            .is_some();
        if emptied_at.is_some() && !current {
            failed_at = Some(tick);
            break;
        }
    }
    let (emptied_at, failed_at) = (emptied_at.unwrap(), failed_at.unwrap());
    assert!(
        failed_at > emptied_at + 1,
        "the worker delivers its last load before its order fails"
    );
    assert_eq!(cargo(&app), 0, "the failed worker holds no cargo");
    assert_eq!(
        credits(&app),
        credits_at_start + 2,
        "the depot converts the last load"
    );
    let depleted = turts_sim::node_view(app.world(), turts_sim::NodeId(1)).unwrap();
    assert_eq!(
        depleted.remaining, 0,
        "ordinary authored gathering exhausts the real surface node"
    );
    let entity = app
        .world()
        .iter_entities()
        .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == worker))
        .unwrap();
    let orders = entity.get::<UnitOrders>().unwrap();
    let failure = orders
        .last_failure
        .as_ref()
        .expect("owner-facing worker failure survives cleanup");
    assert_eq!(
        failure.order,
        ContentId::parse("muster:order_gather").unwrap()
    );
    assert_eq!(failure.reason, RejectReason::NodeDepleted);
    (emptied_at, failed_at)
}

#[test]
fn actual_muster_attack_approaches_follows_focus_and_attack_move_resumes() {
    // Ordered Unit Attack approaches from outside weapon range, then kills the named target.
    let mut app = app_with_muster();
    let actor = 40;
    let target = 46;
    isolate_order_trial(&mut app, actor, &[target]);
    let start = Vec2::new(10.0, 30.0);
    let target_pos = Vec2::new(20.0, 30.0);
    set_pos(&mut app, actor, start);
    set_pos(&mut app, target, target_pos);
    enable_audit(&mut app);
    let result = apply(
        &mut app,
        command(
            1,
            "muster:order_attack",
            Behaviour::Attack,
            TargetKind::Unit,
            OrderTargetSim::Unit(target),
            vec![actor],
            QueueMode::Replace,
        ),
    );
    assert_eq!(result.units.accepted, vec![actor]);
    assert!(
        app.world()
            .entity(entity_for(&app, actor))
            .get::<MoveTarget>()
            .is_some(),
        "out-of-range Unit Attack installs a real approach target"
    );
    app.update();
    assert!(
        app.world()
            .entity(entity_for(&app, actor))
            .get::<Position>()
            .unwrap()
            .0
            .distance(target_pos)
            < start.distance(target_pos),
        "the ordered attacker moves toward its target"
    );
    let mut fired = drain_tick_events(&mut app);
    for _ in 0..260 {
        if health(&mut app, target).is_none() {
            break;
        }
        app.update();
        fired.extend(drain_tick_events(&mut app));
    }
    assert!(
        health(&mut app, target).is_none(),
        "the ordered Unit Attack continues until its target is gone"
    );
    assert!(
        fired.iter().any(|event| matches!(event.kind,
        turts_sim::EconomyEventKind::Fired { weapon, target: fired_target, .. }
            if weapon == actor && fired_target == target)),
        "the actual selected actor fires at its ordered target"
    );

    // A closer eligible enemy must not steal a valid in-range ordered target.
    let mut app = app_with_muster();
    let nearer = 45;
    let ordered = 46;
    isolate_order_trial(&mut app, actor, &[nearer, ordered]);
    set_pos(&mut app, actor, Vec2::new(51.8, 32.0));
    set_pos(&mut app, nearer, Vec2::new(54.0, 34.0));
    set_pos(&mut app, ordered, Vec2::new(55.2, 34.0));
    let near_health = health(&mut app, nearer).unwrap();
    let ordered_health = health(&mut app, ordered).unwrap();
    enable_audit(&mut app);
    let accepted = apply(
        &mut app,
        command(
            1,
            "muster:order_attack",
            Behaviour::Attack,
            TargetKind::Unit,
            OrderTargetSim::Unit(ordered),
            vec![actor],
            QueueMode::Replace,
        ),
    );
    assert_eq!(accepted.units.accepted, vec![actor]);
    app.update();
    let events = drain_tick_events(&mut app);
    assert!(events.iter().any(|event| matches!(event.kind,
        turts_sim::EconomyEventKind::Fired { weapon, target, .. } if weapon == actor && target == ordered)),
        "the attack uses the accepted ordered target even though another enemy is closer");
    assert_eq!(
        health(&mut app, nearer),
        Some(near_health),
        "the closer distractor remains undamaged"
    );
    assert!(health(&mut app, ordered).is_none_or(|current| current < ordered_health));

    // Authored Rush (Attack Point + StopAndFire) pauses when an enemy is in range and resumes after it dies.
    let mut app = app_with_muster();
    let enemy = 45;
    isolate_order_trial(&mut app, actor, &[enemy]);
    let actor_start = Vec2::new(50.0, 24.0);
    let enemy_pos = Vec2::new(52.0, 24.0);
    let rush_point = Vec2::new(60.0, 24.0);
    set_pos(&mut app, actor, actor_start);
    set_pos(&mut app, enemy, enemy_pos);
    let enemy_health = health(&mut app, enemy).unwrap();
    enable_audit(&mut app);
    let accepted = apply(
        &mut app,
        command(
            1,
            "muster:rush",
            Behaviour::Attack,
            TargetKind::Point,
            OrderTargetSim::Point(rush_point),
            vec![actor],
            QueueMode::Replace,
        ),
    );
    assert_eq!(accepted.units.accepted, vec![actor]);
    let mut engagement_positions = Vec::new();
    let mut resumed = false;
    let mut fired = Vec::new();
    for _ in 0..220 {
        app.update();
        let position = app
            .world()
            .entity(entity_for(&app, actor))
            .get::<Position>()
            .unwrap()
            .0;
        fired.extend(drain_tick_events(&mut app));
        if health(&mut app, enemy).is_some_and(|current| current < enemy_health) {
            engagement_positions.push(position);
        }
        if health(&mut app, enemy).is_none()
            && app
                .world()
                .entity(entity_for(&app, actor))
                .get::<MoveTarget>()
                .is_some_and(|target| target.0 == rush_point)
        {
            resumed = true;
            break;
        }
    }
    assert!(fired.iter().any(|event| matches!(event.kind,
        turts_sim::EconomyEventKind::Fired { weapon, target, .. } if weapon == actor && target == enemy)),
        "AttackMove actually fires at its in-range target");
    assert!(
        engagement_positions.len() >= 2,
        "AttackMove reaches its stop-to-fire window"
    );
    assert!(
        engagement_positions
            .iter()
            .all(|position| position.distance(engagement_positions[0]) <= 0.001),
        "AttackMove stays stationary while its in-range target remains alive"
    );
    assert!(
        resumed,
        "AttackMove resumes the authored point after the target is gone"
    );
    let resumed_from = app
        .world()
        .entity(entity_for(&app, actor))
        .get::<Position>()
        .unwrap()
        .0;
    for _ in 0..4 {
        app.update();
    }
    let resumed_to = app
        .world()
        .entity(entity_for(&app, actor))
        .get::<Position>()
        .unwrap()
        .0;
    assert!(
        resumed_to.distance(resumed_from) > 0.01
            && (resumed_to - resumed_from).dot(rush_point - resumed_from) > 0.0,
        "AttackMove resumes actual movement toward its point after the target is gone"
    );
}

#[test]
fn authored_force_move_never_fires_and_hold_remains_stationary_while_firing() {
    let actor = 40;
    let enemies: Vec<_> = (45..=54).collect();
    let mut app = app_with_muster();
    isolate_order_trial(&mut app, actor, &enemies);
    let start = Vec2::new(50.0, 24.0);
    let near = 45;
    let second = 46;
    set_pos(&mut app, actor, start);
    set_pos(&mut app, near, Vec2::new(52.0, 24.0));
    set_pos(&mut app, second, Vec2::new(53.0, 24.0));
    let initial: Vec<_> = enemies
        .iter()
        .map(|id| (*id, health(&mut app, *id).unwrap()))
        .collect();
    let weapon = turts_sim::audit_state(app.world_mut())
        .raw_weapons
        .into_iter()
        .find(|weapon| weapon.unit == actor)
        .expect("authored trooper weapon state");
    assert!(
        weapon.enabled && weapon.cooldown == 0 && weapon.effective_damage.is_ok(),
        "the force-move witness starts with a ready, enabled weapon"
    );
    assert!(
        start.distance(Vec2::new(52.0, 24.0)) < 4.0,
        "a real eligible foreign target begins inside the trooper's authored weapon range"
    );
    app.insert_resource(PhaseSamples {
        unit: actor,
        ..Default::default()
    });
    app.add_systems(
        FixedUpdate,
        sample_after_orders
            .after(SimSet::Orders)
            .before(SimSet::Movement),
    );
    app.add_systems(
        FixedUpdate,
        sample_after_movement
            .after(SimSet::Movement)
            .before(SimSet::Gather),
    );
    enable_audit(&mut app);
    let destination = Vec2::new(52.0, 24.0);
    let accepted = apply(
        &mut app,
        command(
            1,
            "muster:order_move",
            Behaviour::Move,
            TargetKind::PointOrUnit,
            OrderTargetSim::Point(destination),
            vec![actor],
            QueueMode::Replace,
        ),
    );
    assert_eq!(accepted.units.accepted, vec![actor]);
    let accepted_tick = app.world().resource::<Tick>().0;
    let mut event_ticks = Vec::new();
    let mut movement_complete_tick = None;
    for _ in 0..180 {
        app.update();
        event_ticks.extend(drain_tick_records(&mut app));
        if let Some((tick, _, _position, movement, _, _)) = app
            .world()
            .resource::<PhaseSamples>()
            .rows
            .lock()
            .unwrap()
            .iter()
            .rev()
            .find(|(_, phase, _, _, _, _)| *phase == "after_movement")
            && movement.is_none()
        {
            movement_complete_tick = Some(*tick);
            break;
        }
    }
    let movement_complete_tick =
        movement_complete_tick.expect("force Move reaches its actual point endpoint");
    let final_position = app
        .world()
        .entity(entity_for(&app, actor))
        .get::<Position>()
        .unwrap()
        .0;
    let actor_radius = match catalog().units[&ContentId::parse("muster:trooper").unwrap()].footprint
    {
        turts_mod_api::Footprint::Circle { radius } => radius,
    };
    let target_radius = match catalog().units[&ContentId::parse("muster:dummy").unwrap()].footprint
    {
        turts_mod_api::Footprint::Circle { radius } => radius,
    };
    let legal_contact = actor_radius + target_radius;
    let final_orders = app
        .world()
        .entity(entity_for(&app, actor))
        .get::<UnitOrders>()
        .unwrap();
    assert!(
        final_orders.last_failure.is_none(),
        "the point-under-enemy order did not fail or become Blocked"
    );
    assert!(
        final_position.distance(start) > 1.0,
        "authored force Move travels through an enemy's weapon range"
    );
    assert!(
        final_position.distance(destination) >= legal_contact
            && final_position.distance(destination) <= legal_contact + 0.25,
        "Move targeting the occupied enemy point stops at legal footprint contact: {final_position:?}, expected [{legal_contact}, {}]",
        legal_contact + 0.25
    );
    let actor_fires = event_ticks
        .iter()
        .filter(|(tick, _)| *tick >= accepted_tick && *tick <= movement_complete_tick)
        .flat_map(|(_, events)| events)
        .any(|event| {
            matches!(event.kind,
            turts_sim::EconomyEventKind::Fired { weapon, .. } if weapon == actor)
        });
    assert!(
        !actor_fires,
        "FirePolicy::Never suppresses actual Fired events for the forced mover"
    );
    let crossed_weapon_range = app
        .world()
        .resource::<PhaseSamples>()
        .rows
        .lock()
        .unwrap()
        .iter()
        .any(|(_, phase, position, _, _, _)| {
            *phase == "after_movement"
                && position.distance(start) > 0.5
                && position.distance(Vec2::new(52.0, 24.0)) < 4.0
        });
    assert!(
        crossed_weapon_range,
        "the armed mover travels while still within an eligible live enemy's weapon range"
    );
    for (id, current) in &initial {
        assert_eq!(
            health(&mut app, *id),
            Some(*current),
            "force Move leaves every live eligible foreign target unharmed"
        );
    }
    assert!(
        accepted_tick < movement_complete_tick,
        "the no-shot interval ends at the real movement completion tick"
    );

    let mut app = app_with_muster();
    isolate_order_trial(&mut app, actor, &[near]);
    set_pos(&mut app, actor, start);
    set_pos(&mut app, near, Vec2::new(52.0, 24.0));
    let held_position = app
        .world()
        .entity(entity_for(&app, actor))
        .get::<Position>()
        .unwrap()
        .0;
    let enemy_health = health(&mut app, near).unwrap();
    enable_audit(&mut app);
    let accepted = apply(
        &mut app,
        command(
            1,
            "muster:order_hold",
            Behaviour::Hold,
            TargetKind::None,
            OrderTargetSim::None,
            vec![actor],
            QueueMode::Replace,
        ),
    );
    assert_eq!(accepted.units.accepted, vec![actor]);
    let mut fired = Vec::new();
    for _ in 0..30 {
        app.update();
        fired.extend(drain_tick_events(&mut app));
    }
    let current_position = app
        .world()
        .entity(entity_for(&app, actor))
        .get::<Position>()
        .unwrap()
        .0;
    assert!(fired.iter().any(|event| matches!(event.kind,
        turts_sim::EconomyEventKind::Fired { weapon, target, .. } if weapon == actor && target == near)),
        "authored Hold with InRange still fires at a real target");
    assert!(health(&mut app, near).is_some_and(|current| current < enemy_health));
    assert_eq!(
        current_position, held_position,
        "Hold does not resume movement while firing"
    );
}
