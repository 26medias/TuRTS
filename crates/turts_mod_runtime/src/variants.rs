//! The uses of the 0.2.4 variants and combinations in the parsed content (design 6.10,
//! "Version-table detection": kinds VAR and COMBO).
//!
//! `ron::Value` drops the name of an enum variant, so the raw document cannot show a variant.
//! The parsed value holds it, and no default writes a new variant, so a parsed variant is a
//! written variant. This module walks the parsed files once and lists each use with the file and
//! the field path where it is written. The rows of `version_table.rs` and the lines of
//! `unbuilt.rs` ask for a use by its name (`Detect::Variant`).
//!
//! One name, one meaning: the name is the text of the row in the message.

use std::collections::BTreeSet;

use turts_mod_api::PaymentPolicy;
use turts_mod_api::{
    AlertTrigger, Behaviour, ButtonAction, ButtonIcon, Capability, DefeatClause, GroundRally,
    InterfaceDef, ItemField, Manifest, MatchDef, OrderRulesDef, Predicate, PrimaryField, Primitive,
    ResourceKind, SlotRelation, SmartOrder, SmartTarget, Source, StartPolicy, TargetKind,
    TextValue, UnitDef, Value, Widget,
};

use crate::content_mc_validation::ParsedMcContent;

/// One use: the name of the row, the file, and the field path in the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VariantUse {
    pub(crate) name: &'static str,
    pub(crate) file: String,
    pub(crate) path: String,
}

struct Uses {
    list: Vec<VariantUse>,
}

impl Uses {
    fn push(&mut self, name: &'static str, file: &str, path: impl Into<String>) {
        self.list.push(VariantUse {
            name,
            file: file.to_string(),
            path: path.into(),
        });
    }
}

/// Every use of a 0.2.4 variant or combination in `content`, in file order.
pub(crate) fn collect(_manifest: &Manifest, content: &ParsedMcContent) -> Vec<VariantUse> {
    let mut uses = Uses { list: Vec::new() };
    for (file, unit) in &content.units {
        unit_uses(&mut uses, file, unit);
    }
    static_outputs(&mut uses, content);
    for (file, resource) in &content.resources {
        if matches!(resource.kind, ResourceKind::Supply { .. }) {
            uses.push("kind: Supply", file, "kind");
        }
    }
    for (file, production) in &content.production {
        if production.payment == PaymentPolicy::ChargeOverTime {
            uses.push("payment: ChargeOverTime", file, "payment");
        }
    }
    for (file, technology) in &content.technologies {
        if technology.payment == PaymentPolicy::ChargeOverTime {
            uses.push("payment: ChargeOverTime", file, "payment");
        }
    }
    for (file, rules) in &content.order_rules {
        rule_uses(&mut uses, content, file, rules);
    }
    for (file, order) in &content.orders {
        if order.target == TargetKind::ContentAtPoint {
            uses.push("target: ContentAtPoint", file, "target");
        }
        let name = match order.behaviour {
            Behaviour::Construct => Some("behaviour Construct"),
            Behaviour::CancelConstruction => Some("behaviour CancelConstruction"),
            Behaviour::SetRally => Some("behaviour SetRally"),
            Behaviour::Repair => Some("behaviour Repair"),
            Behaviour::Surrender => Some("behaviour Surrender"),
            _ => None,
        };
        if let Some(name) = name {
            uses.push(name, file, "behaviour");
        }
    }
    for (file, interface) in &content.interfaces {
        interface_uses(&mut uses, file, interface);
    }
    for (file, definition) in &content.matches {
        match_uses(&mut uses, file, definition);
    }
    uses.list
}

fn relation(uses: &mut Uses, file: &str, slots: SlotRelation, path: &str) {
    match slots {
        SlotRelation::Ally => uses.push("SlotRelation::Ally", file, path),
        SlotRelation::Enemy => uses.push("SlotRelation::Enemy", file, path),
        SlotRelation::Other | SlotRelation::Same | SlotRelation::Any => {}
    }
}

fn unit_uses(uses: &mut Uses, file: &str, unit: &UnitDef) {
    if let Some(weapon) = &unit.weapon {
        relation(uses, file, weapon.targets.slots, "weapon.targets.slots");
    }
    if let Some(repairer) = &unit.repairer {
        relation(uses, file, repairer.targets.slots, "repairer.targets.slots");
    }
    if unit
        .spawn_order
        .as_ref()
        .is_some_and(|order| order.ground_rally == GroundRally::ApplyFirst)
    {
        uses.push("ground_rally: ApplyFirst", file, "spawn_order.ground_rally");
    }
    // COMBO rows: a combination of fields on one unit (rows K1, K7, K8).
    if unit.health.is_some() && unit.removal.is_none() {
        uses.push("health without removal", file, "health");
    }
    let has_queue = unit.producer.is_some()
        || unit.machine.is_some()
        || unit.research.is_some()
        || unit.constructor.is_some();
    if unit.removal.is_some() && has_queue {
        uses.push("removal with a queue", file, "removal");
    }
    if unit.constructor.is_some() && unit.producer.is_some() {
        uses.push("constructor beside producer", file, "constructor");
    }
}

/// The part of an id after the last `:`: the local id.
fn local(reference: &str) -> &str {
    reference.rsplit(':').next().unwrap_or(reference)
}

/// COMBO (row K4): a production id of `constructor.buildings` whose output unit is static. The
/// ids are compared as local ids inside this package; a production in another package is not
/// seen.
fn static_outputs(uses: &mut Uses, content: &ParsedMcContent) {
    let buildings: BTreeSet<&str> = content
        .units
        .iter()
        .filter_map(|(_, unit)| unit.constructor.as_ref())
        .flat_map(|constructor| constructor.buildings.iter().map(|id| local(id)))
        .collect();
    for (file, production) in &content.production {
        if !buildings.contains(production.id.as_str()) {
            continue;
        }
        let output = local(&production.output_unit);
        let is_static = content
            .units
            .iter()
            .any(|(_, unit)| unit.id == output && unit.movement.is_none());
        if is_static {
            uses.push("production with a static output", file, "output_unit");
        }
    }
}

fn rule_uses(uses: &mut Uses, content: &ParsedMcContent, file: &str, rules: &OrderRulesDef) {
    let lists: [(&str, Option<&Vec<SmartOrder>>); 2] = [
        ("smart", Some(&rules.smart)),
        ("rally", rules.rally.as_ref()),
    ];
    for (name, list) in lists {
        for (index, entry) in list.into_iter().flatten().enumerate() {
            let path = format!("{name}[{index}]");
            if entry.target == SmartTarget::OwnDamaged {
                uses.push("smart target OwnDamaged", file, format!("{path}.target"));
            }
            match entry.needs {
                Capability::Constructor => {
                    uses.push("needs: Constructor", file, format!("{path}.needs"));
                }
                Capability::Repairer => {
                    uses.push("needs: Repairer", file, format!("{path}.needs"));
                }
                _ => {}
            }
            let sets_rally = content.orders.iter().any(|(_, order)| {
                order.id == local(&entry.order) && order.behaviour == Behaviour::SetRally
            });
            if entry.target == SmartTarget::Ground
                && entry.needs == Capability::Producer
                && sets_rally
            {
                uses.push("smart Ground with needs: Producer", file, path);
            }
        }
    }
}

fn match_uses(uses: &mut Uses, file: &str, definition: &MatchDef) {
    match definition.start {
        StartPolicy::Immediate => {}
        StartPolicy::AllSeated { countdown_secs } if countdown_secs > 0 => {
            uses.push("AllSeated with countdown_secs above 0", file, "start");
        }
        StartPolicy::AllSeated { .. } => {}
        StartPolicy::AllSeatedReady { .. } => uses.push("AllSeatedReady", file, "start"),
    }
    if let Some(turts_mod_api::VictoryRule::LastTeamStanding { defeat }) = &definition.victory {
        for (index, clause) in defeat.iter().enumerate() {
            if matches!(clause, DefeatClause::DisconnectedFor { .. }) {
                uses.push("DisconnectedFor", file, format!("victory.defeat[{index}]"));
            }
        }
    }
}

// ---- The interface file ----

struct InterfaceWalk<'a> {
    uses: &'a mut Uses,
    file: &'a str,
}

fn interface_uses(uses: &mut Uses, file: &str, interface: &InterfaceDef) {
    let mut walk = InterfaceWalk { uses, file };
    for (index, alert) in interface.alerts.iter().enumerate() {
        let path = format!("alerts[{index}]");
        let trigger = match &alert.trigger {
            AlertTrigger::ConstructionComplete => Some("AlertTrigger::ConstructionComplete"),
            AlertTrigger::SupplyShort(_) => Some("AlertTrigger::SupplyShort"),
            AlertTrigger::SupplyRestored(_) => Some("AlertTrigger::SupplyRestored"),
            AlertTrigger::InsufficientFunds(_) => Some("AlertTrigger::InsufficientFunds"),
            AlertTrigger::SlotDisconnected => Some("AlertTrigger::SlotDisconnected"),
            AlertTrigger::SlotReconnected => Some("AlertTrigger::SlotReconnected"),
            AlertTrigger::GathererIdle(_) => Some("AlertTrigger::GathererIdle"),
            _ => None,
        };
        if let Some(name) = trigger {
            walk.uses.push(name, file, format!("{path}.trigger"));
        }
        // Edition row T4: a placeholder is replaced from the edition 0.2.4 on. The list does not
        // look at the edition: `unbuilt.rs` asks for the edition 0.2.4 only, and no version
        // row names a placeholder.
        walk.placeholders(&alert.text, &format!("{path}.text"), true);
    }
    for widget in &interface.root {
        walk.widget(widget, &format!("root[{}]", widget.id));
    }
    for page in &interface.guide {
        for widget in &page.widgets {
            walk.widget(
                widget,
                &format!("guide[{}].widgets[{}]", page.id, widget.id),
            );
        }
    }
}

impl InterfaceWalk<'_> {
    fn push(&mut self, name: &'static str, path: &str) {
        self.uses.push(name, self.file, path);
    }

    /// `{unit}` (alert texts only) and `{slot}` (alert texts and `Format` texts).
    fn placeholders(&mut self, text: &str, path: &str, alert: bool) {
        if alert && text.contains("{unit}") {
            self.push("placeholder {unit}", path);
        }
        if text.contains("{slot}") {
            self.push("placeholder {slot}", path);
        }
    }

    fn widget(&mut self, widget: &Widget, path: &str) {
        for predicate in &widget.visible {
            self.predicate(predicate, &format!("{path}.visible"));
        }
        if let Some(value) = &widget.tooltip {
            self.value(value, &format!("{path}.tooltip"));
        }
        match &widget.widget {
            Primitive::Box { children, .. } => {
                for child in children {
                    self.widget(child, &format!("{path}.widget.children[{}]", child.id));
                }
            }
            Primitive::Image { image, .. } => self.icon(image, &format!("{path}.widget.image")),
            Primitive::Text { text, .. } => self.text(text, &format!("{path}.widget.text")),
            Primitive::Bar { value, max, .. } => {
                self.value(value, &format!("{path}.widget.value"));
                self.value(max, &format!("{path}.widget.max"));
            }
            Primitive::Button {
                icon,
                label,
                action,
                font,
                size,
                ..
            } => {
                if let Some(icon) = icon {
                    self.icon(icon, &format!("{path}.widget.icon"));
                }
                if let Some(label) = label {
                    self.text(label, &format!("{path}.widget.label"));
                }
                if matches!(action, ButtonAction::Input(name) if name == "leave_match") {
                    self.push(
                        "engine action leave_match",
                        &format!("{path}.widget.action"),
                    );
                }
                if font.is_written() {
                    self.push("Button.font", &format!("{path}.widget.font"));
                }
                if size.is_written() {
                    self.push("Button.size", &format!("{path}.widget.size"));
                }
            }
            Primitive::List { source, item, .. } | Primitive::Grid { source, item, .. } => {
                self.source(source, &format!("{path}.widget.source"));
                self.widget(item, &format!("{path}.widget.item"));
            }
            Primitive::Tooltip { value } => self.value(value, &format!("{path}.widget.value")),
            Primitive::Cursor { .. } | Primitive::MinimapView { .. } => {}
        }
    }

    fn icon(&mut self, icon: &ButtonIcon, path: &str) {
        if let ButtonIcon::Bind(value) = icon {
            self.value(value, path);
        }
    }

    fn text(&mut self, text: &TextValue, path: &str) {
        match text {
            TextValue::Literal(_) => {}
            TextValue::Bind(value) => self.value(value, path),
            TextValue::Item(field) => self.item_field(*field, path),
            TextValue::Format(pattern, values) => {
                self.placeholders(pattern, path, false);
                for value in values {
                    self.value(value, path);
                }
            }
        }
    }

    fn value(&mut self, value: &Value, path: &str) {
        let name = match value {
            Value::Supply(..) => Some("Value::Supply"),
            Value::ConstructionQueue => Some("Value::ConstructionQueue"),
            Value::Sites => Some("Value::Sites"),
            Value::Notices => Some("Value::Notices"),
            Value::SelectionTypes => Some("Value::SelectionTypes"),
            Value::Connection => Some("Value::Connection"),
            Value::Outcome => Some("Value::Outcome"),
            Value::OutcomeReason => Some("Value::OutcomeReason"),
            Value::MatchStats(_) => Some("Value::MatchStats"),
            Value::MatchClock => Some("Value::MatchClock"),
            Value::Hint => Some("Value::Hint"),
            _ => None,
        };
        if let Some(name) = name {
            self.push(name, path);
        }
        match value {
            Value::Item(field) => self.item_field(*field, path),
            Value::SelectionPrimary(field) => self.primary_field(*field, path),
            Value::Format(pattern, values) => {
                self.placeholders(pattern, path, false);
                for value in values {
                    self.value(value, path);
                }
            }
            _ => {}
        }
    }

    fn source(&mut self, source: &Source, path: &str) {
        let name = match source {
            Source::ConstructionQueue => Some("Source::ConstructionQueue"),
            Source::Sites => Some("Source::Sites"),
            Source::Notices => Some("Source::Notices"),
            Source::SelectionTypes => Some("Source::SelectionTypes"),
            _ => None,
        };
        if let Some(name) = name {
            self.push(name, path);
        }
    }

    fn primary_field(&mut self, field: PrimaryField, path: &str) {
        let name = match field {
            PrimaryField::Activity => Some("PrimaryField::Activity"),
            PrimaryField::Cargo => Some("PrimaryField::Cargo"),
            PrimaryField::Armour => Some("PrimaryField::Armour"),
            _ => None,
        };
        if let Some(name) = name {
            self.push(name, path);
        }
    }

    fn item_field(&mut self, field: ItemField, path: &str) {
        let name = match field {
            ItemField::Role => Some("ItemField::Role"),
            ItemField::BuildTimeNow => Some("ItemField::BuildTimeNow"),
            ItemField::SupplyChange => Some("ItemField::SupplyChange"),
            ItemField::CausesShortage => Some("ItemField::CausesShortage"),
            ItemField::State => Some("ItemField::State"),
            ItemField::Paid => Some("ItemField::Paid"),
            ItemField::Refund => Some("ItemField::Refund"),
            ItemField::Badge => Some("ItemField::Badge"),
            _ => None,
        };
        if let Some(name) = name {
            self.push(name, path);
        }
    }

    fn predicate(&mut self, predicate: &Predicate, path: &str) {
        let name = match predicate {
            Predicate::ItemHas(_) => Some("Predicate::ItemHas"),
            Predicate::PrimaryHasValue(_) => Some("Predicate::PrimaryHasValue"),
            Predicate::SupplyShort(_) => Some("Predicate::SupplyShort"),
            Predicate::MatchOver => Some("Predicate::MatchOver"),
            Predicate::OutcomeIs(_) => Some("Predicate::OutcomeIs"),
            Predicate::OutcomeReasonIs(_) => Some("Predicate::OutcomeReasonIs"),
            Predicate::Placing => Some("Predicate::Placing"),
            Predicate::HintShown => Some("Predicate::HintShown"),
            _ => None,
        };
        if let Some(name) = name {
            self.push(name, path);
        }
        match predicate {
            Predicate::ItemHas(field) => self.item_field(*field, path),
            Predicate::PrimaryHasValue(field) => self.primary_field(*field, path),
            Predicate::Not(inner) => self.predicate(inner, path),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{VariantUse, collect};
    use crate::content_mc_validation::ParsedMcContent;
    use turts_mod_api::{
        InterfaceDef, Manifest, MatchDef, OrderDef, OrderRulesDef, ProductionDef, ResourceDef,
        TechnologyDef, UnitDef,
    };

    fn manifest() -> Manifest {
        ron::from_str(
            r#"(id: "zeta", version: "0.1.0", kind: Game, engine_api: "^0.2.4", dependencies: {}, exports: (units: [], maps: [], visuals: [], resources: [], nodes: [], production: []), context: Shared, capabilities: [], license: "test")"#,
        )
        .unwrap()
    }

    const UNIT: &str = r#"id: "u", name: "U", footprint: Circle(radius: 1.0), movement: None, inventory: None, gatherer: None, drop_off: None, producer: None"#;
    const WEAPON: &str = "range: 5.0, cooldown_ms: 900, damage: 8, ammo: None";

    fn unit(tail: &str) -> UnitDef {
        ron::from_str(&format!("({UNIT}, {tail})")).unwrap_or_else(|e| panic!("{tail}: {e}"))
    }

    fn found(content: &ParsedMcContent) -> Vec<(&'static str, String, String)> {
        collect(&manifest(), content)
            .into_iter()
            .map(|VariantUse { name, file, path }| (name, file, path))
            .collect()
    }

    fn one(content: &ParsedMcContent, name: &str, file: &str, path: &str) {
        let uses = found(content);
        assert!(
            uses.iter()
                .any(|(n, f, p)| *n == name && f == file && p == path),
            "`{name}` at {file}: {path} is not in {uses:?}"
        );
    }

    #[test]
    fn nothing_is_a_use_in_an_empty_package() {
        assert!(found(&ParsedMcContent::default()).is_empty());
    }

    #[test]
    fn unit_variants_and_combinations() {
        let units = [
            (
                format!("weapon: Some(({WEAPON}, targets: (slots: Ally)))"),
                "SlotRelation::Ally",
                "weapon.targets.slots",
            ),
            (
                format!("weapon: Some(({WEAPON}, targets: (slots: Enemy)))"),
                "SlotRelation::Enemy",
                "weapon.targets.slots",
            ),
            (
                "repairer: Some((rate_per_second: 1, reach: 1.0, cost: [], targets: (slots: Enemy)))".into(),
                "SlotRelation::Enemy",
                "repairer.targets.slots",
            ),
            (
                r#"spawn_order: Some((order: "a:o", target: NearestNode, ground_rally: ApplyFirst))"#.into(),
                "ground_rally: ApplyFirst",
                "spawn_order.ground_rally",
            ),
            (
                "health: Some((max: 5))".into(),
                "health without removal",
                "health",
            ),
            (
                "removal: Some((by_command: true, contents: Destroy)), machine: Some((recipe: \"a:r\"))".into(),
                "removal with a queue",
                "removal",
            ),
            (
                "constructor: Some((buildings: [], concurrent_sites: 1, queue_capacity: 1))".into(),
                "constructor beside producer",
                "constructor",
            ),
        ];
        for (tail, name, path) in units {
            let tail = if name == "constructor beside producer" {
                // A producer and a constructor on one unit.
                let text = format!(
                    "({}, producer: Some((recipes: [], queue_capacity: 1, spawn_offsets: [])), {tail})",
                    UNIT.replace(", producer: None", "")
                );
                let unit: UnitDef = ron::from_str(&text).unwrap();
                ParsedMcContent {
                    units: vec![("units/u.ron".into(), unit)],
                    ..ParsedMcContent::default()
                }
            } else {
                ParsedMcContent {
                    units: vec![("units/u.ron".into(), unit(&tail))],
                    ..ParsedMcContent::default()
                }
            };
            one(&tail, name, "units/u.ron", path);
        }
        // A unit with a weapon aimed at `Other`, and with health and removal, is no use.
        let plain = ParsedMcContent {
            units: vec![(
                "units/u.ron".into(),
                unit(&format!(
                    "weapon: Some(({WEAPON}, targets: (slots: Other))), health: Some((max: 5)), removal: Some((by_command: true, contents: Destroy))"
                )),
            )],
            ..ParsedMcContent::default()
        };
        assert!(found(&plain).is_empty(), "{:?}", found(&plain));
    }

    #[test]
    fn a_production_with_a_static_output_in_a_constructor_list_is_a_use() {
        let building: UnitDef = unit("");
        let constructor: UnitDef = unit(
            r#"constructor: Some((buildings: ["zeta:build_tower"], concurrent_sites: 1, queue_capacity: 1))"#,
        );
        let mut tower = building;
        tower.id = "tower".into();
        let production: ProductionDef = ron::from_str(
            r#"(id: "build_tower", name: "T", output_unit: "zeta:tower", duration_ms: 1, costs: [], payment: ChargeOnQueue, cancel_refunds: (queued: (numerator: 1, denominator: 1), running: (numerator: 1, denominator: 1), output_blocked: (numerator: 1, denominator: 1)))"#,
        )
        .unwrap();
        let content = ParsedMcContent {
            units: vec![
                ("units/c.ron".into(), constructor),
                ("units/tower.ron".into(), tower),
            ],
            production: vec![("production/t.ron".into(), production)],
            ..ParsedMcContent::default()
        };
        one(
            &content,
            "production with a static output",
            "production/t.ron",
            "output_unit",
        );
    }

    #[test]
    fn resource_payment_order_and_rule_variants() {
        let supply: ResourceDef = ron::from_str(
            r#"(id: "p", name: "P", unit_name: "w", kind: Supply(scope: Slot, counts: Completed))"#,
        )
        .unwrap();
        let production = |payment: &str| -> ProductionDef {
            ron::from_str(&format!(
                r#"(id: "p", name: "P", output_unit: "a:u", duration_ms: 1, costs: [], payment: {payment}, cancel_refunds: (queued: (numerator: 1, denominator: 1), running: (numerator: 1, denominator: 1), output_blocked: (numerator: 1, denominator: 1)))"#
            ))
            .unwrap()
        };
        let technology: TechnologyDef = ron::from_str(
            r#"(id: "t", name: "T", prerequisites: [], duration_ms: 1, costs: [], payment: ChargeOverTime, cancel_refunds: (queued: (numerator: 1, denominator: 1), running: (numerator: 1, denominator: 1), output_blocked: (numerator: 1, denominator: 1)), effects: [])"#,
        )
        .unwrap();
        let order = |behaviour: &str, target: &str| -> OrderDef {
            ron::from_str(&format!(
                r#"(id: "o", name: "O", behaviour: {behaviour}, target: {target}, fire: None, engage: None, icon: "i", hotkey: "k", cursor: None, card: None, units: [], description: "d")"#
            ))
            .unwrap()
        };
        let content = ParsedMcContent {
            resources: vec![("resources/p.ron".into(), supply)],
            production: vec![
                ("production/a.ron".into(), production("ChargeOnQueue")),
                ("production/b.ron".into(), production("ChargeOverTime")),
            ],
            technologies: vec![("technologies/t.ron".into(), technology)],
            orders: vec![
                ("orders/a.ron".into(), order("Construct", "ContentAtPoint")),
                ("orders/b.ron".into(), order("CancelConstruction", "None")),
                ("orders/c.ron".into(), order("SetRally", "Point")),
                ("orders/d.ron".into(), order("Repair", "Unit")),
                ("orders/e.ron".into(), order("Surrender", "None")),
                ("orders/f.ron".into(), order("Move", "Point")),
            ],
            ..ParsedMcContent::default()
        };
        one(&content, "kind: Supply", "resources/p.ron", "kind");
        one(
            &content,
            "payment: ChargeOverTime",
            "production/b.ron",
            "payment",
        );
        one(
            &content,
            "payment: ChargeOverTime",
            "technologies/t.ron",
            "payment",
        );
        one(&content, "target: ContentAtPoint", "orders/a.ron", "target");
        for (name, file) in [
            ("behaviour Construct", "orders/a.ron"),
            ("behaviour CancelConstruction", "orders/b.ron"),
            ("behaviour SetRally", "orders/c.ron"),
            ("behaviour Repair", "orders/d.ron"),
            ("behaviour Surrender", "orders/e.ron"),
        ] {
            one(&content, name, file, "behaviour");
        }
        // The control: `ChargeOnQueue` and `Move` are no use.
        let uses = found(&content);
        assert!(
            !uses
                .iter()
                .any(|(_, f, _)| f == "production/a.ron" || f == "orders/f.ron")
        );
    }

    #[test]
    fn smart_rules_list_the_new_targets_needs_and_the_rally_rule() {
        let rules: OrderRulesDef = ron::from_str(
            r#"(queue_depth: 16, smart: [(target: OwnDamaged, needs: Repairer, order: "a:rep"), (target: Ground, needs: Constructor, order: "a:build"), (target: Ground, needs: Producer, order: "a:rally"), (target: Ground, needs: Producer, order: "a:produce")], rally: Some([(target: OwnDamaged, needs: Movement, order: "a:rep")]))"#,
        )
        .unwrap();
        let order = |id: &str, behaviour: &str, target: &str| -> OrderDef {
            ron::from_str(&format!(
                r#"(id: "{id}", name: "O", behaviour: {behaviour}, target: {target}, fire: None, engage: None, icon: "i", hotkey: "k", cursor: None, card: None, units: [], description: "d")"#
            ))
            .unwrap()
        };
        let content = ParsedMcContent {
            order_rules: vec![("orders/rules.ron".into(), rules)],
            orders: vec![
                (
                    "orders/rally.ron".into(),
                    order("rally", "SetRally", "Point"),
                ),
                (
                    "orders/produce.ron".into(),
                    order("produce", "Produce", "Content"),
                ),
            ],
            ..ParsedMcContent::default()
        };
        let file = "orders/rules.ron";
        one(&content, "smart target OwnDamaged", file, "smart[0].target");
        one(&content, "needs: Repairer", file, "smart[0].needs");
        one(&content, "needs: Constructor", file, "smart[1].needs");
        // `Ground` with `Producer` is a use only when the order is a `SetRally` order.
        one(
            &content,
            "smart Ground with needs: Producer",
            file,
            "smart[2]",
        );
        assert!(
            !found(&content)
                .iter()
                .any(|(n, _, p)| *n == "smart Ground with needs: Producer" && p == "smart[3]")
        );
        // The rally list is read as the smart list is.
        one(&content, "smart target OwnDamaged", file, "rally[0].target");
    }

    #[test]
    fn match_file_variants() {
        let definition = |text: &str| -> MatchDef { ron::from_str(text).unwrap() };
        let content = ParsedMcContent {
            matches: vec![
                (
                    "match/a.ron".into(),
                    definition("(start: AllSeated(countdown_secs: 0))"),
                ),
                (
                    "match/b.ron".into(),
                    definition("(start: AllSeated(countdown_secs: 3))"),
                ),
                (
                    "match/c.ron".into(),
                    definition("(start: AllSeatedReady(countdown_secs: 0))"),
                ),
                (
                    "match/d.ron".into(),
                    definition(
                        r#"(victory: Some(LastTeamStanding(defeat: [KeyUnitsLost(units: ["a:b"]), DisconnectedFor(secs: 9)])))"#,
                    ),
                ),
            ],
            ..ParsedMcContent::default()
        };
        let uses = found(&content);
        assert!(
            !uses.iter().any(|(_, f, _)| f == "match/a.ron"),
            "countdown 0 is built"
        );
        one(
            &content,
            "AllSeated with countdown_secs above 0",
            "match/b.ron",
            "start",
        );
        one(&content, "AllSeatedReady", "match/c.ron", "start");
        one(
            &content,
            "DisconnectedFor",
            "match/d.ron",
            "victory.defeat[1]",
        );
    }

    const INTERFACE: &str = r##"(id: "hud", root: [@ROOT@], tooltip_style: (background: "#000000", colour: "#ffffff", font: None, size: 12), cursors: {}, camera: (pitch: 0.7, yaw: 0.0, field_of_view: 60.0, minimum: 10.0, maximum: 100.0, start: MapCentre, height: 40.0, margin: 0.0, pan_speed: 1.0, edge_band: true), light: None, slot_colours: [], world: (ground: "#000000", grid: "#ffffff", selection: "#ffffff", health: "#00ff00", range: "#ffffff", show_range: true, tooltip: (background: "#000000", colour: "#ffffff", font: None, size: 12), health_bar: (width: 32, height: 4, background: "#000000", health: "#00ff00")), alerts: [@ALERTS@], reasons: {}, units: {}, selection_priority: [], guide: [])"##;

    fn interface(root: &str, alerts: &str) -> ParsedMcContent {
        let text = INTERFACE
            .replace("@ROOT@", root)
            .replace("@ALERTS@", alerts);
        let interface: InterfaceDef =
            ron::from_str(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
        ParsedMcContent {
            interfaces: vec![("interface/hud.ron".into(), interface)],
            ..ParsedMcContent::default()
        }
    }

    fn widget(slot: &str, visible: &str) -> String {
        format!(
            r#"(id: "w", at: Some((anchor: TopLeft, offset: (0.0, 0.0), pivot: None)), size: (w: Px(1.0), h: Px(1.0)), visible: [{visible}], widget: {slot})"#
        )
    }

    fn text(value: &str) -> String {
        widget(
            &format!(
                r##"Text(text: {value}, font: None, size: 1, colour: "#fff", align: Left, wrap: false)"##
            ),
            "",
        )
    }

    #[test]
    fn widget_variants_are_found_wherever_a_value_can_stand() {
        // The text of a Text widget, a Bar, a tooltip, a button label, a button icon, a nested
        // Format, a guide page: each place that holds a `Value`.
        let places = [
            (text("Bind(Outcome)"), "root[w].widget.text"),
            (
                widget(
                    r##"Bar(value: Supply("a:b", Consumed), max: Literal("1"), fill: Colour("#fff"), background: Colour("#000"), direction: Right)"##,
                    "",
                ),
                "root[w].widget.value",
            ),
            (
                widget(
                    r##"Bar(value: Literal("1"), max: Hint, fill: Colour("#fff"), background: Colour("#000"), direction: Right)"##,
                    "",
                ),
                "root[w].widget.max",
            ),
            (
                widget(r#"Tooltip(value: MatchClock)"#, ""),
                "root[w].widget.value",
            ),
            (
                widget(
                    r#"Button(icon: Some(Bind(Sites)), label: None, action: Input("menu"), hotkey_label: false)"#,
                    "",
                ),
                "root[w].widget.icon",
            ),
            (
                widget(
                    r#"Button(icon: None, label: Some(Bind(Notices)), action: Input("menu"), hotkey_label: false)"#,
                    "",
                ),
                "root[w].widget.label",
            ),
            (
                text(r#"Format("{0}", [Format("{0}", [Connection])])"#),
                "root[w].widget.text",
            ),
        ];
        let names = [
            "Value::Outcome",
            "Value::Supply",
            "Value::Hint",
            "Value::MatchClock",
            "Value::Sites",
            "Value::Notices",
            "Value::Connection",
        ];
        for ((root, path), name) in places.iter().zip(names) {
            one(&interface(root, ""), name, "interface/hud.ron", path);
        }
        // A tooltip on the widget.
        let tip = r#"(id: "w", at: None, size: (w: Px(1.0), h: Px(1.0)), tooltip: Some(SelectionTypes), widget: Box(fill: None, border: None, children: []))"#;
        one(
            &interface(tip, ""),
            "Value::SelectionTypes",
            "interface/hud.ron",
            "root[w].tooltip",
        );
        // A child of a Box and the item of a List.
        let nested = widget(
            &format!(
                r#"Box(fill: None, border: None, children: [{}])"#,
                text("Bind(OutcomeReason)")
            ),
            "",
        );
        one(
            &interface(&nested, ""),
            "Value::OutcomeReason",
            "interface/hud.ron",
            "root[w].widget.children[w].widget.text",
        );
        let list = widget(
            &format!(
                r#"List(source: Sites, direction: Vertical, gap: 1, max_items: 1, item: {})"#,
                text("Item(Paid)")
            ),
            "",
        );
        let content = interface(&list, "");
        one(
            &content,
            "Source::Sites",
            "interface/hud.ron",
            "root[w].widget.source",
        );
        one(
            &content,
            "ItemField::Paid",
            "interface/hud.ron",
            "root[w].widget.item.widget.text",
        );
        let grid = widget(
            &format!(
                r#"Grid(source: ConstructionQueue, columns: 1, rows: 1, cell: (1, 1), gap: 1, item: {})"#,
                text("Bind(SelectionPrimary(Cargo))")
            ),
            "",
        );
        let content = interface(&grid, "");
        one(
            &content,
            "Source::ConstructionQueue",
            "interface/hud.ron",
            "root[w].widget.source",
        );
        one(
            &content,
            "PrimaryField::Cargo",
            "interface/hud.ron",
            "root[w].widget.item.widget.text",
        );
    }

    #[test]
    fn predicates_item_and_primary_fields_and_nested_not() {
        let visible = text("Literal(\"x\")").replace("visible: []", "visible: [Not(Placing), ItemHas(Badge), PrimaryHasValue(Armour), SupplyShort(\"a:b\"), OutcomeReasonIs(Surrendered)]");
        let content = interface(&visible, "");
        for name in [
            "Predicate::Placing",
            "Predicate::ItemHas",
            "ItemField::Badge",
            "Predicate::PrimaryHasValue",
            "PrimaryField::Armour",
            "Predicate::SupplyShort",
            "Predicate::OutcomeReasonIs",
        ] {
            one(&content, name, "interface/hud.ron", "root[w].visible");
        }
    }

    #[test]
    fn button_font_size_and_the_leave_match_action() {
        let button = |tail: &str, action: &str| {
            widget(
                &format!(
                    r#"Button(icon: None, label: None, action: {action}, hotkey_label: false{tail})"#
                ),
                "",
            )
        };
        let content = interface(&button(", font: None", r#"Input("leave_match")"#), "");
        one(
            &content,
            "Button.font",
            "interface/hud.ron",
            "root[w].widget.font",
        );
        one(
            &content,
            "engine action leave_match",
            "interface/hud.ron",
            "root[w].widget.action",
        );
        assert!(!found(&content).iter().any(|(n, _, _)| *n == "Button.size"));
        let content = interface(&button(", size: Some(12)", r#"Input("menu")"#), "");
        one(
            &content,
            "Button.size",
            "interface/hud.ron",
            "root[w].widget.size",
        );
        assert!(
            !found(&interface(&button("", r#"Input("menu")"#), ""))
                .iter()
                .any(|(n, _, _)| n.starts_with("Button"))
        );
    }

    #[test]
    fn alert_triggers_and_placeholders() {
        let alert = |trigger: &str, text: &str| {
            format!(
                r#"(id: "a", trigger: {trigger}, sound: None, text: "{text}", minimap_ping: false, cooldown: 1.0, priority: 0, coalesce_radius: 0.0)"#
            )
        };
        for (trigger, name) in [
            ("ConstructionComplete", "AlertTrigger::ConstructionComplete"),
            (r#"SupplyShort("a:b")"#, "AlertTrigger::SupplyShort"),
            (r#"SupplyRestored("a:b")"#, "AlertTrigger::SupplyRestored"),
            (
                r#"InsufficientFunds("a:b")"#,
                "AlertTrigger::InsufficientFunds",
            ),
            ("SlotDisconnected", "AlertTrigger::SlotDisconnected"),
            ("SlotReconnected", "AlertTrigger::SlotReconnected"),
            ("GathererIdle(5.0)", "AlertTrigger::GathererIdle"),
        ] {
            let content = interface("", &alert(trigger, "x"));
            one(&content, name, "interface/hud.ron", "alerts[0].trigger");
        }
        // The triggers of today are no use.
        assert!(found(&interface("", &alert("OwnUnitDamaged", "x"))).is_empty());
        let content = interface("", &alert("OwnUnitDamaged", "{unit} hit {slot}"));
        one(
            &content,
            "placeholder {unit}",
            "interface/hud.ron",
            "alerts[0].text",
        );
        one(
            &content,
            "placeholder {slot}",
            "interface/hud.ron",
            "alerts[0].text",
        );
        // `{slot}` in a `Format` text; `{unit}` there is no placeholder (design 6.10 row H17).
        let format = text(r#"Format("{slot} {unit}", [])"#);
        let content = interface(&format, "");
        one(
            &content,
            "placeholder {slot}",
            "interface/hud.ron",
            "root[w].widget.text",
        );
        assert!(
            !found(&content)
                .iter()
                .any(|(n, _, _)| *n == "placeholder {unit}")
        );
    }
}
