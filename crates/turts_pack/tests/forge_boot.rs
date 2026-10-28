use std::{
    fs,
    path::{Path, PathBuf},
};

use turts_mod_api::{ContentId, SourceDef};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_pack::load_package;
use turts_sim::{audit_state, headless_app, stage_package};

fn forge_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/forge")
}

fn forge_archive() -> PackageArchive {
    PackageArchive::parse(&pack_dir(&forge_dir()).expect("pack Forge")).expect("parse Forge")
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

fn forge_archive_with_map_edits(tag: &str, edits: &[(&str, &str)]) -> PackageArchive {
    let package = std::env::temp_dir().join(format!("turts-forge-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&package);
    copy_tree(&forge_dir(), &package);
    let map_path = package.join("maps/proving_ground.ron");
    let mut map = fs::read_to_string(&map_path).unwrap();
    for (from, to) in edits {
        assert!(map.contains(from), "fixture text must match: {from}");
        map = map.replace(from, to);
    }
    fs::write(&map_path, map).unwrap();
    let archive = PackageArchive::parse(&pack_dir(&package).expect("pack modified Forge")).unwrap();
    fs::remove_dir_all(package).unwrap();
    archive
}

fn forge_archive_with_map_change(tag: &str, from: &str, to: &str) -> PackageArchive {
    forge_archive_with_map_edits(tag, &[(from, to)])
}

fn forge_archive_with_arm_durations(tag: &str, swing_ms: u64, return_ms: u64) -> PackageArchive {
    let package =
        std::env::temp_dir().join(format!("turts-forge-arm-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&package);
    copy_tree(&forge_dir(), &package);
    let arm_path = package.join("units/arm_loader.ron");
    let arm = fs::read_to_string(&arm_path).unwrap();
    let arm = arm.replace("swing_ms: 250", &format!("swing_ms: {swing_ms}"));
    let arm = arm.replace("return_ms: 250", &format!("return_ms: {return_ms}"));
    fs::write(&arm_path, arm).unwrap();
    let archive =
        PackageArchive::parse(&pack_dir(&package).expect("pack modified Forge arm")).unwrap();
    fs::remove_dir_all(package).unwrap();
    archive
}

#[test]
fn forge_map_boots_with_expected_ids() {
    let mut app = headless_app(20);
    load_package(app.world_mut(), &forge_archive()).expect("load Forge through the public loader");
    let mut state = audit_state(app.world_mut());
    state.units.sort_by_key(|unit| unit.unit_id);

    let expected = [
        (1, "forge:drill", 6.0, 8.0),
        (2, "forge:arm_loader", 8.0, 8.0),
        (3, "forge:arm_loader", 15.0, 8.0),
        (4, "forge:press", 17.0, 8.0),
        (5, "forge:arm_loader", 17.0, 10.0),
        (6, "forge:arm_loader", 17.0, 6.0),
        (7, "forge:turret", 17.0, 12.0),
        (8, "forge:lab", 17.0, 4.0),
        (9, "forge:drone", 24.0, 12.0),
        (10, "forge:drone", 30.0, 4.0),
        (11, "forge:belt", 9.0, 8.0),
        (12, "forge:belt", 10.0, 8.0),
        (13, "forge:belt", 11.0, 8.0),
        (14, "forge:belt", 12.0, 8.0),
        (15, "forge:belt", 13.0, 8.0),
        (16, "forge:belt", 14.0, 8.0),
    ];
    assert_eq!(state.units.len(), expected.len());
    for (unit, (id, content, x, y)) in state.units.iter().zip(expected) {
        assert_eq!(unit.unit_id, id);
        assert_eq!(unit.content_id, content);
        assert_eq!((unit.pos.x, unit.pos.y), (x, y));
    }
}

#[test]
fn placed_conveyor_requires_explicit_belt_line() {
    let archive = forge_archive_with_map_change(
        "placed-conveyor",
        "placements: [",
        r#"placements: [
        (unit: "forge:belt", owner_slot: 0, pos: (x: 2.0, y: 2.0), contents: []),"#,
    );
    let mut app = headless_app(20);
    let error = load_package(app.world_mut(), &archive)
        .expect_err("a conveyor piece in placements needs an explicit belt line");
    assert_eq!(error.file.as_deref(), Some("maps/proving_ground.ron"));
    assert_eq!(error.field_path, "placements[0].unit");
}

#[test]
fn overlapping_footprint_rejects() {
    let archive = forge_archive_with_map_change(
        "overlap",
        "pos: (x: 30.0, y: 4.0)",
        "pos: (x: 17.0, y: 4.0)",
    );
    let mut app = headless_app(20);
    let error = load_package(app.world_mut(), &archive).expect_err("overlapping footprints reject");
    assert_eq!(error.file.as_deref(), Some("maps/proving_ground.ron"));
    assert_eq!(error.field_path, "placements[9].pos");
}

#[test]
fn ambiguous_endpoint_rejects_with_moderror_on_facing() {
    let archive = forge_archive_with_map_edits(
        "ambiguous",
        &[(
            r#"(conveyor: "forge:belt", owner_slot: 0, start: (x: 9.0, y: 8.0), facing: PosX, pieces: 6)"#,
            r#"(conveyor: "forge:belt", owner_slot: 0, start: (x: 9.0, y: 8.0), facing: PosX, pieces: 1),
        (conveyor: "forge:belt", owner_slot: 0, start: (x: 14.0, y: 7.5), facing: PosY, pieces: 2)"#,
        )],
    );
    let mut app = headless_app(20);
    let error =
        load_package(app.world_mut(), &archive).expect_err("ambiguous tangent endpoints reject");
    assert_eq!(error.file.as_deref(), Some("maps/proving_ground.ron"));
    assert_eq!(error.field_path, "placements[2].facing");
    assert!(error.message.contains("matches multiple units"));
}

#[test]
fn inserter_wrong_owner_rejects_with_moderror_on_facing() {
    let archive = forge_archive_with_map_change(
        "wrong-owner",
        r#"unit: "forge:arm_loader", owner_slot: 0, pos: (x: 17.0, y: 10.0)"#,
        r#"unit: "forge:arm_loader", owner_slot: 1, pos: (x: 17.0, y: 10.0)"#,
    );
    let mut app = headless_app(20);
    let error = load_package(app.world_mut(), &archive).expect_err("wrong-owner endpoint rejects");
    assert_eq!(error.file.as_deref(), Some("maps/proving_ground.ron"));
    assert_eq!(error.field_path, "placements[4].facing");
    assert!(error.message.contains("owned by slot 0"));
}

#[test]
fn drill_without_node_rejects() {
    let archive = forge_archive_with_map_change(
        "no-node",
        r#"nodes: [
        (node: "forge:ore_vein", pos: (x: 3.5, y: 8.0), remaining: 300),
    ]"#,
        "nodes: []",
    );
    let mut app = headless_app(20);
    let error = load_package(app.world_mut(), &archive).expect_err("unbound extractor rejects");
    assert_eq!(error.file.as_deref(), Some("maps/proving_ground.ron"));
    assert_eq!(error.field_path, "placements[0].unit");
}

#[test]
fn direct_catalog_rejects_zero_inserter_duration_with_unit_source() {
    let archive = forge_archive();
    let mut catalog = archive.content().unwrap();
    let arm_id = ContentId::parse("forge:arm_loader").unwrap();
    catalog
        .units
        .get_mut(&arm_id)
        .unwrap()
        .inserter
        .as_mut()
        .unwrap()
        .swing_ms = 0;
    let (map_file, map) = archive.maps().unwrap().into_iter().next().unwrap();
    let app = headless_app(20);
    let error = stage_package(
        app.world(),
        catalog,
        SourceDef {
            file: map_file,
            value: map,
        },
        20,
    )
    .expect_err("direct catalog boot must reject zero swing duration");
    assert_eq!(error.package, "forge");
    assert_eq!(error.file.as_deref(), Some("units/arm_loader.ron"));
    assert_eq!(error.field_path, "inserter.swing_ms");
}

#[test]
fn inserter_duration_bounds_follow_configured_tick_rate() {
    for hz in [10_u16, 20, 30] {
        let highest_ms = (u64::from(u32::MAX) * 1000) / u64::from(hz);
        let first_over_ms = highest_ms + 1;

        let valid =
            forge_archive_with_arm_durations(&format!("{hz}-valid"), highest_ms, highest_ms);
        let mut app = headless_app(hz);
        load_package(app.world_mut(), &valid).expect("u32::MAX tick duration is representable");

        let over_swing =
            forge_archive_with_arm_durations(&format!("{hz}-swing-over"), first_over_ms, 250);
        let mut app = headless_app(hz);
        let error = load_package(app.world_mut(), &over_swing)
            .expect_err("swing duration above u32 elapsed range must reject");
        assert_eq!(error.package, "forge");
        assert_eq!(error.file.as_deref(), Some("units/arm_loader.ron"));
        assert_eq!(error.field_path, "inserter.swing_ms");

        let over_return =
            forge_archive_with_arm_durations(&format!("{hz}-return-over"), 250, first_over_ms);
        let mut app = headless_app(hz);
        let error = load_package(app.world_mut(), &over_return)
            .expect_err("return duration above u32 elapsed range must reject");
        assert_eq!(error.package, "forge");
        assert_eq!(error.file.as_deref(), Some("units/arm_loader.ron"));
        assert_eq!(error.field_path, "inserter.return_ms");
    }
}

#[test]
fn turts_pack_binary_accepts_forge() {
    let output_archive = forge_dir().with_extension("tar");
    let _ = fs::remove_file(&output_archive);
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_turts-pack"))
        .arg(forge_dir())
        .output()
        .expect("start turts-pack");
    let _ = fs::remove_file(output_archive);
    assert!(
        output.status.success(),
        "turts-pack failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
