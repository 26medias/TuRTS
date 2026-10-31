//! Camera control extension surface. Task 11 supplies the controller behavior.

use turts_mod_api::CameraDef;
use turts_net_protocol::UnitId;

use crate::{CameraState, Effect, InputEvent, Viewport, WorldView};

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
pub(crate) struct CameraController {}

impl CameraController {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn step(
        &mut self,
        _camera: &mut CameraState,
        _events: &[InputEvent],
        _world: &dyn WorldView,
        _view: &Viewport,
        _now_ms: u64,
        _selected: &[UnitId],
        _effects: &mut Vec<Effect>,
    ) {
        // Task 11 fills this stable dispatch point.
    }
}
