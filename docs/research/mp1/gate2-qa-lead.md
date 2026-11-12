# MP1 plan gate 2: test instruments (qa-lead)

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=FAIL HIGH=3

Plan 24a18e1, design rev 6, slice head a6f3f66 (it descends from 85f34cb). Mode: review-plan, instrument lens. All three High findings have a text repair. I did not find a design defect. The defect is in when the tests are first run red, and in how the plants are re-run.

## Findings

H1 (High). P:82-83, P:102 (R9), P:707, P:717, P:737, P:757, P:811. Every stage-1 pixel plant is first run red in I-PIX1 (W13, offset 27-31). The feature tasks merge at W8-W9 (offset 10-24). Rows hit: GHO1 (the whole row is pixels), HUD5, HUD6/s1, KEY1, the pixel parts of ALR2/s1 and FX1/s1, the RES1/s1 reason plant, and GOAL1 plants (a), (b), (c), (e), (f), (g). The lane reviewer cannot run its own fault on these rows. So each feature merges with a test that nobody saw red.
- Repair text: "A task that writes a pixel row runs it green one time and runs one plant red in its lane, before merge. It uses the headed wrapper, 1 headed run (about +6 runs in total). It also adds `node --test <row>-oracle.test.js` with one recorded red fixture for each plant (no GPU), as MC did (`pt1-oracle.test.js`). B1 then re-runs the rows two times."

H2 (High). P:109 (R16), P:281 (F-REF), P:819, P:896, P:916. No MP1 plant is put in a registry before I2-PIX (W21).
- F-REF uses `parallel_refutations.py`. Its `--set` choices are only `ci-browser`, `ci-native` and `m3-all` (I read `--help`). So no MP1 plant is re-run when a later task edits the same file. Example: L-PWR and L-DES edit `production.rs` and `construction.rs` after L-CON1 (P:265).
- REG2-1 cannot re-run any MP1 plant. I2-PIX must re-anchor about 200 plants on the stage-2 tree inside 4 runs.
- The red builds that REG2 names are MC plants only (the camera plant and the SEL1 lead).
- Repair text: "Each task writes `docs/research/mp1/plants/<task>.json` (row, plant id, file, patch, command, expected red text, exit, commit). New W0 task `Q-PLANTS` (qa-lead) builds `scripts/qa/mp1_plants.py --files <changed>|--all` and adds an `mp1` set to the runner. MP1 F-REF runs the plants of the changed files. REG2-1 runs `--all` and names CON1 plant (1) as a second red build."

H3 (High). P:891, P:896, P:916. PRES-ART puts in 9 models. The MC unit finders filter on blob size and aspect (`tests/browser/hud-driver.js:43-57`). No task re-runs GOAL1, GHO1, FX1/s1, RES1/s1 or HUD5 on the stage-2 tree. I2-PIX runs the stage-2 rows only, and REG2-2 runs the MC rows. A grep for "calibrat" in the plan gives 0 hits.
- Repair text: "I2-PIX re-runs every stage-1 MP1 pixel row and one plant of each with models on. A finder change is a re-calibration commit. bevy-rendering-engineer reviews it with the plant red before and after. REG2-2 includes all MP1 stage-1 rows."

M1 (Medium). P:209, P:277. The plan check script cannot see the order of rows and features, or missing plants. Three mutations stayed GREEN with exit 0:
- I-PIX1 without its C1-FINAL input;
- the L-STO plant text deleted;
- I-HEAD with only L0-EXIT as input.

Repair: add a `Feeds` column, and a rule that the owner descends from each feeder (through inputs or merge-after). Add a rule that each block with rows names its D plant labels. Add the three mutations to `--self-test`.

M2 (Medium). P:573, P:588, P:882; there is no `acceptance.sh` line for any MP1 test.
- L-MATCH-SRV1 and L-LOBBY-C1 name no test file and no shared N-client helper. Today such tests are ad hoc only: a file-local `start_client` (`crates/turts_server/tests/acceptance.rs:124`) and `economy.rs:3579-3603`.
- `scripts/acceptance.sh:86-142` lists each test one by one, so the new MP1 process tests run in no automatic gate.
- Some hooks have no task: the `surrender` action (D:1231, needed by MAT1 (e) and (h)), and the report fields `lobby_refusals` and `stats_first_seen_tick` (D:1247-1248).
- Repair: L-MATCH-SRV1 creates `tests/support/match.rs` (`spawn_clients(n)`) plus `imm1.rs` and `mat1.rs`. L-LOBBY-C1 creates `lob1.rs` and `leave1.rs`. Each task that adds an `#[ignore]` test also adds its line to `acceptance.sh`. Add `surrender` and `lobby_refusals` to L-LOBBY2-UI, and `stats_first_seen_tick` to UI2-RESULT.

M3 (Medium). P:841-850, P:926-931, P:809. The C1 and PT3 evidence is not defined.
- The plan names no frames, no record of sound (the `player` agent cannot hear), and no timings. A blind check is therefore not repeatable.
- In MC, the "player harness" was the scripted `pt1-driver.js` (MC design :1191). No MP1 task builds a driver that plays to the result screen.
- The person's note records PASS or FAIL and the stop words, not what the person did.
- `mp1-play.sh` does not write an `--order-log` and a `--tick-log` for each match, which the PT3 timing script needs.
- No tool takes `turts_debug.hints` from a person's browser.
- Repair: I-PIX1 defines the evidence set: the first frame after "Click to play" in each window, each hint change, the first site, the first "+N", the result screen, the `turts_debug.audio` text, and `site_ms`. It also adds a capture step that `mp1-play.sh` runs, with logs for each match. The note adds the machine, the match length and `site_ms`.

M4 (Medium). P:426-435, P:811, P:1052.
- E-GPU has no fail condition and no decision rule.
- The hosted-runner row runs in I-PIX1, not "early" as D asks.
- Repair: "FAIL if a window shows under 1 frame/s for 5 s, or a client disconnects. On a FAIL, Root changes the B1 form before I-PIX1. Run the hosted probe in E-GPU with MC row MM1 (2 windows)."

L1 (Low/Medium). Gate sets and closing work:
- L-INC (P:613), L-PWR (P:657) and L-STO (P:687) say "gate" but do not name F-FROZEN. The stage-2 table (P:874-896) names no gate set. Add: "every turts_sim/pack/server change runs F-FROZEN".
- No task sets the `docs/qa/specs.tsv` MP1 row to `done`, or turns the coverage `in` rows into `done:`.
- No task needs a new row in `yardstick-a.tsv`.
- Reviews to merge: L-PAY with L-INC, UI2-HUD with UI2-SMART, UI2-RESULT with UI2-CONN. This saves about 3 Opus runs.
- Reviews to strengthen: the I2-PIX reviewer must run one PT3-script plant and one registered plant.

## Hook table (code at a6f3f66)

| Hook | Exists | Built in | First row that needs it |
| --- | --- | --- | --- |
| `--order-log`, `--economy-log`, `--command-script`, `--tick-log`, `--stats` | yes, `turts_server/src/config.rs:156-162` | - | - |
| `--start-now`, `--input-log`, the 3 `--stats` fields, `after_match_secs` | no | L-MATCH-SRV1 (P:576), W7 | IMM1/MAT1 W7; STO1 in L-STO W8: in time |
| `Engine` source in the order log | no | L0b-ORD W6 | STO1: in time |
| `--rejoin`, `leave`, `on:{event}`, report fields stage 1 | no (`--actions` at `options.rs:190`) | L-LOBBY-C1 W8 | LOB1/LEAVE1, same task |
| `connection` script kind, `all_disconnected` | no | L-MATCH-SRV2 (P:881) | MAT1/s2, same task |
| `ready`, `unready`, `set_colour`, `lobby_move`, `stale_revision` | no | L-LOBBY2-UI | LOB1/s2, same task |
| `surrender` action, `lobby_refusals`, `stats_first_seen_tick` | no | **no task** | MAT1 (e), LOB1 run 2, RES1/s2 |
| `mods/standoff`, `mp1-serve.sh` | no | L-MATCH-SRV1 | MAT1, LEAVE1 |
| `frame_stall_ms` | no | PRES-MEASURE W17 | VIS3 W21+ |
| N-client process harness | ad hoc only | **no task** | LOB1 run 0 (3 clients) |

## 15 rows

| Row | Owner (wave), writer / reviewer | Plant named in the task | First red run |
| --- | --- | --- | --- |
| GOAL1 | I-PIX1 W13, qa-lead / bevy | (d), (h) in UI-HINT; the rest in B1 | W13, after merge: **finding** |
| STO1/s1 | I-HEAD W13; plants in L-STO W8 | (a)-(e), test package only | W8, with the feature |
| ALR2/s1 | UI-ALERT W9, builder / ux | (a)-(i) | headless W9; pixel part W13 |
| ORDW1 | I-HEAD; cases in L0-WIRE W3 | (1)-(4); (5) only in D | W3, with the feature |
| LOB1/s1 | L-LOBBY-C1 W8, builder / web | run 0 and run 4 plants | W8, with the feature |
| LEAVE1/s1 | L-LOBBY-C1 W8 | 8 plants (P:591) | W8, with the feature |
| RES1/s1 | UI-RESULT W9 | reason plant (pixels) | W13: **finding** |
| GHO1 | UI-PLACE W8 | (a)-(e) "prepared for B1" | W13: **finding** |
| HUD5 | UI-HUD W9 | (a)-(c) | W13: **finding** |
| FX1/s1 | L-FX-PRES W8 | (a) in L-FX-WIRE W7; (b) pixels | partly W13 |
| CON1 | L-CON1 W7, builder / qa-lead | (1)-(6) | W7, with the feature |
| MAT1/s1, IMM1 | L-MATCH-SRV1 W7, builder / network | named (P:578) | W7 |
| VIS3 | MEAS-EXIT W21+ | stall plants in PRES-MEASURE W17 | W17, recorded; D-N is Proposed |
| REG2/s1 | REG2-1 W14 | camera plant (reproduced, see below); SEL1 lead | MC plants only: **H2** |
| PT3 | PT3-PEOPLE; script plants (1)-(3) in I2-PIX | yes | W21; evidence capture: **M3** |

## What I ran

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| `mp1_plan_check.py <plan> <design>` | P:209 GREEN | exit 1 | exit 0, GREEN |
| the same with `--self-test` | P:277: 6 mutations go red | one mutation GREEN | 6/6 RED, exit 0 |
| 4 mutated plan copies | the check sees the row order | a mutation stays GREEN | GOAL1 unowned: exit 1; the other three: exit 0 (blind) |
| `waves.py` (scratch) | P:555: rows red with the feature | owner before feeder | 7 rows depend on merge-after only; pixel reds at W13 |
| `driver_rule.py --self-test`, then the scan | P:285 | exit 1 | 47 cases PASS, exit 0; scan exit 0 |
| legacy-bridge row (until MP1, then MP2) in a scratch copy | the ledger stays at 0 rows | exit 0 | DR7 "exceeds limit 0", exit 1 |
| greps of the hooks in config.rs, options.rs and the tests | P:576, P:588 | hook exists | table above |
| REG2 plant on camera.rs:358 in a scratch copy | D:2460 | stays green | green 0, red 101 at camera.rs:345, restore cmp 0, green 0 |
| `parallel_refutations.py --help` | P:281 F-REF covers MP1 | an mp1 set exists | 3 MC sets only |
| `sha256sum mc_headed_v4.sh` | P:81 | a different hash | 66c105e5…cec235, the same |

## Not checked

- The MC player harness for C1 (NOT TRACED beyond MC design :1191).
- Whether a `runs: ci` row needs the GPU lock (NOT TRACED).
- The plant count of about 200 is the orchestrator's estimate; I did not count it.
- I did not compile or run any MP1 code; it does not exist yet.

## What I changed

None in the repository. Memory only: `/home/julien/Projects/TuRTS/.claude/agent-memory/qa-lead/mp1-plan-gate2-2026-11-12.md`, plus one line in `MEMORY.md`.

Scratch files, all under `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/gate2-qa/`:
- `waves.py`, `waves.out`
- `m1-goal1-unowned.md`, `m2-ipix1-no-c1final.md`, `m3-sto-no-plants.md`, `m4-ihead-no-stohead.md` and their `.out` files
- `reg2-red.out`, `reg2-green1.out`, `reg2-green2.out`
- `check.out`, `selftest.out`

## Open questions for the orchestrator

1. Does the CEO accept one headed run before merge for each client task (H1, about +6 headed runs)?
2. Who owns `Q-PLANTS` and the `mp1` runner set (H2)?
3. Is the C1 `player` run meant as blind play or as a judge of a scripted driver?
