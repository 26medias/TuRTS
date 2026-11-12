//! Scene oracle for `draw_world_visuals` (task L0-SPLIT, step 1).
//!
//! A headless app runs `prepare_assets` and `draw_world_visuals` on fixed scenes. After each
//! update the test records every visual entity (id, shared mesh, material, transform,
//! visibility, ring metadata), every health-bar fill, every tooltip label, every gizmo line
//! and the debug feed `markers_shown`. The scenes are compared with the committed fixture
//! `scene_oracle.txt`. The fixture was recorded on the code before the function was split, so
//! a pure move keeps it green. The ignored test `bless_scene_fixture` writes a new record to
//! `TURTS_BLESS_SCENES=<path>`; the normal test panics when that variable is set.

use super::*;
use crate::{
    acquire::Acquired,
    ingest::WelcomeInfo,
    replica::{Replica, UnitReplica},
};
use bevy::{
    asset::AssetPlugin,
    camera::{CameraProjection, ComputedCameraValues, RenderTargetInfo},
    gizmos::{GizmoAsset, GizmoHandles, config::DefaultGizmoConfigGroup},
};
use std::{any::TypeId, fmt::Write as _, path::PathBuf, time::Duration};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{
    HealthRecord, OrderEnvelope, OrderSummary, QueueMode, UnitId, UnitOrderRecord,
};

const FIXTURE: &str = include_str!("scene_oracle.txt");

fn muster_content() -> crate::content::ClientContent {
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/muster");
    let archive =
        PackageArchive::parse(&pack_dir(&package).expect("pack Muster")).expect("parse Muster");
    crate::content::ClientContent::from_archives(&[archive]).expect("resolve Muster")
}

fn unit(owner_slot: u8, x: f32, y: f32) -> UnitReplica {
    UnitReplica {
        content_id: "muster:trooper".into(),
        owner_slot,
        facing: None,
        samples: VecDeque::from([(1, Pos { x, y })]),
    }
}

fn presented_unit(unit_id: u32, x: f32, y: f32) -> crate::present::PresentedUnit {
    crate::present::PresentedUnit {
        unit_id,
        content_id: "muster:trooper".into(),
        pos: Pos { x, y },
        lo_tick: 1,
        hi_tick: 1,
        fraction: 0.0,
    }
}

fn order(kind: &str, target: OrderTarget) -> OrderEnvelope {
    OrderEnvelope {
        order: format!("muster:order_{kind}"),
        units: vec![UnitId(1)],
        target,
        queue: QueueMode::Replace,
        group: 0,
    }
}

fn summary(x: f32, y: f32) -> OrderSummary {
    OrderSummary {
        order: "muster:order_move".into(),
        target: OrderTarget::Point(Pos { x, y }),
    }
}

/// The world state that the scenes share: four units, two slots, order records, health and
/// marker intents. Units: 1 own, 2 foreign, 3 own and damaged, 4 foreign and blocked.
fn base_app(content: Option<crate::content::ClientContent>, welcome: bool) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()));
    app.init_asset::<Mesh>()
        .init_asset::<StandardMaterial>()
        .init_asset::<RingOverlayMaterial>()
        .init_asset::<GizmoAsset>()
        .init_resource::<GizmoHandles>()
        .init_gizmo_group::<DefaultGizmoConfigGroup>();
    app.insert_gizmo_config(OrderMarkerGizmoConfigGroup, order_marker_gizmo_config());
    let full = muster_content();
    let mut replica = Replica::new(20);
    for (id, slot, x, y) in [
        (1, 0, 10.0, 10.0),
        (2, 1, 14.0, 10.0),
        (3, 0, 10.0, 14.0),
        (4, 1, 14.0, 14.0),
    ] {
        replica.units.insert(id, unit(slot, x, y));
    }
    replica.last_applied_tick = Some(1);
    replica.snapshot_tick = Some(1);
    replica.healths = vec![
        HealthRecord {
            unit: UnitId(1),
            current: 50,
        },
        HealthRecord {
            unit: UnitId(2),
            current: 25,
        },
        HealthRecord {
            unit: UnitId(3),
            current: 20,
        },
        HealthRecord {
            unit: UnitId(4),
            current: 50,
        },
    ];
    replica.unit_orders.insert(
        1,
        UnitOrderRecord {
            unit: UnitId(1),
            activity: Activity::Moving,
            current: Some(summary(12.0, 12.0)),
            queued: vec![
                summary(16.0, 12.0),
                OrderSummary {
                    order: "muster:order_attack".into(),
                    target: OrderTarget::Unit(UnitId(2)),
                },
                summary(16.0, 16.0),
            ],
            last_failure: None,
        },
    );
    replica.unit_orders.insert(
        2,
        UnitOrderRecord {
            unit: UnitId(2),
            activity: Activity::Moving,
            current: Some(summary(1.0, 1.0)),
            queued: Vec::new(),
            last_failure: None,
        },
    );
    replica.unit_orders.insert(
        4,
        UnitOrderRecord {
            unit: UnitId(4),
            activity: Activity::Blocked,
            current: None,
            queued: Vec::new(),
            last_failure: None,
        },
    );
    let state = ReplicaState {
        replica: Some(replica),
        welcome: welcome.then_some(WelcomeInfo {
            slot: 0,
            tick: 1,
            tick_hz: 20,
            match_id: 7,
        }),
        content: content.or(Some(full.clone())),
        config: Default::default(),
    };
    let acquired = Acquired {
        lockfile_hash: String::new(),
        visuals: HashMap::new(),
        map_size: (64.0, 64.0),
        unit_names: HashMap::new(),
        content: full,
        presentation: Default::default(),
        verified_root_mod: None,
    };
    let palette = {
        let mut materials = app.world_mut().resource_mut::<Assets<StandardMaterial>>();
        SlotMaterials(vec![
            materials.add(unlit(Color::srgb_u8(0, 114, 178))),
            materials.add(unlit(Color::srgb_u8(230, 159, 0))),
        ])
    };
    app.insert_resource(state)
        .insert_resource(AcquiredPackages(acquired))
        .insert_resource(palette)
        .insert_resource(Presented {
            frame_ns: 1,
            t: 1.0,
            latest_tick: 1,
            units: vec![
                presented_unit(1, 10.0, 10.0),
                presented_unit(2, 14.0, 10.0),
                presented_unit(3, 10.0, 14.0),
                presented_unit(4, 14.0, 14.0),
            ],
        })
        .init_resource::<ReceiptWitnesses>()
        .init_resource::<CoreRuntime>()
        .init_resource::<SelectionFeed>()
        .init_resource::<HealthDisplay>()
        .init_resource::<OrderMarkerFeed>()
        .init_resource::<CommandResults>()
        .init_resource::<VisualAssets>()
        .init_resource::<VisualState>()
        .init_resource::<MarkersShown>()
        .add_systems(Update, (prepare_assets, draw_world_visuals).chain());
    app.world_mut()
        .resource_mut::<ReceiptWitnesses>()
        .next_epoch = 1;
    app
}

/// A perspective camera at a fixed pose. No plugin computes it, so the test sets the values
/// that `camera_system` would set.
fn spawn_camera(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            Camera3d::default(),
            Camera {
                computed: ComputedCameraValues {
                    clip_from_view: PerspectiveProjection::default().get_clip_from_view(),
                    target_info: Some(RenderTargetInfo {
                        physical_size: UVec2::new(1280, 720),
                        scale_factor: 1.0,
                    }),
                    ..default()
                },
                ..default()
            },
            GlobalTransform::from(
                Transform::from_xyz(12.0, 30.0, 30.0)
                    .looking_at(Vec3::new(12.0, 0.0, 12.0), Vec3::Y),
            ),
        ))
        .id()
}

fn spawn_window_over(app: &mut App, camera: Entity, ground: Pos) {
    let cursor = camera_pixel(app, camera, ground);
    let mut window = Window::default();
    window.set_cursor_position(Some(cursor));
    app.world_mut().spawn((window, PrimaryWindow));
}

fn camera_pixel(app: &App, camera: Entity, ground: Pos) -> Vec2 {
    let transform = app.world().get::<GlobalTransform>(camera).unwrap();
    app.world()
        .get::<Camera>(camera)
        .unwrap()
        .world_to_viewport(transform, world_to_scene(ground))
        .expect("the hovered unit is in view")
}

fn push_intents(app: &mut App) {
    let mut feed = app.world_mut().resource_mut::<OrderMarkerFeed>();
    for (id, kind, target) in [
        (1, "move", OrderTarget::Point(Pos { x: 5.0, y: 5.0 })),
        (2, "attack", OrderTarget::Point(Pos { x: 6.0, y: 6.0 })),
        (3, "gather", OrderTarget::Point(Pos { x: 7.0, y: 7.0 })),
        (4, "rush", OrderTarget::Point(Pos { x: 8.0, y: 8.0 })),
        (5, "move", OrderTarget::None),
    ] {
        feed.intents.push(OrderMarkerIntent {
            id,
            kind: kind.into(),
            order: order(kind, target),
        });
    }
    feed.sent.push_back(SentOrderFragment {
        intent_id: 1,
        seq: 10,
        order: order("move", OrderTarget::Point(Pos { x: 5.0, y: 5.0 })),
    });
    app.world_mut()
        .resource_mut::<CommandResults>()
        .0
        .push(CommandResult::Order {
            seq: 10,
            tick: 2,
            accepted: vec![UnitId(1)],
            refused: Vec::new(),
            envelope: None,
            job_id: None,
        });
}

fn select(app: &mut App, ids: &[u32]) {
    app.world_mut().resource_mut::<SelectionFeed>().0 = ids.to_vec();
}

/// The scene with a camera, a Welcome, a hover over unit 4 and a full set of intents.
fn full_scene() -> App {
    let mut app = base_app(None, true);
    let camera = spawn_camera(&mut app);
    spawn_window_over(&mut app, camera, Pos { x: 14.0, y: 14.0 });
    select(&mut app, &[1, 2, 3]);
    push_intents(&mut app);
    app
}

fn spawn_stale_visuals(app: &mut App) {
    app.world_mut().spawn((
        VisualId {
            kind: VisualKind::Selected,
            unit: 9,
            ordinal: 0,
            generation: 0,
            identity: 0,
        },
        Mesh3d(Handle::default()),
        MeshMaterial3d::<StandardMaterial>(Handle::default()),
        Transform::default(),
    ));
    app.world_mut().spawn((
        HealthFill(9),
        Mesh3d(Handle::default()),
        MeshMaterial3d::<StandardMaterial>(Handle::default()),
        Transform::default(),
    ));
    app.world_mut().spawn((
        WorldTooltip(9),
        Text::new("stale"),
        TextFont::default(),
        Node::default(),
    ));
}

fn round(value: f32) -> String {
    let value = if value.abs() < 5e-5 { 0.0 } else { value };
    format!("{value:.4}")
}

fn vec3(value: Vec3) -> String {
    format!("[{} {} {}]", round(value.x), round(value.y), round(value.z))
}

fn val(value: Val) -> String {
    match value {
        Val::Px(px) => format!("{}px", round(px)),
        other => format!("{other:?}"),
    }
}

struct Names {
    meshes: Vec<(AssetId<Mesh>, &'static str)>,
    standard: Vec<(AssetId<StandardMaterial>, String)>,
    overlay: Vec<(AssetId<RingOverlayMaterial>, String)>,
}

impl Names {
    fn capture(world: &World) -> Self {
        let assets = world.resource::<VisualAssets>();
        let palette = world.resource::<SlotMaterials>();
        let mut meshes = Vec::new();
        for (handle, name) in [
            (&assets.ring, "ring"),
            (&assets.foreign_ring, "foreign_ring"),
            (&assets.bar, "bar"),
        ] {
            if let Some(handle) = handle {
                meshes.push((handle.id(), name));
            }
        }
        let mut standard = Vec::new();
        for (handle, name) in [
            (&assets.blocked_material, "blocked"),
            (&assets.range_material, "range"),
            (&assets.bar_background, "bar_background"),
            (&assets.bar_health, "bar_health"),
        ] {
            if let Some(handle) = handle {
                standard.push((handle.id(), name.to_string()));
            }
        }
        for (index, handle) in palette.0.iter().enumerate() {
            standard.push((handle.id(), format!("palette[{index}]")));
        }
        let mut overlay = Vec::new();
        for (index, handle) in assets.ring_overlay_materials.iter().enumerate() {
            overlay.push((handle.id(), format!("ring_overlay[{index}]")));
        }
        for (index, handle) in assets.neutral_ring_overlays.iter().enumerate() {
            overlay.push((handle.id(), format!("neutral_overlay[{index}]")));
        }
        if let Some(handle) = &assets.marker_overlay_material {
            overlay.push((handle.id(), "marker_overlay".to_string()));
        }
        Self {
            meshes,
            standard,
            overlay,
        }
    }

    fn mesh(&self, id: AssetId<Mesh>) -> String {
        self.meshes
            .iter()
            .find(|(known, _)| *known == id)
            .map_or("unknown", |(_, name)| name)
            .to_string()
    }

    fn standard(&self, id: AssetId<StandardMaterial>) -> String {
        self.standard
            .iter()
            .find(|(known, _)| *known == id)
            .map_or("unknown".to_string(), |(_, name)| name.clone())
    }

    fn overlay(&self, id: AssetId<RingOverlayMaterial>) -> String {
        self.overlay
            .iter()
            .find(|(known, _)| *known == id)
            .map_or("unknown".to_string(), |(_, name)| name.clone())
    }
}

fn gizmo_lines(world: &World, config: TypeId) -> Vec<String> {
    let Some(handle) = world
        .resource::<GizmoHandles>()
        .handles()
        .get(&config)
        .cloned()
        .flatten()
    else {
        return Vec::new();
    };
    let asset = world
        .resource::<Assets<GizmoAsset>>()
        .get(&handle)
        .expect("the gizmo asset of the frame");
    asset
        .list_positions
        .chunks(2)
        .zip(asset.list_colors.chunks(2))
        .map(|(points, colours)| {
            format!(
                "{} -> {} colour=[{} {} {}]",
                vec3(points[0]),
                vec3(points[1]),
                round(colours[0].red),
                round(colours[0].green),
                round(colours[0].blue)
            )
        })
        .collect()
}

/// One text record of the visible state after an update. Entity ids are not part of it.
fn record(app: &mut App, title: &str) -> String {
    let names = Names::capture(app.world());
    let mut lines = Vec::new();
    {
        let world = app.world_mut();
        let mut visuals = world.query::<(
            &VisualId,
            &Mesh3d,
            Option<&MeshMaterial3d<StandardMaterial>>,
            Option<&MeshMaterial3d<RingOverlayMaterial>>,
            &Transform,
            Option<&Visibility>,
            Option<&RingMetadata>,
        )>();
        for (id, mesh, standard, overlay, transform, visibility, ring) in visuals.iter(world) {
            let material = match (standard, overlay) {
                (Some(handle), None) => names.standard(handle.0.id()),
                (None, Some(handle)) => names.overlay(handle.0.id()),
                _ => "invalid".to_string(),
            };
            lines.push(format!(
                "visual {id:?} mesh={} material={material} t={} s={} r=[{} {} {} {}] vis={visibility:?} ring={ring:?}",
                names.mesh(mesh.0.id()),
                vec3(transform.translation),
                vec3(transform.scale),
                round(transform.rotation.x),
                round(transform.rotation.y),
                round(transform.rotation.z),
                round(transform.rotation.w),
            ));
        }
        let mut fills = world.query::<(
            &HealthFill,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &Transform,
            &ChildOf,
        )>();
        let fills = fills
            .iter(world)
            .map(|(fill, mesh, material, transform, parent)| {
                (
                    fill.0,
                    names.mesh(mesh.0.id()),
                    names.standard(material.0.id()),
                    *transform,
                    parent.parent(),
                )
            })
            .collect::<Vec<_>>();
        for (unit, mesh, material, transform, parent) in fills {
            lines.push(format!(
                "fill unit={unit} mesh={mesh} material={material} t={} s={} parent={:?}",
                vec3(transform.translation),
                vec3(transform.scale),
                world.get::<VisualId>(parent)
            ));
        }
        let mut labels = world.query::<(&WorldTooltip, &Text, &Node)>();
        for (tooltip, text, node) in labels.iter(world) {
            lines.push(format!(
                "tooltip unit={} text={:?} left={} top={}",
                tooltip.0,
                text.0,
                val(node.left),
                val(node.top)
            ));
        }
    }
    let world = app.world();
    for line in gizmo_lines(world, TypeId::of::<DefaultGizmoConfigGroup>()) {
        lines.push(format!("gizmo default {line}"));
    }
    for line in gizmo_lines(world, TypeId::of::<OrderMarkerGizmoConfigGroup>()) {
        lines.push(format!("gizmo order_marker {line}"));
    }
    lines.sort();
    let mut out = format!("--- {title}\n");
    for line in lines {
        let _ = writeln!(out, "{line}");
    }
    for shown in &world.resource::<MarkersShown>().0 {
        let _ = writeln!(out, "markers_shown {shown}");
    }
    let state = world.resource::<VisualState>();
    let mut blocked_active = state.blocked_active.iter().copied().collect::<Vec<_>>();
    blocked_active.sort_unstable();
    let mut blocked_since = state.blocked_since.keys().copied().collect::<Vec<_>>();
    blocked_since.sort_unstable();
    let feed = world.resource::<OrderMarkerFeed>();
    let _ = writeln!(
        out,
        "state active={} hover={:?} hover_started={} generation={} session={:?} retained_markers={:?} blocked_active={blocked_active:?} blocked_since={blocked_since:?} feed_intents={} feed_sent={}",
        state.active,
        state.hover_unit,
        state.hover_started.is_some(),
        state.generation,
        state.session,
        state
            .markers
            .iter()
            .map(|marker| marker.id)
            .collect::<Vec<_>>(),
        feed.intents.len(),
        feed.sent.len(),
    );
    out
}

fn visual_entities(app: &mut App) -> HashMap<String, Entity> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &VisualId)>();
    query
        .iter(world)
        .map(|(entity, id)| (format!("{id:?}"), entity))
        .collect()
}

fn count<C: Component>(app: &mut App) -> usize {
    let world = app.world_mut();
    world.query::<&C>().iter(world).count()
}

/// Every scene, in a fixed order. This text is the fixture.
fn all_scenes() -> String {
    let mut out = String::new();

    // Exit 1: no camera. Stale visuals of every kind must be removed.
    let mut app = base_app(None, true);
    select(&mut app, &[1, 2, 3]);
    push_intents(&mut app);
    spawn_stale_visuals(&mut app);
    for _ in 0..3 {
        app.update();
    }
    let _ = writeln!(out, "=== exit: no camera");
    out += &record(&mut app, "after 3 updates");

    // Exit 2: a camera but no Welcome.
    let mut app = base_app(None, false);
    spawn_camera(&mut app);
    select(&mut app, &[1, 2, 3]);
    push_intents(&mut app);
    spawn_stale_visuals(&mut app);
    for _ in 0..3 {
        app.update();
    }
    let _ = writeln!(out, "=== exit: no welcome");
    out += &record(&mut app, "after 3 updates");

    // Exit 3: a camera and a Welcome but the replica is gone. The session is none, so the
    // intent feed is cleared as well.
    let mut app = base_app(None, true);
    spawn_camera(&mut app);
    select(&mut app, &[1, 2, 3]);
    push_intents(&mut app);
    spawn_stale_visuals(&mut app);
    app.world_mut().resource_mut::<ReplicaState>().replica = None;
    for _ in 0..3 {
        app.update();
    }
    let _ = writeln!(out, "=== exit: no replica");
    out += &record(&mut app, "after 3 updates");

    // The full scene over three updates. Between the updates: a health change and an old
    // hover start (update 2), a deselection and a moved unit (update 3).
    let mut app = full_scene();
    let _ = writeln!(out, "=== full scene");
    app.update();
    out += &record(&mut app, "update 1");
    app.world_mut().resource_mut::<VisualState>().hover_started =
        Some(Instant::now() - Duration::from_secs(1));
    app.world_mut()
        .resource_mut::<ReplicaState>()
        .replica
        .as_mut()
        .unwrap()
        .healths[2]
        .current = 10;
    app.update();
    out += &record(&mut app, "update 2");
    select(&mut app, &[1, 3]);
    app.world_mut().resource_mut::<Presented>().units[0].pos = Pos { x: 11.0, y: 10.0 };
    app.update();
    out += &record(&mut app, "update 3");

    // A new connection epoch is a new session: the generation moves and every visual is
    // rebuilt.
    let mut app = full_scene();
    app.update();
    app.world_mut()
        .resource_mut::<ReceiptWitnesses>()
        .next_epoch = 2;
    app.update();
    let _ = writeln!(out, "=== session change");
    out += &record(&mut app, "update 2 after the epoch change");

    // Health mode Always: bars for every unit, and none for a unit with no health.
    let mut app = full_scene();
    app.world_mut().resource_mut::<HealthDisplay>().0 = HealthMode::Always;
    select(&mut app, &[]);
    app.update();
    let _ = writeln!(out, "=== health mode always, nothing selected");
    out += &record(&mut app, "update 1");

    // A full update, then the camera goes: `active` must fall back to false and every visual,
    // fill and label must go (the ring collector reads `active`).
    let mut app = full_scene();
    app.update();
    app.update();
    despawn_camera(&mut app);
    app.update();
    let _ = writeln!(out, "=== exit after a full update: camera removed");
    out += &record(&mut app, "update 3");

    // The same with the Welcome removed.
    let mut app = full_scene();
    app.update();
    app.update();
    app.world_mut().resource_mut::<ReplicaState>().welcome = None;
    app.update();
    let _ = writeln!(out, "=== exit after a full update: welcome removed");
    out += &record(&mut app, "update 3");

    // Hover on an own unit: the solid ring and no underlay; then the hover leaves.
    let mut app = base_app(None, true);
    let camera = spawn_camera(&mut app);
    spawn_window_over(&mut app, camera, Pos { x: 10.0, y: 14.0 });
    let _ = writeln!(out, "=== hover on an own unit, then the hover leaves");
    app.update();
    out += &record(&mut app, "update 1");
    app.world_mut().resource_mut::<VisualState>().hover_started =
        Some(Instant::now() - Duration::from_secs(1));
    app.update();
    out += &record(&mut app, "update 2 (tooltip)");
    set_cursor(&mut app, None);
    app.update();
    out += &record(&mut app, "update 3 (no hover)");

    // A blocked mark that is 3 s old is past its 2 s lifetime and is not drawn, although the
    // unit is still blocked.
    let mut app = base_app(None, true);
    spawn_camera(&mut app);
    app.update();
    app.world_mut()
        .resource_mut::<VisualState>()
        .blocked_since
        .insert(4, Instant::now() - Duration::from_secs(3));
    app.update();
    let _ = writeln!(out, "=== blocked mark past its 2 s lifetime");
    out += &record(&mut app, "update 2");

    // Order markers past their 60 s lifetime are retired.
    let mut app = base_app(None, true);
    spawn_camera(&mut app);
    push_intents(&mut app);
    app.update();
    for marker in &mut app.world_mut().resource_mut::<VisualState>().markers {
        marker.started = Instant::now() - Duration::from_secs(61);
    }
    app.update();
    let _ = writeln!(out, "=== order markers past their 60 s lifetime");
    out += &record(&mut app, "update 2");
    out
}

fn despawn_camera(app: &mut App) {
    let world = app.world_mut();
    let camera = world
        .query_filtered::<Entity, With<Camera3d>>()
        .single(world)
        .expect("one camera");
    world.despawn(camera);
}

fn set_cursor(app: &mut App, cursor: Option<Vec2>) {
    let world = app.world_mut();
    let mut windows = world.query_filtered::<&mut Window, With<PrimaryWindow>>();
    windows
        .single_mut(world)
        .expect("one primary window")
        .set_cursor_position(cursor);
}

#[test]
fn scene_oracle_matches_the_committed_fixture() {
    assert!(
        std::env::var_os("TURTS_BLESS_SCENES").is_none(),
        "TURTS_BLESS_SCENES is set: run the ignored bless test; this test never passes with it set"
    );
    let scenes = all_scenes();
    if scenes != FIXTURE {
        let first = scenes
            .lines()
            .zip(FIXTURE.lines())
            .position(|(new, old)| new != old);
        let (new, old) = first.map_or(("(a line count differs)", ""), |at| {
            (
                scenes.lines().nth(at).unwrap_or(""),
                FIXTURE.lines().nth(at).unwrap_or(""),
            )
        });
        panic!(
            "the scene record differs from the fixture at line {:?}\n  now:     {new}\n  fixture: {old}",
            first.map(|at| at + 1)
        );
    }
}

/// Writes a new scene record to the path in `TURTS_BLESS_SCENES`. It compares nothing.
/// Run: `TURTS_BLESS_SCENES=<path> cargo test -p turts_client --lib bless_scene_fixture -- --ignored`.
#[test]
#[ignore = "writes the scene record; needs TURTS_BLESS_SCENES"]
fn bless_scene_fixture() {
    let path = std::env::var_os("TURTS_BLESS_SCENES").expect("set TURTS_BLESS_SCENES to a path");
    std::fs::write(path, all_scenes()).expect("write the scene record");
}

#[test]
fn exit_after_a_full_update_clears_active_and_every_visual() {
    for remove_camera in [true, false] {
        let mut app = full_scene();
        app.update();
        assert!(app.world().resource::<VisualState>().active);
        assert!(count::<VisualId>(&mut app) > 0);
        if remove_camera {
            despawn_camera(&mut app);
        } else {
            app.world_mut().resource_mut::<ReplicaState>().welcome = None;
        }
        app.update();
        assert!(
            !app.world().resource::<VisualState>().active,
            "remove_camera={remove_camera}"
        );
        assert_eq!(count::<VisualId>(&mut app), 0);
        assert_eq!(count::<HealthFill>(&mut app), 0);
        assert_eq!(count::<WorldTooltip>(&mut app), 0);
        assert!(app.world().resource::<MarkersShown>().0.is_empty());
    }
}

#[test]
fn every_early_exit_removes_all_visuals_labels_and_fills() {
    for (camera, welcome, replica) in [
        (false, true, true),
        (true, false, true),
        (true, true, false),
    ] {
        let mut app = base_app(None, welcome);
        if camera {
            spawn_camera(&mut app);
        }
        select(&mut app, &[1, 2, 3]);
        spawn_stale_visuals(&mut app);
        if !replica {
            app.world_mut().resource_mut::<ReplicaState>().replica = None;
        }
        app.update();
        assert_eq!(
            count::<VisualId>(&mut app),
            0,
            "{camera} {welcome} {replica}"
        );
        assert_eq!(
            count::<HealthFill>(&mut app),
            0,
            "{camera} {welcome} {replica}"
        );
        assert_eq!(
            count::<WorldTooltip>(&mut app),
            0,
            "{camera} {welcome} {replica}"
        );
        assert!(!app.world().resource::<VisualState>().active);
        assert!(app.world().resource::<MarkersShown>().0.is_empty());
    }
}

#[test]
fn the_full_scene_keeps_its_entities_between_updates() {
    let mut app = full_scene();
    app.update();
    let first = visual_entities(&mut app);
    assert!(first.len() >= 8, "the full scene draws many visuals");
    app.update();
    app.update();
    let third = visual_entities(&mut app);
    assert_eq!(first, third, "retained visuals keep their entities");
    assert!(app.world().resource::<VisualState>().active);
}
