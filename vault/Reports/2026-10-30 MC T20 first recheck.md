---
type: report
date: 2026-10-30
tags: [turts, mc, review]
ai-first: true
source: codex
---

Task 20 independent recheck: **FAIL — High: 0; Medium: 1.**

Immutable candidate: `a8a375c2f8fb2ca8cce36d056c96649b69fd2a0a`, eight-file repair compared with `9a3365a`. Seat: web-platform-engineer, RUN15. No source edits, plants, staging, push or merge were performed. The prior exact-key/page-exposure plant remains valid; the ten-key frozen boundary was not changed by this repair.

**Medium: a modifier held through the CDP down path is lost by subsequent normal key and pointer calls.** At `tests/browser/harness/driver-io.js:87`, `opts.key` sends the modifier transition directly through CDP. Line 93 records that transition in the new held-modifier map. The normal key branch at line 131 and mouse calls at lines 56-57 still use Playwright, whose keyboard modifier state was not updated by that CDP event. The map fixes subsequent CDP overrides but does not carry held state across these input paths.

Concrete sequential reproduction on the actual compiled game:

```js
await io.key("Shift", [], { action: "down", key: "Shift" });
await io.key("KeyB");
await io.click(100, 100);
await io.key("Shift", [], { action: "up", key: "Shift" });
```

Expected: both the KeyB events and the click events have `shiftKey=true` until the explicit Shift-up. Actual at DPR1 and DPR2: Shift-down sequence 18 has `shiftKey=true`; KeyB-down sequence 19 and pointerdown sequence 22 both have `shiftKey=false`. The later document listener reads the actual page global during event propagation. All explicit events matched their sequence numbers, so the failure is not an ambient-event classification result. This is one modifier-state handoff defect affecting both subsequent input paths.

Receipt: `recheck-game.cjs`, `recheck-game.log`, `recheck-game.exit` (actual 1), `recheck-game-dpr1.json` and `recheck-game-dpr2.json`, all in this report directory. The command was `REVIEW_GAME_URL=<live service URL> env -u VK_ICD_FILENAMES -u VK_DRIVER_FILES node .../t20-review/recheck-game.cjs`. It ran the candidate factory and exact candidate init-script body, with sequential io calls, against the compiled game served by the candidate shell runner. No forced software ICD was set.

Both earlier findings are closed in their original cases:

- Normal Shift-down followed by `io.key("KeyA", [], {key:"q"})` now delivers `shiftKey=true` on the compiled game at DPR1 and DPR2. The recheck receipt shows sequence 15 true at both DPRs. The actual fixture also checks logical-key press, down and up, a held modifier retained after a temporary modifier, and isolation across the two windows.
- The public `--windows 2` option now works. Independent command: `MC_BROWSER_HEADED=1 bash scripts/mc-browser.sh mods/acme harness/harness-ok-driver.js harness/harness-ok-oracle.js 1280x720 --params tests/qa/driver/HARNESS_WINDOWS_ONE.json --windows 2`. It produced `PASS harness-ok-oracle.js`, actual exit 0 (`recheck-direct.log` / `.exit`). The input JSON says windows 1; generated params say 2, the oracle checks frozen params.windows is 2, and it inspects the second joined window. Its completion receipt records exit 0.

Independent checks and receipt review:

- `node --test tests/browser/harness/io-contract.test.js tests/browser/harness/cli.test.js`: 15/15, actual exit 0 (`recheck-node.log` / `.exit`). The added transport-spy test does not exercise the remaining CDP-to-Playwright transition.
- Six public CLI invalid-count cases (missing, zero, negative, fractional, unsafe integer and duplicate option): each actual exit 2, and none created its requested artifact directory. Commands and outputs are in `recheck-invalid-cli.json`.
- Parsed raw actual/downstream events from the independent direct run and the builder's final direct DPR1 run plus both final DPR2 row runs. Exact sequence and type equality holds for every explicit event; sequences are 1..40 in window 1 and 1..12 in window 2. Checked held Shift, temporary Shift plus held Control, release, cross-window isolation, params.windows and PNG 1280x720 (`recheck-raw-audit.json`). Filtering excludes only null-sequence ambient events; every numbered event is compared against the expected sequence and type arrays.
- Read the actual builder postcommit `repair/gqa-final-*.log` and `repair/t7-final/{trunk-build,t7}.log`: each required gate records exit 0. The builder final direct and row receipts record PASS and numeric exit 0. The remaining finding is outside their current event combinations.
- Started one candidate `--serve-only` service for the compiled-game reproduction. It stayed alive until explicit TERM, then recorded completion exit 143. The direct CLI service recorded completion exit 0. Both runs used the existing single `target/mc-t20` cache. The service sessions ended; no review browser/compiler remains active.
- Final source status is clean (`recheck-status.txt`, empty). No repeat source plant was needed because the frozen capability boundary did not change and its named red/restored-green proof already exists.

The ambient capture guard is unchanged. Compiled-game events in this recheck all matched their explicit sequence; the passive synthetic-wheel observation from RUN13 remains excluded. “Downstream” here means actual DOM propagation on the game page. It does not establish that the core applied an event in a presented frame. Applied-frame instrumentation, Task16 persistence/storage/readiness cases and Task24 CI registration of these Node suites remain work for their owning MC tasks.

M2 scope is unchanged: the original-source native baseline passed all 15 registered cases; the historical hosted failure mechanism remains **NOT TRACED**. Design section 1.6 assigns its fix to Task21. This candidate does not fix hosted CI or identify an engine cause. No new design/spec contradiction was found. The remaining finding is a repairable Task20 input contract defect, not a request to change the design.
