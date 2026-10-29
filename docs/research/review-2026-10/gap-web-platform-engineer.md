VERDICT=fail ITEMS=20 MISSING=17 PARTIAL=2 CONFLICT=0 RUNS=12 NOT_TRACED=7

Seat: web-platform-engineer. Browser tested: Google Chrome 156.0.8078.47, Linux, headed, X display :1, Playwright default args. Host keyboard layout: AZERTY (probe output).
S = `RTS-Engine-Project-Specifications.md` v0.7. Probe files: `scratchpad/review/wpe-probe/` (page/, script/, probe-out.json, hidden-out.json, src/ = fetched Chromium source).

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A-IN02 | Action mapping layer | partial | S:122 "input" in one table cell. Searched: hotkey, keybind, key binding, rebind, shortcut: 0 player hits in S | mechanism (engine maps keys to named actions); policy (mod gives default bindings) | none | M4, before K1 drag-select ships | none | yes: client has no keyboard input; `KeyCode` 0 hits in `crates/turts_client/src` | Direction: one engine action table that UI, hotkeys and tests all use. |
| A-IN08 | Browser input capture (context menu, reserved keys, pointer lock) | missing | Searched S: contextmenu, context menu, pointer lock, reserved, preventDefault: 0 hits | mechanism | none | M4 | IN02 | no for context menu: winit calls `preventDefault` on `contextmenu` and `keydown` on the canvas (`~/.cargo/registry/src/*/winit-0.30.13/src/platform_impl/web/web_sys/canvas.rs:296-303,435-439`), on because `prevent_default_event_handling: true` (`crates/turts_client/src/lib.rs:254`). This is a side effect, not a design. | Right-click works today by accident of a default. No test asserts it. |
| A-IN09 | Text input mode (chat does not fire hotkeys) | missing | Searched S: chat, text input: "chat" 0 hits | mechanism | none | with chat (no milestone has chat) | IN02 | NOT TRACED (no chat exists) | An HTML text field takes focus from the canvas; winit stops key events then. Direction: decide canvas-only UI or HTML overlay first. |
| A-IN10 | Touch and gamepad | present (as deferral) | S:77 D54 Decided; S:99 "Phones/tablets and mobile controls are deferred" | mechanism | later phase | later phase | IN02 | no; note winit calls `preventDefault` on `touchstart` (`canvas.rs:245-251`) | Deferral is decided. Keep the action table input-device neutral so touch can map later. |
| A-CA02 | Edge scroll | missing | Searched S: edge scroll, scroll, pan: 0 control hits. S:60 D37 gives the camera to mods | both: engine camera input; mod sets limits and speed (D37) | none | M4 (terrain makes the map larger than one view) | AC05, wpe-7 | yes: camera is auto-fit with no pan (`crates/turts_client/src/render.rs:133,245`); canvas is a fixed 1280x720 box (`crates/turts_client_web/index.html:8`) | In a non-fullscreen tab the cursor leaves the canvas before it hits a screen edge. Edge scroll needs pointer lock or a canvas-edge band. |
| A-CA03 | Key scroll | missing | Searched S: arrow, WASD: 0 hits | mechanism | none | M4 | IN02, wpe-2 | yes: no keyboard input in client | WASD must bind by physical code; on AZERTY the probe read KeyA = "q". |
| A-AC01 | Rebindable keys | missing | Searched S: rebind, binding, key map: 0 hits | mechanism; mod supplies defaults | none | after IN02, before 16.2 first playable mod | IN02, wpe-9 | no | Storage is not defined; see wpe-9. |
| A-AC05 | Cursor confine (pointer lock) | missing | Searched S: pointer, cursor, confine: 0 hits | mechanism | none | M4 with CA02 | IN08, wpe-4 | no | Pointer lock hides the system cursor (MDN). |
| A-PE01 | Order responsiveness and input latency | partial | S:243 "immediate input acknowledgement" (Decided paragraph, Baseline item); S:560 numeric latency thresholds Open | mechanism | none measured | M4: measure click-to-local-feedback in the browser | IN02 | NOT TRACED: no input-latency figure in `docs/measurements/m1.md` (grep "latency": only S quotes at :283) | Single-threaded wasm: input is read once per frame. A software cursor (wpe-4) adds frame latency to the cursor itself. |
| B-12 | Hotkeys for commands and production | missing | Same terms as IN02; inventory-player.md:378 | both | none | M4 foundation; per-button keys with the command card | IN02, wpe-1, wpe-2 | yes: no keys exist | Default keys must avoid the Chrome reserved set (wpe-1). |
| B-77 | Rebindable hotkeys in game | missing | Same as AC01 | mechanism | none | as AC01 | AC01 | no | Same gap as AC01. |
| wpe-1 | Reserved-key matrix per browser and mode | missing | Searched S: reserved, shortcut, Ctrl: 0 hits. Chromium `accelerator_table.cc:347-359` reserves CLOSE_TAB, CLOSE_WINDOW, NEW_TAB, NEW_WINDOW, NEW_INCOGNITO_WINDOW, RESTORE_TAB, SELECT_NEXT/PREVIOUS_TAB, CYCLE_TO_NEXT/PREV_TAB, EXIT. `browser_command_controller.cc:523-543`: in fullscreen only FULLSCREEN and EXIT stay reserved | mechanism (engine refuses a binding the browser will eat) | none | M4, before any default key is chosen | none | no | Windowed Chrome: Ctrl+W, Ctrl+F4, Ctrl+T, Ctrl+N, Ctrl+Shift+T/N/W, Ctrl+Tab, Ctrl+PgUp/PgDn cannot be taken. Ctrl+1..9, F1-F12 except reserved commands are not reserved, so `preventDefault` takes them. Firefox and Safari: NOT TRACED. |
| wpe-2 | Keyboard layouts (physical code versus label) | missing | Searched S: layout, AZERTY, QWERTY: 0 input hits | mechanism | none | M4 with IN02 | none | no | Probe: this host is AZERTY. `getLayoutMap` gave KeyQ="a", KeyA="q", Digit1="&". Control groups 1..9 need physical codes; labels need the layout map. `getLayoutMap` outside Chrome: NOT TRACED. |
| wpe-3 | Fullscreen and Keyboard Lock | missing | Searched S: fullscreen, keyboard lock: 0 hits | mechanism | none | M4 decision; implement with options menu | wpe-1 | no | Chrome doc: Keyboard Lock works only in JS-started fullscreen; a 2 s Esc hold exits. Esc is the usual RTS cancel key, so Esc design is a browser decision. Probe: `navigator.keyboard.lock` present in Chrome 156. |
| wpe-4 | Software cursor under pointer lock | missing | Searched S: cursor: 0 hits | mechanism; mod supplies cursor art (MD08) | none | M4 with AC05 | AC05 | no | MDN: pointer lock hides the cursor and needs a user gesture; Esc exits it. The engine must draw the cursor. Chrome doc says lock requests need permission since Chrome 130 (detail NOT TRACED). |
| wpe-5 | Audio unlock (autoplay policy) | missing | S:122 "audio" is one word; searched autoplay, sound: 0 hits | mechanism | none | the first milestone with sound (none planned; M4 lists sound out of scope, `docs/superpowers/specs/2026-10-28-m4-terrain-design.md:23`) | a user gesture in the load path (S:282 setup flow) | no: the wasm build has no audio; `cargo tree -p turts_client_web --target wasm32-unknown-unknown -i bevy_audio` prints "nothing to print" | Probe: `AudioContext` state "suspended" at load, "running" after click plus `resume()`. Direction: unlock audio on the join click. |
| wpe-6 | Tab hidden, focus loss, pause | missing | Searched S: visibility (only fog-of-war senses), focus, background tab: 0 client hits | mechanism | none | M4 test; design before M10 reconnect | replica resync (exists, T8) | NOT TRACED: winit Poll uses `scheduler.postTask` (`winit-0.30.13/src/platform_impl/web/web_sys/schedule.rs:42`); its rate in a hidden tab is not measured. Bevy releases held keys on focus loss (`bevy_input-0.20.0/src/keyboard.rs:199`) | Chrome doc: rAF waits until the page is visible; timers run 1/s, then 1/min after 5 min hidden. The M3 playtest uses two tabs in one browser (`vault/Reports/2026-10-28 M3 human playtest repair.md:19`), so one client is hidden. Server behaviour toward a hidden client: NOT TRACED. |
| wpe-7 | Canvas focus and full-viewport canvas | missing | Searched S: canvas, viewport: 0 hits | mechanism | none | M4 | none | yes: `#wrap` is fixed 1280x720 (`crates/turts_client_web/index.html:8`) | winit listens for keys on the canvas only (`canvas.rs:296-303`). Keys do nothing until the canvas has focus. |
| wpe-8 | Test instrument for browser input | missing | `rtk proxy grep -c "keyboard\.(press|down|type)" tests/browser/*.js`: 0 in all 12 files | test | none | M4 | wpe-1 | yes: add keyboard paths to browser tests | Playwright key events go through CDP to the page and bypass browser accelerators (probe: Ctrl+Digit1 and F5 reached the page). Such a test cannot go red for a reserved key. Playwright also kept a background and a minimized page "visible" (hidden-out.json), so a hidden-tab test cannot go red under Playwright defaults. |
| wpe-9 | Storage of player settings (bindings, volume) | missing | S:282 browser storage covers package setup only; searched settings, preferences: 0 hits | mechanism | none | with AC01 | AC01 | NOT TRACED for design; fact: each server serves the client from its own origin (`crates/turts_server/src/http.rs:53`) | Browser storage is per origin. Bindings saved on server A are absent on server B. Eviction (S:282) also applies. Needs decision: per-origin, export/import, or account-bound. |

## Conflicts with the goal

- No Decided item in section 2 blocks a browser RTS control scheme. D02 (S:25), D54 (S:77) and D37 (S:60) support it.
- D37 (Decided, S:60) gives the camera to mods. It does not say who owns camera input (edge scroll, key scroll). Not a conflict. Needs decision on the engine/mod split.
- Wording defect, not a Decided item: S:790 says "desktop-first delivery" is settled. D02 (S:25) and S:99 say browser first. S:790 probably means "desktop browsers first" (D54). Needs a wording fix.

## Lens answer

No document plans how an RTS control scheme survives in a browser tab. S has 0 hits for pointer lock, context menu, fullscreen, reserved keys, keyboard layout and autoplay. M4 adds left-drag and right-click (`m4-terrain-design.md:264-268`) and puts hotkeys out of scope (:23).

Today the web client has one mouse path: left selects, right orders (`crates/turts_client/src/input.rs:44-45`). It has no keyboard input, no camera control, and a fixed 1280x720 canvas. Right-click works only because winit suppresses the context menu by default.

Browser constraints that decide the control design before UI work starts:
1. Windowed Chrome reserves Ctrl+W/T/N/Tab/PgUp/PgDn and their Shift forms. Ctrl+1..9 and most F-keys are free in Chrome. Fullscreen frees all except F11 and exit. Firefox and Safari are NOT TRACED.
2. Bind by physical key code. AZERTY digits need Shift (probe: Digit1 = "&").
3. Edge scroll needs pointer lock or a full-viewport canvas. Pointer lock hides the cursor, needs a gesture, and Esc exits it. Keyboard Lock needs fullscreen and uses a 2 s Esc hold.
4. Audio starts suspended until a gesture.
5. A hidden tab stops rAF and throttles timers.
6. Settings storage is per origin, and each server is its own origin.
7. Playwright cannot prove reserved-key capture or hidden-tab behaviour. These need a different instrument.

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| `rtk proxy grep -n -i -E "browser|keyboard|mouse|input|audio|focus|mobile|touch|fullscreen|pointer|..." S` | S plans browser input (S:99, S:122) | any hit naming pointer lock, context menu, reserved keys, fullscreen, autoplay | 0 such hits; S:99, S:122 only |
| `rtk proxy grep -rn "KeyCode|ButtonInput<KeyCode>" crates/turts_client/src` | client has keyboard input | any hit | 0 hits |
| `rtk proxy grep -c "keyboard.(press|down|type)" tests/browser/*.js` | browser tests cover keys | count above 0 | 0 in 12 files |
| `cargo tree -p turts_client_web --target wasm32-unknown-unknown -i bevy_audio` | wasm client builds audio | a dependency path | "nothing to print" |
| read winit 0.30.13 `canvas.rs` | right-click and keys are suppressed by design | no preventDefault on contextmenu/keydown | preventDefault on both, gated by one flag |
| curl Chromium `accelerator_table.cc`, `browser_command_controller.cc` (main) | Ctrl+1..9 is browser-reserved (yardstick IN08) | SELECT_TAB_n in kReservedCommandIds | not in the list; only close/new/cycle tab, window, exit |
| WebFetch MDN Pointer Lock | pointer lock keeps cursor visible | doc says cursor stays | cursor hidden; gesture needed; Esc exits |
| WebFetch Chrome autoplay | AudioContext runs at load | doc says it runs | suspended until gesture + resume |
| WebFetch Chrome keyboard-lock | Keyboard Lock works windowed | doc says windowed works | fullscreen only; 2 s Esc hold |
| WebFetch Chrome timer throttling | hidden tab keeps frames | doc says rAF runs | rAF waits for visible; timers 1/s, 1/min |
| `node script/probe.js` (Chrome 156) | autoplay, layout, capability claims above | audio "running" at load; layout QWERTY | suspended then running; AZERTY; lock/layoutMap/webgpu present |
| `node script/hidden.js` (minimized window) | a Playwright test can see a hidden tab | visibilityState "hidden" | "visible", rAF 240 in 4 s: instrument cannot go red |

## NOT TRACED

1. Firefox and Safari reserved keys, Keyboard Lock and `getLayoutMap` support.
2. Real OS key input for Ctrl+1..9 in Chrome (source says not reserved; CDP probe bypasses accelerators).
3. Rate of winit `scheduler.postTask` loop in a hidden tab.
4. Server behaviour toward a hidden, non-acknowledging client.
5. Chrome 130 permission rule for pointer lock (quoted from a fetch summary, not read in full).
6. Input-latency figure for the wasm client.
7. Who owns camera input under D37 (engine or mod).
