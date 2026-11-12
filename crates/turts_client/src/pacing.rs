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
//! - Not focused: 10 frames per second. Measured on the MC host (two headed clients, Chrome,
//!   window canvas blurred): 9.7 to 9.8 frames per second, and the replica of each window still
//!   advanced 19.7 to 20.1 ticks per second with no tick gap between the windows. Each update
//!   reads all the messages that arrived (`ingest.rs`) and the presentation time comes from the
//!   wall clock (`present.rs`), so a paced client does not fall behind the server. The 100 ms
//!   period stays below the 250 ms that Bevy accepts as one time step (`Time<Virtual>` maximum
//!   delta), so the fixed ticks of the link do not drop.
//!
//! A limited window uses `reactive_low_power`: it updates when its wait has elapsed, or on a
//! window or user event (input still gets a frame at the next display refresh); it ignores
//! device events (pointer motion outside the window). The headless client has no window and
//! does not use this.

use std::time::Duration;

use bevy::winit::{UpdateMode, WinitSettings};

/// The default limit when the window has the focus: no limit.
pub const DEFAULT_FRAME_LIMIT: u32 = 0;

/// The default limit when the window does not have the focus.
pub const DEFAULT_BACKGROUND_FRAME_LIMIT: u32 = 10;

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
            UpdateMode::Continuous => panic!("expected a limited mode"),
        }
    }

    #[test]
    fn default_is_no_limit_when_focused_and_ten_when_not() {
        let settings = default_winit_settings();
        assert_eq!(settings.update_mode(true), UpdateMode::Continuous);
        let unfocused = wait_of(settings.update_mode(false));
        assert_eq!(unfocused, Duration::from_millis(100));
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
