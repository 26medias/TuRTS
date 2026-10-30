//! T6 scheduled extraction smoke test through package boot and public views.

use std::collections::BTreeMap;

use bevy_app::FixedUpdate;
use turts_mod_api::{
    ContentCatalog, ContentId, ExtractorDef, Footprint, MapDef, MapSize, NodePlacement, Placement,
    ResolvedInventory, ResolvedNode, ResolvedUnit, ResourceDef, ResourceKind, SourceDef, WorldPos,
};
use turts_sim::{SlotId, headless_app, install_package, inventory_view, node_view, stage_package};

#[test]
fn extractor_runs_in_fixed_extract_set() {
    let ore = ContentId::parse("probe:ore").unwrap();
    let drill_id = ContentId::parse("probe:drill").unwrap();
    let node_id = ContentId::parse("probe:deposit").unwrap();
    let drill = ResolvedUnit {
        id: drill_id.clone(),
        name: "Drill".into(),
        footprint: Footprint::Circle { radius: 1.0 },
        movement: None,
        inventory: Some(ResolvedInventory {
            capacity: 10,
            accepts: vec![ore.clone()],
        }),
        gatherer: None,
        drop_off: None,
        producer: None,
        output_inventory: None,
        extractor: Some(ExtractorDef {
            per_second: 2,
            interaction_range: 1.0,
        }),
        conveyor: None,
        inserter: None,
        machine: None,
        research: None,
        health: None,
        weapon: None,
        removal: None,
    };
    let catalog = ContentCatalog {
        resources: BTreeMap::from([(
            ore.clone(),
            ResourceDef {
                id: "ore".into(),
                name: "Ore".into(),
                unit_name: "ore".into(),
                kind: ResourceKind::Item,
            },
        )]),
        units: BTreeMap::from([(drill_id, drill)]),
        nodes: BTreeMap::from([(
            node_id.clone(),
            ResolvedNode {
                id: node_id.clone(),
                name: "Deposit".into(),
                item: ore.clone(),
                radius: 1.0,
            },
        )]),
        production: BTreeMap::new(),
        recipes: BTreeMap::new(),
        technologies: BTreeMap::new(),
        orders: BTreeMap::new(),
        source_files: BTreeMap::new(),
    };
    let map = MapDef {
        id: "field".into(),
        size: MapSize { x: 32.0, y: 32.0 },
        placements: vec![Placement {
            unit: "probe:drill".into(),
            owner_slot: 0,
            pos: WorldPos { x: 12.0, y: 10.0 },
            contents: vec![],
            facing: None,
        }],
        nodes: vec![NodePlacement {
            node: "probe:deposit".into(),
            pos: WorldPos { x: 10.0, y: 10.0 },
            remaining: 10,
        }],
        slot_economy: vec![],
        belts: vec![],
    };
    let mut app = headless_app(20);
    let staged = stage_package(
        app.world(),
        catalog,
        SourceDef {
            file: "maps/field.ron".into(),
            value: map,
        },
        20,
    )
    .unwrap();
    install_package(app.world_mut(), staged);

    for _ in 0..10 {
        app.world_mut().run_schedule(FixedUpdate);
    }

    let inventory = inventory_view(app.world(), SlotId(0), 1).unwrap();
    assert_eq!(
        inventory
            .items
            .iter()
            .find(|item| item.resource == ore)
            .unwrap()
            .on_hand,
        1
    );
    assert_eq!(
        node_view(app.world(), turts_sim::NodeId(1))
            .unwrap()
            .remaining,
        9
    );
}
