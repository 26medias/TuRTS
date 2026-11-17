//! The utility types and the owner utility view for the interaction core (M6). They come from
//! the accepted catalog and the committed replica only.

use crate::{content::ClientContent, replica::Replica};
use bevy::math::Vec2;
use turts_interaction::utility::{Facts, UtilityFact};

pub(in crate::core_adapter) fn build(
    facts: &mut Facts,
    replica: &Replica,
    content: &ClientContent,
) {
    if content.catalog.utilities.is_empty() {
        if *facts != Facts::default() {
            *facts = Facts::default();
        }
        return;
    }
    // The catalog part does not change during a match: build it once.
    if facts.utilities.len() != content.catalog.utilities.len() {
        facts.utilities = content
            .catalog
            .utilities
            .iter()
            .map(|(id, utility)| {
                (
                    id.to_string(),
                    UtilityFact {
                        name: utility.name.clone(),
                        depth_default: utility.depth.default,
                        depth_min: utility.depth.min,
                        depth_max: utility.depth.max,
                        reach: utility.reach,
                        max_segment_length: utility.max_segment_length,
                        cost: utility
                            .segment_cost
                            .iter()
                            .map(|(resource, amount)| {
                                (
                                    content
                                        .catalog
                                        .resources
                                        .get(resource)
                                        .map_or_else(|| resource.local.clone(), |r| r.name.clone()),
                                    *amount,
                                )
                            })
                            .collect(),
                        ports: utility
                            .ports
                            .iter()
                            .map(|(unit, offset)| (unit.to_string(), Vec2::new(offset.x, offset.y)))
                            .collect(),
                    },
                )
            })
            .collect();
    }
    // The view changes with a committed transaction: copy it only then.
    match replica.utilities.as_ref() {
        Some(view) if facts.view != *view => facts.view = view.clone(),
        None if facts.view != Default::default() => facts.view = Default::default(),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utilities::tests::{LINE, content, replica, view};
    use turts_net_protocol::UtilitySegmentState;

    #[test]
    fn the_facts_hold_the_catalog_part_and_follow_the_replica_view() {
        let content = content();
        let mut replica = replica(&content, view(UtilitySegmentState::Conducting, false));
        let mut facts = Facts::default();
        build(&mut facts, &replica, &content);
        let fact = &facts.utilities[LINE];
        assert_eq!(
            (
                fact.name.as_str(),
                fact.depth_default,
                fact.depth_min,
                fact.depth_max
            ),
            ("Power line", 2, 1, 4)
        );
        assert_eq!(fact.reach, 20.0);
        assert_eq!(fact.ports["skirmish:base"], Vec2::new(0.0, 1.0));
        assert_eq!(facts.view, view(UtilitySegmentState::Conducting, false));
        // A replacement view reaches the facts; a catalog with no utility type clears them.
        replica.utilities = Some(view(UtilitySegmentState::Disconnected, true));
        build(&mut facts, &replica, &content);
        assert_eq!(facts.view, view(UtilitySegmentState::Disconnected, true));
        build(&mut facts, &replica, &ClientContent::default());
        assert_eq!(facts, Facts::default());
    }
}
