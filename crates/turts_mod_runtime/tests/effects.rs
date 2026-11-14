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

#[test]
fn local_effect_images_obey_the_pack_limit() {
    use image::ImageEncoder;
    let mut files = files("(id:\"hit\",layers:[(kind:Flash(),image:Some(\"acme:large\"))])");
    let mut png = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png)
        .write_image(&vec![255; 513 * 4], 513, 1, image::ExtendedColorType::Rgba8)
        .unwrap();
    let mut assets: turts_mod_api::AssetsDef =
        ron::de::from_bytes(&files["assets/assets.ron"]).unwrap();
    assets.images.push(turts_mod_api::ImageAsset {
        id: "large".into(),
        path: "assets/large.png".into(),
        nine_slice: None,
    });
    files.insert(
        "assets/assets.ron".into(),
        ron::ser::to_string(&assets).unwrap().into_bytes(),
    );
    files.insert("assets/large.png".into(), png);
    let error = check(&files).unwrap_err().to_string();
    assert!(error.contains("513x1 exceeds 512 px"), "{error}");
}
#[test]
fn a_damage_list_and_the_alert_jump_key_require_api_0_2_7() {
    for kind in ["damaged", "jump"] {
        let mut files = files("(id:\"hit\",layers:[])");
        // Keep this version probe independent from the effect-layer version row.
        files.insert("effects.ron".into(), b"(effects:[])".to_vec());
        if kind == "damaged" {
            files.insert("visuals/scout.ron".into(),b"(target:Unit(\"acme:scout\"),shape:Cuboid(x:1.0,y:0.8,z:1.4),effects:Some((damaged:Some([]))))".to_vec());
        } else {
            let text = String::from_utf8(files["input/bindings.ron"].clone()).unwrap();
            let mut input: turts_mod_api::InputMapDef = ron::from_str(&text).unwrap();
            input.engine_overrides.push(turts_mod_api::EngineOverride {
                name: "jump_to_alert".into(),
                bindings: vec![],
            });
            files.insert(
                "input/bindings.ron".into(),
                ron::ser::to_string(&input).unwrap().into_bytes(),
            );
        }
        let text = String::from_utf8(files["manifest.ron"].clone())
            .unwrap()
            .replace("^0.2.7", "^0.2.6");
        files.insert("manifest.ron".into(), text.into_bytes());
        let error = check(&files).unwrap_err().to_string();
        assert!(error.contains("needs engine api 0.2.7"), "{kind}: {error}");
    }
}
