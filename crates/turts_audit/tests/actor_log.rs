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
fn header(slot: u8) -> EconomyLogRecord {
    EconomyLogRecord::Actor(ActorLogRecord::Installation(
        turts_audit::ActorInstallation {
            locked_package_hash: turts_mod_runtime::hash::sha256_hex(b"fixture package lock"),
            engine_api: turts_mod_api::ENGINE_API_VERSION.into(),
            protocol: turts_net_protocol::NET_PROTOCOL_VERSION,
            bots: vec![turts_audit::ActorBotInstallation {
                slot,
                profile: turts_mod_api::ContentId::parse("fixture:policy").unwrap(),
                seed: 17,
                profile_hash: "cd".repeat(32),
            }],
            limits: turts_audit::ActorOperatorLimits {
                queries: 32,
                candidates: 4096,
                rows: 1024,
                proposals: 32,
                commands: 8,
                affected_units: 256,
                pending_requests: 64,
                pending_accounts: 512,
                projection_rows: 65536,
                lease_steps: 8,
                page_rows: 64,
                cursor_records: 1024,
            },
        },
    ))
}
fn refusal_stream(human: bool) -> Vec<EconomyLogRecord> {
    let mut app = turts_sim::headless_app(20);
    let mut state = turts_sim::audit_state(app.world_mut());
    let tick = state.economy.tick;
    state.claims[usize::from(human)] = Some(u64::MAX - u64::from(human));
    let actor = if human {
        state.claims[0] = Some(44);
        ActorLogIdentity::Human {
            player_id: 44,
            slot: 0,
            connection: 1,
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
        header(u8::from(human)),
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
    absent.remove(2);
    assert!(validate_records(&absent).unwrap_err().contains("lifecycle"));
    let mut late = records.clone();
    late.swap(2, 3);
    assert!(validate_records(&late).unwrap_err().contains("lifecycle"));
    let mut backwards = records;
    let EconomyLogRecord::Actor(ActorLogRecord::Lifecycle { tick, .. }) = backwards[2] else {
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
    duplicate.push(records[5].clone());
    assert!(
        validate_records(&duplicate)
            .unwrap_err()
            .contains("duplicate actor result")
    );
    let mut wrong_source = records.clone();
    if let EconomyLogRecord::Actor(ActorLogRecord::Result { actor, .. }) = &mut wrong_source[5] {
        *actor = ActorLogIdentity::Human {
            player_id: u64::MAX,
            slot: 0,
            connection: 1,
        };
    }
    assert!(
        validate_records(&wrong_source)
            .unwrap_err()
            .contains("identity")
    );
    let mut duplicate_attempt = records.clone();
    duplicate_attempt.insert(4, records[3].clone());
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
    if let EconomyLogRecord::Actor(ActorLogRecord::Result { result, .. }) = &mut lost_member[5] {
        result.refused.pop();
    }
    assert!(
        validate_records(&lost_member)
            .unwrap_err()
            .contains("ordinary witness")
    );
    let mut changed_reason = records.clone();
    if let EconomyLogRecord::Actor(ActorLogRecord::Result { result, .. }) = &mut changed_reason[5] {
        result.refused[0].reason = CommandRejectReason::NotOwner;
    }
    assert!(
        validate_records(&changed_reason)
            .unwrap_err()
            .contains("ordinary witness")
    );
    let mut changed_envelope = records.clone();
    if let EconomyLogRecord::Actor(ActorLogRecord::Submission { envelope, .. }) =
        &mut changed_envelope[3]
    {
        envelope.order.group += 1;
    }
    assert!(
        validate_records(&changed_envelope)
            .unwrap_err()
            .contains("ordinary witness")
    );
    let mut missing_witness = records.clone();
    missing_witness.remove(4);
    assert!(
        validate_records(&missing_witness)
            .unwrap_err()
            .contains("ordinary witness")
    );
    let mut duplicate_witness = records.clone();
    duplicate_witness.insert(5, records[4].clone());
    assert!(
        validate_records(&duplicate_witness)
            .unwrap_err()
            .contains("ordinary witness")
    );
}
#[test]
fn actor_metadata_does_not_authorize_gameplay_changes_or_invalid_claims() {
    let mut changed_state = refusal_stream(false);
    if let EconomyLogRecord::Admission { after, .. } = &mut changed_state[4] {
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
        &mut fabricated_human[3]
    {
        *actor = ActorLogIdentity::Human {
            player_id: u64::MAX,
            slot: 0,
            connection: 1,
        };
        *task = None;
    }
    if let EconomyLogRecord::Actor(ActorLogRecord::Result { actor, .. }) = &mut fabricated_human[5]
    {
        *actor = ActorLogIdentity::Human {
            player_id: u64::MAX,
            slot: 0,
            connection: 1,
        };
    }
    assert!(
        validate_records(&fabricated_human)
            .unwrap_err()
            .contains("trusted claim")
    );
}

#[test]
fn installation_binds_bot_identity_versions_limits_and_reserved_claims() {
    let records = refusal_stream(false);
    let mut absent = records.clone();
    absent.remove(1);
    assert!(
        validate_records(&absent)
            .unwrap_err()
            .contains("installation")
    );
    let mut duplicate = records.clone();
    duplicate.insert(2, records[1].clone());
    assert!(
        validate_records(&duplicate)
            .unwrap_err()
            .contains("installation")
    );
    for field in ["profile", "slot", "protocol", "lease", "digest"] {
        let mut changed = records.clone();
        if let EconomyLogRecord::Actor(ActorLogRecord::Installation(header)) = &mut changed[1] {
            match field {
                "profile" => {
                    header.bots[0].profile = turts_mod_api::ContentId::parse("other:bot").unwrap()
                }
                "slot" => header.bots[0].slot = 1,
                "protocol" => header.protocol += 1,
                "lease" => header.limits.lease_steps = 0,
                "digest" => header.bots[0].profile_hash = "invalid".into(),
                _ => unreachable!(),
            }
        }
        assert!(validate_records(&changed).is_err(), "changed {field}");
    }
    let mut wrong_claim = records;
    if let EconomyLogRecord::Initial(state) = &mut wrong_claim[0] {
        state.claims[0] = Some(99);
    }
    assert!(
        validate_records(&wrong_claim)
            .unwrap_err()
            .contains("reserved slot claim")
    );
}
#[test]
fn reconnect_connections_are_distinct_trusted_input_epochs() {
    let first = refusal_stream(true);
    let mut records = first.clone();
    let mut next = first[3..].to_vec();
    for record in &mut next {
        match record {
            EconomyLogRecord::Actor(ActorLogRecord::Submission {
                actor: ActorLogIdentity::Human { connection, .. },
                ordinal,
                ..
            })
            | EconomyLogRecord::Actor(ActorLogRecord::Result {
                actor: ActorLogIdentity::Human { connection, .. },
                ordinal,
                ..
            }) => {
                *connection = 2;
                *ordinal = 2;
            }
            EconomyLogRecord::Admission {
                connection,
                ordinal,
                ..
            } => {
                *connection = 2;
                *ordinal = 2;
            }
            _ => unreachable!(),
        }
    }
    records.extend(next);
    // The sequence repeats, but the transport epoch is a different trusted input.
    validate_records(&records).unwrap();
    if let EconomyLogRecord::Admission { connection, .. } = &mut records[7] {
        *connection = 3;
    }
    assert!(
        validate_records(&records)
            .unwrap_err()
            .contains("ordinary witness")
    );
}

#[test]
fn package_locks_use_the_existing_prefixed_format_and_profiles_use_bare_hex() {
    let records = refusal_stream(false);
    validate_records(&records).expect("actual lockfile hash encoding is accepted");
    for invalid in [
        "ab".repeat(32),
        format!("SHA256:{}", "ab".repeat(32)),
        format!("sha512:{}", "ab".repeat(32)),
        format!("sha256:{}", "AB".repeat(32)),
        format!("sha256:{}", "g0".repeat(32)),
        format!("sha256:{}", "a".repeat(63)),
        format!("sha256:{}", "a".repeat(65)),
        format!(" sha256:{}", "ab".repeat(32)),
        format!("sha256:{} ", "ab".repeat(32)),
        format!("sha256:sha256:{}", "ab".repeat(32)),
    ] {
        let mut changed = records.clone();
        if let EconomyLogRecord::Actor(ActorLogRecord::Installation(header)) = &mut changed[1] {
            header.locked_package_hash = invalid.clone();
        }
        assert!(
            validate_records(&changed).unwrap_err().contains("digests"),
            "invalid package lock {invalid:?}"
        );
    }
    let mut prefixed_profile = records;
    if let EconomyLogRecord::Actor(ActorLogRecord::Installation(header)) = &mut prefixed_profile[1]
    {
        header.bots[0].profile_hash = format!("sha256:{}", header.bots[0].profile_hash);
    }
    assert!(
        validate_records(&prefixed_profile).is_err(),
        "profile digests remain bare hexadecimal"
    );
}
