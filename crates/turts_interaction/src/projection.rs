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

/// Projects a point on the ground plane to logical viewport coordinates.
///
/// Task 9 preparation stub. The production implementation is shared by selection and adapter tests.
pub fn world_to_screen(_point: Vec2, _viewport: &Viewport, _camera: CameraState) -> Option<Vec2> {
    let (view, projection) = matrices(_viewport, _camera)?;
    let world = view * world_plane_point(_point).extend(1.0);
    let clip = projection * world;
    if !clip.is_finite() || clip.w <= 0.0 {
        return None;
    }
    let ndc = clip.truncate() / clip.w;
    if !ndc.is_finite() || ndc.z < -1.0 || ndc.z > 1.0 {
        return None;
    }
    Some(Vec2::new(
        (ndc.x + 1.0) * 0.5 * _viewport.size.x,
        (1.0 - ndc.y) * 0.5 * _viewport.size.y,
    ))
}

/// Intersects a logical viewport coordinate with the ground plane.
///
/// Task 9 preparation stub. The production implementation is shared by picking and the adapter.
pub fn screen_to_world(_point: Vec2, _viewport: &Viewport, _camera: CameraState) -> Option<Vec2> {
    let (view, projection) = matrices(_viewport, _camera)?;
    let x = 2.0 * _point.x / _viewport.size.x - 1.0;
    let y = 1.0 - 2.0 * _point.y / _viewport.size.y;
    let inverse = (projection * view).inverse();
    if !inverse.is_finite() {
        return None;
    }
    let near = inverse * glam::Vec4::new(x, y, 1.0, 1.0);
    let far = inverse * glam::Vec4::new(x, y, 0.000_1, 1.0);
    if near.w.abs() < f32::EPSILON || far.w.abs() < f32::EPSILON {
        return None;
    }
    let origin = near.truncate() / near.w;
    let end = far.truncate() / far.w;
    let direction = end - origin;
    if !direction.is_finite() || direction.y.abs() < 1.0e-6 {
        return None;
    }
    let distance = -origin.y / direction.y;
    if distance < 0.0 || !distance.is_finite() {
        return None;
    }
    let point = origin + direction * distance;
    Some(Vec2::new(point.x, point.z))
}

fn matrices(viewport: &Viewport, camera: CameraState) -> Option<(Mat4, Mat4)> {
    if !viewport.size.is_finite()
        || viewport.size.x <= 0.0
        || viewport.size.y <= 0.0
        || !camera.look_at.is_finite()
        || !camera.height.is_finite()
        || camera.height <= 0.0
        || !camera.pitch.is_finite()
        || !camera.yaw.is_finite()
        || !camera.vertical_fov.is_finite()
        || camera.vertical_fov <= 0.0
        || camera.vertical_fov >= std::f32::consts::PI
    {
        return None;
    }
    let horizontal = camera.height * camera.pitch.tan();
    let eye = glam::Vec3::new(
        camera.look_at.x + camera.yaw.sin() * horizontal,
        camera.height,
        camera.look_at.y + camera.yaw.cos() * horizontal,
    );
    let target = world_plane_point(camera.look_at);
    let view = glam::camera::rh::view::look_at_mat4(eye, target, glam::Vec3::Y);
    let projection = perspective_matrix(viewport, camera);
    Some((view, projection))
}
