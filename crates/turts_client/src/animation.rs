//! Client-only animation state, skinned model hierarchies, and bounded visual bodies.
use crate::{ingest::ReplicaState, models::Quality, present::Presented, render::PresentedUnit};
use bevy::{
    animation::{AnimatedBy, AnimationTargetId},
    gltf::{Gltf, GltfMesh, GltfNode, GltfSkin},
    mesh::skinning::SkinnedMesh,
    prelude::*,
};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use turts_mod_api::{AnimationClipDef, ModelAnimations};

#[derive(Resource, Default)]
pub struct VisualEvents {
    pub fired: HashSet<u32>,
    pub destroyed: HashSet<u32>,
    sequence: u64,
}
pub fn collect_events(state: Res<ReplicaState>, mut events: ResMut<VisualEvents>) {
    events.fired.clear();
    events.destroyed.clear();
    let Some(replica) = state.replica.as_ref() else {
        return;
    };
    for event in &replica.visual_events {
        if event.sequence <= events.sequence {
            continue;
        }
        events.sequence = event.sequence;
        if event.tick.saturating_add(4) < replica.last_applied_tick.unwrap_or(0) {
            continue;
        }
        match event.effect {
            turts_net_protocol::Effect::Fired { unit, .. } => {
                events.fired.insert(unit.0);
            }
            turts_net_protocol::Effect::Destroyed { unit } => {
                events.destroyed.insert(unit.0);
            }
            _ => {}
        }
    }
}

#[derive(Clone)]
pub struct Clip {
    pub node: AnimationNodeIndex,
    pub duration: f32,
}
#[derive(Clone)]
pub struct ClipSet {
    pub graph: Handle<AnimationGraph>,
    pub clips: HashMap<String, Clip>,
}
#[derive(Resource, Default)]
pub struct AnimationLibrary(pub HashMap<AssetId<Gltf>, ClipSet>);
#[derive(Component)]
pub struct UnitAnimation {
    pub player: Entity,
    pub pose_nodes: Vec<Entity>,
    pub clips: ClipSet,
    pub mapping: ModelAnimations,
    pub current: Option<String>,
    pub distance_loops: f32,
    pub previous: Vec2,
    pub shot_left: f32,
    pub dead: bool,
    pub screen_height: f32,
    pub quality: Quality,
    pub visible: bool,
    pub yaw_offset: f32,
    phase: f32,
    pending_time: f32,
}
#[derive(Component)]
pub struct VisualBody {
    pub age: f32,
    pub ground_y: f32,
    pub serial: u64,
}
#[derive(Resource, Default)]
pub struct BodySerial(pub u64);

/// Build the same named hierarchy and skin links as the glTF loader, with shared render assets.
#[allow(clippy::too_many_arguments)]
pub fn spawn_rig(
    commands: &mut Commands,
    root: Entity,
    id: u32,
    pos: Vec2,
    lift: f32,
    scale: f32,
    gltf: &Gltf,
    asset: AssetId<Gltf>,
    nodes: &Assets<GltfNode>,
    meshes: &Assets<GltfMesh>,
    skins: &Assets<GltfSkin>,
    clips: &Assets<AnimationClip>,
    graphs: &mut Assets<AnimationGraph>,
    library: &mut AnimationLibrary,
    materials: &HashMap<AssetId<bevy::gltf::GltfMaterial>, Handle<StandardMaterial>>,
    mapping: &ModelAnimations,
) -> Option<(
    Entity,
    Vec<(Entity, AssetId<Mesh>, AssetId<StandardMaterial>)>,
)> {
    let source = gltf.source.as_ref()?;
    let set = library
        .0
        .entry(asset)
        .or_insert_with(|| {
            let mut named: Vec<_> = gltf.named_animations.iter().collect();
            named.sort_by(|a, b| a.0.cmp(b.0));
            let (graph, indices) =
                AnimationGraph::from_clips(named.iter().map(|(_, h)| (*h).clone()));
            ClipSet {
                graph: graphs.add(graph),
                clips: named
                    .into_iter()
                    .zip(indices)
                    .map(|((name, handle), node)| {
                        (
                            name.to_string(),
                            Clip {
                                node,
                                duration: clips.get(handle).map_or(0., AnimationClip::duration),
                            },
                        )
                    })
                    .collect(),
            }
        })
        .clone();
    let player = commands
        .spawn((
            Transform::from_xyz(0., -lift, 0.).with_scale(Vec3::splat(scale)),
            Visibility::default(),
            ChildOf(root),
            AnimationPlayer::default(),
            AnimationTransitions::default(),
            AnimationGraphHandle(set.graph.clone()),
        ))
        .id();
    let mut entities = HashMap::new();
    let mut parents = HashMap::new();
    for node in source.nodes() {
        for child in node.children() {
            parents.insert(child.index(), node.index());
        }
    }
    for node in source.nodes() {
        let data = nodes.get(&gltf.nodes[node.index()])?;
        entities.insert(
            node.index(),
            commands
                .spawn((
                    data.transform,
                    Visibility::default(),
                    Name::new(data.name.clone()),
                ))
                .id(),
        );
    }
    let mut gpu = Vec::new();
    for node in source.nodes() {
        let data = nodes.get(&gltf.nodes[node.index()])?;
        let entity = entities[&node.index()];
        let parent = parents.get(&node.index()).map_or(player, |p| entities[p]);
        commands.entity(entity).insert(ChildOf(parent));
        let mut path = vec![Name::new(data.name.clone())];
        let mut cursor = node.index();
        while let Some(parent) = parents.get(&cursor) {
            if path.len() > 128 {
                return None;
            }
            path.push(Name::new(nodes.get(&gltf.nodes[*parent])?.name.clone()));
            cursor = *parent;
        }
        path.reverse();
        commands.entity(entity).insert((
            AnimationTargetId::from_names(path.iter()),
            AnimatedBy(player),
        ));
        if let Some(mesh) = &data.mesh {
            for primitive in &meshes.get(mesh)?.primitives {
                let material = materials
                    .get(&primitive.material.clone().unwrap_or_default().id())?
                    .clone();
                let child = commands
                    .spawn((
                        Mesh3d(primitive.mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::IDENTITY,
                        ChildOf(entity),
                    ))
                    .id();
                if let Some(skin) = &data.skin {
                    let skin = skins.get(skin)?;
                    let joints = skin
                        .joints
                        .iter()
                        .map(|h| nodes.get(h).and_then(|n| entities.get(&n.index)).copied())
                        .collect::<Option<Vec<_>>>()?;
                    commands.entity(child).insert((
                        SkinnedMesh {
                            inverse_bindposes: skin.inverse_bind_matrices.clone(),
                            joints,
                        },
                        bevy::camera::visibility::NoFrustumCulling,
                    ));
                }
                gpu.push((child, primitive.mesh.id(), material.id()));
            }
        }
    }
    commands.entity(root).insert(UnitAnimation {
        player,
        pose_nodes: (0..gltf.nodes.len()).map(|i| entities[&i]).collect(),
        clips: set,
        mapping: mapping.clone(),
        current: None,
        distance_loops: 0.,
        previous: pos,
        shot_left: 0.,
        dead: false,
        screen_height: 1.,
        quality: Quality::High,
        visible: true,
        yaw_offset: 0.,
        phase: ((id.wrapping_mul(2654435761) % 1000) as f32) / 1000.,
        pending_time: 0.,
    });
    Some((player, gpu))
}

/// Unit movement is inferred from shown displacement, so blocked units do not walk in place.
pub fn animate_units(
    time: Res<Time>,
    presented: Res<Presented>,
    replica: Res<ReplicaState>,
    events: Res<VisualEvents>,
    mut rigs: Query<(Option<&PresentedUnit>, &mut UnitAnimation)>,
    mut players: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &mut Transform,
    )>,
) {
    let positions: HashMap<_, _> = presented
        .units
        .iter()
        .map(|u| (u.unit_id, Vec2::new(u.pos.x, u.pos.y)))
        .collect();
    for (unit, mut rig) in &mut rigs {
        let Ok((mut player, mut transitions, mut rig_transform)) = players.get_mut(rig.player)
        else {
            continue;
        };
        let dt = time.delta_secs();
        let id = unit.map(|u| u.0);
        let pos = id
            .and_then(|id| positions.get(&id).copied())
            .unwrap_or(rig.previous);
        let distance = pos.distance(rig.previous);
        rig.previous = pos;
        let moving = distance > 0.00001;
        let fired = id.is_some_and(|id| events.fired.contains(&id));
        let activity = id.and_then(|id| {
            replica
                .replica
                .as_ref()?
                .unit_orders
                .get(&id)
                .map(|r| r.activity)
        });
        let attacking = activity == Some(turts_net_protocol::Activity::Attacking)
            || id.is_some_and(|id| {
                replica.replica.as_ref().is_some_and(|r| {
                    r.weapons
                        .iter()
                        .any(|w| w.unit.0 == id && w.last_target.is_some())
                })
            });
        if fired {
            rig.shot_left = 0.001;
        }
        let idle = rig.mapping.idle.as_ref();
        let entry: Option<&AnimationClipDef> = if rig.dead {
            rig.mapping.death.as_ref().or(idle)
        } else if fired || rig.shot_left > 0. {
            rig.mapping.attack.as_ref().or(idle)
        } else if moving {
            rig.mapping.r#move.as_ref().or(idle)
        } else if activity == Some(turts_net_protocol::Activity::Gathering) {
            rig.mapping.work.as_ref().or(idle)
        } else if attacking {
            rig.mapping.attack.as_ref().or(idle)
        } else {
            idle
        };
        let Some(entry) = entry.cloned() else {
            continue;
        };
        let holding = attacking && !fired && rig.shot_left <= 0. && !moving && !rig.dead;
        let settings = entry.settings();
        let name = if holding {
            settings
                .and_then(|s| s.hold.as_deref())
                .or_else(|| idle.map(AnimationClipDef::clip))
                .unwrap_or(entry.clip())
        } else {
            entry.clip()
        }
        .to_owned();
        let Some(clip) = rig.clips.clips.get(&name).cloned() else {
            continue;
        };
        rig.yaw_offset = if attacking && !rig.dead {
            settings.map_or(0., |s| s.yaw_offset_deg.to_radians())
        } else {
            0.
        };
        rig_transform.rotation = Quat::from_rotation_y(rig.yaw_offset);
        let changed = rig.current.as_ref() != Some(&name) || fired;
        if changed {
            let active = transitions.play(&mut player, clip.node, Duration::from_secs_f32(0.15));
            if !rig.dead && !(fired || rig.shot_left > 0.) {
                active.repeat();
                active.seek_to(rig.phase * clip.duration);
            }
            if fired {
                let release = settings.and_then(|s| s.fire_at).unwrap_or(0.) * clip.duration;
                let start = (release - 0.05).max(0.);
                active.seek_to(start);
                rig.shot_left = (clip.duration - start).max(0.001);
            }
            rig.current = Some(name);
        }
        let visible = rig.visible || rig.dead;
        let cadence = match rig.quality {
            Quality::High => 0.,
            Quality::Medium if rig.screen_height < 0.04 => 1. / 15.,
            Quality::Low if rig.screen_height < 0.07 => 1. / 10.,
            _ => 0.,
        };
        rig.pending_time += dt;
        if moving
            && !rig.dead
            && !attacking
            && let Some(stride) = settings.and_then(|s| s.stride)
        {
            rig.distance_loops += distance / stride;
        }
        if let Some(active) = player.animation_mut(clip.node) {
            // Manual advancement supports culling and quality cadence without changing game time.
            active.resume().set_speed(0.);
            if visible
                && rig.pending_time >= cadence
                && moving
                && !rig.dead
                && !attacking
                && settings.and_then(|s| s.stride).is_some()
            {
                active.seek_to(((rig.phase + rig.distance_loops) % 1.) * clip.duration);
            } else if visible && rig.pending_time >= cadence {
                let next = active.seek_time() + rig.pending_time;
                active.seek_to(if rig.dead || rig.shot_left > 0. {
                    next.min(clip.duration)
                } else {
                    next % clip.duration.max(0.001)
                });
            }
        }
        if rig.pending_time >= cadence || !visible {
            rig.pending_time = 0.;
        }
        rig.shot_left = (rig.shot_left - dt).max(0.);
    }
}

/// The final model emerges through the opaque ground as construction progresses.
pub fn construction_offset(height: f32, fraction: f32) -> f32 {
    -height * (1. - fraction.clamp(0., 1.))
}

pub fn age_bodies(
    mut commands: Commands,
    time: Res<Time>,
    mut bodies: Query<(Entity, &mut VisualBody, &mut Transform)>,
) {
    let mut order = Vec::new();
    for (entity, mut body, mut transform) in &mut bodies {
        body.age += time.delta_secs();
        transform.translation.y = body.ground_y - ((body.age - 3.).max(0.) / 1.5).min(1.);
        if body.age >= 4.5 {
            commands.entity(entity).despawn();
        } else {
            order.push((body.serial, entity));
        }
    }
    order.sort_unstable_by_key(|(serial, _)| *serial);
    for (_, entity) in order.iter().take(order.len().saturating_sub(64)) {
        commands.entity(*entity).despawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::present::PresentedUnit as Shown;
    use turts_net_protocol::{Activity, Pos, UnitId, UnitOrderRecord};
    fn fixture() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.insert_resource(Time::<()>::default())
            .init_resource::<Presented>()
            .init_resource::<ReplicaState>()
            .init_resource::<VisualEvents>();
        app.add_systems(Update, (animate_units, age_bodies).chain());
        let mut graph = AnimationGraph::new();
        let clips = ["Idle", "Walk", "Shoot", "Aim", "Work", "Death"]
            .into_iter()
            .map(|name| {
                (
                    name.into(),
                    Clip {
                        node: graph.add_clip(Handle::default(), 1., graph.root),
                        duration: 1.,
                    },
                )
            })
            .collect();
        let player = app
            .world_mut()
            .spawn((
                AnimationPlayer::default(),
                AnimationTransitions::default(),
                Transform::default(),
            ))
            .id();
        let mapping = ron::from_str::<ModelAnimations>(r#"(idle: Some((clip: "Idle")),move: Some((clip: "Walk",stride: Some(0.5))),attack: Some((clip: "Shoot",fire_at: Some(0.5),hold: Some("Aim"))),work: Some((clip: "Work")),death: Some((clip: "Death")))"#).unwrap();
        let root = app
            .world_mut()
            .spawn((
                PresentedUnit(1),
                Transform::default(),
                UnitAnimation {
                    player,
                    pose_nodes: Vec::new(),
                    clips: ClipSet {
                        graph: Handle::default(),
                        clips,
                    },
                    mapping,
                    current: None,
                    distance_loops: 0.,
                    previous: Vec2::ZERO,
                    shot_left: 0.,
                    dead: false,
                    screen_height: 1.,
                    quality: Quality::High,
                    visible: true,
                    yaw_offset: 0.,
                    phase: 0.,
                    pending_time: 0.,
                },
            ))
            .id();
        app.world_mut().resource_mut::<ReplicaState>().replica =
            Some(crate::replica::Replica::new(20));
        (app, root, player)
    }
    fn step(app: &mut App, x: f32, dt: f32, activity: Activity) {
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(dt));
        app.world_mut().resource_mut::<Presented>().units = vec![Shown {
            unit_id: 1,
            content_id: "test:soldier".into(),
            pos: Pos { x, y: 0. },
            lo_tick: 1,
            hi_tick: 1,
            fraction: 0.,
        }];
        app.world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .unwrap()
            .unit_orders
            .insert(
                1,
                UnitOrderRecord {
                    unit: UnitId(1),
                    activity,
                    current: None,
                    queued: vec![],
                    last_failure: None,
                    rally: None,
                },
            );
        app.update();
    }
    #[test]
    fn an9_construction_shows_half_height_at_half_progress_and_full_height_at_completion() {
        assert_eq!(construction_offset(4., 0.5), -2.);
        assert_eq!(construction_offset(4., 1.), 0.);
        assert_eq!(construction_offset(4., 0.), -4.);
    }
    #[test]
    fn review_offscreen_moving_unit_does_not_advance_pose() {
        let (mut app, root, player) = fixture();
        step(&mut app, 0.01, 0.016, Activity::Moving);
        app.world_mut()
            .get_mut::<UnitAnimation>(root)
            .unwrap()
            .visible = false;
        let node = app.world().get::<UnitAnimation>(root).unwrap().clips.clips["Walk"].node;
        let before = app
            .world()
            .get::<AnimationPlayer>(player)
            .unwrap()
            .animation(node)
            .unwrap()
            .seek_time();
        step(&mut app, 0.02, 0.016, Activity::Moving);
        let after = app
            .world()
            .get::<AnimationPlayer>(player)
            .unwrap()
            .animation(node)
            .unwrap()
            .seek_time();
        assert_eq!(
            before, after,
            "offscreen motion must not evaluate a new pose"
        );
    }
    #[test]
    fn review_low_quality_far_motion_waits_for_animation_cadence() {
        let (mut app, root, player) = fixture();
        step(&mut app, 0.01, 0.016, Activity::Moving);
        {
            let mut rig = app.world_mut().get_mut::<UnitAnimation>(root).unwrap();
            rig.quality = Quality::Low;
            rig.screen_height = 0.01;
        }
        let node = app.world().get::<UnitAnimation>(root).unwrap().clips.clips["Walk"].node;
        let before = app
            .world()
            .get::<AnimationPlayer>(player)
            .unwrap()
            .animation(node)
            .unwrap()
            .seek_time();
        step(&mut app, 0.02, 0.016, Activity::Moving);
        let after = app
            .world()
            .get::<AnimationPlayer>(player)
            .unwrap()
            .animation(node)
            .unwrap()
            .seek_time();
        assert_eq!(
            before, after,
            "far low-quality motion must wait for the 10 Hz cadence"
        );
    }

    #[test]
    fn review_destroyed_live_root_becomes_unpickable_visual_body() {
        let (mut app, root, _) = fixture();
        let package = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
        let bytes = turts_mod_runtime::pack::pack_dir(&package).unwrap();
        let archive = turts_mod_runtime::archive::PackageArchive::parse(&bytes).unwrap();
        let acquired = crate::acquire::build_acquired(&[archive], String::new()).unwrap();
        app.insert_resource(crate::join::AcquiredPackages(acquired))
            .insert_resource(Assets::<Mesh>::default())
            .init_resource::<crate::render::SlotMaterials>()
            .init_resource::<BodySerial>()
            .add_systems(Update, crate::render::sync_units.before(animate_units));
        app.world_mut()
            .resource_mut::<VisualEvents>()
            .destroyed
            .insert(1);
        app.world_mut().resource_mut::<Presented>().units.clear();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(0.016));
        app.update();
        assert!(
            app.world().get_entity(root).is_ok(),
            "AN5: Destroyed must preserve a visual body"
        );
        assert!(
            app.world().get::<VisualBody>(root).is_some(),
            "AN5: body marker missing"
        );
        assert!(
            app.world().get::<PresentedUnit>(root).is_none(),
            "AN5: dead body must not be pickable as a unit"
        );
        assert_eq!(
            app.world()
                .get::<UnitAnimation>(root)
                .unwrap()
                .current
                .as_deref(),
            Some("Death")
        );
    }
    #[test]
    fn an1_motion_selects_walk_and_a_stationary_move_order_selects_idle() {
        let (mut app, root, _) = fixture();
        step(&mut app, 0.1, 0.1, Activity::Moving);
        assert_eq!(
            app.world()
                .get::<UnitAnimation>(root)
                .unwrap()
                .current
                .as_deref(),
            Some("Walk")
        );
        step(&mut app, 0.1, 0.1, Activity::Moving);
        assert_eq!(
            app.world()
                .get::<UnitAnimation>(root)
                .unwrap()
                .current
                .as_deref(),
            Some("Idle")
        );
    }
    #[test]
    fn an2_five_seconds_of_motion_follow_measured_stride() {
        let (mut app, root, player) = fixture();
        for frame in 1..=300 {
            step(&mut app, frame as f32 * 0.01, 1. / 60., Activity::Moving);
        }
        let rig = app.world().get::<UnitAnimation>(root).unwrap();
        assert!(
            (rig.distance_loops - 6.).abs() < 0.0001,
            "3 map units at stride 0.5 must play 6 loops"
        );
        assert!(
            app.world()
                .get::<AnimationPlayer>(player)
                .unwrap()
                .animation(rig.clips.clips["Walk"].node)
                .is_some()
        );
    }
    #[test]
    fn an3_shot_release_is_within_a_tenth_second_then_aims_between_shots() {
        let (mut app, root, player) = fixture();
        app.world_mut()
            .resource_mut::<VisualEvents>()
            .fired
            .insert(1);
        step(&mut app, 0., 0.016, Activity::Attacking);
        let rig = app.world().get::<UnitAnimation>(root).unwrap();
        let time = app
            .world()
            .get::<AnimationPlayer>(player)
            .unwrap()
            .animation(rig.clips.clips["Shoot"].node)
            .unwrap()
            .seek_time();
        assert!(
            (time - 0.5).abs() <= 0.1,
            "release fraction 0.5, got {time}"
        );
        app.world_mut().resource_mut::<VisualEvents>().fired.clear();
        for _ in 0..80 {
            step(&mut app, 0., 0.016, Activity::Attacking);
        }
        assert_eq!(
            app.world()
                .get::<UnitAnimation>(root)
                .unwrap()
                .current
                .as_deref(),
            Some("Aim")
        );
    }
    #[test]
    fn an4_stationary_gatherer_works_and_returning_gatherer_walks() {
        let (mut app, root, _) = fixture();
        step(&mut app, 0., 0.016, Activity::Gathering);
        assert_eq!(
            app.world()
                .get::<UnitAnimation>(root)
                .unwrap()
                .current
                .as_deref(),
            Some("Work")
        );
        step(&mut app, 0.05, 0.016, Activity::Gathering);
        assert_eq!(
            app.world()
                .get::<UnitAnimation>(root)
                .unwrap()
                .current
                .as_deref(),
            Some("Walk")
        );
    }
    #[test]
    fn an5_body_is_visual_only_and_expires_after_four_and_a_half_seconds() {
        let (mut app, root, _) = fixture();
        app.world_mut()
            .entity_mut(root)
            .remove::<PresentedUnit>()
            .insert(VisualBody {
                age: 0.,
                ground_y: 0.8,
                serial: 1,
            });
        app.world_mut().get_mut::<UnitAnimation>(root).unwrap().dead = true;
        step(&mut app, 0., 0.1, Activity::Idle);
        assert_eq!(
            app.world()
                .get::<UnitAnimation>(root)
                .unwrap()
                .current
                .as_deref(),
            Some("Death")
        );
        assert!(app.world().get::<PresentedUnit>(root).is_none());
        for _ in 0..43 {
            step(&mut app, 0., 0.1, Activity::Idle);
        }
        assert!(app.world().get_entity(root).is_ok());
        assert!(app.world().get::<Transform>(root).unwrap().translation.y < 0.);
        for _ in 0..2 {
            step(&mut app, 0., 0.1, Activity::Idle);
        }
        assert!(app.world().get_entity(root).is_err());
    }
    #[test]
    fn an5_sixty_fifth_body_removes_oldest() {
        let (mut app, _, _) = fixture();
        let mut first = None;
        for serial in 1..=65 {
            let id = app
                .world_mut()
                .spawn((
                    VisualBody {
                        age: 0.,
                        ground_y: 0.,
                        serial,
                    },
                    Transform::default(),
                ))
                .id();
            if first.is_none() {
                first = Some(id);
            }
        }
        step(&mut app, 0., 0.016, Activity::Idle);
        assert!(app.world().get_entity(first.unwrap()).is_err());
        assert_eq!(
            app.world_mut()
                .query::<&VisualBody>()
                .iter(app.world())
                .count(),
            64
        );
    }
}

/// Evaluate equal crowd poses once, then copy local bone transforms to their instances.
/// Phase buckets retain individual movement clocks and blend weights. No gameplay state is shared.
#[derive(Resource, Default)]
pub struct SharedPoses {
    copies: Vec<(Entity, Entity)>,
    pub evaluated_rigs: usize,
    pub shared_rigs: usize,
}
#[allow(clippy::type_complexity)]
pub fn group_crowd_poses(
    rigs: Query<&UnitAnimation>,
    players: Query<&AnimationPlayer>,
    mut targets: Query<&mut AnimatedBy>,
    mut poses: ResMut<SharedPoses>,
) {
    let mut groups: HashMap<(AssetId<AnimationGraph>, Vec<(usize, i32, i32)>), Vec<Entity>> =
        HashMap::new();
    poses.copies.clear();
    poses.evaluated_rigs = 0;
    poses.shared_rigs = 0;
    for rig in &rigs {
        let Ok(player) = players.get(rig.player) else {
            continue;
        };
        if !rig.visible && !rig.dead {
            for node in &rig.pose_nodes {
                if let Ok(mut by) = targets.get_mut(*node) {
                    by.0 = Entity::PLACEHOLDER;
                }
            }
            continue;
        }
        let mut phases: Vec<_> = player
            .playing_animations()
            .map(|(node, active)| {
                (
                    node.index(),
                    (active.seek_time() * 30.).round() as i32,
                    (active.weight() * 32.).round() as i32,
                )
            })
            .collect();
        phases.sort_unstable();
        let key = (rig.clips.graph.id(), phases);
        if let Some(source) = groups.get(&key) {
            poses.shared_rigs += 1;
            for (node, master) in rig.pose_nodes.iter().zip(source) {
                if let Ok(mut by) = targets.get_mut(*node) {
                    by.0 = Entity::PLACEHOLDER;
                }
                poses.copies.push((*node, *master));
            }
        } else {
            poses.evaluated_rigs += 1;
            for node in &rig.pose_nodes {
                if let Ok(mut by) = targets.get_mut(*node) {
                    by.0 = rig.player;
                }
            }
            groups.insert(key, rig.pose_nodes.clone());
        }
    }
}
/// Run after Bevy evaluates the master poses and before global transforms propagate.
pub fn copy_crowd_poses(poses: Res<SharedPoses>, mut transforms: Query<&mut Transform>) {
    for (target, source) in &poses.copies {
        let Ok(value) = transforms.get(*source).copied() else {
            continue;
        };
        if let Ok(mut transform) = transforms.get_mut(*target)
            && *transform != value
        {
            *transform = value;
        }
    }
}

#[cfg(test)]
mod crowd_tests {
    use super::*;
    #[test]
    fn equal_phases_share_evaluation_and_copy_local_pose_without_copying_world_position() {
        let mut app = App::new();
        app.init_resource::<SharedPoses>()
            .add_systems(Update, (group_crowd_poses, copy_crowd_poses).chain());
        let graph = Handle::<AnimationGraph>::default();
        let mut nodes = Vec::new();
        for x in [1., 10.] {
            let player = app
                .world_mut()
                .spawn((AnimationPlayer::default(), Transform::from_xyz(x, 0., 0.)))
                .id();
            let node = app
                .world_mut()
                .spawn((
                    Transform::from_xyz(x, 1., 0.),
                    AnimatedBy(player),
                    ChildOf(player),
                ))
                .id();
            nodes.push((player, node));
            app.world_mut().spawn(UnitAnimation {
                player,
                pose_nodes: vec![node],
                clips: ClipSet {
                    graph: graph.clone(),
                    clips: HashMap::new(),
                },
                mapping: ModelAnimations::default(),
                current: None,
                distance_loops: 0.,
                previous: Vec2::ZERO,
                shot_left: 0.,
                dead: false,
                screen_height: 1.,
                quality: Quality::High,
                visible: true,
                yaw_offset: 0.,
                phase: 0.,
                pending_time: 0.,
            });
        }
        app.update();
        let poses = app.world().resource::<SharedPoses>();
        assert_eq!((poses.evaluated_rigs, poses.shared_rigs), (1, 1));
        assert_eq!(
            app.world().get::<Transform>(nodes[0].1),
            app.world().get::<Transform>(nodes[1].1)
        );
        assert_eq!(
            app.world()
                .get::<Transform>(nodes[1].0)
                .unwrap()
                .translation
                .x,
            10.
        );
    }
}
