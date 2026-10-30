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
                .any(|hit| hit.source.ends_with(format!("plant_{i}.rs"))),
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

#[test]
fn e1_scans_forge_tokens_and_stray_ron_rules() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mods = repo.join("mods");
    let catalog = derive_tokens(&mods).expect("all exported definitions derive");
    let probe = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("forge_token_probe.rs");
    std::fs::write(
        &probe,
        "const A: &str = \"forge:make_shells\"; const B: &str = \"forge:hardened_shells\"; const C: &str = \"forge:drill\";",
    ).unwrap();
    let hits = scan_rust(std::slice::from_ref(&probe), &catalog).expect("token probe parses");
    for (token, category) in [
        ("forge:make_shells", "recipe"),
        ("forge:hardened_shells", "technology"),
        ("forge:drill", "unit"),
    ] {
        assert!(
            hits.iter()
                .any(|hit| hit.token == token && hit.category == category),
            "E1 did not scan forge definition token {token}: {hits:#?}"
        );
    }
    let _ = std::fs::remove_file(probe);

    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("forge_stray_ron");
    for (kind, path) in [
        ("recipes", "recipes/stray.ron"),
        ("technologies", "technologies/stray.ron"),
    ] {
        let _ = std::fs::remove_dir_all(&scratch);
        copy_tree(&mods, &scratch);
        std::fs::write(scratch.join("forge").join(path), "()").unwrap();
        assert!(
            derive_tokens(&scratch).is_err(),
            "unexported {kind} RON must be rejected"
        );
    }
    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn hud3_order_id_is_a_content_token() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("hud3_order_token");
    let mods = scratch.join("mods");
    let _ = std::fs::remove_dir_all(&scratch);
    copy_tree(&repo.join("mods"), &mods);
    let acme = mods.join("acme");
    for path in ["orders", "input", "assets", "interface"] {
        let path = acme.join(path);
        if path.exists() {
            std::fs::remove_dir_all(path).unwrap();
        }
    }
    let manifest_path = mods.join("acme/manifest.ron");
    std::fs::write(
        &manifest_path,
        r#"(
            id: "acme", version: "0.3.0", kind: Game, engine_api: "^0.2.3",
            dependencies: {},
            exports: (
                units: ["units/scout.ron"], maps: ["maps/field.ron"],
                visuals: ["visuals/scout.ron"], resources: [], nodes: [],
                production: [], orders: ["orders/rush.ron"],
                input: ["input/bindings.ron"], assets: ["assets/main.ron"],
            ),
            context: Shared, capabilities: [], license: "LicenseRef-TBD",
        )"#,
    )
    .unwrap();
    std::fs::create_dir_all(mods.join("acme/orders")).unwrap();
    std::fs::create_dir_all(mods.join("acme/input")).unwrap();
    std::fs::create_dir_all(mods.join("acme/assets")).unwrap();
    std::fs::write(mods.join("acme/orders/rush.ron"), r#"(id:"rush",name:"Rush",behaviour:Move,target:Point,fire:Some(Never),engage:None,icon:"rocket",hotkey:"launch",cursor:None,card:None,units:["acme:scout"],description:"Move quickly")"#).unwrap();
    std::fs::write(
        mods.join("acme/input/bindings.ron"),
        r#"(actions:[(name:"launch",default:[Key(code:"KeyL",mods:[])])],engine_overrides:[(name:"select",bindings:[Key(code:"KeyS",mods:[])])])"#,
    )
    .unwrap();
    std::fs::write(mods.join("acme/assets/main.ron"), r#"(images:[(id:"rocket",path:"assets/rocket.png",nine_slice:None)],fonts:[],sounds:[],cursors:[])"#).unwrap();
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    png.extend_from_slice(&32u32.to_be_bytes());
    png.extend_from_slice(&32u32.to_be_bytes());
    png.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0]);
    std::fs::write(mods.join("acme/assets/rocket.png"), png).unwrap();

    let catalog = derive_tokens(&mods).expect("HUD3 fixture package derives");
    let probe = scratch.join("rush_probe.rs");
    std::fs::write(
        &probe,
        "fn probe() { let s = \"rush\"; let a = \"launch\"; let i = \"rocket\"; let e = \"select\"; }",
    )
    .unwrap();
    let hits = scan_rust(std::slice::from_ref(&probe), &catalog).unwrap();
    assert!(
        hits.iter().any(|hit| {
            hit.token == "rush"
                && hit.package == "acme"
                && hit.definition_path == "orders/rush.ron"
                && hit.field == "id"
        }),
        "order id must be present in the content token scan: {hits:#?}"
    );
    assert!(
        hits.iter()
            .any(|hit| hit.token == "launch" && hit.package == "acme" && hit.field == "name"),
        "input action name must be a scanned token: {hits:#?}"
    );
    assert!(
        hits.iter()
            .any(|hit| hit.token == "rocket" && hit.package == "acme" && hit.field == "id"),
        "asset id must be a scanned token: {hits:#?}"
    );
    assert!(
        !hits.iter().any(|hit| hit.token == "select"),
        "engine override names must not be mod content tokens: {hits:#?}"
    );
}

fn assert_forge_plant_is_found(name: &str, source: &str, token: &str, category: &str) {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = derive_tokens(&repo.join("mods")).expect("all exported definitions derive");
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("forge_plant_{name}"));
    let _ = std::fs::remove_dir_all(&root);
    let src = root.join("src");
    std::fs::create_dir_all(&src).unwrap();
    let plant = src.join(format!("{name}.rs"));
    std::fs::write(&plant, source).unwrap();
    let hits = scan_rust(&[src], &catalog).expect("production Rust plant parses");
    assert!(
        hits.iter()
            .any(|hit| { hit.token == token && hit.category == category && hit.source == plant }),
        "production Rust plant escaped E1: {hits:#?}"
    );
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn e1_rejects_production_recipe_reference_plant() {
    assert_forge_plant_is_found(
        "recipe",
        "const RECIPE: &str = \"forge:make_shells\";",
        "forge:make_shells",
        "recipe",
    );
}

#[test]
fn e1_rejects_production_technology_reference_plant() {
    assert_forge_plant_is_found(
        "technology",
        "const TECHNOLOGY: &str = \"forge:hardened_shells\";",
        "forge:hardened_shells",
        "technology",
    );
}

#[test]
fn e1_rejects_production_unit_reference_plant() {
    assert_forge_plant_is_found(
        "unit",
        "const UNIT: &str = \"forge:drill\";",
        "forge:drill",
        "unit",
    );
}
