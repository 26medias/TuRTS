//! The placement ghost, the build-area ring and the gap ring of `UI-PLACE` (design 7.2). L0-CORE made
//! this file and wired it once: the resource `Feed` is a field of `VisualInputs`, `register` runs
//! at plugin build, and `draw` is called in `draw_world_visuals` after the last present concern and
//! before `update_health_fills`. `UI-PLACE` edits this file only. Until then `draw` adds nothing.

use super::*;

/// What the ghost draw reads (the last `Ghost` of the core, from `CoreEffects`). Empty until
/// `UI-PLACE`.
#[derive(Resource, Default, Debug, Clone, PartialEq)]
pub(super) struct Feed {}

/// Registers the resources and the systems of the placement visuals. Called once by `VisualsPlugin`.
pub(super) fn register(app: &mut App) {
    app.init_resource::<Feed>();
}

/// Adds the placement visuals to `desired`. Stub: adds nothing.
pub(super) fn draw(_desired: &mut DesiredVisuals, _feed: &Feed) {}
