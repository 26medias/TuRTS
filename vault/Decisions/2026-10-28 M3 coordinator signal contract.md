---
date: 2026-10-28
status: adopted
tags: [turts, m3, testing, privacy]
---

# F11 coordinator signal and report contract

User decision: research remains owner-private. Coordinator observes owner replica, then signals opponent. No opponent research wait or server-private report shortcut.

Native CLI: `--produce UNIT,TECH` requests exactly one Produce. Optional `--produce-when-inventory UNIT,ITEM,N` gates that request until coherent owner Main inventory at watched UNIT has unreserved ITEM amount>=N. N positive; malformed numbers/refs/invalid combinations rejected by option parsing. Unit/content identifiers are parameters, never Forge constants in engine.

Native `--live-report PATH`: atomically replace PATH with ordinary client report JSON whenever coherent replica tick changes, including terminal/error state as needed. Extend existing native atomic JSON writer or equivalent; no per-frame gameplay effects. This works headless and windowed, does not alter final `--out` semantics. Research report derives from own acquired replica only. Native file IO excluded for wasm.

`--move-release-file PATH`: in conjunction with existing `--move X,Y --move-unit ID`, withhold ordinary one-shot MoveTo until local PATH exists AND normal coherent join/state checks pass. File is in fresh test-run directory, created by process coordinator only after parsing owner's live report completion. No new server message/endpoint, no transfer of technology state to opponent. Existing `--move-after-ticks` timing preserved for existing M2 tests; signal gate adds requirement only when flag supplied.

Report fields: top-level `health_trace` is an array of `{unit_id, entries: [{tick,current}]}` records (the accepted T15a representation); records initial snapshot health and each changed coherent absolute value, including killing0 before removal; retained for dead units; stale/replayed batches cannot duplicate or rewrite it. `research_completed_tick: Option<u32>` is earliest coherent replica state tick proving the technology requested by `--produce` is complete. It is an observation watermark, not a recovered historical ResearchCompleted event timestamp. Process test separately checks server's actual audited event tick. For late snapshot, observed tick can be later than actual completion; this is safe for gating. No completed state -> null.

Process F11: start both clients before first shot; choose fixed permanent player IDs/slots in fresh server instance. Poll owner live report; verify own research completion and coherent tick; atomically create opponent release file. Assert actual recorded MoveTo applied tick strictly greater than server audited ResearchCompleted event tick, not wall time. Assert opponent decoded replica has no owner research. For each drone health trace has initial100 plus exact10 steps-10 or5steps-20 ending0, derive shots from trace, compare complete audited server events as independent witness. Explicit deadlines fail with logs; no arbitrary sleep drives gameplay actions.

Browser: use same production command sender and replica-gated predicates. Existing window.turts_debug exposes permitted current report; test-only UI action harness must operate actual controls, not mutate replica or inventory. Browser need not expose native file path flags.

Ruling: observation tick semantics distinguish what client can know from historical private event; cost if wrong is report/coordinator assertions, no research replication policy change.
