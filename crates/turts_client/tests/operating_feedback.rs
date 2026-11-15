mod support;

use bevy::{app::App, platform::time::Instant};
use support::feedback::{alert_state, feedback_app};
use turts_client::{
    debug_feed::{AlertPingsFeed, AlertsFeed},
    feedback::FrameFeedback,
    ingest::ReplicaState,
    replica::{DeltaOutcome, Replica, ReplicaContext, SnapshotOutcome},
};
use turts_mod_api::{
    AlertTrigger, ContentId, ResourceDef, ResourceKind, SupplyCounts, SupplyScope,
};
use turts_net_protocol::{
    DeltaRecords, SlotId, Snapshot, SnapshotRecords, SupplyRecord, WorldDelta,
};

fn power(provided: u64, consumed: u64) -> SupplyRecord {
    SupplyRecord {
        slot: SlotId(0),
        resource: "muster:power".into(),
        provided,
        consumed,
    }
}

fn supply_app(provided: u64, consumed: u64) -> App {
    let (mut state, mut acquired) = alert_state();
    for name in ["power", "population"] {
        acquired.0.content.catalog.resources.insert(
            ContentId::parse(&format!("muster:{name}")).unwrap(),
            ResourceDef {
                id: name.into(),
                name: name.into(),
                unit_name: name.into(),
                kind: ResourceKind::Supply {
                    scope: SupplyScope::Slot,
                    counts: SupplyCounts::Completed,
                },
            },
        );
    }
    let interface = acquired.0.presentation.interface.as_mut().unwrap();
    let template = interface.alerts[0].clone();
    interface.alerts = [
        (
            "power_short",
            AlertTrigger::SupplyShort("muster:power".into()),
        ),
        (
            "power_restored",
            AlertTrigger::SupplyRestored("muster:power".into()),
        ),
    ]
    .into_iter()
    .map(|(id, trigger)| {
        let mut rule = template.clone();
        rule.id = id.into();
        rule.trigger = trigger;
        rule.units.clear();
        rule.offscreen_only = false;
        rule.cooldown = 0.0;
        rule.sound = None;
        rule.minimap_ping = true;
        rule
    })
    .collect();
    let mut replica = Replica::new(20);
    replica.set_context(ReplicaContext::new(acquired.0.content.clone(), 0));
    assert_eq!(
        replica.apply_snapshot(
            &Snapshot {
                tick: 1,
                records: SnapshotRecords {
                    supply: vec![power(provided, consumed)],
                    ..Default::default()
                },
                ..Default::default()
            },
            Instant::now(),
        ),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    state.replica = Some(replica);
    feedback_app(state, acquired)
}

fn delta(app: &mut App, provided: u64, consumed: u64) {
    let mut state = app.world_mut().resource_mut::<ReplicaState>();
    let replica = state.replica.as_mut().unwrap();
    let base_tick = replica.last_applied_tick.unwrap();
    assert_eq!(
        replica.apply_delta(
            &WorldDelta {
                tick: base_tick + 1,
                base_tick,
                records: DeltaRecords {
                    supply: vec![power(provided, consumed)],
                    ..Default::default()
                },
                ..Default::default()
            },
            Instant::now(),
        ),
        DeltaOutcome::Applied
    );
}

fn frame_alerts(app: &App) -> Vec<String> {
    app.world()
        .resource::<FrameFeedback>()
        .alerts
        .iter()
        .map(|row| row["id"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn operating_supply_alerts_emit_once_without_inventing_a_unit_or_map_anchor() {
    let mut app = supply_app(10, 8);
    app.update();
    assert!(
        frame_alerts(&app).is_empty(),
        "initial supply is a baseline"
    );
    delta(&mut app, 4, 8);
    app.update();
    assert_eq!(frame_alerts(&app), ["power_short"]);
    let frame = app.world().resource::<FrameFeedback>();
    assert_eq!(frame.alerts[0]["unit"], 0);
    assert!(frame.alerts[0]["position"].is_null());
    assert!(app.world().resource::<AlertPingsFeed>().0.is_empty());
    for _ in 0..3 {
        app.update();
        assert!(
            frame_alerts(&app).is_empty(),
            "stable shortage must not repeat"
        );
    }
    delta(&mut app, 8, 8);
    app.update();
    assert_eq!(
        frame_alerts(&app),
        ["power_restored"],
        "equality restores supply"
    );
    app.update();
    assert!(frame_alerts(&app).is_empty());
    assert_eq!(app.world().resource::<AlertsFeed>().0.len(), 2);
    delta(&mut app, 0, 0);
    app.update();
    assert!(
        frame_alerts(&app).is_empty(),
        "explicit zero totals are sufficient supply"
    );
}

#[test]
fn operating_supply_snapshots_and_new_matches_establish_silent_baselines() {
    let mut app = supply_app(0, 8);
    app.update();
    assert!(
        frame_alerts(&app).is_empty(),
        "initial shortage is a baseline"
    );
    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        assert_eq!(
            state.replica.as_mut().unwrap().apply_snapshot(
                &Snapshot {
                    tick: 10,
                    records: SnapshotRecords {
                        supply: vec![power(10, 8)],
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Instant::now(),
            ),
            SnapshotOutcome::Applied { replayed: 0 }
        );
    }
    app.update();
    assert!(
        frame_alerts(&app).is_empty(),
        "resync recovery is a baseline"
    );
    delta(&mut app, 0, 8);
    app.update();
    assert_eq!(frame_alerts(&app), ["power_short"]);
    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        state.welcome.as_mut().unwrap().match_id = 2;
        // A replacement can reuse the old snapshot tick. Match identity must reset history too.
        let mut replacement = state.replica.clone().unwrap();
        let tick = replacement.last_applied_tick.unwrap();
        assert_eq!(
            replacement.apply_delta(
                &WorldDelta {
                    tick: tick + 1,
                    base_tick: tick,
                    records: DeltaRecords {
                        supply: vec![power(10, 8)],
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Instant::now(),
            ),
            DeltaOutcome::Applied
        );
        state.replica = Some(replacement);
    }
    app.update();
    assert!(
        frame_alerts(&app).is_empty(),
        "new-match recovery is a baseline"
    );
    delta(&mut app, 0, 8);
    app.update();
    assert_eq!(frame_alerts(&app), ["power_short"]);
}

#[test]
fn operating_supply_rejects_foreign_records_without_creating_owner_alerts() {
    let mut app = supply_app(10, 8);
    app.update();
    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        let replica = state.replica.as_mut().unwrap();
        let mut foreign = power(0, 100);
        foreign.slot = SlotId(1);
        assert_eq!(
            replica.apply_snapshot(
                &Snapshot {
                    tick: 5,
                    records: SnapshotRecords {
                        supply: vec![foreign],
                        ..Default::default()
                    },
                    ..Default::default()
                },
                Instant::now(),
            ),
            SnapshotOutcome::Invalid
        );
        assert_eq!(
            replica.supply("muster:power", turts_mod_api::SupplySide::Provided),
            Some(10)
        );
    }
    app.update();
    assert!(frame_alerts(&app).is_empty());
    assert!(app.world().resource::<AlertsFeed>().0.is_empty());
}

#[test]
fn operating_supply_distinct_transitions_are_not_lost_with_zero_authored_cooldown() {
    let mut app = supply_app(10, 8);
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(100),
    ));
    app.update();
    for (provided, expected) in [
        (0, "power_short"),
        (10, "power_restored"),
        (0, "power_short"),
    ] {
        delta(&mut app, provided, 8);
        app.update();
        assert_eq!(frame_alerts(&app), [expected]);
    }
}
