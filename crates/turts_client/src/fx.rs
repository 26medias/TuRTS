//! Local combat timing. Authoritative records are never changed by visual delays.
use crate::{
    animation::VisualEvents, ingest::ReplicaState, join::AcquiredPackages, present::Presented,
    render::world_to_scene,
};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use turts_mod_api::EffectLayerKind;
pub mod camera_feedback;
pub mod decals;
pub mod emitters;
pub mod flash;
pub mod model_particles;
pub mod particles;
pub mod ribbons;
pub(crate) mod validate;
pub mod warnings;
use turts_net_protocol::Effect;

#[derive(Clone)]
pub struct Actor {
    pub content: String,
    pub position: Vec3,
    pub height: f32,
}
#[derive(Clone)]
pub struct Emission {
    pub effect: String,
    pub source: Vec3,
    pub target: Vec3,
    pub critical: bool,
    pub window: Option<f32>,
    pub anchor: Option<u32>,
    pub target_unit: Option<u32>,
}
#[derive(Clone)]
pub struct Impact {
    pub target: u32,
    pub tick: u32,
    pub at: f64,
    pub hit: Option<String>,
    pub position: Vec3,
    pub health: Option<u64>,
}
#[derive(Clone, Copy)]
pub struct HeldHealth {
    pub value: u64,
    pub until: f64,
}
#[derive(Resource, Default)]
pub struct CombatPresentation {
    pub actors: HashMap<u32, Actor>,
    pub displayed_health: HashMap<u32, u64>,
    health_tick: HashMap<u32, u32>,
    pub held_health: HashMap<u32, HeldHealth>,
    pub impacts: VecDeque<Impact>,
    pub death_at: HashMap<u32, f64>,
    pub model_remove_at: HashMap<u32, f64>,
    pub flashes: HashMap<u32, f64>,
    pub emissions: Vec<Emission>,
    pub destroyed: HashSet<u32>,
    pub released_deaths: HashSet<u32>,
    pub now: f64,
    snapshot: Option<u32>,
    session: Option<u64>,
    observed_tick: Option<u32>,
    coherent: bool,
    terminal_seen: bool,
    loops: HashMap<(u32, String), f64>,
    work_phase: HashMap<u32, f32>,
}
#[derive(Component)]
pub struct UnitMuzzle(pub u32);
#[derive(Component)]
pub struct PendingDeath {
    pub at: f64,
    pub remove_model: bool,
}

pub fn flight_seconds(distance: f32, speed: f32) -> f64 {
    if distance.is_finite() && speed.is_finite() && speed > 0. {
        (distance.max(0.) / speed).min(0.3) as f64
    } else {
        0.
    }
}
impl CombatPresentation {
    fn emit(&mut self, effect: Option<&String>, source: Vec3, target: Vec3, critical: bool) {
        if let Some(effect) = effect {
            self.emissions.push(Emission {
                effect: effect.clone(),
                source,
                target,
                critical,
                window: None,
                anchor: None,
                target_unit: None,
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_combat(
    time: Res<Time>,
    state: Res<ReplicaState>,
    presented: Res<Presented>,
    acquired: Option<Res<AcquiredPackages>>,
    events: Res<VisualEvents>,
    rigs: Query<(
        &crate::render::PresentedUnit,
        &crate::animation::UnitAnimation,
    )>,
    muzzles: Query<(&UnitMuzzle, &GlobalTransform)>,
    players: Query<&AnimationPlayer>,
    mut fx: ResMut<CombatPresentation>,
) {
    fx.now = time.elapsed_secs_f64();
    fx.emissions.clear();
    fx.released_deaths.clear();
    let (Some(replica), Some(acquired)) = (state.replica.as_ref(), acquired) else {
        return;
    };
    if replica.knowledge.is_some() {
        let known = |id: &u32| {
            replica.units.contains_key(id)
                || replica
                    .event_units
                    .iter()
                    .any(|row| row.unit.unit_id.0 == *id)
        };
        fx.actors.retain(|id, _| known(id));
        fx.displayed_health
            .retain(|id, _| replica.units.contains_key(id));
        fx.health_tick
            .retain(|id, _| replica.units.contains_key(id));
        fx.held_health
            .retain(|id, _| replica.units.contains_key(id));
        fx.impacts.retain(|impact| known(&impact.target));
        fx.death_at.retain(|id, _| known(id));
        fx.model_remove_at.retain(|id, _| known(id));
        fx.flashes.retain(|id, _| known(id));
        fx.loops.retain(|(id, _), _| replica.units.contains_key(id));
        fx.work_phase.retain(|id, _| replica.units.contains_key(id));
    }
    let session = state.welcome.map(|w| w.match_id);
    let terminal = replica
        .match_state()
        .outcomes
        .iter()
        .flatten()
        .any(|result| {
            matches!(
                result.outcome,
                turts_net_protocol::Outcome::Victory | turts_net_protocol::Outcome::Draw
            )
        });
    let snapshot_changed = fx.snapshot != replica.snapshot_tick || fx.session != session;
    // The server ends with a reliable state snapshot, without a terminal delta's visual facts.
    // Show only newly missing, known actors at a fresh first outcome. Never invent a shot.
    let terminal_removed: Vec<_> = if snapshot_changed
        && fx.session == session
        && terminal
        && !fx.terminal_seen
        && fx.coherent
        && fx
            .observed_tick
            .zip(replica.snapshot_tick)
            .is_some_and(|(before, end)| end > before && end.saturating_sub(before) <= 4)
    {
        fx.actors
            .iter()
            .filter(|(id, _)| !replica.units.contains_key(id) && !fx.death_at.contains_key(id))
            .map(|(&id, actor)| (id, actor.clone()))
            .collect()
    } else {
        Vec::new()
    };
    let terminal_seen = terminal || (fx.session == session && fx.terminal_seen);
    if snapshot_changed {
        *fx = CombatPresentation {
            now: time.elapsed_secs_f64(),
            snapshot: replica.snapshot_tick,
            session,
            terminal_seen,
            ..Default::default()
        };
    }
    fx.observed_tick = replica.last_applied_tick;
    fx.coherent = !replica.in_resync();
    fx.terminal_seen = terminal_seen;
    for (id, actor) in &terminal_removed {
        fx.actors.insert(*id, actor.clone());
    }
    let package = &acquired.0.presentation;
    let now = fx.now;
    fx.model_remove_at.retain(|_, at| *at > now);
    // Keep the prior frame's actor positions until all lethal events have been resolved.
    for unit in &presented.units {
        let height = package
            .model_visuals
            .get(&unit.content_id)
            .and_then(|model| {
                package
                    .model_bounds
                    .get(&model.asset)
                    .map(|(min, max)| (max[1] - min[1]) * model.scale)
            })
            .unwrap_or(1.6)
            .max(0.1);
        fx.actors.insert(
            unit.unit_id,
            Actor {
                content: unit.content_id.clone(),
                position: world_to_scene(unit.pos),
                height,
            },
        );
    }
    // Authorized event endpoints can disappear before the completed state.
    for observation in &replica.event_units {
        let unit = &observation.unit;
        fx.actors.entry(unit.unit_id.0).or_insert_with(|| Actor {
            content: unit.content_id.clone(),
            position: world_to_scene(unit.pos),
            height: 1.6,
        });
    }
    let destroyed_ticks: HashMap<_, _> = events
        .raw
        .iter()
        .filter_map(|event| match event.effect {
            Effect::Destroyed { unit } => Some((unit.0, event.tick)),
            _ => None,
        })
        .collect();
    let muzzle_positions: HashMap<_, _> = muzzles
        .iter()
        .map(|(id, transform)| (id.0, transform.translation()))
        .collect();
    let mut health_releases: HashMap<(u32, u32), (f64, u64)> = HashMap::new();
    for event in &events.raw {
        let Effect::Fired {
            unit,
            target,
            reduced,
            ..
        } = event.effect
        else {
            continue;
        };
        let (Some(shooter), Some(victim)) = (
            fx.actors.get(&unit.0).cloned(),
            fx.actors.get(&target.0).cloned(),
        ) else {
            continue;
        };
        let refs = package.effect_visuals.get(&shooter.content);
        let end = victim.position + Vec3::Y * (victim.height * 0.5);
        let forward = Vec3::new(end.x - shooter.position.x, 0., end.z - shooter.position.z)
            .normalize_or(Vec3::Z);
        let start = muzzle_positions
            .get(&unit.0)
            .copied()
            .unwrap_or(shooter.position + Vec3::Y * (shooter.height * 0.7) + forward * 0.3);
        let speed = refs
            .and_then(|r| r.projectile.as_ref())
            .and_then(|id| package.effects.get(id))
            .and_then(|effect| {
                effect.layers.iter().find_map(|layer| match layer.kind {
                    EffectLayerKind::Projectile { speed, .. } => {
                        Some((speed, Vec3::from_array(layer.offset)))
                    }
                    _ => None,
                })
            });
        let delay = speed.map_or(0., |(speed, offset)| {
            flight_seconds((start + offset).distance(end), speed)
        });
        let at = now + delay;
        fx.emit(refs.and_then(|r| r.fire.as_ref()), start, end, true);
        if let Some(effect) = refs.and_then(|r| r.projectile.as_ref()) {
            fx.emissions.push(Emission {
                effect: effect.clone(),
                source: start,
                target: end,
                critical: true,
                window: None,
                anchor: None,
                target_unit: Some(target.0),
            });
        }
        fx.impacts.push_back(Impact {
            target: target.0,
            tick: event.tick,
            at,
            hit: package
                .effect_visuals
                .get(&victim.content)
                .and_then(|refs| {
                    if reduced {
                        refs.hit_reduced.clone().or_else(|| refs.hit.clone())
                    } else {
                        refs.hit.clone()
                    }
                })
                .or_else(|| {
                    refs.and_then(|refs| {
                        if reduced {
                            refs.hit_reduced.clone().or_else(|| refs.hit.clone())
                        } else {
                            refs.hit.clone()
                        }
                    })
                }),
            position: end,
            health: None,
        });
        if let Some(value) = replica
            .health_trace
            .get(&target.0)
            .and_then(|trace| trace.iter().rev().find(|health| health.tick == event.tick))
            .map(|health| health.current)
            .or_else(|| {
                replica
                    .healths
                    .iter()
                    .find(|health| health.unit == target)
                    .map(|health| health.current)
            })
        {
            let release = health_releases
                .entry((target.0, event.tick))
                .or_insert((at, value));
            release.0 = release.0.max(at);
            release.1 = value;
        }
        if let Some(value) = fx.displayed_health.get(&target.0).copied() {
            let held = fx
                .held_health
                .entry(target.0)
                .or_insert(HeldHealth { value, until: at });
            held.until = held.until.min(at);
        }
        if destroyed_ticks.get(&target.0) == Some(&event.tick) {
            fx.death_at.insert(target.0, at);
        }
    }
    for ((target, tick), (at, value)) in health_releases {
        if let Some(impact) = fx
            .impacts
            .iter_mut()
            .rev()
            .find(|impact| impact.target == target && impact.tick == tick && impact.at == at)
        {
            impact.health = Some(value);
        }
    }
    for id in events
        .destroyed
        .iter()
        .copied()
        .chain(terminal_removed.iter().map(|(id, _)| *id))
    {
        fx.destroyed.insert(id);
        let at = *fx.death_at.entry(id).or_insert(now);
        let peak = fx
            .actors
            .get(&id)
            .and_then(|actor| package.effect_visuals.get(&actor.content))
            .and_then(|refs| refs.death.as_ref())
            .and_then(|id| package.effects.get(id))
            .and_then(|effect| {
                effect
                    .layers
                    .iter()
                    .find(|layer| matches!(layer.kind, EffectLayerKind::Burst { .. }))
            })
            .map_or(0., |layer| f64::from(layer.delay_ms) / 1000.);
        fx.model_remove_at.insert(id, at + peak);
    }
    // Release each impact independently, even when projectile speeds differ.
    let pending = std::mem::take(&mut fx.impacts);
    for impact in pending {
        if impact.at <= now {
            let position = fx
                .actors
                .get(&impact.target)
                .map_or(impact.position, |actor| {
                    actor.position + Vec3::Y * actor.height * 0.5
                });
            fx.emit(impact.hit.as_ref(), position, position, true);
            fx.flashes.insert(impact.target, now + 0.1);
            if let Some(health) = impact.health
                && impact.tick >= fx.health_tick.get(&impact.target).copied().unwrap_or(0)
            {
                fx.displayed_health.insert(impact.target, health);
                fx.health_tick.insert(impact.target, impact.tick);
            }
            fx.held_health.remove(&impact.target);
        } else {
            fx.impacts.push_back(impact);
        }
    }
    for health in &replica.healths {
        let id = health.unit.0;
        let old = fx.displayed_health.get(&id).copied();
        let next_hit = fx
            .impacts
            .iter()
            .filter(|hit| hit.target == id)
            .map(|hit| hit.at)
            .min_by(f64::total_cmp);
        let held = next_hit.and_then(|until| old.map(|value| HeldHealth { value, until }));
        let value = held.map_or(health.current, |held| held.value);
        if held.is_none() && old.is_some_and(|old| value < old) {
            fx.flashes.insert(id, now + 0.1);
        }
        fx.displayed_health.insert(id, value);
    }
    let deaths: Vec<_> = fx
        .death_at
        .iter()
        .filter_map(|(&id, &at)| (at <= now).then_some(id))
        .collect();
    for id in deaths {
        if let Some(actor) = fx.actors.get(&id).cloned() {
            fx.emit(
                package
                    .effect_visuals
                    .get(&actor.content)
                    .and_then(|r| r.death.as_ref()),
                actor.position,
                actor.position,
                false,
            );
        }
        fx.released_deaths.insert(id);
        fx.death_at.remove(&id);
        fx.destroyed.remove(&id);
    }
    // Rebuild conditions from current records. No loop state is authoritative.
    let work_samples: HashMap<_, _> = rigs
        .iter()
        .filter_map(|(id, rig)| {
            let work = rig.mapping.work.as_ref()?;
            if rig.current.as_deref() != Some(work.clip()) {
                return None;
            }
            let clip = rig.clips.clips.get(work.clip())?;
            let phase = players
                .get(rig.player)
                .ok()?
                .animation(clip.node)?
                .seek_time();
            (clip.duration > 0.).then_some((id.0, phase))
        })
        .collect();
    let mut working = HashSet::new();
    let mut loops = Vec::new();
    for unit in &presented.units {
        let Some(refs) = package.effect_visuals.get(&unit.content_id) else {
            continue;
        };
        let Some((position, height)) = fx
            .actors
            .get(&unit.unit_id)
            .map(|actor| (actor.position, actor.height))
        else {
            continue;
        };
        if let Some(damaged) = &refs.damaged {
            let max = turts_mod_api::ContentId::parse(&unit.content_id)
                .ok()
                .and_then(|id| acquired.0.content.catalog.units.get(&id))
                .and_then(|u| u.health)
                .map(|h| h.max);
            let current = fx.displayed_health.get(&unit.unit_id).copied();
            for condition in damaged.as_slice() {
                if let (Some(current), Some(max)) = (current, max)
                    && u128::from(current) * u128::from(condition.below.denominator)
                        < u128::from(max) * u128::from(condition.below.numerator)
                {
                    loops.push((
                        unit.unit_id,
                        condition.effect.clone(),
                        position + Vec3::Y * height,
                        0.25,
                        Some(0.25),
                    ));
                }
            }
        }
        if let Some(effect) = &refs.site
            && replica.is_site(unit.unit_id)
        {
            let radius = turts_mod_api::ContentId::parse(&unit.content_id)
                .ok()
                .and_then(|id| acquired.0.content.catalog.units.get(&id))
                .map_or(1., |unit| match unit.footprint {
                    turts_mod_api::Footprint::Circle { radius } => radius,
                });
            let phase = now as f32 * 2.;
            let edge = position + Vec3::new(phase.cos(), 0., phase.sin()) * radius;
            loops.push((unit.unit_id, effect.clone(), edge, 0.25, Some(0.25)));
        }
        if let Some(effect) = &refs.work
            && replica
                .unit_orders
                .get(&unit.unit_id)
                .is_some_and(|o| o.activity == turts_net_protocol::Activity::Gathering)
            && replica.economy.orders.iter().any(|order| {
                order.unit.0 == unit.unit_id
                    && matches!(order.stage, turts_net_protocol::GatherStage::Gathering)
            })
        {
            working.insert(unit.unit_id);
            let interval = if let Some(&phase) = work_samples.get(&unit.unit_id) {
                let previous = fx.work_phase.insert(unit.unit_id, phase);
                if previous.is_some_and(|previous| phase + 0.0001 >= previous) {
                    continue;
                }
                0.
            } else {
                fx.work_phase.remove(&unit.unit_id);
                1.
            };
            loops.push((
                unit.unit_id,
                effect.clone(),
                position
                    + replica
                        .economy
                        .orders
                        .iter()
                        .find(|order| order.unit.0 == unit.unit_id)
                        .and_then(|order| order.node)
                        .and_then(|node| replica.nodes.get(&node))
                        .map(|node| {
                            Vec3::new(node.pos.x - position.x, 0., node.pos.y - position.z)
                                .normalize_or(Vec3::Z)
                        })
                        .unwrap_or(Vec3::Z)
                        * 0.4
                    + Vec3::Y * (height * 0.55),
                interval,
                None,
            ));
        }
    }
    fx.work_phase.retain(|id, _| working.contains(id));
    let keys: HashSet<_> = loops
        .iter()
        .map(|(id, effect, _, _, _)| (*id, effect.clone()))
        .collect();
    fx.loops.retain(|key, _| keys.contains(key));
    for (id, effect, position, interval, window) in loops {
        let key = (id, effect.clone());
        let next = fx.loops.get(&key).copied().unwrap_or(0.);
        if now >= next {
            fx.emissions.push(Emission {
                effect,
                source: position,
                target: position,
                critical: false,
                window,
                anchor: Some(id),
                target_unit: None,
            });
            fx.loops.insert(key, now + interval);
        }
    }
    fx.held_health.clear();
    let pending: Vec<_> = fx.impacts.iter().map(|hit| (hit.target, hit.at)).collect();
    for (id, until) in pending {
        if let Some(value) = fx.displayed_health.get(&id).copied() {
            fx.held_health
                .entry(id)
                .or_insert(HeldHealth { value, until });
        }
    }
    fx.flashes.retain(|_, until| *until > now);
    let retained: HashSet<_> = replica
        .units
        .keys()
        .copied()
        .chain(fx.death_at.keys().copied())
        .collect();
    fx.actors.retain(|id, _| retained.contains(id));
    fx.displayed_health.retain(|id, _| retained.contains(id));
    fx.health_tick.retain(|id, _| retained.contains(id));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projectile_visual_delay_is_bounded() {
        assert!((flight_seconds(6., 30.) - 0.2).abs() < 1e-6);
        assert!((flight_seconds(100., 1.) - 0.3).abs() < 1e-6);
        assert_eq!(flight_seconds(3., 0.), 0.);
        assert_eq!(flight_seconds(f32::NAN, 1.), 0.);
    }
}

/// A dead unit loses its interaction identity immediately. Only its picture waits for impact.
pub fn release_deaths(
    mut commands: Commands,
    fx: Res<CombatPresentation>,
    mut serial: ResMut<crate::animation::BodySerial>,
    mut bodies: Query<(
        Entity,
        &PendingDeath,
        &Transform,
        Option<&mut crate::animation::UnitAnimation>,
    )>,
) {
    for (entity, pending, transform, animation) in &mut bodies {
        if pending.at > fx.now {
            continue;
        }
        if pending.remove_model {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(mut animation) = animation {
            animation.dead = true;
            animation.current = None;
        }
        serial.0 += 1;
        commands
            .entity(entity)
            .remove::<PendingDeath>()
            .insert(crate::animation::VisualBody {
                age: 0.,
                ground_y: transform.translation.y,
                serial: serial.0,
            });
    }
}

#[cfg(test)]
mod timeline_tests {
    use super::*;
    use crate::{
        acquire::{Acquired, PresentationContent},
        content::ClientContent,
        present::PresentedUnit,
        replica::Replica,
    };
    use std::time::Duration;
    use turts_net_protocol::{HealthRecord, Pos, UnitId};
    fn app() -> App {
        let mut presentation = PresentationContent::default();
        let effect: turts_mod_api::VisualEffectsDef =
            ron::from_str("(effects:[(id:\"bolt\",layers:[(kind:Projectile(speed:30.0))])])")
                .unwrap();
        presentation
            .effects
            .insert("test:bolt".into(), effect.effects[0].clone());
        presentation.effect_visuals.insert(
            "test:soldier".into(),
            turts_mod_api::VisualEffectRefs {
                projectile: Some("test:bolt".into()),
                ..Default::default()
            },
        );
        let acquired = Acquired {
            lockfile_hash: String::new(),
            visuals: HashMap::new(),
            map_size: (64., 64.),
            unit_names: HashMap::new(),
            content: ClientContent::default(),
            presentation,
            verified_root_mod: None,
        };
        let mut replica = Replica::new(20);
        replica.healths.push(HealthRecord {
            unit: UnitId(2),
            current: 100,
        });
        for (id, x) in [(1, 0.), (2, 6.)] {
            replica.units.insert(
                id,
                crate::replica::UnitReplica {
                    content_id: "test:soldier".into(),
                    owner_slot: 0,
                    facing: None,
                    samples: VecDeque::from([(0, Pos { x, y: 0. })]),
                },
            );
        }
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(ReplicaState {
                replica: Some(replica),
                ..Default::default()
            })
            .insert_resource(Presented {
                units: vec![
                    PresentedUnit {
                        unit_id: 1,
                        content_id: "test:soldier".into(),
                        pos: Pos { x: 0., y: 0. },
                        lo_tick: 0,
                        hi_tick: 0,
                        fraction: 0.,
                    },
                    PresentedUnit {
                        unit_id: 2,
                        content_id: "test:soldier".into(),
                        pos: Pos { x: 6., y: 0. },
                        lo_tick: 0,
                        hi_tick: 0,
                        fraction: 0.,
                    },
                ],
                ..Default::default()
            })
            .insert_resource(AcquiredPackages(acquired))
            .init_resource::<VisualEvents>()
            .init_resource::<CombatPresentation>()
            .add_systems(Update, update_combat);
        app.update();
        app
    }
    fn shoot(app: &mut App, health: u64) {
        app.world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .unwrap()
            .healths[0]
            .current = health;
        app.world_mut().resource_mut::<VisualEvents>().raw = vec![crate::replica::VisualEvent {
            sequence: 1,
            tick: 1,
            effect: Effect::Fired {
                reduced: false,
                unit: UnitId(1),
                target: UnitId(2),
                weapon: 0,
            },
        }];
        app.update();
        app.world_mut().resource_mut::<VisualEvents>().raw.clear();
    }
    fn advance(app: &mut App, millis: u64) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(millis));
        app.update();
    }

    #[test]
    fn reduced_impacts_use_the_victim_effect_and_keep_normal_hit_fallbacks() {
        for (reduced, victim_reduced, victim_hit, expected) in [
            (true, true, true, "test:victim_reduced"),
            (false, true, true, "test:victim_hit"),
            (true, false, true, "test:victim_hit"),
            (true, false, false, "test:shooter_reduced"),
            (false, false, false, "test:shooter_hit"),
        ] {
            let mut app = app();
            {
                let mut acquired = app.world_mut().resource_mut::<AcquiredPackages>();
                let visuals = &mut acquired.0.presentation.effect_visuals;
                let shooter = visuals.get_mut("test:soldier").unwrap();
                shooter.hit = Some("test:shooter_hit".into());
                shooter.hit_reduced = Some("test:shooter_reduced".into());
                visuals.insert(
                    "test:victim".into(),
                    turts_mod_api::VisualEffectRefs {
                        hit: victim_hit.then(|| "test:victim_hit".into()),
                        hit_reduced: victim_reduced.then(|| "test:victim_reduced".into()),
                        ..Default::default()
                    },
                );
            }
            app.world_mut().resource_mut::<Presented>().units[1].content_id = "test:victim".into();
            app.world_mut().resource_mut::<VisualEvents>().raw =
                vec![crate::replica::VisualEvent {
                    sequence: 1,
                    tick: 1,
                    effect: Effect::Fired {
                        reduced,
                        unit: UnitId(1),
                        target: UnitId(2),
                        weapon: 0,
                    },
                }];
            app.update();
            let fx = app.world().resource::<CombatPresentation>();
            assert_eq!(fx.impacts.len(), 1);
            assert_eq!(fx.impacts[0].hit.as_deref(), Some(expected));
        }
    }

    #[test]
    fn melee_without_projectile_emits_no_bolt_and_releases_health_immediately() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<AcquiredPackages>()
            .0
            .presentation
            .effect_visuals
            .get_mut("test:soldier")
            .unwrap()
            .projectile = None;
        shoot(&mut app, 75);
        let fx = app.world().resource::<CombatPresentation>();
        assert_eq!(fx.displayed_health[&2], 75);
        assert!(fx.impacts.is_empty(), "melee has no pending flight");
        assert!(
            fx.emissions.is_empty(),
            "a melee strike cannot emit the configured bolt"
        );
        assert!(fx.flashes.contains_key(&2));
    }

    #[test]
    fn fx1_fx2_health_waits_for_real_flight_then_releases() {
        let mut app = app();
        shoot(&mut app, 90);
        assert_eq!(
            app.world()
                .resource::<CombatPresentation>()
                .displayed_health[&2],
            100
        );
        advance(&mut app, 150);
        assert_eq!(
            app.world()
                .resource::<CombatPresentation>()
                .displayed_health[&2],
            100
        );
        advance(&mut app, 60);
        assert_eq!(
            app.world()
                .resource::<CombatPresentation>()
                .displayed_health[&2],
            90
        );
        assert!(
            app.world()
                .resource::<CombatPresentation>()
                .flashes
                .contains_key(&2)
        );
    }
    #[test]
    fn overlapping_hits_release_their_own_health_values() {
        let mut app = app();
        shoot(&mut app, 90);
        advance(&mut app, 100);
        shoot(&mut app, 80);
        advance(&mut app, 110);
        assert_eq!(
            app.world()
                .resource::<CombatPresentation>()
                .displayed_health[&2],
            90
        );
        advance(&mut app, 100);
        assert_eq!(
            app.world()
                .resource::<CombatPresentation>()
                .displayed_health[&2],
            80
        );
    }
    #[test]
    fn two_shots_in_one_tick_do_not_reveal_damage_before_the_last_bolt() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<Presented>()
            .units
            .push(PresentedUnit {
                unit_id: 3,
                content_id: "test:soldier".into(),
                pos: Pos { x: 5., y: 0. },
                lo_tick: 0,
                hi_tick: 0,
                fraction: 0.,
            });
        {
            let mut state = app.world_mut().resource_mut::<ReplicaState>();
            let replica = state.replica.as_mut().unwrap();
            replica.healths[0].current = 70;
            replica.units.insert(
                3,
                crate::replica::UnitReplica {
                    content_id: "test:soldier".into(),
                    owner_slot: 0,
                    facing: None,
                    samples: VecDeque::from([(0, Pos { x: 5., y: 0. })]),
                },
            );
        }
        app.world_mut().resource_mut::<VisualEvents>().raw = vec![
            crate::replica::VisualEvent {
                sequence: 1,
                tick: 1,
                effect: Effect::Fired {
                    reduced: false,
                    unit: UnitId(1),
                    target: UnitId(2),
                    weapon: 0,
                },
            },
            crate::replica::VisualEvent {
                sequence: 2,
                tick: 1,
                effect: Effect::Fired {
                    reduced: false,
                    unit: UnitId(3),
                    target: UnitId(2),
                    weapon: 0,
                },
            },
        ];
        app.update();
        app.world_mut().resource_mut::<VisualEvents>().raw.clear();
        advance(&mut app, 50);
        assert_eq!(
            app.world()
                .resource::<CombatPresentation>()
                .displayed_health[&2],
            100
        );
        advance(&mut app, 160);
        assert_eq!(
            app.world()
                .resource::<CombatPresentation>()
                .displayed_health[&2],
            70
        );
    }
    #[test]
    fn fx5_damage_thresholds_start_and_stop_the_authored_loops() {
        let mut app = app();
        let unit = turts_mod_api::ResolvedUnit {
            vision: None,
            consumes: Vec::new(),
            income: None,
            provides: Vec::new(),

            acquire: None,
            armour: None,
            id: turts_mod_api::ContentId::parse("test:soldier").unwrap(),
            name: "Target".into(),
            footprint: turts_mod_api::Footprint::Circle { radius: 1. },
            movement: None,
            inventory: None,
            gatherer: None,
            drop_off: None,
            producer: None,
            constructor: None,
            repairer: None,
            placement: None,
            build_area: None,
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: None,
            health: Some(turts_mod_api::HealthDef { max: 100 }),
            weapon: None,
            removal: None,
        };
        {
            let mut packages = app.world_mut().resource_mut::<AcquiredPackages>();
            packages.0.content.catalog.units.insert(
                turts_mod_api::ContentId::parse("test:soldier").unwrap(),
                unit,
            );
            packages
                .0
                .presentation
                .effect_visuals
                .get_mut("test:soldier")
                .unwrap()
                .damaged = Some(turts_mod_api::DamagedEffects::Multiple(vec![
                turts_mod_api::DamagedEffect {
                    effect: "test:smoke".into(),
                    below: turts_mod_api::Ratio {
                        numerator: 1,
                        denominator: 2,
                    },
                },
                turts_mod_api::DamagedEffect {
                    effect: "test:fire".into(),
                    below: turts_mod_api::Ratio {
                        numerator: 1,
                        denominator: 4,
                    },
                },
            ]));
        }
        app.world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .unwrap()
            .healths[0]
            .current = 49;
        advance(&mut app, 300);
        let emissions = &app.world().resource::<CombatPresentation>().emissions;
        assert!(emissions.iter().any(|e| e.effect == "test:smoke"));
        assert!(!emissions.iter().any(|e| e.effect == "test:fire"));
        app.world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .unwrap()
            .healths[0]
            .current = 24;
        advance(&mut app, 300);
        let emissions = &app.world().resource::<CombatPresentation>().emissions;
        assert!(emissions.iter().any(|e| e.effect == "test:smoke"));
        assert!(emissions.iter().any(|e| e.effect == "test:fire"));
        app.world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .unwrap()
            .healths[0]
            .current = 100;
        advance(&mut app, 300);
        assert!(
            app.world()
                .resource::<CombatPresentation>()
                .emissions
                .is_empty()
        );
    }
    #[test]
    fn a_snapshot_clears_pending_hits_without_an_old_burst() {
        let mut app = app();
        shoot(&mut app, 90);
        app.world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .unwrap()
            .snapshot_tick = Some(20);
        app.update();
        let fx = app.world().resource::<CombatPresentation>();
        assert!(fx.impacts.is_empty());
        assert!(fx.emissions.is_empty());
        assert_eq!(fx.displayed_health[&2], 90);
    }
    fn prime_terminal_history(app: &mut App, tick: u32) {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        let replica = state.replica.as_mut().unwrap();
        replica.set_context(crate::replica::ReplicaContext::new(
            ClientContent::default(),
            0,
        ));
        replica.last_applied_tick = Some(tick);
        replica.resync = None;
        app.world_mut()
            .resource_mut::<AcquiredPackages>()
            .0
            .presentation
            .effect_visuals
            .get_mut("test:soldier")
            .unwrap()
            .death = Some("test:death".into());
        app.update();
    }
    fn replace_with_empty_snapshot(
        app: &mut App,
        tick: u32,
        outcome: Option<turts_net_protocol::Outcome>,
    ) {
        let snapshot = turts_net_protocol::Snapshot {
            tick,
            records: turts_net_protocol::SnapshotRecords {
                match_state: Some(turts_net_protocol::MatchState {
                    outcomes: outcome
                        .map(|outcome| {
                            vec![Some(turts_net_protocol::SlotOutcome {
                                outcome,
                                reason: turts_net_protocol::OutcomeReason::OpponentsDefeated,
                                tick,
                            })]
                        })
                        .unwrap_or_default(),
                    ..default()
                }),
                ..default()
            },
            ..default()
        };
        let result = app
            .world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .unwrap()
            .apply_snapshot(&snapshot, bevy::platform::time::Instant::now());
        assert!(
            matches!(result, crate::replica::SnapshotOutcome::Applied { .. }),
            "{result:?}"
        );
        app.world_mut().resource_mut::<Presented>().units.clear();
        app.update();
    }
    #[test]
    fn a_fresh_terminal_snapshot_shows_known_deaths_once_without_a_fake_shot() {
        let mut app = app();
        prime_terminal_history(&mut app, 10);
        replace_with_empty_snapshot(&mut app, 11, Some(turts_net_protocol::Outcome::Victory));
        let fx = app.world().resource::<CombatPresentation>();
        assert_eq!(fx.emissions.len(), 2);
        assert!(
            fx.emissions
                .iter()
                .all(|emission| emission.effect == "test:death")
        );
        assert_eq!(fx.released_deaths.len(), 2);
        assert!(fx.impacts.is_empty());
        app.world_mut()
            .resource_mut::<CombatPresentation>()
            .actors
            .insert(
                99,
                Actor {
                    content: "test:soldier".into(),
                    position: Vec3::ZERO,
                    height: 1.,
                },
            );
        replace_with_empty_snapshot(&mut app, 12, Some(turts_net_protocol::Outcome::Victory));
        assert!(
            app.world()
                .resource::<CombatPresentation>()
                .emissions
                .is_empty()
        );
    }
    #[test]
    fn ordinary_or_nonterminal_defeat_snapshots_never_replay_deaths() {
        for outcome in [None, Some(turts_net_protocol::Outcome::Defeat)] {
            let mut app = app();
            prime_terminal_history(&mut app, 10);
            replace_with_empty_snapshot(&mut app, 11, outcome);
            assert!(
                app.world()
                    .resource::<CombatPresentation>()
                    .emissions
                    .is_empty()
            );
        }
    }
    #[test]
    fn a_stale_terminal_snapshot_does_not_replay_deaths_or_arm_a_duplicate() {
        let mut app = app();
        prime_terminal_history(&mut app, 10);
        replace_with_empty_snapshot(&mut app, 20, Some(turts_net_protocol::Outcome::Victory));
        assert!(
            app.world()
                .resource::<CombatPresentation>()
                .emissions
                .is_empty()
        );
        app.world_mut()
            .resource_mut::<CombatPresentation>()
            .actors
            .insert(
                99,
                Actor {
                    content: "test:soldier".into(),
                    position: Vec3::ZERO,
                    height: 1.,
                },
            );
        replace_with_empty_snapshot(&mut app, 21, Some(turts_net_protocol::Outcome::Victory));
        assert!(
            app.world()
                .resource::<CombatPresentation>()
                .emissions
                .is_empty()
        );
    }
    #[test]
    fn joining_an_ended_match_has_no_old_death_burst() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .unwrap()
            .set_context(crate::replica::ReplicaContext::new(
                ClientContent::default(),
                0,
            ));
        replace_with_empty_snapshot(&mut app, 11, Some(turts_net_protocol::Outcome::Draw));
        assert!(
            app.world()
                .resource::<CombatPresentation>()
                .emissions
                .is_empty()
        );
    }
    #[test]
    fn terminal_recovery_from_an_incoherent_history_has_no_old_death_burst() {
        let mut app = app();
        prime_terminal_history(&mut app, 10);
        app.world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .unwrap()
            .resync = Some(vec![]);
        app.update();
        replace_with_empty_snapshot(&mut app, 11, Some(turts_net_protocol::Outcome::Victory));
        assert!(
            app.world()
                .resource::<CombatPresentation>()
                .emissions
                .is_empty()
        );
    }
    #[test]
    fn a_terminal_snapshot_does_not_reconstruct_an_already_pending_death() {
        let mut app = app();
        prime_terminal_history(&mut app, 10);
        app.world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .unwrap()
            .units
            .remove(&2);
        app.world_mut()
            .resource_mut::<CombatPresentation>()
            .death_at
            .insert(2, 0.2);
        app.update();
        replace_with_empty_snapshot(&mut app, 11, Some(turts_net_protocol::Outcome::Victory));
        let fx = app.world().resource::<CombatPresentation>();
        assert_eq!(fx.emissions.len(), 1);
        assert_eq!(fx.emissions[0].source, Vec3::ZERO);
        assert_eq!(fx.released_deaths, HashSet::from([1]));
    }
}

/// Historical cosmetics cannot remain on ground that left the permitted view.
pub(super) fn permitted_position(state: Option<&ReplicaState>, position: Vec3) -> bool {
    let Some(replica) = state.and_then(|state| state.replica.as_ref()) else {
        return true;
    };
    let Some(knowledge) = replica.knowledge.as_deref() else {
        return true;
    };
    let point = turts_mod_api::WorldPos {
        x: position.x,
        y: position.z,
    };
    if crate::visibility::position_state(knowledge, point)
        == Some(crate::visibility::FogCell::Observed)
    {
        return true;
    }
    let shape = knowledge.grid;
    let Some(grid) =
        turts_rules::visibility::Grid::checked(shape.size.x, shape.size.y, shape.cell_size)
    else {
        return false;
    };
    let cell = grid.cell(turts_rules::visibility::Point {
        x: point.x,
        y: point.y,
    });
    cell.is_some()
        && replica.event_units.iter().any(|row| {
            grid.cell(turts_rules::visibility::Point {
                x: row.unit.pos.x,
                y: row.unit.pos.y,
            }) == cell
        })
}
