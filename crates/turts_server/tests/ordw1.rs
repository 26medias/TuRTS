//! L0-WIRE: row ORDW1 (design 6.8, "`unit_orders` as changes" and "Records with a start tick and
//! a rate") at the export level, and the removal rule of every new record that names a unit.
//!
//! The scenarios run the real sim (`headless_app` with the `muster` package) and project each
//! committed tick with `export::delta_for`, the function that the server calls. They print one
//! `ORDW1-BYTES` line for each measured figure; `docs/measurements/mp1.md` records them.
//!
//! Cases here: the idle table (0 bytes), the single `MoveTo` (one record at the accept tick and
//! one at the arrival tick, record bytes and not operations: slice rule, plan section 1), the
//! weapon part of case C, the `Fired` size, and the removal rule.

#[path = "../src/export.rs"]
mod export;

use std::collections::BTreeMap;
use std::path::Path;

use turts_mod_runtime::archive::PackageArchive;
use turts_mod_runtime::pack::pack_dir;
use turts_net_protocol::messages as wire;
use turts_sim::economy::events::{
    ConstructionQueueRow, EffectRow, RateRecordRow, RecordStateRow, SupplyRow, committed_tick,
    completed_world,
};
use turts_sim::{
    Command, CommandKind, CommandQueue, CommittedTick, SlotId, UnitTypes, Vec2, WorldView,
    headless_app, initialize_completed_world, spawn_synthetic_units,
};

fn muster_app() -> bevy_app::App {
    let package_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
    let bytes = pack_dir(&package_dir).unwrap();
    let archive = PackageArchive::parse(&bytes).unwrap();
    let mut app = headless_app(20);
    turts_pack::load_package(app.world_mut(), &archive).unwrap();
    initialize_completed_world(app.world_mut());
    app
}

fn trooper_type(app: &bevy_app::App) -> turts_mod_api::UnitTypeId {
    let types = app.world().resource::<UnitTypes>();
    (0..types.len())
        .map(|index| turts_mod_api::UnitTypeId(index as u32))
        .find(|id| {
            types
                .get(*id)
                .is_some_and(|entry| entry.content_id == "muster:trooper")
        })
        .expect("muster has a trooper")
}

fn spawn_troopers(app: &mut bevy_app::App, positions: &[Vec2]) -> Vec<u32> {
    let kind = trooper_type(app);
    spawn_synthetic_units(app.world_mut(), kind, positions).expect("spawn")
}

/// One tick, projected for `slot`.
fn step(app: &mut bevy_app::App, slot: u8) -> (wire::WorldDelta, CommittedTick, WorldView) {
    app.update();
    let batch = committed_tick(app.world()).clone();
    let view = completed_world(app.world()).clone();
    let delta = export::delta_for(&batch, &view, SlotId(slot));
    (delta, batch, view)
}

/// The postcard size of a list, 0 for an empty list (an empty field costs no record bytes).
fn record_bytes<T: serde::Serialize>(records: &[T]) -> usize {
    if records.is_empty() {
        0
    } else {
        postcard::to_vec::<_, 65536>(records).unwrap().len()
    }
}

/// Case "idle": 200 idle armed units for 100 ticks. The `unit_orders` bytes of each delta after
/// the first are 0, and so are the weapon bytes. Today (the full table in each delta) it is red:
/// plant (2).
#[test]
fn ordw1_idle_units_cost_no_order_or_weapon_bytes() {
    let mut app = muster_app();
    let positions: Vec<Vec2> = (0..200)
        .map(|i| Vec2::new(2.0 + (i % 20) as f32 * 1.5, 2.0 + (i / 20) as f32 * 1.5))
        .collect();
    spawn_troopers(&mut app, &positions);
    // The units were spawned outside a tick: publish them as the world that the first delta
    // is built against, as the server does at boot.
    initialize_completed_world(app.world_mut());
    let mut worst = 0;
    let mut table = 0;
    let mut snapshot_bytes = 0;
    let mut first_weapons = 0;
    for tick in 1..=100 {
        let (delta, _, view) = step(&mut app, 0);
        if tick == 1 {
            table = view
                .unit_orders
                .iter()
                .filter(|record| record.owner_slot == 0)
                .count();
            // The full table exists and has a record for each idle unit: the old delta carried it
            // in each tick, so the test below is not vacuous.
            assert!(table >= 200, "a record for each idle unit: {table}");
            snapshot_bytes = record_bytes(&export::snapshot_for(&view, SlotId(0)).unit_orders);
        }
        // The first delta carries one weapon record for each unit: the status goes from Ready
        // to NoTarget once. The plan counts the weapon bytes "after the first".
        let weapon_bytes = if tick == 1 {
            first_weapons = delta.weapons.len();
            0
        } else {
            record_bytes(&delta.weapons)
        };
        let bytes = record_bytes(&delta.unit_orders)
            + record_bytes(&delta.records.orders_cleared)
            + weapon_bytes;
        worst = worst.max(bytes);
        assert_eq!(
            bytes,
            0,
            "tick {tick}: an idle tick carries {} order records, {} cleared, {} weapon records",
            delta.unit_orders.len(),
            delta.records.orders_cleared.len(),
            delta.weapons.len()
        );
    }
    eprintln!(
        "ORDW1-BYTES idle: {table} idle units of slot 0 (200 spawned and the map units), 100 ticks, worst delta {worst} B of order, cleared and weapon records; the first delta has {first_weapons} weapon records (Ready to NoTarget); the full table is {table} records, {snapshot_bytes} B (the Snapshot, and the old delta in each tick)"
    );
}

/// Case "single move" (R-I): one `MoveTo` on one unit gives exactly one changed record in the
/// delta of the accept tick and one in the delta of the arrival tick. Record bytes, not
/// operations. Plant: the arrival write bypasses the change list.
#[test]
fn ordw1_a_single_move_gives_one_record_at_accept_and_one_at_arrival() {
    let mut app = muster_app();
    let view = completed_world(app.world()).clone();
    let mover = view
        .units
        .iter()
        .find(|unit| unit.content_id == "muster:trooper" && unit.owner_slot == 0)
        .expect("a trooper")
        .clone();
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(Command {
            slot: SlotId(0),
            seq: 1,
            kind: CommandKind::MoveTo {
                unit_ids: vec![mover.unit_id],
                target: Vec2::new(mover.pos.x, mover.pos.y - 6.0),
            },
        });
    let mut with_record = Vec::new();
    let mut sizes = Vec::new();
    let mut others = 0;
    for tick in 1..=400u32 {
        let (delta, _, _) = step(&mut app, 0);
        if let Some(record) = delta
            .unit_orders
            .iter()
            .find(|record| record.unit.0 == mover.unit_id)
        {
            with_record.push((tick, record.activity));
            sizes.push(postcard::to_vec::<_, 65536>(record).unwrap().len());
        }
        others += delta
            .unit_orders
            .iter()
            .filter(|record| record.unit.0 != mover.unit_id)
            .count();
        if with_record.len() >= 2 && tick > with_record[1].0 + 10 {
            break;
        }
    }
    assert_eq!(
        with_record.len(),
        2,
        "one record at the accept tick and one at the arrival tick: {with_record:?}"
    );
    assert_ne!(
        with_record[0].1,
        wire::Activity::Idle,
        "accept: {with_record:?}"
    );
    assert_eq!(
        with_record[1].1,
        wire::Activity::Idle,
        "arrival: {with_record:?}"
    );
    assert_eq!(others, 0, "no other unit has a record");
    eprintln!(
        "ORDW1-BYTES single move: accept tick {} {} B, arrival tick {} {} B, 0 records in the other ticks",
        with_record[0].0, sizes[0], with_record[1].0, sizes[1]
    );
}

fn weapon_key(
    record: &wire::WeaponRecord,
) -> (bool, u32, wire::WeaponStatus, Option<wire::UnitId>) {
    (
        record.enabled,
        record.ready_tick,
        record.status,
        record.last_target,
    )
}

/// Case C, the weapon part, and the removal rule on a real fight: 12 troopers fire at the
/// dummies for 100 ticks. A weapon record is sent only when a field changed (a shot moves
/// `ready_tick`); no record names a unit that its own delta removes.
#[test]
fn ordw1_weapon_records_are_sent_on_change_and_never_name_a_removed_unit() {
    let mut app = muster_app();
    let positions: Vec<Vec2> = (0..12)
        .map(|i| Vec2::new(51.0 + (i % 4) as f32 * 0.9, 31.0 + (i / 4) as f32 * 0.9))
        .collect();
    let spawned = spawn_troopers(&mut app, &positions);
    // The units were spawned outside a tick: publish them as the first completed world.
    initialize_completed_world(app.world_mut());
    let mut last: BTreeMap<u32, (bool, u32, wire::WeaponStatus, Option<wire::UnitId>)> =
        BTreeMap::new();
    let mut total_records = 0;
    let mut bytes = 0;
    let mut kills = 0;
    let mut shots = 0;
    let mut quiet_ticks = 0;
    for tick in 1..=100u32 {
        let (delta, batch, _) = step(&mut app, 0);
        let removed: Vec<u32> = delta.removed.iter().map(|unit| unit.0).collect();
        kills += removed.len();
        for record in &delta.weapons {
            assert!(
                record
                    .last_target
                    .is_none_or(|target| !removed.contains(&target.0)),
                "tick {tick}: the weapon record of unit {} names the removed unit {:?}",
                record.unit.0,
                record.last_target
            );
            let key = weapon_key(record);
            assert_ne!(
                last.insert(record.unit.0, key),
                Some(key),
                "tick {tick}: the weapon record of unit {} repeats the previous record",
                record.unit.0
            );
            if record.ready_tick > batch.tick {
                shots += 1;
            }
        }
        for record in &delta.unit_orders {
            assert!(
                !removed.contains(&record.unit.0),
                "tick {tick}: order record of a removed unit"
            );
        }
        for cleared in &delta.records.orders_cleared {
            assert!(
                !removed.contains(&cleared.0),
                "tick {tick}: cleared record of a removed unit"
            );
        }
        total_records += delta.weapons.len();
        bytes += record_bytes(&delta.weapons);
        if delta.weapons.is_empty() {
            quiet_ticks += 1;
        }
    }
    assert!(
        kills > 0,
        "the fight kills a unit (the removal rule is exercised)"
    );
    assert!(
        shots > 0,
        "units fire (a record with a ready tick in the future)"
    );
    assert!(
        total_records <= spawned.len() * 100 / 4,
        "records are sent on change, not each tick: {total_records} for {} units over 100 ticks",
        spawned.len()
    );
    assert!(
        quiet_ticks >= 50,
        "most ticks carry no weapon record: {quiet_ticks} of 100"
    );
    eprintln!(
        "ORDW1-BYTES weapons: 12 firing troopers, 100 ticks: {total_records} records, {bytes} B total, {} B per tick on average, {quiet_ticks} ticks with 0 B, {kills} kills",
        bytes / 100
    );
}

/// One `Fired` record is at most 8 B in postcard (case C), with an index and no string. The
/// record goes through the export writer, so a string weapon field is red: plant (5). The index
/// is 200 so that a decimal string would add bytes.
#[test]
fn ordw1_a_fired_effect_is_at_most_eight_bytes() {
    let view = view_of_two_units();
    let batch = CommittedTick {
        tick: 9,
        effects: vec![EffectRow::Fired {
            unit: 16_383,
            target: 16_383,
            weapon: 200,
        }],
        ..Default::default()
    };
    let delta = export::delta_for(&batch, &view, SlotId(0));
    let fired = &delta.records.effects[0];
    let size = postcard::to_vec::<_, 64>(fired).unwrap().len();
    eprintln!("ORDW1-BYTES Fired with two unit ids below 16,384 and weapon index 200: {size} B");
    assert!(size <= 8, "Fired is {size} B");
}

/// `orders_cleared` is written for the owner only, and a removed unit is not in it. Plant (1): no
/// `orders_cleared` in the delta.
#[test]
fn orders_cleared_is_written_for_the_owner_and_skips_nothing_that_the_sim_listed() {
    let view = view_of_two_units();
    let batch = CommittedTick {
        tick: 9,
        orders_cleared: vec![(3, 0), (4, 1), (6, 0)],
        ..Default::default()
    };
    let delta = export::delta_for(&batch, &view, SlotId(0));
    assert_eq!(
        delta
            .records
            .orders_cleared
            .iter()
            .map(|unit| unit.0)
            .collect::<Vec<_>>(),
        vec![3, 6]
    );
    let other = export::delta_for(&batch, &view, SlotId(1));
    assert_eq!(other.records.orders_cleared, vec![wire::UnitId(4)]);
}

// ---- the removal rule for the records that name a unit --------------------------------------

fn view_of_two_units() -> WorldView {
    let app = muster_app();
    completed_world(app.world()).clone()
}

fn rate_row(unit: u32, owner: u8, job: Option<u64>) -> RateRecordRow {
    RateRecordRow {
        unit,
        owner_slot: owner,
        job,
        done: 5,
        total: 10,
        per_tick: 1,
        at_tick: 3,
        state: RecordStateRow::Running,
        charged: vec![],
        held: false,
        held_since_tick: 0,
    }
}

/// A site, job or queue record never names a unit that the same delta removes; a record of a
/// live unit is sent to its owner only.
#[test]
fn a_site_job_or_queue_record_of_a_removed_unit_is_not_written() {
    let view = view_of_two_units();
    let batch = CommittedTick {
        tick: 9,
        removed: vec![5],
        sites_completed: vec![5, 6],
        site_progress: vec![
            rate_row(5, 0, None),
            rate_row(6, 0, None),
            rate_row(7, 1, None),
        ],
        job_progress: vec![rate_row(5, 0, Some(1)), rate_row(6, 0, Some(2))],
        construction_queues: vec![
            ConstructionQueueRow {
                constructor: 5,
                owner_slot: 0,
                orders: vec![],
            },
            ConstructionQueueRow {
                constructor: 6,
                owner_slot: 0,
                orders: vec![],
            },
        ],
        ..Default::default()
    };
    let delta = export::delta_for(&batch, &view, SlotId(0));
    let units = |list: Vec<u32>| list;
    assert_eq!(
        units(delta.records.sites.iter().map(|r| r.unit.0).collect()),
        vec![6]
    );
    assert_eq!(
        units(delta.records.jobs.iter().map(|r| r.producer.0).collect()),
        vec![6]
    );
    assert_eq!(
        units(
            delta
                .records
                .construction_queues
                .iter()
                .map(|r| r.constructor.0)
                .collect()
        ),
        vec![6]
    );
    assert_eq!(
        units(delta.records.sites_completed.iter().map(|u| u.0).collect()),
        vec![6]
    );
    // The other slot sees its own site and none of slot 0.
    let other = export::delta_for(&batch, &view, SlotId(1));
    assert_eq!(other.records.sites.len(), 1);
    assert!(other.records.jobs.is_empty());
}

/// Supply and `Credited` are owner only; `Destroyed` and `Fired` are public. An effect is an
/// event, not state: `Destroyed` names the unit that the same delta removes, by design.
#[test]
fn effects_and_supply_go_to_the_right_viewer() {
    let view = view_of_two_units();
    let batch = CommittedTick {
        tick: 9,
        removed: vec![5],
        supply: vec![SupplyRow {
            slot: 0,
            resource: turts_mod_api::ContentId::parse("muster:ore").unwrap(),
            provided: 10,
            consumed: 4,
        }],
        effects: vec![
            EffectRow::Fired {
                unit: 3,
                target: 5,
                weapon: 0,
            },
            EffectRow::Destroyed { unit: 5 },
            EffectRow::Credited {
                unit: 3,
                owner_slot: 0,
                resource: turts_mod_api::ContentId::parse("muster:ore").unwrap(),
                amount: 7,
            },
        ],
        ..Default::default()
    };
    let owner = export::delta_for(&batch, &view, SlotId(0));
    let other = export::delta_for(&batch, &view, SlotId(1));
    assert_eq!(owner.records.supply.len(), 1);
    assert!(other.records.supply.is_empty());
    assert_eq!(owner.records.effects.len(), 3);
    assert_eq!(
        other.records.effects.len(),
        2,
        "no `Credited` for the other slot"
    );
    assert!(
        other
            .records
            .effects
            .iter()
            .all(|effect| !matches!(effect, wire::Effect::Credited { .. }))
    );
}

/// The weapon record never names a unit that the same delta removes, also when the sim keeps
/// `last_target` (an ordered attack keeps it after the kill) and sends no change row: the owner's
/// weapon that names a removed unit is resent with no target. Plant: the resend dropped.
#[test]
fn a_weapon_that_names_a_removed_unit_is_resent_without_the_target() {
    let mut view = view_of_two_units();
    let shooter = view
        .units
        .iter()
        .find(|unit| unit.content_id == "muster:trooper" && unit.owner_slot == 0)
        .expect("a trooper")
        .unit_id;
    let dead = 9_999u32;
    let row = turts_sim::economy::views_m3::RawWeaponAudit {
        unit: shooter,
        enabled: true,
        cooldown: 7,
        effective_damage: Ok(5),
        status: turts_sim::economy::views_m3::RawWeaponStatusAudit::Cooling,
        last_target: Some(dead),
        ordered_target_fired: Some(dead),
    };
    view.m3
        .raw_weapons
        .retain(|existing| existing.unit != shooter);
    view.m3.raw_weapons.push(row);
    view.m3.raw_weapons.sort_by_key(|row| row.unit);
    // No change row in the batch: the countdown is not a change.
    let batch = CommittedTick {
        tick: 40,
        removed: vec![dead],
        ..Default::default()
    };
    let delta = export::delta_for(&batch, &view, SlotId(0));
    let record = delta
        .weapons
        .iter()
        .find(|record| record.unit.0 == shooter)
        .expect("the weapon that named the removed unit is resent");
    assert_eq!(record.last_target, None);
    assert_eq!(record.ready_tick, 47, "tick 40 plus the cooldown 7");
    // A change row that names a unit that does not exist is written without the target.
    let batch = CommittedTick {
        tick: 41,
        weapons: vec![
            view.m3
                .raw_weapons
                .iter()
                .find(|r| r.unit == shooter)
                .unwrap()
                .clone(),
        ],
        ..Default::default()
    };
    let delta = export::delta_for(&batch, &view, SlotId(0));
    assert_eq!(delta.weapons[0].last_target, None);
    // The snapshot never names a missing unit either.
    let snapshot = export::snapshot_for(&view, SlotId(0));
    let in_snapshot = snapshot
        .weapons
        .iter()
        .find(|record| record.unit.0 == shooter)
        .expect("the weapon is in the snapshot");
    assert_eq!(in_snapshot.last_target, None);
}

/// A cooldown countdown is not a change: the weapon row is left out of the change list.
#[test]
fn a_cooldown_countdown_is_not_a_weapon_change() {
    use turts_sim::economy::views_m3::{
        M3AuditState, RawWeaponAudit, RawWeaponStatusAudit, m3_changes,
    };
    let row = |cooldown, status| RawWeaponAudit {
        unit: 1,
        enabled: true,
        cooldown,
        effective_damage: Ok(5),
        status,
        last_target: Some(2),
        ordered_target_fired: None,
    };
    let state = |rows| M3AuditState {
        raw_weapons: rows,
        ..Default::default()
    };
    let cooling = |c| state(vec![row(c, RawWeaponStatusAudit::Cooling)]);
    assert!(m3_changes(&cooling(5), &cooling(4)).weapons.is_empty());
    assert_eq!(
        m3_changes(&cooling(5), &cooling(20)).weapons.len(),
        1,
        "a shot resets it"
    );
    assert_eq!(
        m3_changes(
            &cooling(1),
            &state(vec![row(0, RawWeaponStatusAudit::NoTarget)])
        )
        .weapons
        .len(),
        1,
        "the status changes when the cooldown ends"
    );
    assert!(
        m3_changes(&cooling(5), &cooling(5)).weapons.len() == 1,
        "a held cooldown is a change"
    );
}
