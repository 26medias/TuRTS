# Flat M2 browser schema repair

Commit: 66eb2cc4, branch codex/browser-debug-schema, clean worktree /home/julien/Projects/TuRTS-browser-debug-schema. Based on main2977821e69c922b122b270f89d9c83140a33b887. No main merge/push or production change.

Cause: GitHub CI run43807318401 at fc16df8e, browser job133100947588, reports11 semantic failures and0 infrastructure failures. Every assertion diff contains only the additional world key `terrain`. M4 client debug.rs emits terrain_json(replica), which is explicitly null for the unchanged flat M2 base/haul/acme/cedar packages. The exact test allowlist predates that accepted schema. Current main has the same debug implementation. New run44007208398 was still running when checked; this repair is not in that run.

Fix: tests/browser/m2-oracle-lib.js accepts the declared terrain key and requires its value to be null for these flat fixtures. Unknown top-level/unit fields, foreign unit orders and fog knowledge remain rejected. The historical pre-MC fixture retains its exact legacy schema. The independent audit projection also includes terrain:null, so exact owner-world comparison rejects unexpected terrain instead of discarding it. Updated synthetic current snapshots and added explicit missing/non-null terrain negatives in schema and audit tests. Five test/instrument files changed; no engine, protocol, fixtures, thresholds or scenarios changed.

Validation:
- New projection check first failed against the old allowlist with the named replicated-world assertion and added terrain key: projection-red.log/.exit (exit1).
- node --test tests/browser/m2-projection.test.js tests/browser/m2-audit.test.js tests/browser/m2-oracle.test.js:51PASS,0FAIL, exit0. Existing private-field and foreign-order negatives remain green. See helpers-green.log/.exit.
- Downloaded actual CI artifact14200441251, SHA25602952f5fed8fabb46d254e16cff93f0b046436e737b1695039bde35438783c36. Repaired strict schema accepts159 real captured snapshots across the11 failed cases; each case has at least one checked snapshot. See ci-captured-schema-green.json and ci-captures/. This is schema replay, not a claim that all11 complete case oracles passed.
- Fresh actual browser: bash scripts/m2-browser.sh --case E9-browser-base-selection, M2_PREBUILT=1, exit0/PASS. Source public base package repacked by the normal runner; true recipe times unchanged. Four fresh current snapshots contain explicit terrain:null. Fresh Playwright profile, private Xvfb, explicit --alsa-output-device=null. See fresh-selection.log/.exit and fresh/run-20261117T031250Z-vqzN21/ including normal harness captures, provenance and cleanup.
- Reused immutable skirmish-terrain-route-final-prepared bin/dist only after verifying837 engine/client/vendor/Cargo source hashes match current main, no tracked source delta in those inputs, and all bin/dist hashes match the original manifest. Prepared source2073e15e; source main changes after it do not alter those inputs. Reuse proof: prepared-reuse.json. No manifest rewriting, Cargo build or native lease used. A first launch stopped before browser startup because I supplied a nonexistent logs subdirectory; corrected to the bundle root. That infrastructure attempt remains in fresh-selection-log-path-error.log/.exit and its run directory.
- git diff --check passed; commit uses explicit paths; worktree is clean.

Limits: one new complete browser case passed. Remaining complete M2 cases and CI must run on the integrated repair. No performance/capacity claim. Existing M4b/M5b milestone acceptance is unrelated and remains with root.

CI links:
https://github.com/26medias/TuRTS/actions/runs/43807318401/job/133100947588
https://github.com/26medias/TuRTS/actions/runs/44007208398
