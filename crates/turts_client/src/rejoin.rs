//! Bounded retry state for a browser reload that restored its session identity.

use bevy::prelude::Resource;

pub const REJOIN_WINDOW_SECS: f64 = 15.0;
pub const REJOIN_INTERVAL_SECS: f64 = 1.0;
pub const DUPLICATE_PLAYER_REASON: &str = "This game is open in another window";
pub const REJOIN_FINAL_TEXT: &str =
    "This game is open in another window. Close it, then reload this page";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectionDisposition {
    Ignored,
    RetryWhenDisconnected,
    Terminal(String),
}

/// The current transport entity is also the join-message authority. Only a valid stored browser
/// identity enables the bounded duplicate-window retry policy; native and fresh browser joins
/// keep the existing terminal rejection behavior.
#[derive(Resource, Debug, Clone)]
pub struct RejoinController {
    stored_identity: bool,
    started: bool,
    joined: bool,
    elapsed_secs: f64,
    attempts_started: u8,
    hellos_sent: u8,
    duplicate_rejections: u8,
    hello_elapsed_secs: Vec<f64>,
    next_retry_at_secs: f64,
    attempt_started_at_real_secs: Option<f64>,
    retry_after_disconnect: bool,
    current_attempt: Option<u64>,
    current_connected: bool,
    last_hello_attempt: Option<u64>,
    terminal_text: Option<String>,
}

impl Default for RejoinController {
    fn default() -> Self {
        Self {
            stored_identity: false,
            started: false,
            joined: false,
            elapsed_secs: 0.0,
            attempts_started: 0,
            hellos_sent: 0,
            duplicate_rejections: 0,
            hello_elapsed_secs: Vec::new(),
            next_retry_at_secs: 0.0,
            attempt_started_at_real_secs: None,
            retry_after_disconnect: false,
            current_attempt: None,
            current_connected: false,
            last_hello_attempt: None,
            terminal_text: None,
        }
    }
}

impl RejoinController {
    pub fn for_stored_identity() -> Self {
        Self {
            stored_identity: true,
            ..Self::default()
        }
    }

    pub fn stored_identity(&self) -> bool {
        self.stored_identity
    }

    pub fn current_attempt_is(&self, attempt: u64) -> bool {
        self.current_attempt == Some(attempt) && self.terminal_text.is_none()
    }

    pub fn accepts_messages_from(&self, attempt: u64) -> bool {
        self.current_attempt_is(attempt) && self.current_connected
    }

    pub fn accepts_messages_from_at(&self, attempt: u64, real_now_secs: f64) -> bool {
        self.accepts_messages_from(attempt) && !self.deadline_elapsed_at(real_now_secs)
    }

    pub fn current_attempt(&self) -> Option<u64> {
        self.current_attempt
    }

    pub fn start_attempt(&mut self, attempt: u64) -> bool {
        let real_now_secs = self.elapsed_secs;
        self.start_attempt_at(attempt, real_now_secs)
    }

    pub fn start_attempt_at(&mut self, attempt: u64, real_now_secs: f64) -> bool {
        if self.deadline_elapsed_at(real_now_secs) {
            self.refresh_elapsed_at(real_now_secs);
            return false;
        }
        if self.terminal_text.is_some() || self.joined || self.current_connected {
            return false;
        }
        if self.attempts_started == 0 {
            self.attempts_started = 1;
            if self.stored_identity {
                self.started = true;
                self.attempt_started_at_real_secs = Some(real_now_secs);
            }
        } else {
            if !self.retry_ready() {
                return false;
            }
            self.attempts_started = self.attempts_started.saturating_add(1);
        }
        // A later actual Hello send anchors the next retry interval.
        self.next_retry_at_secs = self.elapsed_secs + REJOIN_INTERVAL_SECS;
        self.current_attempt = Some(attempt);
        self.current_connected = false;
        self.retry_after_disconnect = false;
        true
    }

    pub fn on_connected(&mut self, attempt: u64) -> bool {
        if !self.current_attempt_is(attempt) || self.joined || self.current_connected {
            return false;
        }
        self.current_connected = true;
        true
    }

    pub fn on_connected_at(&mut self, attempt: u64, real_now_secs: f64) -> bool {
        self.refresh_elapsed_at(real_now_secs);
        self.on_connected(attempt)
    }

    pub fn on_hello_sent(&mut self, attempt: u64) -> bool {
        if !self.current_attempt_is(attempt)
            || !self.current_connected
            || self.joined
            || self.last_hello_attempt == Some(attempt)
        {
            return false;
        }
        self.last_hello_attempt = Some(attempt);
        self.hellos_sent = self.hellos_sent.saturating_add(1);
        self.hello_elapsed_secs.push(self.elapsed_secs);
        // The next Hello must be at least one second after this actual send.
        self.next_retry_at_secs = self.elapsed_secs + REJOIN_INTERVAL_SECS;
        true
    }

    pub fn on_hello_sent_at(&mut self, attempt: u64, real_now_secs: f64) -> bool {
        self.refresh_elapsed_at(real_now_secs);
        self.on_hello_sent(attempt)
    }

    pub fn on_disconnected(&mut self, attempt: u64) -> bool {
        if self.current_attempt != Some(attempt) || !self.current_connected {
            return false;
        }
        self.current_connected = false;
        true
    }

    pub fn on_rejection(&mut self, attempt: u64, reason: String) -> RejectionDisposition {
        if !self.current_attempt_is(attempt) || self.joined {
            return RejectionDisposition::Ignored;
        }
        if self.stored_identity
            && self.started
            && self.elapsed_secs < REJOIN_WINDOW_SECS
            && reason == DUPLICATE_PLAYER_REASON
        {
            self.retry_after_disconnect = true;
            self.duplicate_rejections = self.duplicate_rejections.saturating_add(1);
            RejectionDisposition::RetryWhenDisconnected
        } else {
            let text = if self.stored_identity && reason == DUPLICATE_PLAYER_REASON {
                REJOIN_FINAL_TEXT.to_owned()
            } else {
                reason
            };
            self.terminal_text = Some(text.clone());
            RejectionDisposition::Terminal(text)
        }
    }

    pub fn on_rejection_at(
        &mut self,
        attempt: u64,
        reason: String,
        real_now_secs: f64,
    ) -> RejectionDisposition {
        self.refresh_elapsed_at(real_now_secs);
        self.on_rejection(attempt, reason)
    }

    pub fn advance(&mut self, delta_secs: f64, welcomed: bool) -> Option<String> {
        if welcomed {
            self.joined = true;
            self.retry_after_disconnect = false;
            self.terminal_text = None;
            return None;
        }
        if !self.stored_identity || !self.started || self.joined || self.terminal_text.is_some() {
            return None;
        }
        self.elapsed_secs = (self.elapsed_secs + delta_secs.max(0.0)).min(REJOIN_WINDOW_SECS);
        if self.elapsed_secs >= REJOIN_WINDOW_SECS {
            self.terminal_text = Some(REJOIN_FINAL_TEXT.to_owned());
            self.retry_after_disconnect = false;
            return self.terminal_text.clone();
        }
        None
    }

    pub fn advance_to(&mut self, real_now_secs: f64, welcomed: bool) -> Option<String> {
        if welcomed {
            self.joined = true;
            self.retry_after_disconnect = false;
            self.terminal_text = None;
            return None;
        }
        self.refresh_elapsed_at(real_now_secs);
        self.terminal_text.clone()
    }

    fn deadline_elapsed_at(&self, real_now_secs: f64) -> bool {
        self.stored_identity
            && !self.joined
            && self
                .attempt_started_at_real_secs
                .is_some_and(|started_at| real_now_secs - started_at >= REJOIN_WINDOW_SECS)
    }

    fn refresh_elapsed_at(&mut self, real_now_secs: f64) {
        let Some(started_at) = self.attempt_started_at_real_secs else {
            return;
        };
        if self.joined || self.terminal_text.is_some() {
            return;
        }
        self.elapsed_secs = (real_now_secs - started_at)
            .max(0.0)
            .min(REJOIN_WINDOW_SECS);
        if self.elapsed_secs >= REJOIN_WINDOW_SECS {
            self.terminal_text = Some(REJOIN_FINAL_TEXT.to_owned());
            self.retry_after_disconnect = false;
        }
    }

    pub fn terminate(&mut self, text: String) {
        if !self.joined && self.terminal_text.is_none() {
            self.terminal_text = Some(text);
            self.retry_after_disconnect = false;
        }
    }

    pub fn retry_ready(&self) -> bool {
        self.stored_identity
            && self.started
            && !self.joined
            && self.terminal_text.is_none()
            && self.retry_after_disconnect
            && !self.current_connected
            && self.elapsed_secs >= self.next_retry_at_secs
    }

    pub fn visible_text(&self) -> Option<String> {
        if !self.stored_identity {
            return None;
        }
        if let Some(text) = &self.terminal_text {
            return Some(text.clone());
        }
        if !self.started || self.joined {
            return None;
        }
        let remaining = (REJOIN_WINDOW_SECS - self.elapsed_secs).ceil() as u8;
        Some(format!("Reconnecting to your game\n{remaining} s"))
    }

    pub fn elapsed_secs(&self) -> f64 {
        self.elapsed_secs
    }

    pub fn attempts_started(&self) -> u8 {
        self.attempts_started
    }

    pub fn hellos_sent(&self) -> u8 {
        self.hellos_sent
    }

    pub fn duplicate_rejections(&self) -> u8 {
        self.duplicate_rejections
    }

    pub fn hello_elapsed_secs(&self) -> &[f64] {
        &self.hello_elapsed_secs
    }

    pub fn terminal_text(&self) -> Option<&str> {
        self.terminal_text.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DUPLICATE_PLAYER_REASON, REJOIN_FINAL_TEXT, RejectionDisposition, RejoinController,
    };

    #[test]
    fn duplicate_rejection_waits_for_current_transport_disconnect_and_one_second_cadence() {
        let mut state = RejoinController::for_stored_identity();
        assert!(state.start_attempt(11));
        assert!(state.on_connected(11));
        assert!(state.on_hello_sent(11));
        assert!(
            !state.on_hello_sent(11),
            "the current connection emits only one Hello"
        );
        assert_eq!(
            state.on_rejection(11, DUPLICATE_PLAYER_REASON.to_owned()),
            RejectionDisposition::RetryWhenDisconnected
        );
        assert!(!state.retry_ready(), "the rejected socket is still current");
        assert!(
            !state.on_disconnected(10),
            "a stale disconnect cannot free the current attempt"
        );
        assert!(state.on_disconnected(11));
        assert!(
            !state.retry_ready(),
            "a retry cannot be sent before the one-second cadence"
        );
        state.advance(0.999, false);
        assert!(!state.retry_ready());
        state.advance(0.001, false);
        assert!(state.retry_ready());
        assert!(state.start_attempt(12));
        assert_eq!(state.attempts_started(), 2);
        assert!(
            !state.retry_ready(),
            "the next Hello is scheduled for the next one-second boundary"
        );
    }

    #[test]
    fn delayed_hello_uses_the_remaining_window_and_starts_retry_cadence_at_send() {
        let mut state = RejoinController::for_stored_identity();
        assert!(state.start_attempt(21));
        assert_eq!(
            state.visible_text().as_deref(),
            Some("Reconnecting to your game\n15 s"),
            "the visible deadline begins with the actual connection attempt"
        );
        state.advance(5.0, false);
        assert!(state.on_connected(21));
        assert!(state.on_hello_sent(21));
        assert_eq!(
            state.elapsed_secs(),
            5.0,
            "a delayed Hello does not restart the window"
        );
        assert_eq!(
            state.visible_text().as_deref(),
            Some("Reconnecting to your game\n10 s")
        );
        assert_eq!(
            state.on_rejection(21, DUPLICATE_PLAYER_REASON.to_owned()),
            RejectionDisposition::RetryWhenDisconnected
        );
        assert!(state.on_disconnected(21));
        assert!(
            !state.retry_ready(),
            "a slow initial connection does not trigger catch-up Hellos"
        );
        state.advance(0.999, false);
        assert!(!state.retry_ready());
        state.advance(0.001, false);
        assert!(state.retry_ready());
    }

    #[test]
    fn duplicate_rejection_is_terminal_without_a_stored_browser_identity() {
        let mut state = RejoinController::default();
        assert!(state.start_attempt(1));
        assert!(state.on_connected(1));
        assert!(state.on_hello_sent(1));
        assert_eq!(
            state.on_rejection(1, DUPLICATE_PLAYER_REASON.to_owned()),
            RejectionDisposition::Terminal(DUPLICATE_PLAYER_REASON.to_owned())
        );
        assert!(!state.retry_ready());
    }

    #[test]
    fn another_rejection_is_terminal_even_for_a_stored_identity() {
        let mut state = RejoinController::for_stored_identity();
        assert!(state.start_attempt(2));
        assert!(state.on_connected(2));
        assert!(state.on_hello_sent(2));
        assert_eq!(
            state.on_rejection(2, "protocol mismatch".to_owned()),
            RejectionDisposition::Terminal("protocol mismatch".to_owned())
        );
        assert!(!state.retry_ready());
    }

    #[test]
    fn deadline_uses_exact_player_text_and_success_cancels_pending_retries() {
        let mut timed_out = RejoinController::for_stored_identity();
        assert!(timed_out.start_attempt(3));
        assert!(timed_out.on_connected(3));
        assert!(timed_out.on_hello_sent(3));
        assert_eq!(
            timed_out.visible_text().as_deref(),
            Some("Reconnecting to your game\n15 s")
        );
        assert_eq!(timed_out.advance(14.999, false), None);
        assert_eq!(
            timed_out.visible_text().as_deref(),
            Some("Reconnecting to your game\n1 s")
        );
        assert_eq!(
            timed_out.advance(0.001, false).as_deref(),
            Some(REJOIN_FINAL_TEXT)
        );
        assert_eq!(timed_out.visible_text().as_deref(), Some(REJOIN_FINAL_TEXT));
        assert!(!timed_out.retry_ready());

        let mut joined = RejoinController::for_stored_identity();
        assert!(joined.start_attempt(4));
        assert!(joined.on_connected(4));
        assert!(joined.on_hello_sent(4));
        assert_eq!(
            joined.on_rejection(4, DUPLICATE_PLAYER_REASON.to_owned()),
            RejectionDisposition::RetryWhenDisconnected
        );
        assert!(joined.on_disconnected(4));
        joined.advance(1.0, true);
        assert_eq!(joined.visible_text(), None);
        assert!(!joined.retry_ready());
    }

    #[test]
    fn stale_connection_events_cannot_retry_or_reopen_a_terminal_join() {
        let mut state = RejoinController::for_stored_identity();
        assert!(state.start_attempt(9));
        assert!(
            !state.on_connected(8),
            "a late Connected event cannot take ownership"
        );
        assert_eq!(
            state.on_rejection(8, DUPLICATE_PLAYER_REASON.to_owned()),
            RejectionDisposition::Ignored
        );
        assert!(state.on_connected(9));
        assert!(state.on_hello_sent(9));
        assert!(
            !state.on_connected(9),
            "a repeated Connected event cannot reset the established attempt"
        );
        assert_eq!(
            state.on_rejection(9, "bad lockfile".to_owned()),
            RejectionDisposition::Terminal("bad lockfile".to_owned())
        );
        assert!(
            !state.start_attempt(10),
            "a terminal rejection cannot reopen the connection loop"
        );
        assert!(
            !state.on_connected(9),
            "terminal joins ignore later Connected events"
        );
    }

    #[test]
    fn no_retry_hello_is_sent_after_the_deadline() {
        let mut state = RejoinController::for_stored_identity();
        assert!(state.start_attempt(31));
        assert!(state.on_connected(31));
        assert!(state.on_hello_sent(31));
        assert_eq!(
            state.on_rejection(31, DUPLICATE_PLAYER_REASON.to_owned()),
            RejectionDisposition::RetryWhenDisconnected
        );
        assert!(state.on_disconnected(31));
        assert_eq!(
            state.advance(15.0, false).as_deref(),
            Some(REJOIN_FINAL_TEXT)
        );
        assert!(!state.retry_ready());
        assert!(
            !state.start_attempt(32),
            "the controller will not send a new Hello after the 15-second deadline"
        );
        assert!(
            !state.on_connected(31),
            "late Connected events cannot reopen the terminal attempt"
        );
        assert!(!state.accepts_messages_from(31));
    }

    #[test]
    fn stored_identity_countdown_starts_at_the_first_transport_attempt() {
        let mut state = RejoinController::for_stored_identity();
        assert_eq!(
            state.visible_text(),
            None,
            "package acquisition does not consume the retry window"
        );
        assert_eq!(state.advance(5.0, false), None);
        assert!(state.start_attempt(41));
        assert!(state.on_connected(41));
        assert_eq!(
            state.visible_text().as_deref(),
            Some("Reconnecting to your game\n15 s"),
            "a connecting attempt has visible time remaining before Hello"
        );
        assert!(state.on_hello_sent(41));
        assert_eq!(
            state.visible_text().as_deref(),
            Some("Reconnecting to your game\n15 s")
        );
        assert_eq!(state.elapsed_secs(), 0.0);
    }
}
