# Agent file template

Copy this skeleton for every file in `.claude/agents/`. Replace each `<placeholder>`.
Keep the field order, the heading order, the marker lines, and the output contract text.
The `description` value is one physical line in double quotes. Write line breaks as `\n`
and quotes as `\"`. Its parsed value is at most 900 bytes and holds one `<example>` block.

```
---
name: <name>
description: "<One sentence that names the seat and what it judges or builds.> Modes: <review-spec, review-plan, build, test, answer>.\n\n<example>\nContext: <one line>\nuser: \"<one line>\"\nassistant: \"<one line that names this agent>\"\n<commentary>\n<one line>\n</commentary>\n</example>"
tools: <tool list from the Global Constraints>
model: inherit
memory: project            # only technical-director, qa-lead, mod-builder; delete otherwise
color: <red|blue|green|yellow|purple|orange|pink|cyan>
skills: writing-simplified-technical-english   # only spec-rigour-reviewer; delete otherwise
---

## Identity

<One paragraph: who you are, what seat you hold, what you are not responsible for.>

## Read first

Primary: <spec sections>. Second: <spec sections or "none">. Decisions: <IDs>. Read
`Cargo.toml` when it exists. With no workspace, mark every version-dependent fact `NOT TRACED`.
<Memory roles add: You keep notes under `.claude/agent-memory/<name>/`. Read them first. Write only facts you verified.>

## Standing rules

<!-- standing-rules:start -->
<verbatim copy of the block from docs/agents/standing-rules.md>
<!-- standing-rules:end -->

## Expertise

<Role-specific knowledge, heuristics, and the questions this seat always asks. No library version.>

## Modes

<One short paragraph per mode the role has, same list as the description.>

## Output contract

Line 1, machine-readable:
`VERDICT=<pass|pass-with-changes|fail|needs-decision> FINDINGS=<n> HIGH=<n> RUNS=<n> NOT_CHECKED=<n>`

Then these headings, in this order:
- `Findings`: at most 8 entries, each at most 4 lines: `path:line` or `NOT TRACED`, severity, the defect, the fix.
- `What I ran`: one row per run: `command | claim tested | result that would refute | observed`. The claim cell names a statement in the artefact with `path:line`.
- `Not checked`: the parts of the artefact not tested, and why.
- `What I changed`: `none` in review modes; the explicit file list in build and test modes.
- `Open questions for the orchestrator`.
```
