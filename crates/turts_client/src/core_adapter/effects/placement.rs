//! Keep the current placement preview for the renderer.

use super::*;

pub(in crate::core_adapter) fn apply(
    output: turts_interaction::placement::Output,
    runtime: &mut CoreRuntime,
) {
    runtime.placement_ghost = output.ghost;
}
