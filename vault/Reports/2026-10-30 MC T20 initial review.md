---
type: report
date: 2026-10-30
tags: [turts, mc, review]
ai-first: true
source: codex
---

Task 20 independent review: **FAIL — High: 0; Medium: 2.**

Candidate: `9a3365adfc2a6b53b71410ba9f56d02b344bedb5`, compared with `22a4600`. Seat: web-platform-engineer, RUN13. Reviewed all 19 changed files. No source repair, staging, push or merge was performed. The one authorized page-exposure plant was restored.

1. **Medium: `opts.key` drops modifiers held by an earlier `io.key` call.** At `tests/browser/harness/driver-io.js:81`, the CDP branch starts its modifier mask at zero and adds only the current call's `modifiers` array. The separate down/up CDP branch also hard-codes zero at line 71. The harness does not retain the modifiers held by previous calls. Run these sequential calls: `await io.key("Shift", [], {action:"down"}); await io.key("KeyA", [], {key:"q"}); await io.key("Shift", [], {action:"up"});`. The actual compiled game page sees Shift keydown with `shiftKey=true`, followed by KeyA/q keydown and keyup with `shiftKey=false`. An override for the logical key must preserve a separately held physical modifier. This can make layout-sensitive modified bindings fail and can give the game a false modifier state.

    Reproduced with the candidate factory and its exact init-script body on the compiled game, in headed Chrome at DPR1 and DPR2. The downstream document listener reads `window.__turts_io_seq` during propagation. For both runs it observed Shift-down sequence 14 / true, KeyA-down sequence 15 / false, KeyA-up sequence 16 / false, Shift-up sequence 17 / false. All explicit sequences matched the recorded events; this failure is independent of the ambient-event classifier. `real-game.cjs`, `real-game.log`, `real-game.exit` (1), and `real-game-dpr1.json` / `real-game-dpr2.json` contain the reproduction and raw events. The script starts the same game URL supplied by the service receipt and performs the io calls sequentially.

2. **Medium: the advertised `--windows N` CLI option is absent.** The Task 20 interface at `docs/superpowers/plans/2026-10-29-mc-player-control.md:371` defines `io.use(n)` as window n of `--windows N`. `scripts/mc-browser.sh:19` recognizes only `--serve-only` and `--params`; line 24 treats every other token as a positional argument. Thus a trailing `--windows 2` overwrites the viewport with `2`. Row JSON supplies two windows, but it does not implement the advertised direct CLI option. A direct invocation must retain the requested viewport, create two pages and permit `io.use(2)`.

    Reproduction: `MC_BROWSER_HEADED=1 bash scripts/mc-browser.sh mods/acme harness/harness-ok-driver.js harness/harness-ok-oracle.js 1280x720 --windows 2` (normal native selection; no forced Vulkan ICD). Actual output is `invalid viewport: 2`, exit 1 (`mc-browser-windows.log` / `.exit`). Its completion receipt also records exit 1. The runner rejects viewport `2` before it starts the driver. The supplementary `run.js` parser check also rejects `--windows` with exit 1 (`cli-windows.log`), although the public shell CLI is the contract at issue.

Independent validation:

- `node --test tests/browser/harness/io-contract.test.js tests/browser/harness/cli.test.js`: 13/13, exit 0 (`node-green.log` / `.exit`).
- `python3 scripts/qa/driver_rule.py --self-test`: 47/47, exit 0; `python3 scripts/qa/driver_rule.py tests/browser`: exit 0 (`driver-selftest.*`, `driver-tree.*`).
- Authorized plant: insert `page: pages[0]` into the io factory object, then run `node --test tests/browser/harness/io-contract.test.js`: exit 1, named failure `io is frozen with exactly the ten approved functions and exposes no browser handle`, with the extra `page` key in the assertion diff (`plant-page-red.*`). Restore the committed file, then rerun the two Node suites: 13/13, exit 0 (`plant-restored-green.*`).
- Parsed the raw input logs, downstream event arrays, ambient arrays, params and saved PNGs from builder final runs `run-20261030T160250Z-8TTUmb`, `run-20261030T160316Z-eMka9q`, and `run-20261030T160410Z-146BUJ`. Every explicit event matched; window 1 has sequences 1..36 and window 2 has 1..2. Wheel DOM delta is 90. PNG is 1280x720 at DPR1 and DPR2. Ambient records have null sequence. Audit: `candidate-browser-audit.json`. Builder final7 has two PASS runs and exit 0; corrected DPR1 final2 has PASS and exit 0.
- `MC_PIXELS_HEADED=0 bash scripts/mc-pixels.sh tests/qa/driver/HARNESS.json`: `SKIP HARNESS headed`, exit 0, with no PASS (`headed-skip.*`).
- Independently started `scripts/mc-browser.sh mods/acme --serve-only`, read its live URL, and loaded the compiled game for the reproduction. The shell stayed alive until TERM. Cleanup completed with code 143 and both services exited (`game-service.log`, `game-service/.../completion.json`).
- Read actual postcommit builder G-QA/G-WEB logs: all recorded exit 0, including yardstick, completeness self-test/enforce, driver rule, Node, shell syntax, Trunk and T7. T7's named segment oracle passed. No package dependency or lockfile change appears in the candidate diff. `git diff --check 22a4600..9a3365a` passed.

The ambient-event guard was inspected at `tests/browser/harness/run.js:82`. A separate passive synthetic canvas exposed an asynchronous wheel-delivery race: an explicit wheel was marked ambient after mouse.wheel returned. Adding a non-passive canvas wheel handler removed that result at both DPRs. The actual client uses `prevent_default_event_handling: true` at `crates/turts_client/src/lib.rs:254`; the compiled-game reproduction had all sequences matched and wheel 90 at both DPRs. The passive-page observation is excluded from the findings. It is not evidence that the current client masks injected wheel events.

The downstream predicate is bounded to real DOM delivery: capture observers and a later document listener see the page global during propagation. This verifies the event and its sequence reaching the game page; it does not prove that the game core applied the event in a presented frame. Per-frame applied-input bridge records required by design 13.2 are future client instrumentation and were NOT TRACED by this review.

The M2 trace is limited. The clean original-source native runner at `t20-builder/m2-clean-native-20261030T153614Z/runner.log` records all 15 registered M2 cases passing, with `exit-code.txt` equal to 0. Its per-run summaries have no semantic, infrastructure or timeout failures. `root-audit.json` records source `22a4600` and the original m2.js blob. This establishes a local native baseline. It does not establish the cause of the historical hosted timeout or fix hosted CI. The hosted failure mechanism remains **NOT TRACED**. No speculative engine owner is assigned. Design section 1.6 (`...mc-player-control-design.md:205`) gives the hosted fix to Task 21, which rewrites m2.js. The plan's Task 20 step 1 conditional repair wording remains broader; the supplied design-precedence instruction governs that ownership. No m2.js change was made by this candidate or review. No new design/spec contradiction was found.

Final worktree status: clean (`final-status.txt`). The review ran one reversible plant and retained all reports and reproduction files outside the task source tree. The candidate needs both Medium findings repaired and retested before approval.
