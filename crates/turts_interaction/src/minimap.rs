//! Camera and smart-order input on the authored minimap widget.

use glam::Vec2;

use crate::{Button, Core, Effect, Viewport, WorldView};

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
    // UI y increases down the widget while map y increases north.
    let point = Vec2::new(local[0] * map_size.x, (1.0 - local[1]) * map_size.y);
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
