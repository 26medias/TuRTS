//! Differential checks against the pre-optimization owned reader.
use super::*;
use turts_mod_api::{SourceDef, VisionDef};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};

fn fixture(fog: bool) -> bevy_app::App {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/quarry");
    let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
    let mut catalog = archive.content().unwrap();
    let (file, mut map) = archive.maps().unwrap().remove(0);
    map.placements.retain(|p| p.unit == "quarry:walker");
    for unit in catalog.units.values_mut() {
        unit.vision = Some(VisionDef { radius: 9.0 });
    }
    if !fog {
        map.visibility = None;
    }
    let resolved =
        turts_mod_runtime::terrain::validate_terrain_map(&archive.id(), &file, &map, &catalog)
            .unwrap();
    let mut app = crate::headless_app(20);
    let staged = crate::stage_package_with_terrain(
        app.world(),
        catalog,
        SourceDef { file, value: map },
        20,
        ContentId::parse("quarry:field").unwrap(),
        resolved,
        Some(super::super::PrivateTerrainSeed::new(77)),
    )
    .unwrap();
    crate::install_package(app.world_mut(), staged);
    crate::visibility::initialize_context(app.world_mut(), 91, &[0, 1]).unwrap();
    crate::initialize_completed_world(app.world_mut());
    app
}
fn remove_owner_sight(app: &mut bevy_app::App) {
    let own: Vec<_> = crate::snapshot(app.world_mut())
        .into_iter()
        .filter(|u| u.owner_slot == 0)
        .map(|u| u.unit_id)
        .collect();
    for (seq, unit_id) in own.into_iter().enumerate() {
        app.world_mut()
            .resource_mut::<crate::CommandQueue>()
            .0
            .push_back(crate::Command {
                slot: crate::SlotId(0),
                seq: seq as u32 + 1,
                kind: crate::CommandKind::Remove { unit_id },
            });
    }
    app.update();
    assert!(
        app.world()
            .resource::<crate::CommandResults>()
            .0
            .iter()
            .all(|r| r.outcome.is_ok())
    );
}
fn compare(phase: &TerrainPhase) {
    for viewer in [0, 1, turts_mod_api::SLOT_COUNT, u8::MAX] {
        for y in 0..=phase.grid.rows {
            for x in 0..=phase.grid.columns {
                let cell = TerrainCell { x, y };
                let expected = legacy_surface(phase, viewer, cell);
                assert_eq!(phase.surface(viewer, cell), expected);
                let borrowed = phase.surface_borrowed(viewer, cell);
                assert_eq!(
                    borrowed.map(TerrainSurfaceObservationRef::into_owned),
                    expected
                );
                if let Some(material) = borrowed.and_then(|r| r.material) {
                    assert!(
                        phase
                            .materials()
                            .iter()
                            .any(|canonical| std::ptr::eq(material, canonical)),
                        "borrow must retain canonical identity"
                    );
                }
            }
        }
    }
}
fn copied(phase: &TerrainPhase) -> TerrainPhase {
    TerrainPhase {
        grid: phase.grid,
        chunk_width: phase.chunk_width,
        chunk_columns: phase.chunk_columns,
        surface: phase.surface.clone(),
        memory: phase.memory.clone(),
        coverage: phase.coverage.clone(),
        initial: phase.initial.clone(),
        initial_ranges: phase.initial_ranges.clone(),
        materials: phase.materials.clone(),
        ground: phase.ground.clone(),
        work: phase.work.clone(),
        tick: phase.tick,
        phase: phase.phase,
        permissions: phase.permissions.clone(),
    }
}
#[test]
fn borrowed_surfaces_match_legacy_current_stale_unknown_and_retained_phases() {
    for fog in [false, true] {
        let mut app = fixture(fog);
        let retained = phase(app.world(), ObservationPhase::Completed).unwrap();
        compare(&retained);
        let mut known = Vec::new();
        let mut unknown = 0;
        for y in 0..retained.grid.rows {
            for x in 0..retained.grid.columns {
                let cell = TerrainCell { x, y };
                if let Some(row) = retained.surface_borrowed(0, cell) {
                    assert!(row.current);
                    known.push(cell);
                } else {
                    unknown += 1;
                }
            }
        }
        assert!(!known.is_empty());
        if fog {
            assert!(unknown > 0);
        }
        let cell = known[0];
        let held = retained.surface_borrowed(0, cell).unwrap();
        let expected = held.into_owned();
        remove_owner_sight(&mut app);
        let later = phase(app.world(), ObservationPhase::Completed).unwrap();
        compare(&later);
        let after = later.surface_borrowed(0, cell).unwrap();
        assert_eq!(after.current, !fog);
        assert_eq!(
            held.into_owned(),
            expected,
            "retained borrow changed after a later phase"
        );
        assert_eq!(legacy_surface(&retained, 0, cell), Some(expected));
        if fog {
            assert!(
                known
                    .iter()
                    .all(|cell| !later.surface_borrowed(0, *cell).unwrap().current)
            );
        }
        // A retained phase owns its dictionary/history independently of the live app.
        drop(later);
        drop(app);
        assert_eq!(held.cell, cell);
        compare(&retained);
    }
}
#[test]
fn borrowed_surfaces_preserve_invalid_material_keys_and_bedrock_facts() {
    let app = fixture(false);
    let source = phase(app.world(), ObservationPhase::Completed).unwrap();
    let cell = TerrainCell { x: 0, y: 0 };
    let valid = source.surface(0, cell).unwrap();
    assert!(valid.material.is_some());
    let mut malformed = copied(&source);
    malformed.materials = Arc::from([]);
    let missing = malformed.surface_borrowed(0, cell).unwrap();
    assert!(missing.material.is_none());
    assert_eq!(
        missing.provenance, valid.provenance,
        "invalid material key must not silently clear provenance"
    );
    compare(&malformed);
    let mut bedrock = copied(&source);
    let (chunk, offset) = bedrock.location(cell).unwrap();
    let mut rows = (**bedrock.memory[&0].get(chunk).unwrap()).clone();
    let row = rows.rows.as_mut().unwrap()[offset].as_mut().unwrap();
    row.elevation = bedrock.grid.floor;
    row.material = None;
    row.provenance = None;
    let table = bedrock.memory[&0].replaced(chunk, Arc::new(rows)).unwrap();
    bedrock.memory.insert(0, table);
    let fact = bedrock.surface_borrowed(0, cell).unwrap();
    assert_eq!(fact.elevation, bedrock.grid.floor);
    assert_eq!(fact.material, None);
    assert_eq!(fact.provenance, None);
    compare(&bedrock);
}

// Frozen original body: this oracle does not call either new API.
fn legacy_surface(
    phase: &TerrainPhase,
    viewer: u8,
    cell: TerrainCell,
) -> Option<TerrainSurfaceObservation> {
    if viewer >= turts_mod_api::SLOT_COUNT {
        return None;
    }
    let (chunk, offset) = phase.location(cell)?;
    let row = phase
        .memory
        .get(&viewer)?
        .get(chunk)?
        .rows
        .as_ref()?
        .get(offset)?
        .as_ref()?;
    let material = row
        .material
        .and_then(|key| phase.materials.get(key as usize))
        .cloned();
    let current = super::super::history::seen(
        phase.permissions.as_deref(),
        phase.coverage.as_deref(),
        viewer,
        phase.grid,
        Cell {
            x: cell.x,
            y: cell.y,
        },
    );
    Some(TerrainSurfaceObservation {
        cell,
        elevation: row.elevation,
        material,
        provenance: row.provenance,
        last_seen_tick: row.last_seen_tick,
        current,
    })
}
