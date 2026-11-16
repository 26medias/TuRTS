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
        "const D: &str = \"cedar.tar\";",
        "const E: &str = \"mods/haul/units/worker.ron\";",
        "const F: &str = \"haul/units/worker.ron\";",
        "const G: &str = \"acme:field\";",
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
        "struct Generic { rifleman: usize }\nfn field() { let field = Generic { rifleman: 2 }; let _ = field.rifleman; }\n// base:rifleman\n",
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
fn ordinary_identifiers_and_display_words_are_not_content_selectors() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = derive_tokens(&repo.join("mods")).unwrap();
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ordinary_vocabulary");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("vocabulary.rs"), r#"
fn rifleman(unit: u64, depot: u64, credits: u64, contact: u64) {}
fn r#rifleman() { rifleman!(); }
enum Behaviour { Construct, Hold }
const LABELS: &[&str] = &["flat", "base", "credit", "deposit", "light", "unit", "Construct", "Hold", "Rifleman"];
"#).unwrap();
    let hits = scan_rust(&[root], &catalog).unwrap();
    assert!(
        hits.is_empty(),
        "ordinary vocabulary is not a content dependency: {hits:?}"
    );
}

#[test]
fn explicit_content_selector_literals_remain_rejected() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = derive_tokens(&repo.join("mods")).unwrap();
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("explicit_selectors");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("selectors.rs"),
        r#"
const A: &str = "base:flat";
const B: &[u8] = b"base:credits";
const C: &str = "maps/flat.ron";
const D: &str = "cedar.tar";
"#,
    )
    .unwrap();
    let hits = scan_rust(&[root], &catalog).unwrap();
    for token in ["base:flat", "base:credits", "maps/flat.ron", "cedar.tar"] {
        assert!(
            hits.iter().any(|hit| hit.token == token),
            "selector escaped: {token}: {hits:?}"
        );
    }
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
    std::fs::write(src.join("fresh.rs"), "const X: &str = \"base:credits\";").unwrap();
    let hits = scan_rust(&[src], &catalog).unwrap();
    assert!(
        hits.iter().any(|hit| hit.token == "base:credits"
            && hit.package == "base"
            && hit.definition_path == "resources/credits.ron"
            && hit.field == "id"),
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
        "fn probe() { let s = \"acme:rush\"; let a = \"launch\"; let i = \"rocket\"; let e = \"select\"; }",
    )
    .unwrap();
    let hits = scan_rust(std::slice::from_ref(&probe), &catalog).unwrap();
    assert!(
        hits.iter().any(|hit| {
            hit.token == "acme:rush"
                && hit.package == "acme"
                && hit.definition_path == "orders/rush.ron"
                && hit.field == "id"
        }),
        "order id must be present in the content token scan: {hits:#?}"
    );
    assert!(
        !hits
            .iter()
            .any(|hit| matches!(hit.token.as_str(), "launch" | "rocket" | "select")),
        "bare action, asset and engine labels are ambiguous vocabulary: {hits:?}"
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

/// Scan a tree that holds the listed file `visuals/tests.rs` with a content id inside it.
/// `parent_source` is the text of the parent file `visuals.rs`.
fn scan_listed_test_module(
    name: &str,
    parent_source: &str,
) -> (
    Result<Vec<support::Hit>, support::ScanError>,
    PathBuf,
    PathBuf,
) {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = derive_tokens(&repo.join("mods")).expect("all exported definitions derive");
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("test_module_{name}"));
    let _ = std::fs::remove_dir_all(&root);
    let src = root.join("crates/turts_client/src");
    std::fs::create_dir_all(src.join("visuals")).unwrap();
    let parent = src.join("visuals.rs");
    let file = src.join("visuals/tests.rs");
    std::fs::write(&parent, parent_source).unwrap();
    std::fs::write(&file, "const RECIPE: &str = \"forge:make_shells\";").unwrap();
    let result = scan_rust(&[root.join("crates")], &catalog);
    let _ = std::fs::remove_dir_all(root);
    (result, parent, file)
}

fn assert_test_module_plant_is_red(name: &str, parent_source: &str, why: &str) {
    let (result, parent, file) = scan_listed_test_module(name, parent_source);
    let error = result.expect_err("an ungated listed test-module file must make the scan red");
    assert!(
        matches!(error, support::ScanError::TestModule(..)),
        "wrong error kind: {error}"
    );
    let text = error.to_string();
    for needed in [
        file.display().to_string(),
        parent.display().to_string(),
        why.to_string(),
    ] {
        assert!(text.contains(&needed), "message lacks {needed:?}: {text}");
    }
}

#[test]
fn listed_test_module_with_cfg_test_gate_is_skipped() {
    let (result, ..) = scan_listed_test_module("gated", "fn a() {}\n#[cfg(test)]\nmod tests;\n");
    let hits = result.expect("a gated test module parses and is skipped");
    assert!(
        hits.is_empty(),
        "gated test file must stay skipped: {hits:#?}"
    );
}

#[test]
fn listed_test_module_without_cfg_test_is_red() {
    assert_test_module_plant_is_red("no_cfg", "fn a() {}\nmod tests;\n", "is not gated");
}

#[test]
fn listed_test_module_with_not_test_gate_is_red() {
    assert_test_module_plant_is_red(
        "not_test",
        "#[cfg(not(test))]\nmod tests;\n",
        "is not gated",
    );
}

#[test]
fn listed_test_module_with_inline_body_is_red() {
    assert_test_module_plant_is_red(
        "inline",
        "#[cfg(test)]\nmod tests { const A: u8 = 1; }\n",
        "has an inline body",
    );
}

#[test]
fn listed_test_module_without_declaration_is_red() {
    assert_test_module_plant_is_red("missing", "fn a() {}\n", "declares no `mod tests`");
}

#[test]
fn api17_terrain_exports_and_refs_remain_content_tokens() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = derive_tokens(&repo.join("mods")).expect("all API17 public exports derive");
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("terrain_content_refs");
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).unwrap();
    let file = scratch.join("refs.rs");
    std::fs::write(
        &file,
        r#"fn render(light: u8) {}
const MATERIAL: &str="quarry:soil_layer";
const TOOL: &str="quarry:spade";
const CLASS: &str="quarry:light";
const WRONG: &str="light";
"#,
    )
    .unwrap();
    let hits = scan_rust(std::slice::from_ref(&scratch), &catalog).unwrap();
    for token in ["quarry:soil_layer", "quarry:spade", "quarry:light"] {
        assert!(
            hits.iter().any(|hit| hit.token == token),
            "missing actual content token {token}: {hits:?}"
        );
    }
    assert!(
        !hits.iter().any(|hit| hit.token == "light"),
        "generic light identifier must not become a mod dependency"
    );
    std::fs::remove_dir_all(scratch).unwrap();
}

#[test]
fn explicit_relative_test_path_is_safe_only_without_a_production_inclusion() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let catalog = derive_tokens(&repo.join("mods")).unwrap();
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("relative_test_path");
    let _ = std::fs::remove_dir_all(&root);
    let src = root.join("src");
    std::fs::create_dir_all(src.join("bots")).unwrap();
    std::fs::write(src.join("main.rs"), "mod bots;").unwrap();
    std::fs::write(
        src.join("bots/mod.rs"),
        r#"#[cfg(test)] #[path="../nested_tests.rs"] mod tests;"#,
    )
    .unwrap();
    std::fs::write(
        src.join("nested_tests.rs"),
        r#"const RECIPE: &str="forge:make_shells";"#,
    )
    .unwrap();
    assert!(
        scan_rust(std::slice::from_ref(&src), &catalog)
            .unwrap()
            .is_empty()
    );
    std::fs::write(
        src.join("leak.rs"),
        r#"#[path="nested_tests.rs"] mod production;"#,
    )
    .unwrap();
    let error = scan_rust(&[src], &catalog)
        .expect_err("a second production inclusion cannot inherit a test exemption");
    assert!(
        matches!(error, support::ScanError::TestModule(..)),
        "{error}"
    );
    assert!(error.to_string().contains("is not gated"), "{error}");
    std::fs::remove_dir_all(root).unwrap();
}
