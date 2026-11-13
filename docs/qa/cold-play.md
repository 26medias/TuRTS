# Cold play: the person-readiness instrument (Q-COLD)

**The answer of an agent, "yes" or "no", is never the verdict.** An agent reads every text and obeys it; a person clicks
what they see. On 2026-11-12 a fresh agent with no project memory played the MC `9b3a7bd` forge build with this tool
(session `scratchpad/c5-cold/session-1`, 24 inputs, statement stored at 62 s). It clicked "Click to play", then "Close",
read the guide, finished the research goal in 54 s and answered "Yes, a person can play it". The CEO, a person, clicked
the buildings first on the same build and got no response. The verdict of a readiness run therefore comes only from
checks that do not depend on the agent: scripted probes that the tool runs by itself, and the UI record. The checker has
no input for the answer of an agent. If a probe fails, stop: no agent session and no request to a person.

Source: the second play test of the CEO (`docs/research/mp1/input-ceo-second-play-test.md`, section 5), the orchestrator
ruling of 2026-11-12 (`errata-pending-6.3.md`, last section) and the review `review-q-cold.md`.

## 1. A readiness run, in order

Every step that opens a headed Chrome runs under `mc_headed_v6.sh --mode pre-merge` (the GPU lock; call it with
`bash`). Every probe and every agent session needs its own **fresh game**: a player slot is permanent (D25), and a second
client on one server sees "This game is full". Session folders must not exist and must be outside the git tree of the lane.

1. **Prepare**, outside the lock: `(cd tests/browser && npm ci)`; for a game of this tree
   `bash scripts/mp1-play.sh --prepare PREP --package mods/PACKAGE` (a prepared folder is stale after any commit:
   `docs/qa/prepared-runs.md`).
2. **The reference red, once for each tool version** (check C5; section 5). Four runs on the reference build: the first-click
   probe and its control (`--control close-layer`) at 1280x720 and at 1024x768. `session.json` of each records the tool
   commit and the sha256 of each tool file. If the tool files change, these runs are stale: run them again.
3. **First-click probe of the candidate build at 1280x720, then at 1024x768** (`--probe first-click --viewport WxH`).
   Both must PASS (P1, P3, P4), with P5, F9, F9b, F3 and F6 clean.
4. **Buttons probe** (`--probe buttons`): F8-start, F8, and the 20 s watch for F6.
5. **If step 3 or 4 failed, stop.** The result is NOT-READY. Run no agent session. Ask no person.
6. **Agent session** (300 s, section 4): the statement at 60 s and the report.
7. **The checker** over all folders:
   `python3 -I -B scripts/qa/cold_play_check.py FC1280 --also FC1024 --also BUTTONS --also AGENT --reference REF1 --reference REF2 ... [--receipt <label dir>/lock-receipt.json]`.
   Read `RESULT:` (section 6).
8. **The manual checks** (section 7). Only a run with `MACHINE-PASS (manual checks open)` and all manual checks passed
   goes to a person, with a request that holds only the start, the time limit and "say where you stopped".

## 2. The parts

| File | Role |
| --- | --- |
| `tests/browser/cold-play.js` | The call of the agent: `node tests/browser/cold-play.js <session folder> <call>`. It has no browser code. |
| `tests/browser/cold-session.js` | The session process: the Chrome connection, real mouse and keyboard events, the screenshots, the held-back records, the evidence folder, `session.json` with the tool hashes. |
| `tests/browser/cold-probe.js`, `cold-pixels.js` | The probes and the two pixel measures. Role `instrument`: they read the bridges; nothing of this goes to an agent. |
| `scripts/cold-play.sh` | Starts the game and a headed Chrome (`scripts/mp1-play.sh --silent`), then the session process. |
| `scripts/cold-reference.sh` | `rebuild`, `verify` and `serve` for the reference build. |
| `scripts/qa/cold_play_check.py` | The checker for the reviewer. No browser. |
| `tests/browser/cold-fixture.js`, `cold-probe.test.js`, `cold-play.test.js`, `tests/qa/test_cold_play_check.py` | The fixture page and the tests. Plants: `docs/research/mp1/plants/q-cold.json`. |

## 3. The probes (no agent)

Started with `scripts/cold-play.sh --probe <kind>`. The session runs the probe and ends; the folder holds
`probe-<kind>.json`, `ui-records.json`, the screenshots, the input log and `session.json`. The tool reads the scene,
debug and UI bridges for this. The page is "ready" when the UI record, the replica and the scene have content.

### 3.1 First-click probe (P1, P3, P4, P5)

- **Watch and start.** The probe records the UI each 1 s for 8 s with no input, then makes ONE click to start: the centre
  of the text of the start control that the UI record shows (`focus_overlay`), or the page centre when there is none.
- **P1, the units** (condition M1). One left click at the projected centre of the visible body of each own unit on screen (one of each content id
  first, at most 60). No key. Each click is **seen** only when there is a pixel change in a box around the unit (half
  side = the larger of 48 px and 1.5 times the projected `pick_radius`, so the box scales with the object) or a change of a
  panel in the UI record (a widget that is not an alert line, not a tooltip and not what changes by itself in the 200 ms
  before). A change elsewhere on the page does not count (an alert line that appears by itself does not make a click
  seen). PASS only when every click gives a selection that holds the unit AND is seen. No own unit on screen is FAIL.
- **P3, the orders** (M3; the place where the CEO looked). For the first unit of each content id, the probe selects it
  and clicks each visible enabled order button of that selection one time (the destructive ones, Remove and the like,
  last; the pointer rests on the button first). Each needs a result within 1 s: a change of the panel or state, a pixel
  change at the unit, or a new notice text. PASS needs at least one order button to exist. A hotkey-only order (no button)
  fails P3. A selection of a content id with no order button is not a failure.
- **P4, four more clicks.** One enemy unit (its panel must show); one resource node (a name must show, as a panel or a
  tooltip); one left click on open ground after a selection (the selection must clear); one right click on open ground
  with a unit selected (a notice, an order mark or a pixel mark within 1 s). Open ground is a point near the own units
  with no unit or node within 2 times its radius (at least 60 px) and under no panel. A thing that is not on screen is
  skipped, and the record says so.
- **P5, a control outside the page.** A visible button whose centre is outside the page, or a Close-like button that the
  UI record marks as not visible because its place is outside the page (the client clips it), is a FAIL. At 1024x768 the
  "Close" of the forge guide has a fixed offset of 1176 px and is outside.
- **The positive control** (`--control close-layer`). This overlay-only control uses ground centres. It does not certify visible-body picking in the old reference client. After the start click the tool closes the layer that met it, by the
  visible button whose text is Close, Dismiss, OK or the like (at most 3 clicks). The control is judged on the layer
  only (`PC`): the units must pass; the orders and extra clicks are shown (`INFO`), not judged. It must PASS on the build
  where the plain probe fails: this shows that the probe can pass. A control whose close button is outside the page is a
  `CONTROL-FAIL` (P5 at that size).

### 3.2 Buttons probe (F8-start, F8, F6)

- **Watch 20 s** with no input: the UI record each 1 s (F6, below).
- **F8-start.** If a start overlay shows and a button is visible under it, that button (the first, top to bottom) gets ONE
  click before any start click; the pointer rests on it 300 ms first. A lost click is a separate finding that is a
  **FAIL of its own** (`F8s`). The product fix is an open MP1 item (section 8).
- **F8.** Then the start click (if the overlay is still there), and one click on each visible button, top to bottom. The
  button **acted** when the UI record changed (the start overlay and tooltips left out, and what changes by itself),
  or the pixels changed in a box around the button, or the camera moved (the median shift of the unit centres in the
  scene is more than 3 px: an alert line that centres the camera has no other mark). There is no page-wide pixel rule.

### 3.3 The UI record and the checks from it

The session keeps a UI record before the first input and after the first click (+200 ms and +1000 ms), in agent sessions
too. It is held back like the sound records and released with them.

| Check | Fact (FAIL when true) | Number and reason |
| --- | --- | --- |
| F9 | after the first click a visible widget text of more than 40 words that no input opened | 40 words: adults read silent non-fiction at 238 words a minute (range 175 to 300; Brysbaert 2019), so 40 words take 10 to 14 s, a fifth of the first minute |
| F9b | the visible text in all (start overlay, tooltips and world labels left out) is more than 60 words | 60 words take 15 to 21 s at that rate. It closes the split case: a 189-word guide cut into 38-word widgets passes F9 and fails F9b |
| G1 | recorded, not a verdict: the words of the first text that states a goal (a word like goal, objective, destroy, win, research) | a rule for the goal line (the review asks 15 words or fewer) needs the declared goal text of the design; it is not a verdict yet |
| F3 | after the first click a widget of more than half of the page that takes pointer input (`capture_root` or `focus_policy_block`) and lets the world show through: at least 5 % of its pixels (text and a 2 px margin left out) are not its own fill colour; or the layer cannot be measured | The UI record has no fill alpha (see below), so F3 reads the picture. The 5 % is provisional: 3 times below the 15 % of the reference. Weakness: on a dark or flat map a see-through layer can measure below 5 %. |
| F6 | an alert line or a debug alert at any second of the watch (8 s in the first-click probe, 20 s in the buttons probe) or in the record before the first input | The design rule ALR2 forbids an alert line at any time before the first input (D 2475). 20 s: at the slow end of the reading rate a 40-word block takes about 14 s, plus some seconds to look around. The 8 s wait before the first click stays: a layer that opens at the join is there at any time, so the P1 result does not change with time. |
| F5b | an alert sound before the first input | as before |

**F3 and the fill alpha.** The product knows the fill alpha of a layer (`hud.ron`, for example `#07111ed0`), but `turts_ui`
does not export it. The export is in the product (the UI snapshot), not in the instrument, so it is **owed**: add the
alpha (and the font size, below) to the UI snapshot (owner: the UI snapshot task, `L0-DBG`). Until then F3 keeps the 5 %
picture rule with the weakness above.

## 4. The agent session

The agent plays one input at a time from pixels (the call list, below). It does **not** give the verdict; it gives
evidence that a person would also meet (the faults that no probe knows yet).

    move X Y | click X Y left|right | drag X1 Y1 X2 Y2 | wheel DY | key CODE | wait S | status | statement FILE | report FILE | end

- A point is a pixel of the screenshot (1280 x 720 by default, in CSS pixels). `click` and `drag` press a real button; `key`
  presses one key by its code; `wheel` turns where the pointer is; `wait S` is time with no input (0 to 30 s).
- Each call returns the file paths of two screenshots (200 ms and 1000 ms after the input ended) and the clocks.
  The tool finds nothing for the agent: no bridge value, no binding, no coordinate of a thing, no element lookup.
  The tests check that every reply line is one of a fixed list of forms.
- **The limit is 300 s** (`--limit-secs`, 1 to 1500; the wrapper holds 1800 s). The faults of the first cold session came
  in its first 60 s. The tool has no idle limit.
- **The 60 s statement and the gate (check C3).** The clock starts at the first `click` or `drag`. At 60 s the reply says
  `STATEMENT DUE` and every input is refused until `statement FILE` stores a file of six lines (GOAL, GOAL_SHOT,
  FIRST_ACTION, FIRST_ACTION_SHOT, WORKED, WORKED_SHOT; each `KEY: value`, each SHOT a screenshot of the session). The
  sound records and the UI records stay in memory until then; `releaseSounds` is the only writer of their files and only
  two places call it: the statement (after `statement.txt` is written) and the end of the session.
- **The report form** (`report FILE`, after the statement, one time; check C6). A fixed form, so that builds can be compared:

        FINDING 1: CLASS | SHOT | INPUT | EXPECTED | SEEN
        FINDING 2: ...
        STUCK: CALL | SHOT | TEXT        or        STUCK: none

  CLASS is one of `no-response`, `wrong-response`, `stale-text`, `hidden-control`, `covered-control`, `unclear-text`,
  `too-much-text`, `sound`, `other`. SHOT is a screenshot file of the session; INPUT is the number of the call that caused it,
  or `none`; EXPECTED and SEEN are 12 words or fewer. There is **no yes or no line**; the tool refuses one.
- **How a reviewer reads the report.** The checker lists each FINDING and the STUCK line (`INFO C6`). Open each SHOT and
  confirm that it shows what SEEN says. Compare builds by CLASS and by the widget text in SEEN: a class that comes again on a
  new build is a candidate for a new machine rule. The first cold session found three faults that no probe saw (Disable shows
  nothing; a stale refusal text; a tooltip over a button). P3 now covers the first one. A STUCK line is the place to look
  first for F10. The report never changes the result line.
- **The prompt (check C0).** Only the question and the command; no goal, key, unit, step, coordinate, guide or file. The agent
  needs a tool that can show a PNG file. Give it exactly:

        A person plays this game next, with a mouse and a keyboard. Can they?
        You play it with this command (run it with no call first to read what it does):
            node /ABS/PATH/tests/browser/cold-play.js /ABS/PATH/SESSION

  Do not give the agent the output of the start command or any file of `SESSION/internal/`: they name the debug port.

## 5. The reference build and check C5

The instrument must be shown to go red on the build where a person failed: MC commit `9b3a7bd`, package `forge` (the
operator's play build). Because the proof must not depend on a worktree or on `/tmp`:

- **The copy.** `/home/julien/Projects/TuRTS/target/mc-evidence/q-cold-reference/9b3a7bd-forge/` holds `turts_server`,
  `forge.tar`, `dist/` and `SHA256SUMS`. Check it: `bash scripts/cold-reference.sh verify <that folder>`.
- **Rebuild it** (outside the GPU lock; about 520 s with a cold cache at 4 jobs; needs `trunk`, `wasm-bindgen-cli`, `wasm-opt`:
  `scripts/setup-dev.sh`). From the repository root of this tool:

        git worktree add --detach /tmp/WT 9b3a7bd
        mkdir -p /tmp/WT/target/mc-q-cold-ref
        cd /tmp/WT && MC_JOBS=4 python3 -I -B /home/julien/Projects/TuRTS/target/mc-evidence/gate-runner/exec_in_lane_v4.py \
          --lane /tmp/WT --target mc-q-cold-ref --head 9b3a7bd --label qcold-ref-rebuild --audio -- \
          bash /PATH/TO/THIS/TOOL/scripts/cold-reference.sh rebuild /tmp/WT-out

  `rebuild` runs `cargo build --release -p turts_server`, `cargo build --release -p turts_pack --bin turts-pack`,
  `turts-pack mods/forge`, `trunk build --release` in `crates/turts_client_web`, and copies the three results to the output
  folder with a `SHA256SUMS`. The runner exits 2 at the end because the pack and `dist/` are written into the lane
  (source changed); read `command_exit` in `receipt.json`: it must be 0.
- **Verified once** on 2026-11-12 (rustc 1.99.1, cargo 1.99.1, trunk 0.22.0, the commands above in a scratch worktree
  at a path that differs from the play build): **all five files are byte-equal to the copy**: `turts_server`
  (`9d8e2986...`), `forge.tar` (`01fa7db5...`), the wasm (`3cb81774...`), the js (`88cfaa59...`) and `index.html`
  (`80637a76...`; `cmp` and `sha256sum`). They are equal although the worktree path differs, so no path is embedded in them.
  This holds for this host and these tool versions; another Rust or trunk version can give other bytes, and then the copy
  stays the reference and the rebuild is only a check.
- **Start a fresh game** (each probe and each session):

        bash scripts/cold-reference.sh serve /home/julien/Projects/TuRTS/target/mc-evidence/q-cold-reference/9b3a7bd-forge STATE &

  It prints `URL: ...` and `PIDS: STATE/pids.txt` and stops on TERM. Use the URL with `scripts/cold-play.sh --url URL`. Stop
  the game by the pids in the file, never by a pattern.
- **The reference runs for a tool version** (step 2 of section 1): `scripts/cold-play.sh --url URL --probe first-click`,
  and the same with `--control close-layer`, at `--viewport 1280x720` and at `--viewport 1024x768`, each with a fresh
  game, `--build "MC 9b3a7bd forge ..."`, under the wrapper. Keep the four folders; pass them to the checker with
  `--reference`.
- **C5 in the checker.** C5a: every reference folder records the sha256 of the tool files that are in this checkout
  (`cold-probe.js`, `cold-pixels.js`, `cold-session.js`, `cold-play.js`, `harness/png.js`, `cold_play_check.py`) and a clean
  tool tree; a difference is red: run the reference again for this tool version. C5b: the first-click probe FAILS and its
  control PASSES. C5c: the folders name the build `9b3a7bd`. A readiness run with no `--reference` is NOT-READY.

## 5b. Sound: no Chrome of this tool plays on the desktop

`scripts/cold-play.sh` starts the play script with `--silent`. In the silent form `scripts/mp1-play.sh` gives the Chrome the
**argument** `--alsa-output-device=null` and the **environment** `PULSE_SERVER=unix:/nonexistent/mc-tests-no-pulse` and a null
ALSA configuration (`</usr/share/alsa/alsa.conf>` then `pcm.!default { type null }`, written next to the profile and removed
at the end). The environment alone is not enough (a process tree can lose it); the argument is required. The silent form is the
default under the lock wrapper (`MC_LOCK_RUN` set), with `--url` and with `--silent`. The person form (a human at a real desktop)
keeps the normal sound: `scripts/mp1-play.sh --prepared DIR` outside the wrapper, or `--with-sound`. No `--mute-audio`; the
system sound is never changed. Tests: `cold-play.test.js` starts the script with a shim for the Chrome and asserts the arguments
and the environment that the script gave (silent by default, `--silent`, and `--with-sound` as the person form);
`harness/silent-chrome-scan.test.js` scans the test files and the shell scripts under `scripts/` that start a Chrome.

## 6. The checker and the result line

    python3 -I -B scripts/qa/cold_play_check.py FOLDER [--also FOLDER ...] [--reference FOLDER ...] [--receipt R] [--no-lock-check] [--partial] [--json]

The result line is `RESULT: NOT-READY (...)` when any machine check or probe fails or a part is missing (the first-click
probe at 1280x720 and at 1024x768, the buttons probe, the agent session, the reference run), and
`RESULT: MACHINE-PASS (manual checks open)` otherwise. A folder of a control run is judged on its probe only
(`CONTROL-PASS`). `--partial` (tests, a look at one folder) prints `PARTIAL-PASS`. Exit 0 for a pass, 1 for NOT-READY, 2 when
a folder cannot be read. It never reads the agent's answer and never says PERSON-READY.

Machine-checked, in every folder: E1 the event log is complete and in order; E2 every screenshot is there with its sha256, and
every call has its two pictures in time; L1 the session limit is at most 1500 s, the session ended cleanly and on time; L2 the
lock marker of the wrapper (L2b: with `--receipt`, the receipt says `ok` and the hold is inside its limit); S1 the sound
records are final and the bridge answered; F5b; U0 the UI records exist; F9, F9b, F3, F6. First-click probe: P1, P3, P4, P5.
Buttons probe: F8s, F8. Agent session also: C2a (every `key` names a key; the keys are listed with the last screenshot
before each), C3 (the statement exists, was stored before the release and 60 s or more after the first click, with no input
between; six parts; none "do not know"; each part names a session screenshot taken before it), C6 (the report in its fixed
form), F5a (no sound id more than 3 times in 60 s). F5a is not judged in a probe: its clicks make the sounds.

## 7. The manual checks

| # | What | Method |
| --- | --- | --- |
| C0 | The prompt | `grep -n -i -E "goal\|key\|unit\|click\|guide\|button\|\.md\|[0-9]+, *[0-9]+" PROMPT` on the prompt that was given. FAIL when it names a goal, key, unit, step, coordinate, guide or file. The prompt of section 4 passes (only the command and the question). |
| C1 | The agent read nothing before the statement | In the transcript list each tool call before the time of `statement_stored` (events.jsonl). FAIL on any read of a file that is not a screenshot of the session, on any `Read` of a repository file or memory note, and on any look into `SESSION/internal/`. |
| C2 | Each input targets what an earlier screenshot showed | For each click, drag and wheel line of `inputs.jsonl` open the last screenshot before it and confirm that the point is on a visible thing. For each `key`, the checker prints the last screenshot before it: confirm that a screenshot showed that key (on a label or a key list). Write one row per input: `call, point or key, shot, seen: yes/no`. FAIL on a `no`. |
| C3m | The statement is true | Open the three shots of `statement.txt`: each must show what its line says. |
| C4 | The faults that no machine check covers | F1: in the screenshot 5 s after the first click, is there a visible goal text (the G1 line says where the machine found one)? F4: is any action only a hotkey, or only in a list that needs a scroll? F7: for a refused order, is there a reason text within 1 s (P4 covers the right click on ground only in part)? F10: does the STUCK line or any screenshot show 30 s with no way forward? F2 is machine-checked by P1. |

## 8. Open product items (not changed by this tool)

- **Start overlay and the first click** (review finding M4; ruling (c) for the tool). On a build where a HUD button shows
  under "Click to play", the first click on that button is taken by the overlay (`client_shell.rs:868-897`). The tool
  records it as `F8s`, a FAIL of its own. The fix is a product condition for `L-LOBBY-C1` and `UI-HUD`: the overlay passes the
  first click to the button under it, or shows nothing clickable under it. Until then every build with such a button is
  NOT-READY.
- **Fill alpha and font size are not in the UI snapshot** (section 3.3): owed to the UI snapshot task.
- **The goal-line rule** (G1) is recorded, not judged.
- **muster at this head:** the guide opens at the join (F9, 66 words): condition M2, `UI-HINT`.

## 9. Not in this pass (owed)

- **The cursor-truth check** (the cursor says "select" over a layer that takes the click; M6 of the CEO record): it needs
  the cursor context at each click and a rule for what the context must be; the debug record has `cursor.context`, but the
  ruling did not take the check in this pass.
- **The text-size check** (body text at least 12 px at 1280x720, from XAG 101: 18 px at 1080p for PC): the UI snapshot has no
  font size.

## 10. Limits (known)

- The screenshots show the page, not the pointer or its cursor shape. The page is 1280 x 720 or 1024 x 768 at a pixel ratio
  of 1, set by emulation (`--viewport none` keeps the real window). The window size of a person is NOT TRACED.
- The inputs are real mouse and keyboard events of the browser, sent over the debug port (trusted events, not events of the
  window system).
- The statement gate and the order of the log are the tool's own record; the plants show that the tests go red when the gate
  is cut. Sound times are samples (0.5 s).
- The probes click the first 60 own units, one of each content id first; a crowded map is sampled.
- A pass with nothing to click (a buttons probe with no visible button) says "no visible button" in its reason: read it.
- Timing: the +200 ms picture can start late under a heavy load; the checker fails a picture that starts more than 500 ms
  (+200 ms) or 700 ms (+1000 ms) late.

## 11. Results

Evidence is in the scratch folder `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/q-cold/` (not in the repository; the reference build and its sha256 values are in
the durable folder of section 5). Tool commit of these runs: `8155550e` (the tool files are unchanged since; the later commits
only change the start scripts, tests and plants). Every run was under `mc_headed_v6.sh --mode pre-merge`, each on a fresh
game of the reference copy.

**Reference build `9b3a7bd` forge** (`p3c/`, labels `qcold3c-*`):

| Folder | Result |
| --- | --- |
| `fc-1280/` first-click probe, 1280x720 | **FAIL** P1: "first click on unit 1 (forge:drill): selection [], no visible change; 14 of 14 clicks failed". F9 189 words; F9b 292 words; F3 guide area 100 %, see-through 15 %; F6 1 alert line. |
| `ctl-1280/` control, 1280x720 | `CONTROL-PASS`: the tool closed the guide in 1 click; 14 of 14 units selected and seen. P4 PASS (4 of 4 extra clicks answered). **P3 FAIL: no visible result within 1 s for Disable this machine [D] on forge:drill, arm_loader, press and turret; Enable on arm_loader, press and turret; Hold on turret; Cancel and Research on lab** (the place where the CEO looked; the first cold agent's "Disable shows nothing"). |
| `fc-1024/` first-click probe, 1024x768 | FAIL P1 as above, plus **P5: the guide_close button is outside the page**. F3 also sees `guide.4.bindings` (area 52 %, see-through 27 %). |
| `ctl-1024/` control, 1024x768 | `CONTROL-FAIL`: "the close control is outside the page: ['guide_close']". A person at this size has no mouse way out of the guide. |
| `btn/` buttons probe | F8-start **FAIL**: "the first click on guide_close changed nothing: the start overlay took it". F8 PASS (3 of 3 buttons acted after the start: Close by a UI change, the two alert lines by a moved camera). F6 first alert line at 6.1 s of the 20 s watch. |

`check-p3c-ref.txt`: `RESULT: NOT-READY (failed: P1, F9, F9b, F3, F6 [1280]; P1, P5, F9, F9b, F3, F6 [1024]; F8s, F9, F9b, F3, F6 [buttons]; missing: agent session)`,
with **C5a, C5b, C5c all PASS** (the four reference folders record the sha256 of the tool files that are in the checkout).

**MP1 `muster` at this branch head** (prepared; `p3e/`): first-click **P1 PASS 60 of 60**; **P4 PASS**; **P3 FAIL**: 11 of 13
order buttons show nothing within 1 s (Move, Stop, Hold, Attack, Queue, Gather: instant or modal orders with no visible
mark in the UI record) and **their labels are empty** (the button text is only ` [M]`, ` [S]`: the hotkey without a name);
F9 `guide: 66 words`, F9b 69 words (the guide opens at the join: condition M2, `UI-HINT`); G1: no visible goal text; the
buttons probe passes with "no visible button after the start click: nothing to click" (read that reason).

**Silence proof** (2026-11-12, after the fix of the start scripts; `p3d/silence.log`, `p3d/silence.py`). A control session on the
reference build was run under the wrapper, and each 3 s a read-only `pw-dump` and `ps` were taken: 39 samples with the Chrome of the session alive
(17 processes; the browser process has `--alsa-output-device=null` in its arguments: `ps` shows
`chrome --user-data-dir=/tmp/turts-play-profile.* --remote-debugging-port=... --alsa-output-device=null`); in the same time the page
started 40 sounds (7 alert sounds, `sound-records.json`). In no sample was there a PipeWire client whose `application.name` contains
"ALSA", a client whose process id is one of the session's Chrome processes, or an audio output stream of those processes (16 clients,
all desktop clients that were there before). Before the fix the same session started the Chrome with neither the argument nor the
environment and the alert sound played on the desktop (the operator heard it; a "PipeWire ALSA [chrome]" client was seen).

