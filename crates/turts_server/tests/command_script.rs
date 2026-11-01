//! Process acceptance for fixed-tick commands without a client.

mod support;

use std::fs;
use std::thread;
use std::time::{Duration, Instant};

use turts_net_protocol::{OrderEnvelope, OrderTarget, Pos, QueueMode, UnitId};

#[derive(serde::Serialize)]
struct ScriptEntry<'a> {
    tick: u32,
    slot: u8,
    envelope: &'a OrderEnvelope,
}

#[test]
#[ignore = "Task 7 process acceptance"]
fn command_script_moves_acme_unit_at_fixed_tick_without_a_client() {
    let dir = support::test_dir("command_script");
    let package = support::pack("acme", &dir);
    let script = dir.join("commands.json");
    let order_log = dir.join("orders.log");
    let envelope = OrderEnvelope {
        order: "acme:order_move".into(),
        units: vec![UnitId(1)],
        target: OrderTarget::Point(Pos { x: 12.0, y: 8.0 }),
        queue: QueueMode::Replace,
        group: 0,
    };
    fs::write(
        &script,
        serde_json::to_vec(&[ScriptEntry {
            tick: 10,
            slot: 0,
            envelope: &envelope,
        }])
        .unwrap(),
    )
    .unwrap();

    let mut server = support::spawn_server(
        &package,
        &[
            "--command-script",
            script.to_str().unwrap(),
            "--order-log",
            order_log.to_str().unwrap(),
            "--run-ticks",
            "30",
        ],
    );
    let started = Instant::now();
    let status = loop {
        if let Some(status) = server.child.try_wait().unwrap() {
            break status;
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "server did not exit"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert!(status.success(), "server log:\n{}", server.log());

    let rows = support::logs::read_tick_log(&server.tick_log);
    let scout: Vec<_> = rows.iter().filter(|row| row.unit_id == 1).collect();
    assert_eq!(
        scout.len(),
        30,
        "the scripted scout is present at every tick"
    );
    for tick in 1..10 {
        let row = scout.iter().find(|row| row.tick == tick).unwrap();
        assert_eq!(
            row.pos,
            (4.0, 4.0),
            "the script must not move before tick 10"
        );
    }
    let first_move = scout.iter().find(|row| row.tick == 10).unwrap();
    // Acme's scout moves at 6 units/s; one 20 Hz step along (8, 4) is (0.268328, 0.134164).
    assert!((first_move.pos.0 - 4.268_328).abs() < 0.000_1);
    assert!((first_move.pos.1 - 4.134_164).abs() < 0.000_1);
    let last = scout.iter().find(|row| row.tick == 30).unwrap();
    assert!(last.pos.0 > first_move.pos.0 && last.pos.1 > first_move.pos.1);
    let order_lines: Vec<_> = fs::read_to_string(&order_log)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    let expected = "10,1,Moving,-";
    assert_eq!(order_lines.first().map(String::as_str), Some(expected));
}
