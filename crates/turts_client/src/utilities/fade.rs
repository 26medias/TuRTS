//! While the utility view is on, the terrain tiles use a blended copy of their material at
//! alpha 0.35. Units, buildings and the ground margin keep their solid material. When the view
//! turns off, each tile gets its original material back (spec "Browser mode").
//!
//! The tiles share the material of the ground source entity. A tile therefore never edits that
//! material: it points to a separate copy, one for each original, made when the view turns on.

use crate::{core_adapter::CoreRuntime, ground::GroundMaterial, terrain::render::TerrainTile};
use bevy::prelude::*;
use std::collections::HashMap;

/// The alpha of the terrain while the view is on.
pub const FADE_ALPHA: f32 = 0.35;

#[derive(Resource, Default)]
pub struct FadeState {
    /// Original material id to its blended copy.
    standard_copies: HashMap<AssetId<StandardMaterial>, Handle<StandardMaterial>>,
    ground_copies: HashMap<AssetId<GroundMaterial>, Handle<GroundMaterial>>,
    /// Copy id to the original handle, for the restore.
    standard_originals: HashMap<AssetId<StandardMaterial>, Handle<StandardMaterial>>,
    ground_originals: HashMap<AssetId<GroundMaterial>, Handle<GroundMaterial>>,
}

impl FadeState {
    /// True while at least one tile has a blended copy.
    pub fn active(&self) -> bool {
        !self.standard_originals.is_empty() || !self.ground_originals.is_empty()
    }
}

fn blended(mut material: StandardMaterial) -> StandardMaterial {
    material.alpha_mode = AlphaMode::Blend;
    material.base_color = material.base_color.with_alpha(FADE_ALPHA);
    material
}

#[allow(clippy::type_complexity)]
pub(crate) fn fade_terrain(
    runtime: Option<Res<CoreRuntime>>,
    mut state: ResMut<FadeState>,
    mut standard: ResMut<Assets<StandardMaterial>>,
    mut ground: ResMut<Assets<GroundMaterial>>,
    mut standard_tiles: Query<&mut MeshMaterial3d<StandardMaterial>, With<TerrainTile>>,
    mut ground_tiles: Query<&mut MeshMaterial3d<GroundMaterial>, With<TerrainTile>>,
) {
    let on = runtime
        .as_ref()
        .is_some_and(|runtime| runtime.utility_output().view_on);
    if !on && !state.active() {
        return;
    }
    let state = &mut *state;
    for mut tile in &mut standard_tiles {
        let current = tile.0.id();
        if on {
            if state.standard_originals.contains_key(&current) {
                continue;
            }
            let original = tile.0.clone();
            let copy = match state.standard_copies.get(&original.id()) {
                Some(copy) => copy.clone(),
                None => {
                    let Some(material) = standard.get(&original).cloned() else {
                        continue;
                    };
                    let copy = standard.add(blended(material));
                    state.standard_copies.insert(original.id(), copy.clone());
                    copy
                }
            };
            state.standard_originals.insert(copy.id(), original);
            tile.0 = copy;
        } else if let Some(original) = state.standard_originals.get(&current) {
            tile.0 = original.clone();
        }
    }
    for mut tile in &mut ground_tiles {
        let current = tile.0.id();
        if on {
            if state.ground_originals.contains_key(&current) {
                continue;
            }
            let original = tile.0.clone();
            let copy = match state.ground_copies.get(&original.id()) {
                Some(copy) => copy.clone(),
                None => {
                    let Some(mut material) = ground.get(&original).cloned() else {
                        continue;
                    };
                    material.base = blended(material.base);
                    let copy = ground.add(material);
                    state.ground_copies.insert(original.id(), copy.clone());
                    copy
                }
            };
            state.ground_originals.insert(copy.id(), original);
            tile.0 = copy;
        } else if let Some(original) = state.ground_originals.get(&current) {
            tile.0 = original.clone();
        }
    }
    if !on {
        // Every tile has its original back: drop the copies (the assets go with the handles).
        state.standard_copies.clear();
        state.standard_originals.clear();
        state.ground_copies.clear();
        state.ground_originals.clear();
    }
}
