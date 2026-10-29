# M4 — Terrain and geology: design

Date: 2026-10-28. Status: DRAFT for gate 1. Calls K1 to K10 wait for the user (section 1.1).
Source of record: `RTS-Engine-Project-Specifications.md` v0.7 (cited as S:line). The M3 design
(`docs/superpowers/specs/2026-10-27-m3-industrial-design.md`) is cited as M3D:line.
Research input: `docs/research/m4/` (engine, content, seams). It is input, not a contract.

## 1. Contract

### 1.1 Goal, scope and the calls

S:679 gives the M4 demonstration: "Excavate ore-bearing ridge and use opened passage".
Acceptance evidence: "Correct resource yield, depleted deposits stay depleted, navigation updates
locally". This design makes both true, and it adds the first human-readability gate (K1).

In scope: a heightfield; geology columns; `Excavate` and `Fill` commands; grid navigation with
local invalidation; one static-unit foundation rule; wire protocol 4; a terrain client with
height-aware picking, drag-select and readable labels; a new package `quarry`; an oracle, 13
cases and 13 mutation plants.

Out of scope: discovery and secrecy (M5), underground utilities (M6), scale and flow fields (M7),
player construction, line of sight, unit clearance and collision, instant edits, smooth terrain,
sound, art assets, a minimap, hotkeys, control groups, a Fill user interface.

| Call | Decision | Default if the user does not object |
| --- | --- | --- |
| K1 | M4 adds drag-select (box selection), one group `MoveTo`, and a **player-control gate**: a person must finish the demo with mouse and keyboard only (section 9, F9). | yes |
| K2 | The demo is a new package `quarry`, not a second map in `forge`. Reason: `load_package` accepts exactly one map per package (`crates/turts_pack/src/lib.rs:16-23`), and the M3 fixed point depends on `proving_ground` (M3D section 13). | yes |
| K3 | Engine API 0.2.1 to 0.2.2, additive with `serde(default)`. A bump to 0.3.0 would break the `^0.2` and `^0.2.1` manifests of acme, base, cedar, haul and forge. | 0.2.2 |
| K4 | Wire protocol 3 to 4. | yes |
| K5 | Grid numbers: cell 1.0 world unit, height step 0.25, chunk 16 x 16 cells. Proposals; S:436 and S:706 stay Open. | these numbers |
| K6 | Placed fill yields nothing (M4 prototype limit). It does not close the Open policy on placed-material yield (S:480, S:542). | yes |
| K7 | A dig step that does not fit in the inventory does not commit (`CargoFull`). The surplus is never destroyed. | block |
| K8 | A unit with no route keeps its order. It re-searches after the next terrain edit. | keep |
| K9 | Foundations: `ForbidEdit` is the only value. A static unit blocks edits of the cells under its footprint (S:470 forbids a silent policy; this is a labelled M4 limit, like M3 `ContentsPolicy`). | yes |
| K10 | Heights and the top material of each cell are public. Geology below the top never goes on the wire. Units are points; clearance is deferred (S:459). | yes |

Decision IDs that bind M4: D09 to D14 (heightfield, no tunnels, buried deposits stored apart,
discovery later), D63 (navigation: Baseline), D15 (versioned API), D18 (authoritative server).

### 1.2 Verified baseline (path:line from HEAD 5461486)

- A position is `Position(Vec2)`, a target is `MoveTarget(Vec2)`: `crates/turts_sim/src/world.rs:21,33`.
  Movement is a straight line, with no blocking: `movement.rs:42-49`. `MoveTo` checks only the map
  rectangle: `command.rs:287-292`. `MoveTo` carries a list of unit ids, up to a maximum:
  `command.rs:257-294`.
- Tick order: Movement, Gather, Extract, Transport, Inserters, Production, Combat:
  `plugin.rs:89-101`. Plant C15 anchors on the exact text `SimSet::Production,\n SimSet::Combat,`.
- `WriteSet` overlays, `preflight`, `apply_overlays`, `extract_from_node`:
  `economy/transaction.rs:218-245`, `transaction_m3.rs:48,172,284-351`.
- `MapDef` and `Exports` use `deny_unknown_fields` (`schema.rs:56-66,455-464`). The additive pattern
  is `serde(default, skip_serializing_if)` plus a `was_present` entry in `MapPresence`
  (`content_m3_validation.rs:637,707-720`).
- `NET_PROTOCOL_VERSION = 3`: `crates/turts_net_protocol/src/lib.rs:13`; pinned in two tests only:
  `tests/wire.rs:23`, `tests/protocol3.rs:17`.
- The client ground is a flat plane: `render.rs:348`; `ground_point` uses the plane y=0:
  `input.rs:14-21`; `Selection(pub Option<u32>)`: `economy_ui.rs:31`.
- A gatherer unloads at a drop-off; there is no deposit command: `mods/haul/units/worker.ron`,
  `messages.rs:55-77`. So a depot is out of M4.
- NOT TRACED (a plan task must trace each before code depends on it): (a) whether the audit
  exemption for the movement phase (`economylog.rs:579-711`) covers a new resource; (b) whether
  economy events reach clients; (c) the maximum lightyear message size; (d) the full count of
  `MapDef { .. }` struct literals.

### 1.3 Rules that carry over from M3

One subject, one tick, one `transact` (M3D:454). The oracle parses the package itself and does not
read the catalog of the simulation. Every check has a named wrong build that turns it red. A map
without `terrain` behaves exactly as in M3, and the frozen M2 and M3 operation streams stay
byte-identical.

## 2. Schema (API 0.2.2)

All additions use `serde(default, skip_serializing_if)`. A package that uses any of them must
declare an `engine_api` range that excludes 0.2.0 and 0.2.1 (the M3 version-honesty rule,
`content_m3_validation.rs:503-515,639-653`).

```ron
MapDef.terrain: Option<TerrainDef>
TerrainDef(
    step: 0.25,                       // world units per height step
    base: 0,                          // default height in steps
    heights: [HeightRect(min, max, height)],   // applied in list order
    columns: [ColumnRect(min, max, column)],   // geology column per rect, list order
    zones:   [ZoneDef(name, min, max)],        // named rectangles for labels and outlines
    guide:   ["text", ...],                    // static player instructions, shown in the panel
)
CellPos(x: u16, y: u16)
Exports.materials: [path]   // MaterialDef files
Exports.columns:   [path]   // ColumnDef files
MaterialDef(id, name, yields: [ItemStack], dig_ms_per_step: u64, diggable: bool)
ColumnDef(id, intervals: [IntervalDef(material, top: i16, bottom: i16)])   // absolute steps
UnitDef.excavator: Option<ExcavatorDef(reach: f32, fill: Option<FillDef(material, consumes: [ItemStack])>)>
UnitDef.foundation: Option<FoundationDef(edit: ForbidEdit)>
MovementDef.max_climb: Option<u16>   // height steps per cell edge
```

Validation (each error names package, file and field, as in M3):

- Cell size is fixed at 1.0 in M4; `step` is finite and above 0; rects are inside `ceil(size)`.
- Intervals in a column do not overlap and `top > bottom`. Elevations are **absolute steps above the
  datum** (S:478). Below the lowest interval is an implicit undiggable floor.
- A package that has `terrain` gives `max_climb` to every unit with `movement`. A static unit
  (no `movement`) on a terrain map has `foundation`, and the cells under its footprint are flat
  (all the same height). The unit centre cell decides its footprint cells (radius rounded up).
- `ExcavatorDef` needs an `inventory`. `FillDef.material` is a material with `yields: []`.
- Content names from `mods/quarry` must not appear as identifiers in engine code
  (`turts_checks/tests/no_content.rs`).

## 3. Terrain and geology

### 3.1 State

`Heightfield { width, height, step, heights: Vec<i16>, chunk_version: Vec<u32>, edit_epoch: u32 }`
and `Geology { cells: Vec<CellGeology> }` are simulation resources. They exist only when the map has
`terrain`. Chunks are 16 x 16 cells. `CellGeology { natural_top: i16, placed: Vec<MaterialId> }`
with the invariant `heights[c] - natural_top == placed.len()`. `natural_top` never rises.

### 3.2 Operations

- Excavate one step of cell c: if `placed` is not empty, pop one placed step (it yields nothing, K6).
  Else the interval that holds elevation `heights[c] - 1` gives its `yields`; `heights[c] -= 1`;
  `natural_top = heights[c]`. A cell at the floor, or whose next material is not `diggable`, cannot
  be dug (`Undiggable`).
- Fill one step: push the fill material; `heights[c] += 1`. `natural_top` does not change. The
  excavator pays `consumes` from its inventory in the same transaction.
- Provenance: natural yield for a cell is the material between `natural_top` and its boot height,
  so it is awarded at most once. Placed steps never become natural. This stops a dig-fill-dig loop.
  (Engine probe: a 1,000-column, 400-operation model gave excess 0 for this rule and +780 and +660
  for the two wrong rules. `scratchpad/m4/probes/geology_loop.py`, run by the engine lens; the
  gate reviewer must re-run it.)

## 4. Excavation

Commands (sim `CommandKind`, wire `PlayerCommandKind`, validated in `drain_commands`):

- `Excavate { unit_id, min: CellPos, max: CellPos, target: i16 }`
- `Fill { unit_id, min, max, target: i16, material: String }`

Command-time validation, in this order: owner; unit has `excavator` (`MissingCapability`);
rect inside the map; at most 64 cells (`AreaTooLarge`); no cell under a static footprint
(`FoundationProtected`); `target` not below the floor for Excavate (`Undiggable`); for Fill the unit
has `fill` with that material. New reject reasons: `AreaTooLarge`, `FoundationProtected`,
`Undiggable`, plus wire mirrors (`wire_reason`, `command_reject_reason`).

An `Excavate` or `Fill` order replaces the unit's earthwork order. `MoveTo` and `Remove` cancel it.
A cancelled step loses its progress and changes nothing (no yield, no height change).

Execution, new set `Earthwork` (section 7):

1. Pick the next cell: the cell with the highest height above `target` (Excavate) or the lowest
   below `target` (Fill); tie by lowest cell index.
2. If the unit centre is farther than `reach` from the cell centre, set a `MoveTarget` to the
   nearest traversable cell centre within `reach`, and wait. If none exists, status `NoRoute`.
3. In reach: `elapsed += 1` each tick. At `need = ceil(dig_ms_per_step * tick_hz / 1000)` ticks
   (M3D:88 rule), commit exactly one cell step in **one `transact`**: the new height, the
   inventory credit (or debit for Fill), the `Excavated` or `Filled` event, and the chunk bump.
4. If the yield does not fit the inventory, nothing commits (status `CargoFull`, K7). If a fill
   lacks items, nothing commits (status `NeedItems`).
5. A step re-checks `FoundationProtected` at commit time. A unit standing on a cell that is dug
   stands lower; units do not block edits.

`WriteSet` gains `cells: BTreeMap<u32, Option<CellState>>` with `stage_cell`, a `preflight` line and
an `apply_overlays` arm (the M3 `extract_from_node` pattern). New events: `Excavated { unit, cell,
from, to, natural, yields }` and `Filled { unit, cell, from, to, consumed }`. The oracle
`balance_expected` (`conservation.rs:258`) gets arms for both.

## 5. Navigation

- The goal stays `MoveTarget` (so Gather and every M3 system keep working). On a terrain map the
  navigation set turns a goal into a `Route { cells, next, versions }`. On a map with no terrain
  there is no `Route` and movement is the M3 straight line.
- Edge rule: moving between 8-neighbour cells a and b is allowed when
  `|h(a) - h(b)| <= max_climb` of the unit; no corner cutting. Search: A* with integer costs 10 and
  14, an octile heuristic, tie-break by (f, h, cell index). Waypoints are cell centres; the last
  waypoint is the exact target.
- A search runs for a unit with a new goal, or a broken route, or status `NoRoute` after an epoch
  change. FIFO by (request tick, unit id), at most **4 searches per tick** (proposal, measure).
  A unit that waits holds position.
- Local invalidation: a commit bumps `chunk_version` of the chunk that holds the cell and of every
  chunk that holds a neighbour cell (at most 4 chunks), and bumps `edit_epoch`. Each tick, a unit
  whose remaining route crosses a bumped chunk re-checks only the edges in those chunks. A broken
  route stops the unit at the last valid cell centre and queues a re-search.
- `NoRoute` memo: a unit with `NoRoute` re-searches only when `edit_epoch` changed (K8). This stops
  Gather's per-tick `set_target` (`gather.rs:1236`) from searching every tick.
- Movement on a route follows waypoints in 2D at the unit speed. `Position` stays `Vec2`; the
  client samples height at render time.
- Cost probe (engine lens, i9-14900K, release): 256x256, open route 119 us (1,905 expansions),
  closed ridge 3.6 ms (32,000 expansions); a one-cell edit dirtied 2 chunks and re-checked 23 to
  33 route edges. The quarry map is 24 x 14, so M4 sits far below these. Not re-run by the
  orchestrator (NOT TRACED until gate 1).
- Known M4 limits: weapons, gather range and spawn placement still use 2D distance and ignore
  height (`combat.rs:165-192`, `gather.rs:438-456`, `production.rs:419-470`); the quarry package
  uses none of them. `--synthetic-units` units may pile up at walls on terrain maps.

## 6. Foundations (K9)

`UnitDef.foundation: Some((edit: ForbidEdit))`. The cells under a static unit footprint cannot be
excavated or filled. Boot rejects a static unit that stands on non-flat cells. The engine does not
destroy or invalidate a static unit. Later values (`Invalidate`, `ModHook`) are future work.

## 7. Tick order, audit, protocol 4

Order, with two new sets before Movement (the C15 anchor text stays byte-identical):

`Commands -> Earthwork -> Navigate -> Movement -> Gather -> Extract -> Transport -> Inserters ->
Production -> Combat -> Collect`

An edit committed in tick T is seen by `Navigate` in tick T, before any unit moves. So a unit never
moves on a stale route.

Audit: terrain cell and geology changes happen only inside `transact`, so they are captured. `Route`
and `edit_epoch` are navigation state in the movement exemption (trace (a), section 1.2, decides
whether the exemption needs a code change). `AuditState` gains terrain bytes with
`skip_serializing_if`, so flat maps emit no new bytes.

Protocol 4:

- `Snapshot.terrain: Option<TerrainRecord { width, height, step, cells: Vec<CellRecord> }>`;
  `CellRecord { height: i16, top_material: u8 }`. 24 x 14 x 3 bytes is about 1 KB. Resync resends
  it. Chunking for larger maps is future work.
- `WorldDelta.cells: Vec<CellRecord + cell index>` for cells that changed this tick.
- Owner-only: `EarthworkRecord { unit, cell, step_index, steps_total, elapsed, need, status }`
  and `NavRecord { unit, state: Idle | Moving | Searching | NoRoute }`.
- `Geology` has no wire type. `top_material` is the visible surface, which is public by nature.
- Tests that pin 3: `tests/wire.rs:23`, `tests/protocol3.rs:17` (rename to `protocol4.rs`),
  `turts_server/tests/economy.rs:2388-2399`, `support/wire_sender.rs`. The postcard test buffer
  of 16,384 bytes must grow for a terrain snapshot test.

## 8. The quarry package

Package `quarry` (kind Game, `engine_api: "^0.2.2"`). One map `ridge_pass`, 24 x 14, one player slot.
All numbers are proposals; the case builder derives the fixed point by running the simulation.

| Item | Value |
| --- | --- |
| Heights (steps) | west plain x 0-9 and east plain x 12-23: 0. Ridge x 10-11, y 0-11: 4. Saddle x 10-11, y 12-13: 2. |
| Column of a ridge cell, top down | soil [3,4], rock [2,3], ore-bearing [0,2] (ore, 2 per step), floor at 0. |
| Materials | `soil`: 2,000 ms, yields 1 spoil. `rock`: 4,000 ms, yields 1 spoil. `ore_bearing`: 3,000 ms, yields 2 ore. `fill`: 1,000 ms, yields nothing. `bedrock`: not diggable. |
| `quarry:excavator` | speed 1.5, `max_climb` 1, reach 2.5, inventory 24 (ore, spoil), fill consumes 1 spoil per step. |
| `quarry:scout_rover` | speed 2.0, `max_climb` 2, no excavator, no inventory. |
| `quarry:survey_post` | static, radius 1.0, `foundation: ForbidEdit`, at (7.5, 9.5). |
| Zones | `cut_zone` x 10-11, y 5-6 (4 cells); `ridge`; `saddle`. |
| Start | excavator (4, 6); scout rover (3, 3); post (7.5, 9.5). |
| Guide lines | 5 static lines (look at the ridge, select the Excavator, press Dig and drag over the cut zone, watch the ore count, send the Scout Rover east). |

Per cell: 1 soil + 1 rock + 2 ore steps = 2 + 4 + 6 = 12 s = 240 ticks at 20 Hz, 2 spoil, 4 ore.
Four cells: 960 ticks, 8 spoil, 16 ore (excavator inventory 24 = full, exact). The excavator digs
layer by layer (highest cell first), so the passage (cells at height 1) opens at tick 720 of
digging. These numbers are hand-derived and unrun. Case G1 computes the real values.

The excavator cannot cross the ridge (edge 0 to 4, limit 1) and cannot use the saddle (edge 0 to 2,
limit 1). The scout rover can use the saddle (limit 2); the passage shortens its route to a target
at (20, 6).

## 9. Client: terrain, picking, drag-select, readability

Engine data comes from section 7. Client work:

- Terrain mesh: one stepped mesh from replicated cells; each height step has its own shade; cells
  are coloured by `top_material` with a legend. Chunk meshes rebuild only for changed cells.
- Picking: `ground_point` marches the ray against the heightfield (replaces the plane y=0).
  The camera fit includes the highest cell.
- **K1 drag-select:** a left drag draws a box and selects the player's own units inside it.
  `Selection` holds a set of unit ids. A right click on the ground sends one `MoveTo` with all
  selected ids (`MoveTo` already carries a list, `command.rs:257-294`). No hotkeys, no control
  groups, no formations.
- Dig mode: with an excavator selected, the panel shows a **Dig** button. Press it, then drag a
  rectangle on the map to send `Excavate` with `target` = the floor. Right click stays "move".

Readability requirements. Each has a proof; the first six are the cut order from last to first
(H8 and H7 go first if M4 is too big).

| Id | Requirement | Proof |
| --- | --- | --- |
| H1 | Every unit and zone shows a catalog or zone name in the world. | Browser reads label text for each id. |
| H2 | Height shows as stepped shades; no numbers needed. | Screenshot pixels of a height-4 and a height-0 cell differ; after a dig step the cell pixel changes. |
| H3 | Soil, rock, ore-bearing, bedrock and fill have distinct colours and a legend. | Pixel test per material; legend text present. |
| H4 | The selected excavator shows **Dig** and a one-line hint; hovering a cell shows its height, material and "diggable" or the reason. | Browser: select then assert text; hover then assert text. |
| H5 | An order shows a target outline and a progress line ("step 2 of 4", status). | Send order, assert status text changes within 1 s and progress rises. |
| H6 | A blocked order shows a reason from a fixed list: No route, Cargo full, Cell is protected, Cannot dig, Needs spoil. | One browser test per reason. |
| H7 | Inventories show item names and counts. | Assert "Ore 16" and "Spoil 8". |
| H8 | The panel shows the static `guide` lines. (Tick-off of finished steps is future work.) | Assert the guide text. |
| H9 | Drag-select (K1). | Real pointer drag selects owned units only; the next right click sends one `MoveTo` that carries all ids. |

## 10. Oracle, checks and cases

The oracle parses `mods/quarry` itself and rebuilds heights from events. It does not read the
simulation catalog. New checks (the M3 checks stay):

| Check | Statement | Wrong build that turns it red |
| --- | --- | --- |
| GEO-yield | Each `Excavated` yield equals the interval table of the oracle's own column at that elevation; total ore yielded never exceeds the natural ore of the cells. | plants M4-1, M4-2, M4-3 |
| GEO-atomic | The height change, the inventory credit and the event are in one operation; never one without the others. | M4-4 |
| GEO-natural | `natural_top` never rises; a placed step yields nothing. | M4-2, M4-3 |
| NAV | Every unit step between cells, in every tick, is an allowed edge under that tick's heights. | M4-5, M4-7 |
| LOCAL | A commit bumps only the chunks of the cell and its neighbours; a tick re-checks only edges in bumped chunks; at most 4 searches per tick. | M4-6, M4-8 |
| FOUND | No cell under a static footprint changes height. | M4-10 |
| CARGO | Cargo never drops items: a refused step changes no state. | M4-11 |
| STATE | Every state change happens inside a captured operation, except the movement exemption. | M4-4 |

Cases (all run on the real simulation; each asserts its own precondition, so none passes vacuously):

- G1 exact yield: dig the cut zone to the floor; ore 16, spoil 8; the step commits at the computed ticks.
- G2 depleted stays depleted: fill 2 cells to height 4 (8 spoil), then dig again; ore +0, spoil +0.
- G3 cannot cross, then can: the excavator's `MoveTo` east gives `NoRoute`; after the passage opens
  (cells at height 1) the order succeeds without a new command (K8).
- G4 in-flight re-route: the scout rover is on its saddle route when the passage opens (precondition:
  it is at least 3 cells short of the saddle waypoint at the commit tick); it re-routes; a rover
  far from the edit keeps its route (LOCAL).
- G5 locality: an edit near x=10 bumps at most 4 chunks; re-checks stay under the bound.
- G6 foundation: `Excavate` under the survey post is rejected `FoundationProtected`; nothing changes.
- G7 fill blocks again: after a fill that closes the passage, a unit in flight stops at its last
  valid cell and gets `NoRoute`.
- G8 cargo full (variant package, inventory 12): the step does not commit; status `CargoFull`; no state change.
- G9 interrupt: `MoveTo` during a step discards the progress; no yield, no height change.
- G10 protocol: a late joiner sees the same cells as a live client; deltas equal the committed cells;
  no geology bytes on the wire.
- G11 flat-map regression: the frozen M2 stream (`tests/m3/m2_operation_stream.json`) and the M3 F2
  stream stay byte-identical; the 17 M3 plants stay green.
- G12 determinism: two runs give the same routes and the same audit stream hash.
- G13 package validation: each bad terrain package gives an error that names package, file and field.

Mutation plants (`tests/m4/refutations.json`, same mechanism as M3: byte-exact anchor, plant, run,
expect red, restore):

M4-1 natural depth measured from the surface; M4-2 fill raises `natural_top`; M4-3 a placed step
yields; M4-4 the yield is credited in a separate operation from the height change; M4-5 skip the
route re-check; M4-6 bump every chunk; M4-7 ignore `max_climb`; M4-8 remove the search cap;
M4-9 drop the order on `NoRoute`; M4-10 allow an edit under a footprint; M4-11 destroy the surplus
when the cargo is full; M4-12 flat maps emit terrain bytes; M4-13 a late snapshot omits the
terrain. CI gets a step "M4 semantic refutations (all 13 plants)".

## 11. Acceptance

| Test | Evidence |
| --- | --- |
| F9 | Player-control gate |

- F1 `cargo test --workspace`, fmt, strict clippy, wasm32 check and `scripts/boundary.sh` pass.
- F2 G11: the M2 and M3 operation streams are unchanged, and all 17 M3 plants stay red-then-green.
- F3 G1 and G2 pass; GEO checks pass on the full run.
- F4 G3, G4 and G7 pass; NAV passes.
- F5 G5 passes; LOCAL passes; the measured p99 tick time with 50 units ordered to an unreachable
  goal on the quarry map is recorded (no budget; S:706 stays Open).
- F6 G6, G8, G9 pass.
- F7 G10, G12, G13 pass.
- F8 All 13 M4 plants go red on their named check and green after restore.
- F9 **Player-control gate:** the `player` agent plays the quarry demo in the browser with only the
  mouse and keyboard, with the in-game guide and no other document, and completes: dig the cut
  zone, see the ore count, send the rover through the passage. The agent writes a verdict. A
  failure to find an object, an action or a reason is a finding that blocks the milestone.
- F10 CI: all jobs green on the pushed commit (hosted CI, not only local runs).

## 12. Risks and cut line

1. Double yield by dig-fill-dig. Guard: GEO checks, plants M4-1 to M4-3.
2. Stale route. Guard: NAV, M4-5.
3. Non-local rebuild or search spike. Guard: LOCAL, M4-6, M4-8, the 4-per-tick cap.
4. Regression on flat maps. Guard: F2, M4-12.
5. The client grows large: terrain mesh, ray-march picking, camera fit, drag-select, panel and labels
   touch `render.rs`, `input.rs`, `economy_ui.rs` (1,425 lines). Guard: split the client into
   separate tasks and cut H8 and H7 first.
6. The hot files in `transaction.rs`, `schema.rs`, `bootstrap.rs` and `refutations.json` collide.
   Guard: one owner per task, the orchestrator composes merges (as in M3).

Cut line, in order: H8; the tick of the guide; the scout rover with G4 and the saddle; G13 cases
beyond the first three bad packages. Never cut: GEO, NAV, LOCAL, G1 to G3, G11, K1, F9.

## Yardstick coverage

| id | disposition | test |
| --- | --- | --- |
| IN01 | in | F9 |
| IN02 | done:crates/turts_client/tests/acquire.rs | - |
| IN03 | excluded:D01 | - |
| IN04 | later:M5 | - |
| IN05 | later:M5 | - |
| IN08 | later:M5 | - |
| IN09 | later:M5 | - |
| PC01 | in | F9 |
| PC02 | in | F9 |
| PC04 | later:M5 | - |
| PC05 | later:M5 | - |
| PC08 | later:M5 | - |
| PC09 | later:M5 | - |
| PC10 | later:M5 | - |
| PC16 | later:M5 | - |
| PC17 | later:M5 | - |
| PC19 | later:M5 | - |
| PC20 | later:M5 | - |
| PC21 | later:M5 | - |
| PC22 | later:M5 | - |
| PC23 | later:M5 | - |
| OR01 | later:M5 | - |
| OR02 | later:M5 | - |
| OR03 | later:M5 | - |
| OR04 | later:M5 | - |
| OR05 | later:M5 | - |
| OR08 | later:M5 | - |
| OR11 | in | F9 |
| OR12 | later:M5 | - |
| OR15 | later:M5 | - |
| OR16 | later:M5 | - |
| OR17 | later:M5 | - |
| OR20 | later:M5 | - |
| OR22 | later:M5 | - |
| OR23 | later:M5 | - |
| OR24 | in | - |
| OR25 | later:M5 | - |
| OR26 | later:M5 | - |
| OR31 | later:M5 | - |
| GM01 | later:M5 | - |
| GM04 | later:M5 | - |
| GM06 | later:M5 | - |
| GM07 | later:M5 | - |
| GM09 | later:M5 | - |
| GM10 | later:M5 | - |
| GM11 | later:M5 | - |
| CA01 | in | F9 |
| CA02 | later:M5 | - |
| CA03 | later:M5 | - |
| CA08 | later:M5 | - |
| CA09 | later:M5 | - |
| CA13 | later:M5 | - |
| CA14 | later:M5 | - |
| CA15 | later:M5 | - |
| CA16 | later:M5 | - |
| UI01 | later:M5 | - |
| UI02 | later:M5 | - |
| UI03 | later:M5 | - |
| UI04 | later:M5 | - |
| UI05 | later:M5 | - |
| UI08 | later:M5 | - |
| UI09 | later:M5 | - |
| UI10 | later:M5 | - |
| UI11 | later:M5 | - |
| UI12 | later:M5 | - |
| UI14 | later:M5 | - |
| UI16 | later:M5 | - |
| UI17 | later:M5 | - |
| UI19 | later:M5 | - |
| UI22 | later:M5 | - |
| UI23 | later:M5 | - |
| CO01 | later:M5 | - |
| CO02 | later:M5 | - |
| CO03 | later:M5 | - |
| CO04 | later:M5 | - |
| CO07 | later:M5 | - |
| CO08 | later:M5 | - |
| CO20 | later:M5 | - |
| CO21 | later:M5 | - |
| PR01 | later:M5 | - |
| PR02 | later:M5 | - |
| PR03 | later:M5 | - |
| PR04 | later:M5 | - |
| PR05 | later:M5 | - |
| PR06 | later:M5 | - |
| PR07 | later:M5 | - |
| PR08 | later:M5 | - |
| PR10 | later:M5 | - |
| PR15 | later:M5 | - |
| PR17 | later:M5 | - |
| EC01 | later:M5 | - |
| EC02 | later:M5 | - |
| EC03 | later:M5 | - |
| EC04 | later:M5 | - |
| EC05 | later:M5 | - |
| EC10 | later:M5 | - |
| CB01 | later:M5 | - |
| CB02 | later:M5 | - |
| CB03 | later:M5 | - |
| CB04 | later:M5 | - |
| CB05 | later:M5 | - |
| CB07 | later:M5 | - |
| CB09 | later:M5 | - |
| CB10 | later:M5 | - |
| CB21 | later:M5 | - |
| CB25 | later:M5 | - |
| FV01 | later:M5 | - |
| FV02 | later:M5 | - |
| FV03 | later:M5 | - |
| FV04 | later:M5 | - |
| FV05 | later:M5 | - |
| FV10 | later:M5 | - |
| UN01 | later:M5 | - |
| UN07 | later:M5 | - |
| UN11 | later:M5 | - |
| UN13 | later:M5 | - |
| PF01 | later:M5 | - |
| PF02 | later:M5 | - |
| PF03 | later:M5 | - |
| PF04 | later:M5 | - |
| PF05 | later:M5 | - |
| PF10 | later:M5 | - |
| MF01 | later:M5 | - |
| MF02 | later:M5 | - |
| MF03 | later:M5 | - |
| MF04 | later:M5 | - |
| MF05 | later:M5 | - |
| MF06 | later:M5 | - |
| MF07 | later:M5 | - |
| MF08 | later:M5 | - |
| MF10 | later:M5 | - |
| MF12 | later:M5 | - |
| MF14 | later:M5 | - |
| SV01 | later:M5 | - |
| SV04 | later:M5 | - |
| SV05 | later:M5 | - |
| MP01 | later:M5 | - |
| MP02 | later:M5 | - |
| MP05 | later:M5 | - |
| MP07 | later:M5 | - |
| MP09 | later:M5 | - |
| MD01 | later:M5 | - |
| MD02 | later:M5 | - |
| MD03 | later:M5 | - |
| MD04 | later:M5 | - |
| MD05 | later:M5 | - |
| MD06 | later:M5 | - |
| MD07 | later:M5 | - |
| MD08 | later:M5 | - |
| MD09 | later:M5 | - |
| MD10 | later:M5 | - |
| MD11 | later:M5 | - |
| MD12 | later:M5 | - |
| MD17 | later:M5 | - |
| MD18 | later:M5 | - |
| BT01 | later:M5 | - |
| BT02 | later:M5 | - |
| BT03 | later:M5 | - |
| TM01 | later:M5 | - |
| TM02 | later:M5 | - |
| TM03 | later:M5 | - |
| AU01 | later:M5 | - |
| AU02 | later:M5 | - |
| AU03 | later:M5 | - |
| AU06 | later:M5 | - |
| AU07 | later:M5 | - |
| AN01 | later:M5 | - |
| AN02 | later:M5 | - |
| AN03 | later:M5 | - |
| AN05 | later:M5 | - |
| AN06 | later:M5 | - |
| AN07 | later:M5 | - |
| AN08 | in | F9 |
| AN11 | later:M5 | - |
| AC05 | later:M5 | - |
| AC06 | later:M5 | - |
| PE01 | later:M5 | - |
| PE02 | later:M5 | - |
| PE03 | later:M5 | - |
| PE04 | later:M5 | - |
| PE07 | later:M5 | - |

## Player path

1. Drag a box around both rovers.
2. Right-click past the ridge.
