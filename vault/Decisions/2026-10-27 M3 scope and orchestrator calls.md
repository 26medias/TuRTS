---
type: decision
date: 2026-10-27
tags:
  - turts
  - decision
  - m3
ai-first: true
source: mcp
status: proposed
---

## For future Claude
M3 (industrial loop) design and plan are written and passed gate 1 and gate 2. The user asked the orchestrator to write the spec and plan and to have a Codex agent implement them. The user has **not** answered the eight calls below. Do not start implementation until the user confirms or changes K1 to K8.

**Who:** orchestrator, on three brainstorm reports (game-designer: content and demonstration; simulation-core-engineer: engine design; recon: M2 code seams). Gate 1 reviewers: qa-lead (instrument), technical-director (architecture).
**Spec:** `docs/superpowers/specs/2026-10-27-m3-industrial-design.md`. **Plan:** `docs/superpowers/plans/2026-10-27-m3-industrial.md`. **Research:** `docs/research/m3/`.

**The calls (spec section 1.1):**
- K1 minimal combat (health, hitscan weapon with ammunition, destruction) moves from M7 into M3.
- K2 machines are placed by the map; no player construction; `Remove` and `SetEnabled` commands exist.
- K3 removal destroys and counts contents; it is the only M3 removal policy.
- K4 engine API 0.2.1, additive, serde defaults; it refines M2D:98 and M2D:509.
- K5 wire protocol 2 to 3.
- K6 research reuses the M2 production queue and `ReserveThenCharge`; the repeat, duplicate and stacking rules are M3 engine prototype limits and do not close the Open policies in spec S:428.
- K7 an inserter picks only when the destination has room now.
- K8 an enabled machine restarts in the same tick as a successful cancel.

**Gate 1 outcome:** qa-lead found four HIGH instrument defects (C15 plant could not go red, C7 plant stayed green, five cases had no named check, the oracle read sim output). technical-director found one HIGH (research rules were labelled package-scoped but are engine rules). All fixed in the spec and re-checked. **Gate 2** found two HIGH plan defects from technical-director (circular Tasks 3 and 4, builds that break outside a task's file list) and two from qa-lead (DMG per operation cannot see C15; health trace counts one hit short). All fixed; each reviewer re-checked its own findings. Last verdicts: technical-director pass-with-changes (0 High, leftovers fixed), qa-lead pass-with-changes (1 High, fixed afterwards without a further check).
