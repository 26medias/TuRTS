//! Typed M3 content resolution. Portable M3 policy checks live beside this resolver.

use std::collections::BTreeSet;

use turts_mod_api::content::{
    ContentCatalog, ResolvedAmmo, ResolvedConveyor, ResolvedEffect, ResolvedInventory,
    ResolvedItemStack, ResolvedMachine, ResolvedRecipe, ResolvedResearchStation,
    ResolvedTechnology, ResolvedWeapon, SourceDef,
};
use turts_mod_api::schema::{EffectDef, RecipeDef, TechnologyDef, UnitDef};
use turts_mod_api::{ContentId, ModError, PackageId};

use crate::content::{error, local_id, resolve_one};

#[path = "content_m3_validation.rs"]
mod validation;
pub(crate) use validation::{validate_m3, validate_m3_presence};

pub(crate) fn resolve_content_m3(
    origin: &PackageId,
    catalog: &mut ContentCatalog,
    units: &[SourceDef<UnitDef>],
    recipes: Vec<SourceDef<RecipeDef>>,
    technologies: Vec<SourceDef<TechnologyDef>>,
) -> Result<(), ModError> {
    let mut ids = catalog
        .resources
        .keys()
        .chain(catalog.units.keys())
        .chain(catalog.nodes.keys())
        .chain(catalog.production.keys())
        .chain(catalog.recipes.keys())
        .chain(catalog.technologies.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let recipe_ids = collect_ids(origin, &recipes, &mut ids)?;
    let technology_ids = collect_ids(origin, &technologies, &mut ids)?;

    for src in units {
        let id = local_id(origin, &src.file, &src.value.id)?;
        let unit = catalog.units.get_mut(&id).ok_or_else(|| {
            error(
                origin,
                &src.file,
                "id",
                format!("resolved unit `{id}` is missing from the content catalog"),
            )
        })?;
        let output_inventory = src
            .value
            .output_inventory
            .as_ref()
            .map(|inventory| {
                let accepts = inventory
                    .accepts
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        resolve_one(
                            origin,
                            &src.file,
                            &format!("output_inventory.accepts[{index}]"),
                            item,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ResolvedInventory {
                    capacity: inventory.capacity,
                    accepts,
                })
            })
            .transpose()?;
        let conveyor = src
            .value
            .conveyor
            .as_ref()
            .map(|conveyor| {
                let accepts = conveyor
                    .accepts
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        resolve_one(
                            origin,
                            &src.file,
                            &format!("conveyor.accepts[{index}]"),
                            item,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ResolvedConveyor {
                    slots_per_piece: conveyor.slots_per_piece,
                    slots_per_second: conveyor.slots_per_second,
                    accepts,
                })
            })
            .transpose()?;
        let machine = src
            .value
            .machine
            .as_ref()
            .map(|machine| {
                resolve_one(origin, &src.file, "machine.recipe", &machine.recipe)
                    .map(|recipe| ResolvedMachine { recipe })
            })
            .transpose()?;
        let research = src
            .value
            .research
            .as_ref()
            .map(|research| {
                let technologies = research
                    .technologies
                    .iter()
                    .enumerate()
                    .map(|(index, technology)| {
                        resolve_one(
                            origin,
                            &src.file,
                            &format!("research.technologies[{index}]"),
                            technology,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(ResolvedResearchStation {
                    technologies,
                    queue_capacity: research.queue_capacity,
                })
            })
            .transpose()?;
        let weapon = src
            .value
            .weapon
            .as_ref()
            .map(|weapon| {
                let ammo = weapon
                    .ammo
                    .as_ref()
                    .map(|ammo| {
                        resolve_one(origin, &src.file, "weapon.ammo.item", &ammo.item).map(|item| {
                            ResolvedAmmo {
                                item,
                                per_shot: ammo.per_shot,
                            }
                        })
                    })
                    .transpose()?;
                Ok(ResolvedWeapon {
                    range: weapon.range,
                    cooldown_ms: weapon.cooldown_ms,
                    damage: weapon.damage,
                    ammo,
                    targets: weapon.targets,
                })
            })
            .transpose()?;

        unit.output_inventory = output_inventory;
        unit.extractor = src.value.extractor.clone();
        unit.conveyor = conveyor;
        unit.inserter = src.value.inserter.clone();
        unit.machine = machine;
        unit.research = research;
        unit.health = src.value.health;
        unit.weapon = weapon;
        unit.removal = src.value.removal;
    }

    for src in recipes {
        let id = recipe_ids[&src.value.id].clone();
        let inputs = resolve_item_stacks(origin, &src, "inputs")?;
        let outputs = resolve_item_stacks(origin, &src, "outputs")?;
        catalog.source_files.insert(id.clone(), src.file.clone());
        catalog.recipes.insert(
            id.clone(),
            ResolvedRecipe {
                id,
                name: src.value.name,
                inputs,
                outputs,
                duration_ms: src.value.duration_ms,
                cancel_refunds: src.value.cancel_refunds,
            },
        );
    }

    for src in technologies {
        let id = technology_ids[&src.value.id].clone();
        catalog.source_files.insert(id.clone(), src.file.clone());
        let prerequisites = src
            .value
            .prerequisites
            .iter()
            .enumerate()
            .map(|(index, reference)| {
                resolve_one(
                    origin,
                    &src.file,
                    &format!("prerequisites[{index}]"),
                    reference,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let costs = src
            .value
            .costs
            .iter()
            .enumerate()
            .map(|(index, cost)| {
                Ok(crate::content::ResolvedCost {
                    resource: resolve_one(
                        origin,
                        &src.file,
                        &format!("costs[{index}].resource"),
                        &cost.resource,
                    )?,
                    amount: cost.amount,
                    source: cost.source,
                })
            })
            .collect::<Result<Vec<_>, ModError>>()?;
        let effects = src
            .value
            .effects
            .iter()
            .enumerate()
            .map(|(index, effect)| match effect {
                EffectDef::WeaponModifier { unit, stat, add } => {
                    Ok(ResolvedEffect::WeaponModifier {
                        unit: resolve_one(
                            origin,
                            &src.file,
                            &format!("effects[{index}].unit"),
                            unit,
                        )?,
                        stat: *stat,
                        add: *add,
                    })
                }
            })
            .collect::<Result<Vec<_>, ModError>>()?;
        catalog.technologies.insert(
            id.clone(),
            ResolvedTechnology {
                id,
                name: src.value.name,
                prerequisites,
                duration_ms: src.value.duration_ms,
                costs,
                payment: src.value.payment,
                cancel_refunds: src.value.cancel_refunds,
                effects,
            },
        );
    }
    Ok(())
}

fn collect_ids<T>(
    origin: &PackageId,
    sources: &[SourceDef<T>],
    ids: &mut BTreeSet<ContentId>,
) -> Result<std::collections::BTreeMap<String, ContentId>, ModError>
where
    T: DefinitionId,
{
    let mut out = std::collections::BTreeMap::new();
    for src in sources {
        let text = src.value.id();
        let id = local_id(origin, &src.file, text)?;
        if !ids.insert(id.clone()) {
            return Err(error(
                origin,
                &src.file,
                "id",
                "duplicate definition id across content kinds",
            ));
        }
        out.insert(text.to_owned(), id);
    }
    Ok(out)
}

trait DefinitionId {
    fn id(&self) -> &str;
}

impl DefinitionId for RecipeDef {
    fn id(&self) -> &str {
        &self.id
    }
}

impl DefinitionId for TechnologyDef {
    fn id(&self) -> &str {
        &self.id
    }
}

fn resolve_item_stacks(
    origin: &PackageId,
    src: &SourceDef<RecipeDef>,
    field: &str,
) -> Result<Vec<ResolvedItemStack>, ModError> {
    let stacks = if field == "inputs" {
        &src.value.inputs
    } else {
        &src.value.outputs
    };
    stacks
        .iter()
        .enumerate()
        .map(|(index, stack)| {
            Ok(ResolvedItemStack {
                item: resolve_one(
                    origin,
                    &src.file,
                    &format!("{field}[{index}].item"),
                    &stack.item,
                )?,
                amount: stack.amount,
            })
        })
        .collect()
}
