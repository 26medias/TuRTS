//! MD4: real package refusals for model contract violations, plus animated input acceptance.
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};
use turts_mod_runtime::{
    archive::PackageArchive,
    models::{inspect_glb, validate_models},
    pack::pack_files,
};
fn bytes(doc: &Value, blob: &[u8]) -> Vec<u8> {
    let mut json = serde_json::to_vec(doc).unwrap();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let mut bin = blob.to_vec();
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let mut out = Vec::new();
    for n in [
        0x46546c67_u32,
        2,
        (28 + json.len() + bin.len()) as u32,
        json.len() as u32,
        0x4e4f534a,
    ] {
        out.extend(n.to_le_bytes());
    }
    out.extend(json);
    out.extend((bin.len() as u32).to_le_bytes());
    out.extend(0x004e4942_u32.to_le_bytes());
    out.extend(bin);
    out
}
fn sample() -> (Value, Vec<u8>) {
    let positions = [-0.4_f32, 0., 0., 0.4, 0., 0., 0., 1., 0.4];
    let mut blob = Vec::new();
    for v in positions {
        blob.extend(v.to_le_bytes());
    }
    (
        json!({"asset":{"version":"2.0"},"scene":0,"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0,"name":"body"}],"buffers":[{"byteLength":blob.len()}],"bufferViews":[{"buffer":0,"byteLength":blob.len()}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[-0.4,0.,0.],"max":[0.4,1.,0.4]}],"meshes":[{"primitives":[{"attributes":{"POSITION":0},"material":0}]}],"materials":[{"name":"team"}]}),
        blob,
    )
}
fn files() -> BTreeMap<String, Vec<u8>> {
    fn read(root: &Path, path: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let p = entry.unwrap().path();
            if p.is_dir() {
                read(root, &p, out);
            } else {
                out.insert(
                    p.strip_prefix(root).unwrap().to_str().unwrap().to_owned(),
                    std::fs::read(p).unwrap(),
                );
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
    let mut files = BTreeMap::new();
    read(&root, &root, &mut files);
    let manifest_text = String::from_utf8(files["manifest.ron"].clone()).unwrap();
    let manifest: turts_mod_api::Manifest = ron::from_str(&manifest_text).unwrap();
    let mut isolated = manifest_text.replace(&manifest.engine_api, "^0.2.6");
    // Model-contract fixtures do not import later effect, alert, or input features.
    for (name, paths) in [
        ("visual_effects", manifest.exports.visual_effects),
        ("interface", manifest.exports.interface),
    ] {
        let names = paths
            .iter()
            .map(|path| format!("{path:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        let field = format!("{name}: [{names}],");
        assert!(isolated.contains(&field), "missing fixture field {field}");
        isolated = isolated.replace(&field, "");
    }
    files.insert("manifest.ron".into(), isolated.into_bytes());
    let mut input: turts_mod_api::InputMapDef =
        ron::de::from_bytes(&files["input/bindings.ron"]).unwrap();
    input.engine_overrides.clear();
    files.insert(
        "input/bindings.ron".into(),
        ron::ser::to_string(&input).unwrap().into_bytes(),
    );
    // This fixture targets API 0.2.6. Later combat fields do not belong to its checks.
    let path = "units/swordsman.ron";
    let text = String::from_utf8(files[path].clone())
        .unwrap()
        .replace(", range_mode: Surface", "");
    files.insert(path.into(), text.into_bytes());
    // Tests isolate one model and retain all other content as shapes.
    for name in ["base", "barracks", "worker", "trees", "swordsman"] {
        let p = format!("visuals/{name}.ron");
        let text = String::from_utf8(files[&p].clone()).unwrap();
        if let Some(start) = text.find(", model:") {
            files.insert(p, format!("{})\n", &text[..start]).into_bytes());
        }
    }
    let mut assets: turts_mod_api::AssetsDef =
        ron::de::from_bytes(&files["assets/assets.ron"]).unwrap();
    assets.models = vec![turts_mod_api::FileAsset {
        id: "test".into(),
        path: "assets/test.glb".into(),
    }];
    files.insert(
        "assets/assets.ron".into(),
        ron::ser::to_string(&assets).unwrap().into_bytes(),
    );
    files.insert("visuals/soldier.ron".into(),b"(target: Unit(\"skirmish:soldier\"),shape: Capsule(radius: 0.4,height: 1.6),model: Some((asset: \"skirmish:test\", team_material: Some(\"team\"), scale: 1.0)))".to_vec());
    files
}
fn validate(
    doc: &Value,
    blob: &[u8],
    model_edit: Option<(&str, &str)>,
) -> Result<Vec<String>, turts_mod_api::ModError> {
    let mut files = files();
    files.insert("assets/test.glb".into(), bytes(doc, blob));
    if let Some((a, b)) = model_edit {
        let p = "visuals/soldier.ron";
        let s = String::from_utf8(files[p].clone()).unwrap().replace(a, b);
        files.insert(p.into(), s.into_bytes());
    }
    let archive = PackageArchive::parse(&pack_files(&files).unwrap()).unwrap();
    validate_models(&archive)
}
#[test]
fn md4_refuses_invalid_geometry_and_material_packages() {
    for case in [
        "uri",
        "image_uri",
        "required_extension",
        "blend",
        "materials",
        "joints",
        "triangles",
        "image_size",
        "image_type",
    ] {
        let (mut doc, mut blob) = sample();
        let expected = match case {
            "uri" => {
                doc["buffers"][0]["uri"] = json!("https://invalid/model.bin");
                "external uri"
            }
            "image_uri" => {
                doc["images"] = json!([{"uri":"https://invalid/image.png"}]);
                "external uri"
            }
            "required_extension" => {
                doc["extensionsRequired"] = json!(["UNKNOWN_extension"]);
                "UNKNOWN_extension"
            }
            "blend" => {
                doc["materials"][0]["alphaMode"] = json!("BLEND");
                "BLEND"
            }
            "materials" => {
                doc["materials"] = json!(
                    (0..9)
                        .map(|i| json!({"name":format!("mat{i}")}))
                        .collect::<Vec<_>>()
                );
                "8 materials"
            }
            "joints" => {
                let mut nodes = vec![json!({"mesh":0})];
                nodes.extend((0..129).map(|_| json!({})));
                doc["nodes"] = json!(nodes);
                doc["skins"] = json!([{"joints":(1..130).collect::<Vec<_>>()}]);
                "128 joints"
            }
            "triangles" => {
                doc["accessors"][0]["count"] = json!(300003);
                "100000"
            }
            "image_size" | "image_type" => {
                use image::ImageEncoder;
                let mut image = Vec::new();
                image::codecs::png::PngEncoder::new(&mut image)
                    .write_image(
                        &vec![255; 4097 * 4],
                        4097,
                        1,
                        image::ExtendedColorType::Rgba8,
                    )
                    .unwrap();
                let offset = blob.len();
                blob.extend(image);
                doc["bufferViews"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"buffer":0,"byteOffset":offset,"byteLength":blob.len()-offset}));
                doc["buffers"][0]["byteLength"] = json!(blob.len());
                doc["images"] = json!([{"bufferView":1,"mimeType":if case=="image_type"{"image/ktx2"}else{"image/png"}}]);
                if case == "image_size" {
                    "4096 px"
                } else {
                    "PNG and JPEG"
                }
            }
            _ => unreachable!(),
        };
        let error = validate(&doc, &blob, None).unwrap_err().to_string();
        assert!(error.contains(expected), "MD4 {case}: {error}");
        assert!(
            error.contains("skirmish/assets/test.glb"),
            "MD4 error location: {error}"
        );
    }
}
#[test]
fn md4_refuses_names_fit_pivot_scale_and_lod_contract_errors() {
    for (field, value, expected) in [
        (
            "team_material: Some(\"team\")",
            "team_material: Some(\"tem\")",
            "did you mean `team`",
        ),
        ("scale: 1.0", "scale: 0.1", "expected 0.5 to 1.5"),
        ("scale: 1.0", "scale: 4.0", "expected 0.5 to 1.5"),
        ("scale: 1.0", "scale: -1.0", "finite and positive"),
        (
            "scale: 1.0",
            "turret_node: Some(\"typo\"), scale: 1.0",
            "names: body",
        ),
        (
            "scale: 1.0",
            "muzzle_node: Some(\"typo\"), scale: 1.0",
            "names: body",
        ),
        (
            "scale: 1.0",
            "lods: [(asset: \"skirmish:test\", below_screen_height: 0.1),(asset: \"skirmish:test\", below_screen_height: 0.2)], scale: 1.0",
            "strictly decrease",
        ),
    ] {
        let (doc, blob) = sample();
        let error = validate(&doc, &blob, Some((field, value)))
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "MD4: {error}");
    }
    let (mut doc, mut blob) = sample();
    for offset in [4, 16, 28] {
        blob[offset..offset + 4].copy_from_slice(&1_f32.to_le_bytes());
    }
    doc["accessors"][0]["min"] = json!([-0.4, 1., 0.]);
    doc["accessors"][0]["max"] = json!([0.4, 1., 0.4]);
    assert!(
        validate(&doc, &blob, None)
            .unwrap_err()
            .to_string()
            .contains("ground"),
        "MD4 pivot"
    );
}
#[test]
fn md4_refuses_version_and_file_size() {
    let (doc, blob) = sample();
    let mut glb = bytes(&doc, &blob);
    glb[4..8].copy_from_slice(&1_u32.to_le_bytes());
    assert!(
        inspect_glb("bad", "bad.glb", &glb)
            .unwrap_err()
            .to_string()
            .contains("glTF 2.0")
    );
    let glb = vec![0; 32 * 1024 * 1024 + 1];
    assert!(
        inspect_glb("bad", "bad.glb", &glb)
            .unwrap_err()
            .to_string()
            .contains("32 MiB")
    );
}
#[test]
fn md4_accepts_skin_and_animation_without_a_format_change() {
    let (mut doc, mut blob) = sample();
    let mut accessor = |values: &[f32], kind: &str, count: usize| {
        let offset = blob.len();
        for v in values {
            blob.extend(v.to_le_bytes());
        }
        let index = doc["accessors"].as_array().unwrap().len();
        let view = doc["bufferViews"].as_array().unwrap().len();
        doc["bufferViews"]
            .as_array_mut()
            .unwrap()
            .push(json!({"buffer":0,"byteOffset":offset,"byteLength":blob.len()-offset}));
        doc["accessors"]
            .as_array_mut()
            .unwrap()
            .push(json!({"bufferView":view,"componentType":5126,"count":count,"type":kind}));
        index
    };
    let times = accessor(&[0., 1.], "SCALAR", 2);
    let translations = accessor(&[0., 0., 0., 0., 0., 0.], "VEC3", 2);
    let weights = accessor(&[1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0.], "VEC4", 3);
    let joints = doc["accessors"].as_array().unwrap().len();
    let view = doc["bufferViews"].as_array().unwrap().len();
    let offset = blob.len();
    blob.extend([0_u8; 24]);
    doc["bufferViews"]
        .as_array_mut()
        .unwrap()
        .push(json!({"buffer":0,"byteOffset":offset,"byteLength":24}));
    doc["accessors"]
        .as_array_mut()
        .unwrap()
        .push(json!({"bufferView":view,"componentType":5123,"count":3,"type":"VEC4"}));
    doc["meshes"][0]["primitives"][0]["attributes"]["JOINTS_0"] = json!(joints);
    doc["meshes"][0]["primitives"][0]["attributes"]["WEIGHTS_0"] = json!(weights);
    doc["accessors"][times]["min"] = json!([0.0]);
    doc["accessors"][times]["max"] = json!([1.0]);
    doc["scenes"][0]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!(1));

    doc["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"joint"}));
    doc["skins"] = json!([{"joints":[1]}]);
    doc["nodes"][0]["skin"] = json!(0);
    doc["animations"] = json!([{"name":"Idle","samplers":[{"input":times,"output":translations}],"channels":[{"sampler":0,"target":{"node":1,"path":"translation"}}]}]);
    doc["buffers"][0]["byteLength"] = json!(blob.len());
    validate(&doc, &blob, None).expect("MD4 valid animated model packs");
}

#[test]
fn md4_refuses_a_lod_with_more_triangles() {
    let (doc, blob) = sample();
    let mut larger = doc.clone();
    let primitive = larger["meshes"][0]["primitives"][0].clone();
    larger["meshes"][0]["primitives"]
        .as_array_mut()
        .unwrap()
        .push(primitive);
    let mut files = files();
    files.insert("assets/test.glb".into(), bytes(&doc, &blob));
    files.insert("assets/more.glb".into(), bytes(&larger, &blob));
    let mut assets: turts_mod_api::AssetsDef =
        ron::de::from_bytes(&files["assets/assets.ron"]).unwrap();
    assets.models.push(turts_mod_api::FileAsset {
        id: "more".into(),
        path: "assets/more.glb".into(),
    });
    files.insert(
        "assets/assets.ron".into(),
        ron::ser::to_string(&assets).unwrap().into_bytes(),
    );
    let visual = String::from_utf8(files["visuals/soldier.ron"].clone())
        .unwrap()
        .replace(
            "scale: 1.0",
            "scale: 1.0, lods: [(asset: \"skirmish:more\", below_screen_height: 0.1)]",
        );
    files.insert("visuals/soldier.ron".into(), visual.into_bytes());
    let archive = PackageArchive::parse(&pack_files(&files).unwrap()).unwrap();
    assert!(
        validate_models(&archive)
            .unwrap_err()
            .to_string()
            .contains("more triangles"),
        "MD4: increasing LOD geometry must be refused"
    );
}
#[test]
fn md4_refuses_total_model_bytes_above_512_mib() {
    let (mut doc, mut blob) = sample();
    blob.resize(24 * 1024 * 1024, 0);
    doc["buffers"][0]["byteLength"] = json!(blob.len());
    let model = bytes(&doc, &blob);
    let mut files = files();
    let mut assets: turts_mod_api::AssetsDef =
        ron::de::from_bytes(&files["assets/assets.ron"]).unwrap();
    assets.models.clear();
    for i in 0..22 {
        let name = if i == 0 {
            "test".to_owned()
        } else {
            format!("m{i}")
        };
        let path = format!("assets/{name}.glb");
        files.insert(path.clone(), model.clone());
        assets
            .models
            .push(turts_mod_api::FileAsset { id: name, path });
    }
    files.insert(
        "assets/assets.ron".into(),
        ron::ser::to_string(&assets).unwrap().into_bytes(),
    );
    let archive = PackageArchive::parse(&pack_files(&files).unwrap()).unwrap();
    let error = validate_models(&archive).unwrap_err().to_string();
    assert!(error.contains("512 MiB"), "MD4 total model limit: {error}");
}
#[test]
fn md4_accepts_jpeg_and_reports_non_power_of_two_images_and_cameras() {
    use image::ImageEncoder;
    for jpeg in [false, true] {
        let (mut doc, mut blob) = sample();
        let mut image = Vec::new();
        if jpeg {
            image::codecs::jpeg::JpegEncoder::new(&mut image)
                .write_image(&[150; 3 * 5 * 3], 3, 5, image::ExtendedColorType::Rgb8)
                .unwrap();
        } else {
            image::codecs::png::PngEncoder::new(&mut image)
                .write_image(&[150; 3 * 5 * 4], 3, 5, image::ExtendedColorType::Rgba8)
                .unwrap();
        }
        let offset = blob.len();
        blob.extend(image);
        doc["bufferViews"]
            .as_array_mut()
            .unwrap()
            .push(json!({"buffer":0,"byteOffset":offset,"byteLength":blob.len()-offset}));
        doc["buffers"][0]["byteLength"] = json!(blob.len());
        doc["images"] = json!([{"bufferView":1,"mimeType":if jpeg{"image/jpeg"}else{"image/png"}}]);
        doc["cameras"] = json!([{"type":"perspective","perspective":{"yfov":0.7,"znear":0.1}}]);
        let warnings = validate(&doc, &blob, None).unwrap();
        assert!(warnings.iter().any(|w| w.contains("not a power of two")));
        assert!(
            warnings
                .iter()
                .any(|w| w.contains("cameras and lights are ignored"))
        );
    }
}
#[test]
fn model_fields_require_api_0_2_5_and_absent_fields_keep_old_defaults() {
    let visual:turts_mod_api::VisualDef=ron::from_str("(target: Unit(\"x:y\"),shape: Capsule(radius: 0.4,height: 1.6),model: Some((asset: \"x:model\")))").unwrap();
    let model = visual.model.unwrap();
    assert_eq!(model.scale, 1.0);
    assert!(model.lods.is_empty() && model.animations.is_none());
    let (doc, blob) = sample();
    let mut files = files();
    files.insert("assets/test.glb".into(), bytes(&doc, &blob));
    let manifest = String::from_utf8(files["manifest.ron"].clone())
        .unwrap()
        .replace("^0.2.6", "^0.2.4");
    files.insert("manifest.ron".into(), manifest.into_bytes());
    let archive = PackageArchive::parse(&pack_files(&files).unwrap()).unwrap();
    let error = archive.visuals().unwrap_err().to_string();
    assert!(
        error.contains("model.scale") && error.contains("0.2.5"),
        "new field version refusal: {error}"
    );
}

#[test]
fn an8_clip_names_are_checked_and_legacy_names_still_parse() {
    let old: turts_mod_api::ModelAnimations =
        ron::from_str(r#"(idle:Some("Idle"),move:Some("Walking_A"))"#).unwrap();
    assert_eq!(old.idle.unwrap().clip(), "Idle");
    let rich: turts_mod_api::ModelAnimations = ron::from_str(
        r#"(idle:Some((clip:"Idle")),move:Some((clip:"Walking_A",stride:Some(0.5))))"#,
    )
    .unwrap();
    assert_eq!(rich.r#move.unwrap().settings().unwrap().stride, Some(0.5));
    let mut files = files();
    let (doc, blob) = sample();
    files.insert("assets/test.glb".into(), bytes(&doc, &blob));
    let visual = String::from_utf8(files["visuals/soldier.ron"].clone())
        .unwrap()
        .replace(
            "scale: 1.0",
            r#"scale: 1.0, animations: Some((idle:Some((clip:"Missing"))))"#,
        );
    files.insert("visuals/soldier.ron".into(), visual.into_bytes());
    let archive = PackageArchive::parse(&pack_files(&files).unwrap()).unwrap();
    let error = validate_models(&archive).unwrap_err().to_string();
    assert!(
        error.contains("Missing") && error.contains("names:"),
        "AN8: {error}"
    );
}

#[test]
fn animated_skirmish_idle_pose_keeps_the_original_footprint_fit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
    let bytes = std::fs::read(root.join("assets/models/soldier_anim.glb")).unwrap();
    let info = inspect_glb("skirmish", "soldier_anim.glb", &bytes).unwrap();
    let visual: turts_mod_api::VisualDef =
        ron::from_str(&std::fs::read_to_string(root.join("visuals/soldier.ron")).unwrap()).unwrap();
    let model = visual.model.unwrap();
    let (min, max) = turts_mod_runtime::models::visual_bounds(&info, &model);
    assert!(
        min[0]
            .abs()
            .max(max[0].abs())
            .hypot(min[2].abs().max(max[2].abs()))
            * model.scale
            <= 0.55 * 1.5
    );
    assert!((min[1] * model.scale).abs() <= 0.05, "ground pivot {min:?}");
}

#[test]
fn unused_cycles_and_multiple_parent_nodes_are_refused_before_rig_loading() {
    let (mut doc, blob) = sample();
    doc["nodes"].as_array_mut().unwrap().extend([
        json!({"name":"unused_a","children":[2]}),
        json!({"name":"unused_b","children":[1]}),
    ]);
    let error = inspect_glb("test", "cycle.glb", &bytes(&doc, &blob))
        .unwrap_err()
        .to_string();
    assert!(error.contains("cycle"), "{error}");
    let (mut doc, blob) = sample();
    doc["nodes"][0]["children"] = json!([1]);
    doc["nodes"].as_array_mut().unwrap().extend([
        json!({"name":"shared"}),
        json!({"name":"other","children":[1]}),
    ]);
    let error = inspect_glb("test", "shared.glb", &bytes(&doc, &blob))
        .unwrap_err()
        .to_string();
    assert!(error.contains("multiple parents"), "{error}");
}
