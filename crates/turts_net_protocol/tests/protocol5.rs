//! L0-WIRE: the shapes of protocol 5 (design 6.7.13 and 6.8): every new record round-trips in
//! postcard, the version is 5 and version 4 is old, and no record holds a bevy or library type.

use serde::Serialize;
use serde::de::DeserializeOwned;
use turts_net_protocol::NET_PROTOCOL_VERSION;
use turts_net_protocol::messages::*;

fn round_trip<T>(value: T)
where
    T: Serialize + DeserializeOwned + PartialEq + core::fmt::Debug,
{
    let bytes = postcard::to_vec::<T, 16384>(&value).expect("serialize");
    let back: T = postcard::from_bytes(&bytes).expect("deserialize");
    assert_eq!(back, value);
}

#[test]
fn the_protocol_version_is_5() {
    assert_eq!(NET_PROTOCOL_VERSION, 5);
}

#[test]
fn the_lobby_and_match_records_round_trip() {
    round_trip(LobbyState {
        revision: 3,
        you: Some(1),
        seats: vec![
            Seat {
                index: 0,
                occupant: SeatOccupant::Human {
                    label: "Player 1".into(),
                },
                ready: true,
                colour: 2,
            },
            Seat {
                index: 1,
                occupant: SeatOccupant::Bot {
                    content: "muster:bot".into(),
                },
                ready: false,
                colour: 0,
            },
            Seat {
                index: 2,
                occupant: SeatOccupant::Open,
                ready: false,
                colour: 1,
            },
        ],
        settings: vec![],
        start: LobbyStart::Countdown {
            remaining_ms: 4_000,
        },
    });
    round_trip(LobbyAction {
        revision: 9,
        action: LobbyActionKind::SetColour(3),
    });
    round_trip(LobbyAction {
        revision: 9,
        action: LobbyActionKind::SetReady(true),
    });
    for reason in LobbyRefusal::ALL {
        round_trip(LobbyRefused {
            revision: 1,
            reason: *reason,
        });
    }
    round_trip(LeaveMatch);
    round_trip(Welcome {
        slot_id: SlotId(1),
        tick: 0,
        tick_hz: 20,
        match_id: 77,
        start: MatchStart {
            start_tick: 1,
            teams: vec![0, 1],
            colours: vec![0, 1],
        },
    });
    round_trip(MatchState {
        connections: vec![SlotConnection {
            slot: SlotId(0),
            connected: false,
            since_tick: 40,
        }],
        outcomes: vec![
            Some(SlotOutcome {
                outcome: Outcome::Defeat,
                reason: OutcomeReason::Surrendered,
                tick: 99,
            }),
            None,
        ],
    });
    round_trip(MatchResult {
        end_tick: 100,
        slots: vec![ResultSlot {
            slot: SlotId(0),
            label: "Player 1".into(),
            outcome: Outcome::Victory,
            reason: OutcomeReason::OpponentsDefeated,
            tick: 100,
        }],
        stats: vec![SlotStats {
            slot: SlotId(0),
            units_trained: 5,
            units_lost: 1,
            units_destroyed: 4,
            structures_completed: 2,
            structures_lost: 0,
            gathered: vec![GatheredStat {
                resource: "frontier:ore".into(),
                amount: 900,
            }],
        }],
        duration_ticks: 100,
    });
}

#[test]
fn the_record_list_round_trips() {
    let progress = RateRecord {
        done: 5,
        total: 100,
        per_tick: 2,
        at_tick: 30,
        state: RecordState::WaitingForFunds("frontier:credits".into()),
        charged: vec![],
        held: true,
        held_since_tick: 31,
    };
    let records = DeltaRecords {
        sites_completed: vec![UnitId(4)],
        sites: vec![SiteProgress {
            unit: UnitId(5),
            progress: progress.clone(),
        }],
        jobs: vec![JobProgress {
            producer: UnitId(6),
            job: JobId(7),
            progress,
        }],
        construction_queues: vec![ConstructionQueue {
            constructor: UnitId(8),
            orders: vec![HeldOrder {
                content: "frontier:depot".into(),
                point: Pos { x: 1.0, y: 2.0 },
            }],
        }],
        supply: vec![SupplyRecord {
            slot: SlotId(0),
            resource: "frontier:power".into(),
            provided: 100,
            consumed: 40,
        }],
        orders_cleared: vec![UnitId(9)],
        effects: vec![
            Effect::Fired {
                unit: UnitId(1),
                target: UnitId(2),
                weapon: 0,
            },
            Effect::Destroyed { unit: UnitId(2) },
            Effect::Credited {
                unit: UnitId(3),
                resource: "frontier:credits".into(),
                amount: 25,
            },
        ],
        match_state: Some(MatchState::default()),
    };
    round_trip(records);
    round_trip(RallyTarget::Unit(UnitId(3)));
    round_trip(RallyTarget::Node(NodeId(2)));
    round_trip(OrderTarget::ContentAtPoint {
        content: "frontier:depot".into(),
        point: Pos { x: 4.0, y: 5.0 },
    });
}

#[test]
fn the_new_reasons_have_their_keys_in_order() {
    let new: Vec<_> = CommandRejectReason::ALL[37..]
        .iter()
        .map(|reason| reason.as_str())
        .collect();
    assert_eq!(
        new,
        [
            "site_outside_map",
            "site_out_of_area",
            "site_blocked",
            "site_near_building",
            "site_near_resource",
            "construction_queue_full",
            "match_not_started",
            "slot_defeated",
            "match_over",
        ]
    );
    for reason in CommandRejectReason::ALL {
        round_trip(*reason);
    }
}

/// The public protocol holds no Bevy type and no library type: the crate has no `bevy*`
/// dependency (`Cargo.toml`), and an effect names units by `UnitId`.
#[test]
fn the_protocol_crate_has_no_engine_dependency() {
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml")).unwrap();
    for banned in ["bevy", "lightyear", "aeronet"] {
        assert!(!manifest.contains(banned), "{banned} in Cargo.toml");
    }
}
