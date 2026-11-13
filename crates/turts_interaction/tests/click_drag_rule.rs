//! The click against drag distance of the interaction core and its constant `CLICK_DRAG_PIXELS`
//! (read by the click repair of `turts_client`, `press_release.rs`) are one rule. The core keeps the
//! literal `4.0` in `step` (the L0-CORE proof compares `step` with the code before the split), so this
//! test pins the literal to the constant: a change of one without the other is red.

#[test]
fn the_core_uses_the_click_drag_distance() {
    let source = include_str!("../src/lib.rs");
    let literal = "start.distance(*px) > 4.0";
    assert_eq!(
        source.matches(literal).count(),
        2,
        "the core tells a drag from a click with `{literal}` in `on_pointer_move` and `on_pointer_up`"
    );
    assert_eq!(
        turts_interaction::CLICK_DRAG_PIXELS,
        4.0,
        "the constant that the client repair reads is the distance of those two lines"
    );
}
