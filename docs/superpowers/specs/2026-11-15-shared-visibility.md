# Shared visibility and remembered sightings

Status: Anvil spec and plan reviews approved. One reviewer completed one spec turn and one plan turn.
Baseline: SDK15 candidate `a96cbaeb`. Its BF8/BF9 acceptance is separate from this milestone.
Public authoring API: `0.2.16`. Network protocol: `9`.
Review budget: one Codex reviewer for the spec, then the same reviewer for the plan.

## Result and boundary

A mod can enable unit fog without an engine edit.
Players explore dark ground. Previously explored ground becomes dim when sight leaves it.
A lost enemy becomes a visibly marked remembered sighting at its last observed position.
Humans and bots receive the same permitted facts for the same slot and observation tick.
Combat and orders cannot use a hidden target's current position through another engine path.

This closes the shared knowledge part of MP2 and supplies the unit-visibility foundation of M5.
It does not complete M5. Geology, prospecting, terrain occlusion, stealth, and detectors remain later capabilities.
New lobby controls, spectators, scripts, save files, and multiple active maps are outside this milestone.
The current two-slot limit remains. Team sharing must work through the existing team contract.
Skirmish keeps its current all-visible default. A separate public fixture demonstrates enabled fog.
The remaining visible-ID arbitration triage is independent. This milestone must not introduce a new ID-based winner.

## Public authoring

Add optional `MapDef.visibility: VisibilityDef` and `UnitDef.vision: VisionDef`.
`VisibilityDef` contains `cell_size: f32`, `sharing: OwnSlot | Team`, and optional `memory_limit: u32`.
`VisionDef` contains `radius: f32` in world units. Both numeric values must be finite and positive.
An absent map setting selects AllVisible. An absent unit setting contributes no sight in a fog-enabled map.
Written visibility/vision fields require API16, including explicit null values under an older manifest.
Library definitions and qualified references follow the existing catalog rules.

The square grid starts at map origin. Boundary cells stop at the authored map bounds.
A cell is currently observed when its centre lies within an eligible source's inclusive sight circle.
The source's own cell is also observed. Unit/node centres select their containing cell.
Grid membership, maximum-edge handling, and zero normalization use one shared pure rule.
A foreign unit or surface node is currently visible exactly when its centre cell is observed.
The entity footprint does not enlarge sight. This grid approximation is part of the authored rule.

A sight source is an owned, completed unit with authored vision and positive health, if it has health.
A unit without health can contribute sight. An unfinished site cannot contribute sight.
Manual disable and economic shortage do not remove sight. Removal or zero health does remove it.
Team mode unions sight and explored coverage for slots with the same installed team identifier.
A slot without a team shares with nobody. Team membership does not grant control or private accounts.
Sight and memories are match-local. Package replacement clears them before installing a new initial basis.

Validate grid dimensions and checked allocation sizes before changing the world.
The map may have at most 1,048,576 cells. Checked total slot coverage storage must not exceed 64 MiB.
Oversized grids fail with a field path. Coverage cannot silently truncate or become all-visible.
A large finite radius clips to the map. It cannot overflow grid arithmetic or allocate an unbounded circle.
No sight callback, script, asset name, or Skirmish-specific rule supplies permission.
Future terrain/stealth rules must extend the same provider rather than bypass it.

## Current knowledge and memory

The server owns current coverage, explored coverage, and remembered public facts.
Explored coverage is the union of cells observed during this match. It never shrinks.
Each permitted view carries its map identity and completed simulation tick.
Current facts, remembered facts, and coverage in that view belong to one immutable basis.
Coverage is slot knowledge. It is not a camera-distance filter.
Own units and own private economy remain available offscreen and outside current coverage.
An unseen allied unit does not become public merely because it is allied in OwnSlot mode.

A remembered unit contains its ID, content, owner, position, last public facing when available, site status, and `last_seen_tick`.
Those values come only from its last permitted observation. Remembered units carry no current health or private economy.
Surface-node memory similarly retains only last observed public identity, position, and resource state, with its tick.
Stale node resource state is explicitly marked remembered. It is not an extraction promise.
Current observations replace memories. An authorized observed death/removal clears the corresponding memory.
Hidden movement, damage, ownership changes, completion, depletion, and death do not update a memory.
Reobserving the stored centre cell clears a memory when that subject is no longer there.
This disproves the old location. It does not assert that the subject died.
An identical hidden-moved and hidden-destroyed scene must produce the same permitted result.

Memory has at most one record per previously observed subject. It does not retain trajectory or event history.
`memory_limit` defaults to 65,536 stale records per slot. Authored limits must be within 1..=65,536.
When full, discard the oldest last-seen tick group until within the limit. Exact ties disappear together.
Only stale records can be discarded. Discarding never asserts death. Expose a retention-truncated flag to human/bot memory consumers.
The flag stays true until map reset. It means some past knowledge was discarded; no permanent HUD warning is required.
Do not select retained memories by unit ID. Current observations and own accounting remain complete.
Repeated hide/reveal transitions cannot grow duplicate memories.
Team mode shares observations and remembered facts from those shared observations.
It does not transmit one member's private production, orders, balances, or hidden initial map state.

## Authority and timing

Use one authoritative visibility mechanism for simulation targeting, human export, and bot projection.
They may request different explicit phase snapshots. They must not maintain different sight rules.
Tick zero has a complete initial basis before any participant receives state or a bot makes a decision.
Commands and order execution use the start-of-tick permission basis.
After movement and production, refresh tactical visibility before acquisition and firing.
Refresh completed knowledge after combat/removal and before completed-world publication.
Viewers and bot decisions then use that completed basis, including its tick.
A later source death cannot erase a previously authorized observation or retroactively authorize a hidden event.
Event export uses permission at the event's phase. It must not consult a later hidden target position.
Same-build replay reproduces coverage, memory, permitted records, and ordinary command results.
Entity allocation order, hash iteration, and viewer connection order do not choose visible outcomes.

Explicit target admission requires current permitted identity and ordinary capability/relation checks.
A hidden ID and an absent ID have the same generic refusal. Neither refusal asserts death.
Remembered sightings are not live targets. A player or controller can issue an ordinary move/attack-move to a remembered point.
An ongoing explicit attack loses its live target when that target becomes hidden.
It may travel only to the last permitted target point, then finish without reporting hidden death or movement.
Automatic acquisition drops hidden targets. It reacquires only currently permitted targets.
Weapon range, cooldown, ammunition, and damage rules remain unchanged.
Firing cannot select a target through a visibility-free fallback.
Gather/extract/repair target lookups and queued targeted orders use the same permission boundary.
Ground movement into unexplored space stays available. Placement advice reads only permitted facts and public rules.
Public map-bound and own-capability failures may be reported directly.
Other placement verdicts require observation of the complete rule-dependent region, not only the proposed footprint.
Include node_clearance, static gap, potential blocker radii, and any anchor region not proved by permitted own facts.
Derive conservative maxima from public definitions. Hidden objects must not affect the chosen region or its coverage answer.
Return Unknown if any required region is unobserved. Never call a full-truth placement check for client/bot advice.
Authoritative placement still revalidates actual blockers. Hidden and absent blockers receive the same generic refusal shape.
Ordinary physical collision remains real. Secrecy does not promise to hide consequences of a participant's physical interaction.

## Protocol, queries, and browser

Protocol9 adds bounded coverage updates and explicit remembered-record updates/removals.
Each completed change atomically groups visibility, memory, and every affected gameplay row.
Visibility loss is distinct from entity destruction. A loss cannot trigger a death effect or a kill statistic.
A reveal supplies complete permitted public state before later deltas reference the entity.
A hide clears its live health, selection/picking target, current model, minimap marker, and live interpolation track.
The client may create a separate remembered presentation from the authorized stale record.
Remembered presentation uses a dim outline/marker and a visible last-seen indication.
It has no live animation, health bar, selection command target, or hidden-position interpolation.
Remembered node information also has a stale indication in the HUD.
Ground and minimap show the same unexplored/current/explored states.
Fog is applied to all world presentations, overlays, effects, labels, and minimap layers.
Client-local zoom, selection, and camera movement cannot bypass it.

Sanitize all IDs and positions referenced by orders, rally, repairs, gatherers, weapons, and effects.
An owner-only record is not permission to expose its hidden foreign target.
Current enemy health is public only while the enemy is currently visible.
A hidden attacker's hit may change own health without revealing its ID or position.
The current engine's firing effects require permitted endpoints; otherwise omit the effect.
This milestone adds no simulated projectile. Future projectiles must declare separate permitted trajectory/impact records.
Audio and cosmetic effects cannot reveal events rejected by export.

Each connection retains one active permitted baseline and at most one pending permitted transaction basis.
Full snapshots, resync, and oversized ordinary patches use a bounded atomic state transaction.
Its stamp contains match/map identity, completed tick, a connection-local transaction ID, and its parent basis ID.
Chunk indexes/count and declared total bytes are checked before allocation. Complete encoded chunks are at most 1,024 bytes.
Keep the transport's existing MTU/frame cap; shrink chunks further if its framing leaves less payload space.
Transmit transactions on a reliable ordered state channel. Do not enqueue a whole oversized transaction without byte accounting.
Sender queues, encoded pending data, and receiver transaction staging each have a 64 MiB per-connection cap.
At most one incomplete transaction exists at either endpoint. Its bytes cannot become gameplay state before complete validation.
The receiver activates the whole new basis atomically, then acknowledges its stamp. Partial coverage never changes permission.
While incomplete, retain at most 8 MiB of encoded post-base patches per connection. They name their required parent basis.
Apply them only in order after that base activates. Never mix StateChannel deltas into an incomplete transaction.
Duplicates must be identical. Wrong stamps, gaps, conflicting chunks, and excessive sizes cannot activate a partial view.
Overflow or a 5-second unacknowledged transaction cancels it and restarts from the latest permitted completed basis.
Ignore cancelled transaction IDs. After three restarts in a rolling 30 seconds, disconnect that slow client explicitly.
Disconnect immediately if a single permitted basis exceeds 64 MiB. Never fall back to AllVisible or truncate state.
Cancellation releases queued payloads and staging. State transport backpressure cannot stall simulation or other viewers.
Reliable gameplay results keep their bounded ordinary path. A slow state transfer cannot accumulate unlimited transport messages.
Retain no per-tick world history. Logical baselines, unsent changes, transport queues, and decoder allocations count in memory evidence.
Initial snapshots, resync, reconnect, visibility loss/reveal, and late initial observation include only permitted knowledge.
A full resync contains current coverage, explored coverage, and that slot's retained memories.
It must not rebuild memories from current hidden truth. Changing visibility membership must not require a full resync every tick.
Protocol8 peers fail normal version negotiation. Old authored packages run AllVisible under protocol9.
AllVisible uses an explicit fast path, has no memories/fog payload, and preserves previous gameplay and omission rules.
Downloaded public map definitions remain public. This milestone does not conceal authored placements from package inspection.
Private future geology/seeds must remain authoritative; fog payloads must not introduce a new full-map truth channel.

Bot spatial/direct-ID/placement/affordability queries use only the same permitted basis as human export.
Expose remembered records as a separate bounded query with `last_seen_tick`, not as current `UnitObservation` rows.
Stock controllers may use remembered positions for ordinary scouting/attack-move. They cannot target remembered IDs.
Extend leases and query metadata to cover the memory/coverage basis without reading newer hidden truth.
Hidden-present and hidden-absent worlds with equal permitted knowledge return equal rows and query metadata.
This includes examined counts, budgets, continuation records, and coverage answers.
Own complete accounting and provisional commitments keep the SDK15 contract.

## Acceptance

Use two structurally different public packages, including a renamed fixture. Neither receives privileged vision code.
Keep one feature report with the row results and evidence paths. No new gate framework is required.

| Row | Required result |
| --- | --- |
| SV1 | Package loading covers API floors, absent/default settings, both sharing modes, nonfinite/zero values, boundary/radius rules, allocation overflow, oversized grids, and atomic reinstall. Old packages keep AllVisible gameplay. |
| SV2 | Real movement, site completion, disable, source damage/death, overlapping circles, teams, and boundary crossings produce exact current/explored coverage. Inverted entity creation and viewer connection order preserve knowledge. |
| SV3 | Observe, hide, move/change/damage/kill while hidden, reobserve, and resync. Exact stale records/ticks persist correctly. Observed empty locations and observed death clear memory without inventing hidden death. No duplicate history grows. |
| SV4 | Real human wire capture and public bot queries agree per slot/tick. Hidden-present/absent paired worlds give equal payloads and query metadata. Check health, every foreign reference, effects, initial snapshot, deltas, reconnect, and resync. Include appear-and-die within one tick and the final sight source dying after its authorized shot; no event references an unknown handle. Large interrupted/reordered snapshot and patch tests prove atomic activation, caps, cancellation, stale chunk rejection, restart/disconnect, and viewer isolation. Private allied economy remains private. |
| SV5 | Ordinary explicit/queued attack, acquisition, firing, gather, and repair cannot follow hidden targets. Placement advice stays Unknown over any unobserved rule-dependent region. Pair hidden-present/absent objects outside the footprint but inside node-clearance/static-gap regions; compare rows and all metadata. Forged hidden IDs refuse generically. Repeated command-log replay reproduces permitted views and results. |
| SV6 | Fresh silent browsers show different knowledge for two slots. Move a scout, observe an enemy, withdraw, then reveal its old empty location. Assert ground/minimap fog, marked stale sighting, no hidden model/health/picking, and correct clearing. Run scripted cold-play probes on the final build. |
| SV7 | Use the frozen workload below. At 20 Hz, visibility+memory+permitted human/bot projection costs p99 <= 5 ms over 1,000 warmed 20-tick intervals on four selected CPUs. Each sample sums all 20 ticks. Include snapshot encoding, chunk scheduling, deferred work, client activation CPU, allocations, resync bytes, steady traffic, and retained memory. No threshold adjustment after the run. |

SV7 uses 1,000 units split equally across two slots on a 128x128 world, cell_size 2, and source radius 12.
Each slot has 100 moving sight sources, 150 other moving combat units, 100 stationary combat units, 100 gatherers, and 50 producers.
Only the 100 sight sources per slot emit vision. Gatherers and producers work; permitted combat fires during measured intervals.
The 500 designated moving units follow fixed coordinate routes. No source movement is disabled to improve the score.
Each slot must have at least 20 foreign hide transitions and 20 reveal transitions in every measured interval.
Use 100 warm-up intervals. Request one full resync per slot every 100 measured intervals; charge all its deferred work.
Report server and client CPU separately and their sum for the threshold. Exclude network wait, not encoding/decoding/activation work.
SV7 also records the accepted 400-unit detailed-model browser scenario with fog enabled and both camera zoom extremes.
The browser must sustain at least 30 fps after warm-up. Do not reduce model/texture quality to pass.
Record the equivalent AllVisible baseline and the concentrated all-visible battle case.
Demonstrate bounded memory across repeated scouting cycles and no full-resync storm on routine visibility transitions.
These checks do not replace future M7 combined terrain/script workloads or M12 large-scale budgets.
