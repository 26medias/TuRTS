# Review of L0-DBG (performance-architect)

Source: subagent report. Agent output; claims are leads until verified.

VERDICT=FAIL HIGH=1

Review of L0-DBG (8b87961, base 51f1223), performance-architect. What I changed: none. The worktree is clean at 8b87961, and `cmp` shows the scratch edits are restored exactly. Scratch folder: /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/l0-dbg-review

Headed runs. Each row is one v6 `--mode pre-merge` call. "fix" means 8b87961 plus my scratch repair (`--allow-dirty`), now reverted.
| row | commit | exit | pass | wait_s | hold_s | debug builds |
| T7 | 8b87961 | 0 | 1/1 | 475.0 | 238.0 | 31 in 576 frames |
| HUD1 | 8b87961 | 0 | 6/6 | 155.7 | 234.0 | 4 per run |
| PT1 | 8b87961 | 0 | 2/2 | 136.6 | 727.7 | W1 178/8,499 frames; W2 118/2,725 |
| SH1 | 8b87961 | 1 | 0/2 | 401.2 | 137.4 | 1,450 (request(0)) |
| ORD3 | 8b87961 | 1 | 0/2 | 367.2 | 61.6 | 2 |
| SLOT1 | 8b87961 | 1 | 0/2 | 376.9 | 54.7 | - |
| M2 E9-base-public-convergence | 8b87961 | 1 | 0/1 | 96.8 | 41.5 | - |
| SEL1 | 8b87961 | 1 | 0/2 | 0.01 | 118.0 | 81 |
| SH1 / SLOT1 / M2 / ORD3 / SEL1 | 51f1223 | 1 / 1 / 0 / 1 / 0 | 1/2, 0/2, 1/1, 0/2, 2/2 | 0.0 / 145.8 / 768.0 / 522.7 / 951.8 | 145.4 / 122.9 / 30.1 / 105.1 / 95.6 | - |
| SEL1 / M2 / SH1 | fix | 0 / 0 / 1 | 2/2, 1/1, 1/2 (same as base) | 178.9 / 0.01 / 0.03 | 87.8 / 56.4 / 172.5 | - |

How to read the reds:
- **ORD3:** red at the base with the same message. It is an old red, not this task.
- **SH1:** run 1 fails at the base and at the head with the same error ("Execution context was destroyed"). That part is old. Run 2 at the head is a new semantic red: `connection-lost-reload`. The rejoin screenshot is byte-equal to the screenshot after the reload click (sha 484e423b…).
- **SEL1 and M2:** new reds at the head.
- **SLOT1:** red at the base too, but the head fails earlier and in a new way: owner A is still at `FetchAdvertisement`.
- **With the fix:** SEL1 and M2 go green, and SH1 matches the base.

Item 2, frame coherence. My script (`coherence.py`) compares `debug.units[].pos` with `scene.units[].translation` (x, z) in each checkpoint. I calibrated it on old runs: 0 mismatches.
- New runs: 0 mismatches in every checkpoint I compared (T7 30, HUD1 13, PT1 579, SEL1, SH1).
- Scene and debug come from the same frame, but now both are later than the screenshot. The lag from screenshot to record goes from p50 1.3 ms / max 50 ms (PT1, hud2-run130-l-pt1) to p50 25 ms / max 104 ms (T7 max 61, SH1 max 216).

Item 3, held frames. The checkpoint reads the published record when held (scene-checkpoints.js:22-25). SH1 sends request(0) on each frame (sh1-lifecycle.js:22), so the record is of the held frame. `view.bridge` (view.js:24) does not check the hold: a read during a hold waits 10 s, then rejects. In all my runs I found 0 "no frame served the request" errors. The SH1 holds were verify and joining, and both were released.

Item 4, the cap.
- Cost: PT1 checkpoint lag in total is 7.8 s and 8.1 s per run, against 1.1 s and 1.3 s before. That is about +7 s per run, not +4 min. The run span is 321 s and 325 s, against 319 s.
- Ruling proposal: keep 4 Hz for the default request. `min_interval_ms` 0 is a legal test-only form, limited to the SH1 per-frame phase capture. The `debug_builds` counter shows its rate.
- DBG1 erratum: "With a request, the rate is at most 4 Hz; a harness can pass `min_interval_ms` 0 for a per-frame capture (SH1 only, test-only)."

Item 5, counter and trace bound.
- `cargo test -p turts_client --locked --test debug_request`: 4/4 green.
- My plant P1 (`pending` not cleared after a build): red at debug_request.rs:54, left 6, right 3.
- My plant P2 (trace bound per world, not per unit): red at replica_m3.rs:809, left 0, right 3.
- Both are restored and green again (exit 0).
- Build ratio: T7 5.4 %; PT1 2.1 % and 4.3 %. Before the change it was 100 %. The frame count comes from `frames.recent[].frame_index`; whether it resets at `frames_clear` is NOT TRACED.

Item 6, the cost that is left.
- A frame with no request runs only cheap checks (no build, no copy): one mutex take in `take_debug_request` (web lib.rs:435-439), one run condition (debug.rs:88-90), and one u64 compare in `publish_debug` (web lib.rs:446-451).
- N waiting promises share one build and one `JsValue` (web lib.rs:452-459).
- Size: wasm 28,437,056 → 28,446,082 B (+9,026); JS glue +2,598 B. My build is byte-equal to the builder's (`cmp`).
- Ten-times-cheaper candidate: `publish_scene` and `publish_ui` still run on every frame. Gate them the same way.

Item 7, capture form for a person's Chrome. It is not delivered: there is no `tests/browser/mp1-capture.js` and no `scripts/mp1-play.sh`. Plan Step 2 (plan:574) asks for both, plus a test. The play script must pick a free port at start and print it. The sheet must give that printed port, never 9222. It must also give the non-default `--user-data-dir`. The tool must take the port as an argument.

Item 8, gates. All green:
- fmt: 0. clippy `-D warnings`: 0. `cargo test -p turts_client`: 445 pass, 0 fail.
- Node tests: CI list 42/42; harness 164/164.
- driver_rule: self-test 0; tree 0. The rule bans every `turts_*` identifier (driver_rule.py:128). My two scratch drivers that call `turts_debug_request` went red A1.
- Frozen sha256 matches.

Item 9, merge readiness. `merge-tree` gives a clean tree (43ab10d), with no conflict in ci.yml. On the merged tree, the runner `--list` accepts the 4 l0-dbg records (exit 0). `--all` exits 2 because of the l0-split.json records, and it does the same on integration alone, so that is older than this task.

Findings
1. HIGH: four `page.waitForFunction(async …)` predicates: run.js:806, :818 and sh1-lifecycle.js:127, :198.
    - Playwright 1.64.1 does not await the predicate (coreBundle.js:25707-25709). My probe resolved `async () => false` in 5 ms with `false`.
    - So the join gate (run.js:287) and the SH1 and SLOT1 waits no longer wait.
    - HUD1: 5 of 6 first checkpoints are at Connecting or Joining.
    - PT1: 6 startup observations and none at Joined, against 14 before. PT1 still passes, so its oracle cannot see this.
    - Result: new reds in SEL1, M2 and SH1.
    - Repair: use sync predicates that call `turts_debug_request?.().catch(()=>{})` and read `turts_debug()`. The patch is repair.patch in the scratchpad.
    - Instrument: the fake in checkpoint-run.test.js:27 treats a Promise as true. Add a test or rule that bans `waitForFunction(async`.
2. MEDIUM: PT1's oracle `judgeLoad` (pt1-oracle.js:227) does not require the final startup observation to be Joined. Add that check.
3. MEDIUM: the plan Step 2 capture tool and its test are missing (item 7).
4. LOW: `view.bridge` waits 10 s during a hold (view.js:24). Check `__turts_input_held` there as well.
5. LOW: the record is up to about 0.2 s later than its screenshot (item 2). Ask for the record before the screenshot.

Notes on the v6 wrapper from real use:
- 16 calls (15 rows plus the SH1 fix run). Waits ran 0.01 to 951.8 s. Holds ran 30 to 728 s, all under the 1,800 s limit. 0 yields, 0 survivors, and the status always matched the exit.
- In-lock builds did not compile (0 "Compiling" lines in the HUD1 and PT1 logs), except the T7 release test binary of t7.sh.
- A checkout to the base and back makes cargo rebuild everything (mtime), so build outside the lock again after it.
- The lock holder I saw was a v4 caller (flock pid 4190715, mc-hud lane). v4 calls bypass the ticket queue.

Still owed:
- KM1 not run.
- Re-gate after the H1 repair: SLOT1 at the fix (red at the base, so it needs the pixel lane's fixes), plus T7, HUD1 and PT1 on the repaired harness.
- Item 4 wall times at the base: I measured the cap cost from checkpoint lag, not from base-row receipts.

