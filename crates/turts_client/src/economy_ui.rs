//! Shared native/wasm M2 economy panel backed by the validated replica and downloaded catalog.

const PANEL_WIDE_WIDTH: f32 = 390.0;

use std::collections::BTreeMap;

use crate::commands::ClientCommand;
use bevy::camera::visibility::InheritedVisibility;
use bevy::{
    prelude::*,
    ui::{
        CalculatedClip, ComputedNode, ComputedUiTargetCamera, InteractionDisabled, Pressed,
        UiGlobalTransform,
    },
    ui_widgets::{Activate, Button, ScrollArea, observe},
};
use turts_mod_api::{ContentId, CostSource, Footprint, ResourceKind};
use turts_net_protocol::{
    EconomyState, GatherStage, InventoryPort, JobId, JobStage, NodeId, Pos, UnitId,
};

use crate::{
    commands::{PendingCommands, submit},
    content::ClientContent,
    ingest::{CommandResult, CommandResults, ReplicaState},
    join::{AcquiredPackages, Phase},
    replica::Replica,
};

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Selection(pub Option<u32>);

#[derive(Resource, Debug, Default)]
struct UiActionNotice(Option<String>);

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiAction {
    Select { unit: u32 },
    Produce { producer: u32, recipe: String },
    Cancel { producer: u32, job: JobId },
    SetEnabled { unit: u32, enabled: bool },
    Remove { unit: u32 },
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct UiActionTarget(pub UiAction);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiControlEnabled(pub bool);

#[derive(Component)]
pub struct UiPanelRoot;

#[derive(Component)]
pub struct UiSnapshotText;

#[derive(Component, Debug, Clone, Copy)]
struct UiActionOwner(Entity);

#[derive(Resource, Default)]
struct PanelEntities {
    root: Option<Entity>,
    docked_bottom: Option<bool>,
    lines: Vec<Entity>,
    controls: std::collections::HashMap<UiAction, (Entity, Entity)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiControl {
    pub label: String,
    pub action: UiAction,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PanelModel {
    pub lines: Vec<String>,
    pub controls: Vec<UiControl>,
}

/// Build visible copy solely from downloaded definitions and coherent replicated state.
pub fn panel_model(
    replica: &Replica,
    content: &ClientContent,
    selection: &Selection,
    results: &CommandResults,
) -> PanelModel {
    let mut model = PanelModel::default();
    let industrial = content
        .catalog
        .units
        .values()
        .any(|def| def.research.is_some() || def.machine.is_some() || def.extractor.is_some());
    // Names and capabilities come from the downloaded package, not a Forge ID list.
    for (&id, unit) in &replica.units {
        let Some(def) = ContentId::parse(&unit.content_id)
            .ok()
            .and_then(|id| content.catalog.units.get(&id))
        else {
            continue;
        };
        if industrial
            && (def.research.is_some()
                || def.movement.is_some()
                || def.machine.is_some()
                || def.weapon.is_some()
                || def.extractor.is_some()
                || def.producer.is_some())
        {
            model.controls.push(UiControl {
                label: format!("Select {}", def.name),
                action: UiAction::Select { unit: id },
                enabled: true,
            });
        }
    }
    let Some(selected) = selection.0 else {
        if industrial {
            model
                .lines
                .push("Your units: select a named button below.".into());
            model.lines.push(
                "Factory: Select a lab to start research. Machines run automatically.".into(),
            );
            model
                .lines
                .push("Mobile units: select one, then right-click the ground to move.".into());
        } else {
            model.lines.push("Select a unit or producer".into());
        }
        append_last_result(&mut model, results);
        append_nodes(&mut model, replica, content);
        return model;
    };
    let Some(unit) = replica.units.get(&selected) else {
        model.lines.push("Selection unavailable".into());
        return model;
    };
    let parsed = ContentId::parse(&unit.content_id).ok();
    let def = parsed.as_ref().and_then(|id| content.catalog.units.get(id));
    let name = def.map_or_else(|| unit.content_id.clone(), |d| d.name.clone());
    model.lines.push(format!("Selected: {name} · {selected}"));
    if industrial && let Some(def) = def {
        model.lines.push(if def.movement.is_some() {
            "Mobile: right-click the ground to move.".into()
        } else {
            "This unit is stationary; it cannot move.".into()
        });
        if def.extractor.is_some()
            || def.machine.is_some()
            || def.inserter.is_some()
            || def.weapon.is_some()
        {
            model
                .lines
                .push("Runs automatically. Disable pauses it; Remove destroys it.".into());
        }
    }
    append_balances(&mut model, &replica.economy, content);
    append_inventory(&mut model, &replica.economy, content, selected, def);
    append_order(&mut model, &replica.economy, selected);
    append_m3_records(&mut model, replica, selected, def);
    append_nodes(&mut model, replica, content);

    let queue = replica
        .economy
        .queues
        .iter()
        .find(|q| q.producer.0 == selected);
    if let Some(producer) = def.and_then(|d| d.producer.as_ref()) {
        let queue_len = queue.map_or(0, |q| q.jobs.len());
        for recipe in &producer.recipes {
            let recipe_name = content
                .catalog
                .production
                .get(recipe)
                .map_or_else(|| recipe.to_string(), |r| r.name.clone());
            let definition = content.catalog.production.get(recipe);
            if let Some(definition) = definition {
                append_costs(
                    &mut model.lines,
                    "cost",
                    &definition
                        .costs
                        .iter()
                        .map(|cost| turts_mod_api::Cost {
                            resource: cost.resource.to_string(),
                            amount: cost.amount,
                            source: cost.source,
                        })
                        .collect::<Vec<_>>(),
                    content,
                );
            }
            let affordable = definition
                .is_some_and(|definition| recipe_affordable(replica, selected, &definition.costs));
            model.controls.push(UiControl {
                label: format!("Produce {recipe_name}"),
                action: UiAction::Produce {
                    producer: selected,
                    recipe: recipe.to_string(),
                },
                enabled: queue_len < usize::from(producer.queue_capacity) && affordable,
            });
        }
        if let Some(queue) = queue {
            for job in &queue.jobs {
                let recipe_name = ContentId::parse(&job.recipe)
                    .ok()
                    .and_then(|id| content.catalog.production.get(&id))
                    .map_or_else(|| job.recipe.clone(), |r| r.name.clone());
                let stage = match job.stage {
                    JobStage::Queued => "queued",
                    JobStage::Running => "running",
                    JobStage::OutputBlocked => "output_blocked",
                };
                model.lines.push(format!(
                    "Job {} · {recipe_name} · {stage} · {}/{} ticks",
                    job.id.0, job.elapsed_ticks, job.duration_ticks
                ));
                append_costs(&mut model.lines, "charged", &job.charged, content);
                append_costs(&mut model.lines, "reserved", &job.reserved, content);
                model.controls.push(UiControl {
                    label: format!("Cancel job {}", job.id.0),
                    action: UiAction::Cancel {
                        producer: selected,
                        job: job.id,
                    },
                    enabled: true,
                });
            }
        }
    }
    if let Some(station) = def.and_then(|unit| unit.research.as_ref()) {
        let queue_len = queue.map_or(0, |queue| queue.jobs.len());
        for technology in &station.technologies {
            let completed = replica.research.iter().any(|record| {
                record
                    .completed
                    .iter()
                    .any(|id| id == &technology.to_string())
            });
            if completed {
                let name = content.catalog.technologies.get(technology).map_or_else(
                    || technology.to_string(),
                    |technology| technology.name.clone(),
                );
                model.lines.push(format!("Research complete: {name}"));
                continue;
            }
            let definition = content.catalog.technologies.get(technology);
            let name = definition.map_or_else(
                || technology.to_string(),
                |technology| technology.name.clone(),
            );
            let affordable = definition
                .is_some_and(|technology| recipe_affordable(replica, selected, &technology.costs));
            if let Some(definition) = definition {
                append_costs(
                    &mut model.lines,
                    "Research cost",
                    &definition
                        .costs
                        .iter()
                        .map(|cost| turts_mod_api::Cost {
                            resource: cost.resource.to_string(),
                            amount: cost.amount,
                            source: cost.source,
                        })
                        .collect::<Vec<_>>(),
                    content,
                );
            }
            if queue_len > 0 {
                model
                    .lines
                    .push("Research in progress; wait for completion.".into());
            } else if !affordable {
                model.lines.push(
                    "Waiting for required items in the lab inventory or slot balance.".into(),
                );
            } else {
                model
                    .lines
                    .push("Ready: use the Research button below.".into());
            }
            model.controls.push(UiControl {
                label: format!("Research {name}"),
                action: UiAction::Produce {
                    producer: selected,
                    recipe: technology.to_string(),
                },
                enabled: queue_len < usize::from(station.queue_capacity) && affordable,
            });
        }
    }
    if def.is_some_and(|unit| unit.research.is_some())
        && let Some(queue) = queue
    {
        for job in &queue.jobs {
            model.lines.push(format!(
                "Research progress: {}/{} ticks",
                job.elapsed_ticks, job.duration_ticks
            ));
        }
    }
    if let Some(definition) = def {
        let toggleable = definition.extractor.is_some()
            || definition.inserter.is_some()
            || definition.machine.is_some()
            || definition.research.is_some()
            || definition.producer.is_some()
            || definition.weapon.is_some();
        if let Some(enabled) = unit_enabled(replica, selected).filter(|_| toggleable) {
            model.lines.push(format!("Enabled: {enabled}"));
            model.controls.push(UiControl {
                label: if enabled {
                    "Disable unit"
                } else {
                    "Enable unit"
                }
                .into(),
                action: UiAction::SetEnabled {
                    unit: selected,
                    enabled: !enabled,
                },
                enabled: true,
            });
        }
        if definition
            .removal
            .as_ref()
            .is_some_and(|policy| policy.by_command)
        {
            model.controls.push(UiControl {
                label: "Remove unit".into(),
                action: UiAction::Remove { unit: selected },
                enabled: true,
            });
        }
    }
    append_last_result(&mut model, results);
    model
}

fn append_m3_records(
    model: &mut PanelModel,
    replica: &Replica,
    selected: u32,
    definition: Option<&turts_mod_api::ResolvedUnit>,
) {
    let Some(definition) = definition else { return };
    if definition.conveyor.is_some() {
        match replica
            .belts
            .iter()
            .find(|record| record.piece.0 == selected)
        {
            Some(record) => model.lines.push(format!(
                "Belt: {} / {} items",
                record.slots.iter().flatten().count(),
                record.slots.len()
            )),
            None => model.lines.push("Belt: unavailable".into()),
        }
    }
    if definition.machine.is_some() {
        match replica
            .economy
            .queues
            .iter()
            .find(|record| record.producer.0 == selected)
        {
            Some(record) => {
                let stage = record
                    .jobs
                    .first()
                    .map(|job| format!("{:?}", job.stage))
                    .unwrap_or_else(|| "idle".into());
                model.lines.push(format!(
                    "Machine: {stage} · {}",
                    if record.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                ));
            }
            None => model.lines.push("Machine: unavailable".into()),
        }
    }
    if definition.inserter.is_some() {
        match replica
            .inserters
            .iter()
            .find(|record| record.unit.0 == selected)
        {
            Some(record) => model.lines.push(format!(
                "Inserter: {:?} · {}",
                record.stage,
                if record.enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            )),
            None => model.lines.push("Inserter: unavailable".into()),
        }
    }
    if definition.extractor.is_some() {
        match replica
            .extractors
            .iter()
            .find(|record| record.unit.0 == selected)
        {
            Some(record) => model.lines.push(format!(
                "Extractor: {:?} · {}",
                record.status,
                if record.enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            )),
            None => model.lines.push("Extractor: unavailable".into()),
        }
    }
    if definition.weapon.is_some() {
        match replica
            .weapons
            .iter()
            .find(|record| record.unit.0 == selected)
        {
            Some(record) => {
                model.lines.push(format!(
                    "Weapon: {:?} · {}",
                    record.status,
                    if record.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                ));
                match record.damage {
                    Ok(damage) => model.lines.push(format!("Damage: {damage}")),
                    Err(reason) => model
                        .lines
                        .push(format!("Damage: unavailable · {}", reason.as_str())),
                }
            }
            None => model.lines.push("Weapon: unavailable".into()),
        }
    }
    if definition.health.is_some() {
        match replica
            .healths
            .iter()
            .find(|record| record.unit.0 == selected)
        {
            Some(record) => model.lines.push(format!("Health: {}", record.current)),
            None => model.lines.push("Health: unavailable".into()),
        }
    }
}

fn unit_enabled(replica: &Replica, unit: u32) -> Option<bool> {
    replica
        .economy
        .queues
        .iter()
        .find(|record| record.producer.0 == unit)
        .map(|record| record.enabled)
        .or_else(|| {
            replica
                .inserters
                .iter()
                .find(|record| record.unit.0 == unit)
                .map(|record| record.enabled)
        })
        .or_else(|| {
            replica
                .extractors
                .iter()
                .find(|record| record.unit.0 == unit)
                .map(|record| record.enabled)
        })
        .or_else(|| {
            replica
                .weapons
                .iter()
                .find(|record| record.unit.0 == unit)
                .map(|record| record.enabled)
        })
}

fn recipe_affordable(
    replica: &Replica,
    producer: u32,
    costs: &[turts_mod_api::ResolvedCost],
) -> bool {
    let mut totals = BTreeMap::<(CostSource, ContentId), u128>::new();
    for cost in costs {
        let total = totals
            .entry((cost.source, cost.resource.clone()))
            .or_default();
        let Some(sum) = total.checked_add(u128::from(cost.amount)) else {
            return false;
        };
        *total = sum;
    }

    totals.into_iter().all(|((source, resource), total)| {
        let Ok(required) = u64::try_from(total) else {
            return false;
        };
        if required == 0 {
            return true;
        }
        let resource = resource.to_string();
        let available = match source {
            CostSource::SlotBalance => replica
                .economy
                .balances
                .iter()
                .find(|balance| balance.resource == resource)
                .and_then(|balance| balance.on_hand.checked_sub(balance.reserved)),
            CostSource::ProducerInventory => replica
                .economy
                .inventories
                .iter()
                .find(|inventory| {
                    inventory.unit.0 == producer && inventory.port == InventoryPort::Main
                })
                .and_then(|inventory| {
                    inventory
                        .items
                        .iter()
                        .find(|item| item.resource == resource)
                })
                .and_then(|item| item.on_hand.checked_sub(item.reserved)),
        };
        available.is_some_and(|available| available >= required)
    })
}

fn append_balances(model: &mut PanelModel, economy: &EconomyState, content: &ClientContent) {
    let ids: Vec<_> = content
        .catalog
        .resources
        .iter()
        .filter(|(_, def)| def.kind == ResourceKind::Balance)
        .map(|(id, def)| (id.to_string(), def.name.clone()))
        .collect();
    if ids.is_empty() {
        return;
    }
    model.lines.push("Balances".into());
    for (id, name) in ids {
        if let Some(balance) = economy.balances.iter().find(|b| b.resource == id) {
            match balance.on_hand.checked_sub(balance.reserved) {
                Some(available) => model.lines.push(format!(
                    "{name}: {} on hand · {available} available · {} reserved",
                    balance.on_hand, balance.reserved
                )),
                None => model.lines.push(format!(
                    "{name}: {} on hand · reserved unavailable",
                    balance.on_hand
                )),
            }
        }
    }
}

fn append_inventory(
    model: &mut PanelModel,
    economy: &EconomyState,
    content: &ClientContent,
    selected: u32,
    def: Option<&turts_mod_api::ResolvedUnit>,
) {
    let Some(unit_def) = def else { return };
    let main = unit_def.inventory.as_ref();
    let output = unit_def.output_inventory.as_ref();

    // Keep the established M2 copy and spacing for units with one Main inventory.
    if let (Some(main_def), None) = (main, output) {
        let Some(inventory) = economy.inventories.iter().find(|inventory| {
            inventory.unit.0 == selected && inventory.port == InventoryPort::Main
        }) else {
            model
                .lines
                .push("Inventory · this unit · unavailable".into());
            return;
        };
        model.lines.push("Inventory · this unit".into());
        append_inventory_contents(model, inventory, content, main_def.capacity);
        return;
    }

    for (port, label, inventory_def) in [
        (InventoryPort::Main, "Main", main),
        (InventoryPort::Output, "Output", output),
    ] {
        let Some(inventory_def) = inventory_def else {
            continue;
        };
        model.lines.push(format!("Inventory · {label}"));
        let Some(inventory) = economy
            .inventories
            .iter()
            .find(|inventory| inventory.unit.0 == selected && inventory.port == port)
        else {
            model.lines.push("Stock: unavailable".into());
            model
                .lines
                .push(format!("Capacity: unavailable/{}", inventory_def.capacity));
            continue;
        };
        append_inventory_contents(model, inventory, content, inventory_def.capacity);
    }
}

fn append_inventory_contents(
    model: &mut PanelModel,
    inventory: &turts_net_protocol::InventoryState,
    content: &ClientContent,
    capacity: u64,
) {
    let mut total = Some(0u64);
    for item in &inventory.items {
        total = total.and_then(|value| value.checked_add(item.on_hand));
        let name = ContentId::parse(&item.resource)
            .ok()
            .and_then(|id| content.catalog.resources.get(&id))
            .map_or_else(|| item.resource.clone(), |r| r.name.clone());
        model.lines.push(format!(
            "{name}: {} on hand · {} reserved",
            item.on_hand, item.reserved
        ));
    }
    match total {
        Some(on_hand) => model.lines.push(format!("Capacity: {on_hand}/{capacity}")),
        None => model
            .lines
            .push(format!("Capacity: unavailable/{capacity}")),
    }
}

fn append_order(model: &mut PanelModel, economy: &EconomyState, selected: u32) {
    if let Some(order) = economy.orders.iter().find(|o| o.unit.0 == selected) {
        let stage = match &order.stage {
            GatherStage::ToNode => "to_node".into(),
            GatherStage::Gathering => "gathering".into(),
            GatherStage::ToDropOff => "to_drop_off".into(),
            GatherStage::Unloading => "unloading".into(),
            GatherStage::Idle => "idle".into(),
            GatherStage::Waiting { reason } => format!("waiting · {}", reason.as_str()),
        };
        model.lines.push(format!("Gather: {stage}"));
    }
}

fn append_nodes(model: &mut PanelModel, replica: &Replica, content: &ClientContent) {
    let mut nodes: Vec<_> = replica.nodes.values().collect();
    nodes.sort_by_key(|node| node.id.0);
    if nodes.is_empty() {
        return;
    }
    model.lines.push("Nodes".into());
    for node in nodes {
        let name = ContentId::parse(&node.content_id)
            .ok()
            .and_then(|id| content.catalog.nodes.get(&id))
            .map_or_else(|| node.content_id.clone(), |n| n.name.clone());
        model
            .lines
            .push(format!("{name} · {} remaining", node.remaining));
    }
}

fn append_costs(
    lines: &mut Vec<String>,
    label: &str,
    costs: &[turts_mod_api::Cost],
    content: &ClientContent,
) {
    for cost in costs {
        let name = ContentId::parse(&cost.resource)
            .ok()
            .and_then(|id| content.catalog.resources.get(&id))
            .map_or_else(|| cost.resource.clone(), |r| r.name.clone());
        let source = match cost.source {
            CostSource::SlotBalance => "slot balance",
            CostSource::ProducerInventory => "this inventory",
        };
        lines.push(format!("{label}: {} {name} from {source}", cost.amount));
    }
}

fn append_last_result(model: &mut PanelModel, results: &CommandResults) {
    match results.0.last() {
        Some(CommandResult::Acked { seq, tick }) => model
            .lines
            .push(format!("Last command: acked · seq {seq} · tick {tick}")),
        Some(CommandResult::Applied { seq, tick, job_id }) => {
            let job = job_id.map_or_else(String::new, |id| format!(" · job {}", id.0));
            model.lines.push(format!(
                "Last command: applied · seq {seq} · tick {tick}{job}"
            ));
        }
        Some(CommandResult::Rejected { seq, tick, reason }) => model.lines.push(format!(
            "Last command: rejected · {} · seq {seq} · tick {tick}",
            reason.as_str()
        )),
        Some(CommandResult::Order {
            seq,
            tick,
            accepted,
            refused,
            envelope,
            job_id,
            ..
        }) => {
            if accepted.is_empty() {
                let reason = envelope
                    .or_else(|| refused.first().map(|unit| unit.reason))
                    .unwrap_or(turts_net_protocol::CommandRejectReason::Internal);
                model.lines.push(format!(
                    "Last command: rejected · {} · seq {seq} · tick {tick}",
                    reason.as_str()
                ));
            } else {
                let job = job_id.map_or_else(String::new, |id| format!(" · job {}", id.0));
                model.lines.push(format!(
                    "Last command: applied · seq {seq} · tick {tick}{job}"
                ));
            }
        }
        None => {}
    }
}

/// Choose the nearest owned unit whose declared footprint contains the map point.
pub fn hit_test_unit(
    replica: &Replica,
    content: &ClientContent,
    point: Pos,
    owner: u8,
) -> Option<u32> {
    let time = replica.presentation_time(bevy::platform::time::Instant::now())?;
    let mut hits = Vec::new();
    for (&id, unit) in &replica.units {
        if unit.owner_slot != owner {
            continue;
        }
        let Some(def) = ContentId::parse(&unit.content_id)
            .ok()
            .and_then(|id| content.catalog.units.get(&id))
        else {
            continue;
        };
        let Footprint::Circle { radius } = &def.footprint;
        let Some(pos) = replica.sample(id, time).map(|p| p.pos) else {
            continue;
        };
        let d2 = (pos.x - point.x).powi(2) + (pos.y - point.y).powi(2);
        if d2 <= radius * radius {
            hits.push((d2, id));
        }
    }
    hits.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    hits.first().map(|(_, id)| *id)
}

/// Choose the nearest declared node whose package radius contains the map point.
pub fn hit_test_node(replica: &Replica, content: &ClientContent, point: Pos) -> Option<NodeId> {
    let mut hits = Vec::new();
    for node in replica.nodes.values() {
        let Some(def) = ContentId::parse(&node.content_id)
            .ok()
            .and_then(|id| content.catalog.nodes.get(&id))
        else {
            continue;
        };
        let d2 = (node.pos.x - point.x).powi(2) + (node.pos.y - point.y).powi(2);
        if d2 <= def.radius * def.radius {
            hits.push((d2, node.id));
        }
    }
    hits.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.0.cmp(&b.1.0)));
    hits.first().map(|(_, id)| *id)
}

fn activate_action(
    trigger: On<Activate>,
    actions: Query<&UiActionTarget>,
    state: Res<ReplicaState>,
    acquired: Option<Res<AcquiredPackages>>,
    mut notice: ResMut<UiActionNotice>,
    mut pending: ResMut<PendingCommands>,
    mut selection: ResMut<Selection>,
) {
    let Ok(action) = actions.get(trigger.entity) else {
        return;
    };
    if let Err(reason) =
        validate_action_owner_and_capability(&state, acquired.as_deref(), &action.0)
    {
        notice.0 = Some(reason.into());
        return;
    }
    notice.0 = None;
    if let UiAction::Select { unit } = action.0 {
        selection.0 = Some(unit);
        return;
    }
    submit(
        &mut pending,
        match &action.0 {
            UiAction::Select { .. } => return,
            UiAction::Produce { producer, recipe } => ClientCommand::Produce {
                producer_id: UnitId(*producer),
                recipe_id: recipe.clone(),
            },
            UiAction::Cancel { producer, job } => ClientCommand::CancelProduction {
                producer_id: UnitId(*producer),
                job_id: *job,
            },
            UiAction::SetEnabled { unit, enabled } => ClientCommand::SetEnabled {
                unit_id: UnitId(*unit),
                enabled: *enabled,
            },
            UiAction::Remove { unit } => ClientCommand::Remove {
                unit_id: UnitId(*unit),
            },
        },
    );
}

fn validate_action_owner_and_capability(
    state: &ReplicaState,
    acquired: Option<&AcquiredPackages>,
    action: &UiAction,
) -> Result<(), &'static str> {
    let welcome = state.welcome.ok_or("not_owner")?;
    let replica = state.replica.as_ref().ok_or("not_owner")?;
    let unit_id = match action {
        UiAction::Produce { producer, .. } | UiAction::Cancel { producer, .. } => *producer,
        UiAction::Select { unit }
        | UiAction::SetEnabled { unit, .. }
        | UiAction::Remove { unit } => *unit,
    };
    let unit = replica.units.get(&unit_id).ok_or("not_owner")?;
    if unit.owner_slot != welcome.slot {
        return Err("not_owner");
    }
    let def = acquired
        .and_then(|acquired| {
            ContentId::parse(&unit.content_id)
                .ok()
                .and_then(|id| acquired.0.content.catalog.units.get(&id))
        })
        .ok_or("unit_capability_unavailable")?;
    match action {
        UiAction::Remove { .. }
            if !def.removal.as_ref().is_some_and(|policy| policy.by_command) =>
        {
            Err("not_removable")
        }
        UiAction::SetEnabled { .. }
            if !(def.extractor.is_some()
                || def.inserter.is_some()
                || def.machine.is_some()
                || def.research.is_some()
                || def.producer.is_some()
                || def.weapon.is_some()) =>
        {
            Err("not_toggleable")
        }
        UiAction::Produce { .. } if def.producer.is_none() && def.research.is_none() => {
            Err("not_producer")
        }
        UiAction::Cancel { .. } if def.producer.is_none() && def.research.is_none() => {
            Err("not_producer")
        }
        _ => Ok(()),
    }
}

/// Keep the panel widgets alive across replica progress updates so Bevy's pointer
/// press/release state remains attached to the same action entity.
pub fn sync_panel(world: &mut World) {
    let joined = matches!(*world.resource::<Phase>(), Phase::Joined { .. });
    if !joined {
        return;
    }
    let state = world.resource::<ReplicaState>();
    let acquired = world.get_resource::<AcquiredPackages>();
    let (Some(replica), Some(content)) = (state.replica.as_ref(), acquired.map(|a| &a.0.content))
    else {
        return;
    };
    let model = panel_model(
        replica,
        content,
        world.resource::<Selection>(),
        world.resource::<CommandResults>(),
    );
    let owner_slot = state.welcome.map(|welcome| welcome.slot);
    let mut model = model;
    model.controls.retain(|control| {
        let unit = match control.action {
            UiAction::Produce { producer, .. } | UiAction::Cancel { producer, .. } => producer,
            UiAction::Select { unit }
            | UiAction::SetEnabled { unit, .. }
            | UiAction::Remove { unit } => unit,
        };
        owner_slot.is_some_and(|slot| {
            replica
                .units
                .get(&unit)
                .is_some_and(|unit| unit.owner_slot == slot)
        })
    });
    model
        .controls
        .sort_by_key(|control| matches!(control.action, UiAction::Select { .. }));
    if let Some(message) = world.resource::<UiActionNotice>().0.as_deref() {
        model.lines.push(format!("Action denied: {message}"));
    }
    let viewport_width = world
        .iter_entities()
        .find(|entity| entity.contains::<bevy::window::PrimaryWindow>())
        .and_then(|entity| entity.get::<Window>())
        .map(Window::width)
        .unwrap_or(f32::INFINITY);
    let dock_bottom = PANEL_WIDE_WIDTH / viewport_width > 1.0 / 3.0;
    if world.resource::<PanelEntities>().root.is_none() {
        let root = world
            .spawn((
                UiPanelRoot,
                ScrollArea,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(12),
                    top: px(12),
                    width: px(PANEL_WIDE_WIDTH),
                    max_height: percent(94),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(5),
                    padding: UiRect::all(px(12)),
                    overflow: Overflow::scroll_y(),
                    border_radius: BorderRadius::all(px(6)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.045, 0.075, 0.09, 0.92)),
            ))
            .id();
        world.resource_mut::<PanelEntities>().root = Some(root);
    }
    let root = world
        .resource::<PanelEntities>()
        .root
        .expect("panel root created");
    if world.resource::<PanelEntities>().docked_bottom != Some(dock_bottom) {
        if let Some(mut node) = world.get_mut::<Node>(root) {
            node.left = px(12);
            node.top = if dock_bottom { Val::Auto } else { px(12) };
            node.right = if dock_bottom { px(12) } else { Val::Auto };
            node.bottom = if dock_bottom { px(12) } else { Val::Auto };
            node.width = if dock_bottom {
                Val::Auto
            } else {
                px(PANEL_WIDE_WIDTH)
            };
            node.max_height = percent(if dock_bottom { 35 } else { 94 });
        }
        world.resource_mut::<PanelEntities>().docked_bottom = Some(dock_bottom);
    }

    while world.resource::<PanelEntities>().lines.len() < model.lines.len() {
        let entity = world
            .spawn((
                Text::default(),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
                TextColor(Color::srgb(0.9, 0.93, 0.91)),
                UiSnapshotText,
                ChildOf(root),
            ))
            .id();
        world.resource_mut::<PanelEntities>().lines.push(entity);
    }
    let line_entities = world.resource::<PanelEntities>().lines.clone();
    for (index, entity) in line_entities.into_iter().enumerate() {
        if let Some(mut text) = world.get_mut::<Text>(entity)
            && let Some(line) = model.lines.get(index)
        {
            text.0.clone_from(line);
        }
        if let Some(mut node) = world.get_mut::<Node>(entity) {
            node.display = if index < model.lines.len() {
                Display::Flex
            } else {
                Display::None
            };
        }
    }

    let actions: std::collections::HashSet<_> =
        model.controls.iter().map(|c| c.action.clone()).collect();
    let stale: Vec<_> = world
        .resource::<PanelEntities>()
        .controls
        .keys()
        .filter(|action| !actions.contains(*action))
        .cloned()
        .collect();
    for action in stale {
        if let Some((button, _label)) = world
            .resource_mut::<PanelEntities>()
            .controls
            .remove(&action)
        {
            // ChildOf uses linked spawning, so despawning the button also retires
            // its label. Actions still present in `model.controls` retain their
            // entity identity (and Bevy's pointer state) across this sync.
            world.despawn(button);
        }
    }
    for control in &model.controls {
        let (button, label) = if let Some(pair) = world
            .resource::<PanelEntities>()
            .controls
            .get(&control.action)
            .copied()
        {
            pair
        } else {
            let button = world
                .spawn((
                    Button,
                    UiActionTarget(control.action.clone()),
                    UiControlEnabled(control.enabled),
                    Node {
                        min_height: px(30),
                        padding: UiRect::axes(px(8), px(5)),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.16, 0.29, 0.31)),
                    observe(activate_action),
                    ChildOf(root),
                ))
                .id();
            let label = world
                .spawn((
                    Text::new(control.label.clone()),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                    TextColor(Color::srgb(0.92, 0.95, 0.92)),
                    UiSnapshotText,
                    UiActionTarget(control.action.clone()),
                    UiActionOwner(button),
                    UiControlEnabled(control.enabled),
                    ChildOf(button),
                ))
                .id();
            world
                .resource_mut::<PanelEntities>()
                .controls
                .insert(control.action.clone(), (button, label));
            (button, label)
        };
        if let Some(mut text) = world.get_mut::<Text>(label) {
            text.0.clone_from(&control.label);
        }
        if let Some(mut node) = world.get_mut::<Node>(button) {
            node.display = Display::Flex;
        }
        if let Some(mut node) = world.get_mut::<Node>(label) {
            node.display = Display::Flex;
        }
        if let Some(mut enabled) = world.get_mut::<UiControlEnabled>(button) {
            enabled.0 = control.enabled;
        }
        if let Some(mut enabled) = world.get_mut::<UiControlEnabled>(label) {
            enabled.0 = control.enabled;
        }
        if control.enabled {
            world.entity_mut(button).remove::<InteractionDisabled>();
            world
                .entity_mut(button)
                .insert(BackgroundColor(Color::srgb(0.16, 0.29, 0.31)));
        } else {
            world.entity_mut(button).insert((
                InteractionDisabled,
                BackgroundColor(Color::srgb(0.18, 0.19, 0.19)),
            ));
        }
        if let Some(mut action) = world.get_mut::<UiActionTarget>(button) {
            action.0.clone_from(&control.action);
        }
        if let Some(mut action) = world.get_mut::<UiActionTarget>(label) {
            action.0.clone_from(&control.action);
        }
    }
    if model
        .controls
        .iter()
        .any(|control| matches!(control.action, UiAction::Select { .. }))
    {
        let entities = world.resource::<PanelEntities>();
        let mut ordered = entities.lines.clone();
        ordered.extend(
            model
                .controls
                .iter()
                .filter_map(|c| entities.controls.get(&c.action).map(|pair| pair.0)),
        );
        world.entity_mut(root).replace_children(&ordered);
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RenderedUiNode {
    pub kind: String,
    pub text: String,
    pub rect: [f32; 4],
    pub visible: bool,
    pub enabled: bool,
    pub action: Option<UiAction>,
    pub button_pressed: Option<bool>,
}

/// Read actual Bevy text and post-layout geometry for the native/browser oracle.
pub fn rendered_ui_snapshot(world: &World) -> Vec<RenderedUiNode> {
    let mut rows = Vec::new();
    for entity in world.iter_entities() {
        if entity.get::<UiSnapshotText>().is_none() {
            continue;
        }
        let text = entity.get::<Text>();
        let node = entity.get::<ComputedNode>();
        let transform = entity.get::<UiGlobalTransform>();
        let visible = text.is_some()
            && node.zip(transform).is_some_and(|(node, transform)| {
                actual_layout_visible(world, entity.id(), node, transform)
            });
        let action = entity.get::<UiActionTarget>().map(|value| value.0.clone());
        let action_owner = entity
            .get::<UiActionOwner>()
            .and_then(|owner| world.get_entity(owner.0).ok());
        let enabled = action.as_ref().is_none_or(|_| {
            action_owner.is_some_and(|button| {
                button.contains::<Button>() && !button.contains::<InteractionDisabled>()
            })
        });
        let button_pressed = action
            .as_ref()
            .map(|_| action_owner.is_some_and(|button| button.contains::<Pressed>()));
        let (rect, text_value) = match (text, node, transform) {
            (Some(text), Some(node), Some(transform)) => {
                let center = transform.to_scale_angle_translation().2;
                let size = node.size();
                (
                    [
                        center.x - size.x / 2.0,
                        center.y - size.y / 2.0,
                        size.x,
                        size.y,
                    ],
                    text.0.clone(),
                )
            }
            (Some(text), _, _) => ([0.0; 4], text.0.clone()),
            _ => continue,
        };
        rows.push(RenderedUiNode {
            kind: "text".into(),
            text: text_value,
            rect,
            visible,
            enabled,
            action,
            button_pressed,
        });
    }
    for entity in world.iter_entities() {
        if !entity.contains::<UiPanelRoot>() {
            continue;
        }
        let Some(node) = entity.get::<ComputedNode>() else {
            continue;
        };
        let Some(transform) = entity.get::<UiGlobalTransform>() else {
            continue;
        };
        let center = transform.to_scale_angle_translation().2;
        let size = node.size();
        rows.push(RenderedUiNode {
            kind: "panel".into(),
            text: String::new(),
            rect: [
                center.x - size.x / 2.0,
                center.y - size.y / 2.0,
                size.x,
                size.y,
            ],
            visible: actual_layout_visible(world, entity.id(), node, transform),
            enabled: true,
            action: None,
            button_pressed: None,
        });
    }
    rows.sort_by(|a, b| {
        a.rect[1]
            .total_cmp(&b.rect[1])
            .then(a.rect[0].total_cmp(&b.rect[0]))
    });
    rows
}

fn actual_layout_visible(
    world: &World,
    entity: Entity,
    node: &ComputedNode,
    transform: &UiGlobalTransform,
) -> bool {
    let Some(visibility) = world.get::<InheritedVisibility>(entity) else {
        return false;
    };
    if !visibility.get() || node.size().min_element() <= 0.0 {
        return false;
    }
    let Some(camera_id) = world
        .get::<ComputedUiTargetCamera>(entity)
        .and_then(ComputedUiTargetCamera::get)
    else {
        return false;
    };
    let Some(camera) = world
        .get::<Camera>(camera_id)
        .filter(|camera| camera.is_active)
    else {
        return false;
    };
    let Some(viewport) = camera.physical_viewport_rect() else {
        return false;
    };
    let center = transform.to_scale_angle_translation().2;
    let half = node.size() * 0.5;
    let min = center - half;
    let max = center + half;
    let viewport = Rect::from_corners(Vec2::ZERO, viewport.size().as_vec2());
    let clip = world.get::<CalculatedClip>(entity);
    if clip.is_some_and(CalculatedClip::is_fully_clipped) {
        return false;
    }
    // UI picking tests the same computed clip chain. Sampling the rendered text
    // bounds catches fully clipped/off-viewport labels while allowing partial
    // visibility at panel edges.
    [
        Vec2::ZERO,
        Vec2::new(0.5, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 0.5),
        Vec2::splat(0.5),
        Vec2::new(1.0, 0.5),
        Vec2::new(0.0, 1.0),
        Vec2::new(0.5, 1.0),
        Vec2::ONE,
    ]
    .into_iter()
    .map(|fraction| min + (max - min) * fraction)
    .any(|point| viewport.contains(point) && clip.is_none_or(|clip| clip.contains_point(point)))
}

/// Convert a logical window cursor into UI camera viewport pixels.
pub fn cursor_in_ui_viewport(cursor: Vec2, camera: &Camera) -> Option<Vec2> {
    let viewport = camera.physical_viewport_rect()?;
    let physical = cursor * camera.target_scaling_factor()?;
    let min = viewport.min.as_vec2();
    let max = viewport.max.as_vec2();
    if physical.x < min.x || physical.y < min.y || physical.x >= max.x || physical.y >= max.y {
        return None;
    }
    Some(physical - min)
}

fn validate_selection(mut selection: ResMut<Selection>, state: Res<ReplicaState>) {
    let Some(selected) = selection.0 else {
        return;
    };
    let (Some(welcome), Some(replica)) = (state.welcome, state.replica.as_ref()) else {
        return;
    };
    if replica
        .units
        .get(&selected)
        .is_none_or(|unit| unit.owner_slot != welcome.slot)
    {
        selection.0 = None;
    }
}

pub struct EconomyUiPlugin;
impl Plugin for EconomyUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Selection>()
            .init_resource::<PanelEntities>()
            .init_resource::<UiActionNotice>()
            .add_systems(
                Update,
                (validate_selection, sync_panel)
                    .chain()
                    .after(crate::present::update_presented),
            );
    }
}

#[cfg(test)]
mod activation_tests {
    use super::*;
    use crate::{
        acquire::build_acquired,
        commands::PendingCommands,
        ingest::{ReplicaState, WelcomeInfo},
        join::AcquiredPackages,
        replica::{Replica, UnitReplica},
    };
    use std::{collections::VecDeque, path::PathBuf};
    use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};

    #[test]
    fn one_activate_event_submits_one_exact_produce_or_cancel_command() {
        let mut world = World::new();
        world.init_resource::<PendingCommands>();
        world.init_resource::<UiActionNotice>();
        world.init_resource::<Selection>();
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
        let archive = PackageArchive::parse(&pack_dir(&base).expect("pack base"))
            .expect("parse base archive");
        let acquired = build_acquired(&[archive], String::new()).expect("acquire base package");
        let (producer_id, producer) = acquired
            .content
            .catalog
            .units
            .iter()
            .find(|(_, unit)| unit.producer.is_some())
            .expect("base has a producer");
        let recipe_id = producer.producer.as_ref().unwrap().recipes[0].to_string();
        let producer_id = producer_id.to_string();
        let mut replica = Replica::new(20);
        replica.units.insert(
            8,
            UnitReplica {
                content_id: producer_id,
                owner_slot: 0,
                facing: None,
                samples: VecDeque::new(),
            },
        );
        let state = ReplicaState {
            welcome: Some(WelcomeInfo {
                slot: 0,
                tick: 0,
                tick_hz: 20,
                match_id: 0,
            }),
            replica: Some(replica),
            ..ReplicaState::default()
        };
        world.insert_resource(state);
        world.insert_resource(AcquiredPackages(acquired));

        let actions = [
            (
                UiAction::Produce {
                    producer: 8,
                    recipe: recipe_id.clone(),
                },
                ClientCommand::Produce {
                    producer_id: UnitId(8),
                    recipe_id,
                },
            ),
            (
                UiAction::Cancel {
                    producer: 8,
                    job: JobId(91),
                },
                ClientCommand::CancelProduction {
                    producer_id: UnitId(8),
                    job_id: JobId(91),
                },
            ),
        ];
        for (index, (action, kind)) in actions.into_iter().enumerate() {
            let entity = world
                .spawn((UiActionTarget(action), observe(activate_action)))
                .id();
            world.trigger(Activate { entity });
            let pending = &world.resource::<PendingCommands>().0;
            assert_eq!(pending.len(), index + 1);
            assert_eq!(
                pending.back(),
                Some(&crate::commands::PendingCommand::Submit {
                    kind,
                    trace_step: None,
                })
            );
        }
        let selector = world
            .spawn((
                UiActionTarget(UiAction::Select { unit: 8 }),
                observe(activate_action),
            ))
            .id();
        world.trigger(Activate { entity: selector });
        assert_eq!(world.resource::<Selection>().0, Some(8));
        assert_eq!(
            world.resource::<PendingCommands>().0.len(),
            2,
            "selection is local, never a server command"
        );
        world.resource_mut::<Selection>().0 = None;
        world
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .unwrap()
            .units
            .get_mut(&8)
            .unwrap()
            .owner_slot = 1;
        world.trigger(Activate { entity: selector });
        assert_eq!(
            world.resource::<Selection>().0,
            None,
            "foreign unit selection is rejected"
        );
        assert_eq!(world.resource::<PendingCommands>().0.len(), 2);
    }
}
