#[path = "../src/export.rs"]
mod export;

use bevy::math::Vec2;
use turts_mod_api::{ContentId, CostSource, WorldPos};
use turts_net_protocol::{
    messages as wire,
    messages::{Pos, UnitId, UnitSnapshot},
};
use turts_sim::{
    CommittedTick, SlotId, Vec2 as SimVec2, WorldView,
    economy::{
        gather::{GatherStage, WaitReason},
        production::JobStage,
        views::{
            BalanceView, CostView, EconomyChangesView, EconomyStateView, InventoryView, ItemView,
            JobView, NodeView, OrderView, QueueView,
        },
    },
    ids::NodeId,
    world::UnitState,
};

fn content(s: &str) -> ContentId {
    ContentId::parse(s).unwrap()
}
fn pos(x: f32, y: f32) -> WorldPos {
    WorldPos { x, y }
}
fn unit(id: u32, owner: u8, content_id: &str, p: Vec2) -> UnitState {
    UnitState {
        unit_id: id,
        content_id: content_id.into(),
        owner_slot: owner,
        facing: None,
        pos: p,
    }
}
fn balance(slot: u8, resource: &str, on_hand: u64, reserved: u64) -> BalanceView {
    BalanceView {
        slot,
        resource: content(resource),
        on_hand,
        reserved,
    }
}
fn inv(unit: u32, owner_slot: u8, resource: &str, amount: u64) -> InventoryView {
    InventoryView {
        unit,
        owner_slot,
        capacity: 999,
        items: vec![ItemView {
            resource: content(resource),
            on_hand: amount,
            reserved: 0,
        }],
    }
}
fn cost(source: CostSource, resource: &str, amount: u64) -> CostView {
    CostView {
        source,
        resource: content(resource),
        amount,
    }
}
fn job(id: u64, stage: JobStage, charged: Vec<CostView>, reserved: Vec<CostView>) -> JobView {
    JobView {
        id,
        recipe: content("acme:worker"),
        stage,
        elapsed_ticks: 7,
        duration_ticks: 11,
        charged,
        reserved,
    }
}
fn fixture() -> WorldView {
    WorldView {
        tick: 42,
        m3: turts_sim::economy::views_m3::M3AuditState::default(),
        units: vec![
            unit(1, 0, "acme:harvester", Vec2::new(3., 4.)),
            unit(2, 1, "acme:private", Vec2::new(900., 901.)),
        ],
        nodes: vec![NodeView {
            id: NodeId(8),
            content_id: content("acme:ore"),
            pos: pos(1000., -20.),
            remaining: 0,
        }],
        economy: EconomyStateView {
            balances: vec![
                balance(0, "acme:ore", 0, 0),
                balance(1, "secret:crystal", 77, 12),
            ],
            inventories: vec![inv(1, 0, "acme:ore", 0), inv(2, 1, "secret:crystal", 77)],
            orders: vec![
                OrderView {
                    unit: 1,
                    owner_slot: 0,
                    node: Some(NodeId(8)),
                    receiver: Some(3),
                    stage: GatherStage::Idle,
                },
                OrderView {
                    unit: 2,
                    owner_slot: 1,
                    node: None,
                    receiver: None,
                    stage: GatherStage::Waiting {
                        reason: WaitReason::NoReceiver,
                    },
                },
            ],
            queues: vec![
                QueueView {
                    producer: 4,
                    owner_slot: 0,
                    jobs: vec![
                        job(
                            20,
                            JobStage::Queued,
                            vec![],
                            vec![cost(CostSource::SlotBalance, "acme:ore", 5)],
                        ),
                        job(
                            21,
                            JobStage::Running,
                            vec![cost(CostSource::ProducerInventory, "acme:ore", 6)],
                            vec![],
                        ),
                        job(22, JobStage::OutputBlocked, vec![], vec![]),
                    ],
                },
                QueueView {
                    producer: 5,
                    owner_slot: 1,
                    jobs: vec![job(
                        99,
                        JobStage::Running,
                        vec![cost(CostSource::SlotBalance, "secret:crystal", 1234567)],
                        vec![],
                    )],
                },
            ],
        },
    }
}
fn decode_snapshot(slot: u8, view: &WorldView) -> wire::Snapshot {
    let value = export::snapshot_for(view, SlotId(slot));
    let mut buffer = [0u8; 4096];
    let bytes = postcard::to_slice(&value, &mut buffer).unwrap();
    postcard::from_bytes(bytes).unwrap()
}

#[test]
fn two_slots_receive_only_own_economy_snapshot_and_delta_and_every_field_roundtrips() {
    let view = fixture();
    let before = view.clone();
    let a = decode_snapshot(0, &view);
    let b = decode_snapshot(1, &view);
    assert_eq!(view, before);
    assert_eq!(a.tick, 42);
    assert_eq!(
        a.units,
        vec![
            UnitSnapshot {
                unit_id: UnitId(1),
                content_id: "acme:harvester".into(),
                owner_slot: turts_net_protocol::messages::SlotId(0),
                pos: Pos { x: 3., y: 4. }
            },
            UnitSnapshot {
                unit_id: UnitId(2),
                content_id: "acme:private".into(),
                owner_slot: turts_net_protocol::messages::SlotId(1),
                pos: Pos { x: 900., y: 901. }
            }
        ]
    );
    assert_eq!(
        a.nodes,
        vec![wire::NodeState {
            id: wire::NodeId(8),
            content_id: "acme:ore".into(),
            pos: Pos { x: 1000., y: -20. },
            remaining: 0
        }]
    );
    assert_eq!(
        a.economy.balances,
        vec![wire::BalanceState {
            resource: "acme:ore".into(),
            on_hand: 0,
            reserved: 0
        }]
    );
    assert_eq!(
        a.economy.inventories[0],
        wire::InventoryState {
            unit: UnitId(1),
            items: vec![wire::BalanceState {
                resource: "acme:ore".into(),
                on_hand: 0,
                reserved: 0
            }]
        }
    );
    assert_eq!(
        a.economy.orders[0],
        wire::OrderState {
            unit: UnitId(1),
            node: Some(wire::NodeId(8)),
            receiver: Some(UnitId(3)),
            stage: wire::GatherStage::Idle
        }
    );
    assert_eq!(
        a.economy.queues[0].jobs,
        vec![
            wire::JobState {
                id: wire::JobId(20),
                recipe: "acme:worker".into(),
                stage: wire::JobStage::Queued,
                elapsed_ticks: 7,
                duration_ticks: 11,
                charged: vec![],
                reserved: vec![turts_mod_api::Cost {
                    source: CostSource::SlotBalance,
                    resource: "acme:ore".into(),
                    amount: 5
                }]
            },
            wire::JobState {
                id: wire::JobId(21),
                recipe: "acme:worker".into(),
                stage: wire::JobStage::Running,
                elapsed_ticks: 7,
                duration_ticks: 11,
                charged: vec![turts_mod_api::Cost {
                    source: CostSource::ProducerInventory,
                    resource: "acme:ore".into(),
                    amount: 6
                }],
                reserved: vec![]
            },
            wire::JobState {
                id: wire::JobId(22),
                recipe: "acme:worker".into(),
                stage: wire::JobStage::OutputBlocked,
                elapsed_ticks: 7,
                duration_ticks: 11,
                charged: vec![],
                reserved: vec![]
            },
        ]
    );
    assert_eq!(
        b.economy.balances,
        vec![wire::BalanceState {
            resource: "secret:crystal".into(),
            on_hand: 77,
            reserved: 12
        }]
    );
    assert_eq!(
        b.economy.orders[0].stage,
        wire::GatherStage::Waiting {
            reason: wire::GatherWaitReason::NoDropOff
        }
    );
    let mut b_buffer = [0u8; 4096];
    let b_bytes = postcard::to_slice(&b, &mut b_buffer).unwrap();
    let independently_decoded: wire::Snapshot = postcard::from_bytes(b_bytes).unwrap();
    assert_eq!(independently_decoded, b);
    assert_eq!(
        a.economy
            .inventories
            .iter()
            .map(|i| i.unit.0)
            .collect::<Vec<_>>(),
        vec![1]
    );
    assert_eq!(
        b.economy
            .inventories
            .iter()
            .map(|i| i.unit.0)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert!(!format!("{a:?}").contains("secret:crystal"));
    assert!(!format!("{a:?}").contains("1234567"));
    assert!(!format!("{:?}", b.economy).contains("acme:ore"));
}

#[test]
fn foreign_private_mutations_do_not_change_another_slots_projection() {
    let mut original = fixture();
    let baseline = export::snapshot_for(&original, SlotId(0));
    original
        .economy
        .balances
        .iter_mut()
        .find(|b| b.slot == 1)
        .unwrap()
        .on_hand = u64::MAX;
    original
        .economy
        .inventories
        .iter_mut()
        .find(|i| i.owner_slot == 1)
        .unwrap()
        .items[0]
        .reserved = u64::MAX;
    original
        .economy
        .orders
        .iter_mut()
        .find(|o| o.owner_slot == 1)
        .unwrap()
        .stage = GatherStage::Waiting {
        reason: WaitReason::DestinationOverflow,
    };
    original
        .economy
        .queues
        .iter_mut()
        .find(|q| q.owner_slot == 1)
        .unwrap()
        .jobs[0]
        .elapsed_ticks = u64::MAX;
    assert_eq!(export::snapshot_for(&original, SlotId(0)), baseline);
}

#[test]
fn delta_filters_before_projection_keeps_public_lifecycle_and_empty_replacements() {
    let batch = CommittedTick {
        tick: 43,
        spawned: vec![
            unit(30, 0, "acme:worker", Vec2::new(10., 11.)),
            unit(31, 1, "acme:secret", Vec2::new(-5., 6.)),
        ],
        removed: vec![2],
        positions: vec![(1, SimVec2::new(7., 8.))],
        nodes: vec![NodeView {
            id: NodeId(8),
            content_id: content("acme:ore"),
            pos: pos(1000., -20.),
            remaining: 0,
        }],
        economy: EconomyChangesView {
            balances: vec![
                balance(0, "acme:ore", 0, 0),
                balance(1, "secret:crystal", 8, 2),
            ],
            inventories: vec![
                InventoryView {
                    unit: 1,
                    owner_slot: 0,
                    capacity: 1,
                    items: vec![],
                },
                InventoryView {
                    unit: 2,
                    owner_slot: 1,
                    capacity: 9,
                    items: vec![],
                },
            ],
            orders: vec![
                OrderView {
                    unit: 1,
                    owner_slot: 0,
                    node: None,
                    receiver: None,
                    stage: GatherStage::Idle,
                },
                OrderView {
                    unit: 2,
                    owner_slot: 1,
                    node: None,
                    receiver: None,
                    stage: GatherStage::Idle,
                },
            ],
            queues: vec![
                QueueView {
                    producer: 4,
                    owner_slot: 0,
                    jobs: vec![],
                },
                QueueView {
                    producer: 5,
                    owner_slot: 1,
                    jobs: vec![job(99, JobStage::Queued, vec![], vec![])],
                },
            ],
        },
        output_inventories: vec![],
        belts: vec![],
        inserters: vec![],
        extractors: vec![],
        weapons: vec![],
        healths: vec![],
        research: vec![],
        enabled: vec![],
        events: vec![],
    };
    let before = batch.clone();
    let d = export::delta_for(&batch, SlotId(0));
    let mut buffer = [0u8; 4096];
    let bytes = postcard::to_slice(&d, &mut buffer).unwrap();
    let decoded: wire::WorldDelta = postcard::from_bytes(bytes).unwrap();
    assert_eq!(decoded, d);
    assert_eq!(batch, before);
    assert_eq!((d.tick, d.base_tick), (43, 42));
    assert_eq!(d.spawned[0].unit_id, UnitId(30));
    assert_eq!(d.spawned[0].content_id, "acme:worker");
    assert_eq!(
        d.spawned[0].owner_slot,
        turts_net_protocol::messages::SlotId(0)
    );
    assert_eq!(d.spawned[0].pos, Pos { x: 10., y: 11. });
    assert_eq!(d.spawned[1].unit_id, UnitId(31));
    assert_eq!(d.removed, vec![UnitId(2)]);
    assert_eq!(
        d.positions,
        vec![turts_net_protocol::messages::UnitDelta {
            unit_id: UnitId(1),
            pos: Pos { x: 7., y: 8. }
        }]
    );
    assert_eq!(
        decoded
            .economy
            .inventories
            .iter()
            .map(|i| i.unit)
            .collect::<Vec<_>>(),
        vec![UnitId(1)]
    );
    assert_eq!(decoded.economy.inventories[0].items, vec![]);
    assert_eq!(
        decoded
            .economy
            .orders
            .iter()
            .map(|o| o.unit)
            .collect::<Vec<_>>(),
        vec![UnitId(1)]
    );
    assert_eq!(decoded.economy.orders[0].stage, wire::GatherStage::Idle);
    assert_eq!(
        decoded
            .economy
            .queues
            .iter()
            .map(|q| q.producer)
            .collect::<Vec<_>>(),
        vec![UnitId(4)]
    );
    assert_eq!(decoded.economy.queues[0].jobs, vec![]);
    assert!(!format!("{:?}", decoded.economy).contains("secret:crystal"));
    assert_eq!(decoded.nodes.len(), 1);

    let other = export::delta_for(&batch, SlotId(1));
    let mut other_buffer = [0u8; 4096];
    let other_bytes = postcard::to_slice(&other, &mut other_buffer).unwrap();
    let other_decoded: wire::WorldDelta = postcard::from_bytes(other_bytes).unwrap();
    assert_eq!(other_decoded, other);
    assert_eq!(other.economy.balances[0].resource, "secret:crystal");
    assert_eq!(
        other_decoded
            .economy
            .inventories
            .iter()
            .map(|i| i.unit)
            .collect::<Vec<_>>(),
        vec![UnitId(2)]
    );
    assert_eq!(
        other_decoded
            .economy
            .orders
            .iter()
            .map(|o| o.unit)
            .collect::<Vec<_>>(),
        vec![UnitId(2)]
    );
    assert_eq!(
        other_decoded
            .economy
            .queues
            .iter()
            .map(|q| q.producer)
            .collect::<Vec<_>>(),
        vec![UnitId(5)]
    );
    assert_eq!(
        other_decoded
            .economy
            .queues
            .iter()
            .flat_map(|q| q.jobs.iter().map(|j| j.id))
            .collect::<Vec<_>>(),
        vec![wire::JobId(99)]
    );
    assert_eq!(other.economy.queues[0].jobs[0].id, wire::JobId(99));
    assert!(!format!("{:?}", other.economy).contains("acme:ore"));
}

#[test]
fn every_gather_stage_and_wait_reason_maps_exhaustively() {
    let mut view = fixture();
    view.economy.orders = vec![
        GatherStage::MovingToNode,
        GatherStage::Gathering,
        GatherStage::MovingToReceiver,
        GatherStage::Unloading,
        GatherStage::Idle,
        GatherStage::Waiting {
            reason: WaitReason::NoDeposit,
        },
        GatherStage::Waiting {
            reason: WaitReason::NoReceiver,
        },
        GatherStage::Waiting {
            reason: WaitReason::DestinationFull,
        },
        GatherStage::Waiting {
            reason: WaitReason::DestinationOverflow,
        },
    ]
    .into_iter()
    .enumerate()
    .map(|(i, stage)| OrderView {
        unit: i as u32 + 1,
        owner_slot: 0,
        node: None,
        receiver: None,
        stage,
    })
    .collect();
    let snap = decode_snapshot(0, &view);
    let stages: Vec<_> = snap
        .economy
        .orders
        .iter()
        .map(|o| o.stage.clone())
        .collect();
    assert_eq!(
        stages,
        vec![
            wire::GatherStage::ToNode,
            wire::GatherStage::Gathering,
            wire::GatherStage::ToDropOff,
            wire::GatherStage::Unloading,
            wire::GatherStage::Idle,
            wire::GatherStage::Waiting {
                reason: wire::GatherWaitReason::NoDeposit
            },
            wire::GatherStage::Waiting {
                reason: wire::GatherWaitReason::NoDropOff
            },
            wire::GatherStage::Waiting {
                reason: wire::GatherWaitReason::DestinationFull
            },
            wire::GatherStage::Waiting {
                reason: wire::GatherWaitReason::DestinationOverflow
            },
        ]
    );
}

#[test]
fn empty_batch_is_an_empty_heartbeat_with_consecutive_ticks() {
    let batch = CommittedTick {
        tick: 1,
        ..Default::default()
    };
    let d = export::delta_for(&batch, SlotId(0));
    assert_eq!((d.tick, d.base_tick), (1, 0));
    assert!(d.spawned.is_empty() && d.removed.is_empty() && d.positions.is_empty());
    assert!(d.economy.balances.is_empty() && d.economy.inventories.is_empty());
    assert!(d.economy.orders.is_empty() && d.economy.queues.is_empty());
    let mut buffer = [0u8; 4096];
    let bytes = postcard::to_slice(&d, &mut buffer).unwrap();
    let decoded: wire::WorldDelta = postcard::from_bytes(bytes).unwrap();
    assert_eq!(decoded, d);
}
