//! The order markers: the click intents that the player sent and their lifetime.

use super::*;
use crate::replica::Replica;

/// Take the new intents, the sent fragments and the results into the retained markers.
pub(super) fn intake_markers(
    marker_feed: &mut OrderMarkerFeed,
    visuals: &mut VisualState,
    replica: &Replica,
    results: &CommandResults,
    now: Instant,
) {
    for intent in marker_feed.intents.drain(..) {
        if let Some(marker) = retained_marker(intent, replica, now) {
            visuals.markers.push_back(marker);
        }
    }
    for fragment in marker_feed.sent.drain(..) {
        if let Some(marker) = visuals
            .markers
            .iter_mut()
            .find(|marker| marker.id == fragment.intent_id)
        {
            marker.fragments.push(fragment);
        }
    }
    for marker in &mut visuals.markers {
        for fragment in &marker.fragments {
            if marker.seen_results.contains(&fragment.seq) {
                continue;
            }
            if let Some(result) = results.0.iter().find_map(|result| match result {
                CommandResult::Order {
                    seq,
                    tick,
                    accepted,
                    ..
                } if *seq == fragment.seq => Some(MarkerOrderResult {
                    tick: *tick,
                    accepted: accepted.iter().map(|unit| unit.0).collect(),
                }),
                _ => None,
            }) {
                marker.seen_results.insert(fragment.seq);
                marker.results.insert(fragment.seq, result);
            }
        }
    }
    let replica_tick = replica.last_applied_tick.unwrap_or(0);
    visuals
        .markers
        .retain(|marker| marker_is_live(marker, replica, replica_tick, now));
}

/// Draw each retained marker and list it in `markers_shown`.
pub(super) fn draw_order_markers(
    desired: &mut DesiredVisuals,
    markers_shown: &mut MarkersShown,
    marker_gizmos: &mut Gizmos<OrderMarkerGizmoConfigGroup>,
    assets: &VisualAssets,
    visuals: &VisualState,
) {
    for marker in &visuals.markers {
        let material = assets.marker_overlay_material.clone();
        if let Some(material) = material {
            let kind = marker_kind(&marker.kind);
            let position = Vec3::new(marker.position.x, 0.035, marker.position.y);
            let id = VisualId {
                kind: VisualKind::OrderMarker,
                unit: 0,
                ordinal: 0,
                generation: visuals.generation,
                identity: marker.id,
            };
            match kind {
                "move" => {
                    if let Some(mesh) = assets.ring.clone() {
                        desired.insert(
                            id,
                            (
                                mesh,
                                VisualMaterial::RingOverlay(material.clone()),
                                Transform {
                                    translation: position,
                                    scale: Vec3::splat(0.38),
                                    ..default()
                                },
                            ),
                        );
                    }
                }
                "attack" => {
                    marker_gizmos.line(
                        position + Vec3::new(-0.3, 0.04, -0.3),
                        position + Vec3::new(0.3, 0.04, 0.3),
                        Color::srgb(0.96, 0.88, 0.2),
                    );
                    marker_gizmos.line(
                        position + Vec3::new(-0.3, 0.04, 0.3),
                        position + Vec3::new(0.3, 0.04, -0.3),
                        Color::srgb(0.96, 0.88, 0.2),
                    );
                }
                "gather" => {
                    if let Some(mesh) = assets.bar.clone() {
                        desired.insert(
                            id,
                            (
                                mesh,
                                VisualMaterial::RingOverlay(material.clone()),
                                Transform {
                                    translation: position,
                                    scale: Vec3::splat(0.16),
                                    ..default()
                                },
                            ),
                        );
                    }
                }
                _ => {}
            }
            markers_shown.0.push(serde_json::json!({
                "kind": kind,
                "intent_id": marker.id,
                "order": &marker.order.order,
                "units": marker.order.units.iter().map(|unit| unit.0).collect::<Vec<_>>(),
                "x": marker.position.x,
                "y": marker.position.y
            }));
        }
    }
}
