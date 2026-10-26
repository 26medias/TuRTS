//! Network spike P8 (Task 6b): the wasm client in headless Chrome.
//!
//! Page URL: `index.html?ws=127.0.0.1:<port>`. The client connects to the spike server over
//! WebSocket (the Task 6a setup shape, wasm `ClientConfig::default()`), sends `Hello` with
//! the hash `sha256:spike`, and sets `window.spike_state` each frame to `"connecting"`,
//! `"welcomed"` or `"rejected"`. `tests/browser/spike_p8.js` reads it.

use std::net::SocketAddr;

use bevy::prelude::*;
use lightyear::netcode::client_plugin::NetcodeConfig;
use lightyear::prelude::client::*;
use lightyear::prelude::*;
use turts_net_lightyear::{ProtocolPlugin, ReliableChannel, tick_duration};
use turts_net_protocol::NET_PROTOCOL_VERSION;
use turts_net_protocol::messages::*;
use wasm_bindgen::prelude::*;

const HASH: &str = "sha256:spike";

#[derive(Resource)]
struct Cfg {
    ws: SocketAddr,
    player_id: u64,
}

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
enum SpikeState {
    Connecting,
    Welcomed,
    Rejected,
}

impl SpikeState {
    fn as_str(self) -> &'static str {
        match self {
            SpikeState::Connecting => "connecting",
            SpikeState::Welcomed => "welcomed",
            SpikeState::Rejected => "rejected",
        }
    }
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    publish(SpikeState::Connecting);
    let ws = ws_from_url()?;
    // Placeholder identity per page load (design 5: random per session).
    let player_id = (js_sys::Math::random() * 9_007_199_254_740_991.0) as u64;
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(ClientPlugins {
        tick_duration: tick_duration(20),
    });
    app.add_plugins(ProtocolPlugin);
    app.insert_resource(Cfg { ws, player_id });
    app.insert_resource(SpikeState::Connecting);
    app.add_systems(Startup, connect);
    app.add_observer(on_connected);
    app.add_systems(PreUpdate, receive.after(MessageSystems::Receive));
    app.add_systems(Last, |state: Res<SpikeState>| publish(*state));
    app.run();
    Ok(())
}

/// Reads `?ws=<host:port>` from the page URL.
fn ws_from_url() -> Result<SocketAddr, JsValue> {
    let window = web_sys::window().ok_or("no window")?;
    let search = window.location().search()?;
    let ws = search
        .trim_start_matches('?')
        .split('&')
        .find_map(|pair| pair.strip_prefix("ws="))
        .ok_or("the page URL needs ?ws=<host:port>")?;
    let ws = String::from(js_sys::decode_uri_component(ws)?);
    ws.parse()
        .map_err(|e| JsValue::from_str(&format!("?ws={ws}: {e}")))
}

fn publish(state: SpikeState) {
    if let Some(window) = web_sys::window() {
        let _ = js_sys::Reflect::set(
            &window,
            &JsValue::from_str("spike_state"),
            &JsValue::from_str(state.as_str()),
        );
    }
}

// On wasm32 the aeronet `ClientConfig` is a unit struct. `::default()` keeps the Task 6a shape.
#[allow(clippy::default_constructed_unit_structs)]
fn connect(mut commands: Commands, cfg: Res<Cfg>) -> Result {
    let ws_addr = cfg.ws;
    let entity = commands
        .spawn((
            Client,
            Link::default(),
            NetcodeClient::new(
                Authentication::Manual {
                    server_addr: ws_addr,
                    client_id: cfg.player_id,
                    private_key: [0u8; 32], // M1 placeholder; format open (spec 15)
                    protocol_id: NET_PROTOCOL_VERSION as u64,
                },
                NetcodeConfig {
                    client_timeout_secs: 10,
                    token_expire_secs: -1,
                    ..Default::default()
                },
            )?,
            WebSocketClientIo::from_url(ClientConfig::default(), format!("ws://{ws_addr}")),
        ))
        .id();
    commands.trigger(Connect { entity });
    Ok(())
}

fn on_connected(
    trigger: On<Add<Connected>>,
    mut hello: Query<&mut MessageSender<Hello>>,
    cfg: Res<Cfg>,
) {
    if let Ok(mut sender) = hello.get_mut(trigger.entity) {
        sender.send::<ReliableChannel>(Hello {
            player_id: cfg.player_id,
            protocol_version: NET_PROTOCOL_VERSION,
            lockfile_hash: HASH.to_string(),
        });
    }
}

fn receive(
    mut state: ResMut<SpikeState>,
    mut welcome: Query<&mut MessageReceiver<Welcome>>,
    mut rejected: Query<&mut MessageReceiver<JoinRejected>>,
) {
    for mut receiver in welcome.iter_mut() {
        if receiver.receive().next().is_some() {
            *state = SpikeState::Welcomed;
        }
    }
    for mut receiver in rejected.iter_mut() {
        if receiver.receive().next().is_some() {
            *state = SpikeState::Rejected;
        }
    }
}
