# M4b: group movement on editable terrain

Status: Anvil spec and plan review complete. M4 acceptance remains a separate prerequisite for integration.

## Player result

The player selects an army and gives one ordinary move order across a ridge passage.
Units use the opening without overlap, wall crossings or repeated direction changes at its entrance.
Small units can use an opening that a larger vehicle cannot use.
The larger vehicle waits with a useful status. It does not stop eligible small units.

A terrain edit can open or close the route during movement.
Affected units update their routes. Units elsewhere keep their valid routes.
The player can replace or queue orders while this happens.
Existing selection, queues, formations, rally orders and camera controls keep their current bindings.

## Scope

Complete the group movement milestone in the project specification, section14.
Use the existing public movement orders, terrain profiles, integer geometry and collision rules.
Shared routing must work for any package that uses these contracts.
Add a public authoring field only if an existing contract cannot express a required rule.
Any such change needs its version floor and ordinary package validation.

This milestone does not add terrain sight occlusion, geological discovery, new artwork, spells or a new scripting system.
Those remain later milestones. It does not claim the M7 or M12 hosting limits.

## Physical rules

The first unit to arrive keeps its occupied point.
Later units stop at the closest feasible non-overlapping position.
An arrival or clearance result must never depend on unit IDs, entity allocation or iteration order.
Destination planning cannot evict a unit that already arrived.

Routing, steering, collision and final arrival use the complete authored circular footprint.
They use the same exact plateau support and swept-traversal rules as M4.
A wide unit cannot borrow a narrow unit's clearance result.
Diagonal motion cannot cut a cliff corner. A fast step cannot cross an unchecked wall.

Units in transit can yield or take a feasible side route to resolve congestion.
They cannot overlap, teleport or displace a settled arrival to make progress.
Exact simultaneous physical conflicts need a defined geometric or contested outcome.
An ID or a hidden material field cannot break the tie.

If enough space and a valid route exist, a finite group must make bounded progress through a choke.
If no legal destination space exists, units stop safely and report waiting or blocked.
They must not oscillate indefinitely or accumulate an unbounded search queue.
Opposing traffic must either make bounded progress or expose a stable, truthful blocked state.
Do not claim that every geometrically impossible crossing can complete.
The frozen fixture must state whether a feasible crossing order exists and where units can yield.
A narrow passage with passing space at both ends is a feasible sequential crossing case.
Both opposing groups must finish that case within the declared bound.
Stable blocked acceptance is limited to a stated geometric or settled-occupancy impossibility.
Controller failure to find a feasible sequence is not evidence of impossibility.

## Shared routing and authority

Separate route guidance from local collision and arrival decisions.
Reuse route work for compatible shared destinations and traversal profiles.
Unique orders and different radii must retain their own correct results.
No algorithm is selected by this spec.

Admission, route work, local neighbor work and cache memory need explicit bounds.
A route that exceeds its current work budget remains pending. It does not become straight-line permission.
One expensive request cannot prevent other eligible movement from receiving service indefinitely.
Bounded work must include cancellation and invalidation, not only successful searches.

Planning reads the viewer's permitted current terrain, stale memory and public initial geometry.
Execution certifies actual physical segments before committing them.
Hidden edits cannot change planning metadata before authorized observation or physical contact.
Shared caches cannot carry one viewer's newly learned terrain to another viewer.
Physical contact authorizes only the contacted local geometry for that viewer.
It does not expose remote edits, buried material or another viewer's cached knowledge.
Paired worlds that differ outside the contacted region must retain equal permitted metadata afterward.

Version the regions needed by a route or shared field.
Relevant edits invalidate dependent results. Distant edits do not flush unrelated routes.
A pending or completed asynchronous result, if used, must match its map, viewer, order and geometry basis before activation.
Replacing an order retires its pending result. Appending an order retains the active order.
Stop, death, owner change and match reset must release retained memberships and work.
Allocation, member enumeration and cache admission pressure cannot turn IDs into visible service priority.
Repeat cancellation and owner-change checks under those reversals and admission pressure.

Keep server authority and same-build replay.
Reversed unit allocation and peer connection order must preserve equivalent physical trajectories and final occupancy.
Normalize opaque handles when comparing evidence. Do not normalize away different movement or arrival outcomes.

## Public demonstrations

Extend Quarry and the renamed Terrain Stockyard package through the public API.
Use both small workers and larger vehicles, with the existing models and textures.
The camera and control quality must remain unchanged.
The demonstration must include the passage before and after an ordinary excavation order.
No test-only movement command or terrain mutation can substitute for the player action.

## Acceptance

Freeze exact fixture maps, dimensions, footprints, unit coordinates, commands and time bounds before measuring them.
The following fixtures are product checks, not a new task gate framework.
Include a legal nonzero step and grade, their refused boundaries, exact-radius tangency, a diagonal cliff corner and a fast wall sweep.
The legal fixture must complete movement. Refusing every difficult movement cannot pass these checks.

| Row | Required result |
| --- | --- |
| GM1 | Both public packages load. A group of24 small units and12 large units receives one ordinary point order. All eligible units cross a sufficiently wide passage and settle without overlap or cliff penetration within120 simulated seconds. Repeat with reversed allocation and command-member enumeration. |
| GM2 | A narrow opening admits small units and refuses the large footprint. The small units finish within the same120-second bound. Large units keep safe positions and a truthful wait state. Opening a sufficiently wide passage resumes the blocked large units without a replacement player order. |
| GM3 | Two groups share a destination; two groups cross opposing routes; one unit changes to a unique destination. Cover a movable obstruction and a permanently occupied goal. Prove progress when the authored fixture permits it. Prove stable safe waiting for the impossible fixture. First physical arrival retains its point under ID reversal. |
| GM4 | Ordinary excavation/fill closes and reopens a route while units move. Check full swept clearance on every committed segment, relevant invalidation, stale-result rejection and unchanged distant route revisions. No ID-based winner or overlap is permitted during retries. |
| GM5 | Replace, Append, Stop, death, owner change and a new match release the correct route memberships. Repeated shared and unique orders have bounded retained memory. Same-seed command replay reproduces physical outcomes. Captured audit traces certify the supported transitions and reject forged movement. |
| GM6 | Paired hidden terrain worlds produce equal permitted route/query/status metadata until observation or physical contact. Exercise two viewers and connection-order reversal. A private route cache cannot become an information channel. |
| GM7 | Run the unchanged frozen M4 native workload with the completed group movement path. Keep its1,000 units,100 working tools, both visibility modes, ACK delays, sample counts and deadline criteria. Record whole service, debt, actual movement, edits, route reuse, neighbor work, invalidation, traffic and retained memory. No workload reduction or quality reduction is allowed. |
| GM8 | Fresh silent browsers use actual selection and pointer orders to pass GM1-GM4. The unchanged400 detailed models retain at least30FPS at normal and both extreme zooms during group movement and terrain work. Final scripted cold-play probes pass before a CEO play request. |

Record commands, exact fixture hashes, raw results and remaining limits in one milestone report.
One independent foundation review must run the assembled code. Fix its findings.
After acceptance, update README progress and merge/push under the CEO's standing authorization.

## Exact traffic-index metadata bound (2026-11-16 amendment)

A distant hidden body's radius must not change another actor's query service or decisions.
Grid-cell aliases do not grant permission to spend that actor's neighbor quota.
Use exact intersection of conservative actor-query and body envelopes before charging reported neighbor rows.
Keep the existing physical-row admission cap, owner-local neighbor quotas,128-candidate limit and all movement/performance acceptance targets.
Dense relevant queries still fail closed. Partial query results cannot guide movement.

Index metadata has a separate, nonexhaustible public bound. It cannot spend the neighbor quota or select query recipients.
Let `M` be the unchanged physical-row cap, `Q <=36,864` the admitted queries, and `W = ceil(M/64)`.
The initial discovery pass admits at most4,096 queries. Each lazy chord starts at its valid indexed actor and includes that actor's owner-mask bit, even for a zero-length chord.
It is admitted only with positive owner body credit and therefore charges at least one actual row. Invalid input/index and no-credit requests stop before metadata service.
The complete index rejects any invalid row; it never silently omits the actor.
Summed owner traffic credits are `SLOTS * 65,536/(2*SLOTS) =32,768`. Thus whole-index queries are bounded by `4,096+32,768=36,864`.
Four sorted endpoint arrays and prefix bitsets every64 rows report exact envelope intersections.
Building them uses four bounded `O(M log M)` sorts and at most `4*(floor(M/64)+1)*W` prefix-word writes.
Each query uses at most `4*ceil(log2(M+1))` rank comparisons,252 tail entries, and `21*W+504` word reads/writes.
These are service bounds, not new overflow conditions. Every admitted query receives this metadata service.
The reported intersecting bodies still consume the existing owner-local65,536 aggregate neighbor envelope.
The final body guard retains its existing pair/work limits and physical certification.

Record actual rank comparisons, tail entries, word operations and index-build work beside their public bounds.
Do not present the old spatial-bucket counter as proof of bitset work.
Keep index, guidance, retained traffic, and all simultaneous buffers inside the existing8MiB traffic allocation.
Use the existing aggregate64MiB navigation limit. Keep the frozen GM7 workload and deadline targets unchanged.
Test distant large-radius alias and index-pressure cases, including physical ID and enumeration reversal.
Equivalent protected envelopes must retain equal service decisions and reported-neighbor charges.

## Local routes around disclosed bodies

An ordinary Move can route around a foreign body whose current pose is permitted at the immutable Start observation phase.
Explicit AllVisible has no restricted observation basis. Its immutable pre-movement world borrow supplies the public body facts for this planning call, guarded by the current Start terrain phase. A missing or mismatched restricted basis never selects this fallback.
Use its captured position and public content-defined footprint. Do not read foreign orders, targets, progress, or current private motion to select a route.
Treat each disclosed foreign pose as a stationary local obstacle. This grants no cooperative gate priority or future-position reservation.
Hidden bodies and historical-only sightings cannot enter this local proposal search or its charged neighbor set.
Retain every foreign witness used to construct saved corner geometry. Discard or recompute the route when any generating pose or footprint changes or loses current permission. Recheck both saved legs against the complete current disclosed obstacle set before treating them as a freshly certified route, advancing a corner by a connector proof, or using a new lookahead.

Optional-service exhaustion does not disprove a retained route. After the current generating witness, owner, requested endpoint, radius and kind still match, keep its existing corner and cursor as deferred private intent. The bounded straight fallback may aim at that corner; it is not a reused movement certificate. Changed coherence/predictive advice still requires complete current known-static and permitted-terrain proof. Every actual segment still passes authoritative swept-body and terrain checks. Do not advance the saved cursor or retain a new lookahead from incomplete work. A complete negative saved-leg body/terrain proof invalidates the old route even if a later check exhausts. A complete clear direct-goal proof releases it. Missing or changed generating facts always invalidate immediately. Never report a deferred intent as a fresh planning certificate.
Earlier permitted clearance failures can explain which candidate was cached; they cannot supply hidden geometry to later checks.
Equal permitted observation histories and own controller state must produce equal proposals and query service until observation or physical contact changes those facts.
Final authoritative swept-body and terrain checks still certify every actual committed segment.
Reuse the bounded local detour architecture and traffic allocations; keep all quotas, frozen workloads, and completion deadlines unchanged.
The cooperative index and at most one disclosed-body index per viewer are built sequentially. At most `1+SLOTS` traffic indexes are built per tick; multiply the existing per-index build-work bound by this count. Count every query against the existing whole-index query bound and owner credit, including empty disclosed-body queries.
Verify actual return movement around the frozen visible enemy formation, all required visibility churn, hidden-world equivalence, and allocation/ID reversal.

## Cooperative certification of friendly movement

First compute the existing complete, continuously certified baseline result in its existing physical arrival order.
A second bounded pass can replace same-owner in-flight trajectories with mutually certified full proposed trajectories.
Keep every actual baseline destination arrival and settled body at its baseline trajectory. A unit held short of a raw arrival may retry its full proposal only as its own root, in the unchanged physical order. It cannot join another root's support set. Non-root support members must remain in flight after their raw move. Freeze baseline completed, resting, stationary-holder, and unadmitted rows.
Include full raw envelopes for eligible arriving roots as well as support members. Later roots must find every earlier upgraded trajectory.
For a staged friendly set, check every potentially touching staged pair exactly. Check every outsider against its currently certified trajectory, never a speculative future proposal. A later upgrade must also respect earlier successful upgrades.
Only same-owner raw proposals can supply support. Foreign raw proposals, targets, and progress cannot select or validate support.
Publish the whole staged set only after all checks pass. An incomplete, unsafe, or over-budget attempt changes no baseline or previously certified output.
Keep the existing 128-candidate, 65,536 shared-work, per-actor pair, physical-row and 5 MiB guard-scratch limits. At most the public physical-row cap can be staged. Share per-actor and owner charges across baseline, every failed attempt, and every successful upgrade; never replenish them.
Record actual support work separately. Preserve first-arrival retention, exact tangency, non-deepening recovery of invalid initial overlap, hidden-world equivalence, and ID/enumeration independence.

## Bounded preferred-velocity steering

Design draft. Replace legacy soft separation and endpoint projection only for admitted terrain-movement proposals. Retain long-range routing, destination selection, exact certification and all existing acceptance workloads.

The OC7 trace proves the defect: at tick74, scout7 has a certified goal below its position, but raw separation steers upward into the stationary wall. Other raw paths genuinely intersect. Relaxing collision checks cannot repair this. Evidence: `terrain-occlusion/assembled/final-leg-warmed-functional/cohort-trace/`.

Use a deterministic, bounded agent-only ORCA solver. Its result is advice. Existing exact swept-body checks, authored speed/world bounds, terrain checks and retries remain authoritative. No public schema or wire change is needed.

Run the motion hook after the existing planner selects the effective goal and before motion/progress updates. An admitted finite displacement replaces soft separation and endpoint projection; neither may rotate it afterward. Preserve claim selection, cancellation, physical escape, waiting and terminal states. A private waypoint is not an owned destination. Keep fractional arrival at actual destinations and the closest feasible stop beside a holder. Update progress once from final accepted motion.

Freeze a permitted view per owner before generation. It contains own current poses, footprints and explicitly labeled predictions from frozen own movement commands. Stopped/resting own bodies have zero predicted velocity. Command predictions are not measurements of accepted motion. Foreign poses/footprints come only from the exact CURRENT Start witness used for observed detours. Treat foreign bodies as stationary, with full avoidance responsibility on the actor. Never read foreign intent, hidden live geometry or another owner's velocity cache. Explicit AllVisible uses its existing pinned public port. Missing restricted witnesses never fall back to truth.

Store only final accepted own motion. Reuse history only for the same owner, consecutive tick and matching accepted pose. Reset on ownership change, removal, skipped history or external pose change. Raw proposals and rejected retries must not become history. Equal permitted histories and own controller state must produce equal proposals and charged service under ID/allocation/enumeration reversal.

Model at most eight nearby bodies per actor, selected by distance and physical state through bounded spatial queries. Discover along the complete bounded preferred step, including a validated coherence step when available, rather than only around the starting point. Keep the existing local padding around this segment. A five-meter step must be able to model a current body at its far end. This is a permitted-view heuristic, not a complete prediction of all moving trajectories. Keep at most eight neighbors, the same actual operation/visit/memory caps and the same conservative response to incomplete discovery. Final exact guards still certify all actual motion. Initially take full avoidance responsibility for EVERY neighbor, including own movers, holders, waiters and actors without a model attempt. Do not assume another actor will reciprocate. Use the frozen own command predictions as model inputs; never present them as measured velocities. All foreign predicted velocities remain zero. Do not use IDs or ambient randomness for ordering or degeneracy handling. Unresolved initial overlap may decline the model; the exact guard retains its recovery rule.

For eligible terrain motion, a declined or unadmitted predictive model retains a completed, validated coherence step only for the identical effective target. Otherwise it uses the bounded straight preferred step toward that target. Unchanged exact guards follow either result. It must not rerun soft separation or endpoint projection. This fallback is not an authorization to move through an obstacle. Actual short-destination arrivals and existing terminal/recovery policies retain their separate semantics.

Keep the existing 65,536 shared-work limit, owner credits, physical-pair limits, row cap, 5 MiB guard scratch and 8 MiB traffic allowance. Models and existing detour guidance share the SAME optional half-pool; do not create another allowance. Permit at most 128 model operations per actor per tick and at most the remaining optional credit per owner (16,384 before other guidance with current constants). A discovery query, extracted candidate examination, constraint construction and each half-plane evaluation consume model service; count repeated evaluations. Also charge discovery through existing owner admission.

Before optional traffic routing, reserve 8,192 of each owner's existing optional pair credits and neighbor visits for steering. Routing uses the remainder. Refusal of an optional traffic query preserves the incoming permitted waypoint and bypasses only the controller stages that require that query. Real route, gate, capacity and permission waits remain waits. Final exact certification still receives its reserved half-pool.

First compute cheap group coherence, then admit expensive predictive solves from the remaining credit. Coherence uses a work-conserving pass in oldest-attempt then physical-state order. Start an actor only with at least 32 operations available; allow at most 128 total steering operations per actor across coherence, prediction and retries. Release unused coherence credit for the next actor in this frozen pass. Do not refill any actor's spent credit. Predictive solves need a whole remaining actor ticket of at least 64 operations; otherwise retain the completed coherence result or straight fallback. Rotate actual service across ticks; do not allocate unusably small shares. Coherence and predictive attempts have separate service ages. Cheap-stage work and unused reservations cannot reset the waiting age of the expensive stage. Known short actual-destination arrivals keep metadata but do not reserve predictive tickets. Treat indistinguishable admission-boundary ties consistently, without an ID choice. Keep tickets, spent counters and frozen inputs through retries. Record the actual attempted service and fallback cause. Incomplete discovery, exhausted service or nonfinite output falls back without losing the order. Include all history, view indexes and solver storage in allocation bounds. Do not retain multiple large traffic indexes.

### Fast group coherence (design draft)

The corrected private-waypoint contract passes 90 movement tests and 11 adapter tests. The actual default OC7 tape still fails at tick 60. Reserving only pair credits reached tick 120 with own-command predictions; reserving neighbor visits also exposed an optional-query refusal that stopped an isolated spotter. Those experiments are diagnostic, not accepted policy. Evidence: `terrain-occlusion/assembled/preferred-velocity-real-destinations/` and `preferred-velocity-row-reservation-experiment/`.

Freeze original own order-group membership separately from temporary waypoint state. Group identity is an equality key only. It must not order service or select winners. Plain or unrelated orders do not join a group. Use spatially local alignment within each active order group and equal footprint radius. Build cells of width four times that radius, anchored at the componentwise minimum position of the complete contributing subgroup. Each actor uses only its own cell and eight adjacent cells. Compute cell sums once in canonical physical order, then combine the nine cell aggregates in fixed spatial order. Do not mix distant opposite flanks into one global heading. Exclude waiting, resting, cancelled, physical-recovery and invalid/zero-heading rows; actual short-destination arrivals do not contribute. Charge every contributing row. Publish no partial mean if its complete owner batch cannot be charged. A local neighborhood needs two distinct valid contributors; a small or cancelling mean supplies no coherence advice. Charge every distinct cell query through the existing operation and visit counters. Refused/incomplete local discovery produces no partial mean. Cell coordinates must remain finite and query aliases must not duplicate a contributor.

Blend an actor's own preferred heading equally with that group heading only when their dot product exceeds 0.5. Keep the authored speed cap, cap short private-waypoint preference to its distance, and preserve actual destination/closest-holder arrival handling. This is alignment within an existing order, not a new destination or a force that pulls unrelated units together.

Compute and validate coherence before expensive model admission. Every changed coherence chord needs complete CURRENT known-static clearance and permitted terrain clearance. A compact static-only spatial index can avoid scanning moving rows again; its capacities count toward the same 8 MiB bound. Refused discovery, proof or credit retains the straight preferred fallback. Exact moving-body and physical terrain certification remains mandatory. A stored coherence result applies only to its unchanged effective target; a retry or selected claim that changes that target cannot reuse stale advice. Freeze all inputs before motion generation. Never use a peer's primary steering output as a new model input.

Prove unrelated-order isolation, coherent turns, static/cliff refusal, stationary holders, private/real arrival semantics, useful service under dense routing, owner privacy, ID/enumeration reversal, work and memory limits. Then run the unchanged failing tape and full acceptance workloads. Coherence is not accepted merely because its small tests pass.

Implement the small deterministic solver with attribution to the [ORCA paper](https://gamma.cs.unc.edu/ORCA/publications/ORCA.pdf) and [RVO2 reference](https://github.com/snape/RVO2). Direct use of [dodgy_2d 0.5.5 source](https://docs.rs/dodgy_2d/0.5.5/src/dodgy_2d/lib.rs.html) needs modification: its degenerate collision branch uses ambient randomness.

Prove the raw-motion seam, actual-destination retention, exact collision/speed/terrain safety, stationary holders, parallel and opposing flow, numerical refusal, quota fairness and hidden-world equivalence. Then run the actual failing OC7 return, full warmup and unchanged measured campaign; GM1–GM8; final browser/player probes; and existing full gates. A small solver demonstration alone is not acceptance.


### Swept predictive discovery correction

The unchanged OC7 tape at736079bb still fails. A concrete trace shows unit16 at tick17 receives modeled advice across a five-meter step but sees no neighbors in its1.2-meter start neighborhood. Its path ends almost on unit30's original position,4.991 meters away. The exact guard correctly clips the move. Discovering along the preferred segment repairs this blind spot without increasing any work allowance or claiming complete dynamic discovery. Other traced failures are genuine coupled collisions or model refusal; this correction does not promise to solve all congestion. Evidence: `terrain-occlusion/assembled/local-coherence-warmed/`.


### Bounded infeasible-model refinement

The swept discovery correction passes its focused tests but the unchanged default tape still fails at tick60. Saved actual inputs prove empty moving-prediction halfplane intersections, including a robust contradiction of38.72079 at unit3/tick57. Another case needs speed387.53 despite the100 speed cap. Declining every such model leaves the same dense entry stalled. These are not floating-point noise or a reason to increase speed/work limits.

Keep the current full-responsibility boundary construction. If the ordinary closest-velocity program has no solution, allow a bounded RVO2-style projected refinement of incompatible moving-own predictions. Static/stopped own bodies and every CURRENT foreign body remain hard model constraints. This refinement proposes a compromise; it is not a collision certificate and need not satisfy every moving prediction. Preserve the speed circle, finite results, current complete known-static/terrain proof, authoritative exact body/terrain guards and first-actual-arrival rule. If hard constraints cannot be satisfied, arithmetic is unresolved, or existing work credit ends, decline the model. Never publish a partial refinement after credit refusal.

Keep at most eight neighbors and128 total steering operations per actor, including original solve, projected refinement, retries and proof. Use fixed arrays only. Charge every constraint/projection evaluation and retain canonical physical order within hard/soft partitions. Do not use handles or randomness. Keep the prior strict solver available for its existing feasible/degenerate behavior checks. Tests must cover the recorded contradictory input, incompatible speed-circle input, hard obstacle preservation, finite speed-bounded compromise, operation-prefix refusal, degenerate/parallel constraints and order/handle invariance at the adapter. Run the unchanged default tape immediately after focused checks. Other congestion and final capacity remain open until actually passed.


### Work-conserving primary prediction

The current prediction pass reserves every selected actor's full remaining allowance before any of those actors runs. Unused reservations are released only after all raw plans are computed. This can deny prediction to other eligible actors even when the selected actors consume much less than their reserved allowance. The immutable permitted view already makes each primary prediction independent of earlier generated advice. Existing actual tick31 counters prove at least9,401 unused reserved credits while only202 of500 actors complete predictive discovery. Credit stays owner-local; this aggregate lower bound is not a promise of transferable work. More useful prediction can consume guard slack, so the unchanged final physical work cap must still pass.

Evaluate primary predictions in oldest predictive-service age, then canonical physical order. Keep the existing whole-remaining allowance and64-operation minimum at admission,128 total operations per actor and all owner limits. For indistinguishable physical ties, admit the whole tied batch or none; keep its operation reservation until the whole batch finishes. Before a multi-actor tie batch starts, require enough owner visit credit for every actor's full remaining operation allowance plus one possible admission-before-charge overshoot per actor. No other prediction runs during the batch. Charge actual visits only; reservation alone cannot update service age. If either resource cannot protect the whole tied batch, decline it before any prediction work. After a complete actor or batch evaluation, release only its unused reservation before considering the next independent actor or batch. Charge every actual operation. Generated advice must not change any neighbor forecast or query input.

Cache each completed primary result for its captured pose, effective goal, motion kind, speed, radius, timestep and bounds. Preserve validated same-target coherence when prediction is refused. Raw planning can reuse only an identical request. It cannot repeat the prediction program or receive fresh credit. A changed private goal or kind discards the cached proposal and uses the existing bounded fallback and unchanged exact guards. Keep the original accepted destination and actual spent counters. Actual charged primary computation updates predictive service age even when its result is refused; reservation alone does not. Preserve short actual-arrival handling and first-actual-arrival ownership.

This changes scheduling and cache use, not collision authority. Fixed views, selected-neighbor semantics, complete discovery/proofs, hard constraints, speed limits, privacy, and final certification stay intact. Use reusable fixed neighbor/solver scratch for the current primary calculation. Retain only compact per-actor primary results and actual spent counters. Up to the existing4096 actors may receive service, but no array of4096 full neighbor tickets is needed. Include real scratch capacity and cached-result bytes in the unchanged8 MiB bound. Tests must prove reuse of unused credit under the same owner cap, exact tie-batch behavior, unchanged requests without new work, changed-goal/private-kind fallback without credit, independent service clocks, reversal, and maximum allocation. The unchanged default tape remains the acceptance prerequisite.

### Bounded coherence forecast

A returning column can stop when the forecast combines a briefly held front
unit with faster followers. Before the primary ORCA solve, capture the already
validated coherence advice for moving owned bodies. Retain the captured
forecast for bodies without advice, bodies with zero captured velocity, and
physically ambiguous bodies. Treat foreign bodies as stationary.

An advised body keeps its captured forecast when an unadvised or held body is
within the sum of their radii plus 0.05 metres in the direction of the advice.
Apply that rule in up to eight synchronous rounds. Freeze the resulting
velocities before solving primary proposals. This is a forecast only. Final
body and terrain checks still certify every committed movement. The joint
baseline below remains a separate diagnostic.

Charge bounded spatial discovery to the existing owner work and visit limits
and the actor steering limit. Charge lookup by the size of the owner's own
actor set, with the same charge for every ID. Include temporary storage in the
existing scratch bound. Skip the forecast when no actor has coherence advice.
Leave one full predictive ticket for the owner and at least the minimum primary
solve credit for each actor. If a query or allowance is exhausted, retain the
owner's entire captured forecast and keep the work already charged.

A stalled private waypoint permits a detour after about half a second without
certified progress. Its local retry restarts after one second. The earlier
trigger lets peers with different stall counts qualify together. The accepted
order and the exact movement guards still apply.

Verify owner work and forecast output under ID renaming and foreign private
order changes. Run both GM2 cases, the unchanged GM7 workload, and all other
milestone checks on the same candidate.

### Coherent permitted movement plans

The actual default tape at8b01ba99 still fails at tick100. Completed primary proposals do not predict the downstream holds: peer57 plans2.88 meters but moves almost zero; peer61 plans5 meters but moves0.136. A following scout's path is clear against either commanded or completed-primary forecasts, but intersects those shortened paths. Shared responsibility and accepted-velocity diagnostics also fail. More independent velocity tuning does not repair this causal mismatch.

Build an owner-local planning baseline from owned bodies and CURRENT permitted foreign bodies only. No hidden body, authoritative foreign future output, undiscovered terrain or uncommitted truth-guard result may enter the planner. Use captured permitted terrain clearance. Foreign bodies remain stationary planning obstacles. This is a planning model, not authority to move.

The baseline uses bounded preferred/coherence steps and the existing exact same-owner swept certification/support semantics. Use a cheap baseline search with no tangent or24-step prefix search. Unsafe unsupported proposals hold. Preserve actual destinations/private-waypoint meaning and actual-short-arrival policy. If a whole valid baseline cannot be built within the same existing work and memory limits, decline it for that owner and retain the existing safe fallback policy. Do not publish a partial plan.

Once the baseline is complete, use its owned planned velocities for predictive inputs. A changed primary proposal must pass the existing complete static/terrain proof AND a complete exact swept-body proof against the owner's current whole planning baseline. On success replace that actor's planned chord; on refusal keep its certified baseline chord. Later upgrades certify against already replaced chords, so the complete owned planning set remains mutually valid. Do not use an unchecked extra physical pass. Freeze model forecasts from the completed baseline; updating exact certification state does not feed uncommitted truth back into planning.

Retain the same owner-local operation/visit envelopes,128 steering cap, final guard half-pool,8 MiB traffic and5 MiB physical scratch caps. Charge actual baseline and upgrade work; never reset consumed work or hide a second quota. Bound storage before allocation, count simultaneous phase peaks, and retain no duplicate large index unnecessarily. Multi-actor indistinguishable physical batches may conservatively keep their baseline rather than select an upgrade by ID. Final exact authoritative body/terrain certification and first-actual-arrival ownership remain mandatory.

Test known-input-only privacy, all-or-none baseline refusal, convoy support, conflicting crossing proposals, exact safe upgrade preservation, owner isolation, actual/private arrivals, deterministic reversal, work exhaustion and phase memory bounds. Reproduce the recorded hold/follower counterexample. Then run the unchanged actual tape before capacity or browser claims. Start with an explicit test-probes diagnostic switch so baseline behavior remains comparable during integration; do not call that switch a shipping policy.


### Cheap joint baseline staging

The charged original-candidate repair still leaves zero predictive solves on the actual tape.
A diagnostic reservation restores 60–89 solves per tick, but fails movement: one designated unit never moves by tick20.
Larger reservations stop the ordinary spotter at tick1. None is accepted policy.
The cheap baseline currently certifies in-flight proposals against original discs, then repeats related work in atomic support.

For the permitted planning baseline only, ordinary in-flight candidates may begin at their unchanged poses and proceed directly to the existing atomic same-owner support stage.
Derive eligibility from immutable original/raw state before replacing any position with a hold.
Use the full existing support predicate, finite speed-bounded travel, original/raw Resting exclusions, actual semantic destination and explicit recovery exclusion.
Explicit physical escape/recovery, resting bodies, and actual short-destination arrivals retain the ordinary certification path and physical order.
A held in-flight body remains an obstacle during that path.
Do not duplicate original-disc admission or ordinary movement proofs for an in-flight candidate whose only attempted movement proof is the support stage.
Keep eligibility to attempt support separate from completed original admission and certification. Do not fabricate either completed state.
Every support query, extracted row, pair proof and changed terrain chord still pays the existing shared charges.
Each staged dependency and the entire complete peer union must satisfy the unchanged limits before publishing any movement.
A failed or incomplete support set changes no baseline output; unsupported candidates hold.
Keep the complete permitted-world baseline, no-partial-plan publication, whole-request cache binding, privacy and final authoritative guards.
No new owner allowance, actor allowance, weaker arrival rule or acceptance target is introduced.
Keep this path behind an explicit diagnostic switch until the unchanged actual tape passes.
Test convoys, crossing proposals, actual arrivals against held bodies, recovery, refusal without partial commit, deterministic reversal, every charge boundary and phase memory.

## Packed stop behind the holder (operator decision 2026-11-22)

Status: Decided by the operator on 2026-11-22 (option A). This section refines
"Later units stop at the closest feasible non-overlapping position". It changes no
other rule, GM row, deadline or assertion.

Play example: a group arrives at a point where friendly units already stand. One late
unit is boxed out: stopped friendly units stand between it and the point. It stops
where it is and its order ends. It does not wait for the group to leave.

Defect that this closes: a held unit could end its move only when it was as near to the
point as the holder permits (first row). A unit that was blocked in a later row kept its
order and stayed `Blocked` without limit (GM8 browser run, 3 of 3; native replay
`gm3_browser_delayed_rear_group_at_a_held_point_completes` at 56a008b4).

Inputs. All inputs of this rule come from the start-of-tick movement frame: positions,
radii, owners, targets and escape state as they were before this tick moved or stopped
a body. A unit that stops in this tick is a stopped body for other units from the next
tick. The result does not depend on service order or on a unit ID.

Terms:

- The *point* is the public point of the accepted Move order (`parking_public_goal`,
  `crates/turts_sim/src/terrain/navigation.rs:99`). It is never a private stop slot.
- A *stopped friendly body* has the owner of the actor, has no move target and has no
  physical escape. A body of another owner is never used. A moving body is never used.
- The *holder* is a stopped friendly body that occupies the point: its distance to the
  point is less than the sum of its radius and the actor radius.
- gap(a, b) = distance(a, b) - radius(a) - radius(b).
- Two stopped friendly bodies are *packed* when gap < 2 x actor radius. The actor
  cannot pass between them.
- The *pack* is the set of stopped friendly bodies that connect to a holder through
  packed pairs.
- The actor is *in contact* with a body when gap <= `CONTACT_GAP` (0.1 + 1.0e-3,
  `crates/turts_sim/src/movement/route.rs:50`), the existing contact range of the
  movement system. No new tolerance is added.

Rule. The engine ends the move of a held unit at its start-of-tick pose when all these
conditions are true:

1. A clock is complete. Two clocks exist, and each one is sufficient:
   - The held clock of the direct final leg, with no change to it
     (`navigation.rs`, `held_final_leg`): the frame target is the order endpoint, the
     activity is `Blocked`, the unit is not in an arrival yield, it has no physical
     escape, and it made no progress for 3 seconds (`HELD_FINAL_LEG_SECONDS`). The
     first-row stop uses only this clock.
   - The order clock (repair of 2026-11-23, `held_order`). It belongs to the accepted
     Move order and its public point, not to the frame target. It runs on each tick in
     which the order is active on a direct row (no waypoint and no policy wait replaced
     the target), the unit has no physical escape and is not in an arrival yield. The
     frame target can be the endpoint, a private stop place, or a spare or spread slot
     of the same order. A tick in which the local mover refused the row also counts.
     Progress is measured on start-of-tick poses, with a best distance and a tick
     count, by two distances: the distance to the public point, and the distance to
     the current frame target. The clock starts again only when one of them gets
     smaller than its best distance by more than `CONTACT_GAP`. So a unit that walks
     along the pack to a free slot is not stopped on its way, at any speed. A change
     of the frame target sets a new best distance for the new target and does not
     start the clock again. Another motion is no progress. The limit is the same
     3 seconds. The clock ends with the order.
   With the order clock, the unit can have moved in the tick. The rule then applies
   only when the certified pose of the tick is not more than `CONTACT_GAP` from the
   start-of-tick pose, and the move ends at the certified pose.
   A slow unit (speed 2.02 or less at 20 Hz) meets this pose bound while it walks, so
   the bound is not the protection of a walking unit: the frame-target distance of
   the order clock is.

   Slot of a stopped unit: the group mover builds the slot claims on each tick from the
   frame targets of the units (`movement/frame.rs:328-334`). A packed stop removes the
   target of the unit, so from the next tick the unit claims no slot. A later unit can
   use that slot when no body stands on it.
2. The actor is in contact with one or more bodies of the pack.
3. Each body in contact with the actor that is ahead of it (positive component along
   the direction to the point) is a body of the pack. If an enemy, a moving body or a
   stopped body that is not in the pack is in contact ahead, the rule does not apply.
4. Terrain does not hold the unit: the existing swept terrain certificate of the
   direct step toward the point (length: the actor radius, as in condition 5) is
   clear. Then only pack bodies refuse that step. A
   unit that terrain holds keeps the earlier rules.
5. Guard: no free step nearer the point exists. The step set is constant: the direct
   step toward the point and, for each body in contact, the two tangent directions
   around it. The step length is the actor radius. A step is free when the existing
   exact swept body guard and the existing swept terrain guard both accept the whole
   step against the start-of-tick frame. A free step is *nearer* when its end is
   nearer the point than the start by more than `CONTACT_GAP`.

Guard result. When a free nearer step exists, the unit does not stop. The end of the
nearest such step becomes its private stop slot, by the existing spare-slot change of
the plain mover (new slot, new target, stall count 0), and ordinary certified movement
takes it there. The held clock starts again. Each change moves the stop slot nearer
the point by more than `CONTACT_GAP`, so the number of changes for one order is not
more than the start distance divided by `CONTACT_GAP`. A unit cannot stay `Blocked`
without limit while the guard finds a step.

When the rule applies, the result is the result of the existing first-row stop: the
target is removed, the pose becomes the arrival slot, and the activity is `Resting`.
From the next tick the unit is a stopped friendly body, so the units behind it can use
the same rule (the cascade is intended).

Rules that do not change:

- The first physical arrival keeps the point. This rule never moves a stopped body.
- No unit ID, entity order or iteration order enters the conditions.
- The rule reads only bodies of the owner and terrain that the owner is permitted to
  know. It adds no fog disclosure.
- A unit blocked by terrain, by an enemy, by a moving body, or with no holder at the
  point keeps the earlier rules (wait, recover, or truthful `Blocked`).
- A unit never ends idle while it overlaps a body or terrain: a unit with a physical
  escape is excluded by condition 1.
- The impossible fixture of GM3 (`GM3-settled-blocker`) stays a stable wait: no holder
  stands at either point.

Bounds. The engine builds the pack one time for each tick and each (owner, point,
actor diameter), and only when one or more units of that key have a complete held
clock. It uses the existing spatial index of the movement frame. Each body visit and
each guard step is charged to the existing per-owner movement work allowance; no new
allowance is added. A refused charge leaves the
earlier rules in force for that tick. The pack has no fixed size limit: only the
work allowance bounds it. The pack visits are recorded in the navigation
diagnostics and in the GM7 work rows.

Known limit of the allowance: the allowance is `SweepBudget::charge_guidance_pair`
(4096 charges for one unit and tick, and half of the pair credit of the owner). A
pack whose build is refused falls back to the earlier rules on each tick while the
refusal holds: the unit stays `Blocked` and does not stop. Measured: a square pack of
121 bodies and one late unit use 430 charges (test
`a_large_pack_is_bounded_by_the_work_allowance_only`, which uses the real allowance).

Acceptance (added to GM3, nothing removed). Each test names the build where it is red:

- Second-row stop: a fixture with a holder, a packed first row and one late unit in
  contact behind it, no free step. Red before the rule (the unit stays `Blocked`),
  green after. The replay `gm3_browser_delayed_rear_group_at_a_held_point_completes`
  is red at 56a008b4 and green on the repaired build.
- Packed stop, full simulation:
  `gm3_packed_stop_ends_boxed_out_moves_in_the_first_and_in_a_later_row`. Ordinary Move
  orders in the frozen scene `GM3-shared-and-unique`, where `rear.large.2` rests on the
  point. One unit ends its move in contact with the holder, and one unit ends it in a
  later row, in contact with a pack body only. A control unit with a free way reaches
  the holder with no packed stop. The four ID variants give the same poses. Red when
  the packed stop is disabled (the first unit stays `Blocked` past the deadline of
  400 ticks).
- Order clock, full simulation, both red on the build before the repair of 2026-11-23
  (the second unit stays `Blocked` past the deadline of 600 ticks), four ID variants
  equal: `gm3_packed_stop_ends_two_orders_accepted_in_one_tick` (two Move orders to
  the held point in one tick) and `gm3_packed_stop_ends_a_group_order_at_a_held_point`
  (one group order). These cases are also stuck on ce686989 and 3bfd66fa.
- Order clock, unit tests: `order_clock_does_not_complete_while_the_unit_gets_nearer_
  each_second` (an approach of more than `CONTACT_GAP` in each second never completes
  the clock; red if progress is ignored) and `order_clock_counts_small_motion_as_no_
  progress` (motion inside a band narrower than `CONTACT_GAP` completes it).
  `order_clock_does_not_complete_while_a_slow_unit_walks_to_its_slot`: a unit of speed
  1.0 walks to a slot at an equal distance from the point and is not stopped; red when
  the clock measures only the distance to the point (48d686ec).
  `order_clock_is_not_started_again_by_a_slot_change`. Limit: the slow-unit case has
  no full-simulation test, because the frozen GM scenes have no unit of speed 2.02 or
  less (their speeds are 3.5 and 2.5).
- Guard, decision function: a holder, one pack body and a late unit in contact from
  the side, with a free place beside the holder. The result is a step nearer the
  point, not a stop (`a_free_gap_nearer_the_point_must_be_reached_first`). Red when
  the guard is removed. Control: with the place closed the unit stops at contact.
- Guard, full simulation: `gm3_late_unit_reaches_the_free_place_beside_the_holder`.
  Ordinary Move orders build a holder, a wall of three resting bodies and a late unit.
  The late unit completes in contact with the holder, in all four ID variants. Red on
  a build that stops the unit at first contact with the pack (it completes 1.53 from
  the point, behind the wall). In this scene the ordinary mover walks around the
  wall, so the held clock (condition 1) does not complete and the rule does not fire.
- Terrain (condition 4): a unit held by a cliff edge with one pack body in contact at
  a shallow angle ahead does not stop. Red when condition 4 is removed.
- Condition 3 boundaries, one test each, red when the boundary is removed: a pack of
  another owner; a moving body ahead; a stopped friendly body ahead that does not
  connect to the holder.
- Start-of-tick inputs: two held units A and B in a row, B in contact with A only. B
  does not stop in the tick in which A stops; it stops later. The result is equal
  under ID reversal.
- ID reversal of the replay gives the same physical rows.
- All existing GM1 to GM6 native tests pass with no changed expectation.

Measured end state of the browser-timed replay at 56a008b4 (the red case): the late
unit stands at gap 0.000 to `rear.small.3` and gap 0.14 to `front.small.3`, 1.6 from
the point; the holder stands on the point.

### Stale physical escape (defect repair, no rule change)

A second cause had the same visible result in the first row. A physical escape is
owed only while its trigger is true. Every production writer creates it from a body
overlap (distance < r1 + r2 - 1.0e-5): `movement/plain.rs:407`,
`movement/frame.rs:705`, `:737`, `:1978`. The sweep (`movement/sweep.rs`, `finish`)
copied the escape of the start-of-tick row back to the output on each tick in which
it clipped the proposed pose, with no test of the trigger. A unit in contact, whose
proposal was refused on each tick, kept the escape without limit, and condition 1
of the held clock excluded it.

Repair: `finish` keeps the escape only when the certified pose overlaps a body of the
start-of-tick frame, by the same predicate and tolerance. The exclusion of escaping
units from the held clock stays. Tests: a clipped pose in contact does not bring back
a discharged escape (red on the earlier build); a clipped pose that still overlaps
keeps its escape (control); gate `gm3_feasible_opposing_shared_unique_and_movable_
obstruction_complete` with unchanged expectations.

### Work limit of refused direct lines (GM4 repair, no rule change)

A unit asks one swept terrain certificate from its pose to its endpoint. A refused
long line is remembered until the unit leaves its start cell or another input
changes. The fresh direct-line work of the units of one order with no clear line is
limited to 4 x `GEOMETRY_BUDGET` operations per tick. The units of one order take
turns, and an order does not delay another order. One viewer has 8 order accounts
and one shared account, so the worst case for one viewer and tick is
9 x 4 x `GEOMETRY_BUDGET` operations (about 60 ms at 1.7 ms for each refused sweep).
Known limit: the limit bounds only new and refused work. A unit on a long clear
line still makes one fresh certificate on each tick, as before this repair.

Minor list (not in this change): why the late unit did not take a route around the
stopped group to its private stop slot is NOT TRACED. A point beyond two gates, where
gate 1 is packed and gate 2 leads to free space near the point: units at gate 1 stop
outside, because the guard tests only local steps. For the same cause, the guard
does not find a place that needs a walk around the pack: only the held clock
(condition 1) prevents a stop while the ordinary mover makes progress. When the
ordinary mover is wedged for 3 seconds in contact with the pack, the unit stops
there, also if a walk around the pack leads to a nearer place. Native reproductions
of the wedge, on ce686989 and on this build (same pose): in `GM3-shared-and-unique`,
with no other order, a Move of `rear.small.2` to (6, 10) stays `Blocked` at
(18.80, 26.93) between two resting friendly units for 2400 ticks or more.

Pair limit (condition 3): two held units side by side, in contact with each other and
with the pack, are each a moving body in contact ahead of the other when
|dA^2 - dB^2| < |AB|^2 (d = distance to the point). Then the rule applies to neither
unit while both have a target. A full-simulation probe with a closed ring of six units
around a holder and two late units side by side did not reproduce a block: both units
completed in about 40 ticks by another arrival rule, in four ID variants, as two orders
and as one group order (evidence `target/mc-evidence/gm3-delayed-rear-replay/pair-probe/`,
scene 1). The limit stays recorded for the decision function.
