# Full spec review — common brief for every seat (phase 2: gap audit)

## Why this review exists
The CEO's goal, verbatim (2026-10-28): **"Build the Ultimate RTS engine."** Read it as the union of what
StarCraft 1/2, Command & Conquer, Age of Empires 2, Warcraft 3, Supreme Commander and Factorio-style industrial
RTS give a player and a modder, through the public mod API.

After four milestones (M1 to M3 built, M4 drafted), the orchestrator found that no spec, plan or milestone covers the
core RTS control loop: drag-select, group orders, control groups, hotkeys. The client selects one unit at a time
(`crates/turts_client/src/economy_ui.rs:31`). Sixteen agent reviewers at eight gates missed it. The CEO: "I do not
trust the specs, I do not trust the team and I do not trust you. I want a full review of the project specs and plans.
I want the whole team involved. I want the specs to match the project goals."

Root cause: every gate compared the spec with itself. Nobody compared it with an outside list of what an RTS is.
Phase 1 built that list with two seats that had NOT read the spec first. You now audit the spec against it.

## Inputs (read these; do not re-explore the whole repo)
- Yardstick A (engine side, 355 items, 25 areas, CORE/COMMON/OPTIONAL): `scratchpad/review/inventory-engine.md`
- Yardstick B (player side, first ten minutes of six games, 40 union items): `scratchpad/review/inventory-player.md`
- The spec: `RTS-Engine-Project-Specifications.md` (v0.7). Cite as S:line. Respect statuses Decided/Baseline/Open/Proposed.
- Seat ownership table: `docs/superpowers/specs/2026-10-25-studio-agent-team-design.md:128-164`.
- Milestone documents, read the ones your lens needs: `docs/superpowers/specs/2026-10-26-m1-foundation-design.md`,
  `2026-10-26-m2-economy-design.md`, `2026-10-27-m3-industrial-design.md`, `2026-10-28-m4-terrain-design.md` (draft, not
  gated), and the plans in `docs/superpowers/plans/`.
- What the CEO saw when playing M3: `vault/Reports/2026-10-28 M3 human playtest repair.md`, `vault/M3 Playtest Guide.md`.
- Scratchpad root: `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/review/`

## Your task
1. For every yardstick item in YOUR areas (your brief names them), give one verdict:
   `present` (designed, S:line), `partial` (named, no design, S:line), `missing` (no mention; say the terms you searched),
   `wrong-milestone` (planned, but after something that depends on it), or `conflict` (a Decided/Baseline statement
   blocks the goal).
2. Add the gaps in YOUR domain that the yardsticks did not know to ask about. You are the expert; the yardstick is not.
3. For every gap: engine mechanism, mod policy, or both (spec 4.3 split); the milestone it belongs to and why; what it
   depends on; whether anything built in M1 to M3 must change (cite code path:line if you check; else NOT TRACED).
4. State whether any Decided item in section 2 conflicts with the goal. Do not reopen Decided items; name the conflict.
5. Answer your lens question (in your brief).
Do NOT design solutions. Find and classify gaps. One sentence of direction per gap is enough.

## Output format (so the orchestrator can merge 16 files)
Write `scratchpad/review/gap-<seat>.md` with:
- Line 1: `VERDICT=<pass|pass-with-changes|fail> ITEMS=<n> MISSING=<n> PARTIAL=<n> CONFLICT=<n> RUNS=<n> NOT_TRACED=<n>`
- A table, exactly these columns:
  `| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |`
  Use the yardstick ids (A-xx / B-xx) where they exist; your own gaps get `<seat>-N`.
- Then: "Conflicts with the goal" (Decided items), "Lens answer", "What I ran", "NOT TRACED".
Your final message: 400 words or fewer. Line 1 = the VERDICT line. Then the top findings and the file path.

## Rules (standing; they apply to you)
- Verify, never assume. Cite S:line or path:line for every claim. Label what you did not check NOT TRACED. List what
  you ran. Plain `grep`/`wc` output is filtered by a hook here: use `rtk proxy grep -n -i -E` and Python for counts.
- Reviewers run things where a thing can be run (a grep sweep, a Python count, a probe). A review that only reads is a lead.
- Do not edit repo files. Write only under the scratchpad review directory.
- Write in ASD-STE100 Simplified Technical English: short active sentences, one idea per sentence, every number kept.
