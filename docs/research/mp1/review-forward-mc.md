VERDICT=PASS FINDINGS=3 HIGH=0 RUNS=42 NOT_CHECKED=5

# Review: forward merge of MC (9d2ab86b) into MP1 (4553acc4), head 37c41e70 (QA lead, review mode, 2026-11-12)

Lane `/home/julien/Projects/TuRTS-mp1-forward-mc-review` (detached at 37c41e70), cache `target/mc-mp1-forward-mc-review`.
Merge base 113e380d. Scratch: `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/forward-mc-review/`.
The rules of this seat forbid a write in a repository tree, so this file is in the scratch folder, not in `TuRTS-mp1/docs/research/mp1/`.

## Findings

1. Medium. `crates/turts_client/src/replica.rs:441` (the hold: `self.reorder.push(delta.clone())`). No test pins that a held
   delta keeps its records other than the order records. Fault F2 (the held copy loses `records.jobs`): `cargo test -p turts_client`
   514 passed, 0 failed (`target/mc-evidence/fwd-rev-fault-f2/command.log`). The report says the window "covers" every protocol 5
   record (forward-mc-report.md:34). Only `orders_cleared`/`unit_orders` are pinned (`tests/ordw1_replica.rs:320`; fault F5 red).
   Fix: one test like `ordw1_replica.rs:320` with a `JobProgress` in the held tick 12: arrival 12, 11 must give the record in `job_progress`.
2. Medium (sound rule; not caused by the merge, in both parents since 2026-10-30). `docs/research/mc/p1-probe/run.sh:45` starts a
   headed Chrome (`setsid "$BIN" --user-data-dir=... --new-window URL`) and `:47` a headed Firefox, with no `--alsa-output-device=null`,
   no `PULSE_SERVER`, no null ALSA configuration. The page opens and resumes an `AudioContext` (`page.html:11,56`); I found no sound
   source in it. The scan does not see it: `docs/research` is not in `SCOPE` (`tests/browser/harness/silent-chrome-scan.test.js:75`),
   and `scriptProblems` gives `[]` on the real file (the regex at `:144` needs `"$CHROME_BIN" "${` or a literal `google-chrome --x`).
   Fix: needs decision (same options as the builder's ruling b): (a) silent form + new sums, or (b) "do not run" mark; and add
   `docs/research` to `SCOPE`.
3. Medium. The scan is a name check. My probe (`scan-defeat.js`, scratch) gives MISSED for: A1 `chromium.launch.bind(chromium)` then
   a call through the variable; A2 `chromium['launch']({...})`; C1 `SILENT_ARGS` imported from another file (any value passes);
   C4 a start helper in another file (the documented gap, `:11-12`); B1 `google-chrome "$URL" --x`; B2 `google-chrome-stable --x`;
   B3 `setsid "$BIN" --x`; B7 the silent words only in an `echo`. SEEN: A3 `.launch(opts)`, C2/C3 args array from another file under
   another name, B4 `/usr/bin/google-chrome --x`, B5, B6. Fix: a runtime check in the headed wrapper (read `/proc/<pid>/cmdline` of
   each test Chrome, as `watch2.py` does), and widen the shell rule to any start of a browser binary through a variable.

The builder's Medium 4 (gate poll 270 ms, no GPU run) is closed by evidence: 8 of 8 real-GPU convergence runs agreed at the first
poll (289 to 332 ms), and the owner-offset plant was red with 29 shared ticks in 32 polls, then green.

## 1. True merge (per-file table: `merge-table.md` in the scratch folder)

- `contain.py` reverse-applies `git diff 113e380d <side> -- <file>` to the file at 961357e0 (`patch -R --dry-run -F0`). MC: 87 of
  109 files whole; MP1: 469 of 495. I read each rejected hunk (`rej-mc/`, `rej-mp1/`) and found its items with `git grep` at
  37c41e70. All are context rejects. No hunk of either side is lost.
- replica.rs: MC `ResyncReason` :61, `MAX_FRAME_DELTA` :77, `max_catchup_ticks` :87, `reorder_grace_ticks` :102/:109; MP1
  `HEALTH_TRACE_WINDOW` :57,:1547. client_shell: MC fields `client_shell/settings.rs:36-50,61,73-75`, MC tests
  `client_shell/tests.rs:2464,2485` (bodies equal; one rustfmt line join). core_adapter/commands/feedback/alerts/scene_tests: only
  `reorder_grace_override: None` added. mc_kill_replica.rs: merged = MP1 (cmp equal); MC file = merged minus the 9 doc lines :8-16.
- acceptance.sh: both sides (connection_end :103, mc_kill_replica :158, background_pace :155, mc_reorder_stall :159,
  p8_one_flush_order :160, run_logged :17). mc-browser.sh:228 `$SERVER_BIN` with `--stats`. t7.sh: speed block inside the build
  branch, so also under `--prepare`.
- ci.yml: line multiset compare of both parents: only the node list line differs; the node list is the set union (65 = 53 + 12),
  every `tests/browser/**/*.test.js` file (65) is in it. New step `ci.yml:50-54` (refreeze self-test and tests) confirmed.
- m2-oracle.test.js: test-name sets: merged 24 = MP1 8 U MC 20.

## 2. Reorder window on protocol 5

- Only `send::<StateChannel>` in the server: `crates/turts_server/src/outbound.rs:81` (`WorldDelta`). `StateChannel` is
  `UnorderedUnreliable` (`turts_net_lightyear/src/protocol_plugin.rs:20-21`); every other send is `ReliableChannel`
  (`OrderedReliable`, :15-16). Records ride in `WorldDelta.records` (`turts_net_protocol/src/records.rs:320-333`).
- My faults (each restored with `cp` from a backup; `cmp` with HEAD equal; `git status` empty; logs `target/mc-evidence/fwd-rev-fault-f*/`):

| fault | edit | red tests (turts_client full run) |
| --- | --- | --- |
| F1 out of chain order | drain takes the first held with tick > applied; no sort | 2: `ordw1_replica.rs:320`, `t8_resync_logic.rs:474` |
| F2 held delta loses `records.jobs` | push a copy with jobs cleared | 0 (finding 1) |
| F3 hold past the grace, no resync | `grace_expired` always false | 12 (`ordw1_replica.rs:356`, `t8_resync_logic.rs:505` and 10 more) |
| F4 snapshot path: ahead deltas kept with no grace | `ahead_ok` without the grace test | 1: `t8_resync_logic.rs:573` |
| F5 held delta loses `unit_orders` and `orders_cleared` | push a copy with both cleared | 1: `ordw1_replica.rs:320` |
| J1 `driver-io.js:318` back to `waitForFunction` | | 1: wait-sync-scan |
| J2 `client-settings-seed.js:40` no request | | 2: client-settings-seed, debug-request-use |

## 3. Instruments

Node list 421/421; Python list check and 19 other Python commands exit 0; `mp1_plants.py --check --all`: 428 apply, 0 stale.
Sample of 6 plant records through the runner (`plants6/sample.json`): 6 red-restored, 0 failed. Merge-touched: L0-DBG/h8
(m2-oracle.js), L0-DBG/m1-async-wait (run.js), ORDW1/(1) (replica.rs). Others: COMBAT-FIXES/1a, L0-CORE/a-swap..., Q-COLD/play-profile-kept.

## 4. Sound

Wrapper `mc_headed_v6.sh:749-752`: `--alsa-output-device=null` in `MC_CHROME_ARGS_JSON`, `PULSE_SERVER` unusable, `ALSA_CONFIG_PATH`
null. Watcher `watch2.py` (pw-dump S1/S2 each 0.25 s, plus a read-only `/proc` audit of each test Chrome, own or foreign by the
parent chain; TERM to the wrapper pid on a hit). Positive controls red (fake ALSA client: line and TERM; own Chrome with no null
argument: line and TERM); null-argument Chrome and idle host: 0 lines. Every own test Chrome in every call: `null_arg=True`.

| call | sound lines | own Chromes with no null argument |
| --- | --- | --- |
| HD1 native reference build | 0 | 0 |
| fwd-rev-rows-a (stopped while it waited, no lock, no Chrome; the watcher file was removed before the count) | n/a | 0 |
| fwd-rev-rows-a-q, fwd-rev-rows-b-q, 8 M2 calls, 2 T7 calls, owner-offset | 0 each | 0 each |

## 5. Headed rows (`mc_headed_v6.sh --mode pre-merge`, MC_LOCK_MAX_SECS=1800; each call waited for an empty queue and load < 12)

| row | pass | resync | longest server tick gap ms | label / load at lock |
| --- | --- | --- | --- | --- |
| SLOT1, MM1, ORD3, ORD4, ORD5, HUD2, AUD1 | 2/2 each | 0 | 51.7, 51.2, 55.7, 55.8, 55.9, 51.7, 56.1 | fwd-rev-rows-a-q, 5.53 (hold 1114 s, includes a wasm rebuild after my faults) |
| ALR1, PER1-binding, SEL3, CAM1, CLICK9, HD1, NOORD | 2/2 each | 0 | 55.4, 50.9, 55.4, 50.9, 50.9, 56.6, 51.2 | fwd-rev-rows-b-q, 6.67 (hold 486 s) |
| base/haul/acme/cedar public-convergence | 2/2 each (8 calls) | client 0 | no tick log in M2 runs | fwd-rev-m2-*-1/-2, 7.85 to 11.76 |
| owner-offset plant | RED then RESTORED GREEN | n/a | n/a | fwd-rev-m2-owner-offset, 4.74 (hold 976 s) |
| T7 | 2/2 | client 0 | 50.8, 51.0 | fwd-rev-t7-1/-2, 9.36, 5.20 |

No red row. Resync for pixel rows: `resync_requests` in server-stats.json (largest of the runs); M2 and T7: client `resync_requests_sent`.

## 6. Gates at 37c41e70 (exec_in_lane_v4, labels fwd-rev-g-*; wrapper exit 0 each)

fmt; clippy -D warnings; `cargo test --workspace --locked` 2106 passed, 0 failed, 40 ignored; wasm32 check; boundary; probe_neutral
`frame_chains=d0d1b1c572163424 plain_unit_frames=71543 pooled_unit_frames=34093`, `muster_world=aa9db740578ed5c1`; stream sha256
bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b; `rust-toolchain.toml` channel 1.99.1.

## 7. Merge check

- origin/mp1/integration (4553acc4) is an ancestor of 37c41e70: fast-forward.
- origin/mp1/l0-store (d4c94308): 1 conflict, `crates/turts_client/src/replica.rs` trial lines 467-511: forward-mc `apply_chained`,
  `drain_reorder`, `check_grace` against l0-store `stage_delta(&current, &self.store, delta)` -> `(candidate, step)` and
  `commit(candidate, StoreUpdate::Step(step), now)`. The window structure (forward-mc) leads; the store API (l0-store) must go into
  `apply_chained` (trial :468,:472). The auto-merged snapshot replay already uses the store form (trial :355,:395).
- origin/mp1/sound-off-default (ee9bd0dd): 3 conflicts. `.github/workflows/ci.yml` trial :264-270 (node list: forward-mc 65 files and
  the MC `if:`; sound-off adds `harness/sound-switch-pixels.test.js`: take the union and the `if:`). `tests/browser/feedback-driver.js`
  trial :5-9 (both requires), :40-74 and :86-99 (AUD1 flow: sound-off leads with `turnSoundOn`; keep the MC overlay handling: dimmed
  ground and bodies), :267-273 (dimmed own colour: MC `dimmed(SLOT_RGB[0])` vs sound-off `DIMMED_OWN_RGB`). `tests/browser/feedback-oracle.js`
  trial :6-10 (both requires).

## NOT TRACED

- `scripts/acceptance.sh` at 37c41e70 (builder ran it; I did not).
- The hosted CI run of this branch.
- That lightyear puts all fragments of one delta in one flush (the pin `main.rs:192` passed in the workspace run).
- Sound during the node gate: the watcher started late and saw 0 test Chromes.
- One foreign Chrome (not under my wrapper) matched the no-null-argument rule at 20:05:39 (argc 2, gone before I read it).
