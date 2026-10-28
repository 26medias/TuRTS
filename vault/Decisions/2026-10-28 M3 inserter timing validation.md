---
type: decision
date: 2026-10-28
status: adopted
tags: [turts, m3, validation, timing]
---

# M3 inserter timing validation

Orchestrator ruling after T8 R1: swing_ms and return_ms must be positive, and each ceil-converted duration at the configured tick rate must fit the frozen InserterState.elapsed u32 counter. Portable validation rejects zero; boot checks positivity and tick representability for direct resolved-catalog callers as well. Errors retain package, source file and exact inserter field. No arbitrary universal millisecond cap is added.

Independent QA proved the old path accepted zero swing/return through pack_checked and then stuck in ToDestination/Returning. It also accepted swing_ms214748364800 at20Hz (4294967296ticks) and silently skipped the initial pick after a failed u32 conversion. These are invalid engine representations, not alternative timing policies to expose.

The fix retains state/wire shape and literal positive-duration cycle rules. If wider or instantaneous timing is later designed, rework is the validation bounds, the elapsed representation and cycle semantics; no such policy is chosen here. Original candidate b6a8c7a remains unmerged until the scoped fix passes the same reviewer and gates.

Related T11 representation check: weapon cooldowns also use a u32 counter, so boot rejects ceil tick counts above u32::MAX with weapon.cooldown_ms diagnostics. Unlike arm swing/return, zero cooldown is valid under the literal one-shot-per-weapon-per-tick rule; no shared positivity policy is imposed.
