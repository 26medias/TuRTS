//! Shared selection, capture geometry, and presentation scheduling contracts.
use bevy::prelude::{Camera, Component, Resource, SystemSet, Vec2};

#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Selection(pub Option<u32>);

/// UI selection requests consumed by the interaction core on its next input step.
#[derive(Resource, Debug, Default)]
pub struct SelectionRequests(pub Vec<u32>);

/// Root marker for UI geometry that consumes world-pointer input.
#[derive(Component, Debug, Default)]
pub struct UiCaptureRoot;

/// Shared order point for UI presentation owners such as the legacy economy panel and Task 12 widgets.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct UiPresentationSet;

/// Convert a logical window cursor into UI camera viewport pixels.
pub fn cursor_in_ui_viewport(cursor: Vec2, camera: &Camera) -> Option<Vec2> {
    let viewport = camera.physical_viewport_rect()?;
    let physical = cursor * camera.target_scaling_factor()?;
    let min = viewport.min.as_vec2();
    let max = viewport.max.as_vec2();
    if physical.x < min.x || physical.y < min.y || physical.x >= max.x || physical.y >= max.y {
        return None;
    }
    Some(physical - min)
}
