//! A small opponent that submits ordinary mod orders. It has no economy or spawn privileges.
use std::collections::BTreeSet;

use bevy::prelude::*;
use turts_mod_api::{Behaviour, ContentCatalog, ContentId, ResolvedUnit};
use turts_net_protocol::{OrderEnvelope, OrderTarget, PlayerCommand, QueueMode, UnitId};
use turts_sim::{CommandQueue, MatchConfig, SimClock, SimSet, SlotId, Slots, Tick, WorldView};

use crate::{
    orders::{self, ServerCatalog},
    session::{self, RateLimitBuckets},
};

#[derive(Resource)]
pub struct ComputerPlayer {
    pub player_id: u64,
    slot: u8,
    seq: u32,
    started: Option<u32>,
    last_decision: Option<u32>,
    next_wave: u32,
    next_training: u32,
    assault: BTreeSet<u32>,
}

pub fn install(app: &mut App, slot: Option<u8>) -> Result<(), String> {
    let Some(slot) = slot else {
        return Ok(());
    };
    let player_id = u64::MAX - u64::from(slot);
    app.world_mut()
        .resource_mut::<Slots>()
        .claim_at(SlotId(slot), player_id)
        .map_err(|error| format!("cannot reserve computer slot {slot}: {error:?}"))?;
    app.insert_resource(ComputerPlayer {
        player_id,
        slot,
        seq: 0,
        started: None,
        last_decision: None,
        next_wave: 0,
        next_training: 0,
        assault: BTreeSet::new(),
    });
    app.add_systems(
        FixedPreUpdate,
        decide
            .after(SimSet::Construction)
            .before(session::admit_player_commands)
            .before(SimSet::Commands)
            .run_if(turts_sim::match_started)
            .run_if(turts_sim::outcome::match_running),
    );
    Ok(())
}

fn definition<'a>(
    catalog: &'a ContentCatalog,
    unit: &turts_sim::world::UnitState,
) -> Option<&'a ResolvedUnit> {
    catalog.units.get(&ContentId::parse(&unit.content_id).ok()?)
}

fn distance(a: Vec2, b: Vec2) -> f32 {
    a.distance_squared(b)
}

/// Geometry resolves spatial choices. Unit ids identify subjects but do not rank them.
fn nearer(a: Vec2, b: Vec2, origin: Vec2) -> std::cmp::Ordering {
    distance(a, origin)
        .total_cmp(&distance(b, origin))
        .then(a.x.total_cmp(&b.x))
        .then(a.y.total_cmp(&b.y))
}

fn issue(
    world: &mut World,
    bot: &mut ComputerPlayer,
    catalog: &ContentCatalog,
    unit: &turts_sim::world::UnitState,
    behaviour: Behaviour,
    target: OrderTarget,
) {
    let Some((id, _)) = catalog
        .orders
        .iter()
        .find(|(_, order)| order.behaviour == behaviour && order.units.contains(&unit.content_id))
    else {
        return;
    };
    let tick = world.resource::<Tick>().0;
    let hz = world.resource::<SimClock>().tick_hz;
    if !world
        .resource_mut::<RateLimitBuckets>()
        .admit(bot.slot, tick, hz, 1)
    {
        return;
    }
    let command = PlayerCommand {
        seq: bot.seq,
        order: OrderEnvelope {
            order: id.to_string(),
            units: vec![UnitId(unit.unit_id)],
            target,
            queue: QueueMode::Replace,
            group: 0,
        },
    };
    bot.seq = bot.seq.wrapping_add(1);
    if let Ok(commands) = orders::to_sim(SlotId(bot.slot), &command, catalog) {
        world.resource_mut::<CommandQueue>().0.extend(commands);
    }
}

fn current_target(
    view: &WorldView,
    unit: u32,
) -> Option<&turts_sim::economy::views::UnitOrderTargetView> {
    view.unit_orders
        .iter()
        .find(|order| order.unit_id == unit)?
        .current
        .as_ref()
        .map(|order| &order.target)
}

fn decide(world: &mut World) {
    let Some(mut bot) = world.remove_resource::<ComputerPlayer>() else {
        return;
    };
    decide_inner(world, &mut bot);
    world.insert_resource(bot);
}

fn decide_inner(world: &mut World, bot: &mut ComputerPlayer) {
    // A player can read the opening before the opponent starts its economy and attack clock.
    if !world
        .resource::<Slots>()
        .owners()
        .iter()
        .flatten()
        .any(|id| *id != bot.player_id)
    {
        return;
    }
    let tick = world.resource::<Tick>().0;
    let hz = u32::from(world.resource::<SimClock>().tick_hz.max(1));
    let started = *bot.started.get_or_insert(tick);
    if bot
        .last_decision
        .is_some_and(|last| tick.saturating_sub(last) < hz)
    {
        return;
    }
    bot.last_decision = Some(tick);
    let view = turts_sim::completed_world(world).clone();
    let catalog = world.resource::<ServerCatalog>().0.clone();
    let config = world.resource::<MatchConfig>();
    let team = |slot: u8| config.teams.get(usize::from(slot)).copied().unwrap_or(slot);
    let own = view
        .units
        .iter()
        .filter(|unit| {
            unit.owner_slot == bot.slot && !view.construction.sites.contains_key(&unit.unit_id)
        })
        .collect::<Vec<_>>();
    let enemies = view
        .units
        .iter()
        .filter(|unit| team(unit.owner_slot) != team(bot.slot))
        .collect::<Vec<_>>();
    let Some(base) = own
        .iter()
        .copied()
        .find(|unit| definition(&catalog, unit).is_some_and(|def| def.constructor.is_some()))
    else {
        return;
    };
    let Some(enemy_base) = enemies
        .iter()
        .copied()
        .filter(|unit| definition(&catalog, unit).is_some_and(|def| def.build_area.is_some()))
        .min_by(|a, b| nearer(a.pos, b.pos, base.pos))
    else {
        return;
    };

    for gatherer in own
        .iter()
        .copied()
        .filter(|unit| definition(&catalog, unit).is_some_and(|def| def.gatherer.is_some()))
    {
        let def = definition(&catalog, gatherer)
            .unwrap()
            .gatherer
            .as_ref()
            .unwrap();
        let node = view
            .nodes
            .iter()
            .filter(|node| {
                node.remaining > 0
                    && catalog
                        .nodes
                        .get(&node.content_id)
                        .is_some_and(|node| def.items.contains(&node.item))
            })
            .min_by(|a, b| {
                nearer(
                    Vec2::new(a.pos.x, a.pos.y),
                    Vec2::new(b.pos.x, b.pos.y),
                    gatherer.pos,
                )
            });
        if let Some(node) = node
            && !view
                .orders
                .iter()
                .any(|order| order.unit == gatherer.unit_id && order.node == Some(node.id))
        {
            issue(
                world,
                bot,
                &catalog,
                gatherer,
                Behaviour::Gather,
                OrderTarget::Node(turts_net_protocol::NodeId(node.id.0)),
            );
        }
    }

    // Build one military factory. Every candidate still passes the ordinary placement rules.
    if let Some(constructor) = definition(&catalog, base).and_then(|def| def.constructor.as_ref()) {
        for recipe in constructor
            .buildings
            .iter()
            .filter_map(|id| catalog.production.get(id))
        {
            let Some(output) = catalog.units.get(&recipe.output_unit) else {
                continue;
            };
            if !output.producer.as_ref().is_some_and(|producer| {
                producer
                    .recipes
                    .iter()
                    .filter_map(|id| catalog.production.get(id))
                    .any(|recipe| {
                        catalog
                            .units
                            .get(&recipe.output_unit)
                            .is_some_and(|def| def.weapon.is_some() && def.movement.is_some())
                    })
            }) {
                continue;
            }
            if view.units.iter().any(|unit| {
                unit.owner_slot == bot.slot && unit.content_id == recipe.output_unit.to_string()
            }) {
                break;
            }
            let forward = (enemy_base.pos - base.pos).normalize_or_zero();
            let side = Vec2::new(-forward.y, forward.x);
            for (along, across) in [(7.0, 0.0), (7.0, 6.0), (7.0, -6.0), (0.0, 7.0), (0.0, -7.0)] {
                let point = base.pos + forward * along + side * across;
                if turts_sim::economy::construction::check_placement(
                    world,
                    SlotId(bot.slot),
                    output,
                    point,
                    None,
                )
                .is_ok()
                {
                    issue(
                        world,
                        bot,
                        &catalog,
                        base,
                        Behaviour::Construct,
                        OrderTarget::ContentAtPoint {
                            content: recipe.id.to_string(),
                            point: turts_net_protocol::Pos {
                                x: point.x,
                                y: point.y,
                            },
                        },
                    );
                    break;
                }
            }
            break;
        }
    }

    let combatants = own
        .iter()
        .copied()
        .filter(|unit| {
            definition(&catalog, unit)
                .is_some_and(|def| def.weapon.is_some() && def.movement.is_some())
        })
        .collect::<Vec<_>>();
    if tick >= bot.next_training {
        for producer in &own {
            if view
                .queues
                .iter()
                .any(|queue| queue.producer == producer.unit_id && !queue.jobs.is_empty())
            {
                continue;
            }
            let Some(production) =
                definition(&catalog, producer).and_then(|def| def.producer.as_ref())
            else {
                continue;
            };
            let recipe = production
                .recipes
                .iter()
                .filter_map(|id| catalog.production.get(id))
                .find(|recipe| {
                    catalog.units.get(&recipe.output_unit).is_some_and(|def| {
                        (def.weapon.is_some() && def.movement.is_some() && combatants.len() < 6)
                            || (def.gatherer.is_some()
                                && !own.iter().any(|unit| {
                                    definition(&catalog, unit)
                                        .is_some_and(|def| def.gatherer.is_some())
                                }))
                    })
                });
            if let Some(recipe) = recipe {
                issue(
                    world,
                    bot,
                    &catalog,
                    producer,
                    Behaviour::Produce,
                    OrderTarget::Content(recipe.id.to_string()),
                );
                bot.next_training = tick.saturating_add(8 * hz);
                break;
            }
        }
    }
    bot.assault
        .retain(|id| combatants.iter().any(|unit| unit.unit_id == *id));
    if tick.saturating_sub(started) >= 75 * hz && tick >= bot.next_wave {
        let mut available = combatants
            .iter()
            .copied()
            .filter(|unit| !bot.assault.contains(&unit.unit_id))
            .collect::<Vec<_>>();
        available.sort_by(|a, b| nearer(a.pos, b.pos, enemy_base.pos));
        bot.assault
            .extend(available.into_iter().take(2).map(|unit| unit.unit_id));
        bot.next_wave = tick.saturating_add(60 * hz);
    }
    for combatant in combatants {
        let nearby = enemies
            .iter()
            .copied()
            .filter(|enemy| distance(enemy.pos, combatant.pos) < 12.0 * 12.0)
            .min_by(|a, b| nearer(a.pos, b.pos, combatant.pos));
        let target = nearby.or_else(|| {
            bot.assault
                .contains(&combatant.unit_id)
                .then_some(enemy_base)
        });
        if let Some(target) = target
            && !matches!(current_target(&view,combatant.unit_id), Some(turts_sim::economy::views::UnitOrderTargetView::Unit(id)) if *id == target.unit_id)
        {
            issue(
                world,
                bot,
                &catalog,
                combatant,
                Behaviour::Attack,
                OrderTarget::Unit(UnitId(target.unit_id)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
    use turts_sim::outcome::{MatchOutcomes, Outcome};

    fn game() -> App {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
        let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
        let mut app = turts_sim::headless_app(20);
        turts_pack::load_package(app.world_mut(), &archive).unwrap();
        app.insert_resource(ServerCatalog(archive.content().unwrap()));
        app.init_resource::<RateLimitBuckets>();
        install(&mut app, Some(1)).unwrap();
        app
    }
    fn tick(app: &mut App) {
        app.update();
    }
    #[test]
    fn waits_for_a_human_and_reserves_only_its_own_seat() {
        let mut app = game();
        let owners = app.world().resource::<Slots>().owners();
        assert_eq!(owners, vec![None, Some(u64::MAX - 1)]);
        assert!(
            app.world_mut()
                .resource_mut::<Slots>()
                .claim_at(SlotId(2), 42)
                .is_err()
        );
        for _ in 0..100 {
            tick(&mut app);
        }
        assert!(app.world().resource::<ComputerPlayer>().started.is_none());
        assert!(
            turts_sim::completed_world(app.world())
                .construction
                .is_empty()
        );
        assert_eq!(
            app.world_mut().resource_mut::<Slots>().claim(42),
            Ok(SlotId(0))
        );
        tick(&mut app);
        assert!(app.world().resource::<ComputerPlayer>().started.is_some());
    }
    #[test]
    fn replaying_the_same_player_join_repeats_the_opponents_economy_and_fight() {
        let mut states = Vec::new();
        for _ in 0..2 {
            let mut app = game();
            app.world_mut().resource_mut::<Slots>().claim(42).unwrap();
            for _ in 0..3500 {
                tick(&mut app);
            }
            states.push(turts_sim::audit_state(app.world_mut()));
        }
        assert_eq!(states[0], states[1]);
    }
    #[test]
    fn ordinary_orders_build_harvest_train_and_defeat_an_idle_player() {
        let mut app = game();
        app.world_mut().resource_mut::<Slots>().claim(42).unwrap();
        let mut saw_site = false;
        let mut saw_factory = false;
        let mut saw_harvest = false;
        let mut max_soldiers = 1;
        for _ in 0..12_000 {
            tick(&mut app);
            let view = turts_sim::completed_world(app.world());
            saw_site |= !view.construction.sites.is_empty();
            saw_factory |= view.units.iter().any(|unit| {
                unit.owner_slot == 1
                    && unit.content_id == "skirmish:barracks"
                    && !view.construction.sites.contains_key(&unit.unit_id)
            });
            saw_harvest |= view.nodes.iter().any(|node| node.remaining < 2000);
            max_soldiers = max_soldiers.max(
                view.units
                    .iter()
                    .filter(|unit| unit.owner_slot == 1 && unit.content_id == "skirmish:soldier")
                    .count(),
            );
            if app.world().resource::<MatchOutcomes>().end_tick.is_some() {
                break;
            }
        }
        assert!(saw_site, "no construction order took effect");
        assert!(saw_factory, "the factory never completed");
        assert!(
            saw_harvest,
            "the worker did not harvest: {:?}",
            turts_sim::completed_world(app.world())
        );
        assert!(
            max_soldiers > 3,
            "the opponent did not fund its army from timber deliveries"
        );
        let result = app.world().resource::<MatchOutcomes>();
        println!(
            "computer victory at tick {:?}; largest army {max_soldiers}",
            result.end_tick
        );
        assert_eq!(
            result.slots[0].map(|result| result.outcome),
            Some(Outcome::Defeat),
            "{result:?}"
        );
        assert_eq!(
            result.slots[1].map(|result| result.outcome),
            Some(Outcome::Victory)
        );
    }
}
