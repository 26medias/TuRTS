//! Authoritative terrain. Selected geology is private; callers must request permitted facts.
mod audit;
mod ground_cache;
pub(crate) mod history;
pub(crate) mod pause;
pub use pause::TerrainPauseAudit;
pub(crate) mod phase;
pub use phase::{
    TerrainGroundObservation, TerrainPhase, TerrainSurfaceObservation,
    TerrainSurfaceObservationRef, phase,
};
pub(crate) mod cargo;
pub(crate) mod navigation;
pub(crate) mod traffic;
pub use navigation::{
    FIELD_CREDIT as FLOCK_FIELD_CREDIT, FlockAudit, FlockMemberAudit, privileged_flock_audit,
};
pub use navigation::{
    NavigationAudit, NavigationDestinationRegionAudit, NavigationDiagnostics,
    NavigationMemberAudit, NavigationMemberState,
    privileged_diagnostics as privileged_navigation_diagnostics, privileged_navigation_audit,
};
mod state;
pub use audit::{
    TerrainAudit, TerrainClaimAudit, TerrainColumnAudit, TerrainIntervalAudit, TerrainWorkAudit,
    privileged_audit,
};
pub(crate) mod step;
pub(crate) mod work;
pub(crate) use state::{PreparedCellEdit, TerrainState, prepare};
pub use state::{PrivateTerrainSeed, TerrainError};
pub use work::{TerrainWorkView, TerrainWorkWait, rows_for as work_for};

/// Public grid metadata. It contains no selected material or private revision.
pub fn grid(world: &bevy_ecs::world::World) -> Option<turts_rules::terrain::geometry::Grid> {
    world
        .get_resource::<TerrainState>()
        .map(|terrain| terrain.grid)
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainObservedCell {
    pub cell: turts_mod_api::TerrainCell,
    pub elevation: i32,
    pub material: Option<turts_mod_api::ContentId>,
    pub provenance: Option<turts_mod_api::TerrainProvenance>,
}
/// Only a current full-cell observation exposes elevation/material. No hidden revision escapes.
pub fn observe_cell(
    world: &bevy_ecs::world::World,
    viewer: u8,
    cell: turts_mod_api::TerrainCell,
    phase_kind: turts_mod_api::ObservationPhase,
) -> Option<TerrainObservedCell> {
    phase(world, phase_kind)?.observe_cell(viewer, cell)
}
/// Physical support fact. Permission adapters must authorize the unit before exporting it.
/// Phase capture stores this value in immutable sight/event rows and in remembered units.
pub fn ground_elevation(world: &bevy_ecs::world::World, unit: u32) -> Option<i32> {
    let terrain = world.get_resource::<TerrainState>()?;
    let entity = crate::unit_entity(world, unit)?;
    let kind = world.get::<crate::UnitType>(entity)?.0;
    let position = world.get::<crate::Position>(entity)?.0;
    let types = world.get_resource::<crate::UnitTypes>()?;
    let catalog = &world
        .get_resource::<crate::bootstrap::InstalledCatalog>()?
        .0;
    let (radius, max_step) = support_parameters(catalog, types, kind).ok()?;
    let key = ground_cache::Key::new(kind, position, radius, max_step);
    if let Some(elevation) = ground_cache::lookup(world, terrain, unit, key) {
        return Some(elevation);
    }
    let support = exact_support(terrain, position, radius, max_step).ok()?;
    ground_cache::remember(world, terrain, unit, key, &support);
    Some(support.maximum)
}
pub(crate) fn support_elevation(
    terrain: &TerrainState,
    catalog: &turts_mod_api::ContentCatalog,
    types: &crate::UnitTypes,
    kind: turts_mod_api::UnitTypeId,
    position: bevy_math::Vec2,
) -> Result<i32, TerrainError> {
    let (radius, max_step) = support_parameters(catalog, types, kind)?;
    exact_support(terrain, position, radius, max_step).map(|support| support.maximum)
}
fn support_parameters(
    catalog: &turts_mod_api::ContentCatalog,
    types: &crate::UnitTypes,
    kind: turts_mod_api::UnitTypeId,
) -> Result<(f32, u32), TerrainError> {
    let entry = types.get(kind).ok_or(TerrainError::InvalidGrid)?;
    let id = turts_mod_api::ContentId::parse(&entry.content_id)
        .map_err(|_| TerrainError::InvalidGrid)?;
    let definition = catalog.units.get(&id).ok_or(TerrainError::InvalidGrid)?;
    let max_step = if entry.movement.is_none() {
        0
    } else {
        definition
            .terrain_class
            .as_ref()
            .and_then(|id| catalog.terrain_movement_classes.get(id))
            .map_or(0, |class| class.max_step)
    };
    Ok((entry.radius, max_step))
}
fn exact_support(
    terrain: &TerrainState,
    position: bevy_math::Vec2,
    radius: f32,
    max_step: u32,
) -> Result<turts_rules::terrain::geometry::Support, TerrainError> {
    let visits = usize::try_from(u64::from(terrain.grid.columns) * u64::from(terrain.grid.rows))
        .ok()
        .and_then(|cells| cells.checked_mul(2))
        .and_then(|cells| cells.checked_add(16))
        .ok_or(TerrainError::InvalidGrid)?;
    let mut budget = turts_rules::terrain::geometry::Budget::new(visits);
    terrain
        .grid
        .disk_support(
            terrain,
            turts_rules::terrain::geometry::Point {
                x: f64::from(position.x),
                y: f64::from(position.y),
            },
            f64::from(radius),
            max_step,
            &mut budget,
        )
        .map_err(TerrainError::Geometry)
}

/// A surface exposed during a successful step, witnessed before later same-tick deaths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerrainSurfaceWitness {
    pub surface: TerrainObservedCell,
    pub observed_tick: u32,
}
pub fn committed_surface_observations(
    world: &bevy_ecs::world::World,
    viewer: u8,
) -> Vec<TerrainSurfaceWitness> {
    world
        .get_resource::<TerrainState>()
        .map_or_else(Vec::new, |terrain| {
            terrain
                .step_observations
                .iter()
                .filter(|((owner, _), _)| *owner == viewer)
                .map(|(_, row)| row.clone())
                .collect()
        })
}
pub(crate) fn begin_tick(world: &mut bevy_ecs::world::World) {
    if let Some(mut terrain) = world.get_resource_mut::<TerrainState>() {
        terrain.step_observations.clear();
        terrain.clear_changed_chunks();
    }
}

#[cfg(test)]
mod tests;

/// Operator-only local invalidation counter. Do not expose hidden edit counts to players/bots.
pub fn privileged_changed_chunk_count(world: &bevy_ecs::world::World) -> usize {
    world
        .get_resource::<TerrainState>()
        .map_or(0, TerrainState::changed_chunks)
}

pub use navigation::privileged_contact_debug;
pub use traffic::privileged_traffic_debug;
