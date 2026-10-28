use super::conservation::{OracleCatalog, OracleHistory, check_operation, check_tick};
use bevy_app::App;
use std::{
    fs,
    path::{Path, PathBuf},
};
use turts_mod_api::ContentId;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_pack::load_package;
use turts_sim::{
    AuditTrail, Command, CommandKind, CommandResult, OperationAudit, SlotId, apply_command,
    audit_state, headless_app,
};

pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
pub fn forge_dir() -> PathBuf {
    root().join("mods/forge")
}
pub fn package_fixture(name: &str) -> PathBuf {
    root().join("tests/m3/packages").join(name)
}
pub fn cid(text: &str) -> ContentId {
    ContentId::parse(text).unwrap()
}
pub fn copy_tree(source: &Path, destination: &Path) {
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
pub fn app_for(path: &Path) -> App {
    let bytes = pack_dir(path).unwrap();
    let archive = PackageArchive::parse(&bytes).unwrap();
    let mut app = headless_app(20);
    load_package(app.world_mut(), &archive).unwrap();
    app.world_mut().resource_mut::<AuditTrail>().enable();
    app
}
pub fn loaded(path: &Path) -> (App, OracleCatalog, OracleHistory) {
    let oracle = OracleCatalog::parse(path).unwrap();
    let mut app = app_for(path);
    let initial = audit_state(app.world_mut());
    let history = OracleHistory::seed(&initial);
    (app, oracle, history)
}
pub fn audit_ops(
    app: &mut App,
    oracle: &OracleCatalog,
    history: &mut OracleHistory,
) -> Vec<OperationAudit> {
    let ops = app
        .world_mut()
        .resource_mut::<AuditTrail>()
        .drain_operations();
    for op in &ops {
        check_operation(op, oracle)
            .unwrap_or_else(|e| panic!("op {} {}: {}", op.operation_id, e.invariant, e.detail));
    }
    check_tick(&ops, history, oracle)
        .unwrap_or_else(|e| panic!("tick {}: {}", e.invariant, e.detail));
    ops
}
pub fn step(
    app: &mut App,
    oracle: &OracleCatalog,
    history: &mut OracleHistory,
) -> Vec<OperationAudit> {
    app.update();
    audit_ops(app, oracle, history)
}
pub fn command(
    app: &mut App,
    oracle: &OracleCatalog,
    history: &mut OracleHistory,
    slot: u8,
    seq: u32,
    kind: CommandKind,
) -> CommandResult {
    let cmd = Command {
        slot: SlotId(slot),
        seq,
        kind,
    };
    let result = apply_command(app.world_mut(), &cmd);
    audit_ops(app, oracle, history);
    result
}
pub fn inventory_amount(state: &turts_sim::AuditState, unit: u32, item: &str) -> u64 {
    let item = cid(item);
    state
        .economy
        .inventories
        .iter()
        .find(|row| row.unit == unit)
        .and_then(|row| row.items.iter().find(|it| it.resource == item))
        .map_or(0, |it| it.on_hand)
}
pub fn output_amount(state: &turts_sim::AuditState, unit: u32, item: &str) -> u64 {
    let item = cid(item);
    state
        .raw_output_inventories
        .iter()
        .find(|row| row.unit == unit)
        .and_then(|row| row.items.iter().find(|it| it.0 == item))
        .map_or(0, |it| it.1.on_hand)
}
