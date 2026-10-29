# MC — Player control and client foundation: design

Date: 2026-10-29. Revision 4. Status: DRAFT, passed gate 1 (2026-10-29): spec-rigour-reviewer,
technical-director, bevy-rendering-engineer, player, simulation-core-engineer; three repair
passes; calls K1 to K27 wait for the CEO (section 1.4).
Author seat: `ux-designer` (D65, D66, D67, client side of D70; Second on D68).
Source of record: `RTS-Engine-Project-Specifications.md` version 0.8 (cited as S:line).
Other sources: the QA0 design `docs/superpowers/specs/2026-10-29-qa0-instrument-design.md`
(QA0D:line), the M3 design `docs/superpowers/specs/2026-10-27-m3-industrial-design.md` (M3D:line),
the M4 draft `docs/superpowers/specs/2026-10-28-m4-terrain-design.md` (M4D:line), the gap register
`docs/research/review-2026-10/00-gap-register.md` (GR:line). Research is input, not contract.
Code citations are `path:line` at HEAD `7a90f97`. Section 17 lists the gate 1 findings and their
fixes.

## 1. Contract

### 1.1 Goal

S:722 gives the MC demonstration: "Player input map on physical keys, browser capture and focus,
click, box, shift and type selection, control groups, camera pan, zoom, edge scroll and minimap
click, order queue with stop, hold, attack and attack-move, selection and status visuals, health
bars, refusal reasons, event and alert channel (including machine-problem alerts), team colours",
the HUD items of D67, the minimap with minimap click, gather orders, the order envelope and
per-unit group results (D65, D73), and flat-ground group movement. S:722 has no terrain.

S:722 gives the acceptance a player can fail: "Blind playtest: a tester with only the build
completes the M3 industrial demo with mouse and keyboard; 40 units sent to one point arrive as a
group". For the acceptance evidence S:722 says "The producer sets it in the MC spec". Section 14
proposes it for the producer.

MC is the first milestone with a client after the full spec review. Its result: a person plays
the M3 industrial demo (`mods/forge`) in a normal desktop browser with mouse and keyboard. The
person has the goals of section 14 (PT1) and the in-game guide, and no other document.

### 1.2 In scope

- The client interaction core (D74): a new crate with no renderer and no `bevy*` dependency.
  It holds the player input map, selection, control groups, smart command, command composition
  and the camera state. Agents and the headless client drive it (section 2).
- The player input map on physical keys, mod default bindings, player rebinding, and the browser
  key-capture policy (D66, section 3).
- The control model (D65, section 4): click, box, Shift and type selection, control groups,
  smart command. Also a per-unit order queue with move, stop, hold, attack, attack-move, gather.
- Wire protocol 4 (D73, D18, section 5): the order envelope, with the order type as a content id.
  Also per-unit group results, `match_id` in `Welcome`, and owner-only unit order records.
- Simulation (section 6): a new phase `SimSet::Orders`, the order queue per unit, and flat-ground
  group movement. Also audited writes and the lift of the rule "weapons must be static" (K5).
- Rejoin after a page reload into the slot that the player already holds (K27, section 10.5).
- Camera controls (D68, section 7) and one mod light (K23).
- The HUD widget set and its mod schema (D67, section 8).
- Status visuals, feedback, refusal reasons, alerts with the D69 limits, minimal audio (K7)
  (section 9).
- The client shell for MC (D70 client side) and client-local persistence (D72) (section 10).
- Mod API 0.2.3 (section 11). Packages: forge interface content, the four M1 and M2 packages
  migrated (K3), and a new test package `muster` (K15) (section 12).
- The test instrument (section 13): the harness and the pixels-only drivers that replace the five
  ledger files. Also their oracles and the headless drive of the interaction core.

### 1.3 Out of scope

| Item | Where |
| --- | --- |
| Terrain, heights, routes round terrain | M4, M4b |
| Fog of war, server-side visibility | M5 |
| Construction, placement ghost, power, damage types, repair, rally points, retaliation | MP1 |
| Match lifecycle, lobby, victory and score screens | MP1 |
| Bots | MP2 |
| Chat and text entry | M9 |
| Main menu, server browser, identity screens, explicit session replacement (D52), saves | M8, M10 |
| Replays | M11 |
| Abilities, projectiles, splash damage | MB |
| Compiled assets, animation | M13 |
| Mobile and touch | later (D54) |
| WebGL fallback | Open (S:760) |
| Keyboard Lock in fullscreen, localisation | no milestone |
| macOS as a test target | Open (S:751, platform matrix) |

### 1.4 Calls for the CEO

Each call changes what MC builds, and no Decided row settles it. A recommendation becomes the
design only when the CEO accepts it. Acceptance rows name the option that they assume
(section 14, column "Assumes").

| Call | Question | Options | Evidence | Recommendation |
| --- | --- | --- | --- | --- |
| K1 | Form of the interaction core (D74, Proposed). | (a) New crate `turts_interaction`. T1 forbids `bevy*`, `wgpu`, `winit` and `turts_sim` on host and wasm32. (b) A crate with `bevy_ecs` and `bevy_math` only. (c) A module of `turts_client`. | T1 checks crates, not modules (`scripts/boundary.sh:33-44`). Today input runs only with a window (`crates/turts_client/src/lib.rs:278-280`). | (a) |
| K2 | How a mod defines an order (D73 design, Proposed). | (a) A mod `OrderDef` binds one engine behaviour (mechanism) and gives the targeting policy as parameters (section 5.2). A new behaviour is an additive API bump with a content-level version row. (b) Fixed engine ids such as `engine:move`; mods give labels only. | (b) does not let a mod add an order, so it fails the D73 line (S:102). S:180 puts "Orders, targeting" in the engine and "targeting restrictions" (S:179) in mod policy. | (a) |
| K3 | Orders and interface for `acme`, `base`, `cedar`, `haul`. They export none today (`mods/*/manifest.ron`). | (a) An engine fallback set in `turts_mod_api`, used list by list when a package leaves a list empty. Its ids use the reserved package part `turts`. It needs a recorded exception to S:171 and sits outside the lockfile (D35). (b) Migrate the four packages to 0.2.3 with their own files. (c) A library package `turts_defaults` that the server adds to the lockfile of a package with empty lists. | S:171: default gameplay goes "in the default mod or an optional reusable mod module". (a) breaks that and invariant 10 (S:576). (c) adds a package that no manifest asks for, and the library load path is NOT TRACED. (b) changes four manifests, their hashes and the M2 fixtures. | (b) |
| K4 | Units in one envelope, and the merge of split envelopes. Today: 256, all-or-nothing (`crates/turts_sim/src/command.rs:19,267-269`). | (a) Keep 256; the client splits and tags one `group`. (b) 4,096; the client splits above 4,096. (c) No unit cap. For (a) and (b): the server merges envelopes with one key (slot, `group`) in one tick. A part that arrives in a later tick spreads round the reserved slots of the earlier part (6.3). | The lightyear message size is NOT TRACED. Model run C2 (section 18): a second part round a first part gives 0 overlaps and 0 blocked units. | (b) |
| K5 | Mobile units with a weapon in MC. The rule "weapons must be static" refuses them (`crates/turts_mod_runtime/src/content_m3_validation.rs:381-382`). S:723 gives the lift to MP1. | (a) MC lifts this one rule. The two other MP1 rules stay. (b) Keep it: attack works only for static weapons; attack-move acts as move. | With (b) no unit can attack-move, so OR04 has no test that can fail. One test uses the rule (`crates/turts_mod_runtime/src/content_m3.rs:339`). | (a) |
| K6 | A move target outside the map. Today the server refuses it (`command.rs:284-292`). | (a) Refuse a non-finite target with `target_not_finite`; clamp a finite target to the nearest point inside the map, and accept it. (b) Refuse each unit with `target_outside_map`. | Yardstick GM10: "Move to the nearest reachable point. Do not freeze." A clamp alone keeps NaN and accepts +inf (gate 1 sim probe). | (a) |
| K7 | Audio in MC (D69 Proposed; formats Open). The client has no audio (GR:44, F2). | (a) Minimal audio: unit responses, order acknowledgement, alerts, four volume categories, OGG Vorbis, resume on the first gesture. (b) No audio. | Measured cost of (a): plus 68,027 bytes after brotli, plus 1.3 % (gate 1 bevy run). With (b), rule R4 (QA0D:191) moves OR23, PE01, PE07, PF05, UI16, AU01, AU02, AU06 and MD09 to `later:MP1`. | (a) |
| K8 | Source of alerts. | (a) The client derives alerts from permitted replicated records: `JobStage::OutputBlocked` (`crates/turts_net_protocol/src/economy.rs:83-86`), `ExtractorStatus`, `WeaponStatus`, health (`messages.rs:226-259`). (b) A server event channel in protocol 4. | (a) needs no wire change and survives a resync, because a snapshot holds the same records (`messages.rs:266-278`). | (a) |
| K9 | Edge scroll and the cursor in a browser (GR F8). | (a) The canvas fills the page; an edge band inside the canvas; pointer lock is an opt-in setting with an engine-drawn cursor. (b) Pointer lock always on after the first click. (c) No edge scroll in a window. | The page fixes the canvas at 1280 x 720 (`crates/turts_client_web/index.html:8`). Headless Chrome 156 takes the lock and gives `movementX` (gate 1 bevy run). | (a) |
| K10 | Client-local storage (D72, Proposed). | (a) `localStorage` per origin for bindings, settings and groups; `sessionStorage` for the player id (K27). (b) (a) plus export and import of bindings as text. (c) Nothing before M8. | Each server serves the client from its own origin (gap-web-platform-engineer.md, row wpe-9). Two windows of one origin share `localStorage`, so a shared player id would make the second window fail (`crates/turts_sim/src/slots.rs:45-55`). | (a) |
| K11 | Default control-group keys. | (a) Ctrl+digit assigns, digit recalls, Shift+digit adds, a double tap centres. (b) Another modifier for "assign". | Chromium source does not reserve Ctrl+1..9 (GR:43, F8). Firefox reserves Ctrl+T, Ctrl+W, Ctrl+N, Ctrl+Q and Ctrl+Tab ([bug 1052569](https://bugzil.la/1052569), comment 12). Real OS input and Firefox Ctrl+digit are NOT TRACED. | (a), if probe P1 (3.4) is green on Chrome and Firefox. Else the CEO picks with the P1 table. |
| K12 | Asset references, formats and limits (D67 design: Open, S:756). | (a) Content ids in an `AssetsDef` file, and the caps below. (b) Package-relative paths in each widget. Caps of (a) apply to each package and to the whole locked set. Decoded image bytes: at most 256 MiB (width x height x 4, from the PNG IHDR). Each image at most 2048 x 2048 px. Each cursor image at most 128 x 128 px. Font files at most 8 MiB. Sound files at most 32 MiB, sounds at most 300 s in total and 30 s each. | A 2048 px PNG of one colour is 19,771 bytes. 848 of them fit in 16 MiB of file bytes, and they decode to 13.2 GiB (gate 1 bevy run). wasm32 has 4 GiB. Bevy keeps a CPU copy unless the usage is `RENDER_WORLD` (`bevy_asset-0.20.0/src/render_asset.rs:40-51`). Browsers ignore a CSS cursor above 128 px (MDN), and winit uses a CSS cursor (`winit-0.30.13/src/platform_impl/web/cursor.rs:336`). Bevy keeps the bytes of a sound (`bevy_audio-0.20.0/src/audio_source.rs:8-15`). | (a), with the numbers above |
| K13 | Engine API version. | (a) 0.2.2 to 0.2.3, additive, `serde(default)`. (b) 0.3.0. | `^0.2` and `^0.2.1` refuse 0.3.0 (QA0D:527). Gate 1 run: a 0.2.3 row refuses `^0.2.2` and accepts `^0.2.3`. | (a) |
| K14 | Wire protocol version. | (a) 3 to 4 now, with the envelope (D73: "at the next network protocol version bump"). | The M4 draft planned protocol 4 (M4D K4). MC comes first (S:722-725), so M4 takes 5. | (a) |
| K15 | The package for the group checks. | (a) A new package `muster`. (b) 40 more units on the forge map. | The M3 fixed point depends on `proving_ground` (M3D section 13). One map per package (M4D:28). | (a) |
| K16 | Foreign units before fog (M5). | (a) A click on a foreign unit inspects it read-only, and foreign units show health bars and weapon range. (b) No foreign inspection before M5. | The server sends every unit and every health record to every slot (`crates/turts_server/src/export.rs:94`; GR F6). Weapon records go to the owner only (`export.rs:87-93`), so (a) shows no hidden value. | (a) |
| K17 | The test reading of "arrive as a group" (S:712). | (a) At rest, the footprint discs projected to the screen do not overlap. Each unit stands within R_near = r_max x (sqrt(N / 0.6) + 1) of the target (4.58 for 40 units of radius 0.5). (b) The mesh silhouettes do not overlap. | A tilted camera makes tall meshes of a tight group overlap at rest, so (b) cannot pass. Model run A: the farthest of 40 units stands 3.81 from the target. | (a) |
| K18 | A floor for text size and button size in mod layouts. | (a) `turts-pack` warns below 11 px text and below 24 x 24 px buttons, and never refuses. (b) Refuse. | The D67 constraint (S:96) says "at any size". (b) breaks it. | (a) |
| K19 | How the economy-log validator accepts the new movement. Today it replays the M1 straight line, wants orders unchanged, and keeps a `movement_pending` exception (`crates/turts_server/src/economylog.rs:326,553-556,578-663`). Movement writes no operation today. | (a) The sim movement step becomes a pure function over unit records. The sim system and the validator both call it. MC adds no movement operation. (b) One captured operation per unit per tick. Each writes two full `AuditState` values (`transaction.rs:55-60`, `economylog.rs:22`), so the log grows O(N x E). (c) One captured movement operation per tick for all units, O(E) per tick. | Frozen M2 stream: the test counts every operation (`crates/turts_pack/tests/m3_m2_stream.rs:54-58`), an `acme` unit moves (`:263`), and a test pins the fixture hash (`tests/m3/refutations.py:18`). (a) adds no operation, so the stream stays, if `AuditState` writes `UnitOrders` only when it differs from today's plain move (6.1). (b) and (c) add operations: the stream test must filter movement operations out, or the CEO approves a re-freeze. EL1 bound: in the model, the hard projection moves a unit up to 1.167 x speed x dt in one tick (section 18). So (b) and (c) check at most 2 x speed x dt. Under (a) a movement defect passes the validator; GM40, GMA and GMU still catch it. | (a) |
| K20 | Yardstick dependency changes (the CEO owns the list: QA0D section 3.1, QA0 K6). | Approve or refuse each edge: UI05 no longer needs FV02; CB03 no longer needs FV03; PF01 and MP09 no longer need MP01; PC16 no longer needs FV04. | Each edge holds a row out of MC that MC builds and tests (table in 17.2). Under the current list, 19 rows say `later` although MC tests them (17.2). | Approve all five. Then 19 rows move to `in` in a re-gated amendment. |
| K21 | May MC slip items that S:722 names? | (a) No S:722 item slips without the CEO. (b) The cut line of 15.3 may drop S:722 items. | Cut items that S:722 names: the minimap, type select, hold, attack, attack-move. 15.3 lists the Player path steps that each slip breaks. | (a) |
| K22 | The near forge drone (24,12) stands 7.0 from the turret (17,12), range 8.0 (`mods/forge/maps/proving_ground.ron:11,13`; `mods/forge/units/turret.ron:11`). It dies before window B opens. | (a) The PT1 goals use one drone. (b) The drone steps move to `muster`. (c) A new forge map: it breaks the M3 fixed point. | The server ticks from process start, with no wait for a join (`crates/turts_server/src/stats.rs:131`). PT2 on `muster` covers box selection, groups and spreading. | (a) |
| K23 | Mod light (D37 Decided; lighting API Open, S:750). Code fixes the light (`crates/turts_client/src/render.rs:344`). | (a) A minimal `LightDef` in MC: direction, colour, illuminance, ambient brightness. (b) Keep the code light until M13. | MC adds `CameraDef`. After MC the light is the only presentation constant that a mod cannot set. Cost: about 1 run. | (a) |
| K24 | Depth of the per-unit order queue. | (a) An engine limit of 16. (b) A mod value `queue_depth` (default 16, engine bound 255). | D39 (S:68) avoids arbitrary gameplay caps. The bound protects the owner record size. | (b) |
| K25 | Command rate limits (OR31). | (a) A token bucket per slot in the server session: 64 envelopes, refill 640 per second; 16,384 unit orders, refill 163,840 per second. The simulation also caps spreading at 2,048 units per tick for all slots; the rest waits (6.3). (b) Per-tick counts in the simulation. | The tick rate is Open (D18, S:47), so limits in ticks change with it. The server refuses before the command log, so a replay (invariant 15, S:267) holds only admitted envelopes. | (a) |
| K26 | `glam` for the core math. | (a) `glam = "=0.33.12"`, the version under `bevy_math` 0.20. (b) Own math types. | `Cargo.lock` holds glam 0.33.12 (under `bevy_math`) and 0.34.1 (gate 1 td run). An unpinned dependency can pick 0.34.1. | (a) |
| K27 | Rejoin after a page reload. Today a reload gets a new random id (`crates/turts_client/src/lib.rs:76`). The server refuses the old id with "player already in match", and a new id with "no open slot" (`crates/turts_server/src/session.rs:32,103-105`; `slots.rs:45-55`). | (a) The client keeps its id in `sessionStorage`. `judge_hello` (`session.rs:92`) gives the held slot back when no live session uses it; `Slots::claim` stays as it is. While the old session lives (up to the 10 s netcode timeout), the client retries each second and shows a countdown. (b) (a) plus explicit replacement of a live session (D52). (c) No rejoin; the reload steps leave PT1. | (a) needs a server change in `judge_hello` only. The client timeout is 10 s (`crates/turts_client/src/join.rs:278`; the server takes it from the token, `crates/turts_server/src/net.rs:34-37`). Whether the server sees the WebSocket close at once is NOT TRACED. Invariant 11 (S:577) holds: the slot never changes owner. The id is a bearer value until identity (D50, M10), as today (GR F6). | (a) |

Decided lines that this design keeps, unchanged:

- Per-unit acceptance of group orders, with a reason for each refused unit (D65, G6 B, S:94).
- Engine widgets with mod layout and content, under the D67 constraint (S:96).
- The order type as a content id in an open envelope (D73, G7, S:102).
- Stop and hold in the order list of the default mod (S:803).

### 1.5 Decision IDs this design depends on

| ID | Status (S:line) | Use in MC |
| --- | --- | --- |
| D15 | Baseline (S:44) | No Bevy type in the mod API or the wire. |
| D18 | Decided / implementation open (S:47) | The server decides each order result. The client shows a local acknowledgement only. |
| D25 | Decided (S:54) | A slot stays with its player; a rejoin gives the same slot back (K27). |
| D35 | Decided (S:64) | Every order and interface file comes from a locked package (K3). |
| D37 | Decided (S:66) | Mods set camera constraints and the light (K23). |
| D39 | Decided direction (S:68) | Mod-set queue depth (K24); MC measures limits and promises none. |
| D41 | Decided, local server form Open (S:70) | `turts_client` never depends on `turts_sim`; T1 keeps it. |
| D52 | Decided (S:81) | One session per slot; replacement of a live session stays M10 (K27). |
| D54 | Decided (S:83) | Desktop browsers with mouse and keyboard. |
| D55 | Decided (S:84) | Order rules of 16.2 are mod policy. |
| D65 | Decided line, design Proposed (S:94) | Section 4. |
| D66 | Proposed (S:95) | Section 3. |
| D67 | Decided line and constraint, design Proposed (S:96) | Section 8. |
| D68 | Proposed (S:97) | Section 7. |
| D69 | Proposed, formats Open (S:98) | Sections 9.4 and 9.5. |
| D70 | Proposed (S:99) | Section 10, client side only. |
| D72 | Proposed (S:101) | Section 10.4. |
| D73 | Decided line, design Proposed (S:102) | Section 5. |
| D74 | Proposed (S:103) | Section 2. |

### 1.6 Verified baseline (path:line from HEAD 7a90f97)

- Selection is one optional unit id: `crates/turts_client/src/economy_ui.rs:31`. Left click
  selects, right click orders: `crates/turts_client/src/input.rs:44-45,87-141`. The client has no
  keyboard input (GR:42, F1).
- Picking intersects the plane y = 0: `input.rs:14-21`. Hit tests use the declared footprint
  radius: `economy_ui.rs:721-769`.
- The HUD is one engine panel with English text in Rust (`economy_ui.rs:122-125`) at a fixed
  position (`economy_ui.rs:916-934`). Research is a `Produce` action with a technology id
  (`economy_ui.rs:38-44`).
- The windowed client adds the scene, the panel and the click input; the headless client adds
  none of them: `crates/turts_client/src/lib.rs:278-280`. `prevent_default_event_handling: true`:
  `lib.rs:254`. The player id is random unless `--player-id` gives one: `lib.rs:76`.
- The client places the camera at join (`crates/turts_client/src/render.rs:338-342`), and a
  `PostUpdate` system rewrites it every frame (`render.rs:245,455-456`). Code fixes the light
  (`render.rs:344`). All units share one material (`render.rs:403-406`), and the client writes
  every unit transform every frame (`render.rs:383,390`).
- The page fixes the canvas at 1280 x 720: `crates/turts_client_web/index.html:8`. The web entry
  sets the bridges `turts_debug`, `turts_move`, `turts_scene`, `turts_ui`,
  `turts_memory_bytes`: `crates/turts_client_web/src/lib.rs:76-115`.
- Wire: `NET_PROTOCOL_VERSION = 3` (`crates/turts_net_protocol/src/lib.rs:13`). Six closed
  command kinds (`messages.rs:53-78`); `CancelProduction` carries a `JobId`
  (`messages.rs:67-70`). One reason for each command (`messages.rs:189-194`). `Welcome` has no
  match id (`messages.rs:25-30`). `ContentRef` is a `String` alias (`economy.rs:15`).
- Export: health records go to every slot (`crates/turts_server/src/export.rs:94`); weapon records
  go to the owner only (`export.rs:87-93`).
- Join: `Slots::claim` refuses an id that holds a slot. It gives a new id the first open slot
  (`crates/turts_sim/src/slots.rs:45-55`). The session maps the errors to "player already in
  match" and "no open slot" (`crates/turts_server/src/session.rs:32,103-105`).
- Simulation sets: `Advance, Commands` in `FixedPreUpdate`; `Movement` to `Combat` in
  `FixedUpdate` (`crates/turts_sim/src/plugin.rs:31-42,88-101`). `capture_operation` records an
  audited write (`crates/turts_sim/src/economy/transaction.rs:90`); `AuditState`
  (`crates/turts_sim/src/economy/views.rs:300`). The economy-log validator accepts only the M1
  straight line with unchanged orders (`crates/turts_server/src/economylog.rs:578-663`).
- `MoveTo` refuses the whole list on one bad unit and above 256 units
  (`crates/turts_sim/src/command.rs:19,255-294`). Movement is a straight line with no avoidance
  (`crates/turts_sim/src/movement.rs:12-58`). A weapon picks the nearest target in range, with
  no order, by a scan of all entities (`crates/turts_sim/src/economy/combat.rs:89-200`).
- Validation: "weapons must be static" (`content_m3_validation.rs:381-382`), "a unit with health
  requires removal" (`:374`), "a unit with a queue cannot be removed" (`:423-427`).
- Mod API: `ENGINE_API_VERSION = "0.2.2"` (`crates/turts_mod_api/src/lib.rs:33`), released list
  (`lib.rs:36`). `Exports` and `Manifest` use `deny_unknown_fields`
  (`crates/turts_mod_api/src/schema.rs:54-66,103-114`). The version table tests the `Manifest`
  only (`crates/turts_mod_runtime/src/version_table.rs:6-31`), called from
  `crates/turts_mod_runtime/src/archive.rs:134`.
- `forge` declares `engine_api: "^0.2.1"` and version 0.1.0 (`mods/forge/manifest.ron`); it has
  no slot balances (`mods/forge/maps/proving_ground.ron:19`). The drone has radius 0.6, speed 2.0,
  health 100 (`mods/forge/units/drone.ron`).
- Test instrument: five files in the legacy ledger, each `until MC`
  (`tests/browser/legacy-bridge.tsv:2-6`); `LEDGER_LIMIT = 5` (`scripts/qa/driver_rule.py:13`).
  No harness directory exists. T1 rules: `scripts/boundary.sh:33-44`; their tests:
  `crates/turts_checks/tests/boundary.rs:88-122`.
- Hosted CI: the step "M2 browser proof" is red on `main` since before QA0 (run 40943729334).
  Task 21 owns the fix, because it rewrites `m2.js` (13.3).
- Completeness gate: a self-test case reads its own rows only (`scripts/qa/completeness.py:172-176`,
  commit 7a90f97).

### 1.7 NOT TRACED

Each item needs a plan task that traces it before code depends on it.

1. The lightyear maximum message size (K4).
2. Real OS key input for Ctrl+digit, F1, F10 and Esc in Chrome and Firefox (K11, probe P1).
   Safari: every key fact.
3. `navigator.keyboard.getLayoutMap` outside Chrome.
4. Whether `PackageArchive` exposes the bytes of a file that no RON export lists (PNG, TTF, OGG).
5. The library load path of `load_package` (K3 option c).
6. Browser frame time, draw calls and memory at 5,000 units with the MC visuals (recorded in MC).
7. The tick cost of avoidance, spreading and combat at 1,000 and 5,000 units in the ECS. MC records
   it. The model figures of section 18 are not ECS figures.
8. Font atlas memory per font size (`bevy_text-0.20.0/src/text.rs:686`).
9. macOS behaviour of Ctrl+click. The platform matrix stays Open (S:751); MC tests desktop Chrome
   and Firefox on Linux only.

## 2. Client parts and the interaction core (D74)

Seat (D74, S:103): the `technical-director` builds the skeleton, the contract types, T1 and T1X.
The `bevy-rendering-engineer` builds the logic and the Bevy adapter. The `ux-designer` tests the
core headless.

### 2.1 Parts

| Part | Crate | Holds | Forbidden dependencies (T1) |
| --- | --- | --- | --- |
| Interaction core | `turts_interaction` (new, K1) | Input map, action dispatch, selection, control groups, smart command, command composition, camera state, feedback events | `bevy*`, `wgpu`, `winit`, `turts_sim`; host and wasm32 |
| Client logic | `turts_client` | Join, replica, interpolation, the adapters from Bevy input and the replica to the core, the widget runtime, presentation | `turts_sim` (unchanged) |
| Web entry | `turts_client_web` | Canvas, focus, bridges, layout labels, storage, the audio resume hook | `turts_sim` (unchanged) |

`turts_interaction` depends on `turts_mod_api`, `turts_net_protocol` and `glam = "=0.33.12"`
(K26). It never sees a Bevy entity.

### 2.2 The core contract

The core is a plain Rust state machine. One call per frame:

    Core::step(&mut self, events: &[InputEvent], world: &dyn WorldView, view: &Viewport,
               now_ms: u64) -> Vec<Effect>

- `InputEvent`: `KeyDown { code, mods }`, `KeyUp { code }`, `PointerMove { px }`,
  `PointerDown { button, px }`, `PointerUp { button, px }`, `PointerLeft`, `Wheel { delta }`,
  `FocusLost`, `UiAction { name }`. `code` is a physical key code (3.1). `PointerLeft` comes from
  the Bevy `CursorLeft` event; Bevy then sets the cursor position to `None`
  (`bevy_winit-0.20.0/src/state.rs:323-325`), so the core must not read a missing position as
  "no edge".
- `WorldView`: the permitted state that the client holds. It gives the units (id, content id,
  owner slot, position, footprint radius, capabilities) and the nodes. It also gives the own
  slot, the orders and order rules of the mod, and the input map.
- `Viewport`: size in logical pixels and scale factor. The core holds the camera state (look-at
  point, height, pitch, yaw, vertical field of view). It projects world points to the screen
  with no renderer. The client copies the field of view into the Bevy `Projection`.
- `Effect`: `Send(OrderEnvelope)`, `SelectionChanged`, `GroupsChanged`, `CameraChanged`,
  `Marker { point, kind }`, `Sound { id, category }`, `Notice { text_key, units }`,
  `Cursor { context }`, `Shell { action }`.

Presentation never writes back into the core, except through `InputEvent`. Test PROJ1
(section 14) compares the core projection with `Camera::world_to_viewport`.

### 2.3 Headless drive

The headless client (`--headless`, `lib.rs:263-283`) adds the core plugin too. A new test hook
`--actions <path>` reads a JSON list of named actions with world coordinates. Examples:
`{"tick": 10, "do": "box_select", "world_rect": [2, 2, 12, 9]}`,
`{"do": "smart_command", "world_point": [20, 8], "queue": false}`,
`{"do": "order", "order": "muster:attack", "world_point": [30, 30]}`,
`{"do": "group_assign", "group": 1}`. The hook turns each action into `InputEvent`s against a
fixed virtual viewport of 1280 x 720. So the headless path runs the same core code as the
browser. `--out` adds `selection`, `groups`, and the per-unit results of each envelope. A bot or
an agent drives the game this way (D74). The server gets a separate hook, `--command-script`, for
the replay test DET1.

### 2.4 T1 forbidden-edge rows

Add to `scripts/boundary.sh` after line 37:

    nobevy turts_interaction $HOST ""
    nobevy turts_interaction wasm32-unknown-unknown ""
    forbid turts_interaction $HOST "" turts_sim wgpu winit
    forbid turts_interaction wasm32-unknown-unknown "" turts_sim wgpu winit

Add the four `checked:` lines to the list in `crates/turts_checks/tests/boundary.rs:92-101`.
Test T1X (section 14) proves that each row can go red (section 13.6).

## 3. Player input map and browser key-capture policy (D66)

Seat: `ux-designer`. Second: `web-platform-engineer`.

### 3.1 Named actions on physical keys

- An action has a name, one or more bindings, and a scope (`world`, `card`, `shell`).
- A binding is a physical key code with modifiers, a pointer button with modifiers, or the
  wheel. The key codes are the W3C `KeyboardEvent.code` names, which Bevy `KeyCode` follows.
- The engine owns the engine actions (table 3.3). The mod declares its own actions, one for each
  order hotkey. The mod can change the default bindings of engine actions (`InputMapDef`).
- The player rebinds any action in the menu (10.2). A stored player binding replaces the mod
  default for that action only.
- Labels: the HUD shows the key label of the current layout. In Chrome the web entry reads
  `navigator.keyboard.getLayoutMap()` once at load. Else it shows the US label of the code
  (NOT TRACED outside Chrome). On AZERTY, `KeyA` shows "Q" and keeps its physical place
  (gap-web-platform-engineer.md, row wpe-2).
- Modifier meaning (IN03), engine default: Shift adds to the selection and queues an order. Ctrl
  selects by type and assigns a group. Alt has no default meaning.
- Double click (IN04): two presses of one button within 400 ms and within 4 px. Both numbers are
  player settings.

### 3.2 Browser key-capture policy

| Case | Policy |
| --- | --- |
| Keys that the browser reserves in a window | Chromium: Ctrl+W, Ctrl+F4, Ctrl+T, Ctrl+N, Ctrl+Shift+T/N/W, Ctrl+Tab, Ctrl+Shift+Tab, Ctrl+PgUp/PgDn (GR:43, F8, from Chromium `accelerator_table.cc:347-359`). Firefox: Ctrl+T, Ctrl+W, Ctrl+N, Ctrl+Q, Ctrl+Tab, Ctrl+Shift+Tab ([bug 1052569](https://bugzil.la/1052569)). Also F11 and F12. The engine refuses each as a default binding (`turts-pack` error) and as a player binding (menu notice). |
| Keys that the page can take | Ctrl+digit, F1 to F10, Tab, Space. The canvas calls `preventDefault` on keydown (winit, because of `lib.rs:254`). Probe P1 confirms this with real OS input before the defaults freeze (K11). |
| Right click | The canvas suppresses the context menu (winit, `lib.rs:254`). The shell counts `contextmenu` events on the canvas and records `defaultPrevented` (test KM2). |
| Focus | Keys reach the game only when the canvas has focus. The shell focuses the canvas at join and on each pointer down on it. On focus loss the core gets `FocusLost` and the shell shows the focus overlay (10.1). |
| Esc | Esc cancels a target mode. Else it closes the open menu. Else it opens the menu. Under pointer lock the browser takes the first Esc to end the lock. So right click also cancels a target mode, and F10 also opens the menu. |
| Pointer lock (K9) | Opt-in setting "Keep the cursor in the game". When it is on, the shell asks for the lock on a click, and the engine draws the mod cursor. The engine cursor sums the `MouseMotion` deltas (`winit-0.30.13/src/platform_impl/web/event_loop/runner.rs:283`). |
| Hidden tab | A hidden tab stops frames (gap-web-platform-engineer.md, row wpe-6). PT1 uses two browser windows side by side. A hidden client keeps its slot (D25); its replica resyncs when it shows again (T8 path). |
| Text entry | No text field in MC (chat is M9). The key-capture mode of the menu suspends all actions until the next key or Esc. |

### 3.3 Default bindings

Defaults of the engine actions. A mod can change them; `forge` and `muster` keep them.

| Action | Default | Browser risk |
| --- | --- | --- |
| `select`, `box_select` | Left click, left drag | none |
| `command` (smart command) | Right click | context menu: suppressed, KM2 |
| `add_modifier`, `queue_modifier` | Shift | none |
| `type_modifier` | Ctrl | macOS: NOT TRACED; the platform matrix stays Open (S:751) |
| `group_recall_1` to `_9` | Digit1 to Digit9 | none |
| `group_assign_1` to `_9` | Ctrl+Digit1 to Ctrl+Digit9 | Chromium: not reserved (source); Firefox: NOT TRACED; P1 decides (K11) |
| `group_add_1` to `_9` | Shift+Digit1 to Shift+Digit9 | none |
| `camera_pan_*` | Arrow keys; screen edge band; middle drag | none |
| `zoom_in`, `zoom_out` | Wheel | Ctrl+wheel zooms the page: the engine ignores Ctrl+wheel |
| `centre_selection` | Space | page scroll: prevented on the canvas |
| `cancel` | Esc; right click in a target mode | Esc under pointer lock: 3.2 |
| `menu` | F10; Esc | none |
| `toggle_guide` | F1 | Chrome help page on F1: prevented on the canvas (P1 checks) |

Order hotkeys come from the mod. `forge` and `muster` use M move, S stop, H hold, A attack (attack-move
on the ground) and G gather. They also use D disable, E enable, R research, and the keys of their
production entries.

### 3.4 Probe P1 (first plan task, before the defaults freeze)

The `web-platform-engineer` sends real OS key input with `xdotool` to a headed Chrome and a
headed Firefox. CDP input bypasses browser accelerators (gap-web-platform-engineer.md, row
wpe-8), so P1 does not use it. A test page logs `keydown`, `defaultPrevented` and the focus.
Keys: every row of table 3.3, Ctrl+1 to Ctrl+9, F1 to F12, Esc with and without pointer lock.
Output: a table of key, browser, and three results (reaches the page, default prevented, browser
acted). The CEO picks K11 with it. Safari stays NOT TRACED.

## 4. Control model (D65)

Seat: `ux-designer`. Second: `simulation-core-engineer`. Decided line kept: a group order is
accepted or refused per unit, with a reason for each refused unit.

### 4.1 Selection

| Gesture | Result | Item |
| --- | --- | --- |
| Left click on a unit | Select it alone. The player inspects a foreign unit read-only (K16); the command card stays empty. | PC01, PC16 |
| Left click on the ground | Clear the selection. | PC01 |
| Left drag (more than 4 px) | Box in screen space. Select each own unit whose footprint centre projects inside the box. If the box holds own mobile units, drop own static units (the mod can change the order: `selection_priority`). No foreign unit enters a box selection. | PC02 |
| Shift+click, Shift+drag | Toggle the clicked unit; add the boxed units. | PC04 |
| Ctrl+click, or a double click on an own unit | Select all own units of that content id whose centre is on the screen. | PC05 |
| Hover | A thin ring on the unit under the pointer; the cursor shows the smart command. After 0.5 s the world tooltip shows the unit name and owner. | PC20, PC21 |

Picking uses the footprint disc on the ground plane, as today (`economy_ui.rs:721-769`). A unit
that dies or leaves the replica leaves the selection and every group in the same frame (PC19).
There is no selection cap (D39). The pick pass keeps a dense index of content ids, so it does not
parse a string per unit per frame (GR F9).

### 4.2 Control groups

- Ctrl+N assigns the selection to group N and replaces the group. Shift+N adds the selection.
  N selects group N. N twice within 400 ms centres the camera on the group (PC08, PC09, PC10).
- A unit can be in several groups. A group holds own unit ids only.
- Groups persist in `localStorage` under the `match_id` and the slot (D72, K10). With the rejoin
  of K27, a reload restores them (PC19).

### 4.3 Smart command (right click)

The core resolves the smart command for each selected unit, from the mod rules
(`OrderRulesDef.smart`, section 11) in list order. Default rules of `forge` and `muster`:

| Target under the pointer | Unit needs | Order |
| --- | --- | --- |
| Foreign unit that the weapon may target | weapon | attack (unit target) |
| Resource node whose item the gatherer accepts | gatherer | gather |
| Ground, own unit, node, foreign unit | movement | move |

The core groups the units by their resolved order and sends one envelope for each group, with
one `group` value for all of them. A unit for which no rule applies gets the local notice
`no_applicable_order`, and the core does not send it. Its engine text is "No order for this target"; `forge`
gives "Cannot move" for its static units (9.3). The server still decides each sent unit (D18).

### 4.4 Explicit orders and target mode

A command card button or its hotkey starts the order. An order with no target (`stop`, `hold`)
goes at once. An order with a target enters target mode: the cursor changes, the next left click
gives the target, and right click or Esc cancels. The explicit `move` order on a point under a
foreign unit moves to that point and does not attack: this is the force move (OR05).

### 4.5 Order queue per unit

- Each unit has one current order and a queue of at most `queue_depth` orders (K24).
- No modifier: the order replaces the current order and the queue. Shift: the order appends.
- Each behaviour has a completion rule and a failure rule (table 5.3). When the current order
  completes or fails, the next queued order starts in the same tick.
- An order that fails after `accepted` writes a failure record (`UnitOrderRecord.last_failure`:
  order, reason, tick). The client shows it as a notice, for example "Trooper: Target out of
  range".
- `hold` and `gather` never complete. The server refuses an `Append` behind either,
  with `not_queueable` for that unit.
- `produce`, `cancel`, `remove`, `enable` and `disable` act at once and never enter the unit
  queue. Production keeps its own queue (M2).
- An idle unit with a weapon fires at the nearest target in range (the M3 rule,
  `combat.rs:165-200`). So the forge turret, which never has an order, acts as in M3.

Retaliation and the full interrupt table (OR26) are MP1.

## 5. Order envelope, per-unit results and protocol 4 (D73, D18, D65)

Seat: `mod-runtime-engineer` (D73). Second: `network-engineer`. Decided line kept: the order type
is a content id in an open, engine-defined envelope.

### 5.1 Wire types (protocol 4)

    PlayerCommand { seq: u32, order: OrderEnvelope }
    OrderEnvelope {
        order: ContentRef,        // content id of an OrderDef, for example "forge:move"
        units: Vec<UnitId>,       // the subjects; empty for an order with no unit
        target: OrderTarget,      // None | Point(Pos) | Unit(UnitId) | Node(NodeId) | Job(JobId) | Content(ContentRef)
        queue: QueueMode,         // Replace | Append
        group: u32,               // 0: none; one value for the envelopes of one gesture
    }
    OrderResult {
        seq: u32, tick: u32,
        accepted: Vec<UnitId>,
        refused: Vec<UnitRefusal>,              // UnitRefusal { unit: UnitId, reason: CommandRejectReason }
        envelope: Option<CommandRejectReason>,  // only for an envelope with no unit
        job_id: Option<JobId>,
    }
    Welcome { slot_id, tick, tick_hz, match_id: u64 }
    UnitOrderRecord { unit, activity: Activity, current: Option<OrderSummary>,
                      queued: Vec<OrderSummary>, last_failure: Option<OrderFailure> }
    // Activity: Idle | Pending | Moving | Attacking | Holding | Gathering | Blocked
    // OrderSummary { order: ContentRef, target: OrderTarget }
    // OrderFailure { order: ContentRef, reason: CommandRejectReason, tick: u32 }

- `Job(JobId)` carries the job of `CancelProduction` (today `messages.rs:67-70`).
- `OrderResult` replaces `CommandApplied` and `CommandRejected` (`messages.rs:84-94,189-194`) and
  their lightyear registrations (`crates/turts_net_lightyear/src/protocol_plugin.rs:40-42`).
  `CommandAck` stays.
- `UnitOrderRecord` goes into `Snapshot` and `WorldDelta` for the owner only, as the M3 owner-only
  records do (`crates/turts_server/src/export.rs:87-93`).
- New reasons, appended to `CommandRejectReason` (`messages.rs:92-122`): `unknown_order`,
  `not_applicable`, `target_invalid`, `target_gone`, `out_of_range`, `rate_limited`,
  `not_queueable`, `blocked`. `queue_full` exists and now also means "the unit queue is full".
  `too_many_units` stays for the cap of K4. `target_not_finite` stays (K6).
- Each refused unit gets its own reason. An envelope that fails as a whole (`bad_sequence`,
  `rate_limited`, `unknown_order`, a malformed target) lists every unit in `refused` with that
  reason. So the client can show a reason for each unit (D65).

### 5.2 Order definitions: mechanism and policy (K2)

The engine gives the behaviour: the mechanism (S:180, "Orders, targeting, timers, effects").
`OrderDef` gives the targeting policy as parameters (S:179, "targeting restrictions" are mod
policy). The engine hard-codes no rule such as "a moving unit does not fire".

| Behaviour (engine) | Mechanism | Policy parameters (`OrderDef`) |
| --- | --- | --- |
| `Move` | Go to the slot of the point (6.3) or follow a unit | `fire`: `Never`, `InRange` (default `InRange`) |
| `Stop` | Clear the current order and the queue | none |
| `Hold` | Stay; no push (M4b) | `fire`: `Never`, `InRange` |
| `Attack` | Unit target: approach to weapon range and fire at it. Point target: move to the slot | `engage`: `None`, `StopAndFire` (point target only; acquisition radius = weapon range) |
| `Gather` | The M2 gather cycle | none |
| `Produce` | Queue a recipe or a technology at a producer | none |
| `CancelProduction` | Cancel a job (`Job` target) | none |
| `Remove` | Remove the unit (M3 rule) | none |
| `SetEnabled(bool)` | Enable or disable a machine | none |

`forge` and `muster` set `fire: Never` on `move` and `engage: StopAndFire` on `attack`. A mod adds
an order by data: `muster:charge` binds `Attack` with its own name, icon, hotkey and cursor, and
the engine source has no string "charge" (test ORD1).

A new behaviour is an engine change for all mods (S:211). It is an additive engine API bump under
the K13 rule (0.2.x, because `^0.2` refuses 0.3.0) with a content-level version row. The row is
present when an `OrderDef` uses the new behaviour. MC widens `VersionTableRow.present` from
`&Manifest` (`version_table.rs:6-10`) to the parsed package content, so this row can exist.

The server resolves `order` through the catalog. It checks each unit in this order:

1. The unit exists.
2. The sender owns it (PC17).
3. The unit has the capability of the behaviour: `movement` for `Move` and for `Attack` on a
   point; `weapon` for `Attack` on a unit; `gatherer` for `Gather`; `producer` or `research` for
   `Produce`.
4. The unit passes the `OrderDef` unit filter.
5. The target is finite, of a valid kind, and exists. Then the clamp of K6 applies.
6. The queue accepts it (`not_queueable`, `queue_full`).

### 5.3 Completion and failure of each behaviour

| Behaviour | Completes when | Fails with (record after `accepted`) |
| --- | --- | --- |
| `Move` | the unit rests on its slot | `blocked` (6.5) |
| `Stop` | at once | none |
| `Hold` | never (a new order replaces it) | none |
| `Attack`, unit target | the target leaves the replica or its health is 0 | `out_of_range` (a static weapon whose target leaves range); `target_gone` (gone before the first shot) |
| `Attack`, point target | the unit rests on its slot with no target in range | `blocked` |
| `Gather` | never (the M2 cycle) | `node_depleted` when the node is empty; the unit goes idle |
| `Produce`, `CancelProduction`, `Remove`, `SetEnabled` | at once, as in M2 and M3 | the M2 and M3 reasons, in `OrderResult` |

### 5.4 Command rate limit (OR31, K25)

The server session enforces the limit before a command enters the simulation queue. Each slot has
two token buckets. One holds 64 envelopes, with a refill of 640 per second. The other holds
16,384 unit orders, with a refill of 163,840 per second. The server refuses an envelope that does not fit, with `rate_limited`
for each unit. The command log of G9 holds only admitted envelopes, so a replay (invariant 15, S:267)
needs no bucket state. M7 measures the numbers (S:731).

### 5.5 Version and compatibility

`NET_PROTOCOL_VERSION` goes from 3 to 4 (K14). The server refuses a version 3 `Hello` as today
(`session.rs:97`). The protocol id of netcode stays (`crates/turts_client/src/join.rs:275`,
`crates/turts_server/src/net.rs:44`). Tests that pin 3 (`crates/turts_net_protocol/tests/wire.rs`,
`tests/protocol3.rs`) move to 4. The test hooks `--move` and `--move-unit` send an envelope with
the first `OrderDef` of the package whose behaviour is `Move`.

## 6. Simulation: orders and flat-ground group movement

Seat: `simulation-core-engineer`. Second: `ux-designer` for the player-visible result.

### 6.1 Phase, state and audit

- A new set `SimSet::Orders` runs first in the `FixedUpdate` chain, before `SimSet::Movement`
  (`plugin.rs:88-101`). So the order is: `Commands` (`FixedPreUpdate`), then `Orders`,
  `Movement`, `Gather`, and the rest. `Orders` starts queued orders, applies completion and
  failure (5.3), and runs spreading (6.3).
- `UnitOrders { current: Option<Order>, queue: VecDeque<Order> }` on each unit with `movement` or
  `weapon`. Gather keeps its M2 state (`EconomyOrders`): a gather order sets it, `stop` clears it.
  `MoveTarget` holds the slot of the current move.
- Plain orders write no operation. A plain move is one `Move` order for one unit, with no queue
  and no group of more than one unit. Today `apply_move_to` writes `MoveTarget` with no operation
  (`crates/turts_sim/src/command.rs:236-252`), and `assign_gather` writes `EconomyOrders` with no
  `transact` call (`crates/turts_sim/src/economy/gather.rs:88-175`). MC keeps both paths. A plain move
  writes only the command operation of today, and no order operation; a gather order writes no
  order operation. The movement function ends a plain move: it removes `MoveTarget`, as today.
  That is not an audited order write.
- Every other order write (start, completion, failure, queue change) runs inside
  `capture_operation` (`transaction.rs:90`), one unit per operation. The `Orders` phase writes
  each `blocked` failure (5.3) this way. The movement step never writes an order.
- The validator follows K19 (a). The movement step is one pure function over unit records. The
  `Movement` system and `validate_movement_transition` (`economylog.rs:578-663`) both call it.
  The `movement_pending` rule (`economylog.rs:326,553-556`) stays. Movement adds no operation.
- `AuditState` (`views.rs:300`) holds each input of the movement function, so the validator can
  run it. For each unit: position, slot, radius, speed, `UnitOrders`, group id, activity. Also the
  pass mode (6.4), the stall timer, the best distance and the claim count (6.5). For each group:
  its slot pool with the spares. Also the static footprints. `AuditState` writes each of these fields
  for every unit when its value is not the default. So a stalled plain mover or gatherer is
  covered, and a state of today, with default values, keeps its fields. Log size: about 60 bytes
  for each unit with a value that is not the default, and 20 bytes for each pool slot, in each
  captured state (estimate, NOT TRACED; EL1 records the real figure).
- Test EL1 runs `muster` with the economy log on: GM40, GMA and a queued gather. Under K19 (a)
  the validator accepts each tick on a correct build. It is red on a build that writes positions
  or `UnitOrders` outside the movement function and `capture_operation`.
- All iteration is in unit id order, with no hash-map iteration order. So the same build gives
  the same result for the same commands (invariant 15, S:267).

### 6.2 Weapons on mobile units (K5)

Lift `content_m3_validation.rs:381-382`. `choose_target` (`combat.rs:165-200`) gets one more
argument: the ordered target of an `Attack` order on a unit wins when it is alive and in range.
The `fire` and `engage` parameters of the current `OrderDef` (5.2) decide the rest. A unit with no
order fires at the nearest target in range (M3 rule). No forge unit gets an order in the M3
cases, so the forge streams do not change (REG1).

Target choice uses the neighbour grid of 6.4 instead of a scan of all entities
(`combat.rs:89-199`). The cost is then the weapons times the units in range cells.

### 6.3 Destination spreading (GM01, OR25)

When an envelope, or the merged envelopes of one key (slot, `group`, tick), moves N units to one
point P:

1. Spacing: s = 2 x r_max + 0.1, where r_max is the largest footprint radius in the group.
2. Reserved cells: the slots of resting units of other envelopes, and the live slots of moves in
   progress. The pool skips a slot within s of a reserved cell. It also skips a slot outside
   the map (inset by r_max) and a slot inside a static footprint.
3. Pool: a hexagonal grid round P. The pool holds N + max(6, ceil(N / 4)) slots, sorted by
   distance to P, ties by angle. The first N are the primary slots; the rest are spares.
4. Order-keeping assignment, O(N log N): take the travel direction from the group centroid to P.
   Sort the units and the primary slots by their position along that direction. Cut both lists
   into bands of round(sqrt(N)). In each band, sort by the lateral position and pair them in
   order. Ties use the unit id. One unit alone takes P itself (after the clamp).
5. Each unit moves to its slot. A shift-queued move to a point uses the same rules.

A part of a group that arrives in a later tick (K4) spreads round the reserved slots of the
earlier part. Model run C2 (section 18): 0 overlaps and 0 blocked units.

The reserved-cell test of step 2 uses the neighbour grid of 6.4, so each slot test is O(1).

Work cap (S:651, 13.6). The simulation spreads at most 2,048 units in one tick, for all slots
together. Each slot has its own pending list, in arrival order, with a limit of 16,384 units. The
server refuses an envelope beyond it with `rate_limited` for each unit. Each tick the slots take
turns (round robin, starting one slot later each tick). The head envelope of the turn always
spreads in full, also above the cap (for example 4,096 units, K4). Then the cap counts it, and
the next envelopes wait. So one slot cannot delay the others. Units of a waiting envelope
keep their previous order and show the activity `Pending`. The selection panel and the info
panel show "Order pending" for them. The client
still shows the marker in the frame of the click (9.2). The worst case under K25 is 8 slots x
16,384 unit orders (the bucket burst, K25): 64 ticks of wait. The gate 1 sim probe measured 7.9 ms for one
spreading of 8,192 units.

### 6.4 Local avoidance (GM04, GM06)

- The collision size of a unit is its footprint radius (GM06).
- A uniform grid with cell size 2 x r_max gives the neighbours, in O(N) per tick.
- Separation reach: r_i + r_j + 0.05. This is less than s, so units on adjacent slots do not
  push each other.
- Units of other groups: a moving unit adds a separation vector from each neighbour inside the
  reach. When the neighbour is ahead, it adds the same amount to the right of its travel line.
  After the step, the unit moves out of any overlap with a unit of another group (hard
  projection).
- Units of the same group: two moving units separate as above, with no hard projection. A
  resting unit of the same group yields: a moving unit of its group passes it. A moving unit
  that makes no progress for 1 s stops separation from its own group until it reaches its slot.
- A static footprint pushes a moving unit out. An idle or holding unit of another group does not
  move for others in MC (GM07 push is M4b).
- Units resolve in unit id order (invariant 15).

### 6.5 Arrival (GM09)

- A unit within one step of its slot, or within 0.025 of it, lands exactly on the slot. Resting
  neighbours are then at least s apart, so their discs do not overlap (gap 0.1).
- A unit at rest stays at rest until an order moves it.
- A unit lands on its slot only when the slot is free of other units. If a unit of its group
  passes over the slot, the unit waits on it.
- A unit can make no progress for 1 s against units of other groups. Then it takes the nearest
  free slot of its pool, at most 3 times. After that its activity becomes `Blocked` (PF04: "gives
  up visibly"), and its order fails with `blocked` (5.3).
- A unit leaves every overlap before it rests. A `Blocked` unit can overlap a unit of any group.
  Then it walks at most one step per tick to the nearest free point, and then it rests. Model cases F and I (section 18): 0 overlaps at rest.

### 6.6 Unreachable fallback (GM10, K6)

- A non-finite target: refused with `target_not_finite`.
- A finite target outside the map: clamped to the nearest point inside the map, inset by the unit
  radius. The result is `accepted`.
- A target inside a static footprint: the slots of 6.3 skip the footprint, so the units stand on
  a ring round it.
- MC has no terrain, so no other target is unreachable. M4 adds routes.

### 6.7 Cost

- Spreading: O(N log N) per order, at most 2,048 units per tick (6.3).
- Avoidance: O(N) per tick with the grid.
- Combat: weapons times the units in range cells (6.2).
- Plan task: record the tick time of spreading, avoidance and combat at 1,000 and 5,000 units on
  `muster` in `docs/measurements/mc.md`. No budget (section 15 rows of S stay Open).

## 7. Camera controls (D68) and the mod light (K23)

Seat: `bevy-rendering-engineer`. Second: `ux-designer`.

- The core holds the camera state (2.2). The Bevy camera copies it. MC removes the `PostUpdate`
  fit (`render.rs:245,455-456`) and the fixed start (`render.rs:338-342`).
- Pan: arrow keys, the edge band (K9: 8 logical px inside the canvas), middle drag. The pan speed
  scales with the camera height. The player sets the speed (CA16) and can turn the edge band off.
  On `PointerLeft` the edge band stops.
- Zoom: wheel steps between the mod minimum and maximum height.
- Bounds (CA13): the look-at point stays inside the map rectangle plus the mod margin.
- Centre: Space centres on the selection (CA14); a double tap of a group key centres on the group
  (PC10).
- Start: the mod chooses `OwnedCentre`, `MapCentre` or `Point(x, y)`. `muster` starts on the
  troopers.
- Mod constraints (D37): `CameraDef` in the interface file: pitch, yaw, field of view, minimum,
  maximum and start height, margin, pan speed, edge band. A classic RTS camera is the default
  (S:121), not a rule.
- Light (K23): `LightDef` in the interface file: direction, colour, illuminance, ambient
  brightness. It replaces the fixed light of `render.rs:344`. The default is today's light.
- Minimap click: a left click or a drag on the minimap view moves the look-at point. A right click
  sends the smart command at that ground point (8.2).

## 8. HUD widget set and mod schema (D67)

Seat: `ux-designer`. Second: `mod-runtime-engineer`. Decided line and constraint kept:

- The engine supplies primitives. The mod supplies layout and content.
- A mod places any element at any position and size, with its own images, fonts and cursors.
- The engine fixes no layout and reserves no region.
- Mod code paints no pixels at run time.

### 8.1 What the engine draws and what the mod draws

- During play, every HUD element is a mod widget. The engine draws world-space visuals (rings,
  bars, markers: section 9), styled by mod values, and the cursor (mod images).
- The shell draws only modal screens: load, focus overlay, menu, rejection, connection lost
  (section 10). A modal screen covers the whole canvas while it is open. It is not a reserved
  region of the HUD.
- MC removes the engine panel (`economy_ui.rs:83-337,869-1104`) and its English text. The forge
  text moves into `mods/forge` content (section 12).
- Images load with the usage `RENDER_WORLD` only, so Bevy keeps no CPU copy
  (`bevy_asset-0.20.0/src/render_asset.rs:40-51`). Cursor images keep the CPU copy, because the
  window cursor needs it (`bevy_winit-0.20.0/src/cursor/mod.rs:144-170`). A cursor loads as its
  own asset, never through the handle of an icon with the same path. The asset server returns
  one handle for each path (`bevy_asset-0.20.0/src/server/mod.rs:324`).

### 8.2 Primitives

| Primitive | Fields (besides placement) | Notes |
| --- | --- | --- |
| `Box` | `fill`, `border`, `children` | Fill: a colour or an image with optional nine-slice. |
| `Image` | `image`, `nine_slice`, `tint` | `image` is an asset id, or `Item(Icon)` in a template. |
| `Text` | `text`, `font`, `size`, `colour`, `align`, `wrap` | `text`: `Literal`, `Bind(<value>)`, `Item(<field>)`, or `Format(<pattern>, [<value>, ...])`. |
| `Bar` | `value`, `max`, `fill`, `background`, `direction` | Health, progress, balances. |
| `Button` | `icon`, `label`, `action`, `hotkey_label` | States: normal, hover, pressed, disabled. A disabled button keeps its reason for the tooltip. |
| `List` | `source`, `direction`, `gap`, `max_items`, `item` | `item` is a widget template. |
| `Grid` | `source`, `columns`, `rows`, `cell`, `gap`, `item` | Command card, build menu, selection icons. |
| `Tooltip` | `tooltip` field on any widget; style in `tooltip_style` | Shows name, cost, hotkey, requirement or reason, description. |
| `Cursor` | `cursors` map: context to `(image, hotspot)` | Contexts: `default`, `select`, `move`, `attack`, `gather`, `target`, `forbidden`, `pan_*`. |
| `MinimapView` | `background`, `frame`, `unit_size`, `ping` | Units and nodes in slot colours, the camera frame, alert pings; left click pans, right click orders. |

When a mod needs a missing primitive, the engine adds it for all mods (S:96, S:184).

### 8.3 Placement

Every widget has:

    at: (anchor: <A>, offset: (x, y), pivot: <A>?)   // A: TopLeft, Top, TopRight, Left, Centre,
                                                    // Right, BottomLeft, Bottom, BottomRight, Frac(x, y)
    size: (w: <S>, h: <S>)                           // S: Px(n), Frac(f), Fill, Fit
    layer: i16                                       // draw order, default 0
    visible: [<predicate>, ...]                      // all must be true; empty means always

- The anchor is a point of the parent (the canvas for a root widget). The pivot is the matching
  point of the widget (default: the anchor). The offset is in logical pixels times the UI scale.
- So any position and any size is expressible, and nothing in the engine positions a widget.
- Predicates: `SelectionEmpty`, `SelectionNotEmpty`, `SelectionSingle`, `SelectionMany`,
  `PrimaryOwned`, `PrimaryForeign`, `PrimaryHas(<capability>)`, `GuideOpen`, `Not(<predicate>)`.
- Bevy `UiScale` takes the player UI scale only (0.75 to 2.0, menu). Bevy multiplies it by the
  device scale factor itself (`bevy_ui-0.20.0/src/update.rs:57-62`). A canvas from 1280 x 720 to
  3840 x 2160 uses the same tree.

### 8.4 Sources and values (permitted queries)

A widget reads only these. All come from the client replica, which holds only permitted state
(S:184, S:522-524):

| Source or value | Content |
| --- | --- |
| `Balance(<resource id>)`, `Balances` | own slot balances |
| `SelectionPrimary.<field>` | Name, icon, portrait, owner, health, max health, inventory lines. An own unit with a weapon adds damage (from `WeaponRecord.damage`), range, ammo and cooldown. A foreign unit with a weapon adds range only (catalog). |
| `SelectionUnits` | one item per selected unit: icon, health, count by type |
| `CommandCard` | one item per order of the selection: icon, name, hotkey, action, enabled, reason, card cell |
| `BuildMenu` | one item per production entry or technology of the primary producer: icon, name, cost, time, enabled, reason |
| `ProductionQueue` | one item per job of the primary producer: icon, progress 0 to 1, remaining seconds, cancel action |
| `ControlGroups` | count and icon per group |
| `Alerts` | the last 5 alert lines |
| `LastNotice` | the last refusal or notice text |
| `Guide`, `GuideOpen` | the guide pages of the interface file |
| `Bindings` | one item per action: a player label (for example "Assign group"), the current key label (3.1) |

A button action is `Order(<order id>)`, `Item(Action)` in a template, or `Input(<action name>)`.
A widget never reads hidden state; a test (HUD3) refuses a binding name outside this table.

### 8.5 Example: the D67 probe

This tree puts the command card top left and the minimap bottom right. Test HUD1 loads it in
the fixture package and checks each rendered rectangle.

    (
        id: "hud",
        root: [
            (id: "card", at: (anchor: TopLeft, offset: (8, 8)), size: (w: Px(272), h: Px(204)),
             visible: [SelectionNotEmpty, PrimaryOwned],
             widget: Grid(source: CommandCard, columns: 4, rows: 3, cell: (64, 64), gap: 4,
                 item: (id: "b", size: (w: Fill, h: Fill),
                        widget: Button(icon: Item(Icon), action: Item(Action), hotkey_label: true)),
                 tooltip: Item)),
            (id: "minimap", at: (anchor: BottomRight, offset: (-8, -8)),
             size: (w: Px(200), h: Px(200)),
             widget: MinimapView(frame: "#ffffff")),
            (id: "ore", at: (anchor: Bottom, offset: (0, -4)), size: (w: Px(160), h: Px(28)),
             widget: Text(text: Format("Ore {0}", [Balance("muster:ore")]), size: 18)),
        ],
    )

### 8.6 HUD items of S:722 and where each lives

| S:722 item | Widget and source | Test |
| --- | --- | --- |
| Selection panel | `Grid(SelectionUnits)` with `Image` and `Bar`; a click narrows, Shift+click removes | HUD2 |
| Info panel | `Text`, `Image`, `Bar` on `SelectionPrimary` | HUD2 |
| Command card | `Grid(CommandCard)` with `Button`; hotkey label; disabled with reason | HUD2 |
| Build menu | `Grid(BuildMenu)`; disabled entries name the missing requirement (PR10) | HUD2 |
| Tooltips | `tooltip` on any widget | HUD2 |
| Production queue with progress bar | `List(ProductionQueue)` with `Bar`; click cancels | HUD2 |
| Resource bar | `Text` and `Image` on `Balance` (`muster`; `forge` has no balances) | HUD2 |
| Minimap with minimap click | `MinimapView` | MM1 |


## 9. Visuals, feedback, refusal reasons, alerts and audio

Seat: `bevy-rendering-engineer` (presentation, D69). Second: `ux-designer`.

### 9.1 Selection and status visuals

- Selection ring: a flat ring under each selected unit, in the slot colour, with the footprint
  radius. Own units: a solid ring. Foreign units: a ring with four gaps. So the relation does not
  depend on colour alone.
- Hover: a thin ring under the unit under the pointer. After 0.5 s the world tooltip shows the
  unit name and owner (style `world.tooltip` in the interface file).
- Range circle: a selected unit with a weapon shows a thin circle at its weapon range. This
  applies to own and foreign units, because the range is public catalog data. The mod can turn it
  off.
- Health bars (UI12): over units with a health record. A menu setting picks `selected`,
  `damaged or selected` (default) or `always`. The mod gives size and colours
  (`world.health_bar`).
- Team colours (AN07, TM03): each slot has one material in its colour. This replaces the one
  material of `render.rs:403-406`. Default palette (Okabe-Ito): `#0072B2`, `#E69F00`, `#009E73`,
  `#D55E00`, `#56B4E9`, `#CC79A7`, `#F0E442`, `#000000`. The mod can give its own
  (`slot_colours`). Test VIS1 checks the palette under colour-vision simulation.
- Order markers (AN06): a marker shows at the target in the frame of the click. Move: a ring.
  Attack: a cross. Gather: a dot on the node. The marker of a move stays until every unit of that
  envelope rests or is `Blocked`, with a limit of 60 s. Waypoint lines show the queued orders of own selected units, from
  `UnitOrderRecord`.
- A `Blocked` unit shows a small mark over it for 2 s.

Draw method (5,000 units, D57):

- One shared ring mesh and one shared bar mesh for all units.
- One unlit material per slot colour, plus one for each bar colour.
- A bar fill is a child entity whose x scale is the health fraction.
- The client writes a unit transform only when its presented position or facing changes
  (today it writes every frame: `render.rs:383,390`).
- Plan task: record frame time (p50, p99), draw calls and memory at 5,000 units in Chrome WebGPU
  in `docs/measurements/mc.md` (VIS2). No budget.

### 9.2 Order acknowledgement timeline (OR23, PE01, PE07, PF05)

| When | What the player sees and hears |
| --- | --- |
| The frame of the click | The marker, the cursor click state, and one unit response sound (voice limit 1). |
| `CommandAck` | Nothing new. |
| `OrderResult` | Accepted units: their waypoint lines. Refused units: one notice line for each reason, for example "2 units: Cannot move", and the error sound. |
| The first applied tick | The units start to move (PF05): the order applies in the first tick that drains it. |
| A failure after `accepted` | A notice from `last_failure`, for example "Trooper: Target out of range". |

The client never waits for the server to show that a click registered. It never shows a predicted
position for a unit (no client prediction, D18).

### 9.3 Refusal reasons (OR22)

Each reason code has a player text. The engine ships an English default for each engine reason.
Examples: `not_owner` "Not your unit"; `not_applicable` "Cannot do that"; `out_of_range` "Target
out of range"; `rate_limited` "Too many orders"; `no_applicable_order` "No order for this
target". The mod overrides any text in its interface file (`reasons`). `forge` gives
`no_applicable_order` the text "Cannot move". The notice names the unit and the count, for
example "Shell Turret: Cannot move". A notice never shows a raw code.

### 9.4 Alerts (UI16, AU02) and machine problems

Alert rules are mod content (`alerts` in the interface file). Each rule names one engine trigger,
a text, an optional sound, a minimap ping flag, a cooldown and a priority. The client derives the
triggers from permitted replicated records (K8):

| Trigger | Source record |
| --- | --- |
| `OwnUnitDamaged` | a health drop of an own unit (`HealthRecord`) |
| `OwnUnitDestroyed` | an own unit leaves the replica after a health record of 0 |
| `ProductionComplete`, `ResearchComplete` | job and research records |
| `ResearchReady` | an own research station changes from "cannot start" to "can start" a technology. The client computes this from the station inventory and the catalog cost. It also fires once at join for each station that can start one. The selection plays no part. |
| `ExtractorFull`, `ExtractorDepleted` | `ExtractorStatus` (`messages.rs:226-231`) |
| `OutputBlocked` | `JobStage::OutputBlocked` (`economy.rs:83-86`) |
| `OutOfAmmo` | `WeaponStatus::OutOfAmmo` (`messages.rs:239-245`) |
| `InserterWaiting(seconds)` | an `InserterStage` wait reason for longer than the given time |
| `GathererWaiting(seconds)` | `GatherStage::Waiting` |

Engine limits (D69): each rule has a cooldown of at least 1 s. Alerts of one trigger within
`coalesce_radius` of each other count as one. At most 2 alert sounds play at once; the lower
priority drops. The text list keeps the last 5. A click on an alert line centres the camera on its
place.

### 9.5 Audio (K7, D69)

- One format: OGG Vorbis (Bevy feature `audio`: `bevy_audio` and `vorbis`, gate 1 bevy run).
  The mod gives sound ids in `AssetsDef`.
- Categories: `master`, `voice` (unit responses), `alerts`, `effects` (click, error). Each has a
  volume in the menu (AU06).
- Unit responses (AU01): the interface file maps a unit content id to `select` and `order` sound
  lists. One voice plays at a time; a new one replaces the old one.
- Resume: Bevy opens the audio sink when the plugin builds
  (`bevy_audio-0.20.0/src/audio_output.rs:20-31`), and cpal calls `resume()` only inside `play()`
  (`cpal-0.17.3/src/host/webaudio/mod.rs:430`). Bevy has no resume call. So the web entry adds a
  hook: a page script, loaded before the wasm, records each `AudioContext` that the page creates.
  On the first pointer down or key down, it calls `resume()` on each of them. Until then the client spawns
  no audio player entity, so no sound waits in a queue.
- Positional sound, weapon sounds and the sound cap of AU07 are MP1.

## 10. Client shell (D70, client side), persistence (D72) and rejoin (K27)

Seat: `ux-designer` (client side of D70); `backend-engineer` owns the server side. D72 seat:
`web-platform-engineer`.

### 10.1 Screens in MC

| Screen | Content | Owner |
| --- | --- | --- |
| Load | The phase (fetch, verify, join), a progress bar, the package name and version | shell |
| Rejection | The reason of `rejected_reason` in player words, and what to do | shell |
| Focus overlay | "Click to play" over a dimmed game, when the canvas has no focus or audio is locked | shell |
| Menu (F10 or Esc) | One screen: key bindings with capture and a conflict notice; volumes; scroll speed; edge scroll on or off; UI scale; health bar mode; cursor lock (K9); double-click time | shell |
| Connection lost | "Connection lost" and a reload button when `reconnect_required` is set (`input.rs:46`) | shell |
| HUD, guide | Mod widgets | mod |

The shell uses an engine style (font, colours) that is not mod content. Identity, the server
browser and the main menu are M8 and M10 (S:732-734).

### 10.2 Rebinding

The menu lists each action with its bindings. "Change" enters capture mode; the next key or
button with modifiers becomes the binding. The menu refuses a reserved key (3.2) with a notice. A
binding that is already in use asks to swap or cancel. "Reset" restores the mod default.

### 10.3 Canvas

The page canvas fills the browser window (`index.html:8` changes from a fixed 1280 x 720 box to
100 % of the viewport). The web entry gives the canvas `tabindex`, so it can take focus.

### 10.4 Persistence (D72, K10)

| Data | Browser key | Native file |
| --- | --- | --- |
| Player id (K27) | `sessionStorage`: `turts.v1.player_id` | `--player-id` (exists) |
| Bindings | `localStorage`: `turts.v1.bindings.<mod id>.<line>` | `<user config dir>/turts/bindings-<mod id>-<line>.json` |
| Settings | `localStorage`: `turts.v1.settings` | `<user config dir>/turts/settings.json` |
| Control groups | `localStorage`: `turts.v1.groups.<match_id>.<slot>` | `<user config dir>/turts/groups-<match_id>-<slot>.json` |

- `<line>` is the semver compatibility line of the mod version: the major number when it is 1 or
  more, else `0.<minor>`. So `forge` 0.2.0 uses `0.2`, and a `forge` 0.3.0 starts from its own
  defaults.
- Storage is per origin, so each server has its own copy (wpe-9). `sessionStorage` belongs to one
  window and survives a reload of that window, so two windows on one origin get two ids.
- A value that does not parse gives the defaults and one notice. At join, the client deletes the
  groups of each `match_id` other than the current one.
- These are client-local data, not game saves (D72, S:101).

### 10.5 Rejoin after a reload (K27)

- Today a reload makes a new random id (`lib.rs:76`). The server refuses it with "no open slot"
  when other players hold both slots (`slots.rs:45-55`, `session.rs:103-105`).
- Client: at the first join the web entry stores the id in `sessionStorage`; a reload reads it
  back.
- Server change, in `judge_hello` (`crates/turts_server/src/session.rs:92`) only. `Slots::claim`
  stays as it is. When `claim` gives `AlreadyClaimed` and no live session uses that slot, the
  server gives the same slot back, with a new `Welcome` and a snapshot.
- While the old session lives, the server refuses with "This game is open in another window".
  The old session lives until the netcode timeout of 10 s (`crates/turts_client/src/join.rs:278`),
  unless the server sees the WebSocket close first (NOT TRACED).
- The player sees "Reconnecting to your game" with a countdown from 15 s. The client sends a new
  `Hello` each second. After 15 s with no `Welcome`, the client shows "This game is open in
  another window. Close it, then reload this page".
- Invariant 11 (S:577) holds: the slot never changes owner. Test SLOT1 checks it.
- A duplicated browser tab copies `sessionStorage`, so it gets the refusal above.

## 11. Mod API growth (engine API 0.2.3)

Seat: `mod-runtime-engineer`. Second: `ux-designer` for the interface types.

### 11.1 Manifest

`Exports` (`schema.rs:54-66`) gets four lists. Each is `#[serde(default, skip_serializing_if =
"Vec::is_empty")]`:

| Field | Files | Read by |
| --- | --- | --- |
| `orders` | `OrderDef` and `OrderRulesDef` files | server and client |
| `interface` | `InterfaceDef` files: HUD tree, cursors, camera, light, slot colours, alerts, reasons, unit presentation, world style, guide | client; validated on both paths |
| `input` | `InputMapDef` files: mod actions and default bindings | client; validated on both paths |
| `assets` | `AssetsDef` files: images, fonts, sounds, cursors by id | client; header checks on both paths |

No field of `UnitDef` changes. The unit presentation (icon, portrait, responses) is a map from
unit id in the interface file.

### 11.2 New types (all `deny_unknown_fields`, no Bevy type, D15)

    OrderDef(id, name, behaviour: Behaviour, target: TargetKind, fire: Option<FirePolicy>,
             engage: Option<EngagePolicy>, icon: String, hotkey: String, cursor: Option<String>,
             card: Option<(row: u8, column: u8)>, units: [ContentRef], description: String)
    // Behaviour: Move | Stop | Hold | Attack | Gather | Produce | CancelProduction | Remove | SetEnabled(bool)
    // TargetKind: None | Point | Unit | PointOrUnit | Node | Job | Content
    // FirePolicy: Never | InRange       EngagePolicy: None | StopAndFire
    OrderRulesDef(queue_depth: u8, smart: [(target: SmartTarget, needs: Capability, order: ContentRef)])
    InputMapDef(actions: [(name: String, default: [Binding])], engine_overrides: [(name, [Binding])])
    // Binding: Key(code: String, mods: [Mod]) | Pointer(button: Button, mods: [Mod]) | Wheel(dir)
    AssetsDef(images: [(id, path, nine_slice: Option<(u16, u16, u16, u16)>)],
              fonts: [(id, path)], sounds: [(id, path)], cursors: [(id, image, hotspot: (u16, u16))])
    InterfaceDef(id, root: [Widget], tooltip_style, cursors: {context: id}, camera: CameraDef,
                 light: Option<LightDef>, slot_colours: [String], world: WorldStyle,
                 alerts: [AlertDef], reasons: {code: String}, units: {ContentRef: UnitPresentation},
                 selection_priority: [ContentRef], guide: [GuidePage])
    LightDef(direction: (f32, f32, f32), colour: String, illuminance: f32, ambient: f32)

`queue_depth` is at least 1 and at most 255 (K24). A package with no `OrderRulesDef` uses 16.

### 11.3 Validation (each error names package, file and field: MD18)

- Each id that a widget, order, cursor or unit presentation names exists, with the right kind.
- `OrderDef.behaviour` and `target` agree (for example `Stop` takes `None`, `CancelProduction`
  takes `Job`). `hotkey` names an action of an `InputMapDef`. `card` cells do not collide for one
  unit type.
- Bindings: a code is a known physical key code. A reserved browser key (3.2) is an error.
- Sources, values and predicates are in tables 8.3 and 8.4.
- Assets: the file exists in the archive, and the PNG, TTF or OTF, and OGG headers match. The
  K12 caps hold: decoded image bytes from each PNG IHDR (width x height x 4), font bytes, sound
  durations from the OGG headers. The decoder does not run on the server.
- `turts-pack` warns, with exit 0, for text under 11 px and buttons under 24 x 24 px (K18).
- Content names of `mods/forge` and `mods/muster` do not appear in engine code
  (`crates/turts_checks/tests/no_content.rs` gets the new names).

### 11.4 Version

- `ENGINE_API_VERSION` 0.2.2 to 0.2.3 (`lib.rs:33`); append "0.2.3" to `RELEASED_ENGINE_APIS`
  (`lib.rs:36`). Additive: each 0.2.2 package loads unchanged, because `^0.2`, `^0.2.1` and
  `^0.2.2` match 0.2.3 (K13).
- Four rows in `VERSION_TABLE` (`version_table.rs:20-31`), each `since_api: "0.2.3"`, present when
  the list is not empty: `exports.orders`, `exports.interface`, `exports.input`,
  `exports.assets`. A package that uses any of them declares a requirement that excludes 0.2.2,
  for example `^0.2.3`.
- `VersionTableRow.present` takes the parsed package content, not only the `Manifest` (5.2), so a
  later behaviour can have a content-level row.
- No fallback set (K3 b). A package that exports no orders loads, but its units take no order;
  the client shows "This game defines no orders" once.
- The client compares the advertised engine version for equality (QA0D section 8), so the browser
  client and the server build from one commit.

## 12. Packages

Seat: `ux-designer` writes the interface content. `mod-builder` checks that a modder can write it
from the schema alone.

### 12.1 `forge` 0.2.0

- `engine_api: "^0.2.3"`. Maps, units, recipes and technologies do not change, so the M3 fixed
  point and the frozen streams stay (REG1).
- New files: `orders/*.ron` (move, stop, hold, research, cancel, enable, disable, remove),
  `orders/rules.ron`, `input/bindings.ron`, `assets/assets.ron` (icons, a portrait for each unit
  type, one UI font, cursors, sounds), `interface/hud.ron`.
- The HUD has a selection panel and an info panel. The info panel shows the inventory lines, and
  damage, range and ammo for the own turret. The HUD also has a command card and the lab research
  entry with its reason. It has the production queue with a bar in seconds, the alert lines, the
  minimap, and the guide. Forge has no slot balances
  (`proving_ground.ron:19`), so it has no resource line.
- Alerts: `ResearchReady`, `ResearchComplete`, `OutOfAmmo`, `OutputBlocked`, `InserterWaiting(5)`
  for slot 0; `OwnUnitDamaged` and `OwnUnitDestroyed` for slot 1.
- The guide (F1, open at the first join): five short pages in player words, with no coordinate
  and no studio word. Page 1: what the factory makes. Page 2: how to research. Page 3: how to
  pause a machine. Page 4: what the drone player does. Page 5, "Controls": a `List` on the source
  `Bindings`, so the key labels come from the current input map, and a rebinding updates the
  page. It shows select, box select, order and queue (Shift+right click). It also shows one row
  for each kind of group action: assign (Ctrl+1 to 9), add (Shift+1 to 9), recall (1 to 9), centre
  (the group key twice). Then stop, the menu (F10), and the guide (F1).
- A groups row: a `List` on `ControlGroups` shows each group with its key and unit count. The guide is the only help that the blind
  tester gets (S:708).
- The text of `economy_ui.rs:122-125` and the other engine strings move here, in player words (no
  "ticks", no "Applied": research time in seconds).

### 12.2 `acme`, `base`, `cedar`, `haul` (K3 b)

Each gets `engine_api: "^0.2.3"` and a new minor version. Each gets `orders/` (the orders its
units can use), `orders/rules.ron`, `input/bindings.ron`, and a plain `interface/hud.ron` with text
buttons and no images. Their maps and units do not change. The M2 browser cases run on these HUDs (13.3).

### 12.3 `muster` 0.1.0 (new, K15)

- One map, 64 x 48, with one ore node.
- Slot 0: 40 `trooper` (radius 0.5, speed 3.0, weapon range 4, damage 5, cooldown 1 s, health
  50, removal). Also 2 `worker` (gatherer), 1 `depot` (drop-off), 1 `post` (static, radius 2).
  Also 460 `marker` units (mobile, no weapon) in a far block for SEL2.
- Slot 1: 10 `target` (mobile, health 40, no weapon) and 1 `post`. Slot 0 starts with an ore
  balance, so the resource line has a source.
- Orders: the forge set plus attack, gather and `charge` (`Attack` with another name and icon, for
  ORD1).
- The camera starts on the troopers (`CameraDef.start: Point`).
- The D67 probe layout of 8.5 lives in a test fixture copy of `muster` under
  `crates/turts_pack/tests/fixtures/` (HUD1), not in `muster` itself.

## 13. Test instrument

Seat: `qa-lead`. Second: `ux-designer` (what a player can fail), `web-platform-engineer`
(browser).

### 13.1 Harness

`tests/browser/harness/run.js` and `harness/driver-io.js` (role `harness`), as QA0D section 4.3
gives them (`io.click`, `io.drag`, `io.key`, `io.screenshot`, `io.save`, `io.wait`;
`view.bridge`, `view.inputLog`, `view.screenshot`, `view.serverFile`). MC adds:

- `io.move(x, y)` and `io.wheel(dy)`, from `page.mouse` only, for hover, edge scroll and zoom.
  The input log records them.
- `io.key(code, modifiers, opts)` sends the physical `code`. `opts.key` can give another logical
  key, through the CDP key event that the harness builds (the AZERTY case of KM1). The driver
  never holds `page`.
- `io.reload()`: a page reload, for SEL4, PER1 and SLOT1.
- Viewports: 1280 x 720, 1920 x 1080, and 1280 x 720 at device scale 2.

### 13.2 Bridges for the oracle only

Through `view.bridge`, the oracle reads more fields:

- `turts_debug` gains `selection` (a list), `groups`, `results` (per unit), `notices`, `alerts`.
- It also gains `audio` (context state, started sounds by category), `contextmenu` (count, count
  prevented), `focus`, `cursor` (context and image id), and `player_id`.

`turts_scene` gains `widgets` (id and rect of each rendered mod widget). It already gives
`ground_screen_center` and `pick_radius` for each unit (`crates/turts_client_web/src/lib.rs:415-416`),
and the camera `clip_from_world` (QA0D:432). `turts_ui` stays. `turts_move` stays refused for the oracle
(QA0D:333). A CSS cursor is not in a screenshot (gate 1 bevy run: 0 red pixels), so HUD4 reads
`cursor`.

### 13.3 The five ledger files

| Ledger file | MC replacement | Pass condition |
| --- | --- | --- |
| `t7.js` | `t7-driver.js` + `t7-oracle.js` | QA0D:422-436 conditions 1 to 4. The driver finds the unit by its slot colour in `io.screenshot()`. |
| `m3-human.js` | `pt1-driver.js` + `pt1-oracle.js` | The oracle list of the Player path, steps 1 to 19, from screenshots only. The `player` agent uses this harness for PT1. |
| `m3.js` | `m3-driver.js` + `m3-oracle.js` | The M3 F10 acceptance through the forge HUD: research, enable and disable, remove, a drone move into range; conservation from the server files. |
| `m2.js` | removed. Its pure helpers move to `harness/png.js` (`pure`), its oracle helpers to `m2-oracle-lib.js` (`oracle`) | The M2 browser cases run as `m2-driver.js` + `m2-oracle.js` on the migrated M2 packages (12.2). |
| `m3-mobile.js` | `hud-driver.js` + `hud-oracle.js` | HUD1 at three viewports. Mobile stays later (D54). |

The ledger ends with 0 rows. `qa-lead` lowers `LEDGER_LIMIT` (`driver_rule.py:13`) from 5 to 0
in the same commit, so the ledger cannot grow back (DR7).

### 13.4 How a pixels-only driver finds things

- Units: by slot colour and ring shape in the screenshot. Rings and bars use unlit flat colours.
- HUD buttons: the driver holds the layout of the mod as literals, as a person learns a HUD. It
  checks the button colour in the screenshot before it clicks. It never reads the package files
  (A7, QA0D section 4.4).
- Text: no OCR in MC. The oracle checks text through `turts_ui`.

### 13.5 Headless drive (HD1)

The same action list runs in the headless client (`--actions`) and in a browser driver that turns
each action into pointer and key events. The oracle compares the accepted and refused unit sets of
the two runs.

### 13.6 T1X

- A scratch copy of `turts_interaction` gets its own `[workspace]` table, explicit package fields,
  absolute path dependencies, and `bevy_render`. A plain copy fails with "cargo tree failed".
  That failure has the exit code of a real red (gate 1 td run), so T1X needs its own workspace.
- T1X runs `boundary.sh --rule no-bevy-prefix` on the copy and asserts the exact stderr line
  `forbidden edge: turts_interaction -> bevy_app` (the first `bevy*` name in sorted order).
- T1X runs `--rule forbid --crate turts_interaction --target wasm32-unknown-unknown --names
  turts_mod_api` on the real crate and asserts the red line, as `boundary.rs:105-122` does for
  `turts_client_web`. This proves that the rule reads the real wasm32 tree.
- The full run of `boundary.sh` prints the four new `checked:` lines (2.4).

## 14. Acceptance

The producer confirms this list as the S:722 acceptance evidence. Each row names the build on
which the check goes red. "Pixels" means a pixels-only driver with an oracle (section 13).
"Headless" means a Rust test or the headless client. Hosted CI runs each row except the rows
marked "headed" and the player checks. "Assumes" names the call options that the row needs; a
row with other options needs an amendment.

| Test | Check | Red build | Runs | Assumes |
| --- | --- | --- | --- | --- |
| KM1 | Input map: an action fires on its physical code (code `KeyQ` with logical key "a" fires the `KeyQ` binding). A mod default binding applies. A player binding replaces it and survives a reload. `turts-pack` refuses a reserved key as a default (exit 2), and the menu refuses it as a player binding (notice). | The core matches the logical key; or the client does not write the store. | headless and pixels | K10 a |
| KM2 | Browser capture: a right click on the canvas gives `contextmenu` count = prevented count. The canvas has focus after join. A click outside shows the focus overlay, and the next key does not reach the game. P1 table for Chrome and Firefox with real OS keys. | `prevent_default_event_handling: false` (`lib.rs:254`); or no focus handling. | pixels; headed for P1 | K11 |
| KM3 | Modifiers and double click: Shift+click toggles, Shift+drag adds, Ctrl+click selects by type. Two clicks within 400 ms and 4 px count as a double click; two clicks 500 ms apart do not. | Double-click window 0; Shift ignored. | headless | - |
| SEL1 | A click selects one own unit. A click on a foreign unit shows it read-only, with an empty command card and its range circle. A click on the ground clears. Hover shows the ring, then the world tooltip after 0.5 s. | Pick radius 0; or a foreign click selects it for orders. | pixels | K16 a |
| SEL2 | A box selects the own units whose centres are inside the screen box, and drops static units when mobile units are in the box. Shift+drag adds; Shift+click removes. 500 units: the rings of all 500 show in the first presented frame after the mouse-up (frame log). | The box uses world rectangles; or the selection is a frame late. | pixels and headless | K15 a |
| SEL3 | Ctrl+click or a double click on a `trooper` selects each own trooper on the screen and no trooper off the screen. | Type select ignores the screen. | pixels | K15 a |
| SEL4 | Ctrl+1 assigns, 1 recalls, Shift+1 adds, 1 twice centres the camera on the group. A destroyed unit leaves the selection and the group in the same frame. After `io.reload()`, 1 recalls the same live units. | Groups not stored; or dead ids kept. | pixels | K10 a, K27 a |
| SLOT1 | Invariant 11 (S:577). The driver reloads within 5 s. The client shows the countdown and rejoins the same slot with the stored id within 15 s. A second window with another id gets "no open slot". A second live window with a copy of the stored id gets "This game is open in another window" after the countdown. The slot owner never changes (server stats). | Today's `judge_hello`: the reload gets "no open slot"; or no retry, so a reload within 10 s shows the refusal. | pixels and headless | K27 a |
| ORD1 | Protocol 4 round trip of each `OrderTarget` and of an envelope for each `Behaviour` (all nine). The server refuses a version 3 `Hello`. `muster:charge` (`Attack` with another name) works, and "charge" is not in engine code (no-content scan). | A behaviour without a wire form (for example no `Job` target); or the server matches order id strings in code. | headless | K2 a, K14 a |
| ORD2 | One envelope with 300 units: 290 own mobile, 5 own static, 5 foreign. Result: 290 accepted, 5 `not_applicable`, 5 `not_owner`; the 290 move. | Today's all-or-nothing `validate_move_to` (`command.rs:255-294`): 0 accepted. | headless and pixels | K4 b or c |
| ORD3 | Shift queues three waypoints, and the unit visits them in order. A plain order replaces the queue. S stops and clears. H holds: the unit stays and fires at a target in range. The server refuses an order beyond `queue_depth` with `queue_full` for that unit, and an `Append` behind `hold` or `gather` with `not_queueable`. A queued order after a completed move starts in the same tick. A gather on a node that empties writes `last_failure` `node_depleted`. Waypoint lines show. | Append treated as replace; stop keeps the queue; or no completion rule. | headless and pixels | K24 b |
| ORD4 | `muster`: attack on a target out of range moves into range and fires until the target is gone. Attack-move stops to fire at a target in range, then goes on. The explicit `move` (`fire: Never`) on a point under an enemy moves there with no shot. | Attack-move implemented as move; or `choose_target` ignores the ordered target. | headless and pixels | K2 a, K5 a |
| ORD5 | Right click on the ground: all mobile units move. On the ore node with workers and troopers selected: workers gather, troopers move. On a slot 1 target: troopers attack, workers move. | Smart command resolved once for the whole selection. | pixels and headless | K5 a |
| ORD6 | Each refusal and each failure shows a player text, never a code. The text shows within one frame of the result, with the error sound. The driver sends 100 envelopes at once. The server refuses those beyond the bucket with `rate_limited` for each unit. After 1 s of refill, the server accepts an envelope again. | Reason texts unmapped; or no rate limit. | pixels and headless | K25 a |
| ORD7 | The marker and the response sound start in the first presented frame after the click, before `OrderResult` arrives (frame log and input log). The server tick log shows the first move in the tick that drains the order. | Marker spawned on `OrderResult`; or the order waits one more tick. | pixels and headless | K7 a |
| GM40 | `muster`: box-select the 40 troopers and right-click one point. At rest (no position change above 0.001 for 2 s), these hold. There are 0 `Blocked` units. No two footprint discs overlap. The oracle takes each unit disc from `turts_scene`. The centre is `ground_screen_center`, taken back to the plane y = 0 with `clip_from_world`. The radius is `pick_radius` (world units). Two discs overlap when the distance of their ground centres is less than the sum of their radii minus 0.001. The ground plane maps one to one to the screen, so this is the overlap test of the projected discs (K17 a). The tick log gives the same test. Each unit stands within R_near = 4.58 of the target. | Spreading off: all 40 go to the point (model: 780 overlaps). | headless and pixels | K15 a, K17 a |
| GMA | Two groups of 20 troopers cross at a right angle. No pair of different groups overlaps by more than 20 % of the smaller radius for more than 0.5 s; no pair overlaps at rest. Cases F (a second group while the first moves) and I (a second group through a resting group): 0 overlaps at rest. MC limit, recorded and not gated. In F and I, units can stop `Blocked` with a reachable target (model: 7 and 9). Two units of different groups can overlap by up to 0.92 during motion. M4b (push, chokes) owns this limit. | Avoidance off (gate 1 sim probe: red); or a `Blocked` unit rests inside another unit (re-check cases F, I). | headless | - |
| GMU | The server accepts a move to a finite point outside the map, and the units end at the clamped point. The server refuses a NaN or infinite target with `target_not_finite`. A move onto the `post` ends on a ring round it. A second order of another group to the same point ends with 0 overlaps and 0 `Blocked` units. Each unit rests or is `Blocked` within 2 x (distance / speed) + 5 s. | Clamp off (`target_outside_map`); a clamp that keeps NaN; or slots reused for the second group. | headless | K6 a |
| DET1 | Invariant 15 (S:267). A new server test hook `--command-script <path>` applies a JSON list of (tick, slot, envelope) at fixed ticks, with no client. Two server processes run the GM40 script with `--tick-log`, and the two logs are byte-equal. The plan runs DET1 on the red build first and records the red, then on the fixed build. | Neighbour iteration in `HashMap` order (a new `RandomState` in each process). | headless | K14 a |
| EL1 | `muster` with the economy log on (GM40, GMA, a queued gather): the validator accepts each tick. It calls the same movement function as the sim. | A movement input missing from `AuditState`; a write to a position or to `UnitOrders` outside the movement function and `capture_operation`; or the validator of today (`economylog.rs:578-663`). | headless | K19 a |
| CAM1 | Arrow keys and the edge band pan. The wheel zooms between the mod limits. The look-at point stays inside the map plus margin. Space centres on the selection. The scroll speed setting changes the pan distance per second. The pointer leaves the canvas at the top edge: the pan stops. | The `PostUpdate` fit still rewrites the camera (`render.rs:455-456`); or no `PointerLeft` handling. | pixels | K9 a |
| CAM2 | Turn on "Keep the cursor in the game". The lock starts on a click. The mod cursor draws and follows the summed motion. The edge band pans at the window edge. Esc ends the lock, and F10 opens the menu. | No engine cursor under lock. | pixels (headless Chrome takes the lock) | K9 a |
| PROJ1 | Take 20 world points on three viewports: 1280 x 720, 1920 x 1080, and 1280 x 720 at scale 2. The core projection and `Camera::world_to_viewport` differ by at most 1 px. | The client does not copy the field of view into `Projection`. | headless: a Bevy app with an `Image` render target and no window (NOT TRACED on hosted CI; else a local-only test) | K1 a |
| HUD1 | Load the D67 probe tree of 8.5: card top left, minimap bottom right, text bottom centre. Each widget renders at its declared rect within 1 px on the three viewports. No engine panel shows. A grep finds no position literal in the widget runtime outside the placement code. | An engine-docked panel (as `economy_ui.rs:916-934`). | pixels and headless | - |
| HUD2 | forge HUD: the selection panel lists the selected units with health. The info panel shows the lab inventory, and damage, range and ammo for the own turret. The command card shows each order with its key label. "Research Hardened Shells" is disabled with "Needs 10 Shell (n/10)" while the lab holds fewer than 10. The production queue shows a bar and the seconds left, and a click on it cancels. Each button tooltip names the key. `muster`: the resource line shows the ore balance. | A source returns stale or empty items; a tooltip lacks the key. | pixels | - |
| HUD3 | `turts-pack` refuses each bad case below with package, file and field. Layout: an unknown widget kind, an unknown source, a duplicate widget id, a `card` collision. Images: a missing asset id, a PNG over 2048 px, a cursor image over 128 px, more than 256 MiB of decoded image bytes (IHDR sum) in a package or in the locked set. Fonts and sounds: a missing font file, fonts over 8 MiB, a sound over 30 s, sound files over 32 MiB in one package. Orders: a reserved key binding, `CancelProduction` without a `Job` target. It warns, with exit 0, for 10 px text. The version table refuses `^0.2` with `exports.interface`. | The validator accepts; the cap reads file bytes; or a version row is missing. | headless | K12 a, K13 a, K18 a |
| HUD4 | The driver points at five targets. They are an own unit, a foreign unit (attack), a node (gather), an invalid target (forbidden), and a point in target mode. For each, the oracle reads `turts_debug.cursor`: the context, the mod image id, and the cursor that the window applied (`CursorIcon`). | One cursor for all contexts. | pixels | - |
| MM1 | The minimap shows units in slot colours and the camera frame. A left click and a drag move the camera. A right click sends the smart command at that ground point. | The minimap click does not move the camera. | pixels | - |
| VIS1 | Rings in slot colours, solid for own and with gaps for foreign; range circles; health bars as set; markers that stay until rest. Each slot uses its own material. The default palette keeps a CIEDE2000 distance of at least 20 between each pair under protan, deutan and tritan simulation (Machado 2009 matrices). | One material for all units (`render.rs:403-406`); or a palette pair under 20. | pixels and headless | - |
| VIS2 | Chrome WebGPU, `muster` with 5,000 units: frame time p50 and p99, draw calls and memory are recorded in `docs/measurements/mc.md`. The row is red when a number is missing. | No record. | headed | - |
| ALR1 | Forced states give one alert line each, with sound and ping. The states: own unit damaged, research ready, research complete, extractor full, output blocked, out of ammo, inserter waiting for more than 5 s. A second damage within the cooldown gives no second alert. At most 2 alert sounds play at once. | No derivation; or no cooldown. | headless and pixels | K8 a |
| AUD1 | `turts_debug.audio` shows each context suspended before the first click and running after it. No audio player entity exists while a context is suspended. Select and order responses play in `voice`, one at a time. Each volume changes the gain of its category and survives a reload. An unknown sound id fails `turts-pack`. | No resume hook in the web entry. | pixels | K7 a |
| SH1 | The load screen shows each phase. A wrong lockfile hash shows the rejection text in player words. The menu rebinds, refuses a reserved key, changes scroll speed, UI scale (HUD1 rects scale) and cursor lock. The connection-lost screen shows on `reconnect_required`. F10 and Esc open the same menu. | The scroll speed is ignored; or `UiScale` also multiplies the device factor. | pixels | - |
| PER1 | The client writes bindings, settings and groups under the keys of 10.4. A corrupt value gives the defaults and one notice. At join, the client deletes the groups of an old `match_id`. | No write; or a corrupt value stops the client. | pixels and headless | K10 a |
| T1X | Section 13.6. The copy with its own workspace is red with the exact line. The real crate is red for `turts_mod_api` on wasm32. The full run prints the four `checked:` lines. | The core depends on Bevy; or a row is deleted. | headless | K1 a |
| HD1 | `turts_client --headless --actions` runs one action list with each behaviour, and group assign and recall. Its per-unit results equal those of the browser driver for the same list. | The headless build has no core (`lib.rs:278-280` today). | headless and pixels | K1 a |
| DRV1 | `tests/browser/legacy-bridge.tsv` has 0 rows and `LEDGER_LIMIT` is 0. `python3 scripts/qa/driver_rule.py --self-test` and the check of `tests/browser` exit 0. Each new driver and oracle passes twice in a row. | A driver that reads a bridge (DR2). | headless and CI | - |
| MOD1 | API 0.2.3: the migrated `acme`, `base`, `cedar`, `haul` and `forge` load with `^0.2.3`. A 0.2.2 copy of `acme` with no orders loads and shows "This game defines no orders". Each of the four version rows refuses `^0.2.2` when its list is present. A new order binds a behaviour by data. | A missing version row. | headless | K3 b, K13 a |
| REG1 | `cargo test --workspace`, fmt, strict clippy, the wasm32 check, `scripts/boundary.sh`, `scripts/acceptance.sh`, the M3 fixed point, and the frozen M2 and M3 operation streams pass with no re-freeze (K19 a). The stream test counts operations per tick (`crates/turts_pack/tests/m3_m2_stream.rs:31-36,119`). It compares three fields per operation (`:116-120`): the operation count, `event_kinds`, and `before == after`. All three stay. The plain move of the `acme` scout (`:263`) and the gather order (`:331`) add no operation (6.1). The new `AuditState` fields change only in the `Orders` and `Movement` phases, outside the gatherer operation (`crates/turts_sim/src/economy/gather.rs:226`), so `before == after` stays. A unit with no neighbour keeps today's float step (`crates/turts_sim/src/movement.rs:44-48`). T7 still puts the unit within 0.011 of the ground point. The plan inverts each all-or-nothing test: `crates/turts_pack/tests/economy_ordering.rs:120-175` (`NotMobile` at :153) and `:217-260` (:247); `crates/turts_sim/tests/command.rs:89,116,233,243,268,322`; and the group-move cases of `crates/turts_server/tests/economy.rs:2089-2236`. | Group movement moves a single unit off its point; or the ordered-target change alters the forge stream. | headless and CI | K4 b, K6 a, K19 a |
| PT1 | Blind playtest of the M3 industrial demo (S:708, S:722). The tester gets the build, the in-game guide, the pixels, two browser windows, and these goals only. Window A: "Find the name of each machine. Research the turret upgrade. Check that the turret is stronger. Pause one machine and start it again. Find out why an order is refused." Window B: "Find the enemy turret and its range. Make your drone control group 1, and centre the view on it with the group key. In the menu, make the view scroll faster and change the Stop key. Reload the page, then check that group 1 and the new Stop key still work. Send your drone to two points, one after the other, with one order, and stop it with the new key. Send your drone into the turret range and see the result." The test fails when the tester cannot find an object, an action or a reason. It also fails when 60 s pass with no sign of progress. The oracle checks the Player path list, and each step follows from a goal. The `player` agent runs it through the harness `io` only. A person runs it in a normal browser, and the person signs off (G12, S:703). | Any oracle step that the tester cannot reach. | person and `player` agent | K22 a, K27 a |
| PT2 | A person and the `player` agent open `muster` (the camera starts on the troopers), box-select the 40 troopers, and right-click one point. They judge from the screen, by the ground rings, that the units stop with no overlap, near the marker, with no jitter (S:712). The marker stays until the units rest. The person signs off. | Spreading or arrival off. | person and `player` agent | K15 a, K17 a |
| CI1 | Hosted CI is green on the pushed commit, with `python3 scripts/qa/completeness.py --self-test && python3 scripts/qa/completeness.py --enforce` and the MC row registered. | Any red job. | CI | - |

## 15. Budget, plan shape and cut line

### 15.1 Tasks and estimate

QA0 used 26 runs for 6 tasks. M3 used 75 runs for 20 tasks against an estimate of 47, so it ran
60 % over. This estimate uses about 4 runs for a medium task and a repair allowance of 60 %.

| # | Task | Seat | Needs | Runs |
| --- | --- | --- | --- | --- |
| 1 | P1 key, pointer lock and audio-resume probes | `web-platform-engineer` | - | 3 |
| 2 | Protocol 4: types, server mapping, unit order records, `match_id` (tasks 2 and 7 of revision 1, merged) | `network-engineer` | - | 6 |
| 3 | Mod API 0.2.3: types, validation, the K12 caps, version rows, content-level `present` | `mod-runtime-engineer` | - | 6 |
| 4 | `SimSet::Orders`, queue, behaviours, completion and failure, audited writes, validator (K19) | `simulation-core-engineer` | 2, 3 | 8 |
| 5 | Mobile weapons, attack, attack-move, grid target choice (K5) | `simulation-core-engineer` | 4 | 4 |
| 6 | Group movement: spreading, avoidance, arrival, fallback | `simulation-core-engineer` | 4 | 6 |
| 7 | Server session: rate-limit buckets (K25), rejoin in `judge_hello` (K27), `--command-script` hook | `backend-engineer` | 2 | 4 |
| 8 | `turts_interaction` skeleton, contract types, T1 rows, T1X | `technical-director` | 2, 3 | 2 |
| 9 | Core logic: input map, selection, groups, smart command, camera state, PROJ1 | `bevy-rendering-engineer`; `ux-designer` tests it headless | 1, 8 | 6 |
| 10 | Headless `--actions` hook | `bevy-rendering-engineer` | 9 | 2 |
| 11 | Camera controller and `LightDef` (K23) | `bevy-rendering-engineer` | 9 | 4 |
| 12 | Widget runtime: ten primitives, placement, sources, tooltip, cursor; removal of the engine panel | `bevy-rendering-engineer` with `ux-designer` | 3, 9 | 8 |
| 13 | World visuals: rings, range circles, bars, slot colours, markers, waypoints; VIS2 record | `bevy-rendering-engineer` | 11 | 5 |
| 14 | Minimap view | `bevy-rendering-engineer` | 12, 13 | 3 |
| 15 | Notices, alerts, audio and the resume hook | `bevy-rendering-engineer` | 12 | 5 |
| 16 | Shell screens, menu, rebinding, persistence, full-page canvas, client side of rejoin | `web-platform-engineer` with `ux-designer` | 7, 12, 15 | 6 |
| 17 | forge interface content and guide | `ux-designer`, checked by `mod-builder` | 3, 12 | 4 |
| 18 | Migration of `acme`, `base`, `cedar`, `haul` (K3 b) | `mod-builder` | 3 | 2 |
| 19 | `muster` and the HUD1 fixture | `mod-builder` | 3, 5 | 3 |
| 20 | Harness and `io` | `qa-lead` | - | 3 |
| 21 | Five driver and oracle rewrites, ledger to 0, fix of the red CI step "M2 browser proof" | `qa-lead` | 16, 17, 18, 20 | 8 |
| 22 | GM40, GMA, GMU, EL1, DET1 instruments; tick-cost records | `qa-lead` with `performance-architect` | 6, 7, 19 | 3 |
| 23 | PT1 and PT2 | `player` agent and a person | 21, 22 | 3 |
| 24 | CI wiring, measurements file, vault | orchestrator | 23 | 2 |
| | Subtotal (24 tasks) | | | 106 |
| | Gate 1 (6 lenses, one re-check) and gate 2 (5 lenses, one re-check) | | | 14 |
| | Repair allowance: 60 % of the subtotal (the M3 overrun) | | | 64 |
| | **Total** | | | **about 184 (range 165 to 205)** |

The gap register estimated 60 to 100 runs for MC (GR section 4). This design is above it. The
added work is audio (K7), the five driver rewrites, the interaction core crate, the package
migration (K3), rejoin (K27), and the audited movement (K19).

### 15.2 Critical path and shared files

- Longest path by the Needs column: tasks 2 (or 3), 8, 9, 12, 15, 16, 21, 23, 24. These nine take
  46 runs in series (6 + 2 + 6 + 8 + 5 + 6 + 8 + 3 + 2). The longest simulation path, 2, 4, 5, 19, 22,
  23, 24, takes 29 runs and runs beside it.
- Shared files and their single owner task:

| File | Owner task | Later tasks wait for it |
| --- | --- | --- |
| `crates/turts_net_protocol/src/messages.rs` | 2 | 4, 7, 9 |
| `crates/turts_mod_api/src/schema.rs` | 3 | 5, 12, 17, 18, 19 |
| `crates/turts_client/src/economy_ui.rs` (removed) | 12 | 13, 15, 16, 21 |
| `crates/turts_client/src/render.rs` | 11, then 13, then 14 | in this order |
| `crates/turts_server/src/session.rs` | 2, then 7 | in this order |
| `crates/turts_client/src/lib.rs` | 9, then 10, then 12, then 16 | in this order |
| `scripts/boundary.sh`, `crates/turts_checks/tests/boundary.rs` | 8 | - |
| `crates/turts_client_web/index.html`, `crates/turts_client_web/src/lib.rs` | 15, then 16 | in this order |

### 15.3 Cut line (K21)

Must: the S:722 acceptance (PT1, PT2, GM40), the three Decided lines, every S:722 item, and the
instrument. The double tap of a group key is in the must list (it is part of control groups).

May slip to MP1, in this order. Each slip changes rows of the coverage table, so the spec is
amended and gated again. A slip of an item that S:722 names needs the CEO (K21 a).

| Order | Slip | S:722 item? | Coverage rows that change | Player path steps it breaks |
| --- | --- | --- | --- | --- |
| 1 | Minimap view (MM1) | yes | none (UI05, CA08 are `later:M5`) | 11 changes: the tester finds the turret in the view; 18 loses the minimap ping |
| 2 | Pointer lock (CAM2) | no | AC05 to `later:MP1` | none |
| 3 | Audio | no ("alert channel" is) | AU01, AU02, AU06, MD09; by R4 also OR23, PE01, PE07, PF05, UI16 | 6, 8, 9, 18: the sounds; the text stays |
| 4 | Type select and double click | yes | PC05, IN04 | none |
| 5 | The rebinding screen and stored groups | no | PC19 | 14, 15, 16 |
| 6 | Mobile weapons, attack, attack-move and hold (K5) | yes | OR03 | none in PT1; ORD4 goes |

The gate 1 `technical-director` noted a cheaper split: slips 1, 3 and 6 save about 14 runs. Under
K21 (a), slips 1, 4 and 6 need the CEO.

## 16. Risks

1. The client rewrite is large: `economy_ui.rs` (1,425 lines), `input.rs` and `render.rs` all
   change, and the M2 and M3 browser tests use the panel. Guard: the migrated M2 packages (K3 b);
   the driver rewrites follow the panel removal (15.2); REG1.
2. Protocol number: the M4 draft planned protocol 4 (M4D K4). MC takes 4, so the M4 amendment
   (GR section 8) takes 5.
3. Group movement changes movement for every unit. T7 expects the unit within 0.011 of the
   ground point (QA0D:432-436). Guard: one unit alone takes the point itself (6.3); REG1.
4. The M3 fixed point: the ordered-target argument of `choose_target` must have no effect when no
   unit has an order. Guard: REG1 frozen streams.
5. Browser key facts are NOT TRACED for Firefox and real OS input (K11). Guard: P1 is the first
   task; the defaults freeze after it.
6. The group-movement rules pass in a Python model (section 18), not yet in the ECS. Guard: GM40,
   GMA and GMU in task 22; the separation numbers are prototype values that GM40 gates.
7. Determinism of avoidance (invariant 15). Guard: GM40 compares two server processes byte for
   byte.
8. A pixels-only driver can be flaky. Guard: unlit flat ring colours; each driver runs twice; a
   flaky driver stays out of CI and does not count as a pass (QA0D:441-443).
9. One person plays two slots in two windows; a hidden tab stops frames (wpe-6). Guard: PT1 uses
   two windows side by side.
10. The stored player id is a bearer value for the slot until identity lands (D50, M10). Guard:
    `sessionStorage` (one window), and the server refuses a second live session.
11. Budget: M3 ran 60 % over its estimate. Guard: the repair allowance and the cut line.

## 17. Gate 1 findings and open questions

### 17.1 Gate 1 findings and their fixes

| Round | Lenses | Outcome |
| --- | --- | --- |
| Gate 1 | rigour, technical-director, bevy-rendering, player, simulation | 13 high; repair 1 (revision 2) |
| Re-check 1 | the same five | 5 high, mostly new defects of repair 1; repair 2 (revision 3) |
| Re-check 2 | rigour, technical-director, player, simulation | 4 high; repair 3 |
| Re-check 3 | simulation, technical-director, player | 0 high; gate 1 passed; non-blocking items folded in (revision 4) |

Lenses: `spec-rigour-reviewer` (R), `technical-director` (T), `bevy-rendering-engineer` (B),
`player` (P), `simulation-core-engineer` (S). Commit 7a90f97 closed F-G1 of revision 1, so this
revision removes it.

| Finding | Fix in this revision |
| --- | --- |
| R1, P4 PT1 not blind | PT1 gives goals only; the Player path is the oracle list (14). |
| R2, S2 GM40 cannot pass | New rules in 6.3 to 6.5: spacing s = 2 r_max + 0.1, reach below s, landing on the slot. Also order-keeping assignment, a spare pool, same-group yield. GM40 asserts 0 `Blocked`. Model run in 18. |
| R3 done rows not proven | CB03, PF01, MP09, MD02 move to `later`; R4 moves 8 more rows; K20 asks for five edge changes. |
| R4 cut line drops S:722 items | K21; table 15.3. |
| R5 rows assume options | Column "Assumes" (14). |
| R6 stale F-G1, "D39 note", queue cap | F-G1 removed; the note citations removed; K24. |
| R7 two fallback rules, F10 and Esc, macOS | K3 recommends (b), with no fallback; one menu (10.1); macOS stays Open (1.3). |
| R8 "input", 0.x key, long sentences | "argument" (6.2); `<line>` rule (10.4); sentences shortened. |
| R open: invariant 11, ticks | SLOT1 (14); limits in seconds (5.4, 6.5). |
| T1 no job target | `Job(JobId)` in `OrderTarget` and `TargetKind`; ORD1 covers each behaviour. |
| T2 fallback outside 4.3 and the lockfile | K3 has three options with the evidence; recommendation (b). |
| T3 no wasm32 `nobevy` | Row added (2.4). |
| T4 T1X cannot go red | Own workspace, exact stderr, real-crate red, `checked:` lines (13.6). |
| T5 policy in behaviours | Mechanism and policy table (5.2); `fire`, `engage` parameters; content-level version row. |
| T6 calls missing | K24, K25, K4 merge key, K26. |
| T7 plan shape | Needs column, critical path, shared files, 60 % repair, merged task 2 (15). |
| T8 D18 status, `ContentRef`, `wgpu`, `winit` | Fixed (1.5, 5.1, 2.4). |
| B1 file bytes, not decoded bytes | K12 caps decoded bytes from IHDR; `RENDER_WORLD` except cursors (8.1, 11.3). |
| B2 HUD4 cannot go red | The oracle reads `turts_debug.cursor` (13.2). |
| B3 no leave event | `PointerLeft`; CAM1 case. |
| B4 no projection test | PROJ1; the field of view goes into `Projection` (2.2). |
| B5 no draw method | Draw method and VIS2 (9.1). |
| B6 no audio resume | Web-entry resume hook (9.5). |
| B7 fixed light | K23. |
| B8 `UiScale` | Player scale only (8.3). |
| P1 near drone dies | K22; the PT1 goals use one drone. |
| P2 reload rejection | K27; rejoin (10.5); the reload comes before the drone dies (Player path 15). |
| P3 no damage shown | Weapon fields in `SelectionPrimary` (8.4); HUD2. |
| P5 no world tooltip, no text | World tooltip (4.1, 9.1); `no_applicable_order` text (9.3). |
| P6 no range sign | Range circle (9.1). |
| P7 forge resource line | Removed from forge; HUD2 checks it on `muster`. |
| P8 PT2 by meshes | Ground rings, marker until rest, camera on the troopers (PT2, 7, 9.1). |
| S1 audit | `SimSet::Orders`, `capture_operation`, `AuditState`, EL1, K19 (6.1). |
| S3 second order | Reserved cells (6.3); GMU. |
| S4 O(N^2) | O(N log N) assignment; grid target choice; cost records (6.2, 6.3, 6.7). |
| S5 NaN | `target_not_finite` before the clamp (K6, 6.6). |
| S6 no completion | Table 5.3; ORD3. |
| S7 tests to invert | REG1 list. |
| S8 ticks, owner | Seconds; the server session owns the limit (5.4). |
| Re-check R1 goals and oracle | Each oracle step follows from a goal (PT1). |
| Re-check R2 rects | GM40 tests discs from `ground_screen_center` and `pick_radius`. |
| Re-check R3 EC02 | EC02, EC04, OR17 move to `later:M4` (17.2). |
| Re-check R4, R7, R8 | Slip 1 lists step 18; macOS stays Open; active voice. |
| Re-check path and GMA | Longest path 46 runs (15.2); GMA in seconds. |
| Re-check P N1 research ready | Trigger on the station, not the selection (9.4). |
| Re-check P N2, T N2 rejoin | `judge_hello`, countdown, SLOT1 within 5 s (10.5). |
| Re-check P N3 marker | Until rest, limit 60 s (9.1). |
| Re-check S N1, N4, T N1 | K19 rewritten, recommendation (a); REG1 needs no re-freeze. |
| Re-check S N2 | Leave every overlap before rest (6.5); F and I in GMA. |
| Re-check S N3, N6 | Spreading cap of 2,048 units per tick, `Pending`; grid for reserved cells (6.3). |
| Re-check S N5 | DET1 with `--command-script`; GM40 has no determinism half. |
| Re-check S 7 | REG1 adds `command.rs:322`. |
| Re-check B N1 to N6 | Cursor cap 128 px and applied cursor; locked-set and sound caps; PROJ1 target; no player while suspended; cursor as its own asset. |
| Re-check 2: plain orders, EL1 inputs, head envelope, guide | 6.1 plain-order rule and movement inputs; 6.3 pending lists; guide page 5 and groups row (12.1); goals (PT1). |
| Re-check T N3, N5 | CI step "M2 browser proof" in 1.6 and task 21; `lib.rs` in 15.2. |

### 17.2 K20 detail: rows that MC builds and tests but cannot claim

| Row | Blocking edge (current yardstick) | MC test |
| --- | --- | --- |
| UI05, CA08, CA09, UI08 | UI05 needs FV02 (fog, M5) | MM1 |
| CB03 | needs FV03 (sight radius, M5) | ORD4 |
| OR08 | needs CB03 | ORD3 |
| PF01 | needs MP01 (map format, M4) | GM40, ORD5 |
| OR01, OR04, OR05, PC22, PF04, GM10 | need PF01 (directly or by chain) | ORD4, ORD5, GMU |
| EC02, EC04, OR17 | EC02 needs PF01; EC04 and OR17 need EC02 | ORD3, ORD5 |
| MP09, CA13 | MP09 needs MP01 | GMU, CAM1 |
| PC16 | needs FV04 (server-side visibility, M5) | SEL1 |

If the CEO approves the five edges of K20, an amendment moves these rows to `in` with the tests
above, and the gate checks it.

### 17.3 Open questions for the orchestrator

1. The producer confirms section 14 as the S:722 "acceptance evidence".
2. Who is the person who signs off PT1 and PT2 (G12)?
3. P1 needs a headed Firefox on the CI image or on a GCP VM. Which one?
4. M4 takes protocol 5. Record it in the M4 amendment.
5. K20 changes `docs/qa/yardstick-a.tsv`, which `yardstick.py --check` compares with the
   inventory (QA0D section 3.1). The change needs the controlled path of QA0 K6.

## 18. What I ran

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| Read the client, sim, protocol, mod API, server session, export and boundary sources at HEAD | Baseline 1.6 | A cited line that differs | As cited |
| Local R4 closure over the 181 dispositions, `in` and `done:` rows alike (Python, scratch) | Every `in` or `done:` row has its CORE dependencies `in` or `done:` | A dependency `later` | 0 violations; 62 `in`, 15 `done:`, 104 `later:` |
| Group-movement model of 6.3 to 6.5, rules as written (Python, scratch; r 0.5, speed 3, 20 Hz) | GM40 can pass | An overlap at rest or a `Blocked` unit | A (block to P): 0 overlaps, 0 `Blocked`, farthest 3.81. B (ring to P): 0, 0, 3.81. C2 (second group to the same P): 0 overlaps of 80, 0 `Blocked`. D (new order while moving): 0, 0. Two runs equal: true |
| The same model with spreading off | GM40 can go red | 0 overlaps | 780 overlaps |
| The same model, two groups of 20 crossing | GMA can pass | A cross-group overlap above 20 % r for more than 10 ticks | 4 ticks above 20 % r, longest run 4; 0 overlaps at rest |
| Three earlier rule variants of the model | The final rules are needed | A simpler variant passes | Contact claims: A fails (8 `Blocked`). Yield by push: B fails (7 `Blocked`). Spare-only claims: C2 fails (29 `Blocked`) |
| Model revision 3: land only on a free slot, and leave every overlap before rest (Python, scratch) | The re-check cases F and I pass | An overlap at rest | F: 0 overlaps of 80 (7 `Blocked`). I: 0 overlaps of 80 (9 `Blocked`). A, B, C2, D, GMA unchanged: 0 overlaps, 0 `Blocked`. Largest step in one tick: 1.167 x speed x dt |
| Longest path over the Needs column of 15.1 (Python) | The critical path of 15.2 | Another sum | 46 runs: 2, 8, 9, 12, 15, 16, 21, 23, 24 |
| `sed` and `grep` of `slots.rs`, `session.rs`, `lib.rs:76` | A reload loses the slot | A reclaim path | None: K27 needs a server change |
| `grep` of `export.rs` | Health is public, weapons are owner-only | Another filter | `export.rs:87-94` |
| The three gate commands with this spec registered | The gate is green | A non-zero exit | Recorded in the hand-off report |

## Yardstick coverage

One row for each of the 181 CORE items of `docs/qa/yardstick-a.tsv`, in file order, under the
current yardstick. `in` rows name a test of section 14. `done:` rows name the earlier test that
shows the item. `later:` rows name a milestone after MC in `docs/qa/milestones.tsv`. Some rows
are built by MC but cannot be claimed until the CEO decides K20 (section 17.2). For these rows the
column "MC test" names the test of section 14. The fourth column is the item name, for people.

| id | disposition | test | item | MC test |
| --- | --- | --- | --- | --- |
| IN01 | in | SEL1 | Screen-to-world picking | - |
| IN02 | in | KM1 | Action mapping layer | - |
| IN03 | in | KM3 | Modifier semantics | - |
| IN04 | in | KM3 | Double-click detection | - |
| IN05 | in | HUD2 | Hotkey per command button | - |
| IN08 | in | KM2 | Browser input capture | - |
| IN09 | later:M9 | - | Text input mode | - |
| PC01 | in | SEL1 | Single select | - |
| PC02 | in | SEL2 | Box (drag) select | - |
| PC04 | in | SEL2 | Shift add and remove | - |
| PC05 | in | SEL3 | Type select on screen | - |
| PC08 | in | SEL4 | Control group assign and recall | - |
| PC09 | in | SEL4 | Control group add | - |
| PC10 | in | SEL4 | Group double-tap centres camera | - |
| PC16 | later:M5 | - | Inspect foreign entities | SEL1 (K20) |
| PC17 | in | ORD2 | Ownership gate | - |
| PC19 | in | SEL4 | Selection hygiene | - |
| PC20 | in | SEL1 | Hover highlight | - |
| PC21 | in | HUD4 | Context cursor | - |
| PC22 | later:M4 | - | Smart command | ORD5 (K20) |
| PC23 | in | VIS1 | Selection visual | - |
| OR01 | later:M4 | - | Move | ORD5 (K20) |
| OR02 | in | ORD3 | Stop | - |
| OR03 | in | ORD4 | Attack target | - |
| OR04 | later:M4 | - | Attack-move | ORD4 (K20) |
| OR05 | later:M4 | - | Force move | ORD4 (K20) |
| OR08 | later:M5 | - | Hold position | ORD3 (K20) |
| OR11 | in | ORD3 | Order queue per unit | - |
| OR12 | in | ORD3 | Shift-queued orders | - |
| OR15 | later:MP1 | - | Rally point | - |
| OR16 | later:MP1 | - | Build order | - |
| OR17 | later:M4 | - | Gather order | ORD5 (K20) |
| OR20 | later:MB | - | Ability order | - |
| OR22 | in | ORD6 | Order validation feedback | - |
| OR23 | in | ORD7 | Order acknowledgement | - |
| OR24 | in | ORD2 | Group order fan-out | - |
| OR25 | in | GM40 | Group destination spread | - |
| OR26 | later:MP1 | - | Order state machine | - |
| OR31 | in | ORD6 | Command rate limit | - |
| GM01 | in | GM40 | Destination spreading | - |
| GM04 | in | GMA | Local avoidance | - |
| GM06 | in | GMA | Unit collision size | - |
| GM07 | later:M4b | - | Push idle friends | - |
| GM09 | in | GM40 | Arrival | - |
| GM10 | later:M4 | - | Unreachable target | GMU (K20) |
| GM11 | later:M4b | - | Choke traffic | - |
| CA01 | in | CAM1 | Classic RTS camera | - |
| CA02 | in | CAM1 | Edge scroll | - |
| CA03 | in | CAM1 | Key scroll | - |
| CA08 | later:M5 | - | Minimap click | MM1 (K20) |
| CA09 | later:M5 | - | Minimap order | MM1 (K20) |
| CA13 | later:M4 | - | Bounds clamp | CAM1 (K20) |
| CA14 | in | CAM1 | Centre on selection | - |
| CA15 | later:M4 | - | Height-aware camera | - |
| CA16 | in | SH1 | Scroll speed setting | - |
| UI01 | in | HUD2 | Command card | - |
| UI02 | in | HUD2 | Selection panel | - |
| UI03 | in | HUD2 | Info panel and portrait | - |
| UI04 | in | HUD2 | Resource bar | - |
| UI05 | later:M5 | - | Minimap | MM1 (K20) |
| UI08 | later:M5 | - | Minimap alert flash | MM1 (K20) |
| UI09 | in | HUD2 | Build menu | - |
| UI10 | in | HUD2 | Production queue display | - |
| UI11 | in | HUD2 | Tooltips | - |
| UI12 | in | VIS1 | Health bars | - |
| UI14 | later:M5 | - | Fog rendering | - |
| UI16 | in | ALR1 | Event alerts | - |
| UI17 | later:M9 | - | Chat | - |
| UI19 | later:MP1 | - | Surrender and leave | - |
| UI22 | in | SH1 | In-game options menu | - |
| UI23 | in | HUD4 | Cursor set | - |
| CO01 | later:MP1 | - | Placement ghost | - |
| CO02 | later:MP1 | - | Footprint validity | - |
| CO03 | later:MP1 | - | Placement rules | - |
| CO04 | later:MP1 | - | Construction model | - |
| CO07 | later:MP1 | - | Under construction state | - |
| CO08 | later:MP1 | - | Cancel and refund | - |
| CO20 | later:MP1 | - | Destruction clears footprint | - |
| CO21 | later:MP1 | - | Placement vs units | - |
| PR01 | done:crates/turts_pack/tests/economy_production.rs | - | Production queue | - |
| PR02 | done:crates/turts_pack/tests/forge_cases.rs | - | Cancel and refund | - |
| PR03 | done:crates/turts_pack/tests/economy_production.rs | - | Payment policy | - |
| PR04 | later:MP1 | - | Parallel production | - |
| PR05 | later:MP1 | - | Spawn and exit | - |
| PR06 | later:MP1 | - | Rally applied | - |
| PR07 | done:crates/turts_pack/tests/forge_cases.rs | - | Prerequisites | - |
| PR08 | done:crates/turts_pack/tests/forge_cases.rs | - | Research and upgrades | - |
| PR10 | in | HUD2 | Unavailable reason | - |
| PR15 | done:crates/turts_pack/tests/forge_cases.rs | - | Modifier application | - |
| PR17 | later:MB | - | Asymmetric factions | - |
| EC01 | done:crates/turts_pack/tests/economy_gather.rs | - | Resource types | - |
| EC02 | later:M4 | - | Gather cycle | ORD5 (K20) |
| EC03 | later:MP1 | - | Drop-off choice | - |
| EC04 | later:M4 | - | Node depletion | ORD3 (K20) |
| EC05 | later:MP1 | - | Auto-resume and seek | - |
| EC10 | done:crates/turts_pack/tests/economy_production.rs | - | Insufficient funds | - |
| CB01 | done:crates/turts_pack/tests/forge_cases.rs | - | Health and death | - |
| CB02 | done:crates/turts_pack/tests/forge_cases.rs | - | Weapons | - |
| CB03 | later:M5 | - | Auto-target | ORD4 (K20) |
| CB04 | later:MP1 | - | Target priority | - |
| CB05 | later:MP1 | - | Retaliation | - |
| CB07 | later:MP1 | - | Damage and armour types | - |
| CB09 | later:MB | - | Splash damage | - |
| CB10 | later:MB | - | Projectiles | - |
| CB21 | done:crates/turts_pack/tests/forge_cases.rs | - | Defensive buildings | - |
| CB25 | later:MB | - | Bonus damage | - |
| FV01 | later:M5 | - | Black shroud | - |
| FV02 | later:M5 | - | Fog of war | - |
| FV03 | later:M5 | - | Sight radius | - |
| FV04 | later:M5 | - | Server-side visibility | - |
| FV05 | later:M5 | - | Shared vision | - |
| FV10 | later:M5 | - | Last-seen structures | - |
| UN01 | later:M4 | - | Ground units and movement classes | - |
| UN07 | later:MB | - | Abilities | - |
| UN11 | later:MP1 | - | Workers | - |
| UN13 | done:crates/turts_pack/tests/forge_cases.rs | - | Death and removal | - |
| PF01 | later:M4 | - | Sensible path | GM40, ORD5 (K20) |
| PF02 | later:M4 | - | Repath on change | - |
| PF03 | later:M4 | - | Movement layers | - |
| PF04 | later:M4 | - | No stuck units | GMU (K20) |
| PF05 | in | ORD7 | Instant start | - |
| PF10 | later:M7 | - | Mass pathing | - |
| MF01 | later:M8 | - | Main menu | - |
| MF02 | later:MP1 | - | Lobby | - |
| MF03 | later:M9 | - | Map choice | - |
| MF04 | later:M9 | - | Game settings | - |
| MF05 | later:MP2 | - | AI in slots | - |
| MF06 | later:MP1 | - | Start conditions | - |
| MF07 | later:MP1 | - | Load and countdown | - |
| MF08 | later:MP1 | - | Victory and defeat | - |
| MF10 | later:MP1 | - | Team victory | - |
| MF12 | later:M9 | - | Score screen | - |
| MF14 | later:M10 | - | Disconnect handling | - |
| SV01 | later:M10 | - | Single-player save and load | - |
| SV04 | later:M10 | - | Save version safety | - |
| SV05 | later:M10 | - | Mod save data | - |
| MP01 | later:M4 | - | Map format | - |
| MP02 | later:MB | - | Map editor | - |
| MP05 | later:M9 | - | Map metadata | - |
| MP07 | later:M8 | - | Map distribution | - |
| MP09 | later:M4 | - | Playable bounds | GMU (K20) |
| MD01 | in | MOD1 | Unit definition | - |
| MD02 | later:MB | - | Weapon and projectile definition | - |
| MD03 | later:MP1 | - | Building definition | - |
| MD04 | later:MB | - | Ability definition | - |
| MD05 | done:crates/turts_pack/tests/forge_cases.rs | - | Upgrade and tech definition | - |
| MD06 | later:MB | - | Faction definition | - |
| MD07 | later:MP1 | - | Damage and armour tables | - |
| MD08 | in | HUD3 | UI definitions | - |
| MD09 | in | AUD1 | Audio definitions | - |
| MD10 | later:MP1 | - | Victory definitions | - |
| MD11 | later:MP2 | - | AI definitions | - |
| MD12 | later:M9 | - | Lobby settings schema | - |
| MD17 | later:M13 | - | Asset pipeline | - |
| MD18 | done:crates/turts_pack/tests/bad_packages.rs | - | Validation messages | - |
| BT01 | later:MP2 | - | Skirmish AI | - |
| BT02 | later:M12 | - | Difficulty levels | - |
| BT03 | later:MP2 | - | Same command path | - |
| TM01 | later:MP1 | - | Teams | - |
| TM02 | later:MP1 | - | Alliance effects | - |
| TM03 | in | VIS1 | Player colours | - |
| AU01 | in | AUD1 | Unit responses | - |
| AU02 | in | ALR1 | Alerts | - |
| AU03 | later:MP1 | - | Positional effects | - |
| AU06 | in | AUD1 | Volume categories | - |
| AU07 | later:MP1 | - | Sound limiting | - |
| AN01 | later:M13 | - | Unit animations | - |
| AN02 | later:MP1 | - | Weapon effects | - |
| AN03 | later:MP1 | - | Construction visuals | - |
| AN05 | later:MP1 | - | Death visuals | - |
| AN06 | in | VIS1 | Decals for orders | - |
| AN07 | in | VIS1 | Team colour on models | - |
| AN08 | done:crates/turts_client/tests/interp.rs | - | Tick interpolation | - |
| AN11 | later:M13 | - | Instancing and level of detail | - |
| AC05 | in | CAM2 | Cursor confine | - |
| AC06 | later:M13 | - | Graphics settings | - |
| PE01 | in | ORD7 | Order responsiveness | - |
| PE02 | later:M7 | - | Stable frame rate | - |
| PE03 | later:M7 | - | Unit count | - |
| PE04 | in | SEL2 | Large selection speed | - |
| PE07 | in | ORD7 | Latency hiding | - |

## Player path

This is the oracle list of PT1. The tester does not see it: the tester gets the goals of PT1
(section 14). The demo is `mods/forge` with two slots, in two browser windows side by side, with
mouse and keyboard only. Under K22 (a) the drone player has one drone. The near drone is in turret
range at the start, and it is destroyed before window B joins.

1. Window A: the load screen shows each phase and a progress bar.
2. A click on "Click to play" opens the guide at page 1. The camera shows the own factory.
3. The tester reads the guide and closes it (F1 or its button).
4. The tester pans (arrow keys or the screen edge) and zooms (wheel). Over the drill, the hover
   ring shows, and after 0.5 s the world tooltip "Bore Drill".
5. A click on the Proving Lab: the info panel shows its name and the shell count. While the lab
   holds fewer than 10 shells, the research entry is disabled, and its tooltip says
   "Needs 10 Shell (n/10)" and "R".
6. The tester selects a belt piece and disables it (D or the button). The belt stops; within 10 s
   an alert line says that an arm loader waits, with a sound. Enable (E) starts the items again.
7. The alert line "Research ready" is in the alert list. It fires when the lab can start the
   research, or at join if it already can, whatever the selection is. The tester starts the
   research (R or the button). The production queue shows a bar and the seconds left.
8. The alert "Research complete: Hardened Shells" shows and sounds. A click on the turret: the
   info panel shows damage 20, range 8 and the ammo count.
9. With the turret selected, a right click on the ground gives the notice "Shell Turret: Cannot
   move" and the error sound.
10. Window B: the load screen, then "Click to play". The camera shows the own drone in the colour
    of slot 1.
11. The tester finds the turret on the minimap or in the view, and clicks it. Its ring has gaps
    (a foreign unit), the command card is empty, and its range circle shows.
12. A box drag round the drone selects it; the selection panel shows it with its health bar.
13. Ctrl+1, a click on empty ground, then 1: the drone is selected again. 1 twice: the camera
    centres on it.
14. F10 opens the menu. The tester changes the scroll speed and binds Stop to another key, then
    closes the menu.
15. The tester reloads window B. The client rejoins the same slot (K27). Group 1 and the new Stop
    key still work.
16. Shift and two right clicks away from the turret: waypoint lines show, and the drone goes to
    the points in that order. The new Stop key stops it.
17. The tester right-clicks inside the range circle of the turret. A marker shows at once. The
    drone moves and stops on the point. The marker stays until the drone rests.
18. The turret fires. The health bar of the drone drops, and the alert "Unit under attack" sounds
    and pings the minimap.
19. The drone is destroyed. It leaves the selection and group 1, and the alert "Unit destroyed"
    shows.
