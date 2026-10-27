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

**Parallel M2 execution (user-authorized):** after Tasks 1–2 passed review at `9e383896`, root coordinates three initialized isolated lanes: A simulation Tasks 3–8 at `/home/julien/.codex/worktrees/8b9c/TuRTS` (`codex/m2-sim`); B authored packages/Cedar boot preparation for Tasks 9/14 at `/home/julien/.codex/worktrees/b8a3/TuRTS` (`codex/m2-data`); and C staged protocol-v2 DTOs/nonselected registration preparation at `/home/julien/.codex/worktrees/e8ea/TuRTS` (`codex/m2-wire`). All branch from reviewed base `9e383896`. Task 3 is complete and integrated canonically after Sol rereview; Task 4 commit `5a178fe` is pending Sol review; E1 scan commit `76a9aee` has six findings in fix round 1; R6 splits gather Task 5 and production Task 6 across isolated branches from the exact reviewed Task 4 baseline. Task 7 follows Task 6 review and waits for integrated Tasks 5+6 plus combined timing/atomicity tests before merge; Task 8 waits for integrated/reviewed Tasks 5–7. Coordinator composes shared semantic hunks; do not create production worktree before Task 4 approval. E1 scan `76a9aee` awaits Sol review. B package/Cedar prep (`5009f98` + `c8da658`) is approved and held until A8 and M1 migration. C1/C2 prep is integrated canonically while protocol 1 remains selected; Task 10 remains incomplete. Original task progress is 3/16. R5 adds Task 3 ownership of `crates/turts_sim/src/economy/bootstrap.rs` for validated Inventory `accepted_items`; no lane conflict, small constructor/test rework if wrong. Prep does not mark Tasks 9/10 complete. Root plus three implementers fills all four concurrent slots; reviews may replace idle implementers, never dispatch a fourth child. Review precedes integration in canonical worktree `/home/julien/.codex/worktrees/ee40/TuRTS`. Preparations do not mark original tasks complete. UI preparation follows Bboot only while independent; otherwise that slot goes to Task 14 preparation. Shared activation remains coordinator-owned, with Task 10 atomic. Frozen interfaces and lane ownership: `docs/superpowers/plans/2026-10-26-m2-parallel-execution.md`.

**Codex handoff:** historical Claude routing was superseded by the user's Sol/Luna rule above. The orchestrator uses the available Codex models and states both model and reasoning effort on each dispatch, following that rule. This is a platform adaptation, not a change to the team's responsibilities or review gates.

Codex does not automatically register `.claude/agents/` as native roles. Each dispatch tells the agent to read its role file, standing rules, relevant project notes, and source documents. Claude-specific frontmatter tool names are mapped to the tools available in the session. Role memory remains in `.claude/agent-memory/`; shared skills are available under `~/.agents/skills/`. There are four concurrent slots including the orchestrator, so larger gates run in batches.

**Process:** the Anvil, sized to the task. Brainstorm, spec, gate 1 (four lenses), plan, gate 2, execute, verify. Reviewers run things and cite `path:line`. No Anvil on trivial work.

**Installed skills:** `docs/agents/skills.md`.
