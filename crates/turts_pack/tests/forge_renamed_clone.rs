mod common;
use common::{
    conservation::{OracleCatalog, OracleHistory, check_operation, check_tick},
    forge_support::{copy_tree, forge_dir},
};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};
use turts_mod_api::ContentId;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_pack::load_package;
use turts_sim::{
    AuditTrail, Command, CommandKind, EconomyEventKind, OperationAudit, SlotId, Vec2,
    apply_command, audit_state, headless_app,
};

static NEXT_CLONE: AtomicUsize = AtomicUsize::new(0);

fn renamed_clone() -> PathBuf {
    let clone_number = NEXT_CLONE.fetch_add(1, Ordering::Relaxed);
    let target = std::env::temp_dir().join(format!(
        "turts-t14-renamed-clone-{}-{clone_number}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&target);
    copy_tree(&forge_dir(), &target);
    let mut ids = vec![
        "ore",
        "shell",
        "ore_vein",
        "drill",
        "belt",
        "arm_loader",
        "press",
        "turret",
        "lab",
        "drone",
        "make_shells",
        "hardened_shells",
        "proving_ground",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<Vec<_>>();
    let forge = PackageArchive::parse(&pack_dir(&forge_dir()).unwrap()).unwrap();
    ids.extend(
        forge
            .orders()
            .unwrap()
            .into_iter()
            .map(|(_, order)| order.id),
    );
    let mut files = Vec::new();
    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out)
            } else if path.extension().is_some_and(|x| x == "ron") {
                out.push(path)
            }
        }
    }
    walk(&target, &mut files);
    for path in files {
        let mut text = fs::read_to_string(&path)
            .unwrap()
            .replace("id: \"forge\"", "id: \"clone\"");
        for id in &ids {
            text = text.replace(&format!("forge:{id}"), &format!("clone:x_{id}"));
            text = text.replace(&format!("id: \"{id}\""), &format!("id: \"x_{id}\""));
        }
        fs::write(path, text).unwrap();
    }
    target
}
fn loaded(path: &std::path::Path) -> (bevy_app::App, OracleCatalog, OracleHistory) {
    let bytes = pack_dir(path).unwrap();
    let archive = PackageArchive::parse(&bytes).unwrap();
    let mut app = headless_app(20);
    load_package(app.world_mut(), &archive).unwrap();
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let oracle = OracleCatalog::parse(path).unwrap();
    let history = OracleHistory::seed(&audit_state(app.world_mut()));
    (app, oracle, history)
}
fn drain_checked(
    app: &mut bevy_app::App,
    oracle: &OracleCatalog,
    history: &mut OracleHistory,
) -> Vec<OperationAudit> {
    let ops = app
        .world_mut()
        .resource_mut::<AuditTrail>()
        .drain_operations();
    for op in &ops {
        check_operation(op, oracle).unwrap_or_else(|e| panic!("{}: {}", e.invariant, e.detail));
    }
    check_tick(&ops, history, oracle).unwrap();
    ops
}
fn issue(
    app: &mut bevy_app::App,
    oracle: &OracleCatalog,
    history: &mut OracleHistory,
    slot: u8,
    seq: u32,
    kind: CommandKind,
) -> Vec<OperationAudit> {
    let result = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(slot),
            seq,
            kind,
        },
    );
    assert!(
        result.outcome.is_ok(),
        "logical command rejected: {result:?}"
    );
    drain_checked(app, oracle, history)
}
fn package_content(package: &str, local: &str) -> ContentId {
    let local = if package == "clone" {
        format!("x_{local}")
    } else {
        local.to_owned()
    };
    ContentId::parse(&format!("{package}:{local}")).unwrap()
}
fn inventory_amount(state: &turts_sim::AuditState, unit: u32, item: &ContentId) -> u64 {
    state
        .economy
        .inventories
        .iter()
        .find(|row| row.unit == unit)
        .and_then(|row| row.items.iter().find(|entry| &entry.resource == item))
        .map_or(0, |entry| entry.on_hand)
}
#[derive(Debug, Default, PartialEq, Eq)]
struct FlowCoverage {
    extracted: u64,
    produced: u64,
    fired: u64,
    research_completed: bool,
    research_tick: Option<u32>,
    target10_shots: Vec<(u32, u64)>,
    target10_destroyed_tick: Option<u32>,
}
fn record_flow(
    ops: &[OperationAudit],
    ore: &ContentId,
    shell: &ContentId,
    research: &ContentId,
    coverage: &mut FlowCoverage,
) {
    for event in ops.iter().flat_map(|op| &op.events) {
        match &event.kind {
            EconomyEventKind::Extracted { item, amount, .. } if item == ore => {
                coverage.extracted += amount;
            }
            EconomyEventKind::ItemsProduced { outputs, .. } => {
                coverage.produced += outputs
                    .iter()
                    .filter(|line| &line.resource == shell)
                    .map(|line| line.amount)
                    .sum::<u64>();
            }
            EconomyEventKind::Fired {
                target,
                damage,
                item: Some(item),
                ammo,
                ..
            } if item == shell => {
                coverage.fired += ammo;
                if *target == 10 {
                    coverage.target10_shots.push((event.tick, *damage));
                }
            }
            EconomyEventKind::ResearchCompleted {
                slot, technology, ..
            } if *slot == 0 && technology == research => {
                coverage.research_completed = true;
                coverage.research_tick = Some(event.tick);
            }
            EconomyEventKind::Destroyed { unit: 10, .. } => {
                coverage.target10_destroyed_tick = Some(event.tick);
            }
            _ => {}
        }
    }
}
fn compare_operation_batches(
    left: &[OperationAudit],
    right: &[OperationAudit],
    left_catalog: &Option<turts_sim::economy::views::CatalogAudit>,
    right_catalog: &Option<turts_sim::economy::views::CatalogAudit>,
    label: &str,
    compared_operations: &mut usize,
) {
    assert_eq!(
        left.len(),
        right.len(),
        "operation count differs in {label} at trace index {}",
        *compared_operations
    );
    for (offset, (left_op, right_op)) in left.iter().zip(right).enumerate() {
        assert_eq!(
            left_op.before.catalog.as_ref(),
            left_catalog.as_ref(),
            "left before catalog changed in {label}"
        );
        assert_eq!(
            left_op.after.catalog.as_ref(),
            left_catalog.as_ref(),
            "left after catalog changed in {label}"
        );
        assert_eq!(
            right_op.before.catalog.as_ref(),
            right_catalog.as_ref(),
            "right before catalog changed in {label}"
        );
        assert_eq!(
            right_op.after.catalog.as_ref(),
            right_catalog.as_ref(),
            "right after catalog changed in {label}"
        );
        let mut left_value = serde_json::to_value(left_op).unwrap();
        let mut right_value = serde_json::to_value(right_op).unwrap();
        for value in [&mut left_value, &mut right_value] {
            for snapshot in ["before", "after"] {
                value[snapshot]["catalog"] = serde_json::Value::Null;
            }
        }
        normalize(&mut left_value);
        normalize(&mut right_value);
        let index = *compared_operations + offset;
        if let Some(difference) = first_difference(
            &left_value,
            &right_value,
            &format!("trace[{index}] ({label})"),
        ) {
            panic!("renamed clone operation trace differs: {difference}");
        }
    }
    *compared_operations += left.len();
}

fn normalize(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text) => {
            if !text.contains("clone:x_") {
                return;
            }
            let ids = [
                "ore",
                "shell",
                "ore_vein",
                "drill",
                "belt",
                "arm_loader",
                "press",
                "turret",
                "lab",
                "drone",
                "make_shells",
                "hardened_shells",
                "proving_ground",
            ];
            for id in ids {
                *text = text.replace(&format!("clone:x_{id}"), &format!("forge:{id}"));
            }
        }
        serde_json::Value::Array(rows) => {
            for row in rows {
                normalize(row)
            }
        }
        serde_json::Value::Object(rows) => {
            if rows.get("package") == Some(&serde_json::Value::String("clone".into())) {
                rows.insert("package".into(), serde_json::Value::String("forge".into()));
                if let Some(serde_json::Value::String(local)) = rows.get_mut("local")
                    && let Some(stripped) = local.strip_prefix("x_")
                {
                    *local = stripped.to_owned();
                }
            }
            for row in rows.values_mut() {
                normalize(row)
            }
        }
        _ => {}
    }
}
fn first_difference(a: &serde_json::Value, b: &serde_json::Value, path: &str) -> Option<String> {
    match (a, b) {
        (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
            for (key, left_value) in left {
                let Some(right_value) = right.get(key) else {
                    return Some(format!("{path}.{key}: key presence differs"));
                };
                if let Some(difference) =
                    first_difference(left_value, right_value, &format!("{path}.{key}"))
                {
                    return Some(difference);
                }
            }
            if let Some(key) = right.keys().find(|key| !left.contains_key(*key)) {
                return Some(format!("{path}.{key}: key presence differs"));
            }
            None
        }
        (serde_json::Value::Array(left), serde_json::Value::Array(right)) => {
            if left.len() != right.len() {
                return Some(format!(
                    "{path}: array lengths {} vs {}",
                    left.len(),
                    right.len()
                ));
            }
            left.iter()
                .zip(right)
                .enumerate()
                .find_map(|(i, (x, y))| first_difference(x, y, &format!("{path}[{i}]")))
        }
        _ if a == b => None,
        _ => Some(format!("{path}: {a} != {b}")),
    }
}
#[test]
fn renamed_catalog_lost_ore_fails_balance() {
    let renamed = renamed_clone();
    let (mut app, oracle, _) = loaded(&renamed);
    let before = audit_state(app.world_mut());
    let mut after = before.clone();
    after.nodes.first_mut().unwrap().remaining -= 1;
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
fn renamed_catalog_duplicate_shell_fails_balance() {
    let renamed = renamed_clone();
    let (mut app, oracle, _) = loaded(&renamed);
    let before = audit_state(app.world_mut());
    let shell = package_content("clone", "shell");
    let mut after = before.clone();
    let inventory = after
        .economy
        .inventories
        .iter_mut()
        .find(|row| row.unit == 8)
        .unwrap();
    if let Some(row) = inventory.items.iter_mut().find(|row| row.resource == shell) {
        row.on_hand += 1;
    } else {
        inventory.items.push(turts_sim::economy::views::ItemView {
            resource: shell,
            on_hand: 1,
            reserved: 0,
        });
    }
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
fn forge_renamed_clone_same_commands_and_operation_trace() {
    let original = forge_dir();
    let renamed = renamed_clone();
    let (mut left, left_catalog, mut left_history) = loaded(&original);
    let (mut right, right_catalog, mut right_history) = loaded(&renamed);
    let left_ore = left_catalog.item("forge:ore").unwrap();
    let right_ore = right_catalog.item("clone:x_ore").unwrap();
    let left_shell = left_catalog.item("forge:shell").unwrap();
    let right_shell = right_catalog.item("clone:x_shell").unwrap();
    let left_research = package_content("forge", "hardened_shells");
    let right_research = package_content("clone", "hardened_shells");
    let left_static_catalog = audit_state(left.world_mut()).catalog;
    let right_static_catalog = audit_state(right.world_mut()).catalog;
    let mut left_catalog_value = serde_json::to_value(&left_static_catalog).unwrap();
    let mut right_catalog_value = serde_json::to_value(&right_static_catalog).unwrap();
    normalize(&mut left_catalog_value);
    normalize(&mut right_catalog_value);
    assert_eq!(
        left_catalog_value, right_catalog_value,
        "normalized source catalogs differ"
    );
    let mut left_coverage = FlowCoverage::default();
    let mut right_coverage = FlowCoverage::default();
    let mut compared_operations = 0;
    let mut produce_accepted = false;
    let mut opponent_move_accepted = false;
    let mut opponent_unit_moved = false;
    let mut opponent_unit_initial_pos = None;

    for tick in 1..=5000 {
        left.update();
        right.update();
        let left_tick = drain_checked(&mut left, &left_catalog, &mut left_history);
        let right_tick = drain_checked(&mut right, &right_catalog, &mut right_history);
        record_flow(
            &left_tick,
            &left_ore,
            &left_shell,
            &left_research,
            &mut left_coverage,
        );
        record_flow(
            &right_tick,
            &right_ore,
            &right_shell,
            &right_research,
            &mut right_coverage,
        );
        compare_operation_batches(
            &left_tick,
            &right_tick,
            &left_static_catalog,
            &right_static_catalog,
            &format!("tick {tick}"),
            &mut compared_operations,
        );
        for state in [
            audit_state(left.world_mut()),
            audit_state(right.world_mut()),
        ] {
            if let Some(unit) = state.raw_units.iter().find(|unit| unit.unit_id == 10) {
                let initial = *opponent_unit_initial_pos.get_or_insert((unit.pos.x, unit.pos.y));
                opponent_unit_moved |=
                    (unit.pos.x - initial.0).abs() > 0.01 || (unit.pos.y - initial.1).abs() > 0.01;
            }
        }

        if !produce_accepted {
            let left_state = audit_state(left.world_mut());
            let right_state = audit_state(right.world_mut());
            let left_lab_shells = inventory_amount(&left_state, 8, &left_shell);
            let right_lab_shells = inventory_amount(&right_state, 8, &right_shell);
            assert_eq!(
                left_lab_shells, right_lab_shells,
                "lab stocks at tick {tick}"
            );
            if left_lab_shells >= 10 {
                // Both commands express the same owner action, with package-local recipe refs.
                let left_command = issue(
                    &mut left,
                    &left_catalog,
                    &mut left_history,
                    0,
                    1,
                    CommandKind::Produce {
                        producer_id: 8,
                        recipe_id: left_research.clone(),
                    },
                );
                let right_command = issue(
                    &mut right,
                    &right_catalog,
                    &mut right_history,
                    0,
                    1,
                    CommandKind::Produce {
                        producer_id: 8,
                        recipe_id: right_research.clone(),
                    },
                );
                compare_operation_batches(
                    &left_command,
                    &right_command,
                    &left_static_catalog,
                    &right_static_catalog,
                    &format!("owner Produce at tick {tick}"),
                    &mut compared_operations,
                );
                produce_accepted = true;
            }
        }

        if left_coverage.research_completed && !opponent_move_accepted {
            assert!(right_coverage.research_completed);
            // Slot 1 moves its own drone only after slot 0 completes research.
            let left_command = issue(
                &mut left,
                &left_catalog,
                &mut left_history,
                1,
                1,
                CommandKind::MoveTo {
                    unit_ids: vec![10],
                    target: Vec2::new(24.0, 14.0),
                },
            );
            let right_command = issue(
                &mut right,
                &right_catalog,
                &mut right_history,
                1,
                1,
                CommandKind::MoveTo {
                    unit_ids: vec![10],
                    target: Vec2::new(24.0, 14.0),
                },
            );
            compare_operation_batches(
                &left_command,
                &right_command,
                &left_static_catalog,
                &right_static_catalog,
                &format!("opponent MoveTo after research at tick {tick}"),
                &mut compared_operations,
            );
            opponent_move_accepted = true;
        }
        if left_coverage.target10_destroyed_tick.is_some()
            && right_coverage.target10_destroyed_tick.is_some()
        {
            break;
        }
    }

    assert!(produce_accepted, "owner Produce command was not issued");
    assert!(
        opponent_move_accepted,
        "opponent MoveTo did not follow research completion"
    );
    assert!(
        left_coverage.extracted > 0,
        "E2 must extract source-RON ore"
    );
    assert!(
        left_coverage.produced > 0,
        "E2 must produce source-RON shells"
    );
    assert!(left_coverage.fired > 0, "E2 must fire with source-RON ammo");
    assert!(
        left_coverage.research_completed,
        "E2 must complete source-RON research"
    );
    assert!(
        opponent_unit_moved,
        "post-research MoveTo must move drone B"
    );
    let completion_tick = left_coverage.research_tick.unwrap();
    let postresearch_shots = left_coverage
        .target10_shots
        .iter()
        .filter(|(tick, _)| *tick >= completion_tick)
        .collect::<Vec<_>>();
    assert_eq!(
        postresearch_shots.len(),
        5,
        "drone B must take five source-RON upgraded shots on/after the completion tick: {left_coverage:?}"
    );
    assert!(
        postresearch_shots.iter().all(|(_, damage)| *damage == 20),
        "post-research drone B shots must use the source-RON damage modifier (20)"
    );
    assert!(
        left_coverage
            .target10_destroyed_tick
            .is_some_and(|tick| tick >= completion_tick),
        "post-research drone B must be destroyed by the upgraded shots"
    );
    assert_eq!(
        left_coverage, right_coverage,
        "both package flows must cover the same events"
    );
    assert!(
        compared_operations > 100,
        "the trace contains the actual industrial flow"
    );
}
