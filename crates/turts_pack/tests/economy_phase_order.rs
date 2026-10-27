mod common;

use common::{cid, economy_files, scenario_with};
use turts_sim::{
    Command, CommandKind, CommandQueue, CommandResults, EconomyEventKind, RejectReason, SlotId,
    apply_command, audit_state, committed_tick, economy::views::balance_view,
};

fn fixture(token_balance: u64) -> bevy_app::App {
    let mut files = economy_files()
        .into_iter()
        .map(|(path, bytes)| (path, String::from_utf8(bytes).unwrap()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let hire = files.get_mut("production/hire.ron").unwrap();
    *hire = hire.replace(
        "resource: \"probe:timber\", amount: 50, source: ProducerInventory",
        "resource: \"probe:token\", amount: 2, source: SlotBalance",
    );
    let worker = files.get_mut("units/worker.ron").unwrap();
    *worker = worker
        .replace("capacity: 10", "capacity: 1")
        .replace("unload_per_second: 10", "unload_per_second: 20");
    let map = files.get_mut("maps/test.ron").unwrap();
    *map = map
        .replace(
            "unit: \"probe:worker\", owner_slot: 0, pos: (x: 12.0, y: 8.0), contents: []",
            "unit: \"probe:worker\", owner_slot: 0, pos: (x: 8.0, y: 17.0), contents: [(item: \"probe:timber\", amount: 1)]",
        )
        .replace(
            "balance: \"probe:token\", amount: 200",
            &format!("balance: \"probe:token\", amount: {token_balance}"),
        );
    scenario_with(files, 20)
}

fn gather_command() -> Command {
    Command {
        slot: SlotId(0),
        seq: 1,
        kind: CommandKind::Gather {
            unit_id: 3,
            node_id: turts_sim::NodeId(1),
        },
    }
}

fn produce_command(seq: u32) -> Command {
    Command {
        slot: SlotId(0),
        seq,
        kind: CommandKind::Produce {
            producer_id: 1,
            recipe_id: cid("probe:hire"),
        },
    }
}

#[test]
fn gather_conversion_precedes_reserved_production_start_in_same_tick() {
    let mut app = fixture(2);
    assert!(apply_command(app.world_mut(), &gather_command()).outcome.is_ok());
    app.update();
    app.update();
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(produce_command(2));

    app.update();

    let events = &committed_tick(app.world()).events;
    let converted = events
        .iter()
        .find(|event| matches!(event.kind, EconomyEventKind::Converted { amount: 1, credited: 2, .. }))
        .expect("the loaded worker converts one timber on the witness tick");
    let started = events
        .iter()
        .find(|event| matches!(event.kind, EconomyEventKind::ProductionStarted { producer: 1, .. }))
        .expect("the pre-reserved job starts in the production phase");
    assert!(converted.ordinal < started.ordinal);
    let EconomyEventKind::ProductionStarted { charged, .. } = &started.kind else {
        unreachable!()
    };
    assert_eq!(charged.len(), 1);
    assert_eq!(charged[0].source, turts_sim::EventSource::Slot(0));
    assert_eq!(charged[0].resource, cid("probe:token"));
    assert_eq!(charged[0].amount, 2);

    let balance = balance_view(app.world(), SlotId(0), &cid("probe:token")).unwrap();
    assert_eq!((balance.on_hand, balance.reserved), (2, 0));
    let state = audit_state(app.world_mut());
    let job = &state.economy.queues[0].jobs[0];
    assert_eq!(job.stage, turts_sim::JobStage::Running);
    assert_eq!((job.elapsed_ticks, job.duration_ticks), (0, 120));
    assert_eq!(state.economy.inventories.iter().find(|inv| inv.unit == 3).unwrap().items[0].on_hand, 0);
}

#[test]
fn tick_start_produce_cannot_spend_income_until_the_next_tick() {
    let mut app = fixture(0);
    assert!(apply_command(app.world_mut(), &gather_command()).outcome.is_ok());
    app.update();
    app.update();
    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(produce_command(2));

    app.update();

    assert_eq!(
        app.world().resource::<CommandResults>().0[0].outcome,
        Err(RejectReason::InsufficientBalance)
    );
    assert!(committed_tick(app.world())
        .events
        .iter()
        .any(|event| matches!(event.kind, EconomyEventKind::Converted { credited: 2, .. })));
    assert_eq!(
        (balance_view(app.world(), SlotId(0), &cid("probe:token")).unwrap().on_hand,
         balance_view(app.world(), SlotId(0), &cid("probe:token")).unwrap().reserved),
        (2, 0)
    );
    assert!(audit_state(app.world_mut()).economy.queues.iter().all(|queue| queue.jobs.is_empty()));

    app.world_mut()
        .resource_mut::<CommandQueue>()
        .0
        .push_back(produce_command(3));
    app.update();

    assert!(app.world().resource::<CommandResults>().0[0].outcome.is_ok());
    assert!(committed_tick(app.world())
        .events
        .iter()
        .any(|event| matches!(event.kind, EconomyEventKind::ProductionStarted { producer: 1, .. })));
    let balance = balance_view(app.world(), SlotId(0), &cid("probe:token")).unwrap();
    assert_eq!((balance.on_hand, balance.reserved), (0, 0));
}
