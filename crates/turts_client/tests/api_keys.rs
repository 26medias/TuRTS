//! L0-API1: the key lists of the mod API (`turts_mod_api::keys`) against the wire types that
//! they name. The mod API crate cannot depend on the protocol crate, so this test is in a crate
//! that depends on both. A list that drifts from the wire enums is red here.

use std::collections::BTreeSet;
use turts_mod_api::keys::{REASON_KEYS, STATE_KEYS};
use turts_net_protocol::economy::{GatherStage, GatherWaitReason};
use turts_net_protocol::{Activity, CommandRejectReason};

/// The three keys that only the client writes (design 6.10 row H1).
const CLIENT_REASON_KEYS: [&str; 3] = [
    "no_applicable_order",
    "queue_disabled",
    "already_researching",
];
/// The wire keys that protocol 5 adds (6.1, 6.7.7); `L0-WIRE` adds the variants.
const NEW_WIRE_REASON_KEYS: [&str; 9] = [
    "site_outside_map",
    "site_out_of_area",
    "site_blocked",
    "site_near_building",
    "site_near_resource",
    "construction_queue_full",
    "match_not_started",
    "slot_defeated",
    "match_over",
];

#[test]
fn every_wire_reason_key_is_a_reason_key_in_wire_order() {
    // `ALL` holds the 37 keys of protocol 4 in their old order, then the 9 keys of protocol 5.
    // `REASON_KEYS` puts the 3 client keys between the two groups (design 6.10), so the old
    // keys match by index and the new keys match by name and order at the end of the list.
    let (old_keys, new_keys) = CommandRejectReason::ALL.split_at(37);
    for (index, reason) in old_keys.iter().enumerate() {
        assert_eq!(
            REASON_KEYS.get(index).copied(),
            Some(reason.as_str()),
            "wire key {index}"
        );
    }
    let new_names: Vec<_> = new_keys.iter().map(|reason| reason.as_str()).collect();
    assert_eq!(
        new_names, NEW_WIRE_REASON_KEYS,
        "the protocol 5 keys, in order"
    );
    assert_eq!(
        &REASON_KEYS[REASON_KEYS.len() - NEW_WIRE_REASON_KEYS.len()..],
        &NEW_WIRE_REASON_KEYS[..],
        "the same 9 keys end the mod API list"
    );
}

#[test]
fn every_other_reason_key_is_a_client_key_or_a_new_wire_key() {
    let wire: BTreeSet<_> = CommandRejectReason::ALL
        .iter()
        .map(|r| r.as_str())
        .collect();
    for key in REASON_KEYS {
        let known = wire.contains(key)
            || CLIENT_REASON_KEYS.contains(key)
            || NEW_WIRE_REASON_KEYS.contains(key);
        assert!(
            known,
            "`{key}` is not a wire key, a client key or a key of protocol 5"
        );
    }
}

fn activity_key(activity: Activity) -> &'static str {
    // An exhaustive match: a new `Activity` is a compile error here, then a missing key.
    match activity {
        Activity::Idle => "activity.idle",
        Activity::Pending => "activity.pending",
        Activity::Moving => "activity.moving",
        Activity::Attacking => "activity.attacking",
        Activity::Holding => "activity.holding",
        Activity::Gathering => "activity.gathering",
        Activity::Blocked => "activity.blocked",
    }
}

fn stage_key(stage: &GatherStage) -> String {
    match stage {
        GatherStage::ToNode => "gather.to_node".into(),
        GatherStage::Gathering => "gather.gathering".into(),
        GatherStage::ToDropOff => "gather.to_drop_off".into(),
        GatherStage::Unloading => "gather.unloading".into(),
        GatherStage::Idle => "gather.idle".into(),
        GatherStage::Waiting { reason } => format!("gather.waiting.{}", reason.as_str()),
    }
}

#[test]
fn every_activity_and_gather_stage_is_a_state_key() {
    let state: BTreeSet<_> = STATE_KEYS.iter().copied().collect();
    for activity in [
        Activity::Idle,
        Activity::Pending,
        Activity::Moving,
        Activity::Attacking,
        Activity::Holding,
        Activity::Gathering,
        Activity::Blocked,
    ] {
        assert!(state.contains(activity_key(activity)), "{activity:?}");
    }
    let mut stages = vec![
        GatherStage::ToNode,
        GatherStage::Gathering,
        GatherStage::ToDropOff,
        GatherStage::Unloading,
        GatherStage::Idle,
    ];
    stages.extend(
        GatherWaitReason::ALL
            .iter()
            .map(|&reason| GatherStage::Waiting { reason }),
    );
    for stage in &stages {
        assert!(state.contains(stage_key(stage).as_str()), "{stage:?}");
    }
}

#[test]
fn every_state_key_names_a_job_state_an_activity_or_a_gather_stage() {
    let job_states = [
        "waiting_for_funds",
        "slowed",
        "disabled",
        "output_blocked",
        "held",
    ];
    for key in STATE_KEYS {
        let known =
            job_states.contains(key) || key.starts_with("activity.") || key.starts_with("gather.");
        assert!(known, "`{key}`");
    }
}
