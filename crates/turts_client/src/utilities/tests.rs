//! M6 client tests: the scene data, the panel text, the terrain fade and the route list.

use super::{
    fade::{self, FADE_ALPHA},
    panel,
    scene::{self, PortState},
};
use crate::{
    content::ClientContent,
    core_adapter::CoreRuntime,
    replica::{Replica, ReplicaContext, SnapshotOutcome},
    terrain::{
        render::TerrainTile,
        store::{Authority, Layout, PermittedTerrain},
    },
};
use bevy::{platform::time::Instant, prelude::*};
use std::{collections::BTreeMap, sync::Arc};
use turts_interaction::utility::{Hover, HoverTarget, Output, RoutePreview, SnapKind};
use turts_mod_api::{
    ContentId, KnowledgeBasis, ObservationPhase, ResolvedUtility, TerrainCell, TerrainDef,
    TerrainGenerationDef, UtilityAppearance, UtilityDepth, UtilityExposure, UtilityLabels,
    UtilitySymbol, WorldPos,
};
use turts_net_protocol::{
    Pos, SlotId, Snapshot, SnapshotRecords, TerrainGroundAnchor, TerrainSnapshot,
    TerrainSurfaceRecord, TerrainWorkRecord, UnitId, UnitSnapshot, UtilityEndRecord,
    UtilityNetworkRecord, UtilityNodeRecord, UtilitySegmentRecord, UtilitySegmentState,
    UtilityView,
};

pub(crate) const LINE: &str = "skirmish:power_line";
const BASE: &str = "skirmish:base";
pub(crate) const ORDER: &str = "skirmish:order_route";

fn utility() -> ResolvedUtility {
    ResolvedUtility {
        id: ContentId::parse(LINE).unwrap(),
        name: "Power line".into(),
        carries: ContentId::parse("skirmish:credits").unwrap(),
        ports: BTreeMap::from([(ContentId::parse(BASE).unwrap(), WorldPos { x: 0.0, y: 1.0 })]),
        depth: UtilityDepth {
            default: 2,
            min: 1,
            max: 4,
        },
        reach: 20.0,
        max_segment_length: 16.0,
        segment_cost: vec![],
        exposure: UtilityExposure::Disconnect,
        appearance: UtilityAppearance {
            colour: "#ffcc00".into(),
            exposed_colour: "#ff4400".into(),
            junction: UtilitySymbol::Diamond,
            labels: UtilityLabels {
                powered: "Powered".into(),
                short: "No power".into(),
                unconnected: "Unconnected".into(),
                exposed: "Exposed".into(),
            },
        },
    }
}

pub(crate) fn content() -> ClientContent {
    let mut content = ClientContent {
        catalog: crate::repair_test_catalog::catalog(),
        ..ClientContent::default()
    };
    let utility = utility();
    content
        .catalog
        .utilities
        .insert(utility.id.clone(), utility);
    let mut order = crate::repair_test_catalog::catalog()
        .orders
        .values()
        .next()
        .unwrap()
        .clone();
    order.id = ORDER.into();
    order.name = "route".into();
    order.behaviour = turts_mod_api::Behaviour::UtilityRoute;
    order.target = turts_mod_api::TargetKind::UtilityRoute;
    order.units = vec!["skirmish:worker".into()];
    order.card = None;
    content
        .catalog
        .orders
        .insert(ContentId::parse(ORDER).unwrap(), order);
    content
}

fn node(id: u32, x: f32, y: f32) -> UtilityNodeRecord {
    UtilityNodeRecord {
        id,
        utility: LINE.into(),
        pos: Pos { x, y },
    }
}

fn segment(
    id: u32,
    a: UtilityEndRecord,
    b: UtilityEndRecord,
    state: UtilitySegmentState,
) -> UtilitySegmentRecord {
    UtilitySegmentRecord {
        id,
        utility: LINE.into(),
        a,
        b,
        elevation: -2,
        depth: 2,
        state,
    }
}

/// The base (unit 1, port at (10, 11)) joins node 1 at (14, 11); node 1 joins node 2 at (20, 11).
pub(crate) fn view(second: UtilitySegmentState, short: bool) -> UtilityView {
    UtilityView {
        nodes: vec![node(1, 14.0, 11.0), node(2, 20.0, 11.0)],
        segments: vec![
            segment(
                1,
                UtilityEndRecord::Port(UnitId(1)),
                UtilityEndRecord::Node(1),
                UtilitySegmentState::Conducting,
            ),
            segment(
                2,
                UtilityEndRecord::Node(1),
                UtilityEndRecord::Node(2),
                second,
            ),
        ],
        networks: vec![UtilityNetworkRecord {
            utility: LINE.into(),
            units: vec![UnitId(1)],
            nodes: vec![1, 2],
            provided: 10,
            consumed: if short { 12 } else { 6 },
            short,
        }],
    }
}

pub(crate) fn replica(content: &ClientContent, view: UtilityView) -> Replica {
    let mut replica = Replica::new(20);
    replica.set_context(ReplicaContext::new(content.clone(), 0));
    let unit = |id, name: &str, x, y| UnitSnapshot {
        unit_id: UnitId(id),
        content_id: format!("skirmish:{name}"),
        owner_slot: SlotId(0),
        pos: Pos { x, y },
        facing: None,
        site: false,
    };
    let snapshot = Snapshot {
        tick: 1,
        units: vec![unit(1, "base", 10.0, 10.0), unit(2, "worker", 5.0, 5.0)],
        records: SnapshotRecords {
            utilities: Some(view),
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(
        replica.apply_snapshot(&snapshot, Instant::now()),
        SnapshotOutcome::Applied { replayed: 0 }
    );
    replica
}

struct Public;
impl Authority for Public {
    fn surface(&self, _: TerrainCell, _: u32, _: bool) -> bool {
        true
    }
    fn anchor(&self, _: &TerrainGroundAnchor) -> bool {
        true
    }
    fn required_anchors(&self) -> impl Iterator<Item = u32> {
        std::iter::empty()
    }
    fn work(&self, _: &TerrainWorkRecord) -> bool {
        true
    }
}

/// One cell is 1 world unit wide; one elevation step is 0.25 world units high.
fn terrain(rows: Vec<TerrainSurfaceRecord>) -> PermittedTerrain {
    let layout = Arc::new(
        Layout::new(
            ContentId::parse("test:map").unwrap(),
            &TerrainDef {
                length_scale: 4,
                cell_width: 4,
                elevation_step: 1,
                columns: 32,
                rows: 32,
                chunk_width: 4,
                initial_elevation: 2,
                initial_heights: vec![],
                floor: 0,
                ceiling: 8,
                generation: TerrainGenerationDef::Version1 { bands: vec![] },
            },
        )
        .unwrap(),
    );
    PermittedTerrain::stage_snapshot(
        layout,
        &KnowledgeBasis {
            match_id: 7,
            map: ContentId::parse("test:map").unwrap(),
            tick: 1,
            phase: ObservationPhase::Completed,
        },
        &["test:soil".into()],
        &TerrainSnapshot {
            materials: vec!["test:soil".into()],
            surfaces: rows,
            ..Default::default()
        },
        &Public,
        None,
    )
    .unwrap()
}

fn surface(x: u32, y: u32, elevation: i32, current: bool) -> TerrainSurfaceRecord {
    TerrainSurfaceRecord {
        cell: TerrainCell { x, y },
        elevation,
        material: Some(turts_net_protocol::TerrainMaterialId(0)),
        provenance: Some(turts_mod_api::TerrainProvenance::Natural),
        observed_tick: 1,
        current,
    }
}

#[test]
fn the_scene_holds_only_the_rows_of_the_view_with_their_state_colours() {
    let content = content();
    let replica = replica(&content, view(UtilitySegmentState::Disconnected, false));
    let scene = scene::build(&replica, &content);
    assert_eq!(scene.segments.len(), 2);
    assert_eq!(scene.junctions.len(), 2);
    assert_eq!(scene.ports.len(), 1);
    let first = &scene.segments[0];
    assert_eq!(first.colour, scene::parse("#ffcc00"));
    assert!(!first.dashed());
    let second = &scene.segments[1];
    assert_eq!(
        second.colour,
        scene::parse("#ff4400"),
        "disconnected: the exposed colour"
    );
    assert!(second.dashed(), "a disconnected segment is dashed");
    // A flat map draws at height 0; the port end is at the port point (10, 11).
    assert_eq!((first.a.x, first.a.y, first.a.z), (10.0, 0.0, 11.0));
    assert_eq!(scene.junctions[0].symbol, UtilitySymbol::Diamond);
    let exposed = scene::build(
        &replica_with(&content, UtilitySegmentState::Exposed),
        &content,
    );
    assert_eq!(exposed.segments[1].colour, scene::parse("#ff4400"));
    assert!(
        !exposed.segments[1].dashed(),
        "an exposed segment that conducts is not dashed"
    );
}

fn replica_with(content: &ClientContent, second: UtilitySegmentState) -> Replica {
    replica(content, view(second, false))
}

#[test]
fn a_port_marker_is_green_red_or_grey() {
    let content = content();
    let supplied = scene::build(
        &replica(&content, view(UtilitySegmentState::Conducting, false)),
        &content,
    );
    assert_eq!(supplied.ports[0].state, PortState::Supplied);
    let short = scene::build(
        &replica(&content, view(UtilitySegmentState::Conducting, true)),
        &content,
    );
    assert_eq!(short.ports[0].state, PortState::Short);
    let mut alone = view(UtilitySegmentState::Conducting, false);
    alone.segments.clear();
    alone.nodes.clear();
    alone.networks[0].nodes.clear();
    let grey = scene::build(&replica(&content, alone), &content);
    assert_eq!(
        grey.ports[0].state,
        PortState::Unconnected,
        "no segment: grey"
    );
    let port = &grey.ports[0];
    assert_eq!((port.ground.x, port.ground.z), (10.0, 11.0));
}

#[test]
fn segments_are_drawn_at_the_elevation_times_the_step_height() {
    let content = content();
    let mut replica = replica(&content, view(UtilitySegmentState::Conducting, false));
    replica.terrain = Some(terrain(vec![]));
    let scene = scene::build(&replica, &content);
    // Elevation -2 at 0.25 world units a step.
    assert!((scene.segments[0].a.y - (-0.5)).abs() < 1e-6);
}

#[test]
fn an_empty_replica_view_draws_nothing() {
    let content = content();
    let mut replica = replica(&content, view(UtilitySegmentState::Conducting, false));
    replica.utilities = None;
    assert_eq!(scene::build(&replica, &content), scene::Scene::default());
}

fn hover_segment(id: u32, cell: Option<TerrainCell>) -> Hover {
    Hover {
        target: HoverTarget::Segment(id),
        cell,
    }
}

#[test]
fn hover_shows_name_label_network_depth_elevation_and_cover() {
    let content = content();
    let replica = replica(&content, view(UtilitySegmentState::Conducting, false));
    let view = replica.utilities.clone().unwrap();
    let text = panel::hover_text(&replica, &content, &view, &hover_segment(1, None));
    for expected in [
        "Power line",
        "Powered",
        "provided 10, consumed 6",
        "Depth 2, elevation -2",
        // No terrain: the surface is 0, so the cover is 2.
        "Cover 2",
    ] {
        assert!(text.contains(expected), "{text:?} holds {expected:?}");
    }
    let short = replica_with_short(&content);
    let text = panel::hover_text(
        &short,
        &content,
        short.utilities.as_ref().unwrap(),
        &hover_segment(1, None),
    );
    assert!(text.contains("No power"), "{text:?}");
}

fn replica_with_short(content: &ClientContent) -> Replica {
    replica(content, view(UtilitySegmentState::Conducting, true))
}

#[test]
fn a_disconnected_segment_reads_exposed_with_no_network() {
    let content = content();
    let replica = replica(&content, view(UtilitySegmentState::Disconnected, false));
    let view = replica.utilities.clone().unwrap();
    let text = panel::hover_text(&replica, &content, &view, &hover_segment(2, None));
    assert!(text.contains("Exposed"), "{text:?}");
    assert!(text.contains("not connected"), "{text:?}");
}

#[test]
fn cover_comes_from_permitted_terrain_or_reads_unknown() {
    let content = content();
    let mut replica = replica(&content, view(UtilitySegmentState::Conducting, false));
    replica.terrain = Some(terrain(vec![
        surface(3, 4, 5, true),
        surface(6, 4, 5, false),
    ]));
    let view = replica.utilities.clone().unwrap();
    let text = |cell| panel::hover_text(&replica, &content, &view, &hover_segment(1, cell));
    // Surface 5 minus the line at -2: cover 7.
    assert!(text(Some(TerrainCell { x: 3, y: 4 })).contains("Cover 7"));
    // A surface row that is not current is shown as last seen.
    assert!(text(Some(TerrainCell { x: 6, y: 4 })).contains("Cover 7 (last seen)"));
    // No surface row for the cell: the client does not know.
    assert!(text(Some(TerrainCell { x: 9, y: 9 })).contains("Cover unknown"));
    assert!(text(None).contains("Cover unknown"));
}

#[test]
fn hover_on_a_port_and_a_node_uses_the_mod_labels() {
    let content = content();
    let replica = replica(&content, view(UtilitySegmentState::Conducting, false));
    let view = replica.utilities.clone().unwrap();
    let port = Hover {
        target: HoverTarget::Port(UnitId(1)),
        cell: None,
    };
    let text = panel::hover_text(&replica, &content, &view, &port);
    assert!(
        text.contains("Powered") && text.contains("port"),
        "{text:?}"
    );
    let node = Hover {
        target: HoverTarget::Node(2),
        cell: None,
    };
    assert!(panel::hover_text(&replica, &content, &view, &node).contains("junction"));
}

#[test]
fn route_text_gives_length_depth_cost_and_the_reason() {
    let route = RoutePreview {
        utility: LINE.into(),
        name: "Power line".into(),
        depth: 3,
        depth_min: 1,
        depth_max: 4,
        start: Some(Vec2::new(1.0, 1.0)),
        end: Some(Vec2::new(5.0, 1.0)),
        end_kind: Some(SnapKind::Free),
        length: Some(4.0),
        cost: vec![("Loam".into(), 2)],
        reason: Some(turts_interaction::utility::reason::TOO_LONG),
    };
    let text = panel::route_text(&route);
    for expected in ["Depth 3 (1 to 4)", "Length 4.0", "2 Loam", "too long"] {
        assert!(text.contains(expected), "{text:?} holds {expected:?}");
    }
    let first = RoutePreview {
        start: None,
        length: None,
        reason: None,
        ..route
    };
    assert!(panel::route_text(&first).contains("Click to set the start"));
}

#[test]
fn the_panel_is_empty_when_the_view_is_off() {
    let content = content();
    let replica = replica(&content, view(UtilitySegmentState::Conducting, false));
    assert_eq!(
        panel::panel_text(&replica, &content, &Output::default()),
        ""
    );
    let on = Output {
        view_on: true,
        ..Output::default()
    };
    assert!(!panel::panel_text(&replica, &content, &on).is_empty());
}

#[test]
fn the_bridge_lists_segments_ports_and_junctions_with_screen_points() {
    let content = content();
    let replica = replica(&content, view(UtilitySegmentState::Disconnected, true));
    let on = Output {
        view_on: true,
        ..Output::default()
    };
    // A fixed projection: window x = world x * 10, window y = world z * 10.
    let json = scene::bridge(&replica, &content, &on, &|point| {
        Some(Vec2::new(point.x * 10.0, point.z * 10.0))
    });
    assert_eq!(json["view_on"], true);
    assert_eq!(json["route_mode"], false);
    assert_eq!(json["segments"].as_array().unwrap().len(), 2);
    assert_eq!(json["segments"][1]["state"], "disconnected");
    assert_eq!(json["segments"][1]["dashed"], true);
    assert_eq!(
        json["segments"][0]["a_screen"],
        serde_json::json!([100.0, 110.0])
    );
    assert_eq!(json["ports"][0]["state"], "short");
    assert_eq!(
        json["ports"][0]["screen"],
        serde_json::json!([100.0, 110.0])
    );
    assert_eq!(json["junctions"].as_array().unwrap().len(), 2);
    assert_eq!(json["junctions"][0]["symbol"], "diamond");
    let off = scene::bridge(&replica, &content, &Output::default(), &|_| None);
    assert_eq!(off["view_on"], false);
    assert!(
        off["segments"].as_array().unwrap().is_empty(),
        "the view off lists nothing"
    );
}

fn fade_app() -> App {
    let mut app = App::new();
    app.insert_resource(Assets::<StandardMaterial>::default())
        .insert_resource(Assets::<crate::ground::GroundMaterial>::default())
        .init_resource::<fade::FadeState>()
        .insert_resource(CoreRuntime::default())
        .add_systems(Update, fade::fade_terrain);
    app
}

fn set_view(app: &mut App, on: bool) {
    app.world_mut()
        .resource_mut::<CoreRuntime>()
        .utility_output
        .view_on = on;
}

#[test]
fn the_view_swaps_tile_materials_to_one_blended_copy_and_back() {
    let mut app = fade_app();
    let original = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.5, 0.2),
            ..Default::default()
        });
    let tiles: Vec<_> = (0..3)
        .map(|_| {
            app.world_mut()
                .spawn((TerrainTile, MeshMaterial3d(original.clone())))
                .id()
        })
        .collect();
    // The ground source and any other mesh keep the original material.
    let source = app.world_mut().spawn(MeshMaterial3d(original.clone())).id();
    let handle = |app: &App, entity: Entity| {
        app.world()
            .entity(entity)
            .get::<MeshMaterial3d<StandardMaterial>>()
            .unwrap()
            .0
            .clone()
    };
    app.update();
    assert!(
        tiles.iter().all(|tile| handle(&app, *tile) == original),
        "view off: no swap"
    );
    set_view(&mut app, true);
    app.update();
    let copy = handle(&app, tiles[0]);
    assert_ne!(copy, original);
    assert!(
        tiles.iter().all(|tile| handle(&app, *tile) == copy),
        "the tiles share one copy"
    );
    assert_eq!(
        handle(&app, source),
        original,
        "the source entity is never swapped"
    );
    {
        let assets = app.world().resource::<Assets<StandardMaterial>>();
        let blended = assets.get(&copy).unwrap();
        assert_eq!(blended.alpha_mode, AlphaMode::Blend);
        assert!((blended.base_color.alpha() - FADE_ALPHA).abs() < 1e-6);
        let solid = assets.get(&original).unwrap();
        assert_eq!(
            solid.alpha_mode,
            AlphaMode::Opaque,
            "the shared original is not edited"
        );
        assert_eq!(solid.base_color.alpha(), 1.0);
    }
    // A second frame changes nothing.
    app.update();
    assert!(tiles.iter().all(|tile| handle(&app, *tile) == copy));
    set_view(&mut app, false);
    app.update();
    assert!(
        tiles.iter().all(|tile| handle(&app, *tile) == original),
        "the originals come back"
    );
    assert!(!app.world().resource::<fade::FadeState>().active());
}

#[test]
fn a_tile_spawned_while_the_view_is_on_is_faded_too() {
    let mut app = fade_app();
    let original = app
        .world_mut()
        .resource_mut::<Assets<StandardMaterial>>()
        .add(StandardMaterial::default());
    set_view(&mut app, true);
    app.update();
    let late = app
        .world_mut()
        .spawn((TerrainTile, MeshMaterial3d(original.clone())))
        .id();
    app.update();
    let now = app
        .world()
        .entity(late)
        .get::<MeshMaterial3d<StandardMaterial>>()
        .unwrap()
        .0
        .clone();
    assert_ne!(now, original);
}

#[test]
fn the_route_list_has_one_item_for_each_utility_when_the_order_lists_a_selected_unit() {
    use crate::widgets::{primitives::WidgetAction, sources};
    use turts_mod_api::{ItemField, Source};
    let content = content();
    let replica = replica(&content, view(UtilitySegmentState::Conducting, false));
    let groups = BTreeMap::new();
    let bindings = BTreeMap::new();
    let input_map = turts_mod_api::InputMapDef {
        actions: vec![],
        engine_overrides: vec![],
    };
    let read = |selection: &[u32]| {
        sources::read_source(
            &Source::UtilityRoutes {
                order: ORDER.into(),
            },
            sources::SourceContext {
                binding_labels: None,
                replica: &replica,
                content: &content,
                selection,
                owner_slot: 0,
                interface: None,
                interface_package: Some("skirmish"),
                control_groups: &groups,
                alerts: &[],
                bindings: &bindings,
                input_map: &input_map,
                guide: &[],
                guide_open: false,
                sound_on: false,
                last_notice: None,
                core: sources::CoreFeed::empty(),
            },
        )
    };
    // Unit 2 is a worker, which the order lists.
    let worker = read(&[2]);
    assert_eq!(worker.items.len(), 1);
    assert_eq!(
        worker.items[0].get(ItemField::Name),
        Some(&serde_json::json!("Power line"))
    );
    assert_eq!(
        worker.items[0].get(ItemField::Action),
        serde_json::to_value(WidgetAction::UtilityRoute {
            order: ORDER.into(),
            utility: LINE.into(),
        })
        .ok()
        .as_ref()
    );
    assert_eq!(
        worker.items[0].get(ItemField::Enabled),
        Some(&serde_json::json!(true))
    );
    // Unit 1 is the base: the order does not list it. Nothing is selected: no items.
    assert!(read(&[1]).items.is_empty());
    assert!(read(&[]).items.is_empty());
}
