use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use std::{
    collections::{HashMap, VecDeque},
    path::Path,
};
use turts_client::{
    acquire::Acquired,
    commands::{CommandSendStatus, PendingCommands},
    core_adapter::{CoreAdapterPlugin, CoreInputQueue, CoreRuntime},
    ingest::{CommandResults, ReplicaState, WelcomeInfo},
    join::AcquiredPackages,
    present::{Presented, PresentedUnit},
    replica::{Replica, UnitReplica},
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::Pos;

fn fixture(unit_content: &str) -> App {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
    let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
    let content = turts_client::content::ClientContent::from_archives(&[archive]).unwrap();
    let acquired = Acquired {
        lockfile_hash: String::new(),
        visuals: HashMap::new(),
        map_size: (64.0, 48.0),
        unit_names: HashMap::new(),
        content: content.clone(),
        presentation: Default::default(),
        verified_root_mod: None,
    };
    let mut replica = Replica::new(20);
    replica.units.insert(
        1,
        UnitReplica {
            content_id: unit_content.into(),
            owner_slot: 0,
            facing: None,
            samples: VecDeque::from([(1, Pos { x: 14.0, y: 28.0 })]),
        },
    );
    replica.last_applied_tick = Some(1);
    replica.snapshot_tick = Some(1);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(CoreAdapterPlugin)
        .add_plugins(turts_client::camera::ClientCameraPlugin)
        .insert_resource(ReplicaState {
            replica: Some(replica),
            welcome: Some(WelcomeInfo {
                slot: 0,
                tick: 1,
                tick_hz: 20,
                match_id: 1,
            }),
            content: Some(content),
            config: Default::default(),
            match_start: None,
            reorder_grace_override: None,
        })
        .insert_resource(AcquiredPackages(acquired))
        .insert_resource(Presented {
            frame_ns: 1,
            t: 1.0,
            latest_tick: 1,
            units: vec![PresentedUnit {
                unit_id: 1,
                content_id: unit_content.into(),
                pos: Pos { x: 14.0, y: 28.0 },
                lo_tick: 1,
                hi_tick: 1,
                fraction: 0.0,
            }],
        })
        .init_resource::<PendingCommands>()
        .init_resource::<CommandSendStatus>()
        .init_resource::<CommandResults>();
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.finish();
    app.cleanup();
    app.update();
    app
}

fn click_body(app: &mut App, point: Vec3, button: turts_mod_api::Button) -> Vec2 {
    let camera = app
        .world()
        .resource::<CoreRuntime>()
        .core()
        .unwrap()
        .camera_state();
    let view = turts_interaction::Viewport {
        size: Vec2::new(1280.0, 720.0),
        scale_factor: 1.0,
    };
    let horizontal = camera.height * camera.pitch.tan();
    let eye = Vec3::new(
        camera.look_at.x + camera.yaw.sin() * horizontal,
        camera.height,
        camera.look_at.y + camera.yaw.cos() * horizontal,
    );
    let world_to_view = Transform::from_translation(eye)
        .looking_at(Vec3::new(camera.look_at.x, 0.0, camera.look_at.y), Vec3::Y)
        .to_matrix()
        .inverse();
    let clip = turts_interaction::projection::perspective_matrix(&view, camera)
        * world_to_view
        * point.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    let px = Vec2::new((ndc.x + 1.0) * 640.0, (1.0 - ndc.y) * 360.0);
    app.world_mut().resource_mut::<CoreInputQueue>().0 = vec![
        turts_interaction::InputEvent::PointerDown {
            button,
            px,
            over_widget: false,
        },
        turts_interaction::InputEvent::PointerUp {
            button,
            px,
            over_widget: false,
        },
    ];
    app.update();
    px
}

#[test]
fn visible_bodies_select_workers_soldiers_and_buildings() {
    for content in [
        "skirmish:worker",
        "skirmish:soldier",
        "skirmish:base",
        "skirmish:barracks",
    ] {
        let mut app = fixture(content);
        let lift = turts_client::render::mesh_lift(
            &app.world()
                .resource::<ReplicaState>()
                .content
                .as_ref()
                .unwrap()
                .shapes[content],
        );
        for fraction in [0.35, 0.5, 0.85] {
            let px = click_body(
                &mut app,
                Vec3::new(14.0, lift * 2.0 * fraction, 28.0),
                turts_mod_api::Button::Left,
            );
            assert_eq!(
                app.world()
                    .resource::<CoreRuntime>()
                    .core()
                    .unwrap()
                    .selected_units(),
                &[turts_net_protocol::UnitId(1)],
                "lost {content} body click at {px:?}"
            );
        }
    }
}

#[test]
fn visible_timber_body_selects_and_receives_the_workers_harvest_order() {
    let mut app = fixture("skirmish:worker");
    app.world_mut()
        .resource_mut::<ReplicaState>()
        .replica
        .as_mut()
        .unwrap()
        .nodes
        .insert(
            turts_net_protocol::NodeId(1),
            turts_net_protocol::NodeState {
                id: turts_net_protocol::NodeId(1),
                content_id: "skirmish:trees".into(),
                pos: Pos { x: 20.0, y: 28.0 },
                remaining: 2000,
            },
        );
    app.update();
    click_body(
        &mut app,
        Vec3::new(20.0, 1.0, 28.0),
        turts_mod_api::Button::Left,
    );
    assert_eq!(
        app.world()
            .resource::<CoreRuntime>()
            .core()
            .unwrap()
            .selected_node(),
        Some(turts_net_protocol::NodeId(1))
    );
    click_body(
        &mut app,
        Vec3::new(14.0, 0.55, 28.0),
        turts_mod_api::Button::Left,
    );
    click_body(
        &mut app,
        Vec3::new(20.0, 1.0, 28.0),
        turts_mod_api::Button::Right,
    );
    let commands = &app.world().resource::<PendingCommands>().0;
    assert_eq!(commands.len(), 1);
    let turts_client::commands::PendingCommand::Order { order, .. } = &commands[0] else {
        panic!("expected a normal order");
    };
    assert_eq!(order.order, "skirmish:order_gather");
    assert_eq!(
        order.target,
        turts_net_protocol::OrderTarget::Node(turts_net_protocol::NodeId(1))
    );
}

#[test]
fn visible_foreign_body_receives_an_attack_order() {
    let mut app = fixture("skirmish:soldier");
    app.world_mut()
        .resource_mut::<ReplicaState>()
        .replica
        .as_mut()
        .unwrap()
        .units
        .insert(
            2,
            UnitReplica {
                content_id: "skirmish:soldier".into(),
                owner_slot: 1,
                facing: None,
                samples: VecDeque::from([(1, Pos { x: 20.0, y: 28.0 })]),
            },
        );
    app.world_mut()
        .resource_mut::<Presented>()
        .units
        .push(PresentedUnit {
            unit_id: 2,
            content_id: "skirmish:soldier".into(),
            pos: Pos { x: 20.0, y: 28.0 },
            lo_tick: 1,
            hi_tick: 1,
            fraction: 0.0,
        });
    app.update();
    click_body(
        &mut app,
        Vec3::new(14.0, 0.8, 28.0),
        turts_mod_api::Button::Left,
    );
    click_body(
        &mut app,
        Vec3::new(20.0, 0.8, 28.0),
        turts_mod_api::Button::Right,
    );
    let commands = &app.world().resource::<PendingCommands>().0;
    assert_eq!(commands.len(), 1);
    let turts_client::commands::PendingCommand::Order { order, .. } = &commands[0] else {
        panic!("expected a normal order");
    };
    assert_eq!(order.order, "skirmish:order_attack");
    assert_eq!(
        order.target,
        turts_net_protocol::OrderTarget::Unit(turts_net_protocol::UnitId(2))
    );
}
