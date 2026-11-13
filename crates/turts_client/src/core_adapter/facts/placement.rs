//! Placement reads only the committed replica and the accepted catalog.

use crate::{content::ClientContent, replica::Replica};
use turts_interaction::placement::{Facts, Recipe};
use turts_mod_api::{ContentId, Footprint};
use turts_rules::placement::{BuildSource, Circle, Obstacle, ObstacleKind, Point, QueueFill};

pub(in crate::core_adapter) fn build(
    facts: &mut Facts,
    replica: &Replica,
    content: &ClientContent,
) {
    *facts = Facts::default();
    for (id, recipe) in &content.catalog.production {
        let Some(output) = content.catalog.units.get(&recipe.output_unit) else {
            continue;
        };
        let Some(placement) = output.placement else {
            continue;
        };
        let Footprint::Circle { radius } = output.footprint;
        facts.recipes.insert(
            id.to_string(),
            Recipe {
                radius,
                gap: placement.gap,
                node_clearance: placement.node_clearance,
                in_build_area: placement.in_build_area,
            },
        );
    }
    for (id, unit) in &replica.units {
        let Some(definition) = ContentId::parse(&unit.content_id)
            .ok()
            .and_then(|id| content.catalog.units.get(&id))
        else {
            continue;
        };
        let Some((_, position)) = unit.samples.back() else {
            continue;
        };
        let Footprint::Circle { radius } = definition.footprint;
        let footprint = Circle {
            centre: Point {
                x: position.x,
                y: position.y,
            },
            radius,
        };
        let site = replica.is_site(*id);
        if !site {
            if let Some(area) = definition.build_area {
                facts
                    .build_sources
                    .entry(unit.owner_slot)
                    .or_default()
                    .push(BuildSource {
                        footprint,
                        build_area: area,
                    });
            }
            if let Some(constructor) = &definition.constructor {
                facts.constructors.insert(
                    *id,
                    constructor
                        .buildings
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                );
                facts.queues.insert(
                    *id,
                    QueueFill {
                        held: replica.construction_queue(*id).len() as u32,
                        capacity: u32::from(constructor.queue_capacity),
                    },
                );
            }
        }
        facts.obstacles.push(Obstacle {
            id: *id,
            footprint,
            kind: if definition.movement.is_some() && !site {
                ObstacleKind::Mobile
            } else {
                ObstacleKind::Static {
                    gap: definition.placement.map_or(0.0, |placement| placement.gap),
                }
            },
        });
    }
    for node in replica.nodes.values().filter(|node| node.remaining > 0) {
        let Some(definition) = ContentId::parse(&node.content_id)
            .ok()
            .and_then(|id| content.catalog.nodes.get(&id))
        else {
            continue;
        };
        facts.nodes.push(Circle {
            centre: Point {
                x: node.pos.x,
                y: node.pos.y,
            },
            radius: definition.radius,
        });
    }
}
