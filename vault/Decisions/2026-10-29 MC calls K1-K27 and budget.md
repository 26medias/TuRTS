---
type: decision
date: 2026-10-29
tags:
  - turts
  - decision
  - mc
ai-first: true
source: mcp
status: approved
---

## For future Claude
The MC design (`docs/superpowers/specs/2026-10-29-mc-player-control-design.md`, revision 4, commit `cbe37f5`) passed gate 1 on 2026-10-29 with five lenses (`spec-rigour-reviewer`, `technical-director`, `bevy-rendering-engineer`, `player`, `simulation-core-engineer`) after three repair passes. Gate cost: 6 Opus runs and 17 resumed turns.

**CEO decisions (2026-10-29): "1 full design, go with the recommendations".**

- Budget option 1: the full design, about 184 agent runs (range 165 to 205). The technical director's cut (about 19 runs) is not taken.
- K1 to K27 as recommended in section 1.4: K1 (a), K2 (a), K3 (b), K4 (b), K5 (a), K6 (a), K7 (a), K8 (a), K9 (a), K10 (a), K11 (a) conditional on probe P1 green on Chrome and Firefox, K12 (a), K13 (a), K14 (a), K15 (a), K16 (a), K17 (a), K18 (a), K19 (a), K20 approve all five edges, K21 (a), K22 (a), K23 (a), K24 (b), K25 (a), K26 (a), K27 (a).

**Yardstick change under QA0 K6 (this line is the controlled path).** The CEO approved five dependency edge removals in `docs/research/review-2026-10/inventory-engine.md`: UI05 no longer needs FV02; CB03 no longer needs FV03; PF01 and MP09 no longer need MP01; PC16 no longer needs FV04. `scripts/qa/yardstick.py --extract` rewrote `docs/qa/yardstick-a.tsv` (5 rows changed, 355 rows, 181 CORE unchanged). The MC coverage moves 19 rows from `later` to `in` (81 `in`, 15 `done:`, 85 `later:`). The proposal came from the MC design (section 17.2), not from `game-designer`; the CEO accepted it as the proposal.

**Effects.** The MC registry row is `gated` (`docs/qa/specs.tsv`), so `completeness.py --enforce` checks MC in CI from this commit. Next: the MC plan (`producer`), gate 2 with the gate 1 panel plus `qa-lead`, then a Codex brief.

Related: [[2026-10-29 QA0 calls K1-K6 and must tier]] · [[2026-10-28 Goal statement and full spec review]]
