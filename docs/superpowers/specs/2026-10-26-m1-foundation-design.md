# M1 foundation — design

Date: 2026-10-26. Status: revision 3. Gate 1: rigour and mod-builder failed revision 1; all five
lenses passed revision 2 with changes; revision 3 applies those changes. Gate 1 changes are
marked `(G1)`, re-gate changes `(G1b)`.
Spec: `RTS-Engine-Project-Specifications.md` v0.7, section 14 row M1: "Browser client,
authoritative headless server, one mod-defined unit. Acceptance: browser joins a match and
presents server results; secondary executable interoperability follows its selected packaging
approach."

Inputs: five research reports and five gate 1 reports in the scratchpad (`m1/`, `m1-gate1/`).
Studio calls are marked `(studio)`; CEO items are listed in section 9.

## 1. Intent

Prove the foundation every later milestone stands on. One authoritative headless server runs a
fixed-tick simulation. One browser client joins, sends a command, and presents what the server
decided. The units in the match come from a mod package that the server loads and the client
downloads through the same public path any third-party package uses. M1 makes the
architecture real and measurable. It is not a game.

**Why now.** Spec section 18: the core decisions are enough to begin prototypes; scripting and
networking components get selected through browser-first prototypes. Every open budget in
section 15 needs a running system to measure.

## 2. Goals, non-goals, anti-goals

**Goals**
- A browser client joins a running match and presents the server's authoritative unit position
  with interpolation. A second client sees the same movement. A client that joins late sees the
  current state.
- One `MoveTo` command: acknowledged on receipt, validated by the server, then simulated or
  rejected with a reason.
- Units and their placement come from a declarative package. The same server binary runs the
  default package or a third-party package, chosen by server configuration. (G1)
- The client downloads each client-side package from the lockfile URLs and checks its hash before
  use. (G1: replaces deviation DEV-M1-01.)
- Headless tests that go red on the builds named in section 7.
- First measurements: bytes per tick to one client; server tick p99; browser frame p99; wasm
  memory high-water; plus a synthetic 1,000-unit bytes-per-tick figure. Recorded, not judged.

**Non-goals (owner milestone)**
- Server index (M8), package cache and browser storage (M8), rooms and passwords (M9), real
  identity and reconnect (M10), spectators (M11), economy and the 16.2 roster (M2), belts (M3),
  terrain edits (M4), fog (M5), utilities (M6), combat and performance work and the GCP VM (M7),
  bots (M12).
- Scripting runtime. The language stays open (5.3). M1 packages are declarative only.
- The desktop executable (section 9).

**Anti-goals (failure even if it works)**
- The client computes gameplay truth. The client never depends on `turts_sim`.
- A `bevy*` type or a Bevy entity id appears in `turts_mod_api`, `turts_net_protocol`, or on the
  wire (D15, 5.2).
- Claiming a slot creates a gameplay entity (D27, 6.8).
- Any engine crate names a package, a content id, or a path under `mods/` (invariant 1, 4.3). (G1)
- The default package can do anything a third-party package cannot (invariant 1). (G1)
- Lockstep (D18). Client prediction of simulation state is also out at M1 (studio; spec 13.7
  says not to assume it is needed). lightyear `prediction` and `deterministic` are not compiled in.
- A test that cannot go red. Each acceptance test names the build it fails on.

## 3. Constraints

- Bevy 0.20 (vault decision 2026-10-26). Rust stable per `rust-toolchain.toml`.
- Exact versions pinned in `Cargo.lock`. Licences and notices of the lockfile inspected before
  the networking stack is kept (13.9, D32).
- Four spaces per indentation level. Clean, DRY code.
- Section 15 formats stay open. M1 uses placeholders marked as such in code and docs:
  identifier format, version scheme, package kind values, capability names, hash algorithm
  (`sha256:` prefix), archive format (reproducible ustar).
- `TICK_HZ = 20` and `INTERP_DELAY_TICKS = 1` are prototype constants, commented "measure at M1".
  No budget becomes a claim.

## 4. Architecture

Spec 0.7 alignment: 4.4 (Baseline) requires three representations, authoritative, replicated,
and visual, and a server with no meshes or animation. The crate split below is that rule made
structural. 13.7 sets the replication shape: permitted properties and changes, interpolation,
unchanged-state suppression. 13.9 sets the gate the networking spike must pass (section 8).

### 4.1 Crates (studio)

Workspace at the repository root. Crates under `crates/`. Packages under `mods/`.

| Crate | Responsibility | Depends on |
| --- | --- | --- |
| `turts_mod_api` | Content ids, package schema types (`Manifest`, `UnitDef`, `MapDef`, `VisualDef`), the `UnitRegistry` trait, `ENGINE_API_VERSION`, structured `ModError`. | serde. No `bevy*`. |
| `turts_net_protocol` | Wire messages (section 5), `NET_PROTOCOL_VERSION`. Transport-free. | `turts_mod_api`, serde. No `bevy*`. |
| `turts_sim` | Authoritative world: fixed ticks, flat map, unit spawning from `MapDef`, movement integrator, command validation. Implements `UnitRegistry`. | `turts_mod_api`, `bevy_app`, `bevy_ecs`, `bevy_time`, `bevy_math` with default features (none of them pulls a renderer; `default-features = false` breaks glam's math backend). (G1b) |
| `turts_mod_runtime` | Archive read, manifest and lockfile parse, hash, validation, diagnostics. Returns validated definitions; registers nothing itself. Also the `turts-pack` binary. | `turts_mod_api`, serde, ron, serde_json, sha2, semver, tar. No `bevy*`. |
| `turts_server` (bin, native) | `MinimalPlugins` + `ScheduleRunnerPlugin`; HTTP endpoints for advertisement, lockfile, and package bytes; lightyear server over WebSocket; snapshot and delta sending. | `turts_sim`, `turts_mod_runtime`, `turts_net_protocol`, `bevy` (`default-features = false`), lightyear (server). |
| `turts_client` (lib + native bin; `turts_client_web` is the wasm32 cdylib entry) | `DefaultPlugins`; advertisement and lockfile fetch; package download and hash check; join; `ReplicaState`; interpolation; presentation; input. | `turts_net_protocol`, `turts_mod_api`, `turts_mod_runtime`, `bevy`, lightyear (client). Never `turts_sim`. |

lightyear dependency: `default-features = false`, features `std`, `websocket`, `netcode`, and
`client` or `server`. No `replication`, `prediction`, `interpolation`, or `deterministic`. (G1)

**Forbidden edges.** Each is a test that runs
`cargo tree -p <crate> -e normal --prefix none --target <target>` and asserts the name is absent
from stdout. `-e normal` ignores dev-dependencies; the rule protects shipped binaries only. A
fixture crate at `tests/boundary-fixture/`, not a workspace member and with its own lockfile,
depends on `bevy_render`; the test runs the same check on it with `--manifest-path` and must
see it red before the workspace result is trusted. (G1, G1b)

The server's release build is `cargo build -p turts_server --release`. A workspace-wide build
unifies the client's `bevy` features into the server's; T1 checks the `-p` form. (G1b)
- `turts_sim`, `turts_server` (native): no `bevy_render`, `bevy_winit`, `bevy_pbr`.
- `turts_sim`: no `turts_mod_runtime`.
- `turts_mod_api`, `turts_net_protocol`: no crate whose name starts with `bevy`.
- `turts_client`: no `turts_sim`.
- Whole workspace: no `lightyear_prediction`, no `bevy_replicon`. Adopting the fallback stack
  amends this rule in the same commit. (G1b)

**No-content test.** `grep -rnE '"(base|acme)[:"]|(base|acme)\.tar|rifleman|mods/'
crates/ --include=*.rs --exclude-dir=tests`, with lines whose first non-blank characters are
`//`, `/*`, or `*` removed, returns nothing. T2a backs it up for content the grep cannot see. (G1, G1b)

### 4.2 Scheduling (studio)

Server: `Time::<Fixed>::from_hz(TICK_HZ)`. Received commands drain in `FixedPreUpdate`;
simulation in `FixedUpdate`; snapshot and delta build in `FixedPostUpdate`.

Headless tests: `TimeUpdateStrategy::FixedTimesteps(1)` and `app.update()`, which runs the fixed
loop once per update with the fixed clock in place. The first update runs zero fixed ticks; the
harness makes one warm-up update before any tick count. Never call
`World::run_schedule(FixedUpdate)` directly: that skips the clock swap. (G1, G1b)

Client: no `FixedUpdate` systems. Received messages land in `PreUpdate` into `ReplicaState`,
keyed by `UnitId`, holding the last received samples per unit. Interpolation and input run in
`Update`; command send in `PostUpdate`. Presentation reads `ReplicaState` and never writes it.

### 4.3 Networking foundation (studio, inside spec 6.10's prototype recommendation)

lightyear 0.31 carries our messages only; the library replicates nothing. (G1: messages only.)
Fallback: bevy_replicon 0.45 + bevy_replicon_renet2 1.0 carrying the same `turts_net_protocol`
messages. Spec 6.10 stays "not selected" until the spike in section 8 passes. lightyear's
message codec is postcard; M1 uses it.

Transport: WebSocket (MDN: widely available since July 2015). WebTransport is a later probe.
Accepted risk: lightyear 0.31 depends on `aeronet_io 0.22.0-rc.1`; pinned. Netcode `Manual`
authentication keeps the key in the client and is marked an M1 placeholder; real identity is M10.

### 4.4 Packages (studio)

Format: RON for package files (bevy_asset 0.20 already depends on ron 0.12). JSON for the
lockfile and the advertisement. Archive: reproducible ustar, read in memory. `turts-pack <dir>`
writes the archive and prints its hash. (G1)

**Package layout.**
```
manifest.ron
units/<name>.ron      gameplay definitions; the server reads these
maps/<name>.ron       placements; the server reads these
visuals/<name>.ron    presentation; only the client reads these (4.4, 5.8)
```

**Manifest**, the nine 5.5 fields with placeholder formats:

| Field | Type | M1 value for the default package | Notes |
| --- | --- | --- | --- |
| `id` | string | `"base"` | namespace of every content id in the package |
| `version` | string | `"0.1.0"` | scheme open (15) |
| `kind` | enum `Game` / `Library` | `Game` | values open (15) |
| `engine_api` | string | `"^0.1"` | constraint syntax open (15); M1 uses semver |
| `dependencies` | map | `{}` | M1 rejects a non-empty map |
| `exports` | `(units: [..], maps: [..], visuals: [..])` | paths inside the archive | |
| `context` | enum `Server` / `Client` / `Shared` | `Shared` | |
| `capabilities` | list | `[]` | names open (15) |
| `license` | string | `"LicenseRef-TBD"` | CEO item |

**UnitDef** (`units/rifleman.ron`): `id: "rifleman"` (local; no `:` allowed; the full content id
is `<package id>:<local id>`, here `base:rifleman`), `name: "Rifleman"`, `speed: 3.0` world units per second (finite, > 0, ≤ 100),
`footprint: Circle(radius: 0.4)` (finite, > 0, ≤ 50). M1 footprint variants: `Circle(radius)`
only. One world unit is one metre by convention; the grid is open.

**MapDef** (`maps/flat.ron`): `id: "flat"`, `size: (x: 64.0, y: 64.0)`,
`placements: [(unit: "base:rifleman", owner_slot: 0, pos: (x: 8.0, y: 8.0))]`. (G1)
Rules: a package exports exactly one map at M1, so the server needs no map key. Each placement's
`unit` names a unit of the same package (no dependencies at M1). `owner_slot` is below the slot
count (2). `pos` lies inside `size`. Coordinates are on the ground plane: origin at one map
corner, `x` right, `y` forward; presentation uses `y` up. (G1b)

**VisualDef** (`visuals/rifleman.ron`): `unit: "base:rifleman"`, `shape: Capsule(radius: 0.3,
height: 1.8)`. M1 shape variants, named fields only: `Capsule(radius:, height:)`,
`Cuboid(x:, y:, z:)`. Every unit has exactly one visual; the client rejects a package that
lacks one. The server never deserializes visuals. (G1, G1b)

`ENGINE_API_VERSION` is `"0.1.0"`. A complete example package is in Appendix A. (G1b)

**Diagnostics.** `ModError { package, file: Option<String>, field_path, message,
allowed: Option<Vec<String>> }`. `turts_mod_runtime` fills `file`; `register_unit_type` leaves
it `None` and the loader fills it on return. No Rust type name appears in `message`; the loader
rewrites ron errors. (G1, G1b)

**Registration.** `turts_mod_api` defines
`trait UnitRegistry { fn register_unit_type(&mut self, origin: &PackageId, def: UnitDef) ->
Result<UnitTypeId, ModError>; }`. `turts_sim` implements it. Inside the call: the local id has
no `:`; `origin:id` is unique; speed and radius are in range. `turts_sim` validates `MapDef` the
same way through `load_map(origin, map)`. (G1b) The server loads a
package with `turts_mod_runtime`, then calls `register_unit_type` for each unit and hands each
`MapDef` to `turts_sim`. There is no other path. (G1)

**Server configuration** (`server.ron`): `game_package: "<path to archive>"`,
`listen: "0.0.0.0:<port>"`, `tick_hz: 20`. The server binary has no default package. (G1)

### 4.5 Browser build (studio; from the web-platform report, probes in scratchpad `m1/webprobe`)

- **Crates.** `bevy = { version = "=0.20.0", default-features = false, features = ["3d"] }`;
  the `3d` feature includes `webgl2`. `wasm-bindgen = "=0.2.129"`. The browser entry is a thin
  crate `turts_client_web` with `crate-type = ["cdylib"]` only and `#[wasm_bindgen(start)]`; all
  logic lives in the `turts_client` library. A `cdylib` plus `rlib` crate loses `lto = "fat"` and
  grew the wasm from 44.1 MiB to 65.7 MiB in the probe.
- **Profile `wasm-release`:** inherits release; `opt-level = "z"`, `lto = "fat"`,
  `codegen-units = 1`, `strip = "debuginfo"`, `panic = "abort"`.
- **Build.** trunk 0.22.0. `index.html` holds
  `<link data-trunk rel="rust" data-cargo-profile="wasm-release" data-wasm-opt="z"/>` and a
  `copy-dir` for assets. `Trunk.toml [tools]` pins `wasm_bindgen = "0.2.129"` and
  `wasm_opt = "version_116"`. Dev: `trunk serve`. CI: `trunk build --release --offline`. The
  wasm-bindgen CLI pipeline produced a byte-identical wasm and is the fallback.
- **Graphics: WebGL2 for M1.** A WebGPU-only build panicked in headless Chrome 156 with no adapter,
  and Bevy cannot fall back from WebGPU to WebGL2 in one build. WebGPU becomes a second artefact
  when M7 needs it.
- **Window.** `canvas: Some("#turts")`, `fit_canvas_to_parent: true`,
  `prevent_default_event_handling: true`; the canvas parent has a fixed size.
- **Threads.** None. Bevy disables multi-threading on wasm32; no COOP/COEP headers at M1.
- **Test hooks.** The web crate exports `turts_debug()` returning JSON: presented units
  (`unit_id`, `content_id`, position, visual shape), connection state, and the last frame
  record for T5's oracle. T7 reads it with `playwright-core` driving `/usr/bin/google-chrome`.
  Browser API unit tests use `wasm-bindgen-test` in headless Firefox.
- **Chrome flags.** CI correctness: default software rendering (SwiftShader). Performance runs:
  `--enable-gpu --use-angle=gl`. Frame times from the two are never compared.
- **Measurements.** Memory: `memory_size(0) * 65536` sampled each second; the last value is the
  high-water mark (wasm memory never shrinks). Frame p99 from Bevy `Time` delta (performance.now
  on wasm). Download size: Resource Timing `transferSize` until the first frame.
- **Probe baseline** (empty Bevy app, Chrome 156 headless, RTX 3080, ANGLE GL): first frame
  1,311 ms; 594 frames in 10 s; frame p99 22.0 ms; wasm memory 37,683,200 bytes; 19,704,088 bytes
  before first frame uncompressed, wasm 4,425,762 bytes with brotli. The M1 server serves
  precompressed brotli. The startup payload budget stays open (spec 13).

## 5. Protocol (studio)

All messages are `turts_net_protocol` types; lightyear serializes them with postcard. No Bevy
types, no Bevy entity ids.

- **Advertisement** (HTTP GET, JSON): `{mod_id, mod_version, engine_version, protocol_version,
  lockfile_url, lockfile_hash}` (6.4).
- **Lockfile** (HTTP GET, JSON): one entry per package with the seven 5.5 fields. `urls` point at
  the server's package endpoint. (G1)
- The client checks the lockfile bytes against `lockfile_hash` from the advertisement before it
  reads them (5.5). (G1b)
- **Package bytes** (HTTP GET). The client fetches each `Client` or `Shared` package, checks its
  hash against the lockfile entry before parsing, and keeps it in memory. No cache. (G1)
- **Hello** (client, reliable): `{player_id, protocol_version, lockfile_hash}`. `player_id` is a
  random placeholder per session. Reconnect is not supported at M1 (M10). (G1)
- **Welcome** (server): `{slot_id, tick, tick_hz}`, or **JoinRejected** `{reason}` followed by
  a disconnect.
- **Snapshot** (server, reliable): `{tick, units: [{unit_id, content_id, owner_slot, pos}]}`.
  Sent after `Welcome` and when a client requests resync. (G1)
- **UnitDeltas** (server, sequenced unreliable, every tick, empty when nothing changed):
  `{tick, base_tick, changes: [{unit_id, pos}]}`. `base_tick` is the previous tick. A unit absent
  from `changes` is unchanged at `tick`. When `base_tick` differs from the last tick it applied,
  the client sends one `ResyncRequest` and enters resync. During resync it sends no further
  request and buffers incoming deltas. When the `Snapshot` (tick S) arrives, it applies it, drops
  buffered deltas with `tick` at or below S, and replays the rest in order while their
  `base_tick` chain holds. A break in the chain starts one new resync. (G1, G1b, G1c)
- **MoveTo** (client, reliable): `{seq, unit_ids, target: {x, y}}`.
- **CommandAck** (server, at once): `{seq, tick}`. Means received, not accepted (6.2). (G1)
- **CommandRejected** (server): `{seq, reason}` when validation fails (sender does not own the
  unit; target not finite or outside the map). (G1)
- **ResyncRequest** (client): answered with a `Snapshot`.

**Interpolation.** Presentation time is
`t = latest_tick + (now - receipt_time_of_latest) * tick_hz - INTERP_DELAY_TICKS`, clamped to at
most `latest_tick`. The client presents each unit between its samples at `floor(t)` and
`floor(t) + 1`, with fraction `t - floor(t)`. A unit with one sample is presented at that sample.
Lag at a frame is the server's continuous tick at the frame's wall time minus `t`. (G1b)

## 6. Match shape (studio)

One room from server configuration. Two player slots, transitions open-to-claimed only, no
password. The game package's map places units with an owner slot. Claiming a slot creates no
gameplay entity. Slot records are not gameplay entities.

## 7. Acceptance evidence (qa-lead owns the instruments)

"Gameplay entity" means an entity with a `UnitId` component. Each test names the build it fails on.

| Test | Input | Expected | Fails on |
| --- | --- | --- | --- |
| T1 boundary | forbidden-edge `cargo tree` checks; no-content grep; the boundary fixture | all absent; the fixture run is red | a crate that adds a forbidden edge or names content |
| T2a package parity | server binary started with `base.tar`, then with `acme.tar` (a different unit, speed, map, and a `Cuboid` visual) | each run spawns exactly the units its map places and no other; a native client presents those content ids with the shape from the downloaded VisualDef | a hard-coded placement or package path; a client that ignores downloaded visuals |
| T2b registration rules | the same invalid UnitDef in `base` and in `acme`; a local id with `:`; a map placing a unit of another package, an `owner_slot` of 2, a `pos` outside `size`; a package with two maps | all rejected with a `ModError` naming file and field; no `message` contains a schema type name | a namespace privilege, a skipped check, or a leaked type name |
| T2c package rejects | archives with `engine_api` mismatch, non-empty dependencies, unknown field, a manifest that disagrees with its lockfile entry (checked by the loader against the entry the server wrote); a package with a malformed visual file | each rejected with a `ModError`, no type name in `message`; the malformed visual loads on the server and the client rejects it | a lenient loader; a server that reads visuals |
| T2d hash | a package whose bytes differ by one byte from its lockfile entry; a lockfile whose bytes differ from the advertised hash; `Hello` with a wrong `lockfile_hash` | client refuses before parsing in both cases; server sends `JoinRejected` | a client or server that skips a compare |
| T3 claim creates nothing | headless server with a map that places one unit; claim slot 0 and slot 1 | gameplay entity count is 1 before and after both claims | an engine that spawns on claim, including one that re-spawns map placements |
| T4 command validation | `MoveTo` for a unit the sender does not own; target NaN; target outside the map | `CommandAck` then `CommandRejected`; unit state unchanged | a server that applies without validation |
| T5 convergence (native, localhost) | headless native client at a fixed 60 frames per second joins and sends one `MoveTo`; server logs (tick, wall time, pos); client logs (frame wall time, bracket ticks, fraction, presented pos) | every presented position lies on the segment between the server-log positions at consecutive ticks k and k+1, fraction in [0, 1]; lag p99 ≤ 2 ticks, maximum recorded; arrival equals the server's final position | a client that extrapolates, invents positions, interpolates across a gap, or stalls |
| T6 second viewer | two headless clients | the second client's presented path satisfies T5's oracle against the same server log | replication to one client only |
| T6b late join | a client joins after the unit has arrived | within 3 ticks of `Welcome` the client presents the unit at the final position | a design without catch-up; an empty presentation log |
| T8 lost delta | a client that drops one `UnitDeltas` during the move and one at the stop; a second case that also holds each `Snapshot` back by 3 ticks | exactly one `ResyncRequest` per loss; the client presents the final position in both cases | a client that never detects a missing delta; a resync storm; a loop when the `Snapshot` arrives late |
| T7 browser join | trunk-built client in headless Chromium against the real server; reads `window.turts_debug` | reaches `Welcome`; presents one unit; T5's segment oracle holds; lag recorded, not judged | a wasm build or transport that fails in the browser |
| M1 measurements | T5 run; synthetic run with 1,000 units moving | bytes per tick to one client at WebSocket frame level; server tick p99; browser frame p99; wasm memory high-water | recorded in `docs/measurements/m1.md`, not judged |

T7 runs in a separate CI job, not under `cargo test --workspace`. (G1)

## 8. Build order (producer)

1. Workspace, crates, CI (wasm32 check excludes `turts_server`; T7 job separate), T1 with its
   boundary fixture.
2. In parallel:
   - `turts_mod_api` + `turts_mod_runtime` + `turts-pack` + T2b, T2c.
   - `turts_sim` + T3, T4 (headless).
   - **Networking spike** with lightyear. Pass list, from the network seat (G1):
     P1 lockfile pinned; licences permissive; notices recorded.
     P2 server native with zero render crates; client builds for wasm32.
     P3 native client connects over `ws://`, completes Hello and Welcome; a bad `lockfile_hash`
        gets `JoinRejected` and a disconnect.
     P4 1,000 `MoveTo` each get exactly one `CommandAck`.
     P5 60 s of `UnitDeltas`, in order; bytes per tick recorded, payload and overhead apart.
     P6 with a client killed, server tick p99 stays within 10% (or 1 ms, whichever is larger)
        of the baseline run.
     P7 with a client that stops reading for 5 s, the same bound holds and the send buffer stays
        bounded.
     P8 a wasm client in headless Chromium connects to the same server.
     Any failure: evaluate the fallback on the same list. The result goes to
     `docs/measurements/m1-network-spike.md` and a vault decision note.
3. `turts_net_protocol`, server binary, client native, T2a, T2d, T5, T6, T6b, T8.
4. Browser client, T7, measurements.

## 9. CEO items

1. **Desktop executable.** D02 is Decided (browser first; executable secondary); the native
   versus wrapper choice is open. M1 delivers the browser part; the executable part waits for
   that choice. A native client already builds for tests. (G1)
2. **Default package licence.** `LicenseRef-TBD` until the repository licence is chosen.

## 10. Left to the plan

HTTP server crate for the three endpoints; trunk details from 4.5; test harness layout; the
exact `cargo tree` script.

## Appendix A. Example package `acme` (G1b)

```
acme/
    manifest.ron
    units/scout.ron
    maps/field.ron
    visuals/scout.ron
```

`manifest.ron`
```ron
(
    id: "acme",
    version: "0.1.0",
    kind: Game,
    engine_api: "^0.1",
    dependencies: {},
    exports: (
        units: ["units/scout.ron"],
        maps: ["maps/field.ron"],
        visuals: ["visuals/scout.ron"],
    ),
    context: Shared,
    capabilities: [],
    license: "LicenseRef-TBD",
)
```

`units/scout.ron`
```ron
(id: "scout", name: "Scout", speed: 6.0, footprint: Circle(radius: 0.6))
```

`maps/field.ron`
```ron
(
    id: "field",
    size: (x: 48.0, y: 48.0),
    placements: [(unit: "acme:scout", owner_slot: 0, pos: (x: 4.0, y: 4.0))],
)
```

`visuals/scout.ron`
```ron
(unit: "acme:scout", shape: Cuboid(x: 1.0, y: 0.8, z: 1.4))
```

Pack: `cargo run -p turts_mod_runtime --bin turts-pack -- acme/` writes `acme.tar` and prints
its `sha256:` hash. Run: `server.ron` with `game_package: "acme.tar"`.
