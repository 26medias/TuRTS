//! Task12 pointer capture test seam.

use bevy::math::Rect;
use turts_interaction::InputEvent;

/// Set `over_widget` for pointer events whose logical point falls inside a visible widget.
pub fn mark_over_widget(event: &mut InputEvent, rects: &[Rect]) {
    let (point, over_widget) = match event {
        InputEvent::PointerMove { px, over_widget }
        | InputEvent::PointerDown {
            px, over_widget, ..
        }
        | InputEvent::PointerUp {
            px, over_widget, ..
        } => (Some(*px), over_widget),
        _ => return,
    };
    let Some(point) = point.filter(|point| point.is_finite()) else {
        return;
    };
    *over_widget |= rects.iter().any(|rect| rect.contains(point));
}
