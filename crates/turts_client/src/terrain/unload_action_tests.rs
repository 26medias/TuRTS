//! Exercise the generated public action and its real observer-to-Core dispatch.
use super::*;
use crate::{
    acquire::Acquired,
    commands::{CommandSendStatus, PendingCommand, PendingCommands},
    core_adapter::{CoreAdapterPlugin, CoreAdapterSet},
    ingest::{CommandResults, WelcomeInfo},
    present::{Presented, PresentedUnit},
    replica::{Replica, UnitReplica},
    terrain::store::{Authority, PermittedTerrain},
    widgets::primitives::{WidgetAction, WidgetButtonAction},
};
use std::collections::{HashMap, VecDeque};
use turts_mod_api::{
    KnowledgeBasis, ObservationPhase, TerrainCell, TerrainDesignation, TerrainRect,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
use turts_net_protocol::{
    Activity, OrderSummary, Pos, QueueMode, TerrainGroundAnchor, TerrainSnapshot,
    TerrainWorkRecord, UnitId, UnitOrderRecord,
};

struct OwnFacts;
impl Authority for OwnFacts {
    fn surface(&self, _: TerrainCell, _: u32, _: bool) -> bool {
        true
    }
    fn anchor(&self, row: &TerrainGroundAnchor) -> bool {
        row.current && [1, 2].contains(&row.unit.0)
    }
    fn required_anchors(&self) -> impl Iterator<Item = u32> {
        [1, 2].into_iter()
    }
    fn work(&self, row: &TerrainWorkRecord) -> bool {
        row.unit == UnitId(1)
    }
}
fn fixture(package: &str) -> (App, TerrainDesignation) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods");
    let archive = PackageArchive::parse(&pack_dir(&root.join(package)).unwrap()).unwrap();
    let mut content =
        crate::content::ClientContent::from_archives(std::slice::from_ref(&archive)).unwrap();
    let worker = if package == "quarry" {
        "walker"
    } else {
        "porter"
    };
    let tool = if package == "quarry" {
        "spade"
    } else {
        "scraper"
    };
    let worker_id = ContentId::parse(&format!("{package}:{worker}")).unwrap();
    let depot_id = ContentId::parse(&format!("{package}:depot")).unwrap();
    if package == "terrain_stockyard" {
        // Stockyard deliberately uses slot balances, so natural FullCargo cannot occur.
        // Add explicit test-only cargo/depot capabilities to exercise renamed key handling.
        // This is a client action regression, not Stockyard simulation/authoring evidence.
        content
            .catalog
            .units
            .get_mut(&worker_id)
            .unwrap()
            .terrain_cargo = Some(turts_mod_api::TerrainCargoDef {
            unload_per_second: 8,
            interaction_range: 1.0,
        });
        let inventory = turts_mod_api::ResolvedInventory {
            capacity: 4,
            accepts: vec![ContentId::parse("terrain_stockyard:loam").unwrap()],
        };
        content.catalog.units.get_mut(&worker_id).unwrap().inventory = Some(inventory.clone());
        let depot = content.catalog.units.get_mut(&depot_id).unwrap();
        depot.inventory = Some(inventory);
        depot.drop_off = Some(turts_mod_api::ResolvedDropOff {
            rules: vec![turts_mod_api::ResolvedDropOffRule {
                item: ContentId::parse("terrain_stockyard:loam").unwrap(),
                effect: turts_mod_api::ResolvedDropOffEffect::Store,
            }],
        });
    }
    let designation = TerrainDesignation {
        tool: format!("{package}:{tool}"),
        cells: TerrainRect {
            min: TerrainCell { x: 3, y: 3 },
            max: TerrainCell { x: 4, y: 4 },
        },
        target_elevation: 0,
    };
    let layout = content.terrain.clone().unwrap();
    let basis = KnowledgeBasis {
        match_id: 7,
        map: layout.map.clone(),
        tick: 1,
        phase: ObservationPhase::Completed,
    };
    let materials: Vec<_> = content
        .catalog
        .terrain_materials
        .keys()
        .map(ToString::to_string)
        .collect();
    let terrain = PermittedTerrain::stage_snapshot(
        layout,
        &basis,
        &materials,
        &TerrainSnapshot {
            materials: materials.clone(),
            surfaces: vec![],
            anchors: [1, 2]
                .into_iter()
                .map(|id| TerrainGroundAnchor {
                    unit: UnitId(id),
                    elevation: 0,
                    observed_tick: 1,
                    current: true,
                })
                .collect(),
            work: vec![TerrainWorkRecord {
                unit: UnitId(1),
                designation: designation.clone(),
                current_cell: Some(designation.cells.min),
                completed_work: 3,
                required_work: Some(10),
                wait: Some(TerrainWorkWait::FullCargo),
            }],
        },
        &OwnFacts,
        None,
    )
    .unwrap();
    let mut replica = Replica::new(20);
    let mut shown = Vec::new();
    for (id, kind, position) in [
        (1, worker_id.to_string(), Pos { x: 10.0, y: 8.0 }),
        (2, depot_id.to_string(), Pos { x: 4.0, y: 4.0 }),
    ] {
        replica.units.insert(
            id,
            UnitReplica {
                content_id: kind.clone(),
                owner_slot: 0,
                facing: None,
                samples: VecDeque::from([(1, position)]),
            },
        );
        shown.push(PresentedUnit {
            unit_id: id,
            content_id: kind,
            pos: position,
            lo_tick: 1,
            hi_tick: 1,
            fraction: 0.0,
        });
    }
    replica.unit_orders.insert(
        1,
        UnitOrderRecord {
            unit: UnitId(1),
            activity: Activity::Blocked,
            current: Some(OrderSummary {
                order: format!("{package}:order_work"),
                target: OrderTarget::TerrainWork(designation.clone()),
            }),
            queued: vec![],
            last_failure: None,
        },
    );
    replica.terrain = Some(terrain);
    replica.snapshot_tick = Some(1);
    replica.last_applied_tick = Some(1);
    let acquired = Acquired {
        lockfile_hash: String::new(),
        visuals: HashMap::new(),
        map_size: (64.0, 48.0),
        unit_names: HashMap::new(),
        content: content.clone(),
        presentation: Default::default(),
        verified_root_mod: None,
    };
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(CoreAdapterPlugin)
        .insert_resource(ReplicaState {
            replica: Some(replica),
            welcome: Some(WelcomeInfo {
                slot: 0,
                tick: 1,
                tick_hz: 20,
                match_id: 7,
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
            units: shown,
        })
        .init_resource::<PendingCommands>()
        .init_resource::<CommandSendStatus>()
        .init_resource::<CommandResults>()
        .init_resource::<crate::feedback::FrameFeedback>()
        .init_resource::<WorkPanel>()
        .add_systems(Update, panel.after(CoreAdapterSet));
    app.world_mut().resource_mut::<CoreInputQueue>().0.push(
        turts_interaction::InputEvent::UiAction {
            name: "ui_select:1".into(),
        },
    );
    app.finish();
    app.cleanup();
    app.update();
    (app, designation)
}
#[test]
fn full_cargo_widget_uses_qualified_catalog_key_and_dispatches_one_appended_unload() {
    for package in ["quarry", "terrain_stockyard"] {
        let (mut app, designation) = fixture(package);
        let (button, action) = {
            let mut query = app.world_mut().query::<(Entity, &WidgetButtonAction)>();
            let actions: Vec<_> = query
                .iter(app.world())
                .filter(|(_, a)| matches!(a.0, WidgetAction::UnloadResume { .. }))
                .map(|(entity, a)| (entity, a.0.clone()))
                .collect();
            assert_eq!(
                actions.len(),
                1,
                "{package}: actual FullCargo panel must offer one eligible depot action"
            );
            actions[0].clone()
        };
        let WidgetAction::UnloadResume {
            unload_order,
            work_order,
            unit,
            depot,
            ..
        } = action
        else {
            unreachable!()
        };
        assert_eq!(
            unload_order,
            format!("{package}:order_unload"),
            "catalog definition id is local; public action must use its qualified key"
        );
        assert_eq!(work_order, format!("{package}:order_work"));
        assert_eq!((unit, depot), (1, 2));
        let original = app
            .world()
            .resource::<ReplicaState>()
            .replica
            .as_ref()
            .unwrap()
            .unit_orders[&1]
            .clone();
        app.world_mut().trigger(Activate { entity: button });
        assert_eq!(
            app.world().resource::<CoreInputQueue>().0.len(),
            1,
            "real panel observer must enqueue its ordinary Resume input"
        );
        app.update();
        let pending = &app.world().resource::<PendingCommands>().0;
        assert_eq!(
            pending.len(),
            1,
            "{package}: generated qualified action must reach Core's ordinary send path"
        );
        let PendingCommand::Order { order, .. } = &pending[0] else {
            panic!("expected normal mod-owned order envelope")
        };
        assert_eq!(order.order, format!("{package}:order_unload"));
        assert_eq!(order.units, vec![UnitId(1)]);
        assert_eq!(order.target, OrderTarget::Unit(UnitId(2)));
        assert_eq!(order.queue, QueueMode::Append);
        let replica = app
            .world()
            .resource::<ReplicaState>()
            .replica
            .as_ref()
            .unwrap();
        assert_eq!(
            replica.unit_orders[&1], original,
            "shortcut must not recreate the original work locally"
        );
        assert_eq!(
            replica.terrain.as_ref().unwrap().work[&1].designation,
            designation
        );
    }
}
