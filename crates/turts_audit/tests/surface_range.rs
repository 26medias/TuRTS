//! Replay actual short-range combat, rather than a hand-written after-state.
use std::{collections::BTreeMap, path::Path};
use turts_audit::{EconomyLogRecord, validate_records};
use turts_mod_api::{ContentId, MapDef, MapSize, Placement, SourceDef, WorldPos};
use turts_sim::{
    AuditTrail, Command, CommandKind, SlotId, audit_state,
    bootstrap::{install_package, stage_package},
    headless_app,
    orders::{OrderCommand, OrderTargetSim, QueueMode},
};

fn files(root: &Path, here: &Path, output: &mut BTreeMap<String, Vec<u8>>) {
    for entry in std::fs::read_dir(here).unwrap() {
        let p = entry.unwrap().path();
        if p.is_dir() {
            files(root, &p, output)
        } else {
            output.insert(
                p.strip_prefix(root).unwrap().to_str().unwrap().into(),
                std::fs::read(p).unwrap(),
            );
        }
    }
}
fn run(attack_move: bool) -> Vec<EconomyLogRecord> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
    let mut source = BTreeMap::new();
    files(&root, &root, &mut source);
    let archive = turts_mod_runtime::archive::PackageArchive::parse(
        &turts_mod_runtime::pack::pack_files(&source).unwrap(),
    )
    .unwrap();
    let mut catalog = archive.content().unwrap();
    if attack_move {
        let order = catalog
            .orders
            .get_mut(&ContentId::parse("skirmish:order_attack").unwrap())
            .unwrap();
        order.target = turts_mod_api::TargetKind::PointOrUnit;
        order.engage = Some(turts_mod_api::EngagePolicy::StopAndFire);
    }
    let placement = |unit: &str, slot, x, y| Placement {
        unit: unit.into(),
        owner_slot: slot,
        pos: WorldPos { x, y },
        contents: vec![],
        facing: None,
        start_order: None,
    };
    let map = MapDef {
        id: "field".into(),
        size: MapSize { x: 48.0, y: 32.0 },
        placements: vec![
            placement("skirmish:swordsman", 0, 6.0, 16.0),
            placement("skirmish:soldier", 0, 6.0, 20.0),
            placement("skirmish:base", 1, 24.0, 16.0),
        ],
        nodes: vec![],
        slot_economy: vec![],
        belts: vec![],
        ground: None,
    };
    let mut app = headless_app(20);
    let staged = stage_package(
        app.world(),
        catalog,
        SourceDef {
            file: "fixture.ron".into(),
            value: map,
        },
        20,
    )
    .unwrap();
    install_package(app.world_mut(), staged);
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let initial = audit_state(app.world_mut());
    let definition = initial
        .catalog
        .as_ref()
        .unwrap()
        .order_definitions
        .iter()
        .find(|o| o.id.to_string() == "skirmish:order_attack")
        .unwrap();
    let policy = turts_sim::OrderPolicyAudit {
        id: definition.id.clone(),
        behaviour: definition.behaviour,
        target: definition.target,
        fire: definition.fire,
        engage: definition.engage,
        units: definition.units.clone(),
        queue_depth: definition.queue_depth,
        unit_capabilities: definition.unit_capabilities.clone(),
    };
    let allowed_units = definition.units.clone();
    let mut records = vec![EconomyLogRecord::Initial(initial)];
    let command = Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::Order(Box::new(OrderCommand {
            order: ContentId::parse("skirmish:order_attack").unwrap(),
            behaviour: turts_mod_api::Behaviour::Attack,
            target_kind: if attack_move {
                turts_mod_api::TargetKind::PointOrUnit
            } else {
                turts_mod_api::TargetKind::Unit
            },
            policy: Some(policy),
            allowed_units,
            units: vec![1, 2],
            target: if attack_move {
                OrderTargetSim::Point(bevy_math::Vec2::new(24.0, 16.0))
            } else {
                OrderTargetSim::Unit(3)
            },
            queue: QueueMode::Replace,
            queue_depth: 16,
            group: 0,
        })),
    };
    turts_sim::apply_command(app.world_mut(), &command)
        .outcome
        .unwrap();
    {
        let mut trail = app.world_mut().resource_mut::<AuditTrail>();
        records.extend(
            trail
                .drain_commands()
                .into_iter()
                .map(EconomyLogRecord::CommandBoundary),
        );
        records.extend(
            trail
                .drain_operations()
                .into_iter()
                .map(EconomyLogRecord::Operation),
        );
    }
    for _ in 0..120 {
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
    assert!(records.iter().any(|r|matches!(r,EconomyLogRecord::Operation(o) if o.events.iter().any(|e|matches!(e.kind,turts_sim::EconomyEventKind::Fired{..})))),"weapon never fired");
    let shooters: std::collections::BTreeSet<_> = records
        .iter()
        .flat_map(|r| match r {
            EconomyLogRecord::Operation(o) => o
                .events
                .iter()
                .filter_map(|e| match e.kind {
                    turts_sim::EconomyEventKind::Fired { weapon, .. } => Some(weapon),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            _ => vec![],
        })
        .collect();
    assert_eq!(
        shooters,
        std::collections::BTreeSet::from([1, 2]),
        "both members of the mixed command must fire"
    );
    records
}
#[test]
fn surface_attack_and_attack_move_replay_and_repeat_exactly() {
    for attack_move in [false, true] {
        let first = run(attack_move);
        assert_eq!(first, run(attack_move), "determinism");
        let result = validate_records(&first);
        assert!(result.is_ok(), "attack_move={attack_move}: {result:?}");
    }
}
