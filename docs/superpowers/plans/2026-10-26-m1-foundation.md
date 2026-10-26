# M1 Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One authoritative headless server, one browser client, and one mod-defined unit that move together, with every acceptance test of the design able to go red.

**Architecture:** Seven workspace crates plus one shared lightyear adapter crate. `turts_mod_api` and `turts_net_protocol` hold serde-only types. `turts_sim` runs the fixed-tick world headless and implements `UnitRegistry`. `turts_mod_runtime` reads archives and validates packages. `turts_server` joins sim, runtime, HTTP endpoints and the lightyear server. `turts_client` holds all client logic; `turts_client_web` is a thin wasm32 entry. Mods live under `mods/` and reach the engine only through the public registration path.

**Tech Stack:** Rust stable (rust-toolchain.toml), Bevy 0.20.0, lightyear 0.31.0 (messages only, WebSocket, netcode), ron 0.12.2, serde 1.0.229, sha2 0.11.1, semver 1.0.28, tar 0.4.46, axum 0.8.9, tokio 1.54.1, tower-http 0.7.1, trunk 0.22.0, wasm-bindgen 0.2.129, playwright-core 1.64.1.

**Spec:** `docs/superpowers/specs/2026-10-26-m1-foundation-design.md` (revision 3, gate 1 passed, with the section 5 and T8 resync amendment). Read it in full before any task. Section numbers below refer to it unless marked "spec", which means `RTS-Engine-Project-Specifications.md` v0.7.

**Version evidence.** Every version in this plan comes from one of: the gate probes in the scratchpad (`m1/lyprobe`, `m1/probe`, `m1/webprobe`, `m1/wbt`, `m1-gate1/bevyprobe`, `m1-gate1/simprobe`, `m1/tarprobe`), the lightyear 0.31.0 and Bevy 0.20.0 sources in the local cargo registry, or crates.io on 2026-10-26. Items marked `NOT TRACED` were not run; the task that meets one verifies it first.

## Global Constraints

- Bevy `=0.20.0`. Rust stable per `rust-toolchain.toml` (targets `wasm32-unknown-unknown`). Edition 2024.
- Every dependency pinned with `=` in `[workspace.dependencies]`; `Cargo.lock` committed.
- lightyear: `default-features = false`, features `std`, `websocket`, `netcode`, plus `client` or `server`. Never `replication`, `prediction`, `interpolation`, `deterministic` (section 4.1, 2).
- Forbidden edges (section 4.1): `turts_sim`, `turts_server`: no `bevy_render`, `bevy_winit`, `bevy_pbr`. `turts_sim`: no `turts_mod_runtime`. `turts_mod_api`, `turts_net_protocol`: no crate whose name starts with `bevy`. `turts_client`: no `turts_sim`. Workspace: no `lightyear_prediction`, no `bevy_replicon`.
- No engine crate names a package, a content id, or a path under `mods/` (section 4.1 no-content test). Test code under a `tests` directory is exempt.
- No `bevy*` type and no Bevy entity id in `turts_mod_api`, `turts_net_protocol`, or on the wire.
- Claiming a slot creates no entity with a `UnitId` component (T3).
- `DEFAULT_TICK_HZ = 20` and `INTERP_DELAY_TICKS = 1.0` carry the comment `// prototype constant: measure at M1`.
- Placeholders (section 3) are marked `// M1 placeholder; format open (spec 15)` in code: identifier format, version scheme, package kind values, capability names, hash prefix `sha256:`, archive format ustar, netcode `Manual` key.
- Hash string format: `sha256:` plus 64 lowercase hex characters, over the exact archive bytes.
- `ENGINE_API_VERSION = "0.1.0"`, `NET_PROTOCOL_VERSION: u32 = 1`.
- Four spaces per indentation level, including Rust, RON, YAML, JavaScript. `cargo fmt` with `tab_spaces = 4` (default).
- Prose in ASD-STE100 Simplified Technical English. No emoji.
- Never `git add -A`. Stage explicit paths. Do not commit while a review agent runs.
- Each commit message ends with the attribution lines the executing session was given.
- A test that cannot go red is a defect. Every acceptance test names the build it fails on and the plan names the planted defect used to prove it.

## Review Focus

1. Two clients send `Hello` with the same `player_id`. Expected: the second gets `JoinRejected { reason: "player already in match" }` and the first keeps its slot (D25). Test: Task 4 `claim_same_player_twice_is_rejected`; Task 7 `hello_duplicate_player_is_rejected`.
2. An archive holds an entry whose path starts with `/` or holds `..`, or holds the same path twice. Expected: `PackageArchive::parse` returns a `ModError` with `field_path = "archive"` and never writes to disk (spec 6.5 "constrain extraction"). Test: Task 3 `archive_rejects_unsafe_and_duplicate_paths`.
3. The lockfile lists a package with context `Server`. Expected: the client never requests its URL (spec 6.5 "does not download server-only packages"). Test: Task 8a `client_skips_server_only_packages`.
4. A `MoveTo` with an empty `unit_ids`, more than 256 ids, or a duplicate id. Expected: `CommandRejected` with reason `empty_command`, `too_many_units`, or `duplicate_unit`; state unchanged. Test: Task 4 `validate_move_to_rejects_malformed_lists`.
5. A `UnitDeltas` names a `unit_id` the client has no replica for. Expected: the client treats it as a chain break: one `ResyncRequest`, no panic, no invented unit. Test: Task 8b `delta_with_unknown_unit_requests_resync`.

---

## File structure

```
Cargo.toml                                workspace, pinned versions, wasm-release profile
rust-toolchain.toml                       exists
.github/workflows/ci.yml                  modify: wasm32 check excludes turts_server; browser job
.cargo/config.toml                        wasm32 test runner (wasm-bindgen-test-runner)
scripts/boundary.sh                       T1 cargo tree checks, fixture must be red
scripts/acceptance.sh                     builds client bin then runs the server's acceptance tests
crates/turts_checks/                      tests only: T1 boundary, no-content grep
crates/turts_mod_api/src/{lib,ids,schema,error,registry,lock}.rs
crates/turts_mod_runtime/src/{lib,archive,parse,hash,manifest,lock,pack}.rs
crates/turts_mod_runtime/src/bin/turts-pack.rs
crates/turts_sim/src/{lib,plugin,registry,world,map,movement,command,slots,harness}.rs
crates/turts_net_protocol/src/{lib,messages,advertisement}.rs
crates/turts_net_lightyear/src/{lib,channels,protocol_plugin}.rs    one registration order for both ends
crates/turts_net_lightyear/src/bin/{spike_server,spike_client,tcp_relay}.rs   feature "spike"
crates/turts_net_lightyear/tests/spike.rs
crates/turts_server/src/{main,config,boot,http,net,session,outbound,ticklog,stats,synthetic}.rs
crates/turts_server/tests/acceptance.rs + tests/support/{mod,oracle,logs}.rs
crates/turts_client/src/{lib,options,fetch,acquire,join,replica,interp,ingest,present,framelog,render,input,debug}.rs
crates/turts_client/src/main.rs           native bin: window or --headless
crates/turts_client_web/src/lib.rs        cdylib, #[wasm_bindgen(start)], turts_debug
crates/turts_client_web/{index.html,Trunk.toml,tests/web.rs}
tests/boundary-fixture/                   not a member; depends on bevy_render; must be red
tests/spike-web/                          not a member; P8 wasm spike client
tests/browser/{package.json,t7.js,measure.js}
mods/base/{manifest.ron,units/rifleman.ron,maps/flat.ron,visuals/rifleman.ron}
mods/acme/{manifest.ron,units/scout.ron,maps/field.ron,visuals/scout.ron}
docs/measurements/{m1-network-spike,m1}.md
vault/Decisions/2026-10-26 M1 networking stack.md
```

**Plan refinements of the design** (state them in the commit that introduces each):

- R1. A shared crate `turts_net_lightyear` holds the lightyear `ProtocolPlugin`. lightyear assigns message and channel ids by registration order, so one crate guarantees that both ends register in the same order. `turts_net_protocol` stays free of `bevy*`.
- R2. `server.ron` has five fields: `game_package`, `listen` (HTTP), `ws_listen` (WebSocket), `public_url`, `tick_hz`, plus optional `static_dir`. HTTP and WebSocket cannot share one TCP listener. The advertisement keeps the spec 6.4 fields only; the client takes the WebSocket URL as its own argument.
- R3. Test hooks (`--drop-delta-ticks`, `--hold-snapshot-ticks`, `--frame-log`) compile into the native client binary without a feature flag, so `cargo test --workspace` tests the shipped binary. The web crate has none of them.
- R4. Command results travel from `turts_sim` in a `CommandResults` resource, not a Bevy message, so the sim exposes no schedule detail.

## Parallel groups and models

| Group | Tasks | Runs after |
| --- | --- | --- |
| G1 | 1 | — |
| G2 | 2 | 1 |
| G3 | 3, 4, 5 in parallel | 2 |
| G4 | 6 (spike) in parallel with 3 and 4 | 5 |
| G5 | 7, 8a, 8b in parallel | 3, 4, 5, 6 |
| G6 | 9 and 10 in parallel | 7, 8a, 8b |
| G7 | 11 | 10 |
| G8 | 12, then 13 | 11, 9 |

Models: `sonnet` for 1, 2, 3, 4, 5, 7, 8a, 9, 10, 11, 12, 13. `opus` for 6 (spike) and 8b (replica, resync, interpolation). `haiku` for no task: every task carries a test instrument that must go red.

---

### Task 1: Workspace, crate skeletons, CI, T1 boundary test

**Model:** sonnet. **Group:** G1.

**Files:**
- Create: `Cargo.toml`, `.cargo/config.toml`, `scripts/boundary.sh`, `scripts/acceptance.sh`
- Create: `crates/<name>/Cargo.toml` and `src/lib.rs` (or `src/main.rs`) for `turts_checks`, `turts_mod_api`, `turts_net_protocol`, `turts_sim`, `turts_mod_runtime`, `turts_net_lightyear`, `turts_server`, `turts_client`, `turts_client_web`
- Create: `crates/turts_checks/tests/boundary.rs`, `crates/turts_checks/tests/no_content.rs`
- Create: `tests/boundary-fixture/Cargo.toml`, `tests/boundary-fixture/src/lib.rs`
- Modify: `.github/workflows/ci.yml`, `CLAUDE.md` (build and test commands; crate layout)

**Interfaces:**
- Produces: the workspace with every crate name above, each with the dependency lines of design section 4.1 (bodies empty). Later tasks add code, not dependencies, except where a task says "add dependency".

- [ ] **Step 1: Write the workspace manifest**

```toml
[workspace]
resolver = "3"
members = ["crates/*"]
exclude = ["tests/boundary-fixture", "tests/spike-web"]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "LicenseRef-TBD"
publish = false

[workspace.dependencies]
serde = { version = "=1.0.229", features = ["derive"] }
serde_json = "=1.0.154"
ron = "=0.12.2"
sha2 = "=0.11.1"
semver = "=1.0.28"
tar = { version = "=0.4.46", default-features = false }
bevy = { version = "=0.20.0", default-features = false }
bevy_app = "=0.20.0"
bevy_ecs = "=0.20.0"
bevy_time = "=0.20.0"
bevy_math = "=0.20.0"
lightyear = { version = "=0.31.0", default-features = false, features = ["std", "websocket", "netcode"] }
postcard = "=1.1.3"
axum = "=0.8.9"
tokio = { version = "=1.54.1", features = ["rt-multi-thread", "net"] }
tower-http = { version = "=0.7.1", features = ["fs", "cors"] }
ureq = "=3.5.0"
gloo-net = "=0.7.0"
wasm-bindgen = "=0.2.129"
wasm-bindgen-futures = "=0.4.79"
wasm-bindgen-test = "=0.3.79"
web-sys = "=0.3.106"
js-sys = "=0.3.106"

[profile.wasm-release]
inherits = "release"
opt-level = "z"
lto = "fat"
codegen-units = 1
strip = "debuginfo"
panic = "abort"
```

`tar 0.4.46`, `axum 0.8.9`, `tokio 1.54.1`, `tower-http 0.7.1`, `ureq 3.5.0`, `gloo-net 0.7.0`, `wasm-bindgen-futures 0.4.79`, `js-sys 0.3.106` are crates.io latest on 2026-10-26; `gloo-net` on wasm32 is NOT TRACED until Task 8a builds it. All other versions are from the probes' lockfiles.

- [ ] **Step 2: Write each crate manifest**

Per crate (`[package]` takes `version.workspace`, `edition.workspace`, `license.workspace`, `publish.workspace`):

| Crate | `[dependencies]` | Extra |
| --- | --- | --- |
| `turts_checks` | none | `[dev-dependencies]` none; `src/lib.rs` empty |
| `turts_mod_api` | `serde` | |
| `turts_net_protocol` | `serde`, `turts_mod_api` | `[dev-dependencies] postcard` |
| `turts_sim` | `turts_mod_api`, `bevy_app`, `bevy_ecs`, `bevy_time`, `bevy_math` | default features (section 4.1 G1b) |
| `turts_mod_runtime` | `turts_mod_api`, `serde`, `ron`, `serde_json`, `sha2`, `semver`, `tar` | `[[bin]] name = "turts-pack"`, `path = "src/bin/turts-pack.rs"` |
| `turts_net_lightyear` | `turts_net_protocol`, `serde`, `bevy_app`, `bevy_ecs`, `lightyear` | `[features] client = ["lightyear/client"]`, `server = ["lightyear/server"]`, `spike = ["client", "server", "dep:bevy", "dep:serde_json"]`; `bevy` and `serde_json` optional; three `[[bin]]` with `required-features = ["spike"]` |
| `turts_server` | `turts_sim`, `turts_mod_runtime`, `turts_mod_api`, `turts_net_protocol`, `turts_net_lightyear` (features `server`), `bevy`, `lightyear` (features `server`), `serde`, `serde_json`, `ron`, `axum`, `tokio`, `tower-http` | bin only (`src/main.rs`) |
| `turts_client` | `turts_net_protocol`, `turts_mod_api`, `turts_mod_runtime`, `turts_net_lightyear` (features `client`), `bevy` (features `["3d"]`), `lightyear` (features `client`), `serde`, `serde_json` | lib + `[[bin]] name = "turts_client"`; `[target.'cfg(not(target_family = "wasm"))'.dependencies] ureq`; `[target.'cfg(target_family = "wasm")'.dependencies] gloo-net, wasm-bindgen-futures, js-sys` |
| `turts_client_web` | `turts_client`, `bevy` (features `["3d"]`), `wasm-bindgen`, `js-sys`, `web-sys` (features `["Window"]`) | `[lib] crate-type = ["cdylib"]` only; `[dev-dependencies] wasm-bindgen-test` |

`bevy` with `default-features = false, features = ["3d"]` includes `webgl2` (web report, `Cargo.toml.orig:185-197`). The `turts_server` `bevy` line keeps `default-features = false` with no feature.

- [ ] **Step 3: Write `.cargo/config.toml`**

```toml
[target.wasm32-unknown-unknown]
runner = "wasm-bindgen-test-runner"
```

- [ ] **Step 4: Write the boundary fixture**

`tests/boundary-fixture/Cargo.toml`: package `turts_boundary_fixture`, edition 2024, `[workspace]` (own lockfile), `[dependencies] bevy_render = "=0.20.0"`. `src/lib.rs` empty.

- [ ] **Step 5: Write the failing T1 test**

`crates/turts_checks/tests/boundary.rs`:

```rust
// Runs scripts/boundary.sh from the workspace root and asserts exit 0.
#[test]
fn t1_boundary_script_passes() { /* Command::new("bash").arg("scripts/boundary.sh") from CARGO_MANIFEST_DIR/../.. ; assert status.success(), print stdout+stderr on failure */ }

// The script must refuse a tree that is not red on the fixture.
#[test]
fn t1_script_fails_when_fixture_is_green() { /* run with env BOUNDARY_FIXTURE_MANIFEST=crates/turts_checks/Cargo.toml and BOUNDARY_FIXTURE_PACKAGE=turts_checks (no bevy_render); assert !status.success() and stderr contains "fixture not red" */ }
```

- [ ] **Step 6: Run it and see it fail**

Run: `cargo test -p turts_checks --test boundary`
Expected: FAIL, `scripts/boundary.sh: No such file`.

- [ ] **Step 7: Write `scripts/boundary.sh`**

Behaviour, bash, `set -euo pipefail`, run from the repo root:

1. `HOST=$(rustc -vV | sed -n 's/^host: //p')`.
2. `tree() { cargo tree -p "$1" -e normal --prefix none --target "$2" ${3:+--manifest-path "$3"} 2>/dev/null | sed 's/ .*//' | sort -u; }` (first column is the crate name).
3. `forbid <crate> <target> <names...>`: fail with `forbidden edge: <crate> -> <name>` when a name is in the tree. Checks, in this order:
   - `turts_sim $HOST bevy_render bevy_winit bevy_pbr turts_mod_runtime`
   - `turts_server $HOST bevy_render bevy_winit bevy_pbr`
   - `turts_mod_api $HOST` and `turts_net_protocol $HOST`: fail on any line matching `^bevy`
   - `turts_client $HOST turts_sim` and `turts_client wasm32-unknown-unknown turts_sim`
   - workspace: `cargo tree --workspace -e normal --prefix none` must not contain `^lightyear_prediction ` or `^bevy_replicon `
4. Fixture: `FIX=${BOUNDARY_FIXTURE_MANIFEST:-tests/boundary-fixture/Cargo.toml}`; `PKG=${BOUNDARY_FIXTURE_PACKAGE:-turts_boundary_fixture}`; `tree "$PKG" $HOST "$FIX"` must contain `bevy_render`; else print `fixture not red` to stderr and exit 1.
5. Exit 0 with `boundary ok`.

The fixture has its own `Cargo.lock`; the first run downloads `bevy_render` (`cargo generate-lockfile --manifest-path tests/boundary-fixture/Cargo.toml` once, commit the lock).

- [ ] **Step 8: Write the no-content test**

`crates/turts_checks/tests/no_content.rs`:

```rust
fn scan(root: &Path) -> Vec<String>  // runs the design's grep:
// grep -rnE '"(base|acme)[:"]|(base|acme)\.tar|rifleman|mods/' <root> --include=*.rs --exclude-dir=tests
// then drops lines whose text after "path:line:" starts, after blanks, with "//", "/*" or "*".

#[test] fn no_content_in_engine_crates() { assert!(scan(crates_dir).is_empty(), "{hits:?}") }
#[test] fn no_content_scan_goes_red_on_planted_file() {
    // write target/tmp/no_content/src/x.rs with `let p = "mods/base.tar";` and a second file with only a comment `// rifleman`
    // assert scan(tmp) has exactly one hit and it names x.rs
}
```

- [ ] **Step 9: Run the tests and see them pass**

Run: `cargo test -p turts_checks`
Expected: 4 passed. Then plant: add `bevy_render = "=0.20.0"` to `crates/turts_sim/Cargo.toml`, run again, expected `forbidden edge: turts_sim -> bevy_render`. Revert the plant.

- [ ] **Step 10: Update CI**

In `.github/workflows/ci.yml` `check` job replace the wasm32 step with `cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown`, keep `cargo test --workspace` and add before it `cargo build -p turts_client` (Task 9 needs the bin). Add a `browser` job stub that runs `echo "T7 added in Task 12"`; Task 12 fills it. Drop the `hashFiles('Cargo.toml') == ''` guards.

- [ ] **Step 11: Write `scripts/acceptance.sh`**

```bash
#!/usr/bin/env bash
set -euo pipefail
cargo build -p turts_client -p turts_server
cargo test -p turts_server --test acceptance -- --test-threads=1 "$@"
```

- [ ] **Step 12: Record commands and layout in `CLAUDE.md`**

Replace "There is no build, lint, or test command yet" with: `cargo test --workspace`; `bash scripts/boundary.sh`; `bash scripts/acceptance.sh`; `cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown`; `trunk build --release` in `crates/turts_client_web`. Add the crate table from section 4.1 plus `turts_net_lightyear` (R1) and `turts_checks`.

- [ ] **Step 13: Commit**

```bash
git add Cargo.toml Cargo.lock .cargo crates scripts tests/boundary-fixture .github/workflows/ci.yml CLAUDE.md
git commit -m "M1: workspace, crate skeletons, CI, T1 boundary test"
```

---

### Task 2: `turts_mod_api` schema types

**Model:** sonnet. **Group:** G2.

**Files:**
- Create: `crates/turts_mod_api/src/{lib,ids,schema,error,registry,lock}.rs`
- Test: `crates/turts_mod_api/tests/schema.rs`

**Interfaces:**
- Produces (all `pub`, all `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` unless noted):

```rust
// lib.rs
pub const ENGINE_API_VERSION: &str = "0.1.0";
pub use ids::{ContentId, PackageId, UnitTypeId};
pub use schema::{ExecutionContext, Exports, Footprint, Manifest, MapDef, MapSize, PackageKind, Placement, Shape, UnitDef, VisualDef, WorldPos};
pub use error::ModError;
pub use registry::UnitRegistry;
pub use lock::{LockEntry, Lockfile};

// ids.rs
pub struct PackageId(pub String);                 // M1 placeholder; format open (spec 15): ^[a-z0-9_]+$
impl PackageId { pub fn parse(s: &str) -> Result<Self, String>; }
pub struct ContentId { pub package: PackageId, pub local: String }
impl ContentId {
    pub fn new(package: &PackageId, local: &str) -> Self;
    pub fn parse(s: &str) -> Result<Self, String>;  // exactly one ':'; both parts non-empty
}
impl std::fmt::Display for ContentId { /* "<package>:<local>" */ }
#[derive(Copy, Eq, Hash, PartialOrd, Ord)] pub struct UnitTypeId(pub u32);

// schema.rs  (#[serde(deny_unknown_fields)] on every struct)
pub struct Manifest { pub id: String, pub version: String, pub kind: PackageKind, pub engine_api: String,
    pub dependencies: BTreeMap<String, String>, pub exports: Exports, pub context: ExecutionContext,
    pub capabilities: Vec<String>, pub license: String }
pub enum PackageKind { Game, Library }
pub enum ExecutionContext { Server, Client, Shared }
pub struct Exports { pub units: Vec<String>, pub maps: Vec<String>, pub visuals: Vec<String> }
pub struct UnitDef { pub id: String, pub name: String, pub speed: f32, pub footprint: Footprint }
pub enum Footprint { Circle { radius: f32 } }
pub struct MapDef { pub id: String, pub size: MapSize, pub placements: Vec<Placement> }
pub struct MapSize { pub x: f32, pub y: f32 }
pub struct Placement { pub unit: String, pub owner_slot: u8, pub pos: WorldPos }
pub struct WorldPos { pub x: f32, pub y: f32 }
pub struct VisualDef { pub unit: String, pub shape: Shape }
pub enum Shape { Capsule { radius: f32, height: f32 }, Cuboid { x: f32, y: f32, z: f32 } }

// error.rs
pub struct ModError { pub package: String, pub file: Option<String>, pub field_path: String,
    pub message: String, pub allowed: Option<Vec<String>> }
impl ModError { pub fn new(package: &str, field_path: &str, message: impl Into<String>) -> Self;
    pub fn with_file(self, file: &str) -> Self; pub fn with_allowed(self, allowed: Vec<String>) -> Self; }
impl std::fmt::Display for ModError { /* "<package>/<file or ->: <field_path>: <message>" */ }

// registry.rs
pub trait UnitRegistry {
    fn register_unit_type(&mut self, origin: &PackageId, def: UnitDef) -> Result<UnitTypeId, ModError>;
}

// lock.rs  (JSON documents; seven fields per spec 5.5)
pub struct Lockfile { pub packages: Vec<LockEntry> }
pub struct LockEntry { pub id: String, pub version: String, pub hash: String, pub size: u64,
    pub urls: Vec<String>, pub context: ExecutionContext, pub engine_api: String }
```

- [ ] **Step 1: Write the failing tests**

`tests/schema.rs`, using `ron` and `serde_json` as dev-dependencies (add them):

```rust
#[test] fn manifest_round_trips_from_design_text()      // Appendix A manifest.ron parses; m.id == "acme"; m.exports.maps == ["maps/field.ron"]
#[test] fn manifest_rejects_unknown_field()             // add `hash: "x",` -> ron error mentions "hash"
#[test] fn unit_def_parses_named_footprint()            // `(id: "scout", name: "Scout", speed: 6.0, footprint: Circle(radius: 0.6))`
#[test] fn visual_def_parses_both_shapes()              // Capsule and Cuboid with named fields; `Cuboid(1.0, 0.8, 1.4)` (unnamed) is rejected
#[test] fn content_id_parse_and_display()               // "acme:scout" -> package "acme", local "scout"; "a:b:c" and "scout" are Err
#[test] fn package_id_rejects_bad_chars()               // "Acme", "ac me", "" are Err; "acme_2" Ok
#[test] fn lock_entry_json_has_seven_fields()           // serde_json::to_value(entry).as_object().len() == 7
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p turts_mod_api`
Expected: compile error, types missing.

- [ ] **Step 3: Implement the types as listed in Interfaces**

- [ ] **Step 4: Run the tests and see them pass**

Run: `cargo test -p turts_mod_api` — Expected: 7 passed. Run `bash scripts/boundary.sh` — Expected: `boundary ok`.

- [ ] **Step 5: Commit**

```bash
git add crates/turts_mod_api Cargo.lock
git commit -m "turts_mod_api: content ids, package schema, ModError, UnitRegistry, lockfile types"
```

---

### Task 3: `turts_mod_runtime`, `turts-pack`, the two packages, T2c

**Model:** sonnet. **Group:** G3.

**Files:**
- Create: `crates/turts_mod_runtime/src/{lib,archive,parse,hash,manifest,lock,pack}.rs`, `src/bin/turts-pack.rs`
- Create: `mods/base/{manifest.ron,units/rifleman.ron,maps/flat.ron,visuals/rifleman.ron}` (section 4.4 values), `mods/acme/...` (Appendix A, verbatim)
- Test: `crates/turts_mod_runtime/tests/{archive,t2c_package_rejects,pack}.rs`, fixtures built in tests from `mods/` text with string edits (no committed broken archives)

**Interfaces:**
- Consumes: Task 2 types.
- Produces:

```rust
// hash.rs
pub fn sha256_hex(bytes: &[u8]) -> String;                       // "sha256:" + 64 hex  (M1 placeholder; algorithm open)
pub fn verify_hash(bytes: &[u8], expected: &str) -> Result<(), ModError>;   // field_path "hash", message names both hashes

// parse.rs
pub fn parse_ron<T: serde::de::DeserializeOwned>(package: &str, file: &str, text: &str) -> Result<T, ModError>;
// Maps ron::error::SpannedError { code, span } to ModError:
//   NoSuchStructField { expected, found, .. } -> message "unknown field `<found>`", allowed = expected
//   MissingStructField { field, .. }          -> message "missing field `<field>`"
//   NoSuchEnumVariant { expected, found, .. } -> message "unknown variant `<found>`", allowed = expected
//   DuplicateStructField { field, .. }        -> message "duplicate field `<field>`"
//   anything else                              -> message "cannot parse: <code Display>"
// Every message ends with " at line <span.start.line> column <span.start.col>". `outer` (the type name) is never used.
// field_path = "<file>"; file = Some(file).

// manifest.rs
pub fn validate_manifest(m: &Manifest) -> Result<(), ModError>;
// id: PackageId::parse ok (field_path "id"); version: semver::Version (field "version");
// engine_api: semver::VersionReq that matches ENGINE_API_VERSION (field "engine_api", message
// "engine api `<req>` does not accept engine `0.1.0`"); dependencies empty (field "dependencies",
// message "dependencies are not supported at M1"); exports.maps.len() == 1 (field "exports.maps",
// message "a package exports exactly one map at M1, found <n>"); every export path is relative,
// no `..`, ends with ".ron".

// archive.rs
pub struct PackageArchive { pub hash: String, pub size: u64, pub manifest: Manifest, files: BTreeMap<String, Vec<u8>> }
impl PackageArchive {
    pub fn parse(bytes: &[u8]) -> Result<Self, ModError>;      // tar in memory; rejects absolute paths, "..", duplicates (field "archive"); needs "manifest.ron"; validate_manifest
    pub fn id(&self) -> PackageId;
    pub fn file(&self, path: &str) -> Option<&[u8]>;
    pub fn units(&self) -> Result<Vec<(String, UnitDef)>, ModError>;      // for each exports.units path, parse_ron; missing file -> field_path "exports.units"
    pub fn maps(&self) -> Result<Vec<(String, MapDef)>, ModError>;
    pub fn visuals(&self) -> Result<Vec<(String, VisualDef)>, ModError>;
    pub fn lock_entry(&self, urls: Vec<String>) -> LockEntry;              // id, version, hash, size, urls, context, engine_api from the manifest
    pub fn check_against_lock(&self, entry: &LockEntry) -> Result<(), ModError>;  // all seven fields; first mismatch wins; field_path "lock.<field>"
}

// lock.rs
pub fn write_lockfile(lock: &Lockfile) -> Vec<u8>;                 // serde_json::to_vec_pretty, trailing newline
pub fn read_lockfile(bytes: &[u8]) -> Result<Lockfile, ModError>;  // package "", field_path "lockfile"

// pack.rs
pub fn pack_dir(dir: &Path) -> Result<Vec<u8>, ModError>;
// ustar via tar::Builder, entries sorted by relative path, Header::new_ustar, mtime 0, uid/gid 0, mode 0o644,
// only regular files. Two calls on the same tree give identical bytes.
```

`turts-pack <dir>`: writes `<dir name>.tar` next to `<dir>`, prints `<file> <hash>` on stdout, exits 2 on a `ModError` with its Display on stderr.

- [ ] **Step 1: Write the package sources**

`mods/base`: section 4.4 values (`id: "base"`, `units/rifleman.ron` = `(id: "rifleman", name: "Rifleman", speed: 3.0, footprint: Circle(radius: 0.4))`, `maps/flat.ron` with `size: (x: 64.0, y: 64.0)` and one placement `(unit: "base:rifleman", owner_slot: 0, pos: (x: 8.0, y: 8.0))`, `visuals/rifleman.ron` = `(unit: "base:rifleman", shape: Capsule(radius: 0.3, height: 1.8))`). `mods/acme`: Appendix A verbatim.

- [ ] **Step 2: Write the failing archive and pack tests**

```rust
// tests/pack.rs
#[test] fn pack_is_reproducible()                 // pack_dir(mods/base) twice -> equal bytes; sha256_hex equal
#[test] fn pack_then_parse_round_trips()          // PackageArchive::parse(pack_dir(mods/acme)).manifest.id == "acme"; units()[0].1.speed == 6.0
#[test] fn pack_cli_prints_hash()                 // Command CARGO_BIN_EXE_turts-pack mods/acme into a temp copy; stdout second token starts with "sha256:" and has 71 chars
// tests/archive.rs
#[test] fn archive_rejects_unsafe_and_duplicate_paths()   // hand-built tar with "../x.ron", "/abs.ron", and "manifest.ron" twice -> Err field_path "archive"
#[test] fn archive_without_manifest_is_rejected()
#[test] fn parse_ron_rewrites_errors_without_type_names()
// typo "nmae" in units/scout.ron -> message "unknown field `nmae`", allowed == ["id","name","speed","footprint"], message has no "UnitDef"
#[test] fn verify_hash_names_both_hashes()
```

- [ ] **Step 3: Write the failing T2c test** (`tests/t2c_package_rejects.rs`)

Build each case from `mods/acme` text, pack in memory, assert `Err(e)` with the given `field_path`, and assert for every case that `e.message` contains none of `Manifest`, `UnitDef`, `MapDef`, `VisualDef`, `Exports`, `Footprint`, `Shape`, `Placement`:

```rust
#[test] fn t2c_engine_api_mismatch()        // engine_api: "^0.2" -> field_path "engine_api"
#[test] fn t2c_non_empty_dependencies()     // dependencies: {"lib": "^1"} -> "dependencies"
#[test] fn t2c_unknown_manifest_field()     // add `hash: "sha256:00",` -> "manifest.ron", message contains "unknown field `hash`"
#[test] fn t2c_manifest_disagrees_with_lock()   // entry = archive.lock_entry(vec![]) with version "9.9.9" -> check_against_lock Err "lock.version"; also hash flipped -> "lock.hash"
#[test] fn t2c_malformed_visual_loads_on_server_path_and_fails_on_client_path()
// visuals/scout.ron replaced by `(unit: "acme:scout", shape: Sphere(r: 1.0))`:
// parse() Ok, units() Ok, maps() Ok  (the server path);  visuals() Err with message "unknown variant `Sphere`" and allowed == ["Capsule","Cuboid"]
#[test] fn t2c_two_maps_rejected()          // exports.maps with two paths -> "exports.maps"
```

- [ ] **Step 4: Run them and see them fail**

Run: `cargo test -p turts_mod_runtime` — Expected: compile errors.

- [ ] **Step 5: Implement the modules as listed in Interfaces**

Pack the `mods/` directories in tests with `pack_dir(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/acme")))`.

- [ ] **Step 6: Run the tests and see them pass**

Run: `cargo test -p turts_mod_runtime` — Expected: all pass. Planted defect for T2c: change `parse_ron` to return `e.to_string()` as the message; `parse_ron_rewrites_errors_without_type_names` and `t2c_malformed_visual...` must fail (ron's text includes the type in `outer`; if it does not, the test's `allowed` assertion still fails). Revert.

- [ ] **Step 7: Commit**

```bash
git add crates/turts_mod_runtime mods Cargo.lock
git commit -m "turts_mod_runtime: archive, parse, hash, lockfile, turts-pack; base and acme packages; T2c"
```

---

### Task 4: `turts_sim` headless world, registration rules, T2b, T3

**Model:** sonnet. **Group:** G3.

**Files:**
- Create: `crates/turts_sim/src/{lib,plugin,registry,world,map,movement,command,slots,harness}.rs`
- Test: `crates/turts_sim/tests/{t2b_registration,t3_claim,movement,command}.rs`

**Interfaces:**
- Consumes: Task 2 types.
- Produces:

```rust
// lib.rs
pub const DEFAULT_TICK_HZ: u16 = 20;  // prototype constant: measure at M1
pub use bevy_math::Vec2;
pub use plugin::SimPlugin;
pub use registry::{UnitTypeEntry, UnitTypes};
pub use world::{Owner, Position, MoveTarget, Tick, UnitId, UnitType, ChangedThisTick, UnitState, snapshot, gameplay_entity_count, MapBounds};
pub use map::load_map;
pub use command::{Command, CommandKind, CommandQueue, CommandResult, CommandResults, RejectReason};
pub use slots::{ClaimError, SlotId, Slots};
pub use harness::headless_app;

// plugin.rs
pub struct SimPlugin { pub tick_hz: u16 }
// build: insert Time::<Fixed>::from_hz(tick_hz as f64); init Tick(0), UnitTypes, CommandQueue, CommandResults,
// ChangedThisTick, Slots::new(2); MapBounds is inserted by load_map.
// FixedPreUpdate: (advance_tick, drain_commands).chain(); FixedUpdate: integrate_movement; FixedPostUpdate: collect_changes

// world.rs  (Components derive Component; resources derive Resource)
pub struct UnitId(pub u32);           // Component. "gameplay entity" == entity with UnitId
pub struct Owner(pub SlotId);         // Component
pub struct Position(pub Vec2);        // Component
pub struct UnitType(pub UnitTypeId);  // Component
pub struct MoveTarget(pub Vec2);      // Component, present while moving
pub struct Tick(pub u32);             // Resource
pub struct MapBounds { pub size: Vec2 }  // Resource
pub struct ChangedThisTick(pub Vec<(u32, Vec2)>);  // Resource: (unit_id, pos) with Changed<Position> this tick
pub struct UnitState { pub unit_id: u32, pub content_id: String, pub owner_slot: u8, pub pos: Vec2 }
pub fn snapshot(world: &mut World) -> Vec<UnitState>;        // sorted by unit_id
pub fn gameplay_entity_count(world: &mut World) -> usize;    // count of entities With<UnitId>

// registry.rs
pub struct UnitTypeEntry { pub content_id: String, pub speed: f32, pub radius: f32 }
#[derive(Resource, Default)] pub struct UnitTypes { entries: Vec<UnitTypeEntry> }
impl UnitTypes { pub fn get(&self, id: UnitTypeId) -> Option<&UnitTypeEntry>; pub fn find(&self, content_id: &str) -> Option<UnitTypeId>; pub fn len(&self) -> usize; }
impl UnitRegistry for UnitTypes {
    fn register_unit_type(&mut self, origin: &PackageId, def: UnitDef) -> Result<UnitTypeId, ModError>;
    // checks in order, file None, package = origin:
    //  def.id has no ':'                  -> field_path "id",               message "local id `<id>` must not contain `:`"
    //  speed finite, > 0, <= 100          -> field_path "speed",            message "speed must be finite, above 0 and at most 100, got <v>"
    //  Circle radius finite, > 0, <= 50   -> field_path "footprint.radius"
    //  content id `origin:id` unique      -> field_path "id",               message "unit type `<origin:id>` is already registered"
}

// map.rs
pub fn load_map(world: &mut World, origin: &PackageId, map: &MapDef) -> Result<(), ModError>;
// validates before any spawn: size finite and > 0 ("size"); each placement i: unit parses as ContentId
// with package == origin ("placements[i].unit", message "unit `<x>` is not in package `<origin>`"),
// registered in UnitTypes ("placements[i].unit", "unknown unit type"), owner_slot < 2 ("placements[i].owner_slot"),
// pos finite and 0 <= pos <= size ("placements[i].pos"). Then inserts MapBounds and spawns
// (UnitId(next), UnitType, Owner(SlotId(owner_slot)), Position) per placement; unit ids start at 1 in placement order.

// command.rs
pub struct Command { pub slot: SlotId, pub seq: u32, pub kind: CommandKind }
pub enum CommandKind { MoveTo { unit_ids: Vec<u32>, target: Vec2 } }
#[derive(Resource, Default)] pub struct CommandQueue(pub VecDeque<Command>);
pub struct CommandResult { pub slot: SlotId, pub seq: u32, pub outcome: Result<(), RejectReason> }
#[derive(Resource, Default)] pub struct CommandResults(pub Vec<CommandResult>);   // cleared at the start of each drain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason { EmptyCommand, TooManyUnits, DuplicateUnit, UnknownUnit, NotOwner, TargetNotFinite, TargetOutsideMap }
impl RejectReason { pub fn as_str(self) -> &'static str }  // "empty_command", "too_many_units", "duplicate_unit", "unknown_unit", "not_owner", "target_not_finite", "target_outside_map"
// drain_commands: check order = list empty; len > 256; duplicate id; every id exists; every unit owned by slot;
// target finite; target inside MapBounds (0 <= x <= size.x, same for y). On Ok insert MoveTarget on every listed unit.
// A rejected command changes nothing.

// movement.rs  integrate_movement: step = speed * Time<Fixed>::delta_secs(); if distance <= step: pos = target, remove MoveTarget; else move `step` toward target.

// slots.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)] pub struct SlotId(pub u8);
#[derive(Resource)] pub struct Slots { /* Vec<Option<u64>> */ }
pub enum ClaimError { NoOpenSlot, AlreadyClaimed }
impl Slots { pub fn new(count: u8) -> Self; pub fn claim(&mut self, player_id: u64) -> Result<SlotId, ClaimError>;
    pub fn owner_of(&self, slot: SlotId) -> Option<u64>; pub fn slot_of(&self, player_id: u64) -> Option<SlotId>; }
// claim: AlreadyClaimed when player_id already holds a slot; else first open slot; transitions open -> claimed only (D25, spec 6.8).

// harness.rs
pub fn headless_app(tick_hz: u16) -> App;
// App::new() + bevy_time::TimePlugin + SimPlugin { tick_hz } + TimeUpdateStrategy::FixedTimesteps(1), then one warm-up app.update()
// (the first update runs zero fixed ticks: bevyprobe). After return, each app.update() runs exactly one fixed tick.
```

- [ ] **Step 1: Write the failing T2b test** (`tests/t2b_registration.rs`)

Every case runs twice, with origin `base` and with origin `acme`, and asserts the same `field_path` and an equal `message` modulo the package name; and asserts no message contains `UnitDef`, `MapDef`, `Footprint`, `Placement`:

```rust
#[test] fn t2b_local_id_with_colon_rejected()      // id "acme:scout" -> "id"
#[test] fn t2b_speed_out_of_range_rejected()       // speed 0.0, -1.0, f32::NAN, 101.0 -> "speed"; 100.0 Ok
#[test] fn t2b_radius_out_of_range_rejected()      // 0.0, 50.5 -> "footprint.radius"
#[test] fn t2b_duplicate_content_id_rejected()     // register twice -> "id", message contains "already registered"
#[test] fn t2b_map_unit_of_other_package_rejected()  // origin acme, placement unit "base:rifleman" -> "placements[0].unit"
#[test] fn t2b_map_owner_slot_2_rejected()         // -> "placements[0].owner_slot"
#[test] fn t2b_map_pos_outside_size_rejected()     // pos (70, 8) on 64x64 -> "placements[0].pos"; nothing spawned (gameplay_entity_count == 0)
#[test] fn t2b_same_rules_for_both_namespaces()    // loop over the cases above, compare field paths per origin
```

(`t2b` for "a package with two maps" lives in Task 3 `t2c_two_maps_rejected`; the design row T2b lists it, and the loader owns the check. Do not duplicate.)

- [ ] **Step 2: Write the failing T3 test** (`tests/t3_claim.rs`)

```rust
#[test] fn t3_claim_creates_nothing() {
    let mut app = headless_app(20);
    // register one unit type through UnitRegistry (origin "fixture_pkg"), load_map with one placement
    assert_eq!(gameplay_entity_count(app.world_mut()), 1);
    app.world_mut().resource_mut::<Slots>().claim(11).unwrap();   // slot 0
    app.world_mut().resource_mut::<Slots>().claim(22).unwrap();   // slot 1
    for _ in 0..5 { app.update(); }
    assert_eq!(gameplay_entity_count(app.world_mut()), 1);
    assert_eq!(snapshot(app.world_mut()).len(), 1);
}
#[test] fn claim_same_player_twice_is_rejected()   // claim(11) Ok, claim(11) Err(AlreadyClaimed), slot_of(11) == Some(SlotId(0))
#[test] fn third_claim_has_no_open_slot()
```

- [ ] **Step 3: Write the failing movement and command tests**

```rust
// tests/movement.rs
#[test] fn harness_runs_one_tick_per_update()      // Tick goes 1,2,3 over three updates after headless_app returns
#[test] fn unit_moves_speed_per_second_and_arrives() // speed 3.0, hz 20: from (8,8) toward (8+3, 8): after 10 updates pos.x == 9.5 (1e-5); after 21 updates pos == target and MoveTarget removed
#[test] fn changed_this_tick_lists_only_moved_units()  // two units, one moving -> ChangedThisTick.len() == 1; after arrival next tick len() == 0
// tests/command.rs
#[test] fn validate_move_to_rejects_malformed_lists()   // empty -> EmptyCommand; 257 ids -> TooManyUnits; [1,1] -> DuplicateUnit
#[test] fn move_to_for_unowned_unit_is_not_owner()      // unit owned by slot 0, command from slot 1 -> NotOwner; Position unchanged after 3 updates
#[test] fn move_to_nan_and_outside_map_rejected()       // NAN -> TargetNotFinite; (65, 8) -> TargetOutsideMap; (64, 64) Ok
#[test] fn results_cleared_each_tick()                   // CommandResults holds only this tick's results
```

- [ ] **Step 4: Run them and see them fail**

Run: `cargo test -p turts_sim` — Expected: compile errors.

- [ ] **Step 5: Implement the modules as listed in Interfaces**

Schedules: `bevy_app::{FixedPreUpdate, FixedUpdate, FixedPostUpdate}`. Time: `bevy_time::{Fixed, Time, TimePlugin, TimeUpdateStrategy}`. Never call `World::run_schedule(FixedUpdate)` (section 4.2).

- [ ] **Step 6: Run the tests and see them pass**

Run: `cargo test -p turts_sim` — Expected: all pass. Planted defect for T3: temporarily add to `advance_tick` a spawn of `(UnitId(999), Position(Vec2::ZERO))` when `Slots` has a claimed slot. `t3_claim_creates_nothing` must fail on the count after claims. Revert. Planted defect for T2b: skip the speed check when `origin.0 == "base"`; `t2b_same_rules_for_both_namespaces` must fail. Revert. Run `bash scripts/boundary.sh` — Expected `boundary ok`.

- [ ] **Step 7: Commit**

```bash
git add crates/turts_sim Cargo.lock
git commit -m "turts_sim: fixed-tick headless world, UnitRegistry, load_map, movement, command validation, slots; T2b, T3"
```

---

### Task 5: `turts_net_protocol` and `turts_net_lightyear`

**Model:** sonnet. **Group:** G3.

**Files:**
- Create: `crates/turts_net_protocol/src/{lib,messages,advertisement}.rs`
- Create: `crates/turts_net_lightyear/src/{lib,channels,protocol_plugin}.rs`
- Test: `crates/turts_net_protocol/tests/wire.rs`, `crates/turts_net_lightyear/tests/registers.rs`

**Interfaces:**
- Produces (`turts_net_protocol`, all serde derive, `Debug, Clone, PartialEq`):

```rust
pub const NET_PROTOCOL_VERSION: u32 = 1;
#[derive(Copy, Eq, Hash)] pub struct UnitId(pub u32);
#[derive(Copy, Eq, Hash)] pub struct SlotId(pub u8);
#[derive(Copy)] pub struct Pos { pub x: f32, pub y: f32 }
pub struct Hello { pub player_id: u64, pub protocol_version: u32, pub lockfile_hash: String }
pub struct Welcome { pub slot_id: SlotId, pub tick: u32, pub tick_hz: u16 }
pub struct JoinRejected { pub reason: String }
pub struct UnitSnapshot { pub unit_id: UnitId, pub content_id: String, pub owner_slot: SlotId, pub pos: Pos }
pub struct Snapshot { pub tick: u32, pub units: Vec<UnitSnapshot> }
pub struct UnitDelta { pub unit_id: UnitId, pub pos: Pos }
pub struct UnitDeltas { pub tick: u32, pub base_tick: u32, pub changes: Vec<UnitDelta> }
pub struct MoveTo { pub seq: u32, pub unit_ids: Vec<UnitId>, pub target: Pos }
pub struct CommandAck { pub seq: u32, pub tick: u32 }
pub struct CommandRejected { pub seq: u32, pub reason: String }
pub struct ResyncRequest;
pub struct Advertisement { pub mod_id: String, pub mod_version: String, pub engine_version: String,
    pub protocol_version: u32, pub lockfile_url: String, pub lockfile_hash: String }   // JSON over HTTP
```

- Produces (`turts_net_lightyear`):

```rust
pub struct ReliableChannel;   // ChannelMode::OrderedReliable(ReliableSettings::default()), Bidirectional
pub struct StateChannel;      // ChannelMode::SequencedUnreliable, send_frequency Duration::default(), ServerToClient
pub struct ProtocolPlugin;    // registers, in this fixed order: channels ReliableChannel, StateChannel; messages
                              // Hello (C->S), Welcome (S->C), JoinRejected (S->C), Snapshot (S->C), UnitDeltas (S->C),
                              // MoveTo (C->S), CommandAck (S->C), CommandRejected (S->C), ResyncRequest (C->S)
pub fn tick_duration(tick_hz: u16) -> core::time::Duration;   // Duration::from_secs_f64(1.0 / tick_hz as f64)
```

Registration code is the probe's (`m1/lyprobe/src/lib.rs`): `app.add_channel::<C>(ChannelSettings { mode, ..Default::default() }).add_direction(NetworkDirection::...)` and `app.register_message::<M>().add_direction(...)`, imports from `lightyear::prelude::*`. Add `ProtocolPlugin` after `ClientPlugins`/`ServerPlugins` and before any `Client`/`Server` entity (lightyear 0.31 `lib.rs` note).

- [ ] **Step 1: Write the failing tests**

```rust
// turts_net_protocol/tests/wire.rs
#[test] fn messages_round_trip_through_postcard()    // each type: postcard::to_allocvec then from_bytes equals the value
#[test] fn unit_deltas_payload_size_is_recorded()    // UnitDeltas with 1 change: assert len <= 16; with 1000 changes: assert len <= 1000 * 12 + 16; print both sizes
#[test] fn advertisement_json_has_six_fields()
// turts_net_lightyear/tests/registers.rs (needs features client,server: run with --features client,server)
#[test] fn protocol_plugin_builds_on_client_and_server_apps()
// App::new() + MinimalPlugins-free: bevy_app::App with bevy_time::TimePlugin, ClientPlugins { tick_duration: tick_duration(20) }, ProtocolPlugin; app.finish(); same with ServerPlugins
#[test] fn protocol_plugin_registers_nine_messages()  // app.is_message_registered::<Hello>() ... for all nine (AppMessageExt)
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p turts_net_protocol && cargo test -p turts_net_lightyear --features client,server` — Expected: compile errors.

- [ ] **Step 3: Implement both crates as listed in Interfaces**

- [ ] **Step 4: Run the tests and see them pass**

Run the two commands above — Expected: all pass. Run `cargo check -p turts_net_lightyear --features client --target wasm32-unknown-unknown` — Expected: OK (lyprobe step 3). Run `bash scripts/boundary.sh` — Expected `boundary ok`; `turts_net_protocol`'s tree has no `bevy` line.

- [ ] **Step 5: Commit**

```bash
git add crates/turts_net_protocol crates/turts_net_lightyear Cargo.lock
git commit -m "turts_net_protocol wire types; turts_net_lightyear ProtocolPlugin with one registration order (R1)"
```

---

### Task 6: Networking spike P1 to P8

**Model:** opus. **Group:** G4 (parallel with Tasks 3 and 4).

**Files:**
- Create: `crates/turts_net_lightyear/src/bin/{spike_server,spike_client,tcp_relay}.rs` (feature `spike`)
- Create: `crates/turts_net_lightyear/tests/spike.rs` (`#![cfg(feature = "spike")]`)
- Create: `tests/spike-web/{Cargo.toml,index.html,Trunk.toml,src/lib.rs}` (not a workspace member), `tests/browser/{package.json,spike_p8.js}`
- Create: `docs/measurements/m1-network-spike.md`, `vault/Decisions/2026-10-26 M1 networking stack.md`

**Interfaces:**
- Consumes: Task 5 `ProtocolPlugin`, channels, messages, `tick_duration`.
- Produces: the pass/fail record that section 8 requires before Task 7 starts; the lightyear setup shape that Tasks 7 and 8a copy.

**Setup shape (verified against lightyear 0.31.0 and aeronet_websocket 0.22.0-rc.1 sources and `examples/common/src/{client,server}.rs` at tag 0.31.0):**

```rust
// server (native): imports lightyear::prelude::*, lightyear::prelude::server::*, lightyear::netcode::NetcodeServer
let entity = commands.spawn((
    LocalAddr(ws_addr),                       // aeronet_io::connection::LocalAddr, re-exported in prelude
    Server,
    NetcodeServer::new(NetcodeConfig {
        protocol_id: NET_PROTOCOL_VERSION as u64,
        private_key: [0u8; 32],               // M1 placeholder; real identity is M10
        server_addr_check: false,             // the client's token names 127.0.0.1, the bind address is 0.0.0.0
        ..Default::default()
    }),
    WebSocketEndpoint { config: ServerConfig::builder().with_bind_address(ws_addr).with_no_encryption() },
)).id();
commands.trigger(Start { entity });
// client: imports lightyear::prelude::*, lightyear::prelude::client::*, lightyear::netcode::client_plugin::NetcodeConfig
let entity = commands.spawn((
    Client,
    Link::default(),
    NetcodeClient::new(
        Authentication::Manual { server_addr: ws_addr, client_id: player_id, private_key: [0u8; 32], protocol_id: NET_PROTOCOL_VERSION as u64 },
        NetcodeConfig { client_timeout_secs: 3, token_expire_secs: -1, ..Default::default() },
    )?,
    WebSocketClientIo::from_url(cfg, format!("ws://{ws_addr}")),   // cfg: native ClientConfig::builder().with_no_encryption(); wasm ClientConfig::default()
)).id();
commands.trigger(Connect { entity });
```

Sending: `Query<&mut MessageSender<M>>` then `sender.send::<ReliableChannel>(m)` (senders are required components after `add_direction`). Receiving: `Query<(Entity, &mut MessageReceiver<M>), With<ClientOf>>` on the server, `Query<&mut MessageReceiver<M>>` on the client; receivers appear on first receipt, so an empty query is normal. Run receive systems in `PreUpdate` `.after(MessageSystems::Receive)`. `On<Add<Connected>>` fires on the client entity and on each `ClientOf` entity. lightyear's `TimelinePlugin` sets `Time<Fixed>` to `tick_duration`; pass `tick_duration(20)`. Netcode disconnects a silent peer after `client_timeout_secs = 3`.

- [ ] **Step 1: Write the spike binaries**

`spike_server [--ws 127.0.0.1:0] [--units N] [--stats <path>]`: `MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(1)))`, `ServerPlugins { tick_duration: tick_duration(20) }`, `ProtocolPlugin`, the entity above. Prints `WS_ADDR=<addr>` once `Started` is added (read the bound port from `LocalAddr` when `--ws` port is 0; if lightyear does not update `LocalAddr` on bind, use a fixed free port chosen by the harness: NOT TRACED). On `Hello`: `lockfile_hash == "sha256:spike"` → `Welcome { slot_id: SlotId(0), tick, tick_hz: 20 }`, else `JoinRejected { reason: "lockfile hash mismatch" }` then `Disconnect { entity }` after 10 frames. On `MoveTo`: `CommandAck { seq, tick }`. Each `FixedPostUpdate`: `UnitDeltas { tick, base_tick: tick - 1, changes }` with `--units` synthetic units moving 0.15 per tick, sent to every welcomed `ClientOf`. Every 5 s write `--stats` JSON `{ "window_ticks", "tick_p50_us", "tick_p99_us", "tick_max_us", "delta_payload_bytes_total", "resident_bytes" }` (`/proc/self/statm` page count times 4096).

`spike_client --ws <addr> [--hash <h>] [--moves N] [--run-secs S] [--out <path>]`: `MinimalPlugins` at 60 Hz, `ClientPlugins`, `ProtocolPlugin`, the entity above. Writes `--out` JSON at exit: `{ "welcomed", "rejected_reason", "disconnected", "acks", "ack_seqs_unique", "deltas", "deltas_in_order", "first_tick", "last_tick" }`. `--moves N` sends N `MoveTo` after `Welcome`, one per frame.

`tcp_relay --listen 127.0.0.1:0 --target <addr> --out <path> [--pause-after-secs A --pause-secs B]`: std-only TCP relay; one thread per direction per connection; counts bytes per direction; prints `RELAY_ADDR=<addr>`; after `A` seconds stops reading from the target for `B` seconds (the "client stops reading" model for P7); writes `--out` JSON `{ "to_client_bytes", "to_server_bytes", "seconds" }` at exit (SIGTERM via `ctrlc`-free approach: the harness sends the relay a line on stdin to stop).

- [ ] **Step 2: Write `tests/spike.rs`** (one `#[test]` per item, `--test-threads=1`, binaries via `env!("CARGO_BIN_EXE_spike_server")` etc.)

```rust
#[test] fn p1_lockfile_pinned_and_licences_recorded()
// cargo tree -p turts_net_lightyear --features client,server -e normal --prefix none --format "{p} {l}": every version is exact
// (lock exists), every licence contains one of "MIT", "Apache-2.0", "BSD", "ISC", "Zlib", "Unicode", "MPL-2.0", "CC0"; write the list
// to target/spike/licences.txt; assert no line is empty; fail with the offending line otherwise
#[test] fn p2_server_has_no_render_crates_and_client_checks_on_wasm()
// cargo tree -p turts_net_lightyear --features server --prefix none: no ^bevy_render|^bevy_winit|^bevy_pbr;
// cargo check -p turts_net_lightyear --features client --target wasm32-unknown-unknown succeeds
#[test] fn p3_connect_hello_welcome_and_bad_hash_rejected()
// client --hash sha256:spike -> welcomed == true; client --hash sha256:wrong -> rejected_reason == "lockfile hash mismatch" && disconnected == true
#[test] fn p4_thousand_moves_get_one_ack_each()      // --moves 1000: acks == 1000 && ack_seqs_unique == 1000
#[test] fn p5_sixty_seconds_of_deltas_in_order()
// server --units 1 through the relay for 60 s: deltas_in_order; bytes_per_tick = to_client_bytes / ticks; payload from stats; write both to target/spike/p5.json; repeat with --units 1000
#[test] fn p6_killed_client_keeps_tick_p99()         // baseline 20 s stats; then with a client killed at 10 s: p99 within max(10 %, 1 ms) of baseline
#[test] fn p7_stalled_reader_keeps_tick_p99_and_bounded_memory()
// relay pauses 5 s after 10 s; same bound; resident_bytes after < before + 32 MiB; record what netcode did (timeout at 3 s expected)
```

- [ ] **Step 3: Run them and see them fail**

Run: `cargo test -p turts_net_lightyear --features spike --test spike -- --test-threads=1 --nocapture` — Expected: FAIL at P3 before the binaries exist.

- [ ] **Step 4: Implement the binaries, run until P1 to P7 pass**

- [ ] **Step 5: P8 wasm client**

`tests/spike-web`: cdylib crate (`[workspace]`, own lock) depending on `turts_net_lightyear` (path, features `client`), `bevy = { version = "=0.20.0", default-features = false }`, `wasm-bindgen =0.2.129`, `js-sys`, `web-sys` (`Window`, `Location`, `UrlSearchParams`). `#[wasm_bindgen(start)]`: read `?ws=` from the URL, build `App` with `MinimalPlugins` + `ClientPlugins` + `ProtocolPlugin`, spawn the client entity, set `window.spike_state` each frame to `"connecting" | "welcomed" | "rejected"` via `js_sys::Reflect::set`. `Trunk.toml` `[tools] wasm_bindgen = "0.2.129"`, `wasm_opt = "version_116"`; `index.html` with `<link data-trunk rel="rust" data-cargo-profile="wasm-release" data-wasm-opt="z"/>`; profile copied from the workspace. `tests/browser/package.json`: `{ "private": true, "dependencies": { "playwright-core": "1.64.1" } }`. `spike_p8.js <url> <ws>`: launches `/usr/bin/google-chrome` headless (`m1/pw/measure.js` shape), waits up to 20 s for `window.spike_state === "welcomed"`, exits 0, else prints console logs and exits 1. The harness step: `trunk build --release` in `tests/spike-web`, serve `dist/` with `python3 -m http.server` on a free port, run spike_server, run the script. Record the result in the doc. If the wasm build fails on a `getrandom` backend error, add `tests/spike-web/.cargo/config.toml` with `[target.wasm32-unknown-unknown] rustflags = ["--cfg", "getrandom_backend=\"wasm_js\""]` and record it (NOT TRACED: lyprobe passed `cargo check` without it).

- [ ] **Step 6: Write the result**

`docs/measurements/m1-network-spike.md`: one row per P item with command, result, numbers (P5 bytes per tick for 1 and 1,000 units, payload and overhead apart; P6 and P7 p99 values), the licence list path, the exact commit. `vault/Decisions/2026-10-26 M1 networking stack.md`: lightyear 0.31.0 messages-only over WebSocket selected for M1 (or the fallback bevy_replicon 0.45 + bevy_replicon_renet2 1.0 if any P item failed and the fallback passed the same list; then also amend the T1 rule in `scripts/boundary.sh` in the same commit). Spec 6.10 stays "not selected" beyond M1.

- [ ] **Step 7: Commit**

```bash
git add crates/turts_net_lightyear tests/spike-web tests/browser docs/measurements/m1-network-spike.md "vault/Decisions/2026-10-26 M1 networking stack.md" Cargo.lock
git commit -m "M1 networking spike P1-P8: lightyear 0.31 messages-only over WebSocket"
```

---

### Task 7: `turts_server` binary

**Model:** sonnet. **Group:** G5.

**Files:**
- Create: `crates/turts_server/src/{main,config,boot,http,net,session,outbound,ticklog,stats,synthetic}.rs`
- Test: `crates/turts_server/tests/unit_boot.rs` (in-process boot tests), `crates/turts_server/tests/hello.rs` (process-level Hello tests; harness from Task 9 step 1 is created here in its minimal form and extended in Task 9)

**Interfaces:**
- Consumes: Task 3 `PackageArchive`, `write_lockfile`, `sha256_hex`; Task 4 sim API; Task 5 messages; Task 6 setup shape.
- Produces: the server executable and its CLI, used by Tasks 9, 12, 13.

```rust
// config.rs  (server.ron; R2)
#[derive(Deserialize)] pub struct HostConfig { pub game_package: String, pub listen: String, pub ws_listen: String,
    pub public_url: String, pub tick_hz: u16, #[serde(default)] pub static_dir: Option<String> }
pub struct Cli { pub config: PathBuf, pub tick_log: Option<PathBuf>, pub stats: Option<PathBuf>, pub synthetic_units: u32, pub run_ticks: Option<u32> }
pub fn parse_cli(args: &[String]) -> Result<Cli, String>;   // turts_server <server.ron> [--tick-log P] [--stats P] [--synthetic-units N] [--run-ticks N]

// boot.rs
pub struct Booted { pub archive: PackageArchive, pub advertisement: Vec<u8>, pub lockfile: Vec<u8>, pub lockfile_hash: String, pub package_file_name: String }
pub fn boot(world: &mut World, cfg: &HostConfig, archive_bytes: &[u8]) -> Result<Booted, ModError>;
// PackageArchive::parse; for each (file, def) in units(): UnitTypes::register_unit_type(&archive.id(), def), on Err fill file;
// maps(): exactly one, load_map(world, &id, &map) with file filled on Err; lock entry urls = ["<public_url>/packages/<id>-<version>.tar"];
// lockfile bytes = write_lockfile; lockfile_hash = sha256_hex(lockfile bytes); advertisement = Advertisement { mod_id, mod_version,
// engine_version: ENGINE_API_VERSION, protocol_version: NET_PROTOCOL_VERSION, lockfile_url: "<public_url>/lockfile.json", lockfile_hash } as JSON.
// The server never calls visuals(). No string literal in this crate names a package or a path under mods/.

// http.rs
pub fn spawn_http(listen: SocketAddr, booted: Arc<Booted>, static_dir: Option<PathBuf>) -> std::thread::JoinHandle<()>;
// axum Router: GET /advertisement.json (application/json), GET /lockfile.json, GET /packages/{name} (application/octet-stream, 404 otherwise),
// CorsLayer::permissive(); fallback_service(ServeDir::new(static_dir).precompressed_br()) when static_dir is Some.
// tokio Runtime::new() on the spawned thread; axum::serve(tokio::net::TcpListener::bind(listen), router).

// session.rs
#[derive(Component)] pub struct Session { pub slot: SlotId, pub player_id: u64 }
#[derive(Component)] pub struct PendingDisconnect(pub u8);   // frames left, then commands.trigger(Disconnect { entity })
#[derive(Resource)] pub struct Advertised { pub lockfile_hash: String, pub tick_hz: u16 }
pub fn receive_hello(...)   // PreUpdate after MessageSystems::Receive. Order of checks and reasons:
//   protocol_version != NET_PROTOCOL_VERSION -> "protocol version mismatch"; lockfile_hash mismatch -> "lockfile hash mismatch";
//   Slots::claim Err(AlreadyClaimed) -> "player already in match"; Err(NoOpenSlot) -> "no open slot".
//   On reject: send JoinRejected, insert PendingDisconnect(10). On accept: insert Session, send Welcome { slot_id, tick: Tick.0, tick_hz },
//   then Snapshot { tick: Tick.0, units: snapshot(world) }.
pub fn receive_move_to(...)  // for each MoveTo from a Session entity: send CommandAck { seq, tick: Tick.0 } at once, push Command { slot, seq, MoveTo }
pub fn receive_resync(...)   // ResyncRequest -> Snapshot of the current tick to that entity

// outbound.rs  (FixedPostUpdate, after turts_sim collect_changes)
pub fn send_command_results(...)  // CommandResults with Err(reason) -> CommandRejected { seq, reason: reason.as_str() } to the slot's Session entity
pub fn send_deltas(...)           // UnitDeltas { tick, base_tick: tick - 1, changes: ChangedThisTick as UnitDelta } to every Session entity, every tick, also when empty

// ticklog.rs  --tick-log: one line per unit per tick "tick,wall_ns,unit_id,x,y" (wall_ns = SystemTime UNIX epoch ns), BufWriter, flushed each tick; disabled when --synthetic-units > 0
// stats.rs   --stats: every 5 s and at exit, JSON { "ticks", "tick_p50_us", "tick_p99_us", "tick_max_us", "delta_payload_bytes_total", "resync_requests", "hellos", "resident_bytes" };
//            tick duration measured around the FixedMain run (Res<Time<Fixed>> overstep is not used): wrap with a system at FixedFirst start and FixedLast end
// synthetic.rs  --synthetic-units N: after boot, spawn N more units of the first registered unit type at random positions inside MapBounds
//            through load_map with a generated MapDef (owner_slot 0), and give each a MoveTarget to a random point every 40 ticks. Uses the
//            content id read from UnitTypes, never a literal.

// main.rs: parse_cli; read config; read archive bytes; App::new() + MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_millis(1)))
// + ServerPlugins { tick_duration: tick_duration(cfg.tick_hz) } + ProtocolPlugin + SimPlugin { tick_hz } ; boot(); spawn_http; lightyear server entity + Start;
// --run-ticks N exits with code 0 after N ticks (AppExit). Prints "LISTENING http=<addr> ws=<addr>" on stdout once Started is added.
```

- [ ] **Step 1: Write the failing in-process boot tests** (`tests/unit_boot.rs`, uses `turts_sim::headless_app` and `turts_mod_runtime::pack_dir` on `mods/base` and `mods/acme`)

```rust
#[test] fn boot_registers_units_and_places_map_from_any_package()   // base: UnitTypes.len()==1, gameplay_entity_count==1, snapshot()[0].content_id=="base:rifleman"; acme: "acme:scout" at (4,4)
#[test] fn boot_lockfile_entry_matches_archive()                     // read_lockfile(booted.lockfile).packages[0].hash == archive.hash; urls[0] ends with "/packages/acme-0.1.0.tar"
#[test] fn boot_advertisement_hash_matches_lockfile_bytes()         // sha256_hex(booted.lockfile) == advertisement.lockfile_hash
#[test] fn boot_fills_file_on_registration_error()                   // acme with speed 0 -> Err e, e.file == Some("units/scout.ron"), e.field_path == "speed"
```

- [ ] **Step 2: Write the failing process-level Hello tests** (`tests/hello.rs`; minimal harness `tests/support/mod.rs` with `spawn_server(package_tar, extra_args) -> ServerProc { child, http_url, ws_url, dir }` that picks two free ports, writes `server.ron`, polls `GET /advertisement.json` up to 10 s; `client_exe()` returns `target/<profile>/turts_client` next to `CARGO_BIN_EXE_turts_server` and runs `cargo build -p turts_client` when missing)

```rust
#[test] fn http_endpoints_serve_advertisement_lockfile_and_package()   // ureq GETs; lockfile hash matches; package bytes hash matches lock entry
#[test] fn hello_duplicate_player_is_rejected()   // two headless clients with --player-id 7: one "welcomed", one "player already in match" (reads client --out JSON, Task 8a)
```

`hello_duplicate_player_is_rejected` needs the client of Task 8a; mark it `#[ignore = "needs Task 8a"]` until Task 9 removes the attribute.

- [ ] **Step 3: Run them and see them fail**

Run: `cargo test -p turts_server` — Expected: compile errors.

- [ ] **Step 4: Implement the modules as listed in Interfaces**

- [ ] **Step 5: Run the tests and see them pass**

Run: `cargo test -p turts_server` — Expected: unit_boot 4 passed, http test passed. Run `cargo run -p turts_server -- target/tmp/server.ron` with `mods/base` packed by `turts-pack`; `curl` the three endpoints. Run `bash scripts/boundary.sh` and `cargo test -p turts_checks` — Expected: pass (the no-content grep sees no literal in `crates/turts_server/src`).

- [ ] **Step 6: Commit**

```bash
git add crates/turts_server Cargo.lock
git commit -m "turts_server: boot through the public package path, HTTP endpoints, lightyear server, sessions, deltas, tick log, stats (R2)"
```

---

### Task 8a: `turts_client` acquisition, join, ingest, headless binary

**Model:** sonnet. **Group:** G5 (parallel with 7 and 8b).

**Files:**
- Create: `crates/turts_client/src/{lib,options,fetch,acquire,join,ingest,present,framelog,debug}.rs`, `src/main.rs`
- Test: `crates/turts_client/tests/{acquire,t2d_client_hash}.rs`

**Interfaces:**
- Consumes: Task 3 (`PackageArchive`, `read_lockfile`, `verify_hash`, `sha256_hex`), Task 5, Task 6 setup shape, Task 8b `Replica` (develop against the signatures below; integrate when 8b lands).
- Produces:

```rust
// options.rs
pub struct ClientOptions { pub server_url: String, pub ws_url: String, pub headless: bool, pub player_id: Option<u64>,
    pub frame_log: Option<PathBuf>, pub out: Option<PathBuf>, pub auto_move: Option<(f32, f32)>, pub auto_move_after_ticks: u32,
    pub run_secs: Option<f32>, pub drop_delta_ticks: Vec<u32>, pub hold_snapshot_ticks: u32 }
impl Default for ClientOptions { /* empty urls, headless false, auto_move_after_ticks 5, everything else None, empty or 0 */ }
pub fn parse_cli(args: &[String]) -> Result<ClientOptions, String>;
// --server URL --ws URL [--headless] [--player-id N] [--frame-log P] [--out P] [--move X,Y] [--move-after-ticks N (default 5)]
// [--run-secs S] [--drop-delta-ticks a,b,c] [--hold-snapshot-ticks N]
pub fn options_from_query(query: &str) -> Result<ClientOptions, String>;   // "?server=<url>&ws=<url>" for the browser; Err names the missing key
pub fn random_player_id() -> u64;   // native: std RandomState hashing; wasm: js_sys::Math::random() bits. No getrandom dependency.

// fetch.rs
pub struct FetchTask(bevy::tasks::Task<Result<Vec<u8>, String>>);
pub fn fetch_bytes(url: &str) -> FetchTask;   // IoTaskPool::get().spawn; native: ureq::get(url).call()?.body_mut().read_to_vec(); wasm: gloo_net::http::Request::get(url).send().await?.binary().await
pub fn poll(task: &mut FetchTask) -> Option<Result<Vec<u8>, String>>;   // bevy::tasks::futures_lite::future::block_on(bevy::tasks::poll_once(&mut task.0))

// acquire.rs  (pure functions; the phase machine calls them)
pub struct Acquired { pub lockfile_hash: String, pub visuals: HashMap<String, Shape>, pub map_size: (f32, f32), pub unit_names: HashMap<String, String> }
pub fn check_advertisement(bytes: &[u8]) -> Result<Advertisement, String>;      // protocol_version == NET_PROTOCOL_VERSION, engine_version == ENGINE_API_VERSION
pub fn check_lockfile(bytes: &[u8], advertised_hash: &str) -> Result<Lockfile, String>;   // verify_hash before read_lockfile (section 5)
pub fn packages_to_fetch(lock: &Lockfile) -> Vec<&LockEntry>;                   // context Client or Shared only
pub fn accept_package(bytes: &[u8], entry: &LockEntry) -> Result<PackageArchive, String>;   // verify_hash(bytes, &entry.hash) BEFORE PackageArchive::parse; then check_against_lock
pub fn build_acquired(archives: &[PackageArchive], lockfile_hash: String) -> Result<Acquired, String>;   // visuals(): every unit has exactly one visual (else "unit `<id>` has <n> visuals"); map_size from maps()[0]

#[derive(Resource)] pub enum Phase { FetchAdvertisement, FetchLockfile, FetchPackages { left: usize }, Connecting, Joining, Joined { slot: u8 }, Failed(String) }

// join.rs: on Phase::Connecting spawn the client entity (Task 6 shape) with Welcome/JoinRejected receivers; on Add<Connected> send Hello { player_id, NET_PROTOCOL_VERSION, lockfile_hash };
// Welcome -> Phase::Joined, Replica::new(tick_hz); JoinRejected -> Phase::Failed(reason). Hello is sent once per connection.

// ingest.rs  (PreUpdate after MessageSystems::Receive)
#[derive(Resource, Default)] pub struct IngestHooks { pub drop_delta_ticks: Vec<u32>, pub hold_snapshot_ticks: u32, pub held: Vec<(Snapshot, u32)> }
#[derive(Resource, Default)] pub struct IngestCounters { pub deltas_received: u32, pub deltas_dropped: u32, pub resync_requests_sent: u32, pub snapshots_applied: u32 }
pub fn ingest_messages(...)
// Snapshot: when hold_snapshot_ticks > 0 push to held with release_at = S + hold; release when a delta with tick >= release_at has been received;
//   else Replica::apply_snapshot. UnitDeltas: if tick in drop_delta_ticks -> dropped (count), else Replica::apply_delta.
//   Any ResyncRequested outcome -> send ResyncRequest once, count.

// present.rs (Update): Presented { frame_ns, t, units: Vec<PresentedUnit { unit_id, content_id, pos, lo_tick, hi_tick, fraction }> } from Replica::presentation_time + sample
// framelog.rs: --frame-log lines "frame_ns,unit_id,t,lo_tick,hi_tick,fraction,x,y"; at exit a summary line
//   "# welcome_tick=<w> first_present_tick=<lo of first record> resync_requests_sent=<n> deltas_dropped=<n>"
// debug.rs: #[derive(Resource)] DebugState(String) rebuilt each frame: JSON { phase, units: [{unit_id, content_id, pos:[x,y], shape}], last_frame: {t, lo_tick, hi_tick, fraction} }
// --out P: at exit JSON { "welcomed", "rejected_reason", "slot", "resync_requests_sent", "final_positions": {unit_id: [x,y]} }
// main.rs: parse_cli; headless -> MinimalPlugins.set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1.0 / 60.0))); else DefaultPlugins (Task 10);
//   + ClientPlugins { tick_duration: tick_duration(20) } + ProtocolPlugin + ClientPlugin(options). --run-secs exits with AppExit.
// lib.rs: pub struct ClientPlugin(pub ClientOptions); pub fn build_app(opts: ClientOptions) -> App
```

The auto-move: `--move X,Y` sends `MoveTo { seq: 1, unit_ids: all units whose owner_slot == my slot, target }` `auto_move_after_ticks` ticks after `Welcome`.

- [ ] **Step 1: Write the failing acquisition tests** (`tests/acquire.rs`)

```rust
#[test] fn check_lockfile_refuses_wrong_hash_before_parse()   // bytes "not json" with a hash of other bytes -> Err mentions "hash"; same bytes with their own hash -> Err mentions "lockfile" (parse)
#[test] fn client_skips_server_only_packages()                 // lockfile with contexts Server, Client, Shared -> packages_to_fetch returns 2, none with context Server
#[test] fn build_acquired_requires_exactly_one_visual_per_unit()  // acme with visuals/scout.ron removed from exports -> Err; with two visual files for scout -> Err; Appendix A -> Ok, visuals["acme:scout"] == Cuboid
#[test] fn check_advertisement_refuses_other_protocol()
#[test] fn options_from_query_reads_server_and_ws()    // "?server=http://h:7000&ws=ws://h:7001" -> Ok with both urls; "" -> Err containing "server"
```

- [ ] **Step 2: Write the failing T2d client test** (`tests/t2d_client_hash.rs`)

```rust
#[test] fn t2d_package_one_byte_off_is_refused_before_parse()
// archive = pack_dir(mods/acme); entry = PackageArchive::parse(&archive).lock_entry(vec![]); flip bytes[len/2] ^= 1;
// accept_package(&flipped, &entry) is Err and the message names both hashes; accept_package(&archive, &entry) is Ok
#[test] fn t2d_lockfile_bytes_off_are_refused()     // see check_lockfile test; asserted here with real lockfile bytes from write_lockfile
```

T2d's third case (`Hello` with a wrong `lockfile_hash` gets `JoinRejected`) runs in Task 9 as the process test `t2d_bad_hello_hash_is_rejected`; this task owns T2d's two client-side cases, Task 9 runs the whole T2d row.

- [ ] **Step 3: Run them and see them fail**

Run: `cargo test -p turts_client` — Expected: compile errors.

- [ ] **Step 4: Implement the modules as listed in Interfaces**

For the hash compare in `accept_package`, planted defect check: replace `verify_hash` with `Ok(())`; `t2d_package_one_byte_off_is_refused_before_parse` must fail. Revert.

- [ ] **Step 5: Run the tests and see them pass**

Run: `cargo test -p turts_client`; `cargo check -p turts_client --target wasm32-unknown-unknown` (first `gloo-net` build: if it fails, replace with `web-sys` `fetch` and record NOT TRACED resolved); `bash scripts/boundary.sh` (no `turts_sim` under `turts_client` on both targets).

- [ ] **Step 6: Commit**

```bash
git add crates/turts_client Cargo.lock
git commit -m "turts_client: acquisition with hash checks, join, ingest with test hooks, headless binary (R3)"
```

---

### Task 8b: `turts_client` replica, resync and interpolation logic

**Model:** opus. **Group:** G5 (parallel with 7 and 8a).

**Files:**
- Create: `crates/turts_client/src/{replica,interp}.rs`
- Test: `crates/turts_client/tests/{replica,t8_resync_logic,interp}.rs`

**Interfaces:**
- Consumes: Task 5 messages. `Instant` is `bevy::platform::time::Instant`.
- Produces:

```rust
// interp.rs
pub const INTERP_DELAY_TICKS: f64 = 1.0;   // prototype constant: measure at M1
pub fn presentation_time(latest_tick: u32, since_receipt_secs: f64, tick_hz: u16) -> f64;   // latest + since * hz - INTERP_DELAY_TICKS, clamped to <= latest
pub fn lerp(a: Pos, b: Pos, f: f32) -> Pos;

// replica.rs
pub struct UnitReplica { pub content_id: String, pub owner_slot: u8, pub samples: VecDeque<(u32, Pos)> }   // ascending ticks, consecutive
pub struct Presented { pub pos: Pos, pub lo_tick: u32, pub hi_tick: u32, pub fraction: f32 }
#[derive(Debug, PartialEq)] pub enum DeltaOutcome { Applied, Discarded, Buffered, ResyncRequested }
#[derive(Debug, PartialEq)] pub enum SnapshotOutcome { Applied { replayed: u32 }, ResyncRequested { replayed: u32 } }
pub struct Replica { pub tick_hz: u16, pub units: HashMap<u32, UnitReplica>, pub snapshot_tick: Option<u32>,
    pub last_applied_tick: Option<u32>, pub latest_receipt: Option<Instant>, pub resync: Option<Vec<UnitDeltas>> }
impl Replica {
    pub fn new(tick_hz: u16) -> Self;
    pub fn apply_snapshot(&mut self, s: &Snapshot, now: Instant) -> SnapshotOutcome;
    pub fn apply_delta(&mut self, d: &UnitDeltas, now: Instant) -> DeltaOutcome;
    pub fn in_resync(&self) -> bool;
    pub fn presentation_time(&self, now: Instant) -> Option<f64>;
    pub fn sample(&self, unit_id: u32, t: f64) -> Option<Presented>;
}
```

Rules (section 5, with the resync amendment):

- `apply_delta`: in resync → push to the buffer, `Buffered`. No snapshot yet, or `d.tick <= snapshot_tick` → `Discarded`. `d.base_tick != last_applied_tick` → start resync (`resync = Some(vec![d.clone()])`), `ResyncRequested`. A change names an unknown `unit_id` → same as a chain break. Else for every unit: push `(d.tick, new pos)` when in `changes`, else `(d.tick, last pos)`; `last_applied_tick = d.tick`; `latest_receipt = now`; prune samples with tick `< d.tick - 16`; `Applied`.
- `apply_snapshot`: replace `units` with one sample `(S, pos)` each; `snapshot_tick = last_applied_tick = Some(S)`; `latest_receipt = now`. Then take the buffer (if any), drop entries with `tick <= S`, replay in order: an entry whose `base_tick == last_applied_tick` is applied; the first entry that breaks the chain starts one new resync holding it and the rest → `ResyncRequested { replayed }`; else `Applied { replayed }`.
- `presentation_time`: `None` before the first snapshot; else `interp::presentation_time(last_applied_tick, (now - latest_receipt).as_secs_f64(), tick_hz)`, and never below the unit's earliest sample (the caller clamps per unit in `sample`).
- `sample(u, t)`: lo = floor(t), hi = lo + 1; with both samples: lerp with fraction `t - lo`; with only lo (hi not yet received or `t == last_applied`): pos at lo, hi = lo, fraction 0; with one sample only: that sample; t below the earliest sample: the earliest sample. Never two samples that are not consecutive ticks.

- [ ] **Step 1: Write the failing tests**

```rust
// tests/interp.rs
#[test] fn presentation_time_is_delayed_one_tick_and_clamped()   // latest 10, since 0 -> 9.0; since 0.025 at 20 Hz -> 9.5; since 0.2 -> 10.0 (clamp)
#[test] fn lerp_endpoints()
// tests/replica.rs
#[test] fn deltas_before_snapshot_are_discarded()
#[test] fn delta_at_or_below_snapshot_tick_is_discarded()     // snapshot 10; delta tick 10 base 9 -> Discarded; tick 11 base 10 -> Applied
#[test] fn unchanged_unit_gets_a_sample_each_tick()           // two units, delta changes only one -> both have a sample at tick 11
#[test] fn sample_interpolates_between_consecutive_ticks()    // samples (10,(0,0)),(11,(2,0)); t 10.25 -> (0.5,0), lo 10 hi 11 fraction 0.25
#[test] fn sample_with_one_sample_is_that_sample()
#[test] fn sample_never_extrapolates_past_latest()            // t == 11.0 -> pos (2,0), fraction 0, hi == lo == 11
#[test] fn samples_are_pruned()                               // after 40 applied deltas a unit holds at most 18 samples
// tests/t8_resync_logic.rs
#[test] fn t8_missing_delta_requests_resync_once_and_buffers()
// snapshot 10; deltas 11, 12 applied; 13 lost; delta 14 (base 13) -> ResyncRequested; delta 15 -> Buffered (no second request); in_resync()
#[test] fn t8_snapshot_replays_buffer_and_drops_old()
// continue: Snapshot tick 15 -> Applied { replayed: 0 } (14 and 15 dropped as <= 15); delta 16 (base 15) -> Applied
#[test] fn t8_late_snapshot_replays_newer_deltas()
// snapshot 10; lose 13; deltas 14..=18 buffered; Snapshot tick 15 -> Applied { replayed: 3 } (16,17,18); last_applied == 18
#[test] fn t8_broken_chain_during_replay_starts_one_new_resync()
// buffer 14,15,16,18 (17 lost); Snapshot 15 -> ResyncRequested { replayed: 1 }; resync buffer holds [18]
#[test] fn delta_with_unknown_unit_requests_resync()           // snapshot with unit 1; delta changes unit 9 -> ResyncRequested
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo test -p turts_client --test replica --test t8_resync_logic --test interp` — Expected: compile errors.

- [ ] **Step 3: Implement `replica.rs` and `interp.rs`**

- [ ] **Step 4: Run the tests and see them pass; then plant**

Planted defect for T8 logic: in `apply_delta` remove the `base_tick` compare (apply any delta). `t8_missing_delta_requests_resync_once_and_buffers` must fail. Revert. Planted defect for T5's oracle (extrapolation): in `presentation_time` drop the clamp; `presentation_time_is_delayed_one_tick_and_clamped` must fail. Revert.

- [ ] **Step 5: Commit**

```bash
git add crates/turts_client/src/replica.rs crates/turts_client/src/interp.rs crates/turts_client/tests
git commit -m "turts_client: replica with one-outstanding resync and buffered replay; interpolation with one-tick delay"
```

---

### Task 9: Acceptance runs T2a, T2d, T4, T5, T6, T6b, T8

**Model:** sonnet. **Group:** G6 (parallel with Task 10).

**Files:**
- Create: `crates/turts_server/tests/acceptance.rs`, `tests/support/{oracle,logs}.rs`; extend `tests/support/mod.rs` (Task 7)
- Modify: `crates/turts_server/tests/hello.rs` (remove the `#[ignore]`)

**Interfaces:**
- Consumes: server CLI (Task 7), client CLI and logs (Task 8a), `mods/base`, `mods/acme`, `turts_mod_runtime::pack_dir`.
- Produces:

```rust
// support/logs.rs
pub struct TickRecord { pub tick: u32, pub wall_ns: u128, pub unit_id: u32, pub pos: (f32, f32) }
pub struct FrameRecord { pub frame_ns: u128, pub unit_id: u32, pub t: f64, pub lo: u32, pub hi: u32, pub fraction: f32, pub pos: (f32, f32) }
pub struct FrameSummary { pub welcome_tick: u32, pub first_present_tick: u32, pub resync_requests_sent: u32, pub deltas_dropped: u32 }
pub fn read_tick_log(p: &Path) -> Vec<TickRecord>;
pub fn read_frame_log(p: &Path) -> (Vec<FrameRecord>, FrameSummary);
// support/oracle.rs
pub struct OracleReport { pub frames: usize, pub lag_p99: f64, pub lag_max: f64 }
pub fn check_segments(ticks: &[TickRecord], frames: &[FrameRecord], unit_id: u32) -> Result<OracleReport, String>;
// for each frame: hi == lo || hi == lo + 1; 0 <= fraction <= 1; pos within 1e-3 of lerp(P(lo), P(hi), fraction) where P reads the tick log;
// lag = continuous server tick at frame_ns (linear between the tick log's wall times) minus t; returns p99 and max.
pub fn final_position(ticks: &[TickRecord], unit_id: u32) -> (f32, f32);
```

- [ ] **Step 1: Write the failing tests** (`acceptance.rs`, `--test-threads=1`; every test packs its package into its own temp dir; `move` target for `base` is `(20.0, 8.0)`, so the rifleman moves 12 units at 3.0/s = 80 ticks)

```rust
#[test] fn t2a_package_parity()
// for (pkg, expected_id, expected_shape, expected_pos) in [(base, "base:rifleman", "Capsule", (8,8)), (acme, "acme:scout", "Cuboid", (4,4))]:
//   spawn_server(pkg); headless client --out --run-secs 3; out.final_positions has exactly one unit at expected_pos;
//   DebugState JSON (client writes it to --out as "debug") lists content_id == expected_id and shape variant == expected_shape
#[test] fn t2d_bad_hello_hash_is_rejected()          // client --player-id 5 with a test flag --hello-hash sha256:wrong (add to options, test hook) -> out.rejected_reason == "lockfile hash mismatch"; and the two client cases of Task 8a are referenced, not repeated
#[test] fn t4_command_validation()
// headless client with --move for: a unit it does not own (second client claims slot 1 and sends --move 10,8 --move-unit 1), NAN, (99, 8):
// each out.command_results == [{"seq":1,"ack_tick":k},{"seq":1,"rejected":"not_owner" | "target_not_finite" | "target_outside_map"}] in that order;
// server tick log shows unit 1 at (8,8) for every tick (state unchanged)
#[test] fn t5_convergence()
// server --tick-log; client --headless --frame-log --move 20,8 --run-secs 8; check_segments Ok; lag_p99 <= 2.0; print lag_max;
// last frame pos == final_position == (20,8) within 1e-3; frames >= 300
#[test] fn t6_second_viewer()                        // two clients; the second has no --move; its frame log passes check_segments against the same tick log and ends at (20,8)
#[test] fn t6b_late_join()                           // first client moves and exits; after 2 s a new client joins; summary.first_present_tick <= summary.welcome_tick + 3 and its first record pos == (20,8)
#[test] fn t8_lost_delta()
// client --drop-delta-ticks <welcome+10>,<welcome+90> --move 20,8: summary.resync_requests_sent == 2 and deltas_dropped == 2, final pos (20,8);
// second case adds --hold-snapshot-ticks 3: same assertions (exactly one request per loss, no storm, no loop)
```

- [ ] **Step 2: Run them and see them fail**

Run: `bash scripts/acceptance.sh` — Expected: FAIL (support functions missing; then, with stubs, assertions on empty logs).

- [ ] **Step 3: Implement `support/{logs,oracle}.rs` and the small client additions the tests name**

Client additions (Task 8a's `options.rs` and `--out` JSON): `--hello-hash <h>` override, `--move-unit <id>` (the auto-move names that unit instead of the owned units), `"command_results"` array (acks and rejections in receipt order) and `"debug"` (the last `DebugState` JSON). Keep them in `turts_client`; they are test hooks per R3.

- [ ] **Step 4: Run the tests and see them pass; then plant**

Run: `bash scripts/acceptance.sh` — Expected: 7 passed. Planted defects, one at a time, each must turn its test red, then revert:
- T2a: in `boot.rs` ignore `maps()` and spawn one placement at `(8,8)` from the first unit type → `t2a_package_parity` fails on acme's position.
- T4: in `drain_commands` skip the owner check → `t4_command_validation` fails on `not_owner`.
- T5: in `present.rs` add `0.5` to `t` after the clamp → `check_segments` fails (fraction out of range or position off segment).
- T6: in `send_deltas` send only to the first `Session` → `t6_second_viewer` fails.
- T6b: skip the `Snapshot` after `Welcome` → `t6b_late_join` fails on an empty first record.
- T8: in `ingest.rs` never send `ResyncRequest` → `resync_requests_sent == 0`.

- [ ] **Step 5: Commit**

```bash
git add crates/turts_server/tests crates/turts_client/src scripts/acceptance.sh
git commit -m "M1 acceptance runs T2a, T2d, T4, T5, T6, T6b, T8 with segment oracle"
```

---

### Task 10: Presentation and input (native window)

**Model:** sonnet. **Group:** G6 (parallel with Task 9).

**Files:**
- Create: `crates/turts_client/src/{render,input}.rs`; modify `src/main.rs` (window path), `src/lib.rs`
- Test: `crates/turts_client/tests/render_map.rs`

**Interfaces:**
- Consumes: `Presented`, `Acquired.visuals`, `Acquired.map_size` (Task 8a), `Phase`.
- Produces:

```rust
// render.rs
pub fn world_to_scene(pos: Pos) -> Vec3;             // Vec3::new(pos.x, 0.0, pos.y): map x right, map y forward; scene y up (section 4.4)
pub fn mesh_for(shape: &Shape) -> Mesh;              // Capsule { radius, height } -> Capsule3d::new(radius, height - 2.0 * radius); Cuboid { x, y, z } -> Cuboid::new(x, z, y)
pub fn mesh_lift(shape: &Shape) -> f32;              // half height, so the mesh stands on the ground
#[derive(Component)] pub struct PresentedUnit(pub u32);
pub fn setup_scene(...)     // on first Phase::Joined: Camera3d at (size.x/2, 40, size.y/2 + 30) looking at the map centre; DirectionalLight; ground Plane3d sized map_size
pub fn sync_units(...)      // Update, after present: spawn (Mesh3d, MeshMaterial3d, Transform, PresentedUnit) per new unit; set Transform.translation = world_to_scene(pos) + Vec3::Y * lift; despawn units gone from Presented
// input.rs
pub fn click_to_move(...)   // Update: on MouseButton::Left just pressed, Camera::viewport_to_world(global_transform, cursor) -> Ray3d; ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Vec3::Y)) -> distance;
                            // point = ray.get_point(distance); push PendingMove { target: Pos { x: point.x, y: point.z } }
pub fn send_moves(...)      // PostUpdate: PendingMove -> MoveTo { seq: next, unit_ids: owned units, target } on ReliableChannel
```

`Capsule3d`, `Cuboid`, `Plane3d`, `InfinitePlane3d`, `Ray3d::intersect_plane` live in `bevy::shape` (Bevy 0.20 moved shapes out of `bevy_math`; `bevy::prelude` re-exports them: webprobe used `Cuboid::new` from the prelude). `viewport_to_world` returns `Result<Ray3d, ViewportConversionError>` (bevy_camera 0.20 `camera.rs:648`).

- [ ] **Step 1: Write the failing tests** (`tests/render_map.rs`, pure functions only)

```rust
#[test] fn world_to_scene_maps_y_forward_to_scene_z()      // Pos { x: 8, y: 3 } -> Vec3(8, 0, 3)
#[test] fn mesh_lift_is_half_height()                       // Capsule 0.3/1.8 -> 0.9; Cuboid z 1.4 -> 0.7
#[test] fn mesh_for_builds_both_shapes()                    // returns a Mesh with > 0 vertices for both
```

- [ ] **Step 2: Run them and see them fail** — `cargo test -p turts_client --test render_map`.

- [ ] **Step 3: Implement `render.rs`, `input.rs`; wire the window path in `main.rs`**

Window path: `DefaultPlugins.set(WindowPlugin { primary_window: Some(Window { canvas: Some("#turts".into()), fit_canvas_to_parent: true, prevent_default_event_handling: true, ..default() }), ..default() })` (webprobe). The window path is `cfg`-free; the headless flag selects it at runtime.

- [ ] **Step 4: Run the tests and see them pass; run the client against a server by hand**

`cargo run -p turts_server -- target/tmp/server.ron` and `cargo run -p turts_client -- --server http://127.0.0.1:7000 --ws ws://127.0.0.1:7001`. Click the ground; the unit moves; a second window shows the same movement. Record one line in the work log.

- [ ] **Step 5: Commit**

```bash
git add crates/turts_client
git commit -m "turts_client: presentation from Presented, click-to-move input"
```

---

### Task 11: Browser crate, trunk build, `turts_debug`

**Model:** sonnet. **Group:** G7.

**Files:**
- Create: `crates/turts_client_web/src/lib.rs`, `index.html`, `Trunk.toml`, `tests/web.rs`
- Modify: `crates/turts_client/src/debug.rs` (expose `pub fn debug_json(world: &World) -> String`)

**Interfaces:**
- Consumes: `turts_client::build_app`, `parse_cli`-free construction: `ClientOptions { server_url, ws_url, headless: false, ..Default::default() }`.
- Produces: `dist/` with `index.html`, `turts_client_web-<hash>.js`, `.wasm`; `window.turts_debug()` returning the JSON of Task 8a `debug.rs`.

- [ ] **Step 1: Write the failing wasm-bindgen test** (`tests/web.rs`, runs in headless Firefox through geckodriver: `m1/wbt` proved the instrument)

A `cdylib`-only crate cannot be linked by its own integration tests, so the query parser lives in `turts_client` (Task 8a `options_from_query`, tested natively). The browser test here checks the environment only:

```rust
wasm_bindgen_test_configure!(run_in_browser);
#[wasm_bindgen_test] fn webgl2_context_exists()       // m1/wbt test; its red was proven there with `must_fail_probe`
```

- [ ] **Step 2: Run it and see it fail** — `cargo test -p turts_client_web --target wasm32-unknown-unknown` (needs `wasm-bindgen-test-runner` 0.2.129 and Firefox; `GECKODRIVER=` path if not on PATH). Expected: no test target yet; after adding the file with the probe's `must_fail_probe` kept for one run: one FAIL. Then delete `must_fail_probe`.

- [ ] **Step 3: Implement**

`lib.rs`: `#[wasm_bindgen(start)] pub fn start()`: read `window.location.search`, `turts_client::options_from_query(&search)`, `let mut app = turts_client::build_app(opts)`; install `window.turts_debug` with `Closure::<dyn Fn() -> String>::new(|| DEBUG.lock().unwrap().clone())`, `js_sys::Reflect::set(&window, &"turts_debug".into(), closure.as_ref())`, `closure.forget()`; a system in `Last` copies `debug_json` into `static DEBUG: Mutex<String>`; `app.run()`. On an options error, set `window.turts_error` to the message and return (no panic).

`index.html` (webprobe shape, four-space indent):
```html
<!doctype html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <title>TuRTS</title>
    <link data-trunk rel="rust" data-cargo-profile="wasm-release" data-wasm-opt="z" />
    <link data-trunk rel="copy-dir" href="assets" />
    <style>html,body{width:100%;height:100%;margin:0}#wrap{width:1280px;height:720px}#turts{width:100%;height:100%}</style>
</head>
<body><div id="wrap"><canvas id="turts"></canvas></div></body>
</html>
```
`Trunk.toml`: `[tools] wasm_bindgen = "0.2.129"`, `wasm_opt = "version_116"`. `assets/` holds one `.gitkeep`.

- [ ] **Step 4: Build and check sizes**

Run: `cd crates/turts_client_web && trunk build --release` — Expected: `dist/*.wasm` exists; record its size and `brotli -q 11 -k dist/*.wasm` size against the probe baseline (19,593,752 and 4,425,762 bytes) in the work log. Then `cargo test -p turts_client_web --target wasm32-unknown-unknown` — Expected: 1 passed. Run `cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown`.

- [ ] **Step 5: Commit**

```bash
git add crates/turts_client_web crates/turts_client/src/debug.rs Cargo.lock
git commit -m "turts_client_web: trunk build, window.turts_debug, wasm-bindgen tests"
```

---

### Task 12: T7 browser join and the CI browser job

**Model:** sonnet. **Group:** G8.

**Files:**
- Create: `tests/browser/t7.js`, `tests/browser/measure.js` (from `m1/pw/measure.js`, adapted), `scripts/t7.sh`, `crates/turts_server/tests/t7_oracle.rs`
- Modify: `.github/workflows/ci.yml` (`browser` job), `crates/turts_client_web/src/lib.rs` (`turts_move` export)

**Interfaces:**
- Consumes: server `--static-dir` (serves `dist/` with precompressed brotli; `scripts/t7.sh` runs `brotli -q 11 -k` on `dist/*.wasm` and `dist/*.js` first; `brotli` CLI on the CI runner is NOT TRACED, install `brotli` apt package in the job), `window.turts_debug`.
- Produces: `target/t7/{server-tick.log,frames.jsonl,result.json}`.

- [ ] **Step 1: Write `t7.js`** (`node t7.js <page url> <out dir>`; exits 1 on any failed check)

Launch `/usr/bin/google-chrome` headless (default flags, SwiftShader) at 1280x720; `page.goto(url + "?server=<http>&ws=<ws>")`; poll `window.turts_debug()` every 50 ms for 30 s until `phase` starts with `Joined`; fail otherwise with console logs. Then for 6 s record every poll's `last_frame` and `units` with `performance.timeOrigin + performance.now()` into `frames.jsonl`; send a move through `window.turts_move(20, 8)` (add this export beside `turts_debug` in Task 11's `lib.rs`, calling the same `PendingMove` path as a click) 1 s after join. Write `result.json` `{ "welcomed": true, "units": 1, "content_id", "shape", "frames": n }`.

- [ ] **Step 2: Write `scripts/t7.sh`**

Builds `turts_server` release and `trunk build --release`, compresses `dist`, packs `mods/base`, writes `target/t7/server.ron` with `static_dir: "crates/turts_client_web/dist"`, starts the server with `--tick-log target/t7/server-tick.log`, runs `node tests/browser/t7.js http://127.0.0.1:<port>/ target/t7`, then runs `cargo test -p turts_server --test t7_oracle` which reads the two logs, converts `frames.jsonl` to `FrameRecord`s and runs `check_segments`; prints `lag_p99` and `lag_max` (recorded, not judged). Stops the server.

- [ ] **Step 3: Run it and see it fail** — before `turts_move` exists: `t7.js` fails at the move step. Then implement `turts_move`, rerun.

- [ ] **Step 4: Run it and see it pass**

Run: `bash scripts/t7.sh` — Expected: `result.json.welcomed == true`, oracle test passes, lag printed. Planted defect: start the server with `mods/acme` while `t7.js` expects `content_id` of the package it was told (`--expect-content-id` argument); mismatch → exit 1. Revert.

- [ ] **Step 5: CI job**

`browser` job on `ubuntu-latest`: checkout, rust toolchain with wasm32 target, `Swatinem/rust-cache`, `cargo install --locked trunk --version 0.22.0`, `cargo install --locked wasm-bindgen-cli --version 0.2.129`, `sudo apt-get install -y brotli`, `actions/setup-node@v4` with node 20, `npm ci` in `tests/browser`, `bash scripts/t7.sh`. Chrome: ubuntu-latest ships `google-chrome` (NOT TRACED; if absent use `browser-actions/setup-chrome@v1` and pass its path through `CHROME_BIN`, which `t7.js` reads with the `/usr/bin/google-chrome` default). T7 stays out of `cargo test --workspace` (section 7): `t7_oracle` carries `#[ignore]`, and `scripts/t7.sh` runs it with `cargo test -p turts_server --test t7_oracle -- --ignored`.

- [ ] **Step 6: Commit**

```bash
git add tests/browser scripts/t7.sh crates/turts_server/tests crates/turts_client_web/src/lib.rs .github/workflows/ci.yml
git commit -m "T7 browser join through headless Chrome; separate CI browser job"
```

---

### Task 13: M1 measurements document and roster records

**Model:** sonnet. **Group:** G8, after Task 12.

**Files:**
- Create: `docs/measurements/m1.md`
- Modify: `CLAUDE.md` (final crate table and commands), `README.md` (status line), `vault/Work log` entry per the vault's existing pattern (one line per run)

**Interfaces:**
- Consumes: T5 run (Task 9), the spike relay (Task 6) for WebSocket-frame bytes, server `--stats`, `tests/browser/measure.js`.

- [ ] **Step 1: Produce the five figures**

1. Bytes per tick to one client at WebSocket frame level: run `turts_server` with `mods/base` behind `tcp_relay` for 60 s with one headless client that moves once; `bytes_per_tick = to_client_bytes / ticks`; payload from `--stats delta_payload_bytes_total / ticks`; overhead = difference.
2. Server tick p99 from `--stats` of the same run; and of a synthetic run `--synthetic-units 1000` for 60 s, with its bytes per tick (figure 5 of section 2).
3. Browser frame p99 and 4. wasm memory high-water from `measure.js` (SwiftShader run and, on the dev machine only, `--enable-gpu --use-angle=gl`; the two are never compared), using `window.turts_debug` frame records and `memory_size(0) * 65536` sampled each second (export `turts_memory_bytes()` beside `turts_debug` if `measure.js` needs it).
5. Download size: Resource Timing `transferSize` until the first frame, with brotli on.

- [ ] **Step 2: Write `docs/measurements/m1.md`**

Sections: machine and commit; method per figure (exact commands); a table with the figures; "recorded, not judged" statement; the probe baseline row from section 4.5 for comparison; open budgets this feeds (spec 13, 15).

- [ ] **Step 3: Verify the whole**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && bash scripts/boundary.sh && bash scripts/acceptance.sh && cargo check --workspace --exclude turts_server --target wasm32-unknown-unknown` — Expected: all green. Run `bash docs/agents/check.sh` if any agent file changed.

- [ ] **Step 4: Commit**

```bash
git add docs/measurements/m1.md CLAUDE.md README.md vault
git commit -m "M1 measurements recorded; CLAUDE.md crate layout and commands"
```

---

## Self-review record

**Spec coverage (design sections → tasks).** 2 goals: join and present (9, 12), second client (9 T6), late join (9 T6b), MoveTo ack/validate (7, 9 T4), declarative package through the public path (3, 4, 7, T2a in 9), download with hash check (8a, T2d in 8a and 9), headless red tests (every task), measurements (13). 3 constraints: Task 1 (versions, profile), placeholders marked in 2, 3, 6. 4.1 crates and forbidden edges: 1 (T1), R1 adds one crate. 4.2 scheduling: 4 (harness, FixedTimesteps), 7, 8a. 4.3 networking: 5, 6. 4.4 packages, manifest, diagnostics, registration: 2, 3, 4, 7. 4.5 browser build: 11, 12. 5 protocol and interpolation and resync: 5, 8a, 8b. 6 match shape: 4 (slots), 7. 7 acceptance table: T1 (1), T2a (9), T2b (4), T2c (3), T2d (8a client cases, 9 server case; one row, the plan names both), T3 (4), T4 (9), T5 (9), T6 (9), T6b (9), T8 (8b logic, 9 run), T7 (12), measurements (13). 8 build order: groups G1 to G8 follow it; P1 to P8 are all in Task 6. 9 CEO items: untouched (desktop executable not planned; licence string `LicenseRef-TBD`). 10 "left to the plan": HTTP crate (axum, Task 7), trunk details (11), harness layout (7, 9), the `cargo tree` script (1).

**Known splits to flag at gate 2.** T2d and T8 each have unit-level cases in one task and the process-level run in Task 9; the plan assigns the row to Task 9 and names the unit tests as its instruments. T2b's "two maps" case is checked by the loader (Task 3) because the loader owns `exports.maps`.

**Type consistency.** `SlotId` exists in `turts_sim` (`SlotId(u8)`) and in `turts_net_protocol` (`SlotId(u8)`); the server converts with `.0`. `UnitId` likewise. `Pos` (protocol) versus `Vec2` (sim) and `WorldPos` (schema): the server converts in `outbound.rs`; the client never sees `Vec2` from the sim. `RejectReason::as_str` strings equal the `CommandRejected.reason` strings asserted in T4. `ClientConfig` and `ServerConfig` name the aeronet types; the server's own config is `HostConfig`.

**Proportion.** No function body is written where the signature and tests fix it; bodies appear only for the lightyear entity shapes (verified API), the workspace manifest, `index.html`, and the boundary script's behaviour list.
