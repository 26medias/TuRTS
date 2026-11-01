//! Exact text contract for the public order log writer.

#[path = "../src/orderlog.rs"]
#[allow(dead_code)]
mod orderlog;

#[test]
fn moving_then_blocked_has_exact_lines() {
    let mut output = Vec::new();
    orderlog::write_transition(&mut output, 12, 7, "Moving", None).unwrap();
    orderlog::write_transition(&mut output, 40, 7, "Blocked", Some("blocked")).unwrap();

    assert_eq!(
        String::from_utf8(output).unwrap(),
        "12,7,Moving,-\n40,7,Blocked,blocked\n"
    );
}

fn unit_order(
    unit_id: u32,
    activity: turts_sim::economy::views::UnitOrderActivityView,
) -> turts_sim::economy::views::UnitOrderView {
    turts_sim::economy::views::UnitOrderView {
        unit_id,
        owner_slot: 0,
        activity,
        current: None,
        queued: Vec::new(),
        last_failure: None,
    }
}

#[test]
fn installed_logger_sorts_new_and_done_states_by_unit_id() {
    use bevy::prelude::App;
    use turts_sim::economy::views::{UnitOrderActivityView as Activity, WorldView};

    let path = std::env::temp_dir().join(format!(
        "turts_orderlog_sort_{}_{}.csv",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let _ = std::fs::remove_file(&path);
    let mut app = App::new();
    app.insert_resource(turts_sim::Tick(1));
    app.insert_resource(turts_sim::CompletedWorld(WorldView {
        unit_orders: vec![
            unit_order(1, Activity::Moving),
            unit_order(2, Activity::Moving),
        ],
        ..WorldView::default()
    }));
    orderlog::install(&mut app, Some(&path)).unwrap();
    orderlog::record_changes(app.world_mut());

    app.world_mut().resource_mut::<turts_sim::Tick>().0 = 2;
    app.world_mut()
        .resource_mut::<turts_sim::CompletedWorld>()
        .0
        .unit_orders = vec![unit_order(2, Activity::Blocked)];
    orderlog::record_changes(app.world_mut());

    let lines: Vec<_> = std::fs::read_to_string(&path)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        lines,
        [
            "1,1,Moving,-",
            "1,2,Moving,-",
            "2,1,Done,-",
            "2,2,Blocked,-"
        ]
    );
    drop(app);
    std::fs::remove_file(path).unwrap();
}
