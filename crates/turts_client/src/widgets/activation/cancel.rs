//! The activation of `WidgetAction::CancelProduction` (the arm of `activate_widget`, moved here with
//! its body unchanged by L0-CORE). `UI-HUD` extends it: the right-click cancel of a queue row and
//! of a queue entry (design 7.4, open point 16).

use super::*;

pub(super) fn activate(
    producer: &u32,
    job_id: &u64,
    queue: &mut CoreInputQueue,
    state: &ReplicaState,
    acquired: Option<&AcquiredPackages>,
) {
    let Some(acquired) = acquired.as_ref() else {
        return;
    };
    let Some(slot) = state.welcome.map(|welcome| welcome.slot) else {
        return;
    };
    let Some(replica) = state.replica.as_ref() else {
        return;
    };
    if let Some(unit) = replica
        .units
        .get(producer)
        .filter(|unit| unit.owner_slot == slot)
        .filter(|_| {
            replica.economy.queues.iter().any(|queue| {
                queue.producer.0 == *producer && queue.jobs.iter().any(|job| job.id.0 == *job_id)
            })
        })
    {
        let Some(order) = acquired
            .0
            .content
            .catalog
            .orders
            .iter()
            .find(|(_, order)| {
                order.behaviour == turts_mod_api::Behaviour::CancelProduction
                    && order.target == turts_mod_api::TargetKind::Job
                    && (order.units.is_empty()
                        || order
                            .units
                            .iter()
                            .any(|allowed| allowed == &unit.content_id))
            })
            .map(|(id, _)| id.to_string())
        else {
            return;
        };
        queue.0.push(turts_interaction::InputEvent::UiOrder {
            order,
            units: vec![turts_net_protocol::UnitId(*producer)],
            target: turts_interaction::UiOrderTarget::Job(turts_net_protocol::JobId(*job_id)),
        });
    }
}
