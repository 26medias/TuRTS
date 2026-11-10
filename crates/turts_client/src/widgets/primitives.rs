//! Bevy presentation helpers for the mod-authored widget primitive schema.

use bevy::prelude::*;
use turts_mod_api::{Fill, TextAlign};

use super::placement;

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct AuthoredWidgetId(pub String);

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WidgetAction {
    Input { name: String },
    Order { id: String },
    Select { unit: u32 },
    Produce { producer: u32, content_id: String },
    CancelProduction { producer: u32, job: u64 },
    CenterCamera { x: f32, y: f32 },
}

#[derive(Component, Debug, Clone, PartialEq)]
pub struct WidgetButtonAction(pub WidgetAction);

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct WidgetTooltip(pub String);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetEnabled(pub bool);

#[derive(Component, Debug, Clone, PartialEq)]
pub struct WidgetFill(pub Option<Fill>);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidgetText;

/// A positioned absolute node in the authored parent's logical-pixel space.
pub fn absolute_node(rect: bevy::math::Rect) -> Node {
    Node {
        position_type: PositionType::Absolute,
        left: placement::pixel(rect.min.x),
        top: placement::pixel(rect.min.y),
        width: placement::pixel(rect.width()),
        height: placement::pixel(rect.height()),
        ..default()
    }
}

pub fn fill_colour(fill: Option<&Fill>) -> Option<Color> {
    match fill? {
        Fill::Colour(value) => parse_colour(value),
        Fill::Image { .. } => None,
    }
}

pub fn parse_colour(value: &str) -> Option<Color> {
    let hex = value.strip_prefix('#')?;
    let (r, g, b, a) = match hex.len() {
        3 => (
            u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?,
            u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?,
            u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?,
            255,
        ),
        4 => (
            u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?,
            u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?,
            u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?,
            u8::from_str_radix(&hex[3..4].repeat(2), 16).ok()?,
        ),
        6 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            255,
        ),
        8 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
            u8::from_str_radix(&hex[6..8], 16).ok()?,
        ),
        _ => return None,
    };
    Some(Color::srgba_u8(r, g, b, a))
}

pub fn text_alignment(align: TextAlign) -> Justify {
    match align {
        TextAlign::Left => Justify::Left,
        TextAlign::Centre => Justify::Center,
        TextAlign::Right => Justify::Right,
    }
}

pub fn direction(value: turts_mod_api::Direction) -> FlexDirection {
    match value {
        turts_mod_api::Direction::Horizontal | turts_mod_api::Direction::Right => {
            FlexDirection::Row
        }
        turts_mod_api::Direction::Left => FlexDirection::RowReverse,
        turts_mod_api::Direction::Vertical | turts_mod_api::Direction::Down => {
            FlexDirection::Column
        }
        turts_mod_api::Direction::Up => FlexDirection::ColumnReverse,
    }
}

/// Geometry of an authored bar fill in its containing node.
pub fn bar_fill_node(direction: turts_mod_api::Direction, ratio: f32) -> Node {
    let ratio = if ratio.is_finite() {
        ratio.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let horizontal = matches!(
        direction,
        turts_mod_api::Direction::Horizontal
            | turts_mod_api::Direction::Left
            | turts_mod_api::Direction::Right
    );
    Node {
        position_type: PositionType::Absolute,
        left: if direction == turts_mod_api::Direction::Left {
            placement::percent((1.0 - ratio) * 100.0)
        } else {
            placement::pixel(0.0)
        },
        top: if direction == turts_mod_api::Direction::Up {
            placement::percent((1.0 - ratio) * 100.0)
        } else {
            placement::pixel(0.0)
        },
        width: if horizontal {
            placement::percent(ratio * 100.0)
        } else {
            placement::percent(100.0)
        },
        height: if horizontal {
            placement::percent(100.0)
        } else {
            placement::percent(ratio * 100.0)
        },
        ..default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directional_bar_fill_uses_fraction_and_correct_origin() {
        let cases = [
            (turts_mod_api::Direction::Horizontal, 0.0, 0.0, 40.0, 100.0),
            (turts_mod_api::Direction::Right, 0.0, 0.0, 40.0, 100.0),
            (turts_mod_api::Direction::Left, 60.0, 0.0, 40.0, 100.0),
            (turts_mod_api::Direction::Vertical, 0.0, 0.0, 100.0, 40.0),
            (turts_mod_api::Direction::Down, 0.0, 0.0, 100.0, 40.0),
            (turts_mod_api::Direction::Up, 0.0, 60.0, 100.0, 40.0),
        ];
        for (direction, left, top, width, height) in cases {
            let node = bar_fill_node(direction, 0.4);
            let percent = |value| placement::percent_value(value).expect("percent value");
            let pixels = |value| placement::pixel_value(value).expect("pixel value");
            let left_value = if direction == turts_mod_api::Direction::Left {
                percent(node.left)
            } else {
                pixels(node.left)
            };
            let top_value = if direction == turts_mod_api::Direction::Up {
                percent(node.top)
            } else {
                pixels(node.top)
            };
            assert!(
                (left_value - left).abs() < 0.001,
                "{direction:?} left: {:?}",
                node.left
            );
            assert!(
                (top_value - top).abs() < 0.001,
                "{direction:?} top: {:?}",
                node.top
            );
            assert!(
                (percent(node.width) - width).abs() < 0.001,
                "{direction:?} width: {:?}",
                node.width
            );
            assert!(
                (percent(node.height) - height).abs() < 0.001,
                "{direction:?} height: {:?}",
                node.height
            );
        }
        assert_eq!(
            bar_fill_node(turts_mod_api::Direction::Right, f32::NAN).width,
            placement::percent(0.0)
        );
    }
}
