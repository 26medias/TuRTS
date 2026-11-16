//! Actual renamed packages must resolve through the normal verified archive path.
use std::path::PathBuf;
use turts_client::content::ClientContent;
use turts_mod_api::{ContentId, TerrainOutputDestination};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};

#[test]
fn two_terrain_packages_validate_assets_geometry_and_explicit_tool_bindings() {
    for (name, columns, rows, destination) in [
        ("quarry", 32, 24, TerrainOutputDestination::Inventory),
        (
            "terrain_stockyard",
            24,
            16,
            TerrainOutputDestination::SlotBalances,
        ),
    ] {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../mods")
            .join(name);
        let bytes = pack_dir(&root).unwrap();
        let archive = PackageArchive::parse(&bytes).unwrap();
        turts_pack::check_package(&archive).unwrap_or_else(|error| panic!("{name}: {error}"));
        let content = ClientContent::from_archives(&[archive])
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let layout = content.terrain.as_ref().unwrap();
        assert_eq!((layout.grid.columns, layout.grid.rows), (columns, rows));
        assert_eq!(content.catalog.terrain_materials.len(), 2);
        assert_eq!(content.catalog.terrain_tools.len(), 3);
        assert!(
            content
                .catalog
                .terrain_tools
                .values()
                .all(|tool| tool.output_destination == destination)
        );
        let order = ContentId::parse(&format!("{name}:order_work")).unwrap();
        assert_eq!(
            content.catalog.orders[&order].behaviour,
            turts_mod_api::Behaviour::TerrainWork
        );
        // The archive can contain public generation rules but no realized columns/seed.
        assert!(!root.join("columns.ron").exists());
        assert!(!root.join("seed.ron").exists());
    }
}

#[test]
fn terrain_and_resource_snapshot_activation_is_atomic_on_invalid_account() {
    use bevy::platform::time::Instant;
    use turts_client::replica::{Replica, ReplicaContext};
    use turts_mod_api::{
        KnowledgeBasis, KnowledgeSnapshot, ObservationPhase, TerrainCell, TerrainProvenance,
        VisibilityGrid, WorldPos,
    };
    use turts_net_protocol::{visibility::*, *};
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/quarry");
    let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
    let content = ClientContent::from_archives(&[archive]).unwrap();
    let materials = content
        .catalog
        .terrain_materials
        .keys()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let soil = TerrainMaterialId(
        materials
            .iter()
            .position(|id| id == "quarry:soil_layer")
            .unwrap() as u32,
    );
    let basis = KnowledgeBasis {
        match_id: 12,
        map: ContentId::parse("quarry:field").unwrap(),
        tick: 10,
        phase: ObservationPhase::Completed,
    };
    let knowledge = KnowledgeSnapshot {
        basis: basis.clone(),
        viewer: 0,
        grid: VisibilityGrid {
            size: WorldPos { x: 32., y: 24. },
            cell_size: 2.,
            columns: 16,
            rows: 12,
        },
        observed: vec![255; 24],
        explored: vec![255; 24],
        remembered_units: vec![],
        remembered_nodes: vec![],
        retention_truncated: false,
    };
    let mut state = Snapshot {
        tick: 10,
        units: vec![UnitSnapshot {
            unit_id: UnitId(1),
            content_id: "quarry:walker".into(),
            owner_slot: SlotId(0),
            pos: Pos { x: 13., y: 8. },
            facing: None,
            site: false,
        }],
        ..Default::default()
    };
    state.economy.inventories.push(InventoryState {
        unit: UnitId(1),
        port: InventoryPort::Main,
        items: vec![BalanceState {
            resource: "quarry:soil".into(),
            on_hand: 0,
            reserved: 0,
        }],
    });
    let cell = TerrainCell { x: 15, y: 8 };
    state.records.terrain = Some(TerrainSnapshot {
        materials,
        surfaces: vec![TerrainSurfaceRecord {
            cell,
            elevation: 4,
            material: Some(soil),
            provenance: Some(TerrainProvenance::Natural),
            observed_tick: 10,
            current: true,
        }],
        anchors: vec![TerrainGroundAnchor {
            unit: UnitId(1),
            elevation: 0,
            observed_tick: 10,
            current: true,
        }],
        work: vec![],
    });
    let mut replica = Replica::new(20);
    replica.set_context(ReplicaContext::new(content, 0));
    let stamp = StateTransactionStamp {
        basis,
        transaction_id: 1,
        parent_transaction_id: None,
    };
    let mut stale_contact = state.clone();
    stale_contact.records.terrain.as_mut().unwrap().anchors[0].current = false;
    assert!(
        !replica.activate_transaction(
            &stamp,
            &StateTransactionPayload::Snapshot {
                state: Box::new(stale_contact),
                knowledge: Some(knowledge.clone()),
                event_units: vec![]
            },
            Instant::now()
        ),
        "remembered contact cannot ground a live unit"
    );
    assert!(replica.terrain.is_none());
    let event = EventUnitObservation {
        basis: stamp.basis.clone(),
        unit: UnitSnapshot {
            unit_id: UnitId(2),
            content_id: "quarry:walker".into(),
            owner_slot: SlotId(1),
            pos: Pos { x: 15.5, y: 8. },
            facing: None,
            site: false,
        },
        ground_elevation: None,
    };
    assert!(
        !replica.activate_transaction(
            &stamp,
            &StateTransactionPayload::Snapshot {
                state: Box::new(state.clone()),
                knowledge: Some(knowledge.clone()),
                event_units: vec![event.clone()]
            },
            Instant::now()
        ),
        "opted-in event requires a captured contact"
    );
    assert!(replica.terrain.is_none());
    let mut event = event;
    event.ground_elevation = Some(9);
    assert!(
        !replica.activate_transaction(
            &stamp,
            &StateTransactionPayload::Snapshot {
                state: Box::new(state.clone()),
                knowledge: Some(knowledge.clone()),
                event_units: vec![event.clone()]
            },
            Instant::now()
        ),
        "out of bounds captured contact"
    );
    event.ground_elevation = Some(4);
    assert!(replica.activate_transaction(
        &stamp,
        &StateTransactionPayload::Snapshot {
            state: Box::new(state.clone()),
            knowledge: Some(knowledge.clone()),
            event_units: vec![event]
        },
        Instant::now()
    ));
    assert!(!replica.units.contains_key(&2));
    assert!(
        !replica.terrain.as_ref().unwrap().anchors.contains_key(&2),
        "event contact must not require a persistent anchor"
    );
    let old = replica.terrain.as_ref().unwrap().geometry.clone();
    state.tick = 11;
    state.economy.inventories[0].items[0].resource = "quarry:missing".into();
    state.economy.inventories[0].items[0].on_hand = 1;
    state.records.terrain.as_mut().unwrap().surfaces[0].elevation = 3;
    state.records.terrain.as_mut().unwrap().surfaces[0].observed_tick = 11;
    let mut next = stamp;
    next.basis.tick = 11;
    next.transaction_id = 2;
    let mut knowledge = knowledge;
    knowledge.basis = next.basis.clone();
    assert!(!replica.activate_transaction(
        &next,
        &StateTransactionPayload::Snapshot {
            state: Box::new(state.clone()),
            knowledge: Some(knowledge.clone()),
            event_units: vec![]
        },
        Instant::now()
    ));
    assert_eq!(replica.terrain.as_ref().unwrap().elevation(cell), Some(4));
    assert_eq!(replica.economy.inventories[0].items[0].on_hand, 0);
    assert!(std::sync::Arc::ptr_eq(
        &old,
        &replica.terrain.as_ref().unwrap().geometry
    ));
    state.economy.inventories[0].items[0].resource = "quarry:soil".into();
    assert!(replica.activate_transaction(
        &next,
        &StateTransactionPayload::Snapshot {
            state: Box::new(state),
            knowledge: Some(knowledge),
            event_units: vec![]
        },
        Instant::now()
    ));
    assert_eq!(replica.terrain.as_ref().unwrap().elevation(cell), Some(3));
    assert_eq!(replica.economy.inventories[0].items[0].on_hand, 1);
    assert_eq!(old.elevation(cell), Some(4));
}
