//! Order markers and blocked marks: the retained marker of a sent order, its life, and its
//! reset at a new session.

use super::*;

pub(super) fn retained_marker(
    intent: OrderMarkerIntent,
    replica: &crate::replica::Replica,
    now: Instant,
) -> Option<RetainedMarker> {
    let position = match &intent.order.target {
        OrderTarget::Point(Pos { x, y }) => Vec2::new(*x, *y),
        OrderTarget::Unit(unit) => replica
            .units
            .get(&unit.0)?
            .samples
            .back()
            .map(|(_, pos)| Vec2::new(pos.x, pos.y))?,
        OrderTarget::Node(node) => {
            let pos = replica.nodes.get(node)?.pos;
            Vec2::new(pos.x, pos.y)
        }
        OrderTarget::ContentAtPoint {
            point: Pos { x, y },
            ..
        } => Vec2::new(*x, *y),
        OrderTarget::None | OrderTarget::Job(_) | OrderTarget::Content(_) => return None,
    };
    Some(RetainedMarker {
        id: intent.id,
        position,
        kind: intent.kind,
        order: intent.order,
        fragments: Vec::new(),
        results: HashMap::new(),
        seen_results: HashSet::new(),
        started: now,
    })
}

pub(super) fn marker_is_live(
    marker: &RetainedMarker,
    replica: &crate::replica::Replica,
    replica_tick: u32,
    now: Instant,
) -> bool {
    marker_is_live_for_orders(marker, &replica.unit_orders, replica_tick, now)
}

pub(super) fn marker_is_live_for_orders(
    marker: &RetainedMarker,
    unit_orders: &HashMap<u32, UnitOrderRecord>,
    replica_tick: u32,
    now: Instant,
) -> bool {
    if now.duration_since(marker.started).as_secs_f32() >= 60.0 {
        return false;
    }
    if marker.fragments.is_empty()
        || marker
            .fragments
            .iter()
            .any(|fragment| !marker.results.contains_key(&fragment.seq))
    {
        return true;
    }

    let accepted: Vec<_> = marker
        .fragments
        .iter()
        .flat_map(|fragment| {
            marker.results[&fragment.seq]
                .accepted
                .iter()
                .map(move |unit| (*unit, marker.results[&fragment.seq].tick))
        })
        .collect();
    if accepted.is_empty() {
        return false;
    }
    !accepted.into_iter().all(|(unit, result_tick)| {
        replica_tick >= result_tick
            && unit_orders
                .get(&unit)
                .is_some_and(|record| matches!(record.activity, Activity::Idle | Activity::Blocked))
    })
}

pub(super) fn refresh_blocked_marks(
    blocked_now: HashSet<u32>,
    blocked_active: &mut HashSet<u32>,
    blocked_since: &mut HashMap<u32, Instant>,
    now: Instant,
) {
    for unit in &blocked_now {
        if !blocked_active.contains(unit) {
            blocked_since.insert(*unit, now);
        }
    }
    blocked_since.retain(|unit, start| {
        blocked_now.contains(unit) || now.duration_since(*start).as_secs_f32() < 2.0
    });
    *blocked_active = blocked_now;
}

pub(super) fn reset_marker_session(
    visuals: &mut VisualState,
    marker_feed: &mut OrderMarkerFeed,
    session: Option<VisualSession>,
) -> bool {
    if visuals.session != session {
        visuals.markers.clear();
        visuals.hover_unit = None;
        visuals.hover_started = None;
        visuals.blocked_since.clear();
        visuals.blocked_active.clear();
        if session.is_none() {
            marker_feed.intents.clear();
            marker_feed.sent.clear();
        } else {
            // CoreAdapter clears intents at the beginning of every frame, before
            // any early return. Anything left here therefore belongs to the new
            // session's current input frame, including a click before its ACK.
            let current_ids: HashSet<_> =
                marker_feed.intents.iter().map(|intent| intent.id).collect();
            marker_feed
                .sent
                .retain(|fragment| current_ids.contains(&fragment.intent_id));
        }
        visuals.generation = visuals.generation.wrapping_add(1);
        visuals.session = session;
        visuals.active = false;
        true
    } else {
        false
    }
}

pub(super) fn marker_kind(kind: &str) -> &str {
    match kind {
        "attack" => "attack",
        "gather" => "gather",
        _ => "move",
    }
}
