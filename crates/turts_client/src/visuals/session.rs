//! The visual session: the identity of the current match and connection.

use super::*;

/// Reset the retained visuals when the session changes, and clear the intent feed when there is none.
pub(super) fn sync_session(
    state: &ReplicaState,
    receipts: &ReceiptWitnesses,
    visuals: &mut VisualState,
    marker_feed: &mut OrderMarkerFeed,
    commands: &mut Commands,
    fills: &mut Query<(Entity, &HealthFill, &mut Transform), Without<VisualId>>,
) {
    let visual_session = state
        .welcome
        .map(|welcome| VisualSession {
            match_id: welcome.match_id,
            connection_epoch: receipts.next_epoch,
            local_slot: welcome.slot,
        })
        .filter(|_| state.replica.is_some() && state.content.is_some());
    let session_changed = reset_marker_session(visuals, marker_feed, visual_session);
    if session_changed {
        for (entity, _, _) in fills.iter_mut() {
            commands.entity(entity).despawn();
        }
    }
    if visual_session.is_none() {
        marker_feed.intents.clear();
        marker_feed.sent.clear();
    }
}
