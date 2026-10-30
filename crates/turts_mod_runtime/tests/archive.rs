//! Archive read, RON error rewrite, and hash check.

mod common;

use common::{acme_archive, package_dir, raw_tar, read_files};
use turts_mod_api::{Footprint, UnitDef};
use turts_mod_runtime::archive::PackageArchive;
use turts_mod_runtime::hash::{sha256_hex, verify_hash};
use turts_mod_runtime::lock::{read_lockfile, write_lockfile};
use turts_mod_runtime::parse::parse_ron;

/// The text of one `acme` file.
fn acme_text(path: &str) -> String {
    String::from_utf8(read_files(&package_dir("acme"))[path].clone()).unwrap()
}

#[test]
fn archive_accepts_a_safe_hand_built_archive() {
    // Control for the rejection cases below: the same builder, a safe archive, is accepted.
    let manifest = acme_text("manifest.ron");
    let bytes = raw_tar(&[
        ("manifest.ron", manifest.as_bytes()),
        ("extra/ok.ron", b"()"),
    ]);
    let archive = PackageArchive::parse(&bytes).unwrap();
    assert_eq!(archive.id().0, "acme");
    assert_eq!(archive.file("extra/ok.ron"), Some(&b"()"[..]));
    assert_eq!(archive.file("missing.ron"), None);
}

#[test]
fn archive_rejects_unsafe_and_duplicate_paths() {
    let manifest = acme_text("manifest.ron");
    let manifest = manifest.as_bytes();
    // Every case holds a valid manifest, so the only possible cause of the error is the entry.
    let unsafe_names = [
        "../x.ron",
        "/abs.ron",
        "a/../b.ron",
        "./x.ron",
        "a//b.ron",
        "dir\\x.ron",
    ];
    for name in unsafe_names {
        let bytes = raw_tar(&[("manifest.ron", manifest), (name, b"()")]);
        let error = PackageArchive::parse(&bytes).unwrap_err();
        assert_eq!(error.field_path, "archive", "{name}: {error}");
        assert!(
            error.message.contains("unsafe path") && error.message.contains(name),
            "{name}: {error}"
        );
    }
    let bytes = raw_tar(&[("manifest.ron", manifest), ("manifest.ron", manifest)]);
    let error = PackageArchive::parse(&bytes).unwrap_err();
    assert_eq!(error.field_path, "archive", "{error}");
    assert!(error.message.contains("duplicate path"), "{error}");
    assert!(error.message.contains("manifest.ron"), "{error}");
}

#[test]
fn archive_without_manifest_is_rejected() {
    let bytes = raw_tar(&[("units/scout.ron", acme_text("units/scout.ron").as_bytes())]);
    let error = PackageArchive::parse(&bytes).unwrap_err();
    assert_eq!(error.field_path, "archive", "{error}");
    assert!(error.message.contains("manifest.ron"), "{error}");
    // An archive with no entries has no manifest either.
    let empty = raw_tar(&[]);
    let error = PackageArchive::parse(&empty).unwrap_err();
    assert_eq!(error.field_path, "archive", "{error}");
    assert!(error.message.contains("manifest.ron"), "{error}");
}

#[test]
fn archive_rejects_bytes_that_are_not_a_tar() {
    let error = PackageArchive::parse(&[0xAB; 2048]).unwrap_err();
    assert_eq!(error.field_path, "archive", "{error}");
    assert!(error.message.contains("not a valid tar archive"), "{error}");
}

#[test]
fn archive_rejects_a_symlink_entry() {
    let manifest = acme_text("manifest.ron");
    let mut builder = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_ustar();
    header.set_size(manifest.len() as u64);
    header.set_mode(0o644);
    header.set_entry_type(tar::EntryType::Regular);
    builder
        .append_data(&mut header, "manifest.ron", manifest.as_bytes())
        .unwrap();
    let mut link = tar::Header::new_ustar();
    link.set_size(0);
    link.set_entry_type(tar::EntryType::Symlink);
    builder
        .append_link(&mut link, "link.ron", "manifest.ron")
        .unwrap();
    let error = PackageArchive::parse(&builder.into_inner().unwrap()).unwrap_err();
    assert_eq!(error.field_path, "archive", "{error}");
    assert!(error.message.contains("link.ron"), "{error}");
}

#[test]
fn archive_reports_a_missing_exported_file() {
    let manifest = acme_text("manifest.ron");
    let bytes = raw_tar(&[("manifest.ron", manifest.as_bytes())]);
    let archive = PackageArchive::parse(&bytes).unwrap();
    assert_eq!(archive.units().unwrap_err().field_path, "exports.units");
    assert_eq!(archive.maps().unwrap_err().field_path, "exports.maps");
    assert_eq!(archive.visuals().unwrap_err().field_path, "exports.visuals");
}

#[test]
fn archive_reads_recipe_and_technology_exports_into_catalog() {
    let manifest = r#"(id: "acme", version: "0.2.0", kind: Game, engine_api: "^0.2.1", dependencies: {}, exports: (units: [], maps: ["maps/test.ron"], visuals: [], resources: ["resources/ore.ron", "resources/shell.ron"], nodes: [], production: [], recipes: ["recipes/production-line.ron"], technologies: ["technologies/field-note.ron"]), context: Shared, capabilities: [], license: "x")"#;
    let recipe = r#"(id: "shells", name: "Shells", inputs: [(item: "acme:ore", amount: 2)], outputs: [(item: "acme:shell", amount: 1)], duration_ms: 2000, cancel_refunds: (running: (numerator: 1, denominator: 1), output_blocked: (numerator: 1, denominator: 1)))"#;
    let technology = r#"(id: "hardened", name: "Hardened", prerequisites: [], duration_ms: 10000, costs: [(resource: "acme:shell", amount: 10, source: ProducerInventory)], payment: ReserveThenCharge, cancel_refunds: (queued: (numerator: 1, denominator: 1), running: (numerator: 1, denominator: 2), output_blocked: (numerator: 1, denominator: 1)), effects: [])"#;
    let bytes = raw_tar(&[
        ("manifest.ron", manifest.as_bytes()),
        (
            "resources/ore.ron",
            r#"(id: "ore", name: "Ore", unit_name: "ore", kind: Item)"#.as_bytes(),
        ),
        (
            "resources/shell.ron",
            r#"(id: "shell", name: "Shell", unit_name: "shell", kind: Item)"#.as_bytes(),
        ),
        ("recipes/production-line.ron", recipe.as_bytes()),
        ("technologies/field-note.ron", technology.as_bytes()),
    ]);
    let archive = PackageArchive::parse(&bytes).unwrap();
    assert_eq!(archive.recipes().unwrap().len(), 1);
    assert_eq!(archive.technologies().unwrap().len(), 1);
    let catalog = archive.content().unwrap();
    assert_eq!(catalog.recipes.len(), 1);
    assert_eq!(catalog.technologies.len(), 1);
    assert_eq!(
        catalog.source_files[&turts_mod_api::ContentId::parse("acme:shells").unwrap()],
        "recipes/production-line.ron"
    );
    assert_eq!(
        catalog.source_files[&turts_mod_api::ContentId::parse("acme:hardened").unwrap()],
        "technologies/field-note.ron"
    );
}

#[test]
fn archive_lock_entry_carries_the_manifest_and_the_hash() {
    let bytes = acme_archive();
    let archive = PackageArchive::parse(&bytes).unwrap();
    let entry = archive.lock_entry(vec!["http://h/p.tar".to_string()]);
    assert_eq!(entry.id, "acme");
    assert_eq!(entry.version, "0.3.0");
    assert_eq!(entry.hash, sha256_hex(&bytes));
    assert_eq!(entry.size, bytes.len() as u64);
    assert_eq!(entry.urls, vec!["http://h/p.tar".to_string()]);
    assert_eq!(entry.engine_api, "^0.2.3");
    assert_eq!(archive.check_against_lock(&entry), Ok(()));
}

#[test]
fn parse_ron_rewrites_errors_without_type_names() {
    let text = acme_text("units/scout.ron");
    let typo = text.replacen("name:", "nmae:", 1);
    assert_ne!(typo, text);
    let error = parse_ron::<UnitDef>("acme", "units/scout.ron", &typo).unwrap_err();
    assert_eq!(error.package, "acme");
    assert_eq!(error.file.as_deref(), Some("units/scout.ron"));
    assert_eq!(error.field_path, "units/scout.ron");
    assert!(
        error.message.starts_with("unknown field `nmae`"),
        "{}",
        error.message
    );
    assert!(
        error.message.contains(" at line 1 column "),
        "{}",
        error.message
    );
    assert_eq!(
        error.allowed,
        Some(vec![
            "id".to_string(),
            "name".to_string(),
            "footprint".to_string(),
            "movement".to_string(),
            "inventory".to_string(),
            "gatherer".to_string(),
            "drop_off".to_string(),
            "producer".to_string(),
            "output_inventory".to_string(),
            "extractor".to_string(),
            "conveyor".to_string(),
            "inserter".to_string(),
            "machine".to_string(),
            "research".to_string(),
            "health".to_string(),
            "weapon".to_string(),
            "removal".to_string()
        ])
    );
    assert!(!error.message.contains("UnitDef"), "{}", error.message);

    // The unedited text parses (control).
    let unit = parse_ron::<UnitDef>("acme", "units/scout.ron", &text).unwrap();
    assert_eq!(unit.footprint, Footprint::Circle { radius: 0.6 });
}

#[test]
fn parse_ron_names_a_missing_field() {
    let text = acme_text("units/scout.ron");
    let cut = text.replacen("movement: Some((speed: 6.0)), ", "", 1);
    assert_ne!(cut, text);
    let error = parse_ron::<UnitDef>("acme", "units/scout.ron", &cut).unwrap_err();
    assert!(
        error.message.starts_with("missing field `movement`"),
        "{}",
        error.message
    );
    assert!(
        error.message.contains(" at line 1 column "),
        "{}",
        error.message
    );
    assert!(!error.message.contains("UnitDef"), "{}", error.message);
}

#[test]
fn parse_ron_names_an_unknown_variant_and_its_choices() {
    let text = acme_text("units/scout.ron");
    let bad = text.replacen("Circle", "Square", 1);
    assert_ne!(bad, text);
    let error = parse_ron::<UnitDef>("acme", "units/scout.ron", &bad).unwrap_err();
    assert!(
        error.message.starts_with("unknown variant `Square`"),
        "{}",
        error.message
    );
    assert_eq!(error.allowed, Some(vec!["Circle".to_string()]));
    assert!(!error.message.contains("Footprint"), "{}", error.message);
}

#[test]
fn parse_ron_names_a_duplicate_field() {
    let text = acme_text("units/scout.ron");
    let twice = text.replacen("id: \"scout\",", "id: \"scout\", id: \"again\",", 1);
    assert_ne!(twice, text);
    let error = parse_ron::<UnitDef>("acme", "units/scout.ron", &twice).unwrap_err();
    assert!(
        error.message.starts_with("duplicate field `id`"),
        "{}",
        error.message
    );
    assert!(!error.message.contains("UnitDef"), "{}", error.message);
}

#[test]
fn sc1_typo_helth_names_health() {
    let error = parse_ron::<UnitDef>(
        "acme",
        "units/scout.ron",
        "(id: \"scout\", name: \"Scout\", footprint: Circle(radius: 1.0), helth: Some((max: 5)))",
    )
    .unwrap_err();
    assert!(error.message.contains("did you mean `health`?"), "{error}");
    assert!(error.allowed.unwrap().contains(&"health".to_string()));
}

#[test]
fn sc1_far_name_gives_no_hint() {
    let error = parse_ron::<UnitDef>(
        "acme",
        "units/scout.ron",
        "(id: \"scout\", name: \"Scout\", footprint: Circle(radius: 1.0), zzzzzzzz: None)",
    )
    .unwrap_err();
    assert!(!error.message.contains("did you mean"), "{error}");
}

#[test]
fn sc2_duplicate_unit_key_is_refused() {
    let error = parse_ron::<UnitDef>(
        "acme",
        "units/scout.ron",
        "(id: \"scout\", name: \"Scout\", footprint: Circle(radius: 1.0), health: None, health: None)",
    )
    .unwrap_err();
    assert!(
        error.message.starts_with("duplicate field `health`"),
        "{error}"
    );
}

#[test]
fn parse_ron_wraps_other_errors_with_a_position() {
    let error = parse_ron::<UnitDef>("acme", "units/scout.ron", "(id: ").unwrap_err();
    assert!(
        error.message.starts_with("cannot parse: "),
        "{}",
        error.message
    );
    assert!(error.message.contains(" at line "), "{}", error.message);
    assert_eq!(error.allowed, None);
}

#[test]
fn verify_hash_names_both_hashes() {
    let bytes = b"some archive bytes";
    let good = sha256_hex(bytes);
    assert_eq!(verify_hash(bytes, &good), Ok(()));
    let other = sha256_hex(b"other archive bytes");
    let error = verify_hash(bytes, &other).unwrap_err();
    assert_eq!(error.field_path, "hash");
    assert!(error.message.contains(&good), "{}", error.message);
    assert!(error.message.contains(&other), "{}", error.message);
    // The display form has the shape the client test T2d expects.
    assert!(error.to_string().contains(": hash: "), "{error}");
}

#[test]
fn sha256_hex_matches_a_known_answer() {
    assert_eq!(
        sha256_hex(b""),
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn lockfile_writes_pretty_json_and_reads_it_back() {
    let archive = PackageArchive::parse(&acme_archive()).unwrap();
    let lock = turts_mod_api::Lockfile {
        packages: vec![archive.lock_entry(vec!["http://h/p.tar".to_string()])],
    };
    let bytes = write_lockfile(&lock);
    assert!(
        bytes.ends_with(b"}\n"),
        "{:?}",
        String::from_utf8_lossy(&bytes)
    );
    assert!(bytes.windows(3).any(|w| w == b"\n  "), "pretty printed");
    assert_eq!(read_lockfile(&bytes), Ok(lock));
    let error = read_lockfile(b"{\"packages\": 5}").unwrap_err();
    assert_eq!(error.package, "");
    assert_eq!(error.field_path, "lockfile");
}
