# M1 foundation — design

Date: 2026-10-26. Status: revision 2 after gate 1 (rigour and mod-builder: fail; technical
director, network, mod runtime: pass with changes). Changes from gate 1 are marked `(G1)`.
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
- Lockstep, or client prediction of simulation state (D18). lightyear `prediction` and
  `deterministic` are not compiled in.
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
| `turts_sim` | Authoritative world: fixed ticks, flat map, unit spawning from `MapDef`, movement integrator, command validation. Implements `UnitRegistry`. | `turts_mod_api`, `bevy_app`, `bevy_ecs`, `bevy_time`, `bevy_math` with `default-features = false`. |
| `turts_mod_runtime` | Archive read, manifest and lockfile parse, hash, validation, diagnostics. Returns validated definitions; registers nothing itself. Also the `turts-pack` binary. | `turts_mod_api`, serde, ron, serde_json, sha2, semver, tar. No `bevy*`. |
| `turts_server` (bin, native) | `MinimalPlugins` + `ScheduleRunnerPlugin`; HTTP endpoints for advertisement, lockfile, and package bytes; lightyear server over WebSocket; snapshot and delta sending. | `turts_sim`, `turts_mod_runtime`, `turts_net_protocol`, `bevy` (`default-features = false`), lightyear (server). |
| `turts_client` (bin, wasm32 and native) | `DefaultPlugins`; advertisement and lockfile fetch; package download and hash check; join; `ReplicaState`; interpolation; presentation; input. | `turts_net_protocol`, `turts_mod_api`, `turts_mod_runtime`, `bevy`, lightyear (client). Never `turts_sim`. |

lightyear dependency: `default-features = false`, features `std`, `websocket`, `netcode`, and
`client` or `server`. No `replication`, `prediction`, `interpolation`, or `deterministic`. (G1)

**Forbidden edges.** Each is a test that runs
`cargo tree -p <crate> -e normal --prefix none --target <target>` and asserts the name is absent
from stdout. A fixture crate under `tests/boundary-fixture/` adds one forbidden edge so the test
is shown red before it is trusted. (G1)
- `turts_sim`, `turts_server` (native): no `bevy_render`, `bevy_winit`, `bevy_pbr`.
- `turts_sim`: no `turts_mod_runtime`.
- `turts_mod_api`, `turts_net_protocol`: no crate whose name starts with `bevy`.
- `turts_client`: no `turts_sim`.
- Whole workspace: no `lightyear_prediction`, no `bevy_replicon`.

**No-content test.** `grep -rnE '"(base|acme)[:"]|mods/' crates/ --include=*.rs` excluding
`tests/` directories returns nothing. (G1)

### 4.2 Scheduling (studio)

Server: `Time::<Fixed>::from_hz(TICK_HZ)`. Received commands drain in `FixedPreUpdate`;
simulation in `FixedUpdate`; snapshot and delta build in `FixedPostUpdate`.

Headless tests: `TimeUpdateStrategy::FixedTimesteps(1)` and `app.update()`, which runs the fixed
loop exactly once with the fixed clock in place. Never `World::run_schedule(FixedUpdate)`
directly: that skips the clock swap. (G1)

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

**UnitDef** (`units/rifleman.ron`): `id: "rifleman"` (local; the full content id is
`base:rifleman`), `name: "Rifleman"`, `speed: 3.0` world units per second (finite, > 0, ≤ 100),
`footprint: Circle(radius: 0.4)` (finite, > 0, ≤ 50). M1 footprint variants: `Circle(radius)`
only. One world unit is one metre by convention; the grid is open.

**MapDef** (`maps/flat.ron`): `id: "flat"`, `size: (64.0, 64.0)`,
`placements: [(unit: "base:rifleman", owner_slot: 0, pos: (8.0, 8.0))]`. (G1)

**VisualDef** (`visuals/rifleman.ron`): `unit: "base:rifleman"`, `shape: Capsule(radius: 0.3,
height: 1.8)`. M1 shape variants: `Capsule(radius, height)`, `Cuboid(x, y, z)`. The server never
deserializes visuals. (G1)

**Diagnostics.** `ModError { package, file, field_path, message, allowed: Option<Vec<String>> }`.
No Rust type name appears in `message`. (G1)

**Registration.** `turts_mod_api` defines
`trait UnitRegistry { fn register_unit_type(&mut self, origin: &PackageId, def: UnitDef) ->
Result<UnitTypeId, ModError>; }`. `turts_sim` implements it. Inside the call: the content id's
namespace equals `origin`; the id is unique; speed and radius are in range. The server loads a
package with `turts_mod_runtime`, then calls `register_unit_type` for each unit and hands each
`MapDef` to `turts_sim`. There is no other path. (G1)

**Server configuration** (`server.ron`): `game_package: "<path to archive>"`,
`listen: "0.0.0.0:<port>"`, `tick_hz: 20`. The server binary has no default package. (G1)

### 4.5 Browser build (studio; from the web-platform report)

PENDING: filled from the web-platform-engineer report. Expected: trunk; WebGL2 default with
WebGPU opt-in; canvas `Window`; `wasm-release` profile; headless Chromium driver; a debug hook
`window.turts_debug` that exports the presented units for T7; `performance.now` frame p99 and
`WebAssembly.Memory` high-water.

## 5. Protocol (studio)

All messages are `turts_net_protocol` types; lightyear serializes them with postcard. No Bevy
types, no Bevy entity ids.

- **Advertisement** (HTTP GET, JSON): `{mod_id, mod_version, engine_version, protocol_version,
  lockfile_url, lockfile_hash}` (6.4).
- **Lockfile** (HTTP GET, JSON): one entry per package with the seven 5.5 fields. `urls` point at
  the server's package endpoint. (G1)
- **Package bytes** (HTTP GET). The client fetches each `Client` or `Shared` package, checks its
  hash against the lockfile entry before parsing, and keeps it in memory. No cache. (G1)
- **Hello** (client, reliable): `{player_id, protocol_version, lockfile_hash}`. `player_id` is a
  random placeholder per session. Reconnect is not supported at M1 (M10). (G1)
- **Welcome** (server): `{slot_id, tick, tick_hz}`, or **JoinRejected** `{reason}` followed by
  a disconnect.
- **Snapshot** (server, reliable): `{tick, units: [{unit_id, content_id, owner_slot, pos}]}`.
  Sent after `Welcome` and when a client requests resync. (G1)
- **UnitDeltas** (server, sequenced unreliable, one message per tick): `{tick, changes:
  [{unit_id, pos}]}`, changed units only. (G1)
- **MoveTo** (client, reliable): `{seq, unit_ids, target: {x, y}}`.
- **CommandAck** (server, at once): `{seq, tick}`. Means received, not accepted (6.2). (G1)
- **CommandRejected** (server): `{seq, reason}` when validation fails (sender does not own the
  unit; target not finite or outside the map). (G1)
- **ResyncRequest** (client): sent when a delta's base tick is unknown.

**Interpolation.** The client presents time `t = latest_tick - INTERP_DELAY_TICKS` (fractional),
between the two received samples that bracket `t`. It never presents past the newest sample.

## 6. Match shape (studio)

One room from server configuration. Two player slots, transitions open-to-claimed only, no
password. The game package's map places units with an owner slot. Claiming a slot creates no
gameplay entity. Slot records are not gameplay entities.

## 7. Acceptance evidence (qa-lead owns the instruments)

"Gameplay entity" means an entity with a `UnitId` component. Each test names the build it fails on.

| Test | Input | Expected | Fails on |
| --- | --- | --- | --- |
| T1 boundary | forbidden-edge `cargo tree` checks; no-content grep; the boundary fixture | all absent; the fixture run is red | a crate that adds a forbidden edge or names content |
| T2a package parity | server binary started with `base.tar`, then with `acme.tar` (a different unit, speed, map) | each run spawns its own package's units; a native client presents the right content ids | a hard-coded placement or package path |
| T2b registration rules | the same invalid UnitDef in namespace `base` and in `acme`; a UnitDef whose namespace differs from its package | all rejected with a `ModError` naming file and field | a namespace privilege or a skipped check |
| T2c package rejects | archives with `engine_api` mismatch, non-empty dependencies, unknown field, manifest that disagrees with its lockfile entry | each rejected with a `ModError` | a lenient loader |
| T2d hash | client fetches a package whose bytes differ by one byte from the lockfile hash; separately, Hello with a wrong `lockfile_hash` | client refuses before parsing; server sends `JoinRejected` | a client or server that skips the compare |
| T3 claim creates nothing | headless server with a map that places zero units; claim slot 0 and slot 1 | gameplay entity count stays 0 | an engine that spawns on claim |
| T4 command validation | `MoveTo` for a unit the sender does not own; target NaN; target outside the map | `CommandAck` then `CommandRejected`; unit state unchanged | a server that applies without validation |
| T5 convergence (native, localhost) | headless native client joins, sends one `MoveTo`; server logs (tick, wall time, pos); client logs (frame time, bracket ticks, fraction, presented pos) | every presented position lies on the segment between two consecutive received samples with fraction in [0, 1]; presentation lag ≤ 2 ticks; arrival matches the server's final position | a client that extrapolates, invents positions, or stalls |
| T6 second viewer | two headless clients | the second client's presented path satisfies T5's oracle against the same server log | replication to one client only |
| T6b late join | a client joins after the unit has arrived | first presented position equals the final position | a design without catch-up |
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
     P6 a killed client does not change server tick p99.
     P7 a client that stops reading for 5 s does not change server tick p99; send buffer bounded.
     P8 a wasm client in headless Chromium connects to the same server.
     Any failure: evaluate the fallback on the same list. The result goes to
     `docs/measurements/m1-network-spike.md` and a vault decision note.
3. `turts_net_protocol`, server binary, client native, T2a, T2d, T5, T6, T6b.
4. Browser client, T7, measurements.

## 9. CEO items

1. **Desktop executable.** D02 is Decided (browser first; executable secondary); the native
   versus wrapper choice is open. M1 delivers the browser part; the executable part waits for
   that choice. A native client already builds for tests. (G1)
2. **Default package licence.** `LicenseRef-TBD` until the repository licence is chosen.

## 10. Left to the plan

HTTP server crate for the three endpoints; trunk details from 4.5; test harness layout; the
exact `cargo tree` script.
