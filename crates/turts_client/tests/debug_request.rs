//! Row DBG1 (design D 9, `docs/superpowers/specs/2026-11-12-mp1-playable-skirmish-design.md`):
//! the client builds its debug state only when a harness asks for it, at most 4 times each
//! second. The test app is the real headless client (`build_app`, `MinimalPlugins`); it has no
//! server, so it stays in its first phase. The clock is manual: one frame is 16 ms.

use std::time::Duration;

use bevy::app::App;
use bevy::time::TimeUpdateStrategy;
use turts_client::build_app;
use turts_client::debug::{DEBUG_MIN_INTERVAL_SECS, DebugRequest, DebugState};
use turts_client::options::ClientOptions;

const FRAME: Duration = Duration::from_millis(16);

fn headless_app() -> App {
    let mut options = ClientOptions::default();
    options.headless = true;
    let mut app = build_app(options);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));
    app
}

fn builds(app: &App) -> u64 {
    app.world().resource::<DebugRequest>().builds()
}

#[test]
fn dbg1_a_client_with_no_request_builds_no_debug_state() {
    let mut app = headless_app();
    for _ in 0..100 {
        app.update();
    }
    assert_eq!(
        builds(&app),
        0,
        "debug builds with no request in 100 frames (today: one build in each frame)"
    );
    assert!(
        app.world().resource::<DebugState>().0.is_empty(),
        "no request: the debug state stays empty"
    );
}

#[test]
fn dbg1_each_request_gives_one_build_and_the_record_counts_it() {
    let mut app = headless_app();
    for frame in 0..100 {
        if [10, 40, 70].contains(&frame) {
            app.world_mut().resource_mut::<DebugRequest>().request(None);
        }
        app.update();
    }
    assert_eq!(
        builds(&app),
        3,
        "3 requests, 30 frames (480 ms) apart: 3 builds"
    );
    let record: serde_json::Value =
        serde_json::from_str(&app.world().resource::<DebugState>().0).expect("the record is JSON");
    assert_eq!(
        record["debug_builds"], 3,
        "the published record holds the counter"
    );
    assert!(record["phase"].is_string());
    assert!(!app.world().resource::<DebugRequest>().is_pending());
}

#[test]
fn dbg1_a_request_each_frame_is_served_at_most_4_times_each_second() {
    let mut app = headless_app();
    for _ in 0..100 {
        app.world_mut().resource_mut::<DebugRequest>().request(None);
        app.update();
    }
    // 100 frames of 16 ms are 1.6 s: one build at the start and one for each 250 ms.
    let limit = (100.0 * FRAME.as_secs_f64() / DEBUG_MIN_INTERVAL_SECS) as u64 + 1;
    assert!(
        builds(&app) <= limit,
        "more than 4 debug builds each second: {} builds, limit {limit}",
        builds(&app)
    );
    assert!(
        builds(&app) >= 5,
        "the cap must still serve: {} builds",
        builds(&app)
    );
}

#[test]
fn dbg1_a_request_with_no_wait_is_served_in_the_next_frame() {
    // The SH1 phase capture asks for each frame; it passes 0 (a test harness only).
    let mut app = headless_app();
    for _ in 0..100 {
        app.world_mut()
            .resource_mut::<DebugRequest>()
            .request(Some(0.0));
        app.update();
    }
    assert_eq!(
        builds(&app),
        100,
        "a request with no wait gives one build for each frame"
    );
}
