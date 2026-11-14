//! Public package acceptance, including raw version presence and both validation paths.
mod common;
use turts_mod_api::{GroundOverlayDef, GroundPatchDef, MapDef, MapSize, WorldPos};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_files};
fn layer() -> GroundOverlayDef {
    GroundOverlayDef {
        colour: "#9B8057".into(),
        base_colour: None,
        tile_size: 8.0,
        roughness: 1.0,
        patches: vec![GroundPatchDef {
            centre: WorldPos { x: 10.0, y: 24.0 },
            size: MapSize { x: 14.0, y: 14.0 },
            rotation_deg: 39.3,
            feather: 3.0,
            opacity: 0.8,
        }],
    }
}
fn archive(layer: Option<GroundOverlayDef>, api: &str) -> PackageArchive {
    let mut files = common::read_files(&common::package_dir("skirmish"));
    let text = String::from_utf8(files["manifest.ron"].clone()).unwrap();
    let manifest: turts_mod_api::Manifest = ron::from_str(&text).unwrap();
    files.insert(
        "manifest.ron".into(),
        text.replace(&manifest.engine_api, api).into_bytes(),
    );
    let mut map: MapDef =
        ron::from_str(std::str::from_utf8(&files["maps/field.ron"]).unwrap()).unwrap();
    map.ground.as_mut().unwrap().overlay = layer;
    files.insert(
        "maps/field.ron".into(),
        ron::ser::to_string(&map).unwrap().into_bytes(),
    );
    PackageArchive::parse(&pack_files(&files).unwrap()).unwrap()
}
#[test]
fn absent_layer_is_legacy_and_written_layer_requires_its_release() {
    let old = archive(None, "^0.2.10");
    old.content().unwrap();
    turts_mod_runtime::models::validate_models(&old).unwrap();
    assert!(
        old.maps().unwrap()[0]
            .1
            .ground
            .as_ref()
            .unwrap()
            .overlay
            .is_none()
    );
    let new = archive(Some(layer()), "^0.2.11");
    new.content().unwrap();
    turts_mod_runtime::models::validate_models(&new).unwrap();
    let error = archive(Some(layer()), "^0.2.10")
        .content()
        .unwrap_err()
        .to_string();
    assert!(error.contains("needs engine api 0.2.11"), "{error}");
}
#[test]
fn bad_layer_values_fail_content_and_model_validation_with_their_field() {
    let mut cases = Vec::new();
    for colour in ["blue", "#12345g", "#12345678"] {
        let mut l = layer();
        l.colour = colour.into();
        cases.push((l, "colour"));
    }
    for n in [0.0, -1.0, f32::NAN, f32::INFINITY, 1e-38] {
        let mut l = layer();
        l.tile_size = n;
        cases.push((l, "tile_size"));
    }
    for n in [-0.1, 1.1, f32::NAN, f32::INFINITY] {
        let mut l = layer();
        l.roughness = n;
        cases.push((l, "roughness"));
        let mut l = layer();
        l.patches[0].opacity = n;
        cases.push((l, "opacity"));
    }
    for n in [0.0, -1.0, 10001.0, f32::NAN, f32::INFINITY] {
        let mut l = layer();
        l.patches[0].size.x = n;
        cases.push((l, "size"));
    }
    for n in [-10001.0, 10001.0, f32::NAN, f32::INFINITY] {
        let mut l = layer();
        l.patches[0].centre.y = n;
        cases.push((l, "centre"));
    }
    for n in [0.0, -1.0, 7.1, f32::NAN, f32::INFINITY] {
        let mut l = layer();
        l.patches[0].feather = n;
        cases.push((l, "feather"));
    }
    for n in [-361.0, 361.0, f32::NAN, f32::INFINITY] {
        let mut l = layer();
        l.patches[0].rotation_deg = n;
        cases.push((l, "rotation_deg"));
    }
    let mut l = layer();
    l.patches = vec![l.patches[0].clone(); 17];
    cases.push((l, "patches"));
    let mut l = layer();
    l.base_colour = Some("skirmish:missing".into());
    cases.push((l, "ground"));
    for (l, field) in cases {
        let a = archive(Some(l), "^0.2.11");
        a.maps().unwrap();
        let content = a.content().unwrap_err().to_string();
        assert!(content.contains(field), "{field}: {content}");
        let models = turts_mod_runtime::models::validate_models(&a)
            .unwrap_err()
            .to_string();
        assert!(models.contains(field), "{field}: {models}");
    }
}
#[test]
fn optional_image_and_maximum_patch_count_are_accepted() {
    let mut l = layer();
    l.base_colour = Some("skirmish:ground_grass".into());
    l.patches = vec![l.patches[0].clone(); 16];
    l.patches[0].rotation_deg = -360.0;
    l.patches[0].opacity = 0.0;
    l.patches[1].rotation_deg = 360.0;
    let a = archive(Some(l), "^0.2.11");
    a.content().unwrap();
    turts_mod_runtime::models::validate_models(&a).unwrap();
}
