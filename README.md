# TuRTS

![TuRTS — The Ultimate RTS concept cover](docs/assets/turts-cover.png)

[Concept art](docs/assets/turts-concept-art.png) · [Transparent title](docs/assets/turts-title.png)

[Social media kit](docs/assets/social/README.md) · [Download social images](docs/assets/social/turts-social-kit.zip)


TuRTS is a mod-first real-time strategy engine built with Rust and Bevy. It has a browser WebAssembly client, a
native client, and an authoritative headless server. Game content is delivered as packages through the public mod API.
The browser client is the primary product.

[Install](#install) · [Build](#build) · [Play the demos](#play-the-demos) · [Status](#status) · [Check](#check)

## Install

These steps are used on Linux (x86-64) with an NVIDIA GPU. macOS and Windows are not tested.

1. System packages (Debian or Ubuntu):

   ```sh
   sudo apt-get update
   sudo apt-get install -y pkg-config libx11-dev libasound2-dev libudev-dev libxkbcommon-x11-0 libwayland-dev libxkbcommon-dev brotli
   ```

2. [rustup](https://rustup.rs). `rust-toolchain.toml` selects the Rust version (1.99.1), clippy, rustfmt and the
   `wasm32-unknown-unknown` target.
3. Python 3, Node.js 20 or later, and Google Chrome or Chromium with WebGPU.
4. The source and the build tools (Trunk, `wasm-bindgen-cli`, `wasm-opt`, and profiling tools), without `sudo`:

   ```sh
   git clone https://github.com/26medias/TuRTS.git
   cd TuRTS
   bash scripts/setup-dev.sh
   ```

   The script also runs `rustup update stable`.

## Build

Run these from the repository root. Build each Bevy binary with its own Cargo command.

```sh
cargo build --release -p turts_server --locked          # headless server: target/release/turts_server
cargo build --release -p turts_client --locked          # native client:   target/release/turts_client
(cd crates/turts_client_web && trunk build --release)   # browser client:  crates/turts_client_web/dist
```

You do not need these commands to play a demo: the demo commands build what they need.

## Play the demos

Each demo uses two commands. The first builds the server, the package and the browser client into a folder that you
choose. The first build in a fresh clone takes 10 to 20 minutes; the next ones take about a minute each. The second
command starts a local server and opens the game in a separate Chrome window with a new, empty profile. Your own Chrome
profile is not used. Click the game once to start. Close the window or press Ctrl+C to stop. Each start is a new
match.

```sh
bash scripts/mp1-play.sh --prepare /tmp/turts/quarry_ridge --package mods/quarry_ridge
bash scripts/mp1-play.sh --prepared /tmp/turts/quarry_ridge --package mods/quarry_ridge
```

To play another package, replace `quarry_ridge` in both commands. After you change the source, run `--prepare` again:
`--prepared` refuses a folder that does not match the source.

**Linux and WebGPU.** Chrome on Linux does not offer a WebGPU adapter by default. The play script starts Chrome with
`--enable-unsafe-webgpu --enable-features=Vulkan --use-angle=vulkan`. If you open the printed URL in a Chrome that you
start yourself, use the same flags, or the game does not start.

**Common controls.** Click a unit or drag a box to select. Right-click the ground to move. Move the camera with the
arrow keys or the screen edge, and zoom with the mouse wheel.

### Skirmish against the computer

Build a base, harvest timber, train an army, and destroy the enemy castle.

```sh
bash scripts/skirmish-play.sh --prepare /tmp/turts/skirmish
bash scripts/skirmish-play.sh --prepared /tmp/turts/skirmish
```

Select your castle, click **Build Barracks**, and click free ground near the castle. Send a villager to a timber stand
with a right-click. Train Crossbowmen and Swordsmen in the Barracks, then right-click an enemy to attack. Click or drag
on the minimap (lower right) to look around; right-click it to send the selected units. The first computer attack
starts after 180 seconds. More controls, and two human players on one computer (`--human`):
[package guide](mods/skirmish/README.md).

### Quarry Ridge: dig a passage for a mixed army

`mods/quarry_ridge`: 24 small Walkers, 12 large Crawlers, and a ridge with a narrow opening. Use the two commands at
the start of this section.

1. Drag a box round all your units and right-click the far side of the ridge. The Walkers go through the opening. The
   Crawlers wait.
2. Select the Walkers, click **Dig**, and drag an area. Start the drag on the low ground at the foot of the ridge, and
   drag over the ridge round the opening. The area is lowered to the height where the drag started. Marked cells show
   what changes. Right-click or Esc cancels.
3. When the passage is wide enough, the waiting Crawlers go on with their first order.
4. Select all units and send the group back through the passage.

**Fill** raises ground up to the height where the drag starts. A tool button acts for the first selected unit type
only: select the Crawlers alone to dig with them. Their tool is slower and also digs iron.

### Terrain Stockyard Ridge

`mods/terrain_stockyard_ridge`: the same exercise with Porters (small) and Carriers (large). The tools are **Scraper**
(small units), **Borer** (large units) and **Leveler** (fill).

```sh
bash scripts/mp1-play.sh --prepare /tmp/turts/terrain_stockyard_ridge --package mods/terrain_stockyard_ridge
bash scripts/mp1-play.sh --prepared /tmp/turts/terrain_stockyard_ridge --package mods/terrain_stockyard_ridge
```

### Other playable packages

Use the two `mp1-play.sh` commands with the package name.

| Package | What to try |
| --- | --- |
| `quarry` | Terrain work with cargo: dig, unload at the Material depot when the worker is full, and fill. [Guide](mods/quarry/README.md) |
| `terrain_stockyard` | The same terrain work; dug material goes directly to your balance. [Guide](mods/terrain_stockyard/README.md) |
| `utility_yard` | Underground power lines: press U, connect the generator to the workshop, then dig a line open and cover it again. [Guide](mods/utility_yard/README.md) |
| `forge` | A factory: a drill, belts, arm loaders and a press make shells; research better shells in the Proving Lab for the turret |
| `base` | Gather resources with a Harvester and produce units at the Command Center |
| `stockyard` | An economy with timber as the only cost, made for the public bot interface. [Guide](mods/stockyard/README.md) |
| `muster` | A larger development scene with troopers, workers and training targets, with an in-game guide (F1) |

`acme`, `cedar`, `haul` and `visibility_probe` are test packages.

### Run a server yourself

```sh
cargo build --release -p turts_server --locked
(cd crates/turts_client_web && trunk build --release)
RUN_DIR="$(mktemp -d)"
cp -a mods/forge "$RUN_DIR/forge"
cargo run --release -p turts_pack --bin turts-pack --locked -- "$RUN_DIR/forge"
cat > "$RUN_DIR/server.ron" <<EOF
(
    game_package: "$RUN_DIR/forge.tar",
    http_listen: "127.0.0.1:7000",
    ws_listen: "127.0.0.1:7001",
    public_url: "http://127.0.0.1:7000",
    tick_hz: 20,
    static_dir: "$PWD/crates/turts_client_web/dist",
)
EOF
target/release/turts_server "$RUN_DIR/server.ron"
```

Open `http://127.0.0.1:7000/?server=http%3A%2F%2F127.0.0.1%3A7000&ws=ws%3A%2F%2F127.0.0.1%3A7001` in a Chrome with
WebGPU, or join with the native client:

```sh
target/release/turts_client --server http://127.0.0.1:7000 --ws ws://127.0.0.1:7001 --player-id 1
```

`turts_server --help` lists the options and the configuration fields.

## Status

The target is the full RTS engine in the [project specification](RTS-Engine-Project-Specifications.md).

Accepted milestones of spec section 14 (20 in all): `███████░░░░░░░░░░░░░` 7 of 20.

| Milestones | State |
| --- | --- |
| M1, M2, M3, QA0, M4, MS, M6 | Accepted |
| M4b (group movement on terrain) | Done and playable; a person's acceptance is not recorded. [Results and known limits](docs/measurements/m4b.md) |
| MC, MP1, MP2 | Capabilities delivered; no recorded acceptance by a person |
| **M5 (discovery, with unit fog of war)** | **Next** |
| MB, M7, M8, M9, M10, M11, M12, M13 | Open |

The count is of milestones, not of work. Milestones differ in size, so it gives no date.

M4b known limits, in short: a unit that a standing body stops can end its move far from its goal; a unit that is pushed
does not always return to its place; a terrain tool button acts for the first selected unit type only.

| Capability | Verified progress | Evidence and remaining work |
| --- | --- | --- |
| Core simulation and mod content | Authoritative simulation, package loading, gathering, production, logistics, research, and combat are implemented. | Forge and Skirmish exercise these systems. Incremental payment, repair, cancellation, and safe destruction refunds pass native and browser acceptance. |
| Playable skirmish | Building, harvesting, paid mixed armies, rally points, Victory, Defeat, and restart pass browser checks. | [Normal-stat win and loss report](docs/reports/2026-11-14-full-skirmish.md). Full game content remains open. |
| Human multiplayer | Two browser clients, public HTTPS/WSS hosting, ready lobby, colour selection, and reload recovery pass scripted checks. | [Two-player report](docs/reports/2026-11-14-two-player-skirmish.md), [lobby report](docs/reports/2026-11-14-ready-lobby.md), [rejoin report](docs/reports/2026-11-14-fast-rejoin.md). A full match between two people remains unverified. |
| Presentation | Models, animations, effects, minimap, ground blending, and selected-unit portraits and health are delivered. | [Ground report](docs/reports/2026-11-14-ground-blending.md), [selection report](docs/reports/2026-11-14-unit-selection-panel.md). Custom shader support remains open. |
| Browser performance | All nine 400-unit High cases pass at 59.9-60.0 FPS, including fog and both zoom limits. The last 2,000-unit High check reached 28.81 FPS on an earlier build. | [Visibility measurements](docs/measurements/shared-visibility.md), [larger-model measurements](docs/measurements/models.md). No new 2,000-unit or hosting-capacity claim. |
| Damage system | Type tables, bonuses, optional armour curves, owner armour upgrades, automatic acquisition, Warriors, and Guard towers pass acceptance. | [Acceptance, independent review, and normal win](docs/reports/2026-11-14-damage-system.md). API 0.2.12; protocol 7. An independent test checks upgrade order and privacy in snapshots and deltas. The default game has no research lab yet. |
| Operating resources | Mod-defined slot supply, exact shortage rates, weapon Disable, and periodic income pass native and browser checks. | [Acceptance and limits](docs/reports/2026-11-15-operating-resources.md). API 0.2.13; protocol 7. The default Skirmish balance stays unchanged. Gathered accounting and its display remain open; a future counter test must exclude periodic income. |
| Paid work and repair | Exact incremental payment, construction cancellation, safe destruction refunds, and manual/automatic repair pass feature tests and the full browser fixture. | [Acceptance progress](docs/reports/2026-11-15-paid-work-and-repair.md). API 0.2.14; protocol 8. Acceptance and independent foundation review pass. Research ChargeOverTime and individual held-request cancellation remain open. |
| Public mod bots | Portable profiles, public Rust controllers, bounded observations, exact account coordination, shared command admission, and headless recorded/regenerated replay are implemented. | [SDK15 report](docs/reports/2026-11-15-public-bot-foundation.md). Final Skirmish and Stockyard browser probes pass. BF8 passed at 4.906851 ms p99 before later financial/profile fixes; those fixes were not remeasured. Full human/reference readiness remains open. |
| Shared visibility | Authored fog, team sharing, bounded stale sightings, hidden-target command rules, and atomic permitted human/bot state pass acceptance. | [SDK16 report](docs/reports/2026-11-15-shared-visibility.md). API 0.2.16; protocol 9. Independent code-running review, 2,924 native tests, full network acceptance, 60,000 measured server ticks, nine High camera cases, and scripted player probes pass. |
| Editable terrain | Plateau heights, excavation/fill, private seeded geology, persistent depletion, physical cargo, explicit unloading, permitted surface memory, raised picking, and traversable passages pass acceptance. | [M4 evidence and limits](docs/reports/2026-11-15-terrain-work-orders.md). API 0.2.17; protocol 10. Both public terrain packages, independent code-running review, 80,000 final measured ticks, three High400 views and scripted cold-play probes pass. Fully exposed million-cell maps exceed the decoded-state guard. |
| Dense group movement (M4b) | One flock movement path with movement rules (traffic waits, a passage holder, the slower unit first, pushed idle units go back to their place, busy units stand firm), arrive-first points, terrain work for large units, and a drag gesture for terrain work. FM1 to FM3 and the frame rate row (48.0 to 56.2 FPS, 400 detailed models) pass in the browser on game code c1dc1e60. The browser movement campaign passes 16 of 24 cases. | [M4b results and known limits](docs/measurements/m4b.md). No local path round standing bodies yet (task T-59). |
| Scripting language (MS) | Rune 0.14.2 is selected from measured native and Chrome-wasm runs against budgets set before the runs. Rune adds 13.6 µs (native) and 60 µs (Chrome) script p99 for each tick at 1,000 units and 16 events, and 1,003,799 gzip bytes. | [MS report](docs/reports/2026-11-16-scripting-selection.md), [measurements](docs/measurements/scripting.md). Spike only: no production runtime and no engine change. The selection depends on one configuration decision (core Rune modules only; spec amendment 5). The CEO declined the MPL dependency, so Rune remains selected. Standard license notices still apply. |
| Underground utilities (M6) | Mod-defined utility types, explicit junctions, surface ports, fixed line elevation, terrain-dependent burial with mod exposure policy, aggregate electrical supply per network, and a browser utility view with place, connect and inspect. | [M6 report](docs/reports/2026-11-16-underground-utilities.md). API 0.2.18; protocol 11. Two-viewer wire privacy, real paid terrain edits, independent code-running review, real-input browser case and three scripted cold-play probes pass. No water, sewage or heat model; bots do not use utilities. |
| Remaining engine scope | Geological discovery and unit discovery rules (M5), the production script runtime, rooms, durable identity and saves, statistics, and spectator and replay delivery remain open. | M5 is the next milestone. The full project specification remains the target. |

## Check

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown --locked
cargo test --workspace --exclude turts_server --locked
CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true CARGO_PROFILE_RELEASE_OVERFLOW_CHECKS=true cargo test -p turts_server --release --locked
bash scripts/boundary.sh
bash scripts/acceptance.sh
```

Run the server tests in release: their debug build takes hours. The browser suites and their exact steps are in
[the CI workflow](.github/workflows/ci.yml). A test browser must use a new profile and `--alsa-output-device=null`.

Design and scope: [RTS Engine Project Specifications](RTS-Engine-Project-Specifications.md). Developer setup and crate
commands: [CLAUDE.md](CLAUDE.md). Working rules: [AGENTS.md](AGENTS.md). Measurements:
[M2](docs/measurements/m2.md), [M3](docs/measurements/m3.md), [M4b](docs/measurements/m4b.md),
[models](docs/measurements/models.md). Playtest guide: [M3](vault/M3%20Playtest%20Guide.md).
