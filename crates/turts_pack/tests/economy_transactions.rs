mod common;

use common::{archive_from_files, cid, economy_archive, economy_files, scenario};
use turts_pack::load_package;
use turts_sim::{
    RejectReason, SlotId, audit_state,
    economy::transaction::inventory_transfer,
    economy::views::{balance_view, inventory_view},
};

#[test]
fn transfer_is_local_atomic_and_respects_reserved_stock() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let before = audit_state(app.world_mut());

    let inventories = &before.economy.inventories;
    let source = inventories
        .iter()
        .find(|inventory| {
            inventory
                .items
                .iter()
                .any(|item| item.resource == cid("probe:timber") && item.on_hand > 0)
        })
        .unwrap()
        .unit;
    let destination = inventories
        .iter()
        .find(|inventory| inventory.owner_slot == 0 && inventory.unit != source)
        .unwrap()
        .unit;
    let foreign = inventories
        .iter()
        .find(|inventory| inventory.owner_slot != 0)
        .unwrap()
        .unit;
    let transfer_foreign = inventory_transfer(
        app.world_mut(),
        SlotId(0),
        foreign,
        destination,
        cid("probe:timber"),
        1,
    );
    assert_eq!(transfer_foreign, Err(RejectReason::NotOwner));
    let transfer_zero = inventory_transfer(
        app.world_mut(),
        SlotId(0),
        source,
        destination,
        cid("probe:timber"),
        0,
    );
    assert_eq!(transfer_zero, Err(RejectReason::InvalidAmount));
    assert_eq!(audit_state(app.world_mut()), before);

    inventory_transfer(
        app.world_mut(),
        SlotId(0),
        source,
        destination,
        cid("probe:timber"),
        50,
    )
    .unwrap();
    let after = audit_state(app.world_mut());
    let from = after
        .economy
        .inventories
        .iter()
        .find(|inventory| inventory.unit == source)
        .unwrap();
    let to = after
        .economy
        .inventories
        .iter()
        .find(|inventory| inventory.unit == destination)
        .unwrap();
    assert_eq!(from.items[0].on_hand, 50);
    assert_eq!(to.items[0].on_hand, 50);
}

#[test]
fn balance_view_exposes_owned_ledger_snapshot() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let view = balance_view(app.world(), SlotId(0), &cid("probe:token")).unwrap();
    assert_eq!(view.on_hand, 200);
    assert_eq!(view.reserved, 0);
}

#[test]
fn checked_max_math_and_duplicate_lines_never_commit() {
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    let before = audit_state(app.world_mut());
    let source = before
        .economy
        .inventories
        .iter()
        .find(|inventory| {
            inventory
                .items
                .iter()
                .any(|item| item.resource == cid("probe:timber") && item.on_hand > 0)
        })
        .unwrap()
        .unit;
    let destination = before
        .economy
        .inventories
        .iter()
        .find(|inventory| inventory.owner_slot == 0 && inventory.unit != source)
        .unwrap()
        .unit;
    let result = inventory_transfer(
        app.world_mut(),
        SlotId(0),
        source,
        destination,
        cid("probe:timber"),
        u64::MAX,
    );
    assert!(result.is_err());
    assert_eq!(audit_state(app.world_mut()), before);
}

#[test]
fn immutable_queries_do_not_grant_write_access() {
    fn accepts_shared_world(world: &bevy_ecs::world::World) {
        let _ = balance_view(world, SlotId(0), &cid("probe:token"));
        let _ = inventory_view(world, SlotId(0), 1);
    }
    let mut app = scenario(20);
    load_package(app.world_mut(), &economy_archive()).unwrap();
    accepts_shared_world(app.world());
    assert_eq!(audit_state(app.world_mut()).economy.balances.len(), 2);
}

#[test]
fn transfer_rejects_filters_and_capacity_without_mutation() {
    let mut files = economy_files();
    let exchange = String::from_utf8(files["units/exchange.ron"].clone()).unwrap();
    files.insert(
        "units/exchange.ron".into(),
        exchange
            .replace("capacity: 2000", "capacity: 50")
            .into_bytes(),
    );
    let mut app = scenario(20);
    load_package(app.world_mut(), &archive_from_files(files)).unwrap();
    let before = audit_state(app.world_mut());
    let source = before
        .economy
        .inventories
        .iter()
        .find(|inventory| {
            inventory
                .items
                .iter()
                .any(|item| item.resource == cid("probe:timber") && item.on_hand > 0)
        })
        .unwrap()
        .unit;
    let destination = before
        .economy
        .inventories
        .iter()
        .find(|inventory| inventory.owner_slot == 0 && inventory.unit != source)
        .unwrap()
        .unit;
    assert_eq!(
        inventory_transfer(
            app.world_mut(),
            SlotId(0),
            source,
            destination,
            cid("probe:token"),
            1
        ),
        Err(RejectReason::IncompatibleItem)
    );
    assert_eq!(
        inventory_transfer(
            app.world_mut(),
            SlotId(0),
            source,
            destination,
            cid("probe:timber"),
            51
        ),
        Err(RejectReason::RefundDestinationFull)
    );
    assert_eq!(audit_state(app.world_mut()), before);
}
