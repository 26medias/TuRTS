# MP1 plan revision 2 re-check: test instruments (qa-lead)

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS HIGH=1

Revision 2 (f7090aa) closes most of my gate-2 findings. One new High is open: the GPU lock has no queue rule (N1). It is a short text and wrapper repair. It must land before the first headed lane run, so I do not ask for a re-gate.

**Closure list**
- **H1: PARTIAL.** Nine of the ten pixel rows now go red through pixels in their own feature lane, before merge (table below). Two gaps stay open:
    - GOAL1's pixel plants (a), (b), (e) are still first run red in I-PIX1. Its other plants already go red with their features: (c) is ALR2 (b) in UI-ALERT, (f) and (g) are the C-UI text test (P:1015), (d) and (h) run headless in UI-HINT.
    - Each task runs only one plant red on the real build before merge (P:97, P:426). The other plants are red only on "recorded red fixtures", and the plan does not say what records them. The first real-build red of those plants is in REG2-1 (level 14). See N4.
- **H2: CLOSED.**
    - The record at P:96 is enough to re-run a plant without its author: file, exact edit, command, expected red text.
    - The runner proves its own red: Step 1 uses `todo` bodies; Step 4 plants an exit-only compare (P:585, P:588).
    - Q-PLANTS has no inputs and is an input of L0-EXIT (P:169, P:177). Records written earlier are checked at its merge.
    - Small residuals are N5 and N6.
- **H3: CLOSED** (P:1193, P:1213).
- **M1: CLOSED** for my three mutations, which are now RED. My new mutations found three more blind spots (N2).
- **M2: CLOSED** (P:773, P:788, P:307, P:424, P:1180-1182).
- **M3: PARTIAL.** The evidence set (P:1095-1103), the sheet and I-PLAY1 now exist. Nothing checks the evidence set, and the person-run capture has no traced path (N3).
- **M4: CLOSED** (P:571-573).
- **L1: PARTIAL.** F-FROZEN (P:99, P:421, P:1166), the closing task (P:1232) and the merged reviews (P:90) are in place. The joint reviews state no own fault for INC1, SUR1 or ALR2/s2 (N7).

**Row waves on revision 2** (level = topological level from `waves2.py`, counting inputs, stacked inputs and merge-after)

| Row | Feature task (level) | First pixel red (level) | Other plants: first real-build red |
| --- | --- | --- | --- |
| GHO1 | UI-PLACE 8 | UI-PLACE 8 | REG2-1 14 |
| FX1/s1 px | L-FX-PRES 8 | L-FX-PRES 8 | REG2-1 14 |
| HUD5, HUD6/s1, KEY1 | UI-HUD 9 | UI-HUD 9 (one headed run for three rows) | REG2-1 14 |
| RES1/s1 | UI-RESULT 9 | UI-RESULT 9 | REG2-1 14 |
| ALR2/s1 px, MCX1/s1 | UI-ALERT 10 | UI-ALERT 10 | REG2-1 14 |
| NOGPU1 | WEB-NOGPU 1 | WEB-NOGPU 1 | - |
| GOAL1 | UI-HINT 9 | I-PIX1 13 | REG2-1 14 |

**New findings**

**N1 (High). P:102, P:1335, P:429.** The plan says headed tasks on the critical path "get the lock first". The wrapper's last line is `exec flock -x -o …/gpu-runtime-2.lock`. That lock has no priority, no queue order and no hold limit, so nothing enforces the rule. On top of that, the slow gate runs `mp1_plants.py --all` on each merged tree, and REG2 runs pixel plants headed. A long table run can starve the short pre-merge runs.
- Repair text: "(1) A pre-merge F-HEAD run holds the lock for its rows and one plant each, at most 20 min. It builds wasm before the wrapper, and the wrapper only runs the browser. (2) Table runs (B1, B2, REG2, the pixel plants) start only when no pre-merge run waits, and they free the lock between rows. (3) The slow gate runs `mp1_plants.py --all --native-only`; pixel plants run in REG2 only. (4) The wrapper gets a hold limit (`timeout "$MC_LOCK_MAX_SECS"`) and writes the wait time and hold time to the receipt. Root reads the receipts each day."

**N2 (Medium). P:311-313.** The check script stays GREEN (exit 0) on three new mutations:
- the plant text deleted from the L-STO task step, with the block entry kept;
- the GHO1 plant labelled `lane` in place of `lane+px`, so R-A can be escaped by a label;
- the HUD5 plant cell set to "none".

Repair: derive a row's pixel status from the D Runs cell ("pixels") and require `lane+px` for it. Require each plant cell to hold one D plant label of the row. Require the task step to name `plants` and the row. Add the three mutations to `--self-test`.

**N3 (Medium). P:1057-1059, P:1129, P:550.**
- No oracle checks the evidence set. A build that runs but hides some units, or that writes an empty audio record, reaches the `player` seat with no check going red. The driver stops (red) only when its finders find nothing at all.
- Roles are clear for two files: `mp1-play-driver.js` is a driver, `mp1-play-evidence.js` is an instrument. The plan does not say how the runner loads the instrument; `mc-browser.sh` takes only a driver and an oracle.
- The capture reads `turts_debug` from the person's Chrome. The sheet's Chrome command has no remote debugging port, and the L0-DBG request form is NOT TRACED.
- Repair: add `mp1-play-oracle.js` (role `oracle`) and its unit test. It goes red when an item of 8.2 is missing, a frame is all one colour, the first frame has 0 own units, a sound comes before the first input, or `site_ms` is above 60,000. Plants: "units hidden"; "audio record empty". L0-DBG states the capture form (a debugging port in the sheet, or the client posts the record). I-PLAY1 runs `--capture` against a headed window.

**N4 (Medium). P:97.** Say how each red fixture is made. Repair text: "each fixture is the recorded output of a real plant run; a lane runs all its pixel plants in its one headed session". Or, if the fixtures are synthetic: "fixtures are synthetic; REG2-1 is the first real red of the other plants".

**N5 (Low). P:98, P:287-289.** These forward merges can stop records from applying: the slice A2b1 to A3 (`orders/`, `combat.rs`, `gather.rs`), and stage-2 PRES-MODELS and FX2 (`render.rs`, `visuals/`). Name the owner: "the MP1 owner of the forward merge re-pins it and runs `--files` red in the same merge". The L0-CORE splits are not a hazard: every client plant task descends from L0-EXIT, and the L0-DBG files are not L0-CORE files.

**N6 (Low). P:96.** The record has one `file`, so a plant that edits two files cannot be recorded. Add `files[]` and `env`/`headed` fields.

**N7 (Low). P:90.** Add "the joint reviewer runs one own fault for INC1 and one for SUR1".

**What I ran** (python with `-I -B`; the repository tree stayed clean)
- `mp1_plan_check.py <plan> <design>`: exit 0, GREEN. With `--self-test`: 13 of 13 RED, exit 0. Claim: P:313.
- Six file mutations in scratch copies: m1 (I-PIX1 without C1-FINAL), m2 (L-STO block entry deleted), m3 (I-HEAD with only L0-EXIT) each gave exit 1, RED. m4, m5, m6 (N2) each gave exit 0, GREEN.
- `waves2.py` over the task table: gave the table above. Claim: P:97.
- I read `mc_headed_v4.sh`: the lock is `flock -x` with no priority or hold limit. Claim: P:102.
- greps for `support/match.rs`, `lob1.rs`, `acceptance.sh`, the `oracle.test.js` names and "fixture": claims P:773, P:788, P:97. Seven hits name oracle tests; UI-HUD, UI-ALERT and WEB-NOGPU name none (the general rule covers them).

**Not checked:** I did not run any MP1 code; it does not exist yet. How the runner loads the instrument is NOT TRACED.

**What I changed:** none in the repository. My memory note `mp1-plan-gate2-2026-11-12.md` and its `MEMORY.md` index line are updated.

Scratch files are in `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/gate2-qa/r2/`:
- `waves2.py`
- `m1-ipix1-no-c1final.md` to `m6-hud5-lane.md` (six files) and their `.out` files
- `check.out`, `self.out`
