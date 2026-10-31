---
type: decision
date: 2026-10-29
tags: [turts, decision, mc]
ai-first: true
source: codex
status: approved
---

# MC build instructions from the CEO

The current CEO instruction names base `a0a3076`, integration `codex/mc-integration`, one task branch/worktree per lane, and `CARGO_TARGET_DIR=target/mc-t<N>`. Do not merge into main.

Revision 10 (`488b3d3`) is the design of record. The design wins on a plan conflict; report the conflict. This overrides the brief section 1 sentence that says the plan wins. Stop and report on each stop condition in the brief.

K1 to K27 remain as recommended; K11 depends on P1. The approved total is about 212 runs. Stop at 170 build runs. The CEO plays PT1 and PT2; logs say "not blind". Do not reopen these decisions.

Start with task 1 only. Report its result to the CEO before any other task starts. The task 1 preflight found an output-table conflict; this note does not resolve it. See [[MC Execution Board]].
