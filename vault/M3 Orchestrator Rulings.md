# M3 orchestrator rulings

Recorded for milestone review; extend this record if further rulings are needed. Original user-approved K1–K8 choices remain in the scope decision. These implementation rulings preserve that scope.

| Ruling | Reason | Cost if wrong |
| --- | --- | --- |
| Resolve M3 definitions in T1 before T2 validation | Archive/catalog readers already require resolution; keep the new logic in content_m3.rs. | Move helper code and adjust task ownership. |
| Retain actual definition filenames in ContentCatalog.source_files | T2 must report authored filenames, including safe names differing from content IDs. | Replace additive metadata with a separate diagnostic context. |
| Require explicit BeltLine construction; omit Default | A default ContentId would be invalid. Empty table containers still have Default. | Update callers to use an explicit constructor. |
| Introduce the private ItemTable during T3 | Removal needs ItemIx-to-ContentId conversion before T4 populates it. Index zero remains empty. | Relocate a private mapping helper. |
| Distinguish strict typed M3 endpoints from legacy u32 wrappers | M3 integrity failures and legacy UnknownInventory behavior have different contracts. | Adjust private callers; no schema or wire change. |
| Select the nearest node before testing item compatibility | The spec names nearest geometry, with NodeId as tie-breaker. | Revisit mixed-deposit map expectations. |
| Require conveyors in map.belts, including single pieces | Explicit line ordering is required; ordinary placements cannot infer adjacency. | Move a map placement to its belt declaration. |
| Return Result from the private research modifier helper | Checked arbitrary additions need an overflow channel. | Revise private helper/tests/combat callers. |
| Validate positive inserter durations and representable tick counts | Zero or more than u32 ticks cannot complete the specified cycle; zero weapon cooldown remains supported. | Redesign counters/cycle semantics for instantaneous or wider timings. |
| Retain authored optional facing during authoritative boot | The later snapshot consumer needs data previously discarded after binding. | Revise private storage and internal snapshot construction. |
| Expose checked weapon damage as Result on Protocol 3 | Preserve ArithmeticOverflow explicitly and use completed-world ownership for export. | Revise the wire field and client presentation together. |
| Build isolated T13b/T14 lanes alongside frozen C2 | Dependencies were accepted; canonical merges stayed checkpoint-gated. | Rebase and repeat dependent lane checks if C2 requires repairs. |
| Treat external raw-view visibility as a structural compile contract | Intentional E0603 plus restored compilation proves public accessibility, separately from semantic plants. | Replace the instrument with a dedicated compile-test harness. |
| Integrate T15a while T14 remains under repair; expand C3 scope | W11 explicitly permits that overlap; T15b remains isolated until C3 passes. | Repeat validation against a larger union or rebase dependent UI work. |
| Make C1 capacity loss a three-layer fault and audit 100 retries | One deleted helper guard is masked by phase clipping and transaction preflight. | Maintain a more complex fault registry and longer scenario; production guards remain intact. |
| Force local source freshness after copied caches; isolate mutant targets | Newer artifact mtimes can otherwise run another source tree's binary. Invalid runs are excluded. | Additional local recompilation time. |
| Freeze native coordinator flags and completion observation semantics | The coordinator observes owner-permitted research, then signals the opponent locally; observation ticks are distinct from server event ticks. | Revise CLI/process harnesses together while preserving private research. |

18. Allow T15b to update the shared browser audit projection helper and its existing schema test. A real M2 browser comparison exposed missing facing/industrial fields in the expected projection. Expected values still derive independently from audit or explicit absent-family defaults. Cost if wrong: revise ancillary test projection/schema code; product policy is unchanged.

19. Allow T15b to repair the production-only queue assumption in client validation, with machine/research-capable queues and item-recipe/technology jobs validated against acquired content. The real Forge browser cannot apply any snapshot until this is corrected. Preserve ownership/capacity/cost checks and authoritative queue semantics. Cost if wrong: revise private validation/tests; wire and gameplay policy are unchanged.

20. Promote the isolated industrial-queue correction as a T15a follow-up before starting C3, and refreeze the checkpoint after R0 and canonical gates. No validator has started; UI presentation stays isolated. Cost if wrong: refresh checkpoint source/inventory and review ranges.

Validation repairs also replaced an idle renamed-clone scenario with the complete industrial sequence, derived balance roles from parsed RON, checked dangling stock before balance, and removed duplicate recursive key traversal. These repair the instruments; they do not add product policy. Budget expansion remains a pending user approval, not an orchestrator ruling.
