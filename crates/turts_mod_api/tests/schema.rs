//! Schema tests for the public mod API types (design 4.4, Appendix A).

use turts_mod_api::{
    ContentId, ExecutionContext, Footprint, LockEntry, Manifest, PackageId, PackageKind, Shape,
    UnitDef, VisualDef,
};

/// Appendix A `manifest.ron`, verbatim.
const ACME_MANIFEST: &str = r#"(
    id: "acme",
    version: "0.1.0",
    kind: Game,
    engine_api: "^0.1",
    dependencies: {},
    exports: (
        units: ["units/scout.ron"],
        maps: ["maps/field.ron"],
        visuals: ["visuals/scout.ron"],
    ),
    context: Shared,
    capabilities: [],
    license: "LicenseRef-TBD",
)"#;

#[test]
fn manifest_round_trips_from_design_text() {
    let m: Manifest = ron::from_str(ACME_MANIFEST).expect("Appendix A manifest parses");
    assert_eq!(m.id, "acme");
    assert_eq!(m.version, "0.1.0");
    assert_eq!(m.kind, PackageKind::Game);
    assert_eq!(m.engine_api, "^0.1");
    assert!(m.dependencies.is_empty());
    assert_eq!(m.exports.units, vec!["units/scout.ron"]);
    assert_eq!(m.exports.maps, vec!["maps/field.ron"]);
    assert_eq!(m.exports.visuals, vec!["visuals/scout.ron"]);
    assert_eq!(m.context, ExecutionContext::Shared);
    assert!(m.capabilities.is_empty());
    assert_eq!(m.license, "LicenseRef-TBD");
    // Serialise and parse again: the value must survive the round trip.
    let text = ron::to_string(&m).expect("manifest serialises");
    let again: Manifest = ron::from_str(&text).expect("serialised manifest parses");
    assert_eq!(m, again);
}

#[test]
fn manifest_rejects_unknown_field() {
    let text = ACME_MANIFEST.replacen(
        "    id: \"acme\",",
        "    id: \"acme\",\n    hash: \"x\",",
        1,
    );
    assert_ne!(
        text, ACME_MANIFEST,
        "the test must change the manifest text"
    );
    let err = ron::from_str::<Manifest>(&text).expect_err("unknown field must be rejected");
    assert!(
        err.to_string().contains("hash"),
        "error must name the field: {err}"
    );
}

#[test]
fn unit_def_parses_named_footprint() {
    let u: UnitDef = ron::from_str(
        r#"(id: "scout", name: "Scout", speed: 6.0, footprint: Circle(radius: 0.6))"#,
    )
    .expect("unit def parses");
    assert_eq!(u.id, "scout");
    assert_eq!(u.name, "Scout");
    assert_eq!(u.speed, 6.0);
    assert_eq!(u.footprint, Footprint::Circle { radius: 0.6 });
}

#[test]
fn visual_def_parses_both_shapes() {
    let capsule: VisualDef =
        ron::from_str(r#"(unit: "base:rifleman", shape: Capsule(radius: 0.3, height: 1.8))"#)
            .expect("capsule parses");
    assert_eq!(capsule.unit, "base:rifleman");
    assert_eq!(
        capsule.shape,
        Shape::Capsule {
            radius: 0.3,
            height: 1.8
        }
    );

    let cuboid: VisualDef =
        ron::from_str(r#"(unit: "acme:scout", shape: Cuboid(x: 1.0, y: 0.8, z: 1.4))"#)
            .expect("cuboid parses");
    assert_eq!(
        cuboid.shape,
        Shape::Cuboid {
            x: 1.0,
            y: 0.8,
            z: 1.4
        }
    );

    // Named fields only: the unnamed form is rejected.
    let unnamed =
        ron::from_str::<VisualDef>(r#"(unit: "acme:scout", shape: Cuboid(1.0, 0.8, 1.4))"#);
    assert!(unnamed.is_err(), "unnamed Cuboid fields must be rejected");
}

#[test]
fn content_id_parse_and_display() {
    let id = ContentId::parse("acme:scout").expect("valid content id");
    assert_eq!(id.package, PackageId("acme".to_string()));
    assert_eq!(id.local, "scout");
    assert_eq!(id.to_string(), "acme:scout");
    assert_eq!(ContentId::new(&id.package, "scout"), id);
    assert!(ContentId::parse("a:b:c").is_err());
    assert!(ContentId::parse("scout").is_err());
    assert!(ContentId::parse(":scout").is_err());
    assert!(ContentId::parse("acme:").is_err());
}

#[test]
fn package_id_rejects_bad_chars() {
    assert!(PackageId::parse("Acme").is_err());
    assert!(PackageId::parse("ac me").is_err());
    assert!(PackageId::parse("").is_err());
    assert_eq!(
        PackageId::parse("acme_2"),
        Ok(PackageId("acme_2".to_string()))
    );
}

#[test]
fn lock_entry_json_has_seven_fields() {
    let entry = LockEntry {
        id: "acme".to_string(),
        version: "0.1.0".to_string(),
        hash: format!("sha256:{}", "0".repeat(64)),
        size: 2048,
        urls: vec!["http://localhost:8080/packages/acme.tar".to_string()],
        context: ExecutionContext::Shared,
        engine_api: "^0.1".to_string(),
    };
    let value = serde_json::to_value(&entry).expect("entry serialises");
    assert_eq!(value.as_object().expect("entry is a JSON object").len(), 7);
    let back: LockEntry = serde_json::from_value(value).expect("entry deserialises");
    assert_eq!(back, entry);
}
