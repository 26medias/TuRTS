//! Input.
//!
//! A left click on the ground becomes a move order. `click_to_move` runs in `Update` and only
//! queues a `PlayerCommandKind` in the shared `PendingCommands` FIFO. `send_commands` assigns its sequence and sends it
//! (design 4.2: input in `Update`, command send in `PostUpdate`). Input reads the replica to
//! find the units of this player and never writes it.

use crate::commands::{CommandSendStatus, PendingCommands, submit};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use turts_net_protocol::{PlayerCommandKind, Pos, UnitId};

use crate::ingest::ReplicaState;
use crate::join::Phase;

/// The map position where a ray meets the ground plane, or `None` when it does not meet it in
/// front of the origin. Map `x` is scene `x`, map `y` is scene `z` (design 4.4).
pub fn ground_point(ray: Ray3d) -> Option<Pos> {
    let distance = ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Dir3::Y))?;
    let point = ray.get_point(distance);
    Some(Pos {
        x: point.x,
        y: point.z,
    })
}

/// The ids of the units that this player owns, ascending.
pub fn owned_unit_ids(state: &ReplicaState) -> Vec<UnitId> {
    let (Some(welcome), Some(replica)) = (state.welcome, state.replica.as_ref()) else {
        return Vec::new();
    };
    let mut ids: Vec<UnitId> = replica
        .units
        .iter()
        .filter(|(_, unit)| unit.owner_slot == welcome.slot)
        .map(|(&id, _)| UnitId(id))
        .collect();
    ids.sort_by_key(|id| id.0);
    ids
}

/// `Update`: on a left click in `Phase::Joined`, find the ground point under the cursor and queue
/// it as a target.
pub fn click_to_move(
    buttons: Res<ButtonInput<MouseButton>>,
    phase: Res<Phase>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut pending: ResMut<PendingCommands>,
    status: Res<CommandSendStatus>,
) {
    if status.reconnect_required
        || !buttons.just_pressed(MouseButton::Left)
        || !matches!(*phase, Phase::Joined { .. })
    {
        return;
    }
    let Some(cursor) = windows.iter().find_map(Window::cursor_position) else {
        return;
    };
    let Some((camera, transform)) = cameras.iter().next() else {
        return;
    };
    let Ok(ray) = camera.viewport_to_world(transform, cursor) else {
        return;
    };
    if let Some(target) = ground_point(ray) {
        submit(
            &mut pending,
            PlayerCommandKind::MoveTo {
                unit_ids: Vec::new(),
                target,
            },
        );
    }
}

/// The window path of input: clicks become orders.
pub struct ClickInputPlugin;

impl Plugin for ClickInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, click_to_move);
    }
}
