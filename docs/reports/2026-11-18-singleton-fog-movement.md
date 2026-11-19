# Singleton movement under fog

The legacy single-unit movement path used hidden bodies when selecting a destination. The mover could choose a different target before contact. The movement frame now uses the current visibility basis when it selects a singleton slot. The existing spatial index is reused. A hidden body can affect that unit's selection after physical contact. The final collision certificate still sees all bodies. A visible body still reserves its position.

The paired-world singleton test failed before this change and passed after it. Tests also cover a visible body under fog, group dispatch and a 100-tick group approach. A 400-tick singleton contact test passed without overlap and stopped near the body. All 126 movement library tests passed. The pool-slot clearance path is not covered by these tests.

A release diagnostic used 1,000 units and 500 legacy movers for 20 ticks. With fog it measured 2.309 ms mean and 10.476 ms maximum. With all-visible mode it measured 1.974 ms mean and 12.378 ms maximum. These are single samples with different behavior, not a regression comparison. Raw output is in `target/mc-evidence/singleton-fog-investigation/` in the main checkout. The ignored test `singleton_fog_scale_probe` contains the fixture.

The filter does not add an owner index or exclusion set. Each candidate can perform a binary search over the existing ID-sorted live input frame. The existing spread search has no visit counter. The added visibility checks therefore are not separately charged to a work budget. This is an open limitation. No heap peak was measured. The live ECS input is sorted by unit ID; the private visibility-aware kernel depends on that property. Fog audit replay is not verified. An independent foundation review is still required.

## Pool clearance follow-up

The pool's clearance prefilter also read foreign bodies before the contact-aware slot predicate. A paired index test failed on the previous commit. It passes with the new filter. The prefilter uses own bodies and foreign bodies with the existing contact witness. The final slot predicate and collision certificate remain authoritative. The filter reuses the existing pool index and contact rows. It adds no per-owner pool index.

All 127 movement library tests pass. The occupied-spare performance test uses same-owner blockers. Its earlier foreign blockers were outside contact range and could not legitimately make the mover report blocked. The test still checks that fully occupied known spares avoid repeated pool-wide scans. Mixed-owner pool metadata falls back to a conservative empty clearance index. The possible cost of scanning many such spares is not measured. The earlier Blocked recovery branch still inspects all bodies and needs a separate privacy check.

## Blocked recovery follow-up

A live paired-world test confirmed that a hidden remote body could change the escape target of an overlapping blocked unit. The blocked recovery predicates now use the permitted-body filter for target selection. The existing physical overlap and collision checks still use every body. Paired tests pass for hidden and visible remote bodies. The escape search has no independent work meter. This change adds bounded lookups to its existing candidate checks but does not enforce a new total work cap.

A separate failing probe shows a hidden remote unit with the same movement target can still change blocked recovery. The target-claim counter includes every owner's target. The owner-local target-claim fix closes that leak. The hidden-enemy paired test and a same-owner control pass. In all-visible mode the two counts are added. The live protocol has two slots. The existing claim-map value stays eight bytes on native, and grows from four to eight bytes on wasm for each distinct target. No heap peak was measured.

After the target-claim change, all 127 movement library tests and both cross-owner same-point tests pass.

## Recovery work measurement

At integration commit `6aa77235`, an instrumented release fixture filled a 16-by-16 map with 256 own stationary bodies. It placed 100 own blocked movers in overlap, to exercise recovery. One fog tick generated 351,276 escape candidates and 22,115 permitted-body lookups. It took 8.237 ms in one run. The allocator observed a maximum of 820,956 extra bytes above the start of that whole tick. This was a stress fixture, not a normal game or a GM7 result. It does not prove the 8 MiB traffic phase limit.

The 351,276 candidate count exceeds the existing 65,536 owner route-work envelope. The all-visible control also generated 351,276 candidates, so the ring search predates the fog filter. The fog filter adds the permitted-body lookups. Results and the diagnostic patch are in `target/mc-evidence/singleton-fog-investigation/` in the main checkout. A simple cap would restart incomplete searches and can starve movers. No such cap is included. The existing continuation spike also failed its liveness check; see `target/mc-evidence/m4b-route-spike/REPORT.md`.

The same investigation found that a hidden body changed plain blocked recovery. The plain escape search now applies the permitted-body filter. The physical escape path still checks all bodies. Hidden and visible paired-world tests pass. All 127 movement library tests pass. The plain and pool escape searches still need bounded, fair continuation.
