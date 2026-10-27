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

**Parallel M2 execution (user-authorized):** Tasks 1–9 are complete after review and canonical integration at `1f004e6ad363a5f94f47cb52e4bf98ce5c95aa08` in `/home/julien/.codex/worktrees/ee40/TuRTS`; original progress is 9/16. Task 9 final gate passed with 390 workspace tests, 0 failed, 12 ignored across 68 groups; fmt, locked check, strict Clippy, boundary and wasm passed. Process acceptance passed, and T7 Chrome passed 52/52 frames (`acme:scout`, final `[20,8]`). Task 10 protocol switch and Task 14 public-proof prep are isolated and await root dispatch. Protocol 1 remains selected until Task 10 integrates atomically.

**Codex handoff:** historical Claude routing was superseded by the user's Sol/Luna rule above. The orchestrator uses the available Codex models and states both model and reasoning effort on each dispatch, following that rule. This is a platform adaptation, not a change to the team's responsibilities or review gates.

Codex does not automatically register `.claude/agents/` as native roles. Each dispatch tells the agent to read its role file, standing rules, relevant project notes, and source documents. Claude-specific frontmatter tool names are mapped to the tools available in the session. Role memory remains in `.claude/agent-memory/`; shared skills are available under `~/.agents/skills/`. There are four concurrent slots including the orchestrator, so larger gates run in batches.

**Process:** the Anvil, sized to the task. Brainstorm, spec, gate 1 (four lenses), plan, gate 2, execute, verify. Reviewers run things and cite `path:line`. No Anvil on trivial work.

**Installed skills:** `docs/agents/skills.md`.
