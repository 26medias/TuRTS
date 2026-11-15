//! Local chunk meshes use only the activated permitted store and verified public appearance.
use super::{mesh, store::PermittedTerrain};
use crate::{
    ground::{GroundMaterial, OverlaySource},
    ingest::ReplicaState,
    join::AcquiredPackages,
    models::GroundSurface,
};
use bevy::prelude::*;
use std::collections::BTreeMap;
use turts_mod_api::{ContentId, TerrainCell};

/// The setup plane keeps the shared authored material/texture sources for chunks.
#[derive(Component)]
pub(crate) struct MaterialSource;
#[derive(Component)]
pub(crate) struct TerrainTile;
#[derive(Resource, Default)]
pub(crate) struct ChunkRender {
    identity: Option<(u64, ContentId)>,
    tiles: BTreeMap<(TerrainCell, TerrainCell), Tile>,
    pub rebuilt_tiles: u64,
}
struct Tile {
    entity: Entity,
    mesh: Handle<Mesh>,
    revision: u64,
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn sync(
    mut commands: Commands,
    state: Res<ReplicaState>,
    acquired: Option<Res<AcquiredPackages>>,
    mut renderer: ResMut<ChunkRender>,
    mut meshes: ResMut<Assets<Mesh>>,
    sources: Query<
        (
            &GroundSurface,
            Option<&MeshMaterial3d<StandardMaterial>>,
            Option<&MeshMaterial3d<GroundMaterial>>,
            Option<&OverlaySource>,
        ),
        With<MaterialSource>,
    >,
) {
    let terrain = state
        .replica
        .as_ref()
        .and_then(|replica| replica.terrain.as_ref());
    let identity = terrain.map(|terrain| (terrain.basis.match_id, terrain.basis.map.clone()));
    if renderer.identity != identity {
        for tile in std::mem::take(&mut renderer.tiles).into_values() {
            commands.entity(tile.entity).despawn();
            meshes.remove(tile.mesh.id());
        }
        renderer.identity = identity;
    }
    let (Some(terrain), Some(acquired), Some((source, standard, layered, overlay))) =
        (terrain, acquired, sources.iter().next())
    else {
        return;
    };
    let colors = terrain
        .materials
        .iter()
        .map(|id| {
            ContentId::parse(id)
                .ok()
                .and_then(|id| acquired.0.content.catalog.terrain_materials.get(&id))
                .and_then(|material| {
                    crate::widgets::primitives::parse_colour(&material.surface_color)
                })
                .unwrap_or(Color::WHITE)
                .to_linear()
                .to_vec4()
                .to_array()
        })
        .collect::<Vec<_>>();
    let initial = if acquired.0.presentation.ground.is_some() {
        [1., 1., 1., 1.]
    } else {
        Color::srgb(0.25, 0.4, 0.25)
            .to_linear()
            .to_vec4()
            .to_array()
    };
    let tile_size = acquired
        .0
        .presentation
        .ground
        .as_ref()
        .map_or(1., |ground| ground.tile_size);
    for y in 0..terrain.layout.chunk_rows() {
        for x in 0..terrain.layout.chunk_columns() {
            let chunk = TerrainCell { x, y };
            let revision = terrain.revision(chunk);
            for rect in mesh::tiles(&terrain.layout, chunk) {
                let key = (chunk, rect.min);
                if renderer
                    .tiles
                    .get(&key)
                    .is_some_and(|tile| tile.revision == revision)
                {
                    continue;
                }
                let Some(mesh) = mesh::build(terrain, rect, &colors, initial, tile_size) else {
                    continue;
                };
                if let Some(tile) = renderer.tiles.get_mut(&key) {
                    if let Some(mut current) = meshes.get_mut(tile.mesh.id()) {
                        *current = mesh;
                    } else {
                        let mesh = meshes.add(mesh);
                        commands.entity(tile.entity).insert(Mesh3d(mesh.clone()));
                        tile.mesh = mesh;
                    }
                    tile.revision = revision;
                } else {
                    let mesh = meshes.add(mesh);
                    let mut entity = commands.spawn((
                        TerrainTile,
                        Mesh3d(mesh.clone()),
                        Transform::default(),
                        GroundSurface {
                            base: source.base.clone(),
                            normal: source.normal.clone(),
                        },
                    ));
                    if let Some(material) = standard {
                        entity.insert(MeshMaterial3d(material.0.clone()));
                    }
                    if let Some(material) = layered {
                        entity.insert(MeshMaterial3d(material.0.clone()));
                    }
                    if let Some(overlay) = overlay {
                        entity.insert(OverlaySource(overlay.0.clone()));
                    }
                    let entity = entity.id();
                    renderer.tiles.insert(
                        key,
                        Tile {
                            entity,
                            mesh,
                            revision,
                        },
                    );
                }
                renderer.rebuilt_tiles = renderer.rebuilt_tiles.saturating_add(1);
            }
        }
    }
}

/// A live body is grounded only by its authorized contact record, including sites.
/// It cannot infer an edited hidden height from the public initial surface.
pub(crate) fn live_height(terrain: Option<&PermittedTerrain>, unit: u32) -> Option<f32> {
    match terrain {
        None => Some(0.),
        Some(terrain) => terrain
            .anchors
            .get(&unit)
            .filter(|anchor| anchor.current)
            .map(|anchor| terrain.layout.elevation_height(anchor.elevation)),
    }
}

/// Infrastructure cannot lose support in M4, so its verified initial footprint
/// remains the physical height without exposing any dynamic hidden surface.
pub(crate) fn infrastructure_height(
    layout: Option<&super::store::Layout>,
    position: turts_net_protocol::Pos,
    radius: f32,
) -> Option<f32> {
    let Some(layout) = layout else {
        return Some(0.);
    };
    let mut budget = turts_rules::terrain::geometry::Budget::new(1_048_576);
    layout
        .grid
        .flat_foundation(
            layout,
            turts_rules::terrain::geometry::Point {
                x: f64::from(position.x),
                y: f64::from(position.y),
            },
            f64::from(radius),
            &mut budget,
        )
        .ok()
        .map(|support| support.anchor as f32)
}

/// Permitted top geometry, including remembered/public initial geometry. This is
/// for markers only, never for grounding a live actor or granting cell knowledge.
pub(crate) fn marker_height(
    terrain: Option<&PermittedTerrain>,
    position: turts_net_protocol::Pos,
) -> Option<f32> {
    let Some(terrain) = terrain else {
        return Some(0.);
    };
    let cell = terrain
        .layout
        .grid
        .point_to_cell(turts_rules::terrain::geometry::Point {
            x: f64::from(position.x),
            y: f64::from(position.y),
        })
        .ok()?;
    terrain
        .elevation(TerrainCell {
            x: cell.x,
            y: cell.y,
        })
        .map(|level| terrain.layout.elevation_height(level))
}
