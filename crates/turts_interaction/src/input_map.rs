//! Shared engine defaults and effective authored binding resolution.

use std::collections::BTreeMap;
use turts_mod_api::{ActionDef, Binding, Button, InputMapDef, Mod, WheelDirection};

pub fn engine_defaults() -> Vec<ActionDef> {
    fn key(name: String, code: String, mods: Vec<Mod>) -> ActionDef {
        ActionDef {
            name,
            default: vec![Binding::Key { code, mods }],
        }
    }
    let mut actions = vec![
        ActionDef {
            name: "select".into(),
            default: vec![Binding::Pointer {
                button: Button::Left,
                mods: vec![],
            }],
        },
        ActionDef {
            name: "box_select".into(),
            default: vec![Binding::Pointer {
                button: Button::Left,
                mods: vec![],
            }],
        },
        ActionDef {
            name: "command".into(),
            default: vec![Binding::Pointer {
                button: Button::Right,
                mods: vec![],
            }],
        },
        key("add_modifier".into(), "ShiftLeft".into(), vec![]),
        key("queue_modifier".into(), "ShiftLeft".into(), vec![]),
        key("type_modifier".into(), "ControlLeft".into(), vec![]),
    ];
    for n in 1..=9 {
        actions.push(key(
            format!("group_recall_{n}"),
            format!("Digit{n}"),
            vec![],
        ));
        actions.push(key(
            format!("group_assign_{n}"),
            format!("Digit{n}"),
            vec![Mod::Ctrl],
        ));
        actions.push(key(
            format!("group_add_{n}"),
            format!("Digit{n}"),
            vec![Mod::Shift],
        ));
    }
    for (name, code) in [
        ("camera_pan_up", "ArrowUp"),
        ("camera_pan_down", "ArrowDown"),
        ("camera_pan_left", "ArrowLeft"),
        ("camera_pan_right", "ArrowRight"),
        ("centre_selection", "Space"),
        ("cancel", "Escape"),
        ("toggle_guide", "F1"),
    ] {
        actions.push(key(name.into(), code.into(), vec![]));
    }
    actions.extend([
        ActionDef {
            name: "zoom_in".into(),
            default: vec![Binding::Wheel(WheelDirection::Up)],
        },
        ActionDef {
            name: "zoom_out".into(),
            default: vec![Binding::Wheel(WheelDirection::Down)],
        },
        ActionDef {
            name: "menu".into(),
            default: vec![
                Binding::Key {
                    code: "F10".into(),
                    mods: vec![],
                },
                Binding::Key {
                    code: "Escape".into(),
                    mods: vec![],
                },
            ],
        },
    ]);
    actions
}

/// Engine defaults, overridden engine actions, then mod-authored action defaults.
pub fn effective_actions(input: &InputMapDef) -> BTreeMap<String, Vec<Binding>> {
    let mut result: BTreeMap<_, _> = engine_defaults()
        .into_iter()
        .map(|action| (action.name, action.default))
        .collect();
    for override_action in &input.engine_overrides {
        result.insert(
            override_action.name.clone(),
            override_action.bindings.clone(),
        );
    }
    for action in &input.actions {
        result.insert(action.name.clone(), action.default.clone());
    }
    result
}

/// Resolve engine defaults, mod overrides, authored defaults, then player overrides.
pub fn effective_actions_with_runtime(
    input: &InputMapDef,
    runtime_overrides: &BTreeMap<String, Vec<Binding>>,
) -> BTreeMap<String, Vec<Binding>> {
    let mut result = effective_actions(input);
    for (name, bindings) in runtime_overrides {
        result.insert(name.clone(), bindings.clone());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use turts_mod_api::{ActionDef, EngineOverride};

    #[test]
    fn runtime_bindings_override_engine_and_authored_defaults() {
        let input = InputMapDef {
            engine_overrides: vec![EngineOverride {
                name: "select".into(),
                bindings: vec![Binding::Pointer {
                    button: Button::Right,
                    mods: vec![],
                }],
            }],
            actions: vec![ActionDef {
                name: "select".into(),
                default: vec![Binding::Pointer {
                    button: Button::Middle,
                    mods: vec![],
                }],
            }],
        };
        let engine_only = InputMapDef {
            engine_overrides: input.engine_overrides.clone(),
            actions: Vec::new(),
        };
        assert!(matches!(
            effective_actions(&engine_only)["select"].as_slice(),
            [Binding::Pointer {
                button: Button::Right,
                mods
            }] if mods.is_empty()
        ));
        assert!(matches!(
            effective_actions(&input)["select"].as_slice(),
            [Binding::Pointer {
                button: Button::Middle,
                mods
            }] if mods.is_empty()
        ));
        let mut runtime = BTreeMap::new();
        runtime.insert(
            "select".into(),
            vec![Binding::Pointer {
                button: Button::Back,
                mods: vec![],
            }],
        );
        let effective = effective_actions_with_runtime(&input, &runtime);
        assert_eq!(
            effective["select"], runtime["select"],
            "stored player bindings replace engine and authored defaults"
        );
        assert!(matches!(
            effective["command"].as_slice(),
            [Binding::Pointer {
                button: Button::Right,
                mods
            }] if mods.is_empty()
        ));
    }
}
