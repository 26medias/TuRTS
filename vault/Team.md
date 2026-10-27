---
type: note
date: 2026-10-26
tags:
  - turts
  - team
ai-first: true
source: mcp
---

## For future Claude
The studio roster, who decides what, the model tiers, and the review process.

# Team

**CEO:** the user. Sets high-level goals and breaks ties. Does not need to be in the loop for the studio to progress. Reads [[Home]] and the daily report in `Reports/`.

**Orchestrator:** the main Codex session, taking over from Claude Code on 2026-10-26. Picks lenses per gate, makes studio calls, writes specs and plans, keeps this vault.

**Sixteen seats** in `.claude/agents/` (design: `docs/superpowers/specs/2026-10-25-studio-agent-team-design.md`):

| Seat | Owns |
| --- | --- |
| technical-director | architecture, decision register, crate boundaries, cross-system invariants |
| performance-architect | cost: bytes, cache lines, allocations, bandwidth, tick time |
| simulation-core-engineer | ECS core, fixed ticks, economy, production, research |
| world-engineer | terrain, geology, underground networks, map generation |
| network-engineer | authority, replication, transport, spectators |
| web-platform-engineer | wasm build, browser memory and storage, loading |
| bevy-rendering-engineer | presentation, camera, assets, UI primitives |
| mod-runtime-engineer | sandbox, public API, packages, lockfiles, registries |
| backend-engineer | index, host, rooms, slots, identity, SQLite |
| ai-engineer | conventional bot opponents |
| game-designer | default mod rules, balance, settings schema |
| qa-lead | test instruments, "can this test go red", benchmarks |
| producer | plans, milestone sequence, scope control |
| mod-builder | third-party modder; public API only |
| player | plays the result; plain language |
| spec-rigour-reviewer | contradictions, status drift, untestable claims; STE reader |

**Model routing** (user rule, 2026-10-26): use Sol for reasoning, planning, and reviews; use Luna for execution (edits, implementation, and commands). Keep dispatch scopes and context short, and reuse completed evidence to conserve tokens. This replaces the initial all-Sol routing notes going forward.

**Parallel M2 execution (user-authorized):** Tasks 1–9 are complete after review and canonical integration at `1f004e6ad363a5f94f47cb52e4bf98ce5c95aa08` in `/home/julien/.codex/worktrees/ee40/TuRTS`; original progress is 9/16. Task 9 final gate passed with 390 workspace tests, 0 failed, 12 ignored across 68 groups; fmt, locked check, strict Clippy, boundary and wasm passed. Process acceptance passed, and T7 Chrome passed 52/52 frames (`acme:scout`, final `[20,8]`). Task 10 protocol source `40bbb69` received Sol `REQUEST_CHANGES` (3 medium, 1 low); Luna is fixing the disconnected sender drain, ambiguous duplicate-sequence receipt proof, missing real schedule/transport reversal evidence, and retained spike payload byte undercount. Task 14 mutation runs are ongoing. Independent pure Task 11 exporter preparation is APPROVED_PREPARATION_ONLY in `/home/julien/.codex/worktrees/61b7/TuRTS`: base `1f004e6ad363a5f94f47cb52e4bf98ce5c95aa08`, initial commit `e1467e8`, reviewed head `9fb3620`. Its 5/5 scoped tests and fmt passed; the queue-only delta leak plant failed as expected and passed after restore. This does not complete Task 11: protocol promotion under Task 10 review and all full Task 11 session/synthetic/Tick0 integration remain pending. Protocol 1 remains selected; no protocol 2 activation is canonical. Task 12 context confirms full Task 11 and Task 12 can proceed in parallel after Task 10 acceptance from the same base; Task 13 depends on Task 12. No new design is required.

**Codex handoff:** historical Claude routing was superseded by the user's Sol/Luna rule above. The orchestrator uses the available Codex models and states both model and reasoning effort on each dispatch, following that rule. This is a platform adaptation, not a change to the team's responsibilities or review gates.

Codex does not automatically register `.claude/agents/` as native roles. Each dispatch tells the agent to read its role file, standing rules, relevant project notes, and source documents. Claude-specific frontmatter tool names are mapped to the tools available in the session. Role memory remains in `.claude/agent-memory/`; shared skills are available under `~/.agents/skills/`. There are four concurrent slots including the orchestrator, so larger gates run in batches.

**Process:** the Anvil, sized to the task. Brainstorm, spec, gate 1 (four lenses), plan, gate 2, execute, verify. Reviewers run things and cite `path:line`. No Anvil on trivial work.

**Installed skills:** `docs/agents/skills.md`.

**Current lane checkpoint (2026-10-27; supersedes the in-progress wording above):** Original M2 progress remains 9/16; no new product task has been accepted since the prior report. Builds paused during the disk audit have resumed in both the Task 10 protocol lane and Task 14 proof lane. Task 10 head `4561c48` received `REQUEST_CHANGES` with two evidence gaps remaining: actual `PreUpdate`/last-receiver-zero behavior, and captured transport IDs tied to a product witness helper. Other client and spike fixes were accepted at source. Task 14 head `e1e6742` received `REQUEST_CHANGES`: actual product mutation families, queued cancellation, structured public diagnostics, capture/command association, and the hash documentation remain to address. Task 11 exporter head `9fb3620` remains preapproved preparation and held. Disk audit found 282G free; no artifacts were deleted, and existing targets can be reused. Steam compatibility was assessed as feasible but not Steam-ready; no scope or implementation change was approved.


**Accepted checkpoint (2026-10-27; supersedes prior lane-status paragraphs):** Tasks 1–9 and 14 are complete after review and canonical integration; original M2 progress is 10/16. Task 14 HEAD is `74e2c1068841552e15edb15166081929740c3756`. Its scoped canonical gates passed: economy accounting 42/42, variants 7/7, public path 2/2, no-content scanner 4/4, Python variants 26/26, fmt and strict Clippy. Public-proof mutation evidence covers 25 cases: 18 previously verified cases plus seven new product cases; all seven new cases produced semantic RED, exact restoration, and restored GREEN with source hashes checked. Task 14 final Sol review found no blockers. E8/E9 remain deferred to Tasks 15/16. Task 10 source at `4218f6e` is now Sol-approved and its reviewed commits are being integrated in canonical; Task 10 global and process/browser gates remain outstanding. Protocol 1 remains selected until the atomic Task 10 gate completes.


**Accepted checkpoint (2026-10-27; supersedes prior lane status):** Tasks 1–10 and 14 are complete after Sol/root review and canonical integration; original M2 progress is 11/16. Product checkpoint `f75488082f87c829c3e210b3eccb6b3420a0994c` selects `NET_PROTOCOL_VERSION=2`. Task 10 canonical gates passed: workspace tests 401/0/13 across 67 groups, fmt, locked workspace check, strict Clippy, boundary, workspace wasm, excluded spike-web wasm, process acceptance 1 + 2 + 8 + repeated convergence 1, and Chrome T7 53/53 frames (53 scene checks, `acme:scout`, final `[20,8]`, oracle 1/1). Root verified no browser errors and a clean canonical state. Tasks 11 and 12 are now approved to proceed in parallel from exact product checkpoint `f754880`, reusing `/home/julien/.codex/worktrees/61b7/TuRTS` with `target/m2-export` and `/home/julien/.codex/worktrees/8bdc/TuRTS` with `target/m2-protocol`. Their previous preparation status does not substitute for full task completion. Tasks 13 and 15–16 remain; README, main merge, and push remain deferred until all tasks and final gates are complete.


**Accepted checkpoint (2026-10-27; supersedes prior lane status):** Tasks 1–11 and 14 are complete after Sol/root review and canonical integration; original M2 progress is 12/16. Task 11 source head `aefc69746e5e8846b292a7dbdefd54f64fbec7e4` was integrated in commits `ff10ec5`, `e69fd96`, `2a186f0`; accepted product checkpoint is `2a186f01bc185eb320145ac52bddf8b399ec6c2b`. Scoped gates passed: 262 tests/0 failures/12 ignored across 30 groups (exporter 5/5, unit_boot 10 passed/1 ignored, economy_accounting 42/42, economy_variants 7/7), fmt, locked sim/server check, strict sim/server Clippy, and sim wasm. The accepted review `.superpowers/sdd/2026-10-26-m2-economy/task-11-full-review.md` found no actionable issues. Combined full workspace/process/browser gates are intentionally deferred until Task 12 integration. Task 12 core and resync implementation is underway in `/home/julien/.codex/worktrees/8bdc/TuRTS`; Task 15's unwired JSONL preparation received Sol review in `/home/julien/.codex/worktrees/b54a/TuRTS`, but Task 15 is not complete. Task 13 UI and Task 16 final acceptance remain. Protocol 2 is selected. No all-16 completion or README/main/push closeout is claimed.


**Accepted checkpoint (2026-10-27; supersedes prior lane status):** Tasks 1–12 and 14 are complete after Sol/root review and canonical integration; original M2 progress is 13/16. Task 12 product HEAD is `f17ff7762ce63cc3f2a7a29dda28beb9b5378b78`, accepted by root after the combined gates. Workspace tests passed 427/0/13 across 69 groups; fmt, locked check, strict Clippy, boundary, workspace wasm and excluded spike-web wasm passed. Process acceptance passed 1 + 2 + 8 + repeated T5 1. Chrome T7 passed 45/45 scene checks and oracle 1/1. Base, haul and cedar native headless clients joined and presented units with zero resync requests; reports omit economy/node details, so initial state acceptance is inferred from validation and presentation rather than direct record inspection. Task 13 UI starts at this exact product HEAD. Task 15 standalone JSONL preparation is approved, parser prep is underway, and neither is complete. Task 16 final native/browser acceptance remains. Keep README, main merge and push deferred until all tasks and final gates pass.


**Accepted checkpoint (2026-10-27; supersedes prior lane status):** Tasks 1–14 are complete after Sol/root review and canonical integration; original M2 progress is 14/16. Task 13 product HEAD is `435f74011f0aa577566f132791f596fa1a96dd19`. Sol approved source head `126c1793634c76f197db14c3800ec07cf7787806`; the three reviewed source commits `f12867b66aa8485be73d660b9a93aa7db088151f`, `65ecfcb70fe631bd0eb995389978d20b5ed5fdc0`, `126c1793634c76f197db14c3800ec07cf7787806` integrated as `7757fd1`, `a46d034`, and `435f740`. Combined canonical gates passed: workspace 445/0/13 across 70 groups; fmt, locked check, strict Clippy, boundary, workspace wasm, excluded spike-web wasm and Trunk release; process acceptance 1 + 2 + 8 + repeated T5 1; Chrome T7 52/52 scene checks and oracle 1/1. T7 is not final UI-control proof. Task 15 native process validation is active and incomplete; Task 16 actual native-window controls and browser UI text/click acceptance remains. A preliminary overview screenshot and unverified single selection click are not acceptance evidence. Keep README/main/push closeout deferred until both tasks and final gates pass.
