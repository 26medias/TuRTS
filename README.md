# TuRTS

TuRTS is a mod-first real-time strategy engine built with Rust and Bevy. It has a browser WebAssembly client, a native client, and an authoritative headless server. Game content is delivered as packages through the public mod API.

**Project stage: multiplayer graybox prototype. M2 Tasks 1–16 are accepted.** The current prototype demonstrates package loading, two-player sessions, resource gathering, unit production and cancellation, resynchronization after loss, and convergence of public game state across clients. The interface is functional prototype UI.

The prototype scope includes selecting owned units with the left mouse button, issuing movement or Gather orders with the right mouse button, and using visible producer controls to Produce or Cancel jobs. Browser and native-window proofs covered these interactions against the real server and package fixtures. The demo match has two permanent player slots; a browser tab and native client count toward the same two slots. Closing a client does not release or reconnect its slot. Restart the server to begin a fresh match.

Combat, construction and placement, power networks, gameplay AI, and victory or endgame flows are future work. Steam distribution is not prepared, and the visuals are not a finished art pass.

## Try the base package locally

The following starts a local server that serves the browser build and the package advertisement, lockfile, and archive. It copies the package to a temporary directory so the packer writes its archive outside the repository.

Requirements: Rust toolchain, `wasm32-unknown-unknown` target, Trunk 0.22, and a browser with WebGPU enabled. On Linux, the tested proof used headed Chrome under Xvfb with Vulkan/WebGPU flags; adapter availability depends on the host and browser.

From the repository root, in the first terminal:

```sh
ROOT="$(pwd)"
RUN_DIR="$(mktemp -d)"
cp -a "$ROOT/mods/base" "$RUN_DIR/base"

# Build each Bevy binary in a separate Cargo command.
cargo build --release -p turts_server
cargo build --release -p turts_client
cargo run --release -p turts_pack --bin turts-pack -- "$RUN_DIR/base"
(cd "$ROOT/crates/turts_client_web" && trunk build --release)

cat > "$RUN_DIR/server.ron" <<EOF
(
    game_package: "$RUN_DIR/base.tar",
    http_listen: "127.0.0.1:7000",
    ws_listen: "127.0.0.1:7001",
    public_url: "http://127.0.0.1:7000",
    tick_hz: 20,
    static_dir: "$ROOT/crates/turts_client_web/dist",
)
EOF

"$ROOT/target/release/turts_server" "$RUN_DIR/server.ron"
```

Open this URL in the browser:

```text
http://127.0.0.1:7000/?server=http%3A%2F%2F127.0.0.1%3A7000&ws=ws%3A%2F%2F127.0.0.1%3A7001
```

To join with native clients instead, start one or two clients in separate terminals:

```sh
target/release/turts_client --server http://127.0.0.1:7000 --ws ws://127.0.0.1:7001 --player-id 1
target/release/turts_client --server http://127.0.0.1:7000 --ws ws://127.0.0.1:7001 --player-id 2
```

The server assigns the match slots during join and serves the package from its advertisement and lockfile. No separate lockfile editing is needed. The browser address and server fields above match the configuration schema in `turts_server --help`.

## Build and check

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown --locked
cargo test --workspace --locked
bash scripts/boundary.sh
bash scripts/acceptance.sh
```

The browser proof uses headed Chrome under private Xvfb with Vulkan/WebGPU launch settings from CI. `bash scripts/m2-browser.sh` runs all 15 registered browser cases; list them with `bash scripts/m2-browser.sh --list-cases`.

Design and scope: [RTS Engine Project Specifications](RTS-Engine-Project-Specifications.md). Developer setup and crate commands: [CLAUDE.md](CLAUDE.md). Current proof and measurements: [M2 measurement record](docs/measurements/m2.md).
