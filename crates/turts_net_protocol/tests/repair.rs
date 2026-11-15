use turts_net_protocol::{
    DeltaRecords, NET_PROTOCOL_VERSION, RepairRecord, RepairState, SnapshotRecords, UnitId,
};

#[test]
fn protocol9_repair_states_and_explicit_clear_shapes_round_trip() {
    assert_eq!(NET_PROTOCOL_VERSION, 9);
    for state in [
        RepairState::Approaching,
        RepairState::Working,
        RepairState::WaitingForFunds("test:credits".into()),
    ] {
        let record = RepairRecord {
            repairer: UnitId(7),
            target: UnitId(8),
            state,
        };
        let baseline = SnapshotRecords {
            repairs: vec![record.clone()],
            ..Default::default()
        };
        let encoded = postcard::to_vec::<_, 1024>(&baseline).unwrap();
        assert_eq!(
            postcard::from_bytes::<SnapshotRecords>(&encoded).unwrap(),
            baseline
        );
        let changed = DeltaRecords {
            repairs: vec![record],
            ..Default::default()
        };
        let encoded = postcard::to_vec::<_, 1024>(&changed).unwrap();
        assert_eq!(
            postcard::from_bytes::<DeltaRecords>(&encoded).unwrap(),
            changed
        );
    }
    let cleared = DeltaRecords {
        repairs_cleared: vec![UnitId(7)],
        ..Default::default()
    };
    let encoded = postcard::to_vec::<_, 1024>(&cleared).unwrap();
    assert_eq!(
        postcard::from_bytes::<DeltaRecords>(&encoded).unwrap(),
        cleared
    );
}
