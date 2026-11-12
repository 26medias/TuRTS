//! The visual effects pool of `L-FX-PRES` (design 7.7: one pool of 256 instances, the oldest is
//! reused). L0-CORE made this file and wired it once: the resource `Pool` is a field of
//! `VisualInputs`, `register` runs at plugin build, and `draw` is called in `draw_world_visuals`
//! after the placement concern and before `update_health_fills`. `L-FX-PRES` edits this file only.
//! Until then `draw` adds nothing.

use super::*;

/// The pool of effect instances. Empty until `L-FX-PRES`.
#[derive(Resource, Default, Debug)]
pub(super) struct Pool {}

/// Registers the resources and the systems of the effects. Called once by `VisualsPlugin`.
pub(super) fn register(app: &mut App) {
    app.init_resource::<Pool>();
}

/// Draws the live effects. Stub: adds nothing.
pub(super) fn draw(_desired: &mut DesiredVisuals, _pool: &mut Pool) {}
