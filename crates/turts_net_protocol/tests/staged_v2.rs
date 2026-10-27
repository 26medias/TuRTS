use turts_mod_api::{Cost, CostSource};
use turts_net_protocol::messages::{Pos, SlotId, UnitDelta, UnitId, UnitSnapshot};
use turts_net_protocol::staged_v2::*;

fn assert_postcard_round_trip<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let bytes = postcard::to_vec::<T, 16384>(value).expect("serialize");
    let decoded: T = postcard::from_bytes(&bytes).expect("deserialize");
    assert_eq!(&decoded, value);
}

#[test]
fn command_dtos_round_trip_every_field_and_variant() {
    let commands = [
        PlayerCommand {
            seq: 1,
            kind: PlayerCommandKind::MoveTo {
                unit_ids: vec![UnitId(2), UnitId(3)],
                target: Pos { x: 1.25, y: -2.5 },
            },
        },
        PlayerCommand {
            seq: 4,
            kind: PlayerCommandKind::Gather {
                unit_id: UnitId(5),
                node_id: NodeId(6),
            },
        },
        PlayerCommand {
            seq: 7,
            kind: PlayerCommandKind::Produce {
                producer_id: UnitId(8),
                recipe_id: "mod:recipe".into(),
            },
        },
        PlayerCommand {
            seq: 9,
            kind: PlayerCommandKind::CancelProduction {
                producer_id: UnitId(10),
                job_id: JobId(11),
            },
        },
    ];
    for command in commands {
        assert_postcard_round_trip(&command);
    }
    assert_postcard_round_trip(&CommandApplied {
        seq: 12,
        tick: 13,
        job_id: Some(JobId(14)),
    });
    assert_postcard_round_trip(&CommandApplied {
        seq: 15,
        tick: 16,
        job_id: None,
    });
    for reason in CommandRejectReason::ALL {
        assert_postcard_round_trip(&CommandRejected {
            seq: 17,
            tick: 18,
            reason: *reason,
        });
        assert_eq!(
            serde_json::to_string(reason).unwrap(),
            format!("\"{}\"", reason.as_str())
        );
    }
}

#[test]
fn replication_dtos_round_trip_every_field() {
    let unit = UnitSnapshot {
        unit_id: UnitId(1),
        content_id: "mod:unit".into(),
        owner_slot: SlotId(2),
        pos: Pos { x: 3.5, y: 4.5 },
    };
    let node = NodeState {
        id: NodeId(5),
        content_id: "mod:node".into(),
        pos: Pos { x: 6.5, y: 7.5 },
        remaining: 8,
    };
    let balance = BalanceState {
        resource: "mod:ore".into(),
        on_hand: 9,
        reserved: 10,
    };
    let charged_cost = Cost {
        resource: "mod:ore".into(),
        amount: 11,
        source: CostSource::ProducerInventory,
    };
    let reserved_cost = Cost {
        resource: "mod:credits".into(),
        amount: 12,
        source: CostSource::SlotBalance,
    };
    let job = JobState {
        id: JobId(12),
        recipe: "mod:recipe".into(),
        stage: JobStage::OutputBlocked,
        elapsed_ticks: 13,
        duration_ticks: 14,
        charged: vec![charged_cost],
        reserved: vec![reserved_cost],
    };
    let inventory = InventoryState {
        unit: UnitId(15),
        items: vec![balance.clone()],
    };
    let order = OrderState {
        unit: UnitId(16),
        node: Some(NodeId(17)),
        receiver: Some(UnitId(18)),
        stage: GatherStage::Waiting {
            reason: GatherWaitReason::DestinationOverflow,
        },
    };
    let queue = QueueState {
        producer: UnitId(19),
        jobs: vec![job],
    };
    let economy = EconomyState {
        balances: vec![balance.clone()],
        inventories: vec![inventory.clone()],
        orders: vec![order.clone()],
        queues: vec![queue.clone()],
    };
    let changes = EconomyChanges {
        balances: vec![balance],
        inventories: vec![inventory],
        orders: vec![order],
        queues: vec![queue],
    };
    assert_postcard_round_trip(&node);
    assert_postcard_round_trip(&economy);
    assert_postcard_round_trip(&changes);
    assert_postcard_round_trip(&Snapshot {
        tick: 20,
        units: vec![unit.clone()],
        nodes: vec![node.clone()],
        economy: economy.clone(),
    });
    assert_postcard_round_trip(&WorldDelta {
        tick: 21,
        base_tick: 20,
        spawned: vec![unit],
        removed: vec![UnitId(22)],
        positions: vec![UnitDelta {
            unit_id: UnitId(23),
            pos: Pos { x: 24.5, y: 25.5 },
        }],
        nodes: vec![node],
        economy: changes,
    });
}

#[test]
fn economy_stages_round_trip_every_variant_and_optional_order_reference() {
    for stage in [JobStage::Queued, JobStage::Running, JobStage::OutputBlocked] {
        assert_postcard_round_trip(&stage);
    }
    for stage in [
        GatherStage::ToNode,
        GatherStage::Gathering,
        GatherStage::ToDropOff,
        GatherStage::Unloading,
        GatherStage::Idle,
    ] {
        assert_postcard_round_trip(&stage);
    }
    for reason in GatherWaitReason::ALL {
        assert_postcard_round_trip(&GatherStage::Waiting { reason: *reason });
        assert_eq!(
            serde_json::to_string(reason).unwrap(),
            format!("\"{}\"", reason.as_str())
        );
    }
    for (node, receiver) in [
        (None, None),
        (Some(NodeId(1)), None),
        (None, Some(UnitId(2))),
        (Some(NodeId(3)), Some(UnitId(4))),
    ] {
        assert_postcard_round_trip(&OrderState {
            unit: UnitId(5),
            node,
            receiver,
            stage: GatherStage::Idle,
        });
    }
}
