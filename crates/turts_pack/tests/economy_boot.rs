mod common;

use common::{AuditLookup, archive_from_files, economy_archive, economy_files, scenario};
use turts_pack::load_package;
use turts_sim::{IdAllocators, Slots, UnitTypes, audit_state, world_view};

#[test]
fn failed_late_definition_and_bad_map_leave_world_and_allocators_unchanged() {
    let mut bad_definition = economy_files();
    let worker = String::from_utf8(bad_definition["units/worker.ron"].clone()).unwrap();
    assert!(worker.contains("speed: 3.0"));
    bad_definition.insert(
        "units/worker.ron".into(),
        worker.replace("speed: 3.0", "speed: 0.0").into_bytes(),
    );
    let mut app = scenario(20);
    let before = audit_state(app.world_mut());
    let error = load_package(app.world_mut(), &archive_from_files(bad_definition)).unwrap_err();
    assert_eq!(error.file.as_deref(), Some("units/worker.ron"));
    assert_eq!(error.field_path, "movement.speed");
    assert_eq!(audit_state(app.world_mut()), before);

    let mut bad_map = economy_files();
    let map = String::from_utf8(bad_map["maps/test.ron"].clone()).unwrap();
    assert!(map.contains("probe:worker"));
    bad_map.insert(
        "maps/test.ron".into(),
        map.replacen("probe:worker", "probe:missing", 1)
            .into_bytes(),
    );
    let mut app = scenario(20);
    let before = audit_state(app.world_mut());
    let error = load_package(app.world_mut(), &archive_from_files(bad_map)).unwrap_err();
    assert_eq!(error.file.as_deref(), Some("maps/test.ron"));
    assert_eq!(error.field_path, "placements[2].unit");
    assert_eq!(audit_state(app.world_mut()), before);
}

#[test]
fn boot_seeds_every_slot_once_and_claim_is_neutral() {
    let mut app = scenario(20);
    let archive = economy_archive();
    load_package(app.world_mut(), &archive).unwrap();
    let before_claim = audit_state(app.world_mut());
    assert_eq!(before_claim.balance(0, "probe:token").on_hand, 200);
    assert_eq!(before_claim.balance(1, "probe:token").on_hand, 0);
    app.world_mut().resource_mut::<Slots>().claim(17).unwrap();
    let after_claim = audit_state(app.world_mut());
    assert_eq!(before_claim.economy, after_claim.economy);
    assert_eq!(world_view(app.world_mut()).balances.len(), 2);
}

#[test]
fn static_and_mobile_capabilities_are_explicit() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let types = app.world().resource::<UnitTypes>();
    assert!(
        types
            .get(types.find("probe:yard").unwrap())
            .unwrap()
            .movement
            .is_none()
    );
    assert!(
        types
            .get(types.find("probe:worker").unwrap())
            .unwrap()
            .movement
            .is_some()
    );
    let view = world_view(app.world_mut());
    assert_eq!(view.inventories.len(), 6);
    assert_eq!(view.orders.len(), 2);
    assert_eq!(view.queues.len(), 2);
}

#[test]
fn placement_overlap_bounds_and_id_exhaustion_are_atomic() {
    for (needle, replacement) in [
        ("x: 12.0, y: 8.0", "x: 9.0, y: 8.0"),
        ("size: (x: 64.0", "size: (x: 5.0"),
    ] {
        let mut files = economy_files();
        let map = String::from_utf8(files["maps/test.ron"].clone()).unwrap();
        assert!(map.contains(needle));
        files.insert(
            "maps/test.ron".into(),
            map.replacen(needle, replacement, 1).into_bytes(),
        );
        let mut app = scenario(20);
        let before = audit_state(app.world_mut());
        assert!(load_package(app.world_mut(), &archive_from_files(files)).is_err());
        assert_eq!(audit_state(app.world_mut()), before);
    }
    let mut app = scenario(20);
    let before = audit_state(app.world_mut());
    let mut allocators = *app.world().resource::<IdAllocators>();
    allocators.set_next_values(u32::MAX, u32::MAX, u64::MAX);
    app.world_mut().insert_resource(allocators);
    let before_exhaustion = audit_state(app.world_mut());
    assert!(load_package(app.world_mut(), &economy_archive()).is_err());
    assert_eq!(audit_state(app.world_mut()), before_exhaustion);
    assert_ne!(before, before_exhaustion);
}

#[test]
fn server_boot_ignores_visual_bytes_while_client_validation_rejects_them() {
    let mut files = economy_files();
    files.insert("visuals/yard.ron".into(), b"not valid ron".to_vec());
    let archive = archive_from_files(files);
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive).expect("server does not read visuals");
    assert!(
        turts_mod_runtime::visuals::visuals_by_content(std::slice::from_ref(&archive)).is_err()
    );
    let error = turts_pack::check_package(&archive).unwrap_err();
    assert_eq!(error.file.as_deref(), Some("visuals/yard.ron"));
}

#[test]
fn package_without_balance_resources_creates_no_accounts() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
    let mut files = std::collections::BTreeMap::new();
    for entry in std::fs::read_dir(&root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            files.insert(
                path.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read(path).unwrap(),
            );
        } else {
            for child in std::fs::read_dir(&path).unwrap() {
                let child = child.unwrap().path();
                let key = format!(
                    "{}/{}",
                    path.file_name().unwrap().to_string_lossy(),
                    child.file_name().unwrap().to_string_lossy()
                );
                files.insert(key, std::fs::read(child).unwrap());
            }
        }
    }
    let archive = archive_from_files(files);
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive).unwrap();
    assert!(world_view(app.world_mut()).balances.is_empty());
}

#[test]
fn unit_and_node_visual_targets_keep_their_kinds() {
    let mut files = economy_files();
    let visual = String::from_utf8(files["visuals/yard.ron"].clone()).unwrap();
    assert!(visual.contains("Unit(\"probe:yard\")"));
    files.insert(
        "visuals/yard.ron".into(),
        visual
            .replace("Unit(\"probe:yard\")", "Node(\"probe:yard\")")
            .into_bytes(),
    );
    let archive = archive_from_files(files);
    let error =
        turts_mod_runtime::visuals::visuals_by_content(std::slice::from_ref(&archive)).unwrap_err();
    assert_eq!(error.file.as_deref(), Some("visuals/yard.ron"));
    assert_eq!(error.field_path, "target");
    assert!(error.message.contains("declares a node"));
}

#[test]
fn recipe_duration_ticks_use_actual_tick_rate_and_checked_ceil() {
    let mut files = economy_files();
    let train = String::from_utf8(files["production/train.ron"].clone()).unwrap();
    assert!(train.contains("duration_ms: 5000"));
    files.insert(
        "production/train.ron".into(),
        train
            .replace("duration_ms: 5000", "duration_ms: 5001")
            .into_bytes(),
    );
    let mut app = scenario(30);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    let durations = &app
        .world()
        .resource::<turts_sim::bootstrap::ProductionDurations>()
        .0;
    assert_eq!(durations[&common::cid("probe:hire")], 180);
    assert_eq!(durations[&common::cid("probe:train")], 151);
}

#[test]
fn claiming_is_economy_neutral_even_for_a_slot_without_entities() {
    let mut files = economy_files();
    let text = String::from_utf8(files["maps/test.ron"].clone()).unwrap();
    let start = text.find("placements: [").expect("placements field") + "placements: [".len();
    let end = text[start..].find("], nodes:").expect("end of placements") + start;
    assert!(!text[start..end].trim().is_empty());
    let mut map = text;
    map.replace_range(start..end, "");
    files.insert("maps/test.ron".into(), map.into_bytes());
    let archive = archive_from_files(files);
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive).unwrap();
    assert!(audit_state(app.world_mut()).units.is_empty());
    let before = audit_state(app.world_mut());
    app.world_mut().resource_mut::<Slots>().claim(99).unwrap();
    let after = audit_state(app.world_mut());
    assert_eq!(before.economy, after.economy);
    assert_eq!(before.claims, vec![None, None]);
    assert_eq!(after.claims, vec![Some(99), None]);
}
