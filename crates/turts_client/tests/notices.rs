mod support;

use std::collections::BTreeMap;
use support::feedback::{alert_state, feedback_app, reason_overrides};
use turts_client::{
    debug_feed::NoticesFeed,
    feedback::{format_notice, reason_text},
    ingest::{CommandResult, CommandResults, ReplicaState},
};
use turts_net_protocol::{CommandRejectReason, UnitId};

#[test]
fn no_orders_notice_shows_once_for_a_joined_package_without_orders() {
    let (state, acquired) = alert_state();
    let mut app = feedback_app(state, acquired);
    app.update();
    app.update();

    let notices = &app.world().resource::<NoticesFeed>().0;
    let matches = notices
        .iter()
        .filter(|notice| {
            notice["text"] == "This game defines no orders"
                || notice["message"] == "This game defines no orders"
        })
        .count();
    assert_eq!(
        matches, 1,
        "an empty order catalog produces one notice after join"
    );
}

#[test]
fn no_orders_notice_waits_for_join_and_works_without_an_authored_interface() {
    let (mut state, mut acquired) = alert_state();
    state.welcome = None;
    acquired.0.presentation.interface = None;
    let mut app = feedback_app(state, acquired);
    app.update();
    assert!(
        app.world().resource::<NoticesFeed>().0.is_empty(),
        "no package notice appears before welcome"
    );

    app.world_mut().resource_mut::<ReplicaState>().welcome =
        Some(turts_client::ingest::WelcomeInfo {
            slot: 0,
            tick: 1,
            tick_hz: 20,
            match_id: 1,
        });
    app.update();
    assert!(
        app.world().resource::<NoticesFeed>().0.is_empty(),
        "Welcome alone does not count as actual joined presentation"
    );
    *app.world_mut().resource_mut::<turts_client::join::Phase>() =
        turts_client::join::Phase::Joined { slot: 0 };
    app.update();
    let matches = app
        .world()
        .resource::<NoticesFeed>()
        .0
        .iter()
        .filter(|notice| notice["text"] == "This game defines no orders")
        .count();
    assert_eq!(
        matches, 1,
        "the joined package gets one engine notice even with no interface"
    );
}

#[test]
fn every_engine_reason_has_player_text_without_the_wire_code() {
    let overrides = reason_overrides();
    for reason in CommandRejectReason::ALL {
        let text = reason_text(*reason, &overrides);
        assert!(
            !text.trim().is_empty(),
            "{} has player-facing text",
            reason.as_str()
        );
        assert!(
            !text.contains(reason.as_str()),
            "{} is not shown as a raw code",
            reason.as_str()
        );
    }
    assert_eq!(
        reason_text(CommandRejectReason::NotOwner, &overrides),
        "That unit belongs to the other side."
    );
    assert_eq!(
        reason_text(CommandRejectReason::NotApplicable, &overrides),
        "That unit cannot do this."
    );
    assert_eq!(
        reason_text(CommandRejectReason::RateLimited, &overrides),
        "Too many orders"
    );
    assert_eq!(
        reason_text(CommandRejectReason::OutOfRange, &overrides),
        "That target is out of range."
    );
    assert_eq!(
        reason_text(CommandRejectReason::UnknownRecipe, &BTreeMap::new()),
        "Recipe is unavailable"
    );
}

#[test]
fn local_refusal_names_the_unit_and_reason_text() {
    let notice = format_notice("Shell Turret", "Cannot move", 1);
    assert_eq!(notice, "Shell Turret: Cannot move");
    assert!(!notice.contains("no_applicable_order"));

    let grouped = format_notice("Shell Turret", "Cannot move", 2);
    assert!(
        grouped.contains("Shell Turret"),
        "the grouped notice keeps the unit name"
    );
    assert!(
        grouped.contains('2'),
        "the grouped notice keeps the refused-unit count"
    );
    assert!(grouped.contains("Cannot move"));
}

#[test]
fn a_per_unit_refusal_uses_its_authored_error_response_sound() {
    let (state, acquired) = alert_state();
    let mut app = feedback_app(state, acquired);
    app.update();
    app.world_mut()
        .resource_mut::<CommandResults>()
        .0
        .push(CommandResult::Order {
            seq: 1,
            tick: 2,
            accepted: Vec::new(),
            refused: vec![turts_net_protocol::UnitRefusal {
                unit: UnitId(1),
                reason: CommandRejectReason::NotApplicable,
            }],
            envelope: None,
            job_id: None,
        });
    app.update();

    assert!(
        app.world()
            .resource::<turts_client::feedback::FrameFeedback>()
            .audio_started
            .iter()
            .any(|sound| {
                sound.id == "muster:muster_effect_error" && sound.category == "effects"
            }),
        "refusal feedback resolves the unit's authored error response to its real package sound id"
    );
}

#[test]
fn accepted_orders_are_silent_but_partial_and_envelope_refusals_play_the_error_sound() {
    let (state, acquired) = alert_state();
    let mut app = feedback_app(state, acquired);
    app.update();
    app.world_mut()
        .resource_mut::<CommandResults>()
        .0
        .push(CommandResult::Order {
            seq: 1,
            tick: 2,
            accepted: vec![UnitId(1)],
            refused: Vec::new(),
            envelope: None,
            job_id: None,
        });
    app.update();
    assert!(
        app.world()
            .resource::<turts_client::feedback::FrameFeedback>()
            .audio_started
            .iter()
            .all(|sound| sound.category != "effects"),
        "an accepted order does not use the error response"
    );

    app.world_mut()
        .resource_mut::<CommandResults>()
        .0
        .push(CommandResult::Order {
            seq: 2,
            tick: 3,
            accepted: vec![UnitId(1)],
            refused: vec![turts_net_protocol::UnitRefusal {
                unit: UnitId(2),
                reason: CommandRejectReason::NotOwner,
            }],
            envelope: None,
            job_id: None,
        });
    app.update();
    assert!(
        app.world()
            .resource::<turts_client::feedback::FrameFeedback>()
            .audio_started
            .iter()
            .any(|sound| sound.category == "effects"),
        "a partial refusal plays one authored error sound"
    );

    app.world_mut()
        .resource_mut::<CommandResults>()
        .0
        .push(CommandResult::Order {
            seq: 3,
            tick: 4,
            accepted: Vec::new(),
            refused: Vec::new(),
            envelope: Some(CommandRejectReason::TooManyUnits),
            job_id: None,
        });
    app.update();
    assert!(
        app.world()
            .resource::<turts_client::feedback::FrameFeedback>()
            .audio_started
            .iter()
            .any(|sound| sound.category == "effects"),
        "an envelope rejection plays one authored error sound"
    );
}

#[test]
fn a_new_replicated_last_failure_is_noticed_and_sounded_once() {
    use turts_net_protocol::{Activity, OrderFailure, UnitOrderRecord};
    let (state, acquired) = alert_state();
    let mut app = feedback_app(state, acquired);
    app.update();
    app.world_mut().resource_mut::<CommandResults>().0.clear();
    app.world_mut()
        .resource_mut::<ReplicaState>()
        .replica
        .as_mut()
        .unwrap()
        .unit_orders
        .insert(
            1,
            UnitOrderRecord {
                unit: UnitId(1),
                activity: Activity::Idle,
                current: None,
                queued: Vec::new(),
                last_failure: Some(OrderFailure {
                    order: "muster:rush".into(),
                    reason: CommandRejectReason::OutOfRange,
                    tick: 2,
                }),
                rally: None,
            },
        );
    app.update();

    let feedback = app
        .world()
        .resource::<turts_client::feedback::FrameFeedback>();
    assert!(
        feedback
            .notices
            .iter()
            .any(|notice| notice["text"] == "Trooper: That target is out of range."),
        "a newly replicated last_failure names the unit and resolves its player reason"
    );
    assert!(
        feedback
            .audio_started
            .iter()
            .any(|sound| sound.id == "muster:muster_effect_error" && sound.category == "effects"),
        "the same failure frame uses the authored error response"
    );

    app.update();
    let notices = &app.world().resource::<NoticesFeed>().0;
    assert_eq!(
        notices
            .iter()
            .filter(|notice| notice["text"] == "Trooper: That target is out of range.")
            .count(),
        1,
        "a persistent last_failure record does not repeat its notice"
    );
}
