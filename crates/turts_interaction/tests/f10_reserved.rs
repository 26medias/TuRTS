//! F10 is reserved for the settings menu (re-check of `mp1/sound-off-default`, finding H1; ruling
//! of the orchestrator, option (b)). The settings menu holds the sound switch that every player
//! must be able to reach, so the engine reads a plain F10 as the action `menu` BEFORE any binding
//! of the mod or of the player. A package of the edition 0.2.4 cannot take F10 away (the packer
//! refuses it: `crates/turts_mod_runtime/tests/val1.rs`); a package of an older edition can, and
//! the engine still reads F10 first at run time. These tests use the real `Core::step`.
#[allow(dead_code)]
mod common;
use common::*;
use turts_interaction::{Core, Effect, InputEvent};
use turts_mod_api::{ActionDef, Binding, EngineOverride, InputMapDef};

fn key(code: &str) -> Binding {
    Binding::Key {
        code: code.into(),
        mods: vec![],
    }
}

fn down(code: &str) -> InputEvent {
    InputEvent::KeyDown {
        code: code.into(),
        mods: vec![],
    }
}

fn world_with(input: InputMapDef) -> TestWorld {
    TestWorld {
        input,
        ..TestWorld::default()
    }
}

fn opens_the_menu(world: &TestWorld, code: &str) -> bool {
    let mut core = Core::default();
    let effects = core.step(&[down(code)], world, &VIEW, 100);
    effects
        .iter()
        .any(|effect| matches!(effect, Effect::ShellAction { action } if action == "menu"))
}

/// The package `a1` of the re-check: `menu` on `KeyM`, which `key_move` also uses, and `cancel` with
/// no binding. Before this change F10 opened nothing in it (`action_for_key` takes the first match
/// in the name order, `key_move` comes before `menu`).
#[test]
fn f10_opens_the_menu_when_menu_is_on_a_key_that_another_action_uses() {
    let world = world_with(InputMapDef {
        actions: vec![ActionDef {
            name: "key_move".into(),
            default: vec![key("KeyM")],
        }],
        engine_overrides: vec![
            EngineOverride {
                name: "menu".into(),
                bindings: vec![key("KeyM")],
            },
            EngineOverride {
                name: "cancel".into(),
                bindings: vec![],
            },
        ],
    });
    assert!(
        opens_the_menu(&world, "F10"),
        "F10 opens the menu in package a1"
    );
    assert!(
        !opens_the_menu(&world, "Escape"),
        "the control: Escape does not (cancel is unbound)"
    );
}

/// The package `a5`: `menu` on `ShiftLeft` alone (a modifier key), and `cancel` with no binding.
#[test]
fn f10_opens_the_menu_when_menu_is_on_a_modifier_key_alone() {
    let world = world_with(InputMapDef {
        actions: vec![],
        engine_overrides: vec![
            EngineOverride {
                name: "menu".into(),
                bindings: vec![key("ShiftLeft")],
            },
            EngineOverride {
                name: "cancel".into(),
                bindings: vec![],
            },
        ],
    });
    assert!(
        opens_the_menu(&world, "F10"),
        "F10 opens the menu in package a5"
    );
}

/// A package of an older edition binds F10 to its own action: the engine reads F10 first. Also
/// the player's own binding (a runtime override) cannot take F10 away.
#[test]
fn f10_is_read_for_the_menu_before_the_bindings_of_the_mod_and_of_the_player() {
    let world = world_with(InputMapDef {
        actions: vec![ActionDef {
            name: "key_own".into(),
            default: vec![key("F10")],
        }],
        engine_overrides: vec![],
    });
    let mut core = Core::default();
    let effects = core.step(&[down("F10")], &world, &VIEW, 100);
    assert!(
        effects
            .iter()
            .any(|effect| matches!(effect, Effect::ShellAction { action } if action == "menu")),
        "F10 is the menu, not the action of the package: {effects:?}"
    );
    let mut player = TestWorld::default();
    player
        .runtime_bindings
        .insert("menu".into(), vec![key("KeyQ")]);
    player
        .runtime_bindings
        .insert("key_own".into(), vec![key("F10")]);
    assert!(
        opens_the_menu(&player, "F10"),
        "a rebind by the player does not take F10 from the menu"
    );
}

/// The control: no package change, the usual keys.
#[test]
fn the_default_keys_still_open_the_menu() {
    let world = TestWorld::default();
    assert!(opens_the_menu(&world, "F10"));
    assert!(opens_the_menu(&world, "Escape"));
}

/// Only a plain F10 is reserved: a chord with F10 (Ctrl+F10) is not the menu, and a mod may bind it.
/// (The reservation without its "no modifier held" condition opened the menu on Ctrl+F10.)
#[test]
fn only_a_plain_f10_is_reserved() {
    let world = TestWorld::default();
    let mut core = Core::default();
    let effects = core.step(
        &[InputEvent::KeyDown {
            code: "F10".into(),
            mods: vec![turts_mod_api::Mod::Ctrl],
        }],
        &world,
        &VIEW,
        100,
    );
    assert!(
        !effects
            .iter()
            .any(|effect| matches!(effect, Effect::ShellAction { action } if action == "menu")),
        "Ctrl+F10 is not the menu: {effects:?}"
    );
    assert!(opens_the_menu(&world, "F10"), "the control: a plain F10");
}
