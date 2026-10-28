# TuRTS

![TuRTS — The Ultimate RTS concept cover](docs/assets/turts-cover.png)

[Concept art](docs/assets/turts-concept-art.png) · [Transparent title](docs/assets/turts-title.png)

[Social media kit](docs/assets/social/README.md) · [Download social images](docs/assets/social/turts-social-kit.zip)


TuRTS is a mod-first real-time strategy engine built with Rust and Bevy. It has a browser WebAssembly client, a native client, and an authoritative headless server. Game content is delivered as packages through the public mod API.

**Project stage: multiplayer industrial graybox prototype. M3 features are implemented; final milestone acceptance is in progress.** The engine supports package loading, two-player sessions, gathering, unit production and cancellation, extraction, conveyors, inserters, item recipes, research, ammunition-based combat and destruction. Clients recover through snapshots and deltas. The interface and visuals remain functional prototypes.

The prototype scope includes selecting owned units with the left mouse button, issuing movement or Gather orders with the right mouse button, and using visible producer controls to Produce or Cancel jobs. Browser and native-window proofs covered these interactions against the real server and package fixtures. The demo match has two permanent player slots; a browser tab and native client count toward the same two slots. Closing a client does not release or reconnect its slot. Restart the server to begin a fresh match.

Construction and placement, power networks, gameplay AI, and victory or endgame flows are future work. Steam distribution is not prepared, and the visuals are not a finished art pass.

## Try Forge locally

The following starts a local server that serves the browser build and the package advertisement, lockfile, and archive. It copies the package to a temporary directory so the packer writes its archive outside the repository.

Requirements: Rust toolchain, `wasm32-unknown-unknown` target, Trunk 0.22, and a browser with WebGPU enabled. On Linux, the tested proof used headed Chrome under Xvfb with Vulkan/WebGPU flags; adapter availability depends on the host and browser.

From the repository root, in the first terminal:

```sh
ROOT="$(pwd)"
RUN_DIR="$(mktemp -d)"
DEMO_PACKAGE=forge
cp -a "$ROOT/mods/$DEMO_PACKAGE" "$RUN_DIR/$DEMO_PACKAGE"

# Build each Bevy binary in a separate Cargo command.
cargo build --release -p turts_server
cargo build --release -p turts_client
cargo run --release -p turts_pack --bin turts-pack -- "$RUN_DIR/$DEMO_PACKAGE"
(cd "$ROOT/crates/turts_client_web" && trunk build --release)

cat > "$RUN_DIR/server.ron" <<EOF
(
    game_package: "$RUN_DIR/$DEMO_PACKAGE.tar",
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

## The Forge demonstration

The first joined client owns a factory laid out by the package. Extraction, belts, inserters and the press run automatically. Select units to inspect stock, Main/Output inventory ports and status. Select the Proving Lab and use **Research Hardened Shells** when ten unreserved shells are available. After completion, the turret’s damage rises from 10 to 20.

The second joined client owns the two Target Drones. The nearby drone takes ten 10-damage hits. After the factory player confirms research completion, move the farther drone into turret range; it takes five 20-damage hits. Research is private to its owner. This is a factory/combat demonstration, with no victory condition or player construction.

Visible **Disable/Enable** controls pause eligible units. **Remove** is available only where the package permits it; removal destroys the unit’s contents. The prototype uses simple shapes and text panels, rather than finished game artwork.

Set `DEMO_PACKAGE=base` in the launch commands for the original gathering and unit-production demonstration. Start each test match with a fresh server; the two permanent slots are shared by browser tabs and native clients.

For a guided first session, see the [M3 playtest guide](vault/M3%20Playtest%20Guide.md).

## Build and check

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown --locked
cargo test --workspace --locked
bash scripts/boundary.sh
bash scripts/acceptance.sh
bash scripts/m3-refutations.sh
bash scripts/m3-browser.sh
```

The browser proof uses headed Chrome under private Xvfb with Vulkan/WebGPU launch settings from CI. `bash scripts/m2-browser.sh` runs all 15 registered browser cases; list them with `bash scripts/m2-browser.sh --list-cases`.

Design and scope: [RTS Engine Project Specifications](RTS-Engine-Project-Specifications.md). Developer setup and crate commands: [CLAUDE.md](CLAUDE.md). Proof and measurements: [M3 measurement record](docs/measurements/m3.md) and [M2 measurement record](docs/measurements/m2.md).
