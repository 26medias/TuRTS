mod common;

use common::{cid, economy_files, scenario_with};
use turts_sim::{
    Command, CommandKind, EconomyEventKind, SlotId, apply_command, audit_state, committed_tick,
    economy::views::order_view,
};

fn fixture() -> std::collections::BTreeMap<String, String> {
    economy_files()
        .into_iter()
        .map(|(path, bytes)| (path, String::from_utf8(bytes).unwrap()))
        .collect()
}

fn command(seq: u32, kind: CommandKind) -> Command {
    Command {
        slot: SlotId(0),
        seq,
        kind,
    }
}

fn gather(app: &mut bevy_app::App, node_id: u32) {
    let result = apply_command(
        app.world_mut(),
        &command(
            1,
            CommandKind::Gather {
                unit_id: 3,
                node_id: turts_sim::NodeId(node_id),
            },
        ),
    );
    assert_eq!(result.outcome, Ok(turts_sim::Applied { job_id: None }));
}

fn advance(app: &mut bevy_app::App, count: usize) {
    for _ in 0..count {
        app.update();
    }
}

fn item_total(state: &turts_sim::AuditState, item: &str) -> u64 {
    state
        .economy
        .inventories
        .iter()
        .flat_map(|inventory| &inventory.items)
        .filter(|entry| entry.resource == cid(item))
        .map(|entry| entry.on_hand)
        .sum()
}

#[test]
fn extract_store_and_convert_make_positive_progress() {
    let mut files = fixture();
    let yard = files.get_mut("units/yard.ron").unwrap();
    *yard = yard.replace(
        "drop_off: Some((rules: [(item: \"probe:timber\", effect: Store)]))",
        "drop_off: None",
    );
    let mut app = scenario_with(files, 20);
    gather(&mut app, 1);
    let initial_node = turts_sim::economy::views::node_view(app.world(), turts_sim::NodeId(1))
        .unwrap()
        .remaining;
    let initial_balance =
        turts_sim::economy::views::balance_view(app.world(), SlotId(0), &cid("probe:token"))
            .unwrap()
            .on_hand;
    let mut extracted = 0;
    let mut converted = 0;
    let mut delivered = 0;
    for _ in 0..1200 {
        app.update();
        for event in &committed_tick(app.world()).events {
            match event.kind {
                EconomyEventKind::Extracted {
                    node: 1, amount, ..
                } => extracted += amount,
                EconomyEventKind::Converted {
                    amount, credited, ..
                } => {
                    delivered += amount;
                    converted += credited;
                }
                _ => {}
            }
        }
    }
    assert!(extracted > 0);
    assert!(converted > 0);
    assert_eq!(converted, delivered * 2);
    let final_balance =
        turts_sim::economy::views::balance_view(app.world(), SlotId(0), &cid("probe:token"))
            .unwrap()
            .on_hand;
    let state = audit_state(app.world_mut());
    let remaining = state
        .nodes
        .iter()
        .find(|node| node.id.0 == 1)
        .unwrap()
        .remaining;
    assert_eq!(remaining + extracted, initial_node);
    assert_eq!(final_balance - initial_balance, converted);
}

#[test]
fn self_store_is_skipped_for_a_valid_depot() {
    let mut files = fixture();
    let worker = files.get_mut("units/worker.ron").unwrap();
    *worker = worker.replace(
        "drop_off: None",
        "drop_off: Some((rules: [(item: \"probe:timber\", effect: Store)]))",
    );
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map.replacen("remaining: 1000", "remaining: 1", 1).replace(
        ", (node: \"probe:stand\", pos: (x: 40.0, y: 8.0), remaining: 1000)",
        "",
    );
    let mut app = scenario_with(files, 20);
    gather(&mut app, 1);
    let mut selected_store = None;
    for _ in 0..300 {
        app.update();
        let order = order_view(app.world(), SlotId(0), 3).unwrap();
        if order.stage == turts_sim::GatherStage::MovingToReceiver {
            selected_store = order.receiver;
            break;
        }
    }
    assert_eq!(selected_store, Some(1));
}

#[test]
fn last_node_item_and_partial_load_are_conserved() {
    let mut files = fixture();
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map.replacen("remaining: 1000", "remaining: 1", 1);
    let mut app = scenario_with(files, 20);
    gather(&mut app, 1);
    advance(&mut app, 1000);
    let state = audit_state(app.world_mut());
    let remaining = state
        .nodes
        .iter()
        .find(|node| node.id.0 == 1)
        .unwrap()
        .remaining;
    assert_eq!(remaining, 0);
    let remaining_total: u64 = state.nodes.iter().map(|node| node.remaining).sum();
    assert_eq!(item_total(&state, "probe:timber") + remaining_total, 1101);
}

#[test]
fn no_full_or_overflow_receiver_retains_cargo_and_retries() {
    let mut files = fixture();
    let yard = files.get_mut("units/yard.ron").unwrap();
    *yard = yard.replace(
        "drop_off: Some((rules: [(item: \"probe:timber\", effect: Store)]))",
        "drop_off: None",
    );
    let exchange = files.get_mut("units/exchange.ron").unwrap();
    *exchange = exchange
        .replace("capacity: 2000", "capacity: 1")
        .replace("Convert(balance: \"probe:token\", per_item: 2)", "Store");
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map.replace(
        "unit: \"probe:exchange\", owner_slot: 0, pos: (x: 8.0, y: 20.0), contents: []",
        "unit: \"probe:exchange\", owner_slot: 0, pos: (x: 8.0, y: 20.0), contents: [(item: \"probe:timber\", amount: 1)]",
    );
    let mut app = scenario_with(files, 20);
    gather(&mut app, 1);
    advance(&mut app, 2000);
    let order = order_view(app.world(), SlotId(0), 3).unwrap();
    assert!(matches!(
        order.stage,
        turts_sim::GatherStage::Waiting {
            reason: turts_sim::WaitReason::DestinationFull
        }
    ));
    assert_eq!(
        turts_sim::economy::views::inventory_view(app.world(), SlotId(0), 3)
            .unwrap()
            .items[0]
            .on_hand,
        10
    );
}

#[test]
fn move_interrupt_keeps_cargo_and_resets_carry() {
    let mut app = scenario_with(fixture(), 20);
    gather(&mut app, 1);
    advance(&mut app, 160);
    let before = audit_state(app.world_mut());
    let cargo_before = item_total(&before, "probe:timber");
    let result = apply_command(
        app.world_mut(),
        &command(
            2,
            CommandKind::MoveTo {
                unit_ids: vec![3],
                target: turts_sim::Vec2::new(30.0, 30.0),
            },
        ),
    );
    assert_eq!(result.outcome, Ok(turts_sim::Applied { job_id: None }));
    let after = audit_state(app.world_mut());
    assert_eq!(item_total(&after, "probe:timber"), cargo_before);
    assert!(matches!(
        order_view(app.world(), SlotId(0), 3).unwrap().stage,
        turts_sim::GatherStage::Idle
    ));
}

#[test]
fn rate_and_approach_boundaries_10_20_30_hz() {
    for tick_hz in [10, 20, 30] {
        let mut app = scenario_with(fixture(), tick_hz);
        gather(&mut app, 1);
        let initial = turts_sim::economy::views::node_view(app.world(), turts_sim::NodeId(1))
            .unwrap()
            .remaining;
        advance(&mut app, 1600);
        let remaining = turts_sim::economy::views::node_view(app.world(), turts_sim::NodeId(1))
            .unwrap()
            .remaining;
        assert!(remaining < initial);
        assert!(item_total(&audit_state(app.world_mut()), "probe:timber") > 100);
    }
}

fn fixture_with_stone() -> std::collections::BTreeMap<String, String> {
    let mut files = fixture();
    let manifest = files.get_mut("manifest.ron").unwrap();
    *manifest = manifest.replace(
        "resources: [\"resources/timber.ron\", \"resources/token.ron\"]",
        "resources: [\"resources/timber.ron\", \"resources/token.ron\", \"resources/stone.ron\"]",
    );
    files.insert(
        "resources/stone.ron".into(),
        "(id: \"stone\", name: \"Stone\", unit_name: \"stones\", kind: Item)".into(),
    );
    files
}

#[test]
fn empty_node_and_item_filter_rejections_preserve_orders() {
    let mut files = fixture();
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map.replacen("remaining: 1000", "remaining: 1", 1);
    let mut app = scenario_with(files, 20);
    gather(&mut app, 1);
    advance(&mut app, 250);
    let before = order_view(app.world(), SlotId(0), 3).unwrap();
    let rejected = apply_command(
        app.world_mut(),
        &command(
            2,
            CommandKind::Gather {
                unit_id: 3,
                node_id: turts_sim::NodeId(1),
            },
        ),
    );
    assert_eq!(rejected.outcome, Err(turts_sim::RejectReason::NodeDepleted));
    assert_eq!(order_view(app.world(), SlotId(0), 3).unwrap(), before);

    let mut files = fixture_with_stone();
    let worker = files.get_mut("units/worker.ron").unwrap();
    *worker = worker
        .replace("accepts: [\"probe:timber\"]", "accepts: [\"probe:stone\"]")
        .replace("items: [\"probe:timber\"]", "items: [\"probe:stone\"]");
    let mut app = scenario_with(files, 20);
    let before = order_view(app.world(), SlotId(0), 3).unwrap();
    let rejected = apply_command(
        app.world_mut(),
        &command(
            1,
            CommandKind::Gather {
                unit_id: 3,
                node_id: turts_sim::NodeId(1),
            },
        ),
    );
    assert_eq!(
        rejected.outcome,
        Err(turts_sim::RejectReason::IncompatibleItem)
    );
    assert_eq!(order_view(app.world(), SlotId(0), 3).unwrap(), before);
}

#[test]
fn depleted_deposit_falls_back_then_waits_for_no_deposit() {
    let mut files = fixture();
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map.replace("remaining: 1000", "remaining: 1");
    let mut app = scenario_with(files, 20);
    gather(&mut app, 1);
    advance(&mut app, 1200);
    assert!(
        turts_sim::economy::views::world_view(app.world_mut())
            .nodes
            .iter()
            .all(|node| node.remaining == 0)
    );
    assert!(matches!(
        order_view(app.world(), SlotId(0), 3).unwrap().stage,
        turts_sim::GatherStage::Waiting {
            reason: turts_sim::WaitReason::NoDeposit
        }
    ));
}

#[test]
fn overflow_conversion_wait_keeps_gathered_cargo() {
    let mut files = fixture();
    let yard = files.get_mut("units/yard.ron").unwrap();
    *yard = yard.replace(
        "drop_off: Some((rules: [(item: \"probe:timber\", effect: Store)]))",
        "drop_off: None",
    );
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map.replace("amount: 200", "amount: 18446744073709551615");
    let mut app = scenario_with(files, 20);
    gather(&mut app, 1);
    advance(&mut app, 1200);
    assert!(matches!(
        order_view(app.world(), SlotId(0), 3).unwrap().stage,
        turts_sim::GatherStage::Waiting {
            reason: turts_sim::WaitReason::DestinationOverflow
        }
    ));
    assert!(
        turts_sim::economy::views::inventory_view(app.world(), SlotId(0), 3)
            .unwrap()
            .items[0]
            .on_hand
            > 0
    );
}

#[test]
fn lower_content_id_cargo_is_delivered_first_and_receiver_ties_use_id() {
    let mut files = fixture_with_stone();
    let worker = files.get_mut("units/worker.ron").unwrap();
    *worker = worker
        .replace(
            "accepts: [\"probe:timber\"]",
            "accepts: [\"probe:timber\", \"probe:stone\"]",
        )
        .replace(
            "items: [\"probe:timber\"]",
            "items: [\"probe:timber\", \"probe:stone\"]",
        );
    let yard = files.get_mut("units/yard.ron").unwrap();
    *yard = yard
        .replace("accepts: [\"probe:timber\"]", "accepts: [\"probe:timber\", \"probe:stone\"]")
        .replace("rules: [(item: \"probe:timber\", effect: Store)]", "rules: [(item: \"probe:timber\", effect: Store), (item: \"probe:stone\", effect: Store)]");
    let exchange = files.get_mut("units/exchange.ron").unwrap();
    *exchange = exchange
        .replace("capacity: 2000", "capacity: 2000")
        .replace("accepts: [\"probe:timber\"]", "accepts: [\"probe:timber\", \"probe:stone\"]")
        .replace("Convert(balance: \"probe:token\", per_item: 2)", "Store")
        .replace("rules: [(item: \"probe:timber\", effect: Store)]", "rules: [(item: \"probe:timber\", effect: Store), (item: \"probe:stone\", effect: Store)]");
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map
        .replace("unit: \"probe:exchange\", owner_slot: 0, pos: (x: 8.0, y: 20.0)", "unit: \"probe:exchange\", owner_slot: 0, pos: (x: 36.0, y: 8.0)")
        .replace("unit: \"probe:worker\", owner_slot: 0, pos: (x: 12.0, y: 8.0), contents: []", "unit: \"probe:worker\", owner_slot: 0, pos: (x: 12.0, y: 8.0), contents: [(item: \"probe:timber\", amount: 1), (item: \"probe:stone\", amount: 1)]");
    let mut app = scenario_with(files, 20);
    gather(&mut app, 1);
    let mut stored = Vec::new();
    for _ in 0..1200 {
        app.update();
        stored.extend(committed_tick(app.world()).events.iter().filter_map(
            |event| match &event.kind {
                EconomyEventKind::Stored {
                    from: 3, to, item, ..
                } => Some((*to, item.clone())),
                _ => None,
            },
        ));
    }
    assert_eq!(stored.first(), Some(&(1, cid("probe:stone"))));
}

#[test]
fn receiver_filled_while_moving_is_reselected() {
    let mut files = fixture();
    let yard = files.get_mut("units/yard.ron").unwrap();
    *yard = yard.replace("capacity: 2000", "capacity: 10");
    let exchange = files.get_mut("units/exchange.ron").unwrap();
    *exchange = exchange.replace("Convert(balance: \"probe:token\", per_item: 2)", "Store");
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map
        .replace("unit: \"probe:yard\", owner_slot: 0, pos: (x: 8.0, y: 8.0), contents: [(item: \"probe:timber\", amount: 100)]", "unit: \"probe:yard\", owner_slot: 0, pos: (x: 8.0, y: 8.0), contents: [(item: \"probe:timber\", amount: 9)]")
        .replace("unit: \"probe:exchange\", owner_slot: 0, pos: (x: 8.0, y: 20.0), contents: []", "unit: \"probe:exchange\", owner_slot: 0, pos: (x: 8.0, y: 20.0), contents: [(item: \"probe:timber\", amount: 1)]");
    let mut app = scenario_with(files, 20);
    gather(&mut app, 1);
    let mut filled_while_moving = false;
    for _ in 0..400 {
        app.update();
        let order = order_view(app.world(), SlotId(0), 3).unwrap();
        if order.stage == turts_sim::GatherStage::MovingToReceiver && order.receiver == Some(1) {
            turts_sim::economy::transaction::inventory_transfer(
                app.world_mut(),
                SlotId(0),
                2,
                1,
                cid("probe:timber"),
                1,
            )
            .unwrap();
            filled_while_moving = true;
            break;
        }
    }
    assert!(filled_while_moving);
    let mut stored_to_alternate = false;
    for _ in 0..600 {
        app.update();
        stored_to_alternate |= committed_tick(app.world()).events.iter().any(|event| {
            matches!(
                event.kind,
                EconomyEventKind::Stored {
                    from: 3,
                    to: 2,
                    amount: 1..,
                    ..
                }
            )
        });
    }
    assert!(stored_to_alternate);
}

#[test]
fn absent_receivers_retain_cargo_in_no_receiver_wait() {
    let mut files = fixture();
    for unit in ["units/yard.ron", "units/exchange.ron"] {
        let definition = files.get_mut(unit).unwrap();
        *definition = definition.replace(
            "drop_off: Some((rules: [(item: \"probe:timber\", effect: Store)]))",
            "drop_off: None",
        ).replace(
            "drop_off: Some((rules: [(item: \"probe:timber\", effect: Convert(balance: \"probe:token\", per_item: 2))]))",
            "drop_off: None",
        );
    }
    let mut app = scenario_with(files, 20);
    gather(&mut app, 1);
    advance(&mut app, 2000);
    assert!(matches!(
        order_view(app.world(), SlotId(0), 3).unwrap().stage,
        turts_sim::GatherStage::Waiting {
            reason: turts_sim::WaitReason::NoReceiver
        }
    ));
    assert!(
        turts_sim::economy::views::inventory_view(app.world(), SlotId(0), 3)
            .unwrap()
            .items[0]
            .on_hand
            > 0
    );
}

#[test]
fn nearest_receiver_wins_before_id_tie_break() {
    let mut files = fixture();
    let exchange = files.get_mut("units/exchange.ron").unwrap();
    *exchange = exchange.replace("Convert(balance: \"probe:token\", per_item: 2)", "Store");
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map.replace(
        "unit: \"probe:exchange\", owner_slot: 0, pos: (x: 8.0, y: 20.0)",
        "unit: \"probe:exchange\", owner_slot: 0, pos: (x: 32.0, y: 8.0)",
    );
    let mut app = scenario_with(files, 20);
    gather(&mut app, 1);
    let mut first = None;
    for _ in 0..1200 {
        app.update();
        if let Some(to) =
            committed_tick(app.world())
                .events
                .iter()
                .find_map(|event| match event.kind {
                    EconomyEventKind::Stored { from: 3, to, .. } => Some(to),
                    _ => None,
                })
        {
            first = Some(to);
            break;
        }
    }
    assert_eq!(first, Some(2));
}
