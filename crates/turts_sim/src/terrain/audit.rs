//! Privileged replay facts. Never include these records in WorldView or public snapshots.
use super::{TerrainState, work::TerrainWorkRuntime};
use bevy_ecs::world::World;
use serde::{Deserialize, Serialize};
use turts_mod_api::{ContentId, TerrainCell};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainIntervalAudit {
    pub bottom: i32,
    pub top: i32,
    pub material: ContentId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainColumnAudit {
    pub cell: TerrainCell,
    pub initial_surface: i32,
    pub natural_top: i32,
    pub surface: i32,
    pub revision: u64,
    pub natural: Vec<TerrainIntervalAudit>,
    /// Every placed row has the one supported NoYield provenance.
    pub placed: Vec<TerrainIntervalAudit>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainWorkAudit {
    pub owner: crate::SlotId,
    pub unit: u32,
    pub order: crate::Order,
    pub cell: Option<TerrainCell>,
    pub done: u64,
    pub carry: u64,
    pub banked: bool,
    pub required: Option<u64>,
    pub wait: super::TerrainWorkWait,
    pub source_revision: Option<u64>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainClaimAudit {
    pub cell: TerrainCell,
    pub unit: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TerrainAudit {
    pub columns: u32,
    pub rows: u32,
    pub length_scale: u32,
    pub cell_width: u32,
    pub elevation_step: u32,
    pub floor: i32,
    pub ceiling: i32,
    pub columns_state: Vec<TerrainColumnAudit>,
    pub work: Vec<TerrainWorkAudit>,
    pub claims: Vec<TerrainClaimAudit>,
    pub paused: Vec<super::pause::TerrainPauseAudit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cargo_carries: Vec<(u32, u64)>,
}
/// Explicit privileged inspection. Seed provenance belongs to the private host replay header.
pub fn privileged_audit(world: &World) -> Option<TerrainAudit> {
    let terrain = world.get_resource::<TerrainState>()?;
    let columns_state = terrain
        .columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            let index = index as u64;
            let cell = TerrainCell {
                x: (index % u64::from(terrain.grid.columns)) as u32,
                y: (index / u64::from(terrain.grid.columns)) as u32,
            };
            TerrainColumnAudit {
                cell,
                initial_surface: column.initial_surface(),
                natural_top: column.natural_top(),
                surface: column.surface(),
                revision: column.revision(),
                natural: column
                    .natural_intervals()
                    .iter()
                    .filter_map(|row| {
                        Some(TerrainIntervalAudit {
                            bottom: row.bottom,
                            top: row.top,
                            material: terrain.material(row.material)?.clone(),
                        })
                    })
                    .collect(),
                placed: column
                    .placed_runs()
                    .iter()
                    .filter_map(|row| {
                        Some(TerrainIntervalAudit {
                            bottom: row.bottom,
                            top: row.top,
                            material: terrain.material(row.material)?.clone(),
                        })
                    })
                    .collect(),
            }
        })
        .collect();
    let runtime = world.get_resource::<TerrainWorkRuntime>();
    let work = runtime
        .into_iter()
        .flat_map(|runtime| runtime.units.iter())
        .map(|(&unit, state)| TerrainWorkAudit {
            owner: state.owner,
            unit,
            order: state.order.clone(),
            cell: state.cell,
            done: state.done,
            carry: state.carry,
            banked: state.banked,
            required: state.required,
            wait: state.wait.clone(),
            source_revision: state.source_revision,
        })
        .collect();
    let claims = runtime
        .into_iter()
        .flat_map(|runtime| runtime.claims.iter())
        .map(|(cell, &unit)| TerrainClaimAudit {
            cell: super::step::api_cell(*cell),
            unit,
        })
        .collect();
    Some(TerrainAudit {
        columns: terrain.grid.columns,
        rows: terrain.grid.rows,
        length_scale: terrain.grid.length_scale,
        cell_width: terrain.grid.cell_width,
        elevation_step: terrain.grid.elevation_step,
        floor: terrain.grid.floor,
        ceiling: terrain.grid.ceiling,
        columns_state,
        work,
        claims,
        paused: super::pause::audit(world),
        cargo_carries: super::cargo::audit_carries(world),
    })
}
