# M2 implementation plan review record

Date: 2026-10-26. Codex orchestrator. User approved design revision 2 before planning.
Artifact: `docs/superpowers/plans/2026-10-26-m2-economy.md`.

## Scope and gate selection

Gate 2 checks executable task order, exact producer/consumer interfaces, ownership, and
acceptance instruments against the approved economy-only design. It does not reopen the
approved payment/refund policies or select a scripting runtime.

Four standard lenses: spec-rigour-reviewer (rigour), mod-runtime-engineer (public API),
producer (sequencing), mod-builder (consumer). Simulation-core-engineer and network-engineer
join for accounting and wire/resync changes. Six seats run in batches within four concurrent
session slots. QA's source/instrument preflight precedes the plan; it is not a seventh gate vote.
No new performance budget or capacity promise is part of this plan gate.

## QA preflight

Seat: qa-lead. Scope: current code and test/CI seams, not a verdict on an unfinished plan.

`VERDICT=pass-with-changes FINDINGS=7 HIGH=5 RUNS=3 NOT_CHECKED=2`

High requirements sent to the planner:

1. Explicitly invoke each new ignored economy test target from the acceptance script/CI.
    Check that named tests exist and that nonzero tests ran: an absent exact filter on an
    existing M1 test executable returned success with zero tests.
2. Build a real headless command-trace driver before process scenarios need it. Current
    client options support only one automatic MoveTo. Economy steps need shared sequences,
    CommandApplied JobIds, tick/state predicates and complete result reports.
3. E6 needs authoritative initial/operation/tick state snapshots, including node stock,
    inventories, balances, reservation lines, queues, entity ids and allocator state.
    Expected accounting must be derived independently of event totals; the scratch probe
    showed an internally balanced log hiding an incorrect node change.
4. Preserve M1 tests through fixture migration. Existing process tests assume one unit,
    hard-coded positions/speed and an empty second slot; mirrored economy maps invalidate
    those assumptions. Use explicit movement/empty-slot fixtures while retaining each oracle.
5. Browser tests must run base/haul/acme through real controls and visible text with positive
    progress and produced meshes. The current six-second capture cannot prove the 15-second
    harvester recipe. Use state/tick deadlines with wall time only as a watchdog.

Other requirements: update API incompatibility mutations (`^0.1` to `^0.2` is currently the
negative edit), preserve mutation-hit guards and diagnostics, add browser-accessible semantic
loss/delay/buffer instrumentation, and schedule rebuilt-product red/restored-green evidence
for named plants. Native-only loss flags do not test browser recovery.

Runs: source inventory found no M2 instruments; nonexistent exact test filter returned zero
executed tests with exit 0; independent accounting probe exposed node/log disagreement.
No M2 runtime/browser behavior was run, and the unfinished plan was not reviewed.
Reviewer changed no repository files. Scratch: `/tmp/turts-m2-plan/qa/`.

## Gate 2

### Initial review interruptions

The initial network-engineer, simulation-core-engineer, and producer review turns each
ended at the usage limit before a final report. Their observations are preliminary and are
not final votes. The initial producer note found no High sequencing/scope issue and one low
staging-path-count mismatch; the latter was an editorial note, not a gate verdict.

### Focused review votes received

| Seat | Result | Findings | Checks | Scope |
|---|---|---|---|---|
| mod-builder | `VERDICT=pass FINDINGS=0 HIGH=0 RUNS=2 NOT_CHECKED=1` | None | 2 runs | Public path, variants, actual controls; no runtime behavior claimed |
| mod-runtime-engineer | `VERDICT=pass-with-changes FINDINGS=1 HIGH=0 RUNS=2 NOT_CHECKED=3` | One Low: Task 1 file-count mismatch, resolved by removing numeric staging count | 2 runs | Loader/archive/API/visual separation; consumer docs and public Cedar presence |
| producer | `VERDICT=pass FINDINGS=0 HIGH=0 RUNS=1 NOT_CHECKED=2` | None; confirmed 16 owners, five steps per task, acyclic order; numeric path counts removed | 1 run | Plan structure and sequencing |
| simulation-core-engineer | `VERDICT=pass FINDINGS=0 HIGH=0 RUNS=2 NOT_CHECKED=2` | None; ordering, atomicity, retained capabilities, independently bounded oracle; modeled wrong recipe, wrong cancel, missing rejection boundary, hidden write | 2 runs | Simulation contracts and accounting instruments |
| network-engineer | `VERDICT=pass-with-changes FINDINGS=1 HIGH=0 RUNS=2 NOT_CHECKED=1` | One Medium timing phrase implied a later driver update; resolved to same driver update after arm | 2 runs | Receipt witnesses, mid-trace loss arming, transport recovery |
| spec-rigour-reviewer | `VERDICT=pass-with-changes FINDINGS=1 HIGH=0 RUNS=2 NOT_CHECKED=3` | One Medium same-update vs next-update arm timing contradiction; resolved to same driver update | 2 runs | Internal consistency, status and testable claims |

### Repair status and gate outcome

The three instrument contracts identified in the interrupted reviews have been repaired in
plan revision 2: the accounting oracle receives independently expected command/result data
and boundary audits; native/browser loss hooks support mid-trace arm/reset; and transport
ordering uses raw ReliableChannel `u16` MessageId witnesses with a send-reversal plant.
All six focused review seats have now reported. Their findings were repaired in revision 2,
including the same-driver-update timing clarification raised by network and rigour. Gate 2
**PASSED after repairs**, with no unresolved findings. The plan awaits the user's review before
implementation. Product implementation and M2 runtime results are not claimed by this
documentation checkpoint.
