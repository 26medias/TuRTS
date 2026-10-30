VERDICT=fail FINDINGS=8 HIGH=6 RUNS=13 NOT_CHECKED=5

# MC plan, gate 2, instrument lens (qa-lead)

Artefact: `docs/superpowers/plans/2026-10-29-mc-player-control.md` at `f69badf`. Evidence:
`/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/mc/gate2/qa/evidence.md`.

## Findings

1. Plan:593, High. DET1 cannot go green. The tick log line is `tick,wall_ns,unit_id,x,y` and `wall_ns` is `SystemTime::now()` (`crates/turts_server/src/ticklog.rs:4,64-82`). Also `--run-ticks` "may run a few ticks past N" (`config.rs:49-50`). So `cmp` of two process logs differs on every build, and the red of the plant proves nothing. Fix: compare `cut -d, -f1,3-` up to tick N. Show that projection red on the plant and green on the restore.
2. Plan:630-631, High, needs decision. With `LEDGER_LIMIT = 0` the self-test fails: `CASE dirs/dr6-repaired: want ['DR6'] got ['DR6', 'DR7']`, exit 1. The DR6 "repaired" check needs a ledger row, and any row is now DR7. Task 21 does not list `tests/qa/driver/cases.tsv` or the fixture. Options: (a) `check_directory` takes the limit as an argument, and the self-test keeps 5 for the fixture dirs; (b) the expected set becomes `DR6,DR7`, but then DR6 has no isolated red; (c) a DR6 fixture with its own ledger limit line.
3. Plan:82-90, High. The red window hides defects now, not from task 12. On run 40960227409 the `browser` job fails at "M2 browser proof", and "M3 browser controls" and "M2 browser-only semantic refutations" are `skipped`. All 8 browser plants run `scripts/m2-browser.sh` (`tests/m2/refutations.json`). So no build from today to task 21 runs `m3.js` or a browser plant. Tasks 2b, 9, 11, 13 and 14 edit `ingest.rs`, `replica.rs` and `render.rs`, and their G-REF skips or "only records" the browser cases (plan:323, :458). Task 21 has no step that finds the NOT TRACED cause of the M2 red. Fix: find the cause in wave 1. Run the later browser steps with `if: always()` so each red shows by itself, and name the local command that keeps each of the 8 browser plants red per task.
4. Plan:351-358, High. DRV1 cannot go red for a driver that gets `page` from `io`. Probe p3 `await io.page.evaluate((k) => self[k](), ["tur", "ts_debug"].join(""))` passes `driver_rule.py`, exit 0. Only the shape of `io` stops it (QA0D:327-329), and task 20 has no test of that shape. Fix: a harness self-test asserts that `io` is frozen, `Object.keys(io)` is the listed set, and that `io.key` `opts` calls only `Input.dispatchKeyEvent` with a string `key`. Then plant `io.page = page` and show the red.
5. Plan:628,632, High. The O3 list is not complete. The plan re-targets 4 browser plants. It is silent on E9-browser-position-only-snapshot, E9-browser-unbounded-buffer, E9-browser-duplicate-outstanding and E9-browser-ignore-midtrace-rearm. Their `expected_failure` texts are `m2.js` asserts (`m2.js:662,671,798-811`), and their cases use the `M2_CASE_SUMMARY` protocol (`m2-browser.sh:130-134`). Task 21 removes `m2.js`. The retired-plant list for gate 2 does not exist yet. Fix: list all 8 with the new anchor or the retirement note.
6. Plan:536,568, High. SEL2 "a frame late" and ORD7 "marker on `OrderResult`" have no browser frame record. The web bridge has `last_frame` only (`debug.rs:171`), and R5 adds no frame key. With a local server the reply comes within about 1 frame, so these plants can stay green. Fix: add `turts_debug.frames` (frame index, wall time, ring count, marker count), and show each plant red twice.
7. Plan:663, Medium. REG1 does not test the plain-move route. `m3_m2_stream.rs:258-266` calls `apply_command(CommandKind::MoveTo)` directly, so a `to_sim` that sends a plain move as `CommandKind::Order` stays green. Fix: a test sends a one-unit `Move` envelope through `orders::to_sim` and compares the operation count. Plant: route it to `Order`. The "M3 operation stream" of D:1217 does not exist (plan:716).
8. Plan:66-81, Medium. The G-REF table comes from the files the plan names. Task 3 must wire `content_mc_validation.rs`. `content_m3_validation.rs` is a `#[path]` module of `content_m3.rs:15`, and `content.rs:35` holds the E10-public-error-provenance anchor. It is NOT TRACED where `UnitOrders` is inserted (`bootstrap.rs` has E5 and C6). Fix: compute G-REF from `git diff --name-only` against both registries.

Non-blocking: DET1, EL1 and `mc_cost` must have their own `scripts/acceptance.sh` line, because the script lists each test (`:84-120`). `mc-pixels.sh` must skip `"runs": "headed"` rows with a SKIP line (plan:664 globs all rows). The VIS2 "row check" has no named command. P1: `defaultPrevented` is always yes, because the page calls `preventDefault()` itself (plan:273). Commit the JSONL and `p1-keys.md` under `SHA256SUMS`. Oracles are not require-followed (`driver_rule.py:255-273`).

## What I ran

| command | claim tested | refute | observed |
| --- | --- | --- | --- |
| `driver_rule.py --self-test` | G-QA green (plan:59) | exit 1 | exit 0, 46 PASS |
| `driver_rule.py tests/browser` | same | exit 1 | exit 0 |
| driver copies p0 to p5 | DRV1 red build (D:1215) | p1 green | p1 exit 1 A1,A3; p3 exit 0 |
| limit-0 copy, self-test and dir | task 21 steps 1-2 (plan:630-631) | self-test exit 0 | exit 1, dr6-repaired; dir DR7 |
| `completeness.py --self-test`, `--enforce` | CI1 (D:1220) | exit 1 | exit 0, 21/21, 2 specs |
| `plants.py` on both registries | G-REF table complete (plan:66) | every plant file in table | 3+ files missing |
| `gh run view 40960227409` | red window (plan:86-88) | later steps run | skipped |
| `grep -n` ci.yml | plan cites :29-35, :108-124 | moved line | all match |
| grep `wall_ns` | DET1 byte-equal (D:1197) | no clock column | `ticklog.rs:64` |
| grep acceptance.sh | DET1, EL1 runner | glob | explicit list |
| read `m3_m2_stream.rs` | REG1 plain move (D:1217) | uses `to_sim` | `apply_command` |
| grep frame in web bridge | SEL2, ORD7 frame | frame log | `last_frame` |
| grep `--run-ticks` | DET1 log length | exact N | "past N" |

## Not checked

Local `m2-browser.sh` and `m3-browser.sh` (cost). The PROJ1 adapter need. The lightyear size limit. Tasks 17 and 23 oracle texts. The no-content scan for `move`.

## What I changed

`docs/research/mc/gate2-qa.md` (this report, by request); memory under `.claude/agent-memory/qa-lead/`.

## Open questions for the orchestrator

1. Finding 2: pick option a, b or c.
2. Finding 3: find the M2 red cause in wave 1, or accept that the browser steps stay hidden until task 21.

## Re-check 1

VERDICT=fail HIGH=1. Plan `9910798`, design `89e1fa3` (revision 8). Probes: `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/mc/gate2/qa/re1/`.

| Finding | Status | Plan line | Run |
| --- | --- | --- | --- |
| 1 DET1 | fixed | 658, 663 | Two logs with different `wall_ns` and 3 extra ticks: raw `cmp` exit 1; the `.cut` files `cmp` exit 0; one position changed at tick 400: exit 1. |
| 2 ledger | fixed | 691, 693 | Limit-0 copy on a directory with 1 row: DR7, exit 1; default 5: exit 0. |
| 3 red window | fixed in part | 69-72, 98-106, 373 | The cause is traced in task 20 step 1. Residual (Medium): before task 21 no local gate runs `scripts/m3-browser.sh` (lines 104, 691, 694 only). Before the M2 fix, "plants add no other failure text" cannot see a defect that comes after the failing point. |
| 4 io | fixed | 369, 374, 378 | A mimic of the contract: clean `[]`; with `io.page` planted it reports keys, `page` and `evaluate` reachable. |
| 5 eight plants | fixed | 680-687 | - |
| 6 frames | not fixed for SEL2 | 479, 571, 573 | `FramesFeed` holds no input reference (no event sequence, no wall time), so the oracle cannot find "the first frame after the mouse-up". The frame-late plant stays green. ORD7 is fixed: the marker frame comes before the result frame. |
| 7 REG1 | fixed | 24, 468, 471 | - |
| 8 G-REF | fixed in part | 69-70 | New defect, below. |

New defects:

- High: SEL2 (finding 6). Fix: each frame record holds the last input event sequence (or the input-log wall time) that the frame applied.
- Medium: the G-REF one-liner reads only `plant_path`. It drops E4-local-capacity and E6-capacity-bypass (`transaction.rs`, task 4), and C1 and C5. Fix: also read `plants[].path`.
- Medium: `--ledger-limit` default 5 contradicts D:1170-1171 ("lowers `LEDGER_LIMIT` ... so the ledger cannot grow back"). Only the flag at `ci.yml:149` keeps the ledger at 0, and if that flag is removed no check goes red. Fix: set the default to 0 and give the fixtures `--ledger-limit 5`, or amend the design.

## Re-check 2

VERDICT=pass HIGH=0. Plan `cc514f6`, design `f9353bb` (revision 9).

| Item | Status | Plan line | Run |
| --- | --- | --- | --- |
| 6 SEL2 frame late | fixed (by reading) | 371, 481, 573, 575 | Each frame record holds `applied_seq`, and the oracle selects the record that holds the sequence number of the mouse-up. With the plant, that record has `RingsShown` 0, so the oracle can go red. No browser run was made: the timing of the listener and the drain is NOT TRACED until task 9. |
| Red window, `m3-browser.sh` | fixed | 100 | G-WEB runs it for tasks 2b, 4, 6, 9 and 11. |
| Red window, "no other failure text" | fixed | 72 | The browser cases are "not checked" until the M2 red is fixed. |
| G-REF `plants[].path` | fixed | 70, 72 | The plan's selector on `transaction_m3.rs`, `machine.rs` and `command.rs` lists E4, E6-capacity-bypass, C1 and C5 (and also E6-carried-item-loss, C2, C4, C9, C11), exit 0. |
| `--ledger-limit` default 0 | fixed | 693, 695 | A copy with limit 0: `tests/browser` exit 1 (DR7); a one-row directory exit 1 (DR7). The self-test at 5: exit 0. |

New, low:

- No green `m3-browser.sh` run on hosted CI exists. It was added after the last green CI run, d194fc2. The only green run is local (`vault/Work Log/2026-10-28.md:182`). Add a clean-tree baseline run of `m3-browser.sh` to task 20 step 1.
- The new `cases.tsv` case needs the default limit, but the self-test passes 5 to the fixtures. The case needs its own limit field.
