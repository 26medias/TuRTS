---
type: decision
date: 2026-10-26
tags:
  - turts
  - decision
  - m1
  - quality
ai-first: true
source: mcp
status: active
---

## For future Claude
M1 does not close until five test instruments that stayed green on real defects go red on qa-lead's own plants. "Presents" in M1 means the rendered scene, not only the client's presentation state. Studio call.

**Who:** orchestrator, on the M1 verify by qa-lead (Fable, 18 own plants, 5 stayed green) and mod-builder (Opus, third package `zeta` built from docs alone: worked end to end).
**Green plants:** browser crate depending on the simulation (T1); an entity spawned on slot claim (T3); a browser build with no renderer (T7); a map bound hard-coded to 64 (T4); placements ignoring their owner slot (no test).
**Fix:** Part A (Opus) strengthens each instrument and re-runs QA's plant to red. Part B (Sonnet) applies the modder's polish: `turts-pack` validates before packing, orphan visuals are errors, the client exits non-zero on rejection, errors list allowed names, usage text and docs for every hook.
**Re-check:** qa-lead re-runs its five green plants after the fix.
