//! Environment data must be versioned and validated before a client accepts it.
mod common;
use turts_mod_api::{Antialiasing, DistanceFogDef, EnvironmentDef, MapDef};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_files};

fn environment() -> EnvironmentDef {
    EnvironmentDef {
        background: "#B8D4DF".into(),
        ground_margin: 64.0,
        boundary_colour: Some("#536A46".into()),
        fog: Some(DistanceFogDef {
            colour: "#B8D4DF".into(),
            start: 80.0,
            end: 220.0,
            opacity: 0.2,
        }),
        antialiasing: Antialiasing::Fxaa,
    }
}
fn archive(environment: Option<EnvironmentDef>, api: &str) -> PackageArchive {
    let mut files = common::read_files(&common::package_dir("skirmish"));
    let manifest_text = String::from_utf8(files["manifest.ron"].clone()).unwrap();
    let manifest: turts_mod_api::Manifest = ron::from_str(&manifest_text).unwrap();
    files.insert(
        "manifest.ron".into(),
        manifest_text
            .replace(&manifest.engine_api, api)
            .into_bytes(),
    );
    let mut map: MapDef =
        ron::from_str(std::str::from_utf8(&files["maps/field.ron"]).unwrap()).unwrap();
    map.environment = environment;
    files.insert(
        "maps/field.ron".into(),
        ron::ser::to_string(&map).unwrap().into_bytes(),
    );
    PackageArchive::parse(&pack_files(&files).unwrap()).unwrap()
}

#[test]
fn legacy_maps_keep_absent_environment_and_new_fields_require_their_release() {
    let old = archive(None, "^0.2.9");
    old.content().unwrap();
    assert!(old.maps().unwrap()[0].1.environment.is_none());
    archive(Some(environment()), "^0.2.10").content().unwrap();
    let error = archive(Some(environment()), "^0.2.9")
        .content()
        .err()
        .unwrap()
        .to_string();
    assert!(error.contains("needs engine api 0.2.10"), "{error}");
}

#[test]
fn invalid_environment_values_are_rejected_with_their_field() {
    let mut cases = Vec::new();
    for colour in ["bad", "#12345g", "#12345678"] {
        let mut value = environment();
        value.background = colour.into();
        cases.push((value, "environment.background"));
    }
    let mut value = environment();
    value.boundary_colour = Some("blue".into());
    cases.push((value, "environment.boundary_colour"));
    let mut value = environment();
    value.fog.as_mut().unwrap().colour = "#ABC".into();
    cases.push((value, "environment.fog.colour"));
    for margin in [-1.0, 10001.0, f32::NAN, f32::INFINITY] {
        let mut value = environment();
        value.ground_margin = margin;
        cases.push((value, "environment.ground_margin"));
    }
    for start in [-1.0, f32::NAN, f32::INFINITY] {
        let mut value = environment();
        value.fog.as_mut().unwrap().start = start;
        cases.push((value, "environment.fog.start"));
    }
    for end in [0.0, 80.0, 10001.0, f32::NAN, f32::INFINITY] {
        let mut value = environment();
        value.fog.as_mut().unwrap().end = end;
        cases.push((value, "environment.fog.end"));
    }
    for opacity in [-0.1, 1.1, f32::NAN, f32::INFINITY] {
        let mut value = environment();
        value.fog.as_mut().unwrap().opacity = opacity;
        cases.push((value, "environment.fog.opacity"));
    }
    for (value, field) in cases {
        let package = archive(Some(value), "^0.2.10");
        package.maps().unwrap(); // Exercise validation after successful typed parsing.
        let error = package
            .content()
            .expect_err("invalid environment accepted")
            .to_string();
        assert!(error.contains(field), "{field}: {error}");
    }
}

#[test]
fn accepted_boundary_values_and_optional_effects_are_portable() {
    let mut value = environment();
    value.ground_margin = 10000.0;
    let fog = value.fog.as_mut().unwrap();
    fog.start = 0.0;
    fog.end = 10000.0;
    fog.opacity = 1.0;
    archive(Some(value.clone()), "^0.2.10").content().unwrap();
    value.ground_margin = 0.0;
    value.fog.as_mut().unwrap().opacity = 0.0;
    archive(Some(value.clone()), "^0.2.10").content().unwrap();
    value.fog = None;
    value.boundary_colour = None;
    value.antialiasing = Antialiasing::Off;
    archive(Some(value), "^0.2.10").content().unwrap();
}
