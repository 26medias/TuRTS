//! The validation column of the 0.2.4 table (design 6.10, revision 6.2; task `L0-API2`).
//!
//! One function for each group of the table: the exports (A), the damage table (C), the fields of
//! a unit (U, S, K1 to K3, R1), the construction and payment rows (K4 to K8, P1), the orders (O),
//! the match file and the map (M), the interface (H) and the input map (KEY1). Each check has the
//! text of its row, in the form `<package>/<file>: <field path>: <message>`. A name lookup that
//! fails adds the hint `; did you mean `<name>`?` and the allowed list (`ModError` prints it).
//!
//! The checks apply to the edition 0.2.4 only (`edition.rs`): a package that admits an older API
//! keeps the behaviour of that API (rows T1 to T4) and hears about the version from the version
//! table. The function runs in `PackageArchive::content`, after the version table, the old
//! validation and the `UNBUILT_FEATURES` refusal (`unbuilt.rs`): a package that uses a feature
//! that is not built hears that first. The lane that builds a feature deletes its line, and the
//! checks of the feature are then already in force. A test reaches them while the line stands
//! through `PackageArchive::check_api_0_2_4`.
//!
//! Not done here, and why (each is a NOT TRACED of the task report): the damage table of a
//! dependency (the packer sees one package); the `total` that overflows u64 (the packer does not
//! know the `tick_hz` of `server.ron`, design 6.4); the GLB checks V1 and V2 (`PRES-MODELS`).

use std::collections::{BTreeMap, BTreeSet};

use turts_mod_api::SLOT_COUNT;
use turts_mod_api::keys::{CURSOR_CONTEXTS, REASON_KEYS, STATE_KEYS};
use turts_mod_api::{
    AlertDef, AlertKind, AlertTrigger, AssetsDef, Behaviour, Binding, ButtonIcon, Capability,
    CombatDef, ConstructProgress, DefeatClause, Footprint, HintCondition, HintDef, HintKind,
    InputMapDef, InterfaceDef, MapDef, MatchDef, ModError, OrderDef, OrderRulesDef, PackageId,
    PaymentPolicy, Predicate, Primitive, ProductionDef, RefundRatio, ResourceDef, ResourceKind,
    ShortEffect, SmartOrder, SmartTarget, SoundWhen, StartOrder, StartPolicy, StartTarget,
    StatField, SupplyCounts, TargetKind, TechnologyDef, TextValue, UnitDef, Value, VictoryRule,
    WhenShort, Widget,
};

use crate::archive::{MANIFEST_PATH, PackageArchive};
use crate::content::error;
use crate::edition::is_edition_0_2_4;
use crate::parse::nearest_name;

/// The most that the least common multiple of the `Rate` denominators may be (design 6.4).
const MAX_RATE_LCM: u128 = 65_536;
const PLACEHOLDERS_ALERT: [&str; 2] = ["{slot}", "{unit}"];
const PLACEHOLDERS_FORMAT: [&str; 1] = ["{slot}"];

/// The checks of the edition 0.2.4 on the server path (the gameplay, order, match, interface and
/// input files). `Ok` for a package of an older edition.
pub(crate) fn validate(archive: &PackageArchive) -> Result<(), ModError> {
    if !is_edition_0_2_4(&archive.manifest.engine_api) {
        return Ok(());
    }
    let pack = Pack::read(archive)?;
    pack.exports()?;
    pack.combat_group()?;
    pack.resources_group()?;
    pack.units_group()?;
    pack.production_group()?;
    pack.technologies_group()?;
    pack.rates()?;
    pack.orders_group()?;
    pack.maps_group()?;
    pack.match_group()?;
    pack.interface_group()?;
    pack.input_group()?;
    Ok(())
}

/// The checks of the edition 0.2.4 on the client path (rows V3 and V4: the effect files and the
/// effects of a visual). `PRES-MODELS` owns V1 and V2.
pub(crate) fn validate_presentation(archive: &PackageArchive) -> Result<(), ModError> {
    if !is_edition_0_2_4(&archive.manifest.engine_api) {
        return Ok(());
    }
    let pack = Pack::read(archive)?;
    let effect_files = archive.visual_effect_defs()?;
    let mut effect_ids = BTreeSet::new();
    let sounds = pack.sound_ids();
    for (file, definition) in &effect_files {
        for (index, effect) in definition.effects.iter().enumerate() {
            let path = format!("effects[{index}]");
            if !effect_ids.insert(effect.id.as_str()) {
                return Err(pack.fail(
                    file,
                    &format!("{path}.id"),
                    format!("duplicate effect id `{}`", effect.id),
                ));
            }
            pack.colour(file, &format!("{path}.colour"), &effect.colour)?;
            if let Some(sound) = &effect.sound
                && !sounds.contains(sound.as_str())
            {
                return Err(pack.unknown(
                    file,
                    &format!("{path}.sound"),
                    "sound asset",
                    sound,
                    sounds.iter().map(|name| name.to_string()).collect(),
                ));
            }
        }
    }
    let locals: Vec<String> = effect_ids.iter().map(|id| pack.qualified(id)).collect();
    for (file, visual) in archive.visuals_unchecked()? {
        let Some(effects) = &visual.effects else {
            continue;
        };
        let refs = [
            ("fire", effects.fire.as_ref()),
            ("hit", effects.hit.as_ref()),
            ("hit_reduced", effects.hit_reduced.as_ref()),
            ("death", effects.death.as_ref()),
            (
                "damaged.effect",
                effects.damaged.as_ref().map(|damaged| &damaged.effect),
            ),
        ];
        for (name, reference) in refs {
            let Some(reference) = reference else {
                continue;
            };
            pack.lookup(
                &file,
                &format!("effects.{name}"),
                "effect",
                reference,
                &locals,
            )?;
        }
        if let Some(damaged) = &effects.damaged {
            pack.ratio(&file, "effects.damaged.below", damaged.below)?;
        }
    }
    Ok(())
}

/// The parsed files of one package, read once.
struct Pack {
    package: PackageId,
    dependencies: BTreeSet<String>,
    units: Vec<(String, UnitDef)>,
    resources: Vec<(String, ResourceDef)>,
    production: Vec<(String, ProductionDef)>,
    technologies: Vec<(String, TechnologyDef)>,
    orders: Vec<(String, OrderDef)>,
    rules: Vec<(String, OrderRulesDef)>,
    interfaces: Vec<(String, InterfaceDef)>,
    inputs: Vec<(String, InputMapDef)>,
    assets: Vec<(String, AssetsDef)>,
    combat: Vec<(String, CombatDef)>,
    matches: Vec<(String, MatchDef)>,
    maps: Vec<(String, MapDef)>,
    manifest_exports_match: usize,
}

impl Pack {
    fn read(archive: &PackageArchive) -> Result<Self, ModError> {
        Ok(Self {
            package: archive.id(),
            dependencies: archive.manifest.dependencies.0.keys().cloned().collect(),
            units: archive.units()?,
            resources: archive.resources()?,
            production: archive.production()?,
            technologies: archive.technologies()?,
            orders: archive.orders()?,
            rules: archive.order_rules()?,
            interfaces: archive.interfaces()?,
            inputs: archive.input_maps()?,
            assets: archive.assets()?,
            combat: archive.combat_defs()?,
            matches: archive.match_defs()?,
            maps: archive.maps()?,
            manifest_exports_match: archive.manifest.exports.match_files.len(),
        })
    }

    // ---- Helpers ----

    fn fail(&self, file: &str, path: &str, message: impl Into<String>) -> ModError {
        error(&self.package, file, path, message)
    }

    fn qualified(&self, local: &str) -> String {
        format!("{}:{local}", self.package.0)
    }

    /// An unknown name: the message, the hint for the nearest name, and the allowed list.
    fn unknown(
        &self,
        file: &str,
        path: &str,
        kind: &str,
        given: &str,
        mut allowed: Vec<String>,
    ) -> ModError {
        allowed.sort();
        let hint = nearest_name(given, allowed.iter().map(String::as_str))
            .map(|name| format!("; did you mean `{name}`?"))
            .unwrap_or_default();
        let found = self.fail(file, path, format!("unknown {kind} `{given}`{hint}"));
        if allowed.is_empty() {
            found
        } else {
            found.with_allowed(allowed)
        }
    }

    /// Resolve a qualified reference to a local id of this package. `Ok(None)` for a name in a
    /// dependency (the packer sees one package).
    fn lookup(
        &self,
        file: &str,
        path: &str,
        kind: &str,
        reference: &str,
        locals: &[String],
    ) -> Result<Option<String>, ModError> {
        self.lookup_allowed(file, path, kind, reference, locals, locals)
    }

    /// `lookup` with an allowed list that is shorter than the set that resolves (the productions
    /// that can be a building).
    fn lookup_allowed(
        &self,
        file: &str,
        path: &str,
        kind: &str,
        reference: &str,
        locals: &[String],
        allowed: &[String],
    ) -> Result<Option<String>, ModError> {
        let id = turts_mod_api::ContentId::parse(reference)
            .map_err(|message| self.fail(file, path, message))?;
        if id.package != self.package {
            return if self.dependencies.contains(id.package.0.as_str()) {
                Ok(None)
            } else {
                Err(self.fail(
                    file,
                    path,
                    format!("unknown dependency namespace `{}`", id.package.0),
                ))
            };
        }
        if locals.iter().any(|local| *local == reference) {
            return Ok(Some(id.local));
        }
        Err(self.unknown(file, path, kind, reference, allowed.to_vec()))
    }

    fn colour(&self, file: &str, path: &str, value: &str) -> Result<(), ModError> {
        let bytes = value.as_bytes();
        if bytes.len() == 7 && bytes[0] == b'#' && bytes[1..].iter().all(u8::is_ascii_hexdigit) {
            Ok(())
        } else {
            Err(self.fail(file, path, format!("`{value}` is not a #RRGGBB colour")))
        }
    }

    fn ratio(&self, file: &str, path: &str, value: RefundRatio) -> Result<(), ModError> {
        if value.denominator == 0 {
            return Err(self.fail(file, path, "the denominator must be above 0"));
        }
        if value.numerator > value.denominator {
            return Err(self.fail(file, path, "the numerator is above the denominator"));
        }
        Ok(())
    }

    fn at_least_zero(&self, file: &str, path: &str, value: f32) -> Result<(), ModError> {
        if value.is_finite() && value >= 0.0 {
            Ok(())
        } else {
            Err(self.fail(
                file,
                path,
                format!("must be finite and at least 0, found {value:?}"),
            ))
        }
    }

    fn above_zero(&self, file: &str, path: &str, value: f32) -> Result<(), ModError> {
        if value.is_finite() && value > 0.0 {
            Ok(())
        } else {
            Err(self.fail(
                file,
                path,
                format!("must be finite and above 0, found {value:?}"),
            ))
        }
    }

    fn unit_locals(&self) -> Vec<String> {
        self.units
            .iter()
            .map(|(_, unit)| self.qualified(&unit.id))
            .collect()
    }

    fn resource_locals(&self) -> Vec<String> {
        self.resources
            .iter()
            .map(|(_, resource)| self.qualified(&resource.id))
            .collect()
    }

    fn production_locals(&self) -> Vec<String> {
        self.production
            .iter()
            .map(|(_, production)| self.qualified(&production.id))
            .collect()
    }

    fn order_locals(&self) -> Vec<String> {
        self.orders
            .iter()
            .map(|(_, order)| self.qualified(&order.id))
            .collect()
    }

    fn damage_types(&self) -> Vec<String> {
        self.combat
            .iter()
            .flat_map(|(_, table)| table.damage_types.iter().map(|id| self.qualified(id)))
            .collect()
    }

    fn armour_classes(&self) -> Vec<String> {
        self.combat
            .iter()
            .flat_map(|(_, table)| table.armour_classes.iter().map(|id| self.qualified(id)))
            .collect()
    }

    fn resource(&self, local: &str) -> Option<&ResourceDef> {
        self.resources
            .iter()
            .map(|(_, resource)| resource)
            .find(|resource| resource.id == local)
    }

    fn unit(&self, local: &str) -> Option<&UnitDef> {
        self.units
            .iter()
            .map(|(_, unit)| unit)
            .find(|unit| unit.id == local)
    }

    fn font_ids(&self) -> BTreeSet<&str> {
        self.assets
            .iter()
            .flat_map(|(_, assets)| assets.fonts.iter().map(|font| font.id.as_str()))
            .collect()
    }

    fn sound_ids(&self) -> BTreeSet<&str> {
        self.assets
            .iter()
            .flat_map(|(_, assets)| assets.sounds.iter().map(|sound| sound.id.as_str()))
            .collect()
    }

    /// A reference to a resource of the kind that `want` names. `kind_name` is the text of the
    /// kind in the message (`Supply`, `Balance`).
    fn resource_of_kind(
        &self,
        file: &str,
        path: &str,
        reference: &str,
        want: &[&str],
    ) -> Result<(), ModError> {
        let Some(local) =
            self.lookup(file, path, "resource", reference, &self.resource_locals())?
        else {
            return Ok(());
        };
        let kind = self.resource(&local).map(|resource| resource.kind);
        let name = match kind {
            Some(ResourceKind::Balance) => "Balance",
            Some(ResourceKind::Item) => "Item",
            Some(ResourceKind::Supply { .. }) => "Supply",
            None => return Ok(()),
        };
        if want.contains(&name) {
            return Ok(());
        }
        let wanted = want
            .iter()
            .map(|kind| format!("`{kind}`"))
            .collect::<Vec<_>>()
            .join(" or ");
        Err(self.fail(
            file,
            path,
            format!("`{reference}` is not a {wanted} resource"),
        ))
    }

    // ---- A: exports ----

    fn exports(&self) -> Result<(), ModError> {
        if let [first, _, ..] = self.combat.as_slice() {
            return Err(self.fail(
                MANIFEST_PATH,
                "exports.combat[1]",
                format!(
                    "a match has one damage table; `{}` already declares it",
                    first.0
                ),
            ));
        }
        if self.manifest_exports_match > 1 {
            return Err(self.fail(
                MANIFEST_PATH,
                "exports.match",
                format!("at most one file, found {}", self.manifest_exports_match),
            ));
        }
        Ok(())
    }

    // ---- C: the damage table ----

    fn combat_group(&self) -> Result<(), ModError> {
        for (file, table) in &self.combat {
            for (field, list, kind) in [
                ("damage_types", &table.damage_types, "damage type"),
                ("armour_classes", &table.armour_classes, "armour class"),
            ] {
                if list.is_empty() {
                    return Err(self.fail(file, field, "needs at least 1 entry"));
                }
                let mut seen = BTreeSet::new();
                for (index, id) in list.iter().enumerate() {
                    let path = format!("{field}[{index}]");
                    if id.is_empty() || id.contains(':') {
                        return Err(self.fail(
                            file,
                            &path,
                            "must be a nonempty local id without `:`",
                        ));
                    }
                    if !seen.insert(id.as_str()) {
                        return Err(self.fail(file, &path, format!("duplicate {kind} `{id}`")));
                    }
                }
            }
            let types = self.damage_types();
            let classes = self.armour_classes();
            let mut pairs = BTreeSet::new();
            for (index, cell) in table.cells.iter().enumerate() {
                let path = format!("cells[{index}]");
                self.lookup(
                    file,
                    &format!("{path}.damage"),
                    "damage type",
                    &cell.damage,
                    &types,
                )?;
                self.lookup(
                    file,
                    &format!("{path}.armour"),
                    "armour class",
                    &cell.armour,
                    &classes,
                )?;
                if cell.ratio.denominator == 0 {
                    return Err(self.fail(
                        file,
                        &format!("{path}.ratio"),
                        "the denominator must be above 0",
                    ));
                }
                if !pairs.insert((cell.damage.as_str(), cell.armour.as_str())) {
                    return Err(self.fail(
                        file,
                        &path,
                        format!("second cell for `{}` and `{}`", cell.damage, cell.armour),
                    ));
                }
            }
            for damage in &types {
                for armour in &classes {
                    if !pairs.contains(&(damage.as_str(), armour.as_str())) {
                        return Err(self.fail(
                            file,
                            "cells",
                            format!("no cell for damage `{damage}` and armour `{armour}`"),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    // ---- S1: resources ----

    fn resources_group(&self) -> Result<(), ModError> {
        for (file, resource) in &self.resources {
            if let ResourceKind::Supply { counts, .. } = resource.kind
                && counts == SupplyCounts::Queued
            {
                return Err(self.fail(
                    file,
                    "kind.counts",
                    "`Queued` is reserved for a later engine API",
                ));
            }
        }
        Ok(())
    }

    // ---- U, K1 to K3, R1: the fields of a unit ----

    fn units_group(&self) -> Result<(), ModError> {
        let has_table = !self.combat.is_empty();
        let classes = self.armour_classes();
        let types = self.damage_types();
        let largest = self.largest_mobile();
        let has_build_area = self.units.iter().any(|(_, unit)| unit.build_area.is_some());
        for (file, unit) in &self.units {
            if let Some(armour) = &unit.armour {
                self.lookup(
                    file,
                    "armour.class",
                    "armour class",
                    &armour.class,
                    &classes,
                )?;
            } else if has_table && unit.health.is_some() {
                return Err(self.fail(
                    file,
                    "armour",
                    "a unit with `health` needs an armour class, because the package declares a damage table",
                ));
            }
            if let Some(weapon) = &unit.weapon {
                match &weapon.damage_type {
                    Some(damage) if has_table => {
                        self.lookup(file, "weapon.damage_type", "damage type", damage, &types)?;
                    }
                    Some(_) => {
                        return Err(self.fail(
                            file,
                            "weapon.damage_type",
                            "the package declares no damage table",
                        ));
                    }
                    None if has_table => {
                        return Err(self.fail(
                            file,
                            "weapon.damage_type",
                            "a weapon needs a damage type, because the package declares a damage table",
                        ));
                    }
                    None => {}
                }
                // T3: a weapon damage of 0 packs for the edition 0.2.3 and is refused from 0.2.4.
                if weapon.damage == 0 {
                    return Err(self.fail(file, "weapon.damage", "must be positive"));
                }
                self.when_short(file, "weapon", &weapon.when_short)?;
            }
            if let Some(acquire) = &unit.acquire {
                let Some(weapon) = &unit.weapon else {
                    return Err(self.fail(file, "acquire", "needs `weapon`"));
                };
                if !acquire.range.is_finite() {
                    return Err(self.fail(
                        file,
                        "acquire.range",
                        format!("must be finite, found {:?}", acquire.range),
                    ));
                }
                if acquire.range < weapon.range {
                    return Err(self.fail(
                        file,
                        "acquire.range",
                        format!(
                            "{:?} is below the weapon range {:?}",
                            acquire.range, weapon.range
                        ),
                    ));
                }
                self.at_least_zero(file, "acquire.leash", acquire.leash)?;
            }
            for (field, list) in [("provides", &unit.provides), ("consumes", &unit.consumes)] {
                let mut seen = BTreeSet::new();
                for (index, entry) in list.iter().enumerate() {
                    let path = format!("{field}[{index}]");
                    self.resource_of_kind(
                        file,
                        &format!("{path}.resource"),
                        &entry.resource,
                        &["Supply"],
                    )?;
                    if entry.amount == 0 {
                        return Err(self.fail(file, &format!("{path}.amount"), "must be positive"));
                    }
                    if !seen.insert(entry.resource.as_str()) {
                        return Err(self.fail(
                            file,
                            &format!("{path}.resource"),
                            format!("second entry for `{}`", entry.resource),
                        ));
                    }
                }
            }
            if let Some(producer) = &unit.producer {
                self.when_short(file, "producer", &producer.when_short)?;
            }
            if let Some(income) = &unit.income {
                self.resource_of_kind(file, "income.resource", &income.resource, &["Balance"])?;
                if income.amount == 0 {
                    return Err(self.fail(file, "income.amount", "must be positive"));
                }
                if income.period_ms == 0 {
                    return Err(self.fail(file, "income.period_ms", "must be positive"));
                }
                if unit.health.is_none() {
                    return Err(self.fail(file, "income", "a unit with `income` needs `health`"));
                }
            }
            if let Some(start) = &unit.spawn_order {
                self.start_order(file, "spawn_order", start, Some(&unit.id), None)?;
            }
            if let Some(constructor) = &unit.constructor {
                self.constructor(file, unit, constructor)?;
            }
            if let Some(placement) = &unit.placement {
                if unit.movement.is_some() {
                    return Err(self.fail(
                        file,
                        "placement",
                        "a unit with `placement` must be static",
                    ));
                }
                if !placement.gap.is_finite() {
                    return Err(self.fail(
                        file,
                        "placement.gap",
                        format!("must be finite, found {:?}", placement.gap),
                    ));
                }
                let (diameter, id) = &largest;
                let least = diameter + 0.2;
                if placement.gap < least {
                    return Err(self.fail(
                        file,
                        "placement.gap",
                        format!(
                            "{:?} is below {least:?}, the largest mobile diameter (`{}`, {diameter:?}) plus 0.2",
                            placement.gap,
                            self.qualified(id)
                        ),
                    ));
                }
                self.at_least_zero(file, "placement.node_clearance", placement.node_clearance)?;
                self.ratio(file, "placement.site_health", placement.site_health)?;
                if placement.in_build_area && !has_build_area {
                    return Err(self.fail(
                        file,
                        "placement.in_build_area",
                        "no unit declares `build_area`, so no site can be placed",
                    ));
                }
            }
            if let Some(area) = unit.build_area {
                self.above_zero(file, "build_area", area)?;
            }
            if let Some(repairer) = &unit.repairer {
                self.repairer(file, repairer, &classes)?;
            }
        }
        Ok(())
    }

    /// The largest diameter of a unit with movement in the package, and its local id.
    fn largest_mobile(&self) -> (f32, String) {
        let mut largest = (0.0f32, String::new());
        for (_, unit) in &self.units {
            let Footprint::Circle { radius } = unit.footprint;
            if unit.movement.is_some() && radius * 2.0 > largest.0 {
                largest = (radius * 2.0, unit.id.clone());
            }
        }
        largest
    }

    /// U6: the `when_short` list of one capability.
    fn when_short(&self, file: &str, owner: &str, list: &[WhenShort]) -> Result<(), ModError> {
        for (index, entry) in list.iter().enumerate() {
            let path = format!("{owner}.when_short[{index}]");
            self.resource_of_kind(
                file,
                &format!("{path}.resource"),
                &entry.resource,
                &["Supply"],
            )?;
            let effect = format!("{path}.effect");
            match entry.effect {
                ShortEffect::Rate(ratio) => {
                    if owner == "weapon" {
                        return Err(self.fail(
                            file,
                            &effect,
                            "`Rate` applies to `producer` and `constructor` only",
                        ));
                    }
                    if ratio.numerator == 0
                        || ratio.denominator == 0
                        || ratio.numerator > ratio.denominator
                    {
                        return Err(self.fail(
                            file,
                            &effect,
                            "`Rate` needs 0 < numerator <= denominator",
                        ));
                    }
                }
                ShortEffect::Disable if owner != "weapon" => {
                    return Err(self.fail(file, &effect, "`Disable` applies to `weapon` only"));
                }
                ShortEffect::Disable => {}
                ShortEffect::BlockStart => {
                    return Err(self.fail(
                        file,
                        &effect,
                        "`BlockStart` is reserved for a later engine API",
                    ));
                }
            }
        }
        Ok(())
    }

    /// The `Q` of design 6.4: the least common multiple of every `Rate` denominator. The packer
    /// refuses a `Q` above 65,536.
    fn rates(&self) -> Result<(), ModError> {
        let mut q: u128 = 1;
        for (file, unit) in &self.units {
            let lists = [
                ("producer", unit.producer.as_ref().map(|p| &p.when_short)),
                (
                    "constructor",
                    unit.constructor.as_ref().map(|c| &c.when_short),
                ),
            ];
            for (owner, list) in lists {
                for (index, entry) in list.into_iter().flatten().enumerate() {
                    let ShortEffect::Rate(ratio) = entry.effect else {
                        continue;
                    };
                    let denominator = u128::from(ratio.denominator.max(1));
                    q = q / gcd(q, denominator) * denominator;
                    if q > MAX_RATE_LCM {
                        return Err(self.fail(
                            file,
                            &format!("{owner}.when_short[{index}].effect"),
                            format!(
                                "the least common multiple of the `Rate` denominators is {q} here; the maximum is {MAX_RATE_LCM}"
                            ),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// K1.
    fn constructor(
        &self,
        file: &str,
        unit: &UnitDef,
        constructor: &turts_mod_api::ConstructorDef,
    ) -> Result<(), ModError> {
        let building_outputs: Vec<String> = self
            .production
            .iter()
            .filter(|(_, production)| {
                let output = production
                    .output_unit
                    .rsplit(':')
                    .next()
                    .unwrap_or(&production.output_unit);
                self.unit(output)
                    .is_some_and(|unit| unit.placement.is_some())
            })
            .map(|(_, production)| self.qualified(&production.id))
            .collect();
        let all = self.production_locals();
        for (index, building) in constructor.buildings.iter().enumerate() {
            let path = format!("constructor.buildings[{index}]");
            // The allowed list holds the productions that can be a building.
            self.lookup_allowed(file, &path, "production", building, &all, &building_outputs)?;
            if let Some(producer) = &unit.producer
                && producer.recipes.contains(building)
            {
                return Err(self.fail(
                    file,
                    &path,
                    format!(
                        "`{building}` is also in `producer.recipes` of this unit; a production is a recipe or a building, not both"
                    ),
                ));
            }
            for (other_file, other) in &self.units {
                if other.id == unit.id {
                    continue;
                }
                if other
                    .producer
                    .as_ref()
                    .is_some_and(|producer| producer.recipes.contains(building))
                {
                    return Err(self.fail(
                        file,
                        &path,
                        format!(
                            "`{building}` is also in `producer.recipes` of `{}` ({other_file}); a production is a recipe or a building, not both",
                            self.qualified(&other.id)
                        ),
                    ));
                }
            }
        }
        if constructor.concurrent_sites == 0 {
            return Err(self.fail(file, "constructor.concurrent_sites", "must be at least 1"));
        }
        if constructor.queue_capacity == 0 {
            return Err(self.fail(file, "constructor.queue_capacity", "must be at least 1"));
        }
        if let ConstructProgress::NeedsBuilder { .. } = constructor.progress {
            return Err(self.fail(
                file,
                "constructor.progress",
                "`NeedsBuilder` is reserved for a later engine API",
            ));
        }
        self.when_short(file, "constructor", &constructor.when_short)?;
        // Beside `producer` allowed, beside any other work capability refused.
        let others = [
            ("gatherer", unit.gatherer.is_some()),
            ("machine", unit.machine.is_some()),
            ("research", unit.research.is_some()),
            ("extractor", unit.extractor.is_some()),
            ("inserter", unit.inserter.is_some()),
            ("conveyor", unit.conveyor.is_some()),
            ("weapon", unit.weapon.is_some()),
        ];
        if let Some((name, _)) = others.iter().find(|(_, present)| *present) {
            return Err(self.fail(
                file,
                name,
                "a unit may have at most one work capability; `constructor` is allowed only beside `producer`",
            ));
        }
        Ok(())
    }

    /// R1.
    fn repairer(
        &self,
        file: &str,
        repairer: &turts_mod_api::RepairerDef,
        classes: &[String],
    ) -> Result<(), ModError> {
        if repairer.rate_per_second == 0 {
            return Err(self.fail(file, "repairer.rate_per_second", "must be positive"));
        }
        self.above_zero(file, "repairer.reach", repairer.reach)?;
        for (index, cost) in repairer.cost.iter().enumerate() {
            let path = format!("repairer.cost[{index}]");
            self.resource_of_kind(
                file,
                &format!("{path}.resource"),
                &cost.resource,
                &["Balance"],
            )?;
            if cost.per_health == 0 {
                return Err(self.fail(file, &format!("{path}.per_health"), "must be positive"));
            }
        }
        for (index, class) in repairer.classes.iter().enumerate() {
            self.lookup(
                file,
                &format!("repairer.classes[{index}]"),
                "armour class",
                class,
                classes,
            )?;
        }
        if let Some(range) = repairer.auto_range
            && (!range.is_finite() || range < repairer.reach)
        {
            return Err(self.fail(
                file,
                "repairer.auto_range",
                format!(
                    "must be finite and at least the reach {:?}, found {range:?}",
                    repairer.reach
                ),
            ));
        }
        Ok(())
    }

    /// U8 and M3: the one validator of a start order (design 6.5). `unit` is the unit type that
    /// gets the order; `node_count` is `Some` for a map placement (the number of nodes of the map).
    fn start_order(
        &self,
        file: &str,
        prefix: &str,
        start: &StartOrder,
        unit: Option<&str>,
        node_count: Option<usize>,
    ) -> Result<(), ModError> {
        let order_path = format!("{prefix}.order");
        let target_path = format!("{prefix}.target");
        let local = self.lookup(
            file,
            &order_path,
            "order",
            &start.order,
            &self.order_locals(),
        )?;
        let order = local.and_then(|local| {
            self.orders
                .iter()
                .map(|(_, order)| order)
                .find(|order| order.id == local)
        });
        if let (Some(order), Some(unit)) = (order, unit)
            && !order
                .units
                .iter()
                .any(|listed| *listed == self.qualified(unit))
        {
            return Err(self.fail(
                file,
                &order_path,
                format!("`{}` does not list `{}`", start.order, self.qualified(unit)),
            ));
        }
        if node_count.is_none()
            && let StartTarget::Node(_) | StartTarget::Point(_) = start.target
        {
            let found = match start.target {
                StartTarget::Node(index) => format!("Node({index})"),
                _ => "Point".to_string(),
            };
            return Err(self.fail(
                file,
                &target_path,
                format!("a unit type takes `NearestNode` only, found `{found}`"),
            ));
        }
        if let (Some(count), StartTarget::Node(index)) = (node_count, &start.target)
            && *index as usize >= count
        {
            return Err(self.fail(
                file,
                &target_path,
                format!("node index {index} is outside the {count} map nodes"),
            ));
        }
        if let Some(order) = order {
            let (takes, fits) = match start.target {
                StartTarget::Node(_) | StartTarget::NearestNode => {
                    ("Node", order.target == TargetKind::Node)
                }
                StartTarget::Point(_) => (
                    "Point",
                    matches!(order.target, TargetKind::Point | TargetKind::PointOrUnit),
                ),
            };
            if !fits {
                return Err(self.fail(
                    file,
                    &target_path,
                    format!(
                        "`{}` takes a `{:?}` target, not a `{takes}` target",
                        start.order, order.target
                    ),
                ));
            }
        }
        if node_count.is_some() && start.ground_rally != turts_mod_api::GroundRally::Ignore {
            return Err(self.fail(
                file,
                &format!("{prefix}.ground_rally"),
                format!("a map unit has no rally, found `{:?}`", start.ground_rally),
            ));
        }
        Ok(())
    }

    // ---- K4, K5, O8, O10: productions ----

    fn production_group(&self) -> Result<(), ModError> {
        let buildings: BTreeSet<&str> = self
            .units
            .iter()
            .filter_map(|(_, unit)| unit.constructor.as_ref())
            .flat_map(|constructor| constructor.buildings.iter())
            .map(|id| id.rsplit(':').next().unwrap_or(id))
            .collect();
        let actions = self.action_names();
        let mut cells: BTreeMap<(String, u8, u8), String> = BTreeMap::new();
        for (file, production) in &self.production {
            if buildings.contains(production.id.as_str()) {
                let output = production
                    .output_unit
                    .rsplit(':')
                    .next()
                    .unwrap_or(&production.output_unit);
                match self.unit(output) {
                    Some(unit) if unit.placement.is_some() => {}
                    Some(_) => {
                        return Err(self.fail(
                            file,
                            "output_unit",
                            "a building output needs `placement`",
                        ));
                    }
                    None => {}
                }
            }
            self.ratio(
                file,
                "cancel_refunds.destroyed",
                production.cancel_refunds.destroyed,
            )?;
            if let Some(hotkey) = &production.hotkey
                && !actions.contains(hotkey)
            {
                return Err(self.unknown(
                    file,
                    "hotkey",
                    "input action",
                    hotkey,
                    actions.iter().cloned().collect(),
                ));
            }
            if let Some(cell) = production.card {
                // The menu of a unit: its recipes, then its buildings.
                let id = self.qualified(&production.id);
                for (_, unit) in &self.units {
                    let listed = unit
                        .producer
                        .as_ref()
                        .is_some_and(|producer| producer.recipes.contains(&id))
                        || unit
                            .constructor
                            .as_ref()
                            .is_some_and(|constructor| constructor.buildings.contains(&id));
                    if !listed {
                        continue;
                    }
                    let key = (unit.id.clone(), cell.row, cell.column);
                    if let Some(other) = cells.insert(key, id.clone()) {
                        return Err(self.fail(
                            file,
                            "card",
                            format!(
                                "cell ({}, {}) is also the cell of `{other}`",
                                cell.row, cell.column
                            ),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn technologies_group(&self) -> Result<(), ModError> {
        for (file, technology) in &self.technologies {
            if technology.payment == PaymentPolicy::ChargeOverTime {
                return Err(self.fail(
                    file,
                    "payment",
                    "`ChargeOverTime` is not supported for technologies",
                ));
            }
            self.ratio(
                file,
                "cancel_refunds.destroyed",
                technology.cancel_refunds.destroyed,
            )?;
        }
        Ok(())
    }

    // ---- O: orders and rules ----

    fn orders_group(&self) -> Result<(), ModError> {
        for (file, order) in &self.orders {
            self.order(file, order)?;
        }
        let orders = self.order_locals();
        for (file, rules) in &self.rules {
            for (name, list) in [
                ("smart", Some(&rules.smart)),
                ("rally", rules.rally.as_ref()),
            ] {
                for (index, entry) in list.into_iter().flatten().enumerate() {
                    self.smart_entry(file, name, index, entry, &orders)?;
                }
            }
        }
        Ok(())
    }

    fn order(&self, file: &str, order: &OrderDef) -> Result<(), ModError> {
        if order.target == TargetKind::ContentAtPoint && order.behaviour != Behaviour::Construct {
            return Err(self.fail(
                file,
                "target",
                "`ContentAtPoint` needs behaviour `Construct`",
            ));
        }
        let (needs, has): (Option<&str>, fn(&UnitDef) -> bool) = match order.behaviour {
            Behaviour::Construct => (Some("constructor"), |unit| unit.constructor.is_some()),
            Behaviour::CancelConstruction => (Some("placement"), |unit| unit.placement.is_some()),
            Behaviour::SetRally => (Some("producer"), |unit| unit.producer.is_some()),
            Behaviour::Repair => (Some("repairer"), |unit| unit.repairer.is_some()),
            _ => (None, |_| true),
        };
        if let Some(capability) = needs {
            let units = self.unit_locals();
            for (index, listed) in order.units.iter().enumerate() {
                let path = format!("units[{index}]");
                if let Some(local) = self.lookup(file, &path, "unit", listed, &units)?
                    && let Some(unit) = self.unit(&local)
                    && !has(unit)
                {
                    return Err(self.fail(
                        file,
                        &path,
                        format!("`{listed}` has no `{capability}`"),
                    ));
                }
            }
        }
        if order.behaviour == Behaviour::Surrender && !order.units.is_empty() {
            // Row O9. The text is the reviewer's (review of `L0-API2`, question 3): the design text
            // "lists no units" reads as an error for an empty list.
            return Err(self.fail(
                file,
                "units",
                "a `Surrender` order must list no units; write `units: []`",
            ));
        }
        Ok(())
    }

    fn smart_entry(
        &self,
        file: &str,
        list: &str,
        index: usize,
        entry: &SmartOrder,
        orders: &[String],
    ) -> Result<(), ModError> {
        let path = format!("{list}[{index}].order");
        let local = self.lookup(file, &path, "order", &entry.order, orders)?;
        let Some(order) = local.and_then(|local| {
            self.orders
                .iter()
                .map(|(_, order)| order)
                .find(|order| order.id == local)
        }) else {
            return Ok(());
        };
        if entry.target == SmartTarget::OwnDamaged && order.behaviour != Behaviour::Repair {
            return Err(self.fail(file, &path, "`OwnDamaged` needs a `Repair` order"));
        }
        if entry.target == SmartTarget::Ground
            && entry.needs == Capability::Producer
            && order.behaviour != Behaviour::SetRally
        {
            return Err(self.fail(
                file,
                &path,
                "`Ground` with needs `Producer` needs a `SetRally` order",
            ));
        }
        Ok(())
    }

    // ---- M: maps and the match file ----

    fn maps_group(&self) -> Result<(), ModError> {
        for (file, map) in &self.maps {
            for (index, placement) in map.placements.iter().enumerate() {
                if let Some(start) = &placement.start_order {
                    let local = placement
                        .unit
                        .rsplit(':')
                        .next()
                        .unwrap_or(&placement.unit)
                        .to_string();
                    self.start_order(
                        file,
                        &format!("placements[{index}].start_order"),
                        start,
                        Some(&local),
                        Some(map.nodes.len()),
                    )?;
                }
            }
            for (slot_index, init) in map.slot_economy.iter().enumerate() {
                for (index, balance) in init.balances.iter().enumerate() {
                    let local = balance
                        .balance
                        .rsplit(':')
                        .next()
                        .unwrap_or(&balance.balance);
                    if self.resource(local).is_some_and(|resource| {
                        matches!(resource.kind, ResourceKind::Supply { .. })
                    }) {
                        return Err(self.fail(
                            file,
                            &format!("slot_economy[{slot_index}].balances[{index}].balance"),
                            format!(
                                "`{}` is a `Supply` resource; it has no balance",
                                balance.balance
                            ),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn match_group(&self) -> Result<(), ModError> {
        let constructors = self
            .units
            .iter()
            .any(|(_, unit)| unit.constructor.is_some());
        if constructors && self.matches.is_empty() {
            return Err(self.fail(
                MANIFEST_PATH,
                "exports.match",
                "a package with a constructor needs a match file that sets `open_sites_per_slot`",
            ));
        }
        for (file, definition) in &self.matches {
            self.match_file(file, definition, constructors)?;
        }
        Ok(())
    }

    fn match_file(
        &self,
        file: &str,
        definition: &MatchDef,
        constructors: bool,
    ) -> Result<(), ModError> {
        match definition.open_sites_per_slot {
            None if constructors => {
                return Err(self.fail(
                    file,
                    "open_sites_per_slot",
                    "a package with a constructor needs `open_sites_per_slot`",
                ));
            }
            Some(0) => {
                return Err(self.fail(file, "open_sites_per_slot", "must be at least 1"));
            }
            _ => {}
        }
        let (name, countdown) = match definition.start {
            StartPolicy::Immediate => ("", 0),
            StartPolicy::AllSeated { countdown_secs } => ("AllSeated", countdown_secs),
            StartPolicy::AllSeatedReady { countdown_secs } => ("AllSeatedReady", countdown_secs),
        };
        if countdown > 30 {
            return Err(self.fail(
                file,
                "start.countdown_secs",
                format!("must be 0 to 30, found {countdown}"),
            ));
        }
        if !name.is_empty() {
            // H15: one different colour for each seat that the lobby offers (`SLOT_COUNT`, the
            // engine slot count of a match), not for each slot that the map places units for.
            let seats = usize::from(SLOT_COUNT);
            let colours: BTreeSet<String> = self
                .interfaces
                .iter()
                .flat_map(|(_, interface)| interface.slot_colours.iter())
                .map(|colour| colour.to_ascii_lowercase())
                .collect();
            if colours.len() < seats {
                return Err(self.fail(
                    file,
                    "start",
                    format!(
                        "`{name}` needs {} different slot colours in the interface, found {}",
                        seats,
                        colours.len()
                    ),
                ));
            }
        }
        let Some(VictoryRule::LastTeamStanding { defeat }) = &definition.victory else {
            return Ok(());
        };
        let units = self.unit_locals();
        let mut disconnects = 0;
        for (index, clause) in defeat.iter().enumerate() {
            let path = format!("victory.defeat[{index}]");
            match clause {
                DefeatClause::KeyUnitsLost { units: listed } => {
                    if listed.is_empty() {
                        return Err(self.fail(
                            file,
                            &path,
                            "`KeyUnitsLost` needs at least one unit",
                        ));
                    }
                    for (unit_index, listed) in listed.iter().enumerate() {
                        let unit_path = format!("{path}.units[{unit_index}]");
                        if let Some(local) =
                            self.lookup(file, &unit_path, "unit", listed, &units)?
                            && self.unit(&local).is_some_and(|unit| unit.health.is_none())
                        {
                            return Err(self.fail(
                                file,
                                &unit_path,
                                format!("`{listed}` has no `health`"),
                            ));
                        }
                    }
                }
                DefeatClause::DisconnectedFor { secs } => {
                    disconnects += 1;
                    if *secs == 0 {
                        return Err(self.fail(
                            file,
                            &format!("{path}.secs"),
                            "`DisconnectedFor` secs must be above 0",
                        ));
                    }
                }
            }
        }
        if disconnects > 1 {
            return Err(self.fail(
                file,
                "victory.defeat",
                format!("`DisconnectedFor` is written {disconnects} times; at most 1"),
            ));
        }
        Ok(())
    }

    // ---- KEY1: the input map ----

    fn action_names(&self) -> BTreeSet<String> {
        self.inputs
            .iter()
            .flat_map(|(_, input)| input.actions.iter().map(|action| action.name.clone()))
            .collect()
    }

    /// F10 is reserved for the settings menu (the engine action `menu`). The settings menu holds the
    /// sound switch that every player must be able to reach (the CEO rule: silent by default, with a
    /// way back; a mod may leave the play-screen switch out of its interface only because the engine
    /// draws one, design 6.10 row H25). The engine reads a plain F10 for `menu` before any binding of
    /// the mod or the player (`turts_interaction::bindings::RESERVED_MENU_KEY`). So a package of API
    /// 0.2.4 may not bind F10 to another action, may not take F10 away from `menu` in
    /// `engine_overrides`, and may not author an action named `menu`.
    fn menu_keeps_f10(&self) -> Result<(), ModError> {
        let is_f10 = |binding: &Binding| matches!(binding, Binding::Key { code, mods } if code == "F10" && mods.is_empty());
        let reserved = |name: &str| {
            format!(
                "F10 is reserved for the settings menu (the engine action `menu`); bind another key to `{name}`"
            )
        };
        for (file, input) in &self.inputs {
            for (index, over) in input.engine_overrides.iter().enumerate() {
                if over.name == "menu" {
                    if !over.bindings.iter().any(is_f10) {
                        return Err(self.fail(
                            file,
                            &format!("engine_overrides[{index}].bindings"),
                            "the engine action `menu` must keep F10; F10 is reserved for the settings menu, which holds the sound switch that every player must be able to reach. Add `Key(code: \"F10\", mods: [])` to these bindings, or leave `menu` out of `engine_overrides`",
                        ));
                    }
                    continue;
                }
                if let Some(at) = over.bindings.iter().position(is_f10) {
                    return Err(self.fail(
                        file,
                        &format!("engine_overrides[{index}].bindings[{at}]"),
                        reserved(&over.name),
                    ));
                }
            }
            for (index, action) in input.actions.iter().enumerate() {
                if action.name == "menu" {
                    return Err(self.fail(
                        file,
                        &format!("actions[{index}].name"),
                        "`menu` is an engine action; an authored action cannot have its name. Change its keys in `engine_overrides` (it keeps F10)",
                    ));
                }
                if let Some(at) = action.default.iter().position(is_f10) {
                    return Err(self.fail(
                        file,
                        &format!("actions[{index}].default[{at}]"),
                        reserved(&action.name),
                    ));
                }
            }
        }
        Ok(())
    }

    /// No two actions have the same default key (the check is global; there is no action scope).
    fn input_group(&self) -> Result<(), ModError> {
        self.menu_keeps_f10()?;
        let mut seen: BTreeMap<String, &str> = BTreeMap::new();
        for (file, input) in &self.inputs {
            for (action_index, action) in input.actions.iter().enumerate() {
                for (binding_index, binding) in action.default.iter().enumerate() {
                    let Binding::Key { code, mods } = binding else {
                        continue;
                    };
                    let mut chord: Vec<&str> = [
                        (turts_mod_api::Mod::Ctrl, "Ctrl"),
                        (turts_mod_api::Mod::Shift, "Shift"),
                        (turts_mod_api::Mod::Alt, "Alt"),
                    ]
                    .iter()
                    .filter(|(modifier, _)| mods.contains(modifier))
                    .map(|(_, name)| *name)
                    .collect();
                    chord.push(code);
                    let chord = chord.join("+");
                    if let Some(other) = seen.get(&chord)
                        && *other != action.name
                    {
                        return Err(self.fail(
                            file,
                            &format!("actions[{action_index}].default[{binding_index}]"),
                            format!(
                                "`{chord}` is also the default key of `{other}`; each default key must be unique"
                            ),
                        ));
                    }
                    seen.insert(chord, &action.name);
                }
            }
        }
        Ok(())
    }

    // ---- H: the interface ----

    fn interface_group(&self) -> Result<(), ModError> {
        for (file, interface) in &self.interfaces {
            self.interface(file, interface)?;
        }
        Ok(())
    }

    fn key_list(
        &self,
        file: &str,
        field: &str,
        kind: &str,
        keys: impl Iterator<Item = String>,
        allowed: &[&str],
    ) -> Result<(), ModError> {
        for key in keys {
            if !allowed.contains(&key.as_str()) {
                return Err(self.unknown(
                    file,
                    &format!("{field}.{key}"),
                    kind,
                    &key,
                    allowed.iter().map(|name| name.to_string()).collect(),
                ));
            }
        }
        Ok(())
    }

    fn interface(&self, file: &str, interface: &InterfaceDef) -> Result<(), ModError> {
        // T2: the key lists are closed from the edition 0.2.4 on.
        self.key_list(
            file,
            "reasons",
            "reason key",
            interface.reasons.keys().cloned(),
            REASON_KEYS,
        )?;
        self.key_list(
            file,
            "cursors",
            "cursor context",
            interface.cursors.keys().cloned(),
            CURSOR_CONTEXTS,
        )?;
        self.key_list(
            file,
            "states",
            "state key",
            interface.states.keys().cloned(),
            STATE_KEYS,
        )?;
        for (key, text) in &interface.states {
            self.placeholders(file, &format!("states.{key}"), text, &["{resource}"], false)?;
        }
        let units = self.unit_locals();
        let fonts = self.font_ids();
        let sounds = self.sound_ids();
        if let Some(start) = &interface.start_selection {
            self.lookup(file, "start_selection", "unit", start, &units)?;
        }
        if let Some(voices) = interface.effects_voices
            && !(1..=64).contains(&voices)
        {
            return Err(self.fail(file, "effects_voices", "must be 1 to 64"));
        }
        if let Some(range) = interface.audible_range {
            self.above_zero(file, "audible_range", range)?;
        }
        if let Some(gap) = interface.alert_sound_gap_secs {
            self.at_least_zero(file, "alert_sound_gap_secs", gap)?;
        }
        self.world_styles(file, interface, &fonts, &sounds)?;
        for (index, alert) in interface.alerts.iter().enumerate() {
            self.alert(file, index, alert, &units)?;
        }
        self.hints(file, interface, &units)?;
        for widget in &interface.root {
            self.widget(file, widget, &format!("root[{}]", widget.id), &fonts)?;
        }
        for page in &interface.guide {
            for widget in &page.widgets {
                self.widget(
                    file,
                    widget,
                    &format!("guide[{}].widgets[{}]", page.id, widget.id),
                    &fonts,
                )?;
            }
        }
        Ok(())
    }

    /// H11 and H24: the colours, fonts and sounds of the world styles.
    fn world_styles(
        &self,
        file: &str,
        interface: &InterfaceDef,
        fonts: &BTreeSet<&str>,
        sounds: &BTreeSet<&str>,
    ) -> Result<(), ModError> {
        let world = &interface.world;
        if let Some(style) = &world.placement {
            self.colour(file, "world.placement.valid", &style.valid)?;
            self.colour(file, "world.placement.invalid", &style.invalid)?;
        }
        for (name, line) in [
            ("build_area_ring", &world.build_area_ring),
            ("gap_ring", &world.gap_ring),
            ("rally", &world.rally),
        ] {
            if let Some(line) = line {
                self.colour(file, &format!("world.{name}.colour"), &line.colour)?;
                self.above_zero(file, &format!("world.{name}.width"), line.width)?;
            }
        }
        for (name, bar) in [
            ("progress_bar", &world.progress_bar),
            ("cargo_bar", &world.cargo_bar),
        ] {
            if let Some(bar) = bar {
                self.colour(file, &format!("world.{name}.background"), &bar.background)?;
                self.colour(file, &format!("world.{name}.fill"), &bar.fill)?;
                if bar.width == 0 || bar.height == 0 {
                    return Err(self.fail(
                        file,
                        &format!("world.{name}"),
                        "width and height must be greater than zero",
                    ));
                }
            }
        }
        if let Some(float) = &world.float_text {
            self.colour(file, "world.float_text.colour", &float.colour)?;
            self.font(file, "world.float_text.font", float.font.as_deref(), fonts)?;
            if let Some(sound) = &float.sound
                && !sounds.contains(sound.as_str())
            {
                return Err(self.unknown(
                    file,
                    "world.float_text.sound",
                    "sound asset",
                    sound,
                    sounds.iter().map(|name| name.to_string()).collect(),
                ));
            }
        }
        if let Some(labels) = &world.unit_labels {
            self.colour(file, "world.unit_labels.colour", &labels.colour)?;
            self.font(
                file,
                "world.unit_labels.font",
                labels.font.as_deref(),
                fonts,
            )?;
            if !(8..=32).contains(&labels.size) {
                return Err(self.fail(
                    file,
                    "world.unit_labels.size",
                    format!("{} is outside 8 to 32", labels.size),
                ));
            }
        }
        Ok(())
    }

    fn font(
        &self,
        file: &str,
        path: &str,
        font: Option<&str>,
        fonts: &BTreeSet<&str>,
    ) -> Result<(), ModError> {
        match font {
            Some(font) if !fonts.contains(font) => Err(self.unknown(
                file,
                path,
                "font asset",
                font,
                fonts.iter().map(|name| name.to_string()).collect(),
            )),
            _ => Ok(()),
        }
    }

    /// H17 and T4: the placeholders of a text. A `{...}` that is not in `allowed` is refused.
    fn placeholders(
        &self,
        file: &str,
        path: &str,
        text: &str,
        allowed: &[&str],
        numbered: bool,
    ) -> Result<(), ModError> {
        let mut rest = text;
        while let Some(start) = rest.find('{') {
            let Some(length) = rest[start..].find('}') else {
                break;
            };
            let token = &rest[start..start + length + 1];
            rest = &rest[start + length + 1..];
            let position = numbered
                && token.len() > 2
                && token[1..token.len() - 1]
                    .chars()
                    .all(|c| c.is_ascii_digit());
            if !position && !allowed.contains(&token) {
                return Err(self.unknown_placeholder(file, path, token, allowed));
            }
        }
        Ok(())
    }

    fn unknown_placeholder(
        &self,
        file: &str,
        path: &str,
        token: &str,
        allowed: &[&str],
    ) -> ModError {
        self.unknown(
            file,
            path,
            "placeholder",
            token,
            allowed.iter().map(|name| name.to_string()).collect(),
        )
    }

    /// H4 to H6, H17, H21, H22.
    fn alert(
        &self,
        file: &str,
        index: usize,
        alert: &AlertDef,
        units: &[String],
    ) -> Result<(), ModError> {
        let path = format!("alerts[{index}]");
        let trigger = format!("{path}.trigger");
        match &alert.trigger {
            AlertTrigger::SupplyShort(resource) | AlertTrigger::SupplyRestored(resource) => {
                self.resource_of_kind(file, &trigger, resource, &["Supply"])?;
            }
            AlertTrigger::InsufficientFunds(resource) => {
                self.resource_of_kind(file, &trigger, resource, &["Balance", "Item"])?;
            }
            AlertTrigger::GathererIdle(seconds) | AlertTrigger::GathererWaiting(seconds)
                if !seconds.is_finite() || *seconds <= 0.0 =>
            {
                return Err(self.fail(
                    file,
                    &trigger,
                    "waiting duration must be finite and greater than 0 seconds",
                ));
            }
            _ => {}
        }
        for (unit_index, unit) in alert.units.iter().enumerate() {
            self.lookup(
                file,
                &format!("{path}.units[{unit_index}]"),
                "unit",
                unit,
                units,
            )?;
        }
        if alert.kind == AlertKind::State && !is_state_trigger(&alert.trigger) {
            return Err(self.fail(
                file,
                &format!("{path}.kind"),
                format!(
                    "`State` needs a state trigger; `{}` is an event",
                    trigger_name(&alert.trigger)
                ),
            ));
        }
        if let Some(seconds) = alert.clear_after {
            if alert.kind != AlertKind::State {
                return Err(self.fail(file, &format!("{path}.clear_after"), "needs `kind: State`"));
            }
            self.at_least_zero(file, &format!("{path}.clear_after"), seconds)?;
        }
        if alert.sound_when == SoundWhen::QueueEmpty
            && !matches!(alert.trigger, AlertTrigger::ProductionComplete)
        {
            return Err(self.fail(
                file,
                &format!("{path}.sound_when"),
                "`QueueEmpty` needs the trigger `ProductionComplete`",
            ));
        }
        if let Some(focus) = &alert.focus {
            for (focus_index, unit) in focus.iter().enumerate() {
                self.lookup(
                    file,
                    &format!("{path}.focus[{focus_index}]"),
                    "unit",
                    unit,
                    units,
                )?;
            }
        }
        self.placeholders(
            file,
            &format!("{path}.text"),
            &alert.text,
            &PLACEHOLDERS_ALERT,
            false,
        )
    }

    /// H14: the hints.
    fn hints(
        &self,
        file: &str,
        interface: &InterfaceDef,
        units: &[String],
    ) -> Result<(), ModError> {
        let mut ids = BTreeSet::new();
        for (index, hint) in interface.hints.iter().enumerate() {
            if !ids.insert(hint.id.as_str()) {
                return Err(self.fail(
                    file,
                    &format!("hints[{index}].id"),
                    format!("duplicate hint id `{}`", hint.id),
                ));
            }
        }
        for (index, hint) in interface.hints.iter().enumerate() {
            for (field, condition) in [("starts", &hint.starts), ("done", &hint.done)] {
                self.condition(
                    file,
                    &format!("hints[{index}].{field}"),
                    condition,
                    &interface.hints,
                    units,
                )?;
            }
        }
        Ok(())
    }

    fn condition(
        &self,
        file: &str,
        path: &str,
        condition: &HintCondition,
        hints: &[HintDef],
        units: &[String],
    ) -> Result<(), ModError> {
        let hint_ids: Vec<String> = hints.iter().map(|hint| hint.id.clone()).collect();
        let named = |id: &str, name: &str| -> Result<(), ModError> {
            if hint_ids.iter().any(|known| known == id) {
                return Ok(());
            }
            let hint = nearest_name(id, hint_ids.iter().map(String::as_str))
                .map(|near| format!("; did you mean `{near}`?"))
                .unwrap_or_default();
            let mut sorted = hint_ids.clone();
            sorted.sort();
            let found = self.fail(file, path, format!("`{name}({id})` names no hint{hint}"));
            Err(if sorted.is_empty() {
                found
            } else {
                found.with_allowed(sorted)
            })
        };
        match condition {
            HintCondition::MatchStarted => {}
            HintCondition::OwnUnits { units: listed, .. }
            | HintCondition::Selected { units: listed } => {
                for (index, unit) in listed.iter().enumerate() {
                    self.lookup(file, &format!("{path}.units[{index}]"), "unit", unit, units)?;
                }
            }
            HintCondition::OwnJob { production } => {
                self.lookup(
                    file,
                    path,
                    "production",
                    production,
                    &self.production_locals(),
                )?;
            }
            HintCondition::OwnRally { unit } => {
                self.lookup(file, path, "unit", unit, units)?;
            }
            HintCondition::OrderAccepted { order, .. } => {
                self.lookup(file, path, "order", order, &self.order_locals())?;
            }
            HintCondition::SupplyShort(resource) => {
                self.resource_of_kind(file, path, resource, &["Supply"])?;
            }
            HintCondition::JobOnHold { secs } => {
                self.hint_secs(file, path, "JobOnHold", *secs)?;
            }
            HintCondition::HintDone(id) => named(id, "HintDone")?,
            HintCondition::HintShownFor { id, secs } => {
                named(id, "HintShownFor")?;
                self.hint_secs(file, path, "HintShownFor", *secs)?;
            }
            HintCondition::StepCurrent(id) => {
                named(id, "StepCurrent")?;
                if hints
                    .iter()
                    .any(|hint| hint.id == *id && hint.kind != HintKind::Step)
                {
                    return Err(self.fail(
                        file,
                        path,
                        format!("`StepCurrent({id})` names a situation, not a step"),
                    ));
                }
            }
            HintCondition::All(list) | HintCondition::Any(list) => {
                for inner in list {
                    self.condition(file, path, inner, hints, units)?;
                }
            }
            HintCondition::Not(inner) => self.condition(file, path, inner, hints, units)?,
        }
        Ok(())
    }

    fn hint_secs(&self, file: &str, path: &str, name: &str, secs: f32) -> Result<(), ModError> {
        if secs.is_finite() && secs > 0.0 {
            Ok(())
        } else {
            Err(self.fail(
                file,
                path,
                format!("`{name}` secs must be finite and above 0, found {secs:?}"),
            ))
        }
    }

    /// H7, H10, H13: the widget tree. The paths are the paths of `variants.rs`.
    fn widget(
        &self,
        file: &str,
        widget: &Widget,
        path: &str,
        fonts: &BTreeSet<&str>,
    ) -> Result<(), ModError> {
        for predicate in &widget.visible {
            self.predicate(file, &format!("{path}.visible"), predicate)?;
        }
        if let Some(value) = &widget.tooltip {
            self.value(file, &format!("{path}.tooltip"), value)?;
        }
        match &widget.widget {
            Primitive::Box { children, .. } => {
                for child in children {
                    self.widget(
                        file,
                        child,
                        &format!("{path}.widget.children[{}]", child.id),
                        fonts,
                    )?;
                }
            }
            Primitive::Image { image, .. } => {
                if let ButtonIcon::Bind(value) = image {
                    self.value(file, &format!("{path}.widget.image"), value)?;
                }
            }
            Primitive::Text { text, .. } => {
                self.text(file, &format!("{path}.widget.text"), text)?;
            }
            Primitive::Bar { value, max, .. } => {
                self.value(file, &format!("{path}.widget.value"), value)?;
                self.value(file, &format!("{path}.widget.max"), max)?;
            }
            Primitive::Button {
                icon,
                label,
                font,
                size,
                ..
            } => {
                if let Some(ButtonIcon::Bind(value)) = icon {
                    self.value(file, &format!("{path}.widget.icon"), value)?;
                }
                if let Some(label) = label {
                    self.text(file, &format!("{path}.widget.label"), label)?;
                }
                self.font(
                    file,
                    &format!("{path}.widget.font"),
                    font.value().as_deref(),
                    fonts,
                )?;
                if let Some(size) = size.value()
                    && !(8..=64).contains(size)
                {
                    return Err(self.fail(
                        file,
                        &format!("{path}.widget.size"),
                        format!("{size} is outside 8 to 64"),
                    ));
                }
            }
            Primitive::List { item, .. } | Primitive::Grid { item, .. } => {
                self.widget(file, item, &format!("{path}.widget.item"), fonts)?;
            }
            Primitive::Tooltip { value } => {
                self.value(file, &format!("{path}.widget.value"), value)?;
            }
            Primitive::Cursor { .. } | Primitive::MinimapView { .. } => {}
        }
        Ok(())
    }

    fn text(&self, file: &str, path: &str, text: &TextValue) -> Result<(), ModError> {
        match text {
            TextValue::Literal(_) | TextValue::Item(_) => Ok(()),
            TextValue::Bind(value) => self.value(file, path, value),
            TextValue::Format(pattern, values) => {
                self.placeholders(file, path, pattern, &PLACEHOLDERS_FORMAT, true)?;
                values
                    .iter()
                    .try_for_each(|value| self.value(file, path, value))
            }
        }
    }

    fn value(&self, file: &str, path: &str, value: &Value) -> Result<(), ModError> {
        match value {
            Value::Supply(resource, _) => self.resource_of_kind(file, path, resource, &["Supply"]),
            Value::MatchStats(StatField::Gathered(resource)) => {
                self.resource_of_kind(file, path, resource, &["Balance"])
            }
            Value::Format(pattern, values) => {
                self.placeholders(file, path, pattern, &PLACEHOLDERS_FORMAT, true)?;
                values
                    .iter()
                    .try_for_each(|value| self.value(file, path, value))
            }
            _ => Ok(()),
        }
    }

    fn predicate(&self, file: &str, path: &str, predicate: &Predicate) -> Result<(), ModError> {
        match predicate {
            Predicate::SupplyShort(resource) => {
                self.resource_of_kind(file, path, resource, &["Supply"])
            }
            Predicate::Not(inner) => self.predicate(file, path, inner),
            _ => Ok(()),
        }
    }
}

fn gcd(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// The triggers that hold while their condition holds (design 6.10 row H6).
fn is_state_trigger(trigger: &AlertTrigger) -> bool {
    matches!(
        trigger,
        AlertTrigger::SupplyShort(_)
            | AlertTrigger::InsufficientFunds(_)
            | AlertTrigger::GathererWaiting(_)
            | AlertTrigger::GathererIdle(_)
            | AlertTrigger::SlotDisconnected
            | AlertTrigger::InserterWaiting(_)
            | AlertTrigger::OutOfAmmo
            | AlertTrigger::OutputBlocked
            | AlertTrigger::ExtractorFull
    )
}

fn trigger_name(trigger: &AlertTrigger) -> &'static str {
    match trigger {
        AlertTrigger::OwnUnitDamaged => "OwnUnitDamaged",
        AlertTrigger::OwnUnitDestroyed => "OwnUnitDestroyed",
        AlertTrigger::ProductionComplete => "ProductionComplete",
        AlertTrigger::ResearchComplete => "ResearchComplete",
        AlertTrigger::ResearchReady => "ResearchReady",
        AlertTrigger::ExtractorFull => "ExtractorFull",
        AlertTrigger::ExtractorDepleted => "ExtractorDepleted",
        AlertTrigger::OutputBlocked => "OutputBlocked",
        AlertTrigger::OutOfAmmo => "OutOfAmmo",
        AlertTrigger::InserterWaiting(_) => "InserterWaiting",
        AlertTrigger::GathererWaiting(_) => "GathererWaiting",
        AlertTrigger::ConstructionComplete => "ConstructionComplete",
        AlertTrigger::SupplyShort(_) => "SupplyShort",
        AlertTrigger::SupplyRestored(_) => "SupplyRestored",
        AlertTrigger::InsufficientFunds(_) => "InsufficientFunds",
        AlertTrigger::SlotDisconnected => "SlotDisconnected",
        AlertTrigger::SlotReconnected => "SlotReconnected",
        AlertTrigger::GathererIdle(_) => "GathererIdle",
    }
}
