//! Bounded retry state for a browser reload that restored its session identity.

use bevy::prelude::Resource;

pub const REJOIN_WINDOW_SECS: f64 = 15.0;
pub const REJOIN_INTERVAL_SECS: f64 = 1.0;
pub const DUPLICATE_PLAYER_REASON: &str = "This game is open in another window";
pub const REJOIN_FINAL_TEXT: &str =
    "This game is open in another window. Close it, then reload this page";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeadlineCheckObservation {
    pub check_index: u64,
    pub frame_index: Option<u32>,
    pub source: &'static str,
    pub real_now_secs: f64,
    pub elapsed_secs: f64,
}

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
    terminal_at_real_secs: Option<f64>,
    deadline_check_index: u64,
    last_pending_deadline_check: Option<DeadlineCheckObservation>,
    first_terminal_deadline_check: Option<DeadlineCheckObservation>,
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
            terminal_at_real_secs: None,
            deadline_check_index: 0,
            last_pending_deadline_check: None,
            first_terminal_deadline_check: None,
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

    /// Real monotonic-clock timestamp for the first transport attempt in this rejoin window.
    pub fn first_attempt_real_secs(&self) -> Option<f64> {
        self.attempt_started_at_real_secs
    }

    /// Real monotonic-clock timestamp at which the actual terminal outcome was observed.
    pub fn terminal_at_real_secs(&self) -> Option<f64> {
        self.terminal_at_real_secs
    }

    /// Unclamped elapsed monotonic time from first transport attempt to terminal outcome.
    pub fn terminal_elapsed_secs(&self) -> Option<f64> {
        self.attempt_started_at_real_secs
            .zip(self.terminal_at_real_secs)
            .map(|(started, terminal)| terminal - started)
    }

    pub fn observe_deadline_check(
        &mut self,
        frame_index: Option<u32>,
        source: &'static str,
        real_now_secs: f64,
    ) {
        if !self.stored_identity || !self.started || self.joined || self.terminal_text.is_some() {
            return;
        }
        let Some(started_at) = self.attempt_started_at_real_secs else {
            return;
        };
        let Some(check_index) = self.deadline_check_index.checked_add(1) else {
            return;
        };
        self.deadline_check_index = check_index;
        let elapsed_secs = real_now_secs - started_at;
        let observation = DeadlineCheckObservation {
            check_index,
            frame_index,
            source,
            real_now_secs,
            elapsed_secs,
        };
        if elapsed_secs.is_nan() || elapsed_secs < REJOIN_WINDOW_SECS {
            self.last_pending_deadline_check = Some(observation);
        } else if self.first_terminal_deadline_check.is_none() {
            self.first_terminal_deadline_check = Some(observation);
        }
    }

    pub fn last_pending_deadline_check(&self) -> Option<DeadlineCheckObservation> {
        self.last_pending_deadline_check
    }

    pub fn first_terminal_deadline_check(&self) -> Option<DeadlineCheckObservation> {
        self.first_terminal_deadline_check
    }

    pub fn deadline_checks_are_causal(&self) -> bool {
        let (Some(anchor), Some(terminal_at), Some(last), Some(first)) = (
            self.attempt_started_at_real_secs,
            self.terminal_at_real_secs,
            self.last_pending_deadline_check,
            self.first_terminal_deadline_check,
        ) else {
            return false;
        };
        deadline_observations_are_causal(anchor, terminal_at, last, first)
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
        let disposition = self.on_rejection(attempt, reason);
        if self.terminal_text.is_some() {
            self.terminal_at_real_secs.get_or_insert(real_now_secs);
        }
        disposition
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
        let elapsed = real_now_secs - started_at;
        self.elapsed_secs = if elapsed.is_nan() {
            0.0
        } else {
            elapsed.clamp(0.0, REJOIN_WINDOW_SECS)
        };
        if self.elapsed_secs >= REJOIN_WINDOW_SECS {
            self.terminal_text = Some(REJOIN_FINAL_TEXT.to_owned());
            self.terminal_at_real_secs.get_or_insert(real_now_secs);
            self.retry_after_disconnect = false;
        }
    }

    pub fn terminate(&mut self, text: String) {
        if !self.joined && self.terminal_text.is_none() {
            self.terminal_text = Some(text);
            self.retry_after_disconnect = false;
        }
    }

    pub fn terminate_at(&mut self, text: String, real_now_secs: f64) {
        self.terminate(text);
        if self.terminal_text.is_some() {
            self.terminal_at_real_secs.get_or_insert(real_now_secs);
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

    /// Only the bounded reconnect countdown and duplicate-window final guidance own
    /// the rejoin presentation. Other terminal reasons still need their mapped screen.
    pub(crate) fn owns_rejoin_guidance(&self) -> bool {
        self.stored_identity
            && (self.terminal_text.as_deref() == Some(REJOIN_FINAL_TEXT)
                || (self.started && !self.joined && self.terminal_text.is_none()))
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

fn deadline_observations_are_causal(
    anchor_secs: f64,
    terminal_at_secs: f64,
    last_pending: DeadlineCheckObservation,
    first_terminal: DeadlineCheckObservation,
) -> bool {
    let cutoff = anchor_secs + REJOIN_WINDOW_SECS;
    let (Some(last_frame), Some(first_frame)) =
        (last_pending.frame_index, first_terminal.frame_index)
    else {
        return false;
    };
    let frame_delta = first_frame.wrapping_sub(last_frame);
    anchor_secs.is_finite()
        && terminal_at_secs.is_finite()
        && cutoff.is_finite()
        && last_pending.real_now_secs.is_finite()
        && first_terminal.real_now_secs.is_finite()
        && last_pending.elapsed_secs.is_finite()
        && first_terminal.elapsed_secs.is_finite()
        && !last_pending.source.is_empty()
        && !first_terminal.source.is_empty()
        && last_pending.check_index.checked_add(1) == Some(first_terminal.check_index)
        && frame_delta <= 1
        && last_pending.elapsed_secs == last_pending.real_now_secs - anchor_secs
        && first_terminal.elapsed_secs == first_terminal.real_now_secs - anchor_secs
        && last_pending.real_now_secs < cutoff
        && first_terminal.real_now_secs >= cutoff
        && terminal_at_secs == first_terminal.real_now_secs
}

#[cfg(test)]
mod tests {
    use super::{
        DUPLICATE_PLAYER_REASON, DeadlineCheckObservation, REJOIN_FINAL_TEXT, REJOIN_WINDOW_SECS,
        RejectionDisposition, RejoinController, deadline_observations_are_causal,
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

    #[test]
    fn deadline_observer_retains_the_latest_pending_and_first_eligible_checks() {
        let anchor = 50.0;
        let mut state = RejoinController::for_stored_identity();
        assert!(state.start_attempt_at(41, anchor));
        state.observe_deadline_check(Some(300), "advance_rejoin", anchor + 14.9);
        state.observe_deadline_check(Some(301), "advance_rejoin", anchor + 14.999);
        state.observe_deadline_check(Some(302), "join_rejection", anchor + 15.0165);
        state.advance_to(anchor + 15.0165, false);

        let pending = state.last_pending_deadline_check().unwrap();
        let terminal = state.first_terminal_deadline_check().unwrap();
        assert_eq!(
            pending.check_index, 2,
            "only the latest pending check is retained"
        );
        assert_eq!(pending.frame_index, Some(301));
        assert_eq!(pending.elapsed_secs, pending.real_now_secs - anchor);
        assert_eq!(
            terminal.check_index, 3,
            "the first eligible check is retained once"
        );
        assert_eq!(terminal.frame_index, Some(302));
        assert_eq!(terminal.source, "join_rejection");
        assert_eq!(terminal.real_now_secs, anchor + 15.0165);
        assert!(state.deadline_checks_are_causal());
    }

    #[test]
    fn deadline_observation_controls_reject_early_extra_and_clamped_samples() {
        let anchor = 10.0;
        let pending = DeadlineCheckObservation {
            check_index: 7,
            frame_index: Some(70),
            source: "advance_rejoin",
            real_now_secs: anchor + 14.9,
            elapsed_secs: (anchor + 14.9) - anchor,
        };
        let valid = DeadlineCheckObservation {
            check_index: 8,
            frame_index: Some(71),
            source: "advance_rejoin",
            real_now_secs: anchor + 15.0165,
            elapsed_secs: (anchor + 15.0165) - anchor,
        };
        assert!(deadline_observations_are_causal(
            anchor, 25.0165, pending, valid
        ));

        let early = DeadlineCheckObservation {
            real_now_secs: anchor + 14.1,
            elapsed_secs: (anchor + 14.1) - anchor,
            ..valid
        };
        assert!(!deadline_observations_are_causal(
            anchor, 24.1, pending, early
        ));

        let extra_eligible_update = DeadlineCheckObservation {
            check_index: 9,
            ..valid
        };
        assert!(!deadline_observations_are_causal(
            anchor,
            25.0165,
            pending,
            extra_eligible_update
        ));

        let clamped = DeadlineCheckObservation {
            elapsed_secs: REJOIN_WINDOW_SECS,
            ..valid
        };
        assert!(!deadline_observations_are_causal(
            anchor, 25.0165, pending, clamped
        ));
    }
}
