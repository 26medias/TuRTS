//! API16 authored visibility validation. Simulation acceptance lives separately.
use turts_mod_api::{
    MapDef, MapSize, PackageId, SourceDef, UnitDef, VisibilityDef, VisionDef, VisionSharing,
};
use turts_mod_runtime::{content::validate_content, visibility::validate_visibility};

fn origin() -> PackageId {
    PackageId::parse("sight_fixture").unwrap()
}
fn map() -> MapDef {
    MapDef {
        id: "plain".into(),
        visibility: Some(VisibilityDef {
            cell_size: 2.0,
            sharing: VisionSharing::OwnSlot,
            memory_limit: None,
        }),
        size: MapSize { x: 5.0, y: 7.0 },
        placements: vec![],
        nodes: vec![],
        slot_economy: vec![],
        belts: vec![],
        ground: None,
        environment: None,
    }
}

#[test]
fn grid_dimensions_clip_partial_edge_cells_and_preserve_allvisible_absence() {
    let mut map = map();
    for sharing in [VisionSharing::OwnSlot, VisionSharing::Team] {
        map.visibility.as_mut().unwrap().sharing = sharing;
        let grid = validate_visibility(&origin(), "maps/plain.ron", &map)
            .unwrap()
            .unwrap();
        assert_eq!((grid.columns, grid.rows), (3, 4));
        assert_eq!((grid.size.x, grid.size.y, grid.cell_size), (5.0, 7.0, 2.0));
        assert_eq!(
            map.visibility.as_ref().unwrap().effective_memory_limit(),
            65_536
        );
    }
    map.visibility = None;
    assert!(
        validate_visibility(&origin(), "maps/plain.ron", &map)
            .unwrap()
            .is_none()
    );
}

#[test]
fn invalid_grid_and_memory_limits_fail_with_the_authored_file_and_field() {
    for size in [
        0.0,
        -1.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(1),
    ] {
        let mut map = map();
        map.visibility.as_mut().unwrap().cell_size = size;
        let error = validate_visibility(&origin(), "maps/plain.ron", &map).unwrap_err();
        assert_eq!(error.file.as_deref(), Some("maps/plain.ron"));
        assert_eq!(error.field_path, "visibility.cell_size");
    }
    for limit in [0, 65_537, u32::MAX] {
        let mut map = map();
        map.visibility.as_mut().unwrap().memory_limit = Some(limit);
        assert_eq!(
            validate_visibility(&origin(), "maps/plain.ron", &map)
                .unwrap_err()
                .field_path,
            "visibility.memory_limit"
        );
    }
    let mut map = map();
    map.visibility.as_mut().unwrap().cell_size = 1.0;
    map.size = MapSize {
        x: 1024.0,
        y: 1024.0,
    };
    assert!(validate_visibility(&origin(), "maps/plain.ron", &map).is_ok());
    map.size.x = 1025.0;
    assert!(
        validate_visibility(&origin(), "maps/plain.ron", &map)
            .unwrap_err()
            .to_string()
            .contains("1048576")
    );
    map.size = MapSize {
        x: f32::MAX,
        y: f32::MAX,
    };
    assert!(validate_visibility(&origin(), "maps/plain.ron", &map).is_err());
}

#[test]
fn unit_sight_is_resolved_without_inventing_other_capabilities() {
    let unit: UnitDef = ron::from_str(
        r#"(
        id: "observer", name: "Observer", footprint: Circle(radius: 1.0),
        movement: None, inventory: None, gatherer: None, drop_off: None, producer: None
    )"#,
    )
    .unwrap();
    for radius in [1.0, f32::MAX] {
        let mut unit = unit.clone();
        unit.vision = Some(VisionDef { radius });
        let catalog = validate_content(
            &origin(),
            vec![],
            vec![SourceDef {
                file: "units/observer.ron".into(),
                value: unit,
            }],
            vec![],
            vec![],
        )
        .unwrap();
        let resolved = catalog.units.values().next().unwrap();
        assert_eq!(resolved.vision, Some(VisionDef { radius }));
        assert!(resolved.health.is_none() && resolved.movement.is_none());
    }
    for radius in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let mut unit = unit.clone();
        unit.vision = Some(VisionDef { radius });
        let error = validate_content(
            &origin(),
            vec![],
            vec![SourceDef {
                file: "units/observer.ron".into(),
                value: unit,
            }],
            vec![],
            vec![],
        )
        .unwrap_err();
        assert_eq!(error.file.as_deref(), Some("units/observer.ron"));
        assert_eq!(error.field_path, "vision.radius");
    }
}

#[test]
fn fog_map_identity_fits_each_bounded_transaction_frame() {
    let mut map = map();
    let prefix = origin().0.len() + 1;
    map.id = "a".repeat(turts_mod_api::MAX_VISIBILITY_MAP_ID_BYTES - prefix);
    assert!(validate_visibility(&origin(), "maps/plain.ron", &map).is_ok());
    map.id.push('a');
    let error = validate_visibility(&origin(), "maps/plain.ron", &map).unwrap_err();
    assert_eq!(error.field_path, "id");
    assert_eq!(error.file.as_deref(), Some("maps/plain.ron"));
    map.visibility = None;
    assert!(validate_visibility(&origin(), "maps/plain.ron", &map).is_ok());
}
