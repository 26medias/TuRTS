# Cold play: the step tool for the person-readiness protocol (Q-COLD)

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
| `scripts/qa/cold_play_check.py` | The checker for the reviewer. No browser. |
| `tests/browser/cold-play.test.js`, `tests/qa/test_cold_play_check.py` | The tests. Plants: `docs/research/mp1/plants/q-cold.json`. |

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

## Run a session (the orchestrator)

1. Outside the lock: `(cd tests/browser && npm ci)`, then `bash scripts/mp1-play.sh --prepare PREP --package mods/<package>`
   for a game of this tree (a prepared folder is stale after any new commit: `docs/qa/prepared-runs.md`).
2. Under the headed wrapper, in the background. `SESSION` must not exist and must be outside the git tree of the lane:

       /home/julien/Projects/TuRTS/target/mc-evidence/gate-runner/mc_headed_v6.sh --mode pre-merge \
         --lane LANE --target mc-NAME --head HEAD --label qcold-LABEL --audio -- \
         bash scripts/cold-play.sh --session SESSION --prepared PREP --package mods/PACKAGE --build "text"

   For a game that another command serves (an old build, for example), use `--url URL` (a loopback address) in place of
   `--prepared`.
3. Wait for `SESSION/internal/control.json`. Then start the agent.
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

## Check a session (the reviewer)

    python3 -I -B scripts/qa/cold_play_check.py SESSION [--receipt <label dir>/lock-receipt.json] [--json]

Exit 0 when every machine check passes, 1 when one fails, 2 when the folder cannot be read. The output has two lists.

Machine-checked: E1 the event log is complete and in order; E2 every screenshot file is there with its recorded sha256,
and every call has its +200 ms and +1000 ms pictures, taken in time; L1 the session limit (at most 1500 s, ended
cleanly, on time); L2 the lock marker of the wrapper (and with `--receipt` the receipt: status `ok`, hold inside its
limit); C2a every `key` names a key (the keys are listed with the last screenshot before each, for the manual match);
C3 the statement exists, was stored before the sound records were released, was stored 60 s or more after the first
click, with no input between the 60 s mark and the statement, has the six parts, none of them "do not know" (or
`unknown`, `none`, `not sure`, ...), and each part names a screenshot of the session that was taken before the
statement; S1 the sound records are final and the debug bridge answered; F5a no sound id more than 3 times in 60 s;
F5b no alert sound before the first input.

Manual (listed as `TODO`; the tool does not check them): C0 the prompt; C1 no repository file read; C2 each point and
key was seen in an earlier screenshot; C3m the three parts are true to the screenshots that they name; C4 F1 to F4 and
F6 to F10 (F6 is the alert line; F5b is the alert sound); C5 the reference red. The checker never says PERSON-READY.

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
