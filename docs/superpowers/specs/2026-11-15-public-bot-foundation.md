# Public bot foundation

Status: gate 1 findings incorporated. The plan needs the same reviewer before implementation.
Baseline: SDK14 candidate `216cdd62`. Public authoring API: `0.2.15`. Network protocol stays `8`.
Review budget: one Codex reviewer, reused for the spec and plan. Two review turns.

## Result and limits

Mods can offer conventional opponents through public profiles and a public Rust framework.
The Skirmish opponent uses these facilities. It has no private controller or privileged world queries.
A second package uses the same framework with different ownership, economy, and production arrangements.
The player still starts the current skirmish through its launcher and can finish a match against the opponent.

This is the bot foundation of MP2. It does not complete MP2 or the full engine.
Shared human/bot remembered sightings, fog, geology, scripts, and creator bot selection remain open.
Advanced tactical search, diplomacy, personality systems, and automatic human takeover are outside this delivery.
The current two-slot match limit remains. The runtime supports one independent bot instance per configured bot slot.

## Public extension boundary

Add a renderer-independent Rust crate, `turts_bot`, for controllers and coordination.
It has no Bevy, `turts_sim`, networking implementation, filesystem, clock, or global random-state dependency.
Its public identifiers, definitions, observations, proposals, results, and configuration types are portable Rust data.
Content references use the existing qualified `ContentId` contract.

The public controller interface includes `BotController::step`.
It receives immutable `BotFrame` metadata, a bounded `BotQueries` interface, and a bounded `ProposalSink`.
It does not receive a world, mutable gameplay objects, a server connection, or an unrestricted closure-based query.
`BotCoordinator` accepts proposals from multiple controllers and returns ordinary order requests.
A host can register a controller factory by content identifier through `ControllerRegistry`.
Factory validation returns field-path errors. Each factory creates fresh state for each match and slot.
A third-party Rust integration test must implement and compose a controller through these public interfaces.
It must also replace the supplied strategy without modifying engine code.

Host-linked Rust extensions are trusted host code. They are not downloaded package code or a script sandbox.
The stock host supplies reusable controllers. Downloadable profiles configure and compose those controllers.
Portable archive loading checks profile structure and references, not host factory availability.
Host activation refuses an unavailable factory with the profile's field path before bot admission.
A client does not need a server-only custom factory to load the same locked package.
A custom factory can be linked into a compatible host through the public registry.
Its package still uses ordinary profiles, observations, coordination, and validated orders.
This does not promise that a stock server can execute arbitrary Rust supplied by a package.
Future script controllers can implement the same proposal boundary after the separate scripting selection work.

## Package authoring

Add optional `exports.bots` files containing named `BotDef` definitions.
A definition selects registered controllers and supplies their validated configuration and shared game knowledge.
Shared knowledge names roles, eligible content, production recipes, order definitions, economic goals, and priorities.
References are explicit. The engine must not infer a base from `build_area`, or select the first matching order.
Roles can contain several content types. One unit can satisfy several roles.
A definition also selects its decision period and its start condition.
Start conditions include MatchRunning and HumanPresent. HumanPresent is an authored choice.
No framework rule requires a human before a bot can act.

The supplied controllers support opening goals, economy recovery, production targets, and simple squad attack/defend behavior.
They use the current gathering, production, construction, supply, and payment mechanisms.
Their game knowledge does not contain Skirmish names or fixed army counts, timers, costs, or resource assumptions.
An opening can be interrupted by a higher-priority recovery goal.
Recovering a lost gatherer and restoring a declared operating-resource target are supported mechanisms.
More elaborate strategy remains mod policy and later controller work.

Library import does not install a bot. The game explicitly selects its definitions.
Configuration and state do not leak between instances, matches, or packages.
Reject malformed references, incompatible capability/recipe assignments, invalid periods, and duplicate authored instance or goal keys.
Portable loading does not reject an unavailable host factory. Host activation validates factory-specific configuration and availability.
API version checks also detect explicitly written empty `bots` exports below `0.2.15`.
Packages without bot exports retain their previous behavior and serialized audit shape.

## Observations and knowledge

The host supplies one completed-tick observation basis. Decisions do not read partially applied work.
Queries expose own units, own accounts/reservations, own jobs/sites, public content definitions, permitted units/nodes, and match state.
Private enemy accounts, paid amounts, orders, operating diagnostics, and repair debt are absent from the interface.
Do not pass the full map definition or raw `WorldView` as public game knowledge.
Placement and affordability advice uses only the permitted observation provider and public rule definitions.
Where coverage or an endpoint is not observed, return Unknown/NotObserved instead of consulting hidden state.
Unknown coverage depends on permission, not on whether an unseen blocker exists there.
Direct lookup cannot distinguish hidden from absent. Neither answer asserts death.
Admission separately revalidates authoritative state. Its generic refusal must not disclose a hidden blocker or private account.

The observation provider is a public boundary separate from controller policy.
Human export and bot observations use a shared permission decision before projection.
The present permission mode makes every existing unit and surface node visible, as human export does today.
This foundation adds no sight radius, fog mask, terrain occlusion, or remembered enemy representation.
It must not claim secrecy or scouting that this mode does not provide.
A test provider can withhold records. Controllers cannot discover those records through another query.
Future shared visibility and remembered-sighting records extend this provider; controllers must not depend on raw world access.
A lost observation is not proof that its subject died. Only an authorized event can assert a death.

Every query declares a limit. Results report completion or a continuation and their observation tick.
Partial results never masquerade as a complete count or an empty world.
Pages use deterministic physical/content order and explicitly mark unresolved equal-rank boundaries.
A controller cannot choose a candidate until all potentially better or equal candidates are resolved.
Quota exhaustion retains selection state or explicitly restarts an expired selection. It never chooses a page prefix by identifier.
Paging stays on the same immutable basis for one decision cycle. Expired continuations fail explicitly.
The host bounds retained observation bases. A slow or exhausted bot cannot retain unbounded history or block the match.
Owned aggregate counts and pending-task facts must remain usable when individual result pages are incomplete.

## Coordination and command authority

A proposal names its goal, actors, desired ordinary order, declared priority, and required resources.
The coordinator resolves both spending and unit assignment before it emits commands.
Two controllers cannot allocate the same available stock or assign one unit to conflicting tasks in the same decision.
Track in-flight accepted work until its result or authoritative job/site state resolves it.
Repeated decisions must not duplicate a pending build, reservation, or production target.

Commitments identify the ordinary source endpoint and qualified resource, not only the resource name.
They also identify the authored task, emitted request, affected actor, and observation/result watermark.
Two inventories containing the same item are different accounts.
A provisional commitment persists while admission or per-unit feedback remains unresolved.
An acknowledgement or result alone cannot release it against an older account basis.
Reconcile only when an authoritative basis proves the exact debit, reservation, job/site, or terminal effect for that request.
Partial group acceptance reconciles each actor separately. Unresolved members retain their commitments.
The complete bounded reconciliation metadata accompanies account-basis advancement; partial result pages cannot corrupt that advancement.
Available funds subtract authoritative reservations and only commitments not already represented by that same basis.
ChargeOverTime jobs/sites retain their unpaid declared costs at that basis. Disabled work retains that commitment.
This is planning state. It does not create an authoritative reservation or spend money.
A failed/cancelled task releases its commitment only after the observed basis includes its refusal or terminal effect.
Delayed feedback and reversed observation/result arrival must not double-count a debit or free funds early.
Other actors can change the world. Ordinary command feedback therefore triggers replanning.

Built-in choices and coordinator competition use authored priority, physical source position, and explicit authored goal order.
Unit identifiers identify subjects. They never break a visible priority tie.
Exact equal ranks are refused symmetrically and produce a typed diagnostic.
Controller iteration, registration, query page, and map insertion order must not choose a winner.

Bots submit through a shared actor-admission path with humans.
Reuse catalog/order resolution, sequence checks, resource checks, per-unit ownership checks, budgets, outcomes, and command results.
A bot actor does not impersonate a network connection. It cannot claim another actor's slot.
Results return to its controller/coordinator. Rejected orders cannot silently count as completed work.
Sequences use checked advancement. Exhaustion stops that actor's submissions with a diagnostic; it never wraps.
Bot commands enter the existing command log with actor/source identity and deterministic within-tick order.
Recorded-command replay disables live bot submissions. It must not regenerate and replay the same bot command twice.
A separate comparison mode regenerates bot requests against the same inputs and compares their canonical generated-command trace.
Full replay delivery and its UI remain deferred.
No bot can create a unit, change health, move an account, or complete research through a private mutation path.

## Scheduling and bounds

Use fixed server ticks. Convert the authored decision period with checked ceiling conversion and a minimum of one tick.
RateQ does not change a bot's decision clock.
Derive controller/goal sub-seeds from the match seed, profile, slot, and stable authored instance/goal keys.
Registration, list indices, frame time, and connection timing are not random inputs.
MatchRunning is a per-step guard. HumanPresent also pauses decisions whenever no human is present.
Paused time does not advance bot action timers or produce catch-up decisions.
Pending requests, commitments, and controller memory survive a pause. Authoritative work already ordered can continue.
On resume, reconcile a fresh permitted basis before emitting commands; expired selection leases restart explicitly.
Run only started, non-ended matches. Stop submissions for defeated slots.
A missed decision does not cause an unbounded catch-up burst.
Bot decisions cannot wait for network, model inference, disk, or asynchronous external services.

Operator budgets bound queries, candidates examined, rows returned, proposals, commands, and affected units per decision.
All query work counts, including examined candidates that a filter rejects.
Profiles allocate controller quotas explicitly; their sums must fit the operator budget.
Registration or iteration order does not allocate leftover budget to a preferred controller.
The supplied controllers and coordinator stop cleanly at these limits and continue useful work on later decisions.
Exhaustion preserves valid pending state and produces a bounded diagnostic. It does not stop other participants.
Trusted custom Rust code must honor the contract. This interface does not claim to preempt an arbitrary infinite native loop.

Freeze concrete operator defaults and a representative benchmark configuration in the implementation plan before measurements.
Required workload: two bot instances, 1,000 units, 20 server ticks/second, and one-second decision periods.
On four selected logical CPU cores, the complete active bot phase must have p99 duration at or below 5 ms.
Collect at least 1,000 active decision phases. Accelerated headless ticks are permitted.
Record CPU, selected cores, unit mix, profile hashes, query budgets, tick counts, and phase timings.
Measure observation preparation and command admission as part of that phase, including deferred work attributed to it.
No cloud access or supported maximum unit count is claimed by this local acceptance.

## Host integration and player path

The operator explicitly configures `(slot, profile, seed)` through server configuration or CLI.
Reject duplicate slot assignments, unavailable profiles, invalid slots, and already human-claimed slots before starting a bot.
The existing `--computer-slot` alias can select the root game's sole exported bot profile.
With zero or multiple exported profiles, that alias reports an actionable error instead of selecting by identifier order.
Update the launcher to use the public profile. Remove its dependency on the private Skirmish controller.

This foundation supports the current Immediate start path.
It does not add a client action for creating bots or designate an arbitrary connected client as creator.
The current lobby has no creator/admin contract; its Bot seat variant alone is not an implementation.
Creator-authorized bot selection and ready-lobby integration remain a separate participant-lifecycle delivery.
Bot versus bot starts without a human when its authored start conditions permit it.
Never take over a disconnected human slot or reopen a claimed bot slot during the match.
Human reload/reconnect does not reset the bot's match state.

In the browser, the player builds and fights against the public Skirmish profile, then sees Victory or Defeat.
A separate package demonstrates the same framework with physical inventory costs and separate construction/production actors.
Its policy is authored data. It requires no game-specific server edit or privileged test commands.

## Acceptance

| Row | Required executed result |
| --- | --- |
| BF1 | An external Rust test implements, composes, and replaces a controller through public APIs. A forbidden-edge check proves no renderer, Bevy, simulation, or private server dependency in the framework. |
| BF2 | Whole-package validation loads profiles/libraries, including custom factories absent from clients. Invalid references, duplicate keys, periods, and API floors fail with paths. Host activation rejects unknown factories. Empty written exports also require API15. No import silently installs a bot. |
| BF3 | A Balance/conversion package and an inventory/storage package both build, gather, produce, and attack through ordinary commands. Their actor arrangements differ. A renamed third fixture works without changing Rust strategy code. |
| BF4 | Recovery interrupts an opening after an ordinary gatherer loss or supply-provider loss. Repeated decisions and COT work do not duplicate tasks, spend commitments twice, or assign conflicting unit tasks. Test separate same-item inventories, delayed feedback, partial acceptance, and both observation/result arrival orders. |
| BF5 | Forged ownership, stale/exhausted sequence, invalid content, exhausted budgets, and private enemy state remain safe. Hidden-present and hidden-absent worlds with identical permitted state give identical spatial/direct-ID/placement/affordability advice. No missing lookup asserts death. Page/tie boundaries and expired continuations remain explicit. |
| BF6 | Equal seeds and admitted commands repeat exact server state and bot decisions. Controller registration order and genuine creation-ID inversion preserve physical competition. Authored sub-seeds survive registration/list reordering. Recorded replay submits each bot command once; regeneration compares exact traces. Instances/matches share no mutable policy state. |
| BF7 | Immediate bot-vs-bot can run without a human. HumanPresent pause/resume retains pending state and uses active-time timers. Reconnect preserves bot state. Defeat/end stops commands. Invalid/duplicate/human-owned slot installation changes no ownership. No disconnected human takeover occurs. |
| BF8 | The defined local workload meets its phase budget. Query/candidate/result/proposal/command limits are exercised separately. One exhausted bot does not prevent the other bot or a human command from advancing. |
| BF9 | A fresh silent browser completes the public Skirmish profile loop. A second package also runs its public profile in a browser. Final cold-play probes pass. One independent foundation review runs the completed code. |

Record executed rows, evidence, measured limits, and remaining MP2/full-engine work in one report.
Update README before milestone integration. Do not replace the accepted launcher until final acceptance passes.
Every test Chrome uses a fresh profile and `--alsa-output-device=null`. Do not push main.
