# Studio agent team — design

Date: 2026-10-25. Status: revision 3 (plus G2 alignment). Passed gate 1 re-gate (four lenses, no fail verdict). Changes from the re-gate are marked `(G1b)`.

Revision 2 applies the gate 1 findings of four reviewers (rigour, engine, boundary, consumer).
Each change that came from a reviewer is marked `(G1)`. Each decision the orchestrator made
without the user is marked `(call)`.

## 1. Intent

Build a standing team of Claude Code subagents for this repository. The team represents a
game studio built around the project stack: Rust, Bevy, browser WebAssembly, authoritative
multiplayer, and a sandboxed mod runtime. The orchestrator (the main Claude Code session) uses
the team to write specs, review them, plan, build, and test. The team also includes two
outsider roles that judge the result from outside the studio: third-party mod builders and
players.

**Why.** The project specification (`RTS-Engine-Project-Specifications.md`) holds 59 decisions
and many open items. The Anvil process in the user's global instructions reviews every spec and
plan through four lenses in parallel. Generic agents give generic reviews. The user wants
reviewers who know Bevy scheduling, WASM memory limits, server authority, and mod sandboxing
well enough to run things and break claims. The first use is the ground work: specs and plans
for the foundation (milestone M1). A wrong decision there costs more than any later bug.

## 2. Goals, non-goals, anti-goals

**Goals**
- One agent file per role in `.claude/agents/`, committed to this repository.
- Each agent is an expert in one seat of the studio, scoped to this project's stack and spec.
- The orchestrator can pick any subset of agents for a gate. Section 6 gives a mapping and a
  selection rule. The orchestrator decides per gate.
- Every agent obeys the standing rules of this user: verify never assume, cite `path:line`,
  list what it ran, label unverified claims `NOT TRACED`, write in Simplified Technical
  English (STE), four-space indentation.
- Reviewer agents must run things, not reason about them. They review the instrument (the
  test) as well as the design: "would this test fail, and on which build?"
- Reports stay small so twelve reports in a day do not fill the orchestrator's context. (G1)

**Non-goals**
- No orchestrator agent. The main session orchestrates.
- No workflow scripts or multi-agent fan-out automation. Workflows need an explicit ask per task.
- No agent for roles outside the stack (marketing, legal, art production).

**Anti-goals (failure even if it works)**
- An agent that reads a document and agrees with it. Each reviewer must produce at least one
  concrete, checked finding, or state what it ran, what result would have refuted the
  document, and why nothing broke.
- An agent that invents facts about Bevy, wasm-bindgen, or browser limits without checking
  documentation or running code.
- An agent that reopens a **Decided** item in the spec as if it were open.
- A roster so large that the orchestrator cannot choose. Sixteen roles is the cap. (G1: the
  simulation seat was split, see section 4.)

## 3. Constraints

Facts below about Claude Code were verified by the gate 1 engine reviewer against
https://code.claude.com/docs/en/sub-agents.md on Claude Code 2.1.310.

- **File format.** Markdown with YAML frontmatter. Fields used: `name`, `description`,
  `tools`, `model`, `memory` (optional), `color` (optional). The loader does not require the
  file name to equal `name`; this design requires it as a convention. `claude plugin validate`
  does not flag a file without `name` (verified). Load behaviour for such a file is NOT TRACED.
  Acceptance checks therefore read the `name:` field, not the file name. (G1, G1b)
- **Description.** One physical line, double-quoted, line breaks written as `\n`. Its parsed
  UTF-8 value is at most 900 bytes. Exactly one `<example>` block. (G2) The first sentence names the seat. One line lists the modes the role has:
  `Modes: review-spec, review-plan, build, test, answer` (only the ones that apply). The
  combined descriptions of all agents are shown to the orchestrator at every session start;
  the cap keeps the roster near 4,000 tokens. (G1)
- **Model.** Every file states `model: inherit`. The orchestrator may pass `model` or
  `effort` per call when a gate needs more. No file names a model. (G1, call)
- **Tools.**
  - Reviewer-only roles: `Read, Grep, Glob, Bash, WebFetch, WebSearch`.
  - Roles whose kind includes builder or planner add `Write, Edit`.
  - No role gets `Agent`. The `tools` allowlist enforces this (verified: nesting is on by
    default and omitting `Agent` blocks it).
  - Bash is in every role so it can run tests, builds, and probes. Bash can write files.
    The `tools` list therefore does not make a review read-only. The review-mode rule below
    and the `git status` check in acceptance do that. (G1)
- **Review mode.** In modes review-spec and review-plan, no agent calls `Write` or `Edit` on
  the repository, and no agent writes to the tree through Bash. One exception: a memory role
  may write its own memory under `.claude/agent-memory/<name>/`. Every review report carries
  the line `What I changed: none` (memory writes do not count). If an agent changed any other
  path, it lists the files and the orchestrator treats the review as failed. (G1, G1b)
- **Memory.** `memory: project` on `technical-director`, `qa-lead`, and `mod-builder` only.
  These roles already have `Write, Edit`. Memory turns `Read, Write, Edit` on, so no
  reviewer-only role gets memory. Memory files live in `.claude/agent-memory/<name>/` inside
  the repository and are committed. This directory is the one exception to the rule that agents
  write only to the scratchpad. (G1, call)
- **Context.** Claude Code loads the `CLAUDE.md` hierarchy into every subagent. Agent bodies do
  not re-read it. Each body names the spec sections that belong to its seat and reads those.
- **Versions.** Agent files name no library version. Each agent reads `Cargo.toml` first. When
  no workspace exists, the agent marks every version-dependent fact `NOT TRACED`.
- **Throwaway files** go in the scratchpad directory, never in the repository tree.

## 4. The roster

Sixteen roles. File name equals `name`. "Primary" owns the section; "Second" is the standing
second lens for it. (G1: the boundary reviewer built this table from the spec headings.)

### Studio core

| name | Seat | Kind | Memory |
| --- | --- | --- | --- |
| `technical-director` | Architecture, decision register, crate boundaries, API versioning policy, cross-system invariants | reviewer + builder | project |
| `performance-architect` | Carmack-style: data layout, cache, allocation, bandwidth bytes, tick cost, radical simplification. Reviews specs and plans for cost. Proposes measurements, not opinions. | reviewer | none |
| `simulation-core-engineer` | ECS, fixed ticks, scheduling, mutation and randomness rules, headless-runnable core, economy, production, logistics, research | builder | none |
| `world-engineer` | Terrain heightfield, geology and discovery, underground utility networks, seeded map generation, multi-map simulation | builder | none |
| `network-engineer` | Authoritative server, replication, interest management, interpolation, transport candidates, state sync, spectator distribution | builder | none |
| `web-platform-engineer` | WASM build, wasm-bindgen, browser memory, browser storage, HTTPS/cross-origin, loading, browser-first validation | builder | none |
| `bevy-rendering-engineer` | Presentation layer: rendering, interpolation, camera, lighting, 3D assets, UI primitives, input | builder | none |
| `mod-runtime-engineer` | Sandboxed scripting, public API boundary, engine-versus-mod rule, packages, manifests, lockfiles, registries, publisher keys | builder | none |
| `backend-engineer` | Index, servers, headless host executable, rooms, slots, identity keys, sessions, SQLite, saves, hosting cost | builder | none |
| `ai-engineer` | Conventional bot controllers, priorities, reservations, squads, state machines, headless bot scenarios | builder | none |
| `game-designer` | Default mod rules, economy, balance, first playable loop, room settings schema | builder (content) | none |
| `qa-lead` | Test instruments, headless scenarios, benchmark instrument, acceptance evidence, "can this test go red", agent-first tooling | reviewer + builder (tests) | project |
| `producer` | Plans, milestone sequence, dependencies, scope control, open-decision tracking | reviewer + planner | none |

### Outsiders

| name | Seat | Kind | Memory |
| --- | --- | --- | --- |
| `mod-builder` | Third-party modder. Builds mods with only the public API and documentation. Reports every privileged gap, undocumented step, and sharp edge. Never reads engine internals to work around a gap; files the gap instead. On a spec with no API yet, reads only the sections that define the public surface (5.2, 5.3, 5.5, 4.3) and the acceptance evidence, and reports what it cannot build from them. (G1) | builder (mods) + reviewer | project |
| `player` | Plays or reads the result as a game. Judges clarity, feel, fairness, and fun. Reports in plain language. Does not propose architecture. Not picked for an artefact with no playable loop. (G1) | reviewer | none |
| `spec-rigour-reviewer` | Hunts contradictions, undefined terms, untestable claims, status drift (Open treated as Decided), and acceptance evidence that cannot fail. Stack-agnostic. Also the STE reader. | reviewer | none |

### Ownership of spec sections

| Spec section | Primary | Second |
| --- | --- | --- |
| 2 Decision register | technical-director | producer |
| 3, 3.2 Goals and exclusions | game-designer | technical-director |
| 3.1 Goals (browser-first, platforms) | web-platform-engineer | game-designer |
| 4, 4.1, 4.2 Architecture, modules, entities | technical-director | simulation-core-engineer |
| 4.1 Presentation module; 4.3 Interface row; 3.1 rendering and camera paragraph | bevy-rendering-engineer | technical-director |
| 4.3 Engine mechanisms versus mod policy | mod-runtime-engineer | technical-director |
| 4.4 Separate simulation, replication, presentation representations | technical-director | network-engineer |
| 5 (5.1–5.5, 5.7) Mod system | mod-runtime-engineer | technical-director |
| 5.6 Seeded generation libraries | world-engineer | mod-runtime-engineer |
| 5.8 Compiled 3D asset pipeline | bevy-rendering-engineer | mod-runtime-engineer |
| 6.1 Shared simulation principles | simulation-core-engineer | technical-director |
| 6.2 Authoritative server simulation | network-engineer | performance-architect |
| 6.3 State requiring synchronization | network-engineer | backend-engineer (save contract) |
| 6.4 Discovery and hosting topology | backend-engineer | network-engineer |
| 6.5 Automatic mod acquisition | mod-runtime-engineer | web-platform-engineer (browser storage) |
| 6.6 Rooms, creators, configuration | backend-engineer | game-designer (settings schema) |
| 6.7, 6.8 Match start, slots, ownership | backend-engineer | spec-rigour-reviewer |
| 6.9 Scale, command processing, replication | network-engineer | performance-architect (budgets) |
| 6.10, 6.11 Networking reuse, spectators | network-engineer | backend-engineer |
| 6.12 Multiple maps per match | backend-engineer | world-engineer |
| 6.13 Server implementation and identity | backend-engineer | technical-director |
| 7, 8 Resources, production, logistics, research | simulation-core-engineer | game-designer |
| 9, 10, 11 Terrain, geology, underground | world-engineer | simulation-core-engineer |
| 12 Cross-system invariants | technical-director | spec-rigour-reviewer |
| 13, 13.1, 13.3, 13.9 Performance strategy, research status, prototype gates | performance-architect | qa-lead (13.2 benchmark instrument) |
| 13.4 Large-scale rendering and animation | bevy-rendering-engineer | performance-architect |
| 13.5 Simulation work reduction and visibility | simulation-core-engineer | performance-architect |
| 13.6 Navigation, steering, terrain changes | world-engineer | simulation-core-engineer |
| 13.7 Compact replication and large-world residency | network-engineer | performance-architect |
| 13.8 Benchmark expansion and acceptance evidence | qa-lead | performance-architect |
| 14 Validation milestones | producer | qa-lead (acceptance evidence); performance-architect for M7, M12 |
| 15, 18 Open decisions, next step | producer | technical-director |
| 16.1 Development workflow | qa-lead | producer |
| 16.2 First playable default mod | game-designer | player |
| 17 Conventional AI opponents | ai-engineer | game-designer |

Headless host: `backend-engineer` owns the host executable. `simulation-core-engineer` owns
the headless-runnable core. Both seat texts say this. (G1)

Parent headings 5, 6, and 16 have no body of their own; their subsections carry the owners.
(G1b)

## 5. Agent body structure

Every agent file has the same skeleton. The builder fills each section per role.

1. **Identity.** One paragraph: who you are, what seat you hold, what you are not responsible for.
2. **Read first.** The spec sections this seat owns (section 4 table). The agent cites the
   decision IDs it depends on. `Cargo.toml` when it exists.
3. **Standing rules.** A verbatim block, identical in every file. Its single source is
   `docs/agents/standing-rules.md`. The builder copies it; acceptance diffs each copy against
   the source. (G1) The block says:
   - Verify, never assume. Check every claim against code, a document, or a run before you
     report it. Cite `path:line` for each claim.
   - Label what you did not verify `NOT TRACED`. Do not build on it.
   - Respect spec status. Decided items are not open. Open items are not decided. When you
     comment on an item, state its status. If a decision is needed, say "needs decision" and
     give the options with evidence.
   - Reviewers run things. A review that only reads is incomplete. Review the test instrument
     too: state whether each test can fail, and on which build.
   - A run is: a grep that tests a cross-reference or a contradiction, a fetch of primary
     documentation that confirms or refutes a claim, or a probe built in the scratchpad.
     For each run, state the claim it tested and the result that would have refuted it. A
     run with no claim does not count. The orchestrator judges.
   - In review mode, do not call Write or Edit on the repository, and do not write to the
     repository through Bash. Your own memory directory is the one exception. Report
     `What I changed: none`.
   - Keep the report under 600 words. Put long output in a scratchpad file and cite its path.
   - Write in ASD-STE100 Simplified Technical English.
   - Four spaces per indentation level.
   - Throwaway files go in the scratchpad directory, never in the repository tree.
   - Do not spawn agents.
4. **Expertise.** Role-specific knowledge, heuristics, and the questions this seat always asks.
   Concrete to the stack: Bevy schedules and change detection, wasm32 memory model,
   WebSocket vs WebTransport browser support, Rhai sandboxing limits, and so on. No library
   version is named.
   The source file `docs/agents/standing-rules.md` wraps the block in two marker lines,
   `<!-- standing-rules:start -->` and `<!-- standing-rules:end -->`, and the copies keep
   them. (G1b)
5. **Modes.** What the agent does when asked to review-spec, review-plan, build, test, or
   answer. Only the modes the role has. The description's `Modes:` line must match.
6. **Output contract.** Fixed structure. (G1)
   - Line 1, machine-readable:
     `VERDICT=<pass|pass-with-changes|fail|needs-decision> FINDINGS=<n> HIGH=<n> RUNS=<n> NOT_CHECKED=<n>`
   - `Findings`: at most 8 entries, each at most 4 lines: `path:line` or `NOT TRACED`,
     severity, the defect, the fix.
   - `What I ran`: one row per run: `command | claim tested | result that would refute | observed`.
   - `Not checked`: the parts of the artefact not tested, and why.
   - `What I changed`: `none` in review modes; the explicit file list in build and test modes.
   - `Open questions for the orchestrator`.

## 6. How the orchestrator uses the team

Documented in `CLAUDE.md` under a new "Agent team" section with these bullets. (G1)

- **Lens mapping.** Rigour: `spec-rigour-reviewer`. Engine/domain: the Primary of the section
  the artefact touches, from the ownership table. Boundary/sequencing: `technical-director`
  for architecture, `producer` for plans. Consumer: `mod-builder` when the artefact touches
  4.3, 5, or 6.5; `player` when it touches 3, 6.6, 16.2, or 17; otherwise the Second of the
  section.
- **Selection rule.** Four lenses per gate. When an artefact spans several subsystems, the
  orchestrator may add one domain seat per extra subsystem and must say so. Six agents is the
  cap for one gate; above six, ask the user. When a default lens has no claim to test in the
  artefact, drop it and say why. (G1b)
- **Milestone artefacts.** For a spec or plan on a milestone (spec section 14), the engine
  lens is the Primary of the first subsystem in the Demonstration column, and each further
  subsystem adds its Primary. For M1 this yields `web-platform-engineer` (browser client),
  `network-engineer` (authoritative headless server), `mod-runtime-engineer` (mod-defined
  unit). (G1b)
- **Heavy review.** `performance-architect` joins any gate where cost, scale, or a numeric
  budget is at stake (spec 6.2, 6.9, 13, M7, M12). It is an allowed fifth lens, inside the cap
  of six. (G1b)
- **Minimum prompt.** Artefact path, scope, the lens question, the review-mode rule, and the
  scratchpad path. The body carries the rest.
- **Reading reports.** The orchestrator records line 1 and the High findings of each report in
  its working notes. The rest stays in the report.
- **Fan-out reading.** `Explore` (built-in) for raw search. Domain seats for conclusions.
- **Independent refutation.** `performance-architect` or `qa-lead` to try to break a claim.
- **Build.** Domain builder for the crate. `qa-lead` writes or reviews the test instrument.
- **Verify.** `qa-lead` runs. `player` and `mod-builder` judge.
- **Git.** Never `git add -A` while an agent runs. Stage explicit paths. Memory files under
  `.claude/agent-memory/` are staged on purpose, not by accident.

## 7. Acceptance

Each check names its instrument and the result that fails it. (G1: every criterion was
rewritten because the first version could not go red.)

1. **Load.** `claude plugin validate .claude/agents` exits 0. A script extracts the `name:`
   field from every file in `.claude/agents/`; the set equals the sixteen names in section 4,
   with no duplicates and no file without `name`. After one session restart, the sixteen names
   appear in the session's agent-type list. Fails on: a missing name, a duplicate, or a name
   absent from the list.
2. **Standing rules identity.** A script extracts the text between the two marker lines from
   each file and diffs it against the same span in `docs/agents/standing-rules.md`. Zero
   differences. The same script checks that each file names the five output-contract headings
   `Findings`, `What I ran`, `Not checked`, `What I changed`, `Open questions for the
   orchestrator`, and the `VERDICT=` line 1 format. Fails on any diff or missing string. (G1b)
3. **Dry run with a planted defect.** The orchestrator copies spec section 6.5 to a scratchpad
   file and plants one defect: a Decided item restated as Open. It sends the file to
   `spec-rigour-reviewer`, `technical-director`, `mod-runtime-engineer`, `mod-builder`. Pass
   needs all of: each report starts with a valid line 1; each `What I ran` row has a command,
   a claim, a refuting result, and an observed result; every report states the status of the
   planted item and calls it Decided (a report that calls it Open fails); at least one reviewer
   reports it as a finding; every report has `What I changed: none`; `git status --short`,
   with paths under `.claude/agent-memory/` excluded, shows the same tree before and after.
   Fails on any missing item. A failed dry run fails the agent file that produced the bad
   report, not the spec. (G1b)
4. **CLAUDE.md.** The "Agent team" section has every bullet of section 6. A script extracts
   the bold labels from section 6 of this design and greps `CLAUDE.md` for each. Fails on any
   missing label. (G1b)
5. **STE.** `spec-rigour-reviewer`, with the `writing-simplified-technical-english` skill,
   reads every agent body except its own and lists violations with `path:line`. The
   orchestrator reads the rigour file. An instruction sentence is any sentence in the
   Standing rules or Modes sections. Pass bar: zero passive-voice sentences in the
   standing-rules block, zero instruction sentences over 25 words. Fails on one violation
   above the bar. (G1b)
6. **Description budget.** A script measures each `description` value. Pass: every value is
   900 bytes or less and has a `Modes:` line. Fails on one over.

## 8. Build order

(G1) Files have no references to each other, so any order works, with two constraints:

1. Write `docs/agents/standing-rules.md` first. Every file copies it.
2. Write the four dry-run agents before the rest, so check 3 can run early.
3. Write the "Agent team" section of `CLAUDE.md` after the roster is final and before the
   dry run.
4. Commit at the end: agent files, `docs/agents/`, the design, `CLAUDE.md`, and the spec.
   Explicit paths, no `git add -A`.

## 9. Left to the builder

- Exact wording of each expertise section.
- Choice of `color` values (allowed: red, blue, green, yellow, purple, orange, pink, cyan).
- `permissionMode` is omitted; roles inherit the session mode.
- The extraction scripts for checks 1, 2, and 6 live in `docs/agents/check.sh` or similar,
  not in `.claude/agents/`. Loader behaviour for a non-agent file inside `.claude/agents/` is
  NOT TRACED, so nothing but agent files goes there.
