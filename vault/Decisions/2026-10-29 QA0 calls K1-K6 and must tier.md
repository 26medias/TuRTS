---
type: decision
date: 2026-10-29
tags:
  - turts
  - decision
  - qa0
ai-first: true
source: mcp
status: approved
---

## For future Claude
The CEO decided the six QA0 design calls and the build tier on 2026-10-29: **"all as recommended, must only"**. Design: `docs/superpowers/specs/2026-10-29-qa0-instrument-design.md`, section 1 table and section 10.

- **K1 Schema growth = O1.** Keep `deny_unknown_fields`. Each new field gets `serde(default)` and a patch bump of `ENGINE_API_VERSION`. A version table checked in `PackageArchive::content()` and a nearest-name hint for a typo. O2 (field table) is rejected for cost: 4 edits per field.
- **K2 Red browser tests.** `t7.js` and `m3-human.js` stay red in the ledger. The T7 split is in the "can slip" tier, so it moves to MC, which writes the first real pixel driver.
- **K3 Library packages.** A Library exports zero maps, never boots as the root of a match, and can export definitions (strict reading of S:229 Decided).
- **K4** QA0 and MS carry no coverage table and no Player path section (`client: no`).
- **K5** The server serves libraries at MP2. At QA0 a game that declares a dependency fails with a clear "not provided" error.
- **K6** Only the CEO changes the CORE list, by a line in this decision log, after a `game-designer` proposal.
- **Tier: must only.** Estimate 22 to 26 agent runs. Slipped to MC: the T7 driver/oracle split and harness, the CI process test of the T7 driver, moving the M3 version gate into the version table.

**Why:** the register called QA0 a short slice; the slipped items are rewritten by MC anyway.

Next: plan by `qa-lead`, gate 2 by `spec-rigour-reviewer` and `mod-runtime-engineer` (same panel as gate 1), then build on Sonnet.
