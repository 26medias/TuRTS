# M6: underground utilities

Status: approved. Gate 1 BLOCK (12 findings) and gate 2 PASS-WITH-FIXES (11 findings) are repaired here. One reviewer: `world-engineer` (owner of spec 11).
Base: accepted main `fc16df8e` (API 0.2.17, protocol 10). Branch `codex/m6-underground-utilities`.
Target: branch-local API **0.2.18** and protocol **11**. Root reconciles these numbers with its API19 work.
Source of record: spec sections 9.4, 11, 14 (row M6) and D13, D14.

## Player result

The player turns on the utility view. The terrain fades. Own power lines, junctions and building ports show
in mod colours, with a label for each state.
The player selects a builder, chooses "Power line" and clicks two points. A line goes in at a fixed depth.
A line that crosses another line in plan view does not connect to it. The consumer stays "No power".
The player draws a short route from one line to the other. That route makes two explicit junctions.
The consumer now shows "Powered" and its production runs at the full rate.
A worker fills ground over a line. The line gets deeper. Its elevation does not change.
A worker digs down to the line. The line is exposed, and the mod policy applies (the fixture: Disconnect).
The consumer shows "No power" and its production slows. Filling the cell again restores the connection.
With the view off, normal play is unchanged.

The economy-log audit (`turts_audit`) has no replay rule for `UtilityRoute` or for Network shortage in this
milestone. It refuses such a log with its existing "no replay rule yet" error. This is a stated limit.

Out of scope: water pressure, sewage gradients, heat losses, fluid physics, storage, several underground
floors, underground units, tunnels, a full network editor, segment removal by the player, bot use of
utilities, and map-authored initial lines. Corridor sharing rules and clearance between lines are not
checked: lines of any type may cross or run together. These are stated limits, not hidden defects.

## Public authoring (API 0.2.18)

A new manifest export `utilities: [...]` lists utility type files. A utility type declares:

| Field | Meaning |
| --- | --- |
| `id`, `name` | Content id and display name. |
| `carries` | A `Supply` resource with the new scope `Network`. Each Network resource has exactly one utility type. |
| `ports` | `[(unit, offset:(x,y))]`. One port per unit type and utility type. The unit type has no `movement`. The offset is in world axes (it does not turn with facing), within the footprint radius plus 2 world units. |
| `depth` | `(default, min, max)` in terrain elevation steps. `1 <= min <= default <= max <= 64`. |
| `reach` | Largest plan distance from an issuing unit to each route endpoint, in world units. |
| `max_segment_length` | Largest plan length of one segment, in world units. `0 < length <= 64`. |
| `segment_cost` | `[(resource, amount)]` of `Balance` resources, charged once for each new segment. Can be empty. |
| `exposure` | `Operate`, `Disconnect` or `Remove`. The engine has no default policy. |
| `appearance` | `colour`, `exposed_colour`, `junction` symbol (`Circle`, `Square`, `Diamond`), and `labels` (`powered`, `short`, `unconnected`, `exposed`). |

`SupplyScope` gets the variant `Network`. Unit `provides` and `consumes` keep their meaning, but a Network
resource counts per connected network, not per slot. A unit that provides or consumes a Network resource must
have a port of the utility type that carries it. Producer and weapon `when_short` entries can name a Network
resource only when the unit type has a port of the utility type that carries it. Constructor `when_short`
cannot name a Network resource (a site has no port). The packer refuses each of these cases.
A Network resource never enters slot totals, `supply_rows` or `SupplyRecord`.

A new order behaviour `UtilityRoute` with target `UtilityRoute`. The order's `units` list names the builders.
A new HUD list source `UtilityRoutes(order)` lists the utility types for the selection.
Validation uses field paths and `did you mean`. Every new field and variant has a version-table row with
`since_api: "0.2.18"`. A manifest that admits an older API and uses them is refused. Old packages load and play
as before.

## Topology

One logical underground layer for each slot. It holds junction nodes and segments.
A segment joins two endpoints of the same utility type and owner. An endpoint is a junction or a port.
A port exists only while its unit is a completed, live unit of the owner.
Plan-view crossings never connect. Only a shared endpoint connects two segments.
Node and segment ids are counted per slot. A slot cannot learn the number of another slot's placements.

A route command names a utility type, a requested depth and two explicit endpoints:
- `Port(unit)`: a port of an own completed unit.
- `Node(id)`: an own junction of that utility type.
- `Split(segment, point)`: an own segment of that utility type. The point is within 0.25 world units of the
  segment and more than 0.5 world units from both of its ends. The segment splits into two segments at a new
  junction. Both halves keep the elevation and state of the old segment. A split is free.
- `Free(point)`: a new junction at that point.

The server does not snap a free point to a near node. The client snaps for the player before it sends.

## Placement validation

The command is applied when the server admits it. It does not enter the unit order queue. `Append` acts as
`Replace`. The server checks, in this order, with one refusal reason:

1. Envelope: own, live issuing units of a type in the order's `units` list (existing rules).
2. The utility type exists in the catalog. Depth is inside its `depth`.
3. Endpoints exist, belong to the slot, have the right utility type, and are inside the map. A foreign or
   absent id gives the same refusal `TargetInvalid`.
4. The two endpoints differ. Their plan distance is above 0.5 and at most `max_segment_length`. No other
   segment joins the same two endpoints.
5. At least one issuing unit is within `reach` of both endpoints. Units that are not within reach are refused
   per unit with `OutOfRange`. This rule never uses a unit id to choose a result.
6. The touched cells are the cells whose closed square meets the segment (`turts_rules::utilities`).
   More than 256 cells gives `UtilityLimitReached`. Every touched cell is currently observed by the slot,
   in the Completed observation basis of the previous tick (what the client saw). Else `UtilityNotObserved`.
   The fixed elevation (below) is not under the terrain `floor`. Else `OutOfRange`.
7. Slot limits: 512 junctions and 512 segments per slot, new junctions and splits included. Else
   `UtilityLimitReached`.
8. The slot balance covers `segment_cost`. Else `InsufficientBalance`.

Elevation is fixed at placement: the lowest surface elevation over the touched cells, minus the depth.
A map without terrain uses surface 0 and 1-unit cells.
A split keeps the elevations it had. Nothing changes the elevation later. Terrain edits never move a line.
The payment, the new nodes, any split and the segment commit in one step. A refusal changes nothing.

## Burial and exposure

The cover of a segment at a cell is the surface elevation of that cell minus the segment elevation.
A segment is exposed when any touched cell has a cover of 0 or less.
`TerrainState` records the cells that its edit commit changes (next to the dirty-chunk bits, written by
`apply_prevalidated` and cleared by `terrain::begin_tick`). In the utility pass of each tick, the engine checks
the segments that touch those cells, plus every segment that a placement or a split made in that tick. It
checks no other segment. An operator-only counter records the number of checked segments. Then:
- `Operate`: the segment is marked exposed and still conducts.
- `Disconnect`: the segment is marked exposed and does not conduct. It conducts again when every touched
  cell has a cover above 0.
- `Remove`: the segment is deleted. Junctions with no segment left are deleted.

`Remove` gives no refund. Damage on exposure is not a policy in this milestone.
When a port's unit is removed, each segment at that port keeps its geometry. Its end becomes a new junction,
in ascending segment id order. When the junction limit is full, that segment is deleted instead.
Terrain work keeps its current rules. A line under a cell does not block excavation or fill.

## Operating model: aggregate supply

One system set `SimSet::Utilities` runs after `Outcome` in each tick. It handles port removal, then the
exposure check, then the networks. Connectivity is rebuilt only for the (slot, utility) pairs whose topology
changed in that tick; totals are summed for every network from the live port units.
The engine builds the connected networks of each slot and utility type.
Conducting segments are edges. Junctions and ports are vertices. A port unit with no conducting segment is a
network of one unit.

For each network, provided `P` is the sum of the Network resource in `provides` of its live completed port
units. Consumed `C` is the sum in `consumes`. Sums use checked `u128`.
The network is short when `C > P`. Then every consumer on that network gets its authored `when_short` effects.
When `C <= P`, no consumer is short. There is no storage, no loss and no carry between ticks. The delivered
amount is `min(P, C)`. Unused supply is lost.
All consumers on one network get the same result. No unit id, entity order or hash order chooses a consumer.

The pass of tick `T` writes a `next` state. `run_supply` at the start of tick `T + 1` swaps it in as the
`effective` state. Every system of tick `T + 1` (acquire, production, combat) reads `effective`. The network
rows published at tick `T` come from `effective` of tick `T`, the same buffer as the published producer rates.
Topology rows are current. So a network row can lag a new segment by one tick.
A port unit with no row in `effective` (it completed after the last pass) is a network of its own for that tick.
Every tick, the totals step enumerates all live completed port units, so a port unit with no segment always
has its own network row.
Slot-scope supply rules do not change.
This model is electricity only. It is not water pressure, sewage or heat.

## Privacy and wire (protocol 11)

`SnapshotRecords` and `DeltaRecords` get `utilities: Option<UtilityView>`. In a delta, `Some` replaces the
whole view and `None` means no change. The view rides the existing atomic state transaction.
The view for a viewer holds only the viewer slot's own junctions, segments, network rows and port units.
It holds no foreign row, count, id or state. Bots get no utility view in this milestone.
A segment record holds its id, utility, endpoints, elevation, depth and state (`Conducting`, `Exposed`,
`Disconnected`). It holds no surface or cover value. The client computes cover from its own permitted
terrain. The exposure state of an own line is own-asset state, the same as damage to an own building.
Decision (rule, not a defect): the owner learns one bit for each own line, "the surface at a touched cell is at
or below the line", also under fog, from the segment state and from its consumers. No policy hides this bit.
A foreign line, exposed or not, is never sent, drawn or accepted as a target.
A network row holds utility, port units, `P`, `C` and short.
Two new refusal reasons: `UtilityNotObserved` and `UtilityLimitReached`.
Snapshot `utilities: None` means the catalog has no utility type. With utility types, every snapshot holds
`Some`, also an empty view. A delta holds `Some` only when the viewer's own view differs from the last one sent.
The view size is bounded by the slot limits. It is far below the existing decode-memory cap.

## Browser mode

An engine input action `utility_view` (default key `U`) turns the view on and off. A HUD button can do the same.
In the view, terrain tiles use a blended copy of their material at alpha 0.35. Units and buildings stay solid.
Segments are lines at their fixed elevation, in the mod `colour`. Exposed segments use `exposed_colour`.
Disconnected segments are dashed. Junctions use the mod symbol. Ports are squares on the ground.
A port marker is green when its network is not short, red when it is short, and grey with no segment.
Hover in the view shows a panel: utility name, mod state label, network `P` and `C`, depth, elevation, and
the cover at the hovered cell from permitted terrain ("cover unknown" when the client has no surface row).
Route mode: choose the utility type in the HUD list. The view turns on. Left click sets the start, left click
sets the end and sends the route. The end becomes the next start. `[` and `]` change the depth.
The preview shows length, cost, depth and a refusal reason when the client can know it. Right click or Escape
ends route mode. A server refusal shows within 1 s.
The client snaps a click to an own port, then an own junction, then an own segment (split), within 0.6 world
units of the ground hit. The client draws nothing it did not receive from its own view.

## Public fixture

A new package `utility_yard`, from `terrain_stockyard` art, models and credits, unchanged. It adds a generator
(`provides` 10 power), a workshop (`consumes` 6 power; producer `when_short` Rate 1/4), a depot builder, the
`power_line` utility type (`exposure: Disconnect`, depth 1..4, default 2, cost 1 loam per segment), and a map
with one ridge. The existing workers dig and fill. No new art is generated. Skirmish and the terrain packages
are unchanged.
Player-path note (found during the build): a work order digs only down to an observed reference level. So the
demonstration exposes the depth-1 junction route that lies on the ridge by digging that ridge cell down to the
flat level, and then fills it back to the ridge level.

## Acceptance

One report: `docs/reports/2026-11-16-underground-utilities.md`. Evidence under
`target/mc-evidence/m6-utilities/`. Launcher: `target/mc-evidence/play-utilities.sh`.

| Row | Required result |
| --- | --- |
| UT1 | `utility_yard` loads through public authoring. Refusals with field paths: bad depth range, length, unknown or wrong-kind `carries`, a Network resource with no or two utility types, a provider or consumer with no port, a producer or weapon Network `when_short` with no port, a port on a mobile unit, a duplicate port, a constructor Network `when_short`, and new fields under an older `engine_api`. Old packages keep their accepted tests. |
| UT2 | Real ticks: two crossing segments do not connect; a split route connects them; the consumer goes from short to supplied in the next tick. Port removal makes a junction. |
| UT3 | Fill and excavation by ordinary paid work orders change cover but not elevation. Each policy (`Operate`, `Disconnect`, `Remove`) acts as declared. Disconnect recovers after fill, also when the hole is under one half of a split segment. The checked-segment counter shows that segments on unchanged cells are not checked. |
| UT4 | Refusals with no state change and no charge: foreign or absent ids, wrong utility type, non-builder, out of reach, too long, zero length, duplicate pair, unobserved cell, slot limit, low balance, depth out of range. Foreign and absent ids give identical results. |
| UT5 | Accounting (same-build replay compares the full utility state): `P`, `C` and short are exact for chains, splits and several networks. A generator killed in combat at tick `T` supplies no consumer in tick `T + 1`. Reversed unit creation and reversed command order give the same networks and effects. Same-build replay gives the same state. Slot-scope supply results do not change, and no Network resource is in `supply_rows` or `SupplyRecord`. |
| UT6 | Two-viewer privacy: paired worlds differ only in slot 0's utility state, and slot 0's consumers are outside viewer 1's sight (no lines against many lines, then slot 0 edits, exposure and port removal). Viewer 1's encoded snapshots and deltas are byte-equal across the pair. Viewer 0 gets only its own rows. A slot-1 command that names a slot-0 id gets the same result as an absent id. |
| UT7 | Bounded work: the real pass with `SLOT_COUNT` (2) slots at the limits (512 segments and 512 junctions each), with 16 edited cells under lines each tick; and a synthetic harness of the same pass functions over 8 slots. Record utility pass time (p50, p99, max) and view bytes. This is an isolated measurement, not an M7 claim. |
| UT8 | Fresh silent browsers with real pointer and key input: view on, terrain fades, crossing stays short, split route connects, consumer powered, fill keeps elevation, dig exposes and disconnects, fill restores, view off and normal selection and move. Both zoom limits. Final cold-play scripted probes pass on the final build. |

Budgets (UT7, representative, recorded against these values): utility pass p99 at most 2 ms; one view at most
64 KiB encoded. A miss is reported, not hidden.
