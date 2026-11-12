//! L0-API1: the mod API 0.2.4 as types. Every new struct, field and variant of design 6.10
//! (revision 6.2) must parse from the RON of its example, and the value must survive a write
//! and a read. The block "Defaults and RON examples" is the source of the examples; a row id
//! (A1, U4, H14 ...) in a test name says which table row the example serves.
//!
//! The tests name only types that exist in API 0.2.3 (and the three new top-level files:
//! `CombatDef`, `MatchDef`, `VisualEffectsDef`), so they fail by a parse error, not by a
//! missing name, on a tree that has no 0.2.4 field.

use serde::{Serialize, de::DeserializeOwned};
use std::fmt::Debug;
use turts_mod_api::{
    AssetsDef, CombatDef, InterfaceDef, Manifest, MapDef, MatchDef, OrderDef, OrderRulesDef,
    ProductionDef, ResourceDef, TechnologyDef, UnitDef, VisualDef, VisualEffectsDef,
};

/// Parse `text` as `T`, write the value, read it again, and require the same value. A field
/// that is lost on the way (a missing `skip_serializing_if`, a flag that does not round-trip)
/// fails here.
fn parse<T: DeserializeOwned + Serialize + PartialEq + Debug>(what: &str, text: &str) -> T {
    let value: T = ron::from_str(text).unwrap_or_else(|error| panic!("{what}: {error}\n{text}"));
    let written = ron::to_string(&value).unwrap_or_else(|error| panic!("{what}: write: {error}"));
    let again: T = ron::from_str(&written)
        .unwrap_or_else(|error| panic!("{what}: read again: {error}\n{written}"));
    assert_eq!(value, again, "{what}: the value changed in a round trip");
    value
}

/// A manifest is read only: `UniqueMap` writes `({})`, which no reader accepts (a defect of
/// 0.2.3 that this task leaves as it is; no code writes a manifest back).
fn parse_manifest(what: &str, text: &str) -> Manifest {
    ron::from_str(text).unwrap_or_else(|error| panic!("{what}: {error}\n{text}"))
}

const UNIT: &str = r#"id: "u", name: "U", footprint: Circle(radius: 1.0), movement: None, inventory: None, gatherer: None, drop_off: None, producer: None"#;

fn unit(tail: &str) -> UnitDef {
    parse(tail, &format!("({UNIT}, {tail})"))
}

/// A unit whose `producer` field is `producer`.
fn unit_with_producer(producer: &str) -> UnitDef {
    let text = format!(
        "({}, producer: {producer})",
        UNIT.replace(", producer: None", "")
    );
    parse(producer, &text)
}

/// An interface file with the markers replaced: `@ROOT@` (widgets), `@TOP@` (more top-level
/// fields, each ended with a comma), `@ALERTS@` (the `alerts` field, with its comma), `@WORLD@`
/// (more world fields), `@HEALTH_BAR@` (more health bar fields) and `@UNITS@` (the `units` map).
const INTERFACE: &str = r##"(id: "hud", root: [@ROOT@], tooltip_style: (background: "#000000", colour: "#ffffff", font: None, size: 12), cursors: {}, camera: (pitch: 0.7, yaw: 0.0, field_of_view: 60.0, minimum: 10.0, maximum: 100.0, start: MapCentre, height: 40.0, margin: 0.0, pan_speed: 1.0, edge_band: true), light: None, slot_colours: [], world: (ground: "#000000", grid: "#ffffff", selection: "#ffffff", health: "#00ff00", range: "#ffffff", show_range: true, tooltip: (background: "#000000", colour: "#ffffff", font: None, size: 12), health_bar: (width: 32, height: 4, background: "#000000", health: "#00ff00"@HEALTH_BAR@)@WORLD@), @ALERTS@ reasons: {}, units: {@UNITS@}, selection_priority: [], guide: [], @TOP@)"##;

#[derive(Default)]
struct Parts<'a> {
    root: &'a str,
    top: &'a str,
    alerts: Option<&'a str>,
    /// `top` holds the `alerts` field: leave the default one out.
    no_alerts: bool,
    world: &'a str,
    health_bar: &'a str,
    units: &'a str,
}

fn interface(parts: Parts<'_>) -> InterfaceDef {
    let alerts = match (parts.no_alerts, parts.alerts) {
        (true, _) => String::new(),
        (false, None) => "alerts: [],".to_string(),
        (false, Some(text)) => format!("alerts: [{text}],"),
    };
    let text = INTERFACE
        .replace("@ROOT@", parts.root)
        .replace("@TOP@", parts.top)
        .replace("@ALERTS@", &alerts)
        .replace("@WORLD@", parts.world)
        .replace("@HEALTH_BAR@", parts.health_bar)
        .replace("@UNITS@", parts.units);
    parse("interface", &text)
}

fn widget(slot: &str, visible: &str) -> String {
    format!(
        r#"(id: "w", at: Some((anchor: TopLeft, offset: (0.0, 0.0), pivot: None)), size: (w: Px(10.0), h: Px(10.0)), visible: [{visible}], widget: {slot})"#
    )
}

fn text_widget(text: &str) -> String {
    widget(
        &format!(
            r##"Text(text: {text}, font: None, size: 12, colour: "#ffffff", align: Left, wrap: false)"##
        ),
        "",
    )
}

fn alert(extra: &str) -> String {
    format!(
        r#"(id: "a", trigger: OwnUnitDamaged, sound: None, text: "t", minimap_ping: false, cooldown: 1.0, priority: 0, coalesce_radius: 0.0{extra})"#
    )
}

fn order(behaviour: &str, target: &str) -> OrderDef {
    parse(
        behaviour,
        &format!(
            r#"(id: "o", name: "O", behaviour: {behaviour}, target: {target}, fire: None, engage: None, icon: "i", hotkey: "k", cursor: None, card: None, units: [], description: "d")"#
        ),
    )
}

// ---- Exports and the three new files ----

/// A1, A2, A3: the three new export groups of the manifest.
#[test]
fn a1_a2_a3_manifest_exports_the_three_new_groups() {
    let manifest = parse_manifest(
        "manifest",
        r#"(id: "m", version: "0.1.0", kind: Game, engine_api: "^0.2.4", dependencies: {}, exports: (units: [], maps: [], visuals: [], resources: [], nodes: [], production: [], combat: ["combat/damage.ron"], match: ["match/match.ron"], visual_effects: ["fx/effects.ron"]), context: Shared, capabilities: [], license: "x")"#,
    );
    assert_eq!(manifest.exports.combat, ["combat/damage.ron"]);
    assert_eq!(manifest.exports.match_files, ["match/match.ron"]);
    assert_eq!(manifest.exports.visual_effects, ["fx/effects.ron"]);
}

/// A1, C1 to C4: `CombatDef`, the example of the block.
#[test]
fn c1_c4_combat_def_example() {
    let _: CombatDef = parse(
        "CombatDef",
        r#"(damage_types: ["spear", "bolt"], armour_classes: ["light_hull", "plating"], minimum: 1, cells: [(damage: "tide:spear", armour: "tide:plating", ratio: (numerator: 1, denominator: 4))])"#,
    );
}

/// A2, K6, M1, M2: `MatchDef`, the three start policies and the victory rule.
#[test]
fn a2_k6_m1_m2_match_def_examples() {
    for start in [
        "AllSeated(countdown_secs: 0)",
        "AllSeatedReady(countdown_secs: 5)",
        "Immediate",
    ] {
        let _: MatchDef = parse(start, &format!("(start: {start})"));
    }
    let _: MatchDef = parse(
        "MatchDef",
        r#"(start: AllSeated(countdown_secs: 0), open_sites_per_slot: Some(1), victory: Some(LastTeamStanding(defeat: [KeyUnitsLost(units: ["frontier:command_center"])])))"#,
    );
    let _: MatchDef = parse(
        "both defeat clauses",
        r#"(victory: Some(LastTeamStanding(defeat: [KeyUnitsLost(units: ["a:b"]), DisconnectedFor(secs: 90)])))"#,
    );
    let _: MatchDef = parse("every field absent", "()");
}

/// V3: the visual effects file.
#[test]
fn v3_visual_effects_file_example() {
    let _: VisualEffectsDef = parse(
        "VisualEffectsDef",
        r##"(effects: [(id: "flash", kind: Flash, colour: "#FFD080", size: 0.6, ms: 120, sound: Some("tide:shot")), (id: "spark", kind: Spark, colour: "#FFFFFF", size: 0.2, ms: 80), (id: "t", kind: Tracer, colour: "#FFFFFF", size: 1.0, ms: 50), (id: "b", kind: Burst, colour: "#FFFFFF", size: 2.0, ms: 300), (id: "s", kind: Smoke, colour: "#888888", size: 1.5, ms: 900)])"##,
    );
}

// ---- Unit, weapon and supply fields ----

/// U1: `armour`; `flat` defaults to 0.
#[test]
fn u1_armour_example() {
    unit(r#"armour: Some((class: "tide:plating"))"#);
    unit(r#"armour: Some((class: "tide:plating", flat: 5))"#);
    unit("armour: None");
}

/// U2, U6: the weapon fields.
#[test]
fn u2_u6_weapon_fields() {
    unit(
        r#"weapon: Some((range: 5.0, cooldown_ms: 900, damage: 8, ammo: None, targets: (slots: Other), damage_type: Some("tide:spear")))"#,
    );
    unit(
        r#"weapon: Some((range: 5.0, cooldown_ms: 900, damage: 8, ammo: None, targets: (slots: Other), when_short: [(resource: "tide:current", effect: Disable)]))"#,
    );
}

/// U4: `acquire`, both fields required.
#[test]
fn u4_acquire_example() {
    unit("acquire: Some((range: 6.0, leash: 4.0))");
}

/// U5, U6, U7, S1: `provides`, `consumes`, `income`, `when_short`, `Rate`.
#[test]
fn u5_u6_u7_supply_and_income_examples() {
    unit(r#"provides: [(resource: "tide:current", amount: 50)]"#);
    unit(r#"consumes: [(resource: "tide:current", amount: 20)]"#);
    unit(r#"income: Some((resource: "tide:pearls", amount: 2, period_ms: 1000))"#);
    unit_with_producer(
        r#"Some((recipes: [], queue_capacity: 5, spawn_offsets: [], when_short: [(resource: "tide:current", effect: Rate((numerator: 2, denominator: 3)))]))"#,
    );
    unit_with_producer(
        r#"Some((recipes: [], queue_capacity: 5, spawn_offsets: [], when_short: [(resource: "tide:current", effect: BlockStart)]))"#,
    );
}

/// S1: a `Supply` resource, both fields required.
#[test]
fn s1_supply_resource_example() {
    let _: ResourceDef = parse(
        "Supply",
        r#"(id: "current", name: "Current", unit_name: "amps", kind: Supply(scope: Slot, counts: Completed))"#,
    );
    let _: ResourceDef = parse(
        "Supply Queued",
        r#"(id: "current", name: "Current", unit_name: "amps", kind: Supply(scope: Slot, counts: Queued))"#,
    );
}

/// U8, M3: `StartOrder`, one type for a unit type and a map placement; `ground_rally` is omitted
/// in the example of the block.
#[test]
fn u8_m3_start_order_examples() {
    unit(r#"spawn_order: Some((order: "tide:order_gather", target: NearestNode))"#);
    unit(
        r#"spawn_order: Some((order: "tide:order_gather", target: NearestNode, ground_rally: ApplyFirst))"#,
    );
    unit(
        r#"spawn_order: Some((order: "tide:order_gather", target: Point((x: 1.0, y: 2.0)), ground_rally: Ignore))"#,
    );
    let _: MapDef = parse(
        "start_order on a placement",
        r#"(id: "m", size: (x: 8.0, y: 8.0), placements: [(unit: "tide:skimmer", owner_slot: 0, pos: (x: 1.0, y: 1.0), contents: [], start_order: Some((order: "tide:order_gather", target: Node(0))))], nodes: [], slot_economy: [])"#,
    );
    let _: MapDef = parse(
        "start_order absent",
        r#"(id: "m", size: (x: 8.0, y: 8.0), placements: [(unit: "tide:skimmer", owner_slot: 0, pos: (x: 1.0, y: 1.0), contents: [])], nodes: [], slot_economy: [])"#,
    );
}

// ---- Construction and payment ----

/// K1: `constructor`; `progress` defaults to `Autonomous`, `when_short` to `[]`.
#[test]
fn k1_constructor_example() {
    unit(
        r#"constructor: Some((buildings: ["tide:build_spire"], concurrent_sites: 1, queue_capacity: 4))"#,
    );
    unit(
        r#"constructor: Some((buildings: ["tide:build_spire"], concurrent_sites: 1, queue_capacity: 4, progress: Autonomous, when_short: [(resource: "tide:current", effect: Rate((numerator: 2, denominator: 3)))]))"#,
    );
    unit(
        r#"constructor: Some((buildings: [], concurrent_sites: 1, queue_capacity: 1, progress: NeedsBuilder(reach: 3.0)))"#,
    );
}

/// K2, K3: `placement` and `build_area`.
#[test]
fn k2_k3_placement_and_build_area_examples() {
    unit(
        r#"placement: Some((gap: 2.2, in_build_area: true, site_health: (numerator: 1, denominator: 10)))"#,
    );
    unit(
        r#"placement: Some((gap: 2.2, node_clearance: 2.0, in_build_area: true, site_health: (numerator: 1, denominator: 10)))"#,
    );
    unit("placement: Some((gap: 2.2))");
    unit("build_area: Some(14.0)");
}

/// K4, K5, P1, O8, O10: a building production entry.
#[test]
fn k4_k5_p1_o8_o10_production_examples() {
    let _: ProductionDef = parse(
        "building production",
        r#"(id: "build_spire", name: "Spire", output_unit: "tide:spire", duration_ms: 14000, costs: [(resource: "tide:pearls", amount: 550, source: SlotBalance)], payment: ChargeOverTime, cancel_refunds: (queued: (numerator: 0, denominator: 1), running: (numerator: 1, denominator: 1), output_blocked: (numerator: 0, denominator: 1), destroyed: (numerator: 1, denominator: 4)), hotkey: Some("key_spire"), card: Some((row: 0, column: 0)))"#,
    );
    let _: ProductionDef = parse(
        "no destroyed, no hotkey, no card",
        r#"(id: "p", name: "P", output_unit: "tide:u", duration_ms: 1, costs: [], payment: ChargeOnQueue, cancel_refunds: (queued: (numerator: 1, denominator: 1), running: (numerator: 1, denominator: 1), output_blocked: (numerator: 1, denominator: 1)))"#,
    );
    let _: TechnologyDef = parse(
        "technology with destroyed",
        r#"(id: "t", name: "T", prerequisites: [], duration_ms: 1, costs: [], payment: ChargeOnQueue, cancel_refunds: (queued: (numerator: 1, denominator: 1), running: (numerator: 1, denominator: 1), output_blocked: (numerator: 1, denominator: 1), destroyed: (numerator: 1, denominator: 2)), effects: [])"#,
    );
}

/// R1: `repairer`.
#[test]
fn r1_repairer_example() {
    unit(
        r#"repairer: Some((rate_per_second: 20, reach: 1.5, cost: [(resource: "tide:pearls", amount: 1, per_health: 4)], targets: (slots: Same), classes: [], auto_range: None))"#,
    );
    unit(
        r#"repairer: Some((rate_per_second: 20, reach: 1.5, cost: [], targets: (slots: Ally), classes: ["tide:plating"], auto_range: Some(4.0)))"#,
    );
    unit(r#"repairer: Some((rate_per_second: 20, reach: 1.5, cost: [], targets: (slots: Ally)))"#);
}

/// L1: the new relations in a target filter.
#[test]
fn l1_ally_and_enemy_relations() {
    for relation in ["Ally", "Enemy"] {
        unit(&format!(
            "weapon: Some((range: 5.0, cooldown_ms: 900, damage: 8, ammo: None, targets: (slots: {relation})))"
        ));
    }
}

// ---- Orders, rules ----

/// O1 to O5, O9: the new behaviours and the new target.
#[test]
fn o1_o5_o9_new_behaviours_and_target() {
    order("Construct", "ContentAtPoint");
    order("CancelConstruction", "None");
    order("SetRally", "Point");
    order("Repair", "Unit");
    order("Surrender", "None");
}

/// O6, O7: smart targets, capabilities, and the `rally` list.
#[test]
fn o6_o7_smart_rules_and_rally() {
    let _: OrderRulesDef = parse(
        "rules",
        r#"(queue_depth: 16, smart: [(target: OwnDamaged, needs: Repairer, order: "tide:order_repair"), (target: Ground, needs: Constructor, order: "tide:order_construct")])"#,
    );
    let _: OrderRulesDef = parse(
        "rules with rally",
        r#"(queue_depth: 16, smart: [], rally: Some([(target: Ground, needs: Weapon, order: "tide:order_attack_move")]))"#,
    );
}

// ---- Interface ----

/// H1, H3, H14, H16, H18, H20: the fields of the interface file.
#[test]
fn h14_h16_h18_h20_interface_fields() {
    let hints = r#"start_selection: Some("tide:dock"), hints: [(id: "h_spire", kind: Step, text: "Select your Dock, press B, then click near it.", starts: MatchStarted, done: OwnUnits(units: ["tide:spire"], state: Any, at_least: 1))], hint_fallback: Some("Train Divers."), states: {"waiting_for_funds": "On hold: no {resource}"}, effects_voices: Some(8), audible_range: Some(30.0), alert_sound_gap_secs: Some(2.0),"#;
    interface(Parts {
        top: hints,
        ..Parts::default()
    });
}

/// H14: every `HintCondition`.
#[test]
fn h14_every_hint_condition() {
    for condition in [
        "MatchStarted",
        r#"OwnUnits(units: ["a:b"], state: Pending, at_least: 1)"#,
        r#"OwnUnits(units: ["a:b"], state: Site, at_least: 1)"#,
        r#"OwnUnits(units: ["a:b"], state: Complete, at_least: 2)"#,
        r#"OwnJob(production: "a:b")"#,
        r#"OwnRally(unit: "a:b")"#,
        r#"OrderAccepted(order: "a:b", target: Point)"#,
        r#"OrderAccepted(order: "a:b", target: Unit)"#,
        r#"OrderAccepted(order: "a:b", target: Any)"#,
        r#"SupplyShort("a:b")"#,
        "JobOnHold(secs: 10.0)",
        r#"HintDone("h")"#,
        r#"HintShownFor(id: "h", secs: 5.0)"#,
        r#"Selected(units: ["a:b"])"#,
        r#"StepCurrent("h")"#,
        "All([MatchStarted, MatchStarted])",
        "Any([MatchStarted])",
        "Not(MatchStarted)",
    ] {
        let hint = format!(
            r#"hints: [(id: "h", kind: Situation, text: "t", starts: {condition}, done: Not({condition}))],"#
        );
        interface(Parts {
            top: &hint,
            ..Parts::default()
        });
    }
}

/// H4, H5, H6, H17, H21, H22: the alert triggers and fields.
#[test]
fn h4_h6_h17_h21_h22_alert_fields() {
    for trigger in [
        "ConstructionComplete",
        r#"SupplyShort("a:b")"#,
        r#"SupplyRestored("a:b")"#,
        r#"InsufficientFunds("a:b")"#,
        "SlotDisconnected",
        "SlotReconnected",
        "GathererIdle(5.0)",
    ] {
        let alert = alert("").replace("OwnUnitDamaged", trigger);
        interface(Parts {
            alerts: Some(&alert),
            ..Parts::default()
        });
    }
    // The example of the block, every new alert field written.
    let alert = r#"(id: "a_short", trigger: SupplyShort("tide:current"), kind: State, clear_after: Some(8.0), focus: Some(["tide:spire", "tide:dock"]), sound: Some("tide:alert_power"), sound_when: Always, text: "Low current: Spires are off.", minimap_ping: true, cooldown: 45.0, priority: 2, coalesce_radius: 0.0)"#;
    interface(Parts {
        alerts: Some(alert),
        ..Parts::default()
    });
    let more =
        alert_with(r#"units: ["a:b"], select_on_click: true, sound_when: QueueEmpty, kind: Event"#);
    interface(Parts {
        alerts: Some(&more),
        ..Parts::default()
    });
}

fn alert_with(extra: &str) -> String {
    alert(&format!(", {extra}"))
}

/// H7 to H10: new `Value`, `Source`, `PrimaryField`, `ItemField`, `Predicate` variants.
#[test]
fn h7_h10_new_widget_variants() {
    for value in [
        r#"Bind(Supply("a:b", Provided))"#,
        r#"Bind(Supply("a:b", Consumed))"#,
        "Bind(ConstructionQueue)",
        "Bind(Sites)",
        "Bind(Notices)",
        "Bind(SelectionTypes)",
        "Bind(Connection)",
        "Bind(Outcome)",
        "Bind(OutcomeReason)",
        "Bind(MatchStats(UnitsTrained))",
        "Bind(MatchStats(UnitsLost))",
        "Bind(MatchStats(UnitsDestroyed))",
        "Bind(MatchStats(StructuresCompleted))",
        "Bind(MatchStats(StructuresLost))",
        r#"Bind(MatchStats(Gathered("a:b")))"#,
        "Bind(MatchClock)",
        "Bind(Hint)",
        "Bind(SelectionPrimary(Activity))",
        "Bind(SelectionPrimary(Cargo))",
        "Bind(SelectionPrimary(Armour))",
    ] {
        interface(Parts {
            root: &text_widget(value),
            ..Parts::default()
        });
    }
    for field in [
        "Role",
        "BuildTimeNow",
        "SupplyChange",
        "CausesShortage",
        "State",
        "Paid",
        "Refund",
        "Badge",
    ] {
        interface(Parts {
            root: &text_widget(&format!("Item({field})")),
            ..Parts::default()
        });
    }
    for source in ["ConstructionQueue", "Sites", "Notices", "SelectionTypes"] {
        let list = widget(
            &format!(
                r#"List(source: {source}, direction: Vertical, gap: 1, max_items: 2, item: (id: "i", widget: Box(fill: None, border: None, children: [])))"#
            ),
            "",
        );
        interface(Parts {
            root: &list,
            ..Parts::default()
        });
    }
}

/// H10: the new predicates.
#[test]
fn h10_new_predicates() {
    for predicate in [
        "ItemHas(Role)",
        "PrimaryHasValue(Cargo)",
        r#"SupplyShort("a:b")"#,
        "MatchOver",
        "OutcomeIs(Victory)",
        "OutcomeIs(Defeat)",
        "OutcomeIs(Draw)",
        "OutcomeReasonIs(KeyUnitsLost)",
        "OutcomeReasonIs(Surrendered)",
        "OutcomeReasonIs(Disconnected)",
        "OutcomeReasonIs(OpponentsDefeated)",
        "Placing",
        "HintShown",
        "Not(Placing)",
    ] {
        let widget = widget(
            r##"Text(text: Literal("x"), font: None, size: 12, colour: "#ffffff", align: Left, wrap: false)"##,
            predicate,
        );
        interface(Parts {
            root: &widget,
            ..Parts::default()
        });
    }
}

/// The result screen of the block: `Bind(OutcomeReason)`, `OutcomeReasonIs`, `{slot}` in a Format.
#[test]
fn h10_h17_result_screen_example() {
    let text = format!(
        "{}, {}",
        text_widget("Bind(OutcomeReason)"),
        widget(
            r##"Text(text: Format("Victory. {slot} surrendered.", []), font: None, size: 12, colour: "#ffffff", align: Left, wrap: false)"##,
            "OutcomeReasonIs(Surrendered)"
        )
        .replace("\"w\"", "\"w2\"")
    );
    interface(Parts {
        root: &text,
        ..Parts::default()
    });
}

/// H23: `Input("leave_match")` is a plain string in `ButtonAction::Input`.
#[test]
fn h23_leave_match_button() {
    let button = widget(
        r#"Button(icon: None, label: Some(Literal("Leave")), action: Input("leave_match"), hotkey_label: false)"#,
        "",
    );
    interface(Parts {
        root: &button,
        ..Parts::default()
    });
}

/// H13: `Button.font` and `Button.size`, written, written with `None`, and absent.
#[test]
fn h13_button_font_and_size() {
    for tail in [
        "",
        ", font: None",
        ", size: None",
        r#", font: Some("mono"), size: Some(16)"#,
        ", font: None, size: None",
    ] {
        let button = widget(
            &format!(
                r#"Button(icon: None, label: None, action: Input("menu"), hotkey_label: false{tail})"#
            ),
            "",
        );
        interface(Parts {
            root: &button,
            ..Parts::default()
        });
    }
}

/// H11, H12, H24: the world styles.
#[test]
fn h11_h12_h24_world_styles() {
    let world = r##", placement: Some((valid: "#56B4E9", invalid: "#D55E00", stripes: true)), unit_labels: Some((show: Always, own_only: false, colour: "#ffffff", font: None, size: 14))"##;
    interface(Parts {
        world,
        health_bar: ", scale_to_footprint: true",
        ..Parts::default()
    });
    let world = r##", unit_labels: Some((show: Selected, own_only: true, colour: "#ffffff", font: Some("f"), size: 14))"##;
    interface(Parts {
        world,
        ..Parts::default()
    });
    interface(Parts {
        world: ", unit_labels: None",
        ..Parts::default()
    });
}

/// H19: `UnitPresentation.role`; absent gives no role line.
#[test]
fn h19_unit_presentation_role() {
    interface(Parts {
        units: r#""a:b": (icon: None, portrait: None, responses: {}, role: Some("Tank"))"#,
        ..Parts::default()
    });
    interface(Parts {
        units: r#""a:b": (icon: None, portrait: None, responses: {})"#,
        ..Parts::default()
    });
}

// ---- Assets, visuals ----

/// V1: `models`.
#[test]
fn v1_models_example() {
    let _: AssetsDef = parse(
        "assets with models",
        r#"(images: [], fonts: [], sounds: [], cursors: [], models: [(id: "lancer_glb", path: "assets/models/lancer.glb")])"#,
    );
    let _: AssetsDef = parse(
        "assets without models",
        "(images: [], fonts: [], sounds: [], cursors: [])",
    );
}

/// V2, V4: the visual `model` and `effects`.
#[test]
fn v2_v4_visual_model_and_effects() {
    let _: VisualDef = parse(
        "model",
        r#"(target: Unit("tide:lancer"), shape: Capsule(radius: 0.5, height: 1.2), model: Some((asset: "tide:lancer_glb", team_material: Some("team"), turret_node: None, muzzle_node: Some("muzzle"))))"#,
    );
    let _: VisualDef = parse(
        "model, names absent",
        r#"(target: Unit("tide:lancer"), shape: Capsule(radius: 0.5, height: 1.2), model: Some((asset: "tide:lancer_glb")))"#,
    );
    let _: VisualDef = parse(
        "effects",
        r#"(target: Unit("tide:lancer"), shape: Capsule(radius: 0.5, height: 1.2), effects: Some((fire: Some("tide:flash"), hit: Some("tide:spark"), hit_reduced: None, death: Some("tide:burst"), damaged: None)))"#,
    );
    let _: VisualDef = parse(
        "effects, damaged",
        r#"(target: Unit("tide:lancer"), shape: Capsule(radius: 0.5, height: 1.2), effects: Some((damaged: Some((effect: "tide:smoke", below: (numerator: 1, denominator: 2))))))"#,
    );
}

// ---- The gate-2 sample mod (design revision 4) ----

/// The 18 files of the sample mod `tide`, written by the mod-builder from the design alone at
/// revision 4 (`tests/fixtures/tide/`, a byte-exact copy). Each file parses with the 0.2.4
/// types of design revision 6.2. The one file that is not a whole 0.2.4 file is the HUD
/// fragment: it holds the new fields of an interface file, and one field that is not in the
/// design (`result_widget_guess`, a guess of the sample's author).
#[test]
fn the_tide_sample_mod_parses() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tide");
    let read = |path: &str| {
        std::fs::read_to_string(root.join(path)).unwrap_or_else(|error| panic!("{path}: {error}"))
    };
    let manifest = parse_manifest("manifest", &read("manifest.ron"));
    assert_eq!(manifest.exports.combat, ["combat/damage.ron"]);
    assert_eq!(manifest.exports.match_files, ["match/match.ron"]);
    assert_eq!(manifest.exports.visual_effects, ["fx/effects.ron"]);
    let _: CombatDef = parse("damage", &read("combat/damage.ron"));
    let _: MatchDef = parse("match", &read("match/match.ron"));
    for file in ["diver", "dock", "lancer", "skimmer", "spire"] {
        let _: UnitDef = parse(file, &read(&format!("units/{file}.ron")));
    }
    for file in ["current", "pearls", "shells"] {
        let _: ResourceDef = parse(file, &read(&format!("resources/{file}.ron")));
    }
    for file in ["train_diver", "build_spire"] {
        let _: ProductionDef = parse(file, &read(&format!("production/{file}.ron")));
    }
    for file in ["order_construct", "order_surrender"] {
        let _: OrderDef = parse(file, &read(&format!("orders/{file}.ron")));
    }
    let _: OrderRulesDef = parse("rules", &read("orders/rules.ron"));
    let _: MapDef = parse("reef", &read("maps/reef.ron"));
    // The HUD fragment: the new fields of an interface file. Its last field,
    // `result_widget_guess`, is not in the design (see the test docs); the rest is placed in
    // an interface file as it is.
    let fragment = read("interface/hud_fragment.ron");
    let (fields, guess) = fragment
        .split_once("\n result_widget_guess")
        .expect("the fragment ends with the guess");
    assert!(guess.contains("Bind(Outcome)"), "{guess}");
    interface(Parts {
        top: fields.strip_prefix('(').expect("a struct"),
        no_alerts: true,
        ..Parts::default()
    });
}
