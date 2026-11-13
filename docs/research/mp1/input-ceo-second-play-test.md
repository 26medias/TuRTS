# The CEO's second play test of the MC forge build: first-click record, the gap, and conditions for MP1

Source: `ux-designer`, test mode, 2026-11-12. Build: `TuRTS-mc-play` at `9b3a7bd` (branch `codex/mc-fix-pt1`), the
same dist and release binaries that the CEO's run used (the dist was built at 09:24 local time for that run; my packed
`forge.tar` is byte-equal to the CEO's, `cmp` exit 0). Not committed. Evidence folder:
`/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/first-click/`
(abbreviated `FC/` below).

## 1. What I ran

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| `pgrep -af turts_server` | The CEO's server still runs; I must not touch it | no process | pid 2465796 runs (`run-20261112T142337Z-qUwURP`); not touched |
| `turts-pack FC/pack/forge` then `cmp` with the CEO's tar | My run uses the CEO's package | `cmp` differs | equal |
| `mc_headed_v6.sh --mode pre-merge ... --label ux-ceo2-firstclick-1 -- bash FC/held.sh 1` (own server and static server on new ports; `FC/fc.js`) | A person who clicks each slot-0 building after load gets a response | a selection or an order list after a building click | 9 clicks, selection `[]` after each; no panel; exit 0, 59 s hold |
| same, label `ux-ceo2-firstclick-2a`, `FC/fc2.js` phase A (1280x720): click "Close" first, then the same clicks | The order lists exist once the guide is closed | no order list after the guide closes | the order list shows for every building |
| same, label `ux-ceo2-firstclick-2b`, phase B (1920x1080) | The guide and "Close" look the same in a large window | "Close" in the top-right corner | "Close" floats at x 1179 to 1271, far from the corner |
| reads of `turts_debug` and `turts_ui` after each step (bridges read only to find screen points and to count sounds; every input is a real mouse event) | What is selected, what is shown, what sounds | - | the `*.debug.json` and `*.ui.json` next to each screenshot |
| extraction of the round-2 `player` transcript (`subagents/agent-af381734207815f3d.jsonl`) to `FC/blind-r2-report.txt` | What the blind check looked at | - | section 4 |

Default window: the wrapper's Xvfb screen and the harness default, 1280x720 at DPR 1. The CEO's window size is NOT TRACED.

## 2. First-click record (task 1)

Sound: I did not listen. The counts come from `turts_debug.audio.sink_observations` (one record for each sound that
started; the browser blocks sound until the first click, `FC/run-1/02-after-10s.debug.json`: `context_state:
"suspended"`).

### 2.1 Run 1: a person who does not find "Close" (the most probable path of the CEO)

| Screenshot | Selected? | Name text | Button or option | Goal | Next hint | Sound |
| --- | --- | --- | --- | --- | --- | --- |
| `FC/run-1/01-load.png` (1.2 s) | no | "Loading game", "forge 0.2.0" | none | no | no | none (audio blocked) |
| `FC/run-1/02-after-10s.png` | no | the guide covers the full screen (rect 0,0,1280,720, `02-after-10s.ui.json`); the map shows through a dim layer; "Click to play" in the centre | "Close" (92x44, top right, on the Alerts panel) | yes, in the guide: "Your goal / Blue side: ... Orange side: ..." (9 paragraphs, plus a 21-row key list) | the guide text | none; 2 alert lines already listed before any input |
| `FC/run-1/03-click-drill.png` to `11-click-lab.png` (one left click on drill, 4 arm loaders, a belt piece, press, turret, lab) | no: `selection: []` in each `*.debug.json` | none | none; the Orders panel stays hidden (`command_card` `visible: false`, `03-click-drill.ui.json`) | same guide | none | no select sound (nothing selected); the pointer shows the "select" cursor over each building (`cursor.context: "select"`) |
| `FC/run-1/13-turret-rightclick-ground-200ms.png`, `14-...-1400ms.png` | no | none | none | same | none | none |
| `FC/run-1/16-idle-30s-later.png` (58 s) | no | none | none | same | none | 2 alert sounds in 58 s (`final-debug.json`: `forge:forge_alert` at frames 2309 and 3209) |

Result: from the pixels, a person sees a live-looking map and buildings. Every click does nothing, and the cursor says
it would select. This is the CEO's sentence "I clicked on every building and I don't see a single option".

### 2.2 Run 2a: the same build after two clicks on "Close"

The first click on "Close" did not close the guide (`FC/run-2a/steps.log`: "second click on Close"). The "Click to play"
layer takes the first click (`crates/turts_client/src/client_shell.rs:868-897`, `FocusPolicy::Block`, full screen).

| Screenshot | Selected? how shown | Name text | Options (Orders panel, x 992-1272, y 222-492) | Goal | Hint | Sound |
| --- | --- | --- | --- | --- | --- | --- |
| `A1280-02-guide-closed.png` | no | none | none | bottom box: "You are blue ... 1. ... 5." and "You are orange ... 1. ... 5." (two 4-line blocks) | the task list | - |
| `A1280-03-hover-drill.png` | - | tooltip "Bore Drill · Slot 1" | - | same | - | - |
| `A1280-04-click-drill.png` | yes: panel "Selected: Bore Drill", info panel; no ring is seen on the map | "Bore Drill", "Owner: you" | Disable this machine [D], Enable this machine [E] | same | none | select voice |
| `A1280-05-click-arm_loader_a.png` (and `10`, `11`) | yes, panel; a small unit | "Arm Loader" | Disable, Enable, Remove this machine [X] | same | none | select voice |
| `A1280-06-click-belt.png` | yes, panel; faint ring | "Chain Belt", "Inventory: []" | Remove this machine [X] | same | none | select voice |
| `A1280-09-click-press.png` | yes, panel; no ring seen (the ring radius 1.2 equals the footprint, `A1280-final-debug.json` `rings_shown`) | "Shell Press", inventory, "Working: Make Shells" | Disable, Enable | same | none | select voice |
| `A1280-12-click-turret.png` | yes, panel and range ring | "Shell Turret", damage, range, shells | Disable, Enable, Hold this position [H] | same | none | select voice |
| `A1280-13-click-lab.png` | yes, panel | "Proving Lab", "Research: Hardened Shells — Needs 10 Shell (0/10)" | Cancel this job [C], Disable, Enable, Research an upgrade [R] | same | "This opens at 10 shells, in about 1 minute." | select voice |
| `A1280-14-turret-rightclick-150ms.png` | turret | notice "Shell Turret: Cannot move. It is fixed in place." at 150 ms | - | same | - | error sound |
| `A1280-17-after-disable.png` | press | tooltip "Disable this machine [D] — Disable the selected units." | the press still shows "Make Shells 0.1s remaining"; no "off" word; Enable stays | same | none | order sound; server accepted unit 4 (`command_results`) |
| `A1280-18-idle-20s.png` (56 s) | press | 4 alert lines, 3 of them "Arm Loader: waiting for items. This is normal." | - | same | none | in 56 s: 11 select, 1 error, 1 order, 3 alert sounds |

Run 2b (`FC/run-2b/B1920-01-after-10s.png`): at 1920x1080 the guide text uses the left third, the key list the
middle, and "Close" floats at x 1179, y 15, not in a corner and not near the text.

## 3. The gap for each building (task 2)

Every slot-0 building has at least one order in the package. "Nothing to do" is not true content; it is a display and
input fault.

| Building | Orders in the package | Automatic work |
| --- | --- | --- |
| Bore Drill | Disable, Enable (`mods/forge/orders/order_disable.ron:1`, `order_enable.ron:1`) | extracts ore (`units/drill.ron:10`) |
| Arm Loader | Disable, Enable, Remove (`order_remove.ron:1`) | moves items (`units/arm_loader.ron:10`) |
| Chain Belt | Remove (`order_remove.ron:1`) | carries items (`units/belt.ron:10`) |
| Shell Press | Disable, Enable | makes shells (`units/press.ron:11`) |
| Shell Turret | Disable, Enable, Hold (`order_hold.ron:1`) | fires at slot 1 (`units/turret.ron:10-16`) |
| Proving Lab | Cancel, Disable, Enable, Research (`order_cancel.ron:1`, `order_research.ron:1`) | research (`units/lab.ron:10`) |

Why a person did not see them:

1. The guide opens on every join (`crates/turts_client/src/widgets/guide.rs:13-19`; no stored "seen" flag). The forge
   guide is a full-screen box on layer 40 (`mods/forge/interface/hud.ron:118-128`, `size: (w: Fill, h: Fill)`), with a
   fill of alpha 0xd0, so the map shows through it.
2. A full-screen widget captures the pointer: a press over any widget is sent as `over_widget: true`
   (`crates/turts_client/src/core_adapter.rs:891-913`, `over_widget` at `:581-587`), so no world click selects. The
   cursor still shows "select" over a building (`FC/run-1/03-click-drill.debug.json`); cause of that mismatch NOT TRACED.
3. The Orders panel shows only with a selection (`hud.ron:71`, `visible: [SelectionNotEmpty, PrimaryOwned]`), so with
   the guide open it never shows. The task list box is hidden while the guide is open (`hud.ron:105`).
4. "Close" is the only mouse exit. It sits at a fixed offset from the top-left corner (`hud.ron:125-126`, offset
   1176/1178), so it is in the top-right corner only at a width of 1280. Its label is at the top-left of its box. The
   first click on it goes to "Click to play". F1 also closes the guide; the guide names it only in row 20 of its key list
   ("Toggle guide: F1", `FC/run-1/02-after-10s.png`).
5. When the guide is closed, the options are plain to see (white text on black rows, 28 px high, at the right edge,
   y 255 to 383). The order icons are 1x1 pixel images (`mods/forge/assets/icon_order_*.png`), so the rows have no
   picture. Disable gives no visible result (`A1280-17-after-disable.png`), and Enable shows while the machine is on.

## 4. Why the blind check passed (task 3)

Evidence: the prompt and report of the round-2 `player` agent (`FC/blind-r2-extract.txt`, `FC/blind-r2-report.txt`),
the run it judged (`target/mc-evidence/pt1fix2-headed/runs/run-20261112T055036Z-yrA5gA/`), and the driver
(`TuRTS-mc-fix-pt1/tests/browser/pt1-driver.js`).

- It looked at 27 screenshots that the PT1 driver made. The driver clicks "Click to play" at a calibrated point
  (`tests/browser/rows/PT1.json` `stages.click_to_play`), then closes the guide with the F1 key that it reads from the
  effective bindings, not from the screen (`pt1-driver.js:125-126`, `:146-158`). Then it finds each building and clicks it.
- So the agent saw "guide open", then "guide closed" (`pt1-s03-guide-closed.png`). It wrote "I must close the guide
  (there is an orange Close button)" (`FC/blind-r2-report.txt:7`). It assumed the close worked, and it never saw what a
  click on a building does while the guide is open. The state that stopped the CEO is not in any frame.
- The prompt named the frames in their order ("the first frame, the guide open, the guide closed, then the next six").
  It asked "would I know what to do", but every frame after it was the result of an input the agent did not choose.
- The agent answered the goal from the 9-paragraph guide and the bottom box. An agent reads 300 words in one look; a
  person in a first minute does not.

The method fault in plain words: the agent judged a film made by a player who already knew the game. It answered "can
these steps be done, and do the screens read well when someone does them", not "can I find the first step by myself".
It never chose an input, so it could never be stuck.

The same fault is in the MP1 plan: checkpoint C1 steps 1 and 2 (`C1-PLAYER1`, `C1-PLAYER`) judge the evidence set of the
`I-PLAY1` driver, which "plays both windows ... by the stage-1 Player path inputs" (plan :1114, :1157, :1196-1198).

## 5. Person-readiness protocol for the `player` seat (task 4)

The seat must pass this before anyone asks a person to play. Any FAIL item stops the request.

Instrument (to build; owner `qa-lead`; it does not exist today): a step tool for a headed Chrome under
`mc_headed_v6.sh`. The agent calls it once for each input: `move x y`, `click x y left|right`, `drag x1 y1 x2 y2`,
`wheel dy`, `key <code>`, `wait s`. The tool does the input and returns screenshots at +200 ms and +1,000 ms. It writes
one evidence folder: every screenshot, the input log with times, and the `turts_debug.audio` sound records as text
(released to the agent only after C3). The tool finds nothing for the agent: no bridge, no bindings, no coordinates.
`FC/fc.js` is a fixed-script form of the same idea.

| # | Check | PASS | FAIL |
| --- | --- | --- | --- |
| C0 | The prompt | holds only the start command or URL and "a person plays this next; can they?" | it names a goal, a key, a unit, a step, a coordinate, a guide, or a design file (the reviewer greps the prompt) |
| C1 | Cold start | the agent reads no repository file, memory note or earlier report before C3 | any such read in the transcript |
| C2 | Inputs | each input targets a thing seen in an earlier screenshot, and each key is one that a screenshot showed | a key or point not seen on screen (the reviewer matches the input log to the screenshots) |
| C3 | At 60 s after the first click | the agent writes, before more input: the goal, the first action, and how it knows the action worked, each with a screenshot file | one of the three is missing, "do not know", or has no screenshot |
| C4 | Failure signs | none of F1 to F10 is seen | any one |
| C5 | Reference red | the protocol returns FAIL on `9b3a7bd` forge at the first building click (F2, F3) | it passes on `9b3a7bd`: the instrument is broken |

Failure signs:

- F1: no goal text in the first 5 s after the first click, with no scroll, key or page change.
- F2: a left click on an own object gives no visible change at +200 ms (no ring, panel or text).
- F3: the world shows through a layer that takes the click.
- F4: an action exists only as a hotkey or only in a list that needs a scroll.
- F5: one sound id more than 3 times in 60 s, or any alert sound before the first input.
- F6: an alert line before the first input.
- F7: a refused order with no reason text within 1 s.
- F8: a control that needs two clicks where one was given (a first click taken by another layer).
- F9: a text block of more than 40 words on screen in the first 60 s that the player did not open.
- F10: a screen with no visible way forward for 30 s.

Verdicts: PERSON-READY (C0 to C5 pass) or NOT-READY (give the step and the screenshot). No "with a sheet" verdict: the
request to a person holds only the start, the time limit and "say where you stopped".

## 6. MP1 mapping (task 5)

Status: the MP1 plan and design are gated work; the conditions below are inputs, not edits.

| CEO complaint | MP1 task and row | Red on today's forge build? |
| --- | --- | --- |
| No goal (first try); "I don't get what there is to test" for the game | `UI-HINT`, `C-UI`, row GOAL1 (D:2468 in the design file today; the plan cites D:2461: the goal line "Destroy the enemy Command Centre" and the H1 hint in the first frame); 8.3 points 1, 3, 9 (plan :1169-1177) | GOAL1: red, but only because forge has no goal line and no hint (it is a different package). 8.3 judged by the `player` from a driver set: not red; the round-2 agent stated the goal from the guide |
| No option on any building | GOAL1 (Command Centre selected by `start_selection`; the hint driver clicks ground, then the Command Centre); `UI-HUD` rows HUD5, HUD6/s1 | The `start_selection` part does not test a click. The hint-driver part would go red if a layer took the click (hint X4 stays), but only if no driver closes a guide first. HUD5 and HUD6 test row states, not reachability. Design D:1995 says "the guide does not open at the start", but no task owns it and `guide.rs:13-19` opens it on each join; frontier has a guide in stage 2 (`C2-CONTENT`, plan :1251) |
| "I don't get what you need from me" | `I-PLAY1`, the C1 sheet (plan 8.4, :1184-1192) | no row; the review question of `I-PLAY1` (:1111) is about the browser start, not about what the person reads |
| Non-stop beeps (first try: 330 alert sounds in 213 s, reported in `vault/Reports/2026-11-12 MC play instructions for the CEO.md:10-11`, not re-measured) | `UI-ALERT`, row ALR2 (D:2475: no alert before the first input, one sound when a state starts, 4 s gap); 8.3 point 4 | ALR2: red on the first-try build (sounds 0.6 s apart on average). On today's build: 8.3 point 4 passes (3 alert sounds in 56 s); ALR2 "no alert line before the first input" is red (2 lines at 10 s, `FC/run-1/02-after-10s.debug.json`) |

Missing acceptance conditions, one testable sentence each:

- M1 (first click reaches the world): in each window, a pixels-only driver that presses no key and clicks only "Click to
  play" and then the centre of each own building on screen gets a non-empty `selection` and a visible order list (or
  build menu) within 200 ms of each click, in the first 20 s; plant: a full-screen widget with a fill alpha below 1.0
  makes it red.
- M2 (guide closed at the start, owner needed): with a guide page in the interface, `GuideOpen.is_open` is false in the
  first presented frame after "Click to play"; today's `observe_phase` (`widgets/guide.rs:14-19`) is the red build.
- M3 (one click per control): after "Click to play", the first click on any visible button acts; a driver click on the
  guide close control at its first frame closes it (today: two clicks).
- M4 (close is findable): a modal panel that covers more than half the viewport has its close control within 64 px of
  the top-right corner of the viewport at 1280x720 and at 1920x1080 (pixel check on both sizes).
- M5 (text load): in the first 60 s, no visible text widget that the player did not open holds more than 40 words
  (a `turts_ui` text-length check).
- M6 (cursor is true): when the pointer is over a capture widget, the cursor context is never "select" or an order
  context (a headless check of the cursor feed over a capture panel).
- M7 (the agent gate): `C1-PLAYER1` and `C1-PLAYER` use the cold-start protocol of section 5 with its step tool, not the
  `I-PLAY1` driver set; the protocol returns FAIL on the plant of M1.
- M8 (the request): the C1 sheet fits on one screen (at most 15 lines), names one task, one time limit and what to
  report, and a cold `player` agent given only the sheet says back the task and the stop rule.
- M9 (visible result of an order): after Disable, the selected machine shows an "off" state within 1 s, and Disable is
  hidden while it is off (engine request E2, `input-mc-pt1-engine-requests.md:8-9`).

## 7. NOT TRACED

- The CEO's play run (`TuRTS-mc-play/target/play-runs/runs/run-20261112T142337Z-qUwURP`) ended at 14:55:22Z with
  `exit_code 130` (`completion.json`; 130 is the INT trap of `scripts/mc-browser.sh:221`, as from Ctrl+C). That time is
  inside my run 1 (lock held 14:54:50 to 14:55:49Z). My scripts send no INT and stop only their own two pids
  (`FC/held.sh`); the wrapper ends only processes with its own marker. The cause of the INT is NOT TRACED: ask the CEO
  whether they pressed Ctrl+C.

- The CEO's window size, and whether the CEO closed the guide during the second try. Run 1 reproduces the words; it
  does not prove the path.
- Why the cursor shows "select" over a capture widget.
- Whether the selection ring of a 1.2-radius building is drawn under its mesh (only `rings_shown` and the screenshots
  were read).
- The first-try sound count (taken from the vault report).
- The plan cites GOAL1 at D:2461 and HUD5 at D:2451; in the design file today they are at D:2468 and D:2458 (a line
  shift; the plan text is not changed here).
- Whether frontier stage 1 has any widget with `Not(GuideOpen)` (no frontier package exists in `TuRTS-mp1/mods/`).
