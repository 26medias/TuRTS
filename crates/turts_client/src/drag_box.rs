//! The live rectangle of a pointer drag: the box selection of units, and the work-area drag of a
//! terrain tool. Presentation only. It reads the press that the core adapter recorded and the
//! cursor of the window. It sends no input, changes no action, and is hidden when no drag is in
//! progress, so a picture after the release does not show it.

use crate::core_adapter::CoreRuntime;
use bevy::{prelude::*, window::PrimaryWindow};
use turts_interaction::CLICK_DRAG_PIXELS;

/// What the left drag that started on the world does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DragKind {
    /// The box selection of units.
    Selection,
    /// The work rectangle of a terrain tool that has its reference level.
    TerrainWork,
}

/// The left press that can become a drag: the press point in logical pixels, and its kind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct DragPress {
    pub(crate) at: Vec2,
    pub(crate) kind: DragKind,
}

/// The one node that shows the rectangle.
#[derive(Component)]
pub(crate) struct DragBox;

/// The rectangle of a drag from `press` to `cursor`. `None` while the pointer is inside the click
/// distance of the interaction core: that gesture is still a click.
pub(crate) fn drag_rect(press: Vec2, cursor: Vec2) -> Option<Rect> {
    (press.distance(cursor) > CLICK_DRAG_PIXELS).then(|| Rect::from_corners(press, cursor))
}

/// The border and the fill of a kind: white for the selection, amber for the terrain work.
fn tint(kind: DragKind) -> (Color, Color) {
    match kind {
        DragKind::Selection => (
            Color::srgba(1.0, 1.0, 1.0, 0.9),
            Color::srgba(1.0, 1.0, 1.0, 0.12),
        ),
        DragKind::TerrainWork => (
            Color::srgba(1.0, 0.78, 0.25, 0.95),
            Color::srgba(1.0, 0.78, 0.25, 0.14),
        ),
    }
}

/// Show the rectangle while the left button is down after a recorded press, and hide it at the
/// release, at a cancel (the adapter clears the press), and when the cursor leaves the window.
pub(crate) fn show_drag_box(
    mut commands: Commands,
    mut runtime: Option<ResMut<CoreRuntime>>,
    buttons: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut boxes: Query<
        (
            &mut Node,
            &mut Visibility,
            &mut BackgroundColor,
            &mut BorderColor,
        ),
        With<DragBox>,
    >,
) {
    let held = buttons.is_some_and(|buttons| buttons.pressed(MouseButton::Left));
    if !held && let Some(runtime) = runtime.as_mut() {
        runtime.drag_press = None;
    }
    let shown = runtime
        .as_ref()
        .and_then(|runtime| runtime.drag_press)
        .zip(windows.single().ok().and_then(Window::cursor_position))
        .and_then(|(press, cursor)| Some((drag_rect(press.at, cursor)?, press.kind)));
    let Ok((mut node, mut visibility, mut fill, mut border)) = boxes.single_mut() else {
        if shown.is_some() {
            commands.spawn((
                DragBox,
                bevy::picking::Pickable::IGNORE,
                bevy::ui::FocusPolicy::Pass,
                Node {
                    position_type: PositionType::Absolute,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                Visibility::Hidden,
                BackgroundColor(Color::NONE),
                BorderColor::all(Color::NONE),
                ZIndex(i32::MAX - 1),
            ));
        }
        return;
    };
    let Some((rect, kind)) = shown else {
        visibility.set_if_neq(Visibility::Hidden);
        return;
    };
    node.left = Val::Px(rect.min.x);
    node.top = Val::Px(rect.min.y);
    node.width = Val::Px(rect.width());
    node.height = Val::Px(rect.height());
    let (edge, inside) = tint(kind);
    *border = BorderColor::all(edge);
    fill.0 = inside;
    visibility.set_if_neq(Visibility::Inherited);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_press_inside_the_click_distance_has_no_rectangle() {
        let press = Vec2::new(100.0, 100.0);
        assert_eq!(drag_rect(press, press), None);
        assert_eq!(
            drag_rect(press, press + Vec2::new(CLICK_DRAG_PIXELS, 0.0)),
            None
        );
    }

    #[test]
    fn a_drag_gives_the_rectangle_between_the_press_and_the_cursor_in_each_direction() {
        let press = Vec2::new(100.0, 80.0);
        for cursor in [Vec2::new(160.0, 120.0), Vec2::new(40.0, 20.0)] {
            let rect = drag_rect(press, cursor).expect("a drag");
            assert_eq!(rect.min, press.min(cursor));
            assert_eq!(rect.max, press.max(cursor));
        }
    }

    fn app_with(press: Option<DragPress>, cursor: Option<Vec2>, held: bool) -> App {
        let mut app = App::new();
        let mut window = Window::default();
        window.set_cursor_position(cursor);
        app.world_mut().spawn((window, PrimaryWindow));
        let mut buttons = ButtonInput::<MouseButton>::default();
        if held {
            buttons.press(MouseButton::Left);
        }
        app.insert_resource(buttons);
        let mut runtime = CoreRuntime::default();
        runtime.drag_press = press;
        app.insert_resource(runtime);
        app.add_systems(Update, show_drag_box);
        app
    }

    fn shown(app: &mut App) -> Option<(Val, Val, Val, Val)> {
        let mut query = app
            .world_mut()
            .query_filtered::<(&Node, &Visibility), With<DragBox>>();
        let (node, visibility) = query.single(app.world()).ok()?;
        (*visibility != Visibility::Hidden).then_some((
            node.left,
            node.top,
            node.width,
            node.height,
        ))
    }

    #[test]
    fn the_box_follows_the_cursor_and_hides_at_the_release() {
        let press = DragPress {
            at: Vec2::new(100.0, 80.0),
            kind: DragKind::Selection,
        };
        let mut app = app_with(Some(press), Some(Vec2::new(160.0, 130.0)), true);
        app.update();
        app.update();
        assert_eq!(
            shown(&mut app),
            Some((Val::Px(100.0), Val::Px(80.0), Val::Px(60.0), Val::Px(50.0)))
        );
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .release(MouseButton::Left);
        app.update();
        assert_eq!(shown(&mut app), None);
        assert_eq!(app.world().resource::<CoreRuntime>().drag_press, None);
    }

    #[test]
    fn no_box_without_a_recorded_press_and_no_box_for_a_click() {
        let mut app = app_with(None, Some(Vec2::new(160.0, 130.0)), true);
        app.update();
        app.update();
        assert_eq!(shown(&mut app), None);
        let press = DragPress {
            at: Vec2::new(100.0, 80.0),
            kind: DragKind::TerrainWork,
        };
        let mut app = app_with(Some(press), Some(Vec2::new(102.0, 81.0)), true);
        app.update();
        app.update();
        assert_eq!(shown(&mut app), None);
    }
}
