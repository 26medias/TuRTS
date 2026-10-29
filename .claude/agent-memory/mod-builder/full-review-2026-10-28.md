---
name: full-review-2026-10-28
description: Full spec gap audit (2026-10-28, HEAD 5461486, API 0.2.1); turts-pack probe results for C&C 16.2 and SC-like mods; three M3 rules block combat/construction/victory
metadata:
  type: project
---

Verified by turts-pack runs at HEAD 5461486 (ENGINE_API_VERSION 0.2.1), 2026-10-28:
- "weapon: weapons must be static" — no armed mobile unit (M3 design:139).
- Producer + health: "a unit with health requires removal"; producer + removal: "a unit with a queue
  cannot be removed" — command centers and barracks are indestructible, so 16.2 victory cannot happen.
- ProductionDef: "output must be mobile and cannot be a producer" — no path to make a building.
- Commands are 6 fixed engine variants (turts_net_protocol/src/messages.rs:54-78): no attack/attack-move/build/repair.
- Unknown capability string ("scripts.server") packs, exit 0. A Library-kind package with units+map packs, exit 0.
- Dependencies: "dependencies are not supported at M1" (stale text at M3).
- Error messages now list allowed fields/variants (good; closes earlier "no expected-field list" note in [[m1-test-zeta]]).
- Score: C&C 16.2 50 % weighted (5/17 full); SC-like 29.5 % (3/22 full). No milestone owns 16.2.
Probe kit: scratchpad review/modprobe/{cnc,probe.sh} of that session (throwaway).

**Why:** CEO-ordered full review after core RTS loop was found missing.
**How to apply:** at the next gate re-run the same one-field probes; check each of the three M3 rules closed.
