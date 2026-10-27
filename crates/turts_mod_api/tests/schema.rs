use turts_mod_api::schema::{DropOffEffect, Exports, MapDef, UnitDef};

fn exports_without(field: &str) -> String {
    let fields = [
        ("units", "units: []"),
        ("maps", "maps: []"),
        ("visuals", "visuals: []"),
        ("resources", "resources: []"),
        ("nodes", "nodes: []"),
        ("production", "production: []"),
    ];
    let included = fields
        .iter()
        .filter(|(name, _)| *name != field)
        .map(|(_, field)| *field)
        .collect::<Vec<_>>()
        .join(", ");
    format!("({included})")
}

fn worker_without(field: &str) -> String {
    let fields = [
        ("movement", "movement: None"),
        ("inventory", "inventory: None"),
        ("gatherer", "gatherer: None"),
        ("drop_off", "drop_off: None"),
        ("producer", "producer: None"),
    ];
    let capabilities = fields
        .iter()
        .filter(|(name, _)| *name != field)
        .map(|(_, field)| *field)
        .collect::<Vec<_>>()
        .join(", ");
    format!("(id: \"worker\", name: \"Carrier\", footprint: Circle(radius: 0.5), {capabilities})")
}

fn duplicate_balance_seed() -> Result<MapDef, ron::error::SpannedError> {
    ron::from_str(
        r#"(id: "clearing", size: (x: 64.0, y: 64.0), placements: [], nodes: [], slot_economy: [(slot: 0, balances: [(balance: "cedar:credits", amount: 10), (balance: "cedar:credits", amount: 20)])])"#,
    )
}

fn duplicate_balance_seed_across_slot_records() -> Result<MapDef, ron::error::SpannedError> {
    ron::from_str(
        r#"(id: "clearing", size: (x: 64.0, y: 64.0), placements: [], nodes: [], slot_economy: [(slot: 0, balances: [(balance: "cedar:credits", amount: 10)]), (slot: 0, balances: [(balance: "cedar:credits", amount: 20)])])"#,
    )
}

#[test]
fn all_six_export_lists_and_explicit_options_are_required() {
    for field in [
        "units",
        "maps",
        "visuals",
        "resources",
        "nodes",
        "production",
    ] {
        assert!(
            ron::from_str::<Exports>(&exports_without(field)).is_err(),
            "omitting export list {field} must fail"
        );
    }

    for field in ["movement", "inventory", "gatherer", "drop_off", "producer"] {
        assert!(
            ron::from_str::<UnitDef>(&worker_without(field)).is_err(),
            "omitting capability {field} must fail"
        );
    }
    assert!(ron::from_str::<UnitDef>(&worker_without("not_a_field")).is_ok());

    let structured = r#"(
        id: "worker", name: "Carrier", footprint: Circle(radius: 0.5),
        movement: None, inventory: Some((capacity: 2, accepts: [{package: "cedar", local: "wood"}])),
        gatherer: None, drop_off: None, producer: None,
    )"#;
    assert!(ron::from_str::<UnitDef>(structured).is_err());
}

#[test]
fn drop_off_convert_rejects_unknown_fields() {
    let effect = r#"Convert(balance: "cedar:credits", per_item: 1, typo: 99)"#;
    assert!(ron::from_str::<DropOffEffect>(effect).is_err());
}

#[test]
fn duplicate_balance_seed_rejects_repeated_balance_entries() {
    assert!(duplicate_balance_seed().is_err());
}

#[test]
fn duplicate_balance_seed_rejects_repeated_slot_records() {
    assert!(duplicate_balance_seed_across_slot_records().is_err());
}
