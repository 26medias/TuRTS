//! Data adapters for the public, authored UI source table (design 8.4).
//!
//! These helpers intentionally receive only the replica, the validated catalog, local
//! selection/preferences, and authored guide data. They do not accept a `World` or inspect
//! arbitrary Bevy resources.

use std::collections::BTreeMap;

use turts_mod_api::{
    Behaviour, Binding, ContentId, CostSource, GuidePage, InputMapDef, InterfaceDef, ItemField,
    PrimaryField, Source, Value,
};
use turts_net_protocol::UnitId;

use super::primitives::WidgetAction;
use crate::{content::ClientContent, replica::Replica};

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct ItemData {
    pub fields: Vec<(ItemField, serde_json::Value)>,
}

impl ItemData {
    pub fn get(&self, field: ItemField) -> Option<&serde_json::Value> {
        self.fields
            .iter()
            .find(|(key, _)| *key == field)
            .map(|(_, value)| value)
    }

    fn set(&mut self, field: ItemField, value: impl serde::Serialize) {
        if let Ok(value) = serde_json::to_value(value) {
            self.fields.push((field, value));
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceData {
    pub scalar: Option<serde_json::Value>,
    pub items: Vec<ItemData>,
}

#[derive(Clone, Copy)]
pub struct SourceContext<'a> {
    pub replica: &'a Replica,
    pub content: &'a ClientContent,
    pub selection: &'a [u32],
    pub owner_slot: u8,
    pub interface: Option<&'a InterfaceDef>,
    pub interface_package: Option<&'a str>,
    pub control_groups: &'a BTreeMap<u8, Vec<u32>>,
    pub alerts: &'a [serde_json::Value],
    pub bindings: &'a BTreeMap<String, Vec<Binding>>,
    pub input_map: &'a InputMapDef,
    pub guide: &'a [GuidePage],
    pub guide_open: bool,
    pub last_notice: Option<&'a str>,
}

/// Read one of the sources listed in design 8.4.
pub fn read_source(source: &Source, context: SourceContext<'_>) -> SourceData {
    match source {
        Source::Balance(id) => SourceData {
            scalar: context
                .replica
                .economy
                .balances
                .iter()
                .find(|balance| balance.resource == *id)
                .and_then(|balance| balance.on_hand.checked_sub(balance.reserved))
                .map(|value| serde_json::json!(value)),
            items: Vec::new(),
        },
        Source::Balances => SourceData {
            scalar: None,
            items: context
                .replica
                .economy
                .balances
                .iter()
                .map(|balance| {
                    let mut item = ItemData::default();
                    item.set(
                        ItemField::Name,
                        resource_name(&balance.resource, context.content),
                    );
                    if let Some(icon) = resource_icon(context, &balance.resource) {
                        item.set(ItemField::Icon, icon);
                    }
                    item.set(
                        ItemField::Count,
                        balance.on_hand.saturating_sub(balance.reserved),
                    );
                    item
                })
                .collect(),
        },
        Source::SelectionUnits => selection_units(context),
        Source::CommandCard => command_card(context),
        Source::BuildMenu => build_menu(context),
        Source::ProductionQueue => production_queue(context),
        Source::ControlGroups => control_groups(context),
        Source::Alerts => alert_rows(context.alerts),
        Source::Bindings => binding_rows(context),
        Source::Guide => guide_rows(context),
    }
}

pub fn read_value(value: &Value, context: SourceContext<'_>) -> serde_json::Value {
    read_value_for_item(value, context, None)
}

pub fn read_value_for_item(
    value: &Value,
    context: SourceContext<'_>,
    item: Option<&ItemData>,
) -> serde_json::Value {
    match value {
        Value::Balance(id) => read_source(&Source::Balance(id.clone()), context)
            .scalar
            .unwrap_or(serde_json::Value::Null),
        Value::Balances => serde_json::Value::Array(
            read_source(&Source::Balances, context)
                .items
                .iter()
                .map(item_value)
                .collect(),
        ),
        Value::SelectionPrimary(field) => primary_value(*field, context),
        Value::SelectionUnits => source_items_value(Source::SelectionUnits, context),
        Value::CommandCard => source_items_value(Source::CommandCard, context),
        Value::BuildMenu => source_items_value(Source::BuildMenu, context),
        Value::ProductionQueue => source_items_value(Source::ProductionQueue, context),
        Value::ControlGroups => source_items_value(Source::ControlGroups, context),
        Value::Alerts => source_items_value(Source::Alerts, context),
        Value::Bindings => source_items_value(Source::Bindings, context),
        Value::Guide => serde_json::Value::String(
            context
                .guide
                .iter()
                .flat_map(|page| [page.title.as_str(), page.body.as_str()])
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n"),
        ),
        Value::GuideOpen => serde_json::Value::Bool(context.guide_open),
        Value::LastNotice => context
            .last_notice
            .map_or(serde_json::Value::Null, |text| serde_json::json!(text)),
        Value::Item(field) => item
            .and_then(|item| item.get(*field))
            .cloned()
            .unwrap_or(serde_json::Value::Null),
        Value::Literal(value) => serde_json::Value::String(value.clone()),
        Value::Format(pattern, values) => {
            serde_json::Value::String(format_value(pattern, values, context, item))
        }
    }
}

pub fn read_primary(field: PrimaryField, context: SourceContext<'_>) -> serde_json::Value {
    primary_value(field, context)
}

fn primary_value(field: PrimaryField, context: SourceContext<'_>) -> serde_json::Value {
    let Some(id) = context.selection.first() else {
        return serde_json::Value::Null;
    };
    let Some(unit) = context.replica.units.get(id) else {
        return serde_json::Value::Null;
    };
    let parsed = ContentId::parse(&unit.content_id).ok();
    let definition = parsed
        .as_ref()
        .and_then(|id| context.content.catalog.units.get(id));
    let health = context
        .replica
        .healths
        .iter()
        .find(|record| record.unit == UnitId(*id))
        .map(|record| record.current);
    match field {
        PrimaryField::Name => {
            serde_json::json!(definition.map_or(unit.content_id.as_str(), |def| def.name.as_str()))
        }
        PrimaryField::Icon | PrimaryField::Portrait => serde_json::json!(unit_asset(
            context,
            &unit.content_id,
            field == PrimaryField::Icon
        )),
        PrimaryField::Owner => serde_json::json!(unit.owner_slot),
        PrimaryField::Health => {
            health.map_or(serde_json::Value::Null, |value| serde_json::json!(value))
        }
        PrimaryField::MaxHealth => definition
            .and_then(|def| def.health.as_ref())
            .map_or(serde_json::Value::Null, |health| {
                serde_json::json!(health.max)
            }),
        PrimaryField::Inventory if unit.owner_slot == context.owner_slot => {
            inventory_value(context, *id, definition)
        }
        PrimaryField::Inventory => serde_json::Value::Null,
        PrimaryField::Damage if unit.owner_slot == context.owner_slot => context
            .replica
            .weapons
            .iter()
            .find(|weapon| weapon.unit.0 == *id)
            .and_then(|weapon| weapon.damage.as_ref().ok())
            .map_or(serde_json::Value::Null, |damage| serde_json::json!(damage)),
        PrimaryField::Damage => serde_json::Value::Null,
        PrimaryField::Range => definition
            .and_then(|def| def.weapon.as_ref())
            .map_or(serde_json::Value::Null, |weapon| {
                serde_json::json!(weapon.range)
            }),
        PrimaryField::Ammo if unit.owner_slot == context.owner_slot => definition
            .and_then(|def| def.weapon.as_ref())
            .and_then(|weapon| weapon.ammo.as_ref())
            .and_then(|ammo| {
                let resource = ammo.item.to_string();
                context
                    .replica
                    .economy
                    .inventories
                    .iter()
                    .find(|inventory| {
                        inventory.unit.0 == *id
                            && inventory.port == turts_net_protocol::InventoryPort::Main
                    })
                    .map(|inventory| {
                        serde_json::json!(
                            inventory
                                .items
                                .iter()
                                .find(|item| item.resource == resource)
                                .map_or(0, |item| item.on_hand.saturating_sub(item.reserved))
                        )
                    })
            })
            .unwrap_or(serde_json::Value::Null),
        PrimaryField::Ammo => serde_json::Value::Null,
        PrimaryField::Cooldown if unit.owner_slot == context.owner_slot => context
            .replica
            .weapons
            .iter()
            .find(|weapon| weapon.unit.0 == *id)
            .map_or(serde_json::Value::Null, |weapon| {
                serde_json::json!(weapon.cooldown as f64 / context.replica.tick_hz.max(1) as f64)
            }),
        PrimaryField::Cooldown => serde_json::Value::Null,
    }
}

fn selection_units(context: SourceContext<'_>) -> SourceData {
    SourceData {
        scalar: None,
        items: context
            .selection
            .iter()
            .filter_map(|id| {
                let unit = context.replica.units.get(id)?;
                let mut item = ItemData::default();
                item.set(
                    ItemField::Name,
                    unit_name(&unit.content_id, context.content),
                );
                item.set(ItemField::Icon, unit_asset(context, &unit.content_id, true));
                item.set(
                    ItemField::Count,
                    context
                        .selection
                        .iter()
                        .filter(|other| {
                            context
                                .replica
                                .units
                                .get(other)
                                .is_some_and(|selected| selected.content_id == unit.content_id)
                        })
                        .count(),
                );
                item.set(
                    ItemField::Health,
                    context
                        .replica
                        .healths
                        .iter()
                        .find(|health| health.unit.0 == *id)
                        .map(|health| health.current),
                );
                item.set(ItemField::Action, WidgetAction::Select { unit: *id });
                Some(item)
            })
            .collect(),
    }
}

fn command_card(context: SourceContext<'_>) -> SourceData {
    let selected = context
        .selection
        .iter()
        .filter_map(|id| context.replica.units.get(id).map(|unit| (*id, unit)))
        .collect::<Vec<_>>();
    if selected.is_empty()
        || !selected
            .iter()
            .any(|(_, unit)| unit.owner_slot == context.owner_slot)
    {
        return SourceData::default();
    }
    let mut orders = context.content.catalog.orders.iter().collect::<Vec<_>>();
    orders.sort_by_key(|(id, order)| {
        (
            order
                .card
                .map(|cell| (cell.row, cell.column))
                .unwrap_or((u8::MAX, u8::MAX)),
            (*id).clone(),
        )
    });
    SourceData {
        scalar: None,
        items: orders
            .into_iter()
            .filter(|(_, order)| {
                selected.iter().any(|(_, unit)| {
                    unit.owner_slot == context.owner_slot
                        && order_unit_matches(order, &unit.content_id)
                })
            })
            .map(|(id, order)| {
                let eligible = selected.iter().any(|(_, unit)| {
                    unit.owner_slot == context.owner_slot
                        && order_unit_matches(order, &unit.content_id)
                        && ContentId::parse(&unit.content_id)
                            .ok()
                            .and_then(|content_id| context.content.catalog.units.get(&content_id))
                            .is_some_and(|definition| capability_matches(order, definition))
                });
                let mut item = ItemData::default();
                item.set(ItemField::Icon, qualify_asset(&id.package.0, &order.icon));
                item.set(ItemField::Name, order.name.clone());
                item.set(ItemField::Body, order.description.clone());
                item.set(
                    ItemField::Action,
                    WidgetAction::Order { id: id.to_string() },
                );
                item.set(ItemField::Enabled, eligible);
                item.set(
                    ItemField::Reason,
                    if eligible {
                        String::new()
                    } else {
                        reason(
                            context.interface,
                            "not_applicable",
                            "Requires a selected unit with this capability",
                        )
                    },
                );
                item.set(ItemField::Title, current_hotkey(context, &order.hotkey));
                item.set(ItemField::CardCell, order.card);
                item
            })
            .collect(),
    }
}

fn build_menu(context: SourceContext<'_>) -> SourceData {
    let Some(primary) = context
        .selection
        .first()
        .and_then(|id| context.replica.units.get(id))
    else {
        return SourceData::default();
    };
    if primary.owner_slot != context.owner_slot {
        return SourceData::default();
    }
    let Some(id) = ContentId::parse(&primary.content_id).ok() else {
        return SourceData::default();
    };
    let Some(unit) = context.content.catalog.units.get(&id) else {
        return SourceData::default();
    };
    let producer_id = context.selection[0];
    let mut rows = Vec::new();
    if let Some(producer) = &unit.producer {
        for recipe_id in &producer.recipes {
            if let Some(recipe) = context.content.catalog.production.get(recipe_id) {
                let queue = context
                    .replica
                    .economy
                    .queues
                    .iter()
                    .find(|queue| queue.producer.0 == producer_id);
                let queue_len = queue.map_or(0, |queue| queue.jobs.len());
                let queue_enabled = queue.is_none_or(|queue| queue.enabled);
                let affordable = costs_affordable(context, context.selection[0], &recipe.costs);
                let can_queue = queue_enabled && queue_len < usize::from(producer.queue_capacity);
                let mut row = ItemData::default();
                row.set(ItemField::Name, recipe.name.clone());
                row.set(
                    ItemField::Icon,
                    unit_asset(context, &recipe.output_unit.to_string(), true),
                );
                row.set(
                    ItemField::Action,
                    WidgetAction::Produce {
                        producer: producer_id,
                        content_id: recipe.id.to_string(),
                    },
                );
                row.set(ItemField::Enabled, can_queue && affordable);
                row.set(
                    ItemField::Reason,
                    if !queue_enabled {
                        reason(
                            context.interface,
                            "queue_disabled",
                            "Production is disabled",
                        )
                    } else if !can_queue {
                        reason(context.interface, "queue_full", "Production queue is full")
                    } else if !affordable {
                        affordability_reason(context, producer_id, &recipe.costs)
                    } else {
                        String::new()
                    },
                );
                row.set(ItemField::Cost, cost_label(&recipe.costs, context.content));
                row.set(ItemField::Time, recipe.duration_ms as f64 / 1000.0);
                rows.push(row);
            }
        }
    }
    if let Some(station) = &unit.research {
        for technology_id in &station.technologies {
            if let Some(technology) = context.content.catalog.technologies.get(technology_id) {
                let queue = context
                    .replica
                    .economy
                    .queues
                    .iter()
                    .find(|queue| queue.producer.0 == producer_id);
                let queue_len = queue.map_or(0, |queue| queue.jobs.len());
                let queue_enabled = queue.is_none_or(|queue| queue.enabled);
                let complete = context.replica.research.iter().any(|record| {
                    record.slot.0 == context.owner_slot
                        && record.completed.contains(&technology.id.to_string())
                });
                let prerequisite = technology.prerequisites.iter().find(|required| {
                    !context.replica.research.iter().any(|record| {
                        record.slot.0 == context.owner_slot
                            && record.completed.contains(&required.to_string())
                    })
                });
                let can_queue = queue_enabled && queue_len < usize::from(station.queue_capacity);
                let in_progress = queue.is_some_and(|queue| {
                    queue
                        .jobs
                        .iter()
                        .any(|job| job.recipe == technology.id.to_string())
                });
                let affordable = costs_affordable(context, producer_id, &technology.costs);
                let enabled =
                    !complete && !in_progress && prerequisite.is_none() && can_queue && affordable;
                let mut row = ItemData::default();
                row.set(ItemField::Name, technology.name.clone());
                row.set(
                    ItemField::Action,
                    WidgetAction::Produce {
                        producer: producer_id,
                        content_id: technology.id.to_string(),
                    },
                );
                row.set(ItemField::Enabled, enabled);
                let why = if complete {
                    reason(
                        context.interface,
                        "already_researched",
                        "Already researched",
                    )
                } else if in_progress {
                    reason(
                        context.interface,
                        "already_researching",
                        "Already in progress",
                    )
                } else if let Some(prerequisite) = prerequisite {
                    context
                        .content
                        .catalog
                        .technologies
                        .get(prerequisite)
                        .map_or_else(
                            || prerequisite.to_string(),
                            |required| required.name.clone(),
                        )
                } else if !queue_enabled {
                    reason(
                        context.interface,
                        "queue_disabled",
                        "Production is disabled",
                    )
                } else if !can_queue {
                    reason(context.interface, "queue_full", "Production queue is full")
                } else if !affordable {
                    affordability_reason(context, producer_id, &technology.costs)
                } else {
                    String::new()
                };
                row.set(ItemField::Reason, why);
                row.set(
                    ItemField::Cost,
                    cost_label(&technology.costs, context.content),
                );
                row.set(ItemField::Time, technology.duration_ms as f64 / 1000.0);
                rows.push(row);
            }
        }
    }
    SourceData {
        scalar: None,
        items: rows,
    }
}

fn production_queue(context: SourceContext<'_>) -> SourceData {
    let selected = context.selection.first().copied();
    let mut items = Vec::new();
    if let Some(selected) = selected.filter(|id| {
        context
            .replica
            .units
            .get(id)
            .is_some_and(|unit| unit.owner_slot == context.owner_slot)
    }) && let Some(queue) = context
        .replica
        .economy
        .queues
        .iter()
        .find(|queue| queue.producer.0 == selected)
    {
        for job in &queue.jobs {
            let mut item = ItemData::default();
            let name = job_name(context, &job.recipe);
            item.set(ItemField::Name, name);
            item.set(
                ItemField::Progress,
                if job.duration_ticks == 0 {
                    1.0
                } else {
                    (job.elapsed_ticks as f64 / job.duration_ticks as f64).clamp(0.0, 1.0)
                },
            );
            if let Some(icon) = job_icon(context, &job.recipe) {
                item.set(ItemField::Icon, icon);
            }
            let ticks_left = job.duration_ticks.saturating_sub(job.elapsed_ticks);
            item.set(
                ItemField::RemainingSeconds,
                ticks_left as f64 / context.replica.tick_hz.max(1) as f64,
            );
            item.set(
                ItemField::CancelAction,
                WidgetAction::CancelProduction {
                    producer: selected,
                    job: job.id.0,
                },
            );
            items.push(item);
        }
    }
    SourceData {
        scalar: None,
        items,
    }
}

fn capability_matches(order: &turts_mod_api::OrderDef, unit: &turts_mod_api::ResolvedUnit) -> bool {
    use turts_mod_api::TargetKind;
    match order.behaviour {
        Behaviour::Move => unit.movement.is_some(),
        Behaviour::Attack => match order.target {
            TargetKind::Point => unit.movement.is_some(),
            TargetKind::Unit => unit.weapon.is_some(),
            TargetKind::PointOrUnit => unit.movement.is_some() || unit.weapon.is_some(),
            _ => false,
        },
        Behaviour::Gather => unit.gatherer.is_some(),
        Behaviour::Produce => unit.producer.is_some() || unit.research.is_some(),
        Behaviour::CancelProduction => match order.target {
            TargetKind::Job => {
                unit.producer.is_some() || unit.research.is_some() || unit.machine.is_some()
            }
            _ => unit.producer.is_some() || unit.research.is_some(),
        },
        Behaviour::Remove => unit
            .removal
            .as_ref()
            .is_some_and(|policy| policy.by_command),
        Behaviour::SetEnabled(_) => {
            unit.extractor.is_some()
                || unit.inserter.is_some()
                || unit.machine.is_some()
                || unit.research.is_some()
                || unit.producer.is_some()
                || unit.weapon.is_some()
        }
        Behaviour::Stop | Behaviour::Hold => unit.movement.is_some() || unit.weapon.is_some(),
    }
}

fn costs_affordable(
    context: SourceContext<'_>,
    producer: u32,
    costs: &[turts_mod_api::ResolvedCost],
) -> bool {
    let mut required = BTreeMap::<(CostSource, ContentId), u128>::new();
    for cost in costs {
        let amount = required
            .entry((cost.source, cost.resource.clone()))
            .or_default();
        let Some(sum) = amount.checked_add(u128::from(cost.amount)) else {
            return false;
        };
        *amount = sum;
    }
    required.into_iter().all(|((source, resource), amount)| {
        let Ok(amount) = u64::try_from(amount) else {
            return false;
        };
        let resource = resource.to_string();
        let available = match source {
            CostSource::SlotBalance => context
                .replica
                .economy
                .balances
                .iter()
                .find(|balance| balance.resource == resource)
                .and_then(|balance| balance.on_hand.checked_sub(balance.reserved)),
            CostSource::ProducerInventory => context
                .replica
                .economy
                .inventories
                .iter()
                .find(|inventory| {
                    inventory.unit.0 == producer
                        && inventory.port == turts_net_protocol::InventoryPort::Main
                })
                .and_then(|inventory| {
                    inventory
                        .items
                        .iter()
                        .find(|item| item.resource == resource)
                })
                .and_then(|balance| balance.on_hand.checked_sub(balance.reserved)),
        };
        amount == 0 || available.is_some_and(|available| available >= amount)
    })
}

fn order_unit_matches(order: &turts_mod_api::OrderDef, unit_content_id: &str) -> bool {
    order.units.is_empty() || order.units.iter().any(|unit| unit == unit_content_id)
}

fn cost_label(costs: &[turts_mod_api::ResolvedCost], content: &ClientContent) -> String {
    let mut totals = BTreeMap::<(CostSource, String), u128>::new();
    for cost in costs {
        let total = totals
            .entry((cost.source, cost.resource.to_string()))
            .or_default();
        *total = total.saturating_add(u128::from(cost.amount));
    }
    totals
        .into_iter()
        .map(|((source, resource), amount)| {
            let suffix = match source {
                CostSource::SlotBalance => "",
                CostSource::ProducerInventory => " (Main)",
            };
            format!("{amount} {}{suffix}", resource_name(&resource, content))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn inventory_value(
    context: SourceContext<'_>,
    unit_id: u32,
    definition: Option<&turts_mod_api::ResolvedUnit>,
) -> serde_json::Value {
    use turts_net_protocol::InventoryPort;
    let mut rows = Vec::new();
    for (port, label, inventory_def) in [
        (
            InventoryPort::Main,
            "Main",
            definition.and_then(|unit| unit.inventory.as_ref()),
        ),
        (
            InventoryPort::Output,
            "Output",
            definition.and_then(|unit| unit.output_inventory.as_ref()),
        ),
    ] {
        let state = context
            .replica
            .economy
            .inventories
            .iter()
            .find(|inventory| inventory.unit.0 == unit_id && inventory.port == port);
        if let Some(state) = state {
            for balance in &state.items {
                rows.push(serde_json::json!({
                    "port": label,
                    "name": resource_name(&balance.resource, context.content),
                    "count": balance.on_hand.saturating_sub(balance.reserved),
                    "on_hand": balance.on_hand,
                    "reserved": balance.reserved,
                    "capacity": inventory_def.map(|inventory| inventory.capacity),
                }));
            }
        }
        if inventory_def.is_some() && state.is_none_or(|state| state.items.is_empty()) {
            rows.push(serde_json::json!({"port": label, "name": format!("{label} inventory"), "count": 0, "capacity": inventory_def.map(|inventory| inventory.capacity)}));
        }
    }
    serde_json::Value::Array(rows)
}

fn qualify_asset(package: &str, local: &str) -> String {
    if local.contains(':') {
        local.to_owned()
    } else {
        format!("{package}:{local}")
    }
}

fn resource_icon(_context: SourceContext<'_>, _resource_id: &str) -> Option<String> {
    // Resource definitions have no authored presentation field; do not guess an image id.
    None
}

fn job_icon(context: SourceContext<'_>, id: &str) -> Option<String> {
    ContentId::parse(id)
        .ok()
        .and_then(|id| context.content.catalog.production.get(&id))
        .and_then(|production| unit_asset(context, &production.output_unit.to_string(), true))
}

fn job_name(context: SourceContext<'_>, id: &str) -> String {
    let Ok(content_id) = ContentId::parse(id) else {
        return id.to_owned();
    };
    context
        .content
        .catalog
        .production
        .get(&content_id)
        .map(|entry| entry.name.clone())
        .or_else(|| {
            context
                .content
                .catalog
                .technologies
                .get(&content_id)
                .map(|entry| entry.name.clone())
        })
        .or_else(|| {
            context
                .content
                .catalog
                .recipes
                .get(&content_id)
                .map(|entry| entry.name.clone())
        })
        .unwrap_or_else(|| id.to_owned())
}

pub fn current_hotkey(context: SourceContext<'_>, action: &str) -> String {
    turts_interaction::input_map::effective_actions_with_runtime(
        context.input_map,
        context.bindings,
    )
    .get(action)
    .into_iter()
    .flatten()
    .map(binding_label)
    .collect::<Vec<_>>()
    .join(", ")
}

fn format_value(
    pattern: &str,
    values: &[Value],
    context: SourceContext<'_>,
    item: Option<&ItemData>,
) -> String {
    let mut output = String::with_capacity(pattern.len());
    let chars = pattern.char_indices().collect::<Vec<_>>();
    let mut cursor = 0;
    let mut i = 0;
    while i < chars.len() {
        let (start, ch) = chars[i];
        if ch == '{' {
            let mut j = i + 1;
            let mut index = 0usize;
            let mut has_digit = false;
            while j < chars.len() && chars[j].1.is_ascii_digit() {
                has_digit = true;
                index = index
                    .saturating_mul(10)
                    .saturating_add((chars[j].1 as u8 - b'0') as usize);
                j += 1;
            }
            if has_digit && j < chars.len() && chars[j].1 == '}' {
                output.push_str(&pattern[cursor..start]);
                if let Some(value) = values.get(index) {
                    output.push_str(&display_text_value(&read_value_for_item(
                        value, context, item,
                    )));
                } else {
                    output.push_str(&pattern[start..chars[j].0 + 1]);
                }
                cursor = chars[j].0 + 1;
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    output.push_str(&pattern[cursor..]);
    output
}

fn affordability_reason(
    context: SourceContext<'_>,
    producer: u32,
    costs: &[turts_mod_api::ResolvedCost],
) -> String {
    let mut required = BTreeMap::<(CostSource, String), u128>::new();
    for cost in costs {
        let entry = required
            .entry((cost.source, cost.resource.to_string()))
            .or_default();
        *entry = entry.saturating_add(u128::from(cost.amount));
    }
    let shortages = required
        .into_iter()
        .filter_map(|((source, resource), amount)| {
            let available = match source {
                CostSource::SlotBalance => context
                    .replica
                    .economy
                    .balances
                    .iter()
                    .find(|balance| balance.resource == resource)
                    .map(|balance| balance.on_hand.saturating_sub(balance.reserved)),
                CostSource::ProducerInventory => context
                    .replica
                    .economy
                    .inventories
                    .iter()
                    .find(|inventory| {
                        inventory.unit.0 == producer
                            && inventory.port == turts_net_protocol::InventoryPort::Main
                    })
                    .and_then(|inventory| {
                        inventory
                            .items
                            .iter()
                            .find(|item| item.resource == resource)
                    })
                    .map(|balance| balance.on_hand.saturating_sub(balance.reserved)),
            }
            .unwrap_or(0);
            (u128::from(available) < amount).then(|| {
                format!(
                    "Needs {amount} {} ({available}/{amount})",
                    resource_name(&resource, context.content),
                )
            })
        })
        .collect::<Vec<_>>()
        .join("; ");
    if shortages.is_empty() {
        "Insufficient resources".to_owned()
    } else {
        shortages
    }
}

fn reason(interface: Option<&InterfaceDef>, key: &str, fallback: &str) -> String {
    interface
        .and_then(|interface| interface.reasons.get(key))
        .cloned()
        .unwrap_or_else(|| fallback.to_owned())
}

fn resource_name(id: &str, content: &ClientContent) -> String {
    ContentId::parse(id)
        .ok()
        .and_then(|id| content.catalog.resources.get(&id))
        .map_or_else(|| id.to_owned(), |resource| resource.name.clone())
}

fn control_groups(context: SourceContext<'_>) -> SourceData {
    SourceData {
        scalar: None,
        items: context
            .control_groups
            .iter()
            .map(|(group, units)| {
                let mut item = ItemData::default();
                item.set(ItemField::Name, format!("{group}"));
                item.set(ItemField::Count, units.len());
                if let Some(unit) = units.first().and_then(|id| context.replica.units.get(id)) {
                    item.set(ItemField::Icon, unit_asset(context, &unit.content_id, true));
                }
                item
            })
            .collect(),
    }
}

fn alert_rows(alerts: &[serde_json::Value]) -> SourceData {
    SourceData {
        scalar: None,
        items: alerts
            .iter()
            .rev()
            .take(5)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .map(|alert| {
                let mut item = ItemData::default();
                let text = ["text", "message"]
                    .iter()
                    .filter_map(|key| alert.get(*key).and_then(serde_json::Value::as_str))
                    .find(|text| !text.trim().is_empty())
                    .unwrap_or("");
                item.set(ItemField::Name, text);
                item.set(
                    ItemField::Title,
                    alert
                        .get("title")
                        .and_then(serde_json::Value::as_str)
                        .filter(|title| !title.trim().is_empty())
                        .unwrap_or(text),
                );
                item.set(ItemField::Body, text);
                if let (Some(x), Some(y)) = (
                    alert
                        .get("position")
                        .and_then(|position| position.get("x"))
                        .and_then(serde_json::Value::as_f64),
                    alert
                        .get("position")
                        .and_then(|position| position.get("y"))
                        .and_then(serde_json::Value::as_f64),
                ) && x.is_finite()
                    && y.is_finite()
                    && x.abs() <= f32::MAX as f64
                    && y.abs() <= f32::MAX as f64
                {
                    item.set(
                        ItemField::Action,
                        WidgetAction::CenterCamera {
                            x: x as f32,
                            y: y as f32,
                        },
                    );
                }
                item
            })
            .collect(),
    }
}

fn binding_rows(context: SourceContext<'_>) -> SourceData {
    let actions = turts_interaction::input_map::effective_actions_with_runtime(
        context.input_map,
        context.bindings,
    );
    SourceData {
        scalar: None,
        items: actions
            .iter()
            .map(|(name, bindings)| {
                let mut item = ItemData::default();
                item.set(ItemField::Name, humanize_action(name));
                let label = bindings
                    .iter()
                    .map(binding_label)
                    .collect::<Vec<_>>()
                    .join(", ");
                item.set(ItemField::Title, label);
                item
            })
            .collect(),
    }
}

fn humanize_action(action: &str) -> String {
    let known = if let Some(group) = action.strip_prefix("group_assign_") {
        format!("Assign group {group}")
    } else if let Some(group) = action.strip_prefix("group_recall_") {
        format!("Select group {group}")
    } else if let Some(group) = action.strip_prefix("group_add_") {
        format!("Add to group {group}")
    } else if let Some(direction) = action.strip_prefix("camera_pan_") {
        format!("Pan camera {direction}")
    } else {
        match action {
            "toggle_guide" => "Toggle guide",
            "centre_selection" => "Centre selection",
            "camera_zoom" => "Zoom camera",
            "camera_rotate" => "Rotate camera",
            "selection_add" => "Add to selection",
            "selection_remove" => "Remove from selection",
            other => other,
        }
        .to_owned()
    };
    known
        .replace(['_', '-'], " ")
        .split_whitespace()
        .enumerate()
        .map(|(index, word)| {
            if index == 0 {
                let mut chars = word.chars();
                chars.next().map_or_else(String::new, |first| {
                    first.to_uppercase().chain(chars).collect()
                })
            } else {
                word.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn guide_rows(context: SourceContext<'_>) -> SourceData {
    SourceData {
        scalar: None,
        items: context
            .guide
            .iter()
            .map(|page| {
                let mut item = ItemData::default();
                item.set(ItemField::Name, page.title.clone());
                item.set(ItemField::Title, page.title.clone());
                item.set(ItemField::Body, page.body.clone());
                item
            })
            .collect(),
    }
}

fn unit_asset(context: SourceContext<'_>, content_id: &str, icon: bool) -> Option<String> {
    let presentation = context.interface?.units.get(content_id)?;
    let local = if icon {
        presentation.icon.as_deref()
    } else {
        presentation.portrait.as_deref()
    }?;
    Some(qualify_asset(context.interface_package?, local))
}

fn binding_label(binding: &Binding) -> String {
    match binding {
        Binding::Key { code, mods } => format!("{}{}", modifier_label(mods), key_label(code)),
        Binding::Pointer { button, mods } => format!(
            "{}{} click",
            modifier_label(mods),
            match button {
                turts_mod_api::Button::Left => "Left",
                turts_mod_api::Button::Right => "Right",
                turts_mod_api::Button::Middle => "Middle",
                turts_mod_api::Button::Back => "Back",
                turts_mod_api::Button::Forward => "Forward",
            }
        ),
        Binding::Wheel(direction) => format!(
            "Wheel {}",
            match direction {
                turts_mod_api::WheelDirection::Up => "up",
                turts_mod_api::WheelDirection::Down => "down",
            }
        ),
    }
}

fn modifier_label(mods: &[turts_mod_api::Mod]) -> String {
    mods.iter()
        .map(|modifier| {
            format!(
                "{}+",
                match modifier {
                    turts_mod_api::Mod::Shift => "Shift",
                    turts_mod_api::Mod::Ctrl => "Ctrl",
                    turts_mod_api::Mod::Alt => "Alt",
                }
            )
        })
        .collect()
}

fn key_label(code: &str) -> String {
    match code {
        code if code.starts_with("Key") && code.len() == 4 => code[3..].to_owned(),
        code if code.starts_with("Digit") && code.len() == 6 => code[5..].to_owned(),
        "Escape" => "Esc".to_owned(),
        "Space" => "Space".to_owned(),
        "ArrowUp" => "↑".to_owned(),
        "ArrowDown" => "↓".to_owned(),
        "ArrowLeft" => "←".to_owned(),
        "ArrowRight" => "→".to_owned(),
        other => other.to_owned(),
    }
}

fn unit_name(id: &str, content: &ClientContent) -> String {
    ContentId::parse(id)
        .ok()
        .and_then(|id| content.catalog.units.get(&id))
        .map_or_else(|| id.to_string(), |unit| unit.name.clone())
}

/// Render values used by authored text widgets. Selection inventory remains a typed JSON
/// array for consumers that need its fields; only its text representation becomes readable.
pub fn display_text_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(value) => value.clone(),
        serde_json::Value::Array(rows)
            if !rows.is_empty() && rows.iter().all(is_inventory_text_row) =>
        {
            rows.iter()
                .map(inventory_text_row)
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => value.to_string(),
    }
}

fn is_inventory_text_row(value: &serde_json::Value) -> bool {
    let Some(row) = value.as_object() else {
        return false;
    };
    row.contains_key("port")
        && row.contains_key("name")
        && row.contains_key("count")
        && row.contains_key("capacity")
}

fn inventory_text_row(value: &serde_json::Value) -> String {
    let Some(row) = value.as_object() else {
        return value.to_string();
    };
    let port = display_text_value(row.get("port").unwrap_or(&serde_json::Value::Null));
    let name = display_text_value(row.get("name").unwrap_or(&serde_json::Value::Null));
    let count = display_text_value(row.get("count").unwrap_or(&serde_json::Value::Null));
    let mut line = format!("{port}: {name} {count} available");
    if let Some(on_hand) = row.get("on_hand") {
        line.push_str(&format!(" ({on_hand} on hand"));
        if let Some(reserved) = row.get("reserved")
            && reserved.as_u64().is_some_and(|amount| amount > 0)
        {
            line.push_str(&format!(", {reserved} reserved"));
        }
        line.push(')');
    }
    if let Some(capacity) = row.get("capacity").filter(|value| !value.is_null()) {
        line.push_str(&format!(" (capacity {capacity})"));
    }
    line
}

fn source_items_value(source: Source, context: SourceContext<'_>) -> serde_json::Value {
    serde_json::Value::Array(
        read_source(&source, context)
            .items
            .iter()
            .map(item_value)
            .collect(),
    )
}

fn item_value(item: &ItemData) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    for (field, value) in &item.fields {
        object.insert(item_field_key(*field).to_owned(), value.clone());
    }
    serde_json::Value::Object(object)
}

fn item_field_key(field: ItemField) -> &'static str {
    match field {
        ItemField::Icon => "icon",
        ItemField::Name => "name",
        ItemField::Action => "action",
        ItemField::Enabled => "enabled",
        ItemField::Reason => "reason",
        ItemField::CardCell => "card_cell",
        ItemField::Health => "health",
        ItemField::Count => "count",
        ItemField::Cost => "cost",
        ItemField::Time => "time",
        ItemField::Progress => "progress",
        ItemField::RemainingSeconds => "remaining_seconds",
        ItemField::CancelAction => "cancel_action",
        ItemField::Title => "title",
        ItemField::Body => "body",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replica::UnitReplica;
    use std::collections::HashMap;
    use std::collections::VecDeque;
    use turts_mod_api::interface::{WorldHealthBarStyle, WorldTooltipStyle};
    use turts_mod_api::{
        Behaviour, CameraStart, CancelRefunds, CardCell, ContentId, Footprint, MovementDef,
        OrderDef, PaymentPolicy, RefundRatio, ResolvedProduction, ResolvedTechnology, TargetKind,
        UnitPresentation,
    };

    #[test]
    fn alert_rows_put_the_real_player_text_in_the_rendered_title_field() {
        let rows = alert_rows(&[
            serde_json::json!({"id":"damage","text":"Unit under attack","position":{"x":-4.0,"y":80.0}}),
        ]);
        assert_eq!(
            rows.items[0]
                .get(ItemField::Title)
                .and_then(serde_json::Value::as_str),
            Some("Unit under attack"),
            "Muster renders Item(Title), so the actual row must populate that field"
        );
        assert_eq!(
            rows.items[0]
                .get(ItemField::Action)
                .and_then(|action| serde_json::from_value::<WidgetAction>(action.clone()).ok()),
            Some(WidgetAction::CenterCamera { x: -4.0, y: 80.0 }),
            "the visible alert button carries its finite position through the typed camera action"
        );
    }
    use turts_net_protocol::{
        BalanceState, HealthRecord, JobId, JobStage, Pos, QueueState, UnitId,
    };

    #[allow(clippy::too_many_arguments)]
    fn context<'a>(
        replica: &'a Replica,
        content: &'a ClientContent,
        selection: &'a [u32],
        groups: &'a BTreeMap<u8, Vec<u32>>,
        alerts: &'a [serde_json::Value],
        bindings: &'a BTreeMap<String, Vec<Binding>>,
        input_map: &'a InputMapDef,
        guide: &'a [GuidePage],
    ) -> SourceContext<'a> {
        SourceContext {
            replica,
            content,
            selection,
            owner_slot: 1,
            interface: None,
            interface_package: Some("ui"),
            control_groups: groups,
            alerts,
            bindings,
            input_map,
            guide,
            guide_open: false,
            last_notice: None,
        }
    }

    fn unit(content_id: &str, owner_slot: u8) -> UnitReplica {
        UnitReplica {
            content_id: content_id.to_owned(),
            owner_slot,
            facing: None,
            samples: VecDeque::from([(1, Pos { x: 0.0, y: 0.0 })]),
        }
    }

    fn resolved_unit(id: &str, movement: Option<MovementDef>) -> turts_mod_api::ResolvedUnit {
        turts_mod_api::ResolvedUnit {
            id: ContentId::parse(id).unwrap(),
            name: "Scout".into(),
            footprint: Footprint::Circle { radius: 1.0 },
            movement,
            inventory: None,
            gatherer: None,
            drop_off: None,
            producer: None,
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: None,
            weapon: None,
            removal: None,
        }
    }

    fn refunds() -> CancelRefunds {
        CancelRefunds {
            queued: RefundRatio {
                numerator: 1,
                denominator: 1,
            },
            running: RefundRatio {
                numerator: 1,
                denominator: 1,
            },
            output_blocked: RefundRatio {
                numerator: 1,
                denominator: 1,
            },
        }
    }

    #[test]
    fn selection_units_preserve_each_unit_and_count_by_type() {
        let mut replica = Replica::new(20);
        replica.units = HashMap::from([
            (7, unit("muster:scout", 1)),
            (8, unit("muster:scout", 1)),
            (9, unit("muster:worker", 1)),
        ]);
        replica.healths = vec![
            HealthRecord {
                unit: UnitId(7),
                current: 17,
            },
            HealthRecord {
                unit: UnitId(8),
                current: 13,
            },
        ];
        let mut content = ClientContent::default();
        content.catalog.units.clear();
        let interface = InterfaceDef {
            id: "ui".into(),
            root: vec![],
            tooltip_style: turts_mod_api::TooltipStyle {
                background: "#000".into(),
                colour: "#fff".into(),
                font: None,
                size: 12,
            },
            cursors: BTreeMap::new(),
            camera: turts_mod_api::CameraDef {
                pitch: 1.0,
                yaw: 0.0,
                field_of_view: 1.0,
                minimum: 1.0,
                maximum: 2.0,
                start: CameraStart::MapCentre,
                height: 1.0,
                margin: 0.0,
                pan_speed: 1.0,
                edge_band: false,
            },
            light: None,
            slot_colours: vec![],
            world: turts_mod_api::WorldStyle {
                ground: "#000".into(),
                grid: "#000".into(),
                selection: "#000".into(),
                health: "#000".into(),
                range: "#000".into(),
                show_range: false,
                tooltip: WorldTooltipStyle {
                    background: "#000".into(),
                    colour: "#fff".into(),
                    font: None,
                    size: 12,
                },
                health_bar: WorldHealthBarStyle {
                    width: 1,
                    height: 1,
                    background: "#000".into(),
                    health: "#fff".into(),
                },
            },
            alerts: vec![],
            reasons: BTreeMap::new(),
            units: BTreeMap::from([
                (
                    "muster:scout".into(),
                    UnitPresentation {
                        icon: Some("scout.png".into()),
                        portrait: None,
                        responses: BTreeMap::new(),
                    },
                ),
                (
                    "muster:worker".into(),
                    UnitPresentation {
                        icon: Some("worker.png".into()),
                        portrait: None,
                        responses: BTreeMap::new(),
                    },
                ),
            ]),
            selection_priority: vec![],
            guide: vec![],
        };
        let selection = [7, 8, 9];
        let groups = BTreeMap::new();
        let bindings = BTreeMap::new();
        let alerts = [];
        let input = InputMapDef {
            actions: vec![],
            engine_overrides: vec![],
        };
        let guide = [];
        let ctx = SourceContext {
            interface: Some(&interface),
            ..context(
                &replica, &content, &selection, &groups, &alerts, &bindings, &input, &guide,
            )
        };
        let rows = read_source(&Source::SelectionUnits, ctx).items;
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].get(ItemField::Count), Some(&serde_json::json!(2)));
        assert_eq!(rows[1].get(ItemField::Health), Some(&serde_json::json!(13)));
        assert_eq!(rows[2].get(ItemField::Count), Some(&serde_json::json!(1)));
        assert_eq!(
            rows[0].get(ItemField::Icon),
            Some(&serde_json::json!("ui:scout.png"))
        );
        assert_eq!(
            rows[1].get(ItemField::Action),
            Some(&serde_json::json!({"kind":"select","unit":8}))
        );
    }

    #[test]
    fn balance_source_has_no_rows_when_replica_has_no_balances() {
        let replica = Replica::new(20);
        let content = ClientContent::default();
        let groups = BTreeMap::new();
        let bindings = BTreeMap::new();
        let alerts = [];
        let input = InputMapDef {
            actions: vec![],
            engine_overrides: vec![],
        };
        let guide = [];
        let ctx = context(
            &replica,
            &content,
            &[],
            &groups,
            &alerts,
            &bindings,
            &input,
            &guide,
        );
        let balance_rows = read_source(&Source::Balances, ctx);
        assert!(balance_rows.scalar.is_none());
        assert!(balance_rows.items.is_empty());
    }

    #[test]
    fn duplicate_costs_use_the_correct_source_and_subtract_reserved_stock() {
        let mut replica = Replica::new(20);
        replica.economy.balances.push(BalanceState {
            resource: "muster:ore".into(),
            on_hand: 40,
            reserved: 6,
        });
        let content = ClientContent::default();
        let selection = [];
        let groups = BTreeMap::new();
        let bindings = BTreeMap::new();
        let alerts = [];
        let input = InputMapDef {
            actions: vec![],
            engine_overrides: vec![],
        };
        let guide = [];
        let ctx = context(
            &replica, &content, &selection, &groups, &alerts, &bindings, &input, &guide,
        );
        let costs = [
            turts_mod_api::ResolvedCost {
                resource: ContentId::parse("muster:ore").unwrap(),
                amount: 20,
                source: CostSource::SlotBalance,
            },
            turts_mod_api::ResolvedCost {
                resource: ContentId::parse("muster:ore").unwrap(),
                amount: 15,
                source: CostSource::SlotBalance,
            },
        ];
        assert!(!costs_affordable(ctx, 7, &costs));
        assert_eq!(
            affordability_reason(ctx, 7, &costs),
            "Needs 35 muster:ore (34/35)"
        );
        let wrong_source = [turts_mod_api::ResolvedCost {
            resource: ContentId::parse("muster:ore").unwrap(),
            amount: 1,
            source: CostSource::ProducerInventory,
        }];
        assert!(!costs_affordable(ctx, 7, &wrong_source));
    }

    #[test]
    fn format_arguments_are_substituted_once_with_item_context() {
        let replica = Replica::new(20);
        let content = ClientContent::default();
        let selection = [];
        let groups = BTreeMap::new();
        let bindings = BTreeMap::new();
        let alerts = [];
        let input = InputMapDef {
            actions: vec![],
            engine_overrides: vec![],
        };
        let guide = [];
        let ctx = context(
            &replica, &content, &selection, &groups, &alerts, &bindings, &input, &guide,
        );
        let row = ItemData {
            fields: vec![(ItemField::Name, serde_json::json!("Scout"))],
        };
        let formatted = read_value_for_item(
            &Value::Format(
                "{0} {1}".into(),
                vec![Value::Item(ItemField::Name), Value::Literal("{0}".into())],
            ),
            ctx,
            Some(&row),
        );
        assert_eq!(formatted, serde_json::json!("Scout {0}"));
    }

    #[test]
    fn guide_value_is_authored_readable_text_while_guide_source_stays_structured() {
        let replica = Replica::new(20);
        let content = ClientContent::default();
        let groups = BTreeMap::new();
        let bindings = BTreeMap::new();
        let alerts = [];
        let input = InputMapDef {
            actions: vec![],
            engine_overrides: vec![],
        };
        let guide = vec![
            GuidePage {
                id: "controls".into(),
                title: "Controls".into(),
                body: "Press F1 to open this guide.".into(),
                widgets: vec![],
            },
            GuidePage {
                id: "orders".into(),
                title: "Orders".into(),
                body: "Select a unit first.".into(),
                widgets: vec![],
            },
        ];
        let ctx = context(
            &replica,
            &content,
            &[],
            &groups,
            &alerts,
            &bindings,
            &input,
            &guide,
        );
        assert_eq!(
            read_value(&Value::Guide, ctx),
            serde_json::json!(
                "Controls\nPress F1 to open this guide.\nOrders\nSelect a unit first."
            )
        );
        let rows = read_source(&Source::Guide, ctx).items;
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].get(ItemField::Name),
            Some(&serde_json::json!("Controls"))
        );
        assert_eq!(
            rows[0].get(ItemField::Body),
            Some(&serde_json::json!("Press F1 to open this guide."))
        );
    }

    #[test]
    fn foreign_selection_exposes_public_identity_without_owner_data_or_controls() {
        let mut replica = Replica::new(20);
        replica.units.insert(3, unit("muster:scout", 2));
        replica
            .economy
            .inventories
            .push(turts_net_protocol::InventoryState {
                unit: UnitId(3),
                port: turts_net_protocol::InventoryPort::Main,
                items: vec![BalanceState {
                    resource: "muster:ore".into(),
                    on_hand: 99,
                    reserved: 0,
                }],
            });
        let mut content = ClientContent::default();
        let mut scout = resolved_unit("muster:scout", Some(MovementDef { speed: 1.0 }));
        scout.name = "Public Scout".into();
        content.catalog.units.insert(scout.id.clone(), scout);
        let selection = [3];
        let groups = BTreeMap::new();
        let bindings = BTreeMap::new();
        let alerts = [];
        let input = InputMapDef {
            actions: vec![],
            engine_overrides: vec![],
        };
        let guide = [];
        let ctx = context(
            &replica, &content, &selection, &groups, &alerts, &bindings, &input, &guide,
        );
        assert_eq!(
            read_primary(PrimaryField::Name, ctx),
            serde_json::json!("Public Scout")
        );
        assert_eq!(read_primary(PrimaryField::Owner, ctx), serde_json::json!(2));
        assert_eq!(
            read_primary(PrimaryField::Inventory, ctx),
            serde_json::Value::Null
        );
        assert_eq!(
            read_primary(PrimaryField::Ammo, ctx),
            serde_json::Value::Null
        );
        assert!(
            read_source(&Source::CommandCard, ctx).items.is_empty(),
            "foreign inspection exposes no authored owner controls"
        );
    }

    #[test]
    fn selected_inventory_value_reports_both_ports_and_each_authored_capacity() {
        let mut replica = Replica::new(20);
        replica.units.insert(3, unit("muster:press", 1));
        replica.economy.inventories = vec![
            turts_net_protocol::InventoryState {
                unit: UnitId(3),
                port: turts_net_protocol::InventoryPort::Output,
                items: vec![BalanceState {
                    resource: "muster:shell".into(),
                    on_hand: 3,
                    reserved: 1,
                }],
            },
            turts_net_protocol::InventoryState {
                unit: UnitId(3),
                port: turts_net_protocol::InventoryPort::Main,
                items: vec![BalanceState {
                    resource: "muster:ore".into(),
                    on_hand: 5,
                    reserved: 2,
                }],
            },
        ];
        let mut content = ClientContent::default();
        let mut press = resolved_unit("muster:press", None);
        press.inventory = Some(turts_mod_api::ResolvedInventory {
            capacity: 12,
            accepts: vec![],
        });
        press.output_inventory = Some(turts_mod_api::ResolvedInventory {
            capacity: 7,
            accepts: vec![],
        });
        content.catalog.units.insert(press.id.clone(), press);
        let selection = [3];
        let groups = BTreeMap::new();
        let alerts = [];
        let bindings = BTreeMap::new();
        let input = InputMapDef {
            actions: vec![],
            engine_overrides: vec![],
        };
        let guide = [];
        let ctx = context(
            &replica, &content, &selection, &groups, &alerts, &bindings, &input, &guide,
        );
        let rows = read_primary(PrimaryField::Inventory, ctx);
        assert_eq!(
            rows,
            serde_json::json!([
                {"port":"Main","name":"muster:ore","count":3,"on_hand":5,"reserved":2,"capacity":12},
                {"port":"Output","name":"muster:shell","count":2,"on_hand":3,"reserved":1,"capacity":7}
            ]),
            "inventory values are keyed by port, never by replica row order"
        );
        assert_eq!(
            display_text_value(&rows),
            "Main: muster:ore 3 available (5 on hand, 2 reserved) (capacity 12)\nOutput: muster:shell 2 available (3 on hand, 1 reserved) (capacity 7)"
        );
        assert_eq!(
            read_value(
                &Value::Format(
                    "{0}".into(),
                    vec![Value::SelectionPrimary(PrimaryField::Inventory)]
                ),
                ctx,
            ),
            serde_json::json!(
                "Main: muster:ore 3 available (5 on hand, 2 reserved) (capacity 12)\nOutput: muster:shell 2 available (3 on hand, 1 reserved) (capacity 7)"
            )
        );
        assert_eq!(display_text_value(&serde_json::json!([])), "[]");
        assert_eq!(
            display_text_value(&serde_json::json!([{"arbitrary": 1}])),
            "[{\"arbitrary\":1}]"
        );
    }

    #[test]
    fn selected_turret_ammo_distinguishes_known_empty_from_unknown_or_foreign() {
        let mut replica = Replica::new(20);
        replica.units.insert(7, unit("muster:turret", 1));
        replica
            .economy
            .inventories
            .push(turts_net_protocol::InventoryState {
                unit: UnitId(7),
                port: turts_net_protocol::InventoryPort::Main,
                items: vec![],
            });
        let mut content = ClientContent::default();
        let mut turret = resolved_unit("muster:turret", None);
        turret.weapon = Some(turts_mod_api::ResolvedWeapon {
            range: 8.0,
            cooldown_ms: 1000,
            damage: 10,
            ammo: Some(turts_mod_api::ResolvedAmmo {
                item: turts_mod_api::ContentId::parse("muster:shell").expect("content id"),
                per_shot: 1,
            }),
            targets: turts_mod_api::TargetFilter {
                slots: turts_mod_api::SlotRelation::Other,
            },
        });
        content.catalog.units.insert(turret.id.clone(), turret);
        let selection = [7];
        let groups = BTreeMap::new();
        let bindings = BTreeMap::new();
        let alerts = [];
        let input = InputMapDef {
            actions: vec![],
            engine_overrides: vec![],
        };
        let guide = [];
        let ctx = context(
            &replica, &content, &selection, &groups, &alerts, &bindings, &input, &guide,
        );
        assert_eq!(read_primary(PrimaryField::Ammo, ctx), serde_json::json!(0));

        replica.economy.inventories.clear();
        let unknown_inventory = context(
            &replica, &content, &selection, &groups, &alerts, &bindings, &input, &guide,
        );
        assert_eq!(
            read_primary(PrimaryField::Ammo, unknown_inventory),
            serde_json::Value::Null,
            "missing main inventory must remain unknown, not fabricated as zero"
        );

        replica
            .economy
            .inventories
            .push(turts_net_protocol::InventoryState {
                unit: UnitId(7),
                port: turts_net_protocol::InventoryPort::Main,
                items: vec![],
            });
        replica.units.get_mut(&7).unwrap().owner_slot = 2;
        let foreign = context(
            &replica, &content, &selection, &groups, &alerts, &bindings, &input, &guide,
        );
        assert_eq!(
            read_primary(PrimaryField::Ammo, foreign),
            serde_json::Value::Null,
            "foreign ammo remains private"
        );
    }

    #[test]
    fn producer_inventory_cost_uses_main_only_independent_of_row_order() {
        let mut replica = Replica::new(20);
        replica.economy.inventories = vec![
            turts_net_protocol::InventoryState {
                unit: UnitId(6),
                port: turts_net_protocol::InventoryPort::Output,
                items: vec![BalanceState {
                    resource: "muster:ore".into(),
                    on_hand: 10,
                    reserved: 0,
                }],
            },
            turts_net_protocol::InventoryState {
                unit: UnitId(6),
                port: turts_net_protocol::InventoryPort::Main,
                items: vec![BalanceState {
                    resource: "muster:ore".into(),
                    on_hand: 0,
                    reserved: 0,
                }],
            },
        ];
        let content = ClientContent::default();
        let groups = BTreeMap::new();
        let alerts = [];
        let bindings = BTreeMap::new();
        let input = InputMapDef {
            actions: vec![],
            engine_overrides: vec![],
        };
        let guide = [];
        let costs = [turts_mod_api::ResolvedCost {
            resource: ContentId::parse("muster:ore").unwrap(),
            amount: 10,
            source: CostSource::ProducerInventory,
        }];
        let ctx = context(
            &replica,
            &content,
            &[],
            &groups,
            &alerts,
            &bindings,
            &input,
            &guide,
        );
        assert!(!costs_affordable(ctx, 6, &costs));
        replica.economy.inventories.reverse();
        let ctx = context(
            &replica,
            &content,
            &[],
            &groups,
            &alerts,
            &bindings,
            &input,
            &guide,
        );
        assert!(!costs_affordable(ctx, 6, &costs));
        for inventory in &mut replica.economy.inventories {
            inventory.items[0].on_hand =
                if inventory.port == turts_net_protocol::InventoryPort::Main {
                    10
                } else {
                    0
                };
        }
        for _ in 0..2 {
            let ctx = context(
                &replica,
                &content,
                &[],
                &groups,
                &alerts,
                &bindings,
                &input,
                &guide,
            );
            assert!(costs_affordable(ctx, 6, &costs));
            replica.economy.inventories.reverse();
        }
    }

    #[test]
    fn command_card_filters_foreign_only_and_checks_each_units_capability() {
        let mut replica = Replica::new(20);
        replica.units = HashMap::from([(3, unit("muster:scout", 1)), (4, unit("muster:scout", 2))]);
        let mut content = ClientContent::default();
        let scout = ContentId::parse("muster:scout").unwrap();
        content.catalog.units.insert(
            scout.clone(),
            resolved_unit("muster:scout", Some(MovementDef { speed: 1.0 })),
        );
        let machine_id = ContentId::parse("muster:machine").unwrap();
        let mut machine = resolved_unit("muster:machine", None);
        machine.machine = Some(turts_mod_api::ResolvedMachine {
            recipe: ContentId::parse("muster:recipe").unwrap(),
        });
        content.catalog.units.insert(machine_id, machine);
        let move_id = ContentId::parse("muster:move").unwrap();
        let attack_id = ContentId::parse("muster:attack").unwrap();
        for (id, behaviour) in [(&move_id, Behaviour::Move), (&attack_id, Behaviour::Attack)] {
            content.catalog.orders.insert(
                id.clone(),
                OrderDef {
                    id: id.local.clone(),
                    name: id.local.clone(),
                    behaviour,
                    target: TargetKind::None,
                    fire: None,
                    engage: None,
                    icon: format!("{}_icon", id.local),
                    hotkey: "command".into(),
                    cursor: None,
                    card: Some(CardCell {
                        row: 0,
                        column: if id == &move_id { 0 } else { 1 },
                    }),
                    units: vec!["muster:scout".into()],
                    description: String::new(),
                },
            );
        }
        let attack_point_id = ContentId::parse("muster:attack_point").unwrap();
        content.catalog.orders.insert(
            attack_point_id.clone(),
            OrderDef {
                id: attack_point_id.local.clone(),
                name: "Attack point".into(),
                behaviour: Behaviour::Attack,
                target: TargetKind::Point,
                fire: None,
                engage: None,
                icon: "attack_point".into(),
                hotkey: "attack_point".into(),
                cursor: None,
                card: Some(CardCell { row: 1, column: 0 }),
                units: vec!["muster:scout".into()],
                description: String::new(),
            },
        );
        let cancel_machine_id = ContentId::parse("muster:cancel_machine").unwrap();
        content.catalog.orders.insert(
            cancel_machine_id.clone(),
            OrderDef {
                id: cancel_machine_id.local.clone(),
                name: "Cancel machine".into(),
                behaviour: Behaviour::CancelProduction,
                target: TargetKind::Job,
                fire: None,
                engage: None,
                icon: "cancel_machine".into(),
                hotkey: "cancel_machine".into(),
                cursor: None,
                card: Some(CardCell { row: 1, column: 1 }),
                units: vec!["muster:machine".into()],
                description: String::new(),
            },
        );
        let own_selection = [3];
        let foreign_selection = [4];
        let groups = BTreeMap::new();
        let bindings = BTreeMap::new();
        let alerts = [];
        let input = InputMapDef {
            actions: vec![],
            engine_overrides: vec![],
        };
        let guide = [];
        let own = read_source(
            &Source::CommandCard,
            context(
                &replica,
                &content,
                &own_selection,
                &groups,
                &alerts,
                &bindings,
                &input,
                &guide,
            ),
        )
        .items;
        assert_eq!(own.len(), 3);
        let move_row = own
            .iter()
            .find(|row| row.get(ItemField::Name) == Some(&serde_json::json!("move")))
            .unwrap();
        let attack_row = own
            .iter()
            .find(|row| row.get(ItemField::Name) == Some(&serde_json::json!("attack")))
            .unwrap();
        assert_eq!(
            move_row.get(ItemField::Enabled),
            Some(&serde_json::json!(true))
        );
        assert_eq!(
            move_row.get(ItemField::Action),
            Some(&serde_json::json!({"kind":"order","id":"muster:move"}))
        );
        assert_eq!(
            attack_row.get(ItemField::Enabled),
            Some(&serde_json::json!(false))
        );
        assert_eq!(
            attack_row.get(ItemField::CardCell),
            Some(&serde_json::json!({"row":0,"column":1}))
        );
        let point_attack = own
            .iter()
            .find(|row| row.get(ItemField::Name) == Some(&serde_json::json!("Attack point")))
            .unwrap();
        assert_eq!(
            point_attack.get(ItemField::Enabled),
            Some(&serde_json::json!(true)),
            "a point-target attack follows its movement capability"
        );
        let point_or_unit = OrderDef {
            target: TargetKind::PointOrUnit,
            ..content.catalog.orders[&attack_point_id].clone()
        };
        let unit_attack = OrderDef {
            target: TargetKind::Unit,
            ..content.catalog.orders[&attack_point_id].clone()
        };
        let mobile = &content.catalog.units[&scout];
        assert!(capability_matches(&point_or_unit, mobile));
        assert!(!capability_matches(&unit_attack, mobile));
        let mut static_weapon = mobile.clone();
        static_weapon.movement = None;
        static_weapon.weapon = Some(turts_mod_api::ResolvedWeapon {
            range: 5.0,
            cooldown_ms: 1000,
            damage: 1,
            ammo: None,
            targets: turts_mod_api::TargetFilter {
                slots: turts_mod_api::SlotRelation::Other,
            },
        });
        assert!(!capability_matches(
            &content.catalog.orders[&attack_point_id],
            &static_weapon
        ));
        assert!(!capability_matches(&unit_attack, mobile));
        assert!(capability_matches(&unit_attack, &static_weapon));
        assert!(capability_matches(&point_or_unit, &static_weapon));
        let machine_selection = [5];
        replica.units.insert(5, unit("muster:machine", 1));
        let machine_rows = read_source(
            &Source::CommandCard,
            context(
                &replica,
                &content,
                &machine_selection,
                &groups,
                &alerts,
                &bindings,
                &input,
                &guide,
            ),
        )
        .items;
        let cancel_machine = machine_rows
            .iter()
            .find(|row| row.get(ItemField::Name) == Some(&serde_json::json!("Cancel machine")))
            .unwrap();
        assert_eq!(
            cancel_machine.get(ItemField::Enabled),
            Some(&serde_json::json!(true)),
            "a machine can cancel its own queued recipe job"
        );
        assert_eq!(
            read_source(
                &Source::CommandCard,
                context(
                    &replica,
                    &content,
                    &foreign_selection,
                    &groups,
                    &alerts,
                    &bindings,
                    &input,
                    &guide
                )
            )
            .items
            .len(),
            0
        );
    }

    #[test]
    fn bindings_show_effective_defaults_and_readable_runtime_keys() {
        let replica = Replica::new(20);
        let content = ClientContent::default();
        let selection = [];
        let groups = BTreeMap::new();
        let alerts = [];
        let input = InputMapDef {
            actions: vec![],
            engine_overrides: vec![],
        };
        let runtime = BTreeMap::from([(
            "group_assign_1".into(),
            vec![Binding::Key {
                code: "KeyM".into(),
                mods: vec![turts_mod_api::Mod::Ctrl],
            }],
        )]);
        let guide = [];
        let rows = read_source(
            &Source::Bindings,
            context(
                &replica, &content, &selection, &groups, &alerts, &runtime, &input, &guide,
            ),
        )
        .items;
        let assignment = rows
            .iter()
            .find(|row| row.get(ItemField::Name) == Some(&serde_json::json!("Assign group 1")))
            .unwrap();
        assert_eq!(
            assignment.get(ItemField::Title),
            Some(&serde_json::json!("Ctrl+M"))
        );
        let select = rows
            .iter()
            .find(|row| row.get(ItemField::Name) == Some(&serde_json::json!("Select")))
            .unwrap();
        assert_eq!(
            select.get(ItemField::Title),
            Some(&serde_json::json!("Left click"))
        );
    }

    #[test]
    fn build_menu_reports_reserved_duplicate_costs_queue_capacity_and_missing_research() {
        let mut replica = Replica::new(20);
        replica.units.insert(6, unit("muster:producer", 1));
        replica.economy.balances.push(BalanceState {
            resource: "muster:ore".into(),
            on_hand: 40,
            reserved: 6,
        });
        let content_id = ContentId::parse("muster:scout").unwrap();
        let recipe_id = ContentId::parse("muster:scout_recipe").unwrap();
        let producer_id = ContentId::parse("muster:producer").unwrap();
        let tech_id = ContentId::parse("muster:advanced").unwrap();
        let prior_id = ContentId::parse("muster:prior").unwrap();
        let mut content = ClientContent::default();
        let mut producer = resolved_unit("muster:producer", None);
        producer.producer = Some(turts_mod_api::ResolvedProducer {
            recipes: vec![recipe_id.clone()],
            queue_capacity: 1,
            spawn_offsets: vec![],
        });
        producer.research = Some(turts_mod_api::ResolvedResearchStation {
            technologies: vec![tech_id.clone()],
            queue_capacity: 1,
        });
        content.catalog.units.insert(producer_id.clone(), producer);
        content.catalog.production.insert(
            recipe_id.clone(),
            ResolvedProduction {
                id: recipe_id.clone(),
                name: "Scout".into(),
                output_unit: content_id,
                duration_ms: 4_000,
                costs: vec![
                    turts_mod_api::ResolvedCost {
                        resource: ContentId::parse("muster:ore").unwrap(),
                        amount: 20,
                        source: CostSource::SlotBalance,
                    },
                    turts_mod_api::ResolvedCost {
                        resource: ContentId::parse("muster:ore").unwrap(),
                        amount: 15,
                        source: CostSource::SlotBalance,
                    },
                ],
                payment: PaymentPolicy::ChargeOnQueue,
                cancel_refunds: refunds(),
            },
        );
        content.catalog.technologies.insert(
            tech_id.clone(),
            ResolvedTechnology {
                id: tech_id.clone(),
                name: "Advanced".into(),
                prerequisites: vec![prior_id.clone()],
                duration_ms: 8_000,
                costs: vec![],
                payment: PaymentPolicy::ChargeOnQueue,
                cancel_refunds: refunds(),
                effects: vec![],
            },
        );
        content.catalog.technologies.insert(
            prior_id.clone(),
            ResolvedTechnology {
                id: prior_id,
                name: "Prior Research".into(),
                prerequisites: vec![],
                duration_ms: 1_000,
                costs: vec![],
                payment: PaymentPolicy::ChargeOnQueue,
                cancel_refunds: refunds(),
                effects: vec![],
            },
        );
        let selection = [6];
        let groups = BTreeMap::new();
        let bindings = BTreeMap::new();
        let alerts = [];
        let input = InputMapDef {
            actions: vec![],
            engine_overrides: vec![],
        };
        let guide = [];
        let rows = read_source(
            &Source::BuildMenu,
            context(
                &replica, &content, &selection, &groups, &alerts, &bindings, &input, &guide,
            ),
        )
        .items;
        let recipe = rows
            .iter()
            .find(|row| row.get(ItemField::Name) == Some(&serde_json::json!("Scout")))
            .unwrap();
        assert_eq!(
            recipe.get(ItemField::Enabled),
            Some(&serde_json::json!(false))
        );
        assert_eq!(
            recipe.get(ItemField::Reason),
            Some(&serde_json::json!("Needs 35 muster:ore (34/35)"))
        );
        assert_eq!(
            recipe.get(ItemField::Cost),
            Some(&serde_json::json!("35 muster:ore"))
        );
        assert_eq!(
            recipe.get(ItemField::Action),
            Some(
                &serde_json::json!({"kind":"produce","producer":6,"content_id":"muster:scout_recipe"})
            )
        );
        let research = rows
            .iter()
            .find(|row| row.get(ItemField::Name) == Some(&serde_json::json!("Advanced")))
            .unwrap();
        assert_eq!(
            research.get(ItemField::Reason),
            Some(&serde_json::json!("Prior Research"))
        );

        replica.economy.queues.push(QueueState {
            producer: UnitId(6),
            enabled: true,
            jobs: vec![turts_net_protocol::JobState {
                id: JobId(55),
                recipe: recipe_id.to_string(),
                stage: JobStage::Queued,
                elapsed_ticks: 120,
                duration_ticks: 80,
                charged: vec![],
                reserved: vec![],
            }],
        });
        let rows = read_source(
            &Source::BuildMenu,
            context(
                &replica, &content, &selection, &groups, &alerts, &bindings, &input, &guide,
            ),
        )
        .items;
        let recipe = rows
            .iter()
            .find(|row| row.get(ItemField::Name) == Some(&serde_json::json!("Scout")))
            .unwrap();
        assert_eq!(
            recipe.get(ItemField::Reason),
            Some(&serde_json::json!("Production queue is full"))
        );
        assert_eq!(
            recipe.get(ItemField::Enabled),
            Some(&serde_json::json!(false))
        );
        replica.economy.queues[0].enabled = false;
        replica.economy.queues[0].jobs.clear();
        let disabled_rows = read_source(
            &Source::BuildMenu,
            context(
                &replica, &content, &selection, &groups, &alerts, &bindings, &input, &guide,
            ),
        )
        .items;
        let disabled_recipe = disabled_rows
            .iter()
            .find(|row| row.get(ItemField::Name) == Some(&serde_json::json!("Scout")))
            .unwrap();
        assert_eq!(
            disabled_recipe.get(ItemField::Reason),
            Some(&serde_json::json!("Production is disabled"))
        );
        assert_eq!(
            disabled_recipe.get(ItemField::Enabled),
            Some(&serde_json::json!(false))
        );
        replica.economy.queues[0].enabled = true;
        replica.economy.queues[0]
            .jobs
            .push(turts_net_protocol::JobState {
                id: JobId(55),
                recipe: recipe_id.to_string(),
                stage: JobStage::Queued,
                elapsed_ticks: 120,
                duration_ticks: 80,
                charged: vec![],
                reserved: vec![],
            });
        let queued = read_source(
            &Source::ProductionQueue,
            context(
                &replica, &content, &selection, &groups, &alerts, &bindings, &input, &guide,
            ),
        )
        .items;
        assert_eq!(
            queued[0].get(ItemField::Progress),
            Some(&serde_json::json!(1.0))
        );
        assert_eq!(
            queued[0].get(ItemField::RemainingSeconds),
            Some(&serde_json::json!(0.0))
        );
        assert_eq!(
            queued[0].get(ItemField::CancelAction),
            Some(&serde_json::json!({"kind":"cancel_production","producer":6,"job":55}))
        );
    }
}
