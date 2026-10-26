# Studio Agent Team Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

Revision 2 after gate 2 (four lenses). Changes from gate 2 are marked `(G2)`.

**Goal:** Sixteen Claude Code subagent files in `.claude/agents/`, one standing-rules source, one template, one check script, and an "Agent team" section in `CLAUDE.md`, all passing the six acceptance checks of the design.

**Architecture:** Each agent is a Markdown file with YAML frontmatter and a fixed body skeleton given by a template. A single source file holds the standing-rules block; every agent copies it between marker lines. A shell script is the test instrument: it fails on any file that breaks the format, the budget, the block identity, or the contract headings. The dry run with a planted defect is the only check that needs live agents, and it needs a session restart first.

**Tech Stack:** Claude Code 2.1.310 subagent format, bash, python3 (`-I`), `claude plugin validate`.

**Spec:** `docs/superpowers/specs/2026-10-25-studio-agent-team-design.md` (revision 3). Read it in full before any task. Section numbers below refer to it.

## Global Constraints

- Sixteen agent files, names exactly as in design section 4. File name equals `name` plus `.md`; `check.sh` enforces it. (G2)
- Frontmatter fields, in this order: `name`, `description`, `tools`, `model: inherit`, `memory: project` (three roles only), `color`, `skills` (one role only). No `effort`, no `permissionMode`. (G2)
- `description` is one physical line, double-quoted, with line breaks written as `\n` and quotes as `\"`. Its parsed UTF-8 value is 900 bytes or less, holds exactly one `<example>` block, starts with a sentence that names the seat, and holds one `Modes:` line. (G2: one example is a call that tightens design section 3; the design is aligned.)
- Tools. Reviewer-only (`performance-architect`, `player`, `spec-rigour-reviewer`): `Read, Grep, Glob, Bash, WebFetch, WebSearch`. All others add `Write, Edit`. No role has `Agent` or `Skill`. `spec-rigour-reviewer` alone has `skills: writing-simplified-technical-english` (preload; a subagent without the `Skill` tool cannot invoke a skill on its own). (G2)
- `memory: project` only on `technical-director`, `qa-lead`, `mod-builder`.
- Standing-rules block is byte-identical in every file, between `<!-- standing-rules:start -->` and `<!-- standing-rules:end -->`.
- Output contract: line 1 `VERDICT=<pass|pass-with-changes|fail|needs-decision> FINDINGS=<n> HIGH=<n> RUNS=<n> NOT_CHECKED=<n>`, then headings `Findings`, `What I ran`, `Not checked`, `What I changed`, `Open questions for the orchestrator`.
- Reports under 600 words (`wc -w`). Findings at most 8, each at most 4 lines.
- No library version named in any file. Agents read `Cargo.toml` first; with no workspace, version facts are `NOT TRACED`.
- Prose in ASD-STE100 Simplified Technical English. Four spaces per indentation level.
- Nothing but agent files goes in `.claude/agents/`. Scripts and the template live in `docs/agents/`.
- Never `git add -A`. Stage explicit paths.
- Run `bash docs/agents/check_fixtures.sh` and `bash docs/agents/check.sh` before every commit from Task 2 on. (G2)

## Review Focus

1. A file with valid YAML but no `name:` is skipped by the loader as a reference document, and `claude plugin validate` passes it. Test: Task 1 fixture `no_name`.
2. A description of 901 parsed bytes gives no warning until the whole roster crosses 15,000 tokens. Test: Task 1 fixture `desc_901`, which asserts its own planted size.
3. A memory role writes under `.claude/agent-memory/` during the dry run. Test: Task 6 step 2 excludes that path from the tree diff and stages the memory files on purpose.
4. A copied block drifts by one byte. Test: Task 1 fixture `block_drift`.
5. A reviewer returns `VERDICT=pass` with grep rows whose claim cell is empty or names no statement. Test: Task 6 step 3 fails such a report on the claim cell.

---

### Task 1: Standing-rules source, template, and check script

**Files:**
- Create: `docs/agents/standing-rules.md`
- Create: `docs/agents/TEMPLATE.md`
- Create: `docs/agents/check.sh`
- Create: `docs/agents/check_fixtures.sh`

**Interfaces:**
- Produces: `docs/agents/check.sh [AGENTS_DIR] [SOURCE]`. Defaults: `.claude/agents` and `docs/agents/standing-rules.md`, both relative to the current directory; `SOURCE` may also come from the `SOURCE` environment variable. Exit 0 when all checks pass. Exit 1 and one line per failure, `FAIL <file> <check>: <detail>`. The validator exit code is read directly, never through a pipe. (G2) Checks:
  - `names`: the `name:` set equals the sixteen names; no duplicate; no file without `name:`; file name equals `<name>.md`.
  - `validate`: `claude plugin validate AGENTS_DIR` exits 0.
  - `desc`: one physical line, double-quoted; parsed value 900 bytes or less; exactly one `<example>`; contains `Modes:`.
  - `tools`: the comma-split set equals the role's set from the Global Constraints. Order is free.
  - `block`: the text between the marker lines is byte-identical to the same span in `SOURCE`.
  - `contract`: the five heading strings and `VERDICT=` are present.
  - `model`: `model: inherit` present.
  - `memory`: `memory: project` on exactly the three roles.
  - `skills`: `skills:` present only on `spec-rigour-reviewer`, with the one skill name.
- Produces: the standing-rules block text and the template, consumed verbatim by Tasks 2 to 4.

- [ ] **Step 1: Write `docs/agents/standing-rules.md`**

Content: a one-line title, then the block between the two marker lines. The block is the eleven bullets of design section 5 item 3, in STE, with the memory exception sentence from revision 3. Nothing else between the markers.

- [ ] **Step 2: Write `docs/agents/TEMPLATE.md`** (G2)

The exact skeleton every agent copies. It fixes: frontmatter field order from the Global Constraints; the description as one double-quoted line in the form `"<seat sentence> Modes: <list>.\n\n<example>\nContext: ...\nuser: \"...\"\nassistant: \"...\"\n<commentary>\n...\n</commentary>\n</example>"`; body headings at level `##` in this order: `Identity`, `Read first`, `Standing rules` (markers inside), `Expertise`, `Modes`, `Output contract`; the `Read first` form `Primary: <sections>. Second: <sections>. Decisions: <IDs>. Cargo.toml when it exists.`; the memory sentence for memory roles: `You keep notes under .claude/agent-memory/<name>/. Read them first. Write only facts you verified.`; the output contract text verbatim from design section 5 item 6. Placeholders are in angle brackets.

- [ ] **Step 3: Write the failing fixture test `docs/agents/check_fixtures.sh`**

Builds fixtures in `$SCRATCH/fixtures/<case>/.claude/agents/` from the real `SOURCE` block and the template, runs `check.sh` on each with that `SOURCE`, and compares the exit code. `SCRATCH` defaults to `/tmp/claude-1000/-home-julien-Projects-TuRTS/9b770ab5-f8ef-4e4e-b47b-449c4f660eb4/scratchpad`. Cases:

```bash
# case            expected  what is planted
good              0         sixteen minimal valid files
no_name           1         one file with frontmatter but no name:
dup_name          1         two files with the same name:
filename_mismatch 1         one file whose name differs from the file name
desc_901          1         one description whose parsed value is 901 bytes (the script asserts the count it planted)
two_examples      1         one description with two <example> blocks
desc_multiline    1         one description as a YAML block scalar
block_drift       1         one block with a trailing space on one line
missing_heading   1         one file without "Not checked"
agent_tool        1         one file with Agent in tools
reviewer_write    1         player with Write in tools
memory_wrong      1         performance-architect with memory: project
memory_missing    1         qa-lead without memory: project
no_inherit        1         one file without model: inherit
skills_wrong      1         producer with a skills: line
```

Each case prints `PASS <case>` or `FAIL <case> expected <n> got <m>` and, on failure, the `FAIL` lines from `check.sh`. The script exits 1 on any FAIL.

- [ ] **Step 4: Run the fixture test to verify it fails**

Run: `bash docs/agents/check_fixtures.sh`
Expected: every case FAILs because `check.sh` does not exist.

- [ ] **Step 5: Write `docs/agents/check.sh`**

Bash plus an inline `python3 -I` block for frontmatter parsing, no YAML library. Parse `key: value` lines between the first two `---` lines. For `description`, accept only one physical line that starts and ends with a double quote; decode `\n`, `\"`, `\\`; measure the decoded UTF-8 bytes. The sixteen names and the per-role tool sets are literal arrays at the top of the script.

- [ ] **Step 6: Run the fixture test to verify it passes**

Run: `bash docs/agents/check_fixtures.sh`
Expected: fifteen `PASS` lines, exit 0.

- [ ] **Step 7: Commit** (G2: the spec and `CLAUDE.md` are staged here so later commits do not cite an untracked spec.)

```bash
git add docs/agents/standing-rules.md docs/agents/TEMPLATE.md docs/agents/check.sh docs/agents/check_fixtures.sh RTS-Engine-Project-Specifications.md CLAUDE.md
git commit -m "Add agent standing rules, template, and check script"
```

### Task 2: The four dry-run agents

**Files:**
- Create: `.claude/agents/spec-rigour-reviewer.md`
- Create: `.claude/agents/technical-director.md`
- Create: `.claude/agents/mod-runtime-engineer.md`
- Create: `.claude/agents/mod-builder.md`

**Interfaces:**
- Consumes: `docs/agents/standing-rules.md`, `docs/agents/TEMPLATE.md`, `docs/agents/check.sh`, all committed by Task 1.

- [ ] **Step 1: Run the check to verify it fails**

Run: `bash docs/agents/check.sh`
Expected: `FAIL` on `names` (sixteen missing), exit 1.

- [ ] **Step 2: Write the four files from the template**

Seat text, kind, memory, and tools come from design section 4 and the Global Constraints. Each `Read first` lists Primary and Second from the design ownership table. Role content:

- `spec-rigour-reviewer`: Modes `review-spec, review-plan`. Second: 6.7, 6.8, 12. `skills: writing-simplified-technical-english`. Expertise: status drift test (quote the item, state its spec status, state how the artefact treats it), undefined-term hunt, "can this test go red" per acceptance item, STE reader duty.
- `technical-director`: Modes `review-spec, review-plan, build, answer`. Primary: 2, 4, 4.1, 4.2, 12. Second: 3, 3.2, 4.3, 5, 6.1, 6.13, 15, 18. `memory: project`. Expertise: decision register D01 to D59, crate boundary proposals, cross-system invariants, API versioning policy.
- `mod-runtime-engineer`: Modes `review-spec, review-plan, build, test, answer`. Primary: 4.3, 5 (5.1 to 5.5, 5.7), 6.5. Second: 5.6. Expertise: sandbox budgets, manifest fields, lockfile semantics, the script-per-frame anti-pattern.
- `mod-builder`: Modes `review-spec, review-plan, build, test`. Reads on a spec review only 4.3, 5.2, 5.3, 5.5 and acceptance evidence; writes a sample mod definition from those alone; files one finding per gap where engine internals would be needed. `memory: project`.

- [ ] **Step 3: Run the checks**

Run: `bash docs/agents/check_fixtures.sh && bash docs/agents/check.sh`
Expected: fixtures pass; `check.sh` fails only on `names` (twelve missing).

- [ ] **Step 4: Loader probe** (G2)

Call the Agent tool with `subagent_type: mod-builder` and the prompt "Reply with the single word READY". Record the result in the task notes. If the tool rejects the type, the session started without `.claude/agents/` and will not load it; Task 6 then needs a restart. Either way, continue.

- [ ] **Step 5: Commit**

```bash
git add .claude/agents/spec-rigour-reviewer.md .claude/agents/technical-director.md .claude/agents/mod-runtime-engineer.md .claude/agents/mod-builder.md
git commit -m "Add the four dry-run agents"
```

### Task 3: Studio core builders

**Files:**
- Create: `.claude/agents/performance-architect.md`
- Create: `.claude/agents/simulation-core-engineer.md`
- Create: `.claude/agents/world-engineer.md`
- Create: `.claude/agents/network-engineer.md`
- Create: `.claude/agents/web-platform-engineer.md`
- Create: `.claude/agents/bevy-rendering-engineer.md`
- Create: `.claude/agents/backend-engineer.md`
- Create: `.claude/agents/ai-engineer.md`

- [ ] **Step 1: Write the eight files from the template**

All builder seats here have Modes `review-spec, review-plan, build, test, answer`. (G2)

- `performance-architect`: Modes `review-spec, review-plan, test, answer`. Reviewer-only tools, no memory. Primary: 13. Second: 6.2, 6.9, 14 (M7, M12). Expertise: per-tick cost model (entities × systems × bytes touched), cache-line and struct-of-arrays reasoning, allocation hunting, bandwidth bytes per entity per tick, "what would make this ten times cheaper" on every design, measurement before opinion.
- `simulation-core-engineer`: Primary 6.1, 7, 8. Second: 4, 4.1, 4.2, 9, 10, 11. Headless-runnable core; explicit scheduling; seeded randomness; no rendering plugin in the simulation crate.
- `world-engineer`: Primary 5.6, 9, 10, 11. Second: 6.12. Heightfield, buried deposits apart from the surface, fixed-elevation infrastructure (D09 to D13, D16, D17).
- `network-engineer`: Primary 6.2, 6.3, 6.9, 6.10, 6.11. Second: 6.4. Authority, replication, interest management, transport candidates; no lockstep (D18).
- `web-platform-engineer`: Primary 3.1. Second: 6.5. wasm32 memory model, browser storage eviction, cross-origin, load path; browser first (D02, D54).
- `bevy-rendering-engineer`: Primary 4.1 presentation module, 4.3 interface row, 3.1 rendering and camera paragraph. Presentation consumes simulation state; interpolation; camera and lighting as mod-controlled settings (D37).
- `backend-engineer`: Primary 6.4, 6.6, 6.7, 6.8, 6.12, 6.13. Second: 6.3 save contract, 6.10, 6.11. Headless host executable; rooms; permanent slots (D25); SQLite (D40); identity keys (D50 to D52).
- `ai-engineer`: Primary 17. Modular controllers, priorities and reservations (D47 to D49), no model inference (D45).

- [ ] **Step 2: Run the checks**

Run: `bash docs/agents/check_fixtures.sh && bash docs/agents/check.sh`
Expected: fixtures pass; `check.sh` fails only on `names` (four missing).

- [ ] **Step 3: Commit**

```bash
git add .claude/agents/performance-architect.md .claude/agents/simulation-core-engineer.md .claude/agents/world-engineer.md .claude/agents/network-engineer.md .claude/agents/web-platform-engineer.md .claude/agents/bevy-rendering-engineer.md .claude/agents/backend-engineer.md .claude/agents/ai-engineer.md
git commit -m "Add studio core builder agents"
```

### Task 4: Remaining roles

**Files:**
- Create: `.claude/agents/game-designer.md`
- Create: `.claude/agents/qa-lead.md`
- Create: `.claude/agents/producer.md`
- Create: `.claude/agents/player.md`

- [ ] **Step 1: Write the four files from the template**

- `game-designer`: Modes `review-spec, review-plan, build, answer`. Primary: 3, 3.2, 16.2. Second: 3.1, 6.6 settings schema, 7, 8, 17. Rules are mod policy, not engine rules (D55).
- `qa-lead`: Modes `review-spec, review-plan, build, test`. `memory: project`. Primary: 16.1. Second: 13.2 benchmark instrument, 14 acceptance column. Every test must be shown to go red before it is trusted; headless scenario runner; benchmark instrument on the 4-vCPU, 8-GB budget (D57, D58).
- `producer`: Modes `review-spec, review-plan, answer` (G2). Primary: 14, 15, 18. Second: 2, 16.1. Sequencing, dependencies between milestones, open-decision register; flags scope creep against the decision register.
- `player`: Modes `review-spec, test`. Reviewer-only tools. Second: 16.2. Plain language only; judges a playable loop; declines an artefact with no playable loop with `VERDICT=needs-decision` on line 1.

- [ ] **Step 2: Run the full checks**

Run: `bash docs/agents/check_fixtures.sh && bash docs/agents/check.sh`
Expected: both exit 0, no `FAIL` line.

- [ ] **Step 3: Commit**

```bash
git add .claude/agents/game-designer.md .claude/agents/qa-lead.md .claude/agents/producer.md .claude/agents/player.md
git commit -m "Add remaining studio agents"
```

### Task 5: CLAUDE.md section and load check

**Files:**
- Modify: `CLAUDE.md` (append section "Agent team")
- Create: `docs/agents/check_claude_md.sh`

- [ ] **Step 1: Write `docs/agents/check_claude_md.sh`**

Extracts every `- **Label.**` bold label from section 6 of the design file and greps `CLAUDE.md` for each. Prints `FAIL <label>` per miss, exit 1 on any miss.

- [ ] **Step 2: Run it to verify it fails**

Run: `bash docs/agents/check_claude_md.sh`
Expected: one `FAIL` line per label (eleven at design revision 3), exit 1.

- [ ] **Step 3: Append the "Agent team" section to `CLAUDE.md`**

Content: the bullets of design section 6 with the same bold labels, plus one line that names `docs/agents/check.sh` as the roster check and one line that says a new `.claude/agents/` directory loads only at session start. Run the script again. Expected: exit 0.

- [ ] **Step 4: Validate and commit**

Run: `claude plugin validate .claude/agents`. Expected: `Validation passed`.

```bash
git add CLAUDE.md docs/agents/check_claude_md.sh docs/superpowers/specs/2026-10-25-studio-agent-team-design.md docs/superpowers/plans/2026-10-25-studio-agent-team.md
git commit -m "Add agent team guidance, design, and plan"
```

- [ ] **Step 5: Restart gate** (G2)

If the Task 2 step 4 probe failed, stop here and tell the user: the sixteen agents are committed, and Task 6 needs a new session in this repository. Task 6 resumes in that session.

### Task 6: Dry run and STE check (after restart if needed)

- [ ] **Step 1: Plant the defect**

Extract spec section 6.5 by heading: from the line `### 6.5` to the line before the next `###` heading. (G2: no line numbers.) Save to `$SCRATCH/dryrun/section-6-5.md`. Change the sentence that begins `**Decided user experience:**` to begin `**Open question:**`. Record `git status --short | grep -v agent-memory` to `$SCRATCH/dryrun/before.txt`.

- [ ] **Step 2: Run the four reviewers**

Send the file to `spec-rigour-reviewer`, `technical-director`, `mod-runtime-engineer`, `mod-builder` with the minimum prompt of design section 6: artefact path, scope "gate 1 on spec section 6.5", the lens question, the review-mode rule, the scratchpad path. Save each report to `$SCRATCH/dryrun/<name>.md`. Afterwards, `git status --short | grep -v agent-memory` must equal `before.txt`. Memory files under `.claude/agent-memory/` may appear; they are staged in step 5.

- [ ] **Step 3: Judge the four reports** (G2)

Per report: line 1 matches the `VERDICT=` format; `FINDINGS=` equals the entry count under `Findings`; `NOT_CHECKED=` equals the item count under `Not checked`; every `What I ran` row has four cells and the claim cell names a statement in the artefact with `path:line`; `What I changed: none` is present; `wc -w` under 600. Every report that mentions the planted item states its status as Decided; a report that calls it Open fails. At least one report lists the plant as a finding. Any miss: fix the agent file that produced the bad report, re-run that agent, and record the fix in the commit message.

- [ ] **Step 4: STE check** (G2)

Send `spec-rigour-reviewer` the fifteen other agent files with the pass bar from design check 5. Save the report to `$SCRATCH/ste/report.md`. Read `spec-rigour-reviewer.md` yourself against the same bar. Fix every violation, re-run, repeat until the report lists zero violations above the bar.

- [ ] **Step 5: Commit**

```bash
bash docs/agents/check_fixtures.sh && bash docs/agents/check.sh
git add .claude/agents/<changed files> .claude/agent-memory
git commit -m "Pass dry run and STE check for the agent team"
```

Name every changed agent file in the message. If no agent file and no memory file changed, skip the commit and say so.
