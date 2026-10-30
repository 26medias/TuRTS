//! Camera projection uses glam and remains independent of the renderer.

use glam::{Mat4, Vec2};

use crate::{CameraState, Viewport};

/// Builds the right-handed, reverse-Z perspective matrix used by the core camera.
pub fn perspective_matrix(viewport: &Viewport, camera: CameraState) -> Mat4 {
    glam::camera::rh::proj::directx::perspective_infinite_reverse(
        camera.vertical_fov,
        viewport.size.x / viewport.size.y,
        0.1,
    )
}

/// Converts a world-plane point to the camera's 3D ground plane.
pub fn world_plane_point(point: Vec2) -> glam::Vec3 {
    glam::Vec3::new(point.x, 0.0, point.y)
}
