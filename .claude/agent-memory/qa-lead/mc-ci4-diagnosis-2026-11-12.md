---
name: mc-ci4-diagnosis-2026-11-12
description: Hosted run 43214215645 red jobs are baseline economy/convergence cases timed out by software renderer; m2 suite hid 11 of 15 cases; fix ae04af9
metadata:
  type: project
---

After the cargo-fetch fix (851b39d) the 5 red jobs fail in the BASELINE case: base-economy (5 s recipe vs 8 s
driver step cadence; Cancel button gone), cedar-economy (6 s recipe, 9.6 s), base-public-convergence (windows show
a tick every 28 ticks, phase shifted 16, 0 shared ticks). Green refutations use tick-based baselines (selection,
recovery, loss-final_change). m2-browser.sh default suite aborted under set -e after base package: 11 of 15 cases
never ran; repaired on codex/mc-fix-ci4 ae04af9 with tests/m2/test_m2_browser_suite.py (red 4 of 15, green 15).
Options for GPU families: GitHub GPU runner needs org on Team/Enterprise ($0.052/min; repo owner is a User);
self-hosted on public repo is discouraged by GitHub docs; local gate with receipt. Report at
target/mc-evidence/reviews/ci4-diagnosis.md.
