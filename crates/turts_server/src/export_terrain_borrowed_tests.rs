//! Wire differential oracle frozen from the owned projection before optimization.
use super::*;
use turts_mod_api::{ContentId, SourceDef, VisionDef};
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
    let mut app = turts_sim::headless_app(20);
    let staged = turts_sim::stage_package_with_terrain(
        app.world(),
        catalog,
        SourceDef { file, value: map },
        20,
        ContentId::parse("quarry:field").unwrap(),
        resolved,
        Some(turts_sim::terrain::PrivateTerrainSeed::new(77)),
    )
    .unwrap();
    turts_sim::install_package(app.world_mut(), staged);
    turts_sim::visibility::initialize_context(app.world_mut(), 91, &[0, 1]).unwrap();
    turts_sim::initialize_completed_world(app.world_mut());
    app
}
fn remove_owner_sight(app: &mut bevy_app::App) {
    let own: Vec<_> = turts_sim::snapshot(app.world_mut())
        .into_iter()
        .filter(|u| u.owner_slot == 0)
        .map(|u| u.unit_id)
        .collect();
    for (seq, unit_id) in own.into_iter().enumerate() {
        app.world_mut()
            .resource_mut::<turts_sim::CommandQueue>()
            .0
            .push_back(turts_sim::Command {
                slot: turts_sim::SlotId(0),
                seq: seq as u32 + 1,
                kind: turts_sim::CommandKind::Remove { unit_id },
            });
    }
    app.update();
    assert!(
        app.world()
            .resource::<turts_sim::CommandResults>()
            .0
            .iter()
            .all(|r| r.outcome.is_ok())
    );
}

fn compare(view: &WorldView, policy: &dyn ObservationPermissions) {
    for viewer in [0, 1, turts_mod_api::SLOT_COUNT, u8::MAX] {
        let expected = snapshot_checked_owned_reference(view, SlotId(viewer), policy);
        let actual = snapshot_checked(view, SlotId(viewer), policy);
        assert_eq!(
            actual, expected,
            "owned/borrowed wire projection viewer{viewer}"
        );
        assert_eq!(
            postcard::to_allocvec(&actual).unwrap(),
            postcard::to_allocvec(&expected).unwrap(),
            "encoded terrain payload changed"
        );
        assert_eq!(
            serde_json::to_vec(&actual).unwrap(),
            serde_json::to_vec(&expected).unwrap()
        );
    }
}
#[test]
fn borrowed_terrain_snapshot_matches_owned_wire_bytes_current_stale_retained_and_mismatched() {
    for fog in [false, true] {
        let mut app = fixture(fog);
        let retained = turts_sim::completed_world(app.world()).clone();
        let retained_policy = crate::permissions::completed(app.world());
        compare(&retained, retained_policy.0.as_ref());
        remove_owner_sight(&mut app);
        let later = turts_sim::completed_world(app.world()).clone();
        let later_policy = crate::permissions::completed(app.world());
        compare(&later, later_policy.0.as_ref());
        compare(&retained, retained_policy.0.as_ref());
        if fog {
            assert!(snapshot_checked(&later, SlotId(0), retained_policy.0.as_ref()).is_err());
            compare(&later, retained_policy.0.as_ref());
        }
        let mut flat = later.clone();
        flat.terrain = None;
        compare(&flat, later_policy.0.as_ref());
        assert_eq!(
            snapshot_checked(&flat, SlotId(0), later_policy.0.as_ref()),
            Ok(None)
        );
        drop(app);
        compare(&retained, retained_policy.0.as_ref());
    }
}

// Frozen original full projection, including validation and canonical output order.
fn snapshot_checked_owned_reference(
    view: &WorldView,
    slot: SlotId,
    policy: &dyn ObservationPermissions,
) -> Result<Option<TerrainSnapshot>, &'static str> {
    let Some(phase) = view.terrain.as_deref() else {
        return Ok(None);
    };
    if slot.0 >= turts_mod_api::SLOT_COUNT
        || phase.tick() != view.tick
        || phase.observation_phase() != ObservationPhase::Completed
        || phase.knowledge_basis() != policy.knowledge_basis()
        || phase.knowledge_basis().is_some_and(|basis| {
            basis.tick != view.tick || basis.phase != ObservationPhase::Completed
        })
    {
        return Err("terrain basis does not match the completed permitted view");
    }
    if phase.materials().windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err("terrain material dictionary is not canonical");
    }
    let materials = phase.materials().iter().map(ToString::to_string).collect();
    let dictionary = phase
        .materials()
        .iter()
        .enumerate()
        .map(|(index, material)| {
            u32::try_from(index)
                .map(|index| (material, TerrainMaterialId(index)))
                .map_err(|_| "terrain material dictionary is too large")
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let grid = phase.grid();
    let valid_height = |height| height >= grid.floor && height <= grid.ceiling;
    let mut surfaces = Vec::new();
    for y in 0..grid.rows {
        for x in 0..grid.columns {
            let cell = TerrainCell { x, y };
            let Some(row) = phase.surface(slot.0, cell) else {
                continue;
            };
            if row.cell != cell
                || row.last_seen_tick > view.tick
                || !valid_height(row.elevation)
                || row.material.is_some() != row.provenance.is_some()
                || (row.material.is_none() && row.elevation != grid.floor)
            {
                return Err("terrain surface observation is malformed");
            }
            let material = row
                .material
                .as_ref()
                .map(|material| {
                    dictionary
                        .get(material)
                        .copied()
                        .ok_or("terrain surface names an unknown public material")
                })
                .transpose()?;
            surfaces.push(TerrainSurfaceRecord {
                cell,
                elevation: row.elevation,
                material,
                provenance: row.provenance,
                observed_tick: row.last_seen_tick,
                current: row.current,
            });
        }
    }
    let mut anchors = BTreeMap::new();
    for unit in &view.units {
        if !unit_permitted(policy, slot.0, unit.unit_id, unit.owner_slot) {
            continue;
        }
        let contact = phase
            .ground_anchor(slot.0, unit.unit_id)
            .ok_or("permitted live terrain unit lacks its ground contact")?;
        if contact.unit != unit.unit_id
            || contact.observed_tick != view.tick
            || !valid_height(contact.elevation)
        {
            return Err("live terrain ground contact is malformed");
        }
        if anchors
            .insert(
                unit.unit_id,
                TerrainGroundAnchor {
                    unit: UnitId(unit.unit_id),
                    elevation: contact.elevation,
                    observed_tick: contact.observed_tick,
                    current: true,
                },
            )
            .is_some()
        {
            return Err("permitted terrain view has duplicate unit identities");
        }
    }
    if let Some(knowledge) = policy.knowledge(slot.0) {
        if knowledge.viewer != slot.0 || Some(&knowledge.basis) != phase.knowledge_basis() {
            return Err("remembered terrain contacts have a mismatched knowledge basis");
        }
        for unit in knowledge.remembered_units {
            if anchors.contains_key(&unit.unit) {
                return Err("terrain unit is both live and remembered");
            }
            let elevation = unit
                .ground_elevation
                .ok_or("remembered terrain unit lacks its permitted ground contact")?;
            if unit.last_seen_tick > view.tick || !valid_height(elevation) {
                return Err("remembered terrain ground contact is malformed");
            }
            anchors.insert(
                unit.unit,
                TerrainGroundAnchor {
                    unit: UnitId(unit.unit),
                    elevation,
                    observed_tick: unit.last_seen_tick,
                    current: false,
                },
            );
        }
    } else if phase.knowledge_basis().is_some() {
        return Err("terrain view lacks its matching permitted knowledge");
    }
    let mut work = BTreeMap::new();
    for row in phase.work_for(slot.0) {
        if !anchors.get(&row.unit).is_some_and(|anchor| anchor.current) || row.owner_slot != slot.0
        {
            return Err("terrain work lacks its permitted live owner contact");
        }
        let record = TerrainWorkRecord {
            unit: UnitId(row.unit),
            designation: TerrainDesignation {
                tool: row.tool.to_string(),
                cells: row.cells,
                target_elevation: row.target_elevation,
            },
            current_cell: row.current_cell,
            completed_work: row.completed_work,
            required_work: row.required_work,
            wait: work_wait(&row.wait),
        };
        if work.insert(row.unit, record).is_some() {
            return Err("terrain work has duplicate unit identities");
        }
    }
    Ok(Some(TerrainSnapshot {
        materials,
        surfaces,
        anchors: anchors.into_values().collect(),
        work: work.into_values().collect(),
    }))
}
