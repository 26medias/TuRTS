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

use std::collections::BTreeMap;

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

    fn blocker_id(self) -> Option<u32> {
        self.blocker
            .map(|_| if self.helper_x.is_some() { 4 } else { 3 })
    }
}

fn app(scene: Scene) -> bevy_app::App {
    let mut placements = vec![
        placement("depot", DEPOT_POS),
        placement("worker", Vec2::new(28.0, 10.0)),
    ];
    let mut expected = vec![(DEPOT, DEPOT_POS), (WORKER, Vec2::new(28.0, 10.0))];
    if let Some(x) = scene.helper_x {
        placements.push(placement("worker", Vec2::new(x, 10.0)));
        expected.push((HELPER, Vec2::new(x, 10.0)));
    }
    if let (Some(at), Some(blocker)) = (scene.blocker, scene.blocker_id()) {
        placements.push(placement("worker", at));
        expected.push((blocker, at));
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
            remaining: scene.node_stock,
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
    for (unit_id, at) in expected {
        assert_eq!(
            position(&app, unit_id),
            at,
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
