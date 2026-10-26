//! Browser environment test for the web crate (Task 11), run in headless Firefox.
//!
//! A `cdylib` crate cannot be linked by its own integration tests, so this file checks the
//! environment only: the page parser lives in `turts_client` and is tested natively.

use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn webgl2_context_exists() {
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas: web_sys::HtmlCanvasElement = document
        .create_element("canvas")
        .unwrap()
        .dyn_into()
        .unwrap();
    assert!(
        canvas.get_context("webgl2").unwrap().is_some(),
        "no webgl2 context"
    );
}
