//! Plateau top faces and exposed walls. Mesh tiling never changes authored cells.
use super::store::{Layout, PermittedTerrain};
use bevy::{
    asset::RenderAssetUsages,
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
};
use turts_mod_api::{TerrainCell, TerrainRect};

/// Bound one upload independently of an author's logical chunk size.
const TILE_CELLS: u32 = 16;

pub fn tiles(layout: &Layout, chunk: TerrainCell) -> Vec<TerrainRect> {
    if chunk.x >= layout.chunk_columns() || chunk.y >= layout.chunk_rows() {
        return vec![];
    }
    let min = TerrainCell {
        x: chunk.x * layout.chunk_width,
        y: chunk.y * layout.chunk_width,
    };
    let max = TerrainCell {
        x: min
            .x
            .saturating_add(layout.chunk_width)
            .min(layout.grid.columns),
        y: min
            .y
            .saturating_add(layout.chunk_width)
            .min(layout.grid.rows),
    };
    let mut result = Vec::new();
    for y in (min.y..max.y).step_by(TILE_CELLS as usize) {
        for x in (min.x..max.x).step_by(TILE_CELLS as usize) {
            result.push(TerrainRect {
                min: TerrainCell { x, y },
                max: TerrainCell {
                    x: (x + TILE_CELLS).min(max.x),
                    y: (y + TILE_CELLS).min(max.y),
                },
            });
        }
    }
    result
}

/// Materials are the canonical dictionary's linear RGBA colors. Unknown cells use
/// public map appearance, never a geological material chosen from generation rules.
pub fn build(
    terrain: &PermittedTerrain,
    rect: TerrainRect,
    materials: &[[f32; 4]],
    initial_color: [f32; 4],
    tile_size: f32,
) -> Option<Mesh> {
    if rect.min.x >= rect.max.x
        || rect.min.y >= rect.max.y
        || rect.max.x > terrain.layout.grid.columns
        || rect.max.y > terrain.layout.grid.rows
        || rect.max.x - rect.min.x > TILE_CELLS
        || rect.max.y - rect.min.y > TILE_CELLS
        || !tile_size.is_finite()
        || tile_size <= 0.
    {
        return None;
    }
    let mut data = Faces::default();
    let width = terrain.layout.cell_width();
    for y in rect.min.y..rect.max.y {
        for x in rect.min.x..rect.max.x {
            let cell = TerrainCell { x, y };
            let elevation = terrain.elevation(cell)?;
            let high = terrain.layout.elevation_height(elevation);
            let (x0, x1, z0, z1) = (
                x as f32 * width,
                (x + 1) as f32 * width,
                y as f32 * width,
                (y + 1) as f32 * width,
            );
            let color = terrain
                .surfaces
                .get(&cell)
                .and_then(|row| row.material)
                .and_then(|material| materials.get(material.0 as usize))
                .copied()
                .unwrap_or(initial_color);
            data.quad(
                [
                    [x0, high, z0],
                    [x0, high, z1],
                    [x1, high, z1],
                    [x1, high, z0],
                ],
                [0., 1., 0.],
                color,
                [[x0, z0], [x0, z1], [x1, z1], [x1, z0]],
                tile_size,
            );
            // A wall belongs to its higher cell. The adjacent chunk samples the same
            // permitted neighbor, so neither cracks nor duplicate faces are introduced.
            for (dx, dy, normal) in [
                (-1i64, 0i64, [-1., 0., 0.]),
                (1, 0, [1., 0., 0.]),
                (0, -1, [0., 0., -1.]),
                (0, 1, [0., 0., 1.]),
            ] {
                let (nx, ny) = (i64::from(x) + dx, i64::from(y) + dy);
                let lower = if nx < 0
                    || ny < 0
                    || nx >= i64::from(terrain.layout.grid.columns)
                    || ny >= i64::from(terrain.layout.grid.rows)
                {
                    terrain.layout.grid.floor
                } else {
                    terrain.elevation(TerrainCell {
                        x: nx as u32,
                        y: ny as u32,
                    })?
                };
                if lower >= elevation {
                    continue;
                }
                let low = terrain.layout.elevation_height(lower);
                let points = match (dx, dy) {
                    (-1, 0) => [[x0, low, z0], [x0, low, z1], [x0, high, z1], [x0, high, z0]],
                    (1, 0) => [[x1, low, z1], [x1, low, z0], [x1, high, z0], [x1, high, z1]],
                    (0, -1) => [[x1, low, z0], [x0, low, z0], [x0, high, z0], [x1, high, z0]],
                    _ => [[x0, low, z1], [x1, low, z1], [x1, high, z1], [x0, high, z1]],
                };
                let uvs = points.map(|p| [if dx == 0 { p[0] } else { p[2] }, p[1]]);
                data.quad(points, normal, color, uvs, tile_size);
            }
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, data.positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, data.normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, data.uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, data.colors)
    .with_inserted_indices(Indices::U32(data.indices));
    mesh.generate_tangents().ok()?;
    Some(mesh)
}
#[derive(Default)]
struct Faces {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    uvs: Vec<[f32; 2]>,
    indices: Vec<u32>,
}
impl Faces {
    fn quad(
        &mut self,
        points: [[f32; 3]; 4],
        normal: [f32; 3],
        color: [f32; 4],
        uvs: [[f32; 2]; 4],
        tile_size: f32,
    ) {
        let first = self.positions.len() as u32;
        self.positions.extend(points);
        self.normals.extend([normal; 4]);
        self.colors.extend([color; 4]);
        self.uvs
            .extend(uvs.map(|uv| [uv[0] / tile_size, uv[1] / tile_size]));
        self.indices
            .extend([first, first + 1, first + 2, first, first + 2, first + 3]);
    }
}

/// Keep authored landscape margins without drawing a flat plane through cuts.
/// No margin face overlaps a playable terrain cell.
pub(crate) fn margin(layout: &Layout, margin: f32, tile_size: f32, color: [f32; 4]) -> Mesh {
    let mut data = Faces::default();
    if margin.is_finite() && margin > 0. && tile_size.is_finite() && tile_size > 0. {
        let width = layout.grid.columns as f32 * layout.cell_width();
        let depth = layout.grid.rows as f32 * layout.cell_width();
        let height = layout.elevation_height(layout.grid.floor);
        for (x0, x1, z0, z1) in [
            (-margin, 0., -margin, depth + margin),
            (width, width + margin, -margin, depth + margin),
            (0., width, -margin, 0.),
            (0., width, depth, depth + margin),
        ] {
            data.quad(
                [
                    [x0, height, z0],
                    [x0, height, z1],
                    [x1, height, z1],
                    [x1, height, z0],
                ],
                [0., 1., 0.],
                color,
                [[x0, z0], [x0, z1], [x1, z1], [x1, z0]],
                tile_size,
            );
        }
    }
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, data.positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, data.normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, data.uvs)
    .with_inserted_attribute(Mesh::ATTRIBUTE_COLOR, data.colors)
    .with_inserted_indices(Indices::U32(data.indices));
    let _ = mesh.generate_tangents();
    mesh
}
