//! Camera control extension surface. Task 11 supplies the controller behavior.

use glam::Vec2;
use turts_mod_api::{Binding, CameraDef, CameraStart, WheelDirection};
use turts_net_protocol::UnitId;

use crate::{Button, CameraState, Effect, InputEvent, Viewport, WorldView, projection};

#[derive(Debug, Clone, PartialEq)]
pub struct CameraOptions {
    pub authored: Option<CameraDef>,
    pub scroll_speed: f32,
    pub edge_scroll: bool,
}

impl Default for CameraOptions {
    fn default() -> Self {
        Self {
            authored: None,
            scroll_speed: 1.0,
            edge_scroll: true,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct CameraController {
    last_step_ms: Option<u64>,
    pointer: Option<(Vec2, bool)>,
    drag_last_px: Option<Vec2>,
    last_group_recall: Option<(u64, String)>,
    pan_context: Option<String>,
}

impl CameraController {
    pub(crate) fn pan_context(&self) -> Option<&str> {
        self.pan_context.as_deref()
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn step(
        &mut self,
        camera: &mut CameraState,
        events: &[InputEvent],
        world: &dyn WorldView,
        view: &Viewport,
        now_ms: u64,
        _selected: &[UnitId],
        held_actions: &[String],
        effects: &mut Vec<Effect>,
    ) {
        let start_camera = *camera;
        let options = world.camera_options();
        let (minimum, maximum) = zoom_limits(&options);

        for event in events {
            match event {
                InputEvent::PointerMove { px, over_widget } => {
                    self.pointer = Some((*px, !over_widget));
                    if let Some(previous_px) = self.drag_last_px
                        && let (Some(previous_ground), Some(ground)) = (
                            projection::screen_to_world(previous_px, view, *camera),
                            projection::screen_to_world(*px, view, *camera),
                        )
                    {
                        camera.look_at +=
                            (previous_ground - ground) * options.scroll_speed.max(0.0);
                    }
                    if self.drag_last_px.is_some() {
                        self.drag_last_px = Some(*px);
                    }
                }
                InputEvent::PointerDown {
                    button: Button::Middle,
                    px,
                    over_widget: false,
                } => self.drag_last_px = Some(*px),
                InputEvent::PointerUp {
                    button: Button::Middle,
                    ..
                } => self.drag_last_px = None,
                InputEvent::PointerLeft => {
                    self.pointer = None;
                    self.drag_last_px = None;
                }
                InputEvent::FocusLost => {
                    self.pointer = None;
                    self.drag_last_px = None;
                    self.last_group_recall = None;
                }
                InputEvent::Wheel { delta } if delta.is_finite() && *delta != 0.0 => {
                    if let Some(direction) = wheel_action(world, *delta) {
                        let exponent = if direction { -delta.abs() } else { delta.abs() };
                        let factor = 1.1_f32.powf(exponent);
                        let height = camera.height * factor;
                        camera.height = if height.is_finite() {
                            height.clamp(minimum, maximum)
                        } else if exponent.is_sign_positive() {
                            maximum
                        } else {
                            minimum
                        };
                    }
                }
                _ => {}
            }
        }

        let elapsed = self
            .last_step_ms
            .replace(now_ms)
            .map_or(0.0, |last| now_ms.saturating_sub(last) as f32 / 1_000.0);
        let (horizontal, vertical) = self.pan_axes(&options, view, held_actions);
        self.pan_context = pan_context(horizontal, vertical);
        if elapsed > 0.0 {
            let right = Vec2::new(camera.yaw.cos(), -camera.yaw.sin());
            let forward = Vec2::new(-camera.yaw.sin(), -camera.yaw.cos());
            let direction = right * horizontal + forward * vertical;
            if direction.length_squared() > 0.0 {
                let authored_speed = options
                    .authored
                    .as_ref()
                    .map_or(12.0, |authored| authored.pan_speed);
                let speed = authored_speed.max(0.0) * options.scroll_speed.max(0.0);
                camera.look_at += direction.normalize() * camera.height * speed * 0.01 * elapsed;
            }
        } else {
            self.last_step_ms = Some(now_ms);
        }

        clamp_to_map(camera, world);
        if *camera != start_camera {
            effects.push(Effect::CameraChanged);
        }
    }

    fn pan_axes(
        &self,
        options: &CameraOptions,
        view: &Viewport,
        held_actions: &[String],
    ) -> (f32, f32) {
        let has_action = |name: &str| held_actions.iter().any(|action| action == name);
        let mut horizontal = axis(
            has_action("camera_pan_right"),
            has_action("camera_pan_left"),
        );
        let mut vertical = axis(has_action("camera_pan_up"), has_action("camera_pan_down"));

        if options.edge_scroll
            && options
                .authored
                .as_ref()
                .is_none_or(|authored| authored.edge_band)
            && let Some((point, eligible)) = self.pointer
            && eligible
            && point.x >= 0.0
            && point.y >= 0.0
            && point.x <= view.size.x
            && point.y <= view.size.y
        {
            if point.x < 8.0 {
                horizontal -= 1.0;
            } else if point.x >= view.size.x - 8.0 {
                horizontal += 1.0;
            }
            if point.y < 8.0 {
                vertical += 1.0;
            } else if point.y >= view.size.y - 8.0 {
                vertical -= 1.0;
            }
        }
        (horizontal, vertical)
    }

    pub(crate) fn centre_selection(
        &mut self,
        camera: &mut CameraState,
        selected: &[UnitId],
        world: &dyn WorldView,
    ) -> bool {
        let centre = centroid(selected, world);
        self.centre_on(camera, centre, world)
    }

    pub(crate) fn group_recall_centres(
        &mut self,
        action: &str,
        now_ms: u64,
        interval_ms: u64,
        camera: &mut CameraState,
        selected: &[UnitId],
        world: &dyn WorldView,
    ) -> bool {
        let double = self
            .last_group_recall
            .as_ref()
            .is_some_and(|(last, previous)| {
                previous == action && now_ms.saturating_sub(*last) <= interval_ms
            });
        self.last_group_recall = Some((now_ms, action.to_owned()));
        double && self.centre_selection(camera, selected, world)
    }

    fn centre_on(
        &mut self,
        camera: &mut CameraState,
        centre: Option<Vec2>,
        world: &dyn WorldView,
    ) -> bool {
        let Some(centre) = centre else {
            return false;
        };
        let before = camera.look_at;
        camera.look_at = centre;
        clamp_to_map(camera, world);
        camera.look_at != before
    }
}

fn pan_context(horizontal: f32, vertical: f32) -> Option<String> {
    let horizontal = if horizontal < 0.0 {
        Some("left")
    } else if horizontal > 0.0 {
        Some("right")
    } else {
        None
    };
    let vertical = if vertical > 0.0 {
        Some("up")
    } else if vertical < 0.0 {
        Some("down")
    } else {
        None
    };
    match (vertical, horizontal) {
        (None, None) => None,
        (Some(vertical), None) => Some(format!("pan_{vertical}")),
        (None, Some(horizontal)) => Some(format!("pan_{horizontal}")),
        (Some(vertical), Some(horizontal)) => Some(format!("pan_{vertical}_{horizontal}")),
    }
}

pub fn initial_state(
    definition: Option<&CameraDef>,
    map_size: Vec2,
    owned_centre: Option<Vec2>,
) -> CameraState {
    let Some(definition) = definition else {
        return CameraState {
            look_at: map_size * 0.5,
            height: 40.0,
            pitch: (30.0_f32 / 40.0).atan(),
            yaw: 0.0,
            vertical_fov: std::f32::consts::FRAC_PI_4,
        };
    };
    let look_at = match definition.start {
        CameraStart::OwnedCentre => owned_centre.unwrap_or(map_size * 0.5),
        CameraStart::MapCentre => map_size * 0.5,
        CameraStart::Point(x, y) => Vec2::new(x, y),
    };
    let margin = definition.margin.max(0.0);
    let bounds = (map_size.is_finite() && map_size.min_element() >= 0.0)
        .then_some((Vec2::splat(-margin), map_size + Vec2::splat(margin)));
    let look_at = bounds.map_or(look_at, |(minimum, maximum)| {
        look_at.clamp(minimum, maximum)
    });
    let minimum_height = definition.minimum.max(0.1);
    let maximum_height = definition.maximum.max(minimum_height);
    CameraState {
        look_at,
        height: definition.height.clamp(minimum_height, maximum_height),
        pitch: definition.pitch,
        yaw: definition.yaw,
        vertical_fov: definition.field_of_view.to_radians(),
    }
}

fn zoom_limits(options: &CameraOptions) -> (f32, f32) {
    options.authored.as_ref().map_or((4.0, 120.0), |authored| {
        let minimum = authored.minimum.max(0.1);
        (minimum, authored.maximum.max(minimum))
    })
}

fn wheel_action(world: &dyn WorldView, delta: f32) -> Option<bool> {
    let direction = if delta > 0.0 {
        WheelDirection::Up
    } else {
        WheelDirection::Down
    };
    let actions = crate::input_map::effective_actions_with_runtime(
        world.input_map(),
        world.runtime_binding_overrides(),
    );
    actions.iter().find_map(|(name, bindings)| {
        let zoom_in = match name.as_str() {
            "zoom_in" => true,
            "zoom_out" => false,
            _ => return None,
        };
        bindings
            .iter()
            .any(|binding| matches!(binding, Binding::Wheel(bound) if *bound == direction))
            .then_some(zoom_in)
    })
}

fn axis(positive: bool, negative: bool) -> f32 {
    match (positive, negative) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

fn centroid(selected: &[UnitId], world: &dyn WorldView) -> Option<Vec2> {
    let mut units = selected
        .iter()
        .filter_map(|id| world.units().iter().find(|unit| unit.id == *id));
    let first = units.next()?.position;
    let (sum, count) = units.fold((first, 1_u32), |(sum, count), unit| {
        (sum + unit.position, count + 1)
    });
    Some(sum / count as f32)
}

fn clamp_to_map(camera: &mut CameraState, world: &dyn WorldView) {
    let Some(map_size) = world.map_size() else {
        return;
    };
    let margin = world
        .camera_options()
        .authored
        .as_ref()
        .map_or(0.0, |authored| authored.margin.max(0.0));
    if map_size.is_finite() && map_size.x >= 0.0 && map_size.y >= 0.0 {
        camera.look_at = camera
            .look_at
            .clamp(Vec2::splat(-margin), map_size + Vec2::splat(margin));
    }
}
