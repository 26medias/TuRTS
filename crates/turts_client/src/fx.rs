//! Local combat timing. Authoritative records are never changed by visual delays.
use crate::{
    animation::VisualEvents, ingest::ReplicaState, join::AcquiredPackages, present::Presented,
    render::world_to_scene,
};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use turts_mod_api::EffectLayerKind;
pub mod emitters;
pub mod flash;
pub mod particles;
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
}
#[derive(Clone)]
pub struct Impact {
    pub target: u32,
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
    pub held_health: HashMap<u32, HeldHealth>,
    pub impacts: VecDeque<Impact>,
    pub death_at: HashMap<u32, f64>,
    pub model_remove_at: HashMap<u32, f64>,
    pub flashes: HashMap<u32, f64>,
    pub emissions: Vec<Emission>,
    pub destroyed: HashSet<u32>,
    pub now: f64,
    snapshot: Option<u32>,
    session: Option<u64>,
    loops: HashMap<(u32, String), f64>,
}
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
            });
        }
    }
}

pub fn update_combat(
    time: Res<Time>,
    state: Res<ReplicaState>,
    presented: Res<Presented>,
    acquired: Option<Res<AcquiredPackages>>,
    events: Res<VisualEvents>,
    mut fx: ResMut<CombatPresentation>,
) {
    fx.now = time.elapsed_secs_f64();
    fx.emissions.clear();
    let (Some(replica), Some(acquired)) = (state.replica.as_ref(), acquired) else {
        return;
    };
    let session = state.welcome.map(|w| w.match_id);
    if fx.snapshot != replica.snapshot_tick || fx.session != session {
        *fx = CombatPresentation {
            now: time.elapsed_secs_f64(),
            snapshot: replica.snapshot_tick,
            session,
            ..Default::default()
        };
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
    for event in &events.raw {
        let Effect::Fired { unit, target, .. } = event.effect else {
            continue;
        };
        let (Some(shooter), Some(victim)) = (
            fx.actors.get(&unit.0).cloned(),
            fx.actors.get(&target.0).cloned(),
        ) else {
            continue;
        };
        let refs = package.effect_visuals.get(&shooter.content);
        let start = shooter.position + Vec3::Y * (shooter.height * 0.7);
        let end = victim.position + Vec3::Y * (victim.height * 0.5);
        let speed = refs
            .and_then(|r| r.projectile.as_ref())
            .and_then(|id| package.effects.get(id))
            .and_then(|effect| {
                effect.layers.iter().find_map(|layer| match layer.kind {
                    EffectLayerKind::Projectile { speed, .. } => Some(speed),
                    _ => None,
                })
            });
        let delay = speed.map_or(0., |speed| flight_seconds(start.distance(end), speed));
        let at = now + delay;
        fx.emit(refs.and_then(|r| r.fire.as_ref()), start, end, true);
        fx.emit(refs.and_then(|r| r.projectile.as_ref()), start, end, true);
        fx.impacts.push_back(Impact {
            target: target.0,
            at,
            hit: package
                .effect_visuals
                .get(&victim.content)
                .and_then(|r| r.hit.clone())
                .or_else(|| refs.and_then(|r| r.hit.clone())),
            position: end,
            health: replica
                .healths
                .iter()
                .find(|h| h.unit == target)
                .map(|h| h.current),
        });
        if let Some(value) = fx.displayed_health.get(&target.0).copied() {
            let held = fx
                .held_health
                .entry(target.0)
                .or_insert(HeldHealth { value, until: at });
            held.until = held.until.min(at);
        }
        if events.destroyed.contains(&target.0) {
            fx.death_at.insert(target.0, at);
        }
    }
    for &id in &events.destroyed {
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
            fx.emit(impact.hit.as_ref(), impact.position, impact.position, true);
            fx.flashes.insert(impact.target, now + 0.1);
            if let Some(health) = impact.health {
                fx.displayed_health.insert(impact.target, health);
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
        fx.death_at.remove(&id);
        fx.destroyed.remove(&id);
    }
    // Rebuild conditions from current records. No loop state is authoritative.
    let mut loops = Vec::new();
    for unit in &presented.units {
        let Some(refs) = package.effect_visuals.get(&unit.content_id) else {
            continue;
        };
        let Some(actor) = fx.actors.get(&unit.unit_id) else {
            continue;
        };
        let position = actor.position;
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
                    loops.push((unit.unit_id, condition.effect.clone(), position));
                }
            }
        }
        if let Some(effect) = &refs.site
            && replica.is_site(unit.unit_id)
        {
            loops.push((unit.unit_id, effect.clone(), position));
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
            loops.push((unit.unit_id, effect.clone(), position + Vec3::Y * 0.7));
        }
    }
    let keys: HashSet<_> = loops
        .iter()
        .map(|(id, effect, _)| (*id, effect.clone()))
        .collect();
    fx.loops.retain(|key, _| keys.contains(key));
    for (id, effect, position) in loops {
        let key = (id, effect.clone());
        let next = fx.loops.get(&key).copied().unwrap_or(0.);
        if now >= next {
            fx.emissions.push(Emission {
                effect,
                source: position,
                target: position,
                critical: false,
                window: Some(0.25),
            });
            fx.loops.insert(key, now + 0.25);
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
}
