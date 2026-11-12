//! L0-API1: the version rows and the `UNBUILT_FEATURES` refusals of the mod API 0.2.4
//! (design 6.10, revision 6.2; plan section 5.3), tested on whole packages.
//!
//! Each case edits the repository package `cedar` so that it uses ONE 0.2.4 feature, and packs
//! it twice:
//! - at `engine_api: "^0.2.3"` the version table refuses it, with the text of today
//!   (`cedar/manifest.ron: engine_api: `<feature>` needs engine api 0.2.4; `^0.2.3` admits 0.2.3`)
//!   and, for a feature written outside the manifest, the file and the field path;
//! - at `engine_api: "^0.2.4"` the version table passes, and the `UNBUILT_FEATURES` list refuses it
//!   with `cedar/<file>: <path>: `<feature>` is not built in this engine build; leave it out until
//!   an engine build supports it` (the text names no plan task: a modder cannot act on one),
//!   or (for a feature with no line, `owner` empty) the package loads.
//!
//! A case that passes on a tree with no 0.2.4 row would be a test that cannot fail: the control
//! test packs `cedar` unchanged at both requirements, and every other case is red on a tree
//! where its row is missing (the plants of the task).

use std::collections::BTreeMap;

mod common;

use common::{package_dir, read_files};
use turts_mod_api::{ENGINE_API_VERSION, ModError, RELEASED_ENGINE_APIS};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_files};

/// One edit of a file: the exact `old` text occurs once.
type Edit = (&'static str, &'static str, &'static str);

struct Case {
    /// The name of the version row, as in the message.
    feature: &'static str,
    /// The task that builds the feature (plan 5.3); empty when no line holds the feature back.
    owner: &'static str,
    /// The file in which the feature is written, and the field path in the message.
    file: &'static str,
    path: &'static str,
    edits: &'static [Edit],
    /// New files: `(path, text)`. The edits add them to the manifest.
    adds: &'static [(&'static str, &'static str)],
}

const MANIFEST: &str = "manifest.ron";

fn archive(engine_api: &str, case: &Case) -> PackageArchive {
    let mut files: BTreeMap<String, Vec<u8>> = read_files(&package_dir("cedar"));
    let text = String::from_utf8(files[MANIFEST].clone()).unwrap();
    assert!(
        text.contains("engine_api: \"^0.2.3\""),
        "cedar requires ^0.2.3"
    );
    let manifest = text.replace(
        "engine_api: \"^0.2.3\"",
        &format!("engine_api: \"{engine_api}\""),
    );
    files.insert(MANIFEST.into(), manifest.into_bytes());
    for (file, old, new) in case.edits {
        let text = String::from_utf8(files[*file].clone()).unwrap();
        assert_eq!(
            text.matches(old).count(),
            1,
            "{}: the edit target `{old}` must occur once in {file}",
            case.feature
        );
        files.insert((*file).to_string(), text.replacen(old, new, 1).into_bytes());
    }
    for (file, text) in case.adds {
        files.insert((*file).to_string(), text.as_bytes().to_vec());
    }
    PackageArchive::parse(&pack_files(&files).expect("pack in memory")).expect("package parses")
}

/// The server path and the client path: `content` never reads a visual, and `visuals` does.
fn load(archive: &PackageArchive) -> Result<(), ModError> {
    archive.content()?;
    archive.visuals()?;
    Ok(())
}

fn refusal(archive: &PackageArchive, what: &str) -> ModError {
    match load(archive) {
        Ok(()) => panic!("{what}: the package packs, a refusal was expected"),
        Err(error) => error,
    }
}

fn check(case: &Case) {
    let written_in = if case.file == MANIFEST {
        String::new()
    } else {
        format!(" (written in {}: {})", case.file, case.path)
    };
    let error = refusal(&archive("^0.2.3", case), case.feature);
    assert_eq!(
        error.to_string(),
        format!(
            "cedar/manifest.ron: engine_api: `{}` needs engine api 0.2.4; `^0.2.3` admits 0.2.3{written_in}",
            case.feature
        ),
        "{} at ^0.2.3",
        case.feature
    );
    let current = archive("^0.2.4", case);
    if case.owner.is_empty() {
        load(&current).unwrap_or_else(|error| panic!("{} at ^0.2.4: {error}", case.feature));
    } else {
        assert_eq!(
            refusal(&current, case.feature).to_string(),
            format!(
                "cedar/{}: {}: `{}` is not built in this engine build; leave it out until an engine build supports it",
                case.file, case.path, case.feature
            ),
            "{} at ^0.2.4",
            case.feature
        );
    }
}

// ---- Anchors in the text of `cedar` ----

const WORKER: &str = "units/worker.ron";
const WORKER_TAIL: &str = "drop_off: None, producer: None,";
const DEPOT: &str = "units/depot.ron";
const DEPOT_TAIL: &str = "(x: 0.0, y: -4.0)])),";
const HIRE: &str = "production/hire.ron";
const HIRE_TAIL: &str = "payment: ReserveThenCharge,";
const HUD: &str = "interface/hud.ron";
const RULES: &str = "orders/rules.ron";
const MAP: &str = "maps/clearing.ron";
const MAP_ANCHOR: &str =
    r#"(unit: "cedar:worker", owner_slot: 0, pos: (x: 12.0, y: 8.0), contents: []),"#;
const ASSETS: &str = "assets/assets.ron";
const VISUAL: &str = "visuals/worker.ron";
const EXPORT_ANCHOR: &str = r#"interface: ["interface/hud.ron"],"#;
const ROOT: &str = "    root: [\n";
const ALERTS: &str = "    alerts: [],\n";
const WORLD_END: &str = r##"health: "#00ff00")),"##;
const GUIDE: &str = "    guide: [],";

/// A new order file, listed in the manifest.
const ORDER_FILE: &str = "orders/order_new.ron";
const ORDERS_EXPORT: &str = r#""orders/rules.ron"],"#;

/// A new `Text` widget at the top of the interface file, with `slot` as its text value.
fn text_widget(text: &str) -> String {
    format!(
        r##"{ROOT}        (id: "t_new", at: Some((anchor: TopLeft, offset: (0.0, 0.0), pivot: None)), size: (w: Px(10.0), h: Px(10.0)), widget: Text(text: {text}, font: None, size: 12, colour: "#ffffff", align: Left, wrap: false)),
"##
    )
}

// ---- RAW rows: a field that is written in the file, with any value, also an empty one ----

macro_rules! raw_case {
    ($name:ident, $feature:expr, $owner:expr, $file:expr, $path:expr, $edits:expr) => {
        #[test]
        fn $name() {
            check(&Case {
                feature: $feature,
                owner: $owner,
                file: $file,
                path: $path,
                edits: $edits,
                adds: &[],
            });
        }
    };
}

/// A1, A2, A3: the three export groups of the manifest (an explicit empty list is a use).
#[test]
fn a1_a2_a3_new_export_groups() {
    for (feature, owner, group) in [
        ("exports.combat", "L-DMG", "combat"),
        ("exports.match", "L-MATCH-SRV1", "match"),
        ("exports.visual_effects", "L-FX-PRES", "visual_effects"),
    ] {
        let edits: &'static [Edit] = Box::leak(Box::new([(
            MANIFEST,
            EXPORT_ANCHOR,
            leak(format!("{EXPORT_ANCHOR} {group}: [],")),
        )]));
        check(&Case {
            feature,
            owner,
            file: MANIFEST,
            path: Box::leak(format!("exports.{group}").into_boxed_str()),
            edits,
            adds: &[],
        });
    }
}

raw_case!(
    u1_armour,
    "armour",
    "L-DMG",
    WORKER,
    "armour",
    &[(
        WORKER,
        WORKER_TAIL,
        "drop_off: None, producer: None, armour: None,"
    )]
);
raw_case!(
    u2_weapon_damage_type,
    "weapon.damage_type",
    "L-DMG",
    WORKER,
    "weapon.damage_type",
    &[(
        WORKER,
        WORKER_TAIL,
        "drop_off: None, producer: None, weapon: Some((range: 1.0, cooldown_ms: 1, damage: 1, ammo: None, targets: (slots: Other), damage_type: None)),"
    )]
);
raw_case!(
    u4_acquire,
    "acquire",
    "L-TGT",
    WORKER,
    "acquire",
    &[(
        WORKER,
        WORKER_TAIL,
        "drop_off: None, producer: None, acquire: None,"
    )]
);
raw_case!(
    u5_provides,
    "provides",
    "L-PWR",
    WORKER,
    "provides",
    &[(
        WORKER,
        WORKER_TAIL,
        "drop_off: None, producer: None, provides: [],"
    )]
);
raw_case!(
    u5_consumes,
    "consumes",
    "L-PWR",
    WORKER,
    "consumes",
    &[(
        WORKER,
        WORKER_TAIL,
        "drop_off: None, producer: None, consumes: [],"
    )]
);
raw_case!(
    u6_weapon_when_short,
    "weapon.when_short",
    "L-PWR",
    WORKER,
    "weapon.when_short",
    &[(
        WORKER,
        WORKER_TAIL,
        "drop_off: None, producer: None, weapon: Some((range: 1.0, cooldown_ms: 1, damage: 1, ammo: None, targets: (slots: Other), when_short: [])),"
    )]
);
raw_case!(
    u6_producer_when_short,
    "producer.when_short",
    "L-PWR",
    DEPOT,
    "producer.when_short",
    &[(DEPOT, DEPOT_TAIL, "(x: 0.0, y: -4.0)], when_short: [])),")]
);
raw_case!(
    u7_income,
    "income",
    "L-INC",
    DEPOT,
    "income",
    &[(DEPOT, DEPOT_TAIL, "(x: 0.0, y: -4.0)])), income: None,")]
);
raw_case!(
    u8_spawn_order,
    "spawn_order",
    "L-STO",
    WORKER,
    "spawn_order",
    &[(
        WORKER,
        WORKER_TAIL,
        "drop_off: None, producer: None, spawn_order: None,"
    )]
);
raw_case!(
    k1_constructor,
    "constructor",
    "L-CON1",
    DEPOT,
    "constructor",
    &[(
        DEPOT,
        DEPOT_TAIL,
        "(x: 0.0, y: -4.0)])), constructor: None,"
    )]
);
raw_case!(
    k2_placement,
    "placement",
    "L-CON1",
    DEPOT,
    "placement",
    &[(DEPOT, DEPOT_TAIL, "(x: 0.0, y: -4.0)])), placement: None,")]
);
raw_case!(
    k3_build_area,
    "build_area",
    "L-CON1",
    DEPOT,
    "build_area",
    &[(DEPOT, DEPOT_TAIL, "(x: 0.0, y: -4.0)])), build_area: None,")]
);
raw_case!(
    r1_repairer,
    "repairer",
    "L-REP",
    WORKER,
    "repairer",
    &[(
        WORKER,
        WORKER_TAIL,
        "drop_off: None, producer: None, repairer: None,"
    )]
);
raw_case!(
    k5_cancel_refunds_destroyed,
    "cancel_refunds.destroyed",
    "L-DES",
    HIRE,
    "cancel_refunds.destroyed",
    &[(
        HIRE,
        "output_blocked: (numerator: 1, denominator: 1),",
        "output_blocked: (numerator: 1, denominator: 1), destroyed: (numerator: 0, denominator: 1),"
    )]
);
raw_case!(
    o8_hotkey,
    "hotkey",
    "UI-HUD",
    HIRE,
    "hotkey",
    &[(HIRE, HIRE_TAIL, "payment: ReserveThenCharge, hotkey: None,")]
);
raw_case!(
    o10_card,
    "card",
    "UI-HUD",
    HIRE,
    "card",
    &[(HIRE, HIRE_TAIL, "payment: ReserveThenCharge, card: None,")]
);
raw_case!(
    m3_start_order,
    "start_order",
    "L-STO",
    MAP,
    "placements.start_order",
    &[(
        MAP,
        MAP_ANCHOR,
        r#"(unit: "cedar:worker", owner_slot: 0, pos: (x: 12.0, y: 8.0), contents: [], start_order: None),"#
    )]
);
raw_case!(
    o7_rally,
    "rally",
    "L-RLY",
    RULES,
    "rally",
    &[(RULES, "    ])", "    ], rally: None)")]
);
raw_case!(
    h14_hints,
    "hints",
    "UI-HINT",
    HUD,
    "hints",
    &[(HUD, GUIDE, "    guide: [],\n    hints: [],")]
);
raw_case!(
    h14_hint_fallback,
    "hint_fallback",
    "UI-HINT",
    HUD,
    "hint_fallback",
    &[(HUD, GUIDE, "    guide: [],\n    hint_fallback: None,")]
);
raw_case!(
    h18_start_selection,
    "start_selection",
    "UI-HINT",
    HUD,
    "start_selection",
    &[(HUD, GUIDE, "    guide: [],\n    start_selection: None,")]
);
raw_case!(
    h3_states,
    "states",
    "",
    HUD,
    "states",
    &[(HUD, GUIDE, "    guide: [],\n    states: {},")]
);
raw_case!(
    h20_alert_sound_gap,
    "alert_sound_gap_secs",
    "UI-ALERT",
    HUD,
    "alert_sound_gap_secs",
    &[(
        HUD,
        GUIDE,
        "    guide: [],\n    alert_sound_gap_secs: None,"
    )]
);
raw_case!(
    h16_effects_voices,
    "effects_voices",
    "FX2",
    HUD,
    "effects_voices",
    &[(HUD, GUIDE, "    guide: [],\n    effects_voices: None,")]
);
raw_case!(
    h16_audible_range,
    "audible_range",
    "FX2",
    HUD,
    "audible_range",
    &[(HUD, GUIDE, "    guide: [],\n    audible_range: None,")]
);
raw_case!(
    h19_role,
    "role",
    "UI-HUD",
    HUD,
    "units.*.role",
    &[(
        HUD,
        "    units: {},",
        r#"    units: {"cedar:worker": (icon: None, portrait: None, responses: {}, role: None)},"#
    )]
);
raw_case!(
    h2_cursor_place,
    "cursors.place",
    "UI-PLACE",
    HUD,
    "cursors.place",
    &[(
        HUD,
        "    cursors: {},",
        r#"    cursors: {"place": "icon_order_move"},"#
    )]
);
raw_case!(
    h2_cursor_repair,
    "cursors.repair",
    "UI2-HUD",
    HUD,
    "cursors.repair",
    &[(
        HUD,
        "    cursors: {},",
        r#"    cursors: {"repair": "icon_order_move"},"#
    )]
);
raw_case!(
    h2_cursor_rally,
    "cursors.rally",
    "UI2-HUD",
    HUD,
    "cursors.rally",
    &[(
        HUD,
        "    cursors: {},",
        r#"    cursors: {"rally": "icon_order_move"},"#
    )]
);
raw_case!(
    h6_alert_kind,
    "alerts.kind",
    "UI-ALERT",
    HUD,
    "alerts.kind",
    &[(
        HUD,
        ALERTS,
        r#"    alerts: [(id: "a", trigger: OwnUnitDamaged, kind: Event, sound: None, text: "t", minimap_ping: false, cooldown: 1.0, priority: 0, coalesce_radius: 0.0)],
"#
    )]
);
raw_case!(
    h11_world_placement,
    "world.placement",
    "UI-PLACE",
    HUD,
    "world.placement",
    &[(HUD, WORLD_END, r##"health: "#00ff00"), placement: None),"##)]
);
raw_case!(
    h11_world_float_text,
    "world.float_text",
    "L-FX-PRES",
    HUD,
    "world.float_text",
    &[(
        HUD,
        WORLD_END,
        r##"health: "#00ff00"), float_text: None),"##
    )]
);
raw_case!(
    h12_scale_to_footprint,
    "world.health_bar.scale_to_footprint",
    "UI2-HUD",
    HUD,
    "world.health_bar.scale_to_footprint",
    &[(
        HUD,
        WORLD_END,
        r##"health: "#00ff00", scale_to_footprint: false)),"##
    )]
);
raw_case!(
    h24_unit_labels,
    "world.unit_labels",
    "UI-LABEL",
    HUD,
    "world.unit_labels",
    &[(
        HUD,
        WORLD_END,
        r##"health: "#00ff00"), unit_labels: None),"##
    )]
);
raw_case!(
    v1_models,
    "models",
    "PRES-MODELS",
    ASSETS,
    "models",
    &[(ASSETS, "cursors: [])", "cursors: [], models: [])")]
);
raw_case!(
    v2_model,
    "model",
    "PRES-MODELS",
    VISUAL,
    "model",
    &[(VISUAL, "shape:", "model: None, shape:")]
);
raw_case!(
    v4_effects,
    "effects",
    "L-FX-PRES",
    VISUAL,
    "effects",
    &[(VISUAL, "shape:", "effects: None, shape:")]
);

// ---- VAR rows: the parsed value holds the new variant ----

macro_rules! var_case {
    ($name:ident, $feature:expr, $owner:expr, $file:expr, $path:expr, $edits:expr, $adds:expr) => {
        #[test]
        fn $name() {
            check(&Case {
                feature: $feature,
                owner: $owner,
                file: $file,
                path: $path,
                edits: $edits,
                adds: $adds,
            });
        }
    };
}

// S1: a `Supply` resource, in a new resource file.
var_case!(
    s1_supply_resource,
    "kind: Supply",
    "L-PWR",
    "resources/power.ron",
    "kind",
    &[(
        MANIFEST,
        r#"resources: ["resources/wood.ron"],"#,
        r#"resources: ["resources/wood.ron", "resources/power.ron"],"#
    )],
    &[(
        "resources/power.ron",
        r#"(id: "power", name: "Power", unit_name: "amps", kind: Supply(scope: Slot, counts: Completed))"#
    )]
);

// P1: `payment: ChargeOverTime`.
var_case!(
    p1_charge_over_time,
    "payment: ChargeOverTime",
    "L-PAY",
    HIRE,
    "payment",
    &[(HIRE, HIRE_TAIL, "payment: ChargeOverTime,")],
    &[]
);

// L1: the new relations; no line holds them back (the relation function of `turts_rules`
// serves them in this task).
var_case!(
    l1_ally,
    "SlotRelation::Ally",
    "",
    "units/turret.ron",
    "weapon.targets.slots",
    &[(
        MANIFEST,
        r#"units: ["units/depot.ron", "units/worker.ron"],"#,
        r#"units: ["units/depot.ron", "units/worker.ron", "units/turret.ron"],"#
    )],
    &[(
        "units/turret.ron",
        r#"(id: "turret", name: "Turret", footprint: Circle(radius: 1.0), movement: None, inventory: None, gatherer: None, drop_off: None, producer: None, health: Some((max: 10)), removal: Some((by_command: false, contents: Destroy)), weapon: Some((range: 5.0, cooldown_ms: 900, damage: 8, ammo: None, targets: (slots: Ally))))"#
    )]
);
var_case!(
    l1_enemy,
    "SlotRelation::Enemy",
    "",
    "units/turret.ron",
    "weapon.targets.slots",
    &[(
        MANIFEST,
        r#"units: ["units/depot.ron", "units/worker.ron"],"#,
        r#"units: ["units/depot.ron", "units/worker.ron", "units/turret.ron"],"#
    )],
    &[(
        "units/turret.ron",
        r#"(id: "turret", name: "Turret", footprint: Circle(radius: 1.0), movement: None, inventory: None, gatherer: None, drop_off: None, producer: None, health: Some((max: 10)), removal: Some((by_command: false, contents: Destroy)), weapon: Some((range: 5.0, cooldown_ms: 900, damage: 8, ammo: None, targets: (slots: Enemy))))"#
    )]
);

// O2 to O5, O9: a new order file with a new behaviour or target. `behaviour Construct` and the
// rows of the smart rules cannot be isolated on a whole package (a `Construct` order needs the
// target `ContentAtPoint`, and a smart rule needs its order): the unit tests of
// `version_table.rs` and `variants.rs` test them one by one.
macro_rules! order_case {
    ($name:ident, $feature:expr, $owner:expr, $behaviour:expr, $target:expr, $units:expr, $path:expr) => {
        var_case!($name, $feature, $owner, ORDER_FILE, $path,
            &[(MANIFEST, ORDERS_EXPORT, r#""orders/rules.ron", "orders/order_new.ron"],"#)],
            &[(ORDER_FILE, concat!(r#"(id: "order_new", name: "N", behaviour: "#, $behaviour, ", target: ", $target, r#", fire: None, engage: None, icon: "icon_order_move", hotkey: "key_move", cursor: None, card: None, units: "#, $units, r#", description: "d")"#))]);
    };
}
order_case!(
    o2_cancel_construction,
    "behaviour CancelConstruction",
    "L-CON3",
    "CancelConstruction",
    "None",
    r#"["cedar:depot"]"#,
    "behaviour"
);
order_case!(
    o3_set_rally,
    "behaviour SetRally",
    "L-RLY",
    "SetRally",
    "Point",
    r#"["cedar:depot"]"#,
    "behaviour"
);
order_case!(
    o4_repair,
    "behaviour Repair",
    "L-REP",
    "Repair",
    "Unit",
    r#"["cedar:worker"]"#,
    "behaviour"
);
order_case!(
    o9_surrender,
    "behaviour Surrender",
    "L-MATCH-SIM2",
    "Surrender",
    "None",
    "[]",
    "behaviour"
);
// `Construct` with its target: the row of the target comes first in the table.
order_case!(
    o5_content_at_point,
    "target: ContentAtPoint",
    "L-CON1",
    "Construct",
    "ContentAtPoint",
    r#"["cedar:depot"]"#,
    "target"
);

// H4: the new alert triggers.
macro_rules! trigger_case {
    ($name:ident, $trigger:expr, $feature:expr, $owner:expr) => {
        var_case!($name, $feature, $owner, HUD, "alerts[0].trigger",
            &[(HUD, ALERTS, concat!(r#"    alerts: [(id: "a", trigger: "#, $trigger, r#", sound: None, text: "t", minimap_ping: false, cooldown: 1.0, priority: 0, coalesce_radius: 0.0)],
"#))], &[]);
    };
}
trigger_case!(
    h4_construction_complete,
    "ConstructionComplete",
    "AlertTrigger::ConstructionComplete",
    "UI-ALERT"
);
trigger_case!(
    h4_supply_short,
    r#"SupplyShort("cedar:wood")"#,
    "AlertTrigger::SupplyShort",
    "UI-ALERT"
);
trigger_case!(
    h4_gatherer_idle,
    "GathererIdle(5.0)",
    "AlertTrigger::GathererIdle",
    "UI-ALERT"
);
trigger_case!(
    h4_slot_disconnected,
    "SlotDisconnected",
    "AlertTrigger::SlotDisconnected",
    "UI2-CONN"
);

/// H7 to H10: new widget variants, in the text of a widget.
// `text_widget` builds a `String`; a case holds `&'static str`, so the table tests leak it.
fn leak(text: String) -> &'static str {
    Box::leak(text.into_boxed_str())
}

#[test]
fn h7_value_variants_in_a_text_widget() {
    for (value, feature, owner) in [
        (
            r#"Bind(Supply("cedar:wood", Provided))"#,
            "Value::Supply",
            "UI-HUD",
        ),
        (
            "Bind(ConstructionQueue)",
            "Value::ConstructionQueue",
            "UI-HUD",
        ),
        ("Bind(Sites)", "Value::Sites", "UI-HUD"),
        ("Bind(Notices)", "Value::Notices", "UI-HUD"),
        ("Bind(SelectionTypes)", "Value::SelectionTypes", "UI-HUD"),
        ("Bind(Connection)", "Value::Connection", "UI2-CONN"),
        ("Bind(Outcome)", "Value::Outcome", "UI-RESULT"),
        ("Bind(OutcomeReason)", "Value::OutcomeReason", "UI-RESULT"),
        ("Bind(MatchClock)", "Value::MatchClock", "UI-RESULT"),
        (
            "Bind(MatchStats(UnitsTrained))",
            "Value::MatchStats",
            "UI2-RESULT",
        ),
        ("Bind(Hint)", "Value::Hint", "UI-HINT"),
        (
            "Bind(SelectionPrimary(Activity))",
            "PrimaryField::Activity",
            "UI2-HUD",
        ),
        (
            "Bind(SelectionPrimary(Cargo))",
            "PrimaryField::Cargo",
            "UI2-HUD",
        ),
        (
            "Bind(SelectionPrimary(Armour))",
            "PrimaryField::Armour",
            "UI2-HUD",
        ),
        ("Item(Role)", "ItemField::Role", "UI-HUD"),
        ("Item(State)", "ItemField::State", "UI-HUD"),
        ("Item(Paid)", "ItemField::Paid", "UI-HUD"),
        ("Item(Refund)", "ItemField::Refund", "UI-HUD"),
        ("Item(Badge)", "ItemField::Badge", "UI-HUD"),
        ("Item(BuildTimeNow)", "ItemField::BuildTimeNow", "UI2-HUD"),
        ("Item(SupplyChange)", "ItemField::SupplyChange", "UI2-HUD"),
        (
            "Item(CausesShortage)",
            "ItemField::CausesShortage",
            "UI2-HUD",
        ),
    ] {
        let edit: &'static [Edit] = Box::leak(Box::new([(HUD, ROOT, leak(text_widget(value)))]));
        check(&Case {
            feature,
            owner,
            file: HUD,
            path: "root[t_new].widget.text",
            edits: edit,
            adds: &[],
        });
    }
}

/// H10: the new predicates, in the `visible` list of a widget.
#[test]
fn h10_predicates_in_a_visible_list() {
    for (predicate, feature, owner) in [
        ("Placing", "Predicate::Placing", "UI-PLACE"),
        ("MatchOver", "Predicate::MatchOver", "UI-RESULT"),
        ("OutcomeIs(Victory)", "Predicate::OutcomeIs", "UI-RESULT"),
        ("HintShown", "Predicate::HintShown", "UI-HINT"),
        (
            r#"SupplyShort("cedar:wood")"#,
            "Predicate::SupplyShort",
            "UI-HUD",
        ),
        ("ItemHas(Count)", "Predicate::ItemHas", "UI-HUD"),
        (
            "PrimaryHasValue(Health)",
            "Predicate::PrimaryHasValue",
            "UI2-HUD",
        ),
    ] {
        let widget = format!(
            r##"{ROOT}        (id: "t_new", at: Some((anchor: TopLeft, offset: (0.0, 0.0), pivot: None)), size: (w: Px(10.0), h: Px(10.0)), visible: [{predicate}], widget: Text(text: Literal("x"), font: None, size: 12, colour: "#ffffff", align: Left, wrap: false)),
"##
        );
        let edit: &'static [Edit] = Box::leak(Box::new([(HUD, ROOT, leak(widget))]));
        check(&Case {
            feature,
            owner,
            file: HUD,
            path: "root[t_new].visible",
            edits: edit,
            adds: &[],
        });
    }
}

/// H7: a new `Source` variant, in a list widget.
#[test]
fn h7_source_variants_in_a_list_widget() {
    for (source, feature, owner) in [
        ("ConstructionQueue", "Source::ConstructionQueue", "UI-HUD"),
        ("Sites", "Source::Sites", "UI-HUD"),
        ("Notices", "Source::Notices", "UI-HUD"),
        ("SelectionTypes", "Source::SelectionTypes", "UI-HUD"),
    ] {
        let widget = format!(
            r##"{ROOT}        (id: "t_new", at: Some((anchor: TopLeft, offset: (0.0, 0.0), pivot: None)), size: (w: Px(10.0), h: Px(10.0)), widget: List(source: {source}, direction: Vertical, gap: 1, max_items: 2, item: (id: "t_item", size: (w: Fill, h: Px(8.0)), widget: Text(text: Literal("x"), font: None, size: 12, colour: "#ffffff", align: Left, wrap: false)))),
"##
        );
        let edit: &'static [Edit] = Box::leak(Box::new([(HUD, ROOT, leak(widget))]));
        check(&Case {
            feature,
            owner,
            file: HUD,
            path: "root[t_new].widget.source",
            edits: edit,
            adds: &[],
        });
    }
}

/// H13: `font` and `size` of a `Button`, detected on the parsed value; a `Text` that writes
/// `font` and `size` (every package does) is not a use.
#[test]
fn h13_button_font_and_size_are_a_use_but_a_text_font_is_not() {
    for (tail, feature, path) in [
        (
            ", font: None",
            "Button.font",
            "root[hud_order_move_button].widget.font",
        ),
        (
            ", size: None",
            "Button.size",
            "root[hud_order_move_button].widget.size",
        ),
    ] {
        let old = r#"widget: Button(icon: None, label: Some(Literal("Move selected units")), action: Order("cedar:order_move"), hotkey_label: true)"#;
        let new = format!(
            r#"widget: Button(icon: None, label: Some(Literal("Move selected units")), action: Order("cedar:order_move"), hotkey_label: true{tail})"#
        );
        let edit: &'static [Edit] = Box::leak(Box::new([(HUD, old, leak(new))]));
        check(&Case {
            feature,
            owner: "UI2-MC",
            file: HUD,
            path,
            edits: edit,
            adds: &[],
        });
    }
}

/// H17: a placeholder in an alert text is a use only for the edition 0.2.4 (T4): a `^0.2.3`
/// package shows `{unit}` as written, and packs.
#[test]
fn h17_placeholder_is_a_use_in_the_edition_0_2_4_only() {
    let edit = (
        HUD,
        ALERTS,
        r#"    alerts: [(id: "a", trigger: OwnUnitDamaged, sound: None, text: "{unit} hit", minimap_ping: false, cooldown: 1.0, priority: 0, coalesce_radius: 0.0)],
"#,
    );
    let case = Case {
        feature: "placeholder {unit}",
        owner: "UI-ALERT",
        file: HUD,
        path: "alerts[0].text",
        edits: Box::leak(Box::new([edit])),
        adds: &[],
    };
    archive("^0.2.3", &case)
        .content()
        .expect("edition 0.2.3: `{unit}` is text, not a placeholder");
    assert_eq!(
        refusal(&archive("^0.2.4", &case), "placeholder").to_string(),
        "cedar/interface/hud.ron: alerts[0].text: `placeholder {unit}` is not built in this engine build; leave it out until an engine build supports it"
    );
}

// ---- The control and the bump ----

/// The package with no 0.2.4 feature packs at both requirements: the rows do not fire on
/// today's forms. Every repository package (they all require `^0.2.3`) is checked in
/// `tests/mc_content.rs`, `tests/forge_package.rs` and by the `turts-pack` run of the task.
#[test]
fn a_package_with_no_new_feature_is_accepted_at_both_requirements() {
    let none = Case {
        feature: "none",
        owner: "",
        file: MANIFEST,
        path: "",
        edits: &[],
        adds: &[],
    };
    archive("^0.2.3", &none).content().expect("^0.2.3");
    archive("^0.2.4", &none).content().expect("^0.2.4");
}

/// The bump of design D:2280-2286: the engine is 0.2.4, the list is ascending, and a package
/// that requires `^0.2.3` still accepts the engine.
#[test]
fn the_engine_api_is_0_2_4() {
    assert_eq!(ENGINE_API_VERSION, "0.2.4");
    assert_eq!(RELEASED_ENGINE_APIS.last(), Some(&"0.2.4"));
    assert_eq!(RELEASED_ENGINE_APIS.len(), 6);
}

/// The order rules keep their `rally` list on the way through the archive reader (the raw rules
/// type of `archive.rs` had to learn the field, or the list was lost or refused).
#[test]
fn the_order_rules_reader_keeps_the_rally_list() {
    let case = Case {
        feature: "rally",
        owner: "L-RLY",
        file: RULES,
        path: "rally",
        edits: &[(
            RULES,
            "    ])",
            r#"    ], rally: Some([(target: Ground, needs: Movement, order: "cedar:order_move")]))"#,
        )],
        adds: &[],
    };
    let rules = archive("^0.2.4", &case)
        .order_rules()
        .expect("the rules file parses");
    let [(_, definition)] = rules.as_slice() else {
        panic!("one rules file: {rules:?}");
    };
    assert_eq!(definition.rally.as_ref().map(Vec::len), Some(1));
    // The control: a rules file with no `rally` has `None`.
    let plain = archive("^0.2.4", &Case { edits: &[], ..case })
        .order_rules()
        .unwrap();
    assert_eq!(plain[0].1.rally, None);
}
