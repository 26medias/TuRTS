//! Boot tests. `turts_server` is a binary crate, so this file includes `config.rs` and
//! `boot.rs` by path and calls them in process. The HTTP test starts the real binary.

mod support;

#[allow(dead_code)]
#[path = "../src/config.rs"]
mod config;

#[path = "../src/boot.rs"]
mod boot;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use bevy::app::App;
use config::HostConfig;
use turts_mod_api::{ENGINE_API_VERSION, Lockfile};
use turts_mod_runtime::hash::sha256_hex;
use turts_mod_runtime::lock::read_lockfile;
use turts_mod_runtime::pack::pack_files;
use turts_net_protocol::{Advertisement, NET_PROTOCOL_VERSION};
use turts_sim::{UnitTypes, Vec2, gameplay_entity_count, headless_app, snapshot};

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
    let cases = [
        ("base", "base:rifleman", Vec2::new(8.0, 8.0)),
        ("acme", "acme:scout", Vec2::new(4.0, 4.0)),
    ];
    for (package, content_id, pos) in cases {
        let (mut app, _, _) = boot_package(&format!("boot_registers_{package}"), package);
        assert_eq!(app.world().resource::<UnitTypes>().len(), 1, "{package}");
        assert_eq!(gameplay_entity_count(app.world_mut()), 1, "{package}");
        let units = snapshot(app.world_mut());
        assert_eq!(units[0].content_id, content_id);
        assert_eq!(units[0].pos, pos, "{package}");
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
        entry.urls[0].ends_with("/packages/acme-0.1.0.tar"),
        "{:?}",
        entry.urls
    );
    assert_eq!(
        entry.urls,
        vec![format!("{PUBLIC_URL}/packages/acme-0.1.0.tar")]
    );
    assert_eq!(booted.package_file_name, "acme-0.1.0.tar");
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
    assert_eq!(advertisement.mod_version, "0.1.0");
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
    assert_eq!(error.field_path, "speed");
    assert_eq!(error.package, "acme");
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
