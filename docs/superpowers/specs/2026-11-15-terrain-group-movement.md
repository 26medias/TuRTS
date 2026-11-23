# M4b: group movement on editable terrain

Status: Anvil spec and plan review complete. M4 acceptance remains a separate prerequisite for integration.

Amendment 2026-11-23 (flock model): the player rules marked "Decided 2026-11-23" (operator, through Iris), the GM1 to GM4 text changes, the rows FM1 to FM3 and the retired sections come from section 6 of the design record `2026-11-23-flock-movement-design-record.md` (Viktor, T-6). Iris approved them in task T-7, with one change: FM2 is 60 s, not 120 s. GM1 keeps 120 s. GM5 to GM8 are unchanged.

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

A group order targets an area around the point. The first unit keeps its place in the crowd. Settled units shift a short distance to make room. A unit of a later order may push a settled idle unit. The unit id never decides. (Decided 2026-11-23.)
A unit that touches the settled crowd of its own order has arrived and stops. (Decided 2026-11-23.)
An arrival or clearance result must never depend on unit IDs, entity allocation or iteration order.
A unit of a later order may push a settled idle unit. A unit on Hold is never pushed. (Decided 2026-11-23.)

Routing, steering, collision and final arrival use the complete authored circular footprint.
They use the same exact plateau support and swept-traversal rules as M4.
A wide unit cannot borrow a narrow unit's clearance result.
Diagonal motion cannot cut a cliff corner. A fast step cannot cross an unchecked wall.

A moving unit pushes an idle friendly unit aside. The pushed unit stays where the push leaves it. Never a unit on Hold, an enemy or a building. (Decided 2026-11-23.)
Moving units may touch and overlap a little for a short time in a dense stream. At rest each unit stands apart. Walls, cliffs, buildings and enemies stay solid. (Decided 2026-11-23.)
Exact simultaneous physical conflicts need a defined geometric or contested outcome.
An ID or a hidden material field cannot break the tie.

If enough space and a valid route exist, a finite group must make bounded progress through a choke.
A unit that cannot pass goes to the obstacle and waits at the side of the flow, visibly marked. A reason text needs a wire change and stays out of M4b. (Decided 2026-11-23.)
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
Units move at once and correct the route during the move. (Decided 2026-11-23.) Guidance is advice. The step check refuses any step into a wall.
One expensive request cannot prevent other eligible movement from receiving service indefinitely.
Bounded work must include cancellation and invalidation, not only successful searches.

Planning reads the viewer's permitted current terrain, stale memory and public initial geometry.
Execution checks each actual step against terrain, buildings and foreign bodies before committing it.
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
| GM1 | Both public packages load. A group of24 small units and12 large units receives one ordinary point order. All eligible units cross a sufficiently wide passage and settle without overlap or cliff penetration within120 simulated seconds. The first unit moves within 1 s of the order. Repeat with reversed allocation and command-member enumeration. |
| GM2 | A narrow opening admits small units and refuses the large footprint. The small units finish within the same120-second bound. Large units go to the obstacle and wait at the side of the flow, visibly marked. Opening a sufficiently wide passage resumes the blocked large units within 2 s without a replacement player order. |
| GM3 | Two groups share a destination; two groups cross opposing routes; one unit changes to a unique destination. Cover a movable obstruction and a permanently occupied goal. Prove progress when the authored fixture permits it. Prove stable safe waiting for the impossible fixture. The first arrival keeps its place in the crowd. Results are equal under ID reversal. |
| GM4 | Ordinary excavation/fill closes and reopens a route while units move. Check that no committed step crosses a wall, cliff, building or foreign body, relevant invalidation, stale-result rejection and unchanged distant route revisions. No ID-based winner. No overlap at rest. |
| GM5 | Replace, Append, Stop, death, owner change and a new match release the correct route memberships. Repeated shared and unique orders have bounded retained memory. Same-seed command replay reproduces physical outcomes. Captured audit traces certify the supported transitions and reject forged movement. |
| GM6 | Paired hidden terrain worlds produce equal permitted route/query/status metadata until observation or physical contact. Exercise two viewers and connection-order reversal. A private route cache cannot become an information channel. |
| GM7 | Run the unchanged frozen M4 native workload with the completed group movement path. Keep its1,000 units,100 working tools, both visibility modes, ACK delays, sample counts and deadline criteria. Record whole service, debt, actual movement, edits, route reuse, neighbor work, invalidation, traffic and retained memory. No workload reduction or quality reduction is allowed. |
| GM8 | Fresh silent browsers use actual selection and pointer orders to pass GM1-GM4. The unchanged400 detailed models retain at least30FPS at normal and both extreme zooms during group movement and terrain work. Final scripted cold-play probes pass before a CEO play request. |
| FM1 | First motion (ridge packages, 36 mixed units). The first unit moves within 1 s of the order. Every unit with free space in front of it moves within 1 s. |
| FM2 | Return crossing (Jade's case): 36 units go back through the dug passage. All are settled and apart within 60 s of the return order (Iris, 2026-11-23). No unit has no motion and no settled state for more than 5 s. |
| FM3 | Overlap while moving. The maximum overlap depth is less than 25 % of the smaller radius. No overlap at rest. |

Record commands, exact fixture hashes, raw results and remaining limits in one milestone report.
One independent foundation review must run the assembled code. Fix its findings.
After acceptance, update README progress and merge/push under the CEO's standing authorization.

## Exact traffic-index metadata bound (2026-11-16 amendment)

Retired 2026-11-23 (traffic-index bound; old path). Source: design record `2026-11-23-flock-movement-design-record.md`, section 6; approved by Iris in task T-7. The text at 554775ac stays in git history.

## Local routes around disclosed bodies

Retired 2026-11-23 (kept only for the stationary-obstacle rule below). Source: design record `2026-11-23-flock-movement-design-record.md`, section 6; approved by Iris in task T-7. The text at 554775ac stays in git history.

An ordinary Move can route around a foreign body whose current pose is permitted at the immutable Start observation phase.
Explicit AllVisible has no restricted observation basis. Its immutable pre-movement world borrow supplies the public body facts for this planning call, guarded by the current Start terrain phase. A missing or mismatched restricted basis never selects this fallback.
Use its captured position and public content-defined footprint. Do not read foreign orders, targets, progress, or current private motion to select a route.


## Cooperative certification of friendly movement

Retired 2026-11-23 (cooperative certification; old path). Source: design record `2026-11-23-flock-movement-design-record.md`, section 6; approved by Iris in task T-7. The text at 554775ac stays in git history.


## Bounded preferred-velocity steering

Retired 2026-11-23 (steering, coherence and staging of the old path). Source: design record `2026-11-23-flock-movement-design-record.md`, section 6; approved by Iris in task T-7. The text at 554775ac stays in git history.


## Packed stop behind the holder (operator decision 2026-11-22)

Retired 2026-11-23 (packed stop; replaced by player rule 3). Source: design record `2026-11-23-flock-movement-design-record.md`, section 6; approved by Iris in task T-7. The text at 554775ac stays in git history.

### Stale physical escape (defect repair, no rule change)

Retired 2026-11-23 (stale physical escape; old path). Source: design record `2026-11-23-flock-movement-design-record.md`, section 6; approved by Iris in task T-7. The text at 554775ac stays in git history.


### Work limit of refused direct lines (GM4 repair, no rule change)

Retired 2026-11-23 (work limit of refused direct lines; old path). Source: design record `2026-11-23-flock-movement-design-record.md`, section 6; approved by Iris in task T-7. The text at 554775ac stays in git history.


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
