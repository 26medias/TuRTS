//! The closed key lists of the mod API 0.2.4 (design 6.10, "The full key lists"). A mod writes
//! these names in its interface file (`reasons`, `cursors`, `states`); the packer refuses a name
//! that is not here and prints the list in its `(allowed: ...)` text.
//!
/// Reason keys: the 37 wire keys of `CommandRejectReason::as_str` in its order, the 3 keys that
/// only the client writes, then the 9 new wire keys of protocol 5 (design 6.1, 6.7.7). The lobby
/// refusals (`colour_taken`, `colour_unknown`, `lobby_changed`, `not_in_lobby`) are shell texts,
/// not `reasons` keys. `crates/turts_client/tests/api_keys.rs` checks the wire part against the
/// protocol crate.
pub const REASON_KEYS: &[&str] = &[
    "empty_command",
    "too_many_units",
    "duplicate_unit",
    "unknown_unit",
    "not_owner",
    "not_mobile",
    "target_not_finite",
    "target_outside_map",
    "missing_capability",
    "unknown_node",
    "incompatible_item",
    "node_depleted",
    "unknown_recipe",
    "recipe_not_enabled",
    "queue_full",
    "insufficient_balance",
    "insufficient_items",
    "unknown_job",
    "refund_destination_full",
    "refund_destination_overflow",
    "arithmetic_overflow",
    "id_exhausted",
    "bad_sequence",
    "not_removable",
    "not_toggleable",
    "prerequisites_missing",
    "already_researched",
    "research_in_progress",
    "internal",
    "unknown_order",
    "not_applicable",
    "target_invalid",
    "target_gone",
    "out_of_range",
    "rate_limited",
    "not_queueable",
    "blocked",
    // The client keys of today.
    "no_applicable_order",
    "queue_disabled",
    "already_researching",
    // The new wire keys of protocol 5.
    "site_outside_map",
    "site_out_of_area",
    "site_blocked",
    "site_near_building",
    "site_near_resource",
    "construction_queue_full",
    "match_not_started",
    "slot_defeated",
    "match_over",
];

/// Cursor contexts of the `cursors` map of an interface file (design 6.10 row H2). `place`,
/// `repair` and `rally` are new in 0.2.4.
pub const CURSOR_CONTEXTS: &[&str] = &[
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

/// State keys of the `states` map of an interface file (design 6.10 row H3): the job states of
/// design 1.1, the wire `Activity` values and the wire `GatherStage` values.
///
/// The design lists 22 keys. This list has 23: `gather.waiting.blocked` is the
/// `GatherWaitReason::Blocked` of the wire (the simulation sends it), and the design list left
/// it out.
pub const STATE_KEYS: &[&str] = &[
    "waiting_for_funds",
    "slowed",
    "disabled",
    "output_blocked",
    "held",
    "activity.idle",
    "activity.pending",
    "activity.moving",
    "activity.attacking",
    "activity.holding",
    "activity.gathering",
    "activity.blocked",
    "gather.to_node",
    "gather.gathering",
    "gather.to_drop_off",
    "gather.unloading",
    "gather.idle",
    "gather.waiting.no_drop_off",
    "gather.waiting.destination_full",
    "gather.waiting.destination_overflow",
    "gather.waiting.no_deposit",
    "gather.waiting.depleted",
    "gather.waiting.blocked",
];

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
