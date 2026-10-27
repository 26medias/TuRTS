use std::{collections::VecDeque, path::PathBuf};

use bevy::{
    camera::visibility::InheritedVisibility,
    camera::{CameraPlugin, ComputedCameraValues, RenderTarget, RenderTargetInfo, Viewport},
    image::{ImagePlugin, TextureAtlasPlugin},
    input::{ButtonState, InputPlugin},
    input_focus::{InputDispatchPlugin, InputFocusPlugin},
    mesh::MeshPlugin,
    picking::{
        InteractionPlugin, PickingPlugin,
        pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput},
    },
    prelude::*,
    text::TextPlugin,
    ui::{CalculatedClip, ComputedNode, Pressed, UiGlobalTransform, UiPlugin},
    ui_widgets::{Button, ButtonPlugin, ScrollAreaPlugin},
    window::{PrimaryWindow, WindowPlugin, WindowRef, WindowResolution},
};
use turts_client::{
    acquire::Acquired,
    content::ClientContent,
    economy_ui::EconomyUiPlugin,
    economy_ui::{
        RenderedUiNode, Selection, UiAction, UiActionTarget, UiControlEnabled, UiSnapshotText,
        panel_model, rendered_ui_snapshot,
    },
    ingest::{CommandResult, CommandResults, ReplicaState, WelcomeInfo},
    join::Phase,
    present::update_presented,
    replica::{Replica, UnitReplica},
};
use turts_mod_api::{ContentId, Cost};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{
    BalanceState, GatherStage, GatherWaitReason, InventoryState, JobId, JobStage, NodeId,
    NodeState, OrderState, Pos, QueueState, UnitId,
};

fn base_content() -> ClientContent {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
    let archive =
        PackageArchive::parse(&pack_dir(&base).expect("pack base")).expect("parse base archive");
    ClientContent::from_archives(&[archive]).expect("resolve base definitions")
}

fn producer_fixture() -> (ClientContent, Replica, u32, ContentId, ContentId) {
    let content = base_content();
    let (producer_id, producer) = content
        .catalog
        .units
        .iter()
        .find(|(_, unit)| unit.producer.is_some())
        .expect("base package has a producer");
    let producer_id = producer_id.clone();
    let recipe_id = producer.producer.as_ref().unwrap().recipes[0].clone();
    let id = 7;
    let mut replica = Replica::new(20);
    replica.units.insert(
        id,
        UnitReplica {
            content_id: producer_id.to_string(),
            owner_slot: 0,
            samples: VecDeque::from([(1, Pos { x: 8.0, y: 8.0 })]),
        },
    );
    let balance = content
        .catalog
        .resources
        .iter()
        .find(|(_, resource)| resource.kind == turts_mod_api::ResourceKind::Balance)
        .map(|(id, _)| id.to_string())
        .expect("base has a balance resource");
    replica.economy.balances.push(BalanceState {
        resource: balance,
        on_hand: 500,
        reserved: 25,
    });
    let node_id = content
        .catalog
        .nodes
        .keys()
        .next()
        .expect("base has node content")
        .to_string();
    replica.nodes.insert(
        NodeId(4),
        NodeState {
            id: NodeId(4),
            content_id: node_id,
            pos: Pos { x: 24.0, y: 8.0 },
            remaining: 0,
        },
    );
    let production = &content.catalog.production[&recipe_id];
    let costs: Vec<Cost> = production
        .costs
        .iter()
        .map(|cost| Cost {
            resource: cost.resource.to_string(),
            amount: cost.amount,
            source: cost.source,
        })
        .collect();
    replica.economy.queues.push(QueueState {
        producer: UnitId(id),
        jobs: vec![turts_net_protocol::JobState {
            id: JobId(91),
            recipe: recipe_id.to_string(),
            stage: JobStage::Queued,
            elapsed_ticks: 2,
            duration_ticks: 100,
            charged: costs,
            reserved: Vec::new(),
        }],
    });
    (content, replica, id, producer_id, recipe_id)
}

#[test]
fn names_costs_sources_and_zero_nodes_follow_downloaded_data() {
    let (mut content, replica, id, producer_id, recipe_id) = producer_fixture();
    content.catalog.units.get_mut(&producer_id).unwrap().name = "Timber Yard".into();
    let recipe_name = content.catalog.production[&recipe_id].name.clone();
    let model = panel_model(
        &replica,
        &content,
        &Selection(Some(id)),
        &CommandResults::default(),
    );
    let text = model.lines.join("\n");
    assert!(text.contains("Timber Yard"));
    assert!(text.contains("Credits: 500 on hand · 475 available · 25 reserved"));
    assert!(
        text.contains("0 remaining"),
        "depleted public nodes stay visible: {text}"
    );
    assert!(text.contains("Job 91"));
    assert!(text.contains(&recipe_name));
    assert!(
        model
            .lines
            .iter()
            .any(|line| line.contains("from slot balance") || line.contains("from this inventory"))
    );
    assert!(model.lines.iter().any(|line| line.starts_with("cost: ")));
    assert!(
        model
            .controls
            .iter()
            .any(|control| control.label == format!("Produce {recipe_name}"))
    );
}

#[test]
fn physical_or_empty_catalog_has_no_balance_section() {
    let (mut content, replica, id, _, _) = producer_fixture();
    content
        .catalog
        .resources
        .retain(|_, resource| resource.kind != turts_mod_api::ResourceKind::Balance);
    let model = panel_model(
        &replica,
        &content,
        &Selection(Some(id)),
        &CommandResults::default(),
    );
    assert!(!model.lines.iter().any(|line| line == "Balances"));
    assert!(!model.lines.iter().any(|line| line.starts_with("Credits:")));
}

#[test]
fn controls_bind_one_recipe_and_the_exact_displayed_job_id() {
    let (content, mut replica, id, _, recipe_id) = producer_fixture();
    let mut results = CommandResults::default();
    results.0.push(CommandResult::Applied {
        seq: 12,
        tick: 44,
        job_id: Some(JobId(91)),
    });
    let model = panel_model(&replica, &content, &Selection(Some(id)), &results);
    let produce = model
        .controls
        .iter()
        .find(|control| matches!(control.action, UiAction::Produce { .. }))
        .unwrap();
    assert_eq!(
        produce.action,
        UiAction::Produce {
            producer: id,
            recipe: recipe_id.to_string()
        }
    );
    let cancel = model
        .controls
        .iter()
        .find(|control| matches!(control.action, UiAction::Cancel { .. }))
        .unwrap();
    assert_eq!(
        cancel.action,
        UiAction::Cancel {
            producer: id,
            job: JobId(91)
        }
    );
    assert!(
        model
            .lines
            .iter()
            .any(|line| line.contains("applied · seq 12 · tick 44 · job 91"))
    );

    let duplicate = replica.economy.queues[0].jobs[0].clone();
    replica.economy.queues[0].jobs.push(duplicate);
    let mut content = content;
    let (_, producer) = content
        .catalog
        .units
        .iter_mut()
        .find(|(_, unit)| unit.producer.is_some())
        .unwrap();
    producer.producer.as_mut().unwrap().queue_capacity = 1;
    let full = panel_model(&replica, &content, &Selection(Some(id)), &results);
    assert!(
        full.controls
            .iter()
            .filter(|c| matches!(c.action, UiAction::Produce { .. }))
            .all(|c| !c.enabled)
    );
}

#[test]
fn rejected_result_uses_engine_status_key() {
    let (content, replica, id, _, _) = producer_fixture();
    let mut results = CommandResults::default();
    results.0.push(CommandResult::Rejected {
        seq: 3,
        tick: 17,
        reason: turts_net_protocol::CommandRejectReason::QueueFull,
    });
    let model = panel_model(&replica, &content, &Selection(Some(id)), &results);
    assert!(
        model
            .lines
            .iter()
            .any(|line| line.contains("rejected · queue_full · seq 3 · tick 17"))
    );
}

#[test]
fn snapshot_rejects_text_without_camera_and_real_button_owner() {
    let mut world = World::new();
    let action = UiAction::Cancel {
        producer: 8,
        job: JobId(91),
    };
    world.spawn((
        Text::new("Cancel job 91"),
        UiSnapshotText,
        ComputedNode {
            size: Vec2::new(120.0, 20.0),
            ..default()
        },
        UiGlobalTransform::from_translation(Vec2::new(240.0, 80.0)),
        InheritedVisibility::VISIBLE,
        UiActionTarget(action.clone()),
        UiControlEnabled(true),
    ));
    let rendered: Vec<RenderedUiNode> = rendered_ui_snapshot(&world);
    assert_eq!(rendered.len(), 1);
    assert_eq!(rendered[0].text, "Cancel job 91");
    assert_eq!(rendered[0].rect, [180.0, 70.0, 120.0, 20.0]);
    assert!(!rendered[0].visible && !rendered[0].enabled);
    assert_eq!(rendered[0].action, Some(action));
}

#[test]
fn disabled_producer_button_ignores_real_pointer_activation() {
    let (mut app, window, _, producer) = cpu_ui_app(1.0, 1);
    {
        let mut acquired = app
            .world_mut()
            .resource_mut::<turts_client::join::AcquiredPackages>();
        acquired
            .0
            .content
            .catalog
            .units
            .values_mut()
            .find_map(|unit| unit.producer.as_mut())
            .unwrap()
            .queue_capacity = 1;
    }
    app.update();
    let action = UiAction::Produce {
        producer,
        recipe: app
            .world()
            .iter_entities()
            .find_map(|entity| {
                entity
                    .get::<UiActionTarget>()
                    .and_then(|target| match &target.0 {
                        UiAction::Produce { recipe, .. } => Some(recipe.clone()),
                        _ => None,
                    })
            })
            .unwrap(),
    };
    let (button, point) = widget_center(&app, &action, 1.0);
    assert!(
        app.world()
            .entity(button)
            .contains::<bevy::ui::InteractionDisabled>()
    );
    assert!(
        !rendered_ui_snapshot(app.world())
            .iter()
            .find(|row| row.action.as_ref() == Some(&action))
            .unwrap()
            .enabled
    );

    pointer_event(
        &mut app,
        window,
        point,
        PointerAction::Move { delta: Vec2::ZERO },
    );
    app.update();
    pointer_event(
        &mut app,
        window,
        point,
        PointerAction::Press(PointerButton::Primary),
    );
    app.update();
    pointer_event(
        &mut app,
        window,
        point,
        PointerAction::Release(PointerButton::Primary),
    );
    app.update();
    app.update();
    assert!(
        app.world()
            .resource::<turts_client::commands::PendingCommands>()
            .0
            .is_empty()
    );
}

#[test]
fn dpi_two_panel_border_blocks_world_order_and_outside_click_still_orders() {
    use turts_client::{
        commands::PendingCommands,
        economy_ui::{cursor_in_ui_viewport, hit_test_node},
        input::ground_point,
    };
    let scale = 2.0;
    let (mut app, window, camera_id, producer) = cpu_ui_app(scale, 1);
    let (mobile_id, mobile_def) = app
        .world()
        .resource::<turts_client::join::AcquiredPackages>()
        .0
        .content
        .catalog
        .units
        .iter()
        .find(|(_, definition)| definition.movement.is_some())
        .map(|(id, def)| (id.to_string(), def.clone()))
        .unwrap();
    let mobile = 55;
    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        state.replica.as_mut().unwrap().units.insert(
            mobile,
            UnitReplica {
                content_id: mobile_id,
                owner_slot: 0,
                samples: VecDeque::from([(1, Pos { x: 44.0, y: 44.0 })]),
            },
        );
    }
    app.world_mut().resource_mut::<Selection>().0 = Some(mobile);
    app.update();
    assert!(mobile_def.movement.is_some());
    app.world_mut()
        .get_mut::<Camera>(camera_id)
        .unwrap()
        .viewport = Some(Viewport {
        physical_position: UVec2::new(100, 80),
        physical_size: UVec2::new(1180, 640),
        ..default()
    });
    app.update();

    let camera = app.world().get::<Camera>(camera_id).unwrap().clone();
    let root = app
        .world()
        .iter_entities()
        .find(|entity| entity.contains::<turts_client::economy_ui::UiPanelRoot>())
        .unwrap();
    let root_node = *root.get::<ComputedNode>().unwrap();
    let root_transform = *root.get::<UiGlobalTransform>().unwrap();

    // Logical cursor (63, 70) maps to physical (126, 140), then subtracts
    // the viewport origin (100, 80) to hit the panel's local physical border.
    let border = Vec2::new(63.0, 70.0);
    let ui_border = cursor_in_ui_viewport(border, &camera).unwrap();
    assert!(root_node.contains_point(root_transform, ui_border));
    let camera_transform = *app.world().get::<GlobalTransform>(camera_id).unwrap();
    let ray = camera.viewport_to_world(&camera_transform, border).unwrap();
    let point = ground_point(ray).unwrap();
    let state = app.world().resource::<ReplicaState>();
    assert!(
        hit_test_node(
            state.replica.as_ref().unwrap(),
            &app.world()
                .resource::<turts_client::join::AcquiredPackages>()
                .0
                .content,
            point
        )
        .is_none()
    );

    let click = |app: &mut App, position: Vec2| {
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .set_cursor_position(Some(position));
        app.world_mut()
            .write_message(bevy::input::mouse::MouseButtonInput {
                button: MouseButton::Right,
                state: ButtonState::Pressed,
                window,
            });
        app.update();
        app.world_mut()
            .write_message(bevy::input::mouse::MouseButtonInput {
                button: MouseButton::Right,
                state: ButtonState::Released,
                window,
            });
        app.update();
    };
    click(&mut app, border);
    assert!(
        app.world().resource::<PendingCommands>().0.is_empty(),
        "panel border suppresses the selected mobile's world order"
    );
    assert_eq!(app.world().resource::<Selection>().0, Some(mobile));

    // Screen x=455 is just beyond the panel after scale and viewport offset:
    // (455*2 - 100)=810, beyond its local 804px edge. The old unscaled
    // comparison incorrectly treated logical x=455 as inside.
    let outside = Vec2::new(455.0, 70.0);
    assert!(
        cursor_in_ui_viewport(outside, &camera)
            .is_some_and(|cursor| !root_node.contains_point(root_transform, cursor))
    );
    let ray = camera
        .viewport_to_world(&camera_transform, outside)
        .unwrap();
    let point = ground_point(ray).unwrap();
    let state = app.world().resource::<ReplicaState>();
    assert!(
        hit_test_node(
            state.replica.as_ref().unwrap(),
            &app.world()
                .resource::<turts_client::join::AcquiredPackages>()
                .0
                .content,
            point
        )
        .is_none()
    );
    click(&mut app, outside);
    assert_eq!(
        app.world().resource::<PendingCommands>().0.len(),
        1,
        "an outside click reaches the real world-order path"
    );
    assert!(
        matches!(app.world().resource::<PendingCommands>().0.front(), Some(turts_net_protocol::PlayerCommandKind::MoveTo { unit_ids, .. }) if unit_ids == &vec![UnitId(mobile)])
    );
    assert_ne!(producer, mobile);
}

#[test]
fn real_snapshot_tracks_clipping_and_inactive_camera() {
    let (mut app, _, camera, producer) = cpu_ui_app(1.0, 1);
    let action = UiAction::Produce {
        producer,
        recipe: app
            .world()
            .iter_entities()
            .find_map(|entity| {
                entity
                    .get::<UiActionTarget>()
                    .and_then(|target| match &target.0 {
                        UiAction::Produce { recipe, .. } => Some(recipe.clone()),
                        _ => None,
                    })
            })
            .expect("real producer action"),
    };
    let row = rendered_ui_snapshot(app.world())
        .into_iter()
        .find(|row| row.action.as_ref() == Some(&action))
        .unwrap();
    assert!(
        row.visible && row.enabled,
        "real post-layout button should be visible/enabled: {row:?}"
    );

    let label = app
        .world()
        .iter_entities()
        .find(|entity| {
            entity.get::<Text>().is_some()
                && entity
                    .get::<UiActionTarget>()
                    .is_some_and(|target| target.0 == action)
        })
        .unwrap()
        .id();
    let original_clip = app.world().get::<CalculatedClip>(label).unwrap().clone();
    app.world_mut()
        .entity_mut(label)
        .insert(CalculatedClip::FullyClipped);
    assert!(
        !rendered_ui_snapshot(app.world())
            .iter()
            .find(|row| row.action.as_ref() == Some(&action))
            .unwrap()
            .visible
    );
    app.world_mut().entity_mut(label).insert(original_clip);

    app.world_mut().get_mut::<Camera>(camera).unwrap().is_active = false;
    assert!(
        !rendered_ui_snapshot(app.world())
            .iter()
            .find(|row| row.action.as_ref() == Some(&action))
            .unwrap()
            .visible
    );
}

#[test]
fn owned_move_expansion_uses_only_mobile_records_after_ready_context() {
    use turts_client::{
        commands::owned_mobile_ids,
        ingest::{ReplicaState, WelcomeInfo},
    };
    let content = base_content();
    let static_id = content
        .catalog
        .units
        .iter()
        .find(|(_, unit)| unit.movement.is_none())
        .unwrap()
        .0
        .to_string();
    let mobile_id = content
        .catalog
        .units
        .iter()
        .find(|(_, unit)| unit.movement.is_some())
        .unwrap()
        .0
        .to_string();
    let mut state = ReplicaState::default();
    assert!(owned_mobile_ids(&state).is_empty());
    let mut replica = Replica::new(20);
    replica.units.insert(
        2,
        UnitReplica {
            content_id: static_id,
            owner_slot: 0,
            samples: VecDeque::new(),
        },
    );
    replica.units.insert(
        3,
        UnitReplica {
            content_id: mobile_id.clone(),
            owner_slot: 1,
            samples: VecDeque::new(),
        },
    );
    replica.units.insert(
        1,
        UnitReplica {
            content_id: mobile_id,
            owner_slot: 0,
            samples: VecDeque::new(),
        },
    );
    state.welcome = Some(WelcomeInfo {
        slot: 0,
        tick: 1,
        tick_hz: 20,
    });
    state.content = Some(content);
    state.replica = Some(replica);
    assert_eq!(owned_mobile_ids(&state), vec![UnitId(1)]);
}

#[test]
fn selected_inventory_and_gather_wait_reason_are_catalog_derived() {
    let content = base_content();
    let (unit_id, unit_def) = content
        .catalog
        .units
        .iter()
        .find(|(_, unit)| unit.inventory.is_some() && unit.gatherer.is_some())
        .expect("base has an inventory gatherer");
    let unit_id = unit_id.clone();
    let inventory_def = unit_def.inventory.as_ref().unwrap();
    let item_id = inventory_def.accepts[0].to_string();
    let mut replica = Replica::new(20);
    let id = 22;
    replica.units.insert(
        id,
        UnitReplica {
            content_id: unit_id.to_string(),
            owner_slot: 0,
            samples: VecDeque::from([(1, Pos { x: 12.0, y: 8.0 })]),
        },
    );
    replica.economy.inventories.push(InventoryState {
        unit: UnitId(id),
        items: vec![BalanceState {
            resource: item_id,
            on_hand: 17,
            reserved: 4,
        }],
    });
    replica.economy.orders.push(OrderState {
        unit: UnitId(id),
        node: Some(NodeId(4)),
        receiver: None,
        stage: GatherStage::Waiting {
            reason: GatherWaitReason::DestinationFull,
        },
    });
    let model = panel_model(
        &replica,
        &content,
        &Selection(Some(id)),
        &CommandResults::default(),
    );
    assert!(
        model
            .lines
            .iter()
            .any(|line| line.contains("Inventory · this unit"))
    );
    assert!(
        model
            .lines
            .iter()
            .any(|line| line.contains("17 on hand · 4 reserved"))
    );
    assert!(
        model
            .lines
            .iter()
            .any(|line| line.contains(&format!("Capacity: 17/{}", inventory_def.capacity)))
    );
    assert!(
        model
            .lines
            .iter()
            .any(|line| line == "Gather: waiting · destination_full")
    );
}

#[test]
fn selection_is_cleared_when_a_coherent_replacement_removes_or_foreigns_it() {
    use bevy::prelude::MinimalPlugins;
    let mut replica = Replica::new(20);
    replica.units.insert(
        7,
        UnitReplica {
            content_id: "base:command_center".into(),
            owner_slot: 0,
            samples: VecDeque::from([(1, Pos { x: 8.0, y: 8.0 })]),
        },
    );
    let state = ReplicaState {
        welcome: Some(WelcomeInfo {
            slot: 0,
            tick: 1,
            tick_hz: 20,
        }),
        replica: Some(replica),
        ..Default::default()
    };
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(state)
        .insert_resource(Selection(Some(7)))
        .insert_resource(Phase::Joined { slot: 0 })
        .init_resource::<CommandResults>()
        .init_resource::<turts_client::present::Presented>()
        .add_systems(Update, update_presented)
        .add_plugins(EconomyUiPlugin);
    app.update();
    assert_eq!(app.world().resource::<Selection>().0, Some(7));

    let mut state = app.world_mut().resource_mut::<ReplicaState>();
    state.replica.as_mut().unwrap().units.clear();
    app.update();
    assert_eq!(app.world().resource::<Selection>().0, None);
}

#[test]
fn known_insufficient_replica_source_disables_production() {
    let (content, mut replica, id, _, _) = producer_fixture();
    for balance in &mut replica.economy.balances {
        balance.on_hand = 0;
        balance.reserved = 0;
    }
    let model = panel_model(
        &replica,
        &content,
        &Selection(Some(id)),
        &CommandResults::default(),
    );
    let produces: Vec<_> = model
        .controls
        .iter()
        .filter(|control| matches!(control.action, UiAction::Produce { .. }))
        .collect();
    assert!(!produces.is_empty());
    assert!(produces.iter().all(|control| !control.enabled));
}

fn cpu_ui_app(scale: f32, jobs: usize) -> (App, Entity, Entity, u32) {
    use bevy::prelude::{AssetPlugin, Camera3d, IsDefaultUiCamera, TransformPlugin};
    use bevy::window::WindowRef;
    use turts_client::{
        commands::{CommandSendStatus, PendingCommands},
        ingest::ReplicaState,
        input::ClickInputPlugin,
        join::AcquiredPackages,
        present::Presented,
    };

    let (content, mut replica, producer, _, _) = producer_fixture();
    if jobs > 1 {
        let first = replica.economy.queues[0].jobs[0].clone();
        replica.economy.queues[0].jobs = (0..jobs)
            .map(|index| {
                let mut job = first.clone();
                job.id = JobId(91 + index as u64);
                job
            })
            .collect();
    }
    let state = ReplicaState {
        welcome: Some(WelcomeInfo {
            slot: 0,
            tick: 1,
            tick_hz: 20,
        }),
        content: Some(content.clone()),
        replica: Some(replica),
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
        .insert_resource(Phase::Joined { slot: 0 })
        .insert_resource(state)
        .insert_resource(AcquiredPackages(Acquired {
            lockfile_hash: String::new(),
            visuals: Default::default(),
            map_size: (64.0, 64.0),
            unit_names: Default::default(),
            content,
        }))
        .insert_resource(Selection(Some(producer)))
        .init_resource::<CommandResults>()
        .init_resource::<PendingCommands>()
        .init_resource::<CommandSendStatus>()
        .insert_resource(Presented::default())
        .add_systems(Update, update_presented)
        .add_plugins((EconomyUiPlugin, ClickInputPlugin));

    let window = app
        .world_mut()
        .spawn((
            Window {
                resolution: WindowResolution::new(1280, 720).with_scale_factor_override(scale),
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
                        physical_size: UVec2::new(1280, 720),
                        scale_factor: scale,
                    }),
                    ..default()
                },
                viewport: Some(Viewport {
                    physical_size: UVec2::new(1280, 720),
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
    for _ in 0..4 {
        app.update();
    }
    (app, window, camera, producer)
}

fn pointer_event(app: &mut App, window: Entity, position: Vec2, action: PointerAction) {
    use bevy::camera::NormalizedRenderTarget;
    let target = WindowRef::Entity(window)
        .normalize(Some(window))
        .expect("window target normalizes");
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::Window(target),
            position,
        },
        action,
    ));
}

fn widget_center(app: &App, action: &UiAction, scale: f32) -> (Entity, Vec2) {
    let world = app.world();
    let widget = world
        .iter_entities()
        .find(|entity| {
            entity.contains::<Button>()
                && entity
                    .get::<UiActionTarget>()
                    .is_some_and(|target| &target.0 == action)
        })
        .expect("actual action button exists");
    let entity = widget.id();
    let node = widget
        .get::<ComputedNode>()
        .expect("button has completed layout");
    let transform = widget
        .get::<UiGlobalTransform>()
        .expect("button has transform");
    let center = transform.to_scale_angle_translation().2 / scale;
    assert!(node.size().min_element() > 0.0, "actual layout completed");
    (entity, center)
}

#[test]
fn real_pointer_press_survives_replica_progress_update_and_never_reaches_world() {
    let scale = 1.0;
    use turts_client::{commands::PendingCommands, join::AcquiredPackages};
    let (mut app, window, _, producer) = cpu_ui_app(scale, 1);
    let action = UiAction::Produce {
        producer,
        recipe: panel_model(
            app.world()
                .resource::<ReplicaState>()
                .replica
                .as_ref()
                .unwrap(),
            &app.world().resource::<AcquiredPackages>().0.content,
            &Selection(Some(producer)),
            &CommandResults::default(),
        )
        .controls
        .into_iter()
        .find_map(|control| match control.action {
            UiAction::Produce { recipe, .. } => Some(recipe),
            _ => None,
        })
        .unwrap(),
    };
    let (button, point) = widget_center(&app, &action, scale);
    pointer_event(
        &mut app,
        window,
        point,
        PointerAction::Move { delta: Vec2::ZERO },
    );
    app.update();
    pointer_event(
        &mut app,
        window,
        point,
        PointerAction::Press(PointerButton::Primary),
    );
    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .set_cursor_position(Some(point));
    app.world_mut()
        .write_message(bevy::input::mouse::MouseButtonInput {
            button: MouseButton::Left,
            state: ButtonState::Pressed,
            window,
        });
    app.update();
    assert!(app.world().entity(button).contains::<Pressed>());
    assert!(app.world().resource::<PendingCommands>().0.is_empty());

    app.world_mut()
        .resource_mut::<ReplicaState>()
        .replica
        .as_mut()
        .unwrap()
        .economy
        .queues[0]
        .jobs[0]
        .elapsed_ticks += 1;
    app.update();
    let (same_button, _) = widget_center(&app, &action, scale);
    assert_eq!(
        button, same_button,
        "progress updates retain the button entity"
    );
    assert!(app.world().entity(button).contains::<Pressed>());

    pointer_event(
        &mut app,
        window,
        point,
        PointerAction::Release(PointerButton::Primary),
    );
    app.world_mut()
        .write_message(bevy::input::mouse::MouseButtonInput {
            button: MouseButton::Left,
            state: ButtonState::Released,
            window,
        });
    app.update();
    app.update();
    assert_eq!(
        app.world()
            .resource::<PendingCommands>()
            .0
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec![turts_net_protocol::PlayerCommandKind::Produce {
            producer_id: UnitId(producer),
            recipe_id: match action {
                UiAction::Produce { recipe, .. } => recipe,
                _ => unreachable!(),
            },
        }]
    );
    assert_eq!(app.world().resource::<Selection>().0, Some(producer));
}

#[test]
fn real_scroll_reaches_the_fifth_exact_cancel_action_at_dpi_two() {
    let scale = 2.0;
    let (mut app, window, camera_entity, producer) = cpu_ui_app(scale, 5);
    let root = app
        .world_mut()
        .query_filtered::<Entity, With<turts_client::economy_ui::UiPanelRoot>>()
        .single(app.world())
        .unwrap();
    assert_eq!(
        app.world().get::<Node>(root).unwrap().overflow.y,
        OverflowAxis::Scroll
    );
    let last = UiAction::Cancel {
        producer,
        job: JobId(95),
    };
    assert!(
        !rendered_ui_snapshot(app.world())
            .iter()
            .any(|row| row.action.as_ref() == Some(&last) && row.visible)
    );

    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .resolution
        .set_physical_resolution(800, 480);
    {
        let mut camera = app.world_mut().get_mut::<Camera>(camera_entity).unwrap();
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: UVec2::new(800, 480),
            scale_factor: scale,
        });
        camera.viewport = Some(Viewport {
            physical_size: UVec2::new(800, 480),
            ..default()
        });
    }
    app.update();
    let root_transform = app.world().get::<UiGlobalTransform>(root).unwrap();
    let point = root_transform.to_scale_angle_translation().2 / scale;
    let target = WindowRef::Entity(window).normalize(Some(window)).unwrap();
    let normalized = bevy::camera::NormalizedRenderTarget::Window(target);
    app.world_mut().write_message(PointerInput::new(
        PointerId::Mouse,
        Location {
            target: normalized,
            position: point,
        },
        PointerAction::Move { delta: Vec2::ZERO },
    ));
    app.update();
    let mut reachable = std::collections::HashSet::new();
    for row in rendered_ui_snapshot(app.world()) {
        if row.visible
            && let Some(UiAction::Cancel { job, .. }) = row.action
        {
            reachable.insert(job);
        }
    }
    for _ in 0..12 {
        app.world_mut().write_message(PointerInput::new(
            PointerId::Mouse,
            Location {
                target: bevy::camera::NormalizedRenderTarget::Window(
                    WindowRef::Entity(window).normalize(Some(window)).unwrap(),
                ),
                position: point,
            },
            PointerAction::Scroll {
                unit: bevy::input::mouse::MouseScrollUnit::Line,
                x: 0.0,
                y: -80.0,
                phase: bevy::input::touch::TouchPhase::Moved,
            },
        ));
        app.update();
        for row in rendered_ui_snapshot(app.world()) {
            if row.visible
                && let Some(UiAction::Cancel { job, .. }) = row.action
            {
                reachable.insert(job);
            }
        }
    }
    assert!(app.world().get::<ScrollPosition>(root).unwrap().y > 0.0);
    assert!(
        (91..=95).all(|job| reachable.contains(&JobId(job))),
        "all five exact jobs must be visible while scrolling after resize: {reachable:?}"
    );
    let rows = rendered_ui_snapshot(app.world());
    assert!(
        rows.iter()
            .any(|row| row.action.as_ref() == Some(&last) && row.visible),
        "the fifth cancel action must be visible after wheel scrolling"
    );
    let (button, cancel_point) = widget_center(&app, &last, scale);
    assert!(app.world().entity(button).contains::<Button>());
    pointer_event(
        &mut app,
        window,
        cancel_point,
        PointerAction::Move { delta: Vec2::ZERO },
    );
    app.update();
    pointer_event(
        &mut app,
        window,
        cancel_point,
        PointerAction::Press(PointerButton::Primary),
    );
    app.update();
    pointer_event(
        &mut app,
        window,
        cancel_point,
        PointerAction::Release(PointerButton::Primary),
    );
    app.update();
    app.update();
    assert_eq!(
        app.world()
            .resource::<turts_client::commands::PendingCommands>()
            .0
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec![turts_net_protocol::PlayerCommandKind::CancelProduction {
            producer_id: UnitId(producer),
            job_id: JobId(95)
        }],
        "scrolling and activating the displayed control preserves its exact job ID"
    );
}
