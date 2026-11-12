VERDICT=FAIL HIGH=1

# Review of Q-COLD (branch `mp1/q-cold`, head a406473f): the person-readiness instrument

Seat: `ux-designer`, review mode, 2026-11-12. Worktree `/home/julien/Projects/TuRTS-mp1-q-cold-review` (detached at
a406473f). Scratch: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/q-cold-review/`
(`R/` below); builder scratch `.../scratchpad/q-cold/` (`Q/` below). Not committed.

Summary: the instrument gives the right verdict for the right reason on the reference build, with no agent. I ran the
first-click probe again myself, and it is red for the same reason. One High: check C5 ("run once for each tool
version") cannot be run again or verified from the repository. The rest are gaps: a person can still get stuck on a
build that gets `MACHINE-PASS`. I propose a small set of new machine rules, each with a red case.

## Findings

| # | Sev | Where | Defect | Fix |
| --- | --- | --- | --- | --- |
| H1 | High | `docs/qa/cold-play.md:190-196`, `Q/final-fc/session.json` | C5 says "run once for each tool version" (checker text, `scripts/qa/cold_play_check.py:74` list, C5 TODO). But `session.json` records no tool commit or tool hash (keys: `session_version`, `mode`, ..., `build`, `chrome`, `lock_marker`, `node`). The reference build exists only as `ref-build/SHA256SUMS` in a `/tmp` scratch folder. I could repeat the red only because the operator worktree `TuRTS-mc-play` still exists. When `/tmp` is cleared, the instrument loses its proof that it can go red. | Write the tool commit (`git rev-parse HEAD` of the lane) and the sha256 of `cold-probe.js`, `cold-pixels.js`, `cold-session.js` and `cold_play_check.py` into `session.json`. Make the checker print them and fail C5 when the control folder and the probe folder have different tool hashes. Add the commands that rebuild the reference to the document: a worktree at MC `9b3a7bd`, the build, the server and static start on new ports, `--url`. |
| M1 | Medium | `scripts/qa/cold_play_check.py:186-194` | F9 counts words per widget, so a long text that is split into small widgets passes. Probe (`R/split-f9`): I split the 189-word `guide_copy` of the reference record into 38-word widgets, and F9 then gives PASS. | Add F9b: the total words of all visible text widgets after the first click (focus overlay and world labels left out) is at most 60. Red case: the split record above. |
| M2 | Medium | `tests/browser/cold-probe.js:183-185` | The "visible change" is true for any 2,000 px change anywhere on the page. In the builder's own reference evidence, clicks 5 and 8 count as visible (`Q/final-fc/probe-first-click.json`: `box_changed 0`, `page_changed 9332`): an alert line that appeared on its own. P1 still fails there on the selection. But on a build that selects with no mark and no panel, an unrelated change can pass P1. | The page part counts only when the UI record shows a widget whose text holds the clicked unit's name, or a widget that changes inside 200 ms. Make the box scale with the unit: half side = max(48, 1.5 x the projected `pick_radius`). |
| M3 | Medium | `tests/browser/cold-probe.js:209-249` | The buttons probe clicks only the buttons that show with no selection, so it never tests the command card. That is the place of the CEO's complaint and of the cold agent's finding "Disable shows nothing". | Add a command-card pass to the first-click probe (rule R3 below). |
| M4 | Medium | forecast for MP1 | The buttons probe makes no start click, so the first visible button meets "Click to play" (`client_shell.rs:868-897`, full-screen block). On every build where a HUD button shows under that overlay, F8 is red: by design of the shell, not of the mod. On `frontier` (Command Centre selected, build menu) this can block C1. The defect is real for a person: the CEO record lost the first click on "Close". | Needs decision (owner `L-LOBBY-C1`, the client phase): (a) the start overlay passes the gesture click to the control under it; (b) the overlay is opaque and no button shows under it; (c) F8 starts after the start click and the lost click is recorded as a separate finding. Recommended: (a). |
| L1 | Low | `docs/qa/cold-play.md:186` against `:99-104`, `:162` | `:186` says "F1, F2, F3, F4, F6 to F10 need a person"; `:162` and the table say F3, F6, F8, F9 are machine checks. | Delete `:186` or correct it to "F1, F4, F7, F10". |
| L2 | Low | `docs/qa/cold-play.md:92` against `:240` | `:240` says "the 1 % page rule is gone". That is true for the first-click probe. The buttons probe still uses `PAGE_SHARE = 0.01` (`cold-probe.js:35`, `:247`). | Say which probe has which rule. |

## Run table

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| `python3 -I -B scripts/qa/cold_play_check.py Q/final-fc --also Q/final-btn` (`R/check-ref.txt`) | The reference red is NOT-READY for P1, F9, F3, F6, F8 (`cold-play.md:231`) | MACHINE-PASS, or other checks red | exit 1, `NOT-READY (failed: P1, F9, F3, F6 [first-click]; F8, F9, F3, F6 [buttons]; missing: agent session)` |
| `git diff --stat 2688aa6e a406473f` | The evidence (tool 2688aa6e) is valid for the head | a change in `cold-probe.js`, `cold-pixels.js`, `cold-session.js`, `cold_play_check.py` | only tests, fixture, plants, document and the sound-isolation files changed |
| `mc_headed_v6.sh --mode pre-merge ... --label uxrev-qcold-fc-1 -- bash R/held-ref.sh fc1 --probe first-click` (my own 9b3a7bd forge server on new ports; stopped by its pids) | The red can be repeated, for the same reason | P1 PASS, or a FAIL for another reason | P1 FAIL 14 of 14, "selection [], no visible change"; F9 189 words; F3 15 %; F6 2 lines; hold 46 s, receipt ok (`R/check-fc1.txt`) |
| same, `--control close-layer --viewport 1024x768` (label `uxrev-qcold-ctl1024-1`) | A control placed by a fixed offset leaves the page at another size (`hud.ron:125-126`, offset 1176) | CONTROL-PASS at 1024 | `CONTROL-FAIL`: `layer_close {found: false}`; "Close" is not in the 1024 px page (`R/sess-ctl1024/shots/0001-click-p1000.png`); a person at this size has no mouse way out; hold receipt ok; servers stopped by pid (`R/srv-ctl1024/pids.txt`) |
| reading `Q/final-fc/shots/0002-click-p200.png` and `Q/final-ctl/shots/0003-click-p200.png` | Probe fail = the guide takes the click; control pass = a real panel | the pictures disagree with the record | they agree: the guide over the map; then "Selected: Bore Drill" and the Orders panel |
| `R/split-f9`: the reference record with `guide_copy` cut into 38-word widgets, then the checker with `--partial` | F9 sees "a text block of more than 40 words" (ruling item 2) | F9 FAIL | F9 PASS (M1) |
| `python3 -I -B tests/qa/test_cold_play_check.py` | The checker tests run green | a red test | 53 tests OK |
| checker on `Q/final-muster-fc --also Q/final-muster-btn` (`R/check-muster.txt`) and `Q/final-muster-fc/shots/0001-click-p1000.png` | muster: P1 PASS, F9 FAIL on a 66-word guide at the join | other result | confirmed; section 7 |

## 1. The right verdict for the right reason (question 1)

Yes. The probe clicks the centre of "Click to play" at 9.5 s (`Q/final-fc/inputs.jsonl` line 1). The guide stays. Each
building click gives `selection []` and 0 changed pixels in the box. My repeat run gives the same result. The control
closes the guide with one click and then passes 14 of 14. So the probe can pass, and the red comes from the layer. F9,
F3 and F6 are true facts of the screen (189 words; a full-page layer with 15 % see-through; 1 to 3 alert lines before
the first input). No agent answer enters the verdict (`cold_play_check.py` has no input for it).

## 2. What a first-time person meets that MACHINE-PASS does not see (question 2)

`MACHINE-PASS (manual checks open)` is honest: it never says PERSON-READY. But the cases below are common, and most of
them can be machine checks with the bridges and the UI record that exist today.

| # | Stuck case | Machine check possible today? | Rule (FAIL when) and red case |
| --- | --- | --- | --- |
| R1 | A goal stated nowhere (F1) | Partly. No generic "goal" role exists in the interface schema. | The readiness request gives the goal text of the design (frontier: "Destroy the enemy Command Centre", D 7.8). FAIL when no visible widget after the first click holds that text, or the text has more than 15 words. Red: forge `9b3a7bd` (no such text). |
| R2 | An action that exists only as a hotkey (F4) | Yes | After each own-unit selection of the probe, FAIL when an order of the package for that content id has no visible enabled button (`turts_ui` button `widget_action` id against the order list). Red: forge with `command_card` removed from `hud.ron:67-76`. |
| R3 | A button that does nothing visible (cold-agent finding "Disable shows nothing"; F8 for the command card) | Yes | After each selection, click each visible command-card button one time. FAIL when no UI-record change other than its own tooltip shows within 1 s and no notice shows. Red: forge press Disable (my earlier record `first-click/run-2a/A1280-17-after-disable.png`: only the tooltip changes). |
| R4 | A refused order with no reason, or no response to a right click (F7) | Yes | With each own unit type selected, right-click open ground. FAIL when, within 1 s, there is no accepted order with a visible mark and no new notice text (`turts_debug.notices`, a text change in the UI record). Red: forge with `last_notice` removed (`hud.ron:95-100`). |
| R5 | A stale text (cold-agent finding) | Yes | FAIL when a notice text stays visible more than 8 s after its cause with no new cause. 8 s is the shell's own notice time (`client_shell.rs:904`, `SOUND_NOTICE_SECS`). Red: forge, where "Cannot move. It is fixed in place." is still on screen 25 s later (`first-click/run-2a/A1280-18-idle-20s.png`). |
| R6 | A control outside the window at another size | Yes | Run the probes at 1280x720 and 1920x1080 (`--viewport`). FAIL when a visible button rect is outside the viewport, or the control fails at either size. Red, run: forge `9b3a7bd` at 1024x768: the control finds no "Close" (`R/sess-ctl1024`, `CONTROL-FAIL`), because its fixed offset of 1176 px (`hud.ron:125-126`) is off the page. |
| R7 | A tooltip over a button (cold-agent finding) | Yes | FAIL when a visible `::tooltip` rect covers more than 25 % of another visible enabled button. Red: forge hover on Disable covers Enable (`first-click/run-2a/A1280-17-after-disable.png`). |
| R8 | A cursor that lies (CEO record, condition M6) | Yes | At each probe click, FAIL when `turts_debug.cursor.context` is `select` or an order context and the click gives no selection. Red: forge `9b3a7bd` (`first-click/run-1/03-click-drill.debug.json`: context `select`, selection `[]`). |
| R9 | Text too small | Not with the record today: the UI snapshot has no font size. | Add `font_size` to the snapshot. FAIL when body text is smaller than 12 px at 1280x720. Reason: XAG 101 asks for at least 18 px for PC at 1080p, which is 12 px at 720p. Red: none of the forge texts at size 12 fails; a plant at size 9. Until then: manual. |
| R10 | P1 passes, but nothing says what to do next (F10) | Proxy only | FAIL when, after the probe, no own unit type offers a visible enabled button and no hint or goal text is visible. The true F10 (30 s with no way forward) stays manual. |
| R11 | Too much text in total | Yes | F9b of finding M1. |

Your manual list: F2 is already a machine check (P1). F4 (R2) and F7 (R4) can be machine checks today. F1 can be a
machine check only with the declared goal text (R1). F10 stays manual (R10 is a proxy).

Smallest set to add now: F9b (M1), R3 (command-card buttons), R4 (right click), R8 (cursor), R6 (a second viewport),
and the click rule of section 3. They use only the bridges that the probe already reads.

## 3. Clicks on things that are not the player's own (question 3)

Ruling: yes. The first-click probe adds four clicks after the own units, each with a required visible response:

1. One enemy unit: the name and the owner ("Other player"), or the enemy panel. Selecting it is allowed.
2. One neutral object (a resource node): its name, as a panel or a tooltip, within 1 s.
3. One left click on open ground after an own selection: the selection becomes empty, and the selection panel hides
   or changes.
4. One right click on open ground with an own unit selected: rule R4.

"Open ground" is a point inside the map, with no unit or node within 2 x `pick_radius`, and under no widget of the UI
record. Reason: these are the first clicks of a person who does not know which things are theirs. The CEO clicked
"every building". With the guide closed, forge gives the enemy panel (`hud.ron:31-40`) and a reason for the right click.
For the ore vein, a response is NOT TRACED; if there is none, that is a real finding.

## 4. The numbers (question 4)

| Number | Rule now | Ruling | Reason |
| --- | --- | --- | --- |
| 8 s settle | `cold-probe.js:30` | Keep 8 s for the first click. For F6, record the UI each 1 s for 20 s before the first click. | A layer that opens at the join is present at any time, so 8 s is enough for P1. But ALR2 forbids an alert line at any time before the first input (D:2475), and F6 now depends on the time (1 to 3 lines). 20 s: at the slow end of the adult reading rate (175 wpm, Brysbaert 2019), a 40-word block takes about 14 s, plus some seconds to look around. |
| 96 px box | `BOX_HALF = 48` (`:33`) | max(48 px, 1.5 x the projected `pick_radius`) | The selection ring is drawn at the footprint radius (rings radius 1.2 = footprint in the CEO record). On forge, 1 world unit is about 17 px (drill x 519.6, arm loader x 553.6, 2 units apart), so the ring is at about 20 px and 48 is enough. A Command Centre of radius 2.5 at a closer camera can put the ring outside a fixed 96 px box. |
| 5 % see-through | `SEE_THROUGH_SHARE` (`cold_play_check.py:65`) | Provisional. Replace it with the fill alpha of the layer (alpha below 1.0 = see-through). | Nothing supports 5 % except that it is 3 times below the 15 % of the reference. On a dark or flat map a see-through layer can measure below 5 % (the builder's own NOT TRACED). The engine knows the alpha (`hud.ron:123`, `#07111ed0`); `turts_ui` does not export it. Add it to the snapshot (owner of the snapshot: `L0-DBG`). |
| 40 words | `MAX_WORDS` (`:64`) | Keep 40 for one block. Add 60 words in total (F9b). Keep the goal line at 15 words or fewer (R1). | Brysbaert 2019: adults read 238 wpm (range 175 to 300) silent non-fiction. 8.3 point 1 asks for the goal within 5 s: at 175 wpm that is about 15 words. 40 words take 10 to 14 s, a fifth of the first minute. 60 words in total is about 20 s at the slow rate, a third of the minute; the player must use the rest to act. Whether frontier's first frame stays under 60 is NOT TRACED. |

## 5. The agent session (question 5)

It is worth a short session: it found three real faults that no probe saw (Disable shows nothing; a stale refusal
text; a tooltip over a button). It does not give a verdict. R3, R5 and R7 now turn those three into machine rules, so
the session's value is to find the next kind. Limit the session to 300 s (the faults came in the first 60 s). Fixed
form of its report, so that builds can be compared:

    STATEMENT: the six lines (as now)
    FINDING n: CLASS | SHOT | INPUT | EXPECTED | SEEN
      CLASS one of: no-response, wrong-response, stale-text, hidden-control, covered-control, unclear-text,
                    too-much-text, sound, other
      SHOT the screenshot file; INPUT the call that caused it; EXPECTED what a person expects, in 12 words or fewer;
      SEEN what the screenshots show, in 12 words or fewer
    STUCK: the call number and shot where the agent did not know what to do, or "none"
    NO ANSWER LINE (the checker ignores any yes or no)

The reviewer compares FINDING rows by CLASS and widget text between builds. A CLASS that comes again on a new build
becomes a candidate machine rule.

## 6. Can a seat run the check from `docs/qa/cold-play.md` alone (question 6)

No. Missing:

1. The reference build (H1): the commands to rebuild MC `9b3a7bd` forge and to serve it on new ports. A tool version
   in `session.json`.
2. A readiness run as one numbered list. Today it is spread over `:9-10`, `:75-95`, `:111-127` and `:146-151`: which
   probes, at which viewports, on fresh servers, then the checker with three folders.
3. How to do the manual checks: C2 (match each point to the earlier screenshot), C3m, C4, with a small template.
4. The agent report form of section 5.
5. What to do with M4 (F8 and "Click to play") until it is ruled.
6. The fixes of L1 and L2.

## 7. MP1: the 66-word guide on `muster` (question 7)

Confirmed. In `Q/final-muster-fc/ui-records.json` the widget `guide` (rect 8, 602, 360, 110) holds 66 words in every
record (before the first input, +200 ms, +1000 ms). The screenshot `Q/final-muster-fc/shots/0001-click-p1000.png`
shows "Welcome / Move your units with M or a right click ..." at the bottom left. The text is cut at the lower edge of
its 110 px box ("to the depot."). The cause is the engine rule that opens the guide on every first join
(`crates/turts_client/src/widgets/guide.rs:13-19`; muster `hud.ron:10`, `visible: [GuideOpen]`). That is condition M2.
Owner: `UI-HINT`, by the orchestrator ruling (`docs/research/mp1/errata-pending-6.3.md:222`: "M2 `UI-HINT` (design
D:1995 ... now `UI-HINT`)"). The red build for its test is today's `observe_phase`. `muster` P1 PASS 60 of 60 shows
that the probe goes green where the world takes clicks.

## Not checked

- I did not run the node test files (`cold-probe.test.js`, `cold-play.test.js`): they start Chrome. The builder reports
  320 tests and 0 fails.
- The plants file (`docs/research/mp1/plants/q-cold.json`, 65 records): not run.
- The sound isolation files: read in the builder report only.
- The agent session tool (`cold-session.js` reply forms, the statement gate): not exercised.

## NOT TRACED

- A response for a click on the forge ore vein.
- Whether frontier's first frame stays under 60 words in total.
- The window size of a person.

## What I changed

None in the repository. Scratch only, plus this report file (not committed). npm modules installed in the review
worktree `tests/browser/node_modules` (git-ignored; `git status` clean).

Sources: [Brysbaert 2019, reading-rate meta-analysis](https://www.Gwern.net/doc/psychology/linguistics/2019-brysbaert.pdf);
[Xbox Accessibility Guideline 101](https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/101).
