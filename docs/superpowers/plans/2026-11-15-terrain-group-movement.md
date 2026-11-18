# M4b implementation plan

The spec received one focused review. Its findings are incorporated at44c02543.
The same reviewer approved this plan after one focused recheck atf5f94d0f.
The total was one spec review, one plan review and one focused blocker recheck.
M4 implementation and acceptance continue separately.
Do not change or rebuild the accepted launcher for this planning work.

## Common contract: root

Isolated source preparation can start from the independently reviewed M4 candidate.
Merge the accepted M4 source before M4b acceptance and integration.
During M4 timing windows, these lanes can write source but cannot build or run competing tests.
Keep API17/protocol10 unless implementation reveals a missing public contract.
Ordinary group order IDs remain opaque correlation handles. They cannot select a physical winner.
Keep the existing exact geometry, permission basis and physical arrival rules.

Publish a shared route-key and lifetime contract before dependent lanes diverge.
Key guidance by map, viewer, permitted geometry basis, exact traversal profile, exact radius and destination region.
Keep each actor's actual destination and order epoch separate from shared guidance.
Do not round a radius down to obtain cache reuse.
Reuse a route only after checking its required region leases.
Retire references on completion, Replace, Stop, death, owner change and match reset.

First implement one shared reverse-search tree for each compatible destination region.
Use direct flat certificates before starting a search.
The shared tree guides actors toward that region. Individual final approach and arrival remain authoritative.
The terminal region must be one connected free-space component for the exact profile and radius.
Different components inside one destination chunk need different terminal keys.
Each actor needs a certified connector from a tree terminal to its actual endpoint.
If that connector fails, retain the endpoint and start an individual search or another certified terminal field.
An intermediate waypoint can never complete or replace the stored player destination.
Use graph vertices at cell centers, edge midpoints and corners, with exact footprint admission and swept edges.
Keep valid zero-width tangency connections. This represents the required integer-centerline path through the exactly two-unit opening.
If a required legal connector cannot use this graph, add bounded geometric refinement and retain a pending request until certification.
The legal fixture completion bounds still apply. Safe refusal alone cannot pass.
Fields must contain only permitted current, remembered or public initial geometry.
Exact physical certification still controls the committed segment.
Do not share a field across viewers merely because its current bytes happen to match.

Initial explicit limits are32 resident fields and64MiB aggregate guidance/frontier storage.
Count allocated capacities and references, including incomplete searches.
The search scheduler admits at most128 cell expansions and1,024 edge-certification attempts per tick globally.
Reserve service for already pending fields before admitting a flood of replacements.
Use accepted command age and physical request geometry for service ties, never member IDs.
Exact unresolved ties share service or remain explicitly contested.
These are work bounds, not capacity promises. Measure their actual cost before claiming a result.
Limit admission and invalidation bookkeeping to4,096 entries per tick each, including rejected and cancelled requests.
Publish affected geometry epochs immediately; stale fields cannot guide a committed segment while deferred invalidation waits.
Actor cancellation stops old physical intent immediately. Reclaim its field references through a bounded retirement queue.
Reclaim at most16 storage pages of64KiB per tick. Retired pages still count against the64MiB aggregate cap.
Limit local neighbor visits to65,536 per tick and128 candidates per actor query.
Query overflow keeps the affected actor stationary and its full body present as an obstacle.
It cannot permit motion with omitted collision bodies. Remaining actors receive service through the same physical fairness policy.
Record every overflow and pending age. The frozen feasible cases must still complete within their time bounds.

## Parallel implementation

One lane owns shared terrain routing and local invalidation in new navigation modules.
It implements permitted field construction, membership, admission, cancellation and bounded retention.
It keeps public initial geometry separate from observed edits and private execution checks.

A second lane owns local steering and arrival integration.
It keeps the existing collision solver's physical guarantees and tests each complete swept footprint.
Use spatial neighbor queries. Dense-query overflow must stop safely rather than omit collision bodies.
Track progress at a choke entrance from actual physical arrival.
Choose corridor direction from the first physical entrance arrival and drain a bounded physical batch before yielding direction.
The initial batch limit is four entrants. Entrants already inside finish before the direction changes.
Use entrance arrival tick, sub-tick physical progress and world coordinates for ties.
Do not reserve or evict settled destination space for that priority.
An exact simultaneous entrance conflict uses a documented geometric tie rule; physically indistinguishable starts stay contested.
Waiting actors can use the authored passing bays. They cannot cross another body to reach one.

A third lane owns public fixtures, HUD statuses, actual browser probes and measurement drivers.
It uses ordinary selection, point orders and work orders.
It records waiting, searching, blocked and completed outcomes from public state.
It does not inject movement or terrain through a debug command.

Root owns common interfaces, early integration, replay/audit changes and acceptance.
Use separate worktrees and explicit file ownership. Use the existing shared Cargo target and wrapper.
Only one native build lease or CPU/GPU measurement window runs at a time.

## Frozen fixture contract

Before the first timing run, commit the authored JSON, commands and hashes below.
Fixture construction is validated before it starts a simulation.
Do not move obstructions or reduce actors after a failed timing run.

Use a64×48 world, unit-width terrain cells, quarter-unit height steps and a ridge atX[30,34), across the map height.
The ridge is four elevation steps above the approach plateaus.
Wide passage:Y[20,28). Narrow passage:Y[23,25).
Small radius:0.4. Large radius:1.1. Use the public movement profiles and existing credited models.
Both primary cohorts use max_step0 and max_grade0/1. The four-step ridge is therefore impassable outside the opening.

GM1 starts24 small units in a6×4 grid with1.5-unit spacing, origin(10,17).
It starts12 large units in a4×3 grid with2.5-unit spacing, origin(8,30).
One ordinary group command targets(50,24). Both kinds must finish in120 simulated seconds through the wide passage.
GM2 uses the narrow passage. Small units finish; large units wait.
An ordinary work order then opens the wide passage. The retained large-unit order must resume and finish within120 seconds after the opening.

GM3 uses two opposing cohorts, each six small and three large units.
The single-file passage is three cells wide, with empty6×8 passing bays outside both mouths.
Place the bays atX[24,30) andX[34,40), both atY[20,28).
Require an opposing pair of large bodies to meet this passage. They cannot pass abreast.
The frozen layout must include a concrete feasible sequence: one cohort waits in its bay while the other crosses, then it crosses.
Require both cohorts to finish within120 seconds. Stable waiting cannot pass this feasible case.
The impossible variant parks an arrived radius1.4 body at the passage center. Neither side has enough remaining clearance for a small body.
It must produce stable safe waiting without displacing that body. Record the geometric clearance proof with the fixture.

GM4 closes and reopens a passage while units approach it.
Freeze edit times before observing their outcomes. Use finite authored resources and ordinary fill/excavation commands.
Include a distant active route whose required chunks do not intersect the edit.
Keep its route revision and membership unchanged.

Additional geometry fixtures require a legal one-step rise at grade1/4, then refusals at max_step0 and grade249/1000.
A radius1 body must pass an exactly two-unit opening at tangency. The next representable larger radius must refuse.
Include a diagonal cliff corner and a high-speed segment crossing a one-cell wall.
Legal movements must finish. Every illegal segment must stop at its last certified point.

For secrecy, keep public rules and initial geometry equal and vary only hidden edits.
One actor contacts one local obstacle. A remote hidden difference must still leave route metadata, status and the other viewer's cache equal.
Repeat with connection-order reversal, allocation reversal, cancelled requests, owner changes and full admission pressure.

## Verification and delivery

Run focused geometry, route and replay tests during implementation.
Run the complete native compatibility campaign with no-fail-fast once on the assembled feature.
Fix actual failures and rerun affected checks. Do not add per-task review passes.
One independent foundation reviewer runs the complete assembled change and checks the physical and permission contracts.

GM7 reuses the frozen M4 workload without changing its coordinates, actors, tools, routes, ACK delays, warm-up or1,000 measured intervals.
Keep all service/debt, actual movement, work and client activation requirements.
Record field reuse, expansions, edge checks, neighbors, pending age, local invalidation and peak allocated memory beside the existing raw samples.
Run on the same recorded four CPUs with no competing build, profiler or browser.

Prepare one final browser candidate after native correctness and timing pass.
Run real GM1-GM4 pointer scenarios, the unchanged400 detailed models at all three camera distances, and the final cold-play probes.
Keep High quality, assets, camera bounds, and at least30FPS.
Record every GM1-GM8 row and any unverified limit in one report.
Update README progress, then merge and push under the standing authorization.
Continue to geological discovery and the scripting milestone afterward.

## Exact traffic-index implementation amendment (2026-11-16)

Replace only the traffic query index and its temporary neighbor storage.
Do not change route selection, collision certificates, fairness, quotas, deadlines, or frozen workload.
Keep the physical-row cap returned by the existing controller unchanged.
First record its actual value and locked-toolchain sizes; use that fixed cap in the new representation.

For each body, index its inclusive envelope of radius `4*r*1.000001`.
Opposing discovery uses actor radius `4*r*1.000001` and the f64 `4*(r+peer_r)*1.000001` circle predicate.
Speed does not inflate this discovery radius. Longer candidate movement is queried by its actual chord AABB with actor footprint, then independently certified by final physics.
Service passing and bay queries by oldest actual attempt, then physical geometry. Store one result inline per admitted member and apply it in the existing physical gate order.
A lazy chord query requires existing owner body credit and includes its own actor disc. The initial4,096 discoveries plus32,768 paid lazy queries give at most36,864 metadata services.
Inflate each envelope radius further by `1+2^-18`, add `256*f64::from(f32::from_bits(1))`, then round each f64 bound outward.
This also covers the larger nearest-rounded f32 version: its at-most12 positive primitive roundings have `gamma12 <2^-20` relative error.
The additive margin covers subnormal errors, including multiplication by4. Validated authored inputs cannot overflow that f32 reference.
The old f64 predicate has smaller rounding error. Envelope inflation never replaces the exact final filter.
Use the actual production f64 predicate as the independent extracted-set oracle.
Also test envelope coverage of the f32 reference, including `r=1`, `peer_r=3*2^-24`, zero step and its representable boundary separation.
Build four ascending endpoint arrays: minimum x, maximum x, minimum y, maximum y.
Rank queries select `min_x <= query_max_x`, `max_x >= query_min_x`, and the equivalent y predicates.
Equal endpoints are included. Sort ties may use handles for storage only; rank predicates include every equal value.
Nonfinite actor queries fail closed before extracting candidates.

Store a body-index bitset before each64 sorted endpoints, including the empty prefix.
For a rank, load its whole-block prefix and insert at most63 remaining endpoint bits into a temporary bitset.
Intersect the four masks; complement strict-lower prefixes for the maximum-endpoint predicates.
Unused bit positions remain zero. Count only exact envelope intersections as reported-body visits.
Retain the old circle filter after that charge. A relevant query exceeding128 candidates or its owner quota discards its entire result.
Run queries in the existing oldest-attempt/physical service order. Sort completed results physically before any steering.
No bit position, endpoint tie, partial extraction, or handle chooses which actor receives service.

Replace the temporary neighbor BTreeMap/Vec collection with one exact-capacity row vector and per-physical-row ranges.
Its row capacity is `65,536/2`; the other half stays reserved for the final body guard.
This removes per-query allocator growth without changing the service envelope.
Let `N <= M`, `W=ceil(M/64)`, `U=sizeof(MovementFrameUnit)`, `E=sizeof((f64,usize))`, and `P=sizeof(Option<(usize,usize)>)`.
The new concurrent temporary bound is:

```
N * (sizeof((u32,&MovementFrameUnit)) + 2*sizeof(usize) + U + P + 4*E)
+ 4 * (floor(N/64)+1) * W * sizeof(u64)       # prefix snapshots
+ (65,536/2) * sizeof(usize)                 # flat neighbor rows
+ 4096*128 + 4*4096*sizeof(usize) + 4096*sizeof(u32)
+ 2*W*sizeof(u64) + 4096                     # query masks and fixed bookkeeping
+ SLOT_COUNT*W*sizeof(u64)                   # owner masks before extraction
+ M*sizeof(bool) + sizeof(index wrapper) + sizeof(flat row store)
```

Add retained members/gates to this bound. Assert the peak remains below8MiB for every admitted `N`, with unchanged `M`.
Endpoint sorting uses in-place unstable sorting; its array storage is already included. Record comparison counts and bounded sort-stack allowance.
The locked64-bit allocation check records unchanged `M=12,870`, `U=168`, `E=16`, `P=24`, and `W=202`.
Actual maximum peak: retained2,244,928 + guidance/live/order vectors2,574,000 + fixed688,710 + index2,136,280 + neighbor rows571,072 =8,214,990 bytes.
Retained traffic includes a4KiB root-node reserve; the member payload bound preserves the512-byte per-key BTree allowance.
The existing8MiB limit is8,388,608 bytes. No physical-row cap or quota was reduced.
Evidence: `target/mc-evidence/terrain-group-movement/review/body-guard/lazy-traffic-oracles.log`.
This proves allocation fit; GM7 timing and complete movement acceptance remain unverified.

Track actual build comparisons/prefix writes and query rank comparisons/tail entries/word reads-writes separately.
No metadata counter exhaustion can stop or favor an admitted query. Test the public per-query bounds from the spec amendment.
The old grid-bucket counter does not certify this representation.

Differential tests compare the complete candidate set with an independent exhaustive envelope oracle.
Cover equal endpoints, tangency, boundary floats, mixed radii/speeds, seeded scenes, ID/registration reversal, and dense relevant overflow.
Pair protected x=10 inputs with a distant radius50 body at x=1000, including enough remote bodies to pressure the old index.
Require equal completed protected decisions, reported-body visits and service state under the unchanged quota.
Check exact allocated-capacity accounting and peak buffers at the existing public cap.
Run affected traffic/guard/navigation checks, then the unchanged complete GM scenarios and GM7 measurement.

## Disclosed-body local routing repair

Capture the immutable Start visibility basis beside the existing terrain phase.
For explicit AllVisible only, borrow the fixed pre-movement public poses and content-defined registry radii; do not allocate a second body snapshot. Require the current Start terrain phase. Restricted maps with missing or mismatched captures fail closed.
Build the local obstacle input from own bodies and currently permitted foreign pose/content facts.
Enumerate foreign facts from the pinned capture, never from current live IDs. A legal construction cancellation can despawn a visible site between Start and movement. Keep its captured fact until that observation phase expires. A supplier failure or row-cap refusal invalidates the entire viewer obstacle set.
Apply permission filtering before neighbor extraction and its work charges. Keep all cooperative intent logic owner-only.
Reuse the bounded local detour helper and retained-route lifecycle. Revalidate against the current permitted obstacle facts before reusing a route.
Do not use a live foreign pose as a substitute for its captured pose. Keep the final authoritative physical frame separate from planning facts.
Count any added masks, temporary rows, and retained state against the unchanged allocation bounds.
Run a real controller return around the visible wall and paired hidden-world tests, then the unchanged OC7 preflight and GM acceptance scenarios.
Cover hiding a contributor after a route is retained and changing its live pose while the captured Start facts stay fixed. Neither case may reuse private or historical-only input.

## Friendly swept-path support

Keep baseline physical resolution and service order unchanged. Separate geometric position selection from the final movement/progress update so semantic state is applied once after both passes. Do not clone nested command/progress buffers into guard scratch.
Retain the baseline certified positions. Freeze actual baseline arrivals, resting bodies, stationary holders, completed rows, and unadmitted rows. Use separate root and support masks. A held-short raw arrival can retry only at its own root in the unchanged physical order. Dependency admission must use the strict in-flight support mask.
Build one complete index over the union of raw and baseline trajectory envelopes, dropping the baseline index first. Include raw envelopes for every eligible root, including raw arrivals, and every eligible support member. All final endpoints remain one of these two certified choices.
The shared broad phase can contain raw envelopes, but foreign/outsider hits must be replaced by their current certified envelopes before candidate charges, extraction or exact support checks. Raw false positives receive only nonexhaustible metadata service; they cannot select recipients or consume body quota.
Use an iterative work stack and staged flags, bounded by the unchanged public physical-row cap. A raw path blocked by a same-owner certified path can ask that eligible peer to join the set. Unsafe raw pairs reject the attempt; safe cycles require complete mutual certification.
For staged peers use their full raw paths. For every outsider use its current certified baseline or already upgraded path. Never inspect a foreign raw proposal. This preserves an arriving outsider's baseline path even if it originally lay between friendly members in service order.
On success publish every staged position atomically. On failure clear only temporary staging; keep certified outputs and consumed work. Later attempts cannot roll back an earlier certified path.
Keep explicit per-actor pair charges across both passes and all attempts. Admit every support query with existing owner credit, including empty queries, and reject incomplete candidate extraction. Use complete 128-candidate queries and the existing 65,536 total budgets.
Account position arrays, flags, counters, stack, ranges and simultaneous buffers within 5 MiB. Record actual support queries, pairs, failures and accepted members beside existing index work.
Test packed parallel/diagonal motion, safe cycles, genuine crossing rejection, arriving-outsider priority, foreign-future independence, local rollback, quota exhaustion, index rebuilds, and ID/enumeration reversal. Add an independently retried held-short raw arrival and a later-root check against its upgraded envelope. Preserve the arriving-outsider counterexample. A no-upgrade case must retain the old complete movement/progress result. Then run unchanged GM scenarios and OC7.

## Preferred-velocity replacement

Spec gate 1 used one focused reviewer. Both material findings are incorporated: all neighbors initially receive full avoidance responsibility, and whole attempts share the existing optional guidance pool. No reciprocal assumption or new independent allowance is permitted. Plan gate 2 uses the same reviewer before implementation.

### Shared private interfaces and ownership

Root publishes the interfaces below before parallel edits. No public API or protocol changes.

- Raw-motion owner (Damage): `movement/frame.rs`, `movement/plain.rs`, new `movement/motion.rs` and exports in `movement/mod.rs`. Preserve existing entry points as no-hook wrappers. Add an internal kernel variant accepting a motion-guide object. `MotionRequest` contains unit ID for lookup only, immutable starting pose, selected effective goal, authored speed, radius, world bounds and dt. `MotionGuide::next_position(request, &mut SweepBudget) -> Option<Vec2>` returns a raw next pose. `None` means this motion is outside the new scope. Eligible model refusals return the bounded preferred step, never `None` followed by legacy projection.
- View/admission owner (Visibility): new `terrain/traffic/velocity.rs`, history in `traffic.rs`, `navigation.rs` wiring, work/diagnostic fields in `sweep/bounded.rs` and navigation metrics. Implement the guide, permission boundary, attempt tickets, history and accounting. Keep these edits separate from raw-motion files and the pure solver.
- Root: new `terrain/traffic/velocity_solver.rs`, pure numerical tests and dependency-free solver. Publish `Agent { position:DVec2, velocity:DVec2, radius:f64 }` and `solve(actor, neighbors:&[Agent], preferred:DVec2, max_speed:f64, dt:f64, horizon:f64, charge:&mut dyn FnMut()->bool) -> Option<DVec2>`. All neighbors have full responsibility on the actor. `None` is a model refusal, not permission to omit certification.

### Raw-motion seam

Invoke the guide after destination/claim selection but before straight movement, soft separation or projection. Cover ordinary plain motion, retained plain progress and group motion. For a returned pose, bypass both legacy steering stages and run the existing arrival/progress logic using that pose. Preserve existing blocked/recovery/waiting early exits. The guide bypasses actual short-destination arrivals; private waypoints may use it even within one step. Do not reinterpret waypoint arrival as public point ownership. Defensively reject nonfinite/out-of-speed output and enforce world bounds. The exact body guard and existing terrain retry loop still run afterward. Do not update semantic progress or accepted history from tentative retries.

### Frozen view, history and admission

Build compact per-owner spatial views AFTER the old traffic indexes are dropped. Each view combines own current rows and the already pinned CURRENT foreign visitor; failure or overflow disables modeling for that owner only. Use compact spatial grids, not several retained large envelope indexes. Prove capacities before allocation, including history, selected tickets, rows, grid storage and solver scratch. Keep the same 8 MiB traffic and 5 MiB guard limits. The default preferred-step fallback needs only own pose/selected goal and cannot read missing foreign truth.

Own history stores owner, accepted pose/velocity and tick. Validate consecutive tick/pose/owner before reuse; otherwise velocity is zero. Publish history only from the final accepted outputs after terrain reconciliation. Never update it while generating another actor's proposal. Freeze view, neighbors and ticket counters through retries.

Compute remaining optional credit from `SweepBudget` AFTER existing detour charges. Reserve whole 128-operation tickets in oldest-attempt/physical-state order, with no ID tie break; omit an indistinguishable boundary group rather than select one member by handle. Store actual attempted service so distinct eligible actors rotate across ticks. Charge each query/candidate/constraint/half-plane evaluation against its ticket and `charge_guidance_pair`, recording model work separately from physical pair checks. Do not reset a ticket on retry. Reserve enough discovery credit before querying; incomplete discovery falls back. Release only unused reservations to final certification, not to another model attempt.

Select at most eight modeled neighbors by geometric distance and physical state from the permitted view. Start with a local reach bounded by footprint clearance and at most two capped movement steps; the model need not cover every possible collision because the complete final guard does. Use horizon `2*dt`, never less than dt. Own stationary/terminal peers have zero modeled velocity; other own peers use accepted velocity. All foreign modeled velocities are zero. All responsibility factors are one.

### Numerical solver and validation

Use fixed arrays for at most eight body half-planes and their bounded projection subproblems. Use f64 for proposals; retain exact rational certification. Meter repeated linear-program checks and stop on the caller's refusal. No RNG, IDs, unbounded loops or library simulator. Infeasible, degenerate or nonfinite solves decline safely. To avoid a perfectly collinear stand-off, allow a bounded secondary preferred-direction bias only when the primary solution makes little goalward progress: clockwise then counterclockwise at one tenth of preferred speed. Both attempts share the same ticket. Never bias unconstrained movement. The adapter must still reject a candidate that lacks permitted terrain/known-static clearance before returning it; otherwise use the preferred-step fallback and final exact guards.

Raw owner tests the wall-direction seam and unadmitted fallback; view owner tests privacy, history invalidation, stopped peers, work/memory exhaustion and rotating useful admission; root tests parallel flow, head-on/crossing constraints, degeneracy, numerical bounds and metered refusal. Run focused checks per changed boundary. Integrate once and run the real warmed OC7 tape immediately; no more local corner-offset families. Then run required GM cases and final capacity/browser/gate campaigns. One independent reviewer runs the completed foundation change. Preserve every failed real workload and state plainly if liveness or performance remains unverified.


## Fast group coherence replacement

This section replaces the fixed 128-ticket-only admission and accepted-velocity model policy. Anvil spec gate 1 used the existing reviewer once. No blocker was found. The plan incorporates all four accounting/fallback/membership clarifications. Gate 2 used the same reviewer once and passed. Implementation boundary checks include31/32 coherence credit,63/64 predictive credit, retained-capacity accounting and repeated retries without refunded spent work.

### Frozen aggregate and storage

Root owns new `terrain/traffic/coherence.rs`: bounded Member records, canonical complete-group accumulation and equal-weight heading blend. The key partitions by owner and original order-group identity, then sorts by position, radius, speed, effective goal and preferred-step vector. Handles never break ties. Identical physical rows have identical contributions. A valid group needs at least two contributors and squared mean length greater than 1e-12. Blend only for normalized dot strictly greater than 0.5; keep the actor's preferred displacement length. Test permutations, renamed group/actor handles, cancelling means, threshold boundaries and unrelated groups.

The view owner keeps `velocity.rs` and navigation capture wiring. Capture original group membership from the original frame, before temporary progress is stripped. Eligible contributors are moving, not waiting/resting/cancelled/recovering, and not actual fractional arrivals. Private short waypoints may contribute. Build at most 4,096 Member records; oversized owners decline as a whole. Before aggregation, each owner must afford all of its contributing rows in both existing counters. Precharge one operation and one visit per row in canonical physical order, also charging that actor's cumulative steering counter. If the batch cannot finish, publish no owner mean. This prevents handle-ordered partial groups.

Build a static-only index of u16 offsets into each already frozen View (at most12,870 rows/view), in existing spatial order. No body copies or second large envelope index. Charge its queries/candidates and exact static tests. Include actual Vec capacities, headers, Member scratch, Actor cached advice/counters and static indices in both the compile-time bound and maximum-allocation test. Member scratch is dropped before predictive tickets; account the maximum of those phases, not a hidden simultaneous allocation. Keep 8 MiB traffic and 5 MiB guard limits.

### Useful service before prediction

The controller owner exposes an explicit guided-controller composition; legacy helper wrappers retain their old behavior. Reserve8,192 pair credits and8,192 neighbor visits per owner from the existing optional half-pool before routing. Forward the remaining routing limit explicitly through the existing query paths. Do not use environment variables for shipping policy. Optional own-neighbor query refusal leaves the incoming permitted candidate unchanged and skips only old stages that need complete neighbor rows. Real gate, route, capacity and permission waits remain waits. Tests include the isolated spotter and a real gate hold.

After routing/index retirement, the view owner computes all coherence advice in oldest-service then physical order. Require32 available operations to start, then spend actual work up to the actor's cumulative128 cap. Complete static-only and permitted-terrain proof is mandatory for a changed chord. Release unspent reservation to the next different actor in this frozen pass. Store only completed advice with its effective-target bits. Failed coherence retains straight fallback. No actor receives fresh credit on retry. Aggregate contribution cost counts against the actor cap but resets neither service age. Keep separate coherence and predictive attempt ages; each changes only after that stage actually spends work. Exclude known short actual arrivals from predictive reservation, while retaining their metadata and private-waypoint eligibility. Test prediction rotation when every actor receives coherence each tick, zero-spend/reservation-only clocks, and many short actual arrivals beside one far transit.

Then admit predictive solves from remaining owner credit. Reserve the actor's entire remaining128-cap allowance only when at least64 operations remain. Coherence spending counts toward that cap. Failed prediction retains validated coherence for the unchanged target, else straight. Freeze own commanded-velocity predictions independently of generated advice; foreign velocity is zero. Actual arrival/closest-holder paths still bypass steering. `mark_waypoint` preserves the original destination and spent counters, but discards stale coherence advice. Exact final body/terrain checks and semantic commit remain unchanged.

### Integration and acceptance

Root publishes the Member/blend interface before parallel edits. Root owns pure coherence code/tests; the view owner owns aggregate integration, static index, scheduling/history and allocation tests; the controller owner owns guided routing limits/fallback and focused controller regressions. No per-task reviewers. One central build runs affected movement, privacy, work, memory and replay checks, then the unchanged actual warmed OC7 tape immediately. Run the required native, browser, capacity and final gates after functionality passes. The existing independent reviewer runs the completed foundation change once; no acceptance or main merge while any required row is red.

## Deferred disclosed-route intent repair

One existing reviewer approved the clarified intent/certificate boundary at spec gate1. The same reviewer approved this short plan at gate2. Both gates used one existing reviewer each. No new agent or full review loop.

Root owns the order reset hook in `navigation/shared.rs`. Accepted order changes must call `TrafficState::forget(unit)` before any terrain/navigation early return, including same-coordinate Replace. This keeps the already documented order fence: equal XY does not make a retired route current. Add no route epoch buffer or new quota.

The controller owner owns `observed_detour.rs` and its existing tests. Keep a copy of the entry route only after its current generating witnesses and actor/request/shape identity validate. On optional query or pair-service exhaustion, retain that entry cursor and select its existing corner as private intent. Do not preserve a new continuation, advance a cursor from partial work, claim a fresh planning certificate, refund service, or weaken either final guard. Restore intent on narrow/broad query refusal and saved-leg proof exhaustion. New routes still need their existing complete proof. An already complete negative saved-leg result wins over later exhaustion; missing/changed witness, changed order/owner/shape, completed route and complete clear direct goal discard/release exactly as specified. An exhausted proof alone is not a negative result.

Use the existing Retained record and corner; no new dynamic buffer or historical body row. Deferred intent is not a certified-route cache hit. Coherence/predictive changes still validate the actual proposed chord against current static bodies and permitted terrain. The unchanged exact body and terrain reconciliation remains the only actual movement authority.

Tests seed a real retained route, then exercise zero neighbor credit, zero pair credit and partial-proof exhaustion. Compare the retained cursor/corner and selected private target; charged work never decreases. Include negative-before-exhaustion, changed/missing current witness, same-XY replacement through the real order-change hook, direct-goal release, and handle/enumeration reversal. Run these focused cases once from a pinned combined artifact. Then root runs the unchanged actual warmed tape immediately. The existing reviewer checks the completed narrow repair and runs its focused tests; whole-feature acceptance remains pending all native/browser/capacity rows.

## Spatially local coherence refinement

Spec gate1 and plan gate2 each used the same existing reviewer once and passed. This replaces only the global whole-order mean; quotas, private-destination semantics, Current proofs, stage clocks and final guards stay unchanged.

Root owns `coherence.rs`. Partition complete contributors by owner, original group and exact positive radius. For each subgroup use minXY of its complete valid positions as origin and cell width4r. Compute cell coordinates in f64, but convert only exact-range floor values0..(2^52-2) into checked i64 keys. If any subgroup coordinate is unrepresentable, that subgroup supplies no coherence. Never saturate. Query neighbor offsets with checked_add; no aliases or duplicate contributors.

Reuse Member storage for cell coordinates, cell heading sum and valid-contributor count. Sort by owner/group/radius/cell/complete physical key. Each cell aggregate is complete and summed in physical order. Store its sum/count in the existing bounded Member rows; no separate cell array or index allocation. A representative found by binary search identifies one complete cell. Count actual contributors, including self once, not occupied cells.

Expose prepare_cells(members) and local_mean(members, member_index, charge_cell). local_mean visits the fixed3x3 neighborhood in canonical offset order. Charge every distinct query, including empty cells, before reading an aggregate. Return no partial mean on refusal; require at least2 contributors and the existing mean threshold. Checked keys make all9 addresses distinct; any arithmetic failure conservatively refuses the mean.

The view owner keeps the Member Vec alive through the existing oldest-coherence/physical service pass. Before discovery enforce the32-operation start rule. Charge cell queries to the same owner operation+visit counters and actor cumulative128 counter; aggregate contribution still resets neither service clock. Store a u16 Member lookup in Actor and remove its old global mean field. Changed advice keeps the existing static-only/terrain proof and target-bound cache. Drop the Member allocation before predictive ticket allocation; count changed sizeof/capacities in both static and maximum-allocation proof. Keep predictive64/128 rules and separate ages unchanged.

Root tests opposite separated flanks in one order, neighboring cells across a boundary, radius/owner/group isolation, complete minima, extreme/unrepresentable coordinates, empty-cell charges, exhausted prefixes, and handle/group/enumeration reversal. The view owner adapts only fixtures that must be physically local to reach the intended proof and checks fair service/cumulative credit/memory. Use one combined focused native artifact, then root runs the unchanged warmed tape immediately. Existing final foundation reviewer checks this completed narrow refinement and runs the new cases; no broad repeated reviews or acceptance claims before the actual campaigns.

## Swept predictive discovery correction

The existing reviewer approved the narrow spec. Root supplied the same pinned trace to both model and controller owners; the latter proved current support rejects genuine crossing paths, so support and route rules remain unchanged.

The view owner changes only velocity.rs. Call View.query from request.position to the bounded fallback endpoint (validated coherence when identical-target, otherwise capped straight). Keep the existing radius padding. Replace the origin-radius candidate filter with finite point-to-segment squared distance within that padding. A zero-length segment uses its origin. Rank eligible candidates by existing squared start distance then complete physical state; keep at most eight. Preserve frozen first-attempt neighbors on retries without adding credit, the actual128 steering cap, owner operation/visit limits and conservative incomplete-query refusal. No new index or allocation.

Tests reproduce a current body4.99m ahead of a5m step, a nearby off-segment exclusion, short/zero-length geometry, query exhaustion without partial discovery, enumeration/handle reversal, and unchanged complete body/terrain guards. Include the new contributor-weighting helper test9b8fe69e in the necessary combined native artifact. Then root runs the same default warmed OC7 tape on the pinned release executable. Other congestion mechanisms and full acceptance remain open unless actually passed. The existing reviewer checks the completed changed boundary and runs its code; no new review panel or repeated closed checks.

## Bounded infeasible-model refinement

The existing reviewer approved the spec and required an explicit geometric-infeasibility result, distinct from arithmetic or work refusal. Root owns velocity_solver.rs. Keep the existing strict solve wrapper and add solve_relaxed(actor,neighbors,hard_mask,preferred,max_speed,dt,horizon,charge). hard_mask is a u8 over the same selected eight neighbors. Partition them stably into hard then soft lines without new selection or IDs. The view owner owns velocity.rs: retain captured stationary/foreign flags through top-eight insertion in a parallel fixed bool array/Ticket field; build the mask and call the new function for the original and bias attempts. Hardness must not follow zero velocity alone. Frozen neighbors, capacities, work counters and retry semantics remain intact.

Root factors the scalar program to return complete or a failed-line index plus the last feasible prefix result. Geometric empty intervals or a negative speed-circle discriminant indicate infeasibility. Work refusal and nonfinite construction/intermediates return a separate error and decline immediately. A failure in the hard prefix declines. Only geometric failure in the soft suffix may enter bounded projected refinement. Use fixed eight-line scratch: copy the hard prefix; for each violated soft line construct the canonical RVO2-style projections against prior soft lines, then solve the resulting two-dimensional directional program inside the speed circle and hard prefix. Charge line construction, each violation/projection/nested constraint test and final hard/speed validation. Any work refusal or unresolved/projected program failure declines the entire new advice; do not copy the reference's restore-and-continue arithmetic fallback. No partial refinement survives refusal. Preserve complete current static/terrain proof and final exact body/terrain certification.

Root tests the saved actual contradictory inputs and an incompatible-speed-circle case, finite bounded compromise and hard-line preservation, all charged refusal prefixes, parallel/opposed/degenerate constraints and unchanged strict feasible behavior. The view owner tests captured hardness against own moving-zero, stopped-own and CURRENT foreign rows, classification through top-eight insertion/reversal, cumulative retry accounting and maximum allocation. Use one combined focused artifact; the same reviewer runs the new cases. Root builds and pins one server artifact and runs the unchanged default warmed tape immediately. No model forecast, admission, route or support policy change is part of this refinement.

## Work-conserving primary prediction

The existing reviewer approved the clarified spec. Its material finding was tied-batch visit starvation; the spec now protects both resources before any multi-actor tied batch starts. One primary query/proof must abort at the first refused charge. Thus its actual visits cannot exceed charged operations plus one. Test that bound and exact boundary refusal, without refunding work or aging reservation-only rows.

Root owns new velocity/prediction.rs. Extract the existing primary program into Guide::predict_primary(actor_index, request, &mut Ticket, budget) -> Option<Vec2>. The helper reads the same frozen views, actor state and captured clear callback. It returns Some only after a completed validated primary proposal, None on every refusal. It performs no ticket admission, no cache mutation and no peer forecast update. Keep the current LP2/LP3, selected hardness, complete discovery/proof and bounded diagnostics. Remove references to self.tickets/actor.ticket from the helper; its mutable Ticket is reusable fixed scratch supplied by the caller.

The view owner owns parent velocity.rs, scheduling, state and adapter tests. Add mod prediction. Keep Ticket as a private scratch type and spend as the shared metering helper. Keep compact per-Actor primary proposal and actual predictive-service flag; update total spent after each primary. Guide captures dt/bounds so primary/coherence cache checks also cover speed/radius/timestep/bounds. After Member scratch is dropped and optional diagnostic forecasts are frozen, sort Actors by lookup ID and rebuild prediction-service indices in the existing service Vec. Sort those indices by oldest predictive age and physical state. Construct Guide, then process one batch at a time against its unchanged views.

For a singleton require the same whole remaining allowance with at least64 operations. For a multi-actor physical tie, preflight the sum of all whole remaining allowances against owner operations AND sum(remaining+1) against owner remaining visits. Refuse the entire batch before prediction if either fails. Reserve the whole admitted operation batch. Initialize a local Ticket for each actor with its existing spent count and own remaining reserved allowance; run predict_primary, retain only the completed result and actual spent/service status. Hold unused allowance in one batch accumulator until every batch member completes; then release it once before the next batch. No other prediction interleaves within a batch. No full-neighbor Ticket Vec remains in Guide.

MotionGuide::next_position keeps actual-short/closest-holder handling first. It returns cached primary advice only for the identical captured pose/goal/kind/speed/radius/dt/bounds. Otherwise it uses same-identity completed coherence or capped straight fallback. Raw calls never execute LP/discovery/proof or change steering spend. mark_waypoint discards both proposals without changing accepted destination or spent credit. finish_prediction has no retained reservation to release. Record prediction age only for actual charged primary work.

Tests preserve the original semantic assertions while moving timing expectations to eager capture: unused credit serves more actors under the same owner cap, exact tie operation/visit boundaries, every query/proof failure stops before another overshoot, repeated identical raw requests spend nothing, changed-goal/kind/radius/speed/dt/bounds do not reuse a proposal or acquire credit, short actual arrivals still bypass guidance, same-target coherence survives refused prediction, service clocks rotate independently, and normalized reversal/privacy/max-allocation checks pass. Keep all production limits unchanged. One combined native artifact runs the changed adapter and retained kernels. The same reviewer runs the completed boundary. Root immediately runs the unchanged default warmed tape; more consumed optional work cannot excuse a guard overflow or reduced movement acceptance.

## Coherent permitted movement plans

The existing reviewer approved the spec and required explicit arrival, actor accounting and phase-memory interfaces. This is a diagnostic-first owner-local planner; no global truth-guard output enters it.

Root owns movement/motion.rs, movement/sweep.rs and support.rs. Add defaulted private MotionGuide hooks for certification query admission, certification pair work, and permission for tangent/prefix search. Defaults call the existing body/pair budget methods and preserve the real guard exactly. The planning guide owns one shared per-seed steering counter initialized from that actor's existing spend. BOTH hooks debit that counter and the same owner operation pool, capped at128. The query hook also admits one guidance visit before any index read. It preflights exhausted actor/operation credit so a refused query cannot repeatedly spend visits after steering ends. Exact-pair hooks retain the core local certificate allowance as an additional bound. Support dependencies call these same hooks; there is no separate or reset counter. Planning disables tangent/prefix search and reports the seed totals afterwards. The existing exact initial-overlap recovery predicate is exposed crate-locally for upgrade validation. Return conservative scratch bounds for a planning frame before allocation, including retained pool-slot structures. Do not change real arrival ordering, collision predicates, final guard allowances or default search.

The movement owner implements new terrain/traffic/velocity/joint.rs. Its private Seed rows describe owned captured motion, kind, original destination and preferred/coherence step. Synthetic foreign rows come ONLY from the complete captured View and are stationary. Owned actual-short/closest-holder arrival and waits retain their real raw semantics and necessary owned pool data; far private transit can use a bounded preferred guide. Do not clone a global frame containing hidden foreigners. Count copied units, selected owned pool slots, core scratch, retained views/actors/history and eventual plan/index storage before allocation. Refuse the owner before allocation if the combined8 MiB or physical5 MiB bound cannot fit.

Build the baseline with the existing exact movement kernel and cheap planning guide. Its temporary SweepBudget maps query visits onto the owner's existing guidance visits. Every admitted query admission/extracted-row examination and every exact pair check ALSO consumes one operation through the common planning-guide charge bridge, from the remaining optional half and that initiating actor's cumulative steering counter. Candidate discovery, retries and support dependencies all share this bridge. No geometry query/predicate runs after a refused admission. The original physical certificate-local pair allowance is an additional limit, not a replacement for steering128. Protect the final physical halves. Merge only actual spent deltas and metadata; never refund failed work. Update actor totals from the planning guide. Conservative stationary fallbacks are complete protected chords; invalid/incomplete captured input, allocation refusal or failed known-terrain validation publishes no baseline or forecast. Check every resulting owned chord with the captured permitted-terrain oracle before publication. All temporary raw frames/guard indexes drop before the retained upgrade index is allocated.

JointPlan stores compact permitted bodies and a single EnvelopeIndex. Its fixed envelopes cover every speed-bounded legal upgrade; reject candidates outside that protected envelope. Queries filter to actual current planned chords before exact pair work. try_upgrade certifies a candidate against EVERY relevant owned planned chord and CURRENT stationary foreign body, charging visits and the same actor/owner operation closure. Refusal never changes the plan. Success replaces only that actor's endpoint. Exact overlap recovery semantics match the existing kernel. Include empty query charges,128 candidate refusal, complete checking and all capacity headers. Do not use handles for priority; handles index captured facts only.

The view owner owns parent velocity.rs and prediction.rs integration. After coherence and Actor ID lookup sorting, optional diagnostic mode constructs one owner plan at a time, applies its cumulative spent/visit deltas, then freezes View own predicted velocities from completed baseline chords. Foreign velocities stay zero. Keep only valid whole plans. Primary model preference still targets the captured goal/coherence, not the shortened baseline. A candidate must pass the existing static/terrain proof and JointPlan exact upgrade proof. A refused primary keeps that owner's certified baseline chord under the full cache signature. Changed-goal/kind callbacks invalidate all advice and keep the original bounded fallback without new work. Multi-actor indistinguishable batches conservatively keep the baseline rather than commit one upgrade by ID. All final authoritative certification remains unchanged.

Use a test-probes-only switch while assembling the experiment. Defaults must reproduce the prior actual trace. Tests cover convoy support, crossing rejection, baseline fallback on invalid/over-budget input, known-only foreign projection, waits and actual-short/closest-holder behavior, exact safe upgrades and refusal rollback, cumulative actor/owner/visit counters including query-only exhaustion and support dependencies, scratch phase peaks, order/handle reversal and the recorded stopped-peer follower case. The same reviewer runs the assembled changed boundary. Then root runs the unchanged warmed tape with the diagnostic enabled. No promotion or capacity claim without passing actual workloads.


### Cheap joint baseline staging

Spec gate: one existing foundation reviewer, no blocker. The three required details are included below.
Implementation owner: the core movement agent in its existing isolated worktree.
Root owns the actual tape and integration. Reuse the same reviewer for this plan and final changed-boundary code check.

1. Add a private guide hook for direct in-flight support, false by default.
   Enable it only through an explicit test-probes switch in the permitted planning guide.
   Derive a separate immutable deferred-eligibility array from original frame rows and untouched raw outputs.
   Reuse the complete support predicate, actual semantic destination, finite speed-bounded raw endpoint and explicit physical-escape exclusion.
   Preserve original and raw resting/terminal exclusions. Do not infer eligibility from held output positions.
   Count the extra flat array and simultaneous scratch in the unchanged5MiB/8MiB bounds.
2. For deferred rows, skip initial original-disc admission and ordinary movement certification.
   Keep their ordinary admission range absent. They have permission to attempt support, not a paid certificate.
   Set their baseline position to the unchanged original pose and update the commit mask.
   Keep raw coordinates and raw semantic metadata separately intact.
   Every actual short arrival, recovery and other nondeferred row uses the existing ordinary path in its existing physical order.
   It certifies against original/deferred held bodies and earlier certified outputs.
3. Extend support with the explicit deferred-eligibility array, separate from ordinary admitted facts.
   Deferred candidates may attempt their own root and lend support only under the immutable full eligibility predicate.
   Other rows retain existing root/support rules, including the limited clipped-arrival root retry.
   Fixed ordinary arrivals and resting/recovery bodies remain protected by existing rules.
   Every dependency query, extracted row and pair proof uses the same cumulative guide/owner/actor counters.
   Preserve the complete128-peer union, atomic rollback, whole-owner terrain validation and no partial-plan publication.
   Default guides keep their existing path. No reservation setting becomes shipping policy.
4. Add a meaningful old-path red: a dense legal convoy with enough remaining actor credit for one support proof but not duplicate ordinary/support discovery.
   Expose only the inert private mode hook before running that old-path test; do not accept a compile failure as its red.
   Then verify full motion on the repaired path under the identical cap.
   Test short destination versus private corner, short-arrival ownership against held bodies, explicit recovery, original/raw Resting exclusion, refusal at staged dependencies without partial commit, crossing safety, reversed handles/enumeration, query/row/pair exhaustion, default-path equivalence and maximum phase memory.
   Build once per needed source correction, pin the executable and run focused movement/adapter checks.
   Root then runs the unchanged actual warmed diagnostic with reservations zero and the new switch on.
   Full native capacity, browser and milestone acceptance remain open until that succeeds.
