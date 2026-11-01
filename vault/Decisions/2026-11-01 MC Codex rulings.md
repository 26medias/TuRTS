---
type: decision
date: 2026-11-01
tags: [turts, mc, codex, rulings]
ai-first: true
source: codex
---

# MC Codex rulings

## C161: share two ring topologies without per-unit assets

Conflict: design9.1:864 says one shared ring mesh, while9.1:840-843 requires a solid own ring and a foreign ring with four gaps. The prepared Mesh3d/StandardMaterial renderer represents gaps in geometry; one fixed topology cannot represent both with the same per-slot material. Candidate visuals.rs:28-31,190-191 holds two handles and creates each once.

Choice: use two global shared ring meshes, one solid and one with four gaps, and one global shared bar mesh. Keep one unlit material per slot, no per-unit mesh/material creation, and identical visible geometry. This is the nearest existing render mechanism with the same player-visible shape under brief4. Add a native asset-sharing assertion; actual browser performance is NOT TRACED until VIS2. The design is not edited.

Alternative not taken: lose the foreign gaps, build per-unit assets, add a custom shader/material system only to encode the gap topology, or claim the candidate uses one topology. O13 palette/thresholds are unchanged.14/25complete;65/170runs, no new dispatch.

## C162: reject fire policy on Attack definitions

Conflict: content_mc_validation.rs:1328 permits fire on Move, Hold and Attack, but design14 section5.2:488-498 lists fire only for Move/Hold and engage for Attack. combat.rs:151-160 correctly uses fire only for Move/Hold, so a formerly accepted Attack.fire value would be ignored.

Choice: restrict authored fire to Move/Hold. Reject Attack.fire Some(Never) and Some(InRange) with package/file/field fire; keep Attack.fire None and its required approach/fire mechanism unchanged. Add compiled red-then-green negative validation regression plus positive controls. Shipped Forge/Muster Attack definitions already have fire None; no content, frozen value, design or acceptance criterion changes. This repairs an instrument/schema mismatch under brief4.

Evidence: design5.2:488-498, crates/turts_mod_runtime/src/content_mc_validation.rs:1328-1334, crates/turts_sim/src/economy/combat.rs:151-160, mods/muster/orders/order_attack.ron:1. Reviewer RUN65 raised the edge; builder RUN64 owns the repair and all final gates. Alternative not taken: invent Attack.fire behaviour, silently ignore accepted policy, or weaken audit/validation. Actual new regression is NOT TRACED until builder runs it.14/25complete;65/170runs, no new dispatch.

## C163: connect click markers to real local order intents

Conflict: prepared visuals reads Effect::Marker, but the interaction core declares that variant without producing it. Actual clicks produce Effect::Send (interaction lib.rs:683,844); core_adapter.rs:707 enqueues the envelope, commands.rs:243-310 later assigns sequences. The wire UnitOrderRecord has no sequence. A marker based only on selected units or old Idle can disappear before ACK or never appear.

Choice: extend Task13 ownership narrowly to commands.rs/core_adapter.rs. Create marker intent from each actual Send envelope in the click frame, carrying exact units/target/behaviour and a stable local identity. Carry identity through pending/split sends and correlate actual sequences/results without changing the wire or adding sends. Gate terminal replica checks on accepted result tick, so pre-command Idle cannot end a new marker. Then apply the design unit-rest/Blocked rule and60s limit; handle refused fragments and reset bounded local state on session change. Preserve existing trace/repeat/legacy command semantics. Tests must prove same-frame appearance, stale Idle, split results/refusal and session reset; runtime is NOT TRACED until the Task13 lease.

Evidence: design9.1:856-858 and9.2:875-885; crates/turts_interaction/src/lib.rs:683,844,1091; crates/turts_client/src/core_adapter.rs:707-712; commands.rs:243-310; net_protocol messages.rs:131. Alternative not taken: require a protocol order-id change, show a marker after ACK only, listen to an unused variant, infer its units from a later selection, or invent an arbitrary grace delay. This is the nearest existing mechanism for the unchanged visible contract under brief4.14/25complete;65/170runs, no new dispatch.

## C164: fail closed on an unresolved active firing identity

Conflict: server orders.rs:25-32 rejects unknown authored IDs and captures resolved policy, and economylog.rs:2074 validates it against the immutable audited catalog. The lower-level native OrderCommand accepts trusted resolved requests (orders.rs:460-477); test fixtures intentionally use policy None. Combat's active-order lookup (combat.rs:141-160) conflates a missing definition with valid omitted fire, enabling the InRange default for an invalid identity.

Choice: retain the established trusted-server boundary and native mechanism fixtures. Add a defensive firing guard: an active authored order (including the plain-Move identity) must resolve to a catalog definition with matching behaviour; otherwise it emits no shot. Only a found Move/Hold definition with omitted fire gets InRange default. Units without an order keep the exact M3 rule. Prove unknown/mismatched IDs fail closed and a valid omitted policy still fires, with compiled semantic red before the fix. No new wire/rejection enum, command sends, audit bypass or frozen expected-value changes.

Evidence: crates/turts_server/src/orders.rs:25-32,60-70; crates/turts_server/src/economylog.rs:2074; crates/turts_sim/src/orders.rs:460-477; crates/turts_sim/tests/orders.rs:90-119; combat.rs:141-160. Alternative not taken: permissive fallback for missing identity, reject the existing trusted native fixture API broadly, or claim arbitrary manually forged internal state is a valid wire order. Actual regression is NOT TRACED until RUN64 executes it.14/25complete;65/170runs; no new dispatch.

## C165: non-firing policy does not stop elapsed cooldown

Conflict: combat.rs:160-176 early-continues for valid Move/Hold fire Never and AttackPoint engage None before its normal per-tick cooldown decrement at:228. A unit that fired, moved for one cooldown duration with fire Never, then resumed InRange would wait that duration again. Design5.2 defines fire/engage as targeting/firing policy; it does not pause the existing weapon timer mechanism. The no-order M3 transaction advances cooldown independently of a shot.

Choice: repair this Medium finding now. Valid non-firing policies suppress acquisition/shots but keep the existing fixed-tick timer/status transaction. Keep no-order M3 exact and invalid identity fail-closed. Add focused controls for MoveNever, HoldNever, AttackPoint None with a positive cooldown: timer advances, no forbidden shot, and a later valid firing policy does not incur an extra frozen cooldown. Use deterministic ticks, not wall-clock sleeps. No content, design, wire API or frozen expected value changes.

Evidence: design5.2:488-498; crates/turts_sim/src/economy/combat.rs:160-176,228-248; spec mechanism/policy split:180. Reviewer RUN65 independently classified the source edge, root chose repair within brief4 rights. Alternative not taken: silently add time-pausing semantics to Never/None or defer a simple visible timing defect. Actual new regression is NOT TRACED until RUN64 runs it.14/25complete;65/170runs, no new dispatch.

## C166: keep the Job-support plant compiled and semantic

Conflict: Task5step4 says drop the Job arm of server orders::to_sim. Literal removal from the exhaustive Rust OrderTarget match produces a compile error, which brief4 explicitly does not credit as red.

Choice: for the temporary required production plant only, replace the Job mapping with OrderTarget::Job(_) => OrderTargetSim::Invalid. The real Job envelope no longer has supported simulation Job meaning while the match still compiles. The unchanged every_behaviour_has_an_envelope test must select and fail on its Job target assertion; then exact source restore and selected green. Other arms, input fixtures, expected values, wire schema and production behavior after restore stay unchanged. This is a plant implementation detail under brief4, not retirement.

Evidence: docs/superpowers/plans/2026-10-29-mc-player-control.md:562; crates/turts_server/src/orders.rs:49; brief4 red qualification. Alternative not taken: credit a compiler failure, remove the Job test, weaken expected target, or defer the required plant. Actual plant is NOT TRACED until RUN64 executes it after all gates.14/25complete;65/170runs, no new dispatch.
