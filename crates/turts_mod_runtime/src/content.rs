//! Portable validation and typed reference resolution for staged API 0.2 content.

use std::collections::{BTreeMap, BTreeSet};

pub use turts_mod_api::content::{
    ContentCatalog, ResolvedAmmo, ResolvedConveyor, ResolvedCost, ResolvedDropOff,
    ResolvedDropOffEffect, ResolvedDropOffRule, ResolvedEffect, ResolvedGatherer,
    ResolvedInventory, ResolvedItemStack, ResolvedMachine, ResolvedNode, ResolvedProducer,
    ResolvedProduction, ResolvedRecipe, ResolvedResearchStation, ResolvedTechnology, ResolvedUnit,
    ResolvedWeapon, SourceDef,
};
use turts_mod_api::schema::*;
use turts_mod_api::{ContentId, ModError, PackageId};

const MAX_MOVEMENT_SPEED: f32 = 100.0;
const MAX_FOOTPRINT_RADIUS: f32 = 50.0;

pub fn checked_content_ref(
    origin: &PackageId,
    file: &str,
    field: &str,
    text: &str,
) -> Result<ContentId, ModError> {
    let parsed = ContentId::parse(text).map_err(|e| error(origin, file, field, e))?;
    PackageId::parse(&parsed.package.0).map_err(|e| error(origin, file, field, e))?;
    Ok(parsed)
}

pub(crate) fn error(
    origin: &PackageId,
    file: &str,
    field: &str,
    message: impl Into<String>,
) -> ModError {
    ModError::new(&origin.0, field, message).with_file(file)
}
pub(crate) fn local_id(origin: &PackageId, file: &str, id: &str) -> Result<ContentId, ModError> {
    if id.is_empty() || id.contains(':') {
        return Err(error(
            origin,
            file,
            "id",
            "must be a nonempty local id without `:`",
        ));
    }
    Ok(ContentId::new(origin, id))
}
pub(crate) fn check_finite_positive(
    origin: &PackageId,
    file: &str,
    field: &str,
    value: f32,
    allow_zero: bool,
) -> Result<(), ModError> {
    if !value.is_finite()
        || if allow_zero {
            value < 0.0
        } else {
            value <= 0.0
        }
    {
        return Err(error(
            origin,
            file,
            field,
            if allow_zero {
                "must be finite and >= 0"
            } else {
                "must be finite and > 0"
            },
        ));
    }
    Ok(())
}

fn check_finite_positive_at_most(
    origin: &PackageId,
    file: &str,
    field: &str,
    value: f32,
    maximum: f32,
) -> Result<(), ModError> {
    check_finite_positive(origin, file, field, value, false)?;
    if value > maximum {
        return Err(error(
            origin,
            file,
            field,
            format!("must be finite, > 0 and <= {maximum}"),
        ));
    }
    Ok(())
}

pub fn validate_content(
    origin: &PackageId,
    resources: Vec<SourceDef<ResourceDef>>,
    units: Vec<SourceDef<UnitDef>>,
    nodes: Vec<SourceDef<NodeDef>>,
    production: Vec<SourceDef<ProductionDef>>,
) -> Result<ContentCatalog, ModError> {
    validate_content_with_m3(
        origin,
        resources,
        units,
        nodes,
        production,
        Vec::new(),
        Vec::new(),
    )
}

fn validate_content_m2(
    origin: &PackageId,
    resources: Vec<SourceDef<ResourceDef>>,
    units: Vec<SourceDef<UnitDef>>,
    nodes: Vec<SourceDef<NodeDef>>,
    production: Vec<SourceDef<ProductionDef>>,
) -> Result<ContentCatalog, ModError> {
    let mut all_ids = BTreeSet::new();
    let mut catalog = ContentCatalog {
        resources: BTreeMap::new(),
        units: BTreeMap::new(),
        nodes: BTreeMap::new(),
        production: BTreeMap::new(),
        recipes: BTreeMap::new(),
        technologies: BTreeMap::new(),
        orders: BTreeMap::new(),
        order_queue_depths: std::collections::BTreeMap::new(),
        source_files: BTreeMap::new(),
    };
    for src in resources {
        let id = local_id(origin, &src.file, &src.value.id)?;
        insert_id(origin, &src.file, &src.value.id, &mut all_ids)?;
        catalog.source_files.insert(id.clone(), src.file.clone());
        catalog.resources.insert(id, src.value);
    }
    // Preserve every source definition before resolving. This permits forward references.
    let unit_sources = units;
    let node_sources = nodes;
    let production_sources = production;
    let mut unit_files = BTreeMap::new();
    for src in &unit_sources {
        insert_id(origin, &src.file, &src.value.id, &mut all_ids)?;
        let id = local_id(origin, &src.file, &src.value.id)?;
        unit_files.insert(id.clone(), src.file.clone());
        catalog.source_files.insert(id, src.file.clone());
    }
    for src in &node_sources {
        insert_id(origin, &src.file, &src.value.id, &mut all_ids)?;
        let id = local_id(origin, &src.file, &src.value.id)?;
        catalog.source_files.insert(id, src.file.clone());
    }
    let mut production_files = BTreeMap::new();
    for src in &production_sources {
        insert_id(origin, &src.file, &src.value.id, &mut all_ids)?;
        let id = local_id(origin, &src.file, &src.value.id)?;
        production_files.insert(id.clone(), src.file.clone());
        catalog.source_files.insert(id, src.file.clone());
    }

    for src in unit_sources {
        let value = src.value;
        let file = src.file;
        let id = local_id(origin, &file, &value.id)?;
        match value.footprint {
            Footprint::Circle { radius } => check_finite_positive_at_most(
                origin,
                &file,
                "footprint.radius",
                radius,
                MAX_FOOTPRINT_RADIUS,
            )?,
        }
        if let Some(m) = &value.movement {
            check_finite_positive_at_most(
                origin,
                &file,
                "movement.speed",
                m.speed,
                MAX_MOVEMENT_SPEED,
            )?;
        }
        let inventory = value
            .inventory
            .map(|x| -> Result<_, ModError> {
                if x.capacity == 0 {
                    return Err(error(
                        origin,
                        &file,
                        "inventory.capacity",
                        "must be positive",
                    ));
                }
                let accepts = resolve_refs(origin, &file, "inventory.accepts", &x.accepts)?;
                unique_refs(origin, &file, "inventory.accepts", &accepts)?;
                for (i, r) in accepts.iter().enumerate() {
                    require_kind(
                        origin,
                        &file,
                        &format!("inventory.accepts[{i}]"),
                        r,
                        &catalog.resources,
                        ResourceKind::Item,
                    )?;
                }
                Ok(ResolvedInventory {
                    capacity: x.capacity,
                    accepts,
                })
            })
            .transpose()?;
        let gatherer = value
            .gatherer
            .map(|x| -> Result<_, ModError> {
                if value.movement.is_none() {
                    return Err(error(origin, &file, "gatherer", "requires movement"));
                }
                let inventory = inventory
                    .as_ref()
                    .ok_or_else(|| error(origin, &file, "gatherer", "requires inventory"))?;
                if x.gather_per_second == 0 {
                    return Err(error(
                        origin,
                        &file,
                        "gatherer.gather_per_second",
                        "must be positive",
                    ));
                }
                if x.unload_per_second == 0 {
                    return Err(error(
                        origin,
                        &file,
                        "gatherer.unload_per_second",
                        "must be positive",
                    ));
                }
                check_finite_positive(
                    origin,
                    &file,
                    "gatherer.interaction_range",
                    x.interaction_range,
                    true,
                )?;
                let items = resolve_refs(origin, &file, "gatherer.items", &x.items)?;
                if items.is_empty() {
                    return Err(error(origin, &file, "gatherer.items", "must not be empty"));
                }
                unique_refs(origin, &file, "gatherer.items", &items)?;
                for (i, r) in items.iter().enumerate() {
                    require_kind(
                        origin,
                        &file,
                        &format!("gatherer.items[{i}]"),
                        r,
                        &catalog.resources,
                        ResourceKind::Item,
                    )?;
                    if !inventory.accepts.contains(r) {
                        return Err(error(
                            origin,
                            &file,
                            &format!("gatherer.items[{i}]"),
                            "item is not accepted by inventory",
                        ));
                    }
                }
                Ok(ResolvedGatherer {
                    items,
                    gather_per_second: x.gather_per_second,
                    unload_per_second: x.unload_per_second,
                    interaction_range: x.interaction_range,
                })
            })
            .transpose()?;
        let drop_off = value
            .drop_off
            .map(|x| -> Result<_, ModError> {
                let mut seen = BTreeSet::new();
                let mut rules = Vec::new();
                for (i, rule) in x.rules.into_iter().enumerate() {
                    let field = format!("drop_off.rules[{i}]");
                    let item = resolve_one(origin, &file, &format!("{field}.item"), &rule.item)?;
                    require_kind(
                        origin,
                        &file,
                        &format!("{field}.item"),
                        &item,
                        &catalog.resources,
                        ResourceKind::Item,
                    )?;
                    if !seen.insert(item.clone()) {
                        return Err(error(origin, &file, &field, "duplicate rule for item"));
                    }
                    let effect = match rule.effect {
                        DropOffEffect::Store => {
                            if !inventory
                                .as_ref()
                                .is_some_and(|i| i.accepts.contains(&item))
                            {
                                return Err(error(
                                    origin,
                                    &file,
                                    &format!("{field}.effect"),
                                    "Store requires an accepting inventory",
                                ));
                            }
                            ResolvedDropOffEffect::Store
                        }
                        DropOffEffect::Convert { balance, per_item } => {
                            let balance = resolve_one(
                                origin,
                                &file,
                                &format!("{field}.effect.balance"),
                                &balance,
                            )?;
                            require_kind(
                                origin,
                                &file,
                                &format!("{field}.effect.balance"),
                                &balance,
                                &catalog.resources,
                                ResourceKind::Balance,
                            )?;
                            if per_item == 0 {
                                return Err(error(
                                    origin,
                                    &file,
                                    &format!("{field}.effect.per_item"),
                                    "must be positive",
                                ));
                            }
                            ResolvedDropOffEffect::Convert { balance, per_item }
                        }
                    };
                    rules.push(ResolvedDropOffRule { item, effect });
                }
                Ok(ResolvedDropOff { rules })
            })
            .transpose()?;
        let producer = value
            .producer
            .map(|x| -> Result<_, ModError> {
                if value.movement.is_some() {
                    return Err(error(origin, &file, "producer", "producers must be static"));
                }
                if x.queue_capacity == 0 {
                    return Err(error(
                        origin,
                        &file,
                        "producer.queue_capacity",
                        "must be positive",
                    ));
                }
                if x.recipes.is_empty() {
                    return Err(error(
                        origin,
                        &file,
                        "producer.recipes",
                        "must not be empty",
                    ));
                }
                if x.spawn_offsets.is_empty() {
                    return Err(error(
                        origin,
                        &file,
                        "producer.spawn_offsets",
                        "must not be empty",
                    ));
                }
                let recipes = resolve_refs(origin, &file, "producer.recipes", &x.recipes)?;
                unique_refs(origin, &file, "producer.recipes", &recipes)?;
                Ok(ResolvedProducer {
                    recipes,
                    queue_capacity: x.queue_capacity,
                    spawn_offsets: x.spawn_offsets,
                })
            })
            .transpose()?;
        catalog.units.insert(
            id.clone(),
            ResolvedUnit {
                id,
                name: value.name,
                footprint: value.footprint,
                movement: value.movement,
                inventory,
                gatherer,
                drop_off,
                producer,
                output_inventory: None,
                extractor: None,
                conveyor: None,
                inserter: None,
                machine: None,
                research: None,
                health: None,
                weapon: None,
                removal: None,
            },
        );
    }
    for src in node_sources {
        let value = src.value;
        let file = src.file;
        let id = local_id(origin, &file, &value.id)?;
        let item = resolve_one(origin, &file, "item", &value.item)?;
        require_kind(
            origin,
            &file,
            "item",
            &item,
            &catalog.resources,
            ResourceKind::Item,
        )?;
        check_finite_positive(origin, &file, "radius", value.radius, false)?;
        catalog.nodes.insert(
            id.clone(),
            ResolvedNode {
                id,
                name: value.name,
                item,
                radius: value.radius,
            },
        );
    }
    for src in production_sources {
        let value = src.value;
        let file = src.file;
        let id = local_id(origin, &file, &value.id)?;
        if value.duration_ms == 0 {
            return Err(error(origin, &file, "duration_ms", "must be positive"));
        }
        let output_unit = resolve_one(origin, &file, "output_unit", &value.output_unit)?;
        if !catalog.units.contains_key(&output_unit) {
            return Err(unknown_ref(
                origin,
                &file,
                "output_unit",
                &output_unit,
                catalog.units.keys(),
            ));
        }
        let mut seen = BTreeSet::new();
        let mut costs = Vec::new();
        for (i, cost) in value.costs.into_iter().enumerate() {
            let field = format!("costs[{i}]");
            if cost.amount == 0 {
                return Err(error(
                    origin,
                    &file,
                    &format!("{field}.amount"),
                    "must be positive",
                ));
            }
            let resource =
                resolve_one(origin, &file, &format!("{field}.resource"), &cost.resource)?;
            let kind = match cost.source {
                CostSource::SlotBalance => ResourceKind::Balance,
                CostSource::ProducerInventory => ResourceKind::Item,
            };
            require_kind(
                origin,
                &file,
                &format!("{field}.resource"),
                &resource,
                &catalog.resources,
                kind,
            )?;
            if !seen.insert((cost.source, resource.clone())) {
                return Err(error(
                    origin,
                    &file,
                    &field,
                    "duplicate cost source and resource",
                ));
            }
            costs.push(ResolvedCost {
                resource,
                amount: cost.amount,
                source: cost.source,
            });
        }
        let refunds = value.cancel_refunds;
        for (name, ratio) in [
            ("queued", refunds.queued),
            ("running", refunds.running),
            ("output_blocked", refunds.output_blocked),
        ] {
            if ratio.denominator == 0 || ratio.numerator > ratio.denominator {
                return Err(error(
                    origin,
                    &file,
                    &format!("cancel_refunds.{name}"),
                    "denominator must be positive and numerator must not exceed it",
                ));
            }
        }
        if value.payment == PaymentPolicy::ReserveThenCharge
            && refunds.queued
                != (RefundRatio {
                    numerator: 1,
                    denominator: 1,
                })
        {
            return Err(error(
                origin,
                &file,
                "cancel_refunds.queued",
                "ReserveThenCharge queued refund must be 1/1",
            ));
        }
        catalog.production.insert(
            id.clone(),
            ResolvedProduction {
                id,
                name: value.name,
                output_unit,
                duration_ms: value.duration_ms,
                costs,
                payment: value.payment,
                cancel_refunds: refunds,
            },
        );
    }
    for (unit_id, unit) in &catalog.units {
        let file = unit_source_file(origin, unit_id, &unit_files)?;
        if let Some(producer) = &unit.producer {
            for (i, recipe) in producer.recipes.iter().enumerate() {
                if !catalog.production.contains_key(recipe) {
                    return Err(unknown_ref(
                        origin,
                        file,
                        &format!("producer.recipes[{i}]"),
                        recipe,
                        catalog.production.keys(),
                    ));
                }
            }
        }
    }
    for (recipe_id, recipe) in &catalog.production {
        let file = definition_source_file(origin, recipe_id, &production_files)?;
        let output = catalog.units.get(&recipe.output_unit).ok_or_else(|| {
            error(
                origin,
                file,
                "output_unit",
                format!("unknown unit `{}`", recipe.output_unit),
            )
        })?;
        if output.movement.is_none() || output.producer.is_some() {
            return Err(error(
                origin,
                file,
                "output_unit",
                "output must be mobile and cannot be a producer",
            ));
        }
    }
    for (unit_id, unit) in &catalog.units {
        let file = unit_source_file(origin, unit_id, &unit_files)?;
        if let Some(producer) = &unit.producer {
            for (recipe_index, recipe) in producer.recipes.iter().enumerate() {
                let Some(def) = catalog.production.get(recipe) else {
                    return Err(unknown_ref(
                        origin,
                        file,
                        &format!("producer.recipes[{recipe_index}]"),
                        recipe,
                        catalog.production.keys(),
                    ));
                };
                for (cost_index, cost) in def.costs.iter().enumerate() {
                    if cost.source != CostSource::ProducerInventory {
                        continue;
                    }
                    let inventory = unit.inventory.as_ref().ok_or_else(|| {
                        error(
                            origin,
                            file,
                            &format!("producer.recipes[{recipe_index}].costs[{cost_index}]"),
                            "ProducerInventory cost requires producer inventory",
                        )
                    })?;
                    if !inventory.accepts.contains(&cost.resource) {
                        return Err(error(
                            origin,
                            file,
                            &format!("producer.recipes[{recipe_index}].costs[{cost_index}]"),
                            "ProducerInventory cost resource is not accepted by producer inventory",
                        ));
                    }
                }
            }
        }
    }
    Ok(catalog)
}

pub(crate) fn validate_content_with_m3(
    origin: &PackageId,
    resources: Vec<SourceDef<ResourceDef>>,
    units: Vec<SourceDef<UnitDef>>,
    nodes: Vec<SourceDef<NodeDef>>,
    production: Vec<SourceDef<ProductionDef>>,
    recipes: Vec<SourceDef<RecipeDef>>,
    technologies: Vec<SourceDef<TechnologyDef>>,
) -> Result<ContentCatalog, ModError> {
    let mut catalog = validate_content_m2(origin, resources, units.clone(), nodes, production)?;
    crate::content_m3::resolve_content_m3(origin, &mut catalog, &units, recipes, technologies)?;
    Ok(catalog)
}

fn unit_source_file<'a>(
    origin: &PackageId,
    id: &ContentId,
    files: &'a BTreeMap<ContentId, String>,
) -> Result<&'a str, ModError> {
    definition_source_file(origin, id, files)
}

fn definition_source_file<'a>(
    origin: &PackageId,
    id: &ContentId,
    files: &'a BTreeMap<ContentId, String>,
) -> Result<&'a str, ModError> {
    files.get(id).map(String::as_str).ok_or_else(|| {
        error(
            origin,
            "content",
            "source",
            format!("source file for `{id}` was not retained"),
        )
    })
}

fn insert_id(
    origin: &PackageId,
    file: &str,
    id: &str,
    ids: &mut BTreeSet<ContentId>,
) -> Result<(), ModError> {
    let id = local_id(origin, file, id)?;
    if !ids.insert(id) {
        return Err(error(
            origin,
            file,
            "id",
            "duplicate definition id across content kinds",
        ));
    }
    Ok(())
}
pub(crate) fn resolve_one(
    origin: &PackageId,
    file: &str,
    field: &str,
    text: &str,
) -> Result<ContentId, ModError> {
    let id = checked_content_ref(origin, file, field, text)?;
    if id.package != *origin {
        return Err(error(
            origin,
            file,
            field,
            "cross-package references are not supported",
        ));
    }
    Ok(id)
}
fn resolve_refs(
    origin: &PackageId,
    file: &str,
    field: &str,
    texts: &[String],
) -> Result<Vec<ContentId>, ModError> {
    texts
        .iter()
        .enumerate()
        .map(|(i, text)| resolve_one(origin, file, &format!("{field}[{i}]"), text))
        .collect()
}
fn unique_refs(
    origin: &PackageId,
    file: &str,
    field: &str,
    refs: &[ContentId],
) -> Result<(), ModError> {
    let mut seen = BTreeSet::new();
    for (i, id) in refs.iter().enumerate() {
        if !seen.insert(id) {
            return Err(error(
                origin,
                file,
                &format!("{field}[{i}]"),
                "duplicate reference",
            ));
        }
    }
    Ok(())
}
fn require_kind(
    origin: &PackageId,
    file: &str,
    field: &str,
    id: &ContentId,
    resources: &BTreeMap<ContentId, ResourceDef>,
    kind: ResourceKind,
) -> Result<(), ModError> {
    match resources.get(id) {
        Some(def) if def.kind == kind => Ok(()),
        Some(_) => Err(error(
            origin,
            file,
            field,
            format!("reference must name a {kind:?} resource"),
        )),
        None => Err(unknown_ref(origin, file, field, id, resources.keys())),
    }
}
fn unknown_ref<'a>(
    origin: &PackageId,
    file: &str,
    field: &str,
    id: &ContentId,
    allowed: impl Iterator<Item = &'a ContentId>,
) -> ModError {
    error(origin, file, field, format!("unknown reference `{id}`"))
        .with_allowed(allowed.map(ToString::to_string).collect())
}
