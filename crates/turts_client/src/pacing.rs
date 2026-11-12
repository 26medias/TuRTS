//! Frame pacing of the windowed client.
//!
//! A browser has one GPU process for all its pages. A client that draws as fast as it can keeps
//! that process busy, and a new page can wait a long time for its GPU device (MC row SLOT1: the
//! third window waited 30 s while two clients drew at about 47 frames per second). The client
//! therefore draws at most `FOCUSED_FRAME_HZ` frames per second while its window has the focus,
//! and at most `UNFOCUSED_FRAME_HZ` while it does not. A window event (pointer, keyboard,
//! resize) still gets a frame at the next display refresh, so input does not wait for the
//! next paced frame.
//!
//! Only the presentation slows down. Each update reads all the messages that arrived (ingest),
//! and the presentation time comes from the wall clock (`present.rs`), so a paced client does
//! not fall behind the server. The headless client does not use this (it has no window).

use std::time::Duration;

use bevy::winit::{UpdateMode, WinitSettings};

/// Most frames per second when the window has the focus and no window event arrives.
pub const FOCUSED_FRAME_HZ: f64 = 30.0;

/// Most frames per second when the window does not have the focus and no window event arrives.
/// The period (100 ms) stays below the 250 ms that Bevy accepts as one time step
/// (`Time<Virtual>` maximum delta), so the fixed ticks of the link do not drop.
pub const UNFOCUSED_FRAME_HZ: f64 = 10.0;

/// The update modes of the window. `reactive_low_power` updates when its wait has elapsed, or
/// on a window or user event; it ignores device events (pointer motion outside the window).
pub fn winit_settings() -> WinitSettings {
    WinitSettings {
        focused_mode: UpdateMode::reactive_low_power(Duration::from_secs_f64(
            1.0 / FOCUSED_FRAME_HZ,
        )),
        unfocused_mode: UpdateMode::reactive_low_power(Duration::from_secs_f64(
            1.0 / UNFOCUSED_FRAME_HZ,
        )),
    }
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
                assert!(
                    react_to_user_events,
                    "the web keepalive wake-up must still update"
                );
                assert!(react_to_window_events, "input must still get a frame");
                wait
            }
            UpdateMode::Continuous => panic!("a continuous mode never yields the GPU"),
        }
    }

    #[test]
    fn focused_and_unfocused_windows_are_paced() {
        let settings = winit_settings();
        let focused = wait_of(settings.update_mode(true));
        let unfocused = wait_of(settings.update_mode(false));
        assert!(
            focused >= Duration::from_secs_f64(1.0 / 31.0),
            "{focused:?}"
        );
        assert!(unfocused > focused, "{unfocused:?} <= {focused:?}");
        assert!(unfocused < Duration::from_millis(250), "{unfocused:?}");
    }
}
