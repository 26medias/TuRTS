---
type: decision
date: 2026-10-30
tags: [turts, mc, decision]
ai-first: true
source: codex
status: approved
---

# MC Firefox deferral and continuation

CEO: "Nevermind, ignore firefox for now, continue".

This releases the P1 Firefox stop. Keep K11(a), Ctrl+digit, on the observed Chrome evidence. Firefox stays NOT TRACED; do not claim Firefox acceptance. Continue the full MC plan, with all other stop conditions in force. O9 still requires recording default prevented without judging it. Main must not be merged or pushed.

The CEO also required control of disk growth. Remove verified obsolete build caches; keep source, unique changes, logs, screenshots and proof records. Keep one integration cache, build lanes in sequence when practical, and delete each lane's target/worktree after its verified merge. Use debug=0 and incremental=0 for new builds.
