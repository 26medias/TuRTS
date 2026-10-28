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
fn protocol_version_is_3_and_old_hello_is_rejected_by_version_mismatch() {
    assert_eq!(NET_PROTOCOL_VERSION, 3);
    let old_hello = Hello {
        player_id: 17,
        protocol_version: 2,
        lockfile_hash: "sha256:fixture".into(),
    };
    round_trip(old_hello);
}

#[test]
fn postcard_roundtrip_every_new_record() {
    let unit = UnitSnapshot {
        unit_id: UnitId(5),
        content_id: "forge:arm_loader".into(),
        owner_slot: SlotId(0),
        facing: Some(Facing::PosX),
        pos: Pos { x: 8.0, y: 8.0 },
    };
    let records = Snapshot {
        tick: 2,
        units: vec![unit.clone()],
        nodes: vec![],
        economy: EconomyState {
            balances: vec![],
            inventories: vec![InventoryState {
                unit: UnitId(5),
                port: InventoryPort::Output,
                items: vec![],
            }],
            orders: vec![],
            queues: vec![QueueState {
                producer: UnitId(6),
                enabled: false,
                jobs: vec![],
            }],
        },
        belts: vec![BeltPieceRecord {
            piece: UnitId(20),
            slots: vec![Some("forge:shell".into()), None],
        }],
        inserters: vec![InserterRecord {
            unit: UnitId(6),
            enabled: false,
            stage: InserterStage::AtSource(Some(InserterWaitReason::SourceEmpty)),
            elapsed: 4,
            source: Some(UnitId(5)),
            destination: Some(UnitId(7)),
        }],
        extractors: vec![ExtractorRecord {
            unit: UnitId(4),
            enabled: true,
            node: Some(NodeId(3)),
            status: ExtractorStatus::Extracting,
        }],
        weapons: vec![
            WeaponRecord {
                unit: UnitId(8),
                enabled: true,
                cooldown: 1,
                damage: Ok(9),
                status: WeaponStatus::Cooling,
                last_target: Some(UnitId(11)),
            },
            WeaponRecord {
                unit: UnitId(9),
                enabled: true,
                cooldown: 0,
                damage: Err(CommandRejectReason::ArithmeticOverflow),
                status: WeaponStatus::Ready,
                last_target: None,
            },
        ],
        healths: vec![HealthRecord {
            unit: UnitId(11),
            current: 0,
        }],
        research: vec![ResearchRecord {
            slot: SlotId(0),
            completed: vec!["forge:hardened_shells".into()],
        }],
    };
    round_trip(records.clone());
    round_trip(WorldDelta {
        tick: 3,
        base_tick: 2,
        spawned: vec![unit],
        removed: vec![UnitId(11)],
        positions: vec![],
        nodes: vec![],
        economy: EconomyChanges {
            balances: vec![],
            inventories: vec![InventoryState {
                unit: UnitId(5),
                port: InventoryPort::Main,
                items: vec![],
            }],
            orders: vec![],
            queues: vec![QueueState {
                producer: UnitId(6),
                enabled: true,
                jobs: vec![],
            }],
        },
        belts: records.belts,
        inserters: records.inserters,
        extractors: records.extractors,
        weapons: records.weapons,
        healths: records.healths,
        research: records.research,
    });
}
