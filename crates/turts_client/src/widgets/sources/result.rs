//! Result widgets read the reliable server result. They never infer a winner from units.

use super::SourceContext;
use turts_mod_api::{OutcomeKind, OutcomeReasonKind};
use turts_net_protocol::{Outcome, OutcomeReason};

pub(in crate::widgets) fn value_outcome(context: SourceContext<'_>) -> serde_json::Value {
    context
        .replica
        .outcome()
        .map(|outcome| match outcome {
            Outcome::Victory => "Victory",
            Outcome::Defeat => "Defeat",
            Outcome::Draw => "Draw",
        })
        .map_or(serde_json::Value::Null, |text| serde_json::json!(text))
}

pub(in crate::widgets) fn value_outcome_reason(context: SourceContext<'_>) -> serde_json::Value {
    context
        .replica
        .outcome_reason()
        .map(|reason| match reason {
            OutcomeReason::KeyUnitsLost => "Key units lost",
            OutcomeReason::OpponentsDefeated => "Opponents defeated",
            OutcomeReason::Surrendered => "Surrendered",
            OutcomeReason::Disconnected => "Disconnected",
        })
        .map_or(serde_json::Value::Null, |text| serde_json::json!(text))
}

pub(in crate::widgets) fn value_match_clock(context: SourceContext<'_>) -> serde_json::Value {
    serde_json::json!(
        f64::from(context.replica.match_clock_ticks()) / f64::from(context.replica.tick_hz.max(1))
    )
}

pub(in crate::widgets) fn predicate_match_over(context: &SourceContext<'_>) -> bool {
    context.replica.own_outcome().is_some()
}

pub(in crate::widgets) fn predicate_outcome_is(
    kind: OutcomeKind,
    context: &SourceContext<'_>,
) -> bool {
    context.replica.outcome()
        == Some(match kind {
            OutcomeKind::Victory => Outcome::Victory,
            OutcomeKind::Defeat => Outcome::Defeat,
            OutcomeKind::Draw => Outcome::Draw,
        })
}

pub(in crate::widgets) fn predicate_outcome_reason_is(
    kind: OutcomeReasonKind,
    context: &SourceContext<'_>,
) -> bool {
    context.replica.outcome_reason()
        == Some(match kind {
            OutcomeReasonKind::KeyUnitsLost => OutcomeReason::KeyUnitsLost,
            OutcomeReasonKind::OpponentsDefeated => OutcomeReason::OpponentsDefeated,
            OutcomeReasonKind::Surrendered => OutcomeReason::Surrendered,
            OutcomeReasonKind::Disconnected => OutcomeReason::Disconnected,
        })
}
