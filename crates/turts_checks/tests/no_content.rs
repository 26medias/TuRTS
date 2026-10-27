mod support;

use std::path::PathBuf;
use support::{derive_tokens, scan_rust};

#[test]
fn derived_scan_rejects_all_plants_and_accepts_generic_field() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mods = repo.join("mods");
    let catalog = derive_tokens(&mods).expect("all mod packages derive");
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("derived_scan");
    let src = root.join("src");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&src).unwrap();
    let plants = [
        "const A: &str = \"base:rifleman\";",
        "const B: &str = \"cedar:wood\";",
        "fn rifleman() {}",
        "const D: &str = \"cedar.tar\";",
        "const E: &str = \"mods/haul/units/worker.ron\";",
        "const F: &str = \"haul/units/worker.ron\";",
        "const G: &str = \"acme:field\";",
        "fn r#rifleman() {}",
        "fn macro_path() { rifleman!(); }",
        "const H: &[u8] = b\"base:rifleman\";",
    ];
    for (i, plant) in plants.iter().enumerate() {
        std::fs::write(src.join(format!("plant_{i}.rs")), plant).unwrap();
    }
    std::fs::write(
        src.join("inline.rs"),
        "const X: &str = \"base:rifleman\"; // comment\n",
    )
    .unwrap();
    std::fs::write(
        src.join("generic.rs"),
        "fn field() { let field = 2; }\n// base:rifleman\n",
    )
    .unwrap();
    std::fs::write(
        src.join("tests.rs"),
        "#[cfg(test)] mod tests { const X: &str = \"base:rifleman\"; }\n",
    )
    .unwrap();
    std::fs::write(
        src.join("nested_tests.rs"),
        "#[cfg(all(test, feature = \"fixture\"))] mod tests { const X: &str = \"base:rifleman\"; }\n",
    )
    .unwrap();

    std::fs::write(
        src.join("production_cfg.rs"),
        "#[cfg(not(test))] const A: &str = \"base:rifleman\";\n#[cfg(any(test, feature = \"live\"))] const B: &str = \"base:rifleman\";\n#[cfg(not(not(feature = \"live\")))] const C: &str = \"base:rifleman\";\n#[cfg(not(not(test)))] const D: &str = \"base:rifleman\";\n#[cfg(all(test, feature = \"unit\"))] const E: &str = \"base:rifleman\";",
    )
    .unwrap();
    let hits = scan_rust(&[src], &catalog).expect("fixture sources parse");
    assert!(
        hits.iter().any(|hit| hit.source.ends_with("inline.rs")),
        "inline comment must not hide code: {hits:#?}"
    );
    for i in 0..plants.len() {
        assert!(
            hits.iter()
                .any(|hit| hit.source.ends_with(&format!("plant_{i}.rs"))),
            "plant {i} escaped: {hits:#?}"
        );
    }
    assert!(
        !hits.iter().any(|hit| hit.source.ends_with("generic.rs")),
        "generic field/comments are allowed: {hits:#?}"
    );
    assert!(
        !hits.iter().any(|hit| hit.source.ends_with("tests.rs")),
        "test-only subtree is excluded: {hits:#?}"
    );
    assert!(
        !hits
            .iter()
            .any(|hit| hit.source.ends_with("nested_tests.rs")),
        "nested cfg(test) subtree is excluded: {hits:#?}"
    );
    let cfg_hits = hits
        .iter()
        .filter(|hit| hit.source.ends_with("production_cfg.rs"))
        .collect::<Vec<_>>();
    for production_line in [1, 2, 3] {
        assert!(
            cfg_hits.iter().any(|hit| hit.line == production_line),
            "production cfg line {production_line} must be scanned: {cfg_hits:#?}"
        );
    }
    assert!(
        !cfg_hits.iter().any(|hit| hit.line >= 4),
        "cfg expressions impossible with test=false must be excluded: {cfg_hits:#?}"
    );
}

#[test]
fn credit_exemption_applies_only_to_identifiers() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = derive_tokens(&repo.join("mods")).expect("all mod packages derive");
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("credit_identifier_only");
    let src = root.join("src");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("identifier.rs"),
        "fn ledger() { let credit = 1; { let r#credit = 2; } }",
    )
    .unwrap();
    std::fs::write(
        src.join("literals.rs"),
        "const A: &str = \"credit\"; const B: &[u8] = b\"credit\"; const C: &str = \"base:credits\"; const D: &str = \"resources/credits.ron\";",
    )
    .unwrap();
    std::fs::write(
        src.join("generic_literals.rs"),
        "const A: &str = \"field\"; const B: &str = \"Item\"; const C: &str = \"Balance\"; const D: &str = \"log\";",
    )
    .unwrap();

    let hits = scan_rust(&[src], &catalog).expect("fixture sources parse");
    assert!(
        !hits.iter().any(|hit| hit.source.ends_with("identifier.rs")),
        "generic and raw identifiers should be exempt: {hits:#?}"
    );
    assert!(
        !hits
            .iter()
            .any(|hit| hit.source.ends_with("generic_literals.rs")),
        "existing generic-word literal exceptions must retain their prior behavior: {hits:#?}"
    );
    for (token, category) in [
        ("credit", "unit-name"),
        ("base:credits", "resource-ref"),
        ("resources/credits.ron", "resource-export"),
    ] {
        assert!(
            hits.iter().any(|hit| {
                hit.source.ends_with("literals.rs")
                    && hit.token == token
                    && hit.category == category
            }),
            "literal {token:?} ({category}) must remain a hit: {hits:#?}"
        );
    }
    assert_eq!(
        hits.iter()
            .filter(|hit| hit.source.ends_with("literals.rs") && hit.token == "credit")
            .count(),
        2,
        "both string and byte-string literals must remain visible"
    );
}

#[test]
fn real_source_scan_is_clean() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = derive_tokens(&repo.join("mods")).expect("all mod packages derive");
    let hits = scan_rust(&[repo.join("crates")], &catalog).expect("production Rust parses");
    assert!(hits.is_empty(), "unexpected content references: {hits:#?}");
}

#[test]
fn derivation_tracks_modified_package_and_fails_closed() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("derived_packages");
    let mods = scratch.join("mods");
    let _ = std::fs::remove_dir_all(&scratch);
    copy_tree(&repo.join("mods"), &mods);
    let definition = mods.join("base/resources/credits.ron");
    let original = std::fs::read_to_string(&definition).unwrap();
    std::fs::write(
        &definition,
        original.replace("Credits", "FreshCreditHarvester"),
    )
    .unwrap();
    let catalog = derive_tokens(&mods).expect("mutated package parses");
    let src = scratch.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("fresh.rs"),
        "const X: &str = \"FreshCreditHarvester\";",
    )
    .unwrap();
    let hits = scan_rust(&[src], &catalog).unwrap();
    assert!(
        hits.iter().any(|hit| hit.token == "FreshCreditHarvester"
            && hit.package == "base"
            && hit.definition_path == "resources/credits.ron"
            && hit.field == "name"),
        "changed package tokens must retain provenance: {hits:#?}"
    );

    let missing = scratch.join("missing");
    copy_tree(&mods, &missing);
    std::fs::remove_file(missing.join("base/resources/credits.ron")).unwrap();
    assert!(
        derive_tokens(&missing).is_err(),
        "missing exported definition must fail closed"
    );
    let malformed = scratch.join("malformed");
    copy_tree(&mods, &malformed);
    std::fs::write(malformed.join("base/resources/credits.ron"), "(").unwrap();
    assert!(
        derive_tokens(&malformed).is_err(),
        "malformed exported definition must fail closed"
    );
    let stray = scratch.join("stray");
    copy_tree(&mods, &stray);
    std::fs::write(stray.join("base/units/not_exported.ron"), "()").unwrap();
    assert!(
        derive_tokens(&stray).is_err(),
        "unexported RON definition must fail closed"
    );
    let dangling_map = scratch.join("dangling_map");
    copy_tree(&mods, &dangling_map);
    let map_path = dangling_map.join("acme/maps/field.ron");
    let map_text = std::fs::read_to_string(&map_path).unwrap();
    std::fs::write(&map_path, map_text.replace("acme:scout", "acme:missing")).unwrap();
    assert!(
        derive_tokens(&dangling_map).is_err(),
        "dangling map unit reference must fail closed"
    );
    let dangling_visual = scratch.join("dangling_visual");
    copy_tree(&mods, &dangling_visual);
    let visual_path = dangling_visual.join("acme/visuals/scout.ron");
    let visual_text = std::fs::read_to_string(&visual_path).unwrap();
    std::fs::write(
        &visual_path,
        visual_text.replace("acme:scout", "acme:missing"),
    )
    .unwrap();
    assert!(
        derive_tokens(&dangling_visual).is_err(),
        "dangling visual target must fail closed"
    );
    let empty = scratch.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    assert!(
        derive_tokens(&empty).is_err(),
        "empty package discovery must fail closed"
    );
}

fn copy_tree(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let destination = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            std::fs::copy(entry.path(), destination).unwrap();
        }
    }
}
