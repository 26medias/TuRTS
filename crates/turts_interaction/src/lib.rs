//! Renderer-independent input and interaction contracts for a player client.

pub mod camera;
pub mod compose;
pub mod groups;
pub mod input_map;
pub mod minimap;
pub mod projection;
pub mod selection;
pub mod smart;

use glam::Vec2;
use turts_mod_api::{Button, Capability, InputMapDef, Mod, OrderDef, OrderRulesDef};
use turts_net_protocol::{NodeId, OrderEnvelope, SlotId, UnitId};

/// The plain state machine that turns frame input into client effects.
#[derive(Debug, Default)]
pub struct Core;

impl Core {
    /// Processes one frame of input. Interaction behavior is added by later tasks.
    pub fn step(
        &mut self,
        _events: &[InputEvent],
        _world: &dyn WorldView,
        _view: &Viewport,
        _now_ms: u64,
    ) -> Vec<Effect> {
        Vec::new()
    }
}

/// Input observed by the client adapter during one frame.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    KeyDown {
        code: String,
        mods: Vec<Mod>,
    },
    KeyUp {
        code: String,
    },
    PointerMove {
        px: Vec2,
        over_widget: bool,
    },
    PointerDown {
        button: Button,
        px: Vec2,
        over_widget: bool,
    },
    PointerUp {
        button: Button,
        px: Vec2,
        over_widget: bool,
    },
    PointerLeft,
    Wheel {
        delta: f32,
    },
    FocusLost,
    UiAction {
        name: String,
    },
    /// Pointer input on the minimap, with widget-local coordinates in `0..1`.
    Minimap {
        button: Button,
        local: [f32; 2],
    },
}

/// A unit record that is safe for the renderer-independent interaction core.
#[derive(Debug, Clone, PartialEq)]
pub struct UnitView {
    pub id: UnitId,
    pub content_id: String,
    pub owner_slot: SlotId,
    pub position: Vec2,
    pub footprint_radius: f32,
    pub capabilities: Vec<Capability>,
}

/// A resource-node record visible to the interaction core.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeView {
    pub id: NodeId,
    pub content_id: String,
    pub position: Vec2,
}

/// Read-only view of the replicated world and package input/order definitions.
pub trait WorldView {
    fn units(&self) -> &[UnitView];
    fn nodes(&self) -> &[NodeView];
    fn own_slot(&self) -> SlotId;
    fn orders(&self) -> &[OrderDef];
    fn order_rules(&self) -> &OrderRulesDef;
    fn input_map(&self) -> &InputMapDef;
}

/// Logical viewport dimensions and device scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub size: Vec2,
    pub scale_factor: f32,
}

/// The renderer-independent camera pose held by the interaction core.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraState {
    pub look_at: Vec2,
    pub height: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub vertical_fov: f32,
}

/// An output for the client adapter to apply or present.
#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Send(OrderEnvelope),
    SelectionChanged,
    GroupsChanged,
    CameraChanged,
    Marker {
        point: Vec2,
        kind: String,
    },
    Sound {
        id: String,
        category: String,
    },
    Notice {
        text_key: String,
        units: Vec<UnitId>,
    },
    Cursor {
        context: String,
    },
    ShellAction {
        action: String,
    },
}
