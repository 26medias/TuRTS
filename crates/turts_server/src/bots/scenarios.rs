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
