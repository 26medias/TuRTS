# Cold play: the step tool for the person-readiness protocol (Q-COLD)

**The answer of an agent, "yes" or "no", is never the verdict.** An agent reads every text and obeys it; a person clicks
what they see. On 2026-11-12 a fresh agent with no project memory played the MC `9b3a7bd` forge build with this tool
(session `scratchpad/c5-cold/session-1`, 24 inputs, statement stored at 62 s). It clicked "Click to play", then
"Close", read the guide, finished the research goal in 54 s and answered "Yes, a person can play it". The CEO, a person,
clicked the buildings first on the same build and got no response. The verdict of a readiness run therefore comes only
from checks that do not depend on the agent: two scripted probes that the tool runs by itself, and the UI record. The
order of a readiness run is: **probe first** (a fresh game); if it fails, stop: no agent session and no request to a
person. The checker never reads the agent's answer.

Source: the second play test of the CEO (`docs/research/mp1/input-ceo-second-play-test.md`, section 5). The `player`
agent had judged screenshots that a driver made. The driver closed the guide with a key that it read from the bindings.
So the agent never met the first click that a person makes. This tool lets an agent that has only a URL play one
input at a time, from pixels, and leaves evidence that a reviewer can check.

## The parts

| File | Role |
| --- | --- |
| `tests/browser/cold-play.js` | The call of the agent: `node tests/browser/cold-play.js <session folder> <call>`. It sends one call to the session and prints the reply. It has no browser code. |
| `tests/browser/cold-session.js` | The session process. It holds the Chrome connection, does the inputs with real mouse and keyboard events, takes the screenshots, keeps the sound records back, and writes the evidence folder. |
| `scripts/cold-play.sh` | Starts the game and a headed Chrome (`scripts/mp1-play.sh`), then the session process. Ends both at the session limit. |
| `scripts/mp1-play.sh --url URL` | The play script in its "Chrome only" form: for a game that another command serves. |
| `tests/browser/cold-probe.js`, `cold-pixels.js` | The probes (first click, buttons) and the two pixel measures. Role `instrument`: they read the bridges; nothing of this goes to an agent. |
| `tests/browser/cold-fixture.js` | A small game page for the tests of the probes. |
| `scripts/qa/cold_play_check.py` | The checker for the reviewer. No browser. |
| `tests/browser/cold-play.test.js`, `cold-probe.test.js`, `tests/qa/test_cold_play_check.py` | The tests. Plants: `docs/research/mp1/plants/q-cold.json`. |

## The calls of the agent

    move X Y | click X Y left|right | drag X1 Y1 X2 Y2 | wheel DY | key CODE | wait S | status | statement FILE | end

- A point is a pixel of the screenshot (the page is 1280 x 720 by default, in CSS pixels). The tool refuses a point
  outside the page.
- `click` and `drag` press a real button. `key` presses one key by its code (`KeyA`, `Digit1`, `Escape`, `F1`, `Space`).
  `wheel` turns where the pointer is. `wait S` is time with no input (0 to 30 s); `wait 0` only takes a screenshot.
- Each call returns the file paths of two screenshots: 200 ms and 1000 ms after the input ended (`wait`: after the wait).
  The reply also has the clocks: the time since the first click, and the session time left. Nothing else.
- The tool finds nothing for the agent. A reply holds no bridge value, no binding, no coordinate of a thing and no
  element lookup. The tests check this: every reply line must be one of a fixed list of forms.

### The 60 s statement and the gate (protocol check C3)

The 60 s clock starts at the first `click` or `drag`. Each reply says how far it is. When 60 s have passed, the reply says
`STATEMENT DUE`, and the tool refuses every input (`move`, `click`, `drag`, `wheel`, `key`) until the agent stores its
statement. `wait`, `status` and `end` still work. The statement is a file of six lines:

    GOAL: <what the agent thinks the goal is>
    GOAL_SHOT: <a screenshot file name>
    FIRST_ACTION: <the first thing it would do>
    FIRST_ACTION_SHOT: <a screenshot file name>
    WORKED: <how it knows that the action worked>
    WORKED_SHOT: <a screenshot file name>

`statement FILE` is refused (exit 3) before any click, before 60 s after the first click, and when a part is missing or
names a file that is not a screenshot of this session. A refused statement stores nothing and opens nothing.

The gate: the sound records of the debug state (`turts_debug.audio.sink_observations`, read each 500 ms) stay in the
memory of the session process. They reach the disk only in `releaseSounds`, which only two places call: `storeStatement`
(after `statement.txt` is written) and the end of the session. The log has the order (`statement_stored` before
`sounds_released`). A session that ends with no statement releases the records at the end, and the log says
`end-without-statement`; the checker fails C3 for it.

### The limits

- The session limit is 600 s from the start of the session (`--limit-secs`, 1 to 1500). The headed wrapper holds the lock
  for 1,800 s at most, so the ceiling is 1500 s. At the limit the session stops, writes the sound records and `end.json`,
  and ends Chrome and the game. A call that is running finishes first (it can add about 3 s). Calls after the end exit 4.
- The wait in one call is 30 s at most. The agent can end the session with `end`.
- The tool has no idle limit: an agent that stops holds the lock until the session limit.

## The probes (no agent)

Each probe is a session of its own, started with `scripts/cold-play.sh --probe <kind>` on a **fresh game** (a slot is
permanent). The session runs the probe and ends; the folder holds `probe-<kind>.json`, `ui-records.json`, the screenshots
and the input log. The tool reads the scene, debug and UI bridges for this; the agent never sees them.

- **`--probe first-click`** (condition M1). The page is ready when the UI record, the replica and the scene have content.
  The probe waits 8 s (a person looks first), then one click to start: the centre of the text of the start control that
  the UI record shows (`focus_overlay`), or the page centre when there is none. Then one left click at the screen
  centre of each own unit that is on screen (one of each content id first, at most 60), with no key and no other
  input. For each click it records the selection after the click and the pixel change at +200 ms in a box of 96 px
  around the point and in the whole page (a picture taken 200 ms before gives the idle noise). The probe PASSES only when
  every click gives a selection that holds the clicked unit AND a visible change; a page with no own unit on screen
  FAILS. A layer that takes the click (F2, F3) is a FAIL whatever an agent does later.
- **`--probe first-click --control close-layer`** (the positive control). After the start click the tool closes the
  layer that met it, by the visible button whose text is Close, Dismiss, OK or the like, then probes as above. It must
  PASS on the build where the plain probe fails: this shows that the probe can pass. A control is not a readiness run.
- **`--probe buttons`** (F8). No start click. For each button that the UI record shows as visible, top to bottom: the
  pointer moves onto it and rests 300 ms (a tooltip or a glow is hover feedback, not the click), then one left click.
  The button **acted** when the UI record changed (the start overlay and tooltips are left out; what changed by itself
  in 200 ms is left out) or the page changed by more than 1 % (not counted when the start overlay left on this click).
  A button whose first click changed nothing is a FAIL: a layer took it.

The session also keeps a **UI record** before the first input and after the first click (+200 ms and +1000 ms), in agent
sessions too. It is held back like the sound records and released with them. The checker derives from it:

| Check | Fact (FAIL when true) |
| --- | --- |
| F9 | after the first click a visible widget text of more than 40 words that no input opened |
| F3 | after the first click a widget of more than half of the page that takes pointer input (`capture_root` or `focus_policy_block`) and lets the world show through: at least 5 % of its pixels, text and its 2 px margin left out, are not its own fill colour; or it could not be measured |
| F6 | before the first input an alert line (a visible `alert_line` widget with text) or a debug alert |
| F5b | an alert sound before the first input (as before) |

The UI record has no fill colour, so F3 reads the picture. F3 and F9 look at the first click only: a text that the
player opens later is not F9.

## Run a session (the orchestrator)

1. Outside the lock: `(cd tests/browser && npm ci)`, then `bash scripts/mp1-play.sh --prepare PREP --package mods/<package>`
   for a game of this tree (a prepared folder is stale after any new commit: `docs/qa/prepared-runs.md`).
2. Under the headed wrapper, in the background. `SESSION` must not exist and must be outside the git tree of the lane:

       /home/julien/Projects/TuRTS/target/mc-evidence/gate-runner/mc_headed_v6.sh --mode pre-merge \
         --lane LANE --target mc-NAME --head HEAD --label qcold-LABEL --audio -- \
         bash scripts/cold-play.sh --session SESSION --prepared PREP --package mods/PACKAGE --build "text"

   For a game that another command serves (an old build, for example), use `--url URL` (a loopback address) in place of
   `--prepared`.
   A probe uses the same command with `--probe first-click`, `--probe first-click --control close-layer` or
   `--probe buttons` added, and a fresh game for each. There is no control file: the probe ends the session, and the
   folder is complete when the wrapper command ends (a probe of a small page takes 10 to 40 s of the lock).
3. For an agent session: wait for `SESSION/internal/control.json`. Then start the agent. Do this only when both probes
   passed.
4. When the wrapper command ends (exit 0), the evidence folder is complete. A wrapper exit of 124 or 137 is the hold
   limit, not a test red.

### The prompt of the agent (protocol check C0)

The prompt holds only the question and the command. It names no goal, key, unit, step, coordinate, guide or file. The
agent needs a tool that can show a PNG file. Give it exactly:

    A person plays this game next, with a mouse and a keyboard. Can they?
    You play it with this command (run it with no call first to read what it does):
        node /ABS/PATH/tests/browser/cold-play.js /ABS/PATH/SESSION

Do not give the agent the output of the start command or any file of `SESSION/internal/`: they name the debug port.
The tool cannot stop an agent that looks for them; check C1 (the transcript) finds that.

A game server gives each player slot to one client for the whole match (D25). A second session on the same server
sees "This game is full". Start a new server (a new `--prepared` start, or a new `turts_server`) for each session.

## Check a session (the reviewer)

    python3 -I -B scripts/qa/cold_play_check.py SESSION [--receipt <label dir>/lock-receipt.json] [--json]

Give the three folders of a readiness run: `cold_play_check.py <first-click probe> --also <buttons probe> --also <agent
session>`. The result line is `NOT-READY (...)` when any machine check or probe fails or a part is missing, and
`MACHINE-PASS (manual checks open)` otherwise. A folder of a control run is judged on its probe only (`CONTROL-PASS`).
`--partial` (tests, a look at one folder) prints `PARTIAL-PASS` instead. Exit 0 for a pass, 1 for NOT-READY, 2 when a
folder cannot be read. The output has two lists.

Machine-checked: E1 the event log is complete and in order; E2 every screenshot file is there with its recorded sha256,
and every call has its +200 ms and +1000 ms pictures, taken in time; L1 the session limit (at most 1500 s, ended
cleanly, on time); L2 the lock marker of the wrapper (and with `--receipt` the receipt: status `ok`, hold inside its
limit); C2a every `key` names a key (the keys are listed with the last screenshot before each, for the manual match);
C3 the statement exists, was stored before the sound records were released, was stored 60 s or more after the first
click, with no input between the 60 s mark and the statement, has the six parts, none of them "do not know" (or
`unknown`, `none`, `not sure`, ...), and each part names a screenshot of the session that was taken before the
statement; S1 the sound records are final and the debug bridge answered; F5a no sound id more than 3 times in 60 s;
F5b no alert sound before the first input; P1 the first-click probe passed; U0 the UI records exist; F9, F3, F6 from the UI
record (above); F8 the buttons probe passed.

Manual (listed as `TODO`; the tool does not check them): C0 the prompt; C1 no repository file read; C2 each point and
key was seen in an earlier screenshot; C3m the three parts are true to the screenshots that they name; C4 the failure signs that a
machine cannot see (F1, F4, F7, F10 and the look of F2); C5 the reference red. The checker never says PERSON-READY.

The evidence folder: `session.json`, `events.jsonl`, `inputs.jsonl`, `shots/`, `statement.txt`, `sound-records.json`,
`end.json`, `internal/`. The time of a sound is the time of the first read in which it appears (a resolution of 0.5 s).
A sound that waited for the first click (the browser blocks sound until a gesture) shows at the click.

## Limits (known)

- The screenshots show the page, not the pointer or its cursor shape. A cursor that lies (the M6 case of the CEO record)
  is not visible to the agent.
- The agent sees a page of 1280 x 720 pixels at a pixel ratio of 1, set by emulation (`--viewport none` keeps the
  real window). The window size of a person is NOT TRACED.
- The inputs are real mouse and keyboard events of the browser, sent over the debug port. They are trusted events, but
  they are not events of the window system.
- The statement gate and the order of the log are the tool's own record. They stop an agent from reading the sound
  records by accident. They do not stop a reviewer from a tool that lies; the plants show that the tests go red when
  the gate is cut.
- Sound times are samples (0.5 s). The checker cannot tell a sound that was requested before the first input from one
  that the browser held back until the first click.
- F1, F2, F3, F4, F6 to F10 need a person or an agent that reads the screenshots.
- Timing: the +200 ms picture is taken when the capture starts. Under a heavy load it can start late. The checker
  fails a picture that starts more than 500 ms (+200 ms) or 700 ms (+1000 ms) late.

## The reference red (protocol check C5), run 2026-11-12

The protocol must return FAIL on the build where a person failed. The tool was run on the forge package of the MC
tree at `9b3a7bd` (a copy of the operator's play build: `dist` and `turts_server` with the sha256 values in
`ref-build/SHA256SUMS` of the scratch folder; the server and the static files were started outside the lock, and the
session used `--url`). One scripted session, 9 calls, the wrapper label `qcold-ref-5` (hold 116 s of 1800 s, exit 0).
The evidence folder is `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/q-cold/ref-session-5/`.

| Call | What the screenshots show |
| --- | --- |
| 1 `wait 8` | The page after load: a full-screen guide (goal text, key list) over the map, and "Click to play" in the centre. |
| 2 `click 640 360 left` | "Click to play" is gone. The guide stays, full screen, with the map under it. |
| 3, 4, 5 `click` on the centre of three blue buildings | No ring, no panel, no name, no order list. 0 pixels differ between the pictures before and after each click, apart from 665 pixels (an orange unit and the minimap) at the first. The world shows through the guide, which takes the click (F2, F3). |
| 6, 7 `wait 30`, `wait 20` | At 72 s after the first click the reply says `STATEMENT DUE`. A `click` is refused (exit 3). No sound file exists yet. |
| statement | Stored. The sound records are released after it. |
| 8 `click` on "Close" (one click), 9 `click` on the same building | The guide closes at once. The building click now changes 28 % of the pixels: 182,954 in the left panels and 75,434 in the order panel (the control: the tool does see a change when there is one). |

The checker on this folder (`ref/check-5.txt` in the scratch folder): every machine check passes except F5a:
`forge:forge_alert` started 4 times from 41.0 s to 66.5 s (6 times from 41.0 s to 71.5 s). The audio context was
`suspended` until the first click, so this is the sound that a person who clicks hears. The manual checks stay open.
Pixel numbers: `ref/pixdiff-5.txt` (a probe in the scratch folder, `tests/browser/harness/png.js`).

One thing the first attempts showed: `scripts/mp1-play.sh` (L0-DBG) waited for `<profile>/DevToolsActivePort`, which
Chrome writes only for port 0, so it failed on every start with a port that it picked itself; and its cleanup returned 1
for 4 stops in 12 when Chrome still wrote into the profile. Both are fixed in this task, with a test that starts
Chrome through the script.
