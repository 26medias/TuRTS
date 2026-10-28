mod common;

use common::conservation::{OracleCatalog, OracleHistory, check_operation, check_tick};
use std::{
    fs,
    path::{Path, PathBuf},
};
use turts_mod_api::ContentId;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_pack::load_package;
use turts_sim::{
    AuditState, AuditTrail, Command, CommandKind, EconomyEvent, EconomyEventKind, EventSource,
    JobId, JobStage, OperationAudit, ReservationClaimView, ReservationEndpointView,
    ReservationLineView, SlotId, apply_command, audit_state, headless_app,
};

fn forge_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/forge")
}
fn fixture_copy(name: &str, edits: &[(&str, &str)]) -> PathBuf {
    let source = forge_dir();
    let target = std::env::temp_dir().join(format!("turts-t14-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&target);
    copy_tree(&source, &target);
    for (from, to) in edits {
        let relative = if from.contains("health: Some") {
            "units/drone.ron"
        } else if from.contains("capacity: 4") {
            "units/drill.ron"
        } else {
            "maps/proving_ground.ron"
        };
        let path = target.join(relative);
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains(from), "fixture marker must exist: {from}");
        fs::write(path, text.replace(from, to)).unwrap();
    }
    if name == "c5-refund-room" {
        fs::write(
            target.join("units/ore_depot.ron"),
            r#"(
    id: "ore_depot",
    name: "Ore Depot",
    footprint: Circle(radius: 0.4),
    movement: None,
    inventory: Some((capacity: 100, accepts: ["forge:ore"])),
    gatherer: None,
    drop_off: None,
    producer: None,
)"#,
        )
        .unwrap();
        let manifest = target.join("manifest.ron");
        let text = fs::read_to_string(&manifest).unwrap();
        fs::write(
            manifest,
            text.replace(
                "units/drone.ron\",",
                "units/drone.ron\",\n            \"units/ore_depot.ron\",",
            ),
        )
        .unwrap();
        let map = target.join("maps/proving_ground.ron");
        let text = fs::read_to_string(&map).unwrap();
        fs::write(map, text.replace("(unit: \"forge:drone\", owner_slot: 1, pos: (x: 30.0, y: 4.0), contents: []),", "(unit: \"forge:drone\", owner_slot: 1, pos: (x: 30.0, y: 4.0), contents: []),\n        (unit: \"forge:ore_depot\", owner_slot: 0, pos: (x: 2.0, y: 2.0), contents: []),")).unwrap();
    }
    target
}
fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}
fn archive(path: &Path) -> PackageArchive {
    PackageArchive::parse(&pack_dir(path).unwrap()).unwrap()
}
fn app_for(path: &Path) -> bevy_app::App {
    let mut app = headless_app(20);
    load_package(app.world_mut(), &archive(path)).unwrap();
    app.world_mut().resource_mut::<AuditTrail>().enable();
    app
}
fn catalog(path: &Path) -> OracleCatalog {
    OracleCatalog::parse(path).unwrap()
}
fn cid(text: &str) -> ContentId {
    ContentId::parse(text).unwrap()
}
fn audit_ops(
    app: &mut bevy_app::App,
    oracle: &OracleCatalog,
    history: &mut OracleHistory,
) -> Vec<OperationAudit> {
    let ops = app
        .world_mut()
        .resource_mut::<AuditTrail>()
        .drain_operations();
    for op in &ops {
        check_operation(op, oracle).unwrap_or_else(|e| {
            panic!(
                "operation {}: {}: {}",
                op.operation_id, e.invariant, e.detail
            )
        });
    }
    check_tick(&ops, history, oracle)
        .unwrap_or_else(|e| panic!("tick check {}: {}", e.invariant, e.detail));
    ops
}
fn step(
    app: &mut bevy_app::App,
    oracle: &OracleCatalog,
    history: &mut OracleHistory,
) -> Vec<OperationAudit> {
    app.update();
    audit_ops(app, oracle, history)
}
fn command(
    app: &mut bevy_app::App,
    oracle: &OracleCatalog,
    history: &mut OracleHistory,
    slot: u8,
    seq: u32,
    kind: CommandKind,
) -> turts_sim::CommandResult {
    let command = Command {
        slot: SlotId(slot),
        seq,
        kind,
    };
    let result = apply_command(app.world_mut(), &command);
    audit_ops(app, oracle, history);
    result
}
fn loaded(path: &Path) -> (bevy_app::App, OracleCatalog, OracleHistory) {
    let oracle = catalog(path);
    let mut app = app_for(path);
    let initial = audit_state(app.world_mut());
    let history = OracleHistory::seed(&initial);
    (app, oracle, history)
}
fn inventory_amount(state: &AuditState, unit: u32, item: &str) -> u64 {
    let item = cid(item);
    state
        .economy
        .inventories
        .iter()
        .find(|i| i.unit == unit)
        .and_then(|i| i.items.iter().find(|x| x.resource == item))
        .map_or(0, |x| x.on_hand)
}
fn press_job(state: &AuditState) -> Option<turts_sim::economy::views::RawJobAudit> {
    state
        .raw_queues
        .iter()
        .find(|q| q.producer == 4)
        .and_then(|q| q.jobs.first())
        .cloned()
}
fn run_until(
    app: &mut bevy_app::App,
    oracle: &OracleCatalog,
    history: &mut OracleHistory,
    max_ticks: u32,
    mut pred: impl FnMut(&AuditState) -> bool,
    why: &str,
) -> u32 {
    for tick in 1..=max_ticks {
        step(app, oracle, history);
        let state = audit_state(app.world_mut());
        if pred(&state) {
            return tick;
        }
    }
    panic!("{why}: condition did not occur within {max_ticks} ticks");
}

#[test]
fn oracle_catalog_parses_gameplay_source_ron_without_audit_catalog() {
    let target =
        std::env::temp_dir().join(format!("turts-t14-oracle-source-{}", std::process::id()));
    let _ = fs::remove_dir_all(&target);
    copy_tree(&forge_dir(), &target);
    let turret = target.join("units/turret.ron");
    let text = fs::read_to_string(&turret).unwrap();
    assert!(text.contains("damage: 10"));
    fs::write(turret, text.replace("damage: 10", "damage: 17")).unwrap();
    let parsed = OracleCatalog::parse(&target).unwrap();
    assert_eq!(
        parsed.units[&cid("forge:turret")]
            .weapon
            .as_ref()
            .unwrap()
            .damage,
        17,
        "the oracle reads the edited source RON directly"
    );
}

#[test]
fn bal_ore_fails_on_lost_item() {
    let (mut app, oracle, _) = loaded(&forge_dir());
    let before = audit_state(app.world_mut());
    let mut after = before.clone();
    after.nodes[0].remaining -= 1;
    let op = OperationAudit {
        operation_id: 1,
        before,
        after,
        events: vec![],
    };
    assert_eq!(
        check_operation(&op, &oracle).unwrap_err().invariant,
        "BAL-ore"
    );
}
#[test]
fn bal_shell_fails_on_duplicated_shell() {
    let (mut app, oracle, _) = loaded(&forge_dir());
    let before = audit_state(app.world_mut());
    let mut after = before.clone();
    let inventory = after
        .economy
        .inventories
        .iter_mut()
        .find(|r| r.unit == 4)
        .unwrap();
    inventory.items.push(turts_sim::ItemView {
        resource: cid("forge:shell"),
        on_hand: 1,
        reserved: 0,
    });
    let op = OperationAudit {
        operation_id: 1,
        before,
        after,
        events: vec![],
    };
    assert_eq!(
        check_operation(&op, &oracle).unwrap_err().invariant,
        "BAL-shell"
    );
}
#[test]
fn shot_fails_when_event_ammo_zero() {
    let (mut app, oracle, _) = loaded(&forge_dir());
    let state = audit_state(app.world_mut());
    let event = EconomyEvent {
        tick: 1,
        ordinal: 1,
        owner_slot: 0,
        kind: EconomyEventKind::Fired {
            weapon: 7,
            target: 9,
            item: Some(cid("forge:shell")),
            ammo: 0,
            damage: 10,
            applied: 10,
        },
    };
    let op = OperationAudit {
        operation_id: 1,
        before: state.clone(),
        after: state,
        events: vec![event],
    };
    assert_eq!(check_operation(&op, &oracle).unwrap_err().invariant, "SHOT");
}
#[test]
fn res_fails_on_active_claim_without_job() {
    let (mut app, _, _) = loaded(&forge_dir());
    let mut state = audit_state(app.world_mut());
    state.ledger_claims.push(ReservationClaimView::Active {
        map_job_id: 999,
        token_job_id: 999,
        lines: vec![ReservationLineView {
            endpoint: ReservationEndpointView::Inventory(8),
            resource: cid("forge:shell"),
            amount: 1,
        }],
    });
    let oracle = catalog(&forge_dir());
    assert_eq!(
        common::conservation::check_reservations(&state, &oracle)
            .unwrap_err()
            .invariant,
        "RES"
    );
}
#[test]
fn dmg_fails_on_wrong_damage() {
    let (mut app, oracle, _) = loaded(&forge_dir());
    let mut before = audit_state(app.world_mut());
    let mut after = before.clone();
    let hp = before.raw_healths.iter_mut().find(|h| h.unit == 9).unwrap();
    hp.current = 100;
    let hp = after.raw_healths.iter_mut().find(|h| h.unit == 9).unwrap();
    hp.current = 91;
    let event = EconomyEvent {
        tick: 1,
        ordinal: 1,
        owner_slot: 0,
        kind: EconomyEventKind::Fired {
            weapon: 7,
            target: 9,
            item: Some(cid("forge:shell")),
            ammo: 1,
            damage: 9,
            applied: 9,
        },
    };
    let op = OperationAudit {
        operation_id: 1,
        before,
        after,
        events: vec![event],
    };
    let mut history = OracleHistory::default();
    assert_eq!(
        check_tick(&[op], &mut history, &oracle)
            .unwrap_err()
            .invariant,
        "DMG"
    );
}
#[test]
fn refund_fails_on_wrong_ratio() {
    let (_, oracle, _) = loaded(&forge_dir());
    let mut before = empty_audit();
    let after = before.clone();
    before
        .raw_queues
        .push(turts_sim::economy::views::RawQueueAudit {
            producer: 4,
            jobs: vec![turts_sim::economy::views::RawJobAudit {
                id: 5,
                recipe: cid("forge:make_shells"),
                stage: JobStage::Running,
                elapsed_ticks: 1,
                duration_ticks: 41,
                charged: vec![ReservationLineView {
                    endpoint: ReservationEndpointView::Inventory(4),
                    resource: cid("forge:ore"),
                    amount: 2,
                }],
                reservation: None,
            }],
        });
    let event = EconomyEvent {
        tick: 1,
        ordinal: 1,
        owner_slot: 0,
        kind: EconomyEventKind::ProductionCancelled {
            producer: 4,
            job: 5,
            released: vec![],
            refunded: vec![turts_sim::CostLine {
                source: EventSource::Inventory(4),
                resource: cid("forge:ore"),
                amount: 1,
            }],
        },
    };
    let op = OperationAudit {
        operation_id: 1,
        before,
        after,
        events: vec![event],
    };
    assert_eq!(
        common::conservation::check_refunds(&op, &oracle)
            .unwrap_err()
            .invariant,
        "REFUND"
    );
}
#[test]
fn dangling_fails_on_leftover_row() {
    let (mut app, _, _) = loaded(&forge_dir());
    let mut state = audit_state(app.world_mut());
    state.removed_units.push(4);
    assert_eq!(
        common::conservation::check_dangling(&state)
            .unwrap_err()
            .invariant,
        "DANGLING"
    );
}
fn empty_audit() -> AuditState {
    let mut app = headless_app(20);
    audit_state(app.world_mut())
}

// Scenario acceptance tests below each keep the public audit trail and the source-RON oracle in the loop.
#[test]
fn c1_belt_full_drill_stops() {
    let (mut app, oracle, mut history) = loaded(&forge_dir());
    run_until(
        &mut app,
        &oracle,
        &mut history,
        5000,
        |s| {
            s.raw_extractors.iter().any(|e| {
                e.unit == 1
                    && e.status == turts_sim::economy::views_m3::RawExtractorStatusAudit::Full
            })
        },
        "C1 extractor Full",
    );
    let mut extractor_retry_operations = 0;
    for _ in 0..100 {
        let ops = step(&mut app, &oracle, &mut history);
        extractor_retry_operations += ops
            .iter()
            .filter(|op| {
                let before = op.before.raw_extractors.iter().find(|e| e.unit == 1);
                let after = op.after.raw_extractors.iter().find(|e| e.unit == 1);
                before != after
            })
            .count();
    }
    let s = audit_state(app.world_mut());
    assert_eq!(
        extractor_retry_operations, 100,
        "the full extractor must execute its capacity retry once on every bounded tick"
    );
    assert!(s.raw_extractors.iter().any(|e| {
        e.unit == 1 && e.status == turts_sim::economy::views_m3::RawExtractorStatusAudit::Full
    }));
    assert!(
        s.raw_belts
            .iter()
            .flat_map(|b| &b.raw_slots)
            .filter(|x| x.item.is_some())
            .count()
            <= 24
    );
}
#[test]
fn c2_press_output_blocked_holds_charge_until_one_output_fits() {
    let (mut app, oracle, mut history) = loaded(&forge_dir());
    run_until(
        &mut app,
        &oracle,
        &mut history,
        5000,
        |s| press_job(s).is_some_and(|j| j.stage == JobStage::OutputBlocked),
        "C2 OutputBlocked",
    );
    let blocked = audit_state(app.world_mut());
    let job = press_job(&blocked).unwrap();
    assert_eq!(job.elapsed_ticks, job.duration_ticks);
    let ore_before = inventory_amount(&blocked, 4, "forge:ore");
    let output_before = common::forge_support::output_amount(&blocked, 4, "forge:shell");
    assert!(
        command(
            &mut app,
            &oracle,
            &mut history,
            1,
            1,
            CommandKind::MoveTo {
                unit_ids: vec![10],
                target: turts_sim::Vec2::new(24.0, 14.0)
            }
        )
        .outcome
        .is_ok()
    );
    let mut credited = 0_u64;
    for _ in 0..500 {
        for op in step(&mut app, &oracle, &mut history) {
            for event in op.events {
                if let EconomyEventKind::ItemsProduced {
                    job: completed,
                    outputs,
                    ..
                } = event.kind
                    && completed == job.id
                {
                    credited += outputs.iter().map(|row| row.amount).sum::<u64>();
                }
            }
        }
        if credited > 0 {
            break;
        }
    }
    assert_eq!(
        credited, 1,
        "blocked head job credits exactly one shell when output room returns"
    );
    let after = audit_state(app.world_mut());
    assert!(
        inventory_amount(&after, 4, "forge:ore") <= ore_before,
        "blocked retries do not recharge ore"
    );
    assert!(common::forge_support::output_amount(&after, 4, "forge:shell") >= output_before);
}
#[test]
fn c3_backpressure_chain_stops_in_order() {
    let (mut app, oracle, mut h) = loaded(&forge_dir());
    run_until(
        &mut app,
        &oracle,
        &mut h,
        7000,
        |s| {
            s.raw_extractors
                .iter()
                .any(|e| e.status == turts_sim::economy::views_m3::RawExtractorStatusAudit::Full)
                && press_job(s).is_some_and(|j| j.stage == JobStage::OutputBlocked)
        },
        "C3 blocked chain",
    );
    let s = audit_state(app.world_mut());
    assert!(
        s.raw_belts
            .iter()
            .any(|b| b.raw_slots.iter().all(|x| x.item.is_some()))
    );
    let arm = s.raw_inserters.iter().find(|r| r.unit == 3).unwrap();
    assert_eq!(
        inventory_amount(&s, 3, "forge:ore") + inventory_amount(&s, 3, "forge:shell"),
        0,
        "K7 leaves no held item when destination is full"
    );
    assert!(matches!(
        arm.stage,
        turts_sim::economy::views_m3::RawInserterStageAudit::AtSource(Some(
            turts_sim::economy::views_m3::RawInserterWaitAudit::DestinationFull
        ))
    ));
    assert_eq!(
        s.raw_extractors
            .iter()
            .find(|e| e.unit == 1)
            .unwrap()
            .status,
        turts_sim::economy::views_m3::RawExtractorStatusAudit::Full
    );
}
#[test]
fn c4_cancel_running_press_job_refunds_once() {
    let (mut app, oracle, mut h) = loaded(&forge_dir());
    run_until(
        &mut app,
        &oracle,
        &mut h,
        1500,
        |s| press_job(s).is_some_and(|j| j.stage == JobStage::Running),
        "C4 running press job",
    );
    let j = press_job(&audit_state(app.world_mut())).unwrap();
    let r = command(
        &mut app,
        &oracle,
        &mut h,
        0,
        1,
        CommandKind::CancelProduction {
            producer_id: 4,
            job_id: JobId(j.id),
        },
    );
    assert!(r.outcome.is_ok());
}
#[test]
fn c5_cancel_output_blocked_no_output_credit() {
    let path = fixture_copy("c5-refund-room", &[("capacity: 4", "capacity: 20")]);
    let (mut app, oracle, mut h) = loaded(&path);
    run_until(
        &mut app,
        &oracle,
        &mut h,
        5000,
        |s| press_job(s).is_some_and(|j| j.stage == JobStage::OutputBlocked),
        "C5 blocked job",
    );
    let j = press_job(&audit_state(app.world_mut())).unwrap();
    let transfer = turts_sim::economy::transaction::inventory_transfer(
        app.world_mut(),
        SlotId(0),
        4,
        11,
        cid("forge:ore"),
        2,
    );
    assert!(
        transfer.is_ok(),
        "C5 fixture transfer must create refund capacity: {transfer:?}"
    );
    audit_ops(&mut app, &oracle, &mut h);
    let before = audit_state(app.world_mut());
    let r = command(
        &mut app,
        &oracle,
        &mut h,
        0,
        1,
        CommandKind::CancelProduction {
            producer_id: 4,
            job_id: JobId(j.id),
        },
    );
    assert!(r.outcome.is_ok(), "C5 cancellation result: {:?}", r.outcome);
    let after = audit_state(app.world_mut());
    assert_eq!(
        inventory_amount(&before, 4, "forge:shell"),
        inventory_amount(&after, 4, "forge:shell")
    );
}
#[test]
fn c6_cancel_running_research_uses_source_ratio() {
    let path = fixture_copy(
        "c6-lab-stock",
        &[(
            r#"unit: "forge:lab", owner_slot: 0, pos: (x: 17.0, y: 4.0), contents: []"#,
            r#"unit: "forge:lab", owner_slot: 0, pos: (x: 17.0, y: 4.0), contents: [(item: "forge:shell", amount: 10)]"#,
        )],
    );
    let (mut app, oracle, mut h) = loaded(&path);
    let r = command(
        &mut app,
        &oracle,
        &mut h,
        0,
        1,
        CommandKind::Produce {
            producer_id: 8,
            recipe_id: cid("forge:hardened_shells"),
        },
    );
    assert!(r.outcome.is_ok());
    for _ in 0..2 {
        step(&mut app, &oracle, &mut h);
    }
    let s = audit_state(app.world_mut());
    let j = s.raw_queues.iter().find(|q| q.producer == 8).unwrap().jobs[0].clone();
    assert_eq!(j.stage, JobStage::Running);
    let r = command(
        &mut app,
        &oracle,
        &mut h,
        0,
        2,
        CommandKind::CancelProduction {
            producer_id: 8,
            job_id: JobId(j.id),
        },
    );
    assert!(r.outcome.is_ok());
    let after = audit_state(app.world_mut());
    assert!(
        !after
            .research
            .iter()
            .any(|row| row.technology == cid("forge:hardened_shells"))
    );
    assert_eq!(
        after
            .raw_weapons
            .iter()
            .find(|row| row.unit == 7)
            .unwrap()
            .effective_damage,
        Ok(10)
    );
}
#[test]
fn c7_cancel_queued_releases_reservation() {
    let path = fixture_copy(
        "c7-lab-stock",
        &[(
            r#"unit: "forge:lab", owner_slot: 0, pos: (x: 17.0, y: 4.0), contents: []"#,
            r#"unit: "forge:lab", owner_slot: 0, pos: (x: 17.0, y: 4.0), contents: [(item: "forge:shell", amount: 10)]"#,
        )],
    );
    let (mut app, oracle, mut h) = loaded(&path);
    let shell_before = inventory_amount(&audit_state(app.world_mut()), 8, "forge:shell");
    let a = command(
        &mut app,
        &oracle,
        &mut h,
        0,
        1,
        CommandKind::Produce {
            producer_id: 8,
            recipe_id: cid("forge:hardened_shells"),
        },
    );
    assert!(a.outcome.is_ok());
    let id = a.outcome.unwrap().job_id.unwrap();
    let b = command(
        &mut app,
        &oracle,
        &mut h,
        0,
        2,
        CommandKind::CancelProduction {
            producer_id: 8,
            job_id: id,
        },
    );
    assert!(b.outcome.is_ok());
    let s = audit_state(app.world_mut());
    assert!(
        s.ledger_claims
            .iter()
            .all(|c| !matches!(c, ReservationClaimView::Active { .. }))
    );
    assert_eq!(inventory_amount(&s, 8, "forge:shell"), shell_before);
}
#[test]
fn c8_pause_and_resume_freezes_capability_progress() {
    let (mut app, oracle, mut h) = loaded(&forge_dir());
    run_until(
        &mut app,
        &oracle,
        &mut h,
        1500,
        |s| press_job(s).is_some_and(|j| j.stage == JobStage::Running),
        "C8 running press job",
    );
    for (seq, id) in [1, 2, 4, 8, 7].into_iter().enumerate() {
        assert!(
            command(
                &mut app,
                &oracle,
                &mut h,
                0,
                seq as u32 + 1,
                CommandKind::SetEnabled {
                    unit_id: id,
                    enabled: false
                }
            )
            .outcome
            .is_ok()
        );
    }
    let paused = audit_state(app.world_mut());
    for _ in 0..20 {
        step(&mut app, &oracle, &mut h);
    }
    let frozen = audit_state(app.world_mut());
    assert!(
        frozen.disabled.contains(&1)
            && frozen.disabled.contains(&2)
            && frozen.disabled.contains(&4)
            && frozen.disabled.contains(&8)
            && frozen.disabled.contains(&7)
    );
    assert_eq!(frozen.raw_extractors, paused.raw_extractors);
    assert_eq!(frozen.raw_inserters, paused.raw_inserters);
    assert_eq!(press_job(&frozen), press_job(&paused));
    assert_eq!(frozen.raw_weapons, paused.raw_weapons);
    for (seq, id) in [1, 2, 4, 8, 7].into_iter().enumerate() {
        assert!(
            command(
                &mut app,
                &oracle,
                &mut h,
                0,
                100 + seq as u32,
                CommandKind::SetEnabled {
                    unit_id: id,
                    enabled: true
                }
            )
            .outcome
            .is_ok()
        );
    }
    run_until(
        &mut app,
        &oracle,
        &mut h,
        20,
        |s| {
            press_job(s)
                .is_some_and(|j| j.elapsed_ticks > press_job(&paused).unwrap().elapsed_ticks)
        },
        "C8 resume progress",
    );
}
#[test]
fn c9_remove_piece_thirteen_lists_its_contents_as_destroyed() {
    let (mut app, oracle, mut h) = loaded(&forge_dir());
    run_until(
        &mut app,
        &oracle,
        &mut h,
        700,
        |s| {
            s.raw_belts
                .iter()
                .flat_map(|b| &b.pieces)
                .any(|p| p.unit == 13 && p.slots.iter().any(|x| x.item.is_some()))
        },
        "C9 piece 13 carries items",
    );
    let result = command(
        &mut app,
        &oracle,
        &mut h,
        0,
        1,
        CommandKind::Remove { unit_id: 13 },
    );
    assert!(result.outcome.is_ok());
    let audit = app.world().resource::<AuditTrail>();
    assert!(
        audit
            .commands
            .last()
            .unwrap()
            .after
            .removed_units
            .contains(&13)
    );
}
#[test]
fn c10_remove_inserter_mid_swing_destroys_hand_and_clears_endpoint() {
    let (mut app, oracle, mut h) = loaded(&forge_dir());
    run_until(
        &mut app,
        &oracle,
        &mut h,
        1500,
        |s| {
            (inventory_amount(s, 3, "forge:ore") > 0 || inventory_amount(s, 3, "forge:shell") > 0)
                && s.raw_inserters.iter().any(|r| {
                    r.unit == 3
                        && matches!(
                            r.stage,
                            turts_sim::economy::views_m3::RawInserterStageAudit::ToDestination
                        )
                })
        },
        "C10 inserter holds item mid-swing",
    );
    let result = command(
        &mut app,
        &oracle,
        &mut h,
        0,
        1,
        CommandKind::Remove { unit_id: 3 },
    );
    assert!(result.outcome.is_ok());
    let after = audit_state(app.world_mut());
    assert!(!after.raw_inserters.iter().any(|r| r.unit == 3));
    assert!(!after.economy.inventories.iter().any(|r| r.unit == 3));
}
#[test]
fn c11_node_depletion_stops_at_zero_and_keeps_one_ore() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/m3/packages/forge_c11");
    let (mut app, oracle, mut h) = loaded(&path);
    let mut produced = 0_u64;
    for _ in 0..3000 {
        for op in step(&mut app, &oracle, &mut h) {
            for event in op.events {
                if let EconomyEventKind::ItemsProduced { outputs, .. } = event.kind {
                    produced += outputs
                        .iter()
                        .filter(|row| row.resource == cid("forge:shell"))
                        .map(|row| row.amount)
                        .sum::<u64>();
                }
            }
        }
        let state = audit_state(app.world_mut());
        if state.nodes.first().is_some_and(|n| n.remaining == 0) && produced == 10 {
            break;
        }
    }
    let s = audit_state(app.world_mut());
    assert_eq!(s.nodes[0].remaining, 0);
    assert_eq!(produced, 10);
    assert_eq!(inventory_amount(&s, 4, "forge:ore"), 1);
    assert_eq!(
        s.raw_extractors
            .iter()
            .find(|e| e.unit == 1)
            .unwrap()
            .status,
        turts_sim::economy::views_m3::RawExtractorStatusAudit::Depleted
    );
}
#[test]
fn c12_shared_source_lowest_unit_id_wins() {
    let (mut app, oracle, mut h) = loaded(&forge_dir());
    let mut first = None;
    for tick in 0..1800 {
        step(&mut app, &oracle, &mut h);
        let s = audit_state(app.world_mut());
        if inventory_amount(&s, 5, "forge:shell") > 0 || inventory_amount(&s, 6, "forge:shell") > 0
        {
            first = Some((
                tick,
                inventory_amount(&s, 5, "forge:shell"),
                inventory_amount(&s, 6, "forge:shell"),
            ));
            break;
        }
    }
    let (_, b, c) = first.expect("a shell reaches the shared source");
    assert!(
        b > 0 && c == 0,
        "lowest unit id 5 must take the first shell, got B={b} C={c}"
    );
}
#[test]
fn c13_turret_out_of_ammo_never_fires() {
    let (mut app, oracle, mut h) = loaded(&forge_dir());
    let mut fired = 0;
    for _ in 0..160 {
        for op in step(&mut app, &oracle, &mut h) {
            fired += op
                .events
                .iter()
                .filter(|e| matches!(e.kind, EconomyEventKind::Fired { .. }))
                .count();
        }
    }
    assert_eq!(fired, 0);
    assert_eq!(
        audit_state(app.world_mut())
            .raw_weapons
            .iter()
            .find(|w| w.unit == 7)
            .unwrap()
            .status,
        turts_sim::economy::views_m3::RawWeaponStatusAudit::OutOfAmmo
    );
}
#[test]
fn c14_overkill_destroys_once_and_retargets() {
    let (mut app, oracle, mut history) = loaded(&forge_dir());
    let mut fired = Vec::new();
    for _ in 0..1000 {
        for op in step(&mut app, &oracle, &mut history) {
            for event in op.events {
                if let EconomyEventKind::Fired { target, .. } = event.kind {
                    fired.push(target);
                }
            }
        }
        if audit_state(app.world_mut()).removed_units.contains(&9) {
            break;
        }
    }
    let after_kill = audit_state(app.world_mut());
    assert!(after_kill.removed_units.contains(&9));
    assert_eq!(fired.iter().filter(|id| **id == 9).count(), 10);
    let moved = command(
        &mut app,
        &oracle,
        &mut history,
        1,
        1,
        CommandKind::MoveTo {
            unit_ids: vec![10],
            target: turts_sim::Vec2::new(24.0, 14.0),
        },
    );
    assert!(moved.outcome.is_ok());
    run_until(
        &mut app,
        &oracle,
        &mut history,
        500,
        |state| {
            state
                .raw_healths
                .iter()
                .any(|row| row.unit == 10 && row.current < 100)
        },
        "C14 retarget after kill",
    );
    assert!(
        fired.contains(&10)
            || audit_state(app.world_mut())
                .raw_healths
                .iter()
                .any(|r| r.unit == 10 && r.current < 100)
    );
}
#[test]
fn c15_research_completion_tick_shot_uses_new_damage() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/m3/packages/forge_c15");
    let (mut app, oracle, mut h) = loaded(&path);
    for _ in 0..16 {
        step(&mut app, &oracle, &mut h);
    }
    let r = command(
        &mut app,
        &oracle,
        &mut h,
        0,
        1,
        CommandKind::Produce {
            producer_id: 8,
            recipe_id: cid("forge:hardened_shells"),
        },
    );
    assert!(r.outcome.is_ok());
    let mut fired_ticks = Vec::new();
    let mut research_tick = None;
    for _ in 0..700 {
        let ops = step(&mut app, &oracle, &mut h);
        for op in &ops {
            for e in &op.events {
                match &e.kind {
                    EconomyEventKind::Fired { target, damage, .. } if *target == 9 => {
                        fired_ticks.push((e.tick, *damage))
                    }
                    EconomyEventKind::ResearchCompleted { technology, .. }
                        if technology == &cid("forge:hardened_shells") =>
                    {
                        research_tick = Some(e.tick)
                    }
                    _ => {}
                }
            }
        }
        if research_tick.is_some() {
            break;
        }
    }
    let rt = research_tick.expect("research must complete");
    assert!(
        fired_ticks.iter().any(|(t, _)| *t == rt),
        "C15 precondition: shot must land on research completion tick {rt}; shots: {fired_ticks:?}"
    );
    assert!(fired_ticks.iter().any(|(t, d)| *t < rt && *d == 10));
    assert!(fired_ticks.iter().any(|(t, d)| *t == rt && *d == 20));
}
#[test]
fn c16_target_at_twelve_stays_out_of_range_and_keeps_ammo() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/m3/packages/forge_c16");
    let (mut app, oracle, mut h) = loaded(&path);
    for _ in 0..500 {
        let ops = step(&mut app, &oracle, &mut h);
        assert!(
            ops.iter()
                .flat_map(|o| &o.events)
                .all(|e| !matches!(e.kind, EconomyEventKind::Fired { target: 9, .. }))
        );
    }
    let s = audit_state(app.world_mut());
    assert_eq!(s.units.iter().find(|u| u.unit_id == 9).unwrap().pos.x, 29.0);
    assert_eq!(inventory_amount(&s, 7, "forge:shell"), 5);
}
fn locked_fixture() -> PathBuf {
    let source =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/m3/packages/forge_locked");
    let target =
        std::env::temp_dir().join(format!("turts-t14-forge-locked-{}", std::process::id()));
    let _ = fs::remove_dir_all(&target);
    copy_tree(&source, &target);
    let map = target.join("maps/proving_ground.ron");
    let text = fs::read_to_string(&map).unwrap();
    let from = r#"unit: "forge_locked:lab", owner_slot: 0, pos: (x: 17.0, y: 4.0), contents: []"#;
    let to = r#"unit: "forge_locked:lab", owner_slot: 0, pos: (x: 17.0, y: 4.0), contents: [(item: "forge_locked:shell", amount: 10)]"#;
    assert!(text.contains(from));
    fs::write(map, text.replace(from, to)).unwrap();
    target
}
fn rejected_unchanged(
    app: &mut bevy_app::App,
    oracle: &OracleCatalog,
    history: &mut OracleHistory,
    slot: u8,
    seq: u32,
    kind: CommandKind,
    expected: turts_sim::RejectReason,
) {
    let before = audit_state(app.world_mut());
    let result = command(app, oracle, history, slot, seq, kind);
    assert_eq!(result.outcome.unwrap_err(), expected);
    assert_eq!(
        audit_state(app.world_mut()),
        before,
        "rejected command must leave state and audit unchanged"
    );
}
#[test]
fn c17_ownership_and_policy_rejections_are_exact() {
    let path = fixture_copy(
        "c17-lab-stock",
        &[(
            r#"unit: "forge:lab", owner_slot: 0, pos: (x: 17.0, y: 4.0), contents: []"#,
            r#"unit: "forge:lab", owner_slot: 0, pos: (x: 17.0, y: 4.0), contents: [(item: "forge:shell", amount: 10)]"#,
        )],
    );
    let (mut app, oracle, mut history) = loaded(&path);
    rejected_unchanged(
        &mut app,
        &oracle,
        &mut history,
        1,
        1,
        CommandKind::Remove { unit_id: 11 },
        turts_sim::RejectReason::NotOwner,
    );
    rejected_unchanged(
        &mut app,
        &oracle,
        &mut history,
        1,
        2,
        CommandKind::SetEnabled {
            unit_id: 1,
            enabled: false,
        },
        turts_sim::RejectReason::NotOwner,
    );
    rejected_unchanged(
        &mut app,
        &oracle,
        &mut history,
        0,
        3,
        CommandKind::Remove { unit_id: 4 },
        turts_sim::RejectReason::NotRemovable,
    );
    rejected_unchanged(
        &mut app,
        &oracle,
        &mut history,
        0,
        4,
        CommandKind::SetEnabled {
            unit_id: 11,
            enabled: false,
        },
        turts_sim::RejectReason::NotToggleable,
    );
    let produce = command(
        &mut app,
        &oracle,
        &mut history,
        0,
        5,
        CommandKind::Produce {
            producer_id: 8,
            recipe_id: cid("forge:hardened_shells"),
        },
    );
    assert!(produce.outcome.is_ok());
    rejected_unchanged(
        &mut app,
        &oracle,
        &mut history,
        0,
        6,
        CommandKind::Produce {
            producer_id: 8,
            recipe_id: cid("forge:hardened_shells"),
        },
        turts_sim::RejectReason::ResearchInProgress,
    );
    for _ in 0..205 {
        step(&mut app, &oracle, &mut history);
    }
    rejected_unchanged(
        &mut app,
        &oracle,
        &mut history,
        0,
        7,
        CommandKind::Produce {
            producer_id: 8,
            recipe_id: cid("forge:hardened_shells"),
        },
        turts_sim::RejectReason::AlreadyResearched,
    );
    let owner = command(
        &mut app,
        &oracle,
        &mut history,
        0,
        8,
        CommandKind::SetEnabled {
            unit_id: 1,
            enabled: false,
        },
    );
    assert!(owner.outcome.is_ok(), "owner command still applies");

    let locked = locked_fixture();
    let (mut app, oracle, mut history) = loaded(&locked);
    rejected_unchanged(
        &mut app,
        &oracle,
        &mut history,
        0,
        1,
        CommandKind::Produce {
            producer_id: 8,
            recipe_id: cid("forge_locked:precision_shells"),
        },
        turts_sim::RejectReason::PrerequisitesMissing,
    );
}
