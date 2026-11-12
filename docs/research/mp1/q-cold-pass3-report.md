RESULT=PASS

# Q-COLD pass 3: the reference made durable, the probes made strict, and the sound fix

Seat: `qa-lead`, build mode, 2026-11-12. Branch `mp1/q-cold`, head `91ed67ed` (pushed, not merged). `origin/mp1/integration`
(54c39fdd) is an ancestor of the head. Scratch folder (called `Q/`):
`/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/q-cold/`. Document of the tool:
`docs/qa/cold-play.md` on the branch (rewritten: a readiness run as one numbered list, the reference commands, the probes, the
numbers with their reasons, the report form, the manual-check methods, the open product items, the owed items).

## 0. The beeps (the stop order, done first)

- **Stopped and checked:** when the order came no session of mine was running or queued (the last one had ended); `pgrep` for
  `user-data-dir=/tmp/turts-play-profile` and for the wrapper tickets of `qcold*` found nothing. My servers were stopped by their pid files.
- **Cause (confirmed in the code):** `scripts/mp1-play.sh` started its Chrome with neither `--alsa-output-device=null` nor any sound
  environment of its own. `cold-play.sh` called it with `--url`; the wrapper's environment alone did not reach the audio process.
  The L0-DBG script never had a silent form, and my own test of it set the sound variables in the TEST, not in the script.
- **Fix:** `mp1-play.sh` has `--silent` and `--with-sound`. The silent form gives the Chrome the **argument**
  `--alsa-output-device=null` and the **environment** `PULSE_SERVER=unix:/nonexistent/mc-tests-no-pulse` plus a null ALSA
  configuration (written next to the profile, removed at the end). Silent is the default under the wrapper (`MC_LOCK_RUN`), with `--url`,
  and with `--silent`; `cold-play.sh` always passes `--silent`. The person form (`--prepared DIR` at a real desktop, or `--with-sound`)
  keeps the normal sound. No `--mute-audio`; no system sound change.
- **Tests (red first):** the scan of the shell scripts (`harness/silent-chrome-scan.test.js`) was run over the old `mp1-play.sh`: 3 problems
  (no argument, no pulse variable, no null configuration). `cold-play.test.js` starts the script with a shim for the Chrome that records
  the arguments and the environment that the SCRIPT gave (the test passes no sound variable): silent by default, with `--silent`, and
  `--with-sound` as the person form; a test that `cold-play.sh` passes `--silent`; `--print-plan` shows `SOUND:` and the arguments.
  Seven plants are red (argument, environment, `--url` default, wrapper default, person form silenced, `--silent` dropped, scan blind).
- **Silence proof** (`Q/p3d/silence.log`, `Q/p3d/silence.py`): one control session on the reference build under the wrapper, and each 3 s a
  read-only `pw-dump` and `ps`. 39 samples with the session's Chrome alive (17 processes; the browser process shows
  `--alsa-output-device=null` in `ps`); the page started 40 sounds in that time, 7 of them alert sounds (`Q/p3d/ctl/sound-records.json`).
  In no sample: a PipeWire client whose `application.name` contains "ALSA"; a client whose process id is one of the session's Chrome
  processes; an audio output stream of those processes. (16 clients all the time: the desktop's own.) I could not run the old form for a
  baseline: it would beep again.
- **Honest limit:** the runs `qcold3c-*` (before the fix) played sound on the desktop; the evidence from them is valid (the tool files did
  not change), the sound was not.

## H1. The reference red can be repeated and verified from the repository and a durable copy

- **Tool hashes.** `session.json` records `tool: { commit, dirty, files: { sha256 of cold-probe.js, cold-pixels.js, cold-session.js,
  cold-play.js, harness/png.js, cold_play_check.py } }`.
- **C5 in the checker.** `--reference FOLDER` (repeat): C5a every reference folder records the hashes of the tool files that are in this
  checkout and a clean tool tree (red when they differ: run the reference again for this tool version); C5b the first-click probe FAILS and its
  control PASSES; C5c the folders name the build `9b3a7bd`. A readiness run with no `--reference` is NOT-READY. 7 Python tests + 3 plants.
- **Durable copy.** `/home/julien/Projects/TuRTS/target/mc-evidence/q-cold-reference/9b3a7bd-forge/` (`turts_server`, `forge.tar`, `dist/`,
  `SHA256SUMS`; 51 MB). `bash scripts/cold-reference.sh verify <it>` checks it; `serve REF STATE` starts a new game (prints `URL:` and a pid file;
  stops on TERM; a test starts it twice and checks that the processes stop).
- **Rebuild verified once.** Commands (also in the document): a detached worktree at 9b3a7bd, a cache `target/mc-q-cold-ref`, and
  `exec_in_lane_v4.py --lane WT --target mc-q-cold-ref --head 9b3a7bd ... -- bash scripts/cold-reference.sh rebuild OUT` (cargo release server and
  packer, `turts-pack mods/forge`, `trunk build --release`), outside the GPU lock; 522 s with a cold cache, command exit 0 (the runner exit 2 is
  the "source changed" check: the pack and `dist/` are written into the lane). **All five results are byte-equal to the copy** (`cmp` and
  `sha256sum`): `turts_server`, `forge.tar`, the wasm, the js, `index.html`. They are equal although the worktree path differs, so no path is
  embedded. This holds for this host and rustc/cargo 1.99.1, trunk 0.22.0; another toolchain can give other bytes (then the copy stays the reference).

## M1. F9b, the goal line

F9b: the visible text in all after the first click (start overlay, tooltips and `world*` labels left out) is at most 60 words; the block limit stays 40.
The reviewer's split case (the 189-word guide cut into 38-word widgets) is red for F9b and green for F9 (`test_a_long_text_split_...`; plants). G1 records the
words of the first text that states a goal, as an INFO row, not a verdict (a test pins that it never fails). Reading speed 175 to 238 words a minute
(Brysbaert 2019): 40 words 10 to 14 s, 60 words 15 to 21 s; XAG 101 for the owed text-size item (both in the document).

## M2. A click is "seen" only near the object or in a panel

A pixel change counts only in a box of half side `max(48 px, 1.5 x the projected pick_radius)` around the object, or a change of a panel in the UI record
(alert lines, tooltips and what changes by itself left out). The page-wide 1 % rule is gone from both probes. Red case: a block and an alert line that
appear by themselves 100 ms after the click no longer make a click seen (fixture `noise`; plant). The buttons probe's "acted" is now: a UI-record change
(overlay, tooltips, idle left out), or pixels in a box around the button, or a moved camera (median shift of the unit centres, more than 3 px). The page
rule is gone there too (an alert line that centres the camera is seen by the camera rule).

## M3. The order buttons

After the extra clicks, for the first unit of each content id the probe clicks each visible enabled order button of its selection once (destructive ones last;
the pointer rests first) and needs a result within 1 s: a panel or state change, a pixel change at the unit, or a new notice. **On the reference build, after the control
closes the guide, P3 is RED** (`Q/p3c/ctl-1280/probe-first-click.json`): no visible result within 1 s for **Disable this machine [D] on forge:drill, arm_loader, press and
turret**; Enable on arm_loader, press and turret; Hold on turret; Cancel and Research on the lab. The control itself still passes (it is judged on the layer
only; the orders are shown as `INFO P3`). Fixture red cases: an order with no result (`ordersDead`), no order button at all.

## New clicks (P4), each with a red case on the fixture

One enemy unit (a panel must show), one resource node (a name as a panel or a tooltip), one left click on open ground after a selection (the selection must clear),
one right click on open ground with a unit selected (a notice, an order mark or a pixel mark within 1 s). Four tests, one for each, red with `enemyShows`,
`nodeShows`, `groundKeeps`, `rightDead` false/true; four plants. The reference control passes all four (right click: "Bore Drill: Cannot move. It is fixed in place.").

## Second window size, P5

The first-click probe and the control run at `--viewport 1280x720` and at `1024x768`; the verdict needs both (the checker names the missing size). A visible button outside
the page, or a Close-like button that the client marks as not visible because its place is outside the page, is **P5**. On the reference at 1024x768: P5 "outside:
['guide_close']" and the control is `CONTROL-FAIL` ("the close control is outside the page"), so a person at that size has no mouse way out of the guide (`Q/p3c/fc-1024`,
`ctl-1024`). The real client clips an off-page widget to `visible:false`, so the first version of P5 found nothing; the fixture now clips the same way.

## M4, F8-start

Ruling (c): the buttons probe now clicks the first visible button under the start overlay BEFORE any start click and records a lost click as its own FAIL `F8s`
("the first click on guide_close changed nothing: the start overlay took it" on the reference). F8 proper starts after the start click. The product fix (the overlay passes the
first click to the button under it, or shows nothing clickable) is written in the document as an open MP1 item for `L-LOBBY-C1` and `UI-HUD`; no product code changed.

## Numbers

8 s wait kept for the first click; F6 watches the UI each 1 s (8 s in the first-click probe, 20 s in the buttons probe) and fails on any alert line or alert sound; the first-click
probe cannot watch 20 s without moving its first click, so the 20 s lives in the buttons probe (said in the document). 40/60 words with the reading-rate reason. F3: the UI record
has no fill alpha; the export belongs to the product snapshot, so it is listed as **owed** (with the font size) and the 5 % picture rule stays, with its weakness stated.

## The agent session

Limit 300 s by default. `report FILE` after the statement, in a fixed form (`FINDING n: CLASS | SHOT | INPUT | EXPECTED | SEEN` with 9 classes, `STUCK: ...` or `none`); no yes/no line;
the checker's C6 validates the form and lists the findings; the document says how a reviewer reads it. The statement and the gate are as before.

## The document

First paragraph: the agent's answer is not the verdict, with the session reason. Sections: the run as a numbered list (probe first; stop on a fail); the parts; the probes
and the UI-record checks with numbers and reasons; the agent session and the report form; the reference and C5; the sound rule; the checker; the manual checks with a method for each of C0,
C1, C2, C3m, C4; open product items; owed items; limits; results. The L1 and L2 text defects of the review are gone (F3, F6, F8, F9 are listed as machine checks; the 1 % rule is gone
from both probes). Not in this pass, listed as owed with the reason: the cursor-truth check and the text-size check.

## Results on the reference and on muster (tool 8155550e; details in the document, section 11)

- Reference, 1280x720: first-click FAIL (14 of 14); control PASS; buttons F8s FAIL; F9 189 words, F9b 292, F3 15 %, F6 1 alert line. 1024x768: FAIL + P5; control FAIL.
  Checker with four `--reference` folders: C5a, C5b, C5c PASS; `RESULT: NOT-READY (...; missing: agent session)`.
- muster at this branch head: P1 PASS 60 of 60; P4 PASS; **P3 FAIL** (11 of 13 order buttons show nothing within 1 s, and their labels are empty: ` [M]`, ` [S]`); F9/F9b FAIL (the 66-word guide at the join);
  the buttons probe "no visible button after the start click" (a pass with no content).

## Tests and gates

- Node: 53 test files, **338 tests, 0 failed**; the CI list line: 338 tests, 0 failed; `ci-node-list` and `wait-sync-scan` pass.
- Python: checker tests 73 OK; `test_mc_prepare` OK; `driver_rule.py` self-test 47 cases and the tree exit 0; `mp1_plants.py --self-test` 64/64; **`--check --all` exit 0 (376 records, 0 stale)**;
  `completeness --self-test` 21/21; `yardstick --check` ok.
- Plants: 102 Q-COLD records. The official runner (`mp1_plants.py --all`, in a separate worktree that was then removed) gave 99 of 102 red-restored; the 3 that failed only had an expected-red text that did not follow a renamed test. The texts were corrected and the three ran red-restored alone (3 of 3). `--check --all` exit 0.
- Red first: the probe tests were written before the code (module missing); every new rule has a plant that was seen red on the commit; one plant (the page-change rule) went green after the
  M2 change and was re-pinned; two plants for the yes/no line were dropped because the "unknown line" rule makes them redundant.

## Findings and limits

- One of my own tools destroyed uncommitted edits once (the scratch plant runner restored from HEAD); it now refuses a dirty tree.
- A reference run made BEFORE a tool-file change is stale by design (C5a). The runs `p3c` were made at tool commit 8155550e; the tool files are unchanged since, so C5a passes on them.
- The 1024x768 verdict on a candidate needs a second prepared game and a second hold (about 30 to 40 s each on the reference; a full control run 2 to 4 min).
- The order probe is strict on modal orders: Move, Attack and similar orders need a target and show nothing in the UI record until it comes (muster). That is red by the ruling ("a visible result
  in the panel or state, or a reason text within 1 s"); a reviewer reads the list in `P3`. Whether a hint or cursor change should count is NOT TRACED (the cursor is not in the record).
- A pass with nothing to click ("no visible button") stays a pass; the reason line says so.
- NOT TRACED: the window size of a person; whether frontier's first frame stays under 60 words; the old form's PipeWire client was not reproduced (it would beep).
- Left in `/tmp`: a few `cold-probe-*` and `qc-*` temporary folders from killed runs; the sandbox refused a wildcard delete.
