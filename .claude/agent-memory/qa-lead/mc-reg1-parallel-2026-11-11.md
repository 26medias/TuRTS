---
name: mc-reg1-parallel-2026-11-11
description: RUN125 parallel refutation runner (external, target/mc-evidence/root-regate) - cargo relocation hazard, measured wall times, serial-equal verdicts, GPU lock facts
metadata:
  type: project
---

Verified 2026-11-11 in worktree TuRTS-mc-reg1 at 1e422bc (cargo 1.99.1, 28 threads, ext4: no reflink).

- Serial harness (tests/m2/refutations.py, tests/m3/refutations.py): one scratch + one COLD
  cargo target per run (refutations.py:224-225, 413), CARGO_BUILD_JOBS forced to 2 (:30),
  fail-fast at the first failed case (:378, :414-418). Browser: m2-browser.sh M2_BUILD_JOBS 1-2 only (:22).
- Cargo is relocatable: a moved source + moved target is fully Fresh. HAZARD: fresh test
  binaries keep the seed paths of env!("CARGO_BIN_EXE_*") / CARGO_MANIFEST_DIR (seen with grep
  on forge_cases binary). turts_server tests find turts_client next to CARGO_BIN_EXE_turts_server
  (support/mod.rs:263-270), so E9 client plants would run the SEED client. Fix used: delete the
  native fingerprints of workspace packages per worker, and move the seed away (fail loud).
- Measured: m3 all serial 393 s vs parallel 107 s (17/17 identical status, detail, 3 hashes).
  ci-native parallel 1666 s, 10 workers, 33/33 red-restored; serial sample of 4 (E1,E6-carried,
  E7,E9-midtrace) 1584 s, identical. Hosted CI ci-native step on main bb611bd: 4218 s, failed.
  E9 loss green test alone runs 216-337 s; it is the critical path (case 900-1100 s).
- t21 serial ci-browser at 1e422bc (t21-run118-gref-ci-browser-2): 4670.8 s exit 0, 8/8.
  E9-browser-position-only-snapshot red text holds a varying tick (217 vs 239): compare by
  expected_failure, not by full text.
- wasm-release link of turts_client_web: 7.7-9.5 GB RSS, single thread, >200 s (fat LTO, cgu 1).
- flock on gpu-runtime.lock is not FIFO; other lanes hold it during their own builds.
- A waiting `exec flock` keeps the env of the wrapper version at launch: the --mute-audio
  defect (about one hour, 2026-11-11) stayed in waiters launched before the fix. Check
  /proc/<pid>/environ of waiters after a wrapper fix.
- pkill -f / pgrep -f with a pattern that is in your own command line kills your own shell (exit 144).

- ci-browser parallel (8 workers, 3 wasm slots): 3746 s wall, 8/8 equal to t21 serial (7 exact,
  position-only-snapshot same reason); only 554 s inside the GPU lock, the rest was lock wait
  (other lanes hold it, one ran its 75-min serial ci-browser inside the lock).
- Fault build: product wasm-release 209-237 s, 10.3 GB; env CARGO_PROFILE_WASM_RELEASE_LTO=off
  + _CODEGEN_UNITS=16: 91-110 s, 4.2 GB, same reds 3/3, baseline green, 52-59 fps (product 51-59).
  wasm-opt off (index.html data-wasm-opt="0"): 23 s, wasm 184 MB; verdicts not run.
- Two headed T7 at once (8.4 s overlap): both PASS, 45/46 frames, 31/32 moving; alone 50/46, 35/32.
- Template hazard: never reuse a cargo template across HEADs or after a plant (mtime freshness);
  the runner stamps HEAD/status/profile env and refuses a mismatch (red seen on a changed env).

See [[mc-t22-run112-2026-11-11]].
