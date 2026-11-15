#![cfg(test)]
//! Native scenarios use the same package profiles and factory registry as server startup.
use super::*;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_sim::{SlotId, Slots};

fn archive_lockfile_hash(archive: &PackageArchive) -> String {
    let package_file = format!("{}-{}.tar", archive.manifest.id, archive.manifest.version);
    let bytes = turts_mod_runtime::lock::write_lockfile(&turts_mod_api::Lockfile {
        packages: vec![
            archive.lock_entry(vec![format!("http://bot.test/packages/{package_file}")]),
        ],
    });
    turts_mod_runtime::hash::sha256_hex(&bytes)
}

fn package_app() -> App {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/stockyard");
    let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
    let mut app = turts_sim::headless_app(20);
    turts_pack::load_package(app.world_mut(), &archive).unwrap();
    app.insert_resource(ServerCatalog(archive.content().unwrap()));
    app.insert_resource(session::Advertised {
        lockfile_hash: archive_lockfile_hash(&archive),
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
    {
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
    }
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
            lockfile_hash: archive_lockfile_hash(archive),
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

fn allocated_profile(archive: &PackageArchive, map: turts_mod_api::MapDef) -> App {
    allocated_catalog(
        archive.content().unwrap(),
        map,
        archive.match_defs().unwrap()[0]
            .1
            .open_sites_per_slot
            .unwrap(),
    )
}
fn allocated_catalog(
    catalog: turts_mod_api::ContentCatalog,
    map: turts_mod_api::MapDef,
    construction_limit: u16,
) -> App {
    let mut app = turts_sim::headless_app(20);
    let staged = turts_sim::stage_package(
        app.world(),
        catalog.clone(),
        turts_mod_api::SourceDef {
            file: "maps/allocator-inversion.ron".into(),
            value: map,
        },
        20,
    )
    .unwrap();
    turts_sim::install_package(app.world_mut(), staged);
    app.insert_resource(turts_sim::economy::construction::ConstructionLimit(
        construction_limit,
    ));
    app.insert_resource(ServerCatalog(catalog));
    app.init_resource::<crate::stats::Stats>();
    app.insert_resource(crate::stats::MatchId(1));
    session::install(
        &mut app,
        session::Advertised {
            lockfile_hash: turts_mod_runtime::hash::sha256_hex(b"public-bot-allocator-fixture"),
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
    install(&mut app, &[config(1)]).unwrap();
    turts_sim::initialize_completed_world(app.world_mut());
    app
}
fn physical_unit(view: &turts_sim::WorldView, id: u32) -> serde_json::Value {
    let unit = view.units.iter().find(|unit| unit.unit_id == id).unwrap();
    serde_json::json!({"content":unit.content_id,"owner":unit.owner_slot,"x_bits":unit.pos.x.to_bits(),"y_bits":unit.pos.y.to_bits()})
}
fn physical_decisions(
    view: &turts_sim::WorldView,
    commands: &[EmittedCommand],
) -> Vec<serde_json::Value> {
    commands.iter().map(|command|{
        let target=match &command.order.target {
            BotTarget::Unit(id)=>serde_json::json!({"unit":physical_unit(view,*id)}),
            BotTarget::Node(id)=>{let node=view.nodes.iter().find(|node|node.id.0==*id).unwrap();serde_json::json!({"node":node.content_id,"x_bits":node.pos.x.to_bits(),"y_bits":node.pos.y.to_bits()})},
            BotTarget::Point(point)=>serde_json::json!({"point":[point.x.to_bits(),point.y.to_bits()]}),
            BotTarget::ContentAtPoint{content,point}=>serde_json::json!({"content":content,"point":[point.x.to_bits(),point.y.to_bits()]}),
            BotTarget::Content(content)=>serde_json::json!({"content":content}),
            BotTarget::None=>serde_json::Value::Null,
            BotTarget::Job(id)=>serde_json::json!({"job":id}),
        };
        // Keep emitted command and actor order. Only opaque subject identifiers are normalized.
        serde_json::json!({"sequence":command.sequence,"controller":command.task.controller,"goal":command.task.goal,"order":command.order.content,"queue":command.order.queue,"group":command.order.group,"actors":command.units.iter().map(|id|physical_unit(view,*id)).collect::<Vec<_>>(),"target":target})
    }).collect()
}
#[test]
fn real_allocator_inversion_preserves_public_profile_physical_choices() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/stockyard");
    let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
    let map = archive.maps().unwrap().remove(0).1;
    let mut bindings = Vec::new();
    let mut choices = Vec::new();
    for reversed in [false, true] {
        let mut map = map.clone();
        if reversed {
            map.placements.reverse();
        }
        let mut app = allocated_profile(&archive, map);
        let before = turts_sim::completed_world(app.world()).clone();
        bindings.push(
            before
                .units
                .iter()
                .map(|unit| {
                    (
                        (
                            unit.content_id.clone(),
                            unit.owner_slot,
                            unit.pos.x.to_bits(),
                            unit.pos.y.to_bits(),
                        ),
                        unit.unit_id,
                    )
                })
                .collect::<std::collections::BTreeMap<_, _>>(),
        );
        for _ in 0..6 {
            app.update();
            if !app.world().resource::<BotExecution>().generated.is_empty() {
                break;
            }
        }
        let commands = &app.world().resource::<BotExecution>().generated;
        assert!(
            !commands.is_empty(),
            "public profile must emit a real first decision"
        );
        choices.push(physical_decisions(&before, commands));
    }
    assert_eq!(
        bindings[0].keys().collect::<Vec<_>>(),
        bindings[1].keys().collect::<Vec<_>>()
    );
    assert!(
        bindings[0]
            .iter()
            .any(|(physical, id)| bindings[1][physical] != *id),
        "map reversal must change actual allocated unit identifiers"
    );
    assert_eq!(
        choices[0], choices[1],
        "actual ID allocation cannot change ordered physical/content choices"
    );
    // The authoritative map boundary refuses coincident footprints. The framework's
    // hypothetical exact-rank tie remains covered by the controller tie regression.
    let mut overlapping = map;
    let source = overlapping
        .placements
        .iter()
        .find(|unit| unit.owner_slot == 1 && unit.unit == "stockyard:barracks")
        .unwrap()
        .clone();
    overlapping.placements.push(source);
    let app = turts_sim::headless_app(20);
    let error = turts_sim::stage_package(
        app.world(),
        archive.content().unwrap(),
        turts_mod_api::SourceDef {
            file: "maps/coincident-source.ron".into(),
            value: overlapping,
        },
        20,
    )
    .expect_err("coincident sources must be refused before allocation");
    assert!(error.to_string().contains("overlap"), "{error}");
}

#[test]
fn ordinary_enemy_weapon_losses_restore_worker_and_supply_without_overlapping_paid_work() {
    use turts_net_protocol::{OrderEnvelope, OrderTarget, PlayerCommand, QueueMode, UnitId};
    use turts_sim::{AuditTrail, economy::events::EconomyEventKind};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/stockyard");
    let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
    let mut catalog = archive.content().unwrap();
    let mut raider = catalog.units[&ContentId::parse("stockyard:soldier").unwrap()].clone();
    raider.id = ContentId::parse("stockyard:raider").unwrap();
    raider.health.as_mut().unwrap().max = 1_000_000;
    raider.weapon.as_mut().unwrap().damage = 10_000;
    raider.weapon.as_mut().unwrap().range = 128.0;
    raider.acquire = None;
    for order in catalog
        .orders
        .values_mut()
        .filter(|order| order.units.contains(&"stockyard:soldier".to_owned()))
    {
        order.units.push(raider.id.to_string());
    }
    let mut hold = catalog.orders[&ContentId::parse("stockyard:order_stop").unwrap()].clone();
    hold.id = "raider_hold".into();
    hold.behaviour = turts_mod_api::Behaviour::Hold;
    hold.fire = Some(turts_mod_api::FirePolicy::Never);
    hold.engage = Some(turts_mod_api::EngagePolicy::None);
    hold.units = vec![raider.id.to_string()];
    catalog
        .orders
        .insert(ContentId::parse("stockyard:raider_hold").unwrap(), hold);
    catalog.units.insert(raider.id.clone(), raider);
    let mut map = archive.maps().unwrap().remove(0).1;
    map.placements
        .iter_mut()
        .find(|unit| unit.owner_slot == 0 && unit.unit == "stockyard:soldier")
        .unwrap()
        .unit = "stockyard:raider".into();
    let mut extra = map
        .placements
        .iter()
        .find(|unit| unit.owner_slot == 1 && unit.unit == "stockyard:tower")
        .unwrap()
        .clone();
    extra.pos = turts_mod_api::WorldPos { x: 58., y: 36. };
    map.placements.push(extra);
    let mut app = allocated_catalog(
        catalog,
        map,
        archive.match_defs().unwrap()[0]
            .1
            .open_sites_per_slot
            .unwrap(),
    );
    let initial = turts_sim::completed_world(app.world()).clone();
    let worker = initial
        .units
        .iter()
        .find(|unit| unit.owner_slot == 1 && unit.content_id == "stockyard:worker")
        .unwrap()
        .unit_id;
    let mut victims = vec![worker];
    victims.extend(
        initial
            .units
            .iter()
            .filter(|unit| unit.owner_slot == 1 && unit.content_id == "stockyard:tower")
            .map(|unit| unit.unit_id),
    );
    assert_eq!(victims.len(), 3);
    let attacker = initial
        .units
        .iter()
        .find(|unit| unit.content_id == "stockyard:raider")
        .unwrap()
        .unit_id;
    let connection = app
        .world_mut()
        .query_filtered::<Entity, With<session::Session>>()
        .single(app.world())
        .unwrap();
    let directory = std::env::var_os("TURTS_BOT_EVIDENCE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("turts-public-bot-host-{}", std::process::id()))
        });
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("ordinary-worker-provider-loss.jsonl");
    app.world_mut().resource_mut::<AuditTrail>().enable();
    let state = turts_sim::audit_state(app.world_mut());
    let sink = crate::economylog::EconomyLogSink::create(
        &path,
        app.world().resource::<AuditTrail>(),
        state,
    )
    .unwrap();
    app.insert_resource(sink);
    app.add_systems(
        FixedLast,
        crate::economylog::capture_completed_tick.after(session::ParticipantTraceComplete),
    );
    let attack = |app: &mut App, index: usize| {
        app.world_mut()
            .resource_mut::<session::StagedPlayerCommands>()
            .0
            .push_back(session::StagedPlayerCommand {
                connection,
                slot: SlotId(0),
                envelope: PlayerCommand {
                    seq: index as u32 + 1,
                    order: OrderEnvelope {
                        order: "stockyard:order_attack".into(),
                        units: vec![UnitId(attacker)],
                        target: OrderTarget::Unit(UnitId(victims[index])),
                        queue: QueueMode::Replace,
                        group: 0,
                    },
                },
            })
    };
    attack(&mut app, 0);
    let mut destroyed = 0;
    let mut hits = std::collections::BTreeSet::new();
    let mut recovery = BTreeMap::<String, usize>::new();
    let mut worker_absent = false;
    let mut supply_below_target = false;
    let mut restored = false;
    let mut loss_ticks = Vec::new();
    for _ in 0..2400 {
        let before = turts_sim::completed_world(app.world());
        let active_before = app.world().resource::<BotHost>().bots[0]
            .coordinator
            .pending()
            .into_iter()
            .filter(|pending| {
                pending.work.as_ref().is_some_and(|work| match work.kind {
                    turts_bot::WorkKind::Production => before.economy.queues.iter().any(|queue| {
                        queue.producer == work.unit
                            && queue.jobs.iter().any(|job| job.id == work.id)
                    }),
                    turts_bot::WorkKind::Construction => before
                        .construction
                        .sites
                        .get(&work.unit)
                        .is_some_and(|site| site.job.0 == work.id),
                    _ => false,
                })
            })
            .map(|pending| (pending.task, pending.request.unit))
            .collect::<std::collections::BTreeSet<_>>();
        app.update();
        let view = turts_sim::completed_world(app.world());
        for event in &turts_sim::committed_tick(app.world()).events {
            if let EconomyEventKind::Fired { weapon, target, .. } = event.kind {
                if weapon == attacker {
                    assert!(
                        victims.contains(&target),
                        "controlled ordinary attack hit unintended unit {target}"
                    );
                    hits.insert(target);
                }
            }
        }
        for command in &app.world().resource::<BotExecution>().generated {
            if ["restore_beds", "recover_worker"].contains(&command.task.goal.as_str()) {
                *recovery.entry(command.task.goal.clone()).or_default() += 1;
                for unit in &command.units {
                    assert!(
                        !active_before.contains(&(command.task.clone(), *unit)),
                        "a paid recovery cannot be resubmitted while its exact prior COT work remains active"
                    );
                }
            }
        }
        let owned_workers = view
            .units
            .iter()
            .filter(|unit| unit.owner_slot == 1 && unit.content_id == "stockyard:worker")
            .count();
        worker_absent |= owned_workers == 0;
        let spare = view
            .operating
            .as_ref()
            .and_then(|operating| {
                operating
                    .supply
                    .iter()
                    .find(|row| row.slot == 1 && row.resource.to_string() == "stockyard:beds")
            })
            .map_or(0, |row| row.provided.saturating_sub(row.consumed));
        supply_below_target |= spare < 4;
        let pending = app.world().resource::<BotHost>().bots[0]
            .coordinator
            .pending();
        let mut task_members = std::collections::BTreeSet::new();
        let mut works = std::collections::BTreeSet::new();
        for commitment in pending {
            assert!(
                task_members.insert((commitment.task.clone(), commitment.request.unit)),
                "duplicate task allocation for one actor"
            );
            if let Some(work) = commitment.work {
                assert!(
                    works.insert(work),
                    "the same authoritative COT work cannot be counted twice"
                );
            }
        }
        if destroyed < victims.len()
            && !view
                .units
                .iter()
                .any(|unit| unit.unit_id == victims[destroyed])
        {
            loss_ticks.push(view.tick);
            destroyed += 1;
            if destroyed < victims.len() {
                attack(&mut app, destroyed);
            } else {
                app.world_mut()
                    .resource_mut::<session::StagedPlayerCommands>()
                    .0
                    .push_back(session::StagedPlayerCommand {
                        connection,
                        slot: SlotId(0),
                        envelope: PlayerCommand {
                            seq: 4,
                            order: OrderEnvelope {
                                order: "stockyard:raider_hold".into(),
                                units: vec![UnitId(attacker)],
                                target: OrderTarget::None,
                                queue: QueueMode::Replace,
                                group: 0,
                            },
                        },
                    });
            }
        }
        let view = turts_sim::completed_world(app.world());
        if destroyed == 3
            && worker_absent
            && supply_below_target
            && owned_workers >= 1
            && spare >= 4
            && recovery
                .get("recover_worker")
                .is_some_and(|count| *count > 0)
            && recovery.get("restore_beds").is_some_and(|count| *count > 0)
        {
            assert!(view.units.iter().any(|unit| unit.owner_slot == 1
                && unit.content_id == "stockyard:worker"
                && unit.unit_id != worker));
            assert!(view.units.iter().any(|unit| unit.owner_slot == 1
                && unit.content_id == "stockyard:tower"
                && !victims.contains(&unit.unit_id)));
            assert!(
                owned_workers < 6,
                "optional opening worker expansion is still unfinished when recovery succeeds"
            );
            restored = true;
            break;
        }
    }
    assert!(
        restored,
        "ordinary losses did not trigger both recoveries: destroyed{destroyed} worker_absent{worker_absent} supply_short{supply_below_target} recovery{recovery:?}"
    );
    assert_eq!(
        hits.len(),
        3,
        "every lost unit must have a real ordinary enemy shot witness"
    );
    assert!(
        app.world()
            .resource::<crate::economylog::EconomyLogSink>()
            .error
            .is_none()
    );
    drop(app);
    let records = turts_audit::read_jsonl_file(&path).unwrap();
    turts_audit::validate_records(&records).unwrap();
    println!(
        "BF4_RECOVERY {}",
        serde_json::json!({"evidence":path,"ordinary_enemy_loss_ticks":loss_ticks,"recovery_submissions":recovery,"inflight_paid_duplicates":0,"optional_opening_still_pending":true})
    );
}
