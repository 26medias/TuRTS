# M3 orchestrator rulings

Recorded for final milestone review. Original user-approved K1–K8 choices remain in the scope decision. These implementation rulings preserve that scope.

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

21. Start isolated T16 registry/process/CI work on the accepted T14 union plus gated UI candidate while C3 and UI R1 run. Canonical merges and final evidence remain acceptance-gated; consume UI repairs before final gates. Cost if wrong: rebase and repeat affected acceptance, with no unreviewed canonical merge.

22. C5 uses phantom output credit plus a private capacity-preflight bypass so the full OutputBlocked buffer commits the fault and reaches the specified BAL-shell check. Correct preflight would otherwise reject cancellation first. Cost if wrong: an extra two-file fault definition; correct engine guards remain intact.

23. Move pure SHOT validation after DANGLING and before aggregate balance checks, retaining every invariant, so C13 reports its named SHOT failure. Cost if wrong: diagnostic priority/test fixture maintenance; no product behavior change.

Validation repairs also replaced an idle renamed-clone scenario with the complete industrial sequence, derived balance roles from parsed RON, checked dangling stock before balance, and removed duplicate recursive key traversal. These repair the instruments; they do not add product policy. The user approved an allowance of75 agent runs; this was user authorization, not an orchestrator ruling.


24. Update the two stale M2 mutation anchors from `inventory` to the accepted `inventory_legacy` accessor, preserving selected tests, commands and expectations. This is an equivalent private accessor rename. Cost if wrong: repair mutation setup; no engine behavior change.

25. Update inherited native and browser M2 expected projections with the new industrial/research fields, empty health trace, absent facing defaults and Main inventory port. Preserve all old values and independent full-object assertions. The real M2 process and browser checks exposed stale expected schemas. Cost if wrong: revise test fixtures and projection helpers; no product behavior change.

26. E4/E6 capacity-loss plants bypass both the local legacy guard and central transaction capacity preflight in a private two-file mutation. A local-only bypass stays green because the central guard independently rejects the invalid transfer. Add grouped mutation support while preserving the original fixture, selected test count and failure assertion. Retain and exclude the surviving single-guard run from credited refutations. Cost if wrong: more registry and restoration complexity; correct production guards remain intact.

Proof limits: defensive client panel tests may mutate a resolved catalog beyond what the package loader accepts; they establish panel-boundary behavior, not a valid-package overspend exploit. Compiler failures, infrastructure failures, zero selected tests and surviving mutations are never credited as semantic refutations.

27. Browser label-fault checks wait for the selected replica unit ID, then capture and assert the exact rendered label separately. Combining selection readiness with the expected semantic label caused an incorrect label to become a timeout before the assertion could run. Keep the original exact label expectation; preserve the previous timeout as uncredited evidence and verify baseline/plant phase provenance before rerunning. Cost if wrong: revise browser harness readiness and repeat affected proofs; product behavior and the label requirement remain unchanged.

28. Start Task16 R1 on clean frozen WIP `35355c7` while the long legacy browser fault sweep continues in its previously copied scratch tree. Native workspace, M3 faults, actual F11, T7 and Forge browser proofs are available. Final acceptance still requires every remaining gate to pass; subsequent implementation changes require scoped review. Cost if wrong: re-review changed code and repeat affected proofs, consuming the approved run allowance; no requirement is waived.

29. Preserve same-kind re-arm while the first recovery response is held, and check the exact request budget at final re-arm recovery. Sol traced the recovery matcher and snapshot rebasing and verified that browser polling can observe both recoveries together. The former intermediate `+1` check therefore races a legitimate second recovery. Require final `requests_sent == requestsDuringHold + 2`, `successor_requests == 1` and `max_outstanding_requests == 1`; preserve held-phase exact counts, no active/outstanding requests after recovery, both distinct extraction losses and arm ticks, both audit transitions and the final full-object comparison. Rename evidence that falsely describes a first-only observation. Cost if wrong: revise the harness and repeat affected proofs; no gameplay code or required invariant changes. The earlier baseline failure earns no mutation credit; retain five completed cases with precise prior-harness provenance and rerun the remaining three, followed by all15 positive browser cases.

30. Run C4 native validation on the immutable, Sol-reviewed integration preview while the remaining legacy browser proofs finish in their separate scratch tree. C4 browser work waits for the existing GPU queue. Final Task16 and checkpoint acceptance still require every gate; a later implementation change requires reconciliation and repetition of affected validation. Cost if wrong: additional scoped review and validation on corrected source. This overlaps independent work and does not waive a gate or authorize a push/main merge.

31. Use the current builder-executed positive15 suite as one joint final F2 gate, with independent C4 audit of exact source/run hashes, all15 case counts, raw receipts, audit comparisons, screenshots and unchanged assertions. Label execution and audit separately. C4 still freshly executes native33, formal17, processF11, Forge browser7, T7 and new-function causal probes. Any concrete F2 gap requires replay of affected cases. Identical second execution solely for a role label adds no new evidence. Cost if wrong: replay affected M2 browser cases before acceptance; no required case or assertion is dropped.

32. Require `extractor.interaction_range`, `inserter.reach` and `weapon.range` to be finite and nonnegative in both the public M3 package validator and direct resolved-catalog bootstrap, before geometry or world installation. Preserve source filename, package and exact field diagnostics; allow zero and impose no new upper budget. Cost if wrong: revise the domain check and tests; no gameplay algorithm, balance, wire or API-version change.

33. Correct the Forge playtest guide's press/turret/lab positions, describe the lab's lack of visible queue progress and point to Last command Applied followed by completed research; mark the M2 authoring example historical and implemented with the current backward-compatible loader, linking M3 authoring. A two-page same-context foreground probe with the accepted C4 build rendered both pages and real pointer selection on each, so no UI workaround is authorized. Cost if wrong: revise documentation or the probe; no client behavior change.
