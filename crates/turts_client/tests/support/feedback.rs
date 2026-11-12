use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    time::Duration,
};

use bevy::{app::App, prelude::MinimalPlugins, time::TimeUpdateStrategy};
use turts_client::{
    acquire::{Acquired, PresentationContent},
    content::ClientContent,
    debug_feed::{AlertPingsFeed, AlertsFeed, AudioFeed, NoticesFeed},
    feedback::{FeedbackPlugin, FrameFeedback},
    ingest::{CommandResults, ReplicaState, WelcomeInfo},
    join::{AcquiredPackages, Phase},
    replica::{HealthTraceEntry, Replica, UnitReplica},
};
use turts_mod_api::{
    CancelRefunds, ContentId, Footprint, PackageId, PaymentPolicy, RefundRatio,
    content::{
        ResolvedCost, ResolvedInventory, ResolvedResearchStation, ResolvedTechnology, ResolvedUnit,
    },
};
use turts_net_protocol::{
    EconomyState, ExtractorRecord, ExtractorStatus, HealthRecord, InserterRecord, InserterStage,
    InserterWaitReason, InventoryPort, InventoryState, JobId, JobStage, QueueState, ResearchRecord,
    SlotId, UnitId, WeaponRecord, WeaponStatus,
};

pub fn interface() -> turts_mod_api::InterfaceDef {
    ron::from_str(include_str!(
        "../../../../tests/browser/fixtures/feedback/muster/interface/hud.ron"
    ))
    .expect("controlled feedback interface fixture parses")
}

pub fn alert_state() -> (ReplicaState, AcquiredPackages) {
    let package = PackageId("muster".into());
    let item = ContentId::new(&package, "ore_chunk");
    let technology = ContentId::new(&package, "test_research");
    let completed_technology = ContentId::new(&package, "completed_research");
    let station_id = ContentId::new(&package, "lab");
    let mut content = ClientContent::default();
    content.catalog.technologies.insert(
        technology.clone(),
        ResolvedTechnology {
            id: technology.clone(),
            name: "Test research".into(),
            prerequisites: Vec::new(),
            duration_ms: 1_000,
            costs: vec![ResolvedCost {
                resource: item.clone(),
                amount: 1,
                source: turts_mod_api::CostSource::ProducerInventory,
            }],
            payment: PaymentPolicy::ChargeOnQueue,
            cancel_refunds: CancelRefunds {
                queued: RefundRatio {
                    numerator: 0,
                    denominator: 1,
                },
                running: RefundRatio {
                    numerator: 0,
                    denominator: 1,
                },
                output_blocked: RefundRatio {
                    numerator: 0,
                    denominator: 1,
                },
                destroyed: turts_mod_api::RefundRatio::ZERO,
            },
            effects: Vec::new(),
        },
    );
    content.catalog.technologies.insert(
        completed_technology.clone(),
        ResolvedTechnology {
            id: completed_technology.clone(),
            name: "Completed research".into(),
            prerequisites: Vec::new(),
            duration_ms: 1_000,
            costs: vec![ResolvedCost {
                resource: item.clone(),
                amount: 100,
                source: turts_mod_api::CostSource::ProducerInventory,
            }],
            payment: PaymentPolicy::ChargeOnQueue,
            cancel_refunds: CancelRefunds {
                queued: RefundRatio {
                    numerator: 0,
                    denominator: 1,
                },
                running: RefundRatio {
                    numerator: 0,
                    denominator: 1,
                },
                output_blocked: RefundRatio {
                    numerator: 0,
                    denominator: 1,
                },
                destroyed: turts_mod_api::RefundRatio::ZERO,
            },
            effects: Vec::new(),
        },
    );
    content.catalog.units.insert(
        station_id.clone(),
        ResolvedUnit {
            id: station_id.clone(),
            name: "Research Station".into(),
            footprint: Footprint::Circle { radius: 1.0 },
            movement: None,
            inventory: Some(ResolvedInventory {
                capacity: 10,
                accepts: vec![item.clone()],
            }),
            gatherer: None,
            drop_off: None,
            producer: None,
            output_inventory: None,
            extractor: None,
            conveyor: None,
            inserter: None,
            machine: None,
            research: Some(ResolvedResearchStation {
                technologies: vec![technology, completed_technology],
                queue_capacity: 1,
            }),
            health: None,
            weapon: None,
            removal: None,
        },
    );

    let mut replica = Replica::new(20);
    replica.snapshot_tick = Some(1);
    replica.last_applied_tick = Some(1);
    for (id, content_id, owner_slot) in [
        (1, "muster:trooper", 0),
        (2, "muster:extractor", 0),
        (3, "muster:producer", 0),
        (4, "muster:trooper", 0),
        (5, "muster:inserter", 0),
        (6, "muster:lab", 0),
        (7, "muster:post", 0),
        (8, "muster:trooper", 1),
    ] {
        replica.units.insert(
            id,
            UnitReplica {
                content_id: content_id.into(),
                owner_slot,
                facing: None,
                samples: VecDeque::from([(
                    1,
                    turts_net_protocol::Pos {
                        x: id as f32,
                        y: 1.0,
                    },
                )]),
            },
        );
    }
    replica.healths = vec![HealthRecord {
        unit: UnitId(1),
        current: 50,
    }];
    replica.health_trace.insert(
        1,
        vec![HealthTraceEntry {
            tick: 1,
            current: 50,
        }],
    );
    replica.extractors = vec![ExtractorRecord {
        unit: UnitId(2),
        enabled: true,
        node: None,
        status: ExtractorStatus::Extracting,
    }];
    replica.weapons = vec![WeaponRecord {
        unit: UnitId(4),
        enabled: true,
        ready_tick: 0,
        damage: Ok(1),
        status: WeaponStatus::Ready,
        last_target: None,
    }];
    replica.inserters = vec![InserterRecord {
        unit: UnitId(5),
        enabled: true,
        stage: InserterStage::ToDestination,
        elapsed: 0,
        source: None,
        destination: None,
    }];
    replica.research = vec![ResearchRecord {
        slot: SlotId(0),
        completed: Vec::new(),
    }];
    replica.economy = EconomyState {
        balances: Vec::new(),
        inventories: vec![InventoryState {
            unit: UnitId(6),
            port: InventoryPort::Main,
            items: Vec::new(),
        }],
        orders: Vec::new(),
        queues: vec![QueueState {
            producer: UnitId(3),
            enabled: true,
            jobs: vec![turts_net_protocol::JobState {
                id: JobId(1),
                recipe: "muster:test_recipe".into(),
                stage: JobStage::Running,
                elapsed_ticks: 1,
                duration_ticks: 20,
                charged: Vec::new(),
                reserved: Vec::new(),
            }],
        }],
    };

    let state = ReplicaState {
        replica: Some(replica),
        welcome: Some(WelcomeInfo {
            slot: 0,
            tick: 1,
            tick_hz: 20,
            match_id: 1,
        }),
        content: Some(content.clone()),
        config: Default::default(),
        match_start: None,
    };
    let presentation = PresentationContent {
        interface: Some(interface()),
        interface_package: Some("muster".into()),
        ..Default::default()
    };
    let acquired = AcquiredPackages(Acquired {
        lockfile_hash: "test".into(),
        verified_root_mod: None,
        visuals: HashMap::new(),
        map_size: (64.0, 48.0),
        unit_names: HashMap::from([
            ("muster:trooper".into(), "Trooper".into()),
            ("muster:post".into(), "Shell Turret".into()),
        ]),
        content,
        presentation,
    });
    (state, acquired)
}

pub fn feedback_app(state: ReplicaState, acquired: AcquiredPackages) -> App {
    let phase = state
        .welcome
        .map_or(Phase::Joining, |welcome| Phase::Joined {
            slot: welcome.slot,
        });
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(FeedbackPlugin)
        .insert_resource(state)
        .insert_resource(phase)
        .insert_resource(acquired)
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs(1)))
        .init_resource::<FrameFeedback>()
        .init_resource::<NoticesFeed>()
        .init_resource::<AlertsFeed>()
        .init_resource::<AlertPingsFeed>()
        .init_resource::<AudioFeed>()
        .init_resource::<CommandResults>();
    app
}

// Shared fixture module: this helper is used by the alerts integration target,
// while sibling targets compile the module without needing every helper.
#[allow(dead_code)]
pub fn unit(content_id: &str, owner_slot: u8) -> UnitReplica {
    UnitReplica {
        content_id: content_id.into(),
        owner_slot,
        facing: None,
        samples: VecDeque::from([(1, turts_net_protocol::Pos { x: 9.0, y: 1.0 })]),
    }
}

// Shared fixture module: only the alerts integration target uses this setup.
#[allow(dead_code)]
pub fn force_all_alert_sources(state: &mut ReplicaState) {
    let replica = state.replica.as_mut().expect("test replica exists");
    replica.last_applied_tick = Some(2);
    replica.healths = vec![HealthRecord {
        unit: UnitId(1),
        current: 40,
    }];
    replica
        .health_trace
        .get_mut(&1)
        .unwrap()
        .push(HealthTraceEntry {
            tick: 2,
            current: 40,
        });
    replica.extractors[0].status = ExtractorStatus::Full;
    replica.weapons[0].status = WeaponStatus::OutOfAmmo;
    replica.inserters[0] = InserterRecord {
        unit: UnitId(5),
        enabled: true,
        stage: InserterStage::AtDestination(InserterWaitReason::DestinationFull),
        elapsed: 0,
        source: None,
        destination: None,
    };
    replica.economy.queues[0].jobs[0].stage = JobStage::OutputBlocked;
    replica.research[0].completed = vec!["muster:completed_research".into()];
    replica.economy.inventories[0].items = vec![turts_net_protocol::BalanceState {
        resource: "muster:ore_chunk".into(),
        on_hand: 1,
        reserved: 0,
    }];
}

// Shared fixture module: only the alerts integration target clears these feeds.
#[allow(dead_code)]
pub fn clear_feedback(app: &mut App) {
    app.world_mut()
        .resource_mut::<FrameFeedback>()
        .audio_started
        .clear();
    app.world_mut()
        .resource_mut::<FrameFeedback>()
        .alerts
        .clear();
    app.world_mut()
        .resource_mut::<FrameFeedback>()
        .notices
        .clear();
}

// Shared fixture module: only the notices integration target needs overrides.
#[allow(dead_code)]
pub fn reason_overrides() -> BTreeMap<String, String> {
    interface().reasons
}
