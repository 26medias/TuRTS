//! Binding lookup: modifier chords, and the key, pointer and wheel actions of the input map.

use super::*;

fn modifiers_equal(a: &[Mod], b: &[Mod]) -> bool {
    a.len() == b.len() && a.iter().all(|modifier| b.contains(modifier))
}
pub(super) fn logical_modifier(action: &str) -> Option<Mod> {
    match action {
        "add_modifier" | "queue_modifier" => Some(Mod::Shift),
        "type_modifier" => Some(Mod::Ctrl),
        _ => None,
    }
}
pub(super) fn physical_modifiers(keys: &[String]) -> Vec<Mod> {
    let mut modifiers = Vec::new();
    if keys.iter().any(|key| key.starts_with("Shift")) {
        modifiers.push(Mod::Shift);
    }
    if keys.iter().any(|key| key.starts_with("Control")) {
        modifiers.push(Mod::Ctrl);
    }
    if keys.iter().any(|key| key.starts_with("Alt")) {
        modifiers.push(Mod::Alt);
    }
    modifiers
}
fn modifiers_before_key(code: &str, physical: &[Mod], held_keys: &[String]) -> Vec<Mod> {
    let introduced = if code.starts_with("Shift") {
        Some(Mod::Shift)
    } else if code.starts_with("Control") {
        Some(Mod::Ctrl)
    } else if code.starts_with("Alt") {
        Some(Mod::Alt)
    } else {
        None
    };
    let opposite = match code {
        "ShiftLeft" => Some("ShiftRight"),
        "ShiftRight" => Some("ShiftLeft"),
        "ControlLeft" => Some("ControlRight"),
        "ControlRight" => Some("ControlLeft"),
        "AltLeft" => Some("AltRight"),
        "AltRight" => Some("AltLeft"),
        _ => None,
    };
    let already_held = opposite.is_some_and(|opposite| held_keys.iter().any(|key| key == opposite));
    physical
        .iter()
        .copied()
        .filter(|modifier| already_held || Some(*modifier) != introduced)
        .collect()
}

fn modifier_actions_matching(
    world: &dyn WorldView,
    physical: &[Mod],
    trigger: impl Fn(&turts_mod_api::Binding) -> bool,
) -> Vec<String> {
    fn mods(binding: &turts_mod_api::Binding) -> &[Mod] {
        match binding {
            turts_mod_api::Binding::Key { mods, .. }
            | turts_mod_api::Binding::Pointer { mods, .. } => mods,
            turts_mod_api::Binding::Wheel(_) => &[],
        }
    }
    let actions = input_map::effective_actions_with_runtime(
        world.input_map(),
        world.runtime_binding_overrides(),
    );
    let specific = actions.values().flatten().any(|binding| {
        trigger(binding) && !mods(binding).is_empty() && modifiers_equal(mods(binding), physical)
    });
    actions
        .into_iter()
        .filter_map(|(name, bindings)| {
            (logical_modifier(&name).is_some()
                && bindings.iter().any(|binding| {
                    trigger(binding)
                        && (modifiers_equal(mods(binding), physical)
                            || (!specific && mods(binding).is_empty()))
                }))
            .then_some(name)
        })
        .collect()
}

impl Core {
    pub(super) fn modifier_actions_for_key(
        &self,
        code: &str,
        world: &dyn WorldView,
    ) -> Vec<String> {
        let physical = modifiers_before_key(code, &self.physical_mods, &self.keys);
        modifier_actions_matching(
            world,
            &physical,
            |binding| matches!(binding, turts_mod_api::Binding::Key { code: bound, .. } if bound == code),
        )
    }
    pub(super) fn modifier_actions_for_pointer(
        &self,
        button: Button,
        world: &dyn WorldView,
    ) -> Vec<String> {
        modifier_actions_matching(
            world,
            &self.physical_mods,
            |binding| matches!(binding, turts_mod_api::Binding::Pointer { button: bound, .. } if *bound == button),
        )
    }
    pub(super) fn action_for_wheel(&self, delta: f32, world: &dyn WorldView) -> Option<String> {
        let direction = if delta > 0.0 {
            turts_mod_api::WheelDirection::Up
        } else {
            turts_mod_api::WheelDirection::Down
        };
        input_map::effective_actions_with_runtime(world.input_map(), world.runtime_binding_overrides()).into_iter()
            .find_map(|(name, bindings)| bindings.iter().any(|binding| matches!(binding, turts_mod_api::Binding::Wheel(bound) if *bound == direction)).then_some(name))
    }
    pub(super) fn has_modifier_action(&self, action: &str) -> bool {
        self.held_modifier_actions
            .values()
            .any(|actions| actions.iter().any(|held| held == action))
    }
    pub(super) fn action_for_key(&self, code: &str, world: &dyn WorldView) -> Option<String> {
        let actions = input_map::effective_actions_with_runtime(
            world.input_map(),
            world.runtime_binding_overrides(),
        );
        let physical = modifiers_before_key(code, &self.physical_mods, &self.keys);
        let exact = actions.iter().find_map(|(name, bindings)| bindings.iter().any(|binding| matches!(binding, turts_mod_api::Binding::Key { code: key, mods } if key == code && modifiers_equal(mods, &physical))).then_some(name.clone()));
        exact.or_else(|| {
            (self.physical_mods.len() == 1 && self.physical_mods.contains(&Mod::Shift)).then(|| actions.into_iter().find_map(|(name, bindings)| {
                (world.orders().iter().any(|order| order.hotkey == name) && bindings.iter().any(|binding| matches!(binding, turts_mod_api::Binding::Key { code: key, mods } if key == code && mods.is_empty()))).then_some(name)
            })).flatten()
        })
    }
    pub(super) fn action_for_pointer(
        &self,
        button: Button,
        world: &dyn WorldView,
    ) -> Option<String> {
        let actions = input_map::effective_actions_with_runtime(
            world.input_map(),
            world.runtime_binding_overrides(),
        );
        let exact = actions.iter().find_map(|(name, bindings)| {
            bindings.iter().any(|binding| matches!(binding, turts_mod_api::Binding::Pointer { button: bound, mods } if *bound == button && modifiers_equal(mods, &self.physical_mods))).then_some(name.clone())
        });
        exact.or_else(|| actions.into_iter().find_map(|(name, bindings)| {
            bindings.iter().any(|binding| matches!(binding, turts_mod_api::Binding::Pointer { button: bound, mods } if *bound == button && mods.is_empty() && !self.physical_mods.is_empty())).then_some(name)
        }))
    }
}
