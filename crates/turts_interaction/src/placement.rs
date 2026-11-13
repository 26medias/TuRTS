//! Construction placement uses the same geometry rules as the authoritative server.

use super::{Core, Effect, InputEvent, Vec2, Viewport, WorldView};
use std::collections::BTreeMap;
use turts_mod_api::{Behaviour, Button, TargetKind};
use turts_net_protocol::{OrderEnvelope, OrderTarget, Pos, QueueMode, UnitId};
use turts_rules::placement::{self, BuildSource, Circle, Obstacle, Point, QueueFill};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    pub taking_input: bool,
    constructor: u32,
    content: String,
    order: String,
    pressed: Option<(Button, Vec2)>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Input {
    pub constructor: u32,
    pub content: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Output {
    pub ghost: Option<Ghost>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ghost {
    pub content: String,
    pub point: Vec2,
    pub radius: f32,
    pub valid: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Recipe {
    pub radius: f32,
    pub gap: f32,
    pub node_clearance: f32,
    pub in_build_area: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Facts {
    pub recipes: BTreeMap<String, Recipe>,
    pub constructors: BTreeMap<u32, Vec<String>>,
    pub build_sources: BTreeMap<u8, Vec<BuildSource>>,
    pub obstacles: Vec<Obstacle>,
    pub nodes: Vec<Circle>,
    pub queues: BTreeMap<u32, QueueFill>,
}

impl Core {
    pub fn placement_state(&self) -> &State {
        &self.placement
    }
}

fn cancel(core: &mut Core, effects: &mut Vec<Effect>) {
    core.placement = State::default();
    core.pending_order = None;
    core.pointer_down = None;
    effects.push(Effect::Placement(Output::default()));
    effects.push(Effect::Cursor {
        context: "default".into(),
    });
}

fn ghost(core: &Core, px: Vec2, world: &dyn WorldView, view: &Viewport) -> Option<Ghost> {
    let point = super::projection::screen_to_world(px, view, core.camera)?;
    let facts = world.placement_facts();
    let recipe = facts.recipes.get(&core.placement.content)?;
    let bounds = world.map_size()?;
    let site = placement::Site {
        footprint: Circle {
            centre: Point {
                x: point.x,
                y: point.y,
            },
            radius: recipe.radius,
        },
        gap: recipe.gap,
        node_clearance: recipe.node_clearance,
        in_build_area: recipe.in_build_area,
        queue: facts.queues.get(&core.placement.constructor).copied(),
    };
    let verdict = placement::check(
        &site,
        &placement::View {
            map_width: bounds.x,
            map_height: bounds.y,
            build_sources: facts
                .build_sources
                .get(&world.own_slot().0)
                .map_or(&[], Vec::as_slice),
            obstacles: &facts.obstacles,
            nodes: &facts.nodes,
        },
        |_| true,
    );
    let reason = match verdict {
        placement::Verdict::Valid => None,
        placement::Verdict::Refused { reason, .. } => Some(reason.key().into()),
    };
    Some(Ghost {
        content: core.placement.content.clone(),
        point,
        radius: recipe.radius,
        valid: reason.is_none(),
        reason,
    })
}

pub fn intercept(
    core: &mut Core,
    _input_index: usize,
    event: &InputEvent,
    world: &dyn WorldView,
    view: &Viewport,
    _now_ms: u64,
    effects: &mut Vec<Effect>,
) -> bool {
    if !core.placement.taking_input {
        return false;
    }
    match event {
        InputEvent::KeyDown { code, .. }
            if core.action_for_key(code, world).as_deref() == Some("cancel") =>
        {
            cancel(core, effects);
            true
        }
        InputEvent::PointerDown {
            button,
            px,
            over_widget: false,
        } if core.ui_capture.is_none() && matches!(button, Button::Left | Button::Right) => {
            core.placement.pressed = Some((*button, *px));
            core.pointer_down = None;
            core.pointer = Some((*px, false));
            true
        }
        InputEvent::PointerUp {
            button,
            px,
            over_widget,
        } if matches!(button, Button::Left | Button::Right) => {
            core.pointer_down = None;
            core.held_buttons.retain(|held| held != button);
            core.held_modifier_actions
                .remove(&format!("pointer:{button:?}"));
            let pressed = core.placement.pressed.take();
            if core.ui_capture.is_none()
                && !over_widget
                && pressed.is_some_and(|(down, start)| {
                    down == *button && start.distance(*px) <= super::CLICK_DRAG_PIXELS
                })
            {
                if *button == Button::Right {
                    cancel(core, effects);
                } else if let Some(ghost) = ghost(core, *px, world, view) {
                    if ghost.valid {
                        effects.push(Effect::Send(OrderEnvelope {
                            order: core.placement.order.clone(),
                            units: vec![UnitId(core.placement.constructor)],
                            target: OrderTarget::ContentAtPoint {
                                content: core.placement.content.clone(),
                                point: Pos {
                                    x: ghost.point.x,
                                    y: ghost.point.y,
                                },
                            },
                            queue: QueueMode::Replace,
                            group: 0,
                        }));
                        cancel(core, effects);
                    } else if let Some(reason) = ghost.reason {
                        effects.push(Effect::Notice {
                            text_key: reason,
                            units: vec![UnitId(core.placement.constructor)],
                        });
                    }
                }
            }
            true
        }
        _ => false,
    }
}

pub fn on_input(
    core: &mut Core,
    input: &Input,
    world: &dyn WorldView,
    _view: &Viewport,
    _now_ms: u64,
    effects: &mut Vec<Effect>,
) {
    if core.ui_capture.is_some() {
        return;
    }
    let Some(unit) = world
        .units()
        .iter()
        .find(|unit| unit.id.0 == input.constructor && unit.owner_slot == world.own_slot())
    else {
        return;
    };
    let facts = world.placement_facts();
    if !facts
        .constructors
        .get(&input.constructor)
        .is_some_and(|recipes| recipes.contains(&input.content))
        || !facts.recipes.contains_key(&input.content)
    {
        return;
    }
    let Some(order) = world.orders().iter().find(|order| {
        order.behaviour == Behaviour::Construct
            && order.target == TargetKind::ContentAtPoint
            && (order.units.is_empty() || order.units.contains(&unit.content_id))
    }) else {
        return;
    };
    core.placement = State {
        taking_input: true,
        constructor: input.constructor,
        content: input.content.clone(),
        order: order.id.clone(),
        pressed: None,
    };
    core.pending_order = Some(order.id.clone());
    core.pointer_down = None;
    effects.push(Effect::Cursor {
        context: order.cursor.clone().unwrap_or_else(|| "default".into()),
    });
}

pub fn update(
    core: &mut Core,
    events: &[InputEvent],
    world: &dyn WorldView,
    view: &Viewport,
    _now_ms: u64,
    effects: &mut Vec<Effect>,
) {
    if !core.placement.taking_input {
        return;
    }
    if events
        .iter()
        .any(|event| matches!(event, InputEvent::FocusLost))
        || !world
            .placement_facts()
            .constructors
            .contains_key(&core.placement.constructor)
    {
        cancel(core, effects);
        return;
    }
    // An ordinary cancel action or another targeting mode also ends placement.
    if core.pending_order.as_deref() != Some(core.placement.order.as_str()) {
        core.placement = State::default();
        effects.push(Effect::Placement(Output::default()));
        return;
    }
    let shown = if core.ui_capture.is_none() {
        core.pointer.and_then(|(px, over_widget)| {
            (!over_widget)
                .then(|| ghost(core, px, world, view))
                .flatten()
        })
    } else {
        None
    };
    effects.push(Effect::Placement(Output { ghost: shown }));
}
