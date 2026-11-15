//! Permitted terrain presentation. Private geological columns and seeds have no client type.
//!
//! Implementation in progress. Runtime behavior is unverified until the shared build window.
pub(crate) mod authority;
pub mod mesh;
pub(crate) mod present;
pub(crate) mod render;
pub mod store;

/// Bevy and the pure input core use the identical plateau/wall ray contract.
pub(crate) fn ray_hit(
    terrain: &store::PermittedTerrain,
    ray: bevy::prelude::Ray3d,
) -> Option<turts_rules::terrain::geometry::RayHit> {
    use turts_rules::terrain::geometry::{Budget, Point3, Ray};
    terrain
        .layout
        .grid
        .ray_hit(
            terrain.geometry.as_ref(),
            Ray {
                origin: Point3 {
                    x: f64::from(ray.origin.x),
                    y: f64::from(ray.origin.z),
                    z: f64::from(ray.origin.y),
                },
                direction: Point3 {
                    x: f64::from(ray.direction.x),
                    y: f64::from(ray.direction.z),
                    z: f64::from(ray.direction.y),
                },
            },
            &mut Budget::new(8_388_608),
        )
        .ok()?
}

/// An order record may retain an unavailable target, but its public designation
/// must still be bounded and name an actual capability of that owned actor.
pub(crate) fn valid_designation(
    content: &crate::content::ClientContent,
    actor: &str,
    order: &str,
    work: &turts_mod_api::TerrainDesignation,
) -> bool {
    let Some(layout) = content.terrain.as_ref() else {
        return false;
    };
    let Ok(actor) = turts_mod_api::ContentId::parse(actor) else {
        return false;
    };
    let Ok(order) = turts_mod_api::ContentId::parse(order) else {
        return false;
    };
    let Ok(tool) = turts_mod_api::ContentId::parse(&work.tool) else {
        return false;
    };
    let Some(definition) = content.catalog.units.get(&actor) else {
        return false;
    };
    let Some(order) = content.catalog.orders.get(&order) else {
        return false;
    };
    let r = work.cells;
    order.behaviour == turts_mod_api::Behaviour::TerrainWork
        && order.target == turts_mod_api::TargetKind::TerrainDesignation
        && definition.terrain_tools.contains(&tool)
        && content.catalog.terrain_tools.contains_key(&tool)
        && r.min.x < r.max.x
        && r.min.y < r.max.y
        && r.max.x <= layout.grid.columns
        && r.max.y <= layout.grid.rows
        && u64::from(r.max.x - r.min.x) * u64::from(r.max.y - r.min.y) <= 256
        && (layout.grid.floor..=layout.grid.ceiling).contains(&work.target_elevation)
}
