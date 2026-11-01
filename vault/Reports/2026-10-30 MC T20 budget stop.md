---
type: report
date: 2026-10-30
tags: [turts, mc, budget, stop]
ai-first: true
source: codex
---

# T20 budget stop

STOP for CEO direction. Task20 estimate3; runs7,12,13,14,15,16 total6. The brief section3, line34 says: "A task that reaches twice its estimate stops and reports." Global build use is16/170. No further agent run, task, merge or push is authorized past this stop without a ruling.

Candidate: `72ddb0a6541862b3d26e1376c8bded687f1c6ce5`, branch `codex/mc-t20`, clean worktree `/home/julien/Projects/TuRTS-mc-t20`. It is NOT merged. T1/T2a/T3 remain integrated and gated. Wave1 push/hostedCI are pending; main remains `f503cff7ec9e520526ba08fc471a2ca17427d1bf`.

Run15 closed the original two findings but found one Medium modifier handoff bug. Run16 reproduced it on the compiled game at DPR1 and2, exit1, then repaired the pinned Playwright keyboard-state handoff without extra DOM input. Final builder proofs: G-QA/G-WEB exit0; Node17/17; direct DPR1 CLI --windows2 overrides JSON1; DPR2 two-window row passes twice; actual CDP-held Shift survives normal key/click/drag and release. Raw-page plant fails1; restored tests pass0 and exact source blob matches the commit. T7 segment oracle passes138frames. Root independently checked numeric receipts, hashes, all three raw input/downstream traces, sequence continuity, PNG1280x720, actual handoff flags and clean source. This is an audit of builder proof; the independent recheck is still pending.

Evidence: `target/mc-evidence/t20-builder/final-repair/`, especially `root-stop-audit.json` and `commands-and-compatibility.txt`. Receipt files use both plain exit integers and exit=n markers; root's reader was corrected to accept both without changing any receipt. No candidate failure was inferred from that reader-format correction.

Compatibility limit: the repair uses Playwright1.64.1 in-process private keyboard state. It checks the bridge/object shape before use and fails clearly for an unsupported backend. Dependency/lock hashes are unchanged. This needs independent review along with the actual input proofs.

The clean original-source native M2 baseline passed15/15. Historical hosted software-profile cause remains NOTTRACED and its fix remains Task21 per design1.6. Firefox remains deferred by the CEO. Later persistence/readiness and CI-unit registration checks remain on the board; they are not waived.

Disk: one reusable compiler cache remains for the unmerged task. MC proof records total about171MiB; obsolete caches reclaimed24.24GiB earlier. Filesystem has about701GiB free. Source/history/necessary proof is retained.

Requested ruling: allow one additional independent Sol/high review run (T20cap7, global17/170), then continue the build only if the review passes. No further run has started.
