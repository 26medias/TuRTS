//! Bounded preview quantities from exposed current facts only. Deeper strata are Unknown.
use super::store::PermittedTerrain;
use std::collections::BTreeMap;
use turts_interaction::terrain::Preview;
use turts_mod_api::{
    ContentId, ResolvedTerrainOperation, ResolvedTerrainResourceRate, ResolvedTerrainTool,
    TerrainProvenance,
};

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Advice {
    pub inputs: BTreeMap<ContentId, u64>,
    pub next_outputs: BTreeMap<ContentId, u64>,
    pub no_yield_prisms: usize,
    pub unknown_cells: usize,
    pub deeper_unknown: bool,
}
fn add(
    totals: &mut BTreeMap<ContentId, u64>,
    rates: &[ResolvedTerrainResourceRate],
    steps: u64,
    prism: turts_rules::terrain::material::Prism,
) -> Option<()> {
    for rate in rates {
        let amount = prism
            .amount(
                turts_rules::terrain::Ratio {
                    numerator: u64::from(rate.per_world_cube.numerator),
                    denominator: u64::from(rate.per_world_cube.denominator),
                },
                u64::MAX,
            )
            .ok()?
            .checked_mul(steps)?;
        let total = totals.entry(rate.resource.clone()).or_default();
        *total = total.checked_add(amount)?;
    }
    Some(())
}
pub(crate) fn estimate(
    terrain: &PermittedTerrain,
    catalog: &turts_mod_api::ContentCatalog,
    tool: &ResolvedTerrainTool,
    preview: &Preview,
) -> Option<Advice> {
    let level = preview.target_elevation?;
    let grid = terrain.layout.grid;
    let prism = turts_rules::terrain::material::Prism::new(
        u64::from(grid.length_scale),
        u64::from(grid.cell_width),
        u64::from(grid.elevation_step),
    )
    .ok()?;
    let mut result = Advice::default();
    for cell in &preview.cells {
        // A stale row is deliberately not consulted, including its provenance.
        if !cell.current {
            result.unknown_cells += 1;
            continue;
        }
        match &tool.operation {
            ResolvedTerrainOperation::Fill { material } => {
                let steps = i64::from(level) - i64::from(cell.elevation);
                if steps > 0 {
                    add(
                        &mut result.inputs,
                        &catalog.terrain_materials.get(material)?.fill_inputs,
                        steps as u64,
                        prism,
                    )?;
                }
            }
            ResolvedTerrainOperation::Excavate if cell.elevation > level => {
                let Some(surface) = terrain
                    .surfaces
                    .get(&cell.cell)
                    .filter(|surface| surface.current)
                else {
                    result.unknown_cells += 1;
                    continue;
                };
                result.deeper_unknown |= i64::from(cell.elevation) - i64::from(level) > 1;
                if surface.provenance == Some(TerrainProvenance::Placed) {
                    result.no_yield_prisms += 1;
                    continue;
                }
                if surface.provenance == Some(TerrainProvenance::Natural) {
                    let id = terrain.materials.get(surface.material?.0 as usize)?;
                    let material = catalog.terrain_materials.get(&ContentId::parse(id).ok()?)?;
                    // Public support/hardness can explain an unavailable exposed prism.
                    // It must not estimate a yield that this particular tool cannot collect.
                    if tool.allowed_materials.contains(&material.id)
                        && material.hardness <= tool.max_hardness
                    {
                        add(
                            &mut result.next_outputs,
                            &material.natural_outputs,
                            1,
                            prism,
                        )?;
                    }
                }
            }
            _ => {}
        }
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        content::ClientContent,
        terrain::store::{Authority, PermittedTerrain},
    };
    use turts_mod_api::{KnowledgeBasis, ObservationPhase, TerrainCell};
    use turts_net_protocol::{
        TerrainGroundAnchor, TerrainMaterialId, TerrainSnapshot, TerrainSurfaceRecord,
        TerrainWorkRecord,
    };
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
    fn fixture(
        provenance: TerrainProvenance,
        current: bool,
    ) -> (ClientContent, PermittedTerrain, Preview) {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/quarry");
        let archive = turts_mod_runtime::archive::PackageArchive::parse(
            &turts_mod_runtime::pack::pack_dir(&path).unwrap(),
        )
        .unwrap();
        let content = ClientContent::from_archives(&[archive]).unwrap();
        let materials = content
            .catalog
            .terrain_materials
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        let material = TerrainMaterialId(
            materials
                .iter()
                .position(|id| id == "quarry:soil_layer")
                .unwrap() as u32,
        );
        let cell = TerrainCell { x: 1, y: 1 };
        let basis = KnowledgeBasis {
            match_id: 1,
            map: ContentId::parse("quarry:field").unwrap(),
            tick: 1,
            phase: ObservationPhase::Completed,
        };
        let terrain = PermittedTerrain::stage_snapshot(
            content.terrain.clone().unwrap(),
            &basis,
            &materials,
            &TerrainSnapshot {
                materials: materials.clone(),
                surfaces: vec![TerrainSurfaceRecord {
                    cell,
                    elevation: 0,
                    material: Some(material),
                    provenance: Some(provenance),
                    observed_tick: 1,
                    current,
                }],
                ..Default::default()
            },
            &Public,
            None,
        )
        .unwrap();
        let preview = Preview {
            input: turts_interaction::terrain::Input {
                order: "quarry:order_work".into(),
                tool: "quarry:spade".into(),
            },
            target_elevation: Some(-2),
            cells: vec![turts_interaction::terrain::CellPreview {
                cell,
                elevation: 0,
                current,
            }],
            rectangle: None,
            can_submit: true,
            reason: None,
        };
        (content, terrain, preview)
    }
    #[test]
    fn only_next_exposed_natural_prism_is_estimated_and_placed_fill_is_no_yield() {
        let (content, terrain, preview) = fixture(TerrainProvenance::Natural, true);
        let tool = &content.catalog.terrain_tools[&ContentId::parse("quarry:spade").unwrap()];
        let advice = estimate(&terrain, &content.catalog, tool, &preview).unwrap();
        assert_eq!(
            advice.next_outputs[&ContentId::parse("quarry:soil").unwrap()],
            1
        );
        assert!(advice.deeper_unknown);
        let (_, placed, _) = fixture(TerrainProvenance::Placed, true);
        let advice = estimate(&placed, &content.catalog, tool, &preview).unwrap();
        assert!(advice.next_outputs.is_empty());
        assert_eq!(advice.no_yield_prisms, 1);
    }
    #[test]
    fn unknown_preview_never_reads_stale_material_or_provenance_and_fill_uses_exact_prisms() {
        let (content, natural, mut preview) = fixture(TerrainProvenance::Natural, false);
        let (_, placed, _) = fixture(TerrainProvenance::Placed, false);
        let tool = &content.catalog.terrain_tools[&ContentId::parse("quarry:spade").unwrap()];
        let advice = estimate(&natural, &content.catalog, tool, &preview).unwrap();
        assert_eq!(
            advice,
            estimate(&placed, &content.catalog, tool, &preview).unwrap()
        );
        assert!(advice.next_outputs.is_empty());
        assert_eq!(advice.unknown_cells, 1);
        preview.target_elevation = Some(2);
        preview.cells[0].current = true;
        let fill = &content.catalog.terrain_tools[&ContentId::parse("quarry:backfill").unwrap()];
        let advice = estimate(&natural, &content.catalog, fill, &preview).unwrap();
        assert_eq!(advice.inputs[&ContentId::parse("quarry:soil").unwrap()], 2);
    }
}
