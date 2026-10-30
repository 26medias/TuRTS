//! Portable M3 validation, kept separate from typed reference resolution.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;
use turts_mod_api::content::{ContentCatalog, ResolvedEffect, ResolvedItemStack};
use turts_mod_api::{ContentId, Manifest, ModError, PackageId};

use crate::content::{check_finite_positive, error};

/// Portable rules for API 0.2.1. Keep this separate from reference resolution so the
/// frozen public catalog validator continues to serve API 0.2 callers.
pub(crate) fn validate_m3(manifest: &Manifest, catalog: &ContentCatalog) -> Result<(), ModError> {
    let origin = PackageId(manifest.id.clone());
    for unit in catalog.units.values() {
        let file = catalog
            .source_files
            .get(&unit.id)
            .map(String::as_str)
            .unwrap_or("content");
        if let Some(extractor) = &unit.extractor {
            check_finite_positive(
                &origin,
                file,
                "extractor.interaction_range",
                extractor.interaction_range,
                true,
            )?;
        }
        if let Some(inserter) = &unit.inserter {
            check_finite_positive(&origin, file, "inserter.reach", inserter.reach, true)?;
        }
        if let Some(weapon) = &unit.weapon {
            check_finite_positive(&origin, file, "weapon.range", weapon.range, true)?;
        }
    }
    for recipe in catalog.recipes.values() {
        let file = catalog
            .source_files
            .get(&recipe.id)
            .map(String::as_str)
            .unwrap_or("content");
        for (field, stacks) in [("inputs", &recipe.inputs), ("outputs", &recipe.outputs)] {
            if stacks.is_empty() {
                return Err(error(&origin, file, field, "must not be empty"));
            }
            for (index, stack) in stacks.iter().enumerate() {
                if stack.amount == 0 {
                    return Err(error(
                        &origin,
                        file,
                        &format!("{field}[{index}].amount"),
                        "must be positive",
                    ));
                }
                if catalog.resources.get(&stack.item).is_none_or(|resource| {
                    resource.kind != turts_mod_api::schema::ResourceKind::Item
                }) {
                    return Err(error(
                        &origin,
                        file,
                        &format!("{field}[{index}].item"),
                        "recipe stacks must name Item resources",
                    ));
                }
            }
        }
        if recipe.duration_ms == 0 {
            return Err(error(&origin, file, "duration_ms", "must be positive"));
        }
        for (name, ratio) in [
            ("running", recipe.cancel_refunds.running),
            ("output_blocked", recipe.cancel_refunds.output_blocked),
        ] {
            if ratio.denominator == 0 || ratio.numerator > ratio.denominator {
                return Err(error(
                    &origin,
                    file,
                    &format!("cancel_refunds.{name}"),
                    "denominator must be positive and numerator must not exceed it",
                ));
            }
        }
    }
    for technology in catalog.technologies.values() {
        let file = catalog
            .source_files
            .get(&technology.id)
            .map(String::as_str)
            .unwrap_or("content");
        for (index, prerequisite) in technology.prerequisites.iter().enumerate() {
            if !catalog.technologies.contains_key(prerequisite) {
                return Err(error(
                    &origin,
                    file,
                    &format!("prerequisites[{index}]"),
                    format!("unknown technology `{prerequisite}`"),
                ));
            }
        }
        if technology.duration_ms == 0 {
            return Err(error(&origin, file, "duration_ms", "must be positive"));
        }
        let mut seen = BTreeSet::new();
        for (index, cost) in technology.costs.iter().enumerate() {
            let field = format!("costs[{index}]");
            if cost.amount == 0 {
                return Err(error(
                    &origin,
                    file,
                    &format!("{field}.amount"),
                    "must be positive",
                ));
            }
            let expected_kind = match cost.source {
                turts_mod_api::schema::CostSource::SlotBalance => {
                    turts_mod_api::schema::ResourceKind::Balance
                }
                turts_mod_api::schema::CostSource::ProducerInventory => {
                    turts_mod_api::schema::ResourceKind::Item
                }
            };
            if catalog
                .resources
                .get(&cost.resource)
                .is_none_or(|resource| resource.kind != expected_kind)
            {
                return Err(error(
                    &origin,
                    file,
                    &format!("{field}.resource"),
                    "cost resource kind does not match its source",
                ));
            }
            if !seen.insert((cost.source, cost.resource.clone())) {
                return Err(error(
                    &origin,
                    file,
                    &field,
                    "duplicate cost source and resource",
                ));
            }
        }
        for (name, ratio) in [
            ("queued", technology.cancel_refunds.queued),
            ("running", technology.cancel_refunds.running),
            ("output_blocked", technology.cancel_refunds.output_blocked),
        ] {
            if ratio.denominator == 0 || ratio.numerator > ratio.denominator {
                return Err(error(
                    &origin,
                    file,
                    &format!("cancel_refunds.{name}"),
                    "denominator must be positive and numerator must not exceed it",
                ));
            }
        }
        if technology.payment == turts_mod_api::schema::PaymentPolicy::ReserveThenCharge
            && technology.cancel_refunds.queued
                != (turts_mod_api::schema::RefundRatio {
                    numerator: 1,
                    denominator: 1,
                })
        {
            return Err(error(
                &origin,
                file,
                "cancel_refunds.queued",
                "ReserveThenCharge queued refund must be 1/1",
            ));
        }
    }
    for unit in catalog.units.values() {
        let unit_file = catalog
            .source_files
            .get(&unit.id)
            .map(String::as_str)
            .unwrap_or("content");
        if let Some(station) = &unit.research {
            if station.queue_capacity == 0 {
                return Err(error(
                    &origin,
                    unit_file,
                    "research.queue_capacity",
                    "must be positive",
                ));
            }
            if station.technologies.is_empty() {
                return Err(error(
                    &origin,
                    unit_file,
                    "research.technologies",
                    "must not be empty",
                ));
            }
            for (index, technology_id) in station.technologies.iter().enumerate() {
                let technology = catalog.technologies.get(technology_id).ok_or_else(|| {
                    error(
                        &origin,
                        unit_file,
                        &format!("research.technologies[{index}]"),
                        format!("unknown technology `{technology_id}`"),
                    )
                })?;
                for cost in &technology.costs {
                    if cost.source == turts_mod_api::schema::CostSource::ProducerInventory
                        && !unit
                            .inventory
                            .as_ref()
                            .is_some_and(|inventory| inventory.accepts.contains(&cost.resource))
                    {
                        let file = catalog
                            .source_files
                            .get(&unit.id)
                            .map(String::as_str)
                            .unwrap_or("content");
                        return Err(error(
                            &origin,
                            file,
                            "research.technologies",
                            "ProducerInventory cost must be accepted by the station inventory",
                        ));
                    }
                }
            }
        }
    }
    for unit in catalog.units.values() {
        let file = catalog
            .source_files
            .get(&unit.id)
            .map(String::as_str)
            .unwrap_or("content");
        let fail = |field: &str, message: &str| error(&origin, file, field, message);
        if let Some(inserter) = &unit.inserter {
            if inserter.swing_ms == 0 {
                return Err(fail("inserter.swing_ms", "must be positive"));
            }
            if inserter.return_ms == 0 {
                return Err(fail("inserter.return_ms", "must be positive"));
            }
        }
        if unit.inserter.is_some() && unit.inventory.is_none() {
            return Err(fail(
                "inserter.inventory",
                "inserter requires a hand inventory",
            ));
        }
        if unit.extractor.is_some() && unit.inventory.is_none() {
            return Err(fail(
                "extractor.inventory",
                "extractor requires a main inventory",
            ));
        }
        if unit.extractor.is_some() && unit.movement.is_some() {
            return Err(fail("extractor", "extractors must be static"));
        }
        if let Some(output) = &unit.output_inventory {
            if output.capacity == 0 {
                return Err(fail("output_inventory.capacity", "must be positive"));
            }
            let mut seen = BTreeSet::new();
            for (index, item) in output.accepts.iter().enumerate() {
                if catalog.resources.get(item).is_none_or(|resource| {
                    resource.kind != turts_mod_api::schema::ResourceKind::Item
                }) {
                    return Err(fail(
                        &format!("output_inventory.accepts[{index}]"),
                        "must name an Item resource",
                    ));
                }
                if !seen.insert(item) {
                    return Err(fail(
                        &format!("output_inventory.accepts[{index}]"),
                        "duplicate accepted item",
                    ));
                }
            }
        }
        if let Some(conveyor) = &unit.conveyor {
            if unit.movement.is_some() {
                return Err(fail("conveyor", "conveyor pieces must be static"));
            }
            if !(1..=16).contains(&conveyor.slots_per_piece) {
                return Err(fail("conveyor.slots_per_piece", "must be between 1 and 16"));
            }
            let mut seen = BTreeSet::new();
            for (index, item) in conveyor.accepts.iter().enumerate() {
                if catalog.resources.get(item).is_none_or(|resource| {
                    resource.kind != turts_mod_api::schema::ResourceKind::Item
                }) {
                    return Err(fail(
                        &format!("conveyor.accepts[{index}]"),
                        "must name an Item resource",
                    ));
                }
                if !seen.insert(item) {
                    return Err(fail(
                        &format!("conveyor.accepts[{index}]"),
                        "duplicate accepted item",
                    ));
                }
            }
        }
        if let Some(machine) = &unit.machine {
            let recipe = catalog
                .recipes
                .get(&machine.recipe)
                .ok_or_else(|| fail("machine.recipe", "machine recipe does not exist"))?;
            let input = unit
                .inventory
                .as_ref()
                .ok_or_else(|| fail("machine.inventory", "machine requires an input inventory"))?;
            let output = unit
                .output_inventory
                .as_ref()
                .ok_or_else(|| fail("output_inventory", "machine requires an output inventory"))?;
            for (index, stack) in recipe.inputs.iter().enumerate() {
                if catalog
                    .resources
                    .get(&stack.item)
                    .is_none_or(|r| r.kind != turts_mod_api::schema::ResourceKind::Item)
                {
                    return Err(fail(
                        &format!("machine.recipe.inputs[{index}].item"),
                        "machine inputs must be Item resources",
                    ));
                }
                if !input.accepts.contains(&stack.item) {
                    return Err(fail(
                        &format!("inventory.accepts[{index}]"),
                        "machine input item is not accepted",
                    ));
                }
            }
            let input_sum =
                checked_stack_sum(&origin, file, "machine.recipe.inputs", &recipe.inputs)?;
            if input_sum > u128::from(input.capacity) {
                return Err(fail(
                    "inventory.capacity",
                    "machine inputs exceed input capacity",
                ));
            }
            for (index, stack) in recipe.outputs.iter().enumerate() {
                if catalog
                    .resources
                    .get(&stack.item)
                    .is_none_or(|r| r.kind != turts_mod_api::schema::ResourceKind::Item)
                {
                    return Err(fail(
                        &format!("machine.recipe.outputs[{index}].item"),
                        "machine outputs must be Item resources",
                    ));
                }
                if !output.accepts.contains(&stack.item) {
                    return Err(fail(
                        &format!("output_inventory.accepts[{index}]"),
                        "machine output item is not accepted",
                    ));
                }
            }
            let output_sum =
                checked_stack_sum(&origin, file, "machine.recipe.outputs", &recipe.outputs)?;
            if output_sum > u128::from(output.capacity) {
                return Err(fail(
                    "output_inventory.capacity",
                    "machine outputs exceed output capacity",
                ));
            }
            if unit.movement.is_some() {
                return Err(fail("machine", "machines must be static"));
            }
        }
        if unit.health.is_some() && unit.removal.is_none() {
            return Err(fail("health", "a unit with health requires removal"));
        }
        if unit.health.is_some_and(|health| health.max == 0) {
            return Err(fail("health.max", "must be positive"));
        }
        if let Some(weapon) = &unit.weapon
            && let Some(ammo) = &weapon.ammo
            && !unit
                .inventory
                .as_ref()
                .is_some_and(|inventory| inventory.accepts.contains(&ammo.item))
        {
            return Err(fail(
                "weapon.ammo.item",
                "weapon ammo item must be accepted by the inventory",
            ));
        }
        let mut work = Vec::new();
        if unit.gatherer.is_some() {
            work.push("gatherer");
        }
        if unit.producer.is_some() {
            work.push("producer");
        }
        if unit.machine.is_some() {
            work.push("machine");
        }
        if unit.research.is_some() {
            work.push("research");
        }
        if unit.extractor.is_some() {
            work.push("extractor");
        }
        if unit.inserter.is_some() {
            work.push("inserter");
        }
        if unit.conveyor.is_some() {
            work.push("conveyor");
        }
        if unit.weapon.is_some() {
            work.push("weapon");
        }
        if work.len() > 1 {
            return Err(fail(work[1], "a unit may have at most one work capability"));
        }
        if unit.removal.is_some()
            && (unit.producer.is_some() || unit.machine.is_some() || unit.research.is_some())
        {
            return Err(fail("removal", "a unit with a queue cannot be removed"));
        }
    }
    for production in catalog.production.values() {
        let output = catalog.units.get(&production.output_unit).ok_or_else(|| {
            let file = catalog
                .source_files
                .get(&production.id)
                .map(String::as_str)
                .unwrap_or("content");
            error(
                &origin,
                file,
                "output_unit",
                "production output unit does not exist",
            )
        })?;
        if output.inserter.is_some() {
            let file = catalog
                .source_files
                .get(&production.id)
                .map(String::as_str)
                .unwrap_or("content");
            return Err(error(
                &origin,
                file,
                "output_unit",
                "production output cannot declare an inserter because it has no facing field",
            ));
        }
    }
    for tech in catalog.technologies.values() {
        let file = catalog
            .source_files
            .get(&tech.id)
            .map(String::as_str)
            .unwrap_or("content");
        for (index, effect) in tech.effects.iter().enumerate() {
            let ResolvedEffect::WeaponModifier { unit, .. } = effect;
            if !catalog
                .units
                .get(unit)
                .is_some_and(|definition| definition.weapon.is_some())
            {
                return Err(error(
                    &origin,
                    file,
                    &format!("effects[{index}].unit"),
                    "weapon modifier target must name a unit with a weapon",
                ));
            }
        }
    }
    check_prerequisite_cycles(&origin, catalog)?;
    let item_count = catalog
        .resources
        .values()
        .filter(|resource| resource.kind == turts_mod_api::schema::ResourceKind::Item)
        .count();
    if item_count > usize::from(u16::MAX) {
        let Some(excess) = catalog
            .resources
            .iter()
            .filter(|(_, resource)| resource.kind == turts_mod_api::schema::ResourceKind::Item)
            .nth(usize::from(u16::MAX))
            .map(|(id, _)| id)
        else {
            return Err(error(&origin, "content", "id", "item count exceeds 65535"));
        };
        let file = catalog
            .source_files
            .get(excess)
            .map(String::as_str)
            .unwrap_or("content");
        return Err(error(&origin, file, "id", "item count exceeds 65535"));
    }
    let version = semver::VersionReq::parse(&manifest.engine_api).map_err(|_| {
        error(
            &origin,
            "manifest.ron",
            "engine_api",
            "invalid engine API requirement",
        )
    })?;
    let legacy = semver::Version::new(0, 2, 0);
    if version.matches(&legacy)
        && (manifest_has_m3_syntax(manifest) || catalog_has_m3_content(catalog))
    {
        return Err(error(
            &origin,
            "manifest.ron",
            "engine_api",
            "M3 content requires an engine API requirement that excludes 0.2.0",
        ));
    }
    Ok(())
}

fn checked_stack_sum(
    origin: &PackageId,
    file: &str,
    field: &str,
    stacks: &[ResolvedItemStack],
) -> Result<u128, ModError> {
    stacks.iter().try_fold(0u128, |sum, stack| {
        sum.checked_add(u128::from(stack.amount)).ok_or_else(|| {
            error(
                origin,
                file,
                field,
                "item count sum exceeds the supported range",
            )
        })
    })
}

fn check_prerequisite_cycles(origin: &PackageId, catalog: &ContentCatalog) -> Result<(), ModError> {
    fn visit(
        id: &ContentId,
        catalog: &ContentCatalog,
        marks: &mut BTreeMap<ContentId, u8>,
    ) -> bool {
        match marks.get(id).copied() {
            Some(1) => return false,
            Some(2) => return true,
            _ => {}
        }
        marks.insert(id.clone(), 1);
        if let Some(technology) = catalog.technologies.get(id) {
            for prerequisite in &technology.prerequisites {
                if !visit(prerequisite, catalog, marks) {
                    return false;
                }
            }
        }
        marks.insert(id.clone(), 2);
        true
    }
    let mut marks = BTreeMap::new();
    for technology in catalog.technologies.values() {
        if !visit(&technology.id, catalog, &mut marks) {
            let file = catalog
                .source_files
                .get(&technology.id)
                .map(String::as_str)
                .unwrap_or("content");
            return Err(error(
                origin,
                file,
                "prerequisites",
                "technology prerequisites contain a cycle",
            ));
        }
    }
    Ok(())
}

fn manifest_has_m3_syntax(manifest: &Manifest) -> bool {
    !manifest.exports.recipes.is_empty() || !manifest.exports.technologies.is_empty()
}

fn catalog_has_m3_content(catalog: &ContentCatalog) -> bool {
    !catalog.recipes.is_empty()
        || !catalog.technologies.is_empty()
        || catalog.units.values().any(|u| {
            u.output_inventory.is_some()
                || u.extractor.is_some()
                || u.conveyor.is_some()
                || u.inserter.is_some()
                || u.machine.is_some()
                || u.research.is_some()
                || u.health.is_some()
                || u.weapon.is_some()
                || u.removal.is_some()
        })
}

/// Check original RON syntax. Typed catalogs lose the difference between an absent additive
/// field and an explicit default such as `weapon: None` or `recipes: []`.
pub(crate) fn validate_m3_presence(
    manifest: &Manifest,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<(), ModError> {
    let package = PackageId(manifest.id.clone());
    let manifest_bytes = files
        .get("manifest.ron")
        .ok_or_else(|| error(&package, "manifest.ron", "archive", "manifest is missing"))?;
    let exports: ExportPresence = ron::de::from_bytes(manifest_bytes)
        .map_err(|e| error(&package, "manifest.ron", "manifest", e.to_string()))?;
    let mut present = exports.exports.recipes || exports.exports.technologies;
    for path in &manifest.exports.units {
        let Some(bytes) = files.get(path) else {
            return Err(error(
                &package,
                "manifest.ron",
                "exports.units",
                format!("exported unit file `{path}` is missing"),
            ));
        };
        let fields: UnitPresence = ron::de::from_bytes(bytes)
            .map_err(|e| error(&package, path, "content", e.to_string()))?;
        present |= fields.any();
    }
    for path in &manifest.exports.maps {
        let Some(bytes) = files.get(path) else {
            // `content()` historically validates gameplay sources without loading map files.
            continue;
        };
        let fields: MapPresence = ron::de::from_bytes(bytes)
            .map_err(|e| error(&package, path, "content", e.to_string()))?;
        present |= fields.belts || fields.placements.iter().any(|placement| placement.facing);
    }
    let requirement = semver::VersionReq::parse(&manifest.engine_api).map_err(|_| {
        error(
            &package,
            "manifest.ron",
            "engine_api",
            "invalid engine API requirement",
        )
    })?;
    if present && requirement.matches(&semver::Version::new(0, 2, 0)) {
        return Err(error(
            &package,
            "manifest.ron",
            "engine_api",
            "M3 content requires an engine API requirement that excludes 0.2.0",
        ));
    }
    Ok(())
}

fn was_present<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    serde::de::IgnoredAny::deserialize(deserializer)?;
    Ok(true)
}
#[derive(serde::Deserialize, Default)]
struct ExportPresence {
    #[serde(default)]
    exports: ExportListsPresence,
}
#[derive(serde::Deserialize, Default)]
struct ExportListsPresence {
    #[serde(default, deserialize_with = "was_present")]
    recipes: bool,
    #[serde(default, deserialize_with = "was_present")]
    technologies: bool,
}
#[derive(serde::Deserialize, Default)]
struct UnitPresence {
    #[serde(default, deserialize_with = "was_present")]
    output_inventory: bool,
    #[serde(default, deserialize_with = "was_present")]
    extractor: bool,
    #[serde(default, deserialize_with = "was_present")]
    conveyor: bool,
    #[serde(default, deserialize_with = "was_present")]
    inserter: bool,
    #[serde(default, deserialize_with = "was_present")]
    machine: bool,
    #[serde(default, deserialize_with = "was_present")]
    research: bool,
    #[serde(default, deserialize_with = "was_present")]
    health: bool,
    #[serde(default, deserialize_with = "was_present")]
    weapon: bool,
    #[serde(default, deserialize_with = "was_present")]
    removal: bool,
}
impl UnitPresence {
    fn any(&self) -> bool {
        self.output_inventory
            || self.extractor
            || self.conveyor
            || self.inserter
            || self.machine
            || self.research
            || self.health
            || self.weapon
            || self.removal
    }
}
#[derive(serde::Deserialize, Default)]
struct MapPresence {
    #[serde(default, deserialize_with = "was_present")]
    belts: bool,
    #[serde(default)]
    placements: Vec<PlacementPresence>,
}
#[derive(serde::Deserialize, Default)]
struct PlacementPresence {
    #[serde(default, deserialize_with = "was_present")]
    facing: bool,
}
