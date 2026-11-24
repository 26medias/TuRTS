//! Independent acceptance run with the actual skirmish radii, offsets, speed and recipe.
use turts_mod_api::{
    Behaviour, ContentId, MovementDef, Placement, SourceDef, TargetKind, WorldPos,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::economy::production::queue_production;
use turts_sim::orders::{OrderCommand, OrderTargetSim, QueueMode};
use turts_sim::{
    Command, CommandKind, SlotId, UnitIndex, UnitOrders, Vec2, apply_command, headless_app,
    install_package, snapshot, stage_package,
};

fn app(blockers: bool, mobile_producer: bool) -> bevy_app::App {
    let mut app = headless_app(20);
    let bytes =
        pack_dir(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish"))
            .unwrap();
    let archive = PackageArchive::parse(&bytes).unwrap();
    let mut catalog = archive.content().unwrap();
    let barracks = catalog
        .units
        .get_mut(&ContentId::parse("skirmish:barracks").unwrap())
        .unwrap();
    // Queue length only: submit all thirty jobs once; all physical parameters stay unchanged.
    barracks.producer.as_mut().unwrap().queue_capacity = 64;
    if mobile_producer {
        barracks.movement = Some(MovementDef { speed: 4.0 });
    }
    let (file, mut map) = archive.maps().unwrap().remove(0);
    map.placements.clear();
    map.nodes.clear();
    map.belts.clear();
    let place = |unit: &str, x, y| Placement {
        unit: unit.into(),
        owner_slot: 0,
        pos: WorldPos { x, y },
        contents: vec![],
        facing: None,
        start_order: None,
    };
    map.placements.push(place("skirmish:barracks", 20.0, 20.0));
    if blockers {
        for (x, y) in [(3.5, 0.0), (0.0, 3.5), (0.0, -3.5), (-3.5, 0.0)] {
            map.placements
                .push(place("skirmish:soldier", 20.0 + x, 20.0 + y));
        }
    }
    for slot in &mut map.slot_economy {
        for balance in &mut slot.balances {
            balance.amount = 100_000;
        }
    }
    let staged = stage_package(app.world(), catalog, SourceDef { file, value: map }, 20).unwrap();
    install_package(app.world_mut(), staged);
    app
}
fn queue(app: &mut bevy_app::App) {
    queue_production(
        app.world_mut(),
        SlotId(0),
        1,
        ContentId::parse("skirmish:train_soldier").unwrap(),
    )
    .unwrap();
}
fn run_thirty() -> Vec<Vec<(u32, Vec2)>> {
    let mut app = app(false, false);
    for _ in 0..30 {
        queue(&mut app);
    }
    let centre = Vec2::new(20.0, 20.0);
    let rally = centre + (Vec2::new(32.0, 24.0) - centre).normalize() * 5.5;
    let mut traces = vec![];
    let mut worst = (0.0, 0, 0, 0);
    let mut worst_moving = (0.0, 0, 0, 0);
    let mut first_rest = None;
    let mut first_drift = 0.0_f32;
    for tick in 0..2600 {
        app.update();
        let states = snapshot(app.world_mut());
        let previous: Vec<(u32, Vec2)> = traces.last().cloned().unwrap_or_default();
        let rests = |unit: &turts_sim::UnitState| previous.contains(&(unit.unit_id, unit.pos));
        for (i, a) in states.iter().enumerate() {
            for b in &states[i + 1..] {
                let building = a.unit_id == 1 || b.unit_id == 1;
                let min = if building { 2.05 } else { 1.1 };
                let gap = a.pos.distance(b.pos) - min;
                // Rule 4 (2026-11-23): own units that move may overlap for a short time. A
                // pair is at rest when both units keep their position in the tick.
                let record = if building || (rests(a) && rests(b)) {
                    &mut worst
                } else {
                    &mut worst_moving
                };
                if gap < record.0 {
                    *record = (gap, tick, a.unit_id, b.unit_id);
                }
            }
        }
        if tick == 200 {
            first_rest = Some(states[1].pos);
        }
        if tick > 200 && states.len() > 1 {
            first_drift = first_drift.max(states[1].pos.distance(first_rest.unwrap()));
        }
        traces.push(states.iter().map(|s| (s.unit_id, s.pos)).collect());
    }
    let states = snapshot(app.world_mut());
    assert_eq!(states.len(), 31, "thirty actual soldiers must finish");
    let far = states
        .iter()
        .skip(1)
        .map(|s| s.pos.distance(rally))
        .fold(0.0, f32::max);
    println!(
        "actual skirmish: 30 soldiers, farthest centre from rally {far}, worst overlap at rest {worst:?}, worst overlap in motion {worst_moving:?}, first drift {first_drift}"
    );
    // Rule 2 (2026-11-23): the first unit keeps its place in the crowd, and a settled unit
    // shifts a short distance to make room: at most one unit diameter (1.1).
    assert!(first_drift <= 1.1, "first arrival left its place");
    assert!(far < 7.0, "group must stay near rally");
    assert!(worst.0 >= -0.0001, "EX2: actual skirmish overlap {worst:?}");
    // Rule 4 (2026-11-23): at most 25 % of the smaller radius (0.55) while the units move.
    assert!(
        worst_moving.0 >= -0.25 * 0.55,
        "EX2: actual skirmish overlap in motion {worst_moving:?}"
    );
    traces
}
#[test]
fn actual_skirmish_thirty_no_overlap_first_arrival_keeps_point_deterministic() {
    assert_eq!(run_thirty(), run_thirty());
}
#[test]
fn actual_skirmish_four_idle_exits_fifth_uses_first_ring() {
    let mut app = app(true, false);
    queue(&mut app);
    for _ in 0..82 {
        app.update();
        let states = snapshot(app.world_mut());
        if let Some(last) = states.iter().find(|s| s.unit_id == 6) {
            let direction = (Vec2::new(32.0, 24.0) - Vec2::splat(20.0)).normalize();
            assert!(
                last.pos.distance(Vec2::splat(20.0) + direction * 3.15) < 0.0001,
                "actual EX1 wrong first point: {}",
                last.pos
            );
            return;
        }
    }
    panic!("actual EX1 no fifth soldier");
}
fn order(
    app: &mut bevy_app::App,
    behaviour: Behaviour,
    target: OrderTargetSim,
    target_kind: TargetKind,
) {
    let result = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::Order(Box::new(OrderCommand {
                order: ContentId::parse("skirmish:order_rally").unwrap(),
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
        },
    );
    assert!(result.outcome.is_ok(), "order failed {:?}", result);
}
#[test]
fn producer_keeps_rally_when_its_own_orders_change() {
    let mut app = app(false, true);
    let point = Vec2::new(30.0, 30.0);
    order(
        &mut app,
        Behaviour::SetRally,
        OrderTargetSim::Point(point),
        TargetKind::Point,
    );
    order(
        &mut app,
        Behaviour::Stop,
        OrderTargetSim::None,
        TargetKind::None,
    );
    let entity = app.world().resource::<UnitIndex>().get(1).unwrap();
    assert_eq!(
        app.world().get::<UnitOrders>(entity).unwrap().rally,
        Some(point),
        "Stop must not erase the producer's rally setting"
    );
}
