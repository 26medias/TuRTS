---
type: report
date: 2026-10-30
tags: [turts, mc, review]
ai-first: true
source: codex
---

Task 20 final independent review: **PASS — High: 0; Medium: 0.**

Immutable candidate: `72ddb0a6541862b3d26e1376c8bded687f1c6ce5`. Compared its four-file repair with `a8a375c` and read the prior failed review. Seat: web-platform-engineer, RUN17, the CEO-authorized final Task20 review. No unresolved actionable finding was found within the Task20 contract. This review did not edit source, start another agent/run, stage files, push or merge.

The last Medium finding is closed. `tests/browser/harness/driver-io.js:96` synchronizes a successful CDP modifier transition into the backing Playwright keyboard state. Temporary modifiers are synchronized at lines 111 and 124. The internal helper at line 255 updates the modifier-name and physical-key Sets used by normal keyboard input and the mouse, without emitting a second DOM event.

Independently reran the former failing sequence against the actual compiled game, at DPR1 and DPR2, with the candidate factory and exact candidate init-script body:

```js
await io.key("Shift", [], { action: "down", key: "Shift" });
await io.key("KeyB");
await io.click(100, 100);
await io.drag(100, 100, 120, 120);
await io.key("Shift", [], { action: "up", key: "Shift" });
await io.key("KeyC");
```

Both DPRs now deliver the normal KeyB events with `shiftKey=true` and logical key `B`; every click/drag event retains Shift. After explicit release, KeyC is unmodified with key `c`. The earlier normal Shift-down followed by CDP logical `q` also remains Shift-held. The actual downstream document listener reads the page global during propagation. All explicit event numbers are continuous 1..36 and match the actual/downstream event arrays. Actual command exit: **0**. Receipts: `final-game.cjs`, `final-game.log`, `final-game.exit`, and new `final-game-dpr1.json` / `final-game-dpr2.json`. Earlier red evidence was preserved.

The private state bridge is compatible with the pinned dependency:

- `requirePlaywrightKeyboardState` at `driver-io.js:238` checks the in-process connection bridge, both backing Sets and the shared mouse keyboard object. The unsupported-backend test confirms failure before a CDP event is dispatched. The error explains the missing backend capability.
- Read the installed primary implementation in `tests/browser/node_modules/playwright-core/lib/coreBundle.js`: `createInProcessPlaywright` / `toImpl` at lines 65524-65552, backing Keyboard state and transitions at 22490-22533, and the Mouse keyboard link at 22615. The candidate helper matches those actual structures.
- Independently verified the unchanged hashes: package-lock SHA256 `b59c7c583a62388f558912d0ccfdb81d2a43c6c074a2bc69f9be66ed1b53906e`; core bundle SHA256 `829521b67ed7be90c7d6d6098ff39f2241562d47e4ad7ccab13710924a56001d`. Package version remains exactly 1.64.1. No dependency file was changed.
- The backing keyboard remains internal to the helper. No new io key, function property or result exposes the Page, keyboard implementation, context, browser or input-sequence global. The frozen ten-method boundary at line 210 remains intact. Logical overrides still use CDP; move/wheel/pointer operations still use page.mouse.

Compatibility limit: this implementation intentionally uses private fields of the pinned in-process Playwright backend. A future dependency/backend change must recheck that compatibility. The current guard fails explicitly if its required bridge or shape is absent; this is a stated limitation, not a current failure.

Independent validation:

- `node --test tests/browser/harness/io-contract.test.js tests/browser/harness/cli.test.js`: **17/17, actual exit 0** (`final-node.log` / `.exit`). This includes the no-handle boundary, input sequences, logical overrides, per-window held modifiers, handoff to normal keys/mouse, and unsupported-backend failure before dispatch.
- Fresh public CLI command: `MC_BROWSER_HEADED=1 bash scripts/mc-browser.sh mods/acme harness/harness-ok-driver.js harness/harness-ok-oracle.js 1280x720 --params tests/qa/driver/HARNESS_WINDOWS_ONE.json --windows 2` (normal native selection; forced Vulkan ICD variables unset). It produced **PASS harness-ok-oracle.js, actual exit 0** (`final-direct.log` / `.exit`). The row JSON says one window; the generated frozen params say two, and the oracle inspects both joined pages. Its completion receipt records 0.
- Independently parsed raw logs for that fresh direct run plus builder final direct DPR1 and both final DPR2 row runs. Every numbered downstream event and event type equals the requested sequence; sequences are continuous 1..59 in window 1 and 1..12 in window 2. CDP-held Shift reaches normal key/click/drag, release restores false, params.windows is 2, and saved PNGs are 1280x720. Ambient records retain null sequence. `python3 .../t20-review/final-raw-audit.py`: **actual exit 0** (`final-raw-audit.log` / `.exit` / `.json`). Null-sequence ambient records are excluded only from assertions about explicit events; all numbered events are compared.
- Read actual postcommit builder `final-repair/gqa-final-*.log`: yardstick, completeness self-test/enforce, driver self-test/live tree, Node and shell syntax all record exit 0. Read `final-repair/t7-final/trunk-build.log` and `t7.log`: Trunk and the named T7 segment oracle record exit 0. Builder final direct/row receipts also record actual exit 0, and the page-exposure plant records named red exit 1 followed by restored 17/17 green exit 0. These are audited builder receipts, separate from the fresh independent browser/Node checks above.
- No repeat source plant was needed: the frozen capability boundary was unchanged, prior independent named page-plant/restoration evidence remains valid, and the new compatibility guard has a direct failure-before-dispatch test plus fresh successful runtime proof.

Cleanup and scope:

The scoped serve-only service stayed alive until explicit TERM and completed with 143. The direct CLI service completed with 0. Both sessions ended; the review browsers closed in finally blocks. Only the existing `target/mc-t20` compiler cache was used. `HEAD` remains the immutable candidate; `driver-io.js` blob is `882c3ecc9a9416d9540e758719f8f7c02d70c209`; final status is clean (`final-72ddb0a-status.txt`, empty). All new evidence is outside the task source tree. No additional review or repair run was started.

The actual-page predicate proves DOM delivery and the sequence visible during propagation. It does not prove future per-frame `applied_seq` instrumentation. Task16 persistence/storage/readiness/expected-refusal rows and Task24 Node-suite CI registration remain work for their owning tasks. No Task20 API scope was narrowed for this pass.

M2 status remains: clean original-source native baseline 15/15; historical hosted underlying cause **NOT TRACED**, owned by Task21 under design section 1.6. This review makes no hosted-CI-fix or engine-cause claim. The prior passive synthetic-wheel observation remains excluded; actual compiled-game wheel delivery and sequence checks pass. No new design/spec contradiction was found.
