//! Actor metadata is associated with ordinary evidence, never used as a state model.
use turts_audit::{
    ActorLogIdentity, ActorLogRecord, ActorPhase, ActorTask, EconomyLogRecord, read_jsonl,
    validate_records,
};
use turts_net_protocol::{
    CommandRejectReason, OrderEnvelope, OrderResult, OrderTarget, PlayerCommand, QueueMode, UnitId,
    UnitRefusal,
};

fn bot() -> ActorLogIdentity {
    ActorLogIdentity::Bot {
        profile: turts_mod_api::ContentId::parse("fixture:policy").unwrap(),
        slot: 0,
    }
}
fn refusal_stream(human: bool) -> Vec<EconomyLogRecord> {
    let mut app = turts_sim::headless_app(20);
    let mut state = turts_sim::audit_state(app.world_mut());
    let tick = state.economy.tick;
    let actor = if human {
        state.claims[0] = Some(44);
        ActorLogIdentity::Human {
            player_id: 44,
            slot: 0,
        }
    } else {
        bot()
    };
    let command = PlayerCommand {
        seq: 7,
        order: OrderEnvelope {
            order: "fixture:absent".into(),
            units: vec![UnitId(10), UnitId(20)],
            target: OrderTarget::None,
            queue: QueueMode::Replace,
            group: 8,
        },
    };
    let reason = CommandRejectReason::UnknownOrder;
    let result = OrderResult {
        seq: 7,
        tick,
        accepted: vec![],
        refused: vec![
            UnitRefusal {
                unit: UnitId(10),
                reason,
            },
            UnitRefusal {
                unit: UnitId(20),
                reason,
            },
        ],
        envelope: None,
        job_id: None,
    };
    let mut buffer = vec![0; 512];
    let raw = postcard::to_slice(&command, &mut buffer).unwrap().to_vec();
    vec![
        EconomyLogRecord::Initial(state.clone()),
        EconomyLogRecord::Actor(ActorLogRecord::Lifecycle {
            tick,
            humans_present: human,
            phase: ActorPhase::Running,
        }),
        EconomyLogRecord::Actor(ActorLogRecord::Submission {
            tick,
            ordinal: 1,
            actor: actor.clone(),
            sequence: 7,
            task: (!human).then(|| ActorTask {
                controller: "economy".into(),
                goal: "opening".into(),
                instance: "builder".into(),
            }),
            envelope: command,
        }),
        EconomyLogRecord::Admission {
            ordinal: 1,
            connection: u64::from(human),
            player_id: if human { 44 } else { u64::MAX },
            slot: 0,
            seq: 7,
            tick,
            reason,
            wire_format: "postcard".into(),
            raw_envelope: raw,
            before: state.clone(),
            after: state,
        },
        EconomyLogRecord::Actor(ActorLogRecord::Result {
            tick,
            ordinal: 1,
            actor,
            sequence: 7,
            result,
        }),
    ]
}
#[test]
fn actor_logs_round_trip_and_leave_legacy_encoding_unchanged() {
    let claim = EconomyLogRecord::Claim {
        tick: 3,
        player_id: 44,
        slot: 0,
    };
    assert_eq!(
        serde_json::to_string(&claim).unwrap(),
        r#"{"record":"claim","data":{"tick":3,"player_id":44,"slot":0}}"#
    );
    for human in [false, true] {
        let records = refusal_stream(human);
        validate_records(&records).unwrap();
        let jsonl = records
            .iter()
            .map(|record| serde_json::to_string(record).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(read_jsonl(std::io::Cursor::new(jsonl)).unwrap(), records);
    }
    let mut legacy = refusal_stream(false);
    legacy.retain(|record| !matches!(record, EconomyLogRecord::Actor(_)));
    validate_records(&legacy).unwrap();
}
#[test]
fn lifecycle_must_precede_attempts_and_cannot_move_backwards() {
    let records = refusal_stream(false);
    let mut absent = records.clone();
    absent.remove(1);
    assert!(validate_records(&absent).unwrap_err().contains("lifecycle"));
    let mut late = records.clone();
    late.swap(1, 2);
    assert!(validate_records(&late).unwrap_err().contains("lifecycle"));
    let mut backwards = records;
    let EconomyLogRecord::Actor(ActorLogRecord::Lifecycle { tick, .. }) = backwards[1] else {
        unreachable!()
    };
    backwards.push(EconomyLogRecord::Actor(ActorLogRecord::Lifecycle {
        tick: tick + 1,
        humans_present: false,
        phase: ActorPhase::Waiting,
    }));
    assert!(validate_records(&backwards).unwrap_err().contains("phase"));
}
#[test]
fn missing_duplicate_or_wrong_source_results_are_rejected() {
    let records = refusal_stream(false);
    let mut absent = records.clone();
    absent.pop();
    assert!(
        validate_records(&absent)
            .unwrap_err()
            .contains("complete result")
    );
    let mut duplicate = records.clone();
    duplicate.push(records[4].clone());
    assert!(
        validate_records(&duplicate)
            .unwrap_err()
            .contains("duplicate actor result")
    );
    let mut wrong_source = records.clone();
    if let EconomyLogRecord::Actor(ActorLogRecord::Result { actor, .. }) = &mut wrong_source[4] {
        *actor = ActorLogIdentity::Human {
            player_id: u64::MAX,
            slot: 0,
        };
    }
    assert!(
        validate_records(&wrong_source)
            .unwrap_err()
            .contains("identity")
    );
    let mut duplicate_attempt = records.clone();
    duplicate_attempt.insert(3, records[2].clone());
    assert!(
        validate_records(&duplicate_attempt)
            .unwrap_err()
            .contains("ordinal")
    );
}
#[test]
fn every_per_unit_outcome_and_exact_admitted_envelope_are_checked() {
    let records = refusal_stream(false);
    let mut lost_member = records.clone();
    if let EconomyLogRecord::Actor(ActorLogRecord::Result { result, .. }) = &mut lost_member[4] {
        result.refused.pop();
    }
    assert!(
        validate_records(&lost_member)
            .unwrap_err()
            .contains("ordinary witness")
    );
    let mut changed_reason = records.clone();
    if let EconomyLogRecord::Actor(ActorLogRecord::Result { result, .. }) = &mut changed_reason[4] {
        result.refused[0].reason = CommandRejectReason::NotOwner;
    }
    assert!(
        validate_records(&changed_reason)
            .unwrap_err()
            .contains("ordinary witness")
    );
    let mut changed_envelope = records.clone();
    if let EconomyLogRecord::Actor(ActorLogRecord::Submission { envelope, .. }) =
        &mut changed_envelope[2]
    {
        envelope.order.group += 1;
    }
    assert!(
        validate_records(&changed_envelope)
            .unwrap_err()
            .contains("ordinary witness")
    );
    let mut missing_witness = records.clone();
    missing_witness.remove(3);
    assert!(
        validate_records(&missing_witness)
            .unwrap_err()
            .contains("ordinary witness")
    );
    let mut duplicate_witness = records.clone();
    duplicate_witness.insert(4, records[3].clone());
    assert!(
        validate_records(&duplicate_witness)
            .unwrap_err()
            .contains("ordinary witness")
    );
}
#[test]
fn actor_metadata_does_not_authorize_gameplay_changes_or_invalid_claims() {
    let mut changed_state = refusal_stream(false);
    if let EconomyLogRecord::Admission { after, .. } = &mut changed_state[3] {
        after.economy.tick += 1;
    }
    assert!(
        validate_records(&changed_state)
            .unwrap_err()
            .contains("changed audited state")
    );
    let mut wrong_claim = refusal_stream(true);
    if let EconomyLogRecord::Initial(state) = &mut wrong_claim[0] {
        state.claims[0] = Some(55);
    }
    assert!(
        validate_records(&wrong_claim)
            .unwrap_err()
            .contains("trusted claim")
    );
    let mut fabricated_human = refusal_stream(false);
    if let EconomyLogRecord::Initial(state) = &mut fabricated_human[0] {
        state.claims[0] = Some(u64::MAX);
    }
    if let EconomyLogRecord::Actor(ActorLogRecord::Submission { actor, task, .. }) =
        &mut fabricated_human[2]
    {
        *actor = ActorLogIdentity::Human {
            player_id: u64::MAX,
            slot: 0,
        };
        *task = None;
    }
    if let EconomyLogRecord::Actor(ActorLogRecord::Result { actor, .. }) = &mut fabricated_human[4]
    {
        *actor = ActorLogIdentity::Human {
            player_id: u64::MAX,
            slot: 0,
        };
    }
    assert!(
        validate_records(&fabricated_human)
            .unwrap_err()
            .contains("ordinary witness")
    );
}
