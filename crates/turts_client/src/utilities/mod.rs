//! M6 underground utilities in the client: the owner view store, the view draw, the terrain
//! fade, the hover and route panel, and the scene data for the test bridge.
//!
//! The client draws only rows that its own view holds. Normal play is unchanged while the view
//! is off: no system here spawns, draws or swaps anything then.

pub mod fade;
pub mod panel;
pub mod render;
pub mod scene;
pub mod store;

use bevy::prelude::*;

pub struct UtilityPlugin;

impl Plugin for UtilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<fade::FadeState>()
            .init_resource::<panel::UtilityPanel>()
            .add_systems(
                Update,
                (render::draw, fade::fade_terrain, panel::sync)
                    .after(crate::core_adapter::CoreAdapterSet)
                    .after(crate::terrain::render::sync),
            );
    }
}

#[cfg(test)]
pub(crate) mod tests;
