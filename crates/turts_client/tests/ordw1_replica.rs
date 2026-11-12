//! L0-WIRE: row ORDW1 on the client side (design 6.8, `unit_orders` as changes) and the
//! point-14 test (design 15 point 14, 6.7.7 step 5): the replica applies a `Snapshot` that it did
//! not request, and ignores a stale one.
//!
//! Case A: the delta that clears a record (`orders_cleared`) leaves the unit with no current
//! order. Case B: a client that dropped that delta and resynced shows no current order, because
//! the `Snapshot` builds the table again from an empty map. Plant (1): no `orders_cleared`.
//! Plant (3): the replica merges the `Snapshot` table into the old table.

use std::{path::PathBuf, sync::OnceLock};

use bevy::platform::time::Instant;
use turts_client::content::ClientContent;
use turts_client::replica::{DeltaOutcome, Replica, ReplicaContext, SnapshotOutcome};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{
    Activity, OrderSummary, OrderTarget, Pos, SlotId, Snapshot, UnitDelta, UnitId, UnitOrderRecord,
    UnitSnapshot, WeaponRecord, WeaponStatus, WorldDelta,
};

fn pos(x: f32, y: f32) -> Pos {
    Pos { x, y }
}

/// Unit 1 and unit 2 are troopers of slot 0; unit 3 is a dummy of slot 1.
fn snapshot(tick: u32) -> Snapshot {
    let unit = |id: u32, content: &str, owner: u8| UnitSnapshot {
        unit_id: UnitId(id),
        content_id: content.to_string(),
        owner_slot: SlotId(owner),
        facing: None,
        pos: pos(id as f32, 0.0),
        site: false,
    };
    Snapshot {
        tick,
        units: vec![
            unit(1, "muster:trooper", 0),
            unit(2, "muster:trooper", 0),
            unit(3, "muster:dummy", 1),
        ],
        ..Default::default()
    }
}

fn delta(tick: u32, base_tick: u32) -> WorldDelta {
    WorldDelta {
        tick,
        base_tick,
        positions: (1..=3)
            .map(|id| UnitDelta {
                unit_id: UnitId(id),
                pos: pos(id as f32, tick as f32),
            })
            .collect(),
        ..Default::default()
    }
}

fn record(unit: u32, x: f32) -> UnitOrderRecord {
    UnitOrderRecord {
        unit: UnitId(unit),
        activity: Activity::Moving,
        current: Some(OrderSummary {
            order: "muster:order_move".into(),
            target: OrderTarget::Point(pos(x, 1.0)),
        }),
        queued: Vec::new(),
        last_failure: None,
        rally: None,
    }
}

fn replica() -> Replica {
    static CONTENT: OnceLock<ClientContent> = OnceLock::new();
    let content = CONTENT
        .get_or_init(|| {
            let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
            let archive =
                PackageArchive::parse(&pack_dir(&dir).expect("pack")).expect("parse muster");
            ClientContent::from_archives(&[archive]).expect("resolve muster")
        })
        .clone();
    let mut replica = Replica::new(20);
    replica.set_context(ReplicaContext::new(content, 0));
    replica
}

/// A replica after the `Snapshot` of tick 10 with an order record for units 1 and 2.
fn running() -> Replica {
    let now = Instant::now();
    let mut r = replica();
    let mut initial = snapshot(10);
    initial.unit_orders = vec![record(1, 5.0), record(2, 6.0)];
    assert_eq!(
        r.apply_snapshot(&initial, now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    r
}

#[test]
fn case_a_orders_cleared_removes_the_record_and_an_omitted_record_stays() {
    let now = Instant::now();
    let mut r = running();
    // A delta that omits unit 1 does not touch its record (merge, not replacement).
    assert_eq!(r.apply_delta(&delta(11, 10), now), DeltaOutcome::Applied);
    assert!(
        r.unit_orders[&1].current.is_some(),
        "an omitted record stays"
    );
    assert!(r.unit_orders.contains_key(&2));
    // `orders_cleared` removes the record of unit 1 only.
    let mut clear = delta(12, 11);
    clear.records.orders_cleared = vec![UnitId(1)];
    assert_eq!(r.apply_delta(&clear, now), DeltaOutcome::Applied);
    assert!(
        !r.unit_orders.contains_key(&1),
        "case A: the record is gone"
    );
    assert!(r.unit_orders.contains_key(&2));
}

#[test]
fn a_changed_record_replaces_only_its_own_unit() {
    let now = Instant::now();
    let mut r = running();
    let mut change = delta(11, 10);
    change.unit_orders = vec![record(2, 9.0)];
    assert_eq!(r.apply_delta(&change, now), DeltaOutcome::Applied);
    let target = |r: &Replica, unit: u32| {
        r.unit_orders[&unit]
            .current
            .as_ref()
            .unwrap()
            .target
            .clone()
    };
    assert_eq!(target(&r, 1), OrderTarget::Point(pos(5.0, 1.0)));
    assert_eq!(target(&r, 2), OrderTarget::Point(pos(9.0, 1.0)));
}

#[test]
fn case_b_a_dropped_clearing_delta_is_repaired_by_the_snapshot() {
    let now = Instant::now();
    let mut r = running();
    // Tick 11 clears unit 1 and is lost. Tick 12 arrives and shows the gap.
    assert_eq!(
        r.apply_delta(&delta(12, 11), now),
        DeltaOutcome::ResyncRequested
    );
    assert!(
        r.unit_orders.contains_key(&1),
        "the stale record is still shown"
    );
    // The resync `Snapshot` of tick 12 holds the table: unit 1 has no record.
    let mut recovered = snapshot(12);
    recovered.unit_orders = vec![record(2, 6.0)];
    assert_eq!(
        r.apply_snapshot(&recovered, now),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    assert!(
        !r.unit_orders.contains_key(&1),
        "case B: after the resync the client shows no order for the unit"
    );
    assert!(r.unit_orders.contains_key(&2));
}

/// Point 14: a `Snapshot` that the replica did not request applies (the end-tick `Snapshot` of
/// 6.7.7 step 5 comes with no request), and `IgnoredStale` holds for a tick at or below the last
/// applied tick.
#[test]
fn point_14_an_unrequested_snapshot_applies_and_a_stale_one_is_ignored() {
    let now = Instant::now();
    let mut r = running();
    assert_eq!(r.apply_delta(&delta(11, 10), now), DeltaOutcome::Applied);
    assert!(!r.in_resync(), "the replica asked for nothing");
    let mut later = snapshot(14);
    later.unit_orders = vec![record(2, 7.0)];
    assert_eq!(
        r.apply_snapshot(&later, now),
        SnapshotOutcome::Applied { replayed: 0 },
        "an unrequested newer Snapshot applies"
    );
    assert_eq!(r.last_applied_tick, Some(14));
    assert!(
        !r.unit_orders.contains_key(&1),
        "the Snapshot table replaces the old table"
    );
    assert_eq!(
        r.apply_snapshot(&snapshot(14), now),
        SnapshotOutcome::IgnoredStale
    );
    assert_eq!(
        r.apply_snapshot(&snapshot(12), now),
        SnapshotOutcome::IgnoredStale
    );
    assert_eq!(r.last_applied_tick, Some(14));
}

#[test]
fn a_bad_orders_cleared_list_rejects_the_delta() {
    let now = Instant::now();
    // The unit is in both lists.
    let mut r = running();
    let mut both = delta(11, 10);
    both.unit_orders = vec![record(1, 8.0)];
    both.records.orders_cleared = vec![UnitId(1)];
    assert_eq!(r.apply_delta(&both, now), DeltaOutcome::ResyncRequested);
    // The unit is removed by the same delta: the server never lists it.
    let mut r = running();
    let mut removed = delta(11, 10);
    removed.positions.retain(|p| p.unit_id != UnitId(1));
    removed.removed = vec![UnitId(1)];
    removed.records.orders_cleared = vec![UnitId(1)];
    assert_eq!(r.apply_delta(&removed, now), DeltaOutcome::ResyncRequested);
    // An unknown unit, and a unit of another slot.
    for unit in [99, 3] {
        let mut r = running();
        let mut bad = delta(11, 10);
        bad.records.orders_cleared = vec![UnitId(unit)];
        assert_eq!(
            r.apply_delta(&bad, now),
            DeltaOutcome::ResyncRequested,
            "unit {unit}"
        );
    }
    // A removed unit's record goes with the unit, with no `orders_cleared`.
    let mut r = running();
    let mut gone = delta(11, 10);
    gone.positions.retain(|p| p.unit_id != UnitId(1));
    gone.removed = vec![UnitId(1)];
    assert_eq!(r.apply_delta(&gone, now), DeltaOutcome::Applied);
    assert!(!r.unit_orders.contains_key(&1));
}

fn weapon(unit: u32, target: Option<u32>) -> WeaponRecord {
    WeaponRecord {
        unit: UnitId(unit),
        enabled: true,
        ready_tick: 30,
        damage: Ok(5),
        status: WeaponStatus::Cooling,
        last_target: target.map(UnitId),
    }
}

/// The contract that the server writer keeps (`export::weapon_records`): a weapon record that
/// names a unit must be replaced in the delta that removes that unit. The client rejects the
/// stale form (the MC defect: every later update was rejected), so the server resends it.
#[test]
fn a_weapon_record_that_names_a_removed_unit_must_be_replaced_in_the_same_delta() {
    let now = Instant::now();
    let setup = || {
        let mut r = replica();
        let mut initial = snapshot(10);
        initial.weapons = vec![weapon(1, Some(3))];
        assert_eq!(
            r.apply_snapshot(&initial, now),
            SnapshotOutcome::Applied { replayed: 0 }
        );
        r
    };
    // The server form: the owner's weapon is resent with no target.
    let mut r = setup();
    let mut resent = delta(11, 10);
    resent.positions.retain(|p| p.unit_id != UnitId(3));
    resent.removed = vec![UnitId(3)];
    resent.weapons = vec![weapon(1, None)];
    assert_eq!(r.apply_delta(&resent, now), DeltaOutcome::Applied);
    assert_eq!(r.weapons[0].last_target, None);
    // The stale form: no replacement, so the held record names a missing unit.
    let mut r = setup();
    let mut stale = delta(11, 10);
    stale.positions.retain(|p| p.unit_id != UnitId(3));
    stale.removed = vec![UnitId(3)];
    assert_eq!(r.apply_delta(&stale, now), DeltaOutcome::ResyncRequested);
    // A record in the same delta that names the removed unit is rejected as well.
    let mut r = setup();
    let mut naming = delta(11, 10);
    naming.positions.retain(|p| p.unit_id != UnitId(3));
    naming.removed = vec![UnitId(3)];
    naming.weapons = vec![weapon(1, Some(3))];
    assert_eq!(r.apply_delta(&naming, now), DeltaOutcome::ResyncRequested);
}

/// A `ContentAtPoint` target in an order record needs a finite point, as `Point` does.
#[test]
fn a_content_at_point_record_needs_a_finite_point() {
    let now = Instant::now();
    let with_point = |x: f32| {
        let mut r = record(1, 5.0);
        r.current.as_mut().unwrap().target = OrderTarget::ContentAtPoint {
            content: "muster:depot".into(),
            point: pos(x, 2.0),
        };
        r
    };
    let mut r = running();
    let mut good = delta(11, 10);
    good.unit_orders = vec![with_point(3.0)];
    assert_eq!(r.apply_delta(&good, now), DeltaOutcome::Applied);
    for bad in [f32::NAN, f32::INFINITY] {
        let mut r = running();
        let mut delta = delta(11, 10);
        delta.unit_orders = vec![with_point(bad)];
        assert_eq!(r.apply_delta(&delta, now), DeltaOutcome::ResyncRequested);
    }
}
