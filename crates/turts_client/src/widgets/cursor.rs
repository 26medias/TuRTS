//! Apply the accepted interface's cursor image to the actual primary window.

use bevy::prelude::*;
use bevy::window::{CursorIcon, CursorOptions, CustomCursor, CustomCursorImage, PrimaryWindow};
use turts_mod_api::{ContentId, OrderDef, TargetKind};

use crate::{
    core_adapter::CoreRuntime, debug_feed::CursorFeed, ingest::ReplicaState,
    join::AcquiredPackages, present::Presented, replica::Replica,
};

#[derive(Resource, Default, Debug, Clone)]
pub struct CursorState {
    context: String,
    image_id: Option<String>,
    hotspot: (u16, u16),
}

/// Derive the context from the same public selection/replica facts as the interaction core,
/// then apply the accepted package image and report the actual `CursorIcon` component.
pub fn update_cursor(world: &mut World) {
    if world
        .get_resource::<crate::join::Phase>()
        .is_some_and(|phase| matches!(phase, crate::join::Phase::Failed(_)))
    {
        clear_cursor(world);
        return;
    }
    let Some(acquired) = world.get_resource::<AcquiredPackages>() else {
        return;
    };
    let presentation = acquired.0.presentation.clone();
    let mut content = acquired.0.content.clone();
    let Some(state) = world.get_resource::<ReplicaState>() else {
        return;
    };
    if let Some(current) = state.content.as_ref() {
        content = current.clone();
    }
    let Some(replica) = state.replica.clone() else {
        return;
    };
    let Some(slot) = state.welcome.map(|welcome| welcome.slot) else {
        return;
    };
    let Some(presented) = world.get_resource::<Presented>().cloned() else {
        return;
    };
    let cursor_position = world
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .iter(world)
        .find_map(Window::cursor_position)
        .map(|position| [position.x, position.y]);
    let primary_cursor = world
        .query_filtered::<(Entity, Option<&CursorOptions>), With<PrimaryWindow>>()
        .iter(world)
        .next();
    let lock_requested = primary_cursor.is_some_and(|(_, options)| {
        options.is_some_and(|options| options.grab_mode == bevy::window::CursorGrabMode::Locked)
    });
    let lock_active = world
        .get_resource::<CursorFeed>()
        .and_then(|feed| feed.lock_active)
        == Some(true);
    let engine_position = if lock_active {
        world
            .get_resource::<CoreRuntime>()
            .and_then(CoreRuntime::engine_cursor_position)
    } else {
        cursor_position.map(|position| Vec2::new(position[0], position[1]))
    };
    let selection = world
        .get_resource::<crate::debug_feed::SelectionFeed>()
        .map_or_else(Vec::new, |feed| feed.0.clone());
    let pending_order = world
        .get_resource::<CoreRuntime>()
        .and_then(CoreRuntime::pending_order_id)
        .and_then(|id| ContentId::parse(id).ok());

    let hover = current_hover(world, &replica, &content, &presented, engine_position);
    let (context, override_asset) = if let Some(order_id) = pending_order.as_ref() {
        let order = content.catalog.orders.get(order_id);
        let valid = order.is_some_and(|order| {
            order_target_valid(order, hover, &selection, &replica, &content, slot)
        });
        if !valid {
            ("forbidden".to_string(), None)
        } else {
            let order_cursor =
                order.and_then(|order| order.cursor.as_deref().map(|cursor| (order_id, cursor)));
            (
                "target".to_string(),
                order_cursor.and_then(|(order_id, cursor)| {
                    presentation
                        .cursor_assets
                        .get(&format!("{}:{cursor}", order_id.package.0))
                        .cloned()
                }),
            )
        }
    } else {
        (
            world
                .get_resource::<CoreRuntime>()
                .and_then(CoreRuntime::camera_pan_context)
                .map(str::to_owned)
                .unwrap_or_else(|| hover_context(hover, &selection, &replica, &content, slot)),
            None,
        )
    };
    let selected_cursor = override_asset
        .or_else(|| super::assets::primitive_cursor(&presentation, &context))
        .or_else(|| presentation.cursors.get(&context).cloned())
        .or_else(|| super::assets::primitive_cursor(&presentation, "default"))
        .or_else(|| presentation.cursors.get("default").cloned());
    let (image_id, hotspot) = selected_cursor.unwrap_or((String::new(), (0, 0)));
    let mut cursor_state = world
        .get_resource::<CursorState>()
        .cloned()
        .unwrap_or_default();
    cursor_state.context = context.clone();
    cursor_state.image_id = (!image_id.is_empty()).then(|| image_id.clone());
    cursor_state.hotspot = hotspot;
    world.insert_resource(cursor_state.clone());

    let local_image = image_id
        .split_once(':')
        .map_or(image_id.as_str(), |(_, local)| local)
        .to_owned();
    let cursor_handle = world
        .get_resource::<super::assets::WidgetAssets>()
        .and_then(|assets| assets.cursor_image_for(&image_id, hotspot));
    let mut applied = None;
    if let Some(handle) = cursor_handle.clone() {
        let usable = world
            .get_resource::<Assets<Image>>()
            .and_then(|assets| assets.get(handle.id()))
            .is_some_and(|image| {
                image.texture_descriptor.format
                    == bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb
                    && image.data.is_some()
                    && u32::from(hotspot.0) < image.texture_descriptor.size.width
                    && u32::from(hotspot.1) < image.texture_descriptor.size.height
            });
        if usable {
            let icon = CursorIcon::Custom(CustomCursor::Image(CustomCursorImage {
                handle,
                hotspot,
                ..default()
            }));
            let mut windows = world.query_filtered::<Entity, With<PrimaryWindow>>();
            if let Some(window) = windows.iter(world).next() {
                world.entity_mut(window).insert(icon.clone());
                if world.get::<CursorIcon>(window) == Some(&icon) {
                    applied = Some(local_image.clone());
                }
            }
        }
    }
    if applied.is_none() {
        let mut windows = world.query_filtered::<Entity, With<PrimaryWindow>>();
        if let Some(window) = windows.iter(world).next() {
            world.entity_mut(window).remove::<CursorIcon>();
        }
    }
    update_locked_cursor(world, lock_active, engine_position, cursor_handle, hotspot);
    let (relative_motion, frame_index) =
        world
            .get_resource::<CoreRuntime>()
            .map_or(([0.0, 0.0], 0), |runtime| {
                let motion = runtime.relative_motion_this_frame();
                let frame_index = world
                    .get_resource::<crate::core_adapter::FrameFeeds>()
                    .map_or(0, |frames| frames.frame_index);
                ([motion.x, motion.y], frame_index)
            });
    if let Some(mut feed) = world.get_resource_mut::<CursorFeed>() {
        feed.context = Some(context);
        feed.image_id = (!image_id.is_empty()).then(|| local_image.to_owned());
        feed.applied_icon = applied;
        feed.position = cursor_position;
        feed.engine_position = engine_position.map(|position| [position.x, position.y]);
        feed.lock_requested = lock_requested;
        feed.relative_motion = relative_motion;
        feed.frame_index = frame_index;
    }
}

#[derive(Component)]
struct LockedCursorSprite;

fn update_locked_cursor(
    world: &mut World,
    locked: bool,
    position: Option<Vec2>,
    handle: Option<Handle<Image>>,
    hotspot: (u16, u16),
) {
    let image_size = handle.as_ref().and_then(|handle| {
        world
            .get_resource::<Assets<Image>>()
            .and_then(|images| images.get(handle.id()))
            .map(|image| image.texture_descriptor.size)
    });
    let sprite = world
        .query_filtered::<Entity, With<LockedCursorSprite>>()
        .iter(world)
        .next();
    let (Some(position), Some(handle), Some(size)) =
        (locked.then_some(position).flatten(), handle, image_size)
    else {
        if let Some(sprite) = sprite {
            world.entity_mut(sprite).despawn();
        }
        return;
    };
    // Pointer positions and authored hotspots are window-logical pixels. UI layout
    // applies the player's UI scale separately from the window's device scale.
    let ui_scale = world
        .get_resource::<bevy::ui::UiScale>()
        .map_or(1.0, |scale| scale.0);
    let node = Node {
        position_type: PositionType::Absolute,
        left: Val::Px((position.x - f32::from(hotspot.0)) / ui_scale),
        top: Val::Px((position.y - f32::from(hotspot.1)) / ui_scale),
        width: Val::Px(size.width as f32 / ui_scale),
        height: Val::Px(size.height as f32 / ui_scale),
        ..default()
    };
    let image = ImageNode::new(handle);
    if let Some(sprite) = sprite {
        world
            .entity_mut(sprite)
            .insert((node, image, GlobalZIndex(i32::MAX)));
    } else {
        world.spawn((LockedCursorSprite, node, image, GlobalZIndex(i32::MAX)));
    }
}

pub fn clear_cursor(world: &mut World) {
    let mut windows = world.query_filtered::<Entity, With<PrimaryWindow>>();
    if let Some(window) = windows.iter(world).next() {
        world.entity_mut(window).remove::<CursorIcon>();
    }
    world.insert_resource(CursorState::default());
    if let Some(mut feed) = world.get_resource_mut::<CursorFeed>() {
        feed.context = None;
        feed.image_id = None;
        feed.applied_icon = None;
        feed.position = None;
        feed.engine_position = None;
        feed.lock_requested = false;
        feed.relative_motion = [0.0, 0.0];
    }
    let sprites = world
        .query_filtered::<Entity, With<LockedCursorSprite>>()
        .iter(world)
        .collect::<Vec<_>>();
    for sprite in sprites {
        world.entity_mut(sprite).despawn();
    }
}

#[derive(Clone, Copy)]
enum Hover {
    Ground,
    Unit { id: u32, owner: u8 },
    Node { id: u32 },
}

fn current_hover(
    world: &mut World,
    replica: &Replica,
    content: &crate::content::ClientContent,
    presented: &Presented,
    cursor: Option<Vec2>,
) -> Hover {
    let cursor = cursor.or_else(|| {
        let mut windows = world.query_filtered::<&Window, With<PrimaryWindow>>();
        windows.iter(world).find_map(Window::cursor_position)
    });
    let Some(cursor) = cursor else {
        return Hover::Ground;
    };
    let ray = {
        let mut cameras = world.query_filtered::<(&Camera, &GlobalTransform), With<Camera3d>>();
        cameras
            .iter(world)
            .find_map(|(camera, transform)| camera.viewport_to_world(transform, cursor).ok())
    };
    let Some(point) = ray.and_then(crate::input::ground_point) else {
        return Hover::Ground;
    };
    if let Some(unit) = presented
        .units
        .iter()
        .filter_map(|unit| {
            let replica_unit = replica.units.get(&unit.unit_id)?;
            let definition = ContentId::parse(&unit.content_id)
                .ok()
                .and_then(|id| content.catalog.units.get(&id))?;
            let radius = match &definition.footprint {
                turts_mod_api::Footprint::Circle { radius } => *radius,
            };
            let distance = (unit.pos.x - point.x).powi(2) + (unit.pos.y - point.y).powi(2);
            (distance <= radius * radius).then_some((
                distance,
                unit.unit_id,
                replica_unit.owner_slot,
            ))
        })
        .min_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
        })
    {
        return Hover::Unit {
            id: unit.1,
            owner: unit.2,
        };
    }
    if let Some(id) = crate::input::hit_test_node(replica, content, point) {
        return Hover::Node { id: id.0 };
    }
    Hover::Ground
}

fn hover_context(
    hover: Hover,
    selection: &[u32],
    replica: &Replica,
    content: &crate::content::ClientContent,
    slot: u8,
) -> String {
    let own_selection = selection
        .iter()
        .filter_map(|id| replica.units.get(id))
        .filter(|unit| unit.owner_slot == slot)
        .collect::<Vec<_>>();
    if own_selection.is_empty() {
        return if matches!(hover, Hover::Unit { owner, .. } if owner == slot) {
            "select".into()
        } else {
            "default".into()
        };
    }
    for rule in &content.order_rules.smart {
        let target_kind = rule.target;
        for selected in &own_selection {
            let Some(definition) = ContentId::parse(&selected.content_id)
                .ok()
                .and_then(|id| content.catalog.units.get(&id))
            else {
                continue;
            };
            let target_kinds = actor_target_kinds(hover, definition, replica, content, slot);
            if !target_kinds.contains(&target_kind) || !has_capability(rule.needs, definition) {
                continue;
            }
            let Ok(order_id) = ContentId::parse(&rule.order.to_string()) else {
                continue;
            };
            let Some(order) = content.catalog.orders.get(&order_id) else {
                continue;
            };
            if !order.units.is_empty() && !order.units.contains(&selected.content_id) {
                continue;
            }
            if !order_capability(order, definition) {
                continue;
            }
            let compatible = match target_kind {
                turts_mod_api::SmartTarget::Ground => {
                    matches!(order.target, TargetKind::Point | TargetKind::PointOrUnit)
                }
                turts_mod_api::SmartTarget::OwnUnit => {
                    matches!(order.target, TargetKind::Unit | TargetKind::PointOrUnit)
                        || (order.behaviour == turts_mod_api::Behaviour::Move
                            && order.target == TargetKind::Point)
                }
                turts_mod_api::SmartTarget::ForeignUnitWeaponTarget => {
                    order.behaviour == turts_mod_api::Behaviour::Attack
                        && matches!(order.target, TargetKind::Unit | TargetKind::PointOrUnit)
                }
                turts_mod_api::SmartTarget::ForeignUnit => {
                    !matches!(order.behaviour, turts_mod_api::Behaviour::Attack)
                        || matches!(order.target, TargetKind::Unit | TargetKind::PointOrUnit)
                }
                turts_mod_api::SmartTarget::ResourceNodeAccepted => {
                    order.behaviour == turts_mod_api::Behaviour::Gather
                        && order.target == TargetKind::Node
                }
                turts_mod_api::SmartTarget::Node => {
                    order.behaviour == turts_mod_api::Behaviour::Move
                        && matches!(order.target, TargetKind::Point | TargetKind::PointOrUnit)
                }
                turts_mod_api::SmartTarget::Content => false,
                // API 0.2.4 (row O6): a repair cursor is `UI2-HUD`; until then it is no match.
                turts_mod_api::SmartTarget::OwnDamaged => false,
            };
            if !compatible {
                continue;
            }
            return match order.behaviour {
                turts_mod_api::Behaviour::Move => "move".into(),
                turts_mod_api::Behaviour::Attack => "attack".into(),
                turts_mod_api::Behaviour::Gather => "gather".into(),
                _ => "default".into(),
            };
        }
    }
    "forbidden".into()
}

/// Resolve smart-target classes for one eligible actor. Keeping this actor-specific ensures
/// a gatherer that accepts the hovered node cannot lend that capability to another selection.
fn actor_target_kinds(
    hover: Hover,
    definition: &turts_mod_api::ResolvedUnit,
    replica: &Replica,
    content: &crate::content::ClientContent,
    slot: u8,
) -> Vec<turts_mod_api::SmartTarget> {
    use turts_mod_api::SmartTarget;
    match hover {
        Hover::Ground => vec![SmartTarget::Ground],
        Hover::Unit { owner, .. } if owner == slot => vec![SmartTarget::OwnUnit],
        Hover::Unit { id, owner } => {
            let weapon_target = definition.weapon.as_ref().is_some_and(|weapon| {
                let relation_matches =
                    crate::relation::holds_own_team(weapon.targets.slots, slot, owner);
                relation_matches
                    && replica
                        .units
                        .get(&id)
                        .and_then(|target| ContentId::parse(&target.content_id).ok())
                        .and_then(|id| content.catalog.units.get(&id))
                        .is_some()
            });
            if weapon_target {
                vec![
                    SmartTarget::ForeignUnitWeaponTarget,
                    SmartTarget::ForeignUnit,
                ]
            } else {
                vec![SmartTarget::ForeignUnit]
            }
        }
        Hover::Node { id } => {
            let accepted = replica
                .nodes
                .get(&turts_net_protocol::NodeId(id))
                .and_then(|node| ContentId::parse(&node.content_id).ok())
                .and_then(|id| content.catalog.nodes.get(&id))
                .is_some_and(|node| {
                    definition
                        .gatherer
                        .as_ref()
                        .is_some_and(|gatherer| gatherer.items.contains(&node.item))
                });
            if accepted {
                vec![SmartTarget::ResourceNodeAccepted, SmartTarget::Node]
            } else {
                vec![SmartTarget::Node]
            }
        }
    }
}

fn has_capability(
    capability: turts_mod_api::Capability,
    unit: &turts_mod_api::ResolvedUnit,
) -> bool {
    match capability {
        turts_mod_api::Capability::Movement => unit.movement.is_some(),
        turts_mod_api::Capability::Weapon => unit.weapon.is_some(),
        turts_mod_api::Capability::Gatherer => unit.gatherer.is_some(),
        turts_mod_api::Capability::Producer => unit.producer.is_some(),
        turts_mod_api::Capability::Research => unit.research.is_some(),
        // API 0.2.4 (row O6): the resolved unit has no constructor or repairer yet
        // (`L-CON1`, `L-REP` add them); no unit has the capability until then.
        turts_mod_api::Capability::Constructor | turts_mod_api::Capability::Repairer => false,
    }
}

fn order_target_valid(
    order: &OrderDef,
    hover: Hover,
    selection: &[u32],
    replica: &Replica,
    content: &crate::content::ClientContent,
    slot: u8,
) -> bool {
    let has_eligible_unit = selection
        .iter()
        .filter_map(|id| replica.units.get(id))
        .any(|unit| {
            unit.owner_slot == slot
                && (order.units.is_empty() || order.units.contains(&unit.content_id))
                && ContentId::parse(&unit.content_id)
                    .ok()
                    .and_then(|id| content.catalog.units.get(&id))
                    .is_some_and(|definition| order_capability(order, definition))
        });
    if !has_eligible_unit {
        return false;
    }
    match order.target {
        TargetKind::None => false,
        TargetKind::Point => true,
        TargetKind::PointOrUnit => match hover {
            Hover::Unit { id, owner } if order.behaviour == turts_mod_api::Behaviour::Attack => {
                order_unit_target_relation(order, id, owner, selection, replica, content, slot)
            }
            _ => true,
        },
        TargetKind::Unit => {
            matches!(hover, Hover::Unit { id, owner } if order_unit_target_relation(order, id, owner, selection, replica, content, slot))
        }
        TargetKind::Node => match hover {
            Hover::Node { id: node_id } => {
                order.behaviour != turts_mod_api::Behaviour::Gather
                    || selection
                        .iter()
                        .filter_map(|unit_id| replica.units.get(unit_id))
                        .filter(|unit| {
                            unit.owner_slot == slot
                                && (order.units.is_empty()
                                    || order.units.contains(&unit.content_id))
                        })
                        .any(|unit| {
                            ContentId::parse(&unit.content_id)
                                .ok()
                                .and_then(|id| content.catalog.units.get(&id))
                                .filter(|definition| order_capability(order, definition))
                                .and_then(|definition| definition.gatherer.as_ref())
                                .is_some_and(|gatherer| {
                                    replica
                                        .nodes
                                        .get(&turts_net_protocol::NodeId(node_id))
                                        .and_then(|node| {
                                            ContentId::parse(&node.content_id)
                                                .ok()
                                                .and_then(|id| content.catalog.nodes.get(&id))
                                        })
                                        .is_some_and(|node| gatherer.items.contains(&node.item))
                                })
                        })
            }
            _ => false,
        },
        // `ContentAtPoint` (API 0.2.4, row O5) is the target of a placement: `UI-PLACE`.
        TargetKind::Job | TargetKind::Content | TargetKind::ContentAtPoint => false,
    }
}

fn order_capability(order: &OrderDef, unit: &turts_mod_api::ResolvedUnit) -> bool {
    use turts_mod_api::Behaviour;
    match order.behaviour {
        Behaviour::Move => unit.movement.is_some(),
        Behaviour::Attack => match order.target {
            TargetKind::Point => unit.movement.is_some(),
            TargetKind::PointOrUnit => unit.movement.is_some() || unit.weapon.is_some(),
            _ => unit.weapon.is_some(),
        },
        Behaviour::Gather => unit.gatherer.is_some(),
        Behaviour::Produce => unit.producer.is_some() || unit.research.is_some(),
        Behaviour::CancelProduction => {
            unit.producer.is_some()
                || unit.research.is_some()
                || (order.target == TargetKind::Job && unit.machine.is_some())
        }
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
        // API 0.2.4: no unit is capable of a new behaviour until its lane builds it.
        Behaviour::Construct
        | Behaviour::CancelConstruction
        | Behaviour::SetRally
        | Behaviour::Repair
        | Behaviour::Surrender => false,
    }
}

fn order_unit_target_relation(
    order: &OrderDef,
    target_id: u32,
    target_owner: u8,
    selection: &[u32],
    replica: &Replica,
    content: &crate::content::ClientContent,
    slot: u8,
) -> bool {
    if replica
        .units
        .get(&target_id)
        .is_none_or(|target| target.owner_slot != target_owner)
    {
        return false;
    }
    if order.behaviour != turts_mod_api::Behaviour::Attack {
        return true;
    }
    selection
        .iter()
        .filter_map(|id| replica.units.get(id))
        .filter(|unit| {
            unit.owner_slot == slot
                && (order.units.is_empty() || order.units.contains(&unit.content_id))
        })
        .any(|unit| {
            ContentId::parse(&unit.content_id)
                .ok()
                .and_then(|id| content.catalog.units.get(&id))
                .and_then(|unit| unit.weapon.as_ref())
                .is_some_and(|weapon| {
                    crate::relation::holds_own_team(weapon.targets.slots, slot, target_owner)
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        acquire::{Acquired, PresentationContent, PresentationImage},
        content::ClientContent,
        debug_feed::CursorFeed,
        ingest::{ReplicaState, WelcomeInfo},
        join::AcquiredPackages,
    };

    #[test]
    fn task16_actual_locked_cursor_preserves_window_hotspot_across_ui_and_device_scales() {
        for ui_scale in [1.0, 1.5] {
            for device_scale in [1.0, 2.0] {
                let mut world = World::new();
                world.insert_resource(bevy::ui::UiScale(ui_scale));
                let mut images = Assets::<Image>::default();
                let mut image = Image::default();
                image.texture_descriptor.size.width = 32;
                image.texture_descriptor.size.height = 32;
                let handle = images.add(image);
                world.insert_resource(images);
                update_locked_cursor(
                    &mut world,
                    true,
                    Some(Vec2::new(640.0, 360.0)),
                    Some(handle),
                    (12, 12),
                );
                let node = world
                    .query_filtered::<&Node, With<LockedCursorSprite>>()
                    .single(&world)
                    .unwrap();
                for (value, expected) in [
                    (node.left, 628.0),
                    (node.top, 348.0),
                    (node.width, 32.0),
                    (node.height, 32.0),
                ] {
                    let Val::Px(logical) = value else {
                        panic!("cursor dimensions use logical pixels")
                    };
                    let physical = logical * ui_scale * device_scale;
                    assert!(
                        (physical - expected * device_scale).abs() < 0.001,
                        "cursor hotspot and size must stay in window pixels at UI scale {ui_scale}, device scale {device_scale}: {physical}"
                    );
                }
            }
        }
    }

    #[test]
    fn actual_cursor_component_and_feed_follow_each_authored_hotspot_variant() {
        let mut presentation = PresentationContent::default();
        presentation.images.insert(
            "muster:icon".into(),
            PresentationImage {
                bytes: include_bytes!("../../../../mods/muster/assets/images/cursor_move.png")
                    .to_vec(),
                nine_slice: None,
            },
        );
        presentation
            .cursor_assets
            .insert("muster:first".into(), ("muster:icon".into(), (0, 0)));
        presentation
            .cursor_assets
            .insert("muster:second".into(), ("muster:icon".into(), (3, 4)));
        presentation
            .cursors
            .insert("default".into(), ("muster:icon".into(), (0, 0)));
        let mut image_assets = Assets::<Image>::default();
        let mut font_assets = Assets::<Font>::default();
        let widget_assets = super::super::assets::load_presentation_assets(
            &presentation,
            &mut image_assets,
            &mut font_assets,
            None,
        )
        .unwrap();

        let content = ClientContent::default();
        let acquired = AcquiredPackages(Acquired {
            lockfile_hash: "test".into(),
            visuals: Default::default(),
            map_size: (1.0, 1.0),
            unit_names: Default::default(),
            content: content.clone(),
            presentation,
            verified_root_mod: None,
        });
        let mut world = World::new();
        world.insert_resource(acquired);
        world.insert_resource(ReplicaState {
            replica: Some(Replica::new(20)),
            welcome: Some(WelcomeInfo {
                slot: 0,
                tick: 0,
                tick_hz: 20,
                match_id: 0,
            }),
            content: Some(content),
            ..Default::default()
        });
        world.insert_resource(Presented::default());
        world.insert_resource(widget_assets);
        world.insert_resource(image_assets);
        world.insert_resource(CursorFeed::default());
        let window = world.spawn((PrimaryWindow, Window::default())).id();

        update_cursor(&mut world);
        let first = world.get::<CursorIcon>(window).unwrap().clone();
        let first_feed = world.resource::<CursorFeed>();
        assert_eq!(first_feed.context.as_deref(), Some("default"));
        assert_eq!(first_feed.applied_icon.as_deref(), Some("icon"));
        assert_eq!(first_feed.position, None);
        assert_eq!(
            custom_cursor(&first),
            (
                world
                    .resource::<super::super::assets::WidgetAssets>()
                    .cursor_image_for("muster:icon", (0, 0))
                    .unwrap()
                    .id(),
                (0, 0),
            )
        );

        world
            .resource_mut::<AcquiredPackages>()
            .0
            .presentation
            .cursors
            .insert("default".into(), ("muster:icon".into(), (3, 4)));
        update_cursor(&mut world);
        let second = world.get::<CursorIcon>(window).unwrap().clone();
        assert_ne!(
            first, second,
            "the applied window component changes when only the authored hotspot changes"
        );
        let second_feed = world.resource::<CursorFeed>();
        assert_eq!(second_feed.context.as_deref(), Some("default"));
        assert_eq!(second_feed.applied_icon.as_deref(), Some("icon"));
        assert_eq!(second_feed.position, None);
        assert_eq!(
            custom_cursor(&second),
            (
                world
                    .resource::<super::super::assets::WidgetAssets>()
                    .cursor_image_for("muster:icon", (3, 4))
                    .unwrap()
                    .id(),
                (3, 4),
            )
        );

        {
            let mut accepted = world.resource_mut::<AcquiredPackages>();
            accepted.0.presentation.cursors.clear();
            accepted.0.presentation.cursor_assets.clear();
        }
        update_cursor(&mut world);
        assert!(
            world.get::<CursorIcon>(window).is_none(),
            "a valid replacement with no cursor mappings clears the previously applied OS cursor"
        );
        let feed = world.resource::<CursorFeed>();
        assert_eq!(feed.context.as_deref(), Some("default"));
        assert_eq!(feed.image_id, None);
        assert_eq!(feed.applied_icon, None);

        world
            .get_mut::<Window>(window)
            .expect("primary window remains available")
            .set_cursor_position(Some(Vec2::new(123.0, 234.0)));
        update_cursor(&mut world);
        assert_eq!(
            world.resource::<CursorFeed>().position,
            Some([123.0, 234.0])
        );

        world
            .get_mut::<Window>(window)
            .expect("primary window remains available")
            .set_cursor_position(None);
        update_cursor(&mut world);
        assert_eq!(world.resource::<CursorFeed>().position, None);
    }

    #[test]
    fn hovered_node_gather_context_requires_the_same_selected_actor_to_accept_it() {
        let ore = ContentId::parse("test:ore").unwrap();
        let stone = ContentId::parse("test:stone").unwrap();
        let accepted_worker = ContentId::parse("test:ore_worker").unwrap();
        let incompatible_worker = ContentId::parse("test:stone_worker").unwrap();
        let node_id = ContentId::parse("test:ore_node").unwrap();
        let gather_order = ContentId::parse("test:gather_ore").unwrap();

        let mut content = ClientContent::default();
        content.catalog.nodes.insert(
            node_id.clone(),
            turts_mod_api::ResolvedNode {
                id: node_id.clone(),
                name: "Ore node".into(),
                item: ore.clone(),
                radius: 1.0,
            },
        );
        content.catalog.units.insert(
            accepted_worker.clone(),
            resolved_gatherer(accepted_worker.clone(), ore),
        );
        content.catalog.units.insert(
            incompatible_worker.clone(),
            resolved_gatherer(incompatible_worker.clone(), stone),
        );
        content.catalog.orders.insert(
            gather_order.clone(),
            OrderDef {
                id: gather_order.to_string(),
                name: "Gather ore".into(),
                behaviour: turts_mod_api::Behaviour::Gather,
                target: TargetKind::Node,
                fire: None,
                engage: None,
                icon: String::new(),
                hotkey: String::new(),
                cursor: None,
                card: None,
                // Only the worker that does not accept the hovered ore is eligible for this order.
                units: vec![incompatible_worker.to_string()],
                description: String::new(),
            },
        );
        content.order_rules.smart.push(turts_mod_api::SmartOrder {
            target: turts_mod_api::SmartTarget::ResourceNodeAccepted,
            needs: turts_mod_api::Capability::Gatherer,
            order: gather_order.to_string(),
        });

        let mut replica = Replica::new(20);
        replica.units.insert(
            1,
            crate::replica::UnitReplica {
                content_id: incompatible_worker.to_string(),
                owner_slot: 0,
                facing: None,
                samples: Default::default(),
            },
        );
        replica.units.insert(
            2,
            crate::replica::UnitReplica {
                content_id: accepted_worker.to_string(),
                owner_slot: 0,
                facing: None,
                samples: Default::default(),
            },
        );
        replica.nodes.insert(
            turts_net_protocol::NodeId(9),
            turts_net_protocol::NodeState {
                id: turts_net_protocol::NodeId(9),
                content_id: node_id.to_string(),
                pos: turts_net_protocol::Pos { x: 4.0, y: 5.0 },
                remaining: 10,
            },
        );

        assert_eq!(
            hover_context(Hover::Node { id: 9 }, &[1, 2], &replica, &content, 0,),
            "forbidden",
            "the accepting second actor cannot lend its node capability to the first actor's order"
        );
    }

    #[test]
    fn hover_cursor_respects_authored_smart_rule_precedence_across_selected_actors() {
        use turts_mod_api::{Behaviour, Capability, SmartTarget};

        let ore = ContentId::parse("test:ore").unwrap();
        let node_id = ContentId::parse("test:ore_node").unwrap();
        let gatherer_id = ContentId::parse("test:gatherer").unwrap();
        let mover_id = ContentId::parse("test:mover").unwrap();
        let move_id = ContentId::parse("test:move").unwrap();
        let gather_id = ContentId::parse("test:gather").unwrap();
        let mut content = ClientContent::default();
        content.catalog.nodes.insert(
            node_id.clone(),
            turts_mod_api::ResolvedNode {
                id: node_id.clone(),
                name: "Ore node".into(),
                item: ore.clone(),
                radius: 1.0,
            },
        );
        content.catalog.units.insert(
            gatherer_id.clone(),
            resolved_gatherer(gatherer_id.clone(), ore.clone()),
        );
        let mut mover = resolved_gatherer(mover_id.clone(), ore);
        mover.gatherer = None;
        mover.movement = Some(turts_mod_api::MovementDef { speed: 1.0 });
        content.catalog.units.insert(mover_id.clone(), mover);
        for (id, name, behaviour, target, units) in [
            (
                &move_id,
                "Move",
                Behaviour::Move,
                TargetKind::PointOrUnit,
                vec![mover_id.to_string()],
            ),
            (
                &gather_id,
                "Gather",
                Behaviour::Gather,
                TargetKind::Node,
                vec![gatherer_id.to_string()],
            ),
        ] {
            content.catalog.orders.insert(
                id.clone(),
                OrderDef {
                    id: id.to_string(),
                    name: name.into(),
                    behaviour,
                    target,
                    fire: None,
                    engage: None,
                    icon: String::new(),
                    hotkey: String::new(),
                    cursor: None,
                    card: None,
                    units,
                    description: String::new(),
                },
            );
        }
        content.order_rules.smart = vec![
            turts_mod_api::SmartOrder {
                target: SmartTarget::Node,
                needs: Capability::Movement,
                order: move_id.to_string(),
            },
            turts_mod_api::SmartOrder {
                target: SmartTarget::ResourceNodeAccepted,
                needs: Capability::Gatherer,
                order: gather_id.to_string(),
            },
        ];
        let mut replica = Replica::new(20);
        replica.units.insert(
            1,
            crate::replica::UnitReplica {
                content_id: gatherer_id.to_string(),
                owner_slot: 0,
                facing: None,
                samples: Default::default(),
            },
        );
        replica.units.insert(
            2,
            crate::replica::UnitReplica {
                content_id: mover_id.to_string(),
                owner_slot: 0,
                facing: None,
                samples: Default::default(),
            },
        );
        replica.nodes.insert(
            turts_net_protocol::NodeId(9),
            turts_net_protocol::NodeState {
                id: turts_net_protocol::NodeId(9),
                content_id: node_id.to_string(),
                pos: turts_net_protocol::Pos { x: 4.0, y: 5.0 },
                remaining: 10,
            },
        );

        assert_eq!(
            hover_context(Hover::Node { id: 9 }, &[1, 2], &replica, &content, 0),
            "move",
            "the first compatible authored smart rule wins across the whole selection"
        );
    }

    fn resolved_gatherer(id: ContentId, item: ContentId) -> turts_mod_api::ResolvedUnit {
        turts_mod_api::ResolvedUnit {
            id,
            name: "Worker".into(),
            footprint: turts_mod_api::Footprint::Circle { radius: 1.0 },
            movement: None,
            inventory: None,
            gatherer: Some(turts_mod_api::ResolvedGatherer {
                items: vec![item],
                gather_per_second: 1,
                unload_per_second: 1,
                interaction_range: 1.0,
            }),
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

    fn custom_cursor(icon: &CursorIcon) -> (bevy::asset::AssetId<Image>, (u16, u16)) {
        match icon {
            CursorIcon::Custom(CustomCursor::Image(image)) => (image.handle.id(), image.hotspot),
            other => panic!("expected applied custom cursor, found {other:?}"),
        }
    }
}
