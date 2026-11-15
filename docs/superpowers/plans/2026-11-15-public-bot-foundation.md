# Public bot foundation implementation plan

Status: approved by the same Anvil reviewer. Both final clarifications are incorporated.
Contract: `../specs/2026-11-15-public-bot-foundation.md`.
Budget: the existing reviewer, one plan turn. No new panel or task-review loops.

## Current integration

The pure core passed 39 tests before the final three seed/rollback regressions were added.
The assembled workspace passed a non-test check at `d846fc6c`.
Root owns participant tracing, integration checks, browser acceptance, and delivery.
The execution owner now owns `actors.rs`, accounting, and real scheduled receipt tests.
The host owner owns public bot scenarios, replay, and the complete-phase benchmark.
The projection owner owns shared human permission projection and focused tests.
Next: run all assembled native tests, fix failures, measure the phase, then run browser probes and the one foundation review.
Main remains at the accepted SDK14 milestone until all public-bot acceptance checks pass.

## Delivery and ownership

Root owns public contracts, shared actor admission, authoritative receipts, and permission decisions.
Three children implement independent modules after root publishes those contracts.
Each child publishes a small coherent checkpoint early. A checkpoint can be explicitly uncompiled.
Integrate checkpoints before waiting for each child to complete its full test set.
Root also implements its shared paths and runs integration acceptance.

| Owner | Owned paths and result |
| --- | --- |
| Root | `turts_mod_api/src/bots.rs`, schema/content/lib exports and API15; server `session.rs`, `outbound.rs`, common actor admission/receipts, shared `permissions.rs`, export permission calls, command provenance/audit glue. Public types and one atomic receipt/basis contract come first. |
| Child 1 | New `turts_bot` core: public traits, registry, queries, budgets, seeds, coordinator, commitment reconciliation, and external-controller tests. Exclude `src/controllers/**`. Root reviews shared type additions before they diverge. |
| Child 2 | `turts_bot/src/controllers/**`; runtime bot parsing/validation and its archive/version/reference hooks; bot profiles and the Balance/inventory/renamed public fixtures. Own native profile/controller behavior tests. |
| Child 3 | Server `bots/**`, `computer.rs`, CLI/config/main bot wiring, observation adapter, public Skirmish migration, launcher glue, and server/native bot scenario tests. Root's permission and actor modules remain single-owner dependencies. |

Root owns final benchmark/browser drivers, independent foundation review, README, and acceptance report.
Child 2 owns `mods/skirmish/manifest.ron` and its bot definition. Child 3 consumes its published profile identifier.
Root resolves other manifest/Cargo/module registration edits once, instead of concurrent broad edits.
No child changes an active accepted build or starts a private build cache.

## 1. Freeze and publish the public data contracts

`BotDef` has `id`, `name`, `decision_ms`, `start_when`, `knowledge`, and `controllers`.
`knowledge` has named roles, explicit order/recipe bindings, opening goals, recovery goals, production targets, and squad rules.
A role names a content set and required capabilities. Recipe and order bindings identify their actor roles.
Goals and controller instances have stable unique authored string keys. Reject duplicate keys before registration.
A controller entry has `key`, qualified `factory`, `config: BotValue`, and explicit `quota`.
`BotValue` is a bounded portable value: null, bool, integer, string, list, or duplicate-rejecting string map.
It is configuration/state data, not executable Rust or an unrestricted RON value.
Bound nesting, item count, and encoded bytes through operator limits, with field-path errors.
Portable validation checks structure and content references. Host registry activation checks factory configuration and availability.
A client can load a package that names a custom server factory it does not implement.
Stock factories live under the versioned engine namespace `engine:*`; other factory names retain their qualified package identity.
Importing a library registers no actor. The game/operator selects the profile explicitly.

Public framework interfaces:
- `BotController::step(&mut self, frame: &BotFrame, queries: &mut dyn BotQueries, out: &mut dyn ProposalSink)`.
- `ControllerFactory::{validate, create}` and `ControllerRegistry::{register, activate}`. Duplicate registrations fail.
- `BotCoordinator::{reconcile, select}` over portable proposals/results. The host cannot silently bypass it for stock controllers.
- `BotQueries::query(Query, continuation, limit)` returns an immutable page, coverage status, budget use, and continuation.
- `ProposalSink::propose(Proposal)` either records the proposal or returns a typed budget/validation refusal.

`BotFrame` identifies the basis, completed tick, scheduler tick, active ticks, slot, tick rate, match phase, and remaining budgets.
It also carries the complete bounded reconciliation metadata for that account basis.
`Proposal` identifies controller instance, goal/task, actors, source anchor, priority, authored goal rank, order, and required account rows.
Public IDs are opaque engine identifiers. Public positions use the existing transport-free coordinate types.
No framework type contains Bevy entities, a simulation world, raw map data, transport connections, or private closures.

Derive sub-seeds with SHA-256 over a versioned domain, seed, profile, slot, instance key, and optional goal key.
Use fixed little-endian integers and length-prefixed UTF-8 fields. Take the first eight digest bytes as a little-endian seed.
Reject duplicate instance/goal keys. Reordering registration or configuration lists cannot change these seeds.

## 2. Close actor receipts and commitment accounting first

Use `SpendKey = (SourceEndpoint, qualified resource)`.
`SourceEndpoint` distinguishes `SlotBalance(slot)` and `Inventory(unit, Main/Output)`; forbid foreign account queries/allocations.
Use `TaskKey = (controller-instance key, goal key, task-instance key)`.
Use `RequestKey = (bot actor, checked command sequence, affected unit)` for each group's member.
A coordinator entry stores these keys, predicted costs, work identity, and its observation/result watermark.
Derive/validate the source anchor and costs from the permitted actor facts and resolved order/recipe, not a controller's unverified estimate.
It remains provisional until its exact effect is represented by an authoritative basis.

Root adds internal per-unit `ActorResult`/effect witnesses to ordinary dispatch.
Do not infer group work from the wire result's single optional `job_id`.
Witnesses identify the request, final refusal or acceptance, exact endpoint changes/reservation, and job/site/terminal work identity.
The completed basis declares which finalized request effects it includes. A result tick alone is insufficient association.
Retain bounded receipts until reconciliation acknowledges them; pause does not drop unresolved receipts.
Account basis advancement and its complete reconciliation batch are one immutable publication.
If that batch cannot be complete, keep the previous accounting basis and submit no new spending decisions.
Do not combine a new post-debit account with an old provisional charge.

At one basis, start with `on_hand - authoritative_reserved` for each account.
Subtract provisional commitments whose effects are not in that basis.
ChargeOnQueue commitments disappear only when their exact debit/refusal is included.
ReserveThenCharge commitments transfer to the observed reservation, then debit, without another local subtraction.
COT commitments become declared cost minus paid-at-that-basis, keyed to the exact accepted job/site.
Terminal refunds/removal/refusal release commitments only when their exact terminal effect is included.
Per-unit refusals release only those group members. Missing feedback never releases the others.
Test two producer inventories holding the same item, plus mixed Balance/inventory costs.
Enumerate result-before-basis, basis-before-result, delayed results, partial groups, cancellation, destruction, and paused COT work.
A constructor's removal must not release a commitment for its surviving autonomous site.

Refactor human and bot admission around one actor-neutral validated request path.
Network sessions construct Human actors internally. The bot host constructs Bot actors internally.
No player message accepts a caller-supplied actor kind or another slot.
Keep sequence, size, rate, ownership, catalog, prerequisites, affordability, outcome, and domain validation behavior.
Use one deterministic admitted-order position per tick. Source/actor provenance must not create a new visible ID priority.
Route all per-unit results back to the correct coordinator and preserve existing human wire results.

## 3. Build observation pages and permission-safe advice

Root adds a common permission interface used by human export and bot projection.
Its production provider preserves today's all-visible units/nodes. Its restricted test provider models unobserved coverage.
The bot adapter receives only this provider's permitted records, own account/work records, and public definitions.
It does not call authoritative `construction::check_placement(World)` for advice.
Use public pure placement/affordability rules over observed facts.
Return `Unknown/NotObserved` when coverage or an account is unavailable. The answer depends on coverage, not hidden occupancy.
For direct lookup, hidden and absent both return NotObserved without a death flag.
Authoritative admission checks the real world and returns only the permitted generic refusal.
Test two worlds differing only in a hidden blocker/account through spatial, direct-ID, placement, and affordability queries.
Compare rows, budget use, examined counts, coverage, and continuation values byte-for-byte.
Only permitted candidates count toward public query metadata. Separately charge private projection scans to host limits and phase timings.

Index each immutable basis once. Count examination work, including filtered-out candidates and page preparation.
Page in physical `(x,y,content)` order. Exact physical ties carry an unresolved-rank marker, not a selectable prefix.
An opaque cursor can address storage members within a tie; that order must never rank a proposal or a chosen actor.
Controller candidate state spans pages. Selection waits until every potentially better/equal candidate is resolved.
On budget exhaustion, retain the selection. On lease expiry, discard that selection explicitly and restart without choosing its prefix.
Retain at most one candidate basis per bot and expire it after eight scheduled decision steps or a pause.
Reconciliation/account bases advance only through the complete publication rule above.
Own aggregates and pending-work identities are complete bounded metadata, not totals inferred from truncated pages.

## 4. Supply complete basic controllers and profiles

Provide four stock factories: opening, economy/recovery, production targets, and squad attack/defend.
They use explicit role/order/recipe bindings and coordinator proposals.
A recovery goal outranks opening work through authored priority. It can restore gathering and a declared supply target.
Count completed units and matching pending jobs/sites before proposing replacements.
Use owned paid/reserved work state for local obligations; do not reserve money in the gameplay ledger privately.
Simple squad rules select permitted targets, assemble declared roles, and attack/defend through ordinary orders.
Do not add tactical search, arbitrary script execution, or an unreviewed universal game inference layer.

Migrate Skirmish constants into one public profile and remove the private `ComputerPlayer` strategy.
Preserve its paid mixed army, building, tower, economy recovery, and normal victory/defeat path.
Make a separate inventory/storage package with distinct constructor/producer actors and explicit receiving inventory rules.
Run both packages and a fully renamed fixture through the stock host. No private names or profile-specific server branch is allowed.
An external trusted Rust test implements a different controller and replaces the stock strategy through the registry.

## 5. Integrate lifecycle, replay boundaries, and operational limits

Add explicit `(slot, profile, seed)` configuration and the unambiguous legacy CLI alias.
Validate the whole install batch before changing claims. Keep one controller/coordinator group per configured bot slot.
MatchRunning and HumanPresent are per-tick guards. HumanPresent pauses while no human is connected.
Accumulate active ticks only while the guard holds. Opening timers and decision periods use active ticks.
A pause retains requests, commitments, controller memory, and results. Drop expired candidate leases, not pending work.
Resume with a fresh permitted/reconciled basis. Emit at most one due decision; do not replay missed decision intervals.
Never reset bot state on a human reconnect or take over a human-owned slot.

Use the existing typed command/audit log, not `orderlog.rs`'s CSV state-transition log as a replay input.
Add optional actor/source provenance and a deterministic per-tick admission ordinal. Old actor-free records retain their legacy interpretation.
Record all generated submission attempts and their results in the trusted log so rejected attempts can also be compared.
Record human-presence changes and match-lifecycle inputs before guards for that tick. Replay applies them before controller scheduling.
Test a human disconnect/pause/reconnect while COT requests are pending, comparing active timers, commitments, commands, and state.
`RecordedCommands` replay restores the locked initial state/claims and disables live bot submissions before advancing any tick.
It consumes recorded actor commands once in their logged order, through the shared validated actor path.
`RegenerateAndCompare` feeds only recorded non-bot inputs and enables live bot generation with the recorded profiles/seeds.
It never injects recorded Bot commands. Compare `(tick, actor, seq, task, canonical envelope, result, admission ordinal)` exactly.
Compare server state at every completed tick. A mismatch reports the first differing entry and tick.
These are headless acceptance modes. A replay UI, distribution service, and game-save runtime remain deferred.

Default per-bot decision limits: 32 queries, 4,096 examined candidates, 1,024 returned rows, 32 proposals, eight commands, and 256 affected units.
Bound pending requests at 64 and pending account rows at 512. New work that exceeds them waits with a typed diagnostic.
Page size is at most 64 rows. Query continuations and reconciliation receipts count toward declared bounded state.
Profiles divide controller quotas explicitly; sums must fit the host limits. Host configuration can lower or raise these technical budgets.
Use separate two-slot runs for bot/bot and bot/human starvation checks.
Stock quota split (queries/candidates/rows/proposals): opening 4/512/128/4; economy 12/1536/384/12; production 8/1024/256/8; squads 8/1024/256/8.
Command/unit limits apply to the coordinator's complete ranked selection, not registration-order emission.
Stop before any overflowing counter. Diagnostics are transition-based and bounded, not one message per failed polling call.

Benchmark two bots with 1,000 units total: each owns 100 gatherers, 300 mobile combatants, 50 producers, 25 constructors, and 25 static providers.
Use mixed Balance/inventory production, active collection, pending COT work, and attack targets through the public fixtures.
Run 20 Hz with one-second decisions and four recorded logical CPU cores. Collect 1,000 active phases on accelerated ticks.
Measure query/index preparation, controller/coordinator work, reconciliation, and actor admission, including deferred work.
Compare p99 with the spec's 5 ms budget. Record a baseline without bots; do not subtract required preparation out of the bot result.

## 6. Integrate and accept once

Root assembles early API/receipt/permission checkpoints before native compilation of dependent modules.
Use `~/.codex/bin/turts-cargo` and one shared native cache. Assign its ownership explicitly.
Run focused core/runtime/host tests as dependencies become available. Fix concrete failures directly.
Keep final benchmark, GPU, and browser work serialized.
Switch the shared native cache to the integration checkout before starting the WASM release build.
After that, keep native and WASM source ownership on that checkout until final acceptance finishes.

Root records BF1–BF9 in one report, with paths and exact limits.
Use one independent foundation review that runs completed code, then fix its findings.
Run the full required gates and scripted cold-play probes once on the candidate.
Repeat only checks invalidated by a concrete fix. Update README and retain the accepted launcher until acceptance passes.
A tested milestone may merge under the standing authorization. Do not push main.
