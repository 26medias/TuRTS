use std::{collections::VecDeque, path::PathBuf};

use bevy::{
    camera::visibility::InheritedVisibility,
    prelude::*,
    ui::{ComputedNode, UiGlobalTransform},
};
use turts_client::{
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
fn rendered_snapshot_reads_actual_text_layout_visibility_and_action_components() {
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
    assert!(rendered[0].visible && rendered[0].enabled);
    assert_eq!(rendered[0].action, Some(action));
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
