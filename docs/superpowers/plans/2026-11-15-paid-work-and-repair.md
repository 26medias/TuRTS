# Paid work and repair implementation plan

Status: approved by the same Anvil reviewer. Both final clarifications are incorporated.
Spec: `docs/superpowers/specs/2026-11-15-paid-work-and-repair.md`.
Gate 1 findings: `target/mc-evidence/operating-resources/anvil-paid-spec-review.md` in the main checkout.

## Team and ownership

Use the root integration owner and at most three parallel implementation children.
Each child uses a separate worktree based on the accepted operating-resource milestone and the common contract checkpoint.
Keep this plan as the work list. Do not create lane gates or another tracker.

| Owner | Responsibility and file boundary | Acceptance |
| --- | --- | --- |
| Root | Public API/protocol contracts, runtime resolution/validation, shared transaction/accounting hooks, removal/refunds, common order types/dispatch, acquisition/combat integration, audit validation, final integration | PW1, PW6, shared IO1, IO3 |
| Paid work child | `economy/production.rs`, `construction.rs`, new focused paid-work module if needed, and `turts_pack/tests/paid_work.rs` | PW2–PW5 |
| Repair child | `economy/repair.rs`, repair order-behavior helper, its focused tests, and `turts_pack/tests/repair.rs` | RP1–RP4 |
| Client/acceptance child | Server repair export adapter, client replica/store, public HUD accessors/predicates, targeted protocol/replica tests, and browser fixture/driver | IO1 client/export, IO2 |

Root owns shared `orders/{mod,apply,writer}.rs`, `plugin.rs`, transaction files, API/runtime modules, and audit consumers.
Children publish required hooks to root instead of editing those files independently.
Root remains an implementer. Implement shared transactions and destruction while the children build their assigned capabilities.

## First checkpoint: contracts, then parallel code

Root freezes API 0.2.14 and protocol 8 using the exact spec DTOs and accessor names.
Add `ResolvedUnit.repairer: Option<RepairerDef>` with qualified, validated references.
Publish RepairRecord/RepairState and snapshot/delta fields in the existing transport-free records module.
Add PrimaryField repair accessors, RepairStateKind, and Predicate::RepairStateIs.
Update API exports, version detection, protocol handshake expectations, and constructor defaults together.
Keep research ChargeOverTime refused. Remove only implemented refusals when their complete paths exist.

Publish the internal shared contracts in this checkpoint:

- A progress-payment helper returns the exact proposed debit and charged totals, or a typed funding refusal.
- A repair update contains the validated target health gain, own-slot Balance debits, and proposed clock/remainder state.
- Both helpers stage mutations through Transaction. Their commit hooks cannot fail after validation.
- Refund helpers distinguish manual all-or-none cancellation from mandatory destruction with explicit discarded amounts.
- Repair exposes order start/replace/stop, active-action, and removal hooks, plus a read-only authoritative status projection.
- The shared order state carries the persistent Stop latch, omitted when false in old audit streams.

Publish small WIP commits immediately so dependent owners can compile against real types.
These commits may be untested. Label their status clearly; they are not milestone approval.
Do not wait for a whole lane or a full workspace build before integrating common contracts.
If a helper changes, update its one declaration and notify dependent owners before they duplicate it.

## Paid work

Implement exact cumulative-floor differences using the existing pure charge rules and global RateQ.
Keep source endpoints from queue/site admission, including constructor inventories after the constructor disappears.
Stage every cost, progress step, site health gain, and final activation together.
Preserve the shared physical source pass, queue FIFO, and symmetric source-tie refusal.
Keep normal geometric output blocking distinct from funding failure and operating admission overflow.
Handle zero-cost differences, absent endpoints, pause/resume, and the one-second display-hold recovery streak.
Publish per-tick WaitingForFunds independently from the display hold flag.

Implement site cancellation with None target and constructor Stop clearing all held requests.
Root dispatches grouped site cancellations by coordinates before calling the per-site transaction.
Refuse exact ties as Blocked; preserve per-unit results and atomic refund/removal for each site.
Never use a shifting held-queue index or cancel placed sites when stopping their constructor.

## Shared removal and accounting

Root handles producer, research, site, and constructor cleanup in the existing common removal transaction.
Retain actual-paid stage refunds and release reservations without duplicating credits.
Manual cancellation preflights every refund destination and fails without mutation if any cannot receive its refund.
Mandatory destruction credits representable balance refunds, handles inventory batches as specified, and records every discarded remainder.
Full or missing refund destinations cannot undo lethal damage.
Account for source inventory destruction without fabricating new item drops.
Coalesce charge/refund events and retain raw charged buffers needed to explain every intermediate debit.
Extend the independent audit checker to derive these changes from definitions and before/after state.
Do not weaken existing conservation or claim checks to accept new events.
Root must run independent audit evidence for intermediate paid debits (PW2–PW3),
cancel/death credits and explicit discards (PW5–PW6), and fractional repair debits/healing (RP2–RP3).
Each group includes coherent state/event tampering that the auditor must reject.
Record those streams, commands, and exits under IO3. Operating-resource replay limits do not satisfy these new checks.

## Repair

Use the existing order queue, movement goals, spatial queries, relation filters, and surface reach geometry.
Keep explicit movement/order priority and the persistent Stop latch separate from automatic target state.
An accepted queued non-Stop clears the latch but its queue still suppresses automation.
Hold suppresses automation for its lifetime. Rejected orders and engine orders leave the latch unchanged.
Exact best-target ties acquire neither target. Source ties cannot spend or heal by ID order.

Advance the fractional heal clock on eligible zero-heal ticks; pause it while approaching or unable to pay.
Persist per-resource charge remainders for the repairer's lifetime, including order replacement and retargeting.
Stage only applied healing and every required own-slot Balance debit together.
Use one repair action per source per tick. Cap against the health committed by earlier physical sources.
Root uses the active-repair hook to suppress automatic weapon action and clears linked state on removal.
Keep repair independent of power and complete targets only; do not implement deferred site repair or revival.

## State, transport, and player interface

Write owner-only absolute RepairRecords and explicit clear IDs using the existing export/replica staging pattern.
Snapshots replace the complete set; source removal is represented only by removed, and target removal clears surviving sources.
Reject duplicate keys, missing endpoints, and foreign sources atomically.
Expose the exact frozen PrimaryField values and RepairStateIs predicates with mod-authored names and labels.
Change RateRecord prediction to follow actual state/rate. A Running/Slowed record with held=true must still advance.
Test postcard round trips and malformed records before the browser probe.
Assert that arithmetic/source-tie holds project Working, while real unaffordable Balance projects WaitingForFunds.
Typed audit diagnostics retain the detailed hold cause without promising an owner-visible wire reason.

The client child builds one isolated fixture from existing small assets, without changing Skirmish.
Create damage through ordinary authored gameplay, such as a hostile fixture weapon with one declared ammunition item.
Do not inject target health or balances through debug hooks.
Use real mouse actions for paid construction, stalling/recovery, cancellation, Stop, training, approach, and repair.
Read-only diagnostics can establish authoritative outcomes; screenshots must show the corresponding player state.

## Integrate and verify

Root integrates owner checkpoints early. Run focused checks that answer each current implementation question.
Use `/tmp/codex-mp1-cargo` for every shared native-cache Cargo invocation.
Coordinate one native Cargo owner at a time; share the target cache instead of creating build directories per child.
The client child owns the browser/GPU window. Every test Chrome gets a fresh profile and `--alsa-output-device=null`.
Do not run competing GPU work or heavy builds during performance measurements.

When all acceptance paths are implemented, assemble one candidate and stop changing its source during final verification.
Switch the native cache to the root integration worktree before starting the WASM release build.
Do not let a later worktree switch refresh root source timestamps during that release build.
Prepare the matching server, package, and normal browser release once.
Run the required full integration checks once for that candidate, not once per child or acceptance row.
Run IO2 on that build and the three scripted cold-play probes. Verify the actual launcher.
Use one independent code-running foundation review of the whole feature. Fix its findings directly without task reviews or a fresh review loop.
Repeat a check only when a relevant fix invalidates its earlier result.

Root writes one feature report with evidence paths for PW1–PW6, RP1–RP4, and IO1–IO3.
State deferred work, unverified limits, API/protocol changes, and every spec correction.
Update README progress. Merge the tested milestone to main only after acceptance and the required review pass.
Do not push main. Preserve the accepted launcher and `target/mc-evidence` throughout integration.
