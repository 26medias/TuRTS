VERDICT=PASS

# Re-review of Q-COLD pass 3 (branch `mp1/q-cold`, head 91ed67ed)

Seat: `ux-designer`, review mode, 2026-11-12. Worktree `/home/julien/Projects/TuRTS-mp1-q-cold-review` (detached at
91ed67ed). Scratch: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/q-cold-review/p3/`
(`R/`). Builder evidence: `.../scratchpad/q-cold/` (`Q/`). Not committed.

H1 and M1 to M4 of `review-q-cold.md` are closed; I checked each against its red case. No new High. The P3 result on
`muster` is mostly correct, but it mixes two things: the probe must accept a visible armed state for an order that
needs a target, and the engine owes a visible press on every order button. Both are Medium items below, with an owner.

## Findings

| # | Sev | Where | Item | Fix / owner |
| --- | --- | --- | --- | --- |
| M1 | Medium | product, engine button | An order button shows no press at all. In `Q/p3e/muster-first-click/probe-first-click.json`, Stop, Hold, Attack, Gather and Rush have `box_changed 0` around the button. With the sound off by default (the sound-off branch), a click on Stop for a unit that stands still gives a person no sign that the click registered. That breaks the genre rule "a visible result within one frame". | `UI-HUD`: every order button shows a pressed state for at least 100 ms after a click, whatever the order does. |
| M2 | Medium | `tests/browser/cold-probe.js` (P3) | P3 fails an order that needs a target (Move, Attack, Gather) when it shows nothing in 1 s. The correct behaviour of such an order is to wait for the target. The fault is only when nothing on screen says that the game waits. | Rule below. P3 accepts the armed state when the UI record shows the button marked active and the cursor feed shows the order's cursor context. |
| L1 | Low | `scripts/qa/cold_play_check.py` C4 TODO text | It still says "F3, F5, F6, F8, F9 are machine-checked". It does not name P3, P4, P5 or F9b. | Update the text. |
| L2 | Low | `docs/qa/cold-play.md:169` | The rebuild command has a fixed label `qcold-ref-rebuild`. The lane runner refuses a label folder that exists (`exec_in_lane_v4.py`, `mkdir(exist_ok=False)`), so the second person who copies the command fails. | Write `qcold-ref-rebuild-<date>` and say "a new label each time". |
| L3 | Low | engine label format | An icon-only button with `hotkey_label: true` gives the text " [M]", with a leading space (`Q/p3e/.../probe-first-click.json`). | `UI-HUD`: no separator when the label is empty. |

## Run table

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| `bash scripts/cold-reference.sh verify .../q-cold-reference/9b3a7bd-forge` | the durable copy is whole | a mismatch | 5 of 5 OK; the wasm name `d9cebd42...` is the CEO's play build |
| checker `Q/p3c/fc-1280 --also fc-1024 --also btn --reference` the four `p3c` folders (`R/check-ref.txt`) | the reference is NOT-READY and C5 is green | a pass, or C5 red | NOT-READY (P1, F9 189, F9b 292, F3, F6; at 1024: P1, P5 `guide_close`; buttons: F8s); C5a, C5b, C5c PASS |
| the same, with one reference copy (`R/old-fc`) whose recorded hash of `cold_play_check.py` I changed | C5a catches another tool version | C5a PASS | `FAIL C5a ... old-fc: differs: cold_play_check.py` |
| checker `--partial` on my split-guide folder (`R/../split-f9`) | M1 closed | F9b PASS | F9 PASS, **F9b FAIL (287 words)** |
| checker on `Q/p3c/ctl-1024` | the 1024 control fails because "Close" is off the page | CONTROL-PASS | `CONTROL-FAIL`: "the close control is outside the page: ['guide_close']" |
| reading of the doc rebuild steps (`cold-play.md:163-181`) and `scripts/cold-reference.sh` (rebuild at lines 18-30) | the rebuild commands are complete | a missing step | complete (worktree, cache folder, runner, the four build steps, copy, SHA256SUMS); not run, as asked; L2 |
| `node --test cold-probe.test.js cold-play.test.js harness/silent-chrome-scan.test.js` (headless, silent env) | the tool tests are green | a fail | 61 of 61 passed, including the noise case of M2, the four P4 red cases and the 1024 P5 case |
| `python3 -I -B tests/qa/test_cold_play_check.py` | the checker tests are green | a fail | OK, exit 0 |
| reading `scripts/mp1-play.sh:72`, `:82-86`; `Q/p3e/muster-first-click/internal/play.log`; `Q/p3d/ctl/internal/play.log` | the Chrome gets the null device | no `--alsa-output-device=null` | the code sets it in the silent form; both recorded commands show `SOUND: silent` and `--alsa-output-device=null`, and no `--mute-audio` |
| `git diff --stat 47b2bc09 HEAD --` the four tool files | the muster P3 evidence (tool 47b2bc09) is valid at the head | a change | no change |

No headed run: every question was answered from the evidence.

## 1. H1 (question 1)

Closed. `session.json` records `tool.commit`, `dirty` and the file hashes. C5a goes red on a folder from another tool
version. The durable copy verifies. The rebuild steps are complete apart from L2. C5b pairs the 1280 probe and its
control. The 1024 control fails by design (P5), and the checker shows this as P5, not as a C5 fault.

## 2. M1 to M4 (question 2)

- **M1 (split text):** red by F9b.
- **M2 (noise counted as a response):** the page rule is gone from both probes. The fixture case `noise` (a block and
  an alert line that appear 100 ms after a click) is in the node tests. The reference evidence has no click with a page
  change and no box change, so the fixture case is the only red case.
- **M3 (order buttons):** P3 exists, and it is red on the reference control (Disable shows nothing: the cold agent's
  finding).
- **M4 (lost first click):** `F8s` is red on the reference ("the start overlay took it"). The product item is open for
  `L-LOBBY-C1` and `UI-HUD` (`cold-play.md:240-244`).
- **1024 control:** red as in my review.

## 3. The muster P3 result (question 3)

The evidence (`Q/p3e/muster-first-click`, tool files equal to the head, screenshot `shots/0076-move-p1000.png`): the
card is a grid of 64x64 icon buttons with real images (`mods/muster/assets/images/icon_*.png`), and the hotkey shows in
the corner.

- **Empty labels:** the label is empty because the mod chose icon-only buttons (`mods/muster/interface/hud.ron:4`,
  `label: None`). That is a valid D67 layout and common in the genre. It is not a fault, except the leading space (L3).
  The muster tooltip shows `Item(Reason)`, not the order name, so a person who hovers an icon does not learn its name.
  That is muster content (an MC test package), not the MP1 line.
- **Real faults:**
  - No order button shows a press (M1).
  - An order that needs a target gives no visible armed state in the record. The cursor changes, but the record and
    the screenshots do not show it.
  - Stop and Hold on a unit that does not move change nothing. That is correct for the world, but it needs the press
    feedback of M1.

**Rule (one sentence):** within 1 frame of a click on an order button, the button shows a pressed state; within 1 s
either the world, a panel or a notice changes, or, for an order that needs a target, the button stays marked active
and the cursor shows the order's cursor until the target click or Esc. Owner: `UI-HUD` (the card states, row HUD5).
The probe then accepts "armed" as a result (M2).

## 4. A pass with nothing to click (question 4)

Accept it as a pass, with the reason line. A play screen with no button before a selection is a valid layout (muster
shows its card only after a selection). P3 already fails a build where a selected unit offers no button. Add one rule:
if the buttons probe finds no button AND P3 finds no order button for any own unit type, the result is FAIL ("no
on-screen control at all"). That is the R10 proxy of my review.

## 5. The document (question 5)

Yes, with L1 and L2 fixed. A `player` seat and a reviewer can now run a readiness check from `docs/qa/cold-play.md`
alone:

- the run as one numbered list (`:14-35`);
- the reference copy and its rebuild (`:156-195`);
- the sound rule (`:197-207`);
- the checker (`:209-226`);
- a method for each manual check (`:228-236`);
- the open product items and the owed items (`:238-255`).

## 6. Tests and sound (question 6)

The tool's Chrome has the null device in the code and in two recorded sessions. The builder's silence proof
(`Q/p3d/silence.log`) is consistent with that. I did not repeat `pw-dump`, because I made no headed run.

## Tests

- Node (`cold-probe.test.js`, `cold-play.test.js`, `harness/silent-chrome-scan.test.js`): 61 tests, 61 passed, 0 failed (`R/node-tests.out`). This includes "a change elsewhere on the page does not make a click seen: a block and an alert line that appear by themselves are noise", the four P4 red cases and the 1024 P5 case.
- Python (`tests/qa/test_cold_play_check.py`): OK, exit 0 (`R/py-tests.out`).
- After the runs, no Chrome of the tests is left (`pgrep` 0).

## NOT TRACED

- Whether muster's cursor changes for Move: the cursor is not in the screenshots, and the probe does not read it.
- The node tests that run a real Chrome ran headless with the silent environment, outside the wrapper.

## What I changed

None in the repository.
