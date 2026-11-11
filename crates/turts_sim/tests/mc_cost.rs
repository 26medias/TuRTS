//! Ignored Task22 tick recorder using parsed Muster definitions and isolated legal maps.
//! Timings have no pass threshold and remain untraced until the registered runtime lease.

use std::{collections::BTreeSet, process::Command as ProcessCommand, time::Instant};

use turts_mod_api::{
    ContentId, MapDef, MapSize, OrderDef, PackageId, Placement, SourceDef, WorldPos,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::{
    Command, CommandKind, CommandQueue, CommandResults, OrderCommand, OrderTargetSim, QueueMode,
    SlotId, Vec2, audit_state, headless_app, install_package, stage_package,
};

const GROUP: u32 = 22;
const MAP_SIZE: f32 = 128.0;
const SPREAD_TARGET: Vec2 = Vec2::new(64.0, 64.0);
const MOVE_WARMUP_TICKS: usize = 10;
const MEASURED_TICKS: usize = 20;
const COMBAT_SAMPLES: usize = 20;

#[derive(Clone)]
struct MusterFixture {
    catalog: turts_mod_api::ContentCatalog,
    map_file: String,
    authored_map: MapDef,
    archive_sha256: String,
}

#[derive(Clone, Copy)]
enum Workload {
    Moving,
    Combat,
}

#[test]
#[ignore = "Task22 timing record; run only after a later explicit cache/runtime lease"]
fn record_spread_avoidance_and_combat_ticks_at_one_and_five_thousand_units() {
    let fixture = muster_fixture();
    let machine = machine_record();
    let commit = source_commit();
    let profile = if cfg!(debug_assertions) {
        "debug-assertions"
    } else {
        "optimized-no-debug-assertions"
    };
    eprintln!(
        "MC cost provenance: machine={machine}; logical_parallelism={}; commit={commit}; profile={profile}; tick_hz=20; muster_archive_sha256={}; authored_map={}; controlled_map_size={MAP_SIZE}x{MAP_SIZE}; timing_budget=OPEN",
        std::thread::available_parallelism().map_or(0, |count| count.get()),
        fixture.archive_sha256,
        fixture.authored_map.id,
    );

    for count in [1_000, 5_000] {
        let spread = measure_spread(&fixture, count);
        let avoidance = measure_avoidance(&fixture, count);
        let combat = measure_combat(&fixture, count);
        eprintln!(
            "MC cost workload: units={count}; timed_metric=whole_app_update_tick; included=all_simulation_systems_and_scenario_work; spread_warmup_fixtures=1; spread_samples=1; spread_instrumentation_excluded=true; spread_all_dispatch_ticks_whole_tick_ns={}; actual_move_accepted={}; active_after_spread={}; pending_after_spread={}; spreading_dispatch_ticks={:?}; avoidance_warmup_ticks={MOVE_WARMUP_TICKS}; avoidance_samples={}; avoidance_whole_tick_ns={:?}; avoidance_active_counts={:?}; combat_warmup_fresh_instances=1; combat_samples={}; combat_whole_tick_ns={:?}; combat_live={} armed={} eligible_targets={} fired_events={:?}",
            spread.elapsed_ns,
            spread.accepted,
            spread.active,
            spread.pending,
            spread.dispatch,
            avoidance.raw_ns.len(),
            avoidance.raw_ns,
            avoidance.active,
            combat.raw_ns.len(),
            combat.raw_ns,
            combat.live,
            combat.armed,
            combat.eligible_targets,
            combat.fired,
        );
        assert_eq!(
            spread.accepted, count,
            "all requested movement units are admitted"
        );
        assert_eq!(
            spread.active, count,
            "all admitted movement units are active"
        );
        assert_eq!(spread.pending, 0, "no movement membership remains pending");
        assert!(
            !spread.dispatch.is_empty(),
            "at least one real dispatch tick is recorded"
        );
        assert!(avoidance.active.iter().all(|active| *active > 0));
        assert!(combat.live > 0 && combat.armed > 0 && combat.eligible_targets > 0);
        assert!(combat.fired.iter().all(|events| *events > 0));
    }
}

/// One spreading tick: tick, units spread, active members, pending members, whole-tick ns.
type DispatchRow = (u32, usize, usize, usize, u128);

struct SpreadRecord {
    elapsed_ns: u128,
    accepted: usize,
    active: usize,
    pending: usize,
    dispatch: Vec<DispatchRow>,
}

struct AvoidanceRecord {
    raw_ns: Vec<u128>,
    active: Vec<usize>,
}

struct CombatRecord {
    raw_ns: Vec<u128>,
    fired: Vec<usize>,
    live: usize,
    armed: usize,
    eligible_targets: usize,
}

fn muster_fixture() -> MusterFixture {
    let package_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
    let bytes = pack_dir(&package_dir).expect("pack authored Muster package");
    let archive = PackageArchive::parse(&bytes).expect("parse authored Muster archive");
    let catalog = archive.content().expect("parse authored Muster catalog");
    let (map_file, authored_map) = archive
        .maps()
        .expect("parse authored Muster maps")
        .into_iter()
        .find(|(_, map)| map.id == "field")
        .expect("authored Muster field map exists");
    MusterFixture {
        catalog,
        map_file,
        authored_map,
        archive_sha256: archive.hash,
    }
}

fn app_with_units(fixture: &MusterFixture, count: usize, workload: Workload) -> bevy_app::App {
    assert!(count >= 2 && count.is_multiple_of(2));
    let mut map = fixture.authored_map.clone();
    map.id = format!(
        "mc_cost_{}_{count}",
        match workload {
            Workload::Moving => "movement",
            Workload::Combat => "combat",
        }
    );
    map.size = MapSize {
        x: MAP_SIZE,
        y: MAP_SIZE,
    };
    // The workload does not gather. The authored ore node at (7, 9) would overlap the unit grid,
    // so the isolated cost map holds no node.
    map.nodes.clear();
    map.placements = match workload {
        Workload::Moving => moving_placements(count),
        Workload::Combat => combat_placements(count),
    };

    let mut app = headless_app(20);
    let staged = stage_package(
        app.world(),
        fixture.catalog.clone(),
        SourceDef {
            file: fixture.map_file.clone(),
            value: map,
        },
        20,
    )
    .expect("the isolated large map stages with actual Muster definitions");
    install_package(app.world_mut(), staged);
    assert_eq!(audit_state(app.world_mut()).raw_units.len(), count);
    app
}

fn moving_placements(count: usize) -> Vec<Placement> {
    (0..count)
        .map(|index| {
            let column = index % 100;
            let row = index / 100;
            Placement {
                unit: "muster:trooper".into(),
                owner_slot: 0,
                pos: WorldPos {
                    x: 2.0 + column as f32 * 1.1,
                    y: 2.0 + row as f32 * 1.1,
                },
                contents: Vec::new(),
                facing: None,
            }
        })
        .collect()
}

fn combat_placements(count: usize) -> Vec<Placement> {
    (0..count / 2)
        .flat_map(|index| {
            let column = index % 40;
            let row = index / 40;
            let x = 2.0 + column as f32 * 3.0;
            let y = 2.0 + row as f32 * 1.25;
            [
                Placement {
                    unit: "muster:trooper".into(),
                    owner_slot: 0,
                    pos: WorldPos { x, y },
                    contents: Vec::new(),
                    facing: None,
                },
                Placement {
                    unit: "muster:dummy".into(),
                    owner_slot: 1,
                    pos: WorldPos { x: x + 1.5, y },
                    contents: Vec::new(),
                    facing: None,
                },
            ]
        })
        .collect()
}

fn queue_group_move(app: &mut bevy_app::App, fixture: &MusterFixture, count: usize) {
    let (order_id, definition) = authored_order(&fixture.catalog, "order_move");
    let queue_depth = fixture.catalog.order_queue_depths[&PackageId("muster".into())];
    for (part, ids) in (1..=count as u32)
        .collect::<Vec<_>>()
        .chunks(4096)
        .enumerate()
    {
        let sequence = part as u32 + 1;
        app.world_mut()
            .resource_mut::<CommandQueue>()
            .0
            .push_back(Command {
                slot: SlotId(0),
                seq: sequence,
                kind: CommandKind::Order(Box::new(OrderCommand {
                    order: order_id.clone(),
                    behaviour: definition.behaviour,
                    target_kind: definition.target,
                    policy: None,
                    units: ids.to_vec(),
                    target: OrderTargetSim::Point(SPREAD_TARGET),
                    queue: QueueMode::Replace,
                    group: GROUP,
                    allowed_units: definition.units.clone(),
                    queue_depth,
                })),
            });
    }
}

fn authored_order<'a>(
    catalog: &'a turts_mod_api::ContentCatalog,
    id: &str,
) -> (ContentId, &'a OrderDef) {
    catalog
        .orders
        .iter()
        .find(|(content_id, _)| content_id.package.0 == "muster" && content_id.local == id)
        .map(|(content_id, definition)| (content_id.clone(), definition))
        .expect("authored Muster order exists")
}

fn measure_spread(fixture: &MusterFixture, count: usize) -> SpreadRecord {
    let mut warmup = app_with_units(fixture, count, Workload::Moving);
    queue_group_move(&mut warmup, fixture, count);
    run_admission(&mut warmup, count, false);

    let mut app = app_with_units(fixture, count, Workload::Moving);
    queue_group_move(&mut app, fixture, count);
    let (dispatch, dispatch_ns, accepted_ids) = run_admission(&mut app, count, true);
    let elapsed_ns = dispatch_ns;
    let accepted = accepted_ids.len();
    let state = audit_state(app.world_mut());
    let active = active_group_members(&state);
    let pending = state.pending_movement_orders.pending_member_counts.len();
    SpreadRecord {
        elapsed_ns,
        accepted,
        active,
        pending,
        dispatch,
    }
}

fn measure_avoidance(fixture: &MusterFixture, count: usize) -> AvoidanceRecord {
    let mut app = app_with_units(fixture, count, Workload::Moving);
    queue_group_move(&mut app, fixture, count);
    run_admission(&mut app, count, false);
    for _ in 0..MOVE_WARMUP_TICKS {
        app.update();
    }
    let mut raw_ns = Vec::with_capacity(MEASURED_TICKS);
    let mut active = Vec::with_capacity(MEASURED_TICKS);
    for _ in 0..MEASURED_TICKS {
        let started = Instant::now();
        app.update();
        raw_ns.push(started.elapsed().as_nanos());
        let state = audit_state(app.world_mut());
        let moving = active_group_members(&state);
        assert!(
            moving > 0,
            "avoidance sample tick has live active group work"
        );
        active.push(moving);
    }
    AvoidanceRecord { raw_ns, active }
}

fn measure_combat(fixture: &MusterFixture, count: usize) -> CombatRecord {
    // A separate discarded fixture exercises the same combat phase for warm-up. Each measured
    // sample uses a fresh authored-health target set so every timed tick can fire real weapons.
    let mut warmup = app_with_units(fixture, count, Workload::Combat);
    warmup.update();
    assert!(
        fired_count(&warmup) > 0,
        "combat warm-up fires authored Muster weapons"
    );

    let mut raw_ns = Vec::with_capacity(COMBAT_SAMPLES);
    let mut fired = Vec::with_capacity(COMBAT_SAMPLES);
    let mut live = 0;
    let mut armed = 0;
    let mut eligible_targets = 0;
    for _ in 0..COMBAT_SAMPLES {
        let mut app = app_with_units(fixture, count, Workload::Combat);
        let before = audit_state(app.world_mut());
        live = before.raw_units.len();
        armed = before
            .raw_weapons
            .iter()
            .filter(|weapon| weapon.enabled)
            .count();
        eligible_targets = eligible_target_count(fixture, &before);
        let started = Instant::now();
        app.update();
        raw_ns.push(started.elapsed().as_nanos());
        let shots = fired_count(&app);
        assert!(
            shots > 0,
            "timed combat tick fires at least one real weapon"
        );
        fired.push(shots);
    }
    CombatRecord {
        raw_ns,
        fired,
        live,
        armed,
        eligible_targets,
    }
}

fn run_admission(
    app: &mut bevy_app::App,
    count: usize,
    time_dispatch: bool,
) -> (Vec<DispatchRow>, u128, BTreeSet<u32>) {
    let mut dispatch = Vec::new();
    let mut accepted = BTreeSet::new();
    let mut total_ns = 0;
    for _ in 0..128 {
        let started = time_dispatch.then(Instant::now);
        app.update();
        let tick_ns = started.map_or(0, |started| started.elapsed().as_nanos());
        if !app.world().resource::<CommandResults>().0.is_empty() {
            accepted.extend(accepted_ids(app));
        }
        let state = audit_state(app.world_mut());
        let pending = state.pending_movement_orders.pending_member_counts.len();
        let active = active_group_members(&state);
        let used = state.movement_dispatch.used;
        if used > 0 {
            total_ns += tick_ns;
            dispatch.push((
                state.movement_dispatch.tick.unwrap_or_default(),
                used,
                active,
                pending,
                tick_ns,
            ));
        }
        if pending == 0 && !dispatch.is_empty() {
            break;
        }
    }
    assert!(
        !dispatch.is_empty(),
        "actual spread dispatch ticks are recorded"
    );
    assert_eq!(
        accepted.len(),
        count,
        "command results admit every unit across <=4096-unit parts"
    );
    assert_eq!(accepted, (1..=count as u32).collect());
    let state = audit_state(app.world_mut());
    assert_eq!(
        active_group_members(&state),
        count,
        "all admitted units have the active group order"
    );
    assert_eq!(
        state.pending_movement_orders.pending_member_counts.len(),
        0,
        "the actual spread queue is empty"
    );
    (dispatch, total_ns, accepted)
}

fn accepted_ids(app: &bevy_app::App) -> BTreeSet<u32> {
    let results = app.world().resource::<CommandResults>();
    assert!(
        !results.0.is_empty(),
        "actual command results were captured"
    );
    let mut accepted = BTreeSet::new();
    for result in &results.0 {
        assert!(
            result.outcome.is_ok(),
            "command-level outcome is accepted: {result:?}"
        );
        assert!(
            result.units.refused.is_empty(),
            "no requested unit was refused: {result:?}"
        );
        accepted.extend(result.units.accepted.iter().copied());
    }
    accepted
}

fn active_group_members(state: &turts_sim::AuditState) -> usize {
    state
        .unit_orders
        .iter()
        .filter(|record| {
            record.orders.current.as_ref().is_some_and(|order| {
                order.group == GROUP && order.behaviour == turts_mod_api::Behaviour::Move
            })
        })
        .count()
}

fn fired_count(app: &bevy_app::App) -> usize {
    turts_sim::committed_tick(app.world())
        .events
        .iter()
        .filter(|event| matches!(&event.kind, turts_sim::EconomyEventKind::Fired { .. }))
        .count()
}

fn eligible_target_count(fixture: &MusterFixture, state: &turts_sim::AuditState) -> usize {
    let (_, trooper) = fixture
        .catalog
        .units
        .iter()
        .find(|(id, _)| id.package.0 == "muster" && id.local == "trooper")
        .expect("authored Muster trooper exists");
    let range = trooper
        .weapon
        .as_ref()
        .expect("authored trooper weapon exists")
        .range;
    state
        .raw_units
        .iter()
        .filter(|target| {
            target.owner_slot == 1
                && state.raw_units.iter().any(|attacker| {
                    attacker.owner_slot == 0
                        && Vec2::new(attacker.pos.x, attacker.pos.y)
                            .distance_squared(Vec2::new(target.pos.x, target.pos.y))
                            <= range * range
                })
        })
        .count()
}

fn machine_record() -> String {
    std::fs::read_to_string("/proc/cpuinfo")
        .unwrap_or_default()
        .lines()
        .find_map(|line| line.strip_prefix("model name\t: ").map(str::to_owned))
        .unwrap_or_else(|| format!("{} {}", std::env::consts::OS, std::env::consts::ARCH))
}

fn source_commit() -> String {
    if let Ok(value) = std::env::var("MC_SOURCE_COMMIT") {
        return value;
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = ProcessCommand::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .expect("git resolves the source commit for the eventual measurement record");
    assert!(output.status.success(), "git rev-parse HEAD succeeds");
    String::from_utf8(output.stdout)
        .expect("git commit is UTF-8")
        .trim()
        .to_owned()
}
