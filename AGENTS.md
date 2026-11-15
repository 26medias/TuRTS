# TuRTS working rules

These rules implement the CEO's instructions of 2026-11-14.
They apply to this repository, its worktrees, and all assigned agents.
Current direct user instructions take precedence over these rules.
Do not import Claude instructions or configuration.

## Goal and priorities

Build the full mod-first RTS engine defined in `RTS-Engine-Project-Specifications.md`.
The playable skirmish tests the engine. It does not replace the engine scope.
Finish as fast as possible without reducing quality or hiding missing capabilities.
Maintain a working browser build while completing the engine.
Prefer complete engine capabilities over repeated demo polish.
Choose work by dependency, player value, engine reuse, and measured risk.
Park optional work when it delays required capabilities. Report that choice briefly.

## Act and keep working

The CEO authorizes parallel implementation, review agents, worktrees, tests, and integration on branches other than main.
Use that authorization without asking again.
Make technical decisions yourself. Ask only about consequential player choices or information that cannot be recovered.
Continue until the assigned objective is complete or a real blocker prevents useful work.
A status question does not cancel or pause work. Answer briefly, then continue.
Fix test failures, performance regressions, and integration problems without waiting for a new instruction.
Report a blocker only after trying reasonable alternatives. State the exact missing input or external dependency.
Honor explicit stop conditions in the current task. A local fixable failure is not a reason to abandon the objective.
Do not promise calendar dates without measured delivery data and a defined scope.

## Organize for parallel delivery

Keep one short current plan with the next deliverables, owners, dependencies, and acceptance checks.
Use the existing task tools or a short working document. Do not create a second tracking system.
Split work into independent capabilities with clear file or component ownership.
Use parallel agents whenever independent work can shorten delivery.
Give each agent a concrete result, a scope boundary, relevant paths, and required checks.
Use separate worktrees for concurrent implementations that could conflict.
Share a checkout only for clearly separated file ownership or read-only checks.
Keep shared API and protocol changes under one owner. Publish their contracts before dependent implementations diverge.
Integrate completed work early on a non-main integration branch.
The orchestrator also implements and integrates work. Do not spend the whole turn supervising agents.
Reuse existing agent context. Avoid duplicate investigations and duplicate reviews.
Schedule builds to avoid Cargo locks, excess RAM use, and disk exhaustion.
When sharing a native Cargo target across worktrees, use `~/.codex/bin/turts-cargo`.
It invalidates target-specific local Cargo fingerprints on a worktree change.
It preserves source timestamps and unrelated browser-cache fingerprints.
Cross-target builds also refresh changed local host build-script and proc-macro fingerprints.
Serialize GPU measurements and browser workloads that would invalidate each other's results.

## Specs use Anvil

Use the standalone Anvil skill when writing or materially revising a feature spec.
Use relevant subagents for review. Choose reviewers by the contracts and failure modes involved.
State a small agent budget before the review. One focused reviewer is the default.
Add a second reviewer only when a distinct foundation risk warrants one.
Review the spec, incorporate findings, write a concise implementation plan, and have the same reviewers review the plan.
Then implement in parallel where dependencies permit it.
Use Codex collaboration tools and available models. Do not import Claude agents or settings.
If a named Workflow tool or helper skill is unavailable, perform its steps with Codex tools directly.
Unavailable orchestration wrappers do not block implementation.
Use the supported `collaboration.send_message` tool for reviewer findings.
Do not restart Anvil for unchanged approved specs, ordinary fixes, or acceptance evidence.
Fix review findings directly. Request a focused recheck only for an unresolved material issue.
Do not repeat the whole review after a pass.

## Quality checks that protect the product

For server authority, protocol, determinism, and mod API contracts, keep a short spec and one independent review that runs code.
Review the completed foundation change as a whole. Fix its findings.
For other features, implement, run, test, and integrate. At most one review at the end of the whole feature.
Use tests that check observable behavior, contract boundaries, and meaningful failure cases.
Run focused checks during implementation. Run the required full gates and acceptance script once per integration push.
Repeat checks only when a change or failure makes their earlier result insufficient.
Measure performance with representative workloads before claiming an improvement.
Preserve rendering quality and game rules when optimizing. Do not lower detail to disguise a regression.
Record acceptance results and evidence paths in one concise feature report.
Do not claim a result without running the relevant check. State what remains unverified.
Do not ask the CEO to play until `docs/qa/cold-play.md` scripted probes pass on the final build.
Then give one line describing the action and the expected result.

## Remove process waste

Old lane gates, binding amendments, fault registries, receipt ledgers, and per-task review rules do not bind this workflow.
Do not add planted faults, ledgers, rulings, or gates to maintain process machinery.
If an obsolete instrument blocks correct behavior, repair or remove it. Report the change in one line.
Do not remove tests that protect a real contract merely to obtain a pass.
Use existing evidence and tools when they answer the question. Do not rebuild them for presentation alone.

## Protect the workspace and the CEO

The CEO gives standing approval to merge tested milestones into main.
Before each merge, pass the milestone acceptance checks and required independent review.
Include a `README.md` progress update in the milestone. State completed capabilities, validation, remaining work, and known limits.
Do not merge incomplete milestones. The CEO also authorizes pushing each tested milestone after its merge into main.
This standing push approval was given on 2026-11-15.
Use explicit paths with `git add`. Never use `git add -A`.
Preserve unrelated edits and uncommitted specs. Do not overwrite or delete them during integration.
Every test Chrome must use `--alsa-output-device=null` and a fresh profile.
Never touch the CEO's Chrome, its profile, or system sound settings.
The unit that arrives first keeps a contested point. A later unit stops nearby without overlap.
Do not use unit IDs to determine a visible outcome.
Check free disk space before large builds or creating build caches.
Reuse compatible build caches. Delete rebuildable outputs of finished lanes after checking active processes and runtime dependencies.
Keep `target/mc-evidence`, source changes, and the accepted build's runtime dependencies.

## Communicate briefly

Use Simplified Technical English. Use short sentences with one idea each.
Give brief progress updates during long work. Updates must not stop implementation.
At the end of a working day, state what the build now supports, what was parked, and any real blocker.
At delivery, state what changed, what ran, evidence paths, remaining limits, and any spec correction.
