//! Headless action-file driver for the same interaction core used by the browser.

use bevy::prelude::*;
use serde::Deserialize;
use std::path::PathBuf;
use turts_interaction::{Button, Core, InputEvent, UiOrderTarget, Viewport};
use turts_mod_api::{ContentId, Mod, TargetKind};

use crate::ClientOpts;
use crate::core_adapter::{CoreAdapterSet, CoreInputQueue, CoreRuntime};
use crate::ingest::ReplicaState;
use crate::join::{AcquiredPackages, Phase};

const VIEWPORT: Viewport = Viewport {
    size: Vec2::new(1280.0, 720.0),
    scale_factor: 1.0,
};

/// One named action in the shared headless/browser fixture format.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct Action {
    #[serde(default)]
    tick: Option<u32>,
    #[serde(rename = "do")]
    kind: String,
    #[serde(default)]
    world_point: Option<[f32; 2]>,
    #[serde(default)]
    world_rect: Option<[f32; 4]>,
    #[serde(default)]
    order: Option<String>,
    #[serde(default)]
    content_id: Option<String>,
    #[serde(default)]
    job_id: Option<u64>,
    #[serde(default)]
    units: Vec<u32>,
    /// Browser-only authored UI pixel for a real widget click; native replay ignores it.
    #[serde(default, rename = "ui_point")]
    _ui_point: Option<[f32; 2]>,
    #[serde(default)]
    queue: bool,
    #[serde(default)]
    group: Option<u8>,
}

#[derive(Resource, Default)]
struct ActionRun {
    loaded: bool,
    actions: Vec<Action>,
    next: usize,
    start_tick: Option<u32>,
    error: Option<String>,
}

/// Sends action-file input through `CoreInputQueue`; it does not send commands itself.
pub struct ActionsPlugin;

impl Plugin for ActionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ActionRun>()
            .add_systems(Update, drive_actions.after(CoreAdapterSet));
    }
}

fn drive_actions(
    options: Res<ClientOpts>,
    state: Res<ReplicaState>,
    acquired: Option<Res<AcquiredPackages>>,
    runtime: Option<Res<CoreRuntime>>,
    mut run: ResMut<ActionRun>,
    input: Option<ResMut<CoreInputQueue>>,
    mut phase: ResMut<Phase>,
) {
    if run.error.is_some() {
        return;
    }
    let Some(path) = options.0.actions.as_ref() else {
        run.loaded = true;
        return;
    };
    if !run.loaded {
        match read_actions(path) {
            Ok(actions) => {
                run.actions = actions;
                run.loaded = true;
            }
            Err(error) => {
                let error = format!("--actions: {error}");
                *phase = Phase::Failed(error.clone());
                run.error = Some(error);
                return;
            }
        }
    }
    let (Some(runtime), Some(mut input)) = (runtime, input) else {
        return;
    };
    let (Some(welcome), Some(tick), Some(core)) = (
        state.welcome,
        state
            .replica
            .as_ref()
            .and_then(|replica| replica.last_applied_tick),
        runtime.core(),
    ) else {
        return;
    };
    let start_tick = *run.start_tick.get_or_insert(welcome.tick);
    let elapsed = tick.saturating_sub(start_tick);
    let Some(due_tick) = run.actions.get(run.next).and_then(|action| action.tick) else {
        return;
    };
    if due_tick > elapsed {
        return;
    }
    if let Some(action) = run.actions.get(run.next) {
        match action_events(action, core) {
            Ok(events) => {
                if let Err(error) = validate_action(action, &state, acquired.as_deref()) {
                    let error = format!("--actions action {}: {error}", run.next + 1);
                    *phase = Phase::Failed(error.clone());
                    run.error = Some(error);
                    return;
                }
                input.0.extend(events)
            }
            Err(error) => {
                let error = format!("--actions action {}: {error}", run.next + 1);
                *phase = Phase::Failed(error.clone());
                run.error = Some(error);
                return;
            }
        }
        run.next += 1;
    }
}

fn validate_action(
    action: &Action,
    state: &ReplicaState,
    acquired: Option<&AcquiredPackages>,
) -> Result<(), String> {
    if action.kind != "order" {
        return Ok(());
    }
    let id = action
        .order
        .as_deref()
        .ok_or_else(|| "order requires order".to_owned())?;
    let parsed = ContentId::parse(id)
        .map_err(|_| format!("order `{id}` must be an actual qualified catalogue order id"))?;
    let content = state
        .content
        .as_ref()
        .or_else(|| acquired.map(|packages| &packages.0.content))
        .ok_or_else(|| "accepted content catalogue is not available yet".to_owned())?;
    let definition = content
        .catalog
        .orders
        .get(&parsed)
        .ok_or_else(|| format!("unknown catalogue order `{id}`"))?;
    if action.content_id.is_some() && action.job_id.is_some() {
        return Err("order cannot contain both content_id and job_id".into());
    }
    if action.content_id.is_some()
        && (definition.target != TargetKind::Content || action.world_point.is_some())
    {
        return Err("content order requires a Content target and no world_point".into());
    }
    if action.job_id.is_some()
        && (definition.target != TargetKind::Job || action.world_point.is_some())
    {
        return Err("job order requires a Job target and no world_point".into());
    }
    if action.content_id.is_none() && action.job_id.is_none() {
        if action.units.len() > 0 {
            return Err("units are only valid with content_id or job_id".into());
        }
        if definition.target == TargetKind::None && action.world_point.is_some() {
            return Err("targetless order cannot contain world_point".into());
        }
        if definition.target != TargetKind::None && action.world_point.is_none() {
            return Err("targeted order requires world_point".into());
        }
    }
    Ok(())
}

#[cfg(not(target_family = "wasm"))]
fn read_actions(path: &PathBuf) -> Result<Vec<Action>, String> {
    let bytes =
        std::fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let mut actions: Vec<Action> = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{} is not an action list: {error}", path.display()))?;
    let mut previous = 0;
    for (index, action) in actions.iter_mut().enumerate() {
        if let Some(tick) = action.tick {
            if index > 0 && tick < previous {
                return Err(format!(
                    "action {} has a tick before the previous action",
                    index + 1
                ));
            }
            previous = tick;
        } else {
            action.tick = Some(previous);
        }
    }
    Ok(actions)
}

#[cfg(target_family = "wasm")]
fn read_actions(_path: &PathBuf) -> Result<Vec<Action>, String> {
    Err("action files are available only in a native headless client".into())
}

fn action_events(action: &Action, core: &Core) -> Result<Vec<InputEvent>, String> {
    let mut events = Vec::new();
    let mut pressed_keys = Vec::new();
    match action.kind.as_str() {
        "box_select" => {
            let [x1, y1, x2, y2] = action
                .world_rect
                .ok_or_else(|| "box_select requires world_rect".to_string())?;
            let start = project(core, Vec2::new(x1, y1))?;
            let end = project(core, Vec2::new(x2, y2))?;
            pointer_move(&mut events, start);
            if action.queue {
                key_down(&mut events, "ShiftLeft", &mut pressed_keys);
            }
            pointer_button(&mut events, Button::Left, start, true);
            pointer_move(&mut events, end);
            pointer_button(&mut events, Button::Left, end, false);
            if action.queue {
                key_up(&mut events, "ShiftLeft", &mut pressed_keys);
            }
        }
        "click_select" => {
            let point = action_point(action, core)?;
            pointer_move(&mut events, point);
            pointer_button(&mut events, Button::Left, point, true);
            pointer_button(&mut events, Button::Left, point, false);
        }
        "smart_command" => {
            let point = action_point(action, core)?;
            pointer_move(&mut events, point);
            if action.queue {
                key_down(&mut events, "ShiftLeft", &mut pressed_keys);
            }
            pointer_button(&mut events, Button::Right, point, true);
            pointer_button(&mut events, Button::Right, point, false);
            if action.queue {
                key_up(&mut events, "ShiftLeft", &mut pressed_keys);
            }
        }
        "order" => {
            let order = action
                .order
                .as_deref()
                .ok_or_else(|| "order requires order".to_string())?;
            if order.is_empty() {
                return Err("order must not be empty".into());
            }
            if let Some(content_id) = action.content_id.as_ref() {
                if action.units.is_empty() {
                    return Err("content order requires units".into());
                }
                events.push(InputEvent::UiOrder {
                    order: order.to_owned(),
                    units: action
                        .units
                        .iter()
                        .copied()
                        .map(turts_net_protocol::UnitId)
                        .collect(),
                    target: UiOrderTarget::Content(content_id.clone()),
                });
            } else if let Some(job_id) = action.job_id {
                if action.units.is_empty() {
                    return Err("job order requires units".into());
                }
                events.push(InputEvent::UiOrder {
                    order: order.to_owned(),
                    units: action
                        .units
                        .iter()
                        .copied()
                        .map(turts_net_protocol::UnitId)
                        .collect(),
                    target: UiOrderTarget::Job(turts_net_protocol::JobId(job_id)),
                });
            } else {
                events.push(InputEvent::UiAction {
                    name: order.to_owned(),
                });
            }
            if let Some(point) = action.world_point {
                let point = project(core, Vec2::new(point[0], point[1]))?;
                pointer_move(&mut events, point);
                if action.queue {
                    key_down(&mut events, "ShiftLeft", &mut pressed_keys);
                }
                pointer_button(&mut events, Button::Left, point, true);
                pointer_button(&mut events, Button::Left, point, false);
                if action.queue {
                    key_up(&mut events, "ShiftLeft", &mut pressed_keys);
                }
            }
        }
        "group_assign" | "group_recall" => {
            let group = action
                .group
                .filter(|group| (1..=9).contains(group))
                .ok_or_else(|| "group must be between 1 and 9".to_string())?;
            if action.kind == "group_assign" {
                key_down(&mut events, "ControlLeft", &mut pressed_keys);
            }
            let digit = format!("Digit{group}");
            key_down(&mut events, &digit, &mut pressed_keys);
            key_up(&mut events, &digit, &mut pressed_keys);
            if action.kind == "group_assign" {
                key_up(&mut events, "ControlLeft", &mut pressed_keys);
            }
        }
        kind => return Err(format!("unknown action `{kind}`")),
    }
    Ok(events)
}

fn action_point(action: &Action, core: &Core) -> Result<Vec2, String> {
    let [x, y] = action
        .world_point
        .ok_or_else(|| format!("{} requires world_point", action.kind))?;
    project(core, Vec2::new(x, y))
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::read_actions;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn omitted_action_ticks_inherit_the_previous_effective_tick() {
        let path = PathBuf::from(std::env::temp_dir()).join(format!(
            "turts-hd1-missing-tick-{}.json",
            std::process::id()
        ));
        fs::write(
            &path,
            r#"[{"tick":10,"do":"order","order":"muster:order_stop"},{"do":"order","order":"muster:order_hold"},{"tick":12,"do":"order","order":"muster:order_stop"}]"#,
        ).unwrap();
        let mixed = read_actions(&path).expect("mixed explicit and omitted ticks parse");
        assert_eq!(
            mixed
                .iter()
                .map(|action| action.tick.unwrap())
                .collect::<Vec<_>>(),
            [10, 10, 12]
        );

        fs::write(
            &path,
            r#"[{"do":"order","order":"muster:order_stop"},{"do":"order","order":"muster:order_hold"}]"#,
        ).unwrap();
        let omitted = read_actions(&path).expect("all omitted ticks parse");
        assert_eq!(
            omitted
                .iter()
                .map(|action| action.tick.unwrap())
                .collect::<Vec<_>>(),
            [0, 0]
        );
        fs::remove_file(path).unwrap();
    }
}

fn project(core: &Core, point: Vec2) -> Result<Vec2, String> {
    if !point.is_finite() {
        return Err("world coordinates must be finite".into());
    }
    let screen = core.project_world_to_screen(point, &VIEWPORT);
    if !screen.is_finite()
        || screen.x < 16.0
        || screen.y < 16.0
        || screen.x > 1264.0
        || screen.y > 704.0
    {
        return Err(format!(
            "world point {point:?} projects outside the safe viewport: {screen:?}"
        ));
    }
    Ok(screen)
}

fn pointer_move(events: &mut Vec<InputEvent>, px: Vec2) {
    events.push(InputEvent::PointerMove {
        px,
        over_widget: false,
    });
}

fn pointer_button(events: &mut Vec<InputEvent>, button: Button, px: Vec2, pressed: bool) {
    let event = if pressed {
        InputEvent::PointerDown {
            button,
            px,
            over_widget: false,
        }
    } else {
        InputEvent::PointerUp {
            button,
            px,
            over_widget: false,
        }
    };
    events.push(event);
}

fn key_down(events: &mut Vec<InputEvent>, code: &str, pressed_keys: &mut Vec<String>) {
    if !pressed_keys.iter().any(|key| key == code) {
        pressed_keys.push(code.to_owned());
    }
    let mods = pressed_keys
        .iter()
        .filter_map(|key| {
            if key.starts_with("Shift") {
                Some(Mod::Shift)
            } else if key.starts_with("Control") {
                Some(Mod::Ctrl)
            } else if key.starts_with("Alt") {
                Some(Mod::Alt)
            } else {
                None
            }
        })
        .collect();
    events.push(InputEvent::KeyDown {
        code: code.to_owned(),
        mods,
    });
}

fn key_up(events: &mut Vec<InputEvent>, code: &str, pressed_keys: &mut Vec<String>) {
    events.push(InputEvent::KeyUp {
        code: code.to_owned(),
    });
    pressed_keys.retain(|key| key != code);
}
