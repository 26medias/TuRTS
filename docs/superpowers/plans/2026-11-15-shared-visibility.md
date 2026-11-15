# Shared visibility implementation plan

Status: Anvil spec and plan reviews approved. The same reviewer completed both turns.
Spec: `docs/superpowers/specs/2026-11-15-shared-visibility.md`.
Baseline: SDK15 candidate `a96cbaeb`. SDK15 acceptance remains independent.
Budget: the existing one reviewer performs this plan review. No extra spec-review round or per-task reviews.
Implement only after this review passes. Use three implementation children and one implementing root.

## Shared contracts first: root

Publish API16 authoring/resolved types and validation in `turts_mod_api` and `turts_mod_runtime`.
Publish protocol9 portable knowledge, event-reference, and transaction stamps in `turts_net_protocol`.
Freeze optional/default omission rules and explicit-empty/null floor checks before dependent edits.
Reuse qualified ContentId, map bounds, installed teams, match tick, and the existing public bot provider boundary.
Publish the provider's phase/tick metadata, current/remembered queries, coverage answers, and sticky retention flag.
Keep current, tactical, and completed bases explicit. Nothing receives unrestricted world access through a public query port.
Root owns package boot/reinstall staging, shared schedule registration, common fixture defaults, and declaration conflicts.
Validate grid/allocation limits before world mutation. Build initial coverage/memory before initial exports or bot decisions.
Publish small coherent WIP commits early. Integrate before each child waits for an isolated complete workspace compile.

## Lane 1: authoritative sight, memory, and actions

Own a focused `turts_rules::visibility` module and `turts_sim` visibility state/systems/tests.
Own action permission hooks in orders, acquisition, combat, gather/extract, and repair.
Use one grid rule for coverage, point lookup, boundary clipping, and source circles.
Track source/position/life changes without copying every world per viewer or keeping per-tick history.
Maintain current/explored cells and bounded stale records; oldest timestamp ties expire as a group.
Refresh start, tactical, and completed bases at the specified phase boundaries.
Keep event-time permission witnesses until that completed tick exports; store only permitted endpoint facts.
A subject appearing and dying within one tick needs a permitted transient identity before any event refers to it.
If the last source dies after an authorized shot, export that authorized observation without revealing later hidden truth.
Drop hidden live targets. Preserve only the last permitted destination for an ordinary chase fallback.
Keep movement/physical collisions authoritative; do not introduce ID-based target or memory retention priority.
Provide independent native SV2/SV3/SV5 tests, including construction, source removal, teams, and repeated same-build replay.

## Lane 2: permitted publication, queries, and bounded transport

Own `turts_server` permission/export/session/outbound integration and bot observation/query/controller adapters.
Own shared portable transaction encoding/assembly helpers and their tests; root retains message declarations/registration.
Project every public/reference-bearing field through the authoritative basis before diffing or serialization.
Build placement advice exclusively from permitted rows and public rules.
Its coverage region includes candidate footprint plus public maxima for node clearance, static gaps, and possible blocker radii.
Use own permitted anchors directly; cover any other rule-dependent anchor region before a geometry verdict.
Unknown depends only on this public region and coverage, never on whether hidden blockers exist there.
Add the paired outside-footprint/inside-clearance tests with all query metadata equality assertions.
Preserve SDK15 account basis, provisional debt, leases, fairness, and actor-local failure handling.
Expose stale queries separately. Stock controllers can move to remembered points without targeting stale IDs.

Retain one active baseline, one first-unacknowledged transaction, and the existing <=8 MiB encoded successor ring per connection.
Send successors after their preceding End without waiting for each activation ACK. Keep one incomplete client assembly.
Use one registered serde StateTransactionFrame enum so batched Begin/Chunk/End order survives typed message extraction.
Use the public registered NetId and Lightyear ToBytes framing; Transport::send_mut returns the real reliable MessageId.
Keep a bounded frame-credit window. Carry cancelled submitted credit until its actual transport ACK or disconnect.
Cancellation uses the last issued stamp as a retirement watermark. Discard every incomplete/staged ID through it in the same match/map.
Keep an already complete active view. Reject late suffix frames/activation ACKs and unsent-End ACKs.
Keep the oldest unacknowledged deadline; successor enqueue cannot reset it.
Use match/map/tick/transaction/parent stamps and indexed chunks with checked total size.
Encoded chunks are <=1,024 bytes and must fit the existing transport frame budget after its headers.
Do not enlarge Lightyear transport limits. Register a reliable ordered state channel for transaction traffic.
Bound all queued copies/encoding/staging at 64 MiB and post-base patches at 8 MiB per connection.
Use bounded incremental sending; ordinary command results must not sit behind an unlimited state queue.
Require complete assembly and activation acknowledgement before advancing that connection's baseline.
Cancel/release/restart from latest permitted state on overflow or 5-second timeout; apply the spec's restart/disconnect policy.
Release cancellable application buffers immediately; carry bounded in-flight reliable credit until ACK/disconnect.
Use the vendored WebSocket byte permits and cancellation-aware writes to bound backend retransmission queues.
Superseded stamps cannot reactivate. Ordinary large patches use the same atomic transport without routine full resync.
Do not retain raw worlds for every tick. Retain bounded permitted differences while the base transfers.
Provide SV4 paired-world, interruption/reorder/duplicate, byte-cap, slow-viewer, and healthy-viewer isolation tests.

## Lane 3: browser state activation and fog presentation

Own `turts_client` ingestion/replica, world/minimap fog, stale markers, picking, HUD, and browser probe extensions.
Consume the lane2 assembler; validate a complete stamped replacement/patch before changing any live client tables.
Keep the old complete view during staging. Apply coverage, memories, entities, health, and cleared references together.
Reject stale or conflicting transactions. Never combine unordered legacy deltas with an incomplete restricted transaction.
Clear interpolation, selection targets, health bars, minimap rows, labels, and effects on visibility loss.
Draw remembered records through a separate dim/stale presentation with last-seen information.
Render unexplored and explored-but-unobserved ground consistently in the world and minimap.
Use existing rendering/assets. Do not reduce model or texture quality.
Add native atomic-ingestion tests and the SV6 browser driver using actual pointer/key actions and screenshot checks.
Every browser has a fresh profile and `--alsa-output-device=null`.

## Integration, validation, and delivery: root

Integrate declaration, phase/provider, transport, and client checkpoints early on a non-main branch.
Assemble two public fixtures with different economy/ownership structures and renamed content. Keep Skirmish defaults unchanged.
Root owns the authoritative replay/audit additions and independent whole-feature foundation review that runs code.
Run focused native tests through `~/.codex/bin/turts-cargo`; serialize the shared native cache and avoid duplicate targets.
Before WASM release work, switch the native cache back to integration. Reserve quiet CPU/GPU windows for measurements.
Use one SV1-SV7 evidence report. Each row identifies actual tests, commands, result, and evidence path.
SV7 fixes its authored routes, unit mix, source count, transition counters, resync schedule, and warm-up before timing.
Time every tick and all deferred transport/activation work. Report server/client CPU and their sum; never subtract a baseline.
Use native client-ingestion timing for reproducible CPU accounting and separately verify the real browser's >=30 fps requirement.
Report AllVisible/concentrated-battle comparison, encoded traffic, retained memory, and the largest interrupted transaction.
After focused checks and review fixes, run the required full gates once on the assembled candidate.
Run SV6 and final `docs/qa/cold-play.md` probes before asking the CEO to play.
Update README progress and write a short handoff with verified limits and remaining M5 work.
Merge and push only the fully accepted milestone under standing authorization.

## Integration status

SDK15 is accepted on main. The current tested CI repair is pushed at `80a724d2`. SDK16 remains on its integration branch.
API16, protocol9, authoritative phase permissions, hidden-target actions, memory, client activation, and fog presentation are integrated.
Focused native results: authoring 4 PASS; rules 5 PASS; protocol assembly 7 PASS; simulation 16 PASS; client fixture/activation 7 PASS; verified-map guards 2 PASS.
Independent checks: export 4 PASS; real human wire 1 PASS; outbound 5 PASS; exact vendored socket/permit tests 4 PASS.
Current owners: publication agent fixes ordered pipelining and builds SV7; client agent fixes allocation preflight; root integrates and runs browser checks; the same foundation reviewer covers the final transport correction.
Next checks: allocation preflight, new-match isolation, ordered delayed-ACK transport, SV6 browsers, SV7 frozen measurement, final full gates and cold-play probes.
The qualified fog map identity is limited to 512 UTF-8 bytes so repeated stamps fit the fixed frame cap. Live hot package replacement is not part of this API; second installation refuses atomically, and each new match uses a fresh world.

## Frozen delayed-ACK transport check

The publication owner implements this focused SV4 regression; root runs it in the shared native target.
Run two cases with activation-ACK round trips of 50 ms and 100 ms on a 20 Hz sender.
Use an initial 1000-unit public snapshot and one patch per tick that updates 500 public unit positions.
Use deterministic finite routes within a 128x128 map. Keep the same payload, queue limits, and channels in both cases.
Warm up for 100 ticks, then measure 1200 ticks (60 simulated seconds) per case.
Batch at least two complete successor transactions into one client application update.
The client must activate every completed movement tick exactly once and in order.
After warm-up, require no routine restart, oldest unacknowledged age <=1000 ms, and encoded successor backlog <=1 MiB with no positive end-to-end growth.
These regression limits stay inside the unchanged hard 8 MiB post-base and 64 MiB total caps.
Also cancel a delayed chain before its suffix arrives. Deliver its old frames and ACKs after replacement, then prove the replacement stays active and submitted credit is not released early.
A slow viewer must still reach the existing bounded restart/disconnect policy without stopping a healthy viewer.
The same foundation reviewer inspects the pipeline and runs its focused test. Do not repeat the whole approved feature review.


## SV7 performance contract v2 implementation

Owner: root. The existing focused reviewer reviews this plan after approving the amended spec.
The spec review took one turn and one focused recheck. No new reviewer is required.

Keep the frozen authoring, 100 warm-up intervals, 1,000 measured intervals, three modes, and original subsystem timers.
Keep immutable original diagnostic artifacts and label their v1 ceiling failures explicitly.
Add direct full-simulation timers before FixedPreUpdate SimSet::Advance and after FixedPostUpdate SimSet::Collect.
These timers encompass all fixed simulation schedules, including movement, combat, and completed-world collection.
For each chronological 50 ms authored step, accumulate its full simulation duration and all server work outside those schedules.
Use the existing direct queue, pump/send, bot query, and link clocks. Add actual shared transport receive work conservatively.
Do not add the three visibility timers to full simulation time; they are already inside it.
Only cumulative-counter differences delimit consecutive steps. No baseline cost is subtracted.
Drain publication, client activation, actual transport ACKs, and application activation ACKs before closing each step.
Run scheduled same-tick resync inside that step before recording its sample. It naturally charges to its actual execution step once.
Drain/check the final step before ending the run. Record actual tick and activation counts.

Time each peer's complete consume/assembly/activation call on every application update, including empty updates.
Charge the shared transport-receive duration to both clients when per-endpoint separation is unavailable.
Also charge shared send duration conservatively to each client so activation ACK serialization cannot escape.
Retain every whole-batch duration and sum each peer's batches into that tick's client service sample.
Resync batches belong to the actual update and actual service step; do not add separate diagnostic subtotals again.
The transport throughput test independently supplies multi-transaction batching at 50/100 ms RTT.
Run performance acceptance in release builds during quiet windows. Debug runs retain semantic assertions and report timing only.
Record its whole client receive/activation batch duration too and apply the amended client bounds.

Use nearest-rank p99/p99.9 over all 20,000 measured tick service samples in each mode.
Apply server 40/50/100 ms p99/p99.9/max and chronological 50 ms debt recurrence.
Carry debt across interval boundaries. Require maximum50 ms and final0.
Apply client16.667 ms p99/33.333 ms max to both per-tick service and whole receive batches.
Record peak RSS from the Linux process high-water mark and retain existing copied-state and wire counters.
Add a small synthetic test for quantile/debt boundaries, including a spike across an interval boundary and a batched client overrun.
Keep the original v1 threshold result as a diagnostic field; a v2 pass must not turn that result true.

Commit the amended contract and driver before fresh acceptance timing.
Root runs the fixed full acceptance on CPUs0,2,4,6 with no concurrent builds or browser measurements.
Any v2 failure requires a code fix. Do not change the frozen criteria to fit the result.
Record exact source, timer boundaries, affinity, compiler, machine, flags, all samples, and any failed runs.
Run the unchanged final nine-case detailed-browser campaign separately, with fresh silent profiles.
The same independent whole-feature reviewer runs code after integration and checks timing attribution alongside privacy/atomicity.
