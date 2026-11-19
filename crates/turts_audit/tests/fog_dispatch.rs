//! Replay of real group movement streams under restricted sight (fog).
//! Each log comes from a live headless simulation. No test builds a spread result by hand.
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
/// Slot 0 orders group 1 to this point. A unit of slot 1 stands on it, far from all sight.
const HIDDEN_POINT: (f32, f32) = (30.0, 30.0);
/// Slot 0 orders group 2 to this point. A unit of slot 1 stands on it, in sight of slot 0.
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
/// The policy from the trusted inputs of the match (map rule, map size, room teams).
/// It is not read from a log.
fn trusted_policy() -> SightPolicy {
    SightPolicy {
        grid: VisibilityGrid {
            size: WorldPos { x: 64.0, y: 64.0 },
            cell_size: 2.0,
            columns: 32,
            rows: 32,
        },
        sharing: VisionSharing::OwnSlot,
        teams: vec![0, 1],
    }
}

struct Run {
    app: App,
    seq: u32,
    records: Vec<EconomyLogRecord>,
}
impl Run {
    fn new(fog: bool) -> Self {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar");
        let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
        let mut catalog = archive.content().unwrap();
        let mut map = archive.maps().unwrap().remove(0).1;
        catalog.units.get_mut(&cid("worker")).unwrap().vision = Some(VisionDef { radius: VISION });
        map.nodes.clear();
        map.placements = GROUP_1
            .iter()
            .chain(&GROUP_2)
            .chain([&OBSERVER])
            .map(|at| placement(0, *at))
            .chain([placement(1, HIDDEN_POINT), placement(1, SEEN_POINT)])
            .collect();
        map.visibility = fog.then_some(VisibilityDef {
            cell_size: 2.0,
            sharing: VisionSharing::OwnSlot,
            memory_limit: None,
        });
        let mut app = turts_sim::headless_app(20);
        let staged = turts_sim::stage_package(
            app.world(),
            catalog,
            SourceDef {
                file: "fog-dispatch/field.ron".into(),
                value: map,
            },
            20,
        )
        .unwrap();
        turts_sim::install_package(app.world_mut(), staged);
        turts_sim::visibility::initialize_context(app.world_mut(), 7, &[0, 1]).unwrap();
        turts_sim::initialize_completed_world(app.world_mut());
        app.update();
        app.world_mut().resource_mut::<AuditTrail>().enable();
        let records = vec![EconomyLogRecord::Initial(turts_sim::audit_state(
            app.world_mut(),
        ))];
        Self {
            app,
            seq: 0,
            records,
        }
    }
    fn unit_at(&mut self, (x, y): (f32, f32)) -> u32 {
        turts_sim::audit_state(self.app.world_mut())
            .raw_units
            .iter()
            .find(|unit| unit.pos == WorldPos { x, y })
            .unwrap()
            .unit_id
    }
    fn order(&mut self, group: u32, from: &[(f32, f32)], (x, y): (f32, f32)) {
        self.order_target(group, from, OrderTargetSim::Point(Vec2::new(x, y)));
    }
    fn order_target(&mut self, group: u32, from: &[(f32, f32)], target: OrderTargetSim) {
        let units = from.iter().map(|at| self.unit_at(*at)).collect();
        let state = turts_sim::audit_state(self.app.world_mut());
        let policy = state
            .catalog
            .as_ref()
            .unwrap()
            .order_definitions
            .iter()
            .find(|row| row.id == cid("order_move"))
            .unwrap();
        self.seq += 1;
        let command = Command {
            slot: SlotId(0),
            seq: self.seq,
            kind: CommandKind::Order(Box::new(turts_sim::OrderCommand {
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
                target,
                queue: QueueMode::Replace,
                group,
                allowed_units: policy.units.clone(),
                queue_depth: policy.queue_depth,
            })),
        };
        self.app
            .world_mut()
            .resource_mut::<CommandQueue>()
            .0
            .push_back(command);
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
}

/// One live match: both group orders, then enough ticks for both spreads and first steps.
/// The log goes through its JSON line form, as a log file does.
fn captured(fog: bool) -> Vec<EconomyLogRecord> {
    let mut run = Run::new(fog);
    if fog {
        let hidden = run.unit_at(HIDDEN_POINT);
        let seen = run.unit_at(SEEN_POINT);
        let start = ObservationPhase::Start;
        assert!(
            !turts_sim::visibility::unit_permitted(run.app.world(), 0, hidden, start),
            "the scene needs a unit that slot 0 may not see on the first point"
        );
        assert!(
            turts_sim::visibility::unit_permitted(run.app.world(), 0, seen, start),
            "the scene needs a unit that slot 0 may see on the second point"
        );
    }
    run.order(1, &GROUP_1, HIDDEN_POINT);
    run.ticks(1);
    run.order(2, &GROUP_2, SEEN_POINT);
    run.ticks(8);
    let dispatched = |records: &[EconomyLogRecord]| {
        records
            .iter()
            .filter_map(|record| match record {
                EconomyLogRecord::Tick { state, .. } => Some(state.movement_pools.len()),
                _ => None,
            })
            .max()
    };
    assert_eq!(
        dispatched(&run.records),
        Some(2),
        "both group orders received a spread pool"
    );
    let lines: Vec<_> = run
        .records
        .iter()
        .map(|record| serde_json::to_string(record).unwrap())
        .collect();
    assert_eq!(
        lines.iter().any(|line| line.contains("raw_sight")),
        fog,
        "only a restricted match writes the sight inputs"
    );
    turts_audit::read_jsonl(std::io::Cursor::new(lines.join("\n"))).unwrap()
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
/// The two refusals of a recomputed spread: the slot of the member differs, or the slot is
/// equal and the pool of the envelope differs.
const SPREAD: &str = "differs from its deterministic spread slot";
const POOL: &str = "differs from its independently modeled envelope and slot (pool=false";
fn refused_for(records: &[EconomyLogRecord], reason: &str, case: &str) {
    let error = validate_records(records).expect_err(case);
    assert!(
        error.contains(reason) || (reason == SPREAD && error.contains(POOL)),
        "{case}: {error}"
    );
}

#[test]
fn live_fog_capture_replays_with_auditor_computed_sight() {
    let records = captured(true);
    validate_records(&records).unwrap();
    validate_records_with_sight(&records, Some(&trusted_policy())).unwrap();
}

#[test]
fn live_all_visible_capture_replays_unchanged() {
    let records = captured(false);
    validate_records(&records).unwrap();
    validate_records_with_sight(&records, None).unwrap();
    let error = validate_records_with_sight(&records, Some(&trusted_policy())).unwrap_err();
    assert_eq!(
        error,
        "economy log sight policy differs from the trusted policy"
    );
}

/// The two modes give different spreads in this scene. Each forgery below claims the rule of
/// the other mode in a consistent way in every snapshot. The spread recomputed by the auditor
/// then differs from the live one.
#[test]
fn forged_sight_mode_is_refused() {
    let fog = captured(true);
    let all_visible = captured(false);
    let sight = match &fog[0] {
        EconomyLogRecord::Initial(state) => state.raw_sight.clone().unwrap(),
        _ => panic!("initial record"),
    };
    let stripped = forged(&fog, |sight| *sight = None);
    refused_for(&stripped, SPREAD, "a fog log that claims AllVisible");
    assert_eq!(
        validate_records_with_sight(&stripped, Some(&trusted_policy())).unwrap_err(),
        "economy log sight policy differs from the trusted policy"
    );
    let claimed = forged(&all_visible, |slot| *slot = Some(sight.clone()));
    refused_for(&claimed, SPREAD, "an AllVisible log that claims fog");
}

/// A log cannot move a unit into or out of sight through its sight inputs.
#[test]
fn forged_sight_inputs_are_refused() {
    let fog = captured(true);
    let policy = trusted_policy();

    // Wide vision would show the hidden unit; the auditor then reserves its footprint.
    let wide = forged(&fog, |sight| {
        for kind in &mut sight.as_mut().unwrap().types {
            kind.radius = Some(100.0);
        }
    });
    refused_for(&wide, SPREAD, "forged wide vision");

    // Narrow vision would hide the seen unit; the auditor then releases its footprint.
    let narrow = forged(&fog, |sight| {
        for kind in &mut sight.as_mut().unwrap().types {
            kind.radius = Some(0.5);
        }
    });
    refused_for(&narrow, SPREAD, "forged narrow vision");

    // Team sharing with one team would give slot 0 the sight of slot 1.
    let shared = forged(&fog, |sight| {
        let sight = sight.as_mut().unwrap();
        sight.policy.sharing = VisionSharing::Team;
        sight.policy.teams = vec![0, 0];
    });
    refused_for(&shared, SPREAD, "forged shared sight");
    assert_eq!(
        validate_records_with_sight(&shared, Some(&policy)).unwrap_err(),
        "economy log sight policy differs from the trusted policy"
    );

    // The fail-closed placeholder hides every unit of another slot, also the seen one.
    let placeholder = forged(&fog, |sight| sight.as_mut().unwrap().finalized = false);
    refused_for(&placeholder, SPREAD, "forged placeholder basis");

    // The inputs are constant. One changed snapshot is refused before any replay.
    let mut drifting = fog.clone();
    let mut count = 0;
    each_state(&mut drifting, |state| {
        count += 1;
        if count == 4 {
            state.raw_sight.as_mut().unwrap().policy.teams = vec![0, 0];
        }
    });
    refused_for(
        &drifting,
        "sight inputs changed after the Initial record",
        "sight inputs that change in one snapshot",
    );

    let mut grid = fog.clone();
    each_state(&mut grid, |state| {
        state.raw_sight.as_mut().unwrap().policy.grid.columns = 16;
    });
    refused_for(
        &grid,
        "audited sight grid differs from its checked geometry",
        "a grid that differs from its size and cell size",
    );
}

/// A lobby server writes `Initial` with the placeholder and finalizes the context at the match
/// start. That one change is permitted before the first tick only.
#[test]
fn sight_may_be_finalized_only_before_the_first_tick() {
    let fog = captured(true);
    let placeholder = |sight: &mut Option<SightAudit>| {
        let sight = sight.as_mut().unwrap();
        sight.finalized = false;
        sight.policy.teams.clear();
    };
    let mut at_start = fog.clone();
    each_state(&mut at_start[..1], |state| {
        placeholder(&mut state.raw_sight)
    });
    validate_records(&at_start).unwrap();
    validate_records_with_sight(&at_start, Some(&trusted_policy())).unwrap();

    let first_tick = fog
        .iter()
        .position(|record| matches!(record, EconomyLogRecord::Tick { .. }))
        .unwrap();
    let mut late = fog.clone();
    each_state(&mut late[..=first_tick], |state| {
        placeholder(&mut state.raw_sight)
    });
    refused_for(
        &late,
        "sight inputs changed after the Initial record",
        "a sight context that is finalized after a tick",
    );

    // The start change may not touch the grid, the sharing rule or the unit types.
    let mut other = fog.clone();
    each_state(&mut other[..1], |state| {
        placeholder(&mut state.raw_sight);
        state.raw_sight.as_mut().unwrap().types[0].radius = Some(100.0);
    });
    refused_for(
        &other,
        "sight inputs changed after the Initial record",
        "a start change of a vision radius",
    );
}

/// Not modelled: a pending group move to a unit under fog (the server prunes a hidden
/// target). The auditor must refuse such a log; it must not certify it with the wrong rule.
#[test]
fn fog_group_move_to_a_unit_is_refused_not_certified() {
    let mut run = Run::new(true);
    let seen = run.unit_at(SEEN_POINT);
    run.order_target(1, &GROUP_2, OrderTargetSim::Unit(seen));
    run.ticks(4);
    refused_for(
        &run.records,
        "restricted sight replay does not support a pending Unit-target movement",
        "a pending Unit-target group move under fog",
    );
}

#[test]
fn fog_singleton_move_to_hidden_body_replays() {
    let mut run = Run::new(true);
    run.order(1, &GROUP_1[..1], HIDDEN_POINT);
    run.ticks(2);
    validate_records(&run.records).unwrap();
}

#[test]
fn fog_direct_move_to_hidden_body_replays() {
    let mut run = Run::new(true);
    let id = run.unit_at(GROUP_1[0]);
    let entity = run
        .app
        .world()
        .iter_entities()
        .find(|e| e.get::<turts_sim::UnitId>().is_some_and(|i| i.0 == id))
        .unwrap()
        .id();
    run.app
        .world_mut()
        .entity_mut(entity)
        .insert(turts_sim::world::MoveTarget(Vec2::new(
            HIDDEN_POINT.0,
            HIDDEN_POINT.1,
        )));
    run.records = vec![EconomyLogRecord::Initial(turts_sim::audit_state(
        run.app.world_mut(),
    ))];
    run.ticks(180);
    validate_records(&run.records).unwrap();
    let stripped = forged(&run.records, |sight| *sight = None);
    let error = validate_records(&stripped).expect_err("a false all-visible movement basis");
    assert!(
        error.contains("movement transition changed nonmovement state"),
        "{error}"
    );
}
