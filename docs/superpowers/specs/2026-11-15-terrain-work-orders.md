# M4: terrain work, material accounting, and an opened passage

Status: Anvil spec and implementation plan approved. Implementation waits for accepted SDK16.
One reviewer completed one spec turn and one focused recheck. The revision defines exact material quantities, geological exhaustion, and mixed-knowledge work previews.
Dependency: accepted SDK16 shared knowledge and its atomic state transport.
Target: additive public API 0.2.17 and protocol 10. Confirm the actual predecessor before integration.
Review budget: one focused Codex reviewer for this spec, then the same reviewer for its implementation plan.

## Player result

A worker excavates soil from a ridge over time. A heavier machine removes hard and ore-bearing material.
The removed natural material enters the authored item inventory or slot account exactly once.
The player opens a real passage and sends units through it.
Filling raises ground and consumes the authored material. Excavating placed fill cannot recreate natural ore.
The player can cancel work, see why work stopped, unload cargo, and resume work.
The default flat Skirmish remains playable. Its game rules and art do not change for this milestone.

This is M4, not the whole engine. Keep existing unit footprints, collision, physical arrival, queues, and authority.
Do not revive the old point-unit assumption or the old gate and fault-record machinery.
M4b remains responsible for the full terrain group-movement campaign and difficult chokes.
M5 still needs prospecting equipment, scanners, geological discovery records, terrain sight obstruction, and other advanced visibility rules.
Scripts, underground infrastructure, tunnels, overlapping ground surfaces, falling buildings, erosion, and creative instant edits are outside this slice.
The 2026-10-28 terrain draft is historical input. This spec replaces its stale API/protocol and player-control assumptions.

## Public authoring

A map can opt into a regular, chunked heightfield. An absent terrain declaration keeps the current flat rules.
The author declares a positive integer length scale per world unit, cell width and elevation-step length in those quanta, dimensions, chunk width, initial integer elevations, and height bounds.
Rendering converts the integer lengths to world coordinates. Material accounting never uses floating-point volume.
Defaults for the public fixture are 1 world unit per cell, 0.25 units per elevation step, and 16 by 16 cells per chunk.
These values are authored defaults, not engine constants. Boundary and height arithmetic use checked integers.
Validate complete authoring and checked allocations before installing any world state.
Reject nonfinite dimensions, invalid references, overlapping intervals, impossible height ranges, excessive grids, and allocation overflow with field paths.
Limit the grid to 1,048,576 cells. Declare and enforce a finite total interval-storage limit before installation.
Validate a public worst-case storage bound from cell count and height span, including placed-material runs. Hidden column complexity cannot choose an observable capacity refusal.
No single command can designate more than 256 cells. Larger work can use several ordinary orders.

Materials have public identities, visible surface appearance, hardness, allowed tool capabilities, natural removal outputs, and fill costs.
Work definitions declare operation, work rate, reach, supported materials, costs, and permitted destinations for outputs.
Any eligible unit can use a declared tool. Unit names and package names cannot grant behavior.
Movement classes declare terrain traversal limits. Units reference those classes through the public API.
Ground units with no terrain class can traverse equal-height ground only on an opted-in terrain map.
Existing packages with no terrain keep their previous movement and placement behavior.

A static unit declares its terrain foundation rule. The first supported rule is explicit ForbidEdit.
The engine must not silently choose a collapse, fall, or destruction policy.
A cell intersecting a live unit or unfinished site's footprint cannot change elevation in this slice.
This includes a worker standing on the cell. The worker approaches a valid work position beside it.
Surface infrastructure support must stay valid. Reject edits that would remove support for an existing surface belt, inserter, or anchored node.
The refusal describes the public occupied/support condition only when the viewer may observe the affected region.

New written fields require the new API floor, including explicit null fields under an older manifest.
Qualified references and library packages use the existing catalog/version rules.
The contract must support two structurally different public packages without an engine edit.

## Geological truth and knowledge

Store one ground surface and compact material intervals below it. There is no underground traversable volume.
Anchor intervals to a stable world datum. Lowering the surface exposes remaining intervals; it never moves a deposit upward.
Natural depletion persists. Filling adds placed material above what remains.
Initial natural intervals cover every elevation step from the declared dig floor to the initial surface, contiguously and without overlap.
Their endpoints use the integer elevation datum. Reject gaps and an initial surface outside that coverage before installation.
The dig floor is non-extractable bedrock. Reaching it stops work with an observed floor limit; no synthetic soil or ore appears.
The remaining natural top only moves downward. Placed fill stacks above that depleted top and cannot move its natural intervals.
Placed material has explicit provenance. Its initial supported removal policy is NoYield.
That policy is declared publicly. It is a stated capability limit, not an implicit conversion rule.

Initial elevations and public material/tool definitions may be public package content.
Actual hidden geological columns and a private generation seed must not enter a downloadable package, browser asset, advertisement, or human/bot snapshot.
Public generation rules can be authored in the mod. Generate the hidden realization on the server from a private match seed.
Record that seed in server replay/save provenance. It is not a client field.
Expose generation inputs through the public server/mod API, not a special Quarry generator.
A deterministic seeded generator must reproduce the same columns on the same build.
An authored public-geology mode can exist, but it must explicitly advertise that its strata are public.
A package cannot claim competitive hidden geology while distributing its actual columns.

The shared knowledge provider controls dynamic surface observations and exposed material facts.
Do not send hidden edits, buried deposits, or live material changes through an unrestricted terrain channel.
A previously observed surface can remain stale while it is unobserved. Mark that stale information clearly.
The work actor needs current observation of its work cell before a material step starts.
Exploratory digging remains possible because observing the surface does not reveal what lies beneath it.
Hidden and absent deposits with the same permitted surface must give equal queries, metadata, advice, and wire payloads.
A real authorized excavation may reveal newly exposed material. That is an observation, not a query leak.
Scanners and uncertain subsurface queries remain M5 work.

## Work orders and accounting

Excavate and Fill are ordinary typed, queued work orders. An order names a bounded cell rectangle and a target elevation.
An excavation lowers cells toward that elevation. Filling raises them toward it. It cannot cross the declared map height bounds.
The worker approaches reachable cells and performs authored work over simulation ticks.
Choose cells and work positions by geometry and permitted observations. A unit ID cannot select a visible winner.
Enumerate every cell in the submitted rectangle from public coordinates, independent of hidden material or occupancy.
Unknown cells remain explicit Unknown entries. Do not filter them, rank them, or change public counts/timing metadata from hidden truth.
A worker can approach an unknown designated cell through ordinary movement. It cannot start its material step until the cell is currently observed.
Until then, work status cannot reveal hidden eligibility, hardness, blockers, or refusal details.
Route advice uses permitted terrain; authoritative physical movement still checks real support and obstruction.
One worker owns an active cell operation. Later workers wait or select another designated cell.
The first worker that begins valid work keeps that operation until completion, cancellation, or loss of eligibility.
Simultaneous conflicting starts with equal physical precedence remain contested; do not break the tie with IDs or hash order.
A later command cannot steal an operation already in progress.

One material step is one complete cell-width by cell-width by elevation-step prism.
Its integer volume is cell_width_quanta squared times elevation_step_quanta, with checked u128 arithmetic.
Material output, fill input, and required work coefficients are public integer ratios per cubic world unit.
For a coefficient numerator/denominator, a prism amount equals volume_quanta * numerator / (length_scale cubed * denominator).
Check both products before division. The length scale changes precision, not material density.
All coefficients must produce exact, bounded integer amounts for one authored prism. Required work is strictly positive. Reject fractional or overflowing per-step amounts before installation.
Changing grid resolution therefore changes the per-step quantity by its actual prism volume, not by a fixed reward per cell.
A natural interval boundary cannot split an elevation step. The step removes exactly its exposed material.
Check tool eligibility and hardness for that exposed material, not the next buried interval.
The declared hardness/work coefficient gives the required integer work for the prism.
The tool contributes its authored work units per second through the shared integer rate-carry rule at the match tick rate.
A step commits when its accumulated work reaches that requirement. No input charge or output credit occurs earlier.
Retain partial work and fractional rate carry on ordinary pause/resume. Cancellation discards uncommitted work and carry, not resources or committed edits.
Work does not transfer to a competing worker after cancellation. The next operation starts fresh.

Each material step is one authoritative transaction: validate permission, occupancy, capability, destination capacity, and costs; then commit height, interval/provenance, resource changes, and revision together.
Two overlapping orders cannot remove or credit the same natural step twice.
The transaction cannot leave a height change without its corresponding account change, or an account change without the edit.
Use shared cost, progress, and refund rules. Keep checked arithmetic and all-or-nothing capacity checks.
If the output will not fit, retain the material and elevation. Show Cargo full and pause that step.
If fill inputs are missing, retain the material and elevation. Show the missing-input condition without inventing a payment.
Unloading uses an ordinary capability and the existing physical transfer rules. It cannot teleport items or turn them into another resource.
A player must have a visible way to unload and resume the designated work.

Cancellation stops future work and releases uncommitted reservations. Committed edits and credits remain.
Queued work revalidates current permission and conditions when it starts.
Ownership loss, worker destruction, disable, blocked reach, and a disappearing destination cannot duplicate or destroy material.
Public work status exposes progress, current cell, target level, and an actionable refusal/wait reason to its owner.
Foreign private accounts and inventories remain private. Bots use the same public work and terrain contracts as humans.

## Navigation, collision, placement, and revisions

An accepted edit changes simulation terrain before dependent movement or placement can use that region again.
Update affected cells/chunks and local neighbors. Do not rebuild the entire map for one cell edit.
Invalidate dependent route and placement caches by map identity, movement class, and affected revisions.
A route cannot retain permission to cross a new cliff or an occupied footprint after an edit.
A blocked order stays visible. It can resume after a relevant edit creates a valid route.
No-route work must not spin through an unbounded search every tick.
Use bounded admission/search work and expose waiting status. Do not silently drop the command.

Traversal checks use the unit's full radius/footprint and authored terrain class.
Keep local collision and physical arrival rules. Units cannot stack, cross cliff faces, or squeeze through a gap narrower than their body.
Evaluate swept movement, not only the destination cell, so a fast unit cannot skip a blocked cell.
Height-step and slope limits must have one shared pure interpretation on both targets.
Same-build replay, reversed entity creation, and changed viewer connection order must preserve edits, accounts, routes, and physical results.
A smaller worker and a larger rover must demonstrate different authored access limits.

Placement rechecks support, footprint flatness, current terrain, and ordinary blockers.
Advice uses permitted terrain and the complete rule-dependent region. Unknown terrain produces Unknown advice.
Server admission revalidates truth. Hidden and absent reasons use the same generic refusal boundary.
The ForbidEdit rule protects finished structures and unfinished sites immediately.

## Browser and wire behavior

Terrain is real height-aware ground. Camera picking, work ghosts, selection, unit placement, and model feet use that ground.
Render affected chunks and exposed material surfaces. Keep the current model, texture, shadow, and animation quality.
Show work progress and a short status. A refusal must appear within one second.
Use mod-defined command labels and capabilities. Do not hardcode Quarry controls into gameplay code.

The work flow has two steps: choose a reference elevation on observed ground, then drag the work rectangle.
The preview shows the target elevation, designated cells, operation, and permitted cost information before submission.
It uses designation coordinates and permitted observations only. Unknown cells stay marked Unknown; previews never inspect hidden truth.
The ordinary queue modifier can append a work order. Stop and Cancel remain visible and work through the normal order API.
The fixture must not require repeated single-cell clicks to open its passage.
A first-time player gets a short visible goal and contextual control hints. Do not open a long guide over the world.

Replicate only permitted terrain, exposed material, and work facts through the existing bounded atomic transaction mechanism.
Terrain rows, visibility/memory, unit positions, and resource changes activate as one completed basis.
No client frame can show a resource credit paired with the old authoritative terrain revision.
Snapshots, large edits, resync, cancellation, reconnect, and slow viewers retain SDK16's memory/frame/timeout/isolation guarantees.
Do not add an unbounded terrain queue or send actual hidden columns in a full snapshot.
Client caches clear on match/map reset. Stale old-map edits cannot change a new map.

## Public fixtures

Quarry uses physical cargo, natural ore, a worker, a larger hard-material machine, a depot, and a ridge separating a destination.
The player excavates a passage, unloads material, crosses the new route, and fills an authored test area.
Use existing documented art where suitable. New mesh/texture polish is not a dependency of this engine slice.
A second renamed package uses different materials, tools, terrain dimensions, traversal classes, and slot-account outputs.
It must use the same public engine API without package-specific code.

## Acceptance

Record one report with commands, results, and evidence paths. These are product checks, not a new gate framework.

| Row | Required result |
| --- | --- |
| TW1 | Both packages load through public authoring. Cover API floors, explicit nulls, defaults, library references, checked dimensions/storage, malformed intervals, invalid materials/classes/tools, and atomic rejection. Old flat packages retain their accepted gameplay. |
| TW2 | Excavate and fill through ordinary orders and real ticks. Check exact natural yield, stable datum, persistent depletion, placed provenance, missing inputs, full cargo, cancellation, disable, worker loss, and destination loss. Conservation holds after every committed step. Include changed cell resolution, interval boundaries, invalid gaps, initial-surface mismatch, bedrock exhaustion, and repeated fill/removal. |
| TW3 | Concurrent overlapping orders, contested starts, reversed IDs, and repeated command replay cannot duplicate material or select an ID-based winner. Equivalent item and slot-account packages obey their declared destinations. |
| TW4 | A real ridge blocks the original route. Local edits open it. Radius-aware worker and rover routes use the passage. New cliffs and occupied cells stop swept movement. A pending blocked order resumes after a relevant edit. Unrelated chunks retain their navigation/render revisions. Placement and foundations use current permitted terrain. |
| TW5 | Capture real human wire and public bot queries for hidden-present/absent geological worlds. Payloads and metadata are equal until permitted exposure. Inspect public archives, assets, advertisements, snapshots, deltas, resync, and reconnect for hidden columns/seed leakage. Include mixed observed/unknown rectangles and compare preview, enumeration, selection order, status, counts and metadata before exposure. Check stale terrain, atomic terrain/account activation, interrupted large updates, caps, and viewer isolation. |
| TW6 | Fresh silent browsers use real pointer/key input to designate work, see progress, pause for full cargo, unload, resume, open a passage, move the rover through, fill, cancel, and see refusal/status. Check height-aware picking and ghosts at both zoom extremes. Final cold-play scripted probes pass. |
| TW7 | Measure before declaring capacity. On four selected CPUs, run 100 working tools among 1,000 units with navigation, combat, fog, human/bot projection and two client activations. Record full 20 Hz service/debt, work counters, traffic, memory, and local invalidation. Use the reviewed SDK16 deadline criteria, with a fixed representative terrain/workload declared before timing. Require at least 30 FPS for the unchanged 400-unit detailed browser scene on editable terrain at both zoom extremes. Preserve failed evidence; optimize code, not the workload or quality. |

Before TW7, freeze the exact terrain, unit/tool mix, edit rate, changed-chunk count, routes, work counters, warm-up, sample count, resync schedule, and raw timer attribution in the implementation plan.
The foundation reviewer runs code after the build. Fix findings before acceptance and integration.
Merge and push only after acceptance, the README progress update, and final scripted player probes.
