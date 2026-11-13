# MP1 design revision 5 re-check: rigour

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=FAIL HIGH=1

Rigour re-check of design revision 5 (head 12bca5b). Mode: review-spec. What I changed: none. I read the diff faf70c5..12bca5b first. Design lines below are revision 5 lines. The repair is real: 10 of my 12 findings are closed, and the stage lists, key lists and budget arithmetic now agree. One High stays open, and its fix is one sentence in GOAL1.

## My revision-4 findings

| Finding | Status | Evidence |
| --- | --- | --- |
| H1 (`queue_full` had two texts) | CLOSED | :466-471 adds `construction_queue_full`; CON1 plant (6) and PRD1 plant (d) test it |
| H2 (`Immediate` timing) | CLOSED | `AllSeated(0)` at :814-830; GOAL1 now measures from the start tick, with plant (e); LOB1 run 0 |
| M1 (stage-1 hints name stage-2 actions) | PARTIAL | The stage-1 hint data has no H3 (:1990), and the production cancel is in stage 1 (:648-651, PAY2). Plant (f), the test of this repair, depends on the hint driver (new High N-H1) |
| M2 (two lobby reload rules) | CLOSED | :962, :1037 and :1909 give one rule; LOB1 run 4 has a plant |
| M3 (PT3 plants missed `site_ms`) | CLOSED | PT3 plants (1) to (3) check `site_ms` against `--order-log` |
| M4 (stage lists disagree) | CLOSED | My script finds no stage mismatch in sections 4, 10 and 11. The budget sums are correct (62 to 106, 39 to 66, 132 to 225) |
| M5 (lobby words) | CLOSED | Player path L3 :2914 uses the 7.6 words |
| L1 ("seat" for an agent) | PARTIAL | :29 still says "five seat inputs" |
| L2 (TGT2 range 8) | CLOSED | TGT2 :2451 now names a test package |
| L3 (reason list; draw by disconnect) | CLOSED | :1934 and 6.8 |
| L4 (ignored test split) | CLOSED | :2496 |
| L5 (PE04 note) | CLOSED | :2896 |

## High findings of the other lenses

| Finding | Status | What covers it |
| --- | --- | --- |
| player N1 | PARTIAL | The hint data repair is real. Its red is GOAL1 plant (f), which needs the hint driver (N-H1) |
| player N2 | CLOSED | STO1 run 3 with plant (e); RLY1 plant (2); ALR2 case C with plant (e). The cited code at `transaction.rs:710-719` and `gather.rs:237-239` matches |
| player N3 | PARTIAL | Plant (g) needs the hint driver. Plant (h) is a check on the `turts_debug.hints` record, so it can be built |
| mod H1 | PARTIAL | `focus`: the ALR2 click check is red on a build that ignores the field, but ALR2 names no plant for it. `Gathered`: no row can fail the rule at :1122-1123, "the free income and refunds do not count", because RES1 compares the screen with `match_result`, and both show the same count |
| net N1 | CLOSED | ORDW1 case A with plant (1), case B with plant (3). `replica.rs:403-418` and `:563` match |

## New findings

**N-H1 (High). GOAL1 asks for a hint driver that cannot be built.**
- GOAL1 :2436 says that the driver "does only what the shown hint line says … a text with no table entry stops it red". So the driver must read the text of the hint line.
- A `driver` cannot read text:
    - rule A1 bans the bridges, including `turts_debug` (`scripts/qa/driver_rule.py`);
    - the MC design says "Text: no OCR in MC. The oracle checks text through `turts_ui`" (`2026-10-29-mc-player-control-design.md:1206`);
    - the PT1 driver follows the same rule (`tests/browser/pt1-driver.js:260`).
- Open point 17 (section 15) defers the question to gate 2 as NOT TRACED. But plants (f) and (g), the tests that close player N1, player N3 and my M1, rest on this driver.
- Smallest repair, a replacement for the hint-driver sentence of GOAL1:
    - "The driver runs the stage-1 inputs of the Player path in order. It starts with a left click on open ground. It finds objects by pixels, and it reads no text."
    - "The oracle reads `turts_debug.hints`. It is red when the hint shown at a driver step is not the text of that step, or when H7 does not show within 300 s."
- With this text, plant (f) is red because the oracle sees H3, and plant (g) is red because the oracle sees a text that is not in the table. Then open point 17 closes.

**Medium**
- M-a: no row can fail the `Gathered` count rule (:1122-1123). Repair: add a RES1 case. A slot that has the free income on, and that cancels one job, has `Gathered(credits)` equal to the sum of its deliveries. Red build: the income or the refund counted.
- M-b: if the producer has a rally on a node, the rally gather comes first, and "the spawn order then never starts" (:711-712). The queued spawn order then stays in `unit_orders.queued` for the life of the unit. No text says whether a player sees it. NOT TRACED: whether today's queue accepts a queued Gather. Repair: "When the rally follow order is a gather order, the engine does not queue the spawn order."

**Low**
- 6.4 :648-651 names only the running and queued cases, but PAY2 also tests `output_blocked` in stage 1.
- GOAL1 says "X4" with no "hint" before it. The id rule at :140 asks for "hint X4".
- ALR2 has no plant for `focus`: a build that ignores the field fails the click check, but the row does not name that build.

## Which build makes each changed row red

| Row | Red build | Named? |
| --- | --- | --- |
| GOAL1 | Plant (e), measured from the start tick | yes |
| GOAL1 hint driver | Plants (f) and (g) | named, but the driver cannot be built (N-H1) |
| STO1 | Plant (e), today's idle spawn | yes |
| ALR2 | Plants (e) to (h) | yes; no plant for `focus` |
| ORDW1 | Plants (1), (3), (4) and (5) | yes |
| LOB1 | Run 0: the sim ticks, or a double fixed step. Run 4: a seat bound to the player id | yes |
| PT3 | Plants (1) to (3) | yes |

**Seams that hold**
- `AllSeated(0)` in stage 1 needs only `LobbyState` (:822-827). Lane 0 freezes every record, so no stage-1 row waits for a stage-2 record.
- `GathererIdle` reads the stage `Idle` with no current order; `GathererWaiting` reads the stage `Waiting` (`feedback.rs:821`). So both cannot fire for one hauler at the same time.
- Section 15 points 14, 15, 16 and 18 each have an owner and a closing stage, and none hides a High.

## What I ran

| Command | Result |
| --- | --- |
| `completeness.py --self-test` | exit 0, 21/21 cases |
| `completeness.py --enforce` | exit 0, 3 specs |
| `driver_rule.py --self-test` | exit 0 |
| xcheck.py on revision 5 | exit 0. 42 rows, each in a stage list. No stage mismatch in 4, 10 and 11. All cited ids exist (the ids not in section 12 are MC rows). All D- ids exist |
| keys5.py | `REASON_KEYS` = 49, all unique. All 37 wire keys of today are in the list. No reason key in the design is outside the list (the other tokens are fields and lobby refusals) |
| tokens.py | 67/67 `done:` tokens are in their files |
| 14 new code citations, checked with `sed` | all match: `transaction.rs:710-719`, `gather.rs:237-239`, `orders.rs:39`, `schema.rs:312`, `sources.rs:574-579`, `interface.rs:382`, `messages.rs:104-112`, `economy.rs:90-108`, `replica.rs:403`, `replica.rs:563`, `ingest.rs:462`, `ingest.rs:796`, `audio.rs:436`, `feedback.rs:819-830` |
| grep in the MC design and `pt1-driver.js` for OCR | the no-OCR rule exists (gives N-H1) |

Not checked:
- I did not read the plan diff.
- I did not run a cargo build or a browser driver.
- I did not trace the new rulings R1 to R10 in the vault.

My scratch files are in /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/regate-rigour/: r5.diff, xcheck5.out, rows5.txt and keys5.py.
