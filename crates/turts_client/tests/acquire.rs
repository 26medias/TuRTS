//! Task 8a-1: acquisition checks (advertisement, lockfile, package, visuals).
//! Test code may name the example packages.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use turts_client::acquire::{
    build_acquired, check_advertisement, check_lockfile, packages_to_fetch,
};
use turts_mod_api::{ExecutionContext, LockEntry, Lockfile, Shape};
use turts_mod_runtime::archive::PackageArchive;
use turts_mod_runtime::hash::sha256_hex;
use turts_mod_runtime::lock::write_lockfile;
use turts_mod_runtime::pack::pack_files;
use turts_net_protocol::{Advertisement, NET_PROTOCOL_VERSION};

fn acme_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/acme"))
}

/// Every file under `dir`, as `relative path -> bytes`.
fn read_files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).expect("read dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let relative = path.strip_prefix(root).expect("under root");
                let key = relative.to_str().expect("utf-8 path").replace('\\', "/");
                out.insert(key, fs::read(&path).expect("read file"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// The `acme` package files with one text edit in the manifest. Panics when the edit
/// changes nothing, so a case can never pass because its edit missed.
fn acme_with_manifest_edit(from: &str, to: &str) -> BTreeMap<String, Vec<u8>> {
    let mut files = read_files(&acme_dir());
    let text = String::from_utf8(files["manifest.ron"].clone()).expect("utf-8 manifest");
    assert!(
        text.contains(from),
        "edit target `{from}` is not in manifest.ron"
    );
    files.insert(
        "manifest.ron".to_string(),
        text.replacen(from, to, 1).into_bytes(),
    );
    files
}

fn parse(files: &BTreeMap<String, Vec<u8>>) -> PackageArchive {
    PackageArchive::parse(&pack_files(files).expect("pack in memory")).expect("parse the package")
}

fn lock_entry(id: &str, context: ExecutionContext) -> LockEntry {
    LockEntry {
        id: id.to_string(),
        version: "0.1.0".to_string(),
        hash: sha256_hex(id.as_bytes()),
        size: 1,
        urls: vec![format!("http://h/{id}.tar")],
        context,
        engine_api: "^0.2".to_string(),
    }
}

#[test]
fn check_lockfile_refuses_wrong_hash_before_parse() {
    let bytes = b"not json";
    let own = sha256_hex(bytes);
    let other = sha256_hex(b"other bytes");

    // Wrong hash: the hash check fails and names both hashes. A parse error cannot do that.
    let error = check_lockfile(bytes, &other).unwrap_err();
    assert!(error.contains("hash"), "{error}");
    assert!(error.contains(&own) && error.contains(&other), "{error}");

    // Right hash: the hash check passes and the parse fails.
    let error = check_lockfile(bytes, &own).unwrap_err();
    assert!(error.contains("lockfile"), "{error}");
    assert!(!error.contains(&own), "{error}");

    // A real lockfile with its own hash is read.
    let lock = Lockfile {
        packages: vec![lock_entry("acme", ExecutionContext::Shared)],
    };
    let bytes = write_lockfile(&lock);
    assert_eq!(check_lockfile(&bytes, &sha256_hex(&bytes)).unwrap(), lock);
}

#[test]
fn client_skips_server_only_packages() {
    let lock = Lockfile {
        packages: vec![
            lock_entry("server_only", ExecutionContext::Server),
            lock_entry("client_only", ExecutionContext::Client),
            lock_entry("shared", ExecutionContext::Shared),
        ],
    };
    let fetched = packages_to_fetch(&lock);
    let ids: Vec<&str> = fetched.iter().map(|entry| entry.id.as_str()).collect();
    assert_eq!(ids, ["client_only", "shared"]);
    assert!(
        fetched
            .iter()
            .all(|entry| entry.context != ExecutionContext::Server)
    );
}

#[test]
fn build_acquired_requires_exactly_one_visual_per_unit() {
    let lockfile_hash = sha256_hex(b"lockfile");

    // Appendix A: one unit, one visual.
    let acquired = build_acquired(&[parse(&read_files(&acme_dir()))], lockfile_hash.clone())
        .expect("acme has one visual per unit");
    assert_eq!(
        acquired.visuals["acme:scout"],
        Shape::Cuboid {
            x: 1.0,
            y: 0.8,
            z: 1.4
        }
    );
    assert_eq!(acquired.visuals.len(), 1);
    assert_eq!(acquired.map_size, (48.0, 48.0));
    assert_eq!(acquired.unit_names["acme:scout"], "Scout");
    assert!(
        acquired
            .content
            .catalog
            .units
            .contains_key(&turts_mod_api::ContentId::parse("acme:scout").unwrap())
    );
    assert_eq!(acquired.lockfile_hash, lockfile_hash);

    // The visual file is not exported: the unit has no visual.
    let no_visual = acme_with_manifest_edit(r#"visuals: ["visuals/scout.ron"]"#, "visuals: []");
    let error = build_acquired(&[parse(&no_visual)], lockfile_hash.clone()).unwrap_err();
    assert!(error.contains("unit `acme:scout` has no visual"), "{error}");
    assert!(
        error.starts_with("acme/manifest.ron: exports.visuals: "),
        "{error}"
    );

    // A second visual file for the same unit: the unit has two visuals.
    let mut two_visuals = acme_with_manifest_edit(
        r#"visuals: ["visuals/scout.ron"]"#,
        r#"visuals: ["visuals/scout.ron", "visuals/scout_again.ron"]"#,
    );
    let first = two_visuals["visuals/scout.ron"].clone();
    two_visuals.insert("visuals/scout_again.ron".to_string(), first);
    let error = build_acquired(&[parse(&two_visuals)], lockfile_hash).unwrap_err();
    assert!(
        error.starts_with("acme/visuals/scout_again.ron: target: "),
        "{error}"
    );
    assert!(error.contains("`acme:scout`"), "{error}");
}

#[test]
fn build_acquired_refuses_a_visual_of_an_unknown_unit() {
    let lockfile_hash = sha256_hex(b"lockfile");
    // A second visual file whose unit no unit file defines. The real unit keeps its visual.
    let mut orphan = acme_with_manifest_edit(
        r#"visuals: ["visuals/scout.ron"]"#,
        r#"visuals: ["visuals/scout.ron", "visuals/ghost.ron"]"#,
    );
    orphan.insert(
        "visuals/ghost.ron".to_string(),
        b"(target: Unit(\"acme:ghost\"), shape: Capsule(radius: 0.3, height: 1.0))".to_vec(),
    );
    let error = build_acquired(&[parse(&orphan)], lockfile_hash.clone()).unwrap_err();
    assert!(
        error.starts_with("acme/visuals/ghost.ron: target: "),
        "{error}"
    );
    assert!(error.contains("`acme:ghost`"), "{error}");
    // The message lists the unit that the package does define.
    assert!(error.contains("(allowed: acme:scout)"), "{error}");

    // A typo in the unit of the only visual: the visual is an orphan, and the message says so.
    let mut typo = read_files(&acme_dir());
    let text = String::from_utf8(typo["visuals/scout.ron"].clone()).unwrap();
    assert!(text.contains("acme:scout"));
    typo.insert(
        "visuals/scout.ron".to_string(),
        text.replace("acme:scout", "acme:scoot").into_bytes(),
    );
    let error = build_acquired(&[parse(&typo)], lockfile_hash).unwrap_err();
    assert!(
        error.starts_with("acme/visuals/scout.ron: target: "),
        "{error}"
    );
    assert!(error.contains("`acme:scoot`"), "{error}");
}

#[test]
fn check_advertisement_refuses_other_protocol() {
    let good = Advertisement {
        mod_id: "acme".to_string(),
        mod_version: "0.1.0".to_string(),
        engine_version: turts_mod_api::ENGINE_API_VERSION.to_string(),
        protocol_version: NET_PROTOCOL_VERSION,
        lockfile_url: "http://h/lockfile.json".to_string(),
        lockfile_hash: sha256_hex(b"lockfile"),
    };
    let json = |ad: &Advertisement| serde_json::to_vec(ad).expect("serialize");

    assert_eq!(check_advertisement(&json(&good)).unwrap(), good);

    let other_protocol = Advertisement {
        protocol_version: NET_PROTOCOL_VERSION + 1,
        ..good.clone()
    };
    let error = check_advertisement(&json(&other_protocol)).unwrap_err();
    assert!(error.contains("protocol"), "{error}");

    let other_engine = Advertisement {
        engine_version: "0.1.0".to_string(),
        ..good.clone()
    };
    let error = check_advertisement(&json(&other_engine)).unwrap_err();
    assert!(error.contains("engine"), "{error}");

    let error = check_advertisement(b"not json").unwrap_err();
    assert!(error.contains("advertisement"), "{error}");
}
