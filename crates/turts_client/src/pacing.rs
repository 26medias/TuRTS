//! Frame pacing of the windowed client: the player setting "frame limit".
//!
//! A browser has one GPU process for all its pages. A client that draws as fast as it can keeps
//! the main thread of that process busy, and a new page can wait a long time for its GPU device
//! (`docs/measurements/mc-gpu-process-saturation.md`). The player setting "frame limit" sets the
//! most frames per second of the window, one value when it has the focus and one when it does
//! not. 0 means "no limit": the window draws once per display refresh, as before this setting.
//!
//! Defaults:
//! - Focused: no limit (`UpdateMode::Continuous`). On the web a continuous update requests the
//!   next animation frame after each update, so the window draws at the display refresh rate.
//!   A paced mode at 1/60 s is not the same for a player: it stops at about 60 frames per second
//!   on a display that refreshes faster.
//! - Not focused: 60 frames per second. This is the behaviour before this setting: the Bevy
//!   default `WinitSettings::game()` updates a window with no focus each 1/60 s
//!   (`bevy_winit-0.20.0/src/winit_config.rs:18-23`, `:72-75`), and `update_mode(60)` gives the
//!   same mode. A lower value is a player choice. Measured on the MC host with 10 (two headed
//!   clients, Chrome, canvas blurred, `pacing-probe-cand-a`): 9.7 to 9.8 frames per second, and
//!   the replica of each window still advanced 19.7 to 20.0 ticks per second with no tick gap
//!   between the windows. Each update reads all the messages that arrived (`ingest.rs`) and the
//!   presentation time comes from the wall clock (`present.rs`), so a paced client does not
//!   fall behind the server. A period of 100 ms stays below the 250 ms that Bevy accepts as one
//!   time step (`Time<Virtual>` maximum delta); the native test `tests/background_pace.rs`
//!   drives the client at that period against a real server.
//!
//! A limited window uses `reactive_low_power`: it updates when its wait has elapsed, or on a
//! window or user event (input still gets a frame at the next display refresh); it ignores
//! device events (pointer motion outside the window). The headless client has no window and
//! does not use this.

use std::time::Duration;

use bevy::winit::{UpdateMode, WinitSettings};

/// The default limit when the window has the focus: no limit.
pub const DEFAULT_FRAME_LIMIT: u32 = 0;

/// The default limit when the window does not have the focus: the 60 Hz of `game()`.
pub const DEFAULT_BACKGROUND_FRAME_LIMIT: u32 = 60;

/// The smallest limit a player can store; 0 (no limit) is also valid.
pub const MIN_FRAME_LIMIT: u32 = 10;

/// The largest limit a player can store.
pub const MAX_FRAME_LIMIT: u32 = 240;

/// True when `limit` is 0 (no limit) or in `MIN_FRAME_LIMIT..=MAX_FRAME_LIMIT`.
pub fn valid_frame_limit(limit: u32) -> bool {
    limit == 0 || (MIN_FRAME_LIMIT..=MAX_FRAME_LIMIT).contains(&limit)
}

/// The update mode for one limit. 0 gives `Continuous`; an invalid limit also gives
/// `Continuous` (the settings path refuses an invalid stored value before it gets here).
pub fn update_mode(limit: u32) -> UpdateMode {
    if limit == 0 || !valid_frame_limit(limit) {
        return UpdateMode::Continuous;
    }
    UpdateMode::reactive_low_power(Duration::from_secs_f64(1.0 / f64::from(limit)))
}

/// The window update modes for the two limits.
pub fn winit_settings(frame_limit: u32, background_frame_limit: u32) -> WinitSettings {
    WinitSettings {
        focused_mode: update_mode(frame_limit),
        unfocused_mode: update_mode(background_frame_limit),
    }
}

/// The window update modes for the default limits.
pub fn default_winit_settings() -> WinitSettings {
    winit_settings(DEFAULT_FRAME_LIMIT, DEFAULT_BACKGROUND_FRAME_LIMIT)
}

/// The limit that one applied update mode gives: 0 for `Continuous`, else the rounded
/// frames per second of its wait.
pub fn applied_limit(mode: UpdateMode) -> u32 {
    match mode {
        UpdateMode::Continuous => 0,
        UpdateMode::Reactive { wait, .. } => {
            let secs = wait.as_secs_f64();
            if secs > 0.0 {
                (1.0 / secs).round() as u32
            } else {
                0
            }
        }
    }
}

/// The applied limits for the debug state (`turts_debug.frame_limits`): read from the
/// `WinitSettings` that the window loop uses, not from the stored setting. `null` without a
/// window (headless).
pub fn frame_limits_json(settings: Option<&WinitSettings>) -> serde_json::Value {
    settings.map_or(serde_json::Value::Null, |settings| {
        serde_json::json!({
            "focused": applied_limit(settings.update_mode(true)),
            "background": applied_limit(settings.update_mode(false)),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wait_of(mode: UpdateMode) -> Duration {
        match mode {
            UpdateMode::Reactive {
                wait,
                react_to_device_events,
                react_to_user_events,
                react_to_window_events,
            } => {
                assert!(!react_to_device_events, "device events must not add frames");
                // On wasm, lightyear's WebKeepalivePlugin sends `WinitUserEvent::WakeUp` while
                // the page is hidden (lightyear-0.31.0/src/client.rs:63-66,
                // lightyear_web-0.31.0/src/background_worker.rs).
                assert!(react_to_user_events, "a wake-up event must still update");
                assert!(react_to_window_events, "input must still get a frame");
                wait
            }
            UpdateMode::Continuous => panic!("expected a limited mode"),
        }
    }

    #[test]
    fn default_is_no_limit_when_focused_and_the_game_default_when_not() {
        let settings = default_winit_settings();
        assert_eq!(settings.update_mode(true), UpdateMode::Continuous);
        assert_eq!(
            settings.update_mode(false),
            WinitSettings::game().update_mode(false),
            "the default with no focus must be the behaviour before the setting"
        );
        assert_eq!(
            wait_of(settings.update_mode(false)),
            Duration::from_secs_f64(1.0 / 60.0)
        );
    }

    #[test]
    fn the_debug_state_reads_the_applied_modes() {
        assert_eq!(frame_limits_json(None), serde_json::Value::Null);
        let applied = winit_settings(20, 60);
        assert_eq!(
            frame_limits_json(Some(&applied)),
            serde_json::json!({"focused": 20, "background": 60})
        );
        assert_eq!(
            frame_limits_json(Some(&default_winit_settings())),
            serde_json::json!({"focused": 0, "background": 60})
        );
    }

    #[test]
    fn a_limit_gives_its_period() {
        let settings = winit_settings(30, 10);
        assert_eq!(
            wait_of(settings.update_mode(true)),
            Duration::from_secs_f64(1.0 / 30.0)
        );
        assert_eq!(update_mode(0), UpdateMode::Continuous);
    }

    #[test]
    fn the_valid_range_is_zero_or_ten_to_two_hundred_forty() {
        for limit in [0, 10, 30, 60, 144, 240] {
            assert!(valid_frame_limit(limit), "{limit}");
        }
        for limit in [1, 9, 241, u32::MAX] {
            assert!(!valid_frame_limit(limit), "{limit}");
            assert_eq!(update_mode(limit), UpdateMode::Continuous);
        }
    }
}
