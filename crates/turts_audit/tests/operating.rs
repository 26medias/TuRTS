//! Record a real package and fixed ticks. Replay must derive operating effects.
use std::{collections::BTreeMap, path::Path};
use turts_audit::{EconomyLogRecord, validate_records};
use turts_mod_api::{MapDef, MapSize, Placement, SourceDef, WorldPos};
use turts_sim::{
    AuditTrail, EconomyEventKind, audit_state,
    bootstrap::{install_package, stage_package},
    headless_app,
};

fn files(root: &Path, here: &Path, output: &mut BTreeMap<String, Vec<u8>>) {
    for entry in std::fs::read_dir(here).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(root, &path, output);
        } else {
            output.insert(
                path.strip_prefix(root).unwrap().to_str().unwrap().into(),
                std::fs::read(path).unwrap(),
            );
        }
    }
}
fn append_fields(bytes: &mut Vec<u8>, fields: &str) {
    let mut text = String::from_utf8(bytes.clone()).unwrap();
    text.insert_str(text.rfind(')').unwrap(), fields);
    *bytes = text.into_bytes();
}
fn run() -> Vec<EconomyLogRecord> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar");
    let mut source = BTreeMap::new();
    files(&root, &root, &mut source);
    source.insert(
        "resources/power.ron".into(),
        br#"(id:"power",name:"Power",unit_name:"watt",kind:Supply(scope:Slot,counts:Completed))"#
            .to_vec(),
    );
    source.insert(
        "resources/credits.ron".into(),
        br#"(id:"credits",name:"Credits",unit_name:"credit",kind:Balance)"#.to_vec(),
    );
    source.insert(
        "manifest.ron".into(),
        String::from_utf8(source["manifest.ron"].clone())
            .unwrap()
            .replace("^0.2.4", "^0.2.13")
            .replace(
                "resources: [",
                "resources: [\"resources/power.ron\", \"resources/credits.ron\", ",
            )
            .into_bytes(),
    );
    append_fields(
        source.get_mut("units/depot.ron").unwrap(),
        r#"health:Some((max:300)), removal:Some((by_command:true,contents:Destroy)), provides:[(resource:"cedar:power",amount:10)], income:Some((resource:"cedar:credits",amount:3,period_ms:1050)),"#,
    );
    append_fields(
        source.get_mut("units/worker.ron").unwrap(),
        r#"health:Some((max:60)), removal:Some((by_command:true,contents:Destroy)), consumes:[(resource:"cedar:power",amount:2)],"#,
    );
    let archive = turts_mod_runtime::archive::PackageArchive::parse(
        &turts_mod_runtime::pack::pack_files(&source).unwrap(),
    )
    .unwrap();
    let placement = |unit: &str, x| Placement {
        unit: unit.into(),
        owner_slot: 0,
        pos: WorldPos { x, y: 16. },
        contents: vec![],
        facing: None,
        start_order: None,
    };
    let map = MapDef {
        environment: None,
        id: "income".into(),
        size: MapSize { x: 48., y: 32. },
        placements: vec![
            placement("cedar:depot", 8.),
            placement("cedar:depot", 24.),
            placement("cedar:worker", 16.),
        ],
        nodes: vec![],
        slot_economy: vec![],
        belts: vec![],
        ground: None,
    };
    let mut app = headless_app(20);
    let staged = stage_package(
        app.world(),
        archive.content().unwrap(),
        SourceDef {
            file: "fixture.ron".into(),
            value: map,
        },
        20,
    )
    .unwrap();
    install_package(app.world_mut(), staged);
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let mut records = vec![EconomyLogRecord::Initial(audit_state(app.world_mut()))];
    for _ in 0..45 {
        app.update();
        let mut trail = app.world_mut().resource_mut::<AuditTrail>();
        records.extend(
            trail
                .drain_operations()
                .into_iter()
                .map(EconomyLogRecord::Operation),
        );
        records.extend(
            trail
                .drain_ticks()
                .into_iter()
                .map(EconomyLogRecord::from_tick),
        );
    }
    records
}
#[test]
fn operating_income_and_supply_replay_real_recorded_stream() {
    let records = run();
    let EconomyLogRecord::Initial(initial) = &records[0] else {
        unreachable!()
    };
    let operating = initial.raw_operating.as_ref().unwrap();
    assert!(
        operating
            .live_supply
            .iter()
            .any(|row| row.slot == 0 && row.provided == 20 && row.consumed == 2)
    );
    assert_eq!(initial.raw_income_clocks.len(), 2);
    let credits: Vec<_> = records
        .iter()
        .filter_map(|r| {
            if let EconomyLogRecord::Operation(op) = r {
                Some(op)
            } else {
                None
            }
        })
        .flat_map(|op| op.events.iter())
        .filter_map(|event| {
            if let EconomyEventKind::IncomeCredited { credited, .. } = event.kind {
                Some((event.tick, credited))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(credits, vec![(21, 3), (21, 3), (42, 3), (42, 3)]);
    validate_records(&records).unwrap();
    assert_eq!(
        serde_json::to_value(&records).unwrap(),
        serde_json::to_value(run()).unwrap()
    );
    if let Ok(path) = std::env::var("TURTS_OPERATING_AUDIT_OUT") {
        let text = records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap() + "\n")
            .collect::<String>();
        std::fs::write(path, text).unwrap();
    }
}
#[test]
fn replay_rejects_catalog_inconsistent_initial_income_clock_and_supply() {
    let records = run();
    let mut tampered = records.clone();
    let EconomyLogRecord::Initial(initial) = &mut tampered[0] else {
        unreachable!()
    };
    initial.raw_income_clocks[0].amount += 1;
    let error = validate_records(&tampered).unwrap_err();
    assert!(error.contains("income"), "{error}");
    let mut tampered = records.clone();
    let EconomyLogRecord::Initial(initial) = &mut tampered[0] else {
        unreachable!()
    };
    initial.raw_operating.as_mut().unwrap().active[0].supply[0].1 += 1;
    let error = validate_records(&tampered).unwrap_err();
    assert!(error.contains("operating"), "{error}");
}
#[test]
fn replay_rejects_income_credit_tampered_with_consistent_record_shape() {
    let mut records = run();
    // End on an operation so later continuity cannot be the reason for rejection.
    let index=records.iter().position(|r| matches!(r,EconomyLogRecord::Operation(op) if op.events.iter().any(|e|matches!(e.kind,EconomyEventKind::IncomeCredited{..})))).unwrap();
    records.truncate(index + 1);
    let EconomyLogRecord::Operation(op) = records.last_mut().unwrap() else {
        unreachable!()
    };
    if let EconomyEventKind::IncomeCredited { credited, .. } = &mut op.events[0].kind {
        *credited += 1;
    }
    let ordinal = op.events[0].ordinal;
    *op.after
        .pending_events
        .iter_mut()
        .find(|event| event.ordinal == ordinal)
        .unwrap() = op.events[0].clone();
    let error = validate_records(&records).unwrap_err();
    assert!(error.contains("income"), "{error}");
}
