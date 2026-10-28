use serde::Serialize;
use serde::de::DeserializeOwned;
use turts_net_protocol::NET_PROTOCOL_VERSION;
use turts_net_protocol::advertisement::Advertisement;
use turts_net_protocol::messages::*;

fn encode<T: Serialize>(value: &T) -> Vec<u8> {
    postcard::to_vec::<T, 16384>(value)
        .expect("serialize")
        .to_vec()
}
fn round_trip<T>(value: T)
where
    T: Serialize + DeserializeOwned + PartialEq + core::fmt::Debug,
{
    let bytes = encode(&value);
    let back: T = postcard::from_bytes(&bytes).expect("deserialize");
    assert_eq!(back, value);
}

#[test]
fn wire_v3_roundtrips_original_fields_and_reasons() {
    assert_eq!(NET_PROTOCOL_VERSION, 3);
    round_trip(UnitId(4_000_000_000));
    round_trip(SlotId(255));
    round_trip(Pos { x: 1.5, y: -2.25 });
    round_trip(Hello {
        player_id: u64::MAX,
        protocol_version: NET_PROTOCOL_VERSION,
        lockfile_hash: "sha256:abc".into(),
    });
    round_trip(Welcome {
        slot_id: SlotId(1),
        tick: 99,
        tick_hz: 20,
    });
    round_trip(JoinRejected {
        reason: "player already in match".into(),
    });
    let unit = UnitSnapshot {
        unit_id: UnitId(3),
        content_id: "pkg:unit".into(),
        owner_slot: SlotId(0),
        pos: Pos { x: 10.0, y: 20.0 },
        facing: None,
    };
    round_trip(unit.clone());
    let balance = BalanceState {
        resource: "base:ore".into(),
        on_hand: 19,
        reserved: 7,
    };
    let inventory = InventoryState {
        unit: UnitId(3),
        items: vec![balance.clone()],

        port: InventoryPort::Main,
    };
    let order = OrderState {
        unit: UnitId(3),
        node: Some(NodeId(8)),
        receiver: Some(UnitId(4)),
        stage: GatherStage::Waiting {
            reason: GatherWaitReason::DestinationOverflow,
        },
    };
    let job = JobState {
        id: JobId(12),
        recipe: "base:scout".into(),
        stage: JobStage::OutputBlocked,
        elapsed_ticks: 17,
        duration_ticks: 29,
        charged: vec![turts_mod_api::Cost {
            resource: "base:ore".into(),
            amount: 9,
            source: turts_mod_api::CostSource::SlotBalance,
        }],
        reserved: vec![turts_mod_api::Cost {
            resource: "base:crystal".into(),
            amount: 2,
            source: turts_mod_api::CostSource::ProducerInventory,
        }],
    };
    let node = NodeState {
        id: NodeId(8),
        content_id: "base:iron_node".into(),
        pos: Pos { x: 5.0, y: 6.0 },
        remaining: 44,
    };
    let queue = QueueState {
        producer: UnitId(4),
        jobs: vec![job],

        enabled: true,
    };
    let state = EconomyState {
        balances: vec![balance.clone()],
        inventories: vec![inventory.clone()],
        orders: vec![order.clone()],
        queues: vec![queue.clone()],
    };
    round_trip(Snapshot {
        tick: 5,
        units: vec![unit.clone()],
        nodes: vec![node.clone()],
        economy: state,

        belts: vec![],
        inserters: vec![],
        extractors: vec![],
        weapons: vec![],
        healths: vec![],
        research: vec![],
    });
    round_trip(WorldDelta {
        tick: 8,
        base_tick: 7,
        spawned: vec![unit],
        removed: vec![UnitId(9)],
        positions: vec![UnitDelta {
            unit_id: UnitId(8),
            pos: Pos { x: 0.5, y: 0.25 },
        }],
        nodes: vec![node],
        economy: EconomyChanges {
            balances: vec![balance],
            inventories: vec![inventory],
            orders: vec![order],
            queues: vec![queue],
        },

        belts: vec![],
        inserters: vec![],
        extractors: vec![],
        weapons: vec![],
        healths: vec![],
        research: vec![],
    });
    for kind in [
        PlayerCommandKind::MoveTo {
            unit_ids: vec![UnitId(1), UnitId(2)],
            target: Pos { x: 3.0, y: 4.0 },
        },
        PlayerCommandKind::Gather {
            unit_id: UnitId(2),
            node_id: NodeId(3),
        },
        PlayerCommandKind::Produce {
            producer_id: UnitId(4),
            recipe_id: "base:hire".into(),
        },
        PlayerCommandKind::CancelProduction {
            producer_id: UnitId(4),
            job_id: JobId(5),
        },
    ] {
        round_trip(PlayerCommand { seq: 12, kind });
    }
    round_trip(CommandAck { seq: 12, tick: 40 });
    round_trip(CommandApplied {
        seq: 12,
        tick: 41,
        job_id: Some(JobId(9)),
    });
    for reason in CommandRejectReason::ALL {
        round_trip(CommandRejected {
            seq: 12,
            tick: 42,
            reason: *reason,
        });
        assert_eq!(
            reason.as_str(),
            serde_json::to_value(reason).unwrap().as_str().unwrap()
        );
    }
    round_trip(ResyncRequest);
}

#[test]
fn advertisement_json_has_six_fields() {
    let advertisement = Advertisement {
        mod_id: "base".into(),
        mod_version: "0.1.0".into(),
        engine_version: "0.1.0".into(),
        protocol_version: NET_PROTOCOL_VERSION,
        lockfile_url: "http://127.0.0.1:8080/lockfile.json".into(),
        lockfile_hash: "sha256:abc".into(),
    };
    let json = serde_json::to_value(&advertisement).expect("to json");
    assert_eq!(json.as_object().unwrap().len(), 6);
    let back: Advertisement = serde_json::from_value(json).expect("from json");
    assert_eq!(back, advertisement);
}
