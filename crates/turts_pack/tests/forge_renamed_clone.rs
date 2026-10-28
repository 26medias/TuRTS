mod common;
use common::{
    conservation::{OracleCatalog, OracleHistory, check_operation, check_tick},
    forge_support::{copy_tree, forge_dir},
};
use std::{fs, path::PathBuf};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_pack::load_package;
use turts_sim::{
    AuditTrail, Command, CommandKind, OperationAudit, SlotId, apply_command, audit_state,
    headless_app,
};

fn renamed_clone() -> PathBuf {
    let target =
        std::env::temp_dir().join(format!("turts-t14-renamed-clone-{}", std::process::id()));
    let _ = fs::remove_dir_all(&target);
    copy_tree(&forge_dir(), &target);
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
        for id in ids {
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
    seq: u32,
) -> Vec<OperationAudit> {
    let result = apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq,
            kind: CommandKind::SetEnabled {
                unit_id: 1,
                enabled: seq == 2,
            },
        },
    );
    assert!(result.outcome.is_ok());
    drain_checked(app, oracle, history)
}
fn normalize(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::String(text) => {
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
            let mut mapped = text.clone();
            for id in ids {
                mapped = mapped.replace(&format!("clone:x_{id}"), &format!("forge:{id}"));
            }
            *text = mapped.replace("\"clone\"", "\"forge\"");
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
            for key in left.keys().chain(right.keys()) {
                if !left.contains_key(key) || !right.contains_key(key) {
                    return Some(format!("{path}.{key}: key presence differs"));
                }
                if let Some(d) = first_difference(&left[key], &right[key], &format!("{path}.{key}"))
                {
                    return Some(d);
                }
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
fn forge_renamed_clone_same_commands_and_operation_trace() {
    let original = forge_dir();
    let renamed = renamed_clone();
    let (mut left, left_catalog, mut left_history) = loaded(&original);
    let (mut right, right_catalog, mut right_history) = loaded(&renamed);
    let mut left_ops = Vec::new();
    let mut right_ops = Vec::new();
    left_ops.extend(issue(&mut left, &left_catalog, &mut left_history, 1));
    right_ops.extend(issue(&mut right, &right_catalog, &mut right_history, 1));
    for _ in 0..250 {
        left.update();
        right.update();
        left_ops.extend(drain_checked(&mut left, &left_catalog, &mut left_history));
        right_ops.extend(drain_checked(
            &mut right,
            &right_catalog,
            &mut right_history,
        ));
    }
    left_ops.extend(issue(&mut left, &left_catalog, &mut left_history, 2));
    right_ops.extend(issue(&mut right, &right_catalog, &mut right_history, 2));
    assert!(left_ops.len() > 100, "the trace contains real operations");
    let mut left_value = serde_json::to_value(&left_ops).unwrap();
    let mut right_value = serde_json::to_value(&right_ops).unwrap();
    normalize(&mut left_value);
    normalize(&mut right_value);
    if let Some(difference) = first_difference(&left_value, &right_value, "trace") {
        panic!("renamed clone operation trace differs after local-ID normalization: {difference}");
    }
}
