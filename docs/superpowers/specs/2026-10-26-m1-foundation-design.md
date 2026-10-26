# M1 foundation — design

Date: 2026-10-26. Status: draft for gate 1. Spec: `RTS-Engine-Project-Specifications.md` v0.7,
section 14 row M1: "Browser client, authoritative headless server, one mod-defined
unit. Acceptance: browser joins a match and presents server results; secondary executable
interoperability follows its selected packaging approach."

Inputs: five research reports (network-engineer, producer, technical-director,
mod-runtime-engineer, web-platform-engineer), scratchpad `m1/`. Studio calls are marked
`(studio)`; items for the CEO are marked `(CEO)`.

## 1. Intent

Prove the foundation every later milestone stands on: one authoritative headless server runs a
fixed-tick simulation; one browser client joins, sends a command, and presents what the server
decided; the one unit in the match is defined by a mod package loaded through the same public
path a third-party package will use. Nothing else. M1 exists to make the architecture real and
measurable, not to be a game.

**Why now.** Spec section 18 says the core decisions are enough to begin prototypes and that
scripting and networking components get selected through browser-first prototypes. M1 is that
prototype. Every open numeric budget in section 15 needs a running system to measure.

## 2. Goals, non-goals, anti-goals

**Goals**
- A browser client joins a running match over the network and presents the server's authoritative
  unit position with interpolation; a second browser client sees the same movement.
- One `MoveTo` command, validated by the server, acknowledged at once, then simulated.
- One unit type from a declarative mod package (`mods/base/`), registered through the public API.
- A lockfile with one entry, hash-checked by the client, so invariant 10 (session compatibility)
  is testable from day one.
- Headless tests that go red on the builds the producer named (section 7).
- First measurements: bytes per tick to one client; server tick time; browser frame time and
  memory high-water mark.

**Non-goals (owner milestone)**
- Server index (M8), rooms and passwords (M9), real identity and reconnect (M10), spectators
  (M11), economy and the 16.2 roster (M2), belts (M3), terrain edits (M4), fog (M5), utilities
  (M6), combat and performance work and the GCP VM (M7), bots (M12).
- Package download, cache, browser storage (M8). See deviation DEV-M1-01.
- Scripting runtime. The language stays open (5.3). M1 is declarative only.
- Desktop executable. See section 9, CEO item 1.

**Anti-goals (failure even if it works)**
- The client computes any gameplay truth of its own. The client never depends on `turts_sim`.
- A Bevy entity id, schedule label, or any `bevy*` type crosses the public mod API or the wire
  envelope the mod API exposes (D15, 5.2). Library replication frames carry Bevy ids; they stay
  behind the engine's own envelope.
- Claiming a slot creates an entity (D27, D28). The map places the unit with an owner slot.
- The default mod gets a load path a third-party package cannot use (invariant 1).
- Lockstep or client prediction of simulation state (D18). Lightyear's `deterministic` and
  `prediction` features stay off.
- A test that cannot go red. Every acceptance test names the build it fails on.

## 3. Constraints

- Bevy 0.20 (decision note 2026-10-26). Rust stable per `rust-toolchain.toml`.
- Exact versions pinned in `Cargo.lock` (D35 spirit; D32 licence check on the lockfile).
- Four spaces per indentation level. Clean, DRY code (4.2).
- Section 15 formats stay open; M1 uses placeholders that are marked as such in code and docs:
  identifier format, version scheme, package kind values, capability names, hash algorithm
  (`sha256:` placeholder), archive format (reproducible ustar read in memory).
- No numeric budget becomes a claim. `TICK_HZ = 20` is a configurable prototype constant with
  the comment "measure at M1".

## 4. Architecture

Spec 0.7 alignment: section 4.4 (Baseline) requires three representations, authoritative,
replicated, and visual, and a server with no meshes or animation. The crate split in 4.1 below
is that rule made structural. Section 13.7 sets the replication shape M1's protocol follows
(permitted properties and changes, interpolation, unchanged-state suppression). Section 13.9
sets the selection gate the networking spike must pass before lightyear is kept: a pinned
mutually compatible stack, licences and notices inspected, wasm and headless verified, and the
behaviour tested with our scenario.

### 4.1 Crates (studio)

Workspace at the repository root, crates under `crates/`, the default mod under `mods/base/`.

| Crate | Responsibility | Depends on |
| --- | --- | --- |
| `turts_mod_api` | Versioned content ids, unit definition schema, command and query envelopes, `ENGINE_API_VERSION`. | serde only. No `bevy*`. |
| `turts_net_protocol` | Wire messages (section 5), `NET_PROTOCOL_VERSION`. Transport-free. | `turts_mod_api`, serde. No `bevy*`. |
| `turts_sim` | Authoritative world: fixed ticks, flat map stub, one unit, movement integrator, command validation. | `turts_mod_api`, `bevy_app`, `bevy_ecs`, `bevy_time`, `bevy_math`. |
| `turts_mod_runtime` | Manifest and lockfile parse, archive read, hash check, declarative load, `register_unit_type`. | `turts_mod_api`, serde, ron, serde_json, sha2, semver, tar. No `bevy*`, no scripts. |
| `turts_server` (bin, native) | `MinimalPlugins` + `ScheduleRunnerPlugin`; HTTP advertisement and lockfile endpoints; WebSocket transport; replication. | `turts_sim`, `turts_mod_runtime`, `turts_net_protocol`, lightyear (server features). |
| `turts_client` (bin, wasm32 and native) | `DefaultPlugins`; join; presents replicated state; input to commands. | `turts_net_protocol`, `turts_mod_api`, `turts_mod_runtime`, lightyear (client features). Never `turts_sim`. |

Forbidden edges, each enforced by a test that runs `cargo tree` and asserts a zero count:
`turts_sim` on `bevy_render`, `bevy_winit`, `bevy_pbr`; `turts_sim` on `turts_mod_runtime`;
`turts_mod_api` and `turts_net_protocol` on any `bevy*`; `turts_client` on `turts_sim`.

### 4.2 Scheduling (studio)

Server: `Time::<Fixed>::from_hz(TICK_HZ)`. Commands drain in `FixedPreUpdate`; simulation in
`FixedUpdate`; snapshot build in `FixedPostUpdate`. Headless tests call
`World::run_schedule(FixedUpdate)` directly, with `TimeUpdateStrategy` fixed.

Client: no `FixedUpdate` systems. Received snapshots land in `PreUpdate` into a `ReplicaState`
resource keyed by stable unit id; interpolation and input in `Update`; command send in
`PostUpdate`. Presentation reads `ReplicaState` and never writes it.

### 4.3 Networking foundation (studio, inside spec 6.10's prototype recommendation)

lightyear 0.31 with the `websocket` transport and `replication` on; `prediction` and
`deterministic` off. Fallback: bevy_replicon 0.45 + bevy_replicon_renet2 1.0 over WebSocket.
Spec 6.10 stays "not selected" until the bytes-per-tick measurement exists. The library is an
internal carrier; the public envelope is `turts_net_protocol`.

Transport: WebSocket for M1 (MDN: widely available since 2015). WebTransport is a follow-up probe,
not the M1 path. Risk accepted: lightyear 0.31 depends on an aeronet release candidate; pinned.

### 4.4 Mod package v0 (studio)

Format: RON for the manifest and unit definitions (bevy_asset 0.20 already depends on ron 0.12).
JSON for the lockfile, a protocol document. Archive: reproducible ustar, read in memory.

Manifest, the nine 5.5 fields with placeholder formats: `id: "m1test"`, `version: "0.1.0"`,
`kind: Game`, `engine_api: "^0.1"`, `dependencies: {}`, `exports: (units: [...])`,
`context: Shared`, `capabilities: []`, `license: "LicenseRef-TBD"`.

Unit `m1test:rifleman`: `speed: 3.0` world units per second (engine converts per tick),
footprint `Circle(radius: 0.4)`, visual `Primitive(Capsule(0.3, 1.8))`. Engine mechanism:
navigation stub, collision stub, movement integrator, unit-type registry. Mod policy: speed,
footprint, name, visual (4.3).

Server load path: read archive, hash, check `engine_api`, reject non-empty dependencies at M1,
write the lockfile entry (seven fields per 5.5), parse and validate each unit, call the public
`register_unit_type`. The test fixture package in `turts_mod_runtime/tests/fixtures/` goes through
the same call.

**DEV-M1-01 (studio deviation, retire at M8):** the client build embeds the same package archive.
The join handshake carries the lockfile entry. The client hashes the embedded bytes and refuses
the join on mismatch. No download, cache, or browser storage at M1. Invariant 10 stays testable.

### 4.5 Browser build (studio; from the web-platform report)

PENDING: filled from the web-platform-engineer report before gate 1. Expected content: trunk or
wasm-bindgen CLI; WebGL2 default with WebGPU as an opt-in feature; canvas `Window` setup;
`wasm-release` profile; headless Chromium driver for tests; `performance.now` tick p99 and
`WebAssembly.Memory` high-water measurement.

## 5. Protocol (studio)

All messages are `turts_net_protocol` types, serde-serialized with a compact binary codec
(postcard or bincode; the plan picks one and measures). No Bevy types.

- **Advertisement** (HTTP GET, JSON): `{mod_id, mod_version, engine_version, protocol_version,
  lockfile_url, lockfile_hash}` (spec 6.4).
- **Lockfile** (HTTP GET, JSON): entries with the seven 5.5 fields. One entry at M1.
- **Hello** (client, reliable): `{identity: PlayerId placeholder, protocol_version, lockfile_hash}`.
- **Welcome** (server): `{slot_id, tick, tick_hz}` or **Reject** `{reason}`. Reconnect with the
  same identity restores the slot (D52 spirit; real identity is M10).
- **MoveTo** (client, reliable): `{seq, unit_ids: [UnitId], target: {x, y}}`.
- **Ack** (server, at once): `{seq, tick}` (immediate input acknowledgement, 6.2). Then the
  server validates ownership and applies on the next tick.
- **UnitState** (server, unreliable, prioritised): `{unit_id, content_id, owner_slot, pos: {x, y},
  tick}`, only for changed units. Interest at M1: every unit in the match.
- Client interpolation: between the last two received ticks, rendered `tick_hz` behind.

## 6. Match shape (studio, from the producer's cut)

One room from server config. Two player slots, transitions open-to-claimed only, no password.
Placeholder `PlayerId` (random per session). One flat map that places one `m1test:rifleman`
owned by slot 0. The second slot owns nothing and watches. Claiming a slot creates zero entities.

## 7. Acceptance evidence (qa-lead owns the instruments)

Each test names the build on which it fails.

| Test | Input | Expected | Fails on |
| --- | --- | --- | --- |
| T1 boundary | `cargo tree` per forbidden edge | count 0 | any crate that adds a forbidden edge |
| T2 package parity | load `mods/base` and a third-party fixture through `register_unit_type` | both register one unit type; a one-byte change in either archive fails the hash check | a private load path, or a hash that is not checked |
| T3 claim creates nothing | headless server; claim slot 1 | entity count unchanged | an engine that spawns on claim |
| T4 reject malformed | `MoveTo` with a unit the sender does not own, and one with a NaN target | `Reject`, state unchanged | a server that applies without validation |
| T5 convergence | headless native client joins, sends one `MoveTo`; record server position per tick and client presented position per frame | after `Ack`, presented position equals authoritative position of tick T within 2 ticks for every tick until arrival; the client never presents a position the server did not emit | a client that extrapolates or a server that stops sending |
| T6 second viewer | two headless clients | the second client's presented path equals the first's within 2 ticks | replication that targets one client |
| T7 browser join | trunk-built client in headless Chromium, real server | the page reaches `Welcome`, renders one unit, and T5's check passes against the browser's presented positions | a wasm build or transport that fails in the browser |
| M1 measurements | T5 run | bytes per tick to one client; server tick p99; browser frame p99; `WebAssembly.Memory` high-water | recorded, not judged (section 15 budgets stay open) |

## 8. Build order (producer)

1. Workspace, crates, CI fix (wasm32 check excludes `turts_server`), T1.
2. In parallel: `turts_mod_api` + `turts_mod_runtime` + T2; `turts_sim` + T3 + T4 (headless);
   networking spike with lightyear over WebSocket (native server, native client) that proves the
   fallback is not needed.
3. `turts_net_protocol`, server binary, T5, T6 with native headless clients.
4. Browser client, T7, measurements.

## 9. CEO items

1. **Desktop executable.** The M1 acceptance row names a secondary executable; D02 is open. The
   studio accepts the browser part of M1 now and blocks the executable part on D02. Recommendation:
   decide D02 after M1 measurements exist; a native Bevy client already builds from `turts_client`
   for tests, so the wrapper question can wait.
2. **Default mod licence.** Placeholder `LicenseRef-TBD` until the repository licence is chosen.
3. (Resolved) The origin thread's v0.6 is merged as 0.7.

## 10. Left to the plan

Codec choice (postcard vs bincode); HTTP server crate for the two endpoints; lightyear channel
configuration; test harness crate layout; trunk versus wasm-bindgen CLI details from section 4.5.
