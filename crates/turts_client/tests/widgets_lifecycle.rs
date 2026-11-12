//! Installed authored-widget lifecycle contracts, exercised as a crate consumer.

use std::{collections::VecDeque, path::PathBuf};

use bevy::{
    camera::{
        CameraPlugin, ComputedCameraValues, NormalizedRenderTarget, RenderTarget, RenderTargetInfo,
        Viewport,
    },
    image::{ImagePlugin, TextureAtlasPlugin},
    input::{
        ButtonState, InputPlugin,
        keyboard::{Key, KeyCode, KeyboardInput},
        mouse::{MouseButton, MouseButtonInput},
    },
    input_focus::{InputDispatchPlugin, InputFocusPlugin},
    mesh::MeshPlugin,
    picking::{
        InteractionPlugin, PickingPlugin,
        pointer::{Location, PointerAction, PointerId, PointerInput},
    },
    prelude::*,
    text::TextPlugin,
    ui::{CalculatedClip, ComputedNode, Pressed, ScrollPosition, UiGlobalTransform, UiPlugin},
    ui_widgets::{Button, ButtonPlugin, ScrollArea, ScrollAreaPlugin},
    window::{PrimaryWindow, WindowEvent, WindowPlugin, WindowRef, WindowResolution},
};
use turts_client::{
    acquire::{Acquired, build_acquired},
    commands::{ClientCommand, CommandSendStatus, PendingCommand, PendingCommands},
    core_adapter::{CoreAdapterPlugin, CoreInputQueue, CoreRuntime},
    ingest::{CommandResults, ReplicaState, WelcomeInfo},
    join::{AcquiredPackages, Phase},
    present::Presented,
    replica::{Replica, UnitReplica},
    ui_shared::Selection,
    widgets::{
        WidgetsPlugin,
        primitives::{AuthoredWidgetId, WidgetAction, WidgetButtonAction, WidgetEnabled},
        snapshot,
    },
};
use turts_interaction::Viewport as InteractionViewport;
use turts_mod_api::{
    ActionField, Anchor, ButtonAction, Dimension, Direction, ItemField, Primitive, Size, Source,
    TextValue, Widget,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{BalanceState, JobId, JobStage, QueueState, UnitId};

const PRODUCER: u32 = 7;
const FIRST_JOB: u64 = 101;

fn accepted_fixture() -> (Acquired, Replica, u32, String) {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
    let archive = PackageArchive::parse(&pack_dir(&base).expect("pack accepted base fixture"))
        .expect("parse accepted base fixture");
    let acquired = build_acquired(&[archive], String::new()).expect("resolve accepted fixture");
    let content = &acquired.content;
    let (producer_content, definition) = content
        .catalog
        .units
        .iter()
        .find(|(_, unit)| unit.producer.is_some())
        .expect("accepted catalog contains a producer");
    let recipe = definition
        .producer
        .as_ref()
        .and_then(|producer| producer.recipes.first())
        .expect("producer has an authored recipe")
        .to_string();

    let mut replica = Replica::new(20);
    replica.units.insert(
        PRODUCER,
        UnitReplica {
            content_id: producer_content.to_string(),
            owner_slot: 0,
            facing: None,
            samples: VecDeque::from([(1, turts_net_protocol::Pos { x: 8.0, y: 8.0 })]),
        },
    );
    replica.economy.balances = content
        .catalog
        .resources
        .keys()
        .map(|id| BalanceState {
            resource: id.to_string(),
            on_hand: 50_000,
            reserved: 0,
        })
        .collect();
    (acquired, replica, PRODUCER, recipe)
}

fn set_jobs(replica: &mut Replica, producer: u32, recipe: &str, job_ids: &[u64]) {
    replica.economy.queues = if job_ids.is_empty() {
        Vec::new()
    } else {
        vec![QueueState {
            producer: UnitId(producer),
            enabled: true,
            jobs: job_ids
                .iter()
                .map(|id| turts_net_protocol::JobState {
                    id: JobId(*id),
                    recipe: recipe.to_owned(),
                    stage: JobStage::Running,
                    duration_ticks: 20,
                    charged: Vec::new(),
                    reserved: Vec::new(),
                })
                .collect(),
        }]
    };
}

fn authored_list(id: &str, source: Source, item_action: ActionField, max_items: u16) -> Widget {
    Widget {
        id: id.into(),
        at: Some(turts_mod_api::At {
            anchor: Anchor::TopLeft,
            offset: (18.0, 18.0),
            pivot: None,
        }),
        size: Size {
            w: Dimension::Px(250.0),
            h: Dimension::Px(76.0),
        },
        layer: 0,
        visible: Vec::new(),
        tooltip: None,
        widget: Primitive::List {
            source,
            direction: Direction::Vertical,
            gap: 2,
            max_items,
            item: Box::new(Widget {
                id: "action".into(),
                at: None,
                size: Size {
                    w: Dimension::Px(230.0),
                    h: Dimension::Px(24.0),
                },
                layer: 0,
                visible: Vec::new(),
                tooltip: Some(turts_mod_api::Value::Item(ItemField::Reason)),
                widget: Primitive::Button {
                    icon: None,
                    label: Some(TextValue::Item(ItemField::Name)),
                    action: ButtonAction::Item(item_action),
                    hotkey_label: false,
                    font: Default::default(),
                    size: Default::default(),
                },
            }),
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn installed_app(
    acquired: Acquired,
    replica: Replica,
    source: Source,
    item_action: ActionField,
    job_ids: &[u64],
    scale: f32,
    width: u32,
    height: u32,
) -> (App, Entity, Entity, u32) {
    installed_app_with_updates(
        acquired,
        replica,
        source,
        item_action,
        job_ids,
        scale,
        width,
        height,
        4,
    )
}

#[allow(clippy::too_many_arguments)]
fn installed_app_with_updates(
    mut acquired: Acquired,
    mut replica: Replica,
    source: Source,
    item_action: ActionField,
    job_ids: &[u64],
    scale: f32,
    width: u32,
    height: u32,
    settle_updates: usize,
) -> (App, Entity, Entity, u32) {
    let producer = PRODUCER;
    let recipe = acquired
        .content
        .catalog
        .units
        .values()
        .find_map(|unit| unit.producer.as_ref())
        .and_then(|producer| producer.recipes.first())
        .expect("fixture recipe")
        .to_string();
    set_jobs(&mut replica, producer, &recipe, job_ids);
    replica.snapshot_tick = Some(1);
    replica.last_applied_tick = Some(1);
    let mut interface = acquired
        .presentation
        .interface
        .take()
        .expect("accepted interface");
    interface.root = vec![authored_list("test.list", source, item_action, 16)];
    acquired.presentation.interface = Some(interface);

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
        .insert_resource(Selection(Some(producer)))
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
                    pos: unit.samples.back().expect("fixture unit position").1,
                    lo_tick: 1,
                    hi_tick: 1,
                    fraction: 0.0,
                })
                .collect(),
        })
        .insert_resource(turts_client::debug_feed::SelectionFeed(vec![producer]))
        .init_resource::<CoreInputQueue>()
        .init_resource::<PendingCommands>()
        .init_resource::<CommandResults>()
        .init_resource::<CommandSendStatus>()
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
    let camera = app
        .world_mut()
        .spawn((
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
        ))
        .id();
    app.world_mut().spawn(PointerId::Mouse);
    app.finish();
    app.cleanup();
    for _ in 0..settle_updates {
        app.update();
    }
    (app, window, camera, producer)
}

fn authored_button(app: &App, action: &WidgetAction) -> Entity {
    app.world()
        .iter_entities()
        .find(|entity| {
            entity.contains::<Button>()
                && entity
                    .get::<WidgetButtonAction>()
                    .is_some_and(|current| &current.0 == action)
        })
        .unwrap_or_else(|| {
            let ids = app
                .world()
                .iter_entities()
                .filter_map(|entity| entity.get::<AuthoredWidgetId>().map(|id| id.0.clone()))
                .collect::<Vec<_>>();
            panic!(
                "installed action button {:?}; phase={:?}, widget_ids={ids:?}, asset_revision={:?}",
                action,
                app.world().get_resource::<Phase>(),
                app.world()
                    .get_resource::<turts_client::widgets::assets::WidgetAssets>()
                    .map(|assets| assets.revision)
            )
        })
        .id()
}

fn pointer_location(app: &App, window: Entity, entity: Entity, scale: f32) -> Location {
    let transform = app
        .world()
        .get::<UiGlobalTransform>(entity)
        .expect("computed UI transform");
    let center = transform.to_scale_angle_translation().2 / scale;
    let target = WindowRef::Entity(window)
        .normalize(Some(window))
        .expect("window target");
    Location {
        target: NormalizedRenderTarget::Window(target),
        position: center,
    }
}

fn press(app: &mut App, window: Entity, camera: Entity, entity: Entity, scale: f32) {
    let _ = camera;
    let location = pointer_location(app, window, entity, scale);
    let point = location.position;
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location.clone(),
        PointerAction::Move { delta: Vec2::ZERO },
    ));
    app.update();
    app.world_mut()
        .get_mut::<Window>(window)
        .expect("primary window")
        .set_cursor_position(Some(point));
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location,
        PointerAction::Press(bevy::picking::pointer::PointerButton::Primary),
    ));
    app.world_mut()
        .write_message(bevy::input::mouse::MouseButtonInput {
            button: bevy::input::mouse::MouseButton::Left,
            state: ButtonState::Pressed,
            window,
        });
    app.update();
}

fn release_as_click(app: &mut App, window: Entity, camera: Entity, entity: Entity, scale: f32) {
    let _ = camera;
    let location = pointer_location(app, window, entity, scale);
    let point = location.position;
    app.world_mut()
        .get_mut::<Window>(window)
        .expect("primary window")
        .set_cursor_position(Some(point));
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location,
        PointerAction::Release(bevy::picking::pointer::PointerButton::Primary),
    ));
    app.world_mut()
        .write_message(bevy::input::mouse::MouseButtonInput {
            button: bevy::input::mouse::MouseButton::Left,
            state: ButtonState::Released,
            window,
        });
    app.update();
    app.update();
}

fn submitted_commands(app: &App) -> Vec<ClientCommand> {
    app.world()
        .resource::<PendingCommands>()
        .0
        .iter()
        .filter_map(|pending| match pending {
            PendingCommand::Submit { kind, .. } => Some(kind.clone()),
            _ => None,
        })
        .collect()
}

fn submitted_order_envelopes(app: &App) -> Vec<turts_net_protocol::OrderEnvelope> {
    app.world()
        .resource::<PendingCommands>()
        .0
        .iter()
        .filter_map(|pending| match pending {
            PendingCommand::Order { order, .. } => Some(order.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn installed_build_menu_activation_submits_one_exact_produce_and_disabled_activation_submits_none()
{
    let (acquired, replica, producer, recipe) = accepted_fixture();
    let producer_content = acquired
        .content
        .catalog
        .units
        .values()
        .find(|unit| unit.producer.is_some())
        .expect("producer definition")
        .id
        .to_string();
    let order = acquired
        .content
        .catalog
        .orders
        .iter()
        .find(|(_, order)| {
            order.behaviour == turts_mod_api::Behaviour::Produce
                && order.target == turts_mod_api::TargetKind::Content
                && (order.units.is_empty()
                    || order
                        .units
                        .iter()
                        .any(|allowed| allowed == &producer_content))
        })
        .map(|(id, _)| id.to_string())
        .expect("public qualified Produce order");
    let (mut app, window, camera, _) = installed_app(
        acquired,
        replica,
        Source::BuildMenu,
        ActionField::Action,
        &[],
        1.0,
        1280,
        720,
    );
    let action = WidgetAction::Produce {
        producer,
        content_id: recipe.clone(),
    };
    let entity = authored_button(&app, &action);
    press(&mut app, window, camera, entity, 1.0);
    assert!(
        app.world().get::<Pressed>(entity).is_some(),
        "real Button press is held"
    );
    release_as_click(&mut app, window, camera, entity, 1.0);
    assert_eq!(
        submitted_order_envelopes(&app),
        vec![turts_net_protocol::OrderEnvelope {
            order,
            units: vec![UnitId(producer)],
            target: turts_net_protocol::OrderTarget::Content(recipe),
            queue: turts_net_protocol::QueueMode::Replace,
            group: 0,
        }]
    );
    assert!(
        submitted_commands(&app).is_empty(),
        "production does not enqueue legacy client commands"
    );

    let (acquired, mut replica, producer, recipe) = accepted_fixture();
    let capacity = acquired
        .content
        .catalog
        .units
        .values()
        .find_map(|unit| {
            unit.producer
                .as_ref()
                .map(|producer| producer.queue_capacity)
        })
        .expect("producer capacity");
    let ids = (0..capacity)
        .map(|index| FIRST_JOB + u64::from(index))
        .collect::<Vec<_>>();
    set_jobs(&mut replica, producer, &recipe, &ids);
    let (mut disabled_app, window, camera, _) = installed_app(
        acquired,
        replica,
        Source::BuildMenu,
        ActionField::Action,
        &ids,
        1.0,
        1280,
        720,
    );
    let disabled = authored_button(&disabled_app, &action);
    assert_eq!(
        disabled_app.world().get::<WidgetEnabled>(disabled),
        Some(&WidgetEnabled(false))
    );
    press(&mut disabled_app, window, camera, disabled, 1.0);
    release_as_click(&mut disabled_app, window, camera, disabled, 1.0);
    assert!(
        submitted_commands(&disabled_app).is_empty(),
        "disabled real pointer activation submits nothing"
    );
}

#[test]
fn installed_build_menu_activation_skips_foreign_first_produce_order_and_uses_intended_producer() {
    let (mut acquired, replica, producer, recipe) = accepted_fixture();
    let producer_content = acquired
        .content
        .catalog
        .units
        .values()
        .find(|unit| unit.producer.is_some())
        .expect("producer definition")
        .id
        .to_string();
    let other_content = acquired
        .content
        .catalog
        .units
        .keys()
        .map(ToString::to_string)
        .find(|id| id != &producer_content)
        .expect("a distinct unit content id for the restricted order");
    let template = acquired
        .content
        .catalog
        .orders
        .values()
        .find(|order| {
            order.behaviour == turts_mod_api::Behaviour::Produce
                && order.target == turts_mod_api::TargetKind::Content
        })
        .expect("accepted Produce order")
        .clone();
    let stale = acquired
        .content
        .catalog
        .orders
        .iter()
        .filter(|(_, order)| {
            order.behaviour == turts_mod_api::Behaviour::Produce
                && order.target == turts_mod_api::TargetKind::Content
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    for id in stale {
        acquired.content.catalog.orders.remove(&id);
    }

    let restricted_id = turts_mod_api::ContentId::parse("aaa:order_produce_foreign").unwrap();
    let mut restricted = template.clone();
    restricted.id = restricted_id.local.clone();
    restricted.units = vec![other_content];
    acquired
        .content
        .catalog
        .orders
        .insert(restricted_id, restricted);
    let intended_id = turts_mod_api::ContentId::parse("zzz:order_produce_intended").unwrap();
    let mut intended = template;
    intended.id = intended_id.local.clone();
    intended.units = vec![producer_content];
    acquired
        .content
        .catalog
        .orders
        .insert(intended_id, intended);

    let (mut app, window, camera, _) = installed_app(
        acquired,
        replica,
        Source::BuildMenu,
        ActionField::Action,
        &[],
        1.0,
        1280,
        720,
    );
    let action = WidgetAction::Produce {
        producer,
        content_id: recipe.clone(),
    };
    let entity = authored_button(&app, &action);
    press(&mut app, window, camera, entity, 1.0);
    release_as_click(&mut app, window, camera, entity, 1.0);
    assert_eq!(
        submitted_order_envelopes(&app),
        vec![turts_net_protocol::OrderEnvelope {
            order: "zzz:order_produce_intended".into(),
            units: vec![UnitId(producer)],
            target: turts_net_protocol::OrderTarget::Content(recipe),
            queue: turts_net_protocol::QueueMode::Replace,
            group: 0,
        }],
        "the installed widget selects a qualified order allowed for this producer"
    );
}

#[test]
fn installed_cancel_activation_skips_foreign_first_job_order_and_keeps_job_identity() {
    let (mut acquired, replica, producer, _recipe) = accepted_fixture();
    let producer_content = acquired
        .content
        .catalog
        .units
        .values()
        .find(|unit| unit.producer.is_some())
        .expect("producer definition")
        .id
        .to_string();
    let other_content = acquired
        .content
        .catalog
        .units
        .keys()
        .map(ToString::to_string)
        .find(|id| id != &producer_content)
        .expect("distinct unit type for restricted order");
    let template = acquired
        .content
        .catalog
        .orders
        .values()
        .find(|order| {
            order.behaviour == turts_mod_api::Behaviour::CancelProduction
                && order.target == turts_mod_api::TargetKind::Job
        })
        .expect("accepted CancelProduction order")
        .clone();
    let stale = acquired
        .content
        .catalog
        .orders
        .iter()
        .filter(|(_, order)| {
            order.behaviour == turts_mod_api::Behaviour::CancelProduction
                && order.target == turts_mod_api::TargetKind::Job
        })
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    for id in stale {
        acquired.content.catalog.orders.remove(&id);
    }

    let restricted_id = turts_mod_api::ContentId::parse("aaa:order_cancel_foreign").unwrap();
    let mut restricted = template.clone();
    restricted.id = restricted_id.local.clone();
    restricted.units = vec![other_content];
    acquired
        .content
        .catalog
        .orders
        .insert(restricted_id, restricted);
    let intended_id = turts_mod_api::ContentId::parse("zzz:order_cancel_intended").unwrap();
    let mut intended = template;
    intended.id = intended_id.local.clone();
    intended.units = vec![producer_content];
    acquired
        .content
        .catalog
        .orders
        .insert(intended_id, intended);

    let (mut app, window, camera, _) = installed_app(
        acquired,
        replica,
        Source::ProductionQueue,
        ActionField::CancelAction,
        &[FIRST_JOB],
        1.0,
        1280,
        720,
    );
    let action = WidgetAction::CancelProduction {
        producer,
        job: FIRST_JOB,
    };
    let entity = authored_button(&app, &action);
    press(&mut app, window, camera, entity, 1.0);
    release_as_click(&mut app, window, camera, entity, 1.0);
    assert_eq!(
        submitted_order_envelopes(&app),
        vec![turts_net_protocol::OrderEnvelope {
            order: "zzz:order_cancel_intended".into(),
            units: vec![UnitId(producer)],
            target: turts_net_protocol::OrderTarget::Job(JobId(FIRST_JOB)),
            queue: turts_net_protocol::QueueMode::Replace,
            group: 0,
        }],
        "the installed queue button sends the qualified job order allowed for its producer"
    );
}

#[test]
fn held_cancel_button_keeps_identity_through_progress_and_retired_rows_are_pruned() {
    let (acquired, replica, producer, _recipe) = accepted_fixture();
    let (mut app, window, camera, _) = installed_app(
        acquired,
        replica,
        Source::ProductionQueue,
        ActionField::CancelAction,
        &[FIRST_JOB, FIRST_JOB + 1],
        1.0,
        1280,
        720,
    );
    let first_action = WidgetAction::CancelProduction {
        producer,
        job: FIRST_JOB,
    };
    let second_action = WidgetAction::CancelProduction {
        producer,
        job: FIRST_JOB + 1,
    };
    let first = authored_button(&app, &first_action);
    let second = authored_button(&app, &second_action);
    press(&mut app, window, camera, first, 1.0);

    {
        // The job advances: protocol 5 carries the progress as a record.
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        let replica = state.replica.as_mut().unwrap();
        let queue = &replica.economy.queues[0];
        let (producer, job, total) = (
            queue.producer.0,
            queue.jobs[0].id.0,
            queue.jobs[0].duration_ticks,
        );
        replica.store_for_tests().seed_job_progress(
            producer,
            job,
            turts_net_protocol::RateRecord {
                done: 1,
                total,
                per_tick: 0,
                at_tick: 0,
                state: turts_net_protocol::RecordState::Running,
                charged: vec![],
                held: false,
                held_since_tick: 0,
            },
        );
    }
    app.update();
    assert_eq!(
        authored_button(&app, &first_action),
        first,
        "source progress retains the pressed entity"
    );
    assert!(
        app.world().get::<Pressed>(first).is_some(),
        "the original button remains pressed during progress"
    );
    assert_eq!(
        app.world().get::<WidgetButtonAction>(first).unwrap().0,
        first_action
    );
    release_as_click(&mut app, window, camera, first, 1.0);
    assert_eq!(
        submitted_order_envelopes(&app),
        vec![turts_net_protocol::OrderEnvelope {
            order: "base:order_cancel".into(),
            units: vec![UnitId(producer)],
            target: turts_net_protocol::OrderTarget::Job(JobId(FIRST_JOB)),
            queue: turts_net_protocol::QueueMode::Replace,
            group: 0,
        }],
        "release dispatches exactly the displayed job"
    );

    let mut state = app.world_mut().resource_mut::<ReplicaState>();
    state.replica.as_mut().unwrap().economy.queues[0]
        .jobs
        .remove(0);
    app.update();
    assert!(
        app.world().get_entity(first).is_err(),
        "retired job button is despawned"
    );
    assert_eq!(
        authored_button(&app, &second_action),
        second,
        "surviving job control keeps identity"
    );
    assert_eq!(
        app.world()
            .iter_entities()
            .filter(|entity| entity.contains::<Button>())
            .count(),
        1,
        "retired rows do not accumulate"
    );
}

#[test]
fn bounded_queue_scroll_reaches_fifth_cancel_at_dpr_two_and_keeps_world_input_clear() {
    let (acquired, replica, producer, _recipe) = accepted_fixture();
    let job_ids = (FIRST_JOB..FIRST_JOB + 5).collect::<Vec<_>>();
    let (mut app, window, camera, _) = installed_app(
        acquired,
        replica,
        Source::ProductionQueue,
        ActionField::CancelAction,
        &job_ids,
        2.0,
        1280,
        720,
    );
    let list = app
        .world()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<AuthoredWidgetId>()
                .is_some_and(|id| id.0 == "test.list")
        })
        .expect("authored bounded list")
        .id();
    let list_node = app
        .world()
        .get::<ComputedNode>(list)
        .expect("real list layout");
    assert!(
        list_node.size().x > 0.0 && list_node.size().y > 0.0,
        "list has computed size"
    );
    assert!(
        app.world().get::<ScrollArea>(list).is_some(),
        "bounded List installs Bevy ScrollArea"
    );
    let fifth = WidgetAction::CancelProduction {
        producer,
        job: FIRST_JOB + 4,
    };
    let fifth_button = authored_button(&app, &fifth);
    assert!(
        app.world().get::<ComputedNode>(fifth_button).is_some(),
        "fifth row participates in real layout"
    );
    assert!(
        app.world().get::<CalculatedClip>(fifth_button).is_some(),
        "fifth row inherits actual list clipping"
    );

    let location = pointer_location(&app, window, list, 2.0);
    let camera_height = app
        .world()
        .resource::<CoreRuntime>()
        .core()
        .expect("joined accepted snapshot initializes the interaction core")
        .camera_state()
        .height;
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location.clone(),
        PointerAction::Move { delta: Vec2::ZERO },
    ));
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        location.clone(),
        PointerAction::Scroll {
            unit: bevy::input::mouse::MouseScrollUnit::Line,
            x: 0.0,
            y: -8.0,
            phase: bevy::input::touch::TouchPhase::Moved,
        },
    ));
    app.world_mut()
        .resource_mut::<Messages<WindowEvent>>()
        .write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
            window,
            position: location.position,
            delta: None,
        }));
    app.world_mut()
        .resource_mut::<Messages<WindowEvent>>()
        .write(WindowEvent::MouseWheel(bevy::input::mouse::MouseWheel {
            unit: bevy::input::mouse::MouseScrollUnit::Line,
            x: 0.0,
            y: -8.0,
            window,
            phase: bevy::input::touch::TouchPhase::Moved,
        }));
    app.update();
    assert!(
        app.world()
            .get::<ScrollPosition>(list)
            .is_some_and(|position| position.y > 0.0),
        "real scroll observer moves the authored list"
    );
    assert_eq!(
        app.world()
            .resource::<CoreRuntime>()
            .core()
            .unwrap()
            .camera_state()
            .height,
        camera_height,
        "the same real wheel event is captured by the scrollable widget"
    );

    let outside = Vec2::new(1100.0, 650.0);
    app.world_mut()
        .resource_mut::<Messages<WindowEvent>>()
        .write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
            window,
            position: outside,
            delta: None,
        }));
    app.world_mut()
        .resource_mut::<Messages<WindowEvent>>()
        .write(WindowEvent::MouseWheel(bevy::input::mouse::MouseWheel {
            unit: bevy::input::mouse::MouseScrollUnit::Line,
            x: 0.0,
            y: 1.0,
            window,
            phase: bevy::input::touch::TouchPhase::Moved,
        }));
    app.update();
    assert_ne!(
        app.world()
            .resource::<CoreRuntime>()
            .core()
            .unwrap()
            .camera_state()
            .height,
        camera_height,
        "outside UI, the actual adapter forwards wheel zoom to the camera"
    );

    let node = app.world().get::<ComputedNode>(fifth_button).unwrap();
    let clip = app.world().get::<CalculatedClip>(fifth_button).unwrap();
    let transform = app.world().get::<UiGlobalTransform>(fifth_button).unwrap();
    let center = transform.to_scale_angle_translation().2;
    let CalculatedClip::Rects(clips) = clip else {
        panic!("fifth action is not fully clipped after scroll")
    };
    assert!(
        clips.iter().any(|clip| {
            let local = clip.world_to_clip_local.transform_point2(center);
            clip.rect.contains(local)
        }),
        "fifth button center is within a computed clip after DPR2 scrolling"
    );
    assert!(
        node.size().min_element() > 0.0,
        "fifth action has an actual computed hit area"
    );
    press(&mut app, window, camera, fifth_button, 2.0);
    release_as_click(&mut app, window, camera, fifth_button, 2.0);
    assert_eq!(
        submitted_order_envelopes(&app),
        vec![turts_net_protocol::OrderEnvelope {
            order: "base:order_cancel".into(),
            units: vec![UnitId(producer)],
            target: turts_net_protocol::OrderTarget::Job(JobId(FIRST_JOB + 4)),
            queue: turts_net_protocol::QueueMode::Replace,
            group: 0,
        }]
    );
    assert!(
        submitted_commands(&app).is_empty(),
        "scroll and cancel do not enqueue legacy client commands"
    );
}

#[test]
fn actual_window_adapter_suppresses_widget_targeting_then_routes_outside_move_order() {
    let (acquired, mut replica, producer, _) = accepted_fixture();
    let (harvester_content, has_movement) = acquired
        .content
        .catalog
        .units
        .iter()
        .find_map(|(id, unit)| unit.movement.is_some().then(|| (id.to_string(), true)))
        .expect("accepted catalog includes a mobile unit");
    let mobile = 8;
    replica.units.insert(
        mobile,
        UnitReplica {
            content_id: harvester_content,
            owner_slot: 0,
            facing: None,
            samples: VecDeque::from([(1, turts_net_protocol::Pos { x: 12.0, y: 12.0 })]),
        },
    );
    let (mut app, window, _, _) = installed_app(
        acquired,
        replica,
        Source::ProductionQueue,
        ActionField::CancelAction,
        &[FIRST_JOB],
        2.0,
        1280,
        720,
    );
    assert!(has_movement);
    app.world_mut().insert_resource(Selection(Some(mobile)));
    app.world_mut()
        .insert_resource(turts_client::debug_feed::SelectionFeed(vec![mobile]));
    app.update();
    assert_eq!(
        app.world()
            .resource::<CoreRuntime>()
            .core()
            .unwrap()
            .selected_units(),
        &[UnitId(mobile)],
        "the actual adapter owns a selected mobile unit from an accepted snapshot"
    );

    let send_key = |app: &mut App| {
        app.world_mut()
            .resource_mut::<Messages<WindowEvent>>()
            .write(WindowEvent::KeyboardInput(KeyboardInput {
                key_code: KeyCode::KeyM,
                logical_key: Key::Character("m".into()),
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            }));
        app.update();
    };
    let click = |app: &mut App, point: Vec2| {
        {
            let mut events = app.world_mut().resource_mut::<Messages<WindowEvent>>();
            events.write(WindowEvent::CursorMoved(bevy::window::CursorMoved {
                window,
                position: point,
                delta: None,
            }));
            events.write(WindowEvent::MouseButtonInput(MouseButtonInput {
                button: MouseButton::Left,
                state: ButtonState::Pressed,
                window,
            }));
            events.write(WindowEvent::MouseButtonInput(MouseButtonInput {
                button: MouseButton::Left,
                state: ButtonState::Released,
                window,
            }));
        }
        app.update();
    };

    send_key(&mut app);
    assert_eq!(
        app.world().resource::<CoreRuntime>().pending_order_id(),
        Some("base:order_move")
    );
    click(&mut app, Vec2::new(25.0, 25.0));
    assert!(
        submitted_commands(&app).is_empty(),
        "a real DPR2 click over the authored scroll area cannot target the world"
    );
    assert_eq!(
        app.world().resource::<CoreRuntime>().pending_order_id(),
        Some("base:order_move"),
        "captured widget input leaves the pending target mode intact"
    );

    let viewport = {
        let window = app.world().get::<Window>(window).unwrap();
        InteractionViewport {
            size: Vec2::new(window.width(), window.height()),
            scale_factor: window.scale_factor(),
        }
    };
    let point = app
        .world()
        .resource::<CoreRuntime>()
        .core()
        .unwrap()
        .project_world_to_screen(Vec2::new(40.0, 40.0), &viewport);
    click(&mut app, point);
    let order =
        app.world()
            .resource::<PendingCommands>()
            .0
            .iter()
            .find_map(|pending| match pending {
                PendingCommand::Order { order, .. } => Some(order.clone()),
                _ => None,
            });
    let order = order.expect("outside actual window input submits one world order");
    assert_eq!(order.order, "base:order_move");
    assert_eq!(order.units, vec![UnitId(mobile)]);
    assert!(matches!(
        order.target,
        turts_net_protocol::OrderTarget::Point(_)
    ));
    assert_eq!(
        app.world().resource::<CoreRuntime>().pending_order_id(),
        None,
        "the accepted target consumes the actual Core order mode"
    );
    let _ = producer;
}

#[test]
fn first_frame_widget_snapshot_tracks_computed_rows_clip_and_active_camera() {
    let (acquired, replica, _, _) = accepted_fixture();
    let (mut app, _window, camera, producer) = installed_app_with_updates(
        acquired,
        replica,
        Source::BuildMenu,
        ActionField::Action,
        &[],
        1.0,
        1280,
        720,
        0,
    );
    app.update();
    let feed = app
        .world()
        .resource::<turts_client::debug_feed::UiSnapshotFeed>()
        .0
        .clone();
    let direct = snapshot::snapshot(app.world());
    assert_eq!(feed, direct, "Last publishes the completed same-frame tree");
    let row = direct
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["widget_action"]["kind"] == "produce")
        .expect("the accepted authored BuildMenu renders its action row");
    assert_eq!(row["visible"], true);
    assert!(row["rect"][2].as_f64().unwrap() > 0.0);
    assert!(row["rect"][3].as_f64().unwrap() > 0.0);

    let entity = app
        .world()
        .iter_entities()
        .find(|entity| {
            entity
                .get::<WidgetButtonAction>()
                .is_some_and(|action| matches!(action.0, WidgetAction::Produce { producer: id, .. } if id == producer))
        })
        .unwrap()
        .id();
    app.world_mut()
        .entity_mut(entity)
        .insert(CalculatedClip::FullyClipped);
    assert_eq!(
        snapshot::snapshot(app.world())
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == app.world().get::<AuthoredWidgetId>(entity).unwrap().0)
            .unwrap()["visible"],
        false,
        "a fully clipped real authored row is not published as visible"
    );
    app.world_mut()
        .entity_mut(entity)
        .remove::<CalculatedClip>();
    app.world_mut().get_mut::<Camera>(camera).unwrap().is_active = false;
    assert_eq!(
        snapshot::snapshot(app.world())
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == app.world().get::<AuthoredWidgetId>(entity).unwrap().0)
            .unwrap()["visible"],
        false,
        "an inactive target camera cannot publish a visible UI row"
    );
}

#[test]
fn nested_fit_widgets_converge_after_dynamic_text_changes() {
    let (acquired, replica, _, _) = accepted_fixture();
    let resource = acquired
        .content
        .catalog
        .resources
        .keys()
        .next()
        .expect("accepted fixture has a resource")
        .to_string();
    let (mut app, _, _, _) = installed_app(
        acquired,
        replica,
        Source::Balances,
        ActionField::Action,
        &[],
        2.0,
        1600,
        1000,
    );
    app.world_mut().insert_resource(bevy::ui::UiScale(1.25));
    let text = Widget {
        id: "fit.text".into(),
        at: Some(turts_mod_api::At {
            anchor: Anchor::BottomRight,
            offset: (-4.0, -3.0),
            pivot: Some(Anchor::BottomRight),
        }),
        size: Size {
            w: Dimension::Fit,
            h: Dimension::Fit,
        },
        layer: 0,
        visible: vec![],
        tooltip: None,
        widget: Primitive::Text {
            text: TextValue::Bind(turts_mod_api::Value::Balance(resource.clone())),
            font: None,
            size: 18,
            colour: "#ffffff".into(),
            align: turts_mod_api::TextAlign::Left,
            wrap: false,
        },
    };
    let inner = Widget {
        id: "fit.inner".into(),
        at: Some(turts_mod_api::At {
            anchor: Anchor::BottomRight,
            offset: (-6.0, -5.0),
            pivot: Some(Anchor::BottomRight),
        }),
        size: Size {
            w: Dimension::Fit,
            h: Dimension::Fit,
        },
        layer: 0,
        visible: vec![],
        tooltip: None,
        widget: Primitive::Box {
            fill: None,
            border: None,
            children: vec![text],
        },
    };
    let outer = Widget {
        id: "fit.outer".into(),
        at: Some(turts_mod_api::At {
            anchor: Anchor::TopLeft,
            offset: (30.0, 30.0),
            pivot: None,
        }),
        size: Size {
            w: Dimension::Fit,
            h: Dimension::Fit,
        },
        layer: 0,
        visible: vec![],
        tooltip: None,
        widget: Primitive::Box {
            fill: None,
            border: None,
            children: vec![inner],
        },
    };
    app.world_mut()
        .resource_mut::<AcquiredPackages>()
        .0
        .presentation
        .interface
        .as_mut()
        .unwrap()
        .root = vec![outer];

    let find = |app: &App, id: &str| {
        app.world()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<AuthoredWidgetId>()
                    .is_some_and(|authored| authored.0 == id)
            })
            .unwrap_or_else(|| panic!("missing widget {id}"))
            .id()
    };
    let mut prior_width = 0.0;
    let mut retained_text = None;
    for amount in [1_u64, 12_345, 7] {
        {
            let mut state = app.world_mut().resource_mut::<ReplicaState>();
            state
                .replica
                .as_mut()
                .unwrap()
                .economy
                .balances
                .iter_mut()
                .find(|balance| balance.resource == resource)
                .unwrap()
                .on_hand = amount;
        }

        let mut settled_widths = Vec::new();
        for step in 0..20 {
            app.update();
            if step < 15 {
                continue;
            }
            let outer_id = find(&app, "fit.outer");
            let inner_id = find(&app, "fit.inner");
            let text_id = find(&app, "fit.text");
            if let Some(existing) = retained_text {
                assert_eq!(
                    text_id, existing,
                    "dynamic value refresh retains the text entity"
                );
            }
            retained_text = Some(text_id);
            let outer_size = app.world().get::<ComputedNode>(outer_id).unwrap().size();
            let inner_size = app.world().get::<ComputedNode>(inner_id).unwrap().size();
            let text_size = app.world().get::<ComputedNode>(text_id).unwrap().size();
            assert!(
                text_size.min_element() > 0.0,
                "font measurement must be positive"
            );
            assert!(
                (inner_size - text_size - Vec2::new(4.0, 3.0) * 2.5)
                    .abs()
                    .max_element()
                    < 1.0,
                "inner Fit follows actual measured text and bottom-right inset: {inner_size:?} vs {text_size:?}"
            );
            assert!(
                (outer_size - inner_size - Vec2::new(6.0, 5.0) * 2.5)
                    .abs()
                    .max_element()
                    < 1.0,
                "outer Fit follows actual nested measurement: {outer_size:?} vs {inner_size:?}"
            );
            assert_eq!(
                app.world().get::<Text>(text_id).unwrap().0,
                amount.to_string()
            );
            settled_widths.push(outer_size.x);
        }
        assert!(
            settled_widths
                .iter()
                .all(|width| (width - settled_widths[0]).abs() < 0.1),
            "nested Fit must settle without oscillation: {settled_widths:?}"
        );
        if amount == 12_345 {
            assert!(
                settled_widths[0] > prior_width + 10.0,
                "more digits increase measured natural width"
            );
        }
        if amount == 7 {
            assert!(
                settled_widths[0] < prior_width - 10.0,
                "shorter text shrinks the retained Fit tree"
            );
        }
        prior_width = settled_widths[0];
    }
}

#[test]
fn fit_queue_reflows_rows_and_retires_extent_when_empty() {
    let (acquired, replica, _, recipe) = accepted_fixture();
    let (mut app, window, camera, producer) = installed_app(
        acquired,
        replica,
        Source::ProductionQueue,
        ActionField::CancelAction,
        &[201, 202, 203],
        2.0,
        1600,
        1000,
    );
    let mut list = authored_list(
        "fit.queue",
        Source::ProductionQueue,
        ActionField::CancelAction,
        16,
    );
    list.size = Size {
        w: Dimension::Fit,
        h: Dimension::Fit,
    };
    app.world_mut()
        .resource_mut::<AcquiredPackages>()
        .0
        .presentation
        .interface
        .as_mut()
        .unwrap()
        .root = vec![list];
    for _ in 0..12 {
        app.update();
    }

    let widget = |app: &App, id: &str| {
        app.world()
            .iter_entities()
            .find(|entity| {
                entity
                    .get::<AuthoredWidgetId>()
                    .is_some_and(|value| value.0 == id)
            })
            .unwrap_or_else(|| panic!("missing installed widget {id}"))
            .id()
    };
    let top = |app: &App, entity: Entity| {
        app.world()
            .get::<UiGlobalTransform>(entity)
            .unwrap()
            .to_scale_angle_translation()
            .2
            .y
            - app.world().get::<ComputedNode>(entity).unwrap().size().y / 2.0
    };
    let list_entity = widget(&app, "fit.queue");
    let row_entities = [201, 202, 203]
        .map(|job| authored_button(&app, &WidgetAction::CancelProduction { producer, job }));
    let initial = app.world().get::<ComputedNode>(list_entity).unwrap().size();
    assert!(
        (initial - Vec2::new(460.0, 152.0)).abs().max_element() < 1.0,
        "three measured 230x24 rows with 2px gaps at DPR2 should fit their authored content: {initial:?}"
    );
    let row_tops = row_entities.map(|row| top(&app, row));
    assert!(
        (row_tops[1] - row_tops[0] - 52.0).abs() < 1.0
            && (row_tops[2] - row_tops[1] - 52.0).abs() < 1.0,
        "reconciled rows must retain their new authored flow positions: {row_tops:?}"
    );

    set_jobs(
        app.world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .unwrap(),
        producer,
        &recipe,
        &[],
    );
    for _ in 0..12 {
        app.update();
    }
    assert_eq!(widget(&app, "fit.queue"), list_entity);
    let empty = app.world().get::<ComputedNode>(list_entity).unwrap().size();
    assert!(
        empty.max_element() < 1.0,
        "an empty authored Fit List must return to empty natural bounds: {empty:?}"
    );
    let snapshot_value = snapshot::snapshot(app.world());
    let row = snapshot_value
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["id"] == "fit.queue")
        .expect("the authored list remains in the semantic snapshot");
    assert_eq!(
        row["visible"], false,
        "a zero-area empty Fit List must not retain a stale pointer capture rectangle"
    );

    set_jobs(
        app.world_mut()
            .resource_mut::<ReplicaState>()
            .replica
            .as_mut()
            .unwrap(),
        producer,
        &recipe,
        &[301, 302],
    );
    for _ in 0..12 {
        app.update();
    }
    let restored = app.world().get::<ComputedNode>(list_entity).unwrap().size();
    assert!(
        (restored - Vec2::new(460.0, 100.0)).abs().max_element() < 1.0,
        "a repopulated two-row Fit List returns to measured content bounds: {restored:?}"
    );
    let current = authored_button(&app, &WidgetAction::CancelProduction { producer, job: 302 });
    press(&mut app, window, camera, current, 2.0);
    release_as_click(&mut app, window, camera, current, 2.0);
    let orders = submitted_order_envelopes(&app);
    assert!(
        matches!(orders.as_slice(), [order]
            if order.units == [UnitId(producer)] && order.target == turts_net_protocol::OrderTarget::Job(JobId(302))),
        "the repopulated row retains its exact current producer/job action: {orders:?}"
    );
}
