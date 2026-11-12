---
type: decision
date: 2026-11-11
status: decided
tags: [turts, mc, orchestrator-ruling, simulation]
---

# O16: a harvester delivers its cargo before its gather order ends at an empty node

## The defect

Found by hosted run 43121618105 on the slow hosted runner: the baseline of fault case E9-midtrace-rearm-ignored failed with "automatic operation 6744: gather state differs from the independent full transition projection". Traced by RUN134 and RUN141. When a resource node became empty, the server ended the Gather order with `NodeDepleted` at once (crates/turts_sim/src/orders.rs:1016-1027), also for a harvester that carried cargo to the drop-off. The unit stopped where it was and kept its cargo for ever. No resource was destroyed (a per-tick conservation check holds), but the cargo never reached the receiver. Only a long run reaches an empty node, so only the slow runner found it.

## The texts

- MC design 5.3 (line 536): "`node_depleted` when the node is empty; the unit goes idle".
- M2 economy design (lines 279, 296): "A full cargo or depleted node initiates delivery, even for a partial load"; "never discard".

The texts do not order the two rules for a unit that holds cargo.

## The ruling

When the named node is empty and the active cycle holds unreserved cargo, the unit delivers first. Then the order fails with `NodeDepleted`, and the unit is idle at the receiver. With no cargo, the order fails at once. The unit does not seek another node (the seek is yardstick item EC05, in MP1).

A player can see this change: a harvester finishes its trip before it stops. It agrees with both texts and with what a player expects. The CEO can reverse it.

Alternative not taken: keep the old behaviour and make the independent model predict it. The stranded cargo would stay, and MP1 puts harvesters at the centre of the economy.

## Not changed (recorded)

Player commands that replace a gather order (Stop, Hold, Move, Attack, the legacy MoveTo, a Gather on a node that is already empty) keep the cargo on the unit, as the M2 design says (lines 302-304). The death of the receiver or of the gatherer while it holds cargo is NOT TRACED; the review of the repair traces it.

## Evidence

Repair commit 43282fe on codex/mc-fix-deplete (simulation-core-engineer, RUN141): native test crates/turts_sim/tests/gather_depletion.rs red on the old sim (3 of 4 cases) and green on the new; a new in-process E9 test that reaches depletion on every run without wall time; 1,772 tests passed; the frozen stream hash is unchanged. Independent review in progress.
