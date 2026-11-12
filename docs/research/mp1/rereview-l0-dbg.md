VERDICT=PASS
# Re-review of L0-DBG repair (bced6df), performance-architect

Mode: review. What I changed: none. The worktree is clean at bced6df; every scratch plant was restored (`git checkout`, `cmp`, `git status` empty).
Integration at review time: origin/mp1/integration = 4e140f7. There are no High findings.

## Headed runs (mc_headed_v6.sh, pre-merge, prepared form, 3 lock holds)
| hold | label | rows | exit | wait_s | hold_s |
| 1 | l0dbg-rerev-hold1 | T7, PT1 | 0 | 74.3 | 677.4 |
| 2 | l0dbg-rerev-hold2 | SH1, M2 E9-base-public-convergence x3 | 1 (SH1 only) | 487.9 | 108.2 |
| 3 | l0dbg-rerev-hold3 | M2 plants A, B, old sampler C x2 | 0 | 280.1 | 53.6 |
- None of the holds had a yield or a survivor. Hold 3 kept the source unchanged (true).
- Each of the four prepare steps ran OUTSIDE the lock (labels l0dbg-rerev-prep2-*), and each exited with 0.
- T7: PASS. 41 frames (25 moving), lag_p99 1.477 ticks. Before this task the usual range was 41 to 47 frames and about 1.4 ticks.
- PT1: 2/2 PASS. The startup records now reach Joined: 12 and 11 observations, and the last one in each window is Joined. At 8b87961 there were 6 observations and no Joined.
    Checkpoint coherence (scene against debug): 0 mismatches in 573 checkpoints.
    Lag from screenshot to record: p50 26-28 ms, max 145 ms.
    Debug builds: 180/7,985 frames (W1) and 118/2,568 frames (W2).
- SH1: 1/2. The red run has the OLD stack: collectStorageAudit at driver-io.js:371, "Execution context was destroyed". At the base, run 1 was red and run 2 green; here, run 1 is green and run 2 red.
    The new red of 8b87961 (`connection-lost-reload`) did not come back.
    Owner of the old red: the harness (`qa-lead`). The storage audit evaluates while the reload navigates. NOT TRACED on the board.
- M2 convergence: 3/3 PASS.

## 1. The M2 convergence flake: cause and sampler
- Cause: harness timing under the 4 Hz cap. It is not a product effect. I ran the old sampler (8b87961 form, logged) at bced6df, plant C, 2 runs.
    In every round, window 2 was 2 to 4 ticks behind window 1 at first. The gap drifted to 0 only at round 29, after 7.3 s, against a 10 s limit.
    `same_projection` was true in 29 of 29 rounds, so the worlds agreed and only the pair timing differed.
    Model probe (old-sampler-model.js, the builder's virtual windows, random first build times): 3,248 of 10,000 start states never share a tick in 10 s. The gap between the two builds stays locked from round to round.
- The new sampler keeps the old condition: the same tick AND the same public projection (m2-oracle.js:176-178). With plant A it found an equal tick in 21 of 32 rounds.
- Can it pass when the worlds differ? Plant A (window 2 sees its first unit 1 m off): red, "projections differ". This is correct.
- The coordinator named the leg `E9-browser-convergence-owner-offset`. It does not exist in the tree (`git grep` 0 hits), so I made plants A and B.
- Plant B (window 2 must be 1 tick later) stayed GREEN. This world is static (equal projections at different ticks), so the tick check carries no load in this case. The old sampler has the same weakness. This is finding M2.

## 2. params-contract.test.js
- It is red at f372c77 and at 4e140f7 (14 pass, 4 fail). It is not in the CI node list (ci.yml:114, 0 hits), so no gate saw it.
- The fix sets SERVER_BIN in the test env. This makes argv[0] tautological: the test no longer checks that the binary comes from TARGET (mc-browser.sh:102) or from PREPARED (:115). Finding L1.

## 3. High 1 (async predicates): closed for every form in the tree
- The scan (wait-predicate.test.js) goes red for: direct async, async function expression, `.then` on a request, and a `.mjs` file.
- It stays GREEN for 5 planted forms:
    - an object method `preds.joined`
    - a factory `make()`
    - `() => Promise.resolve(x)` (a Promise with no `async`)
    - the computed key `page["waitForFunction"](async …)`
    - an alias `const wait = page.waitForFunction.bind(page)`
- No such form exists today. The grep over tests/browser found only the two fakes. It is the same kind of gap as the driver-rule A7 note. Finding M1.
- The fake (checkpoint-run.test.js:29-33) refuses a Promise.

## 4. Mediums 2 and 3 of the first review: closed
- Removing the new Joined step makes pt1-oracle.test.js red (5 pass, 1 fail). Removing the `!held` check in view.js makes debug-request.test.js red (8 pass, 1 fail).
- mp1-capture.test.js: 9/9 green, and that includes a real headless Chrome that I started.
- scripts/mp1-play.sh:
    - it picks a free port at each start and prints it, with no 9222
    - it uses a `mktemp -d` profile and removes it at the end
    - it kills only the pids it started (cleanup)
    - it has no `--mute-audio` and no mixer command
    - it never connects to another Chrome
- Only sh1-lifecycle.js:22 passes `min_interval_ms` 0.

## 5. Owed items: accept as owed
- The record is up to about 0.2 s later than its screenshot (p50 26 ms, max 145 ms). Owner: I-PIX1 (plan:1099), before a row compares the pixels of moving units with scene positions.
- `publish_scene` and `publish_ui` still run on every frame. Owner: PRES-MEASURE (plan:1246). Measure them on and off together with the debug state.

## 7. Merge readiness
- `git merge-tree --write-tree origin/mp1/integration(4e140f7) bced6df`: clean tree ddc46a6, no conflict.
- `mp1_plants.py --check --all` on the merged tree: 143 records apply and 4 are stale. All 4 are known l0-raw.json records (4, 5, 8, 10); there are no others. Exit 2.

## Gates I ran at bced6df
- Node CI list: 84/84. Harness tests: 175/175. driver_rule self-test: 0. Tree check: 0.
- NOT RUN by me: cargo gates, HUD1, SEL1. The builder's claims for those remain leads only.

## Findings
- M1: the scan misses 5 forms (item 3).
    Repair: put one harness helper `waitForSync(page, fn, arg, opts)` in front of every wait. It throws when the predicate result has `.then`. Then make the scan red on ANY `waitForFunction` token outside that helper; this closes the computed key and the alias too.
- M2 (pre-existing, not L0-DBG): the convergence tick check is not load-bearing in a static world (plant B green).
    Repair: use a case world that changes, or compare the ticks against the server tick log. Owner: qa-lead.
- L1: params-contract is not in the CI node list, and the fix hides the SERVER_BIN contract.
    Repair: add it to ci.yml:114. Assert argv[0] for both the TARGET branch and the PREPARED branch.
- L2: an old SH1 flake (item 6). Owner: qa-lead.
- L3: with 20 ms polling, each poll queues one waiter and one 10 s timer. That is about 12 per build normally, and up to about 500 while a frame is held. Repair: start a request only when none is pending.
