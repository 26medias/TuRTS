---
type: decision
date: 2026-10-28
status: adopted
tags: [turts, m3, arithmetic, implementation]
---

# M3 checked research modifier

Orchestrator ruling during T10: change the planned private helper return from `u64` to `Result<u64, RejectReason>`. Sum matching completed-technology effects with checked `u128`, then checked conversion to `u64`; overflow returns `ArithmeticOverflow`.

SPEC9 requires a checked sum but the original plan's infallible return cannot report overflow for arbitrary package additions. A checked result preserves that requirement without choosing saturation or an implicit default. T11 has not started and must propagate the failure through its atomic shot path, spending no ammunition or applying damage when effective damage cannot be represented. Research completion remains nonblocking; the helper is computed on demand.

This refines an internal implementation interface. Public package schema and protocol do not change, and K6's prototype rules and product Open research policies remain as specified. If a future design instead validates an infallible aggregate at load time, the rework is the private helper, its tests and its combat caller.
