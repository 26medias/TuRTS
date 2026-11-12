//! VAL1 (plan `L0-API2`; design 6.10, the validation and error-text columns): a red package for
//! each check of the 0.2.4 table, and the exact text of its row.
//!
//! Two kinds of package:
//! - the package `val1` (`tests/fixtures/val1/base`), a complete 0.2.4 package that passes
//!   `check_api_0_2_4`. Each case edits ONE value of it and expects the text of its row. The checks
//!   are reached through `PackageArchive::check_api_0_2_4`, because `content` refuses a feature
//!   that no lane has built with the `UNBUILT_FEATURES` text first (`unbuilt.rs`); the control
//!   test shows that `content` does refuse the unedited package for that reason and no other;
//! - the repository package `cedar` (it uses no 0.2.4 feature), for the checks that no
//!   `UNBUILT_FEATURES` line holds back: they run through `content`, the path of the packer. The
//!   edition rows T2 to T4 and the rows H1 to H3, H17 and KEY1 are here, as a `^0.2.3` and
//!   `^0.2.4` pair.
//!
//! A case that passes on a tree with no check is a test that cannot fail: the control tests
//! show that the unedited packages pass, and each case is red when its check is removed (the plants
//! of the task, `docs/research/mp1/plants/l0-api2.json`).

use std::collections::BTreeMap;
use std::path::Path;

mod common;

use common::{package_dir, read_files};
use turts_mod_api::keys::{CURSOR_CONTEXTS, REASON_KEYS, STATE_KEYS};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_files};

// ---- The package `val1` ----

/// An edit of one file of the package: the exact `old` text occurs once.
type Edit<'a> = (&'a str, &'a str, &'a str);

fn base_files() -> BTreeMap<String, Vec<u8>> {
    let mut files =
        read_files(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/val1/base"));
    // The icons of the orders are the images of `cedar`.
    for (path, bytes) in read_files(&package_dir("cedar")) {
        if path.starts_with("assets/") {
            files.insert(path, bytes);
        }
    }
    files
}

fn apply(files: &mut BTreeMap<String, Vec<u8>>, edits: &[Edit<'_>]) {
    for (file, old, new) in edits {
        let text = String::from_utf8(files[*file].clone()).unwrap();
        assert_eq!(
            text.matches(old).count(),
            1,
            "the edit target `{old}` must occur once in {file}"
        );
        files.insert((*file).to_string(), text.replacen(old, new, 1).into_bytes());
    }
}

fn archive(files: &BTreeMap<String, Vec<u8>>) -> PackageArchive {
    PackageArchive::parse(&pack_files(files).expect("pack in memory")).expect("package parses")
}

fn val1(edits: &[Edit<'_>], adds: &[(&str, &str)]) -> PackageArchive {
    let mut files = base_files();
    for (file, text) in adds {
        files.insert((*file).to_string(), text.as_bytes().to_vec());
    }
    apply(&mut files, edits);
    archive(&files)
}

/// The text of the refusal of the validation column, or a panic when the package passes.
fn refusal(row: &str, edits: &[Edit<'_>], adds: &[(&str, &str)]) -> String {
    match val1(edits, adds).check_api_0_2_4() {
        Ok(()) => panic!("{row}: the package passes, a refusal was expected"),
        Err(error) => error.to_string(),
    }
}

/// One red package of the package `val1`: `(row, edits, expected text)`.
fn reds(cases: &[(&str, &[Edit<'_>], &str)]) {
    for (row, edits, expected) in cases {
        assert_eq!(refusal(row, edits, &[]), *expected, "row {row}");
    }
}

fn allowed(names: &[&str]) -> String {
    let mut sorted = names.to_vec();
    sorted.sort_unstable();
    sorted.join(", ")
}

const UNITS_ALLOWED: &str = "val1:command_center, val1:engineer, val1:guard_turret, val1:hauler, val1:power_station, val1:tank";
const ORDERS_ALLOWED: &str = "val1:order_cancel_construction, val1:order_construct, val1:order_gather, val1:order_move, val1:order_rally, val1:order_repair, val1:order_surrender";

// ---- Controls ----

/// The unedited package passes the validation column. Every case below changes one value of it, so
/// a refusal is the refusal of that value.
#[test]
fn the_package_val1_passes_the_validation_column() {
    val1(&[], &[])
        .check_api_0_2_4()
        .unwrap_or_else(|error| panic!("{error}"));
}

/// `content` refuses the unedited package for the first `UNBUILT_FEATURES` line and for no other
/// reason: the old validation, the version table and the new column all passed before it.
#[test]
fn content_refuses_the_package_val1_only_as_unbuilt() {
    let error = val1(&[], &[]).content().unwrap_err();
    assert_eq!(
        error.to_string(),
        "val1/manifest.ron: exports.combat: `exports.combat` is not built in this engine build; leave it out until an engine build supports it"
    );
}

/// An edition older than 0.2.4 keeps its behaviour: the column does not run for it (rows T1 to T4).
#[test]
fn the_column_does_not_run_for_the_edition_0_2_3() {
    let package = val1(
        &[
            (
                "manifest.ron",
                r#"engine_api: "^0.2.4""#,
                r#"engine_api: "^0.2.3""#,
            ),
            ("units/tank.ron", "damage: 8,", "damage: 0,"),
        ],
        &[],
    );
    package
        .check_api_0_2_4()
        .expect("the edition 0.2.3 is not checked");
}

// ---- A: exports ----

#[test]
fn a1_a2_the_export_groups() {
    let damage = "combat/damage.ron";
    let other = String::from_utf8(base_files()[damage].clone()).unwrap();
    let second_match = String::from_utf8(base_files()["match/match.ron"].clone()).unwrap();
    assert_eq!(
        refusal(
            "A1 two damage tables",
            &[(
                "manifest.ron",
                r#"combat: ["combat/damage.ron"],"#,
                r#"combat: ["combat/damage.ron", "combat/other.ron"],"#
            )],
            &[("combat/other.ron", &other)],
        ),
        "val1/manifest.ron: exports.combat[1]: a match has one damage table; `combat/damage.ron` already declares it"
    );
    assert_eq!(
        refusal(
            "A2 two match files",
            &[(
                "manifest.ron",
                r#"match: ["match/match.ron"],"#,
                r#"match: ["match/match.ron", "match/second.ron"],"#
            )],
            &[("match/second.ron", &second_match)],
        ),
        "val1/manifest.ron: exports.match: at most one file, found 2"
    );
}

// ---- C: the damage table ----

#[test]
fn c1_c3_the_damage_table() {
    let cell = |from: &'static str, to: &'static str| -> [Edit<'static>; 1] {
        [("combat/damage.ron", from, to)]
    };
    reds(&[
        (
            "C1 empty damage_types",
            &[(
                "combat/damage.ron",
                r#"damage_types: ["bullet", "rocket"],"#,
                "damage_types: [],",
            )],
            "val1/combat/damage.ron: damage_types: needs at least 1 entry",
        ),
        (
            "C1 duplicate damage type",
            &[(
                "combat/damage.ron",
                r#"damage_types: ["bullet", "rocket"],"#,
                r#"damage_types: ["rocket", "rocket"],"#,
            )],
            "val1/combat/damage.ron: damage_types[1]: duplicate damage type `rocket`",
        ),
        (
            "C2 duplicate armour class",
            &[(
                "combat/damage.ron",
                r#"armour_classes: ["light", "vehicle", "structure"],"#,
                r#"armour_classes: ["light", "light"],"#,
            )],
            "val1/combat/damage.ron: armour_classes[1]: duplicate armour class `light`",
        ),
        (
            "C3 unknown damage type",
            &cell(
                r#"(damage: "val1:rocket", armour: "val1:light","#,
                r#"(damage: "val1:rockt", armour: "val1:light","#,
            ),
            "val1/combat/damage.ron: cells[3].damage: unknown damage type `val1:rockt`; did you mean `val1:rocket`? (allowed: val1:bullet, val1:rocket)",
        ),
        (
            "C3 second cell for a pair",
            &cell(
                r#"(damage: "val1:rocket", armour: "val1:structure","#,
                r#"(damage: "val1:rocket", armour: "val1:vehicle","#,
            ),
            "val1/combat/damage.ron: cells[5]: second cell for `val1:rocket` and `val1:vehicle`",
        ),
        (
            "C3 no cell for a pair",
            &cell(
                "        (damage: \"val1:rocket\", armour: \"val1:structure\", ratio: (numerator: 1, denominator: 1)),\n",
                "",
            ),
            "val1/combat/damage.ron: cells: no cell for damage `val1:rocket` and armour `val1:structure`",
        ),
        (
            "C3 ratio denominator 0",
            &cell(
                r#"armour: "val1:light", ratio: (numerator: 1, denominator: 1)"#,
                r#"armour: "val1:light", ratio: (numerator: 1, denominator: 0)"#,
            ),
            "val1/combat/damage.ron: cells[0].ratio: the denominator must be above 0",
        ),
    ]);
}

// ---- U, S1: units and resources ----

#[test]
fn u1_u2_u3_armour_and_weapon_fields() {
    reds(&[
        (
            "U1 a unit with health needs an armour class",
            &[(
                "units/tank.ron",
                r#"armour: Some((class: "val1:vehicle", flat: 1)),"#,
                "armour: None,",
            )],
            "val1/units/tank.ron: armour: a unit with `health` needs an armour class, because the package declares a damage table",
        ),
        (
            "U1 unknown armour class",
            &[(
                "units/tank.ron",
                r#"class: "val1:vehicle""#,
                r#"class: "val1:vehicl""#,
            )],
            "val1/units/tank.ron: armour.class: unknown armour class `val1:vehicl`; did you mean `val1:vehicle`? (allowed: val1:light, val1:structure, val1:vehicle)",
        ),
        (
            "U2 unknown damage type",
            &[(
                "units/tank.ron",
                r#"damage_type: Some("val1:rocket")"#,
                r#"damage_type: Some("val1:rockt")"#,
            )],
            "val1/units/tank.ron: weapon.damage_type: unknown damage type `val1:rockt`; did you mean `val1:rocket`? (allowed: val1:bullet, val1:rocket)",
        ),
        (
            "U2 a weapon needs a damage type",
            &[(
                "units/tank.ron",
                r#", damage_type: Some("val1:rocket")"#,
                "",
            )],
            "val1/units/tank.ron: weapon.damage_type: a weapon needs a damage type, because the package declares a damage table",
        ),
        (
            "U3 weapon damage 0",
            &[("units/tank.ron", "damage: 8,", "damage: 0,")],
            "val1/units/tank.ron: weapon.damage: must be positive",
        ),
    ]);
}

#[test]
fn u4_acquire() {
    reds(&[
        (
            "U4 range below the weapon range",
            &[(
                "units/tank.ron",
                "acquire: Some((range: 6.0, leash: 4.0))",
                "acquire: Some((range: 4.0, leash: 4.0))",
            )],
            "val1/units/tank.ron: acquire.range: 4.0 is below the weapon range 5.0",
        ),
        (
            "U4 negative leash",
            &[("units/tank.ron", "leash: 4.0", "leash: -1.0")],
            "val1/units/tank.ron: acquire.leash: must be finite and at least 0, found -1.0",
        ),
        (
            "U4 needs a weapon",
            &[(
                "units/hauler.ron",
                "spawn_order:",
                "acquire: Some((range: 6.0, leash: 4.0)), spawn_order:",
            )],
            "val1/units/hauler.ron: acquire: needs `weapon`",
        ),
    ]);
}

#[test]
fn u5_provides_and_consumes() {
    let provides = r#"provides: [(resource: "val1:power", amount: 50)],"#;
    reds(&[
        (
            "U5 not a Supply resource",
            &[(
                "units/power_station.ron",
                provides,
                r#"provides: [(resource: "val1:credits", amount: 50)],"#,
            )],
            "val1/units/power_station.ron: provides[0].resource: `val1:credits` is not a `Supply` resource",
        ),
        (
            "U5 amount 0",
            &[(
                "units/power_station.ron",
                provides,
                r#"provides: [(resource: "val1:power", amount: 0)],"#,
            )],
            "val1/units/power_station.ron: provides[0].amount: must be positive",
        ),
        (
            "U5 second entry for a resource",
            &[(
                "units/power_station.ron",
                provides,
                r#"provides: [(resource: "val1:power", amount: 5), (resource: "val1:power", amount: 5)],"#,
            )],
            "val1/units/power_station.ron: provides[1].resource: second entry for `val1:power`",
        ),
        (
            "U5 unknown resource",
            &[(
                "units/guard_turret.ron",
                r#"consumes: [(resource: "val1:power", amount: 10)]"#,
                r#"consumes: [(resource: "val1:powr", amount: 10)]"#,
            )],
            "val1/units/guard_turret.ron: consumes[0].resource: unknown resource `val1:powr`; did you mean `val1:power`? (allowed: val1:credits, val1:ore, val1:power)",
        ),
    ]);
}

#[test]
fn u6_when_short() {
    let turret = r#"when_short: [(resource: "val1:power", effect: Disable)]"#;
    let centre =
        r#"when_short: [(resource: "val1:power", effect: Rate((numerator: 1, denominator: 2)))]"#;
    reds(&[
        (
            "U6 Rate on a weapon",
            &[(
                "units/guard_turret.ron",
                turret,
                r#"when_short: [(resource: "val1:power", effect: Rate((numerator: 1, denominator: 2)))]"#,
            )],
            "val1/units/guard_turret.ron: weapon.when_short[0].effect: `Rate` applies to `producer` and `constructor` only",
        ),
        (
            "U6 BlockStart is reserved",
            &[(
                "units/command_center.ron",
                centre,
                r#"when_short: [(resource: "val1:power", effect: BlockStart)]"#,
            )],
            "val1/units/command_center.ron: producer.when_short[0].effect: `BlockStart` is reserved for a later engine API",
        ),
        (
            "U6 Disable on a producer",
            &[(
                "units/command_center.ron",
                centre,
                r#"when_short: [(resource: "val1:power", effect: Disable)]"#,
            )],
            "val1/units/command_center.ron: producer.when_short[0].effect: `Disable` applies to `weapon` only",
        ),
        (
            "U6 Rate numerator 0",
            &[(
                "units/command_center.ron",
                centre,
                r#"when_short: [(resource: "val1:power", effect: Rate((numerator: 0, denominator: 2)))]"#,
            )],
            "val1/units/command_center.ron: producer.when_short[0].effect: `Rate` needs 0 < numerator <= denominator",
        ),
        (
            "U6 not a Supply resource",
            &[(
                "units/command_center.ron",
                centre,
                r#"when_short: [(resource: "val1:credits", effect: Rate((numerator: 1, denominator: 2)))]"#,
            )],
            "val1/units/command_center.ron: producer.when_short[0].resource: `val1:credits` is not a `Supply` resource",
        ),
    ]);
}

/// Design 6.4: the packer refuses a `Q` (the least common multiple of every `Rate` denominator)
/// above 65,536. 300 and 301 have no common factor, so the second entry makes 90,300.
#[test]
fn u6_q_above_65536() {
    reds(&[(
        "U6 Q above 65,536",
        &[
            (
                "units/command_center.ron",
                "denominator: 2)))]",
                "denominator: 300)))]",
            ),
            (
                "units/command_center.ron",
                "concurrent_sites: 2, queue_capacity: 4)",
                r#"concurrent_sites: 2, queue_capacity: 4, when_short: [(resource: "val1:power", effect: Rate((numerator: 1, denominator: 301)))])"#,
            ),
        ],
        "val1/units/command_center.ron: constructor.when_short[0].effect: the least common multiple of the `Rate` denominators is 90300 here; the maximum is 65536",
    )]);
    // Control: 300 and 300 make 300.
    let package = val1(
        &[
            (
                "units/command_center.ron",
                "denominator: 2)))]",
                "denominator: 300)))]",
            ),
            (
                "units/command_center.ron",
                "concurrent_sites: 2, queue_capacity: 4)",
                r#"concurrent_sites: 2, queue_capacity: 4, when_short: [(resource: "val1:power", effect: Rate((numerator: 1, denominator: 300)))])"#,
            ),
        ],
        &[],
    );
    package.check_api_0_2_4().expect("Q is 300");
}

#[test]
fn s1_supply_resource() {
    reds(&[
        (
            "S1 counts Queued is reserved",
            &[("resources/power.ron", "counts: Completed", "counts: Queued")],
            "val1/resources/power.ron: kind.counts: `Queued` is reserved for a later engine API",
        ),
        (
            "S1 a Supply resource has no balance",
            &[(
                "maps/field.ron",
                r#"(slot: 0, balances: [(balance: "val1:credits", amount: 300)]),"#,
                r#"(slot: 0, balances: [(balance: "val1:credits", amount: 300), (balance: "val1:power", amount: 5)]),"#,
            )],
            "val1/maps/field.ron: slot_economy[0].balances[1].balance: `val1:power` is a `Supply` resource; it has no balance",
        ),
    ]);
}

#[test]
fn u7_income() {
    let income = r#"income: Some((resource: "val1:credits", amount: 3, period_ms: 1000)),"#;
    reds(&[
        (
            "U7 not a Balance resource",
            &[(
                "units/command_center.ron",
                income,
                r#"income: Some((resource: "val1:ore", amount: 3, period_ms: 1000)),"#,
            )],
            "val1/units/command_center.ron: income.resource: `val1:ore` is not a `Balance` resource",
        ),
        (
            "U7 amount 0",
            &[(
                "units/command_center.ron",
                income,
                r#"income: Some((resource: "val1:credits", amount: 0, period_ms: 1000)),"#,
            )],
            "val1/units/command_center.ron: income.amount: must be positive",
        ),
        (
            "U7 period 0",
            &[(
                "units/command_center.ron",
                income,
                r#"income: Some((resource: "val1:credits", amount: 3, period_ms: 0)),"#,
            )],
            "val1/units/command_center.ron: income.period_ms: must be positive",
        ),
        (
            "U7 the unit has health",
            &[(
                "units/command_center.ron",
                "    health: Some((max: 500)), removal: Some((by_command: false, contents: Destroy)),\n",
                "",
            )],
            "val1/units/command_center.ron: income: a unit with `income` needs `health`",
        ),
    ]);
}

#[test]
fn u8_m3_start_orders() {
    reds(&[
        (
            "U8 a unit type takes NearestNode only",
            &[("units/hauler.ron", "target: NearestNode", "target: Node(3)")],
            "val1/units/hauler.ron: spawn_order.target: a unit type takes `NearestNode` only, found `Node(3)`",
        ),
        (
            "U8 the order does not list the unit",
            &[(
                "orders/order_gather.ron",
                r#"units: ["val1:hauler"]"#,
                r#"units: ["val1:tank"]"#,
            )],
            "val1/units/hauler.ron: spawn_order.order: `val1:order_gather` does not list `val1:hauler`",
        ),
        (
            "U8 unknown order",
            &[(
                "units/hauler.ron",
                r#"order: "val1:order_gather""#,
                r#"order: "val1:order_gathr""#,
            )],
            &format!(
                "val1/units/hauler.ron: spawn_order.order: unknown order `val1:order_gathr`; did you mean `val1:order_gather`? (allowed: {ORDERS_ALLOWED})"
            ),
        ),
        (
            "U8 the target kind does not match the order",
            &[(
                "units/hauler.ron",
                r#"order: "val1:order_gather""#,
                r#"order: "val1:order_move""#,
            )],
            "val1/units/hauler.ron: spawn_order.target: `val1:order_move` takes a Point target, not a Node target",
        ),
        (
            "M3 node index outside the map nodes",
            &[("maps/field.ron", "target: Node(0)", "target: Node(40)")],
            "val1/maps/field.ron: placements[1].start_order.target: node index 40 is outside the 1 map nodes",
        ),
        (
            "M3 a map unit has no rally",
            &[(
                "maps/field.ron",
                "target: Node(0)",
                "target: Node(0), ground_rally: ApplyFirst",
            )],
            "val1/maps/field.ron: placements[1].start_order.ground_rally: a map unit has no rally, found `ApplyFirst`",
        ),
    ]);
}

// ---- K, P: construction and payment ----

#[test]
fn k1_constructor() {
    let buildings = r#"buildings: ["val1:build_power_station", "val1:build_guard_turret"]"#;
    reds(&[
        (
            "K1 unknown production",
            &[(
                "units/command_center.ron",
                buildings,
                r#"buildings: ["val1:build_power_statio", "val1:build_guard_turret"]"#,
            )],
            "val1/units/command_center.ron: constructor.buildings[0]: unknown production `val1:build_power_statio`; did you mean `val1:build_power_station`? (allowed: val1:build_guard_turret, val1:build_power_station)",
        ),
        (
            "K1 a production in recipes and buildings",
            &[(
                "units/command_center.ron",
                buildings,
                r#"buildings: ["val1:build_power_station", "val1:train_hauler"]"#,
            )],
            "val1/units/command_center.ron: constructor.buildings[1]: `val1:train_hauler` is also in `producer.recipes` of this unit",
        ),
        (
            "K1 concurrent_sites 0",
            &[(
                "units/command_center.ron",
                "concurrent_sites: 2",
                "concurrent_sites: 0",
            )],
            "val1/units/command_center.ron: constructor.concurrent_sites: must be at least 1",
        ),
        (
            "K1 queue_capacity 0",
            &[(
                "units/command_center.ron",
                "queue_capacity: 4)",
                "queue_capacity: 0)",
            )],
            "val1/units/command_center.ron: constructor.queue_capacity: must be at least 1",
        ),
        (
            "K1 NeedsBuilder is reserved",
            &[(
                "units/command_center.ron",
                "queue_capacity: 4)",
                "queue_capacity: 4, progress: NeedsBuilder(reach: 2.0))",
            )],
            "val1/units/command_center.ron: constructor.progress: `NeedsBuilder` is reserved for a later engine API",
        ),
        (
            "K1 beside any other work capability",
            &[(
                "units/command_center.ron",
                "build_area: Some(12.0),",
                r#"machine: Some((recipe: "val1:ammo")), build_area: Some(12.0),"#,
            )],
            "val1/units/command_center.ron: machine: a unit may have at most one work capability; `constructor` is allowed only beside `producer`",
        ),
    ]);
}

#[test]
fn k2_k3_placement_and_build_area() {
    let station = r#"placement: Some((gap: 2.2, in_build_area: true, site_health: (numerator: 1, denominator: 10))),"#;
    reds(&[
        (
            "K2 gap below the largest mobile diameter plus 0.2",
            &[("units/power_station.ron", "gap: 2.2", "gap: 0.5")],
            "val1/units/power_station.ron: placement.gap: 0.5 is below 2.0, the largest mobile diameter (`val1:tank`, 1.8) plus 0.2",
        ),
        (
            "K2 a unit with placement is static",
            &[(
                "units/tank.ron",
                "acquire:",
                "placement: Some((gap: 3.0)), acquire:",
            )],
            "val1/units/tank.ron: placement: a unit with `placement` must be static",
        ),
        (
            "K2 negative node_clearance",
            &[(
                "units/guard_turret.ron",
                "node_clearance: 1.0",
                "node_clearance: -1.0",
            )],
            "val1/units/guard_turret.ron: placement.node_clearance: must be finite and at least 0, found -1.0",
        ),
        (
            "K2 site_health numerator above the denominator",
            &[(
                "units/power_station.ron",
                station,
                r#"placement: Some((gap: 2.2, in_build_area: true, site_health: (numerator: 11, denominator: 10))),"#,
            )],
            "val1/units/power_station.ron: placement.site_health: the numerator is above the denominator",
        ),
        (
            "K2 site_health denominator 0",
            &[(
                "units/power_station.ron",
                station,
                r#"placement: Some((gap: 2.2, in_build_area: true, site_health: (numerator: 0, denominator: 0))),"#,
            )],
            "val1/units/power_station.ron: placement.site_health: the denominator must be above 0",
        ),
        (
            "K3 build_area 0",
            &[(
                "units/command_center.ron",
                "build_area: Some(12.0)",
                "build_area: Some(0.0)",
            )],
            "val1/units/command_center.ron: build_area: must be finite and above 0, found 0.0",
        ),
        (
            "K3 in_build_area and no unit declares build_area",
            &[(
                "units/command_center.ron",
                "    build_area: Some(12.0),\n",
                "",
            )],
            "val1/units/power_station.ron: placement.in_build_area: no unit declares `build_area`, so no site can be placed",
        ),
    ]);
}

#[test]
fn k4_k5_k6_p1_production_and_match_rows() {
    let technology = r#"(id: "armour", name: "Armour", prerequisites: [], duration_ms: 1000, costs: [], payment: ChargeOverTime, cancel_refunds: (queued: (numerator: 1, denominator: 1), running: (numerator: 1, denominator: 1), output_blocked: (numerator: 1, denominator: 1)), effects: [])"#;
    assert_eq!(
        refusal(
            "P1 ChargeOverTime on a technology",
            &[(
                "manifest.ron",
                r#"combat: ["combat/damage.ron"],"#,
                r#"combat: ["combat/damage.ron"], technologies: ["technologies/armour.ron"],"#
            )],
            &[("technologies/armour.ron", technology)],
        ),
        "val1/technologies/armour.ron: payment: `ChargeOverTime` is not supported for technologies"
    );
    reds(&[
        (
            "K4 a building output needs placement",
            &[(
                "production/build_power_station.ron",
                r#"output_unit: "val1:power_station""#,
                r#"output_unit: "val1:tank""#,
            )],
            "val1/production/build_power_station.ron: output_unit: a building output needs `placement`",
        ),
        (
            "K5 destroyed numerator above the denominator",
            &[(
                "production/build_guard_turret.ron",
                "output_blocked: (numerator: 1, denominator: 1)),",
                "output_blocked: (numerator: 1, denominator: 1), destroyed: (numerator: 3, denominator: 2)),",
            )],
            "val1/production/build_guard_turret.ron: cancel_refunds.destroyed: the numerator is above the denominator",
        ),
        (
            "K6 a package with a constructor needs open_sites_per_slot",
            &[("match/match.ron", "open_sites_per_slot: Some(2), ", "")],
            "val1/match/match.ron: open_sites_per_slot: a package with a constructor needs `open_sites_per_slot`",
        ),
        (
            "K6 open_sites_per_slot 0",
            &[(
                "match/match.ron",
                "open_sites_per_slot: Some(2)",
                "open_sites_per_slot: Some(0)",
            )],
            "val1/match/match.ron: open_sites_per_slot: must be at least 1",
        ),
        (
            "K6 no match file",
            &[("manifest.ron", r#"match: ["match/match.ron"],"#, "")],
            "val1/manifest.ron: exports.match: a package with a constructor needs a match file that sets `open_sites_per_slot`",
        ),
    ]);
}

// ---- O, R, L: orders, repair, relations ----

#[test]
fn o1_to_o10_orders_and_rules() {
    reds(&[
        (
            "O1 a Construct order lists a unit with no constructor",
            &[(
                "orders/order_construct.ron",
                r#"units: ["val1:command_center"]"#,
                r#"units: ["val1:tank"]"#,
            )],
            "val1/orders/order_construct.ron: units[0]: `val1:tank` has no `constructor`",
        ),
        (
            "O2 a CancelConstruction order lists a unit with no placement",
            &[(
                "orders/order_cancel_construction.ron",
                r#"units: ["val1:power_station", "val1:guard_turret"]"#,
                r#"units: ["val1:tank"]"#,
            )],
            "val1/orders/order_cancel_construction.ron: units[0]: `val1:tank` has no `placement`",
        ),
        (
            "O3 a SetRally order lists a unit with no producer",
            &[(
                "orders/order_rally.ron",
                r#"units: ["val1:command_center"]"#,
                r#"units: ["val1:tank"]"#,
            )],
            "val1/orders/order_rally.ron: units[0]: `val1:tank` has no `producer`",
        ),
        (
            "O4 a Repair order lists a unit with no repairer",
            &[(
                "orders/order_repair.ron",
                r#"units: ["val1:engineer"]"#,
                r#"units: ["val1:tank"]"#,
            )],
            "val1/orders/order_repair.ron: units[0]: `val1:tank` has no `repairer`",
        ),
        (
            "O5 ContentAtPoint needs Construct",
            &[(
                "orders/order_move.ron",
                "target: Point",
                "target: ContentAtPoint",
            )],
            "val1/orders/order_move.ron: target: `ContentAtPoint` needs behaviour `Construct`",
        ),
        (
            "O6 OwnDamaged needs a Repair order",
            &[(
                "orders/rules.ron",
                r#"(target: OwnDamaged, needs: Repairer, order: "val1:order_repair")"#,
                r#"(target: OwnDamaged, needs: Repairer, order: "val1:order_move")"#,
            )],
            "val1/orders/rules.ron: smart[3].order: `OwnDamaged` needs a `Repair` order",
        ),
        (
            "O6 Ground with needs Producer needs a SetRally order",
            &[(
                "orders/rules.ron",
                r#"(target: Ground, needs: Producer, order: "val1:order_rally")"#,
                r#"(target: Ground, needs: Producer, order: "val1:order_move")"#,
            )],
            "val1/orders/rules.ron: rally[0].order: `Ground` with needs `Producer` needs a `SetRally` order",
        ),
        (
            "O7 an order of the rally list resolves",
            &[(
                "orders/rules.ron",
                r#"order: "val1:order_rally""#,
                r#"order: "val1:order_rall""#,
            )],
            &format!(
                "val1/orders/rules.ron: rally[0].order: unknown order `val1:order_rall`; did you mean `val1:order_rally`? (allowed: {ORDERS_ALLOWED})"
            ),
        ),
        (
            "O8 hotkey names an input action",
            &[(
                "production/train_hauler.ron",
                r#"hotkey: Some("key_train")"#,
                r#"hotkey: Some("key_trian")"#,
            )],
            "val1/production/train_hauler.ron: hotkey: unknown input action `key_trian`; did you mean `key_train`? (allowed: key_build_power, key_build_turret, key_cancel_site, key_construct, key_gather, key_move, key_rally, key_repair, key_surrender, key_train)",
        ),
        (
            "O9 a Surrender order lists no units",
            &[(
                "orders/order_surrender.ron",
                "units: []",
                r#"units: ["val1:tank"]"#,
            )],
            "val1/orders/order_surrender.ron: units: a `Surrender` order lists no units",
        ),
        (
            "O10 two entries in one card cell",
            &[(
                "production/build_guard_turret.ron",
                "card: Some((row: 0, column: 2))",
                "card: Some((row: 0, column: 1))",
            )],
            "val1/production/build_guard_turret.ron: card: cell (0, 1) is also the cell of `val1:build_power_station`",
        ),
    ]);
}

#[test]
fn r1_repairer() {
    let repairer = r#"repairer: Some((rate_per_second: 20, reach: 1.5, cost: [(resource: "val1:credits", amount: 1, per_health: 4)], targets: (slots: Same), classes: [], auto_range: None)),"#;
    let with =
        |text: &'static str| -> [Edit<'static>; 1] { [("units/engineer.ron", repairer, text)] };
    reds(&[
        (
            "R1 rate_per_second 0",
            &with(
                r#"repairer: Some((rate_per_second: 0, reach: 1.5, cost: [], targets: (slots: Same), classes: [], auto_range: None)),"#,
            ),
            "val1/units/engineer.ron: repairer.rate_per_second: must be positive",
        ),
        (
            "R1 reach 0",
            &with(
                r#"repairer: Some((rate_per_second: 20, reach: 0.0, cost: [], targets: (slots: Same), classes: [], auto_range: None)),"#,
            ),
            "val1/units/engineer.ron: repairer.reach: must be finite and above 0, found 0.0",
        ),
        (
            "R1 per_health 0",
            &with(
                r#"repairer: Some((rate_per_second: 20, reach: 1.5, cost: [(resource: "val1:credits", amount: 1, per_health: 0)], targets: (slots: Same), classes: [], auto_range: None)),"#,
            ),
            "val1/units/engineer.ron: repairer.cost[0].per_health: must be positive",
        ),
        (
            "R1 a cost resource is a Balance resource",
            &with(
                r#"repairer: Some((rate_per_second: 20, reach: 1.5, cost: [(resource: "val1:ore", amount: 1, per_health: 4)], targets: (slots: Same), classes: [], auto_range: None)),"#,
            ),
            "val1/units/engineer.ron: repairer.cost[0].resource: `val1:ore` is not a `Balance` resource",
        ),
        (
            "R1 classes resolve",
            &with(
                r#"repairer: Some((rate_per_second: 20, reach: 1.5, cost: [], targets: (slots: Same), classes: ["val1:ligt"], auto_range: None)),"#,
            ),
            "val1/units/engineer.ron: repairer.classes[0]: unknown armour class `val1:ligt`; did you mean `val1:light`? (allowed: val1:light, val1:structure, val1:vehicle)",
        ),
        (
            "R1 auto_range at least the reach",
            &with(
                r#"repairer: Some((rate_per_second: 20, reach: 1.5, cost: [], targets: (slots: Same), classes: [], auto_range: Some(1.0))),"#,
            ),
            "val1/units/engineer.ron: repairer.auto_range: must be finite and at least the reach 1.5, found 1.0",
        ),
    ]);
}

// ---- M: the match file ----

#[test]
fn m1_m2_h15_the_match_file() {
    let defeat = r#"defeat: [KeyUnitsLost(units: ["val1:command_center"])]"#;
    let victory =
        |text: &'static str| -> [Edit<'static>; 1] { [("match/match.ron", defeat, text)] };
    reds(&[
        (
            "M2 countdown_secs 0 to 30",
            &[(
                "match/match.ron",
                "AllSeated(countdown_secs: 0)",
                "AllSeated(countdown_secs: 45)",
            )],
            "val1/match/match.ron: start.countdown_secs: must be 0 to 30, found 45",
        ),
        (
            "H15 one different colour for each player slot",
            &[(
                "interface/hud.ron",
                r##"slot_colours: ["#56B4E9", "#D55E00"],"##,
                r##"slot_colours: ["#56B4E9"],"##,
            )],
            "val1/match/match.ron: start: `AllSeated` needs 2 different slot colours in the interface, found 1",
        ),
        (
            "M1 KeyUnitsLost needs at least one unit",
            &victory("defeat: [KeyUnitsLost(units: [])]"),
            "val1/match/match.ron: victory.defeat[0]: `KeyUnitsLost` needs at least one unit",
        ),
        (
            "M1 an unknown unit",
            &victory(r#"defeat: [KeyUnitsLost(units: ["val1:comand_center"])]"#),
            &format!(
                "val1/match/match.ron: victory.defeat[0].units[0]: unknown unit `val1:comand_center`; did you mean `val1:command_center`? (allowed: {UNITS_ALLOWED})"
            ),
        ),
        (
            "M1 a unit with health",
            &victory(r#"defeat: [KeyUnitsLost(units: ["val1:hauler"])]"#),
            "val1/match/match.ron: victory.defeat[0].units[0]: `val1:hauler` has no `health`",
        ),
        (
            "M1 DisconnectedFor secs above 0",
            &victory(
                r#"defeat: [KeyUnitsLost(units: ["val1:command_center"]), DisconnectedFor(secs: 0)]"#,
            ),
            "val1/match/match.ron: victory.defeat[1].secs: `DisconnectedFor` secs must be above 0",
        ),
        (
            "M1 DisconnectedFor at most once",
            &victory(r#"defeat: [DisconnectedFor(secs: 5), DisconnectedFor(secs: 6)]"#),
            "val1/match/match.ron: victory.defeat: `DisconnectedFor` is written 2 times; at most 1",
        ),
    ]);
}

// ---- H: the interface ----

#[test]
fn h1_h2_h3_the_key_lists() {
    assert_eq!(REASON_KEYS.len(), 49, "design 6.10: 37 + 3 + 9 keys");
    assert_eq!(
        STATE_KEYS.len(),
        23,
        "the 22 of design 6.10 and `gather.waiting.blocked`"
    );
    reds(&[
        (
            "H1 an unknown reason key",
            &[(
                "interface/hud.ron",
                r#"reasons: {"queue_full": "The queue is full"}"#,
                r#"reasons: {"queue_ful": "The queue is full"}"#,
            )],
            &format!(
                "val1/interface/hud.ron: reasons.queue_ful: unknown reason key `queue_ful`; did you mean `queue_full`? (allowed: {})",
                allowed(REASON_KEYS)
            ),
        ),
        (
            "H2 an unknown cursor context",
            &[(
                "interface/hud.ron",
                "cursors: {},",
                r#"cursors: {"plcae": "c"},"#,
            )],
            "val1/interface/hud.ron: cursors.plcae: unknown cursor context `plcae`; did you mean `place`? (allowed: attack, default, forbidden, gather, move, place, rally, repair, select)",
        ),
        (
            "H3 an unknown state key",
            &[(
                "interface/hud.ron",
                r#"states: {"slowed": "Slowed"}"#,
                r#"states: {"slowd": "Slowed"}"#,
            )],
            &format!(
                "val1/interface/hud.ron: states.slowd: unknown state key `slowd`; did you mean `slowed`? (allowed: {})",
                allowed(STATE_KEYS)
            ),
        ),
    ]);
    assert_eq!(CURSOR_CONTEXTS.len(), 9);
}

#[test]
fn h4_h5_h6_h17_h21_h22_alerts() {
    let short = r#"trigger: SupplyShort("val1:power"), kind: State, clear_after: Some(8.0)"#;
    let alert = |text: &'static str| -> [Edit<'static>; 1] { [("interface/hud.ron", short, text)] };
    reds(&[
        (
            "H4 SupplyShort names a Supply resource",
            &alert(r#"trigger: SupplyShort("val1:credits"), kind: State, clear_after: Some(8.0)"#),
            "val1/interface/hud.ron: alerts[0].trigger: `val1:credits` is not a `Supply` resource",
        ),
        (
            "H4 InsufficientFunds names a Balance or Item resource",
            &alert(
                r#"trigger: InsufficientFunds("val1:power"), kind: State, clear_after: Some(8.0)"#,
            ),
            "val1/interface/hud.ron: alerts[0].trigger: `val1:power` is not a `Balance` or `Item` resource",
        ),
        (
            "H4 GathererIdle seconds above 0",
            &alert("trigger: GathererIdle(0.0), kind: State, clear_after: Some(8.0)"),
            "val1/interface/hud.ron: alerts[0].trigger: waiting duration must be finite and greater than 0 seconds",
        ),
        (
            "H5 alerts.units resolve",
            &alert(
                r#"trigger: SupplyShort("val1:power"), kind: State, clear_after: Some(8.0), units: ["val1:tnak"]"#,
            ),
            &format!(
                "val1/interface/hud.ron: alerts[0].units[0]: unknown unit `val1:tnak`; did you mean `val1:tank`? (allowed: {UNITS_ALLOWED})"
            ),
        ),
        (
            "H6 State needs a state trigger",
            &[(
                "interface/hud.ron",
                "trigger: ProductionComplete,",
                "trigger: ProductionComplete, kind: State,",
            )],
            "val1/interface/hud.ron: alerts[1].kind: `State` needs a state trigger; `ProductionComplete` is an event",
        ),
        (
            "H6 clear_after needs State",
            &[(
                "interface/hud.ron",
                "trigger: ProductionComplete,",
                "trigger: ProductionComplete, clear_after: Some(3.0),",
            )],
            "val1/interface/hud.ron: alerts[1].clear_after: needs `kind: State`",
        ),
        (
            "H6 clear_after at least 0",
            &alert(r#"trigger: SupplyShort("val1:power"), kind: State, clear_after: Some(-1.0)"#),
            "val1/interface/hud.ron: alerts[0].clear_after: must be finite and at least 0, found -1.0",
        ),
        (
            "H17 an unknown placeholder in an alert text",
            &[(
                "interface/hud.ron",
                "Low power: {unit} is",
                "Low power: {units} is",
            )],
            "val1/interface/hud.ron: alerts[0].text: unknown placeholder `{units}`; did you mean `{unit}`? (allowed: {slot}, {unit})",
        ),
        (
            "H21 QueueEmpty needs ProductionComplete",
            &alert(
                r#"trigger: SupplyShort("val1:power"), kind: State, clear_after: Some(8.0), sound_when: QueueEmpty"#,
            ),
            "val1/interface/hud.ron: alerts[0].sound_when: `QueueEmpty` needs the trigger `ProductionComplete`",
        ),
        (
            "H22 focus resolves",
            &alert(
                r#"trigger: SupplyShort("val1:power"), kind: State, clear_after: Some(8.0), focus: Some(["val1:tnak"])"#,
            ),
            &format!(
                "val1/interface/hud.ron: alerts[0].focus[0]: unknown unit `val1:tnak`; did you mean `val1:tank`? (allowed: {UNITS_ALLOWED})"
            ),
        ),
    ]);
}

#[test]
fn h7_h10_h13_widgets() {
    let title = r##"widget: Text(text: Literal("Val1"), font: None, size: 14, colour: "#ffffff", align: Left, wrap: false)"##;
    let text = |value: &'static str| -> [Edit<'static>; 1] {
        [("interface/hud.ron", r#"Literal("Val1")"#, value)]
    };
    reds(&[
        (
            "H7 Supply names a Supply resource",
            &text(r#"Bind(Supply("val1:credits", Provided))"#),
            "val1/interface/hud.ron: root[title].widget.text: `val1:credits` is not a `Supply` resource",
        ),
        (
            "H7 Gathered names a Balance resource",
            &text(r#"Bind(MatchStats(Gathered("val1:power")))"#),
            "val1/interface/hud.ron: root[title].widget.text: `val1:power` is not a `Balance` resource",
        ),
        (
            "H17 an unknown placeholder in a Format text",
            &text(r#"Format("{slot} {1x}", [])"#),
            "val1/interface/hud.ron: root[title].widget.text: unknown placeholder `{1x}` (allowed: {slot})",
        ),
        (
            "H10 SupplyShort names a Supply resource",
            &[(
                "interface/hud.ron",
                "size: (w: Px(200.0), h: Px(28.0)),",
                r#"size: (w: Px(200.0), h: Px(28.0)), visible: [SupplyShort("val1:credits")],"#,
            )],
            "val1/interface/hud.ron: root[title].visible: `val1:credits` is not a `Supply` resource",
        ),
        (
            "H13 a Button size 8 to 64",
            &[(
                "interface/hud.ron",
                title,
                r#"widget: Button(icon: None, label: Some(Literal("x")), action: Input("menu"), hotkey_label: false, size: Some(70))"#,
            )],
            "val1/interface/hud.ron: root[title].widget.size: 70 is outside 8 to 64",
        ),
        (
            "H13 a Button font exists",
            &[(
                "interface/hud.ron",
                title,
                r#"widget: Button(icon: None, label: Some(Literal("x")), action: Input("menu"), hotkey_label: false, font: Some("nope"))"#,
            )],
            "val1/interface/hud.ron: root[title].widget.font: unknown font asset `nope`",
        ),
    ]);
}

#[test]
fn h11_h24_world_styles() {
    let placement = r##"placement: Some((valid: "#56B4E9", invalid: "#D55E00", stripes: true)),"##;
    let labels = r##"unit_labels: Some((show: Always, own_only: false, colour: "#ffffff", font: None, size: 14))"##;
    reds(&[
        (
            "H11 a colour is #RRGGBB",
            &[(
                "interface/hud.ron",
                placement,
                r##"placement: Some((valid: "#56B4E9", invalid: "#D55E0", stripes: true)),"##,
            )],
            "val1/interface/hud.ron: world.placement.invalid: `#D55E0` is not a #RRGGBB colour",
        ),
        (
            "H11 a ring colour",
            &[(
                "interface/hud.ron",
                placement,
                r##"build_area_ring: Some((colour: "red", width: 2.0)),"##,
            )],
            "val1/interface/hud.ron: world.build_area_ring.colour: `red` is not a #RRGGBB colour",
        ),
        (
            "H11 a ring width above 0",
            &[(
                "interface/hud.ron",
                placement,
                r##"gap_ring: Some((colour: "#ffffff", width: 0.0)),"##,
            )],
            "val1/interface/hud.ron: world.gap_ring.width: must be finite and above 0, found 0.0",
        ),
        (
            "H11 a bar has a size",
            &[(
                "interface/hud.ron",
                placement,
                r##"progress_bar: Some((width: 0, height: 4, background: "#000000", fill: "#ffffff")),"##,
            )],
            "val1/interface/hud.ron: world.progress_bar: width and height must be greater than zero",
        ),
        (
            "H11 a float_text font exists",
            &[(
                "interface/hud.ron",
                placement,
                r##"float_text: Some((colour: "#ffffff", font: Some("nope"), size: 14)),"##,
            )],
            "val1/interface/hud.ron: world.float_text.font: unknown font asset `nope`",
        ),
        (
            "H11 a float_text sound exists",
            &[(
                "interface/hud.ron",
                placement,
                r##"float_text: Some((colour: "#ffffff", size: 14, sound: Some("snd"))),"##,
            )],
            "val1/interface/hud.ron: world.float_text.sound: unknown sound asset `snd`",
        ),
        (
            "H24 unit_labels size 8 to 32",
            &[(
                "interface/hud.ron",
                labels,
                r##"unit_labels: Some((show: Always, own_only: false, colour: "#ffffff", font: None, size: 40))"##,
            )],
            "val1/interface/hud.ron: world.unit_labels.size: 40 is outside 8 to 32",
        ),
        (
            "H24 unit_labels colour",
            &[(
                "interface/hud.ron",
                labels,
                r##"unit_labels: Some((show: Selected, own_only: true, colour: "white", font: None, size: 14))"##,
            )],
            "val1/interface/hud.ron: world.unit_labels.colour: `white` is not a #RRGGBB colour",
        ),
    ]);
}

#[test]
fn h14_hints() {
    reds(&[
        (
            "H14 hint ids are unique",
            &[("interface/hud.ron", r#"id: "h_wait""#, r#"id: "h_build""#)],
            "val1/interface/hud.ron: hints[1].id: duplicate hint id `h_build`",
        ),
        (
            "H14 HintDone names a hint",
            &[(
                "interface/hud.ron",
                r#"done: HintDone("h_build")"#,
                r#"done: HintDone("h_buil")"#,
            )],
            "val1/interface/hud.ron: hints[1].done: `HintDone(h_buil)` names no hint; did you mean `h_build`? (allowed: h_build, h_wait)",
        ),
        (
            "H14 StepCurrent names a step",
            &[(
                "interface/hud.ron",
                "starts: MatchStarted",
                r#"starts: StepCurrent("h_wait")"#,
            )],
            "val1/interface/hud.ron: hints[0].starts: `StepCurrent(h_wait)` names a situation, not a step",
        ),
        (
            "H14 secs above 0",
            &[(
                "interface/hud.ron",
                "JobOnHold(secs: 2.0)",
                "JobOnHold(secs: 0.0)",
            )],
            "val1/interface/hud.ron: hints[1].starts: `JobOnHold` secs must be finite and above 0, found 0.0",
        ),
        (
            "H14 references resolve",
            &[(
                "interface/hud.ron",
                r#"units: ["val1:power_station"]"#,
                r#"units: ["val1:power_statio"]"#,
            )],
            &format!(
                "val1/interface/hud.ron: hints[0].done.units[0]: unknown unit `val1:power_statio`; did you mean `val1:power_station`? (allowed: {UNITS_ALLOWED})"
            ),
        ),
    ]);
}

#[test]
fn h16_h18_h20_interface_fields() {
    let states = r#"states: {"slowed": "Slowed"},"#;
    reds(&[
        (
            "H16 effects_voices 1 to 64",
            &[(
                "interface/hud.ron",
                states,
                r#"states: {"slowed": "Slowed"}, effects_voices: Some(70),"#,
            )],
            "val1/interface/hud.ron: effects_voices: must be 1 to 64",
        ),
        (
            "H16 audible_range above 0",
            &[(
                "interface/hud.ron",
                states,
                r#"states: {"slowed": "Slowed"}, audible_range: Some(0.0),"#,
            )],
            "val1/interface/hud.ron: audible_range: must be finite and above 0, found 0.0",
        ),
        (
            "H20 alert_sound_gap_secs at least 0",
            &[(
                "interface/hud.ron",
                states,
                r#"states: {"slowed": "Slowed"}, alert_sound_gap_secs: Some(-1.0),"#,
            )],
            "val1/interface/hud.ron: alert_sound_gap_secs: must be finite and at least 0, found -1.0",
        ),
        (
            "H18 start_selection resolves",
            &[(
                "interface/hud.ron",
                r#"start_selection: Some("val1:command_center")"#,
                r#"start_selection: Some("val1:comand_center")"#,
            )],
            &format!(
                "val1/interface/hud.ron: start_selection: unknown unit `val1:comand_center`; did you mean `val1:command_center`? (allowed: {UNITS_ALLOWED})"
            ),
        ),
    ]);
}

// ---- V3, V4: the effect files and the effects of a visual ----

#[test]
fn v3_v4_effects() {
    let effects = r##"(effects: [(id: "flash", kind: Flash, colour: "#FFD080", size: 0.6, ms: 120, sound: None)])"##;
    let visual = |refs: &str| {
        format!(
            r#"(target: Unit("val1:tank"), shape: Capsule(radius: 0.5, height: 1.2), effects: Some({refs}))"#
        )
    };
    let manifest = [
        (
            "manifest.ron",
            "visuals: [],",
            r#"visuals: ["visuals/tank.ron"],"#,
        ),
        (
            "manifest.ron",
            r#"combat: ["combat/damage.ron"],"#,
            r#"combat: ["combat/damage.ron"], visual_effects: ["fx/effects.ron"],"#,
        ),
    ];
    let run = |effects: &str, refs: &str| {
        let visual = visual(refs);
        val1(
            &manifest,
            &[
                ("fx/effects.ron", effects),
                ("visuals/tank.ron", visual.as_str()),
            ],
        )
        .check_api_0_2_4_presentation()
    };
    run(effects, r#"(fire: Some("val1:flash"))"#).expect("the control passes");
    let cases: [(&str, String, String, &str); 5] = [
        (
            "V3 effect ids are unique",
            effects.replace(
                "])",
                r##", (id: "flash", kind: Spark, colour: "#FFD080", size: 0.6, ms: 120)])"##,
            ),
            r#"(fire: Some("val1:flash"))"#.into(),
            "val1/fx/effects.ron: effects[1].id: duplicate effect id `flash`",
        ),
        (
            "V3 a colour is #RRGGBB",
            effects.replace("#FFD080", "#FFD08"),
            r#"(fire: Some("val1:flash"))"#.into(),
            "val1/fx/effects.ron: effects[0].colour: `#FFD08` is not a #RRGGBB colour",
        ),
        (
            "V3 a sound exists",
            effects.replace("sound: None", r#"sound: Some("fx_shot")"#),
            r#"(fire: Some("val1:flash"))"#.into(),
            "val1/fx/effects.ron: effects[0].sound: unknown sound asset `fx_shot`",
        ),
        (
            "V4 an effect reference resolves",
            effects.into(),
            r#"(fire: Some("val1:flsh"))"#.into(),
            "val1/visuals/tank.ron: effects.fire: unknown effect `val1:flsh`; did you mean `val1:flash`? (allowed: val1:flash)",
        ),
        (
            "V4 damaged.below is a ratio",
            effects.into(),
            r#"(damaged: Some((effect: "val1:flash", below: (numerator: 3, denominator: 2))))"#
                .into(),
            "val1/visuals/tank.ron: effects.damaged.below: the numerator is above the denominator",
        ),
    ];
    for (row, effects, refs, expected) in &cases {
        let error = run(effects, refs).expect_err(row);
        assert_eq!(error.to_string(), *expected, "row {row}");
    }
}

// ---- The package `cedar`: the checks that no UNBUILT line holds back, through `content` ----

/// A row of the edition pairs: the name, the edits, the added files, the text at `^0.2.4`.
type EditionPair<'a> = (&'a str, Vec<Edit<'a>>, Vec<(&'a str, &'a str)>, String);

const CEDAR_MANIFEST: &str = "manifest.ron";
const CEDAR_HUD: &str = "interface/hud.ron";

fn cedar(engine_api: &str, edits: &[Edit<'_>], adds: &[(&str, &str)]) -> PackageArchive {
    let mut files = read_files(&package_dir("cedar"));
    for (file, text) in adds {
        files.insert((*file).to_string(), text.as_bytes().to_vec());
    }
    apply(&mut files, edits);
    apply(
        &mut files,
        &[(
            CEDAR_MANIFEST,
            r#"engine_api: "^0.2.3""#,
            &format!(r#"engine_api: "{engine_api}""#),
        )],
    );
    archive(&files)
}

/// The edition rows T2, T3 and T4 as a pair: the package packs at `^0.2.3` (the behaviour of today)
/// and is refused at `^0.2.4` with the text of its row. The control packs at both.
#[test]
fn t2_t3_t4_edition_pairs_and_the_control() {
    for api in ["^0.2.3", "^0.2.4"] {
        cedar(api, &[], &[])
            .content()
            .unwrap_or_else(|error| panic!("control at {api}: {error}"));
    }
    let turret_manifest: Edit = (
        CEDAR_MANIFEST,
        r#"units: ["units/depot.ron", "units/worker.ron"],"#,
        r#"units: ["units/depot.ron", "units/worker.ron", "units/turret.ron"],"#,
    );
    let turret = r#"(id: "turret", name: "Turret", footprint: Circle(radius: 1.0), movement: None, inventory: None, gatherer: None, drop_off: None, producer: None, health: Some((max: 10)), removal: Some((by_command: false, contents: Destroy)), weapon: Some((range: 5.0, cooldown_ms: 900, damage: 0, ammo: None, targets: (slots: Other))))"#;
    let alert = |text: &str| -> String {
        format!(
            r#"    alerts: [(id: "a", trigger: OwnUnitDamaged, sound: None, text: "{text}", minimap_ping: false, cooldown: 1.0, priority: 0, coalesce_radius: 0.0)],
"#
        )
    };
    let pairs: [EditionPair; 5] = [
        (
            "T2 reasons",
            vec![(CEDAR_HUD, "    reasons: {},", r#"    reasons: {"queue_ful": "x"},"#)],
            vec![],
            format!(
                "cedar/interface/hud.ron: reasons.queue_ful: unknown reason key `queue_ful`; did you mean `queue_full`? (allowed: {})",
                allowed(REASON_KEYS)
            ),
        ),
        (
            "T2 cursors",
            vec![
                (CEDAR_HUD, "    cursors: {},", r#"    cursors: {"plcae": "c"},"#),
                (
                    "assets/assets.ron",
                    "cursors: []",
                    r#"cursors: [(id: "c", image: "icon_order_move", hotspot: (0, 0))]"#,
                ),
            ],
            vec![],
            "cedar/interface/hud.ron: cursors.plcae: unknown cursor context `plcae`; did you mean `place`? (allowed: attack, default, forbidden, gather, move, place, rally, repair, select)".to_string(),
        ),
        (
            "T3 weapon damage 0",
            vec![turret_manifest],
            vec![("units/turret.ron", turret)],
            "cedar/units/turret.ron: weapon.damage: must be positive".to_string(),
        ),
        (
            "T4 an unknown placeholder",
            vec![(CEDAR_HUD, "    alerts: [],\n", Box::leak(alert("Low {units}").into_boxed_str()))],
            vec![],
            "cedar/interface/hud.ron: alerts[0].text: unknown placeholder `{units}`; did you mean `{unit}`? (allowed: {slot}, {unit})".to_string(),
        ),
        (
            "T4 `{foo}` in a Format text",
            vec![(CEDAR_HUD, r#"Literal("Move selected units")"#, r#"Format("{foo}", [])"#)],
            vec![],
            "cedar/interface/hud.ron: root[hud_order_move_button].widget.label: unknown placeholder `{foo}` (allowed: {slot})".to_string(),
        ),
    ];
    for (row, edits, adds, expected) in &pairs {
        // `^0.2.3`: the behaviour of today, the package packs. T2 and T4 reach the table only
        // through a key or a placeholder that the 0.2.3 engine shows as written.
        cedar("^0.2.3", edits, adds)
            .content()
            .unwrap_or_else(|error| panic!("{row} at ^0.2.3: {error}"));
        let error = cedar("^0.2.4", edits, adds).content().expect_err(row);
        assert_eq!(error.to_string(), *expected, "row {row} at ^0.2.4");
    }
}

/// T1 (the approach point, design 6.9) refuses nothing: the package packs at both requirements.
/// The behaviour that the edition selects is in the simulation (`L-OBS`), not in the packer.
#[test]
fn t1_the_edition_refuses_nothing() {
    for api in ["^0.2.3", "^0.2.4"] {
        cedar(api, &[], &[])
            .content()
            .unwrap_or_else(|error| panic!("{api}: {error}"));
    }
}

/// KEY1 (b): two actions on one default key are refused (the check is global; there is no action
/// scope). Through `content`, on `cedar`, at `^0.2.4`; the check does not run for `^0.2.3`.
#[test]
fn key1_b_unique_default_keys() {
    let same = [(
        "input/bindings.ron",
        r#"(name: "key_cancel", default: [Key(code: "KeyC", mods: [])])"#,
        r#"(name: "key_cancel", default: [Key(code: "KeyP", mods: [])])"#,
    )];
    cedar("^0.2.3", &same, &[])
        .content()
        .expect("^0.2.3 keeps the check of today");
    assert_eq!(
        cedar("^0.2.4", &same, &[])
            .content()
            .unwrap_err()
            .to_string(),
        "cedar/input/bindings.ron: actions: KeyP is the default of key_produce and key_cancel; each default key must be unique"
    );
}

/// KEY1 (c): a default on a key that the browser reserves is refused, at both editions (the check
/// is the one of today).
#[test]
fn key1_c_reserved_browser_keys() {
    let reserved = [(
        "input/bindings.ron",
        r#"(name: "key_cancel", default: [Key(code: "KeyC", mods: [])])"#,
        r#"(name: "key_cancel", default: [Key(code: "KeyT", mods: [Ctrl])])"#,
    )];
    for api in ["^0.2.3", "^0.2.4"] {
        assert_eq!(
            cedar(api, &reserved, &[])
                .content()
                .unwrap_err()
                .to_string(),
            "cedar/input/bindings.ron: actions[4].default[0]: Ctrl+KeyT is reserved by the browser",
            "KEY1 (c) at {api}"
        );
    }
}

/// H23: `leave_match` is an engine action, so the input map does not declare it. At `^0.2.3` the
/// version table refuses the use; at `^0.2.4` the `UNBUILT` line of `UI-RESULT` does. Neither is
/// the text of an unknown input action.
#[test]
fn h23_leave_match_is_an_engine_action() {
    let button = [(
        CEDAR_HUD,
        r#"action: Order("cedar:order_cancel")"#,
        r#"action: Input("leave_match")"#,
    )];
    assert_eq!(
        cedar("^0.2.3", &button, &[])
            .content()
            .unwrap_err()
            .to_string(),
        "cedar/manifest.ron: engine_api: `engine action leave_match` needs engine api 0.2.4; `^0.2.3` admits 0.2.3 (written in interface/hud.ron: root[hud_order_cancel_button].widget.action)"
    );
    assert_eq!(
        cedar("^0.2.4", &button, &[])
            .content()
            .unwrap_err()
            .to_string(),
        "cedar/interface/hud.ron: root[hud_order_cancel_button].widget.action: `engine action leave_match` is not built in this engine build; leave it out until an engine build supports it"
    );
}

/// H13 on the two packages that already write `font` and `size` in a `Text` widget: the detection
/// reads a `Button` only (design 6.10, [mod M1]). A key walk of `ron::Value` would refuse the
/// copies of `forge` and `muster`. They pack at their own requirement, on both paths.
#[test]
fn h13_forge_and_muster_copies_pack() {
    for name in ["forge", "muster"] {
        let files = read_files(&package_dir(name));
        let text = files
            .iter()
            .filter(|(path, _)| path.starts_with("interface/"))
            .map(|(_, bytes)| String::from_utf8_lossy(bytes).into_owned())
            .collect::<String>();
        assert!(
            text.contains("font: Some("),
            "{name} writes `font` and `size` in a Text widget"
        );
        let package = archive(&files);
        package
            .content()
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        package
            .visuals()
            .unwrap_or_else(|error| panic!("{name}: {error}"));
    }
}

/// K4 (design 6.10): the M2 rule "output must be mobile and cannot be a producer" does not apply to
/// a production that a constructor lists as a building. A unit-training production of the same
/// package keeps the rule. The test calls the portable validation with the parsed files of the
/// package `val1`: `content` holds the package back as unbuilt before this rule (`L-CON1` deletes
/// the lines).
#[test]
fn k4_a_building_production_may_have_a_static_output() {
    use turts_mod_api::PackageId;
    use turts_mod_runtime::content::validate_content;
    let package = val1(&[], &[]);
    let run = |production: Vec<(String, turts_mod_api::ProductionDef)>| {
        validate_content(
            &PackageId::parse("val1").unwrap(),
            sources(package.resources().unwrap()),
            sources(package.units().unwrap()),
            sources(package.nodes().unwrap()),
            sources(production),
        )
    };
    run(package.production().unwrap()).unwrap_or_else(|error| panic!("{error}"));
    // A unit-training production with a static output is still refused.
    let mut trained = package.production().unwrap();
    for (_, production) in &mut trained {
        if production.id == "train_hauler" {
            production.output_unit = "val1:power_station".into();
        }
    }
    assert_eq!(
        run(trained).unwrap_err().to_string(),
        "val1/production/train_hauler.ron: output_unit: output must be mobile and cannot be a producer"
    );
}

fn sources<T>(files: Vec<(String, T)>) -> Vec<turts_mod_api::SourceDef<T>> {
    files
        .into_iter()
        .map(|(file, value)| turts_mod_api::SourceDef { file, value })
        .collect()
}
