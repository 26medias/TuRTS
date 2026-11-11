---
type: report
date: 2026-11-11
tags: [turts, mc, codex, handoff]
---

# MC orchestrator handoff

You are taking over as Codex orchestrator for the MC build in `/home/julien/Projects/TuRTS` on this desktop. The CEO authorizes you to resume the paused work, finish Task16, and continue the remaining MC tasks to completion. This supersedes the previous pause instruction for the receiving agent. Do not merge or push main.

## Read first

From `/home/julien/Projects/TuRTS-mc-integration`, read:

1. `vault/Reports/2026-11-11.md`, especially its final Pause checkpoint section. Earlier entries are chronological history.
2. `vault/MC Execution Board.md`.
3. `vault/Reports/2026-10-29 MC Codex brief.md`, especially sections3–6.
4. `docs/superpowers/plans/2026-10-29-mc-player-control.md`.
5. Relevant sections of `docs/superpowers/specs/2026-10-29-mc-player-control-design.md`, revision14.

Follow AGENTS.md. Do not read or import CLAUDE.md, vault/_CLAUDE.md, .claude/, Claude agent files or settings. The brief's inherited instructions to read those files are overridden. Use the plan's roles and Codex-side instructions. The design wins over the plan. Apply the CEO's widened decision rights in brief section4; record Codex rulings C<n> on the board and in `vault/Decisions/2026-10-30 MC Codex rulings.md`. Last ruling is C381.

## Current verified state

- 20/25 lanes complete;110/170 agent build runs used.
- Remaining tasks:16,22,21,23,24. Wave8 is incomplete.
- Main: `bb611bd035d90924177e19c942fa01d003ccce0c`, unchanged.
- Integration: `/home/julien/Projects/TuRTS-mc-integration`, branch `codex/mc-integration`. Product-code integration remains at51170cb4. The pause checkpoint was `c1e7a71ba54e73e56a697516b78657ed6cd8feb5`, clean and pushed; this handoff is a later documents-only addition. Verify current HEAD.
- Task16: `/home/julien/Projects/TuRTS-mc-t16`, branch `codex/mc-t16`, HEAD `43cfe942f22cf17209823c1b41192cd36c9af568`, clean and pushed. NOT merged into integration.
- Task22: `/home/julien/Projects/TuRTS-mc-t22`, branch `codex/mc-t22`, base `fd369dc22e58e6606d93d15559942764424c1393`. There are16 pending tracked/untracked paths. Preserve them. No Task22 runtime acceptance yet.

## First action: complete Task16 independent review

Task16 has:

- All15 committed normal gates green.
- Native workspace1665passed/0failed/21ignored; Node158passed.
- Actual seven browser rows and supplemental SH1 DPR2 proof.
- All9 required Root fault cycles: actual semantic red, exact restoration, full fresh green. RUN110 independently qualified all9.
- RUN110's distinct own storage-pruning fault also completed: baseline0, named red1, exact restoration, full fresh green0.

Final independent approval is missing. RUN110's agent session failed with the platform error “This content was flagged for possible cybersecurity risk” after launching its own test script. The already-running script finished correctly. Root verified its actual raw/source/runtime evidence, but did NOT invent final reviewer approval or merge Task16. The platform error was not a product-code finding.

Register a new independent reviewer continuation BEFORE dispatch:

- Next global run111/170.
- Next Task16 run22; estimate6, hard stop24.
- Use the brief's reviewer model/role and state model/effort.
- Review retained evidence and complete final approval. Do not repeat passing tests merely because this is a new session. Rerun only for a source change, failure or unresolved concern.
- Do not bypass or misrepresent any platform restriction.

## Evidence

All paths in this section are under `/home/julien/Projects/TuRTS/target/mc-evidence/`.

Task16:

- `t16-root/committed-gate-audit-v5.json`
- `t16-root/nine-root-cycle-audit-v1.json`
- `t16-root/formal-cycles/*/cycle.json`
- `t16-review/run110/committed-43cfe94/source-and-gates-review.md`
- `t16-review/run110/committed-43cfe94/required-gates-actual-audit.json`
- `t16-review/run110/p9-closed-cycle-v1/review.md`
- `t16-review/run110/own-plant-cycle-v1/cycle.json`
- `t16-root/pause-20261111/own-cycle-and-review-status.json`
- `t16-root/pause-20261111/remote-checkpoint.json`
- `t16-root/source-checkpoint-43cfe94/` contains the committed151-path source archive, patch and manifest.

Reviewer own cycle:

- Baseline raw `80be99d3c3acf37ff186f83d1637c87d6739d26938b0358c0929b8d9049b462f`.
- Red raw `86afd0ab711d3012fc27368f4f88fef797a3cf44fab38392898e0954ec5874cf`; named `PER1.old-match-groups-pruned`. Actual old namespaces remain when browser removal is disabled.
- Green raw `9e0d4141188a2968001217f6b05d16baf56c82fd1a49e5a6ac7ac727ed402c1d`.
- Source restored exactly; real HTTP program assets, server and native helper match the proven baseline.

P9's raw startup timeout alone has NO semantic-red credit. Its separate retained qualifier proves the actual corrupt-settings panic in the seeded document and passes12 negative controls. Read its qualification record and independent report.

Task22:

- `t22-root/run101-bounded-repair/source-v3/` holds exact copies of all16 pending paths.
- `t22-root/pause-source-preservation-20261111.json` verifies the live files match.
- RUN101 source review426ed0d5 was source-ready; runtime, metrics and plants remain required.

## After Task16 final approval

Merge only into `codex/mc-integration`. Re-gate integration after the merge: G-RUST, G-QA, G-FROZEN and G-WEB. Prepared external script `t16-root/integration-regate.py` is available; inspect it before execution. Push integration and read hosted CI. Then continue Task22 and the remaining plan tasks in dependency/wave order.

## Cache and disk

- One compiler/browser/GPU lease at a time.
- The sole useful compiler cache is currently Task16's `target/mc-t16`, about47GiB.
- Move that idle cache to integration's `target/mc-integration` after merge; do not duplicate dependencies. Inspect `target/mc-evidence/cache_handoff_v2.py` and prior handoff receipts. Invalidate local workspace outputs across profiles when changing worktrees.
- Use assigned CARGO_TARGET_DIR for each lane.
- Temporary `baseline-cache-v1/cache` and `/dist` payloads were removed after all cycles finished. Its manifest remains. Old cache recovery scripts cannot run against those missing payloads. Create a fresh proven baseline archive if another mutation cycle requires one.
- Preserve raw evidence and actual mutation programs.
- Archive provenance and dirty content before cleanup; inspect active processes first. Retire Task16's worktree/target only after merge and preservation.
- Last disk check:544GiB free.

Validated external runners under the primary evidence directory:

- `gate-runner/exec_in_lane_v2.py`
- `gate-runner/mc_headed_v3.sh`

These exclude Claude guidance BEFORE reading/hash calculation. Do not revert to old runners.

Headed desktop browser profile:

```text
--enable-gpu
--use-angle=vulkan
--enable-features=Vulkan
--enable-unsafe-webgpu
--disable-vulkan-surface
```

Ignore default `--enable-unsafe-swiftshader`. Use the wrappers' isolated Xvfb/GPU lock and verified ALSA environment. Default1280x720/DPR1; SH1-dpr2 uses1280x720@2. SEL4 uses `tests/browser/fixtures/sel4-muster`, not mods/forge.

## Decisions and stop rules

- K1–K27 remain approved; full scope.
- Firefox P1 is deferred by CEO; do not reopen it.
- O9: record event.defaultPrevented after the handler; K11 judges reaches-page/browser-acted only.
- O10: `Effect::ShellAction { action }`; client_shell.rs, not shell.rs.
- O12: N legal primary slots required atomically; spares zero or more.
- O13: unchanged Okabe-Ito palette; CIEDE2000>=20 normal, >=10 under each Machado2009 severity1 simulation.
- Frozen stream SHA must remain `bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b`.
- Decide the conflict kinds delegated by brief section4; do not stop for those.
- Stop/report for the narrowed stop list: frozen-value changes, suspect Decided/K decisions, player-visible design changes, global170 or task4x budget, plant retirement, Task23 human URL, or main merge.
- Before Task23, send the actual serve-only page URL and WAIT for CEO play. CEO plays PT1/PT2; log “not blind”.

## CI and completion

Last hosted wave7 run42789795527 had agents PASS and the two O13-approved legacy red-window failures owned by Task21. Preserve that attribution until traced/fixed. The documents-only pause checkpoint c1e7a71 used skip-ci; GitHub reported no workflow runs/statuses for it. This is NOT CI acceptance.

Done requires all25 lanes integrated, all required acceptance and refutation evidence, legacy bridge0rows, CEO PT1/PT2 signoff, final check/agents/browser CI green, measurements and vault complete. Do not merge/push main; report the integration commit and evidence for CEO approval.

## Work style

Verify actual files/raw receipts; summaries are leads. Preserve compiled semantic red→restored green evidence and exit codes. Stage explicit paths; never `git add -A`. Maintain one ledger row per agent run, current board/Home/Tasks, daily work log and end-of-session report. Vault-only commits use `[skip ci]` and Codex attribution.

Keep working autonomously until completion or a real stop condition. A progress/ETA question is not a request to stop: answer briefly and continue. Send useful commentary at least every60seconds during active work. Group verification where valid, but never omit mandatory integration gates after each merge.
