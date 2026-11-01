//! Exact text contract for the public order log writer.

#[path = "../src/orderlog.rs"]
#[allow(dead_code)]
mod orderlog;

#[test]
fn moving_then_blocked_has_exact_lines() {
    let mut output = Vec::new();
    orderlog::write_transition(&mut output, 12, 7, "Moving", None).unwrap();
    orderlog::write_transition(&mut output, 40, 7, "Blocked", Some("blocked")).unwrap();

    assert_eq!(
        String::from_utf8(output).unwrap(),
        "12,7,Moving,-\n40,7,Blocked,blocked\n"
    );
}
