mod support;

use std::collections::BTreeSet;

use support::feedback::{alert_state, clear_feedback, feedback_app, force_all_alert_sources};
use turts_client::feedback::FrameFeedback;
use turts_client::{
    debug_feed::{AlertPingsFeed, AlertsFeed},
    ingest::ReplicaState,
};

#[test]
fn forced_replicated_states_produce_alert_lines_sounds_and_pings() {
    let (state, acquired) = alert_state();
    let mut app = feedback_app(state, acquired);
    app.update();

    force_all_alert_sources(&mut app.world_mut().resource_mut::<ReplicaState>());
    clear_feedback(&mut app);
    app.update();

    let mut ids: BTreeSet<String> = app
        .world()
        .resource::<FrameFeedback>()
        .alerts
        .iter()
        .filter_map(|alert| alert["id"].as_str().map(str::to_owned))
        .collect();
    app.world_mut()
        .resource_mut::<ReplicaState>()
        .replica
        .as_mut()
        .unwrap()
        .last_applied_tick = Some(102);
    app.update();
    ids.extend(
        app.world()
            .resource::<FrameFeedback>()
            .alerts
            .iter()
            .filter_map(|alert| alert["id"].as_str().map(str::to_owned)),
    );
    assert!(
        app.world()
            .resource::<FrameFeedback>()
            .alerts
            .iter()
            .all(|alert| alert["id"] != "feedback_inserter_waiting"),
        "exactly five seconds is not longer than the configured wait threshold"
    );
    app.world_mut()
        .resource_mut::<ReplicaState>()
        .replica
        .as_mut()
        .unwrap()
        .last_applied_tick = Some(103);
    app.update();
    ids.extend(
        app.world()
            .resource::<FrameFeedback>()
            .alerts
            .iter()
            .filter_map(|alert| alert["id"].as_str().map(str::to_owned)),
    );
    assert_eq!(
        ids,
        BTreeSet::from([
            "muster_unit_damaged".to_owned(),
            "feedback_research_ready".to_owned(),
            "feedback_research_complete".to_owned(),
            "feedback_extractor_full".to_owned(),
            "feedback_output_blocked".to_owned(),
            "feedback_out_of_ammo".to_owned(),
            "feedback_inserter_waiting".to_owned(),
        ])
    );
    assert_eq!(
        app.world().resource::<AlertsFeed>().0.len(),
        5,
        "the alert list keeps its last five player-facing lines"
    );
    assert_eq!(app.world().resource::<AlertPingsFeed>().0.len(), 7);
}

#[test]
fn repeated_damage_inside_the_rule_cooldown_does_not_add_an_alert() {
    let (state, acquired) = alert_state();
    let mut app = feedback_app(state, acquired);
    app.update();

    force_all_alert_sources(&mut app.world_mut().resource_mut::<ReplicaState>());
    clear_feedback(&mut app);
    app.update();
    let before = app
        .world()
        .resource::<AlertsFeed>()
        .0
        .iter()
        .filter(|alert| alert["id"] == "muster_unit_damaged")
        .count();
    assert_eq!(
        before, 1,
        "first own-unit health loss creates one damage alert"
    );

    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(100),
    ));
    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        let replica = state.replica.as_mut().unwrap();
        replica.last_applied_tick = Some(3);
        replica.healths[0].current = 30;
        replica
            .health_trace
            .get_mut(&1)
            .unwrap()
            .push(turts_client::replica::HealthTraceEntry {
                tick: 3,
                current: 30,
            });
    }
    clear_feedback(&mut app);
    app.update();

    let after = app
        .world()
        .resource::<AlertsFeed>()
        .0
        .iter()
        .filter(|alert| alert["id"] == "muster_unit_damaged")
        .count();
    assert_eq!(
        after, before,
        "the second damage occurs before the fixture's five-second cooldown"
    );
}

#[test]
fn foreign_health_changes_do_not_create_own_unit_alerts() {
    let (state, acquired) = alert_state();
    let mut app = feedback_app(state, acquired);
    app.update();

    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        let replica = state.replica.as_mut().unwrap();
        replica.last_applied_tick = Some(2);
        replica.healths.push(turts_net_protocol::HealthRecord {
            unit: turts_net_protocol::UnitId(8),
            current: 20,
        });
        replica.health_trace.insert(
            8,
            vec![
                turts_client::replica::HealthTraceEntry {
                    tick: 1,
                    current: 40,
                },
                turts_client::replica::HealthTraceEntry {
                    tick: 2,
                    current: 20,
                },
            ],
        );
    }
    app.update();

    let damage = app
        .world()
        .resource::<AlertsFeed>()
        .0
        .iter()
        .filter(|alert| alert["id"] == "muster_unit_damaged")
        .count();
    assert_eq!(
        damage, 0,
        "foreign-owned health loss is not an own-unit alert"
    );
}

#[test]
fn research_ready_uses_the_same_catalog_cost_for_both_sides_of_the_transition() {
    use turts_mod_api::{ContentId, PackageId};
    let (state, mut acquired) = alert_state();
    let technology = ContentId::new(&PackageId("muster".into()), "test_research");
    acquired
        .0
        .content
        .catalog
        .technologies
        .get_mut(&technology)
        .unwrap()
        .costs[0]
        .amount = 10;
    let mut app = feedback_app(state, acquired);
    app.update();

    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        let replica = state.replica.as_mut().unwrap();
        replica.last_applied_tick = Some(2);
        replica.economy.inventories[0].items = vec![turts_net_protocol::BalanceState {
            resource: "muster:ore_chunk".into(),
            on_hand: 9,
            reserved: 0,
        }];
    }
    app.update();
    assert!(
        app.world()
            .resource::<FrameFeedback>()
            .alerts
            .iter()
            .all(|alert| alert["id"] != "feedback_research_ready"),
        "nine units do not meet the catalog's ten unit research cost"
    );

    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        let replica = state.replica.as_mut().unwrap();
        replica.last_applied_tick = Some(3);
        replica.economy.inventories[0].items[0].on_hand = 10;
        replica.economy.inventories[0].items[0].reserved = 1;
    }
    app.update();
    assert!(
        app.world()
            .resource::<FrameFeedback>()
            .alerts
            .iter()
            .all(|alert| alert["id"] != "feedback_research_ready"),
        "reserved inventory is not spendable toward the research cost"
    );

    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        let replica = state.replica.as_mut().unwrap();
        replica.last_applied_tick = Some(4);
        replica.economy.inventories[0].items[0].reserved = 0;
        replica.economy.queues = vec![turts_net_protocol::QueueState {
            producer: turts_net_protocol::UnitId(6),
            enabled: false,
            jobs: Vec::new(),
        }];
    }
    app.update();
    assert!(
        app.world()
            .resource::<FrameFeedback>()
            .alerts
            .iter()
            .all(|alert| alert["id"] != "feedback_research_ready"),
        "a disabled station queue cannot start a technology"
    );

    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        let replica = state.replica.as_mut().unwrap();
        replica.last_applied_tick = Some(5);
        replica.economy.queues[0].enabled = true;
        replica.economy.queues[0]
            .jobs
            .push(turts_net_protocol::JobState {
                id: turts_net_protocol::JobId(9),
                recipe: "muster:other_job".into(),
                stage: turts_net_protocol::JobStage::Running,
                elapsed_ticks: 1,
                duration_ticks: 20,
                charged: Vec::new(),
                reserved: Vec::new(),
            });
    }
    app.update();
    assert!(
        app.world()
            .resource::<FrameFeedback>()
            .alerts
            .iter()
            .all(|alert| alert["id"] != "feedback_research_ready"),
        "a full station queue cannot start another technology"
    );

    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        let replica = state.replica.as_mut().unwrap();
        replica.last_applied_tick = Some(6);
        replica.economy.queues[0].jobs[0].recipe = "muster:test_research".into();
    }
    app.update();
    assert!(
        app.world()
            .resource::<FrameFeedback>()
            .alerts
            .iter()
            .all(|alert| alert["id"] != "feedback_research_ready"),
        "a technology already present in the station queue is not startable"
    );

    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        let replica = state.replica.as_mut().unwrap();
        replica.last_applied_tick = Some(7);
        replica.economy.queues[0].jobs.clear();
    }
    app.update();
    assert!(
        app.world()
            .resource::<FrameFeedback>()
            .alerts
            .iter()
            .any(|alert| alert["id"] == "feedback_research_ready"),
        "the station emits a ready transition when its catalog cost and queue constraints are satisfied"
    );
}

#[test]
fn research_ready_is_blocked_by_the_same_slot_station_in_progress_elsewhere() {
    let (state, acquired) = alert_state();
    let mut app = feedback_app(state, acquired);
    app.update();
    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        let replica = state.replica.as_mut().unwrap();
        replica
            .units
            .insert(9, support::feedback::unit("muster:lab", 0));
        replica
            .economy
            .inventories
            .push(turts_net_protocol::InventoryState {
                unit: turts_net_protocol::UnitId(9),
                port: turts_net_protocol::InventoryPort::Main,
                items: vec![turts_net_protocol::BalanceState {
                    resource: "muster:ore_chunk".into(),
                    on_hand: 1,
                    reserved: 0,
                }],
            });
        replica.economy.queues.push(turts_net_protocol::QueueState {
            producer: turts_net_protocol::UnitId(9),
            enabled: true,
            jobs: vec![turts_net_protocol::JobState {
                id: turts_net_protocol::JobId(2),
                recipe: "muster:test_research".into(),
                stage: turts_net_protocol::JobStage::Running,
                elapsed_ticks: 1,
                duration_ticks: 20,
                charged: Vec::new(),
                reserved: Vec::new(),
            }],
        });
        replica.last_applied_tick = Some(2);
        replica.economy.inventories[0].items = vec![turts_net_protocol::BalanceState {
            resource: "muster:ore_chunk".into(),
            on_hand: 1,
            reserved: 0,
        }];
    }
    app.update();
    assert!(
        app.world()
            .resource::<FrameFeedback>()
            .alerts
            .iter()
            .all(|alert| alert["id"] != "feedback_research_ready"),
        "a same-slot job at another owned station makes this technology unavailable everywhere"
    );
}

#[test]
fn research_in_progress_at_another_slot_does_not_block_own_research_ready() {
    let (state, acquired) = alert_state();
    let mut app = feedback_app(state, acquired);
    app.update();
    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        let replica = state.replica.as_mut().unwrap();
        replica
            .units
            .insert(9, support::feedback::unit("muster:lab", 1));
        replica.economy.inventories[0].items = vec![turts_net_protocol::BalanceState {
            resource: "muster:ore_chunk".into(),
            on_hand: 1,
            reserved: 0,
        }];
        replica
            .economy
            .inventories
            .push(turts_net_protocol::InventoryState {
                unit: turts_net_protocol::UnitId(9),
                port: turts_net_protocol::InventoryPort::Main,
                items: Vec::new(),
            });
        replica.economy.queues.push(turts_net_protocol::QueueState {
            producer: turts_net_protocol::UnitId(9),
            enabled: true,
            jobs: vec![turts_net_protocol::JobState {
                id: turts_net_protocol::JobId(2),
                recipe: "muster:test_research".into(),
                stage: turts_net_protocol::JobStage::Running,
                elapsed_ticks: 1,
                duration_ticks: 20,
                charged: Vec::new(),
                reserved: Vec::new(),
            }],
        });
        replica.last_applied_tick = Some(2);
    }
    app.update();
    assert!(
        app.world()
            .resource::<FrameFeedback>()
            .alerts
            .iter()
            .any(|alert| alert["id"] == "feedback_research_ready"),
        "only same-slot stations share the in-progress restriction"
    );
}

#[test]
fn research_completion_ping_uses_the_station_that_queued_the_completed_technology() {
    use std::collections::VecDeque;
    use turts_client::replica::UnitReplica;
    use turts_mod_api::{ContentId, PackageId};

    let (state, mut acquired) = alert_state();
    let package = PackageId("muster".into());
    let technology = ContentId::new(&package, "test_research");
    let second_station = ContentId::new(&package, "second_lab");
    let mut second_definition =
        acquired.0.content.catalog.units[&ContentId::new(&package, "lab")].clone();
    second_definition.id = second_station.clone();
    acquired
        .0
        .content
        .catalog
        .units
        .insert(second_station.clone(), second_definition);
    let mut state = state;
    let replica = state.replica.as_mut().unwrap();
    replica.units.insert(
        9,
        UnitReplica {
            content_id: second_station.to_string(),
            owner_slot: 0,
            facing: None,
            samples: VecDeque::from([(1, turts_net_protocol::Pos { x: 19.0, y: 7.0 })]),
        },
    );
    replica.economy.queues.push(turts_net_protocol::QueueState {
        producer: turts_net_protocol::UnitId(9),
        enabled: true,
        jobs: vec![turts_net_protocol::JobState {
            id: turts_net_protocol::JobId(2),
            recipe: technology.to_string(),
            stage: turts_net_protocol::JobStage::Running,
            elapsed_ticks: 19,
            duration_ticks: 20,
            charged: Vec::new(),
            reserved: Vec::new(),
        }],
    });

    let mut app = feedback_app(state, acquired);
    app.update();
    app.world_mut().resource_mut::<AlertPingsFeed>().0.clear();
    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        let replica = state.replica.as_mut().unwrap();
        replica.last_applied_tick = Some(2);
        replica.research[0].completed.push(technology.to_string());
        replica
            .economy
            .queues
            .iter_mut()
            .find(|queue| queue.producer.0 == 9)
            .unwrap()
            .jobs
            .clear();
    }
    app.update();

    let pings = &app.world().resource::<AlertPingsFeed>().0;
    assert_eq!(pings.len(), 1);
    assert_eq!(
        pings[0]["unit"], 9,
        "completion follows the producer recorded on the removed technology job"
    );
    assert_eq!(pings[0]["x"], 19.0);
    assert_eq!(pings[0]["y"], 7.0);
}

#[test]
fn production_completion_needs_output_evidence_and_does_not_treat_job_removal_as_success() {
    use std::collections::VecDeque;
    use turts_client::replica::UnitReplica;
    use turts_mod_api::{AlertDef, AlertTrigger, ContentId, PackageId, ResolvedProduction};
    let (state, mut acquired) = alert_state();
    let package = PackageId("muster".into());
    let recipe = ContentId::new(&package, "test_recipe");
    let output_unit = ContentId::new(&package, "produced_crate");
    acquired.0.content.catalog.production.insert(
        recipe.clone(),
        ResolvedProduction {
            id: recipe,
            name: "Crate production".into(),
            output_unit: output_unit.clone(),
            duration_ms: 1_000,
            costs: Vec::new(),
            payment: turts_mod_api::PaymentPolicy::ChargeOnQueue,
            cancel_refunds: turts_mod_api::CancelRefunds {
                queued: turts_mod_api::RefundRatio {
                    numerator: 0,
                    denominator: 1,
                },
                running: turts_mod_api::RefundRatio {
                    numerator: 0,
                    denominator: 1,
                },
                output_blocked: turts_mod_api::RefundRatio {
                    numerator: 0,
                    denominator: 1,
                },
                destroyed: turts_mod_api::RefundRatio::ZERO,
            },
        },
    );
    acquired
        .0
        .presentation
        .interface
        .as_mut()
        .unwrap()
        .alerts
        .push(AlertDef {
            id: "production_done_test".into(),
            trigger: AlertTrigger::ProductionComplete,
            sound: None,
            text: "Production complete".into(),
            minimap_ping: false,
            cooldown: 1.0,
            priority: 1,
            coalesce_radius: 0.0,
            units: Vec::new(),
            kind: turts_mod_api::AlertKind::Event,
            clear_after: None,
            select_on_click: false,
            focus: None,
            sound_when: turts_mod_api::SoundWhen::Always,
        });
    let mut app = feedback_app(state, acquired);
    app.update();

    app.world_mut()
        .resource_mut::<ReplicaState>()
        .replica
        .as_mut()
        .unwrap()
        .economy
        .queues[0]
        .jobs
        .clear();
    app.update();
    assert!(
        app.world()
            .resource::<FrameFeedback>()
            .alerts
            .iter()
            .all(|alert| alert["id"] != "production_done_test"),
        "a removed running job without its declared output can be cancellation"
    );

    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        let replica = state.replica.as_mut().unwrap();
        replica.last_applied_tick = Some(3);
        replica.economy.queues[0]
            .jobs
            .push(turts_net_protocol::JobState {
                id: turts_net_protocol::JobId(2),
                recipe: "muster:test_recipe".into(),
                stage: turts_net_protocol::JobStage::Running,
                elapsed_ticks: 1,
                duration_ticks: 20,
                charged: Vec::new(),
                reserved: Vec::new(),
            });
    }
    app.update();
    {
        let mut state = app.world_mut().resource_mut::<ReplicaState>();
        let replica = state.replica.as_mut().unwrap();
        replica.last_applied_tick = Some(4);
        replica.economy.queues[0].jobs.clear();
        replica.units.insert(
            99,
            UnitReplica {
                content_id: output_unit.to_string(),
                owner_slot: 0,
                facing: None,
                samples: VecDeque::from([(4, turts_net_protocol::Pos { x: 3.0, y: 4.0 })]),
            },
        );
    }
    app.update();
    assert!(
        app.world()
            .resource::<FrameFeedback>()
            .alerts
            .iter()
            .any(|alert| alert["id"] == "production_done_test"),
        "a removed production job with its catalog output unit is completion evidence"
    );
}
