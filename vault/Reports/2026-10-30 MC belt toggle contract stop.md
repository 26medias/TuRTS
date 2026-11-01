---
type: report
date: 2026-10-30
tags: [turts, mc, stop, contract]
ai-first: true
source: codex
---

# MC stop: PT1 belt toggle contract

STOP pending CEO/orchestrator ruling. No workaround selected. Build25/170. Task8 is complete; Task18 and Task19 remain unmerged.

## Conflict and evidence

Design revision11 at main b29e3d8 is authoritative. Its PT1 player path step6 requires the tester to select a belt, disable it with D, observe that it stops and an arm loader waiting alert within10s, then enable it with E (docs/superpowers/specs/2026-10-29-mc-player-control-design.md:1693). But section5.3 preserves Produce/CancelProduction/Remove/SetEnabled as the M2/M3 mechanism (:532).

The existing SetEnabled mechanism in crates/turts_sim/src/command_m3.rs:73-80 accepts extractors, inserters, machines, research, producers and weapons. It excludes conveyors and returns NotToggleable for them. The existing client uses the same applicability in crates/turts_client/src/economy_ui.rs:309-315. Plan Task2b passes SetEnabled through to the existing mechanism (docs/superpowers/plans/2026-10-29-mc-player-control.md:341). Task4 adds the order machinery but assigns no conveyor toggle extension (:455-472); Task21 migrates the complete PT1 player path into its oracle (:676). The current package filters likewise exclude belts. Thus the required belt step cannot pass with the assigned existing mechanism.

The brief requires: "Stop and report to the CEO, do not decide" when "a plan step contradicts the design or the spec" (vault/Reports/2026-10-29 MC Codex brief.md:47). No change to design, plan, scanner, belt behavior or PT1 oracle was made. No frozen value changed.

A ruling must either revise the PT1 belt requirement to an existing supported interaction, or explicitly assign belt-toggle behavior and its validation to a build task. This report does not choose either outcome. Any revised alert timing also needs a defined acceptance check; it has not been traced for an alternative target.

## Completed Task8 and O10

O10 is logged in [[Decisions/2026-10-30 MC ruling O10]]. Integration and open lanes were rebased on b29e3d8. Effect::ShellAction retains its action field and behavior. Scanner and existing content unchanged. Future client module is client_shell.rs.

Task8 candidate01621d0922e66ec20a5350eaa3029255cbf24bc0 merged at461d10555ad3a34d2c87e81bb7bfcf7aa2496674. Review0High/0Medium. Full suite861passed/0failed/18ignored. Original missing-row red101/green0 retained; lock-seed issue fixed with named semantic red101/green0; independent missing-row plant101, byte-identical restoration and boundary13/13 green0. Integration G-RUST/G-QA/G-FROZEN all10commands exit0; frozen stream hash unchanged. Source-clean completed lane removed. [[Reports/2026-10-30 MC T8 review]].

## Open lanes at stop

Task18 RUN23 builder HEAD0e83be525b3e7cfa237957b349dc56db77d5affb, clean. Its four focused tests, five package packs, five final scans, fmt/Clippy/wasm/boundary, G-QA and G-FROZEN passed. Full workspace test failed in the old HUD3 fixture on duplicate orders export; output retained but outer process exit code was not captured, so it is not reported as a complete gate receipt. G-ACC was interrupted at e9_loss_and_small_buffer_converge, outer exit130, not accepted; remaining steps did not run. No plants ran. No task-owned compiler/server/test process remained after cleanup.

Task18 RUN25 initial independent review has0High/4Medium, not a PASS: Forge Move fire policy; missing Cedar/Haul production/cancel orders; accepted-node Gather priority; Forge Node movement. Additional Forge subject coverage omits lab/turret although the existing mechanism permits them. These findings and the stale HUD3 fixture are unaddressed. Root authorized routine repair before the conflict was found, but builder made no such repairs before stop. Reviewer ran only read-only checks, no Cargo or plants. All21PNG icons decode, four HUDs text-only,22frozen content files byte-identical. Actual resolver execution remains NOT TRACED until the owning later task. Earlier scan hits were only new display names Move/Stop, not prescribed table IDs; all approved IDs stayed fixed.

Task19 RUN24 HEADf564f88bd3bf91162e9fce950866fc363206c2c2. Sole change: untracked crates/turts_pack/tests/muster.rs, SHA3fc474607e563d0d3d902b8d32592964cf68f691ab3291e6cb9f7f2fe0046acc. Partial external package draft preserved; no HUD or fixture yet. No Cargo/scan/package implementation ran. Test archived in target/mc-evidence/belt-toggle-stop/muster.rs in main worktree.

## CI, disk and preservation

Wave1 CI41097571682 on pushed8e95e42: agents success; T7 success; browser has the unchanged E9-browser-base-economy 15000ms timeout; later browser steps skipped. Native check still in progress at M2 native refutations. No hosted success claimed. Link: https://github.com/26medias/TuRTS/actions/runs/41097571682. No Wave2 push because the wave is incomplete. Rebased integration history will need a lease-protected push when the wave completes. Main remains b29e3d8 and was not written by Codex.

One warm cache retained in Task18 target/mc-t18. Earlier cleanup reclaimed24.24GiB; avoidable debug output later removed another approximately56GiB of allocated disk after provenance archive and idle-process check. All subsequent Cargo commands explicitly use debug0/testdebug0/incremental0/jobs4. No source/evidence removed. At stop, free disk is about700GiB; current output retained for resume rather than duplicate compilation.

Primary receipts in main worktree: target/mc-evidence/belt-toggle-stop/root-stop-state.json; t18-builder/logs; t18-review/initial-findings.md; after-t8; t8-review/final-review.md; o10-rebase/after.json; o10-cache-cleanup. All lanes stopped; no further dispatch is authorized before the ruling. No main merge or push is authorized.
