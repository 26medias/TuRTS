//! Camera and smart-order input on the authored minimap widget.

use glam::Vec2;

use crate::{Button, Core, Effect, Viewport, WorldView};

/// The minimap has the orientation of the main view. The camera (yaw 0 in each package) looks toward -y, so map y = 0
/// is at the top of the screen and at the top of the minimap. Both directions are 0..1 across the widget.
pub fn map_to_minimap(point: Vec2, map_size: Vec2) -> Vec2 {
    point / map_size
}
/// The map point under a widget point of the minimap; the inverse of [`map_to_minimap`].
pub fn minimap_to_map(local: Vec2, map_size: Vec2) -> Vec2 {
    local * map_size
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_input(
    core: &mut Core,
    button: Button,
    local: [f32; 2],
    world: &dyn WorldView,
    _view: &Viewport,
    _now_ms: u64,
    effects: &mut Vec<Effect>,
) {
    if !local.into_iter().all(f32::is_finite)
        || local
            .into_iter()
            .any(|coordinate| !(0.0..=1.0).contains(&coordinate))
    {
        return;
    }
    let Some(map_size) = world
        .map_size()
        .filter(|size| size.is_finite() && size.x > 0.0 && size.y > 0.0)
    else {
        return;
    };
    let point = minimap_to_map(Vec2::from(local), map_size);
    match button {
        Button::Left => {
            if core
                .camera_controller
                .centre_world(&mut core.camera, point, world)
            {
                effects.push(Effect::CameraChanged);
            }
        }
        Button::Right => {
            if core.pending_order.take().is_some() {
                effects.push(Effect::Cursor {
                    context: "default".into(),
                });
            } else {
                core.smart_command_at_world_point(point, world, effects);
            }
        }
        Button::Middle | Button::Back | Button::Forward => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The operator's case (2026-11-26): what stands at the top of the main view stands at the top of the minimap.
    #[test]
    fn the_top_of_the_main_view_is_the_top_of_the_minimap() {
        let size = Vec2::new(64.0, 48.0);
        let core = Core::default();
        let view = Viewport {
            size: Vec2::new(1280.0, 720.0),
            scale_factor: 1.0,
        };
        let (far, near) = (Vec2::new(32.0, 10.0), Vec2::new(32.0, 30.0));
        let on_screen = |p| core.project_world_to_screen(p, &view).y;
        assert!(
            on_screen(far) < on_screen(near),
            "the camera sees `far` higher"
        );
        assert!(map_to_minimap(far, size).y < map_to_minimap(near, size).y);
        // Left stays left, and a click on the minimap returns the point.
        assert!(map_to_minimap(Vec2::new(5.0, 10.0), size).x < map_to_minimap(far, size).x);
        for point in [far, near, Vec2::ZERO, size] {
            assert!(minimap_to_map(map_to_minimap(point, size), size).distance(point) < 1e-4);
        }
    }
}
