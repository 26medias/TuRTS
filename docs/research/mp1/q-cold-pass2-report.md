RESULT=PASS

# Q-COLD pass 2: the probes, the UI-record checks, the NOT-READY verdict, and the sound isolation of the test Chromes

Seat: `qa-lead`, build mode, 2026-11-12. Branch `mp1/q-cold`, head `a406473f`, pushed, not merged. `origin/mp1/integration` (54c39fdd)
is merged (68ea39a8; `git merge-base --is-ancestor` is true). The scratch folder is
`/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/q-cold/` (called `Q/` below).
Document of the tool: `docs/qa/cold-play.md` on the branch (its first paragraph says that the agent's "yes" or "no" is not the verdict, with the session
`c5-cold/session-1` as the reason, and gives the order: probe first; if it fails, stop).

## What changed (commits after pass 1, 4b5eff61)

- Merge of `origin/mp1/integration`; `cold-probe.js`, `cold-pixels.js`, `cold-fixture.js`, probes in `cold-session.js` and `cold-play.sh`, checker, tests (48114cfd and the commits that follow).
- Fixes found by real runs (each with a test): hover tooltip read as the click acting (the pointer rests on the button first); alert lines missed at 0.5 s (the probes wait 8 s for a person's first look); a name panel far from the point is a visible change (the 1 % page rule is gone); F5a is not judged in a probe (the 60 select sounds are the probe's own clicks); no input means no `inputs.jsonl`.
- Sound isolation (urgent request): `tests/browser/harness/silent-chrome.js`; the four test files that start a Chrome use it; `silent-chrome-scan.test.js` scans the test files (see below).

## 1. The probes (the tool runs them; no agent)

`scripts/cold-play.sh --probe first-click | buttons [--control close-layer]`. Each probe is a session of its own on a fresh game (a slot is permanent); the session ends when the probe ends.

- **first-click**: wait for ready, wait 8 s, one click at the centre of the text of the start control that the UI record shows (`focus_overlay`; else the page centre), then one left click at the screen centre of each own unit that is on screen (one of each content id first, at most 60), no key, no other input. Per click: selection after the click (from `turts_debug`), pixel change at +200 ms in a 96 px box and in the page, idle noise from a picture 200 ms before. PASS only when every click gives a selection that holds the unit AND a visible change; no own unit on screen is FAIL.
- **control close-layer**: after the start click the tool closes the layer by the visible button Close/Dismiss/OK, then probes. It is judged on the probe only (`CONTROL-PASS`).
- **buttons** (F8): no start click; for each visible button, top to bottom: the pointer rests 300 ms, then one click. "Acted" = the UI record changed (start overlay, tooltips and idle changes left out) or the page changed (not counted when the start overlay left on this click).
- The tool is role `instrument`; the driver rule passes (self-test 47 cases, tree exit 0). Nothing of the bridge data reaches an agent.

### Reference red (check C5) on MC `9b3a7bd` forge, headed under `mc_headed_v6.sh`, fresh server for each run (head 2688aa6e; the later commits only add test files)

| Run (label) | Folder | Result |
| --- | --- | --- |
| `qcold2-fc-5` first-click | `Q/final-fc/` | **FAIL**: first click on unit 1 (forge:drill): selection [], no visible change; 14 of 14 clicks failed |
| `qcold2-ctl-5` control | `Q/final-ctl/` | **PASS** 14 of 14 (the guide closed by its Close button) |
| `qcold2-btn-5` buttons | `Q/final-btn/` | **FAIL (F8)**: the first click on guide_close changed nothing (the start overlay took it); 3 alert lines did nothing |

Checker on `final-fc` + `final-btn` (`Q/ref/check-final-ref.txt`): `RESULT: NOT-READY (failed: P1, F9, F3, F6 [first-click]; F8, F9, F3, F6 [buttons]; missing: agent session)`. Facts: F9 `guide_copy: 189 words`; F3 `guide (area 100 %, see-through 15 %)`; F6 1 to 3 alert lines before the first input (it varies with time). Hold 33 to 37 s each.

MP1 `muster` at this head (prepared; `final-muster-fc`, `final-muster-btn`): first-click **PASS 60 of 60**; buttons PASS with "no visible button" (a pass with no content; the reason says so); F9 FAIL `guide: 66 words` (the guide opens at the join; a real finding for the UI tasks).

Earlier runs of pass 2 (`probe-fc-1/2/3`, `probe-btn-1/2/3`, `probe-muster-*`) are evidence of the three probe faults above, not of the final state.

## 2. Machine checks from the UI record (in the checker)

The session keeps a UI record (`ui-records.json`: widgets, alert lines, big layers) before the first input and after the first click (+200 and +1000 ms), in agent sessions too. It is held back like the sound records and released with them; a test pins one writer of each file.

| Check | Rule | Red on | Green on |
| --- | --- | --- | --- |
| F9 | after the first click a visible widget text of more than 40 words | ref (189 words); fixture with 41 and 90 words | 40 words; 12 words |
| F3 | after the first click a widget of more than half of the page with `capture_root` or `focus_policy_block` and at least 5 % non-flat pixels (text and a 2 px margin left out); an unmeasured layer fails | ref (15 %); fixture see-through layer | opaque fixture layer (0 %); a layer that takes no input |
| F6 | an alert line or a debug alert in the record before the first input | ref (1 to 3); fixture with 2 alerts | green fixture |
| F8 | the buttons probe, above | ref (Close); fixture with a button under the start overlay | a button that acts; a dead button with hover feedback is still "not acted"; a tooltip alone is not "acted" |
| F5b | an alert sound before the first input (as before) | agent and probe folders | |

## 3. The result line

`RESULT: NOT-READY (...)` when any machine check or probe fails or a part is missing (first-click probe, buttons probe, agent session); `RESULT: MACHINE-PASS (manual checks open)` otherwise. `--also` gives the other folders; `--partial` (tests) prints `PARTIAL-PASS`. A control folder gives `CONTROL-PASS`/`CONTROL-FAIL`. The checker has no input for the answer of an agent. Exit 0 pass, 1 NOT-READY, 2 unreadable.

## 4. The agent session is as it was

The statement, the gate and the findings are unchanged (pass 1). Its UI records join the release. Its "yes" or "no" is not read.

## Sound isolation of the Chromes that tests start (urgent request)

- `tests/browser/harness/silent-chrome.js`: `silentEnv(folder)` sets `PULSE_SERVER=unix:/nonexistent/mc-tests-no-pulse` and writes a null ALSA config (`</usr/share/alsa/alsa.conf>` then `pcm.!default { type null }`) and sets `ALSA_CONFIG_PATH` to it; `SILENT_ARGS` is `--alsa-output-device=null`. No `--mute-audio`, no change of the system sound.
- Used by `cold-probe.test.js`, `cold-play.test.js` (the real Chrome case, and the shim for `mp1-play.sh --url`, which also records the environment it saw and the test asserts it), `mp1-capture.test.js`, `harness/wait-sync.test.js`.
- `harness/silent-chrome-scan.test.js` (in the CI node list): scans every `*.test.js` under `tests/browser` for `spawn`, `execFile`, `spawnSync` whose first argument names the Chrome, for `.launch(`, and for a shim that execs the Chrome; each must show `silentEnv(` and `SILENT_ARGS`. **Red first:** run over the five Chrome starts of the tree before the change, it listed all five (cold-probe, cold-play spawn, cold-play shim, mp1-capture, wait-sync). Five plants (pulse value removed, ALSA device not null, args emptied, scan env not required, a test Chrome left bare) are red.
- `scripts/mp1-play.sh` in its form for a person is unchanged (normal sound). Under the wrapper the Chrome has the wrapper's isolation.
- Not covered: a Chrome that a helper starts out of sight of the scan (known gap, in the file header); `/proc/<pid>/environ` of Chrome cannot show the environment (Chrome rewrites it), so the test uses the shim's record.
- Servers: all reference `turts_server` and `http.server` processes were stopped one by one by the pids in `Q/ref/run-N/pids.txt` (checked afterwards: none left). Earlier hand-run headless probes (a shim loop, pass 1) are over; no Chrome of mine runs now.

## Tests and plants

- Node: `cold-probe.test.js` (18 tests, real headless Chrome on fixture pages, each with a 120 s limit), `cold-play.test.js` (23), `silent-chrome-scan.test.js` (3). All 53 browser test files and the CI node list line: 320 tests, 0 failed. CI list check passes.
- Python: `tests/qa/test_cold_play_check.py` 53 tests; `test_mc_prepare.py` 37; `completeness --self-test` 21/21; `yardstick --check` ok; `mp1_plants.py --self-test` 64/64.
- Plants `docs/research/mp1/plants/q-cold.json`: 65 records (30 + 2 of pass 1, 28 for pass 2, 5 for the sound isolation); `mp1_plants.py --check --all` exit 0 (339 records apply, 0 stale). The official runner (`--all`, a separate worktree, then removed) at 245d697e gave 64 of 65 red-restored; the one that stayed green (`probe-box-needs-too-many-pixels`: the relaxed page rule now covers the fixture) was fixed with a ring-only fixture test and a re-pinned plant, and then ran red-restored alone at a406473f.
- Reds seen first: `cold-probe.test.js` red on the missing modules before the implementation; the pixel, probe, UI, buttons, settle and verdict tests each have a plant that shows the red.

## Findings for the caller

- `muster` at this head: the guide opens at the join (66 words): F9 FAIL.
- The `focus_overlay` of the real client dims the whole page (it is see-through and blocking): it is excluded from the "button acted" test when it leaves, and from F3 only because F3 looks after the first click. A build whose first click does not remove it would be F3.
- The F6 count at the first input depends on the time of the input (1 to 3 on the reference build): the probes wait 8 s; a person's first look is not measured (NOT TRACED).
- The buttons probe passes with no visible button (muster): the reason says "nothing to click". A reviewer must read that line.
- A cap of 60 own units: a crowded map is sampled (one of each content id first), not exhaustive.
- Left in `/tmp`: a few `cold-probe-*` temporary folders from killed runs (the sandbox refused a wildcard delete; they are small).

## NOT TRACED

- The window size a person uses; the cursor shape is not in a screenshot.
- Whether a person-ready verdict needs a probe with a real mouse of the window system (the probes send browser input events over the debug port).
- The F3 threshold (5 %) on a build whose see-through layer shows little (a nearly flat map).
- The prepared-run path was run with `muster` at head 2688aa6e only.
