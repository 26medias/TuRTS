//! Node depletion during an MC Gather order (design 5.3, M2 design 5).
//!
//! The rule: when the named node of a Gather order empties, a harvester that holds cargo
//! delivers that cargo to a receiver first. Then the order fails with `node_depleted` and the
//! harvester is idle at the receiver. A harvester with no cargo fails at once. A gather trip
//! that a unit blocks does not fail: the cycle waits with `Waiting(Blocked)` and tries the
//! approach again. With no receiver, the cycle keeps its cargo and waits. No resource is
//! created or destroyed: node stock, cargo and receiver stock always sum to the start stock.
//!
//! Each case drives fixed ticks from a tick counter only. No case reads wall time.

use std::collections::{BTreeMap, BTreeSet};

use turts_mod_api::{
    Behaviour, ContentCatalog, ContentId, ContentsPolicy, Footprint, MapDef, MapSize,
    NodePlacement, OrderDef, Placement, RemovalDef, ResolvedDropOff, ResolvedDropOffEffect,
    ResolvedDropOffRule, ResolvedGatherer, ResolvedInventory, ResolvedNode, ResolvedUnit,
    ResourceDef, ResourceKind, SourceDef, TargetKind, WorldPos,
};
use turts_sim::{
    Command, CommandKind, GatherStage, MoveTarget, NodeId, OrderTargetSim, Position, QueueMode,
    RejectReason, SlotId, UnitId, UnitOrders, Vec2, WaitReason, apply_command, headless_app,
    install_package, inventory_view, node_view, order_view, orders::OrderCommand, stage_package,
};

const TICK_HZ: u16 = 20;
const DEPOT: u32 = 1;
const WORKER: u32 = 2;
const HELPER: u32 = 3;
const NODE: NodeId = NodeId(1);
const NODE_POS: Vec2 = Vec2::new(30.0, 10.0);
const DEPOT_POS: Vec2 = Vec2::new(4.0, 10.0);
/// The approach point of the worker at the depot: depot radius 1.0 + worker radius 0.5 +
/// interaction range 0.5, on the line from the depot to the node.
const DEPOT_APPROACH: Vec2 = Vec2::new(6.0, 10.0);
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

fn order_def(local: &str, behaviour: Behaviour, target: TargetKind) -> OrderDef {
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
        units: Vec::new(),
        description: String::new(),
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
    depot.removal = Some(RemovalDef {
        by_command: true,
        contents: ContentsPolicy::Destroy,
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
        orders: BTreeMap::from([
            (
                id("gather"),
                order_def("gather", Behaviour::Gather, TargetKind::Node),
            ),
            (
                id("stop"),
                order_def("stop", Behaviour::Stop, TargetKind::None),
            ),
            (
                id("move"),
                order_def("move", Behaviour::Move, TargetKind::Point),
            ),
        ]),
        order_queue_depths: BTreeMap::new(),
        source_files: BTreeMap::new(),
    }
}

fn placement(local: &str, at: Vec2) -> Placement {
    Placement {
        unit: format!("deplete:{local}"),
        owner_slot: 0,
        pos: WorldPos { x: at.x, y: at.y },
        contents: Vec::new(),
        facing: None,
    }
}

/// One fixture. The depot is unit 1 and the worker is unit 2; the worker starts in reach of
/// the node. The helper, when present, is unit 3: a second gatherer that starts east of the
/// node at `helper_x` and arrives later. The blocker, when present, is the next unit: an idle
/// worker with no order that stands at `blocker`.
#[derive(Clone, Copy)]
struct Scene {
    node_stock: u64,
    helper_x: Option<f32>,
    blocker: Option<Vec2>,
    /// Remove the depot by command on the tick the node empties.
    remove_depot_at_depletion: bool,
}

impl Scene {
    const fn node(node_stock: u64) -> Self {
        Self {
            node_stock,
            helper_x: None,
            blocker: None,
            remove_depot_at_depletion: false,
        }
    }

    fn gatherers(self) -> Vec<u32> {
        let mut units = vec![WORKER];
        if self.helper_x.is_some() {
            units.push(HELPER);
        }
        units
    }
}

fn app(scene: Scene) -> bevy_app::App {
    let mut units = vec![Vec2::new(28.0, 10.0)];
    units.extend(scene.helper_x.map(|x| Vec2::new(x, 10.0)));
    units.extend(scene.blocker);
    app_with(scene.node_stock, &units)
}

/// The depot is unit 1; `units[i]` is a worker with id i + 2.
fn app_with(node_stock: u64, units: &[Vec2]) -> bevy_app::App {
    let mut placements = vec![placement("depot", DEPOT_POS)];
    placements.extend(units.iter().map(|at| placement("worker", *at)));
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
    assert_eq!(position(&app, DEPOT), DEPOT_POS, "fixture depot position");
    for (unit_id, at) in (WORKER..).zip(units) {
        assert_eq!(
            position(&app, unit_id),
            *at,
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

fn entity(app: &bevy_app::App, unit_id: u32) -> Option<bevy_ecs::world::EntityRef<'_>> {
    app.world()
        .iter_entities()
        .find(|entity| entity.get::<UnitId>().is_some_and(|id| id.0 == unit_id))
}

fn position(app: &bevy_app::App, unit_id: u32) -> Vec2 {
    entity(app, unit_id).unwrap().get::<Position>().unwrap().0
}

/// The stock of a unit inventory, or 0 when the unit is gone.
fn stock(app: &bevy_app::App, unit_id: u32) -> u64 {
    inventory_view(app.world(), SlotId(0), unit_id).map_or(0, |inventory| {
        inventory.items.iter().map(|item| item.on_hand).sum()
    })
}

fn node_stock(app: &bevy_app::App) -> u64 {
    node_view(app.world(), NODE).unwrap().remaining
}

/// One observation of one gatherer.
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

fn seen(app: &bevy_app::App, unit_id: u32, tick: u32) -> Seen {
    let unit = entity(app, unit_id).unwrap();
    let orders = unit.get::<UnitOrders>().unwrap();
    Seen {
        tick,
        stage: order_view(app.world(), SlotId(0), unit_id).unwrap().stage,
        has_current_order: orders.current.is_some(),
        failure: orders
            .last_failure
            .as_ref()
            .map(|failure| (failure.reason, failure.tick)),
        cargo: stock(app, unit_id),
        depot: stock(app, DEPOT),
        moving: unit.get::<MoveTarget>().is_some(),
        position: position(app, unit_id),
    }
}

/// What one case observed: the worker at the tick the node empties; each gatherer at the tick
/// its order ended (or at the tick limit); the worker stages in between.
#[derive(Debug)]
struct Outcome {
    at_depletion: Seen,
    at_end: Seen,
    helper_at_end: Option<Seen>,
    worker_stages: Vec<GatherStage>,
}

fn run_case(scene: Scene) -> Outcome {
    let mut app = app(scene);
    let total = |app: &bevy_app::App| {
        node_stock(app)
            + stock(app, DEPOT)
            + scene
                .gatherers()
                .iter()
                .map(|unit| stock(app, *unit))
                .sum::<u64>()
    };
    let start_total = total(&app);
    for (seq, unit) in (1..).zip(scene.gatherers()) {
        gather(&mut app, seq, unit);
    }
    let mut at_depletion = None;
    let mut ended: BTreeMap<u32, Seen> = BTreeMap::new();
    let mut worker_stages = Vec::new();
    for tick in 1..=TICK_LIMIT {
        app.update();
        if !(scene.remove_depot_at_depletion && at_depletion.is_some()) {
            assert_eq!(
                total(&app),
                start_total,
                "tick {tick}: no ore is created or destroyed"
            );
        }
        let now = seen(&app, WORKER, tick);
        if worker_stages.last() != Some(&now.stage) {
            worker_stages.push(now.stage);
        }
        if at_depletion.is_none() && node_stock(&app) == 0 {
            assert!(
                !scene.remove_depot_at_depletion || stock(&app, DEPOT) == 0,
                "the removed depot holds no ore"
            );
            at_depletion = Some(now.clone());
            if scene.remove_depot_at_depletion {
                let result = apply_command(
                    app.world_mut(),
                    &Command {
                        slot: SlotId(0),
                        seq: 9,
                        kind: CommandKind::Remove { unit_id: DEPOT },
                    },
                );
                assert!(result.outcome.is_ok(), "{result:?}");
                assert!(entity(&app, DEPOT).is_none(), "the depot is gone");
            }
        }
        if at_depletion.is_some() {
            for unit in scene.gatherers() {
                let unit_now = seen(&app, unit, tick);
                if !unit_now.has_current_order {
                    ended.entry(unit).or_insert(unit_now);
                }
            }
            if ended.len() == scene.gatherers().len() {
                break;
            }
        }
    }
    let at_depletion = at_depletion.expect("the node empties inside the tick limit");
    let at_end = ended
        .get(&WORKER)
        .cloned()
        .unwrap_or_else(|| seen(&app, WORKER, TICK_LIMIT));
    let helper_at_end = scene.helper_x.map(|_| {
        ended
            .get(&HELPER)
            .cloned()
            .unwrap_or_else(|| seen(&app, HELPER, TICK_LIMIT))
    });
    eprintln!(
        "depletion case node={} helper={:?} blocker={:?} remove_depot={}",
        scene.node_stock, scene.helper_x, scene.blocker, scene.remove_depot_at_depletion
    );
    eprintln!("    at depletion: {at_depletion:?}");
    eprintln!("    at end:       {at_end:?}");
    eprintln!("    helper end:   {helper_at_end:?}");
    eprintln!("    stages:       {worker_stages:?}");
    Outcome {
        at_depletion,
        at_end,
        helper_at_end,
        worker_stages,
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
    assert!(
        outcome.at_end.depot >= outcome.at_depletion.depot + cargo_at_depletion,
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

/// The helper empties the node itself, so it holds the last load. It must deliver that load,
/// then fail with node_depleted, exactly as the worker does.
fn assert_helper_delivered_then_failed(case: &str, outcome: &Outcome) {
    let helper = outcome
        .helper_at_end
        .as_ref()
        .expect("the case has a helper");
    assert!(
        helper.failure.is_some_and(
            |(reason, tick)| reason == RejectReason::NodeDepleted && tick == helper.tick
        ),
        "{case}: the helper order fails with node_depleted: {outcome:?}"
    );
    assert_eq!(helper.cargo, 0, "{case}: the helper delivers its cargo");
    assert_eq!(helper.stage, GatherStage::Idle, "{case}");
    assert!(
        at_receiver(helper.position),
        "{case}: the helper is idle at the receiver: {outcome:?}"
    );
}

/// Case (a): the worker empties the node itself. The last extraction leaves a partial load.
#[test]
fn worker_that_empties_the_node_delivers_its_partial_load_then_fails() {
    let outcome = run_case(Scene::node(5));
    assert_eq!(
        outcome.at_depletion.stage,
        GatherStage::MovingToReceiver,
        "case a: the node empties while the worker is at the node: {outcome:?}"
    );
    assert!(outcome.at_depletion.position.distance(NODE_POS) <= 2.0 + 1.0e-3);
    assert_delivered_then_failed("case a", &outcome, 5);
    assert_eq!(outcome.at_end.depot, 5);
}

/// Case (a'): the last extraction fills the cargo and empties the node in the same tick.
/// This is the E9 base trace: 1,000 stock and 100 capacity.
#[test]
fn worker_whose_last_full_load_empties_the_node_delivers_it_then_fails() {
    let outcome = run_case(Scene::node(CAPACITY));
    assert_eq!(
        outcome.at_depletion.stage,
        GatherStage::MovingToReceiver,
        "case a': {outcome:?}"
    );
    assert_delivered_then_failed("case a'", &outcome, CAPACITY);
    assert_eq!(outcome.at_end.depot, CAPACITY);
}

/// Case (b): a helper empties the node while the worker carries a full load to the depot.
#[test]
fn worker_on_its_way_to_the_receiver_delivers_its_load_then_fails() {
    let outcome = run_case(Scene {
        helper_x: Some(44.0),
        ..Scene::node(CAPACITY + 2)
    });
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
    assert_helper_delivered_then_failed("case b", &outcome);
    assert_eq!(
        stock_of_end(&outcome),
        CAPACITY + 2,
        "case b: the depot holds all ore"
    );
}

/// The depot stock after both gatherers ended: the later of the two observations.
fn stock_of_end(outcome: &Outcome) -> u64 {
    outcome
        .helper_at_end
        .as_ref()
        .map_or(outcome.at_end.depot, |helper| {
            helper.depot.max(outcome.at_end.depot)
        })
}

/// Case (c): a helper empties the node while the worker returns to it with no cargo.
#[test]
fn worker_on_its_way_back_with_no_cargo_fails_at_once_where_it_is() {
    let outcome = run_case(Scene {
        helper_x: Some(68.0),
        ..Scene::node(CAPACITY + 2)
    });
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
    assert_helper_delivered_then_failed("case c", &outcome);
    assert_eq!(stock_of_end(&outcome), CAPACITY + 2);
}

/// Case (d): an idle worker stands on the approach point of the depot: its footprint (radius
/// 0.5, centre 0.2 short of the point on the approach line) covers the point. The old build
/// fails this trip with `Blocked` at tick 117 and the worker keeps its 5 ore. The blocked trip
/// must not fail: the cycle waits with Waiting(Blocked), tries another approach point,
/// delivers, and then the order fails with node_depleted.
#[test]
fn worker_whose_receiver_approach_is_blocked_waits_then_delivers_then_fails() {
    let outcome = run_case(Scene {
        blocker: Some(DEPOT_APPROACH - Vec2::new(0.2, 0.0)),
        ..Scene::node(5)
    });
    assert!(
        outcome.worker_stages.contains(&GatherStage::Waiting {
            reason: WaitReason::Blocked
        }),
        "case d: the blocked trip waits visibly: {outcome:?}"
    );
    assert_delivered_then_failed("case d", &outcome, 5);
    assert_eq!(outcome.at_end.depot, 5, "case d: the depot holds the ore");
}

/// Case (e): the receiver is gone. The cycle keeps its cargo and waits with
/// Waiting(NoReceiver); the order does not end (M2 design 5: never discard cargo).
#[test]
fn worker_whose_receiver_is_gone_keeps_its_cargo_and_waits() {
    let outcome = run_case(Scene {
        remove_depot_at_depletion: true,
        ..Scene::node(5)
    });
    assert_eq!(outcome.at_depletion.cargo, 5, "case e: {outcome:?}");
    assert_eq!(
        outcome.at_end.tick, TICK_LIMIT,
        "case e: the order never ends"
    );
    assert!(outcome.at_end.has_current_order, "case e: {outcome:?}");
    assert_eq!(outcome.at_end.failure, None, "case e: {outcome:?}");
    assert_eq!(
        outcome.at_end.stage,
        GatherStage::Waiting {
            reason: WaitReason::NoReceiver
        },
        "case e: {outcome:?}"
    );
    assert_eq!(
        outcome.at_end.cargo, 5,
        "case e: the worker keeps its cargo"
    );
}

// ---------------------------------------------------------------------------------------
// Blocked trips: the retry, the free approach point, Stop and Move during the wait, and the
// node side. All of these drive a gatherer whose approach a unit footprint covers.
// ---------------------------------------------------------------------------------------

const BLOCKED: GatherStage = GatherStage::Waiting {
    reason: WaitReason::Blocked,
};

fn stage_of(app: &bevy_app::App, unit_id: u32) -> GatherStage {
    order_view(app.world(), SlotId(0), unit_id).unwrap().stage
}

/// Order one unit with a plain behaviour that has the registered content id `local`.
fn order(
    app: &mut bevy_app::App,
    seq: u32,
    unit_id: u32,
    (local, behaviour, kind): (&str, Behaviour, TargetKind),
    target: OrderTargetSim,
) {
    let command = Command {
        slot: SlotId(0),
        seq,
        kind: CommandKind::Order(Box::new(OrderCommand {
            order: id(local),
            behaviour,
            target_kind: kind,
            policy: None,
            units: vec![unit_id],
            target,
            queue: QueueMode::Replace,
            group: 0,
            allowed_units: Vec::new(),
            queue_depth: 16,
        })),
    };
    let result = apply_command(app.world_mut(), &command);
    assert_eq!(result.units.accepted, vec![unit_id], "{result:?}");
}

/// Points on a circle of `radius` around `center`, 360 / `count` degrees apart, from angle 0.
/// The k-th point has unit id `first_id + k` in a fixture whose units are listed in this order.
fn ring(center: Vec2, radius: f32, count: u32) -> Vec<Vec2> {
    (0..count)
        .map(|k| {
            let angle = f64::from(k) * std::f64::consts::TAU / f64::from(count);
            center + Vec2::new(angle.cos() as f32, angle.sin() as f32) * radius
        })
        .collect()
}

/// The worker is unit 2. Units 3 to 10 are idle workers on the reach ring of the depot (radius
/// 2.0): they cover the direct approach point and all 16 candidate points of the worker.
fn ringed_depot(node_stock: u64) -> bevy_app::App {
    let mut units = vec![Vec2::new(28.0, 10.0)];
    units.extend(ring(DEPOT_POS, 2.0, 8));
    app_with(node_stock, &units)
}

/// Drive ticks until `stop(app)` holds. Fail at `TICK_LIMIT` ticks.
fn run_until(app: &mut bevy_app::App, case: &str, stop: impl Fn(&bevy_app::App) -> bool) {
    for _ in 1..=TICK_LIMIT {
        app.update();
        if stop(app) {
            return;
        }
    }
    panic!("{case}: the condition never held in {TICK_LIMIT} ticks");
}

/// The ringed depot, one Gather order, run until the worker waits with `Waiting(Blocked)`
/// while it holds its 5 ore.
fn ringed_and_waiting() -> bevy_app::App {
    let mut app = ringed_depot(5);
    gather(&mut app, 1, WORKER);
    run_until(&mut app, "ringed depot", |app| {
        stage_of(app, WORKER) == BLOCKED
    });
    assert_eq!(stock(&app, WORKER), 5, "the waiting worker holds its cargo");
    app
}

/// Drive ticks until every unit of `units` has no current order. Return each unit as it was on
/// the tick its order ended, and the set of units that were in `Waiting(Blocked)` on a tick.
fn run_until_ended(
    app: &mut bevy_app::App,
    case: &str,
    units: &[u32],
) -> (BTreeMap<u32, Seen>, BTreeSet<u32>) {
    let mut ended = BTreeMap::new();
    let mut waited = BTreeSet::new();
    for tick in 1..=TICK_LIMIT {
        app.update();
        for unit in units {
            if stage_of(app, *unit) == BLOCKED {
                waited.insert(*unit);
            }
            let now = seen(app, *unit, tick);
            if !now.has_current_order {
                ended.entry(*unit).or_insert(now);
            }
        }
        if ended.len() == units.len() {
            return (ended, waited);
        }
    }
    panic!("{case}: not every order ended in {TICK_LIMIT} ticks: ended {ended:?}");
}

fn assert_node_depleted_with_no_cargo(case: &str, end: &Seen) {
    assert_eq!(
        end.failure.map(|(reason, _)| reason),
        Some(RejectReason::NodeDepleted),
        "{case}: {end:?}"
    );
    assert_eq!(
        end.cargo, 0,
        "{case}: the unit delivered its cargo: {end:?}"
    );
}

/// Finding 1 of the re-review: two gatherers fill on the same tick and walk to one depot whose
/// direct approach point a third, idle unit covers. A blocked trip must pick the nearest free
/// point on the reach ring (`free_approach`), not the covered direct point and not a covered
/// ring point. Both orders end with `node_depleted` and the depot holds all 20 ore.
#[test]
fn two_gatherers_and_one_blocker_on_the_depot_approach_both_deliver_then_fail() {
    let units = [
        Vec2::new(28.0, 10.0),
        Vec2::new(32.0, 10.0),
        DEPOT_APPROACH - Vec2::new(0.2, 0.0),
    ];
    let mut app = app_with(2 * CAPACITY, &units);
    gather(&mut app, 1, 2);
    gather(&mut app, 2, 3);
    let (ended, waited) = run_until_ended(&mut app, "two gatherers", &[2, 3]);
    assert!(
        !waited.is_empty(),
        "the blocker makes at least one trip wait: {ended:?}"
    );
    for unit in [2, 3] {
        assert_node_depleted_with_no_cargo(&format!("unit {unit}"), &ended[&unit]);
    }
    assert_eq!(
        stock(&app, DEPOT),
        2 * CAPACITY,
        "the depot holds all the ore"
    );
}

/// The blocker leaves while the worker waits. The trip must resume at the next retry, deliver,
/// and end the order with `node_depleted`.
#[test]
fn blocked_trip_resumes_when_the_blocker_leaves() {
    let mut app = ringed_and_waiting();
    // Unit 3 is ring point 0: it stands on the direct approach point of the worker.
    assert_eq!(position(&app, 3), DEPOT_APPROACH);
    let away = Vec2::new(12.0, 17.0);
    let result = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 5,
            kind: CommandKind::MoveTo {
                unit_ids: vec![3],
                target: away,
            },
        },
    );
    assert!(result.outcome.is_ok(), "{result:?}");
    // The blocker needs about 5 ticks to clear the point; the retry period is 10 ticks.
    let mut left_the_wait = None;
    for tick in 1..=30_u32 {
        app.update();
        if stage_of(&app, WORKER) != BLOCKED {
            left_the_wait = Some(tick);
            break;
        }
    }
    assert!(
        left_the_wait.is_some(),
        "the worker leaves Waiting(Blocked) within 30 ticks of the blocker leaving"
    );
    let (ended, _) = run_until_ended(&mut app, "blocker left", &[WORKER]);
    assert_node_depleted_with_no_cargo("blocker left", &ended[&WORKER]);
    assert_eq!(ended[&WORKER].depot, 5, "the depot holds the ore");
}

/// Stop during the wait: the worker is Idle at once, keeps its cargo, has no move, and the wait
/// does not come back.
#[test]
fn stop_during_the_blocked_wait_leaves_the_worker_idle_with_its_cargo() {
    let mut app = ringed_and_waiting();
    order(
        &mut app,
        7,
        WORKER,
        ("stop", Behaviour::Stop, TargetKind::None),
        OrderTargetSim::None,
    );
    let at_stop = position(&app, WORKER);
    for tick in 1..=300_u32 {
        app.update();
        let now = seen(&app, WORKER, tick);
        assert_eq!(now.stage, GatherStage::Idle, "tick {tick}: {now:?}");
        assert!(!now.has_current_order, "tick {tick}: {now:?}");
        assert!(!now.moving, "tick {tick}: no move after Stop: {now:?}");
        assert_eq!(now.position, at_stop, "tick {tick}: the worker stays");
        assert_eq!(now.cargo, 5, "tick {tick}: the worker keeps its cargo");
    }
}

/// Move during the wait: the worker walks to the goal with its cargo and never waits again.
#[test]
fn move_during_the_blocked_wait_reaches_the_goal_and_never_waits_again() {
    let mut app = ringed_and_waiting();
    let goal = Vec2::new(40.0, 15.0);
    order(
        &mut app,
        7,
        WORKER,
        ("move", Behaviour::Move, TargetKind::Point),
        OrderTargetSim::Point(goal),
    );
    for tick in 1..=600_u32 {
        app.update();
        assert_ne!(
            stage_of(&app, WORKER),
            BLOCKED,
            "tick {tick}: a Move never waits as a gather"
        );
    }
    let now = seen(&app, WORKER, 600);
    assert!(
        now.position.distance(goal) < 0.6,
        "the worker reaches the goal: {now:?}"
    );
    assert_eq!(now.stage, GatherStage::Idle, "{now:?}");
    assert!(!now.has_current_order, "{now:?}");
    assert_eq!(now.cargo, 5, "the worker keeps its cargo: {now:?}");
}

/// The node side: eight idle units cover the reach ring of the node while the worker unloads
/// its first load, so the trip back TO the node is blocked. The worker has no cargo, waits with
/// `Waiting(Blocked)`, and when one ring unit leaves, the retry resumes the trip. The order
/// then ends with `node_depleted` and the depot holds all 15 ore.
#[test]
fn blocked_trip_to_the_node_waits_and_retries() {
    let mut units = vec![Vec2::new(28.0, 10.0)];
    units.extend((0..8).map(|k| Vec2::new(40.0 + 4.0 * k as f32, 18.0)));
    let mut app = app_with(15, &units);
    gather(&mut app, 1, WORKER);
    let spots = ring(NODE_POS, 2.0, 8);
    run_until(&mut app, "first unload", |app| {
        stage_of(app, WORKER) == GatherStage::Unloading
    });
    for (unit, spot) in (3..).zip(&spots) {
        let entity = entity(&app, unit).unwrap().id();
        app.world_mut().entity_mut(entity).insert(Position(*spot));
    }
    run_until(&mut app, "node side wait", |app| {
        stage_of(app, WORKER) == BLOCKED
    });
    assert_eq!(stock(&app, WORKER), 0, "the node-side wait holds no cargo");
    assert!(node_stock(&app) > 0, "the node still holds ore");
    // Ring point 4 is the west point, the direct point of a worker that comes from the depot.
    let entity = entity(&app, 3 + 4).unwrap().id();
    app.world_mut()
        .entity_mut(entity)
        .insert(Position(Vec2::new(70.0, 2.0)));
    let (ended, _) = run_until_ended(&mut app, "node side", &[WORKER]);
    assert_node_depleted_with_no_cargo("node side", &ended[&WORKER]);
    assert_eq!(stock(&app, DEPOT), 15, "the depot holds all the ore");
}
