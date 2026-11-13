---
name: mp1-forward-mc-review-2026-11-12
description: Review of the MC forward merge into MP1 (37c41e70): hunk-containment probe, reorder-window faults (job records of a held delta unpinned), sound-scan blind forms, p1-probe run.sh
metadata:
  type: project
---

Head 37c41e70 (merge 961357e0 = 4553acc4 + 9d2ab86b, base 113e380d). Lane TuRTS-mp1-forward-mc-review, cache mc-mp1-forward-mc-review.
- Merge probe: scratch `contain.py` reverse-applies `git diff base side` per file with `patch -R --dry-run -F0`; every FAIL hunk then checked by grep. All MC and MP1 items present (context-only rejects).
- Gates green at 37c41e70: test 2106/0/40, node 421/421, plants --check 428/0, probe_neutral d0d1b1c572163424 / aa9db740578ed5c1, stream bcbf72c2...
- Reorder faults (turts_client full tests): out-of-chain drain red 2 (ordw1:320, t8:474); hold past grace red 12; snapshot ahead_ok no grace red 1 (t8:573); held delta loses unit_orders+orders_cleared red 1 (ordw1:320 only); held delta loses records.jobs: GREEN (no test).
- Sound scan (silent-chrome-scan.test.js) misses: `chromium.launch.bind`, `chromium['launch']`, a name SILENT_ARGS imported from another file, a helper in another file, shell `"$BIN" --x`, `google-chrome-stable --x`, `google-chrome "$URL" --x`, null words only in an echo. Real miss: docs/research/mc/p1-probe/run.sh starts headed Chrome with no null device (out of scope dirs too).
- Watcher v2 (scratch watch2.py): pw-dump S1/S2 + /proc Chrome audit with own/foreign by ppid chain; TERM to wrapper pid on a hit. A foreign Chrome of another lane matched C1 once (argc 2, gone before inspection).
- Pitfall again: `pkill -f <scratch path>` killed my own tool shell (exit 144). Use pgrep with `^python3` anchor.
- Bash tool: foreground timeout over ~115 s moves to background; keep waits at 110 s.
- Headed (pre-merge, wait for empty queue AND load < 12 before the call): rows A+B 28/28 runs, M2 convergence 8/8 at the first poll (289-332 ms), owner-offset red (29 shared ticks of 32 polls) then green, T7 2/2; 0 resync; max tick gap 56.6 ms; sound 0 lines.
- Lesson: a fault or plant restore changes mtimes, so cargo rebuilds; prebuild again after faults or the wasm rebuild runs inside the GPU lock.
- Lesson: a load check before the wrapper is not the load at the lock; a pre-merge queue of another lane (short E9 calls) and table calls slip in between my calls.
- M2 runs have no server tick log or --stats; the client counter is `resync_requests_sent` in scene-checkpoints.json.
