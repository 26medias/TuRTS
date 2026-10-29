VERDICT=fail ITEMS=128 MISSING=94 PARTIAL=31 CONFLICT=0 RUNS=9 NOT_TRACED=4

# Gap audit: presentation, input, camera, HUD, audio, VFX, accessibility (seat: bevy-rendering-engineer)

Sources: spec v0.7 (S:line). Yardstick A = `inventory-engine.md` (ids IN, PC, OR, CA, UI, CO, AU, AN, AC, PE).
Yardstick B = `inventory-player.md` (numbers B-n, mapped in the note column). Term sweep output:
`/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/review/gap-bevy-rendering-engineer-greps.txt`.

Milestone codes used in "milestone proposed" (proposals, not decisions):

- **P1** = a NEW player-control milestone, placed before M4. Reason: M4 K1 already adds drag-select and a
  "player-control gate" (`docs/superpowers/specs/2026-10-28-m4-terrain-design.md:27`), and every later demo is
  judged by a human. P1 holds input, selection, camera, the HUD primitives, and the mod UI contract.
- **P2** = a NEW milestone for the first playable default mod (S:724-751). Today it is only "additional proposed
  validation" (S:694), not a milestone row. P2 holds audio, VFX, alerts, minimap, health bars, placement ghost.
- **M5** = first visibility milestone (S:680). **M13** = assets and crowds (S:688). "later" = with the feature
  that needs it.

"M1-M3 change?" uses these code facts (checked; cite in the row):

- C1 `crates/turts_client/src/economy_ui.rs:31` `Selection(pub Option<u32>)`: one unit, owned only.
- C2 `crates/turts_client/src/input.rs:44-46,87-90`: left click = select one; right click = one command. No keyboard.
  No `KeyCode` in any client source (run R4).
- C3 `crates/turts_client/src/render.rs:22-29,338-342` hardcoded camera; `render.rs:245-284,455-459`
  `fit_scene_camera` rewrites the camera `Transform` each frame in `PostUpdate` (`render.rs:281`). A pan or zoom
  system would be overwritten. Replace, do not extend.
- C4 `render.rs:343-346` hardcoded `DirectionalLight`; `render.rs:347-351` flat ground plane.
- C5 `render.rs:403-406` one material for all units (`srgb(0.8,0.3,0.2)`): no team colour.
- C6 `render.rs:33` and `input.rs:15`: scene y = 0 and plane picking (M4 draft replaces the picking, M4D:262).
- C7 `economy_ui.rs:83-163,905-1029`: the HUD is one engine-owned panel with English text built in Rust
  (example `economy_ui.rs:124` "Factory: Select a lab to start research..."), a content heuristic
  (`economy_ui.rs:90-94` "industrial"), and a fixed layout (`economy_ui.rs:916-934`). `lib.rs:279` adds
  `EconomyUiPlugin` for every windowed client; no mod can replace it.
- C8 `economy_ui.rs:817-865` re-implements server capability checks on the client.
- C9 bevy_audio is not in the client build on host or wasm32 (run R5); client features are
  `["3d","ui","default_font"]` (`crates/turts_client/Cargo.toml:13`).
- C10 mod visuals are one primitive shape: `Shape::{Capsule,Cuboid}` (`crates/turts_mod_api/src/schema.rs:48-51`);
  example `mods/*/visuals/*` = `(target: Unit("acme:scout"), shape: Cuboid(...))`. No camera, colour, icon,
  hotkey, cursor, sound, or layout field in `turts_mod_api` or `mods/` (run R6).
- C11 the wire carries fixed engine records (`crates/turts_net_protocol/src/messages.rs:267-293`: weapons,
  healths, research ...). No event or alert channel (run R7).
- C12 `crates/turts_client/src/live_view.rs:365` and `debug.rs:66-67` read `Selection.0` as one id: the test
  instruments change when selection becomes a set.

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A-IN01 | Screen-to-world picking | partial | S:610, S:688 "picking" as a crowd-render check only | M | M2 (ground plane hit test) | P1 | CA01, height data (M4) | yes: C6; `economy_ui.rs:721-750` pick by declared radius, owned only | Pick by gameplay footprint, not by mesh, is correct (S:132). Keep that rule; add height and entity kinds. |
| A-IN02 | Action mapping layer | missing | "input" only S:122 (table cell); "hotkey/shortcut/key bind" 0 player hits (S:97 is "privileged shortcut") | M | none | P1 | none | yes: C2 (no keyboard path exists) | Engine owns named actions; mods declare actions and default bindings. B-12. |
| A-IN03 | Modifier semantics | missing | "shift/ctrl/alt/modifier": hits S:161, S:426, S:428 are tech modifiers only | B | none | P1 | IN02 | yes: C2 | Engine gives a default modifier meaning; mod may rebind. |
| A-IN04 | Double-click detection | missing | "double-click" 0 hits | M | none | P1 | IN02 | no | Needed by PC05, PC10. |
| A-IN05 | Hotkey per command button | missing | "hotkey" 0 hits | B | none | P1 | IN02, UI01 | yes: C7 (buttons have no key) | Mod sets default key per command definition. B-12. |
| A-IN06 | Grid hotkey layout | missing | "grid" hits are terrain only | B | none | later | IN05 | no | OPTIONAL. |
| A-IN07 | Command scheme option | missing | "left-click/right-click/click" 0 hits | M | none | later | IN02 | yes: `input.rs:1` fixes right = order | OPTIONAL; keep the binding layer able to swap buttons. |
| A-IN08 | Browser input capture | missing | "context menu", "pointer lock", "browser hotkey" 0 hits; S:99 names browser primary | M | M1 (side effect) | P1 | IN02 | partial: `lib.rs:254` sets `prevent_default_event_handling: true`; Bevy says this blocks "F5, F12, Ctrl+R, tab, etc." (bevy_window-0.20.0 `window.rs:262`) | Ctrl+W/T/N and Ctrl+1..9 capture: NOT TRACED. MDN: Keyboard Lock is experimental, not Baseline (R8). Control groups need a binding plan that works without it. |
| A-IN09 | Text input mode | missing | "chat", "text input" 0 hits | M | none | P1 | IN02, UI17 | no | `bevy_input_focus` is in the build (R5 tree); focus routing is not designed. |
| A-IN10 | Touch and gamepad | partial | D54 S:77, S:99 defer mobile | M | none | later | IN02 | no | Decided deferral. Keep the action layer device-neutral so touch fits later. |
| A-PC01 | Single select | partial | "select" 37 hits, all library/slot/server senses (sweep); no unit-select rule | M | M2 | P1 | IN01 | yes: C1, C2 | Exists in code without a spec line. B-1. |
| A-PC02 | Box select | missing | "drag", "box" 0 player hits | M | M4 draft K1 | P1 | IN01, CA01 | yes: C1 (M4D:265 changes it to a set) | M4 extends the engine panel; see lens answer. B-2. |
| A-PC03 | Box filter rules | missing | 0 hits | B | none | P1 | PC02 | no | Mod gives selection priority per unit. |
| A-PC04 | Shift add/remove | missing | "shift" 0 hits | M | none | P1 | PC01, PC02, IN03 | yes: C1 | B-3. |
| A-PC05 | Type select on screen | missing | "double-click" 0 hits | M | none | P1 | PC01, IN04, CA01 | yes: C1 | B-4. |
| A-PC06 | Type select on map | missing | 0 hits | M | none | P1 | PC05 | no | |
| A-PC07 | Selection cap | missing | 0 hits | B | none | P1 | PC02 | no | Mod value; engine default "none" fits D39 (S:62). |
| A-PC08 | Control group assign/recall | missing | "control group" 0 hits | M | none | P1 | IN02, IN03, IN08 | no | M4 puts it out of scope (M4D:23). B-11. |
| A-PC09 | Control group add | missing | 0 hits | M | none | P1 | PC08 | no | |
| A-PC10 | Double-tap centres camera | missing | 0 hits | M | none | P1 | PC08, CA14 | no | |
| A-PC11 | Steal from group | missing | 0 hits | M | none | later | PC08 | no | OPTIONAL. |
| A-PC12 | Subgroups and Tab | missing | 0 hits | M | none | P1 | PC02, UI01, UI02 | no | B-44. |
| A-PC13 | Building hotkeys | missing | 0 hits | B | none | P2 | PC01 | no | |
| A-PC14 | Idle worker select | missing | "idle" hits are factory sleep (S:616-618) | B | none | P2 | UN11, UI15 | no | Needs a replicated "idle" fact. B-34. |
| A-PC15 | Select all army | missing | 0 hits | B | none | P2 | PC06 | no | Mod defines "army" tag. B-73. |
| A-PC16 | Inspect foreign entities | missing | 0 hits | M | none | P1 | PC01, FV04 | yes: `economy_ui.rs:730` and `:1279-1284` reject or clear non-owned selection | Must read only permitted state (S:634, S:496). |
| A-PC19 | Selection hygiene | missing | 0 hits | M | M2 (one id) | P1 | PC08 | yes: `economy_ui.rs:1272-1286` clears one id | Groups must survive resync and reconnect. |
| A-PC20 | Hover highlight | missing | "highlight/hover" 0 hits | M | none | P1 | IN01 | no | |
| A-PC21 | Context cursor | missing | "cursor" 0 hits | B | none | P1 | IN01, OR22, UI23 | no | B-35. |
| A-PC22 | Smart command | missing | "right-click" 0 hits | B | M2 (move or gather only) | P1 | OR01, OR03, EC02 | yes: `input.rs:109-141` hardcodes gather-else-move | Mod must define the target-to-command rule. B-5. |
| A-PC23 | Selection visual | missing | "circle", "ring", "outline" 0 player hits | M | none (text label only, `render.rs:563`) | P1 | AN06 | yes: no selection decal exists | B-13. |
| A-OR13 | Waypoint display | missing | "waypoint" 0 hits | M | none | P2 | OR11, AN06 | no | |
| A-OR23 | Order acknowledgement | partial | S:243 "immediate input acknowledgement" (Decided paragraph; Baseline item) | B | none | P1 | AU01, AN06, PE01 | no | Phrase only; no marker, no sound. |
| A-CA01 | Classic RTS camera | partial | D37 S:60, S:101 (Decided); camera API Open S:701 | M | M1 (hardcoded) | P1 | none | yes: C3 replace | Hardcoded camera contradicts D37 (mods control camera). |
| A-CA02 | Edge scroll | missing | "scroll", "edge" 0 camera hits | M | none | P1 | CA01, IN08, AC05 | yes: C3 | B-25. |
| A-CA03 | Key scroll | missing | 0 hits | M | none | P1 | CA01, IN02 | yes: C3 | |
| A-CA04 | Drag pan | missing | "pan" 0 hits | M | none | P1 | CA01 | yes: C3 | |
| A-CA05 | Zoom | missing | "zoom" 0 hits | B | none | P1 | CA01 | yes: C3 | Mod sets limits (S:101 "motion constraints"). |
| A-CA06 | Strategic zoom | partial | S:604 "optional mod-appropriate strategic icons" (render LOD only) | M | none | later | CA05, UI05 | no | B-45. CORE for a SupCom-like mod. |
| A-CA07 | Rotation | missing | "rotat" hits S:221, S:384 are key rotation | M | none | later | CA01 | no | OPTIONAL; mod permits or forbids. |
| A-CA08 | Minimap click | missing | "minimap" only S:338 (replication) | M | none | P2 | UI05 | no | B-24. |
| A-CA09 | Minimap order | missing | same | M | none | P2 | UI05, OR24 | no | |
| A-CA10 | Camera bookmarks | missing | "bookmark" 0 hits | M | none | later | CA01 | no | B-72. |
| A-CA11 | Jump to last alert | missing | 0 hits | M | none | P2 | UI16 | no | B-29. |
| A-CA12 | Follow unit | missing | "follow" hits are other senses | M | none | later | CA01 | no | |
| A-CA13 | Bounds clamp | partial | S:101 "motion constraints" | B | none | P1 | MP09 | yes: C3 | |
| A-CA14 | Centre on selection | missing | 0 hits | M | none | P1 | CA01 | yes: C3 | |
| A-CA15 | Height-aware camera | missing | "height" hits are terrain data (S:125, S:436) | M | M4 draft (fit includes highest cell, M4D:263) | P1 | CA01, M4 heights | yes: C3, C6 | |
| A-CA16 | Scroll speed setting | missing | 0 hits | M | none | P1 | UI22 | no | |
| A-CA17 | Second view | missing | 0 hits | M | none | later | CA01 | no | OPTIONAL. |
| A-UI01 | Command card | partial | S:164 "command panels" (Decided principle; mod policy) | B | M2 (button list) | P1 | PC12, IN05 | yes: C7 replace | B-15. |
| A-UI02 | Selection panel | missing | "portrait", "selection panel" 0 hits | M | none | P1 | PC02 | yes: C1, C7 | B-14. |
| A-UI03 | Info panel and portrait | missing | 0 hits | B | M3 (text lines, `economy_ui.rs:455-463`) | P1 | PC01 | yes: C7 | Needs icon or portrait assets (own-8). |
| A-UI04 | Resource bar | partial | S:164 "Resource displays" | B | M2 (panel lines, `economy_ui.rs:547`) | P1 | EC01 | yes: C7 | B-21. |
| A-UI05 | Minimap | partial | S:338 "minimap data may still be required" | M | none | P2 | FV02 | no | Engine widget; mod colours and icons. B-24. |
| A-UI07 | Minimap ping | missing | "ping" hits are "mapping" (noise) | M | none | later | UI05, TM02 | no | B-69. |
| A-UI08 | Minimap alert flash | missing | 0 hits | M | none | P2 | UI05, UI16 | no | |
| A-UI09 | Build menu | missing | "menu", "button" 0 hits | B | none | P2 | PR07 | no | 16.2 needs it (S:744-745). |
| A-UI10 | Production queue display | partial | S:142 queue (sim); display 0 hits | B | M2 (text, cancel buttons) | P1 | PR01 | yes: C7 | B-19. |
| A-UI11 | Tooltips | partial | S:164 "tooltips" | B | none | P1 | MD01 | no | B-16. |
| A-UI12 | Health bars | missing | "health" 0 hits in spec | M | M3 (panel text only) | P2 | CB01 | yes: health is on the wire (`messages.rs:256-259`) but not drawn in the world | B-28. Instanced bars needed at 5,000 units. |
| A-UI13 | Status icons | missing | "icon" only S:604 | B | none | later | UN09 | no | |
| A-UI14 | Fog rendering | partial | S:162 mechanism; S:634 filter; "explored/shroud" 0 hits | M | none | M5 | FV01, FV02 | no | Draw only permitted state (S:496). B-26. |
| A-UI15 | Idle worker button | missing | 0 hits | B | none | P2 | UN11 | no | |
| A-UI16 | Event alerts | partial | S:338, S:634 "alerts" (replication only) | B | none | P2 | AU02, own-3 | yes: C11 no event channel | B-29, B-62. |
| A-UI17 | Chat | missing | "chat" 0 hits | M | none | P2 | IN09 | no | |
| A-UI18 | Pause | missing | "pause" 0 player hits | B | none | later | MF14 | no | |
| A-UI19 | Surrender and leave | missing | "surrender" 0 hits | B | none | P2 | MF08 | no | B-38. |
| A-UI20 | Game speed | missing | "speed" hits are research and units | B | none | later | MF04 | no | |
| A-UI21 | Game clock | missing | "clock" 0 hits | M | none | P2 | none | no | |
| A-UI22 | In-game options menu | missing | "menu" 0 hits; "option/setting" hits are room settings (S:290) | M | none | P1 | none | no | |
| A-UI23 | Cursor set | missing | "cursor" 0 hits | B | none | P1 | PC21 | yes: C10 | |
| A-UI24 | Rally display | missing | "rally" only "literally" S:366 | M | none | P2 | OR15 | no | B-20. |
| A-UI25 | Range display | missing | "range" hits are sight/value ranges | B | none | P2 | CO01 | no | |
| A-UI26 | Building power indicator | missing | 0 hits | B | none | P2 | EC09 | no | 16.2 power rule (S:746) needs it. B-23. |
| A-UI27 | Mod UI layout | partial | S:164; S:179 "UI data" | B | none | P1 | MD08, own-1 | yes: C7 | Core of the lens question. |
| A-UI28 | Floating text | missing | 0 hits | B | none | later | none | no | OPTIONAL. |
| A-UI29 | Army/production overview | missing | 0 hits | M | none | later | PR01 | no | OPTIONAL. |
| A-UI30 | Localisation | missing | "locali/translat/string table" 0 player hits | B | none | P1 | MD13 | yes: C7 English strings in Rust; only `default_font` (`Cargo.toml:13`) | Decide before mod text spreads. |
| A-UI31 | Objectives panel | missing | 0 hits | B | none | later | MF08 | no | |
| A-CO01 | Placement ghost | partial | S:158 placement (mechanism); S:436 "must not conceal ... placement rules"; "ghost/preview" 0 hits | B | none | P2 | CO03 | no | B-18. Client validity must come from a shared rule, not a mesh test (S:132). |
| A-CO02 | Footprint validity display | partial | S:436 | B | none | P2 | CO01 | no | |
| A-AU01 | Unit responses | missing | "voice/sound" 0 hits | B | none | P2 | PC01, OR23 | yes: C9 no audio crate | B-30. |
| A-AU02 | Alerts audio | missing | "audio" only S:122 | B | none | P2 | UI16 | yes: C9 | |
| A-AU03 | Positional effects | missing | 0 hits | M | none | P2 | own-3 | yes: C9 | Browser autoplay policy: NOT TRACED. |
| A-AU04 | Music | missing | "music" 0 hits | B | none | later | none | yes: C9 | |
| A-AU05 | Ambient sound | missing | "ambient" 0 hits | B | none | later | MP01 | no | OPTIONAL. |
| A-AU06 | Volume categories | missing | 0 hits | M | none | P2 | UI22 | yes: C9 | |
| A-AU07 | Sound limiting | missing | 0 hits | M | none | P2 | AU03 | yes: C9 | Budget like S:612 effects. |
| A-AU08 | Repeat-click lines | missing | 0 hits | P | none | later | AU01 | no | OPTIONAL. |
| A-AN01 | Unit animations | partial | S:594-608 render paths (Baseline); D62 S:85; no clip/state binding to sim state | B | M13 | P2 | MD17, own-3 | yes: C10 (no clip field) | Path is designed; the state-to-clip contract is not. |
| A-AN02 | Weapon effects | partial | S:612 effect budgets; S:171 "recoil, and effects" | B | none | P2 | CB10, own-3 | yes: C11 | No event to trigger an effect. B-31. |
| A-AN03 | Construction visuals | missing | "progress" S:158 sim only | B | none | P2 | CO07 | no | |
| A-AN04 | Damage states | missing | "smoke/fire" 0 hits | B | none | later | CB01 | no | |
| A-AN05 | Death visuals | missing | "death" only S:774 (bot recovery) | B | none | P2 | UN13, own-3 | yes: `render.rs:425-429` despawns at once | B-31. |
| A-AN06 | Order and selection decals | missing | "decal/marker" 0 player hits | M | none | P1 | none | no | |
| A-AN07 | Team colour on models | missing | "colour" only S:531 (utility overlay) | M | none | P1 | TM03 | yes: C5 | Two players look identical today. |
| A-AN08 | Tick interpolation | present | S:243 (Decided paragraph, Baseline item); S:634 | M | M1 | M1 | none | no | `present.rs`, `interp.rs` exist. |
| A-AN09 | Terrain decals | missing | 0 hits | B | none | later | MP01 | no | OPTIONAL. |
| A-AN10 | Moddable particles | partial | S:612 Hanabi candidate; WebGPU vs WebGL Open (S:612, S:702) | B | none | P2 | MD17, own-12 | yes: C10 | Blocked by an Open decision. |
| A-AN11 | Instancing and LOD | present | S:596-610 (Baseline) | M | M13 | M13 (probe now, S:690) | PE02 | no | |
| A-AN12 | Day and night | missing | "night" 0 hits; lighting is a mod setting (D37) | B | none | later | FV03 | yes: C4 | OPTIONAL. |
| A-AN13 | Weather | missing | 0 hits | B | none | later | none | no | OPTIONAL. |
| A-AC01 | Rebindable keys | missing | "rebind" 0 hits | M | none | P1 | IN02, own-10 | no | B-77. |
| A-AC02 | Colour-blind support | missing | "colou?r.?blind" 0 hits | B | none | P2 | TM03, AN07 | no | |
| A-AC03 | UI scale | missing | "scale" hits are workload scale | M | none | P1 | UI22 | no | Bevy `UiScale` exists: NOT TRACED for 0.20. |
| A-AC04 | Subtitles and text size | missing | "subtitle" 0 hits | M | none | later | AU01 | no | OPTIONAL. |
| A-AC05 | Cursor confine | missing | "pointer lock" 0 hits | M | none | P1 | CA02, IN08 | no | Browser pointer lock hides the OS cursor; edge scroll needs a design. |
| A-AC06 | Graphics settings | partial | S:229 "cosmetic quality variants"; S:173 detail levels; no player setting | M | none | P2 | own-12 | no | |
| A-AC07 | Reduced motion | missing | "shake/flash" 0 hits | M | none | later | none | no | OPTIONAL. |
| A-PE01 | Order responsiveness | partial | S:243 phrase; latency thresholds Open S:560 | M | none | P1 | OR23, AN08 | no | |
| A-PE02 | Stable frame rate | partial | S:650, S:662 thresholds Open | M | M1 (measured, not judged) | P2 | AN11 | no | HUD cost is not in any S:650 scenario. |
| A-PE04 | Large selection speed | missing | 0 hits | M | none | P1 | PC02 | no | Test box-select of 1,000 units (D57). |
| A-PE07 | Latency hiding | partial | S:243, S:634 | M | M1 (interpolation) | P1 | OR23 | no | Local acknowledgement only; no sim prediction (D18). |
| bevy-rendering-engineer-1 | Mod UI runtime: how a mod defines a HUD (format, client-side script or declarative tree, sandbox, budget) | missing | S:164, S:179 "UI data", S:209 execution context "client"; no UI format | B | none | P1 | Open script runtime (S:193) | yes: C7 | Needs decision. Without it every HUD item stays engine code. |
| bevy-rendering-engineer-2 | Controlled state access: which replicated state a mod UI may read; mod-defined replicated attributes (mana, ammo, custom bars) | partial | S:164 "controlled state access" (two words) | M | none | P1 | network schema (S:704 Open) | yes: C11 fixed records | A mod stat cannot reach the HUD without an engine and protocol change. |
| bevy-rendering-engineer-3 | Presentation event stream (shot, hit, death, complete, alert) from server to client | partial | S:364 "presentation events" for spectators only; S:634 alerts | M | none | P2 | network | yes: C11 | Feeds VFX, audio, alerts, death visuals. |
| bevy-rendering-engineer-4 | Selection and control groups are client-local presentation state, not server state; persistence across resync and reconnect | missing | 0 hits | M | none | P1 | PC08 | yes: C1, C12 | State it, so nobody replicates selection. |
| bevy-rendering-engineer-5 | Input routing: UI before world, focus, capture order | missing | 0 hits | M | M2 (ad hoc, `input.rs:52-71`) | P1 | IN02 | yes: ad hoc hit test against one panel root | |
| bevy-rendering-engineer-6 | Command availability rule shared by client and server (grey-out reason) | missing | S:782 "rejected-command reasons" bots only (Proposed) | B | M2 | P1 | OR22 | yes: C8 duplicated checks | Two copies of one rule will drift. |
| bevy-rendering-engineer-7 | Lighting and environment as mod settings | partial | D37 S:60, S:101; API Open S:701 | B | M1 (hardcoded) | P1 | CA01 API | yes: C4 | |
| bevy-rendering-engineer-8 | Visual definition schema: model, material, team-colour mask, clip-to-state map, icon, portrait, selection radius, bar offset | wrong-milestone | S:225-229 asset pipeline (Baseline) sits in M13 (S:688), after every HUD milestone | B | M13 | P1 (minimum fields), M13 (compiler) | MD17 | yes: C10 | Icons and team colour are needed by P1 and P2. |
| bevy-rendering-engineer-9 | World-space overlays at scale (labels, bars, rings) | missing | 0 hits | M | M3 (one UI text node per unit, `render.rs:601-619`) | P2 | AN11 | yes: per-unit UI nodes do not scale to 5,000 (D57); NOT TRACED by a probe | Measure with a probe before P2. |
| bevy-rendering-engineer-10 | Player-local client settings storage (keys, volume, graphics) in the browser | missing | S:253 "Browser package caches are not local game saves"; no settings line | M | none | P1 | UI22 | no | Not a game save, so D41 does not block it; one sentence needed. |
| bevy-rendering-engineer-11 | Cosmetic-only package kind for HUD skins, hotkey presets, sound packs | partial | S:203; Open S:708 | B | none | later | own-1 | no | |
| bevy-rendering-engineer-12 | WebGPU vs WebGL decision blocks particles and quality tiers | partial | Open S:612, S:702 | M | none | P2 (decide before) | none | no | Needs decision. |
| bevy-rendering-engineer-13 | Generic UI introspection hook for agents (D43) | partial | D43 S:66; `live_view.rs`, `rendered_ui_snapshot` (`economy_ui.rs:1118`) are tied to the engine panel | M | M2-M3 | P1 | own-1 | yes: C12 | The test instrument must read a mod-built HUD. |
| bevy-rendering-engineer-14 | Spectator and multi-map view controls (player switch, map switch UI) | partial | S:366, S:372-374 | B | none | later (M11, map milestone) | CA01 | no | |

## Conflicts with the goal (Decided items)

- No Decided item in section 2 blocks the goal. D37 (S:60), D54 (S:77) and the S:151 principle support it.
- S:164 (Decided principle, S:151) puts "command panels, tooltips, screens" in mod policy. It does not say who
  owns selection, control groups, camera input, or the minimap. Needs decision: the engine/mod line for each
  HUD element. Options: (a) engine widgets plus mod layout and content (SC2, WC3 pattern); (b) a full mod UI
  runtime (SupCom, Factorio pattern); (c) engine-fixed HUD (conflicts with S:164).
- The code conflicts with Decided text, not the reverse: C3 and C4 hardcode camera and light against D37;
  C7 is an engine-owned HUD with content-specific English text against S:164 and S:97 (no privileged
  default-game path).
- D41 (S:64) forbids local game saves. Local key and graphics settings are not saves. No conflict; state it
  (own-10).

## Lens answer

**Home.** The architecture has a name for the home (S:122, S:164) but no room. Input, selection, camera, and
HUD have no section, no status, and no milestone (S:674-688). Only the camera API is in the open list (S:701).

**Split.** Engine mechanism: picking, the action map, modifier and double-click detection, browser capture,
focus routing, selection set and groups (client-local), camera controller and clamp, decal and bar renderers,
minimap and fog renderer, audio mixer and limits, settings storage, the event stream, and a UI widget set.
Mod policy: default bindings, selection priority, smart-command rules, camera limits and angle, lighting,
HUD layout and text, icons, cursors, sounds, colours, alerts.

**Public API a mod needs** (direction only): (1) action declarations with default keys and a command id;
(2) a command-card entry per command (icon, key, grid slot, tooltip text id, availability from the shared
rule, own-6); (3) a cursor table keyed by context; (4) a HUD definition (layout tree bound to permitted state
queries, own-1, own-2); (5) camera and light settings (D37); (6) a string table (UI30); (7) sound and effect
bindings to presentation events (own-3).

**Real client today.** Extend: interpolation (`present.rs`, `interp.rs`); footprint picking
(`economy_ui.rs:721-769`); the UI-over-world guard (`input.rs:52-71`); the read-only `live_view.rs` observer.
Replace: `Selection(Option<u32>)` (C1); the click handler with hardcoded gather-else-move (`input.rs:87-141`);
the camera spawn and the per-frame `fit_scene_camera` (C3); the hardcoded light and one shared unit colour
(C4, C5); the whole `economy_ui.rs` panel model and layout (C7). M4 draft section 9 extends C1, C6 and C7
(M4D:262-269). Building drag-select on C7 adds to code that must be replaced. Recommend P1 before M4.

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| R1 Python regex sweep of 45 term groups over S (output in greps file) | B2 "player-control layer absent" (`inventory-engine.md:566-589`) | a hit in the player sense for cursor, zoom, pan, health, chat, double-click, highlight, bookmark | 0 hits each; "select" 37 hits, all other senses |
| R2 `rtk proxy grep` "accessib/subtitle/colour-blind/scale" in S | no accessibility line in S | any player-accessibility hit | S:213 is map "accessibility policies"; others are workload scale |
| R3 read `input.rs`, `render.rs`, `economy_ui.rs` (parts), `live_view.rs` header | brief claim `economy_ui.rs:31` one-unit selection | `Selection` holds a set | `Selection(pub Option<u32>)` |
| R4 `rtk proxy grep -rn` KeyCode, keyboard, MouseWheel, CursorIcon, Audio, PointerLock, contextmenu in `crates/` (no dist) | client has no keyboard, wheel, cursor, audio code | any source hit | one hit: a test (`tests/economy_ui.rs:1476` scroll unit) |
| R5 `cargo tree -p turts_client -i bevy_audio` host and `--target wasm32-unknown-unknown` | no audio in client (C9) | bevy_audio in the tree | "nothing to print" both; `bevy_picking` present |
| R6 `rtk proxy grep` camera, hotkey, icon, cursor, colour, sound, layout, light in `turts_mod_api/src` and `mods/` | mod API has no presentation fields beyond Shape (C10) | any field | 0 hits; `schema.rs:48-51` Shape only |
| R7 `rtk proxy grep` health, alert, event in `turts_net_protocol/src` | no event channel; health on wire (C11) | an event or alert message | `HealthRecord` at `messages.rs:256`; no alert or event type |
| R8 WebFetch MDN `Keyboard/lock` | browser can capture reserved keys | Baseline support | "experimental", "not Baseline"; Ctrl+W handling not stated |
| R9 `rtk proxy grep` bevy_window-0.20.0 `prevent_default_event_handling` doc | client blocks F5 and Ctrl+R (`lib.rs:254`) | doc says no effect on these | `window.rs:262` "prevent common browser hotkeys like F5, F12, Ctrl+R, tab" |

## NOT TRACED

1. Ctrl+W, Ctrl+T, Ctrl+N, Ctrl+1..9 capture in Chrome, Firefox, Safari (A-IN08, A-PC08).
2. Browser autoplay policy effect on first audio playback (A-AU03).
3. Bevy 0.20 `UiScale` resource for A-AC03.
4. Frame cost of per-unit UI text nodes at 5,000 units (own-9); no probe run.
