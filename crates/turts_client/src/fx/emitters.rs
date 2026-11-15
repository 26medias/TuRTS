//! Translate authored layers into shared render instances.
use super::particles::{ParticlePool, particle};
use super::*;
use turts_mod_api::{EffectKind, EffectLayer, EffectLayerKind as Kind};

#[derive(Clone, Default)]
pub struct Line {
    pub authorized_event: bool,
    pub source: Vec3,
    pub target: Vec3,
    pub colour: Color,
    pub until: f64,
    pub starts: f64,
    pub width: f32,
    pub projectile: bool,
    pub arc: f32,
    pub trail_seconds: f32,
    pub target_unit: Option<u32>,
    pub end_colour: Option<Color>,
    pub end_width: Option<f32>,
    pub additive: bool,
    pub image: Option<String>,
    pub target_offset: Vec3,
    pub visible_at: f64,
}
#[derive(Clone, Default)]
pub struct Decal {
    pub authorized_event: bool,
    pub id: u64,
    pub starts: f64,
    pub until: f64,
    pub position: Vec3,
    pub size: f32,
    pub colour: Color,
    pub image: Option<String>,
    pub end_colour: Option<Color>,
    pub end_size: Option<f32>,
    pub additive: bool,
}
impl Decal {
    pub fn tint(&self, now: f64) -> Vec4 {
        let start = self.colour.to_linear().to_vec4();
        self.end_colour.map_or(
            start * Vec4::new(1., 1., 1., (self.until - now).clamp(0., 1.) as f32),
            |end| start.lerp(end.to_linear().to_vec4(), self.phase(now)),
        )
    }
    pub fn phase(&self, now: f64) -> f32 {
        ((now - self.starts) / (self.until - self.starts).max(0.001)).clamp(0., 1.) as f32
    }
}
#[derive(Clone)]
pub struct ShakePulse {
    pub starts: f64,
    pub until: f64,
    pub strength: f32,
}
#[derive(Clone)]
pub struct FlashLight {
    pub authorized_event: bool,
    pub id: u64,
    pub starts: f64,
    pub until: f64,
    pub position: Vec3,
    pub radius: f32,
    pub colour: Color,
}
#[derive(Resource, Default)]
pub struct EffectSounds(pub Vec<crate::debug_feed::StartedSound>);
#[derive(Resource, Default)]
pub struct Emitters {
    pub lines: VecDeque<Line>,
    pub decals: VecDeque<Decal>,
    decal_serial: u64,
    loop_seen: HashMap<(String, u32), f64>,
    serial: u64,
    epoch: u64,
    pub shakes: VecDeque<ShakePulse>,
    pub lights: VecDeque<FlashLight>,
}
fn layers(effect: &turts_mod_api::VisualEffectDef) -> Vec<EffectLayer> {
    if !effect.layers.is_empty() {
        return effect.layers.clone();
    }
    let kind = match effect.kind {
        EffectKind::Flash => Kind::Flash {
            radius: effect.size,
            light: false,
        },
        EffectKind::Spark => Kind::Sparks {
            count: 1,
            speed: 3.,
            spread_deg: 60.,
            gravity: 9.,
        },
        EffectKind::Tracer => Kind::Tracer { width: effect.size },
        EffectKind::Burst => Kind::Burst {
            radius_start: 0.1,
            radius_end: effect.size,
        },
        EffectKind::Smoke => Kind::Smoke {
            rate: 4.,
            rise: 0.5,
            spread: 0.5,
        },
    };
    vec![EffectLayer {
        kind,
        colour: effect.colour.clone(),
        end_colour: None,
        size: effect.size,
        end_size: None,
        ms: effect.ms,
        delay_ms: 0,
        image: None,
        blend: Default::default(),
        offset: [0.; 3],
        looping: false,
    }]
}
#[allow(clippy::too_many_arguments)]
pub fn emit_effects(
    state: Option<Res<ReplicaState>>,
    fx: Res<CombatPresentation>,
    acquired: Option<Res<AcquiredPackages>>,
    settings: Res<crate::models::GraphicsSettings>,
    camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    mut pool: ResMut<ParticlePool>,
    mut emitters: ResMut<Emitters>,
    mut sounds: ResMut<EffectSounds>,
    volume: Option<Res<crate::volume_settings::VolumeSettings>>,
) {
    if emitters.epoch != fx.epoch {
        *emitters = Emitters {
            epoch: fx.epoch,
            ..default()
        };
    }
    pool.reset_epoch(fx.epoch);
    let Some(acquired) = acquired else {
        return;
    };
    pool.limit = ParticlePool::capacity(settings.quality);
    let Some((camera, transform)) = camera.iter().find(|(c, _)| c.is_active) else {
        return;
    };
    let viewport = camera.logical_viewport_rect();
    for emission in &fx.emissions {
        let Some(effect) = acquired.0.presentation.effects.get(&emission.effect) else {
            continue;
        };
        let travel = effect.kind == EffectKind::Tracer
            || effect.layers.iter().any(|layer| {
                matches!(
                    layer.kind,
                    Kind::Projectile { .. } | Kind::Tracer { .. } | Kind::Trail { .. }
                )
            });
        let in_view = |point| {
            camera
                .world_to_viewport(transform, point)
                .ok()
                .is_some_and(|point| viewport.is_some_and(|rect| rect.contains(point)))
        };
        if !in_view(emission.source) && !(travel && in_view(emission.target)) {
            continue;
        }
        let steady = if let Some(anchor) = emission.anchor {
            emitters
                .loop_seen
                .insert((emission.effect.clone(), anchor), fx.now)
                .is_none_or(|last| fx.now - last > 0.5)
        } else {
            false
        };
        let centre = viewport
            .and_then(|viewport| camera.viewport_to_world(transform, viewport.center()).ok())
            .filter(|ray| ray.direction.y.abs() > 0.001)
            .map(|ray| ray.origin + *ray.direction * (-ray.origin.y / ray.direction.y))
            .unwrap_or_else(|| transform.translation());
        let distance = centre.distance_squared(emission.source);
        if emission.window.is_none()
            && volume.as_deref().is_some_and(|volume| volume.sound_on)
            && let Some(sound) = &effect.sound
            && sounds.0.len() < 64
        {
            sounds.0.push(crate::debug_feed::StartedSound {
                id: sound.clone(),
                category: "effects".into(),
                priority: 0,
            });
        }
        let quality_scale = match settings.quality {
            crate::models::Quality::Low => 0.25,
            crate::models::Quality::Medium => 0.5,
            crate::models::Quality::High => 1.,
        };
        let mut effect_particles = Vec::new();
        let mut projectile_fallbacks = Vec::new();
        for layer in layers(effect) {
            emitters.serial = emitters.serial.wrapping_add(1);
            let seed = emitters.serial;
            let mut base = particle(
                &layer,
                emission.source,
                effect.cost,
                distance,
                emission.critical,
            );
            base.authorized_event = emission.authorized_event;
            let mut particles = Vec::new();
            match &layer.kind {
                Kind::Flash { radius, light } => {
                    if *light && settings.quality == crate::models::Quality::High {
                        while emitters.lights.len() >= 8 {
                            emitters.lights.pop_front();
                        }
                        let id = emitters.serial;
                        emitters.lights.push_back(FlashLight {
                            authorized_event: emission.authorized_event,
                            id,
                            starts: fx.now + f64::from(layer.delay_ms) / 1000.,
                            until: fx.now + f64::from(layer.delay_ms + layer.ms) / 1000.,
                            position: base.position,
                            radius: *radius,
                            colour: Color::from(base.colour),
                        });
                    }
                    let mut p = base;
                    p.size = radius * 2.;
                    p.end_size = radius * 2.5;
                    particles.push(p);
                }
                Kind::Burst {
                    radius_start,
                    radius_end,
                } => {
                    let mut p = base;
                    p.size = radius_start * 2.;
                    p.end_size = radius_end * 2.;
                    particles.push(p);
                }
                Kind::Sparks {
                    count,
                    speed,
                    spread_deg,
                    gravity,
                } => {
                    let count = ((*count as f32 * quality_scale).ceil() as u16).max(1);
                    let direction = (emission.target - emission.source).normalize_or_zero();
                    let axis = if direction.length_squared() > 0.1 {
                        direction
                    } else {
                        Vec3::Y
                    };
                    for i in 0..count {
                        let mut p = base.clone();
                        let (u, v) = noise(seed, i);
                        let angle = u * std::f32::consts::TAU;
                        let cone = v * spread_deg.to_radians() * 0.5;
                        let helper = if axis.dot(Vec3::Y).abs() > 0.9 {
                            Vec3::X
                        } else {
                            Vec3::Y
                        };
                        let right = axis.cross(helper).normalize();
                        let up = right.cross(axis);
                        p.velocity = (axis * cone.cos()
                            + (right * angle.cos() + up * angle.sin()) * cone.sin())
                            * *speed;
                        p.gravity = *gravity;
                        particles.push(p);
                    }
                }
                Kind::Debris {
                    count,
                    speed,
                    image,
                    model,
                } => {
                    for i in 0..((*count as f32 * quality_scale).ceil() as u16).max(1) {
                        let mut p = base.clone();
                        let (u, v) = noise(seed, i);
                        let angle = u * std::f32::consts::TAU;
                        p.velocity = Vec3::new(angle.cos(), 0.5 + v, angle.sin()) * *speed;
                        p.gravity = 9.;
                        p.spin = 3. + v * 5.;
                        p.image = image.clone().or(p.image);
                        p.model = model.clone();
                        particles.push(p);
                    }
                }
                Kind::Smoke { rate, rise, .. } | Kind::Fire { rate, height: rise } => {
                    let spread = if let Kind::Smoke { spread, .. } = layer.kind {
                        spread
                    } else {
                        0.25
                    };
                    let expected = *rate
                        * if steady {
                            layer.ms as f32 / 1000.
                        } else {
                            emission.window.unwrap_or(layer.ms as f32 / 1000.)
                        }
                        * quality_scale;
                    let count = if steady {
                        expected.ceil()
                    } else {
                        expected.floor() + f32::from(noise(seed, 0).0 < expected.fract())
                    }
                    .clamp(0., 64.) as u16;
                    for i in 0..count {
                        let mut p = base.clone();
                        let (u, v) = noise(seed, i);
                        let angle = u * std::f32::consts::TAU;
                        p.position += Vec3::new(angle.cos(), 0., angle.sin()) * spread * v;
                        let speed = if matches!(layer.kind, Kind::Fire { .. }) {
                            *rise / p.life.max(0.001)
                        } else {
                            *rise
                        };
                        p.velocity = Vec3::Y * speed;
                        p.end_size = layer.end_size.unwrap_or(layer.size * 2.);
                        if steady {
                            p.delay = 0.;
                            p.age = u * p.life;
                            p.position += p.velocity * p.age;
                        } else {
                            let delay = i as f32 / (*rate * quality_scale).max(0.1);
                            p.delay += delay;
                            if emission.window.is_none() {
                                p.life = (p.life - delay).max(0.001);
                            }
                        }
                        particles.push(p);
                    }
                }
                Kind::Projectile {
                    speed,
                    arc_height,
                    image,
                    trail,
                    model,
                } => {
                    let mut p = base;
                    p.life = flight_seconds(p.start.distance(emission.target), *speed) as f32;
                    p.target = Some(emission.target);
                    p.follow = emission.target_unit;
                    p.arc = *arc_height;
                    p.image = image.clone().or(p.image);
                    p.model = model.clone();
                    projectile_fallbacks.push(Line {
                        authorized_event: emission.authorized_event,
                        source: p.start,
                        target: emission.target,
                        colour: Color::WHITE,
                        starts: fx.now,
                        until: fx.now + f64::from(p.life),
                        width: 0.08,
                        projectile: true,
                        arc: *arc_height,
                        trail_seconds: 0.04,
                        target_unit: emission.target_unit,
                        ..default()
                    });
                    particles.push(p);
                    let trail_layer = trail
                        .as_ref()
                        .and_then(|id| acquired.0.presentation.effects.get(id))
                        .and_then(|effect| {
                            effect
                                .layers
                                .iter()
                                .find(|layer| matches!(layer.kind, Kind::Trail { .. }))
                        });
                    let (width, trail_seconds) = trail_layer.map_or((0.06, 0.035), |layer| {
                        let Kind::Trail { width, ms } = layer.kind else {
                            unreachable!()
                        };
                        (width, ms as f32 / 1000.)
                    });
                    let colour = trail_layer.map_or(Color::srgb(0.75, 0.62, 0.38), |layer| {
                        Color::from(super::particles::colour(&layer.colour))
                    });
                    let trail_offset =
                        trail_layer.map_or(Vec3::ZERO, |layer| Vec3::from_array(layer.offset));
                    // A moving ribbon also keeps the shot readable while the sprite pipeline warms.
                    emitters.lines.push_back(Line {
                        authorized_event: emission.authorized_event,
                        source: particles[0].start + trail_offset,
                        target: emission.target + trail_offset,
                        colour,
                        until: fx.now + f64::from(particles[0].life),
                        starts: fx.now,
                        width,
                        projectile: true,
                        arc: *arc_height,
                        trail_seconds,
                        target_unit: emission.target_unit,
                        end_colour: trail_layer
                            .and_then(|layer| layer.end_colour.as_deref())
                            .map(super::particles::colour)
                            .map(Color::from),
                        end_width: trail_layer.and_then(|layer| layer.end_size),
                        additive: trail_layer
                            .is_some_and(|layer| layer.blend == turts_mod_api::EffectBlend::Add),
                        image: trail_layer.and_then(|layer| layer.image.clone()),
                        target_offset: trail_offset,
                        visible_at: fx.now
                            + trail_layer.map_or(0., |layer| f64::from(layer.delay_ms) / 1000.),
                    });
                }
                Kind::Tracer { width } | Kind::Trail { width, .. } => {
                    emitters.lines.push_back(Line {
                        authorized_event: emission.authorized_event,
                        source: base.position,
                        target: emission.target + Vec3::from_array(layer.offset),
                        colour: Color::from(base.colour),
                        until: fx.now
                            + f64::from(layer.delay_ms) / 1000.
                            + match layer.kind {
                                Kind::Trail { ms, .. } => f64::from(ms) / 1000.,
                                _ => f64::from(base.life),
                            },
                        starts: fx.now + f64::from(layer.delay_ms) / 1000.,
                        width: *width,
                        projectile: false,
                        arc: 0.,
                        trail_seconds: 0.,
                        target_unit: None,
                        end_colour: layer
                            .end_colour
                            .as_deref()
                            .map(super::particles::colour)
                            .map(Color::from),
                        end_width: layer.end_size,
                        additive: layer.blend == turts_mod_api::EffectBlend::Add,
                        image: layer.image.clone(),
                        ..default()
                    });
                }
                Kind::Decal { size, ms, image } => {
                    while emitters.decals.len() >= 128 {
                        emitters.decals.pop_front();
                    }
                    emitters.decal_serial = emitters.decal_serial.wrapping_add(1);
                    let id = emitters.decal_serial;
                    emitters.decals.push_back(Decal {
                        authorized_event: emission.authorized_event,
                        id,
                        starts: fx.now + f64::from(layer.delay_ms) / 1000.,
                        until: fx.now + f64::from(layer.delay_ms + ms) / 1000.,
                        position: emission.source + Vec3::from_array(layer.offset),
                        size: *size,
                        colour: Color::from(base.colour),
                        image: image.clone().or(layer.image.clone()),
                        end_colour: layer
                            .end_colour
                            .as_deref()
                            .map(super::particles::colour)
                            .map(Color::from),
                        end_size: layer.end_size,
                        additive: layer.blend == turts_mod_api::EffectBlend::Add,
                    });
                }
                Kind::Shake { strength, ms } => {
                    if emitters.shakes.len() >= 16 {
                        emitters.shakes.pop_front();
                    }
                    emitters.shakes.push_back(ShakePulse {
                        starts: fx.now + f64::from(layer.delay_ms) / 1000.,
                        until: fx.now + f64::from(layer.delay_ms + ms) / 1000.,
                        strength: strength.min(0.5) / (1. + distance / 100.),
                    });
                }
            }
            effect_particles.extend(particles);
        }
        let accepted = pool.spawn_batch(effect_particles);
        if emission.critical && !accepted {
            // The informational line is independent of an authored trail's delay or transparency.
            emitters.lines.extend(projectile_fallbacks);
            emitters.lines.push_back(Line {
                authorized_event: emission.authorized_event,
                source: emission.source,
                target: emission.source,
                colour: Color::WHITE,
                starts: fx.now,
                until: fx.now + 0.1,
                width: 0.3,
                projectile: false,
                arc: 0.,
                trail_seconds: 0.,
                target_unit: None,
                ..default()
            });
        }
    }
    emitters.loop_seen.retain(|_, last| fx.now - *last < 5.);
    emitters.shakes.retain(|shake| shake.until > fx.now);
    emitters.lights.retain(|light| {
        light.until > fx.now
            && (light.authorized_event
                || super::permitted_position(state.as_deref(), light.position))
    });
    emitters.lines.retain(|line| {
        line.until > fx.now
            && (line.authorized_event
                || (super::permitted_position(state.as_deref(), line.source)
                    && super::permitted_position(state.as_deref(), line.target)))
    });
    while emitters.lines.len() > 4096 {
        emitters.lines.pop_front();
    }
    emitters.decals.retain(|decal| {
        decal.until > fx.now
            && (decal.authorized_event
                || super::permitted_position(state.as_deref(), decal.position))
    });
}
fn noise(seed: u64, index: u16) -> (f32, f32) {
    let a = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(u64::from(index).wrapping_mul(1442695040888963407));
    (
        (a >> 32) as u32 as f32 / u32::MAX as f32,
        a as u32 as f32 / u32::MAX as f32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        acquire::{Acquired, PresentationContent},
        content::ClientContent,
    };
    use std::time::Duration;
    fn app() -> App {
        let mut presentation = PresentationContent::default();
        let effects: turts_mod_api::VisualEffectsDef =
            ron::from_str("(effects:[(id:\"scorch\",layers:[(kind:Decal(size:2.0,ms:1000))])])")
                .unwrap();
        presentation
            .effects
            .insert("test:scorch".into(), effects.effects[0].clone());
        let acquired = Acquired {
            lockfile_hash: "test".into(),
            visuals: HashMap::new(),
            map_size: (40., 30.),
            unit_names: HashMap::new(),
            content: ClientContent::default(),
            presentation,
            verified_root_mod: None,
        };
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .insert_resource(AcquiredPackages(acquired))
            .init_resource::<CombatPresentation>()
            .init_resource::<ParticlePool>()
            .init_resource::<Emitters>()
            .init_resource::<EffectSounds>()
            .init_resource::<crate::models::GraphicsSettings>()
            .add_systems(Update, emit_effects);
        let mut camera = Camera::default();
        camera.computed.target_info = Some(bevy::camera::RenderTargetInfo {
            physical_size: UVec2::new(1280, 720),
            scale_factor: 1.,
        });
        camera.computed.clip_from_view = Mat4::IDENTITY;
        app.world_mut()
            .spawn((Camera3d::default(), camera, GlobalTransform::IDENTITY));
        app
    }
    #[test]
    fn authorized_event_cosmetics_survive_hidden_ground_and_clear_on_context_reset() {
        let mut app = app();
        let mut replica = crate::replica::Replica::new(20);
        replica.knowledge = Some(Box::new(crate::fx::tests::hidden_knowledge()));
        app.insert_resource(ReplicaState {
            replica: Some(replica),
            ..default()
        });
        let defs: turts_mod_api::VisualEffectsDef = ron::from_str(r#"(effects:[
            (id:"event",layers:[(kind:Projectile(speed:1.0)),(kind:Flash(radius:1.0,light:true)),(kind:Decal(size:2.0,ms:1000))])
        ])"#).unwrap();
        app.world_mut()
            .resource_mut::<AcquiredPackages>()
            .0
            .presentation
            .effects
            .insert("test:event".into(), defs.effects[0].clone());
        for authorized_event in [false, true] {
            app.world_mut()
                .resource_mut::<CombatPresentation>()
                .emissions
                .push(Emission {
                    effect: "test:event".into(),
                    source: Vec3::new(0., 0., 0.5),
                    target: Vec3::new(0.5, 0., 0.5),
                    critical: true,
                    authorized_event,
                    window: None,
                    anchor: None,
                    target_unit: Some(2),
                });
        }
        app.update();
        let emitters = app.world().resource::<Emitters>();
        assert!(!emitters.lines.is_empty());
        assert!(emitters.lines.iter().all(|line| line.authorized_event));
        assert_eq!(
            emitters.decals.len(),
            1,
            "unwitnessed cosmetics still require visible ground"
        );
        assert!(emitters.decals[0].authorized_event);
        let pool = app.world().resource::<ParticlePool>();
        assert!(
            pool.slots
                .iter()
                .filter_map(|slot| slot.particle.as_ref())
                .any(|particle| particle.authorized_event)
        );
        {
            let mut fx = app.world_mut().resource_mut::<CombatPresentation>();
            fx.emissions.clear();
            fx.epoch += 1;
        }
        app.update();
        let emitters = app.world().resource::<Emitters>();
        assert!(emitters.lines.is_empty());
        assert!(emitters.decals.is_empty());
        assert!(emitters.lights.is_empty());
        assert_eq!(app.world().resource::<ParticlePool>().active(), 0);
    }

    #[test]
    fn fx6_real_emitter_caps_decals_and_expires_them() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<CombatPresentation>()
            .emissions = (0..129)
            .map(|_| Emission {
                effect: "test:scorch".into(),
                source: Vec3::new(0., 0., 0.5),
                target: Vec3::ZERO,
                critical: false,
                authorized_event: false,
                window: None,
                anchor: None,
                target_unit: None,
            })
            .collect();
        app.update();
        let decals = &app.world().resource::<Emitters>().decals;
        assert_eq!(decals.len(), 128);
        assert_eq!(decals.front().unwrap().id, 2);
        {
            let mut fx = app.world_mut().resource_mut::<CombatPresentation>();
            fx.emissions.clear();
            fx.now = 1.001;
        }
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_millis(1001));
        app.update();
        assert!(app.world().resource::<Emitters>().decals.is_empty());
    }
    #[test]
    fn review_common_line_offset_is_applied() {
        let mut app = app();
        let defs: turts_mod_api::VisualEffectsDef = ron::from_str(
            r#"(effects:[(id:"line",layers:[(kind:Tracer(width:0.1),offset:(0.2,0.1,0.0))])])"#,
        )
        .unwrap();
        app.world_mut()
            .resource_mut::<AcquiredPackages>()
            .0
            .presentation
            .effects
            .insert("test:line".into(), defs.effects[0].clone());
        app.world_mut()
            .resource_mut::<CombatPresentation>()
            .emissions
            .push(Emission {
                effect: "test:line".into(),
                source: Vec3::new(0., 0., 0.5),
                target: Vec3::new(0.5, 0., 0.5),
                critical: false,
                authorized_event: false,
                window: None,
                anchor: None,
                target_unit: None,
            });
        app.update();
        assert_eq!(
            app.world().resource::<Emitters>().lines[0].source,
            Vec3::new(0.2, 0.1, 0.5)
        );
    }
    #[test]
    fn review_decal_lifetime_includes_common_delay() {
        let mut app = app();
        let defs: turts_mod_api::VisualEffectsDef = ron::from_str(
            r#"(effects:[(id:"delayed",layers:[(kind:Decal(size:2.0,ms:1000),delay_ms:500)])])"#,
        )
        .unwrap();
        app.world_mut()
            .resource_mut::<AcquiredPackages>()
            .0
            .presentation
            .effects
            .insert("test:delayed".into(), defs.effects[0].clone());
        app.world_mut()
            .resource_mut::<CombatPresentation>()
            .emissions
            .push(Emission {
                effect: "test:delayed".into(),
                source: Vec3::new(0., 0., 0.5),
                target: Vec3::ZERO,
                critical: false,
                authorized_event: false,
                window: None,
                anchor: None,
                target_unit: None,
            });
        app.update();
        assert_eq!(app.world().resource::<Emitters>().decals[0].until, 1.5);
    }
    #[test]
    fn review_full_low_pool_keeps_plain_fire_projectile_and_hit() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<crate::models::GraphicsSettings>()
            .quality = crate::models::Quality::Low;
        let layer: EffectLayer = ron::from_str("(kind:Flash())").unwrap();
        {
            let mut pool = app.world_mut().resource_mut::<ParticlePool>();
            pool.limit = 512;
            for _ in 0..512 {
                assert!(pool.spawn(particle(
                    &layer,
                    Vec3::ZERO,
                    turts_mod_api::EffectCost::Small,
                    0.,
                    true
                )));
            }
        }
        let defs: turts_mod_api::VisualEffectsDef=ron::from_str(r##"(effects:[(id:"fire",layers:[(kind:Flash())]),(id:"projectile",layers:[(kind:Projectile(speed:30.0,trail:Some("test:trail")))]),(id:"trail",layers:[(kind:Trail(width:0.02,ms:50),delay_ms:500,colour:"#ffffff00",image:Some("test:stripe"))]),(id:"hit",layers:[(kind:Sparks(count:8))])])"##).unwrap();
        for effect in defs.effects {
            app.world_mut()
                .resource_mut::<AcquiredPackages>()
                .0
                .presentation
                .effects
                .insert(format!("test:{}", effect.id), effect);
        }
        for name in ["fire", "projectile", "hit"] {
            app.world_mut()
                .resource_mut::<CombatPresentation>()
                .emissions
                .push(Emission {
                    effect: format!("test:{name}"),
                    source: Vec3::new(0., 0., 0.5),
                    target: Vec3::new(0.5, 0., 0.5),
                    critical: true,
                    authorized_event: false,
                    window: None,
                    anchor: None,
                    target_unit: None,
                });
        }
        app.update();
        let pool = app.world().resource::<ParticlePool>();
        assert_eq!(pool.active(), 512);
        assert_eq!(pool.slots.len(), 512);
        let lines = &app.world().resource::<Emitters>().lines;
        assert!(lines.iter().any(|line| line.projectile
            && line.starts <= 0.
            && line.visible_at <= 0.
            && line.until > 0.
            && line.width >= 0.06
            && line.image.is_none()
            && line.colour.alpha() > 0.));
        assert!(lines.iter().filter(|line| !line.projectile).count() >= 2);
    }
    #[test]
    fn projectile_uses_the_referenced_trail_layer_parameters() {
        let mut app = app();
        let defs: turts_mod_api::VisualEffectsDef = ron::from_str(r##"(effects:[
            (id:"bolt",layers:[(kind:Projectile(speed:1.0,trail:Some("test:trail")))]),
            (id:"trail",layers:[(kind:Trail(width:0.2,ms:80),colour:"#ff0000",end_colour:Some("#0000ff80"),end_size:Some(0.6),image:Some("test:stripe"),blend:Add,offset:(0.1,0.2,0.3),delay_ms:50)])
        ])"##).unwrap();
        for effect in defs.effects {
            app.world_mut()
                .resource_mut::<AcquiredPackages>()
                .0
                .presentation
                .effects
                .insert(format!("test:{}", effect.id), effect);
        }
        app.world_mut()
            .resource_mut::<CombatPresentation>()
            .emissions
            .push(Emission {
                effect: "test:bolt".into(),
                source: Vec3::new(0., 0., 0.5),
                target: Vec3::new(0.5, 0., 0.5),
                critical: true,
                authorized_event: false,
                window: None,
                anchor: None,
                target_unit: None,
            });
        app.update();
        let line = &app.world().resource::<Emitters>().lines[0];
        assert_eq!(line.width, 0.2);
        assert_eq!(line.end_width, Some(0.6));
        assert_eq!(line.image.as_deref(), Some("test:stripe"));
        assert!(line.additive);
        assert!(line.end_colour.is_some());
        assert_eq!(line.source, Vec3::new(0.1, 0.2, 0.8));
        assert_eq!(line.target, Vec3::new(0.6, 0.2, 0.8));
        assert_eq!(line.visible_at, 0.05);
        assert!((line.trail_seconds - 0.08).abs() < 0.00001);
    }
}
