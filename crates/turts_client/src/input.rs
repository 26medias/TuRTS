//! Shared world input: left selects owned units, right orders the current selected mobile.

use crate::{
    commands::{CommandSendStatus, PendingCommands, submit},
    economy_ui::{Selection, UiPanelRoot, cursor_in_ui_viewport, hit_test_node, hit_test_unit},
    ingest::ReplicaState,
    join::{AcquiredPackages, Phase},
};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use turts_net_protocol::{PlayerCommandKind, Pos, UnitId};

/// Map ground-plane ray intersection: scene x/z map to protocol x/y.
pub fn ground_point(ray: Ray3d) -> Option<Pos> {
    let distance = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Dir3::Y))?;
    let point = ray.get_point(distance);
    Some(Pos {
        x: point.x,
        y: point.z,
    })
}

#[allow(clippy::too_many_arguments)]
pub fn world_pointer_commands(
    buttons: Res<ButtonInput<MouseButton>>,
    phase: Res<Phase>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(Entity, &Camera, &GlobalTransform)>,
    panel_roots: Query<
        (
            &ComputedNode,
            &UiGlobalTransform,
            &InheritedVisibility,
            &ComputedUiTargetCamera,
        ),
        With<UiPanelRoot>,
    >,
    state: Res<ReplicaState>,
    acquired: Option<Res<AcquiredPackages>>,
    mut selection: ResMut<Selection>,
    mut pending: ResMut<PendingCommands>,
    status: Res<CommandSendStatus>,
) {
    let left = buttons.just_pressed(MouseButton::Left);
    let right = buttons.just_pressed(MouseButton::Right);
    if status.reconnect_required || (!left && !right) || !matches!(*phase, Phase::Joined { .. }) {
        return;
    }
    let Some(cursor) = windows.iter().find_map(Window::cursor_position) else {
        return;
    };
    for (node, transform, visible, target_camera) in &panel_roots {
        if !visible.get() {
            continue;
        }
        let Some(camera_id) = target_camera.get() else {
            return;
        };
        let Some((_, camera, _)) = cameras.iter().find(|(entity, _, _)| *entity == camera_id)
        else {
            return;
        };
        let Some(ui_cursor) = cursor_in_ui_viewport(cursor, camera) else {
            continue;
        };
        // On the first frame, layout may not have run yet. Consume the gesture
        // conservatively instead of letting a panel click fall through to the map.
        if node.size().min_element() <= 0.0 || node.contains_point(*transform, ui_cursor) {
            return;
        }
    }
    let (Some(welcome), Some(replica), Some(acquired)) =
        (state.welcome, state.replica.as_ref(), acquired.as_ref())
    else {
        return;
    };
    let Some((_, camera, transform)) = cameras.iter().next() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(transform, cursor) else {
        return;
    };
    let Some(point) = ground_point(ray) else {
        return;
    };

    if left {
        selection.0 = hit_test_unit(replica, &acquired.0.content, point, welcome.slot);
        return;
    }
    let Some(selected) = selection.0 else {
        return;
    };
    let Some(unit) = replica
        .units
        .get(&selected)
        .filter(|u| u.owner_slot == welcome.slot)
    else {
        selection.0 = None;
        return;
    };
    let Some(unit_def) = turts_mod_api::ContentId::parse(&unit.content_id)
        .ok()
        .and_then(|id| acquired.0.content.catalog.units.get(&id))
    else {
        return;
    };

    if let Some(node_id) = hit_test_node(replica, &acquired.0.content, point) {
        let Some(gatherer) = unit_def.gatherer.as_ref() else {
            return;
        };
        let Some(node) = replica.nodes.get(&node_id) else {
            return;
        };
        let Some(node_def) = turts_mod_api::ContentId::parse(&node.content_id)
            .ok()
            .and_then(|id| acquired.0.content.catalog.nodes.get(&id))
        else {
            return;
        };
        if gatherer.items.contains(&node_def.item) {
            submit(
                &mut pending,
                PlayerCommandKind::Gather {
                    unit_id: UnitId(selected),
                    node_id,
                },
            );
        }
        return;
    }
    if unit_def.movement.is_some() {
        submit(
            &mut pending,
            PlayerCommandKind::MoveTo {
                unit_ids: vec![UnitId(selected)],
                target: point,
            },
        );
    }
}

pub struct ClickInputPlugin;
impl Plugin for ClickInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            world_pointer_commands.after(crate::economy_ui::sync_panel),
        );
    }
}
