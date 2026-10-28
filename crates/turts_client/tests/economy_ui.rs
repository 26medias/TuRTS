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
use turts_client::acquire::build_acquired;
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
    render::{
        fit_camera_transform, free_scene_rect, max_declared_pick_radius, projected_ground_center,
    },
    replica::{Replica, UnitReplica},
};
use turts_mod_api::{ContentId, Cost, CostSource, ResolvedCost, ResolvedInventory};
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

fn acquired_package(name: &str) -> Acquired {
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("../../mods/{name}"));
    let archive = PackageArchive::parse(&pack_dir(&package).expect("pack fixture package"))
        .expect("parse fixture package");
    build_acquired(&[archive], String::new()).expect("build fixture acquisition")
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
            facing: None,
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

        enabled: true,
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
fn rendered_ui_snapshot_matches_optional_clip_and_real_button_pressed_state() {
    let (mut app, _, _, _) = cpu_ui_app(1.0, 1);
    let root = app
        .world()
        .iter_entities()
        .find(|entity| entity.contains::<turts_client::economy_ui::UiPanelRoot>())
        .unwrap()
        .id();
    assert!(app.world().get::<CalculatedClip>(root).is_none());

    let snapshot = rendered_ui_snapshot(app.world());
    let panel = snapshot.iter().find(|row| row.kind == "panel").unwrap();
    assert!(
        panel.visible,
        "unclipped panel root is rendered without CalculatedClip"
    );

    let action = snapshot
        .iter()
        .find(|row| row.action.is_some())
        .unwrap()
        .action
        .clone()
        .unwrap();
    let action_row = snapshot
        .iter()
        .find(|row| row.action.as_ref() == Some(&action))
        .unwrap();
    assert_eq!(action_row.button_pressed, Some(false));
    let button = app
        .world()
        .iter_entities()
        .find(|entity| {
            entity.contains::<Button>()
                && entity
                    .get::<UiActionTarget>()
                    .is_some_and(|target| target.0 == action)
        })
        .unwrap()
        .id();
    app.world_mut().entity_mut(button).insert(bevy::ui::Pressed);
    let pressed = rendered_ui_snapshot(app.world());
    assert_eq!(
        pressed
            .iter()
            .find(|row| row.action.as_ref() == Some(&action))
            .unwrap()
            .button_pressed,
        Some(true),
        "pressed witness comes from the real Button owner",
    );

    app.world_mut().entity_mut(root).insert(Visibility::Hidden);
    app.update();
    assert!(
        !rendered_ui_snapshot(app.world())
            .iter()
            .find(|row| row.kind == "panel")
            .unwrap()
            .visible,
        "hidden inherited visibility must still hide the root",
    );
    app.world_mut().entity_mut(root).insert(Visibility::Visible);
    app.world_mut()
        .entity_mut(root)
        .insert(CalculatedClip::FullyClipped);
    assert!(
        !rendered_ui_snapshot(app.world())
            .iter()
            .find(|row| row.kind == "panel")
            .unwrap()
            .visible,
        "fully clipped roots remain excluded",
    );
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
                facing: None,
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

    let viewport_origin = Vec2::new(100.0, 80.0);
    let panel_center = root_transform.to_scale_angle_translation().2;
    let border = (panel_center + viewport_origin) / scale;
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

    let panel_half = root_node.size() * 0.5;
    let panel_bounds = Rect::from_center_half_size(panel_center, panel_half);
    let outside_physical = Vec2::new(panel_center.x, panel_bounds.min.y - 24.0) + viewport_origin;
    let outside = outside_physical / scale;
    assert!(
        cursor_in_ui_viewport(outside, &camera)
            .is_some_and(|cursor| !root_node.contains_point(root_transform, cursor)),
        "point above the measured bottom panel must reach the world"
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
        matches!(app.world().resource::<PendingCommands>().0.front(), Some(turts_client::commands::PendingCommand::Submit { kind: turts_net_protocol::PlayerCommandKind::MoveTo { unit_ids, .. }, trace_step: None }) if unit_ids == &vec![UnitId(mobile)])
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
            facing: None,
            samples: VecDeque::new(),
        },
    );
    replica.units.insert(
        3,
        UnitReplica {
            content_id: mobile_id.clone(),
            owner_slot: 1,
            facing: None,
            samples: VecDeque::new(),
        },
    );
    replica.units.insert(
        1,
        UnitReplica {
            content_id: mobile_id,
            owner_slot: 0,
            facing: None,
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
            facing: None,
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

        port: turts_net_protocol::InventoryPort::Main,
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
            facing: None,
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
fn panel_model_shows_blocked_reason() {
    let (content, mut replica, id, _, _) = producer_fixture();
    replica.economy.queues[0].jobs[0].stage = JobStage::OutputBlocked;
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
            .any(|line| line.contains("output_blocked"))
    );
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
    cpu_ui_app_at(scale, jobs, 1280, 720)
}

fn cpu_ui_app_at(scale: f32, jobs: usize, width: u32, height: u32) -> (App, Entity, Entity, u32) {
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
    for _ in 0..4 {
        app.update();
    }
    (app, window, camera, producer)
}

#[test]
fn narrow_window_docks_actual_scroll_panel_below_the_scene() {
    let (app, window, _, _) = cpu_ui_app_at(1.0, 1, 800, 720);
    let root = app
        .world()
        .iter_entities()
        .find(|entity| entity.contains::<turts_client::economy_ui::UiPanelRoot>())
        .expect("actual economy panel exists");
    let node = root
        .get::<ComputedNode>()
        .expect("panel completed Bevy layout");
    let transform = root
        .get::<UiGlobalTransform>()
        .expect("panel has its computed UI transform");
    let center = transform.to_scale_angle_translation().2;
    let half = node.size() * 0.5;
    let panel = Rect::from_center_half_size(center, half);
    let viewport = app.world().get::<Window>(window).unwrap().physical_width() as f32;
    let height = app.world().get::<Window>(window).unwrap().physical_height() as f32;

    assert!(
        panel.min.y > height * 0.5,
        "narrow panel should dock below the vertical midpoint: {panel:?}"
    );
    assert!(
        node.size().x >= viewport - 24.0,
        "narrow panel should span nearly the available width: {:?} of {viewport}px",
        node.size()
    );
    assert!(
        node.size().y <= height * 0.4,
        "narrow panel height must remain capped and scrollable: {:?}",
        node.size()
    );
    assert_eq!(root.get::<Node>().unwrap().overflow.y, OverflowAxis::Scroll);
}

#[test]
fn actual_panel_redocks_on_resize_in_both_directions() {
    let (mut app, window, camera_entity, _) = cpu_ui_app_at(1.0, 1, 1280, 720);
    let panel_bounds = |app: &App| {
        let panel = app
            .world()
            .iter_entities()
            .find(|entity| entity.contains::<turts_client::economy_ui::UiPanelRoot>())
            .unwrap();
        let node = panel.get::<ComputedNode>().unwrap();
        let center = panel
            .get::<UiGlobalTransform>()
            .unwrap()
            .to_scale_angle_translation()
            .2;
        Rect::from_center_half_size(center, node.size() * 0.5)
    };
    assert!(
        panel_bounds(&app).min.x < 20.0,
        "wide panel starts at the left edge"
    );

    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .resolution
        .set_physical_resolution(800, 720);
    {
        let mut camera = app.world_mut().get_mut::<Camera>(camera_entity).unwrap();
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: UVec2::new(800, 720),
            scale_factor: 1.0,
        });
        camera.viewport = Some(Viewport {
            physical_size: UVec2::new(800, 720),
            ..default()
        });
    }
    app.update();
    app.update();
    assert!(
        panel_bounds(&app).min.y > 360.0,
        "narrow panel docks at the bottom"
    );

    app.world_mut()
        .get_mut::<Window>(window)
        .unwrap()
        .resolution
        .set_physical_resolution(1280, 720);
    {
        let mut camera = app.world_mut().get_mut::<Camera>(camera_entity).unwrap();
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: UVec2::new(1280, 720),
            scale_factor: 1.0,
        });
        camera.viewport = Some(Viewport {
            physical_size: UVec2::new(1280, 720),
            ..default()
        });
    }
    app.update();
    app.update();
    assert!(
        panel_bounds(&app).min.x < 20.0,
        "wide panel returns to the left"
    );
}

#[test]
fn catalog_pick_bounds_fit_the_actual_computed_panel_at_normal_narrow_and_dpi_two() {
    use bevy::camera::{CameraProjection, PerspectiveProjection};

    let packages: Vec<_> = ["base", "haul", "acme", "cedar"]
        .into_iter()
        .map(|name| (name, acquired_package(name)))
        .collect();
    let screen_cases = [
        (1.0, 1280, 720),
        (1.0, 800, 720),
        (1.0, 600, 720),
        (2.0, 1280, 720),
    ];

    for (scale_factor, width, height) in screen_cases {
        let (app, window, camera_entity, _) = cpu_ui_app_at(scale_factor, 1, width, height);
        let window = app.world().get::<Window>(window).unwrap();
        let viewport_size = Vec2::new(
            window.physical_width() as f32,
            window.physical_height() as f32,
        );
        let panel = app
            .world()
            .iter_entities()
            .find(|entity| entity.contains::<turts_client::economy_ui::UiPanelRoot>())
            .expect("actual economy panel exists");
        let node = panel.get::<ComputedNode>().expect("panel completed layout");
        let center = panel
            .get::<UiGlobalTransform>()
            .expect("panel has computed transform")
            .to_scale_angle_translation()
            .2;
        let panel_rect = Rect::from_center_half_size(center, node.size() * 0.5);
        let margin = 12.0 / node.inverse_scale_factor;
        let free = free_scene_rect(viewport_size, Some(panel_rect), margin)
            .expect("panel leaves a nonempty scene viewport");

        let mut perspective = PerspectiveProjection::default();
        perspective.update(viewport_size.x, viewport_size.y);
        let mut camera = app.world().get::<Camera>(camera_entity).unwrap().clone();
        camera.computed.target_info = Some(RenderTargetInfo {
            physical_size: UVec2::new(viewport_size.x as u32, viewport_size.y as u32),
            scale_factor,
        });
        camera.computed.clip_from_view = perspective.get_clip_from_view();

        for (package_name, acquired) in &packages {
            let radius = max_declared_pick_radius(&acquired.content);
            let transform = fit_camera_transform(
                acquired.map_size,
                radius,
                viewport_size,
                free,
                perspective.fov,
                perspective.near,
            )
            .unwrap_or_else(|| {
                panic!("{package_name} should fit at {width}x{height} scale {scale_factor}")
            });
            let global = GlobalTransform::from(transform);
            let (map_width, map_depth) = acquired.map_size;
            for point in [
                Pos {
                    x: -radius,
                    y: -radius,
                },
                Pos {
                    x: map_width + radius,
                    y: -radius,
                },
                Pos {
                    x: -radius,
                    y: map_depth + radius,
                },
                Pos {
                    x: map_width + radius,
                    y: map_depth + radius,
                },
            ] {
                let screen = projected_ground_center(&camera, &global, point)
                    .unwrap_or_else(|| panic!("{package_name} padded map corner {point:?} is outside the actual camera"));
                let physical = screen * scale_factor;
                assert!(
                    free.contains(physical),
                    "{package_name} corner {point:?} projects to {physical:?} outside {free:?} at {width}x{height} scale {scale_factor}"
                );
            }
        }
    }
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
fn retired_job_controls_do_not_accumulate_and_current_controls_keep_identity() {
    let (mut app, _window, _camera, producer) = cpu_ui_app(1.0, 1);
    let produce_action = app
        .world()
        .iter_entities()
        .find_map(|entity| {
            entity
                .get::<UiActionTarget>()
                .filter(|_| entity.contains::<Button>())
                .map(|target| target.0.clone())
        })
        .expect("producer action exists");
    let stable_button = app
        .world()
        .iter_entities()
        .find(|entity| {
            entity.contains::<Button>()
                && entity
                    .get::<UiActionTarget>()
                    .is_some_and(|target| target.0 == produce_action)
        })
        .unwrap()
        .id();

    let stable_button_count = app
        .world()
        .iter_entities()
        .filter(|entity| entity.contains::<Button>())
        .count();
    let mut current_job = JobId(91);
    for job_id in 100..140 {
        let old_cancel = UiAction::Cancel {
            producer,
            job: current_job,
        };
        let old_entities: Vec<_> = app
            .world()
            .iter_entities()
            .filter(|entity| {
                entity
                    .get::<UiActionTarget>()
                    .is_some_and(|target| target.0 == old_cancel)
            })
            .map(|entity| entity.id())
            .collect();

        {
            let mut state = app.world_mut().resource_mut::<ReplicaState>();
            let queue = state
                .replica
                .as_mut()
                .unwrap()
                .economy
                .queues
                .iter_mut()
                .find(|queue| queue.producer.0 == producer)
                .unwrap();
            let mut job = queue.jobs[0].clone();
            job.id = JobId(job_id);
            job.elapsed_ticks += 1;
            queue.jobs = vec![job];
        }
        app.update();
        current_job = JobId(job_id);

        for entity in old_entities {
            assert!(
                app.world().get_entity(entity).is_err(),
                "retired job action descendants are despawned"
            );
        }
        let buttons: Vec<_> = app
            .world()
            .iter_entities()
            .filter(|entity| entity.contains::<Button>())
            .collect();
        assert_eq!(
            buttons.len(),
            stable_button_count,
            "control count stays bounded to the current model"
        );
        assert_eq!(
            app.world()
                .iter_entities()
                .find(|entity| {
                    entity.contains::<Button>()
                        && entity
                            .get::<UiActionTarget>()
                            .is_some_and(|target| target.0 == produce_action)
                })
                .unwrap()
                .id(),
            stable_button,
            "a still-current control keeps its entity identity"
        );
    }
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
        vec![turts_client::commands::PendingCommand::Submit {
            kind: turts_net_protocol::PlayerCommandKind::Produce {
                producer_id: UnitId(producer),
                recipe_id: match action {
                    UiAction::Produce { recipe, .. } => recipe,
                    _ => unreachable!(),
                },
            },
            trace_step: None,
        }]
    );
    assert_eq!(app.world().resource::<Selection>().0, Some(producer));
}

#[test]
fn real_scroll_reaches_the_fifth_exact_cancel_action_at_dpi_two() {
    let scale = 2.0;
    let (mut app, window, _camera_entity, producer) = cpu_ui_app_at(scale, 5, 800, 480);
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
    for _ in 0..280 {
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
                y: -0.25,
                phase: bevy::input::touch::TouchPhase::Moved,
            },
        ));
        app.update();
        let visible_now = rendered_ui_snapshot(app.world());
        for row in visible_now.iter().cloned() {
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
        vec![turts_client::commands::PendingCommand::Submit {
            kind: turts_net_protocol::PlayerCommandKind::CancelProduction {
                producer_id: UnitId(producer),
                job_id: JobId(95),
            },
            trace_step: None,
        }],
        "scrolling and activating the displayed control preserves its exact job ID"
    );
}

fn forge_ui_content() -> ClientContent {
    let forge = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/forge");
    let archive = PackageArchive::parse(&pack_dir(&forge).expect("pack forge UI fixture"))
        .expect("parse forge UI archive");
    ClientContent::from_archives(&[archive]).expect("resolve forge UI definitions")
}

fn forge_lab_replica(main_shells: u64, output_shells: u64) -> (ClientContent, Replica, u32) {
    let mut content = forge_ui_content();
    let lab_id = ContentId::parse("forge:lab").unwrap();
    let shell_id = ContentId::parse("forge:shell").unwrap();
    let lab = content.catalog.units.get_mut(&lab_id).unwrap();
    // The owner catalog in this regression declares both ports, so both rows are valid.
    lab.output_inventory = Some(ResolvedInventory {
        capacity: 10,
        accepts: vec![shell_id.clone()],
    });

    let selected = 71;
    let mut replica = Replica::new(20);
    replica.units.insert(
        selected,
        UnitReplica {
            content_id: lab_id.to_string(),
            owner_slot: 0,
            facing: None,
            samples: VecDeque::from([(1, Pos { x: 8.0, y: 8.0 })]),
        },
    );
    replica.economy.inventories = vec![
        InventoryState {
            unit: UnitId(selected),
            port: turts_net_protocol::InventoryPort::Main,
            items: vec![BalanceState {
                resource: shell_id.to_string(),
                on_hand: main_shells,
                reserved: 0,
            }],
        },
        InventoryState {
            unit: UnitId(selected),
            port: turts_net_protocol::InventoryPort::Output,
            items: vec![BalanceState {
                resource: shell_id.to_string(),
                on_hand: output_shells,
                reserved: 0,
            }],
        },
    ];
    (content, replica, selected)
}

fn research_enabled(content: &ClientContent, replica: &Replica, selected: u32) -> bool {
    panel_model(
        replica,
        content,
        &Selection(Some(selected)),
        &CommandResults::default(),
    )
    .controls
    .into_iter()
    .find(|control| {
        matches!(
            &control.action,
            UiAction::Produce { recipe, .. } if recipe == "forge:hardened_shells"
        )
    })
    .expect("Forge research action")
    .enabled
}

#[test]
fn research_affordability_aggregates_duplicate_costs_and_fails_closed() {
    let (mut content, replica, selected) = forge_lab_replica(475, 0);
    content
        .catalog
        .technologies
        .get_mut(&ContentId::parse("forge:hardened_shells").unwrap())
        .unwrap()
        .costs = vec![
        ResolvedCost {
            resource: ContentId::parse("forge:shell").unwrap(),
            amount: 300,
            source: CostSource::ProducerInventory,
        },
        ResolvedCost {
            resource: ContentId::parse("forge:shell").unwrap(),
            amount: 300,
            source: CostSource::ProducerInventory,
        },
    ];
    assert!(
        !research_enabled(&content, &replica, selected),
        "475 available is below the aggregated 600 shell cost"
    );

    let (mut overflow_content, overflow_replica, overflow_selected) =
        forge_lab_replica(u64::MAX, 0);
    overflow_content
        .catalog
        .technologies
        .get_mut(&ContentId::parse("forge:hardened_shells").unwrap())
        .unwrap()
        .costs = vec![
        ResolvedCost {
            resource: ContentId::parse("forge:shell").unwrap(),
            amount: u64::MAX,
            source: CostSource::ProducerInventory,
        },
        ResolvedCost {
            resource: ContentId::parse("forge:shell").unwrap(),
            amount: 1,
            source: CostSource::ProducerInventory,
        },
    ];
    assert!(
        !research_enabled(&overflow_content, &overflow_replica, overflow_selected),
        "a same-source aggregate above u64 must fail closed"
    );
}

#[test]
fn research_affordability_uses_main_port_independent_of_inventory_row_order() {
    let (mut content, mut replica, selected) = forge_lab_replica(0, 10);
    content
        .catalog
        .technologies
        .get_mut(&ContentId::parse("forge:hardened_shells").unwrap())
        .unwrap()
        .costs = vec![ResolvedCost {
        resource: ContentId::parse("forge:shell").unwrap(),
        amount: 10,
        source: CostSource::ProducerInventory,
    }];
    assert!(!research_enabled(&content, &replica, selected));
    replica.economy.inventories.reverse();
    assert!(
        !research_enabled(&content, &replica, selected),
        "Output stock cannot fund Main-inventory research in either row order"
    );

    for inventory in &mut replica.economy.inventories {
        inventory.items[0].on_hand = if inventory.port == turts_net_protocol::InventoryPort::Main {
            10
        } else {
            0
        };
    }
    assert!(research_enabled(&content, &replica, selected));
    replica.economy.inventories.reverse();
    assert!(
        research_enabled(&content, &replica, selected),
        "Main stock remains available with Output listed first"
    );
}

#[test]
fn machine_panel_shows_main_and_output_inventory_with_each_capacity() {
    let content = forge_ui_content();
    let selected = 72;
    let mut replica = Replica::new(20);
    replica.units.insert(
        selected,
        UnitReplica {
            content_id: "forge:press".into(),
            owner_slot: 0,
            facing: None,
            samples: VecDeque::from([(1, Pos { x: 8.0, y: 8.0 })]),
        },
    );
    replica.economy.inventories = vec![
        InventoryState {
            unit: UnitId(selected),
            port: turts_net_protocol::InventoryPort::Main,
            items: vec![BalanceState {
                resource: "forge:ore".into(),
                on_hand: 2,
                reserved: 0,
            }],
        },
        InventoryState {
            unit: UnitId(selected),
            port: turts_net_protocol::InventoryPort::Output,
            items: vec![BalanceState {
                resource: "forge:shell".into(),
                on_hand: 3,
                reserved: 0,
            }],
        },
    ];
    let text = panel_model(
        &replica,
        &content,
        &Selection(Some(selected)),
        &CommandResults::default(),
    )
    .lines
    .join("\n");
    assert!(
        text.contains("Inventory · Main"),
        "Main port missing: {text}"
    );
    assert!(
        text.contains("Ore: 2 on hand · 0 reserved"),
        "Main ore missing: {text}"
    );
    assert!(
        text.contains("Capacity: 2/6"),
        "Main capacity is wrong: {text}"
    );
    assert!(
        text.contains("Inventory · Output"),
        "Output port missing: {text}"
    );
    assert!(
        text.contains("Shell: 3 on hand · 0 reserved"),
        "Output shell stock missing: {text}"
    );
    assert!(
        text.contains("Capacity: 3/4"),
        "Output capacity is wrong: {text}"
    );
}

#[test]
fn unselected_factory_names_the_lab_and_offers_selection() {
    let (content, replica, _) = forge_lab_replica(0, 0);
    let model = panel_model(
        &replica,
        &content,
        &Selection(None),
        &CommandResults::default(),
    );
    assert!(
        model
            .controls
            .iter()
            .any(|c| c.label == "Select Proving Lab")
    );
    assert!(
        model
            .lines
            .iter()
            .any(|line| line.contains("research") && line.contains("Select"))
    );
}

#[test]
fn research_panel_explains_cost_and_missing_stock() {
    let (content, replica, lab) = forge_lab_replica(0, 0);
    let model = panel_model(
        &replica,
        &content,
        &Selection(Some(lab)),
        &CommandResults::default(),
    );
    assert!(
        model
            .lines
            .iter()
            .any(|line| line.contains("cost") && line.contains("10"))
    );
    assert!(
        model
            .lines
            .iter()
            .any(|line| line.contains("Waiting") && line.contains("inventory"))
    );
    assert!(model.lines.iter().any(|line| line.contains("stationary")));
}
