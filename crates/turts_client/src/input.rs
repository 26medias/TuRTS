//! Input.
//!
//! A left click on the ground becomes a move order. `click_to_move` runs in `Update` and only
//! queues the target in [`PendingMoves`]. `send_moves` runs in `PostUpdate` and sends the orders
//! (design 4.2: input in `Update`, command send in `PostUpdate`). Input reads the replica to
//! find the units of this player and never writes it.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use lightyear::prelude::*;
use turts_net_lightyear::ReliableChannel;
use turts_net_protocol::messages::MoveTo;
use turts_net_protocol::{Pos, UnitId};

use crate::ingest::ReplicaState;
use crate::join::Phase;

/// The `seq` of the first order from a click. The `--move` test hook of `join::auto_move` uses
/// `seq` 1.
const FIRST_CLICK_SEQ: u32 = 2;

/// Targets that the player has chosen and that `send_moves` has not sent.
#[derive(Resource, Default)]
pub struct PendingMoves(pub Vec<Pos>);

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
    mut pending: ResMut<PendingMoves>,
) {
    if !buttons.just_pressed(MouseButton::Left) || !matches!(*phase, Phase::Joined { .. }) {
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
        pending.0.push(target);
    }
}

/// `PostUpdate`: send each queued target as a `MoveTo` for all units of this player. The
/// server validates it; a rejected order comes back as `CommandRejected`.
pub fn send_moves(
    mut pending: ResMut<PendingMoves>,
    state: Res<ReplicaState>,
    mut sender: Query<&mut MessageSender<MoveTo>>,
    mut next_seq: Local<u32>,
) {
    if pending.0.is_empty() {
        return;
    }
    let unit_ids = owned_unit_ids(&state);
    for target in pending.0.drain(..) {
        if unit_ids.is_empty() {
            continue;
        }
        let seq = FIRST_CLICK_SEQ + *next_seq;
        *next_seq += 1;
        for mut sender in sender.iter_mut() {
            sender.send::<ReliableChannel>(MoveTo {
                seq,
                unit_ids: unit_ids.clone(),
                target,
            });
        }
    }
}

/// The window path of input: clicks become orders.
pub struct ClickInputPlugin;

impl Plugin for ClickInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingMoves>()
            .add_systems(Update, click_to_move)
            .add_systems(PostUpdate, send_moves.before(MessageSystems::Send));
    }
}
