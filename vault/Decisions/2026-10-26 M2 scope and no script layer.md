---
type: decision
date: 2026-10-26
tags:
  - turts
  - decision
  - m2
ai-first: true
source: mcp
status: approved
---

## For future Claude
M2 is the economy slice only, and it ships without a script layer. Revision 2 passed gate 1 and was approved by the user on 2026-10-26.

**Who:** orchestrator, on three research reports: producer (scope, acceptance tests E1 to E10), game-designer (two declarative economy loops), mod-runtime-engineer (scripting probe).
**Approval:** the user replied “Approved” to the reviewed revision 2 design and economy-only scope. This approves its balances/inventories, gathering, production/payment/refund, owner-only replication, map-boot initialization, identifier/migration and recovery contracts for M2. It authorizes writing the implementation plan; long-term script/runtime/budget choices and the full section 16.2 validation home stay open.
**Scope:** balances per slot; inventories with reservations; finite ore field; gather and drop-off with a mod-selected drop-off policy; unit production with two payment policies (charge on queue; reserve then charge) and explicit refunds. Out: combat, construction, power, repair, the 16.2 victory (home is a CEO question).
**No script layer at M2:** both loops are declarative plus engine behaviours (spec 5.1 layers 1 and 2). The probe (native and wasm32, budget stop shown for each): Rhai 1.27 adds 1.03 MB of wasm and 0.8 µs per call in V8; piccolo (Lua) adds 0.37 MB but its last release is 28 months old; wasmi adds 0.99 MB at 0.2 µs per call but needs authors to compile to WebAssembly, which spec 5.3 defers. M2 adds a Rust event bus with a boundary a runtime can bind later. Rhai stays the prototype candidate. The language choice goes to the CEO when a rule first needs a script.
