//! Authored placement resolution. Position and size literals stay in this module so every
//! primitive and source uses the same logical-pixel and UI-scale rules.

use bevy::{
    math::{Rect, Vec2},
    ui::{Node, UiRect, UiTransform, Val, Val2},
};
use turts_mod_api::{Anchor, At, Dimension, Size};

/// Resolve one widget rectangle in its parent's logical pixel space.
///
/// Task12 red-phase stub. The placement tests define the behavior before implementation.
pub fn resolve_rect(parent_size: Vec2, at: &At, size: Vec2) -> Option<Rect> {
    if !nonnegative_pair(parent_size) || !nonnegative_pair(size) {
        return None;
    }
    let anchor = anchor_point(at.anchor, parent_size)?;
    let pivot = anchor_point(at.pivot.unwrap_or(at.anchor), size)?;
    let offset = Vec2::from(at.offset);
    if !finite_pair(offset) {
        return None;
    }
    let min = anchor + offset - pivot;
    let max = min + size;
    (finite_pair(min) && finite_pair(max)).then(|| Rect::from_corners(min, max))
}

/// Build a layout node whose `Fit` axes are measured by Bevy's text/image layout.
/// Position is the authored anchor point; UiTransform applies the pivot after Bevy
/// has measured the node, so Fit never depends on guessed glyph widths.
pub fn authored_node(parent_size: Vec2, at: &At, size: Size) -> Option<(Node, UiTransform)> {
    if !nonnegative_pair(parent_size) {
        return None;
    }
    let anchor = anchor_point(at.anchor, parent_size)? + Vec2::from(at.offset);
    let pivot = at.pivot.unwrap_or(at.anchor);
    if !finite_pair(anchor) {
        return None;
    }
    Some((
        Node {
            position_type: bevy::ui::PositionType::Absolute,
            left: pixel(anchor.x),
            top: pixel(anchor.y),
            width: dimension_val(size.w, parent_size.x)?,
            height: dimension_val(size.h, parent_size.y)?,
            ..Node::default()
        },
        UiTransform::from_translation(Val2::percent(
            -anchor_fraction(pivot).x * 100.0,
            -anchor_fraction(pivot).y * 100.0,
        )),
    ))
}

/// Best authored parent extent for descendants before Bevy's text layout pass. Auto axes
/// remain available to Bevy; callers use the containing extent for anchor resolution.
pub fn authored_extent(parent_size: Vec2, size: Size) -> Vec2 {
    Vec2::new(
        known_extent(size.w, parent_size.x),
        known_extent(size.h, parent_size.y),
    )
}

/// Minimum parent extent that contains a measured child using its authored anchor and pivot.
pub fn fit_parent_extent(at: &At, child_size: Vec2) -> Option<Vec2> {
    if !nonnegative_pair(child_size) || !finite_pair(Vec2::from(at.offset)) {
        return None;
    }
    let anchor = anchor_fraction(at.anchor);
    let pivot = anchor_fraction(at.pivot.unwrap_or(at.anchor));
    let offset = Vec2::from(at.offset);
    let extent = |anchor: f32, pivot: f32, offset: f32, child: f32| {
        if !(0.0..=1.0).contains(&anchor) || !(0.0..=1.0).contains(&pivot) {
            return None;
        }
        let left = if anchor > 0.0 {
            ((pivot * child - offset) / anchor).max(0.0)
        } else if offset - pivot * child >= 0.0 {
            0.0
        } else {
            return None;
        };
        let right = if anchor < 1.0 {
            ((offset + (1.0 - pivot) * child) / (1.0 - anchor)).max(0.0)
        } else if offset + (1.0 - pivot) * child <= 0.0 {
            0.0
        } else {
            return None;
        };
        let value = left.max(right);
        value.is_finite().then_some(value)
    };
    Some(Vec2::new(
        extent(anchor.x, pivot.x, offset.x, child_size.x)?,
        extent(anchor.y, pivot.y, offset.y, child_size.y)?,
    ))
}

fn known_extent(dimension: Dimension, parent: f32) -> f32 {
    match dimension {
        Dimension::Px(value) if value.is_finite() && value >= 0.0 => value,
        Dimension::Frac(value) if value.is_finite() && value >= 0.0 => parent * value,
        Dimension::Fill => parent,
        _ => parent,
    }
}

/// Resolve dimensions that do not depend on content. Fit is measured by Bevy during UI layout.
pub fn resolve_dimension(dimension: Dimension, parent: f32) -> Option<f32> {
    if !parent.is_finite() || parent < 0.0 {
        return None;
    }
    let value = match dimension {
        Dimension::Px(value) => value,
        Dimension::Frac(value) => parent * value,
        Dimension::Fill => parent,
        Dimension::Fit => return None,
    };
    (value.is_finite() && value >= 0.0).then_some(value)
}

pub fn authored_size(window_logical_size: Vec2, ui_scale: f32) -> Option<Vec2> {
    if !nonnegative_pair(window_logical_size) || !ui_scale.is_finite() || ui_scale <= 0.0 {
        return None;
    }
    let size = window_logical_size / ui_scale;
    nonnegative_pair(size).then_some(size)
}

pub fn pixel(value: f32) -> Val {
    Val::Px(value)
}

pub fn percent(value: f32) -> Val {
    Val::Percent(value)
}

pub fn tooltip_padding() -> UiRect {
    UiRect::axes(pixel(6.0), pixel(4.0))
}

pub fn physical_to_pixel(value: f32, physical_per_authored_pixel: f32) -> Val {
    if physical_per_authored_pixel.is_finite() && physical_per_authored_pixel > 0.0 {
        pixel(value / physical_per_authored_pixel)
    } else {
        pixel(0.0)
    }
}

pub fn pixel_value(value: Val) -> Option<f32> {
    match value {
        Val::Px(value) => Some(value),
        _ => None,
    }
}

pub fn percent_value(value: Val) -> Option<f32> {
    match value {
        Val::Percent(value) => Some(value),
        _ => None,
    }
}

fn anchor_point(anchor: Anchor, size: Vec2) -> Option<Vec2> {
    let point = match anchor {
        Anchor::TopLeft => Vec2::ZERO,
        Anchor::Top => Vec2::new(0.5, 0.0),
        Anchor::TopRight => Vec2::new(1.0, 0.0),
        Anchor::Left => Vec2::new(0.0, 0.5),
        Anchor::Centre => Vec2::splat(0.5),
        Anchor::Right => Vec2::new(1.0, 0.5),
        Anchor::BottomLeft => Vec2::new(0.0, 1.0),
        Anchor::Bottom => Vec2::new(0.5, 1.0),
        Anchor::BottomRight => Vec2::ONE,
        Anchor::Frac(x, y) => Vec2::new(x, y),
    };
    let point = point * size;
    finite_pair(point).then_some(point)
}

fn anchor_fraction(anchor: Anchor) -> Vec2 {
    match anchor {
        Anchor::TopLeft => Vec2::ZERO,
        Anchor::Top => Vec2::new(0.5, 0.0),
        Anchor::TopRight => Vec2::new(1.0, 0.0),
        Anchor::Left => Vec2::new(0.0, 0.5),
        Anchor::Centre => Vec2::splat(0.5),
        Anchor::Right => Vec2::new(1.0, 0.5),
        Anchor::BottomLeft => Vec2::new(0.0, 1.0),
        Anchor::Bottom => Vec2::new(0.5, 1.0),
        Anchor::BottomRight => Vec2::ONE,
        Anchor::Frac(x, y) => Vec2::new(x, y),
    }
}

fn dimension_val(dimension: Dimension, parent: f32) -> Option<Val> {
    let value = match dimension {
        Dimension::Px(value) => Val::Px(value),
        Dimension::Frac(value) => Val::Percent(value * 100.0),
        Dimension::Fill => Val::Percent(100.0),
        Dimension::Fit => Val::Auto,
    };
    match dimension {
        Dimension::Px(value) if !value.is_finite() || value < 0.0 => None,
        Dimension::Frac(value) if !value.is_finite() || value < 0.0 => None,
        _ if !parent.is_finite() || parent < 0.0 => None,
        _ => Some(value),
    }
}

fn finite_pair(value: Vec2) -> bool {
    value.is_finite()
}

fn nonnegative_pair(value: Vec2) -> bool {
    finite_pair(value) && value.min_element() >= 0.0
}
