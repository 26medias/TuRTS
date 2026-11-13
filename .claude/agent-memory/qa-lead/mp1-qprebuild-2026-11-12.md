---
name: mp1-qprebuild-2026-11-12
description: Q-PREBUILD build (b7ec38f, 1e18d06); mc_prepare.py stale check, exit 66; times; 16 plants red; fault runner not split
metadata:
  type: project
---

- `scripts/mc_prepare.py` (begin, seal, check) + `--prepare DIR` / `--prepared DIR` in mc-browser.sh, mc-pixels.sh, t7.sh, m3-browser.sh, m2-browser.sh. Stale = exit 66 (wrapper uses 2, 3, 75, 124). HEAD is compared strictly: any new commit (even docs) makes a prepared folder stale; re-prepare after the last commit.
- Fingerprint = HEAD + sha of git-listed files under crates, vendor, Cargo.toml/lock, rust-toolchain, .cargo + sha of the package folder + sha of every artefact. Needs a `.git` in the root: the fault runner's scratch copy has none (NOT fixed).
- Times (GM40 row, lane cache mc-mp1-q-prebuild, MC_JOBS=4): prepare cold 582 s, warm 27 to 37 s; default build-inside warm to page served 26.6 s; prepared to page served 0.47 s. KM1 41 s, PT1 38 s, PER1 140 s warm.
- Test `tests/qa/test_mc_prepare.py`: 24 tests, ~6 s, fake cargo/trunk/node shims; 16 plants red (scratch `plants/`, record `docs/research/mp1/plants/q-prebuild.json`).
- Lessons: test helper that reads `proc.stderr.read()` while the child lives hangs the test (an eager argument); `pkill -f` with a pattern in my own command line kills my own shell; runner needs MC_JOBS 1..4 for the browser scripts (6 gives exit 2).
- Fault runner (tests/m2/refutations.py) still builds inside the held command: needs per-phase prepare after the plant, a lock-only wrapper mode (scratch has no .git for exec_in_lane), and a no-git fallback in mc_prepare.py.
