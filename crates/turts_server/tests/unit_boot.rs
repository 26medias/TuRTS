//! Boot tests. `turts_server` is a binary crate, so this file includes `config.rs` and
//! `boot.rs` by path and calls them in process. The HTTP test starts the real binary.

mod support;

#[allow(dead_code)]
#[path = "../src/config.rs"]
mod config;

#[path = "../src/boot.rs"]
mod boot;

#[path = "../src/export.rs"]
mod export;

#[allow(dead_code)]
#[path = "../src/synthetic.rs"]
mod synthetic;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::app::App;
use config::HostConfig;
use turts_mod_api::{ENGINE_API_VERSION, Lockfile};
use turts_mod_runtime::hash::sha256_hex;
use turts_mod_runtime::lock::read_lockfile;
use turts_mod_runtime::pack::pack_files;
use turts_net_protocol::{Advertisement, NET_PROTOCOL_VERSION, messages};
use turts_sim::{
    Command, CommandKind, MapBounds, MoveTarget, SlotId, UnitId, UnitTypes, Vec2, committed_tick,
    completed_world, gameplay_entity_count, headless_app, initialize_completed_world, snapshot,
    world_view,
};

const PUBLIC_URL: &str = "http://127.0.0.1:7000";

fn config(package: &Path) -> HostConfig {
    HostConfig {
        game_package: package.display().to_string(),
        http_listen: "127.0.0.1:0".to_string(),
        ws_listen: "127.0.0.1:0".to_string(),
        public_url: PUBLIC_URL.to_string(),
        tick_hz: 20,
        static_dir: None,
    }
}

/// Pack `mods_dir_name`, boot a headless app with the archive, and return all three.
fn boot_package(test: &str, mods_dir_name: &str) -> (App, boot::Booted, Vec<u8>) {
    let dir = support::test_dir(test);
    let tar = support::pack(mods_dir_name, &dir);
    let bytes = fs::read(&tar).unwrap();
    let mut app = headless_app(20);
    let booted = boot::boot(app.world_mut(), &config(&tar), &bytes).unwrap();
    (app, booted, bytes)
}

/// Read every file under `dir` as `relative path -> bytes`.
fn read_tree(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            read_tree(root, &path, files);
        } else {
            let name = path.strip_prefix(root).unwrap().to_str().unwrap();
            files.insert(name.to_string(), fs::read(&path).unwrap());
        }
    }
}

#[test]
fn boot_registers_units_and_places_map_from_any_package() {
    type ExpectedUnit = (u32, &'static str, u8, Vec2);
    type PackageCase<'a> = (&'static str, &'a [ExpectedUnit]);
    let cases: [PackageCase<'_>; 2] = [
        (
            "base",
            &[
                (1, "base:command_center", 0, Vec2::new(8.0, 8.0)),
                (2, "base:harvester", 0, Vec2::new(12.0, 8.0)),
                (3, "base:command_center", 1, Vec2::new(56.0, 8.0)),
                (4, "base:harvester", 1, Vec2::new(52.0, 8.0)),
            ],
        ),
        ("acme", &[(1, "acme:scout", 0, Vec2::new(4.0, 4.0))]),
    ];
    for (package, expected) in cases {
        let (mut app, _, _) = boot_package(&format!("boot_registers_{package}"), package);
        let expected_type_count = if package == "base" { 3 } else { 1 };
        assert_eq!(
            app.world().resource::<UnitTypes>().len(),
            expected_type_count,
            "{package}"
        );
        assert_eq!(
            gameplay_entity_count(app.world_mut()),
            expected.len(),
            "{package}"
        );
        let units = snapshot(app.world_mut());
        assert_eq!(units.len(), expected.len(), "{package}");
        for (unit, (id, content_id, owner, pos)) in units.iter().zip(expected) {
            assert_eq!(unit.unit_id, *id, "{package}");
            assert_eq!(unit.content_id, *content_id, "{package} unit {id}");
            assert_eq!(unit.owner_slot, *owner, "{package} unit {id}");
            assert_eq!(unit.pos, *pos, "{package} unit {id}");
        }
    }
}

#[test]
fn boot_lockfile_entry_matches_archive() {
    let (_, booted, bytes) = boot_package("boot_lockfile_entry", "acme");
    let lock: Lockfile = read_lockfile(&booted.lockfile).unwrap();
    assert_eq!(lock.packages.len(), 1);
    let entry = &lock.packages[0];
    assert_eq!(entry.hash, booted.archive.hash);
    assert_eq!(entry.hash, sha256_hex(&bytes));
    assert_eq!(entry.size, bytes.len() as u64);
    assert!(
        entry.urls[0].ends_with("/packages/acme-0.2.0.tar"),
        "{:?}",
        entry.urls
    );
    assert_eq!(
        entry.urls,
        vec![format!("{PUBLIC_URL}/packages/acme-0.2.0.tar")]
    );
    assert_eq!(booted.package_file_name, "acme-0.2.0.tar");
    assert_eq!(booted.package_bytes, bytes);
}

#[test]
fn boot_advertisement_hash_matches_lockfile_bytes() {
    let (_, booted, _) = boot_package("boot_advertisement_hash", "acme");
    let advertisement: Advertisement = serde_json::from_slice(&booted.advertisement).unwrap();
    assert_eq!(sha256_hex(&booted.lockfile), advertisement.lockfile_hash);
    assert_eq!(booted.lockfile_hash, advertisement.lockfile_hash);
    assert_eq!(
        advertisement.lockfile_url,
        format!("{PUBLIC_URL}/lockfile.json")
    );
    assert_eq!(advertisement.mod_id, "acme");
    assert_eq!(advertisement.mod_version, "0.2.0");
    assert_eq!(advertisement.engine_version, ENGINE_API_VERSION);
    assert_eq!(advertisement.protocol_version, NET_PROTOCOL_VERSION);
}

#[test]
fn boot_fills_file_on_registration_error() {
    let root = support::mods_dir().join("acme");
    let mut files = BTreeMap::new();
    read_tree(&root, &root, &mut files);
    let unit = files.get_mut("units/scout.ron").unwrap();
    let text = String::from_utf8(unit.clone()).unwrap();
    assert!(text.contains("speed: 6.0"));
    *unit = text.replace("speed: 6.0", "speed: 0.0").into_bytes();
    let bytes = pack_files(&files).unwrap();
    let mut app = headless_app(20);
    let error = boot::boot(app.world_mut(), &config(Path::new("unused.tar")), &bytes).unwrap_err();
    assert_eq!(error.file.as_deref(), Some("units/scout.ron"));
    assert_eq!(error.field_path, "movement.speed");
    assert_eq!(error.package, "acme");
}

/// The options of a run with `count` synthetic units and nothing else.
fn synthetic_cli(count: u32) -> config::Cli {
    config::Cli {
        config: PathBuf::from("server.ron"),
        tick_log: None,
        stats: None,
        economy_log: None,
        synthetic_units: count,
        run_ticks: None,
    }
}

/// The `MoveTarget` of every unit that has one, by unit id.
fn move_targets(app: &mut App) -> BTreeMap<u32, Vec2> {
    let mut query = app.world_mut().query::<(&UnitId, &MoveTarget)>();
    query
        .iter(app.world())
        .map(|(id, target)| (id.0, target.0))
        .collect()
}

#[test]
fn synthetic_units_use_the_registered_type() {
    let (mut app, _, _) = boot_package("synthetic_registered_type", "acme");
    synthetic::spawn(app.world_mut(), 50).unwrap();
    let synthetic_view = world_view(app.world_mut());
    assert!(synthetic_view.economy.inventories.is_empty());
    assert!(synthetic_view.economy.orders.is_empty());
    assert!(synthetic_view.economy.queues.is_empty());
    assert_eq!(gameplay_entity_count(app.world_mut()), 51);
    let size = app.world().resource::<MapBounds>().size;
    let units = snapshot(app.world_mut());
    assert_eq!(units.len(), 51);
    for unit in &units {
        assert_eq!(unit.content_id, "acme:scout", "unit {}", unit.unit_id);
        assert_eq!(unit.owner_slot, 0, "unit {}", unit.unit_id);
        assert!(
            (0.0..=size.x).contains(&unit.pos.x) && (0.0..=size.y).contains(&unit.pos.y),
            "unit {} at {:?} is outside the map {size:?}",
            unit.unit_id,
            unit.pos
        );
    }
    let ids: BTreeSet<u32> = units.iter().map(|unit| unit.unit_id).collect();
    assert_eq!(ids.len(), 51, "unit ids continue after the map's unit");
    // The map's own unit keeps its place and gets no target; each synthetic unit has one.
    assert_eq!(units[0].pos, Vec2::new(4.0, 4.0));
    let targets = move_targets(&mut app);
    assert_eq!(targets.len(), 50);
    assert!(!targets.contains_key(&units[0].unit_id));
}

#[test]
fn synthetic_base_harvesters_have_empty_capabilities_in_the_initial_completed_view() {
    let (mut app, _, _) = boot_package("synthetic_base_capabilities", "base");
    let original_ids: BTreeSet<u32> = snapshot(app.world_mut())
        .into_iter()
        .map(|unit| unit.unit_id)
        .collect();
    synthetic::spawn(app.world_mut(), 2).unwrap();

    let view = world_view(app.world_mut());
    let synthetic_ids: Vec<u32> = view
        .units
        .iter()
        .filter(|unit| !original_ids.contains(&unit.unit_id))
        .map(|unit| unit.unit_id)
        .collect();
    assert_eq!(synthetic_ids.len(), 2);
    for id in &synthetic_ids {
        assert!(
            view.economy.inventories.iter().any(|inventory| {
                inventory.unit == *id && inventory.owner_slot == 0 && inventory.items.is_empty()
            }),
            "synthetic harvester {id} has an explicit empty inventory"
        );
        assert!(
            view.economy.orders.iter().any(|order| {
                order.unit == *id
                    && order.owner_slot == 0
                    && order.stage == turts_sim::GatherStage::Idle
            }),
            "synthetic harvester {id} has an explicit Idle order"
        );
        assert!(
            view.economy
                .queues
                .iter()
                .all(|queue| queue.producer != *id)
        );
    }

    assert_eq!(app.world().resource::<turts_sim::Tick>().0, 0);
    assert_eq!(committed_tick(app.world()).tick, 0);
    assert!(committed_tick(app.world()).spawned.is_empty());
    initialize_completed_world(app.world_mut());
    let completed = completed_world(app.world());
    for id in synthetic_ids {
        assert!(
            completed
                .economy
                .inventories
                .iter()
                .any(|inventory| inventory.unit == id)
        );
        assert!(
            completed
                .economy
                .orders
                .iter()
                .any(|order| { order.unit == id && order.owner_slot == SlotId(0).0 })
        );
    }
}

#[test]
fn completed_output_is_in_same_delta_and_late_join_snapshot_while_cancel_spawns_nothing() {
    let (mut app, _, _) = boot_package("export_actual_output", "base");
    initialize_completed_world(app.world_mut());
    let started = turts_sim::apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::Produce {
                producer_id: 1,
                recipe_id: turts_mod_api::ContentId::parse("base:train_rifleman").unwrap(),
            },
        },
    );
    let job_id = started.outcome.unwrap().job_id.unwrap();
    for _ in 0..101 {
        app.update();
    }

    let batch = committed_tick(app.world());
    let completed = completed_world(app.world());
    let delta = export::delta_for(batch, SlotId(0));
    let late_join = export::snapshot_for(completed, SlotId(0));
    assert_eq!(delta.tick, late_join.tick);
    assert_eq!(delta.tick, 101);
    assert_eq!(delta.spawned.len(), 1);
    let output = &delta.spawned[0];
    assert_eq!(output.unit_id, messages::UnitId(5));
    assert_eq!(output.content_id, "base:rifleman");
    assert_eq!(output.owner_slot, messages::SlotId(0));
    assert_eq!(
        late_join
            .units
            .iter()
            .find(|unit| unit.unit_id == output.unit_id),
        Some(output)
    );
    assert!(batch.events.iter().any(|event| matches!(
        event.kind,
        turts_sim::EconomyEventKind::ProductionCompleted {
            producer: 1,
            job,
            output_unit_id: 5,
        } if job == job_id.0
    )));
    let mut encoded = [0u8; 4096];
    let encoded = postcard::to_slice(&delta, &mut encoded).unwrap();
    let decoded: messages::WorldDelta = postcard::from_bytes(encoded).unwrap();
    assert_eq!(decoded.spawned[0], *output);

    let cancelled = turts_sim::apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 2,
            kind: CommandKind::Produce {
                producer_id: 1,
                recipe_id: turts_mod_api::ContentId::parse("base:train_rifleman").unwrap(),
            },
        },
    );
    let queued_job = cancelled.outcome.unwrap().job_id.unwrap();
    let result = turts_sim::apply_command(
        app.world_mut(),
        &Command {
            slot: SlotId(0),
            seq: 3,
            kind: CommandKind::CancelProduction {
                producer_id: 1,
                job_id: queued_job,
            },
        },
    );
    assert!(result.outcome.is_ok());
    app.update();
    assert!(committed_tick(app.world()).spawned.is_empty());
    assert!(
        export::delta_for(committed_tick(app.world()), SlotId(0))
            .spawned
            .is_empty()
    );
}

#[test]
fn synthetic_units_get_a_new_target_every_40_ticks() {
    let (mut app, _, _) = boot_package("synthetic_retarget", "acme");
    synthetic::install(&mut app, &synthetic_cli(50));
    assert_eq!(gameplay_entity_count(app.world_mut()), 51);
    let size = app.world().resource::<MapBounds>().size;
    // The model: a unit takes the point of `target_point(tick - 1, id)` in tick `tick` when
    // `(tick - 1 + id) % 40 == 0`, and keeps its target otherwise. A unit that arrives loses its
    // target, so a unit may lack one; a unit never has a target that the model does not give.
    let mut expected = move_targets(&mut app);
    assert_eq!(
        expected.len(),
        50,
        "each synthetic unit starts with a target"
    );
    let before = snapshot(app.world_mut());
    let mut retargets: BTreeMap<u32, u32> = BTreeMap::new();
    for tick in 1..=82u32 {
        app.update();
        for (&id, target) in expected.iter_mut() {
            if (u64::from(tick - 1) + u64::from(id)) % 40 == 0 {
                *target = synthetic::target_point(tick - 1, id, size);
                *retargets.entry(id).or_default() += 1;
            }
        }
        let actual = move_targets(&mut app);
        for (id, target) in &actual {
            assert_eq!(*target, expected[id], "tick {tick}: unit {id}");
        }
        for id in expected.keys() {
            if (u64::from(tick - 1) + u64::from(*id)) % 40 == 0 {
                assert!(
                    actual.contains_key(id),
                    "tick {tick}: unit {id} has no new target"
                );
            }
        }
    }
    assert_eq!(retargets.len(), 50, "every unit was retargeted");
    assert!(retargets.values().all(|&times| times >= 2), "{retargets:?}");
    let after = snapshot(app.world_mut());
    assert_eq!(
        after[0].pos, before[0].pos,
        "the map's own unit stands still"
    );
    assert!(!move_targets(&mut app).contains_key(&after[0].unit_id));
    let moved = before
        .iter()
        .zip(&after)
        .filter(|(from, to)| from.pos != to.pos)
        .count();
    assert_eq!(moved, 50, "every synthetic unit moved");
}

#[test]
fn synthetic_spawn_refuses_a_world_without_unit_types() {
    let mut app = headless_app(20);
    let error = synthetic::spawn(app.world_mut(), 5).unwrap_err();
    assert!(error.contains("unit type"), "{error}");
    assert_eq!(gameplay_entity_count(app.world_mut()), 0);
}

#[test]
fn synthetic_targets_depend_on_tick_and_unit_and_stay_inside_the_map() {
    let size = Vec2::new(48.0, 32.0);
    let mut seen = BTreeSet::new();
    for tick in 0..200 {
        for unit_id in 1..200 {
            let point = synthetic::target_point(tick, unit_id, size);
            assert!(
                (0.0..=size.x).contains(&point.x) && (0.0..=size.y).contains(&point.y),
                "tick {tick} unit {unit_id}: {point:?}"
            );
            assert_eq!(point, synthetic::target_point(tick, unit_id, size));
            seen.insert((point.x.to_bits(), point.y.to_bits()));
        }
    }
    assert_eq!(
        seen.len(),
        200 * 199,
        "every (tick, unit) pair has its own point"
    );
}

#[test]
#[ignore]
fn http_endpoints_serve_advertisement_lockfile_and_package() {
    let dir = support::test_dir("http_endpoints");
    let tar = support::pack("acme", &dir);
    let package_bytes = fs::read(&tar).unwrap();
    let server = support::spawn_server(&tar, &[]);
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(5)))
        .http_status_as_error(false)
        .build()
        .into();
    let get = |url: &str| {
        let mut response = agent.get(url).call().unwrap();
        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get("content-type")
            .map(|value| value.to_str().unwrap().to_string());
        let cors = response
            .headers()
            .contains_key("access-control-allow-origin");
        (
            status,
            content_type,
            cors,
            response.body_mut().read_to_vec().unwrap(),
        )
    };

    let (status, content_type, cors, body) =
        get(&format!("{}/advertisement.json", server.http_url));
    assert_eq!(status, 200);
    assert_eq!(content_type.as_deref(), Some("application/json"));
    assert!(cors, "the advertisement carries a CORS header");
    let advertisement: Advertisement = serde_json::from_slice(&body).unwrap();

    let (status, content_type, _, lockfile_bytes) = get(&advertisement.lockfile_url);
    assert_eq!(status, 200);
    assert_eq!(content_type.as_deref(), Some("application/json"));
    assert_eq!(sha256_hex(&lockfile_bytes), advertisement.lockfile_hash);

    let lock = read_lockfile(&lockfile_bytes).unwrap();
    let entry = &lock.packages[0];
    let (status, content_type, _, served) = get(&entry.urls[0]);
    assert_eq!(status, 200);
    assert_eq!(content_type.as_deref(), Some("application/octet-stream"));
    assert_eq!(sha256_hex(&served), entry.hash);
    assert_eq!(served, package_bytes);

    let (status, ..) = get(&format!("{}/packages/other-0.1.0.tar", server.http_url));
    assert_eq!(status, 404);
}
