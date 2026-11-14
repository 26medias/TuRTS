//! Declarative effects are bounded before a package reaches the renderer.
mod common;
use std::collections::BTreeMap;
use turts_mod_runtime::{archive::PackageArchive, pack::pack_files, visuals::visuals_by_content};

fn files(effects: &str) -> BTreeMap<String, Vec<u8>> {
    let mut files = common::read_files(&common::package_dir("acme"));
    let text = String::from_utf8(files["manifest.ron"].clone())
        .unwrap()
        .replace("^0.2.3", "^0.2.7")
        .replace(
            "visuals: [",
            "visual_effects: [\"effects.ron\"], visuals: [",
        );
    files.insert("manifest.ron".into(), text.into_bytes());
    files.insert(
        "effects.ron".into(),
        format!("(effects:[{effects}])").into_bytes(),
    );
    files
}
fn check(files: &BTreeMap<String, Vec<u8>>) -> Result<(), turts_mod_api::ModError> {
    let archive = PackageArchive::parse(&pack_files(files).unwrap()).unwrap();
    visuals_by_content(&[archive]).map(|_| ())
}
#[test]
fn legacy_kinds_and_layered_effects_are_accepted() {
    for kind in ["Flash", "Spark", "Tracer", "Burst", "Smoke"] {
        check(&files(&format!(
            "(id:\"legacy\",kind:{kind},colour:\"#ffffff\",size:1.0,ms:100)"
        )))
        .unwrap();
    }
    check(&files(r##"(id:"hit", layers:[(kind:Sparks(count:8,speed:3.0,spread_deg:60.0,gravity:9.0),colour:"#ffd27a",ms:250),(kind:Flash(radius:0.5),ms:80)],cost:Small)"##)).unwrap();
    check(&files(
        r#"(id:"scorch",layers:[(kind:Decal(size:2.0,ms:60000))])"#,
    ))
    .unwrap();
}
#[test]
fn rejects_layer_count_particle_count_lifetime_cost_and_cycles() {
    let cases = [
        (
            format!("(id:\"bad\",layers:[{}])", ["(kind:Flash())"; 9].join(",")),
            "8 layers",
        ),
        (
            "(id:\"bad\",layers:[(kind:Sparks(count:65))],cost:Large)".into(),
            "64 particles",
        ),
        (
            "(id:\"bad\",layers:[(kind:Burst(),ms:10001)],cost:Large)".into(),
            "10000 ms",
        ),
        (
            "(id:\"bad\",layers:[(kind:Sparks(count:32))],cost:Small)".into(),
            "cost is too small",
        ),
        (
            "(id:\"bad\",layers:[(kind:Projectile(trail:Some(\"acme:bad\")))])".into(),
            "cycle",
        ),
        (
            "(id:\"bad\",layers:[(kind:Trail(ms:10001))])".into(),
            "trail duration",
        ),
    ];
    for (text, expected) in cases {
        let error = check(&files(&text)).unwrap_err();
        assert!(
            error.to_string().contains(expected),
            "expected {expected}: {error}"
        );
        assert!(error.file.is_some(), "{error}");
    }
}
#[test]
fn effect_layers_require_api_0_2_7_even_when_empty() {
    let mut files = files("(id:\"new\",layers:[])");
    let manifest = String::from_utf8(files["manifest.ron"].clone())
        .unwrap()
        .replace("^0.2.7", "^0.2.6");
    files.insert("manifest.ron".into(), manifest.into_bytes());
    let error = check(&files).unwrap_err().to_string();
    assert!(error.contains("needs engine api 0.2.7"), "{error}");
}
