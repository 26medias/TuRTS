//! A press and a release in one frame, on a button that the pointer reached in that frame.
//!
//! Root cause of the lost first click (review M1 of `mp1/sound-off-default`). `bevy_picking`
//! 0.20.0 sends `Pointer<Press>` to the entities of this frame's `HoverMap`
//! (`bevy_picking-0.20.0/src/events.rs:1060`) and sends `Pointer<Click>` and `Pointer<Release>`
//! to the entities of the frame before, `PreviousHoverMap` (`events.rs:1091-1092`). A pointer
//! that moves onto a button and presses and releases in the same frame, with no earlier hover frame,
//! has the button in `HoverMap` and not in `PreviousHoverMap`. The press makes the button
//! `Pressed` (`bevy_ui_widgets-0.20.0/src/button.rs`, `button_on_pointer_down`). The release finds
//! nobody in `PreviousHoverMap`: no `Click`, so no `Activate`, and `Pressed` stays set. The next
//! click then acts. A touch screen, a fast move and click, and a test driver all give this order.
//! It is the same for every `bevy_ui_widgets::Button` of the engine.
//!
//! The repair (pass 3 of the review): after `pointer_events` has run, the click that Bevy dropped
//! is sent: `Click`, then `Release`, to each hovered entity. The order for the entity is then the
//! order of a normal hover-then-click: `Over`, `Press`, `Click`, `Release`. It is sent only when
//! the press and the release are the same click:
//!
//! - the same pointer and the same button, the press and the release in this frame's input;
//! - no drag between them by the rule of the interaction core (`CLICK_DRAG_PIXELS`, 4 px, the rule
//!   of `crates/turts_interaction/src/lib.rs:581` and `:632` that tells a click from a box select):
//!   no input of the pointer between the press and the release is farther than that from the
//!   press, and the release is not farther;
//! - the press position and the release position both hit the entity (a hit test of each position
//!   on the node, as `bevy_ui-0.20.0/src/picking_backend.rs:205` does for the pointer), so a press
//!   on the map and a release over a button, or a press on button A and a release on button B,
//!   click nothing.

use bevy::{
    camera::Camera,
    picking::{
        backend::HitData,
        events::{Pointer, PointerClick, PointerRelease},
        hover::{HoverMap, PreviousHoverMap},
        pointer::{PointerAction, PointerButton, PointerId, PointerInput},
    },
    prelude::*,
    ui::{CalculatedClip, ComputedNode, ComputedUiTargetCamera, Pressed, UiGlobalTransform},
    ui_widgets::Button as UiButton,
};
use std::{collections::HashMap, time::Duration};
use turts_interaction::CLICK_DRAG_PIXELS;

/// One press that is waiting for its release in this frame's input.
struct Press {
    button: PointerButton,
    start: Vec2,
    dragged: bool,
}

/// One click that Bevy dropped: the pointer, the button, the place of the press, the place of the
/// release.
struct DroppedClick {
    pointer: Pointer,
    button: PointerButton,
    press_at: Vec2,
    release_at: Vec2,
}

/// The clicks of this frame's input: a press and its release, the same pointer and button, no drag.
fn clicks_in_one_frame(inputs: impl IntoIterator<Item = PointerInput>) -> Vec<DroppedClick> {
    let mut down: HashMap<PointerId, Press> = HashMap::new();
    let mut found = Vec::new();
    for input in inputs {
        let at = input.location.position;
        match input.action {
            PointerAction::Press(button) => {
                down.insert(
                    input.pointer_id,
                    Press {
                        button,
                        start: at,
                        dragged: false,
                    },
                );
            }
            PointerAction::Move { .. } => {
                if let Some(press) = down.get_mut(&input.pointer_id)
                    && press.start.distance(at) > CLICK_DRAG_PIXELS
                {
                    press.dragged = true;
                }
            }
            PointerAction::Release(button) => {
                let Some(press) = down.remove(&input.pointer_id) else {
                    continue;
                };
                if press.button == button
                    && !press.dragged
                    && press.start.distance(at) <= CLICK_DRAG_PIXELS
                {
                    found.push(DroppedClick {
                        pointer: Pointer::new(input.pointer_id, input.location.clone()),
                        button,
                        press_at: press.start,
                        release_at: at,
                    });
                }
            }
            PointerAction::Cancel => {
                down.remove(&input.pointer_id);
            }
            _ => {}
        }
    }
    found
}

type UiNodes<'w, 's> = Query<
    'w,
    's,
    (
        &'static ComputedNode,
        &'static UiGlobalTransform,
        Option<&'static ComputedUiTargetCamera>,
        Option<&'static CalculatedClip>,
    ),
>;

/// The hit test of one position on one node, as the UI picking backend does it: the position in
/// logical pixels, times the scale of the camera of the node, minus the viewport.
fn node_hit(entity: Entity, position: Vec2, nodes: &UiNodes, cameras: &Query<&Camera>) -> bool {
    let Ok((node, transform, target, clip)) = nodes.get(entity) else {
        return false;
    };
    let camera = target
        .and_then(|target| target.get())
        .and_then(|camera| cameras.get(camera).ok());
    let mut point = position
        * camera
            .and_then(|camera| camera.target_scaling_factor())
            .unwrap_or(1.0);
    if let Some(viewport) = camera.and_then(|camera| camera.physical_viewport_rect()) {
        point -= viewport.min.as_vec2();
    }
    node.contains_point(*transform, point) && clip.is_none_or(|clip| clip.contains_point(point))
}

/// The system. `ClientShellPlugin` adds it after `pointer_events`.
pub fn click_in_one_frame(
    mut inputs: MessageReader<PointerInput>,
    hover: Res<HoverMap>,
    previous: Res<PreviousHoverMap>,
    nodes: UiNodes,
    cameras: Query<&Camera>,
    buttons: Query<(), With<UiButton>>,
    mut commands: Commands,
) {
    for click in clicks_in_one_frame(inputs.read().cloned()) {
        let Some(hovered) = hover.get(&click.pointer.id) else {
            continue;
        };
        for (entity, hit) in hovered {
            let _: &HitData = hit;
            // An entity that was hovered in the frame before got its click from Bevy already
            // (`events.rs:1091`): a second one would act twice (a click with a hover frame and a
            // press and release in one later frame is the usual case of a person).
            if previous
                .get(&click.pointer.id)
                .is_some_and(|before| before.contains_key(entity))
            {
                continue;
            }
            if !node_hit(*entity, click.press_at, &nodes, &cameras)
                || !node_hit(*entity, click.release_at, &nodes, &cameras)
            {
                continue;
            }
            // A move between the press and the release (a slip of the hand inside the limit) is a
            // drag for Bevy: its drag end took `Pressed` off the button
            // (`bevy_ui_widgets-0.20.0/src/button.rs`, `button_on_pointer_drag_end`) before this
            // system ran. The click needs `Pressed`, as the press gave it, so give it back.
            if buttons.contains(*entity) {
                commands.entity(*entity).insert(Pressed);
            }
            commands.trigger(PointerClick {
                entity: *entity,
                pointer: click.pointer.clone(),
                button: click.button,
                hit: hit.clone(),
                duration: Duration::ZERO,
                count: 1,
            });
            commands.trigger(PointerRelease {
                entity: *entity,
                pointer: click.pointer.clone(),
                button: click.button,
                hit: hit.clone(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::{ManualTextureViewHandle, NormalizedRenderTarget};
    use bevy::math::Affine2;
    use bevy::picking::{
        InteractionPlugin, PickingPlugin,
        events::{PointerOver, PointerPress, pointer_events},
        hover::PreviousHoverMap,
        pointer::{Location, PointerLocation},
    };
    use bevy::ui_widgets::{Activate, Button as UiButton, ButtonPlugin};

    /// What happened, in order: the activations and the pointer events that the buttons see.
    #[derive(Resource, Default)]
    struct Log(Vec<&'static str>);

    fn location(at: Vec2) -> Location {
        Location {
            target: NormalizedRenderTarget::TextureView(ManualTextureViewHandle(5)),
            position: at,
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

    /// Button A covers x 0 to 100, button B x 200 to 300, both y 0 to 30. The map is everything else.
    const ON_A: Vec2 = Vec2::new(50.0, 15.0);
    const ON_B: Vec2 = Vec2::new(250.0, 15.0);
    const ON_MAP: Vec2 = Vec2::new(150.0, 150.0);

    fn app() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins((PickingPlugin, InteractionPlugin, ButtonPlugin))
            .init_resource::<Log>()
            .add_observer(|_: On<Activate>, mut log: ResMut<Log>| log.0.push("activate"))
            .add_observer(|_: On<PointerOver>, mut log: ResMut<Log>| log.0.push("over"))
            .add_observer(|_: On<PointerPress>, mut log: ResMut<Log>| log.0.push("press"))
            .add_observer(|_: On<PointerClick>, mut log: ResMut<Log>| log.0.push("click"))
            .add_observer(|_: On<PointerRelease>, mut log: ResMut<Log>| log.0.push("release"));
        app.world_mut()
            .spawn((PointerId::Mouse, PointerLocation::new(location(ON_MAP))));
        let button = |app: &mut App, centre: Vec2| {
            app.world_mut()
                .spawn((
                    UiButton,
                    ComputedNode {
                        size: Vec2::new(100.0, 30.0),
                        ..default()
                    },
                    UiGlobalTransform::from(Affine2::from_translation(centre)),
                ))
                .id()
        };
        let a = button(&mut app, Vec2::new(50.0, 15.0));
        let b = button(&mut app, Vec2::new(250.0, 15.0));
        (app, a, b)
    }

    fn count(app: &App, what: &str) -> usize {
        app.world()
            .resource::<Log>()
            .0
            .iter()
            .filter(|entry| **entry == what)
            .count()
    }

    /// One frame of picking, with the maps that a backend gives for the frame: `before` is the hover
    /// of the frame before, `now` the hover of this frame. The frame sends the given inputs (an
    /// action at a position). `with_the_repair` runs the system after `pointer_events`.
    fn frame(
        app: &mut App,
        before: &[Entity],
        now: &[Entity],
        inputs: &[(PointerAction, Vec2)],
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
        for (action, at) in inputs {
            app.world_mut().write_message(PointerInput::new(
                PointerId::Mouse,
                location(*at),
                *action,
            ));
        }
        app.world_mut().run_system_cached(pointer_events).unwrap();
        app.world_mut().flush();
        if with_the_repair {
            app.world_mut()
                .run_system_cached(click_in_one_frame)
                .unwrap();
            app.world_mut().flush();
        }
        // The messages of this frame are consumed.
        app.world_mut()
            .resource_mut::<Messages<PointerInput>>()
            .clear();
    }

    const PRESS: PointerAction = PointerAction::Press(PointerButton::Primary);
    const RELEASE: PointerAction = PointerAction::Release(PointerButton::Primary);
    const MOVE: PointerAction = PointerAction::Move { delta: Vec2::ONE };

    fn pressed(app: &App, entity: Entity) -> bool {
        app.world().entity(entity).contains::<bevy::ui::Pressed>()
    }

    /// The browser order for a quick click on a button that the pointer reaches in the same frame:
    /// the move, the press and the release all come in one frame. Exactly one activation, and the
    /// events that a normal hover-then-click gives, in the same order.
    #[test]
    fn a_press_and_release_on_one_button_in_one_frame_activate_it_once() {
        let (mut app, a, _) = app();
        frame(
            &mut app,
            &[],
            &[a],
            &[(MOVE, ON_A), (PRESS, ON_A), (RELEASE, ON_A)],
            true,
        );
        assert_eq!(count(&app, "activate"), 1, "exactly one activation");
        assert!(!pressed(&app, a), "the button is not left pressed");
        let quick = app.world().resource::<Log>().0.clone();

        // The person order: a hover frame, the press in a frame, the release in a later frame.
        let (mut person, a, _) = self::app();
        frame(&mut person, &[], &[a], &[(MOVE, ON_A)], true);
        frame(&mut person, &[a], &[a], &[(PRESS, ON_A)], true);
        frame(&mut person, &[a], &[a], &[(RELEASE, ON_A)], true);
        assert_eq!(
            quick,
            person.world().resource::<Log>().0,
            "the quick click gives the events of a normal hover-then-click, in the same order"
        );
        assert_eq!(quick, ["over", "press", "click", "activate", "release"]);
    }

    /// The fault, with the repair left out: no activation and a button that stays `Pressed`. This
    /// pins the cause (`events.rs:1091`): when Bevy fixes it, this test goes red and the repair can go.
    #[test]
    fn without_the_repair_bevy_loses_the_click_and_leaves_the_button_pressed() {
        let (mut app, a, _) = app();
        frame(
            &mut app,
            &[],
            &[a],
            &[(PRESS, ON_A), (RELEASE, ON_A)],
            false,
        );
        assert_eq!(count(&app, "activate"), 0);
        assert!(pressed(&app, a));
        assert_eq!(count(&app, "over"), 1, "Bevy sends the over event");
    }

    /// A button that was hovered in the frame before, and a press and a release in one frame: Bevy
    /// clicks it itself, and the repair must not click it a second time (it would switch twice).
    #[test]
    fn a_button_that_was_hovered_before_is_clicked_once_by_bevy_not_twice() {
        let (mut app, a, _) = app();
        frame(&mut app, &[], &[a], &[(MOVE, ON_A)], true);
        frame(
            &mut app,
            &[a],
            &[a],
            &[(PRESS, ON_A), (RELEASE, ON_A)],
            true,
        );
        assert_eq!(count(&app, "activate"), 1, "one activation, not two");
        assert!(!pressed(&app, a));
    }

    /// The old order stays right: with the repair in place, a press in one frame and a release in a
    /// later frame activate once (Bevy's own click; the repair adds nothing).
    #[test]
    fn a_press_and_a_later_release_activate_once() {
        let (mut app, a, _) = app();
        frame(&mut app, &[], &[a], &[(MOVE, ON_A)], true);
        frame(&mut app, &[a], &[a], &[(PRESS, ON_A)], true);
        frame(&mut app, &[a], &[a], &[(RELEASE, ON_A)], true);
        assert_eq!(count(&app, "activate"), 1);
    }

    /// Review M1: a box select that starts on the map and ends over a button inside one frame is not a
    /// click on the button.
    #[test]
    fn a_press_on_the_map_and_a_release_over_a_button_in_one_frame_activate_nothing() {
        let (mut app, a, _) = app();
        frame(
            &mut app,
            &[],
            &[a],
            &[(PRESS, ON_MAP), (MOVE, ON_A), (RELEASE, ON_A)],
            true,
        );
        assert_eq!(count(&app, "activate"), 0, "no activation");
        assert_eq!(count(&app, "click"), 0);
        // Even a press and a release at the two places with no move between.
        let (mut app, a, _) = self::app();
        frame(
            &mut app,
            &[],
            &[a],
            &[(PRESS, ON_MAP), (RELEASE, ON_A)],
            true,
        );
        assert_eq!(count(&app, "activate"), 0);
    }

    /// A press on button A and a release on button B in one frame click neither.
    #[test]
    fn a_press_on_one_button_and_a_release_on_another_in_one_frame_activate_neither() {
        let (mut app, _, b) = app();
        frame(
            &mut app,
            &[],
            &[b],
            &[(PRESS, ON_A), (MOVE, ON_B), (RELEASE, ON_B)],
            true,
        );
        assert_eq!(count(&app, "activate"), 0);
        assert!(!pressed(&app, b), "the second button was not pressed");
    }

    /// Two buttons that touch: a press on A two pixels from B and a release on B (inside the click
    /// distance) hit different entities, so neither is clicked. The hit test of the press position
    /// on the entity does this, not the distance.
    #[test]
    fn a_press_on_one_button_and_a_release_on_the_button_next_to_it_click_neither() {
        let (mut app, a, _) = app();
        let c = app
            .world_mut()
            .spawn((
                UiButton,
                ComputedNode {
                    size: Vec2::new(100.0, 30.0),
                    ..default()
                },
                UiGlobalTransform::from(Affine2::from_translation(Vec2::new(150.0, 15.0))),
            ))
            .id();
        let (left, right) = (Vec2::new(99.0, 15.0), Vec2::new(101.0, 15.0));
        frame(
            &mut app,
            &[],
            &[c],
            &[(PRESS, left), (RELEASE, right)],
            true,
        );
        assert_eq!(count(&app, "activate"), 0);
        assert!(!pressed(&app, a));
        // The control: press and release both on the right button click it.
        let (mut app, _, _) = self::app();
        let c = app
            .world_mut()
            .spawn((
                UiButton,
                ComputedNode {
                    size: Vec2::new(100.0, 30.0),
                    ..default()
                },
                UiGlobalTransform::from(Affine2::from_translation(Vec2::new(150.0, 15.0))),
            ))
            .id();
        frame(
            &mut app,
            &[],
            &[c],
            &[(PRESS, right), (RELEASE, right)],
            true,
        );
        assert_eq!(count(&app, "activate"), 1);
    }

    /// A press and a release that are not the same button of the pointer are not one click.
    #[test]
    fn a_press_and_a_release_of_two_different_buttons_are_not_a_click() {
        let (mut app, a, _) = app();
        let secondary = PointerAction::Release(PointerButton::Secondary);
        frame(
            &mut app,
            &[],
            &[a],
            &[(PRESS, ON_A), (secondary, ON_A)],
            true,
        );
        assert_eq!(count(&app, "activate"), 0);
    }

    /// A drag by the rule of the interaction core (more than `CLICK_DRAG_PIXELS` from the press) is
    /// not a click, also when it comes back to the button.
    #[test]
    fn a_drag_that_comes_back_to_the_button_in_one_frame_is_not_a_click() {
        let (mut app, a, _) = app();
        // 10 px: farther than the 4 px of the core, whatever the constant is (a plant raises it).
        let away = ON_A + Vec2::new(10.0, 0.0);
        frame(
            &mut app,
            &[],
            &[a],
            &[(PRESS, ON_A), (MOVE, away), (RELEASE, ON_A)],
            true,
        );
        assert_eq!(count(&app, "activate"), 0);
        // A slip of the hand inside the limit is still a click.
        let (mut app, a, _) = self::app();
        let slip = ON_A + Vec2::new(3.0, 0.0);
        frame(
            &mut app,
            &[],
            &[a],
            &[(PRESS, ON_A), (MOVE, slip), (RELEASE, ON_A)],
            true,
        );
        assert_eq!(count(&app, "activate"), 1);
    }

    /// A press and a release on the same button more than the click distance apart, with no move
    /// between (a jump of the pointer): by the rule of the core this is a drag, not a click. The hit
    /// tests alone would pass (both positions are on the button), so the distance rule does this.
    #[test]
    fn a_press_and_a_release_far_apart_on_one_button_are_not_a_click() {
        let (mut app, a, _) = app();
        let (left, right) = (Vec2::new(10.0, 15.0), Vec2::new(90.0, 15.0));
        frame(
            &mut app,
            &[],
            &[a],
            &[(PRESS, left), (RELEASE, right)],
            true,
        );
        assert_eq!(count(&app, "activate"), 0);
    }

    /// Review L4: a release with no press in the same frame is Bevy's business. A press on a button, a
    /// move off it, and a release in the first frame that hovers the button again do not click it.
    /// (The repair without its press guard clicked it.)
    #[test]
    fn a_release_with_no_press_in_the_frame_is_left_to_bevy() {
        let (mut app, a, _) = app();
        frame(&mut app, &[], &[a], &[(MOVE, ON_A), (PRESS, ON_A)], true);
        assert!(pressed(&app, a));
        frame(&mut app, &[a], &[], &[(MOVE, ON_MAP)], true);
        frame(&mut app, &[], &[a], &[(RELEASE, ON_A)], true);
        assert_eq!(
            count(&app, "activate"),
            0,
            "Bevy gives no click for this release"
        );
        assert_eq!(count(&app, "click"), 0);
    }
}
