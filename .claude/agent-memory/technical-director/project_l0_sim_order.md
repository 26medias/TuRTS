---
name: l0-sim-order-ruling
description: 2026-11-12 ruling on the MP1 tick set order (L0-SIM 451f7b6): pre-Commands Supply+Construction, one pay pass in Production, Repair before Combat
metadata:
  type: project
---

Ruling file: /home/julien/Projects/TuRTS-mp1/docs/research/mp1/ruling-l0-sim-order.md (uncommitted at write time).
Frozen order: FixedPreUpdate Advance, Supply, Construction (held-order placement only), Commands; FixedUpdate Income,
Orders, Movement, Gather, Extract, Transport, Inserters, Production (one pay pass: producers and sites by unit id),
Acquire, Repair, Combat, Outcome; FixedPostUpdate Collect.

Verified facts (worktree TuRTS-mp1-l0-sim @451f7b6):
- apply_order has two callers, both in command.rs (:201, :212); server admission only enqueues.
- ProductionQueues is BTreeMap<u32,_> (production.rs:49): producer walk is ascending id.
- Server systems use `.after(SimSet::Advance).before(SimSet::Commands)` (session.rs:120, :1123; command_script.rs:45).
- Gather credits the balance in run_gather (convert, gather.rs:947, :1098): before Production.
- Plan names the Construction slot "construction placement"; L-CON1 takes L-PAY as input and edits production.rs.

**Why:** later lanes and the 6.3 errata depend on this order.
**How to apply:** check any lane that adds a set or moves a charge against this list. See [[mp1-rev6-facts]].
