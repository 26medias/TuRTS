//! Node depletion during an MC Gather order (design 5.3, M2 design 5).
//!
//! The rule: when the named node of a Gather order empties, a harvester that holds cargo
//! delivers that cargo to a receiver first. Then the order fails with `node_depleted` and the
//! harvester is idle at the receiver. A harvester with no cargo fails at once. No resource is
//! created or destroyed: node stock, cargo and receiver stock always sum to the start stock.
//!
//! Each case drives fixed ticks from a tick counter only. No case reads wall time.

use std::collections::BTreeMap;

use turts_mod_api::{
    Behaviour, ContentCatalog, ContentId, Footprint, MapDef, MapSize, NodePlacement, OrderDef,
    Placement, ResolvedDropOff, ResolvedDropOffEffect, ResolvedDropOffRule, ResolvedGatherer,
    ResolvedInventory, ResolvedNode, ResolvedUnit, ResourceDef, ResourceKind, SourceDef,
    TargetKind, WorldPos,
};
use turts_sim::{
    Command, CommandKind, GatherStage, MoveTarget, NodeId, OrderTargetSim, Position, QueueMode,
    RejectReason, SlotId, UnitId, UnitOrders, Vec2, apply_command, headless_app, install_package,
    inventory_view, node_view, order_view, orders::OrderCommand, stage_package,
};

const TICK_HZ: u16 = 20;
const DEPOT: u32 = 1;
const WORKER: u32 = 2;
const HELPER: u32 = 3;
const NODE: NodeId = NodeId(1);
const NODE_POS: Vec2 = Vec2::new(30.0, 10.0);
const DEPOT_POS: Vec2 = Vec2::new(4.0, 10.0);
const CAPACITY: u64 = 10;
const TICK_LIMIT: u32 = 2_000;

fn id(local: &str) -> ContentId {
    ContentId::parse(&format!("deplete:{local}")).unwrap()
}

fn unit(local: &str, radius: f32) -> ResolvedUnit {
    ResolvedUnit {
        id: id(local),
        name: local.into(),
        footprint: Footprint::Circle { radius },
        movement: None,
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
        health: None,
        weapon: None,
        removal: None,
    }
}

fn catalog() -> ContentCatalog {
    let ore = id("ore");
    let mut depot = unit("depot", 1.0);
    depot.inventory = Some(ResolvedInventory {
        capacity: 1_000,
        accepts: vec![ore.clone()],
    });
    depot.drop_off = Some(ResolvedDropOff {
        rules: vec![ResolvedDropOffRule {
            item: ore.clone(),
            effect: ResolvedDropOffEffect::Store,
        }],
    });
    let mut worker = unit("worker", 0.5);
    worker.movement = Some(turts_mod_api::MovementDef { speed: 4.0 });
    worker.inventory = Some(ResolvedInventory {
        capacity: CAPACITY,
        accepts: vec![ore.clone()],
    });
    worker.gatherer = Some(ResolvedGatherer {
        items: vec![ore.clone()],
        gather_per_second: 20,
        unload_per_second: 20,
        interaction_range: 0.5,
    });
    ContentCatalog {
        resources: BTreeMap::from([(
            ore.clone(),
            ResourceDef {
                id: "ore".into(),
                name: "Ore".into(),
                unit_name: "ore".into(),
                kind: ResourceKind::Item,
            },
        )]),
        units: BTreeMap::from([(id("depot"), depot), (id("worker"), worker)]),
        nodes: BTreeMap::from([(
            id("deposit"),
            ResolvedNode {
                id: id("deposit"),
                name: "Deposit".into(),
                item: ore,
                radius: 1.0,
            },
        )]),
        production: BTreeMap::new(),
        recipes: BTreeMap::new(),
        technologies: BTreeMap::new(),
        orders: BTreeMap::from([(
            id("gather"),
            OrderDef {
                id: "gather".into(),
                name: "Gather".into(),
                behaviour: Behaviour::Gather,
                target: TargetKind::Node,
                fire: None,
                engage: None,
                icon: String::new(),
                hotkey: String::new(),
                cursor: None,
                card: None,
                units: Vec::new(),
                description: String::new(),
            },
        )]),
        order_queue_depths: BTreeMap::new(),
        source_files: BTreeMap::new(),
    }
}

fn placement(local: &str, x: f32) -> Placement {
    Placement {
        unit: format!("deplete:{local}"),
        owner_slot: 0,
        pos: WorldPos { x, y: 10.0 },
        contents: Vec::new(),
        facing: None,
    }
}

/// Boot the fixture. The worker starts in reach of the node. The helper, when present,
/// starts east of the node at `helper_x` and arrives later, so it empties the node in a
/// later stage of the worker cycle.
fn app(node_stock: u64, helper_x: Option<f32>) -> bevy_app::App {
    let mut placements = vec![placement("depot", DEPOT_POS.x), placement("worker", 28.0)];
    if let Some(x) = helper_x {
        placements.push(placement("worker", x));
    }
    let map = MapDef {
        id: "strip".into(),
        size: MapSize { x: 80.0, y: 20.0 },
        placements,
        nodes: vec![NodePlacement {
            node: "deplete:deposit".into(),
            pos: WorldPos {
                x: NODE_POS.x,
                y: NODE_POS.y,
            },
            remaining: node_stock,
        }],
        slot_economy: Vec::new(),
        belts: Vec::new(),
    };
    let mut app = headless_app(TICK_HZ);
    let staged = stage_package(
        app.world(),
        catalog(),
        SourceDef {
            file: "maps/strip.ron".into(),
            value: map,
        },
        TICK_HZ,
    )
    .unwrap();
    install_package(app.world_mut(), staged);
    for (unit_id, x) in [(DEPOT, DEPOT_POS.x), (WORKER, 28.0)]
        .into_iter()
        .chain(helper_x.map(|x| (HELPER, x)))
    {
        assert_eq!(
            position(&app, unit_id),
            Vec2::new(x, 10.0),
            "fixture unit {unit_id} has its placement position"
        );
    }
    app
}

fn gather(app: &mut bevy_app::App, seq: u32, unit_id: u32) {
    let command = Command {
        slot: SlotId(0),
        seq,
        kind: CommandKind::Order(Box::new(OrderCommand {
            order: id("gather"),
            behaviour: Behaviour::Gather,
            target_kind: TargetKind::Node,
            policy: None,
            units: vec![unit_id],
            target: OrderTargetSim::Node(NODE),
            queue: QueueMode::Replace,
            group: 0,
            allowed_units: Vec::new(),
            queue_depth: 16,
        })),
    };
    let result = apply_command(app.world_mut(), &command);
    assert_eq!(result.units.accepted, vec![unit_id], "{result:?}");
}

fn entity(app: &bevy_app::App, unit_id: u32) -> bevy_ecs::world::EntityRef<'_> {
    app.world()
        .iter_entities()
        .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == unit_id))
        .unwrap()
}

fn position(app: &bevy_app::App, unit_id: u32) -> Vec2 {
    entity(app, unit_id).get::<Position>().unwrap().0
}

fn stock(app: &bevy_app::App, unit_id: u32) -> u64 {
    inventory_view(app.world(), SlotId(0), unit_id)
        .unwrap()
        .items
        .iter()
        .map(|item| item.on_hand)
        .sum()
}

fn node_stock(app: &bevy_app::App) -> u64 {
    node_view(app.world(), NODE).unwrap().remaining
}

/// One observation of the worker.
#[derive(Debug, Clone, PartialEq)]
struct Seen {
    tick: u32,
    stage: GatherStage,
    has_current_order: bool,
    failure: Option<(RejectReason, u32)>,
    cargo: u64,
    depot: u64,
    moving: bool,
    position: Vec2,
}

fn seen(app: &bevy_app::App, tick: u32) -> Seen {
    let worker = entity(app, WORKER);
    let orders = worker.get::<UnitOrders>().unwrap();
    Seen {
        tick,
        stage: order_view(app.world(), SlotId(0), WORKER).unwrap().stage,
        has_current_order: orders.current.is_some(),
        failure: orders
            .last_failure
            .as_ref()
            .map(|failure| (failure.reason, failure.tick)),
        cargo: stock(app, WORKER),
        depot: stock(app, DEPOT),
        moving: worker.get::<MoveTarget>().is_some(),
        position: position(app, WORKER),
    }
}

/// What one case observed: the worker at the tick the node empties, and the worker at the
/// tick after the order ended (or at the tick limit).
#[derive(Debug)]
struct Outcome {
    at_depletion: Seen,
    at_end: Seen,
}

fn run_case(node_stock_at_start: u64, helper_x: Option<f32>) -> Outcome {
    let mut app = app(node_stock_at_start, helper_x);
    let start_total = node_stock(&app) + stock(&app, DEPOT) + stock(&app, WORKER);
    gather(&mut app, 1, WORKER);
    if helper_x.is_some() {
        gather(&mut app, 2, HELPER);
    }
    let mut at_depletion = None;
    let mut at_end = None;
    for tick in 1..=TICK_LIMIT {
        app.update();
        let helper_cargo = if helper_x.is_some() {
            stock(&app, HELPER)
        } else {
            0
        };
        assert_eq!(
            node_stock(&app) + stock(&app, DEPOT) + stock(&app, WORKER) + helper_cargo,
            start_total,
            "tick {tick}: no ore is created or destroyed"
        );
        let now = seen(&app, tick);
        if at_depletion.is_none() && node_stock(&app) == 0 {
            at_depletion = Some(now.clone());
        }
        if at_depletion.is_some() && !now.has_current_order {
            at_end = Some(now);
            break;
        }
    }
    let at_depletion = at_depletion.expect("the node empties inside the tick limit");
    let at_end = at_end.unwrap_or_else(|| seen(&app, TICK_LIMIT));
    eprintln!("depletion case node={node_stock_at_start} helper={helper_x:?}");
    eprintln!("    at depletion: {at_depletion:?}");
    eprintln!("    at end:       {at_end:?}");
    Outcome {
        at_depletion,
        at_end,
    }
}

/// The receiver approach: the worker rests within its reach of the depot.
fn at_receiver(position: Vec2) -> bool {
    position.distance(DEPOT_POS) <= 0.5 + 1.0 + 0.5 + 1.0e-3
}

fn assert_delivered_then_failed(case: &str, outcome: &Outcome, cargo_at_depletion: u64) {
    assert_eq!(
        outcome.at_depletion.cargo, cargo_at_depletion,
        "{case}: the worker holds cargo when the node empties"
    );
    assert!(
        outcome
            .at_end
            .failure
            .is_some_and(|(reason, tick)| reason == RejectReason::NodeDepleted
                && tick == outcome.at_end.tick),
        "{case}: the order fails with node_depleted: {outcome:?}"
    );
    assert_eq!(
        outcome.at_end.cargo, 0,
        "{case}: the worker delivers all its cargo before the order fails"
    );
    assert_eq!(
        outcome.at_end.depot,
        outcome.at_depletion.depot + cargo_at_depletion,
        "{case}: the receiver gets the cargo that the worker held when the node emptied"
    );
    assert_eq!(outcome.at_end.stage, GatherStage::Idle, "{case}");
    assert!(
        !outcome.at_end.moving,
        "{case}: the idle worker has no move"
    );
    assert!(
        at_receiver(outcome.at_end.position),
        "{case}: the worker is idle at the receiver: {outcome:?}"
    );
}

/// Case (a): the worker empties the node itself. The last extraction leaves a partial load.
#[test]
fn worker_that_empties_the_node_delivers_its_partial_load_then_fails() {
    let outcome = run_case(5, None);
    assert_eq!(
        outcome.at_depletion.stage,
        GatherStage::MovingToReceiver,
        "case a: the node empties while the worker is at the node: {outcome:?}"
    );
    assert!(outcome.at_depletion.position.distance(NODE_POS) <= 2.0 + 1.0e-3);
    assert_delivered_then_failed("case a", &outcome, 5);
}

/// Case (a'): the last extraction fills the cargo and empties the node in the same tick.
/// This is the E9 base trace: 1,000 stock and 100 capacity.
#[test]
fn worker_whose_last_full_load_empties_the_node_delivers_it_then_fails() {
    let outcome = run_case(CAPACITY, None);
    assert_eq!(
        outcome.at_depletion.stage,
        GatherStage::MovingToReceiver,
        "case a': {outcome:?}"
    );
    assert_delivered_then_failed("case a'", &outcome, CAPACITY);
}

/// Case (b): a helper empties the node while the worker carries a full load to the depot.
#[test]
fn worker_on_its_way_to_the_receiver_delivers_its_load_then_fails() {
    let outcome = run_case(CAPACITY + 2, Some(44.0));
    assert_eq!(
        outcome.at_depletion.stage,
        GatherStage::MovingToReceiver,
        "case b: the node empties while the worker travels to the receiver: {outcome:?}"
    );
    assert!(
        !at_receiver(outcome.at_depletion.position)
            && outcome.at_depletion.position.distance(NODE_POS) > 2.5,
        "case b: the worker is between the node and the receiver: {outcome:?}"
    );
    assert_delivered_then_failed("case b", &outcome, CAPACITY);
}

/// Case (c): a helper empties the node while the worker returns to it with no cargo.
#[test]
fn worker_on_its_way_back_with_no_cargo_fails_at_once_where_it_is() {
    let outcome = run_case(CAPACITY + 2, Some(68.0));
    assert_eq!(
        outcome.at_depletion.stage,
        GatherStage::MovingToNode,
        "case c: the node empties while the worker returns to it: {outcome:?}"
    );
    assert_eq!(outcome.at_depletion.cargo, 0, "case c: {outcome:?}");
    assert_eq!(
        outcome.at_end.tick,
        outcome.at_depletion.tick + 1,
        "case c: the next Orders phase ends the order: {outcome:?}"
    );
    assert_eq!(
        outcome.at_end.failure,
        Some((RejectReason::NodeDepleted, outcome.at_end.tick))
    );
    assert_eq!(outcome.at_end.stage, GatherStage::Idle);
    assert_eq!(outcome.at_end.cargo, 0);
    assert_eq!(outcome.at_end.depot, outcome.at_depletion.depot);
    assert!(
        !outcome.at_end.moving,
        "case c: the worker stops where it is"
    );
    assert!(
        outcome
            .at_end
            .position
            .distance(outcome.at_depletion.position)
            <= 0.2 + 1.0e-3,
        "case c: the worker moves at most one tick of travel after the node empties: {outcome:?}"
    );
}
