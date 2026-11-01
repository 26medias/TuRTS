use bevy::prelude::{App, MinimalPlugins, Rect, Vec2};
use turts_client::join::Phase;
use turts_client::widgets::{WidgetsPlugin, guide, hit, placement};
use turts_interaction::InputEvent;
use turts_mod_api::{Anchor, At, Button};

#[test]
fn placement_resolves_each_anchor() {
    let cases = [
        (Anchor::TopLeft, Vec2::new(0.0, 0.0)),
        (Anchor::Top, Vec2::new(40.0, 0.0)),
        (Anchor::TopRight, Vec2::new(80.0, 0.0)),
        (Anchor::Left, Vec2::new(0.0, 35.0)),
        (Anchor::Centre, Vec2::new(40.0, 35.0)),
        (Anchor::Right, Vec2::new(80.0, 35.0)),
        (Anchor::BottomLeft, Vec2::new(0.0, 70.0)),
        (Anchor::Bottom, Vec2::new(40.0, 70.0)),
        (Anchor::BottomRight, Vec2::new(80.0, 70.0)),
    ];

    for (anchor, expected_min) in cases {
        let at = At {
            anchor,
            offset: (0.0, 0.0),
            pivot: None,
        };
        let rect = placement::resolve_rect(Vec2::new(100.0, 80.0), &at, Vec2::new(20.0, 10.0))
            .unwrap_or_else(|| panic!("anchor {anchor:?} did not resolve"));
        assert_eq!(rect.min, expected_min, "anchor {anchor:?}");
        assert_eq!(
            rect.max,
            expected_min + Vec2::new(20.0, 10.0),
            "anchor {anchor:?}"
        );
    }
}

#[test]
fn placement_applies_offset_and_explicit_widget_pivot() {
    let at = At {
        anchor: Anchor::BottomRight,
        offset: (-3.0, -5.0),
        pivot: Some(Anchor::TopLeft),
    };
    let rect = placement::resolve_rect(Vec2::new(100.0, 80.0), &at, Vec2::new(20.0, 10.0))
        .expect("bottom-right placement resolves");

    assert_eq!(rect.min, Vec2::new(97.0, 75.0));
    assert_eq!(rect.max, Vec2::new(117.0, 85.0));
}

#[test]
fn fitted_widget_uses_bevy_auto_measurement_with_authored_bottom_right_pivot() {
    let at = At {
        anchor: Anchor::BottomRight,
        offset: (-8.0, -6.0),
        pivot: Some(Anchor::BottomRight),
    };
    let size = turts_mod_api::Size {
        w: turts_mod_api::Dimension::Fit,
        h: turts_mod_api::Dimension::Fit,
    };
    let (node, transform) = placement::authored_node(Vec2::new(640.0, 360.0), &at, size).unwrap();
    assert_eq!(node.left, bevy::ui::Val::Px(632.0));
    assert_eq!(node.top, bevy::ui::Val::Px(354.0));
    assert_eq!(
        node.width,
        bevy::ui::Val::Auto,
        "Bevy's text/image content measurement determines fitted width"
    );
    assert_eq!(
        node.height,
        bevy::ui::Val::Auto,
        "Bevy's text/image content measurement determines fitted height"
    );
    assert_eq!(
        transform.translation,
        bevy::ui::Val2::percent(-100.0, -100.0)
    );
    assert_eq!(
        placement::resolve_dimension(turts_mod_api::Dimension::Fit, 640.0),
        None
    );
}

#[test]
fn fractional_anchor_uses_the_same_fraction_of_parent_and_widget() {
    let at = At {
        anchor: Anchor::Frac(0.25, 0.75),
        offset: (0.0, 0.0),
        pivot: None,
    };
    let rect = placement::resolve_rect(Vec2::new(100.0, 80.0), &at, Vec2::new(20.0, 10.0))
        .expect("fractional placement resolves");

    assert_eq!(rect.min, Vec2::new(20.0, 52.5));
    assert_eq!(rect.max, Vec2::new(40.0, 62.5));
}

#[test]
fn placement_rejects_invalid_geometry() {
    let at = At {
        anchor: Anchor::TopLeft,
        offset: (0.0, 0.0),
        pivot: None,
    };
    assert!(placement::resolve_rect(Vec2::new(f32::NAN, 80.0), &at, Vec2::splat(10.0)).is_none());
    assert!(placement::resolve_rect(Vec2::new(100.0, 80.0), &at, Vec2::new(-1.0, 10.0)).is_none());
    let invalid_fraction = At {
        anchor: Anchor::Frac(f32::INFINITY, 0.5),
        ..at
    };
    assert!(
        placement::resolve_rect(Vec2::new(100.0, 80.0), &invalid_fraction, Vec2::splat(10.0))
            .is_none()
    );
}

#[test]
fn guide_opens_at_first_join_and_stays_closed_after_later_rejoin() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, WidgetsPlugin));

    let mut guide_open = *app.world().resource::<guide::GuideOpen>();
    assert!(!guide_open.is_open, "guide starts closed before joining");
    guide::observe_phase(&Phase::FetchAdvertisement, &mut guide_open);
    assert!(!guide_open.is_open, "fetching does not open the guide");
    guide::observe_phase(&Phase::Joined { slot: 0 }, &mut guide_open);
    assert!(guide_open.is_open, "guide opens at the first join");

    guide::toggle_guide(&mut guide_open);
    assert!(!guide_open.is_open, "close after the first join");
    guide::observe_phase(&Phase::Joined { slot: 0 }, &mut guide_open);
    assert!(
        !guide_open.is_open,
        "a later joined phase does not reopen a closed guide"
    );
}

#[test]
fn toggle_guide_closes_and_opens() {
    let mut state = guide::GuideOpen::default();
    state.is_open = true;
    guide::toggle_guide(&mut state);
    assert!(!state.is_open, "toggle closes an open guide");
    guide::toggle_guide(&mut state);
    assert!(state.is_open, "toggle opens a closed guide");
}

#[test]
fn pointer_over_a_button_sets_over_widget() {
    let rect = Rect::from_corners(Vec2::new(10.0, 10.0), Vec2::new(40.0, 30.0));
    let mut pointer = InputEvent::PointerDown {
        button: Button::Left,
        px: Vec2::new(20.0, 15.0),
        over_widget: false,
    };
    hit::mark_over_widget(&mut pointer, &[rect]);
    assert!(matches!(
        pointer,
        InputEvent::PointerDown {
            over_widget: true,
            ..
        }
    ));

    let mut outside = InputEvent::PointerUp {
        button: Button::Left,
        px: Vec2::new(80.0, 60.0),
        over_widget: false,
    };
    hit::mark_over_widget(&mut outside, &[rect]);
    assert!(matches!(
        outside,
        InputEvent::PointerUp {
            over_widget: false,
            ..
        }
    ));
}

#[test]
fn pointer_move_over_a_widget_sets_over_widget() {
    let rect = Rect::from_corners(Vec2::new(10.0, 10.0), Vec2::new(40.0, 30.0));
    let mut pointer = InputEvent::PointerMove {
        px: Vec2::new(20.0, 15.0),
        over_widget: false,
    };
    hit::mark_over_widget(&mut pointer, &[rect]);
    assert!(matches!(
        pointer,
        InputEvent::PointerMove {
            over_widget: true,
            ..
        }
    ));
}

#[test]
fn non_pointer_events_are_unchanged_by_widget_hit_testing() {
    let mut key = InputEvent::KeyDown {
        code: "F1".into(),
        mods: Vec::new(),
    };
    hit::mark_over_widget(&mut key, &[]);
    assert!(matches!(key, InputEvent::KeyDown { code, .. } if code == "F1"));
    let mut pointer_left = InputEvent::PointerLeft;
    hit::mark_over_widget(&mut pointer_left, &[]);
    assert!(matches!(pointer_left, InputEvent::PointerLeft));
}
