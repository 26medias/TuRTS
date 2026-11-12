RESULT=PASS
# L0-DBG follow-up: M1, L1, L3 (bevy-rendering-engineer)

Branch `mp1/l0-dbg-fu`, head `01dcd690`, pushed. Worktree `/home/julien/Projects/TuRTS-mp1-l0-dbg`. Scratch: `.../scratchpad/l0dbg-fu/`.
Base: bced6df. `origin/mp1/integration` (b215c3a9) does NOT contain the L0-DBG merge yet (bced6df is not an ancestor of it), so I merged b215c3a9 into the branch (`0df677dc`, no conflict). `--check --all` ran on that merged tree.

Commits: 0df677dc merge; bf259c72 M1+L1+L3 code; 2600b9bd the async check reads the function tag, params-contract pins each assignment once; 01dcd690 plant records.

## M1 done: ONE helper, and a scan that is red on any other spelling
- `tests/browser/harness/wait-sync.js`: `waitForSync(page, fn, arg, opts)` and `waitForDebugRecord(...)`. It is the only caller of `waitForFunction`.
    - Layer 1, before anything is sent: refuses an async function (the tag `[object AsyncFunction]`, so a function of another realm counts too).
    - Layer 2, in the page, at the first poll: throws when the predicate result has `.then`.
- Finding in the repair (checked in Chrome with Playwright 1.64.1, probe `scratchpad/l0dbg-fu/probe-pw.js`): a STRING that only gives a function is never called; the wait ends at once (counter stayed 0). So the helper sends one expression that calls the function, and writes the argument into it. The fake page of the tests now models this. With the old string form, 7 of 9 helper tests are red (`red-fake-string-form.txt`).
- Call sites moved: run.js:809, :823; sh1-lifecycle.js:129, :174, :200; driver-io.js:50. The fakes (checkpoint-run.test.js, debug-request.test.js) accept a string expression.
- `wait-sync-scan.test.js` (replaces `wait-predicate.test.js`): red on the token `waitForFunction` in any non-test source under `tests/browser` and `scripts`. It removes comments, then quotes, `+` and white space, so a bracket key, a built string, a template key, a bind alias and a destructured alias are all found. Red on the old tree with 6 sites (`red-m1-scan.txt`). A key built in a variable passes the scan (known gap, written in the test); the helper still stops an async or Promise predicate that reaches it.
- The 5 planted forms:
| form | direct call | through the helper |
| --- | --- | --- |
| object method `preds.joined` | scan | layer 1 (async method), test "an async predicate is refused: arrow, method, factory" |
| factory `make()` | scan | layer 1 if it returns an async function; layer 2 if it returns a function that returns a Promise |
| Promise with no async | scan | layer 2, run time, first poll (also in real Chrome) |
| `page["waitForFunction"]` | scan | not applicable (the call does not reach the helper) |
| `.bind` alias | scan | not applicable |
- Plants (records `L0-DBG/m1-*` in `docs/research/mp1/plants/l0-dbg.json`, all red-restored): `m1-scan-call` (a direct call in run.js), `m1-scan-weak` (scan without the quote removal), `m1-async-fake`, `m1-async-wait` (async predicate in run.js), `m1-helper-then`, `m1-helper-async`. The h1-* records are replaced.

## L1 done
- `params-contract.test.js` no longer sets `SERVER_BIN`. It takes the one top-level and the one indented `SERVER_BIN=` line of `scripts/mc-browser.sh` (:102, :115) from the script text, runs one of them before the argv recorder, and asserts argv[0] for the TARGET branch and the prepared branch. Zero or two assignments make it red; an ambient `SERVER_BIN` is deleted from the environment. Plants `l1-target`, `l1-prepared` (red).
- `ci-node-list.test.js`: red when any `tests/browser/**/*.test.js` is not in the `node --test` line of ci.yml (it listed 38 names, `red-l1-ci-list.txt`), or when the list names a missing file. Plant `l1-ci-list` (red).
- Tests not in the CI list before: 37. I ran each alone with Chrome off the PATH and DISPLAY unset (`individual.txt`): all 37 pass (the two with a real-Chrome case skip that case). All 37 are added; none is left out. The CI list has 50 files; `LEFT_OUT` is empty.

## L3 done
- `waitForDebugRecord` starts a request only when none is pending (page flag; cleared when the promise settles, also on reject). Test "a held frame: 500 polls start one request, not 500 waiters" (red when the guard is removed: plant `l3-pending`), a test for the restart after resolve and reject, and a real-Chrome test (about 30 polls, 1 request).

## Gates (exec_in_lane_v4, head 01dcd690)
- Node CI list: 277/277. Full set (`find tests/browser -name '*.test.js'`): 277/277 (same set now). Labels `l0dbg-fu-node-ci-1`, `l0dbg-fu-node-all-1`.
- driver_rule self-test and tree: 0. `mp1_plants.py --self-test`: 64 of 64 (`l0dbg-fu-driver-1`).
- `mp1_plants.py --check --all`: 222 apply, 0 stale (`l0dbg-fu-check-1`). The 4 stale l0-raw records of the re-check are gone with b215c3a9.
- l0-dbg records, run with a plants folder that holds only that file: 22 records; 18 red-restored in the first run; `m1-helper-async` re-pinned and red-restored; DBG1 a, b, c red-restored through the wrapper (`l0dbg-fu-plants-dbg1`).
- Not run: cargo fmt, clippy, cargo test, wasm check (no Rust file changed after bced6df except the merge of b215c3a9; the integration gates cover it).

## Headed (mc_headed_v6.sh, pre-merge, prepared, one call each)
| row | label | exit | result | wait_s | hold_s |
| --- | --- | --- | --- | --- | --- |
| HUD1 | l0dbg-fu-h-hud1 | 0 | 6/6; first checkpoints all at Joined in 6 of 6 runs | 0.009 | 33.3 |
| SEL1 | l0dbg-fu-h-sel1 | 0 | 2/2 | 0.009 | 12.5 |
- 0 yields. The SEL1 receipt lists one survivor: `Xvfb :99` pid 2452530. NOT TRACED whether it is the wrapper's own screen or a leftover of another call.
- The helper did not change a wait: same results and checkpoint phases as bced6df.

## Notes
- A plant run with `--files` selects other lanes' records too (`scripts/mc-browser.sh`, `ci.yml`). I stopped one such run and it left a plant of record `Q-PREBUILD/lane-dist-served` in `scripts/mc-browser.sh`; I restored it with `git checkout` and the tree is clean. Use `--plants-dir` for one lane's records.
- The plant runner started `cargo test` outside the lane wrapper and made `target/debug` (1.4 GB), which `exec_in_lane_v4.py` refuses. I deleted it (`rm -rf target/debug`, my own artefact).
- Recorded for `qa-lead`, not done: the convergence tick check carries no load in a static world (plant B green); the old SH1 flake "Execution context was destroyed" at `driver-io.js:371`.

## NOT TRACED
- The Xvfb survivor above.
- That a variable-built key (`page[name]`) is caught: it is not (known gap).
- Real-Chrome tests ran in headless Chrome 157 in this session; the headed form of `mp1-capture.test.js` was run in the first task only.
