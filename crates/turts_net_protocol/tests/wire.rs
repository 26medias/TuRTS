use serde::Serialize;
use serde::de::DeserializeOwned;
use turts_net_protocol::NET_PROTOCOL_VERSION;
use turts_net_protocol::advertisement::Advertisement;
use turts_net_protocol::messages::*;

/// Encode with the heapless buffer. The workspace pins `postcard` with default features, and
/// `to_allocvec` needs the `alloc` feature, which this crate does not enable.
fn encode<T: Serialize>(value: &T) -> Vec<u8> {
    let bytes = postcard::to_vec::<T, 16384>(value).expect("serialize");
    bytes.to_vec()
}

fn round_trip<T>(value: T)
where
    T: Serialize + DeserializeOwned + PartialEq + core::fmt::Debug,
{
    let bytes = encode(&value);
    let back: T = postcard::from_bytes(&bytes).expect("deserialize");
    assert_eq!(back, value);
}

fn deltas(count: u32) -> UnitDeltas {
    UnitDeltas {
        tick: 7,
        base_tick: 6,
        changes: (0..count)
            .map(|i| UnitDelta {
                unit_id: UnitId(i),
                pos: Pos {
                    x: i as f32,
                    y: -(i as f32),
                },
            })
            .collect(),
    }
}

#[test]
fn messages_round_trip_through_postcard() {
    round_trip(UnitId(4_000_000_000));
    round_trip(SlotId(255));
    round_trip(Pos { x: 1.5, y: -2.25 });
    round_trip(Hello {
        player_id: u64::MAX,
        protocol_version: NET_PROTOCOL_VERSION,
        lockfile_hash: "sha256:abc".to_string(),
    });
    round_trip(Welcome {
        slot_id: SlotId(1),
        tick: 99,
        tick_hz: 20,
    });
    round_trip(JoinRejected {
        reason: "player already in match".to_string(),
    });
    let unit = UnitSnapshot {
        unit_id: UnitId(3),
        content_id: "pkg:unit".to_string(),
        owner_slot: SlotId(0),
        pos: Pos { x: 10.0, y: 20.0 },
    };
    round_trip(unit.clone());
    round_trip(Snapshot {
        tick: 5,
        units: vec![unit],
    });
    round_trip(UnitDelta {
        unit_id: UnitId(8),
        pos: Pos { x: 0.5, y: 0.25 },
    });
    round_trip(deltas(3));
    round_trip(MoveTo {
        seq: 12,
        unit_ids: vec![UnitId(1), UnitId(2)],
        target: Pos { x: 3.0, y: 4.0 },
    });
    round_trip(CommandAck { seq: 12, tick: 40 });
    round_trip(CommandRejected {
        seq: 12,
        reason: "empty_command".to_string(),
    });
    round_trip(ResyncRequest);
}

#[test]
fn unit_deltas_payload_size_is_recorded() {
    let one = encode(&deltas(1));
    let many = encode(&deltas(1000));
    println!("UnitDeltas with 1 change: {} bytes", one.len());
    println!("UnitDeltas with 1000 changes: {} bytes", many.len());
    assert!(one.len() <= 16, "one change took {} bytes", one.len());
    assert!(
        many.len() <= 1000 * 12 + 16,
        "1000 changes took {} bytes",
        many.len()
    );
}

#[test]
fn advertisement_json_has_six_fields() {
    let advertisement = Advertisement {
        mod_id: "base".to_string(),
        mod_version: "0.1.0".to_string(),
        engine_version: "0.1.0".to_string(),
        protocol_version: NET_PROTOCOL_VERSION,
        lockfile_url: "http://127.0.0.1:8080/lockfile.json".to_string(),
        lockfile_hash: "sha256:abc".to_string(),
    };
    let json = serde_json::to_value(&advertisement).expect("to json");
    let object = json.as_object().expect("json object");
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "engine_version",
            "lockfile_hash",
            "lockfile_url",
            "mod_id",
            "mod_version",
            "protocol_version"
        ]
    );
    let back: Advertisement = serde_json::from_value(json).expect("from json");
    assert_eq!(back, advertisement);
}
