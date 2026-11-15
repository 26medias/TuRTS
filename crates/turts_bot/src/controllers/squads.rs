//! Squad target ranks use public geometry and content. Identifiers never choose a target.
use super::*;

impl StockController {
    pub(super) fn squads(
        &mut self,
        frame: &BotFrame<'_>,
        queries: &mut dyn BotQueries,
        out: &mut dyn ProposalSink,
    ) {
        for (rank, rule) in frame.profile.knowledge.squads.clone().iter().enumerate() {
            let Some(units) = self.units(frame, &rule.role, queries) else {
                continue;
            };
            if units.len() < rule.min_units as usize {
                continue;
            }
            let Some(anchors) = self.units(frame, &rule.anchor_role, queries) else {
                continue;
            };
            let Some(anchor) = unique_first(anchors) else {
                continue;
            };
            let target_content: Vec<ContentId> = rule
                .target_roles
                .iter()
                .flat_map(|name| {
                    frame
                        .profile
                        .knowledge
                        .roles
                        .iter()
                        .filter(move |r| &r.key == name)
                        .flat_map(|r| r.units.iter())
                })
                .filter_map(|s| ContentId::parse(s).ok())
                .collect();
            let Some(QueryData::Units(mut targets)) = self.collect(
                Query::Units(UnitQuery {
                    relation: SlotRelation::Enemy,
                    content: target_content.clone(),
                    center: None,
                    radius: None,
                }),
                queries,
            ) else {
                continue;
            };
            targets.retain(|u| !u.site);
            targets.sort_by(|a, b| target_rank(a, b, anchor.position, rule.defend_radius));
            let target = targets.first();
            if target.is_some_and(|first| {
                targets.get(1).is_some_and(|next| {
                    target_rank(first, next, anchor.position, rule.defend_radius)
                        == std::cmp::Ordering::Equal
                })
            }) {
                continue;
            }
            let remembered = if target.is_none() {
                match self.collect(
                    Query::RememberedUnits(UnitQuery {
                        relation: SlotRelation::Enemy,
                        content: target_content,
                        center: None,
                        radius: None,
                    }),
                    queries,
                ) {
                    Some(QueryData::RememberedUnits(mut memory)) => {
                        memory.rows.retain(|row| !row.site);
                        memory
                            .rows
                            .sort_by(|a, b| memory_rank(a, b, anchor.position));
                        memory
                            .rows
                            .first()
                            .filter(|first| {
                                memory.rows.get(1).is_none_or(|next| {
                                    memory_rank(first, next, anchor.position)
                                        != std::cmp::Ordering::Equal
                                })
                            })
                            .map(|row| row.position)
                    }
                    _ => None,
                }
            } else {
                None
            };
            let (binding, target) = match (target, remembered) {
                (Some(target), _) => (&rule.attack_order, BotTarget::Unit(target.unit)),
                (None, Some(point)) => (&rule.move_order, BotTarget::Point(point)),
                (None, None)
                    if units.iter().any(|u| {
                        distance(u.position, anchor.position)
                            > rule.assemble_radius * rule.assemble_radius
                    }) =>
                {
                    (&rule.move_order, BotTarget::Point(anchor.position))
                }
                (None, None) => continue,
            };
            if let Some(order) = self.order(frame, binding, target) {
                let _ = out.propose(Proposal {
                    source_basis: self.basis.unwrap_or(frame.basis),
                    task: TaskKey::new(&self.key, &rule.key, "squad"),
                    actors: ranked_sources(
                        units
                            .into_iter()
                            .filter(|u| {
                                u.own.as_ref().is_some_and(|own| {
                                    own.enabled
                                        && own.queued_orders == 0
                                        && own.current_order.as_ref() != Some(&order)
                                })
                            })
                            .collect(),
                        frame.limits.affected_units as usize,
                    )
                    .into_iter()
                    .map(|u| u.unit)
                    .collect(),
                    order,
                    priority: rule.priority,
                    goal_order: rank as u32,
                });
            }
        }
    }
}
fn target_rank(
    a: &UnitObservation,
    b: &UnitObservation,
    anchor: WorldPos,
    defend_radius: f32,
) -> std::cmp::Ordering {
    let da = distance(anchor, a.position);
    let db = distance(anchor, b.position);
    let radius = defend_radius * defend_radius;
    fn coordinate(a: f32, b: f32) -> std::cmp::Ordering {
        if a == b {
            std::cmp::Ordering::Equal
        } else {
            a.total_cmp(&b)
        }
    }
    (da > radius)
        .cmp(&(db > radius))
        .then_with(|| da.total_cmp(&db))
        .then_with(|| coordinate(a.position.x, b.position.x))
        .then_with(|| coordinate(a.position.y, b.position.y))
        .then_with(|| a.content.cmp(&b.content))
}

fn memory_rank(
    a: &turts_mod_api::RememberedUnit,
    b: &turts_mod_api::RememberedUnit,
    anchor: WorldPos,
) -> std::cmp::Ordering {
    let coordinate = |a: f32, b: f32| {
        if a == b {
            std::cmp::Ordering::Equal
        } else {
            a.total_cmp(&b)
        }
    };
    distance(anchor, a.position)
        .total_cmp(&distance(anchor, b.position))
        .then_with(|| coordinate(a.position.x, b.position.x))
        .then_with(|| coordinate(a.position.y, b.position.y))
        .then_with(|| a.content.cmp(&b.content))
        .then_with(|| b.last_seen_tick.cmp(&a.last_seen_tick))
}
