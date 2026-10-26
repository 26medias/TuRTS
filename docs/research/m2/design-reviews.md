# M2 design review record

Date: 2026-10-26. Codex orchestrator. Product code is not changed by this review.

## Acceptance research audit (before gate 1)

Seat: qa-lead. Source: `docs/research/m2/scope-producer.md`.

`VERDICT=pass-with-changes FINDINGS=8 HIGH=5 RUNS=3 NOT_CHECKED=2`

High findings to resolve in the design:

1. E1's package-derived common words match valid generic code (`field` in
    `crates/turts_sim/src/map.rs`). Use classified references and explicit common-word handling;
    keep a nonempty token-set guard and deliberately forbidden references.
2. E4 must compare separate boundaries: A is unchanged after B's failed attempt, reduced by
    an explicit transfer to B, then unchanged while B spends its own inventory.
3. E6 must include initial stock, deposit depletion, cargo, storage, consumption, conversions,
    and declared sinks. Reservations are not extra stock. Inspect authoritative state as well
    as event logs.
4. Successful production needs an output oracle: entity count, type, owner, timing, existing
    client visibility, and late-join visibility. M1 position deltas do not create entities.
5. E2 equality and conservation alone permit a no-op economy. Require positive extraction,
    delivery, payment, completion, and output before comparing traces.

Other findings: exercise capacity, filters, access, rollback, and reservation release; compare
claim/rejection effects at controlled simulation boundaries; inspect visible browser labels,
not debug hooks alone. Production policy remains an explicit design proposal under spec 15.

Checks: package-token scan found generic identifier hits; a throwaway arithmetic/equality
probe demonstrated the E4/E6 contradictions and empty-trace false positive; code search traced
the position-only wire format and unknown-unit resync. No M2 runtime was tested because no M2
product code exists yet. No repository files were changed by the reviewer.

## Gate 1

Artifact: `docs/superpowers/specs/2026-10-26-m2-economy-design.md`, first frozen draft.
Lenses: rigour, public mod API, architecture, third-party mod consumer, simulation accounting,
and networking. Six seats run in batches within the session's four concurrent slots. QA's
audit above is preparation, not a gate vote. Reviewer models: `gpt-6.1-sol`, `high` effort.

| Seat | First verdict | High findings | Status |
| --- | --- | --- | --- |
| spec-rigour-reviewer | `VERDICT=pass-with-changes FINDINGS=4 HIGH=0 RUNS=6 NOT_CHECKED=2` | None | Wording and geometry repaired; orchestrator checked |
| mod-runtime-engineer | `VERDICT=pass-with-changes FINDINGS=1 HIGH=0 RUNS=4 NOT_CHECKED=2` | None | Re-review pass |
| network-engineer | `VERDICT=pass-with-changes FINDINGS=2 HIGH=0 RUNS=3 NOT_CHECKED=2` | None | Re-review pass |
| mod-builder | `VERDICT=fail FINDINGS=1 HIGH=1 RUNS=4 NOT_CHECKED=3` | Standalone package authoring incomplete | Repaired; re-review pass |
| simulation-core-engineer | `VERDICT=fail FINDINGS=2 HIGH=2 RUNS=2 NOT_CHECKED=2` | Self drop-off can stall gathering; payment contention oracle rejects valid immediate charging | Repaired; re-review pass |
| technical-director | `VERDICT=pass FINDINGS=0 HIGH=0 RUNS=5 NOT_CHECKED=2` | None | Architecture boundaries confirmed |

Consumer High: E10 promises authoring from the document alone, but the draft omits concrete
manifest and inherited geometry/identifier/visual serialization forms. Logical sample mods
could be described but not serialized without guesses. Repair with a complete API 0.2 package
example and checked qualified-string references; do not choose a script runtime or a final
packaging standard to close this gap.

Simulation High findings: the valid Gatherer+Store composition can choose its own inventory
as the nearest receiver, then reject every self-transfer despite an available depot. Exclude
identical Store source/destination inventories and test the composition. Also split E6's
last-stock oracle by payment policy: ReserveThenCharge yields one reservation, while
ChargeOnQueue yields one debit and zero reservations. Both accept exactly one enqueue.

Other first-pass corrections: valid mixed-resource costs may share one endpoint (reject only
duplicate endpoint/resource lines); haul's mirrored nodes must not coincide; conversion adds
credits; last-stock assertions distinguish reservation from immediate charge; list reliable
command/result channels; coalesce buffer overflow behind an outstanding resync request and
test bounded buffering with delayed snapshots. Existing M1 test executables used by the API
reviewer were not rebuilt; their passing rejection checks are not M2 runtime evidence.

Reports and models remain in `/tmp/turts-m2/{rigour,mod-api,network,mod-builder}/` for this session.
All reviewers report no repository changes. M2 runtime/browser mutation tests remain unrun
because this gate reviews a design, not an implementation.

## Revision 2 repairs

The orchestrator corrected all first-pass findings in the design: Store selection excludes a
self-transfer; payment-contention assertions are policy-specific; references are checked
qualified strings; a complete ten-file package example and valid balance variant supply the
retained manifest/geometry/visual forms; mixed-resource costs may share endpoints; mirrored
nodes do not coincide; conversion adds credits; reliable channels are explicit; buffer overflow
coalesces behind one outstanding resync response and has a bounded-buffer E9 refutation case.
The companion uses M1's existing package/local identifier rules; no new local-id grammar is
silently selected. Consumer and simulation High findings require re-review before gate closure.

## Re-review and gate outcome

| Seat | Revision 2 verdict | Evidence |
| --- | --- | --- |
| simulation-core-engineer | `VERDICT=pass FINDINGS=0 HIGH=0 RUNS=2 NOT_CHECKED=1` | Self-Store exclusion and both payment oracles passed; removing the exclusion/swapping payment policies broke the models. Queue/refund/spawn models passed. |
| mod-builder | `VERDICT=pass FINDINGS=0 HIGH=0 RUNS=3 NOT_CHECKED=3` | Independent public-document-only checker parsed 42 files across physical/balance Cedar and renamed Birch packages; four malformed forms rejected; M1-valid local id with spaces accepted. No important authoring question remains. |
| network-engineer | `VERDICT=pass FINDINGS=0 HIGH=0 RUNS=2 NOT_CHECKED=2` | Reliable mapping present; bounded-buffer model held one outstanding request and recovered after a delayed response. Duplicate-request plant detected. |
| mod-runtime-engineer | `VERDICT=pass FINDINGS=0 HIGH=0 RUNS=3 NOT_CHECKED=1` | Typed examples parse; structured and malformed references reject; serialized schema uses ContentRef consistently. |

The orchestrator checked the rigour corrections in revision 2 (endpoint/resource uniqueness,
noncoincident 48-unit-map nodes, positive conversion income, policy-specific contention),
reran the strict authoring parser and network/queue refutation models, and checked formatting.
The original architecture pass remains applicable: repairs change no forbidden crate edges or
authority ownership. The API review's independent parser also checks the revised public boundary.

**Gate 1 passed after repairs.** All reported findings are resolved and no High finding remains.
The user approved revision 2 and its M2 scope/gameplay/payment choices on 2026-10-26;
implementation planning is now authorized. This is not M2 acceptance or runtime/browser verification. Scratch evidence is under
`/tmp/turts-m2/{simulation,network,mod-api,mod-builder,authoring-example,architecture}/`.
