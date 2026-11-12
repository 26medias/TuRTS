//! PRES-PROBE probe 2: the HUD change counter (the red build of the client cost rule, design 9).
//! Scratch test. It is NOT wired into the product: it lives in docs/research/mp1/probes/pres-probe/
//! and is copied to crates/turts_client/tests/ only while the probe runs.
//!
//! A headless client app (the pattern of tests/widgets_lifecycle.rs `installed_app_with_updates`), the HUD of
//! mods/forge (`interface/hud.ron`), a static replica (no delta arrives), 61 updates.
//! A counter system in `First` runs once for each frame. `Ref<T>::is_changed()` in a system answers "changed
//! since this system last ran", so the count in `First` of frame k + 1 is the set of writes of frame k.

use std::{collections::VecDeque, path::PathBuf, time::Instant};

use bevy::{
    camera::{
        CameraPlugin, ComputedCameraValues, RenderTarget, RenderTargetInfo, Viewport,
    },
    image::{ImagePlugin, TextureAtlasPlugin},
    input::InputPlugin,
    input_focus::{InputDispatchPlugin, InputFocusPlugin},
    mesh::MeshPlugin,
    picking::{InteractionPlugin, PickingPlugin, pointer::PointerId},
    prelude::*,
    text::TextPlugin,
    ui::UiPlugin,
    ui_widgets::{ButtonPlugin, ScrollAreaPlugin},
    window::{PrimaryWindow, WindowPlugin, WindowRef, WindowResolution},
};
use turts_client::{
    acquire::{Acquired, build_acquired},
    commands::{CommandSendStatus, PendingCommands},
    core_adapter::{CoreAdapterPlugin, CoreInputQueue},
    ingest::{CommandResults, ReplicaState, WelcomeInfo},
    join::{AcquiredPackages, Phase},
    present::Presented,
    replica::{Replica, UnitReplica},
    ui_shared::Selection,
    widgets::{WidgetsPlugin, primitives::AuthoredWidgetId},
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::BalanceState;

/// One row for each frame: changed counts by component type, split into authored widget entities and
/// other entities (the tooltip views).
#[derive(Resource, Default)]
struct HudCounts {
    frames: Vec<FrameCount>,
    widget_entities: usize,
    other_ui_entities: usize,
}

#[derive(Default, Clone, Copy, Debug)]
struct FrameCount {
    widget_text: usize,
    widget_node: usize,
    widget_visibility: usize,
    other_text: usize,
    other_node: usize,
    other_visibility: usize,
}

impl FrameCount {
    fn widget_total(&self) -> usize {
        self.widget_text + self.widget_node + self.widget_visibility
    }
    fn all_total(&self) -> usize {
        self.widget_total() + self.other_text + self.other_node + self.other_visibility
    }
}

#[allow(clippy::type_complexity)]
fn count_changed(
    mut counts: ResMut<HudCounts>,
    widgets: Query<
        (
            Option<Ref<Text>>,
            Option<Ref<Node>>,
            Option<Ref<Visibility>>,
            &AuthoredWidgetId,
        ),
    >,
    others: Query<
        (
            Option<Ref<Text>>,
            Option<Ref<Node>>,
            Option<Ref<Visibility>>,
        ),
        (Without<AuthoredWidgetId>, With<Node>),
    >,
) {
    let mut row = FrameCount::default();
    let list = counts.frames.len() == 5;
    for (t, n, v, id) in &widgets {
        if list {
            println!(
                "FRAME-5 changed id={:?} Text={} Node={} Visibility={}",
                id.0,
                changed_t(&t),
                changed_n(&n),
                changed_v(&v)
            );
        }
        row.widget_text += usize::from(changed_t(&t));
        row.widget_node += usize::from(changed_n(&n));
        row.widget_visibility += usize::from(changed_v(&v));
    }
    for (t, n, v) in &others {
        row.other_text += usize::from(changed_t(&t));
        row.other_node += usize::from(changed_n(&n));
        row.other_visibility += usize::from(changed_v(&v));
    }
    counts.widget_entities = widgets.iter().count();
    counts.other_ui_entities = others.iter().count();
    counts.frames.push(row);
}

fn changed_t(r: &Option<Ref<Text>>) -> bool {
    r.as_ref().is_some_and(|r| r.is_changed())
}
fn changed_n(r: &Option<Ref<Node>>) -> bool {
    r.as_ref().is_some_and(|r| r.is_changed())
}
fn changed_v(r: &Option<Ref<Visibility>>) -> bool {
    r.as_ref().is_some_and(|r| r.is_changed())
}

fn forge_fixture() -> (Acquired, Replica, u32) {
    let forge = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/forge");
    let archive = PackageArchive::parse(&pack_dir(&forge).expect("pack mods/forge"))
        .expect("parse mods/forge");
    let acquired = build_acquired(&[archive], String::new()).expect("resolve mods/forge");
    let content = &acquired.content;
    let mut replica = Replica::new(20);
    // One own unit of each content id, so every list and bar of the HUD has a source.
    let mut selected = 0;
    for (index, (id, def)) in content.catalog.units.iter().enumerate() {
        let unit_id = 10 + index as u32;
        if def.research.is_some() && selected == 0 {
            selected = unit_id;
        }
        replica.units.insert(
            unit_id,
            UnitReplica {
                content_id: id.to_string(),
                owner_slot: 0,
                facing: None,
                samples: VecDeque::from([(
                    1,
                    turts_net_protocol::Pos {
                        x: 8.0 + index as f32 * 3.0,
                        y: 8.0,
                    },
                )]),
            },
        );
    }
    if selected == 0 {
        selected = *replica.units.keys().next().expect("a unit");
    }
    replica.economy.balances = content
        .catalog
        .resources
        .keys()
        .map(|id| BalanceState {
            resource: id.to_string(),
            on_hand: 500,
            reserved: 0,
        })
        .collect();
    replica.snapshot_tick = Some(1);
    replica.last_applied_tick = Some(1);
    (acquired, replica, selected)
}

fn hud_app() -> App {
    let (acquired, replica, selected) = forge_fixture();
    let state = ReplicaState {
        welcome: Some(WelcomeInfo {
            slot: 0,
            tick: 1,
            tick_hz: 20,
            match_id: 0,
        }),
        content: Some(acquired.content.clone()),
        replica: Some(replica.clone()),
        ..Default::default()
    };
    let (width, height, scale) = (1280u32, 720u32, 1.0f32);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .add_plugins((
            InputPlugin,
            WindowPlugin {
                primary_window: None,
                ..default()
            },
            TransformPlugin,
            ImagePlugin::default(),
            TextureAtlasPlugin,
            MeshPlugin,
            TextPlugin,
            CameraPlugin,
        ))
        .add_plugins((
            PickingPlugin,
            InteractionPlugin,
            InputFocusPlugin,
            InputDispatchPlugin,
            UiPlugin,
            ButtonPlugin,
            ScrollAreaPlugin,
        ))
        .add_plugins(CoreAdapterPlugin)
        .insert_resource(Phase::Joined { slot: 0 })
        .insert_resource(state)
        .insert_resource(AcquiredPackages(acquired))
        .insert_resource(Selection(Some(selected)))
        .insert_resource(Presented {
            frame_ns: 1,
            t: 1.0,
            latest_tick: 1,
            units: replica
                .units
                .iter()
                .map(|(&unit_id, unit)| turts_client::present::PresentedUnit {
                    unit_id,
                    content_id: unit.content_id.clone(),
                    pos: unit.samples.back().expect("unit position").1,
                    lo_tick: 1,
                    hi_tick: 1,
                    fraction: 0.0,
                })
                .collect(),
        })
        .insert_resource(turts_client::debug_feed::SelectionFeed(vec![selected]))
        .init_resource::<CoreInputQueue>()
        .init_resource::<PendingCommands>()
        .init_resource::<CommandResults>()
        .init_resource::<CommandSendStatus>()
        .init_resource::<HudCounts>()
        .add_systems(First, count_changed)
        .add_plugins(WidgetsPlugin);
    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: WindowResolution::new(width, height).with_scale_factor_override(scale),
                ..default()
            },
            PrimaryWindow,
        ))
        .id();
    app.world_mut().spawn((
        Camera3d::default(),
        Camera {
            computed: ComputedCameraValues {
                target_info: Some(RenderTargetInfo {
                    physical_size: UVec2::new(width, height),
                    scale_factor: scale,
                }),
                ..default()
            },
            viewport: Some(Viewport {
                physical_size: UVec2::new(width, height),
                ..default()
            }),
            ..default()
        },
        RenderTarget::Window(WindowRef::Entity(window)),
        IsDefaultUiCamera,
        Transform::from_xyz(32.0, 40.0, 62.0).looking_at(Vec3::new(32.0, 0.0, 32.0), Vec3::Y),
    ));
    app.world_mut().spawn(PointerId::Mouse);
    app.finish();
    app.cleanup();
    app
}

/// The rule of design 9: after frame 2, with a static replica, no widget entity has a changed
/// `Text`, `Node` or `Visibility`. RED on today's tree.
#[test]
fn static_replica_hud_changes_no_text_node_or_visibility_after_frame_two() {
    let mut app = hud_app();
    // 61 updates: the counter in `First` of update k + 1 reports the writes of frame k (1 to 60).
    for _ in 0..61 {
        app.update();
    }
    let counts = app.world().resource::<HudCounts>();
    println!(
        "widget_entities(AuthoredWidgetId)={} other_ui_entities(Node, no id)={}",
        counts.widget_entities, counts.other_ui_entities
    );
    println!("frame  w.Text w.Node w.Vis | o.Text o.Node o.Vis   (frame k = writes made in update k)");
    for (i, row) in counts.frames.iter().enumerate().skip(1) {
        let frame = i; // row i is counted in First of update i + 1: it holds the writes of update i
        println!(
            "{frame:>5}  {:>6} {:>6} {:>5} | {:>6} {:>6} {:>5}",
            row.widget_text,
            row.widget_node,
            row.widget_visibility,
            row.other_text,
            row.other_node,
            row.other_visibility
        );
    }
    // Frames 3 to 60.
    let late: Vec<&FrameCount> = counts.frames.iter().enumerate().skip(3).map(|(_, r)| r).collect();
    let frames_with_widget_changes = late.iter().filter(|r| r.widget_total() > 0).count();
    let frames_with_any_changes = late.iter().filter(|r| r.all_total() > 0).count();
    let sum_text: usize = late.iter().map(|r| r.widget_text + r.other_text).sum();
    let sum_node: usize = late.iter().map(|r| r.widget_node + r.other_node).sum();
    let sum_vis: usize = late.iter().map(|r| r.widget_visibility + r.other_visibility).sum();
    println!(
        "frames 3..=60: {} frames; with a widget change={frames_with_widget_changes}; with any change={frames_with_any_changes}; \
         total changed Text={sum_text} Node={sum_node} Visibility={sum_vis}",
        late.len()
    );
    assert!(counts.widget_entities > 0, "the HUD has widget entities");
    assert_eq!(
        frames_with_any_changes, 0,
        "a static replica must change no Text, Node or Visibility after frame 2"
    );
}

/// The time question: 600 updates of the same app, wall time. Run with --nocapture and --ignored.
#[test]
#[ignore]
fn hud_update_time_600_frames() {
    let mut app = hud_app();
    for _ in 0..10 {
        app.update(); // warm up: tree build, fonts, layout
    }
    let start = Instant::now();
    for _ in 0..600 {
        app.update();
    }
    let total = start.elapsed();
    println!(
        "TIMING 600 updates: total_ms={:.1} per_update_us={:.1}",
        total.as_secs_f64() * 1e3,
        total.as_secs_f64() * 1e6 / 600.0
    );
}
