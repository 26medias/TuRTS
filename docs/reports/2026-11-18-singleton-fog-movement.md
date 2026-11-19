# Singleton movement under fog

The legacy single-unit movement path used hidden bodies when selecting a destination. The mover could choose a different target before contact. The movement frame now uses the current visibility basis when it selects a singleton slot. The existing spatial index is reused. A hidden body can affect that unit's selection after physical contact. The final collision certificate still sees all bodies. A visible body still reserves its position.

The paired-world singleton test failed before this change and passed after it. Tests also cover a visible body under fog, group dispatch and a 100-tick group approach. A 400-tick singleton contact test passed without overlap and stopped near the body. All 126 movement library tests passed. The pool-slot clearance path is not covered by these tests.

A release diagnostic used 1,000 units and 500 legacy movers for 20 ticks. With fog it measured 2.309 ms mean and 10.476 ms maximum. With all-visible mode it measured 1.974 ms mean and 12.378 ms maximum. These are single samples with different behavior, not a regression comparison. Raw output is in `target/mc-evidence/singleton-fog-investigation/` in the main checkout. The ignored test `singleton_fog_scale_probe` contains the fixture.

The filter does not add an owner index or exclusion set. Each candidate can perform a binary search over the existing ID-sorted live input frame. The existing spread search has no visit counter. The added visibility checks therefore are not separately charged to a work budget. This is an open limitation. No heap peak was measured. The live ECS input is sorted by unit ID; the private visibility-aware kernel depends on that property. Fog audit replay is not verified. An independent foundation review is still required.

## Pool clearance follow-up

The pool's clearance prefilter also read foreign bodies before the contact-aware slot predicate. A paired index test failed on the previous commit. It passes with the new filter. The prefilter uses own bodies and foreign bodies with the existing contact witness. The final slot predicate and collision certificate remain authoritative. The filter reuses the existing pool index and contact rows. It adds no per-owner pool index.

All 127 movement library tests pass. The occupied-spare performance test uses same-owner blockers. Its earlier foreign blockers were outside contact range and could not legitimately make the mover report blocked. The test still checks that fully occupied known spares avoid repeated pool-wide scans. Mixed-owner pool metadata falls back to a conservative empty clearance index. The possible cost of scanning many such spares is not measured. The earlier Blocked recovery branch still inspects all bodies and needs a separate privacy check.
