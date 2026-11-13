# Review of task E-GPU (candidate 466ed01, branch mp1/e-gpu)

VERDICT=PASS HIGH=0

Reviewer: performance-architect, review mode. Worktree `/home/julien/Projects/TuRTS-mp1-e-gpu-review` (detached at 466ed01; `git status` empty before and after). Scratchpad: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/` (`recompute.py`, `judge_probe.js`, `mut/`).
Evidence: `/home/julien/Projects/TuRTS/target/mc-evidence/egpu-run-1/runs/run-20261112T112818Z-FwOruI/`.

## Findings

1. MEDIUM. `docs/measurements/mp1.md:96`. The run has no single-window control at the same scene (515 units). So the document cannot say what the second window costs. The conclusion "two windows are usable" holds. The conclusion "two windows cost X" has no data. Fix: one 120 s single-window run with the same script, or state the gap in "Not measured" (it is not listed there now).
2. MEDIUM. Scene size. The run covers 515 units. `docs/measurements/mc.md` VIS2 (5,000 units, one window) shows p50 133.3 ms, p99 199.9 ms (about 7.5 fps). At that scene a second window may go near the fail line. Stage-1 pixel rows that load the 5,000-unit scene are not covered by this PASS. Fix: say so in the conclusion for the plan.
3. LOW. `docs/measurements/mp1.md:72`. Frame figures count rAF callbacks, not presented frames. The document says "callbacks" and cross-checks with the app frame counter (0.1 % apart), but that counter also runs one step per callback, so it is not an independent check of presentation. Screenshots exist only at start and end. Add one sentence: "a callback is not proof of a presented frame."
4. LOW. Frame count 17,320 and 16,107 include 2 timestamps outside [0, 600) s each (one at -9 and one at 600000.4 in window 1; -14.1 and 600012 in window 2). Inside the period: 17,318 and 16,105. No percentile changes (see table). Cosmetic.
5. LOW. Test instrument gaps (my mutants, below): max, mean_fps and the exact 5-frame boundary are not asserted. The document reports max and mean. A wrong max or mean would not turn the unit test red. The verdict is not affected.
6. LOW. `.github/workflows/ci.yml:114`: `egpu-stats.test.js` is NOT in the node test list. Confirmed. 13 of the 20 `tests/browser/*.test.js` files are also missing from that list, so this follows an existing practice. The file has no package import (needs no `npm ci`). Cheap to add.
7. LOW. Page visibility was sampled 60 times (every 10 s) in each window. A short hidden spell between samples would not show. The flat rate and equal p50 in both windows make throttling unlikely.

## Q1. Recomputed numbers (own script, `python3 -I -B recompute.py`)

| Figure | Window 1 doc | Window 1 mine | Window 2 doc | Window 2 mine |
| --- | --- | --- | --- | --- |
| Timestamps | 17,320 | 17,320 | 16,107 | 16,107 |
| p50 ms | 33.3 | 33.3 | 33.3 | 33.3 |
| p95 ms | 50.1 | 50.1 | 50.1 | 50.1 |
| p99 ms | 83.3 | 83.3 | 83.3 | 83.3 |
| max ms | 183.3 | 183.3 | 200.1 | 200.1 |
| lowest 5 s | 15.8 (79) | 15.8 (79) | 14.0 (70) | 14.0 (70) |
| seconds with no frame | 0 | 0 | 0 | 0 |
| minute rates | match all 10 | match | match all 10 | match |
| frames per second | 12 to 43 | 12 to 43 | 11 to 39 | 11 to 39 |

All equal. The rule gives PASS on the raw data: the lowest 5 s window holds 70 frames, the fail line is under 5 frames; no second is empty; `events` is empty. The verdict is also PASS when `judge()` runs on summaries made from the raw files.

## Q2. Is the instrument honest

- `egpu-run.js:44-52`: the init script wraps `requestAnimationFrame` and pushes the callback timestamp. It measures callbacks (renderer frame start), not presented frames. The document says "callbacks" (mp1.md:72). OK, plus finding 3.
- Pattern from the raw files: every interval is within 0.17 ms of a multiple of 16.67 ms (mean deviation 0.05 ms); none is under 10 ms. Window 1: 3,162 x 1, 10,502 x 2, 3,144 x 3 periods; window 2: 1,646 / 10,112 / 3,663. So the callbacks follow a 60 Hz begin-frame clock and the page misses every second tick most of the time.
- Most likely cause from the evidence: each frame needs more than 16.7 ms of work (main thread of the wasm client, or the GPU process shared by two windows) and the clock then skips to the next tick. Evidence against Chrome occlusion or throttling: both flags that stop it are set (`mp1.md:61`), `visibility` was `visible` in all 60 samples of each window, and both windows have the same p50. Evidence against a plain Xvfb 60 Hz cap: it would give 60 fps for a light scene, and `docs/measurements/m1.md:140` shows p50 16.7 ms for a one-unit scene (other flags: `--use-angle=gl`). Against "GPU bound": nvidia-smi mean 32.7 %, max 53 % (1 Hz sample, so it does not exclude a stall inside the GPU process). `mc.md` shows 133 ms at 5,000 units in one window, so the cost scales with the scene.
- NOT TRACED: whether the limit is the wasm main thread or the shared GPU process; whether a single window at 515 units also runs at 30 fps; what `--disable-vulkan-surface` does to the frame clock. The document marks the cause NOT TRACED (mp1.md:96). Correct.
- Measurement effect: stated plainly (mp1.md:74-80): reads every 10 s, input, `bringToFront`, screenshots only at both ends; window 1 27.0 fps in round seconds against 29.6 fps otherwise; the two causes are not separated. OK. The probe adds one wrapper function per frame; its cost is listed as not measured.

## Q3. Can the verdict go red

- `node --test tests/browser/egpu-stats.test.js`: 16 pass, 0 fail. It imports only `node:test`, `node:assert/strict` and `./egpu-stats.js`: no `npm ci` needed.
- I mutated copies in the scratchpad (not the repo file; the repo file is untouched, `git status` empty). Twelve mutants, none among the document's 15:

| Mutant | Result |
| --- | --- |
| M1 percentile floor to ceil | red (1) |
| M2 starved `<` to `<=` | SURVIVED (no 5-frame boundary case) |
| M3 judge needs 1 window, not 2 | red (1) |
| M4 secondBins `<` to `<=` | red (1) |
| M5 disconnect `> 0` to `> 1` | red (1) |
| M6 window width less 1 | red (3) |
| M7 zero run never resets | red (1) |
| M8 max read from second-last | SURVIVED |
| M9 mean_fps divisor + 1 | SURVIVED |
| M10 interval from first timestamp | red (2) |
| M11 sliding min skips last window | SURVIVED (a 6 s stall at the end is still caught by the window ending at second 598; only an exact 5 s stall in the last 5 s is missed) |
| M12 `some` to `every` for starved | red (1) |

8 of 12 red; 4 survived (finding 5).
- Synthetic feeds to `judge()` (`judge_probe.js`): a 6 s stall in window 2 gives FAIL starved [false,true]; a 0.8 fps window gives FAIL; a stall in the first 6 s or the last 6 s gives FAIL; `phase_not_joined` gives FAIL count 1; `websocket_close` gives FAIL count 1; real data plus a `websocket_close` or `page_crash` in the period gives FAIL; setupError gives NO_VERDICT, not PASS.
- Not shown red on a real run (the document says so, mp1.md:127): the real disconnect path of `egpu-run.js` and a really starved window.

## Q4. Driver rule and CI

- `python3 -I -B scripts/qa/driver_rule.py tests/browser`: exit 0, no output. Role lines: `egpu-run.js` `instrument`, `egpu-stats.js` `pure`, `egpu-stats.test.js` `unit`. All three are valid roles. A `driver` role would be the strict one; `instrument` is the role of a measuring script that drives a page, which fits.
- CI: `.github/workflows/ci.yml:113-114` lists 7 test files by name; `egpu-stats.test.js` is not there. Builder is right (finding 6).

## Q5. Is the conclusion valid for the plan

Yes, for the rule as written. Two headed windows of the MC build with 515 units each ran 600 s on one RTX 3080 with no disconnect. Margin: lowest 5 s rate 14.0 fps (70 frames) against the fail line 1 fps (5 frames in 5 s): 14 times. Lowest single second: 11 frames. p99 of 83.3 ms. Mean 28.9 and 26.8 fps. So stage-1 pixel rows can use two headed windows on this machine and this GPU at the 515-unit scene.

Not covered by the run:
- Combat: about 110 s of 600 s had fire (11 bursts), and it was one-sided (one armed slot).
- Other lanes compiling: 2 cargo and 8 rustc processes at the start only (receipt `other_compilers_at_start`); their load was not measured; the machine has 28 usable CPUs.
- A third window, a person in a window, a second GPU, the hosted runner (step 2 was not run; the document says so), the 4-vCPU target, more than 515 units (finding 2), a single-window control (finding 1).
- The fail line of 1 fps is very weak for a pixel row that needs timed input; the numbers (min 11 frames in a second) are the useful figure.

## What I ran

| Command | Claim tested | Would refute | Observed |
| --- | --- | --- | --- |
| `python3 -I -B recompute.py <run>` | mp1.md:63-72 figures | any figure differing | all equal |
| `node --test tests/browser/egpu-stats.test.js` | 16 cases pass | any red | 16 pass |
| 12 mutants on copies | the unit test can fail | survivor | 4 survived |
| `node judge_probe.js` | judge gives FAIL on starved and on disconnect | PASS | FAIL each time |
| `python3 -I -B scripts/qa/driver_rule.py tests/browser` | role lines obey the rule | non-zero exit | exit 0 |
| grep `.test.js` in ci.yml | egpu test not in CI list | present | absent |

## What I changed

Nothing in the repository. Only this file and the scratchpad.
