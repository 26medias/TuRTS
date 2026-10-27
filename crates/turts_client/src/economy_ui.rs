//! Shared native/wasm M2 economy panel backed by the validated replica and downloaded catalog.

use bevy::camera::visibility::InheritedVisibility;
use bevy::{
    prelude::*,
    ui::{
        CalculatedClip, ComputedNode, ComputedUiTargetCamera, InteractionDisabled,
        UiGlobalTransform,
    },
    ui_widgets::{Activate, Button, ScrollArea, observe},
};
use turts_mod_api::{ContentId, CostSource, Footprint, ResourceKind};
use turts_net_protocol::{
    EconomyState, GatherStage, JobId, JobStage, NodeId, PlayerCommandKind, Pos, UnitId,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiAction {
    Produce { producer: u32, recipe: String },
    Cancel { producer: u32, job: JobId },
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
    let Some(selected) = selection.0 else {
        model.lines.push("Select a unit or producer".into());
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
    append_balances(&mut model, &replica.economy, content);
    append_inventory(&mut model, &replica.economy, content, selected, def);
    append_order(&mut model, &replica.economy, selected);
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
    append_last_result(&mut model, results);
    model
}

fn recipe_affordable(
    replica: &Replica,
    producer: u32,
    costs: &[turts_mod_api::ResolvedCost],
) -> bool {
    costs.iter().all(|cost| {
        if cost.amount == 0 {
            return true;
        }
        let resource = cost.resource.to_string();
        let available = match cost.source {
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
                .find(|inventory| inventory.unit.0 == producer)
                .and_then(|inventory| {
                    inventory
                        .items
                        .iter()
                        .find(|item| item.resource == resource)
                })
                .and_then(|item| item.on_hand.checked_sub(item.reserved)),
        };
        available.is_some_and(|available| available >= cost.amount)
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
    let Some(inventory_def) = def.and_then(|d| d.inventory.as_ref()) else {
        return;
    };
    let Some(inventory) = economy.inventories.iter().find(|i| i.unit.0 == selected) else {
        model
            .lines
            .push("Inventory · this unit · unavailable".into());
        return;
    };
    model.lines.push("Inventory · this unit".into());
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
        Some(on_hand) => model
            .lines
            .push(format!("Capacity: {on_hand}/{}", inventory_def.capacity)),
        None => model
            .lines
            .push(format!("Capacity: unavailable/{}", inventory_def.capacity)),
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
    mut pending: ResMut<PendingCommands>,
) {
    let Ok(action) = actions.get(trigger.entity) else {
        return;
    };
    submit(
        &mut pending,
        match &action.0 {
            UiAction::Produce { producer, recipe } => PlayerCommandKind::Produce {
                producer_id: UnitId(*producer),
                recipe_id: recipe.clone(),
            },
            UiAction::Cancel { producer, job } => PlayerCommandKind::CancelProduction {
                producer_id: UnitId(*producer),
                job_id: *job,
            },
        },
    );
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
    if world.resource::<PanelEntities>().root.is_none() {
        let root = world
            .spawn((
                UiPanelRoot,
                ScrollArea,
                Node {
                    position_type: PositionType::Absolute,
                    left: px(12),
                    top: px(12),
                    width: px(390),
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
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RenderedUiNode {
    pub text: String,
    pub rect: [f32; 4],
    pub visible: bool,
    pub enabled: bool,
    pub action: Option<UiAction>,
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
        let enabled = action.as_ref().is_none_or(|_| {
            entity
                .get::<UiActionOwner>()
                .and_then(|owner| world.get_entity(owner.0).ok())
                .is_some_and(|button| {
                    button.contains::<Button>() && !button.contains::<InteractionDisabled>()
                })
        });
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
            text: text_value,
            rect,
            visible,
            enabled,
            action,
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
    let Some(clip) = world.get::<CalculatedClip>(entity) else {
        return false;
    };
    if clip.is_fully_clipped() {
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
    .any(|point| viewport.contains(point) && clip.contains_point(point))
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
    use crate::commands::PendingCommands;

    #[test]
    fn one_activate_event_submits_one_exact_produce_or_cancel_command() {
        let mut world = World::new();
        world.init_resource::<PendingCommands>();
        let action = UiAction::Produce {
            producer: 8,
            recipe: "custom:worker".into(),
        };
        let entity = world
            .spawn((UiActionTarget(action), observe(activate_action)))
            .id();
        world.trigger(Activate { entity });
        let pending = &world.resource::<PendingCommands>().0;
        assert_eq!(pending.len(), 1);
        assert_eq!(
            pending[0],
            PlayerCommandKind::Produce {
                producer_id: UnitId(8),
                recipe_id: "custom:worker".into(),
            }
        );
    }
}
