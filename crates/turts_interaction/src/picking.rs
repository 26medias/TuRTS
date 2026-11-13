//! Ray hits against the same simple shapes that the client draws.
use glam::{Quat, Vec2, Vec3};
use turts_mod_api::Shape;

/// Distance along a scene ray to the visible body. Map x/y become scene x/z.
/// Shape height is measured from the ground, as in the renderer.
pub fn shape_hit(
    origin: Vec3,
    direction: Vec3,
    position: Vec2,
    shape: &Shape,
    yaw: f32,
) -> Option<f32> {
    if !origin.is_finite()
        || !direction.is_finite()
        || direction.length_squared() <= 1e-12
        || !position.is_finite()
        || !yaw.is_finite()
    {
        return None;
    }
    let inverse = Quat::from_rotation_y(-yaw);
    let origin = inverse * (origin - Vec3::new(position.x, 0.0, position.y));
    let direction = inverse * direction;
    match *shape {
        Shape::Cuboid { x, y, z } => {
            if ![x, y, z].iter().all(|size| size.is_finite() && *size > 0.0) {
                return None;
            }
            let lower = Vec3::new(-x * 0.5, 0.0, -y * 0.5);
            let upper = Vec3::new(x * 0.5, z, y * 0.5);
            let mut enter = 0.0_f32;
            let mut exit = f32::INFINITY;
            for axis in 0..3 {
                if direction[axis].abs() < 1e-8 {
                    if origin[axis] < lower[axis] || origin[axis] > upper[axis] {
                        return None;
                    }
                } else {
                    let a = (lower[axis] - origin[axis]) / direction[axis];
                    let b = (upper[axis] - origin[axis]) / direction[axis];
                    enter = enter.max(a.min(b));
                    exit = exit.min(a.max(b));
                    if enter > exit {
                        return None;
                    }
                }
            }
            (enter.is_finite() && exit >= 0.0).then_some(enter)
        }
        Shape::Capsule { radius, height } => {
            if !radius.is_finite() || !height.is_finite() || radius <= 0.0 || height < 2.0 * radius
            {
                return None;
            }
            let mut best = None;
            for centre in [Vec3::Y * radius, Vec3::Y * (height - radius)] {
                if let Some(t) = sphere_hit(origin - centre, direction, radius) {
                    record_hit(&mut best, t);
                }
            }
            let a = direction.x * direction.x + direction.z * direction.z;
            let b = origin.x * direction.x + origin.z * direction.z;
            let c = origin.x * origin.x + origin.z * origin.z - radius * radius;
            let discriminant = b * b - a * c;
            if a > 1e-10 && discriminant >= 0.0 {
                for t in [
                    (-b - discriminant.sqrt()) / a,
                    (-b + discriminant.sqrt()) / a,
                ] {
                    let y = origin.y + t * direction.y;
                    if t >= 0.0 && y >= radius && y <= height - radius {
                        record_hit(&mut best, t);
                    }
                }
            }
            best
        }
    }
}

fn sphere_hit(offset: Vec3, direction: Vec3, radius: f32) -> Option<f32> {
    let a = direction.length_squared();
    if a <= 1e-10 {
        return None;
    }
    let b = offset.dot(direction);
    let c = offset.length_squared() - radius * radius;
    let discriminant = b * b - a * c;
    if discriminant < 0.0 {
        return None;
    }
    [
        (-b - discriminant.sqrt()) / a,
        (-b + discriminant.sqrt()) / a,
    ]
    .into_iter()
    .filter(|t| *t >= 0.0)
    .min_by(f32::total_cmp)
}

fn record_hit(best: &mut Option<f32>, t: f32) {
    if t.is_finite() && t >= 0.0 && best.is_none_or(|previous| t < previous) {
        *best = Some(t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rays_hit_the_capsule_body_and_caps_but_miss_its_empty_corner() {
        let shape = Shape::Capsule {
            radius: 0.4,
            height: 1.6,
        };
        for y in [0.1, 0.8, 1.5] {
            assert!(shape_hit(Vec3::new(0.0, y, 5.0), -Vec3::Z, Vec2::ZERO, &shape, 0.0).is_some());
        }
        assert!(
            shape_hit(
                Vec3::new(0.39, 1.59, 5.0),
                -Vec3::Z,
                Vec2::ZERO,
                &shape,
                0.0
            )
            .is_none()
        );
    }
    #[test]
    fn cuboid_picking_uses_the_rendered_height_and_facing() {
        let shape = Shape::Cuboid {
            x: 4.0,
            y: 1.0,
            z: 2.0,
        };
        let origin = Vec3::new(1.5, 1.0, 5.0);
        assert!(shape_hit(origin, -Vec3::Z, Vec2::ZERO, &shape, 0.0).is_some());
        assert!(
            shape_hit(
                origin,
                -Vec3::Z,
                Vec2::ZERO,
                &shape,
                std::f32::consts::FRAC_PI_2
            )
            .is_none()
        );
        assert!(shape_hit(Vec3::new(0.0, 2.1, 5.0), -Vec3::Z, Vec2::ZERO, &shape, 0.0).is_none());
    }
}
