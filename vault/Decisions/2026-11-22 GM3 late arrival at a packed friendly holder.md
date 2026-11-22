---
type: decision
date: 2026-11-22
tags:
  - turts
  - m4b
  - movement
ai-first: true
source: tower
---

## For future Claude
A late unit that touches stopped friendly units packed around the holder of its point ends its order idle when they block its path. First physical arrival still keeps the point.

# GM3: late arrival at a packed friendly holder (operator, 2026-11-22)

**Status:** Decided by the operator on 2026-11-22. Relayed by Tomas (pm) in the M4b task T-1. The operator's own words are not in this record.

**Question (M4b row GM3):** a group is sent to a point that another unit already holds. Stopped friendly units are packed around the holder and touch it. A late unit reaches them, and they block its path. What does the late unit do?

**Decision:**
- The late unit ends its order and becomes idle when it touches stopped friendly units that are packed around the holder and connected to it, and those units block its path.
- Enemy units, moving units and terrain keep the earlier rules.
- The unit that arrives first physically keeps the point. The unit id never decides the winner.

**Refines:** [[Decisions/2026-11-12 Contested move points - arrive first]]. That rule stays in force.

**Owners and follow-up:**
- Kenji (sim-eng): the spec edit in `docs/superpowers/specs/2026-11-15-terrain-group-movement.md` and a guard test.
- Final minor list of M4b: the route-around behavior (whether a late unit tries to route around the packed group before it ends idle) is NOT TRACED. Record it, do not assume it.
