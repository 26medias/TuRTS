//! Task 8b: presentation time and lerp.

use turts_client::interp::{INTERP_DELAY_TICKS, lerp, presentation_time};
use turts_net_protocol::Pos;

#[test]
fn presentation_time_is_delayed_one_tick_and_clamped() {
    assert_eq!(INTERP_DELAY_TICKS, 1.0);
    assert_eq!(presentation_time(10, 0.0, 20), 9.0);
    assert_eq!(presentation_time(10, 0.025, 20), 9.5);
    assert_eq!(presentation_time(10, 0.2, 20), 10.0);
}

#[test]
fn lerp_endpoints() {
    let a = Pos { x: 1.0, y: -2.0 };
    let b = Pos { x: 5.0, y: 6.0 };
    assert_eq!(lerp(a, b, 0.0), a);
    assert_eq!(lerp(a, b, 1.0), b);
    assert_eq!(lerp(a, b, 0.5), Pos { x: 3.0, y: 2.0 });
}
