//! A press and a release in one frame, on a button that the pointer reached in that frame.
//!
//! Root cause of the lost first click (review M1 of `mp1/sound-off-default`). `bevy_picking`
//! 0.20.0 sends `Pointer<Press>` to the entities of this frame's `HoverMap`
//! (`bevy_picking-0.20.0/src/events.rs:1060`) and sends `Pointer<Click>` and `Pointer<Release>`
//! to the entities of the frame before, `PreviousHoverMap` (`events.rs:1091-1092`). A pointer that
//! moves onto a button and presses and releases in the same frame, with no earlier hover frame,
//! has the button in `HoverMap` and not in `PreviousHoverMap`. The press makes the button
//! `Pressed` (`bevy_ui_widgets-0.20.0/src/button.rs`, `button_on_pointer_down`). The release finds
//! nobody in `PreviousHoverMap`: no `Click`, so no `Activate`, and `Pressed` stays set. The next
//! click then acts (the review: "the second click turns it on"). A touch screen, a fast move and
//! click, and a test driver all give this order. It is the same for every `bevy_ui_widgets::Button`
//! of the engine: the widget buttons, the shell buttons and the binding buttons.
//!
//! The repair: after the hover map of the frame is built and before the events go out, when a
//! pointer has a press and then a release in this frame's input, the entities that the pointer
//! hovers now count as hovered before. Only these entities, only for this release.

use bevy::{
    picking::{
        hover::{HoverMap, PreviousHoverMap},
        pointer::{PointerAction, PointerInput},
    },
    prelude::*,
};
use std::collections::HashSet;

/// The system. `ClientShellPlugin` adds it between `generate_hovermap` and `pointer_events`.
pub fn hover_for_a_press_and_release_in_one_frame(
    mut inputs: MessageReader<PointerInput>,
    hover: Res<HoverMap>,
    mut previous: ResMut<PreviousHoverMap>,
) {
    let mut pressed = HashSet::new();
    for input in inputs.read() {
        match input.action {
            PointerAction::Press(_) => {
                pressed.insert(input.pointer_id);
            }
            PointerAction::Release(_) if pressed.contains(&input.pointer_id) => {
                let Some(now) = hover.get(&input.pointer_id) else {
                    continue;
                };
                let before = previous.entry(input.pointer_id).or_default();
                for (entity, hit) in now {
                    before.entry(*entity).or_insert_with(|| hit.clone());
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::{ManualTextureViewHandle, NormalizedRenderTarget};
    use bevy::picking::{
        InteractionPlugin, PickingPlugin,
        backend::HitData,
        events::pointer_events,
        pointer::{Location, PointerButton, PointerId, PointerLocation},
    };
    use bevy::ui_widgets::{Activate, Button as UiButton, ButtonPlugin};

    #[derive(Resource, Default)]
    struct Activated(u32);

    fn location() -> Location {
        Location {
            target: NormalizedRenderTarget::TextureView(ManualTextureViewHandle(5)),
            position: Vec2::new(3.0, 4.0),
        }
    }

    fn hit() -> HitData {
        HitData {
            camera: Entity::PLACEHOLDER,
            depth: 0.0,
            position: None,
            normal: None,
            extra: None,
        }
    }

    fn app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins((PickingPlugin, InteractionPlugin, ButtonPlugin))
            .init_resource::<Activated>()
            .add_observer(|_: On<Activate>, mut count: ResMut<Activated>| count.0 += 1);
        app.world_mut()
            .spawn((PointerId::Mouse, PointerLocation::new(location())));
        let button = app.world_mut().spawn(UiButton).id();
        (app, button)
    }

    /// One frame of picking, with the maps that a backend gives for the frame: `before` is the hover
    /// of the frame before, `now` the hover of this frame. The frame sends the given inputs.
    fn frame(
        app: &mut App,
        before: &[Entity],
        now: &[Entity],
        actions: &[PointerAction],
        with_the_repair: bool,
    ) {
        let map = |entities: &[Entity]| {
            let mut all = bevy::platform::collections::HashMap::default();
            all.insert(
                PointerId::Mouse,
                entities
                    .iter()
                    .map(|entity| (*entity, hit()))
                    .collect::<bevy::ecs::entity::EntityHashMap<HitData>>(),
            );
            all
        };
        app.world_mut()
            .insert_resource(PreviousHoverMap(map(before)));
        app.world_mut().insert_resource(HoverMap(map(now)));
        for action in actions {
            app.world_mut()
                .write_message(PointerInput::new(PointerId::Mouse, location(), *action));
        }
        if with_the_repair {
            app.world_mut()
                .run_system_cached(hover_for_a_press_and_release_in_one_frame)
                .unwrap();
        }
        app.world_mut().run_system_cached(pointer_events).unwrap();
        app.world_mut().flush();
        // The messages of this frame are consumed.
        app.world_mut()
            .resource_mut::<Messages<PointerInput>>()
            .clear();
    }

    const PRESS: PointerAction = PointerAction::Press(PointerButton::Primary);
    const RELEASE: PointerAction = PointerAction::Release(PointerButton::Primary);

    /// The browser order for a quick click on a button that the pointer reaches in the same frame:
    /// the move, the press and the release all come in one frame.
    #[test]
    fn a_press_and_release_with_no_earlier_hover_frame_activates_the_button() {
        let (mut app, button) = app();
        frame(&mut app, &[], &[button], &[PRESS, RELEASE], true);
        assert_eq!(app.world().resource::<Activated>().0, 1, "one activation");
        assert!(
            !app.world().entity(button).contains::<bevy::ui::Pressed>(),
            "the button is not left pressed"
        );
    }

    /// The fault, with the repair left out: no activation and a button that stays `Pressed`.
    /// This test pins the cause (`events.rs:1091`): when Bevy fixes it, this test goes red and the
    /// repair can go.
    #[test]
    fn without_the_repair_bevy_loses_the_click_and_leaves_the_button_pressed() {
        let (mut app, button) = app();
        frame(&mut app, &[], &[button], &[PRESS, RELEASE], false);
        assert_eq!(app.world().resource::<Activated>().0, 0);
        assert!(app.world().entity(button).contains::<bevy::ui::Pressed>());
    }

    /// The old order stays right: a hover frame first, the press in a frame, the release in a later
    /// frame (a person with a mouse).
    #[test]
    fn a_press_and_a_later_release_activate_once_with_or_without_the_repair() {
        for repair in [false, true] {
            let (mut app, button) = app();
            frame(&mut app, &[], &[button], &[], repair);
            frame(&mut app, &[button], &[button], &[PRESS], repair);
            frame(&mut app, &[button], &[button], &[RELEASE], repair);
            assert_eq!(app.world().resource::<Activated>().0, 1, "repair {repair}");
        }
    }

    /// The repair adds only the entities that this frame hovers: a press and a release in one frame
    /// with the pointer over nothing activate nothing, and a button that the pointer does not hover
    /// is not clicked.
    #[test]
    fn a_press_and_release_over_no_button_activate_nothing() {
        let (mut app, button) = app();
        let other = app.world_mut().spawn(UiButton).id();
        frame(&mut app, &[], &[other], &[PRESS, RELEASE], true);
        assert_eq!(
            app.world().resource::<Activated>().0,
            1,
            "the hovered button acts"
        );
        assert!(
            !app.world().entity(button).contains::<bevy::ui::Pressed>(),
            "a button that the pointer did not hover is not pressed"
        );
        frame(&mut app, &[], &[], &[PRESS, RELEASE], true);
        assert_eq!(
            app.world().resource::<Activated>().0,
            1,
            "no hovered button, no activation"
        );
    }
}
