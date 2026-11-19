//! More fog replay scenes, from the independent review of 2026-11-18 (QA). Live headless captures; no hand-built spread.
use std::path::Path;

use bevy_app::App;
use turts_audit::{EconomyLogRecord, validate_records, validate_records_with_sight};
use turts_mod_api::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::visibility::{SightAudit, SightPolicy};
use turts_sim::{
    AuditState, AuditTrail, Command, CommandKind, CommandQueue, OrderTargetSim, QueueMode, SlotId,
    Vec2,
};

const VISION: f32 = 6.0;
const HIDDEN_POINT: (f32, f32) = (30.0, 30.0);
const SEEN_POINT: (f32, f32) = (53.0, 30.0);
const GROUP_1: [(f32, f32); 3] = [(10.0, 10.0), (10.0, 12.0), (10.0, 14.0)];
const GROUP_2: [(f32, f32); 3] = [(50.0, 50.0), (52.0, 50.0), (54.0, 50.0)];
const OBSERVER: (f32, f32) = (50.0, 30.0);

fn cid(local: &str) -> ContentId {
    ContentId::parse(&format!("cedar:{local}")).unwrap()
}
fn placement(slot: u8, (x, y): (f32, f32)) -> Placement {
    Placement {
        unit: cid("worker").to_string(),
        owner_slot: slot,
        pos: WorldPos { x, y },
        contents: vec![],
        facing: None,
        start_order: None,
    }
}
fn policy(sharing: VisionSharing, teams: &[u8]) -> SightPolicy {
    SightPolicy {
        grid: VisibilityGrid {
            size: WorldPos { x: 64.0, y: 64.0 },
            cell_size: 2.0,
            columns: 32,
            rows: 32,
        },
        sharing,
        teams: teams.to_vec(),
    }
}

#[derive(Clone)]
struct Scene {
    fog: Option<VisionSharing>,
    /// None: do not call `initialize_context` (the fail-closed placeholder).
    teams: Option<Vec<u8>>,
    warm: bool,
    health: bool,
    removal: bool,
    extra: Vec<(u8, (f32, f32))>,
    depot: Option<(f32, f32)>,
}
impl Scene {
    fn base(fog: bool) -> Self {
        Self {
            fog: fog.then_some(VisionSharing::OwnSlot),
            teams: Some(vec![0, 1]),
            warm: true,
            health: false,
            removal: false,
            extra: Vec::new(),
            depot: None,
        }
    }
}

struct Run {
    app: App,
    seq: [u32; 2],
    records: Vec<EconomyLogRecord>,
}
impl Run {
    fn new(scene: &Scene) -> Self {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar");
        let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
        let mut catalog = archive.content().unwrap();
        let mut map = archive.maps().unwrap().remove(0).1;
        let worker = catalog.units.get_mut(&cid("worker")).unwrap();
        worker.vision = Some(VisionDef { radius: VISION });
        if scene.health {
            worker.health = Some(HealthDef { max: 100 });
        }
        if scene.removal {
            worker.removal = Some(RemovalDef {
                by_command: true,
                contents: ContentsPolicy::Destroy,
            });
        }
        map.nodes.clear();
        map.placements = GROUP_1
            .iter()
            .chain(&GROUP_2)
            .chain([&OBSERVER])
            .map(|at| placement(0, *at))
            .chain([placement(1, HIDDEN_POINT), placement(1, SEEN_POINT)])
            .chain(scene.extra.iter().map(|(slot, at)| placement(*slot, *at)))
            .chain(scene.depot.iter().map(|at| Placement {
                unit: cid("depot").to_string(),
                owner_slot: 1,
                pos: WorldPos { x: at.0, y: at.1 },
                contents: vec![ItemStack {
                    item: cid("wood").to_string(),
                    amount: 100,
                }],
                facing: None,
                start_order: None,
            }))
            .collect();
        map.visibility = scene.fog.map(|sharing| VisibilityDef {
            cell_size: 2.0,
            sharing,
            memory_limit: None,
        });
        let mut app = turts_sim::headless_app(20);
        let staged = turts_sim::stage_package(
            app.world(),
            catalog,
            SourceDef {
                file: "fog-probe/field.ron".into(),
                value: map,
            },
            20,
        )
        .unwrap();
        turts_sim::install_package(app.world_mut(), staged);
        if let Some(teams) = &scene.teams {
            turts_sim::visibility::initialize_context(app.world_mut(), 7, teams).unwrap();
        }
        turts_sim::initialize_completed_world(app.world_mut());
        if scene.warm {
            app.update();
        }
        app.world_mut().resource_mut::<AuditTrail>().enable();
        let records = vec![EconomyLogRecord::Initial(turts_sim::audit_state(
            app.world_mut(),
        ))];
        Self {
            app,
            seq: [0; 2],
            records,
        }
    }
    fn state(&mut self) -> AuditState {
        turts_sim::audit_state(self.app.world_mut())
    }
    fn unit_at(&mut self, (x, y): (f32, f32)) -> u32 {
        self.state()
            .raw_units
            .iter()
            .find(|unit| unit.pos == WorldPos { x, y })
            .unwrap()
            .unit_id
    }
    fn push(&mut self, slot: u8, kind: CommandKind) {
        self.seq[usize::from(slot)] += 1;
        let command = Command {
            slot: SlotId(slot),
            seq: self.seq[usize::from(slot)],
            kind,
        };
        self.app
            .world_mut()
            .resource_mut::<CommandQueue>()
            .0
            .push_back(command);
    }
    fn order(
        &mut self,
        slot: u8,
        group: u32,
        units: Vec<u32>,
        (x, y): (f32, f32),
        queue: QueueMode,
    ) {
        let state = self.state();
        let policy = state
            .catalog
            .as_ref()
            .unwrap()
            .order_definitions
            .iter()
            .find(|row| row.id == cid("order_move"))
            .unwrap();
        self.push(
            slot,
            CommandKind::Order(Box::new(turts_sim::OrderCommand {
                order: policy.id.clone(),
                behaviour: policy.behaviour,
                target_kind: policy.target,
                policy: Some(turts_sim::OrderPolicyAudit {
                    id: policy.id.clone(),
                    behaviour: policy.behaviour,
                    target: policy.target,
                    fire: policy.fire,
                    engage: policy.engage,
                    units: policy.units.clone(),
                    queue_depth: policy.queue_depth,
                    unit_capabilities: policy.unit_capabilities.clone(),
                }),
                units,
                target: OrderTargetSim::Point(Vec2::new(x, y)),
                queue,
                group,
                allowed_units: policy.units.clone(),
                queue_depth: policy.queue_depth,
            })),
        );
    }
    fn ticks(&mut self, count: u32) {
        for _ in 0..count {
            self.app.update();
            let mut trail = self.app.world_mut().resource_mut::<AuditTrail>();
            self.records.extend(
                trail
                    .drain_commands()
                    .into_iter()
                    .map(EconomyLogRecord::CommandBoundary),
            );
            self.records.extend(
                trail
                    .drain_operations()
                    .into_iter()
                    .map(EconomyLogRecord::Operation),
            );
            self.records.extend(
                trail
                    .drain_ticks()
                    .into_iter()
                    .map(EconomyLogRecord::from_tick),
            );
        }
    }
    fn log(&self) -> Vec<EconomyLogRecord> {
        let lines: Vec<_> = self
            .records
            .iter()
            .map(|record| serde_json::to_string(record).unwrap())
            .collect();
        turts_audit::read_jsonl(std::io::Cursor::new(lines.join("\n"))).unwrap()
    }
    fn pools(&self) -> usize {
        self.records
            .iter()
            .filter_map(|record| match record {
                EconomyLogRecord::Tick { state, .. } => Some(state.movement_pools.len()),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }
}

fn each_state(records: &mut [EconomyLogRecord], mut change: impl FnMut(&mut AuditState)) {
    for record in records {
        match record {
            EconomyLogRecord::Initial(state) => change(state),
            EconomyLogRecord::CommandBoundary(command) => {
                change(&mut command.before);
                change(&mut command.after);
            }
            EconomyLogRecord::Operation(operation) => {
                change(&mut operation.before);
                change(&mut operation.after);
            }
            EconomyLogRecord::Tick { before, state, .. } => {
                change(before);
                change(state);
            }
            EconomyLogRecord::Admission { before, after, .. } => {
                change(before);
                change(after);
            }
            _ => {}
        }
    }
}
fn forged(
    records: &[EconomyLogRecord],
    change: impl Fn(&mut Option<SightAudit>),
) -> Vec<EconomyLogRecord> {
    let mut forged = records.to_vec();
    each_state(&mut forged, |state| change(&mut state.raw_sight));
    forged
}
/// Result of the correct log, and of the same log with the sight inputs removed. The second
/// shows whether the fog rule changed a spread in the scene (a scene where it passes is weak).
fn report(name: &str, records: &[EconomyLogRecord]) -> Result<(), String> {
    let result = validate_records(records);
    let stripped = validate_records(&forged(records, |sight| *sight = None));
    println!("PROBE {name}: correct={result:?} stripped={stripped:?}");
    result
}

fn base_capture(scene: &Scene) -> Run {
    let mut run = Run::new(scene);
    let g1 = GROUP_1.iter().map(|at| run.unit_at(*at)).collect();
    let g2 = GROUP_2.iter().map(|at| run.unit_at(*at)).collect();
    run.order(0, 1, g1, HIDDEN_POINT, QueueMode::Replace);
    run.ticks(1);
    run.order(0, 2, g2, SEEN_POINT, QueueMode::Replace);
    run.ticks(8);
    run
}

// ---------- Completeness: live scenes, correct logs ----------

#[test]
fn c1_team_sharing_live() {
    let mut scene = Scene::base(true);
    scene.fog = Some(VisionSharing::Team);
    scene.teams = Some(vec![0, 0]);
    let run = base_capture(&scene);
    assert_eq!(run.pools(), 2);
    let log = run.log();
    report("c1_team", &log).unwrap();
    validate_records_with_sight(&log, Some(&policy(VisionSharing::Team, &[0, 0]))).unwrap();
}

#[test]
fn c2_order_on_first_tick_no_warm_up() {
    let mut scene = Scene::base(true);
    scene.warm = false;
    let run = base_capture(&scene);
    assert_eq!(run.pools(), 2);
    report("c2_first_tick", &run.log()).unwrap();
}

#[test]
fn c3_placeholder_not_finalized() {
    let mut scene = Scene::base(true);
    scene.teams = None;
    let run = base_capture(&scene);
    println!("PROBE c3 pools={}", run.pools());
    let log = run.log();
    let finalized = match &log[0] {
        EconomyLogRecord::Initial(state) => state.raw_sight.as_ref().map(|sight| sight.finalized),
        _ => None,
    };
    println!("PROBE c3 finalized={finalized:?}");
    report("c3_placeholder", &log).unwrap();
}

#[test]
fn c4_units_with_health() {
    let mut scene = Scene::base(true);
    scene.health = true;
    let mut run = base_capture(&scene);
    let state = run.state();
    println!(
        "PROBE c4 healths={} has_health={:?}",
        state.raw_healths.len(),
        state.raw_sight.as_ref().map(|sight| sight.types.clone())
    );
    assert_eq!(run.pools(), 2);
    report("c4_health", &run.log()).unwrap();
}

/// The observer is removed by a command in the tick of the dispatch. The Start basis of
/// that tick still holds its sight.
#[test]
fn c5_source_removed_in_dispatch_tick() {
    let mut scene = Scene::base(true);
    scene.removal = true;
    let mut run = Run::new(&scene);
    let observer = run.unit_at(OBSERVER);
    let g2: Vec<u32> = GROUP_2.iter().map(|at| run.unit_at(*at)).collect();
    run.push(0, CommandKind::Remove { unit_id: observer });
    run.order(0, 2, g2, SEEN_POINT, QueueMode::Replace);
    run.ticks(6);
    let gone = !run
        .state()
        .raw_units
        .iter()
        .any(|unit| unit.unit_id == observer);
    println!("PROBE c5 observer_gone={gone} pools={}", run.pools());
    assert!(gone, "the scene needs the removal to apply");
    report("c5_remove_source", &run.log()).unwrap();
}

/// The seen unit of slot 1 is removed in the tick of the dispatch, and the order then the
/// removal in the other sequence.
#[test]
fn c6_body_removed_in_dispatch_tick() {
    let mut scene = Scene::base(true);
    scene.removal = true;
    let mut run = Run::new(&scene);
    let seen = run.unit_at(SEEN_POINT);
    let g2: Vec<u32> = GROUP_2.iter().map(|at| run.unit_at(*at)).collect();
    run.order(0, 2, g2, SEEN_POINT, QueueMode::Replace);
    run.push(1, CommandKind::Remove { unit_id: seen });
    run.ticks(6);
    report("c6_remove_body", &run.log()).unwrap();
}

/// Many ticks. The hidden unit walks into sight, the observer walks away, and slot 0 gives a
/// new group order each tick. Sight at each dispatch differs from sight at the prior one.
fn long_run(fog: bool, ticks: u32) -> Run {
    let mut run = Run::new(&Scene::base(fog));
    let hidden = run.unit_at(HIDDEN_POINT);
    let seen = run.unit_at(SEEN_POINT);
    let observer = run.unit_at(OBSERVER);
    let g1: Vec<u32> = GROUP_1.iter().map(|at| run.unit_at(*at)).collect();
    let g2: Vec<u32> = GROUP_2.iter().map(|at| run.unit_at(*at)).collect();
    run.order(1, 100, vec![hidden], (50.0, 34.0), QueueMode::Replace);
    run.order(1, 101, vec![seen], (30.0, 34.0), QueueMode::Replace);
    run.order(0, 102, vec![observer], (40.0, 30.0), QueueMode::Replace);
    run.ticks(1);
    for tick in 0..ticks {
        // Order points on the paths of the two walkers of slot 1.
        let at = match tick % 4 {
            0 => (44.0, 32.0),
            1 => (38.0, 32.0),
            2 => (48.0, 33.0),
            _ => (34.0, 33.0),
        };
        if tick % 2 == 0 {
            run.order(0, 200 + tick, g2.clone(), at, QueueMode::Replace);
        } else {
            run.order(0, 200 + tick, g1.clone(), at, QueueMode::Replace);
        }
        run.ticks(1);
    }
    run.ticks(10);
    run
}
#[test]
fn c7_long_run_sight_changes_between_orders() {
    let clear = long_run(false, 120);
    let clear_result = validate_records(&clear.log());
    println!(
        "PROBE c7 all_visible={clear_result:?} pools={}",
        clear.pools()
    );
    let run = long_run(true, 120);
    println!("PROBE c7 fog pools={}", run.pools());
    let result = report("c7_long_run", &run.log());
    if clear_result.is_ok() {
        result.unwrap();
    } else {
        println!("PROBE c7 fog result={result:?} (AllVisible is also refused: not this change)");
    }
}

/// A queued second leg: its dispatch comes many ticks after the command.
#[test]
fn c8_appended_leg_dispatches_later() {
    for fog in [false, true] {
        let mut run = Run::new(&Scene::base(fog));
        let g2: Vec<u32> = GROUP_2.iter().map(|at| run.unit_at(*at)).collect();
        run.order(0, 1, g2.clone(), (44.0, 30.0), QueueMode::Replace);
        run.order(0, 2, g2.clone(), SEEN_POINT, QueueMode::Append);
        run.order(0, 3, g2, HIDDEN_POINT, QueueMode::Append);
        run.ticks(400);
        let name = if fog {
            "c8_append_fog"
        } else {
            "c8_append_clear"
        };
        let result = report(name, &run.log());
        println!("PROBE {name} pools={} result={result:?}", run.pools());
        if fog {
            result.unwrap();
        }
    }
}

// ---------- Soundness: forgeries on the live fog capture ----------

fn outcome(name: &str, records: &[EconomyLogRecord]) -> Result<(), String> {
    let result = validate_records(records);
    let with = validate_records_with_sight(records, Some(&policy(VisionSharing::OwnSlot, &[0, 1])));
    println!("FORGE {name}: validate_records={result:?} with_sight={with:?}");
    result
}

#[test]
fn s1_forgeries() {
    let fog = base_capture(&Scene::base(true)).log();
    validate_records(&fog).unwrap();

    // No type row: no unit is in the auditor basis.
    let none = forged(&fog, |sight| sight.as_mut().unwrap().types.clear());
    assert!(outcome("types_empty", &none).is_err());
    // A type with no vision.
    let blind = forged(&fog, |sight| {
        for kind in &mut sight.as_mut().unwrap().types {
            kind.radius = None;
        }
    });
    assert!(outcome("radius_none", &blind).is_err());
    // A health claim with no health row: each source is dead.
    let dead = forged(&fog, |sight| {
        for kind in &mut sight.as_mut().unwrap().types {
            kind.has_health = true;
        }
    });
    assert!(outcome("has_health_forged", &dead).is_err());
    // Extreme and bad radii must give a refusal, not a panic or a long run.
    for radius in [f32::MAX, -6.0, 0.0, f32::NAN, f32::INFINITY] {
        let bad = forged(&fog, |sight| {
            for kind in &mut sight.as_mut().unwrap().types {
                kind.radius = Some(radius);
            }
        });
        assert!(outcome(&format!("radius_{radius}"), &bad).is_err());
    }
    // The mode is absent only in Initial, or only in one later snapshot.
    let mut first = fog.clone();
    if let EconomyLogRecord::Initial(state) = &mut first[0] {
        state.raw_sight = None;
    }
    assert!(outcome("none_in_initial_only", &first).is_err());
    let mut one = fog.clone();
    let mut done = false;
    for record in &mut one {
        if let EconomyLogRecord::Operation(operation) = record
            && !done
            && operation
                .before
                .pending_movement_orders
                .queues
                .values()
                .any(|q| !q.is_empty())
        {
            operation.before.raw_sight = None;
            done = true;
        }
    }
    assert!(done);
    assert!(outcome("none_in_one_dispatch_before", &one).is_err());
    // Duplicate type rows (the later row would win in a map).
    let dup = forged(&fog, |sight| {
        let sight = sight.as_mut().unwrap();
        let mut extra = sight.types.clone();
        for kind in &mut extra {
            kind.radius = Some(100.0);
        }
        sight.types.extend(extra);
    });
    assert!(outcome("duplicate_types", &dup).is_err());
    // Teams of another length under OwnSlot, and Team with a short list.
    let short = forged(&fog, |sight| {
        let sight = sight.as_mut().unwrap();
        sight.policy.sharing = VisionSharing::Team;
        sight.policy.teams = vec![0];
    });
    println!(
        "FORGE team_short_list -> {:?}",
        outcome("team_short_list", &short)
    );
    // A different grid that agrees with its own geometry.
    for cell in [1.0_f32, 4.0, 8.0, 64.0] {
        let grid = forged(&fog, |sight| {
            let grid = &mut sight.as_mut().unwrap().policy.grid;
            grid.cell_size = cell;
            grid.columns = (64.0 / cell) as u32;
            grid.rows = (64.0 / cell) as u32;
        });
        let result = outcome(&format!("grid_cell_{cell}"), &grid);
        assert!(
            validate_records_with_sight(&grid, Some(&policy(VisionSharing::OwnSlot, &[0, 1])))
                .is_err()
        );
        println!("FORGE grid_cell_{cell} validate_records -> {result:?}");
    }
    // Radii that differ from the package but keep the hidden set.
    for radius in [3.5_f32, 5.0, 7.0, 15.0, 20.0, 23.0] {
        let near = forged(&fog, |sight| {
            for kind in &mut sight.as_mut().unwrap().types {
                kind.radius = Some(radius);
            }
        });
        println!(
            "FORGE radius_{radius} (trusted policy equal) -> {:?}",
            outcome(&format!("radius_{radius}"), &near)
        );
    }
}

// ---------- Compatibility ----------

#[test]
fn k1_old_log_without_key_parses_and_null_key_is_all_visible() {
    let clear = base_capture(&Scene::base(false));
    let lines: Vec<_> = clear
        .records
        .iter()
        .map(|record| serde_json::to_string(record).unwrap())
        .collect();
    assert!(lines.iter().all(|line| !line.contains("raw_sight")));
    assert!(lines.iter().all(|line| !line.contains("sight")));
    let parsed = turts_audit::read_jsonl(std::io::Cursor::new(lines.join("\n"))).unwrap();
    validate_records(&parsed).unwrap();
    validate_records_with_sight(&parsed, None).unwrap();
    println!(
        "PROBE k1 initial head: {}",
        &lines[0][..120.min(lines[0].len())]
    );
    // A fog log whose key is removed as text parses, as a log from before the field.
    let fog = base_capture(&Scene::base(true));
    let mut value: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&fog.records[0]).unwrap()).unwrap();
    fn strip(value: &mut serde_json::Value) -> usize {
        match value {
            serde_json::Value::Object(map) => {
                let hit = usize::from(map.remove("raw_sight").is_some());
                hit + map.values_mut().map(strip).sum::<usize>()
            }
            serde_json::Value::Array(rows) => rows.iter_mut().map(strip).sum(),
            _ => 0,
        }
    }
    assert_eq!(strip(&mut value), 1);
    let parsed =
        turts_audit::read_jsonl(std::io::Cursor::new(serde_json::to_string(&value).unwrap()))
            .unwrap();
    validate_records(&parsed).unwrap();
}

/// A unit of slot 1 is born from a hidden depot, in sight of slot 0, while slot 0 gives a
/// group order to the spawn point each tick. A second group goes onto the hidden depot.
#[test]
fn c9_unit_born_and_hidden_static_body() {
    let depot = (53.0, 22.0);
    let mut scene = Scene::base(true);
    scene.depot = Some(depot);
    let mut run = Run::new(&scene);
    let before = run.state().raw_units.len();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar");
    let _ = path;
    let g1: Vec<u32> = GROUP_1.iter().map(|at| run.unit_at(*at)).collect();
    let g2: Vec<u32> = GROUP_2.iter().map(|at| run.unit_at(*at)).collect();
    let producer = run
        .state()
        .raw_units
        .iter()
        .find(|unit| {
            unit.pos
                == WorldPos {
                    x: depot.0,
                    y: depot.1,
                }
        })
        .map(|unit| unit.unit_id);
    let Some(producer) = producer else {
        panic!("the scene needs the depot");
    };
    run.push(
        1,
        CommandKind::Produce {
            producer_id: producer,
            recipe_id: cid("hire"),
        },
    );
    run.order(0, 1, g1, depot, QueueMode::Replace);
    run.ticks(100);
    for tick in 0..40 {
        run.order(0, 300 + tick, g2.clone(), (53.0, 26.0), QueueMode::Replace);
        run.ticks(1);
    }
    run.ticks(5);
    let after = run.state().raw_units.len();
    println!(
        "PROBE c9 units before={before} after={after} pools={}",
        run.pools()
    );
    assert_eq!(after, before + 1, "the scene needs the birth");
    report("c9_birth", &run.log()).unwrap();
}

/// What `validate_records_with_sight` binds: the policy only. A placeholder capture
/// (`finalized: false`) with the trusted policy passes it.
#[test]
fn s2_with_sight_refuses_a_placeholder_capture() {
    let mut scene = Scene::base(true);
    scene.teams = None;
    let log = base_capture(&scene).log();
    let claimed = forged(&log, |sight| {
        sight.as_mut().unwrap().policy = policy(VisionSharing::OwnSlot, &[0, 1]);
    });
    let result =
        validate_records_with_sight(&claimed, Some(&policy(VisionSharing::OwnSlot, &[0, 1])));
    assert_eq!(
        result.unwrap_err(),
        "economy log sight context is not finalized"
    );
}

/// The lobby start of the server: `Initial` is written first
/// (`crates/turts_server/src/main.rs:160`), and the sight context is finalized at the match
/// start (`crates/turts_server/src/lobby.rs:254`).
#[test]
fn c10_lobby_start_finalizes_sight_after_initial() {
    for fog in [false, true] {
        let mut scene = Scene::base(fog);
        scene.teams = None;
        scene.warm = false;
        let mut run = Run::new(&scene);
        turts_sim::visibility::initialize_context(run.app.world_mut(), 7, &[0, 1]).unwrap();
        turts_sim::initialize_completed_world(run.app.world_mut());
        let g2: Vec<u32> = GROUP_2.iter().map(|at| run.unit_at(*at)).collect();
        run.order(0, 2, g2, SEEN_POINT, QueueMode::Replace);
        run.ticks(8);
        let lines: Vec<_> = run
            .records
            .iter()
            .map(|record| serde_json::to_string(record).unwrap())
            .collect();
        let result = turts_audit::read_jsonl(std::io::Cursor::new(lines.join("\n")))
            .and_then(|records| validate_records(&records));
        println!(
            "PROBE c10 lobby fog={fog} pools={} result={result:?}",
            run.pools()
        );
        if !fog {
            result.unwrap();
        } else {
            result.expect("a correct fog log of a lobby start");
        }
    }
}

// ---------- The permitted start change (lobby start) ----------

fn placeholder(sight: &mut Option<SightAudit>) {
    let sight = sight.as_mut().unwrap();
    sight.finalized = false;
    sight.policy.teams.clear();
}
fn show(name: &str, records: &[EconomyLogRecord]) -> (bool, bool) {
    let plain = validate_records(records);
    let with = validate_records_with_sight(records, Some(&policy(VisionSharing::OwnSlot, &[0, 1])));
    println!("Z {name}: validate_records={plain:?} | with_sight={with:?}");
    (plain.is_ok(), with.is_ok())
}
fn kinds(records: &[EconomyLogRecord]) -> String {
    records
        .iter()
        .take(12)
        .map(|record| match record {
            EconomyLogRecord::Initial(_) => "I",
            EconomyLogRecord::Operation(_) => "O",
            EconomyLogRecord::Tick { .. } => "T",
            EconomyLogRecord::Admission { .. } => "A",
            EconomyLogRecord::CommandBoundary(_) => "C",
            EconomyLogRecord::Actor(_) => "a",
            EconomyLogRecord::Claim { .. } => "c",
        })
        .collect()
}

/// A live lobby start with both order points (one hidden body, one seen body).
fn lobby(fog: bool) -> Run {
    let mut scene = Scene::base(fog);
    scene.teams = None;
    scene.warm = false;
    let mut run = Run::new(&scene);
    turts_sim::visibility::initialize_context(run.app.world_mut(), 7, &[0, 1]).unwrap();
    turts_sim::initialize_completed_world(run.app.world_mut());
    let g1 = GROUP_1.iter().map(|at| run.unit_at(*at)).collect();
    let g2 = GROUP_2.iter().map(|at| run.unit_at(*at)).collect();
    run.order(0, 1, g1, HIDDEN_POINT, QueueMode::Replace);
    run.ticks(1);
    run.order(0, 2, g2, SEEN_POINT, QueueMode::Replace);
    run.ticks(8);
    run
}

#[test]
fn z1_live_lobby_start_with_hidden_and_seen_bodies() {
    let run = lobby(true);
    assert_eq!(run.pools(), 2);
    let log = run.log();
    println!("Z z1 record kinds: {}", kinds(&log));
    assert_eq!(show("z1_lobby_live", &log), (true, true));
    // The scene is not weak: the AllVisible rule gives another spread.
    assert!(validate_records(&forged(&log, |sight| *sight = None)).is_err());
    // A wrong trusted policy is refused.
    assert!(
        validate_records_with_sight(&log, Some(&policy(VisionSharing::OwnSlot, &[0, 0]))).is_err()
    );
    assert!(validate_records_with_sight(&log, None).is_err());
}

#[test]
fn z2_forgeries_through_the_start_change() {
    let live = lobby(true).log();
    let fog = base_capture(&Scene::base(true)).log();
    println!("Z z2 record kinds fog: {}", kinds(&fog));
    let first = |records: &[EconomyLogRecord], want: &str| {
        records
            .iter()
            .position(|record| match record {
                EconomyLogRecord::Operation(_) => want == "O",
                EconomyLogRecord::Tick { .. } => want == "T",
                EconomyLogRecord::CommandBoundary(_) => want == "C",
                _ => false,
            })
            .unwrap()
    };
    let op = first(&fog, "O");
    let tick = first(&fog, "T");
    let boundary = first(&fog, "C");
    println!("Z z2 first C={boundary} O={op} T={tick}");

    // a. The first operations (the dispatch) read the placeholder; the change comes after.
    let mut a = fog.clone();
    each_state(&mut a[..=op], |state| placeholder(&mut state.raw_sight));
    assert_eq!(show("a_dispatch_reads_placeholder", &a), (false, false));
    // a2. Only the `before` of the first operation is the placeholder.
    let mut a2 = fog.clone();
    each_state(&mut a2[..op], |state| placeholder(&mut state.raw_sight));
    if let EconomyLogRecord::Operation(operation) = &mut a2[op] {
        placeholder(&mut operation.before.raw_sight);
    }
    assert_eq!(show("a2_operation_before_placeholder", &a2), (false, false));

    // b. The change is inside a CommandBoundary before the first tick (two sight values).
    assert!(
        boundary < op.min(tick),
        "the scene needs a CommandBoundary before the first operation"
    );
    let mut b = fog.clone();
    each_state(&mut b[..boundary], |state| {
        placeholder(&mut state.raw_sight)
    });
    if let EconomyLogRecord::CommandBoundary(command) = &mut b[boundary] {
        placeholder(&mut command.before.raw_sight);
    }
    assert_eq!(show("b_change_inside_command_boundary", &b), (false, false));
    // b2. The whole CommandBoundary keeps the placeholder; the change is at the operation.
    // No dispatch reads the placeholder, so the log replays with the final rule.
    let mut b2 = fog.clone();
    each_state(&mut b2[..=boundary], |state| {
        placeholder(&mut state.raw_sight)
    });
    assert_eq!(show("b2_boundary_under_placeholder", &b2), (true, true));

    // c. The first Tick record has the placeholder in `before` and the final value in `state`.
    let mut c = fog.clone();
    each_state(&mut c[..tick], |state| placeholder(&mut state.raw_sight));
    if let EconomyLogRecord::Tick { before, .. } = &mut c[tick] {
        placeholder(&mut before.raw_sight);
    }
    assert_eq!(show("c_change_inside_first_tick", &c), (false, false));

    // d. Teams forged at the change. The trusted policy must refuse; a spread that the
    // forged teams change must refuse in validate_records too.
    let mut team_scene = Scene::base(true);
    team_scene.fog = Some(VisionSharing::Team);
    team_scene.teams = Some(vec![0, 0]);
    let team = base_capture(&team_scene).log();
    let mut d = forged(&team, |sight| {
        sight.as_mut().unwrap().policy.teams = vec![0, 1]
    });
    each_state(&mut d[..1], |state| placeholder(&mut state.raw_sight));
    assert!(
        validate_records(&d).is_err(),
        "forged teams change the spread"
    );
    assert!(validate_records_with_sight(&d, Some(&policy(VisionSharing::Team, &[0, 0]))).is_err());
    // d2. Teams have no effect under OwnSlot: only the trusted policy refuses.
    let mut d2 = live.clone();
    each_state(&mut d2[1..], |state| {
        state.raw_sight.as_mut().unwrap().policy.teams = vec![1, 0];
    });
    assert_eq!(show("d2_teams_forged_own_slot", &d2), (true, false));

    // e. The placeholder returns after the change.
    let mut e = live.clone();
    let last = e.len() - 1;
    each_state(&mut e[last..], |state| placeholder(&mut state.raw_sight));
    assert_eq!(show("e_placeholder_returns", &e), (false, false));
    // e2. A second change of teams after the first one.
    let mut e2 = live.clone();
    each_state(&mut e2[last..], |state| {
        state.raw_sight.as_mut().unwrap().policy.teams = vec![0, 0];
    });
    assert_eq!(show("e2_second_change", &e2), (false, false));

    // f. Placeholder in Initial, then every later snapshot claims AllVisible.
    let mut f = live.clone();
    each_state(&mut f[1..], |state| state.raw_sight = None);
    assert_eq!(show("f_placeholder_then_all_visible", &f), (false, false));

    // g. An AllVisible live capture that claims a lobby fog start.
    let clear = lobby(false).log();
    let final_sight = live[1..]
        .iter()
        .find_map(|record| match record {
            EconomyLogRecord::Tick { state, .. } => state.raw_sight.clone(),
            _ => None,
        })
        .expect("the final sight inputs");
    let mut g = forged(&clear, |sight| *sight = Some(final_sight.clone()));
    each_state(&mut g[..1], |state| placeholder(&mut state.raw_sight));
    assert_eq!(show("g_all_visible_claims_lobby_fog", &g), (false, false));

    // h. The change touches the grid, the sharing rule, `has_health`, or the type list.
    for case in ["grid", "sharing", "health", "extra_type"] {
        let mut h = live.clone();
        each_state(&mut h[..1], |state| {
            let sight = state.raw_sight.as_mut().unwrap();
            match case {
                "grid" => {
                    sight.policy.grid.cell_size = 4.0;
                    sight.policy.grid.columns = 16;
                    sight.policy.grid.rows = 16;
                }
                "sharing" => sight.policy.sharing = VisionSharing::Team,
                "health" => sight.types[0].has_health = !sight.types[0].has_health,
                _ => {
                    sight.types.pop();
                }
            }
        });
        assert_eq!(show(&format!("h_{case}"), &h), (false, false));
    }
}
