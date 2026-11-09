---
type: decision
date: 2026-11-09
tags: [turts, mc, codex, ruling]
---

# MC Codex rulings

## C179: measure the actual hardware WebGPU adapter

Conflict: tests/browser/measure.js:178-198 requires nvidiaAdapter for VIS2 readiness and includes a NVIDIA-specific metadata fallback. Design9.1 requires headed Chrome WebGPU measurements at5000units with no budget; it does not set a GPU vendor. The plan Task13step5 names the desktop RTX3080 as the machine record. The laptop handoff requires honest actual hardware metadata.

Choice: remove the vendor as a pass condition. Keep actual complete adapter metadata, a configured acquired device/context, actual submitted draw calls and texture acquisitions, WebGPU backend, a non-software adapter, headed mode,5000replicated and rendered units, and finite positive frame/memory metrics. NVIDIA may remain diagnostic data. Test alternate hardware vendors and rejected software/incomplete/device-less evidence with bounded pure controls; actual headed measurement is still mandatory. No performance threshold, simulated hardware or fallback measurement is authorized.

Evidence: design9.1 draw-method measurement clause; planTask13step5; measure.js:178-198; [[Reports/2026-11-01 MC laptop handoff]]. Alternative not taken: reject valid Intel/AMD hardware, disguise software rendering as GPU proof, delete adapter metadata, or change the acceptance row.68/170runs;15/25complete.


## C180: retain both work-log histories when importing external docs

Conflict: main advanced externally fromf050e42 tobb611bd through two documentation-only commits. A read-only merge-tree check found one content conflict in vault/Work Log/2026-11-01.md, which would prevent the draft PR merge ref. The incoming side adds CI/transfer history; integration has the full execution log.

Choice: merge main INTO codex/mc-integration, preserve both work-log additions and the externally committed disk audit, then continue the active Task13 lane unchanged. Runtime/design/plan/frozen files are byte-identical before and after this merge. This is branch/document maintenance under brief4, not a merge into main. Normal integration re-gate still follows the later Task13 code merge, before Wave6 push/read. No existing runtime observation is relabelled.

Evidence: main commits eb8d482 andbb611bd; git diff f050e42..main changes only the work log and disk audit; merge-tree reports that one conflict. Alternative not taken: drop either history, alter main, rebase an immutable active runtime tree, or push a PR with an unresolved documentation conflict.68/170runs.
