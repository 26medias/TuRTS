//! Native scenarios use the same package profiles and factory registry as server startup.
use super::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::{SlotId, Slots};

fn package_app() -> App {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/stockyard");
    let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
    let mut app = turts_sim::headless_app(20);
    turts_pack::load_package(app.world_mut(), &archive).unwrap();
    app.insert_resource(ServerCatalog(archive.content().unwrap()));
    app.insert_resource(session::Advertised {
        lockfile_hash: "0".repeat(64),
        tick_hz: 20,
    });
    turts_sim::initialize_completed_world(app.world_mut());
    app
}
fn config(slot: u8) -> BotConfig {
    BotConfig {
        slot,
        profile: "stockyard:opponent".into(),
        seed: 42,
    }
}

#[test]
fn unavailable_factory_and_invalid_install_batch_leave_claims_unchanged() {
    let mut app = package_app();
    let before = app.world().resource::<Slots>().owners();
    let error =
        install_with_registry(&mut app, &[config(1)], &ControllerRegistry::new()).unwrap_err();
    assert!(error.contains("controllers[0].factory"), "{error}");
    assert_eq!(app.world().resource::<Slots>().owners(), before);
    assert!(!app.world().contains_resource::<BotHost>());
    assert!(
        install(&mut app, &[config(1), config(1)])
            .unwrap_err()
            .contains("duplicate")
    );
    assert_eq!(app.world().resource::<Slots>().owners(), before);
    app.world_mut()
        .resource_mut::<Slots>()
        .claim_at(SlotId(0), 7)
        .unwrap();
    let before = app.world().resource::<Slots>().owners();
    assert!(install(&mut app, &[config(1), config(0)]).is_err());
    assert_eq!(
        app.world().resource::<Slots>().owners(),
        before,
        "a later invalid slot must not claim the earlier valid slot"
    );
}
#[test]
fn two_profiles_create_independent_instances_and_recorded_mode_cannot_generate() {
    let mut app = package_app();
    install(&mut app, &[config(0), config(1)]).unwrap();
    let host = app.world().resource::<BotHost>();
    assert_eq!(host.actors().len(), 2);
    assert_ne!(
        host.bots[0].controllers[0].seed,
        host.bots[1].controllers[0].seed
    );
    assert_eq!(host.bots[0].clock.active, 0);
    app.world_mut().resource_mut::<BotExecution>().mode = BotExecutionMode::RecordedCommands;
    drive(app.world_mut());
    assert_eq!(app.world().resource::<BotHost>().decisions, 0);
    assert!(app.world().resource::<BotExecution>().generated.is_empty());
    assert!(
        app.world()
            .resource::<BotHost>()
            .bots
            .iter()
            .all(|bot| bot.coordinator.pending().is_empty())
    );
}
#[test]
fn regenerated_presence_pauses_and_retains_pending_state_without_reading_live_sessions() {
    let mut app = package_app();
    app.world_mut()
        .resource_mut::<ServerCatalog>()
        .0
        .bots
        .get_mut(&ContentId::parse("stockyard:opponent").unwrap())
        .unwrap()
        .start_when = BotStartWhen::HumanPresent;
    install(&mut app, &[config(1)]).unwrap();
    let mut execution = app.world_mut().resource_mut::<BotExecution>();
    execution.mode = BotExecutionMode::RegenerateAndCompare;
    for (tick, present, ended) in [
        (0, true, false),
        (1, false, false),
        (2, false, false),
        (3, true, false),
        (4, true, true),
    ] {
        execution.inputs.insert(
            tick,
            LifecycleInput {
                scheduler_tick: tick,
                human_present: present,
                running: true,
                ended,
                defeated: vec![],
            },
        );
    }
    drop(execution);
    for (tick, active) in [(0, 1), (1, 1), (2, 1), (3, 2), (4, 2)] {
        app.world_mut().resource_mut::<Tick>().0 = tick;
        drive(app.world_mut());
        assert_eq!(
            app.world().resource::<BotHost>().bots[0].clock.active,
            active
        );
        assert_eq!(
            app.world().resource::<BotExecution>().input_trace.len(),
            1,
            "only the current lifecycle input is buffered"
        );
    }
    assert!(app.world().resource::<BotExecution>().generated.is_empty());
}

pub(super) fn live_package(package: &str) -> App {
    live_package_with_guard(package, None)
}
pub(super) fn live_package_with_guard(package: &str, guard: Option<BotStartWhen>) -> App {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mods")
        .join(package);
    let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
    live_archive(&archive, &format!("{package}:opponent"), guard)
}
fn live_archive(archive: &PackageArchive, profile: &str, guard: Option<BotStartWhen>) -> App {
    let mut app = turts_sim::headless_app(20);
    turts_pack::load_package(app.world_mut(), archive).unwrap();
    app.insert_resource(ServerCatalog(archive.content().unwrap()));
    if let Some(guard) = guard {
        app.world_mut()
            .resource_mut::<ServerCatalog>()
            .0
            .bots
            .get_mut(&ContentId::parse(profile).unwrap())
            .unwrap()
            .start_when = guard;
    }
    app.init_resource::<crate::stats::Stats>();
    app.insert_resource(crate::stats::MatchId(1));
    session::install(
        &mut app,
        session::Advertised {
            lockfile_hash: "0".repeat(64),
            tick_hz: 20,
        },
    );
    app.world_mut()
        .resource_mut::<Slots>()
        .claim_at(SlotId(0), 7)
        .unwrap();
    app.world_mut().spawn(session::Session {
        last_seq: None,
        slot: SlotId(0),
        player_id: 7,
    });
    install(
        &mut app,
        &[BotConfig {
            slot: 1,
            profile: profile.to_owned(),
            seed: 42,
        }],
    )
    .unwrap();
    turts_sim::initialize_completed_world(app.world_mut());
    app
}
#[test]
fn public_balance_and_inventory_profiles_build_gather_produce_and_attack() {
    use turts_sim::economy::events::EconomyEventKind;
    for package in ["skirmish", "stockyard", "different_game"] {
        let mut app = if package == "different_game" {
            live_archive(&renamed_archive(), "different_game:rival", None)
        } else {
            live_package(package)
        };
        let mut built = false;
        let mut gathered = false;
        let mut produced = false;
        let mut attacked = false;
        let mut commands = 0;
        for _ in 0..4100 {
            app.update();
            let view = turts_sim::completed_world(app.world());
            built |= view
                .construction
                .sites
                .values()
                .any(|site| site.owner_slot == 1);
            for event in &turts_sim::committed_tick(app.world()).events {
                if event.owner_slot != 1 {
                    continue;
                }
                match event.kind {
                    EconomyEventKind::Extracted { .. } => gathered = true,
                    EconomyEventKind::ProductionCompleted { .. } => produced = true,
                    EconomyEventKind::Fired { .. } => attacked = true,
                    _ => {}
                }
            }
            commands += app.world().resource::<BotExecution>().generated.len();
            if built && gathered && produced && attacked {
                break;
            }
            if view
                .match_outcomes
                .as_ref()
                .is_some_and(|outcomes| outcomes.end_tick.is_some())
            {
                break;
            }
        }
        let view = turts_sim::completed_world(app.world());
        let host = app.world().resource::<BotHost>();
        assert!(
            built && gathered && produced && attacked,
            "{package}: tick{} built{built} gathered{gathered} produced{produced} attacked{attacked} generated{commands} diagnostic{:?}",
            view.tick,
            host.bots[0].last_diagnostic
        );
        assert!(
            commands > 0,
            "the profile must submit ordinary public orders"
        );
        assert!(app.world().resource::<BotExecution>().error.is_none());
    }
}

fn renamed_archive() -> PackageArchive {
    fn visit(
        root: &std::path::Path,
        path: &std::path::Path,
        files: &mut std::collections::BTreeMap<String, Vec<u8>>,
    ) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files)
            } else {
                let key = path
                    .strip_prefix(root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned();
                let bytes = std::fs::read(&path).unwrap();
                let bytes = if key.ends_with(".ron") {
                    let mut text = String::from_utf8(bytes)
                        .unwrap()
                        .replace("stockyard", "different_game");
                    for (old, new) in [
                        ("base", "citadel"),
                        ("barracks", "hall"),
                        ("builder", "carpenter"),
                        ("worker", "porter"),
                        ("soldier", "archer"),
                        ("swordsman", "guard"),
                        ("warrior", "giant"),
                        ("tower", "beacon"),
                        ("wood", "timber"),
                        ("credits", "coins"),
                        ("beds", "lodging"),
                        ("trees", "grove"),
                        ("train_worker", "recruit_porter"),
                        ("train_soldier", "recruit_archer"),
                        ("train_swordsman", "recruit_guard"),
                        ("train_warrior", "recruit_giant"),
                        ("build_barracks", "raise_hall"),
                        ("build_tower", "raise_beacon"),
                        ("order_gather", "collect"),
                        ("order_attack", "assault"),
                        ("order_move", "travel"),
                        ("order_produce", "recruit"),
                        ("order_construct", "raise"),
                        ("home", "anchor"),
                        ("workers", "porters"),
                        ("builders", "carpenters"),
                        ("trainers", "halls"),
                        ("worker_trainers", "porter_halls"),
                        ("defenses", "beacons"),
                        ("ranged", "archers"),
                        ("melee", "guards"),
                        ("heavy", "giants"),
                        ("army", "troops"),
                        ("opponent", "rival"),
                    ] {
                        text = text
                            .replace(
                                &format!("\"different_game:{old}\""),
                                &format!("\"different_game:{new}\""),
                            )
                            .replace(&format!("\"{old}\""), &format!("\"{new}\""));
                    }
                    text.into_bytes()
                } else {
                    bytes
                };
                files.insert(key, bytes);
            }
        }
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/stockyard");
    let mut files = std::collections::BTreeMap::new();
    visit(&root, &root, &mut files);
    let bytes = turts_mod_runtime::pack::pack_files(&files).unwrap();
    let archive = PackageArchive::parse(&bytes).unwrap();
    let catalog = archive.content().unwrap();
    assert!(
        catalog
            .bots
            .contains_key(&ContentId::parse("different_game:rival").unwrap())
    );
    assert!(!catalog.units.keys().any(|id| {
        [
            "base",
            "barracks",
            "builder",
            "worker",
            "soldier",
            "swordsman",
            "warrior",
            "tower",
        ]
        .contains(&id.local.as_str())
    }));
    archive
}
