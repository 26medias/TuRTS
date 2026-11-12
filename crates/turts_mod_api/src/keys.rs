//! The closed key lists of the mod API 0.2.4 (design 6.10, "The full key lists"). A mod writes
//! these names in its interface file (`reasons`, `cursors`, `states`); the packer refuses a name
//! that is not here and prints the list in its `(allowed: ...)` text.
//!
//! RED-PHASE SHELL: the lists are empty. The next commit fills them.

/// Reason keys: the wire keys of `CommandRejectReason::as_str`, the client keys, then the new
/// wire keys of protocol 5.
pub const REASON_KEYS: &[&str] = &[];

/// Cursor contexts of the `cursors` map of an interface file.
pub const CURSOR_CONTEXTS: &[&str] = &[];

/// State keys of the `states` map of an interface file.
pub const STATE_KEYS: &[&str] = &[];

#[cfg(test)]
mod tests {
    use super::{CURSOR_CONTEXTS, REASON_KEYS, STATE_KEYS};
    use std::collections::BTreeSet;

    fn unique(list: &[&str]) -> bool {
        list.iter().collect::<BTreeSet<_>>().len() == list.len()
    }

    #[test]
    fn the_lists_have_the_counts_of_the_design() {
        assert_eq!(
            REASON_KEYS.len(),
            49,
            "design 6.10: 37 wire + 3 client + 9 new"
        );
        assert_eq!(CURSOR_CONTEXTS.len(), 9, "design 6.10 row H2");
        assert_eq!(
            STATE_KEYS.len(),
            23,
            "design 6.10: 22 listed + gather.waiting.blocked"
        );
    }

    #[test]
    fn no_list_repeats_a_key() {
        for list in [REASON_KEYS, CURSOR_CONTEXTS, STATE_KEYS] {
            assert!(unique(list), "{list:?}");
        }
    }

    #[test]
    fn the_new_keys_come_after_the_keys_of_today() {
        let position = |key: &str| REASON_KEYS.iter().position(|name| *name == key);
        assert_eq!(position("empty_command"), Some(0));
        assert_eq!(position("blocked"), Some(36));
        assert_eq!(position("no_applicable_order"), Some(37));
        assert_eq!(position("already_researching"), Some(39));
        assert_eq!(position("site_outside_map"), Some(40));
        assert_eq!(position("match_over"), Some(48));
        // The lobby refusals are shell texts, not `reasons` keys (design 6.10).
        for key in [
            "colour_taken",
            "colour_unknown",
            "lobby_changed",
            "not_in_lobby",
        ] {
            assert_eq!(position(key), None, "{key}");
        }
    }

    #[test]
    fn the_cursor_contexts_are_the_nine_of_row_h2() {
        let want = [
            "default",
            "select",
            "move",
            "attack",
            "gather",
            "forbidden",
            "place",
            "repair",
            "rally",
        ];
        assert_eq!(CURSOR_CONTEXTS, want);
    }

    #[test]
    fn the_state_keys_group_by_prefix() {
        let count = |prefix: &str| {
            STATE_KEYS
                .iter()
                .filter(|key| key.starts_with(prefix))
                .count()
        };
        assert_eq!(count("activity."), 7);
        assert_eq!(count("gather."), 11);
        let job_states = STATE_KEYS.len() - 7 - 11;
        assert_eq!(
            job_states, 5,
            "waiting_for_funds, slowed, disabled, output_blocked, held"
        );
    }
}
