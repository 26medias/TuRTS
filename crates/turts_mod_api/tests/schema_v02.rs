use turts_mod_api::schema_v02::{Exports, UnitDef};

#[test]
fn all_six_export_lists_and_explicit_options_are_required() {
    let missing_resources = r#"(units: [], maps: [], visuals: [], nodes: [], production: [])"#;
    assert!(ron::from_str::<Exports>(missing_resources).is_err());

    let worker = r#"(
        id: "worker", name: "Carrier", footprint: Circle(radius: 0.5),
        movement: Some((speed: 3.0)), inventory: None,
        gatherer: None, producer: None,
    )"#;
    assert!(ron::from_str::<UnitDef>(worker).is_err());

    let worker = r#"(
        id: "worker", name: "Carrier", footprint: Circle(radius: 0.5),
        movement: Some((speed: 3.0)), inventory: None,
        gatherer: None, drop_off: None, producer: None,
    )"#;
    assert!(ron::from_str::<UnitDef>(worker).is_ok());

    let structured = r#"(
        id: "worker", name: "Carrier", footprint: Circle(radius: 0.5),
        movement: None, inventory: Some((capacity: 2, accepts: [{package: "cedar", local: "wood"}])),
        gatherer: None, drop_off: None, producer: None,
    )"#;
    assert!(ron::from_str::<UnitDef>(structured).is_err());
}
