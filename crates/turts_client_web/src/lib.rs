//! Thin wasm32 entry for the browser client.
//!
//! All client logic lives in `turts_client`. This crate reads the page query, builds the app,
//! and installs three test hooks on `window`:
//!
//! - `window.turts_debug()` returns the debug JSON of `turts_client::debug::debug_json`. It
//!   returns an empty string until the first frame has run.
//! - `window.turts_move(x, y)` queues a move order for the units of this player. The order is
//!   sent only when the client is joined.
//! - `window.turts_memory_bytes()` returns the size of the wasm linear memory in bytes. Wasm
//!   memory never shrinks, so the last value that a run reads is its high-water mark
//!   (`tests/browser/measure.js`, design 4.5).
//!
//! The page query is `?server=<http url>&ws=<ws url>`. When it is wrong, the page gets the
//! message in `window.turts_error` and the app does not start.

use std::sync::Mutex;

use bevy::prelude::*;
use turts_client::build_app;
use turts_client::debug::{DebugState, rebuild_debug_state};
use turts_client::input::PendingMoves;
use turts_client::options::options_from_query;
use turts_net_protocol::Pos;
use wasm_bindgen::prelude::*;

/// The last debug JSON. The `window.turts_debug` closure reads it.
static DEBUG: Mutex<String> = Mutex::new(String::new());

/// The targets from `window.turts_move` that no frame has taken yet.
static MOVES: Mutex<Vec<(f32, f32)>> = Mutex::new(Vec::new());

#[wasm_bindgen(start)]
pub fn start() {
    let Some(window) = web_sys::window() else {
        return;
    };
    let query = window.location().search().unwrap_or_default();
    let options = match options_from_query(&query) {
        Ok(options) => options,
        Err(message) => {
            set_global(&window, "turts_error", &JsValue::from_str(&message));
            return;
        }
    };
    let debug = Closure::<dyn Fn() -> String>::new(|| DEBUG.lock().unwrap().clone());
    set_global(&window, "turts_debug", debug.as_ref());
    debug.forget();
    let move_to = Closure::<dyn Fn(f32, f32)>::new(|x, y| MOVES.lock().unwrap().push((x, y)));
    set_global(&window, "turts_move", move_to.as_ref());
    move_to.forget();
    let memory = Closure::<dyn Fn() -> f64>::new(memory_bytes);
    set_global(&window, "turts_memory_bytes", memory.as_ref());
    memory.forget();

    let mut app = build_app(options);
    app.add_systems(Update, drain_moves)
        .add_systems(Last, publish_debug.after(rebuild_debug_state));
    app.run();
}

fn set_global(window: &web_sys::Window, name: &str, value: &JsValue) {
    // `Reflect::set` on a window object does not fail. A failure would only lose a test hook.
    let _ = js_sys::Reflect::set(window, &JsValue::from_str(name), value);
}

/// The size of the wasm linear memory now: `memory_size(0) * 65536` (one page is 64 KiB).
#[cfg(target_family = "wasm")]
fn memory_bytes() -> f64 {
    core::arch::wasm32::memory_size::<0>() as f64 * 65_536.0
}

/// A native build has no wasm memory, and `start` never runs there. The function keeps the
/// native `cargo clippy --workspace --all-targets` build whole.
#[cfg(not(target_family = "wasm"))]
fn memory_bytes() -> f64 {
    0.0
}

/// `Update`: move the targets of `window.turts_move` into the input queue.
fn drain_moves(mut pending: ResMut<PendingMoves>) {
    let mut moves = MOVES.lock().unwrap();
    if moves.is_empty() {
        return;
    }
    pending.0.extend(moves.drain(..).map(|(x, y)| Pos { x, y }));
}

/// `Last`: copy the debug JSON, which `rebuild_debug_state` made this frame, for the page.
fn publish_debug(state: Res<DebugState>) {
    DEBUG.lock().unwrap().clone_from(&state.0);
}
