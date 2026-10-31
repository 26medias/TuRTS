use std::path::Path;

#[path = "../examples/effective_bindings.rs"]
mod effective_bindings;

#[test]
fn forge_effective_bindings_include_p1_keys_and_authored_order_actions() {
    let effective = effective_bindings::effective_bindings(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/forge"),
    )
    .expect("the checked-in Forge package must produce an effective input map");
    assert_eq!(
        effective["actions"]["group_assign_1"],
        serde_json::json!([{"Key":{"code":"Digit1","mods":["Ctrl"]}}])
    );
    assert_eq!(
        effective["actions"]["menu"],
        serde_json::json!([{"Key":{"code":"F10","mods":[]}},{"Key":{"code":"Escape","mods":[]}}])
    );
    assert_eq!(
        effective["actions"]["toggle_guide"],
        serde_json::json!([{"Key":{"code":"F1","mods":[]}}])
    );
    assert_eq!(
        effective["orders"],
        serde_json::json!({
            "forge:order_cancel":[{"Key":{"code":"KeyC","mods":[]}}],
            "forge:order_disable":[{"Key":{"code":"KeyD","mods":[]}}],
            "forge:order_enable":[{"Key":{"code":"KeyE","mods":[]}}],
            "forge:order_hold":[{"Key":{"code":"KeyH","mods":[]}}],
            "forge:order_move":[{"Key":{"code":"KeyM","mods":[]}}],
            "forge:order_remove":[{"Key":{"code":"KeyX","mods":[]}}],
            "forge:order_research":[{"Key":{"code":"KeyR","mods":[]}}],
            "forge:order_stop":[{"Key":{"code":"KeyS","mods":[]}}]
        }),
        "effective bindings must retain every binding under each qualified authored order id"
    );
}
