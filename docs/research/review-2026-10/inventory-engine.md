# Reference inventory: what a moddable RTS engine must provide

Author seat: game-designer. Date: 2026-10-28.
Purpose: an outside yardstick for the full review of `RTS-Engine-Project-Specifications.md`.

Order of work. I wrote Part A from genre knowledge before I opened any repo file. I wrote
Part B after one end-to-end read of the spec. Part A has no spec input.

Bar (CEO goal, 2026-10-28, "Build the Ultimate RTS engine"): the bar is the UNION of what the
named games do. One game that lacks an item does not remove the item. This inventory has no
"out of scope" list. The spec review decides scope.

## Legend

Games:

- SC1 = StarCraft / Brood War. SC2 = StarCraft II.
- CNC = Command & Conquer, Tiberian Dawn to Red Alert 2 (TD, RA, TS, RA2).
- AOE = Age of Empires II (original and DE). WC3 = Warcraft III.
- SUP = Supreme Commander (and Forged Alliance). FAC = Factorio-style industrial RTS.
- "All6" = SC1, SC2, CNC, AOE, WC3, SUP. FAC is listed apart, because it has no classic army
  micro.

Kind (the project splits engine mechanism from mod policy, spec 4.3):

- M = engine mechanism. The engine must supply it. A mod cannot build it.
- P = mod policy. The mod decides values or rules. The engine must expose a hook.
- B = both. The engine supplies the mechanism. The mod supplies the rule, the values, or the
  content.

Tier:

- CORE = every named RTS has it in some form. A game without it is not an RTS.
- COMMON = most named games have it. Players expect it.
- OPTIONAL = some named games have it. It is in the union bar, so the engine must still let a
  mod build it.

"Depends on" lists item IDs. A dependency means: the item cannot work until that item works.

---

# Part A: the inventory

## A1. Input foundation (IN)

These items sit under every player-control item. They are often invisible in a design until
they fail.

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| IN01 | Screen-to-world picking | Map a cursor position to a terrain point and to the entity under it (units, buildings, resources). | All6, FAC | M | CORE | CA01, AN08 |
| IN02 | Action mapping layer | Map physical keys and buttons to named actions. Modifiers (Shift, Ctrl, Alt) are part of the binding. | All6, FAC | M | CORE | none |
| IN03 | Modifier semantics | Shift = add or queue. Ctrl = same type, or assign group. Alt = special (ping, force move). One consistent meaning across all actions. | All6 | B | CORE | IN02 |
| IN04 | Double-click detection | Detect a double click and a double key tap with a timing window. | All6 | M | CORE | IN02 |
| IN05 | Hotkey per command button | Every command-card button has a key. The mod gives default keys. | All6, FAC | B | CORE | IN02, UI01 |
| IN06 | Grid hotkey layout | Keys map to command-card grid positions, not to letters. | SC2, SUP (option), AOE DE | B | OPTIONAL | IN05 |
| IN07 | Command scheme option | Right-click commands (modern) or left-click commands (classic CNC, AOE). | CNC, AOE, all others right-click | M | OPTIONAL | IN02 |
| IN08 | Browser input capture | Suppress the browser context menu on right-click. Take keys the browser reserves (Ctrl+1..9 switches tabs, Ctrl+W closes, F5 reloads). Use pointer lock or a fallback. | Browser-specific; needed for the CORE keys of all6 | M | CORE (for a browser-first engine) | IN02 |
| IN09 | Text input mode | Typing chat must not trigger hotkeys. | All6, FAC | M | CORE | IN02, UI17 |
| IN10 | Touch and gamepad input | Selection and orders without a mouse. | None of the named PC games; console ports exist | M | OPTIONAL | IN02 |

## A2. Player control and selection (PC)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| PC01 | Single select | Left-click one unit or building to select it. The previous selection clears. | All6, FAC | M | CORE | IN01 |
| PC02 | Box (drag) select | Drag a rectangle. All own selectable units inside it are selected. | All6, FAC (selection tools) | M | CORE | IN01, CA01 |
| PC03 | Box filter rules | A box prefers combat units over workers and buildings, or follows a mod rule. | SC2, WC3, AOE, SUP | B | COMMON | PC02 |
| PC04 | Shift add and remove | Shift-click or Shift-drag adds to the selection. Shift-click a selected unit removes it. | All6 | M | CORE | PC01, PC02, IN03 |
| PC05 | Type select on screen | Double-click or Ctrl-click a unit. All own units of that type on screen are selected. | All6 (RA2: T key) | M | CORE | PC01, IN04, CA01 |
| PC06 | Type select on map | Select all own units of a type on the whole map. | RA2 (T T), SUP, SC2 (via hotkey) | M | COMMON | PC05 |
| PC07 | Selection cap | A limit on the selection size (SC1 12, WC3 12, AOE 40 or 60, SC2 none). | All6 differ | B | COMMON | PC02 |
| PC08 | Control group assign and recall | Ctrl+N stores the selection in group N. N recalls it. | All6 | M | CORE | IN02, IN03 |
| PC09 | Control group add | Shift+N adds the selection to group N. | SC2, WC3, AOE, SUP | M | CORE | PC08 |
| PC10 | Group double-tap centres camera | Press N twice: the camera centres on the group. | All6 | M | CORE | PC08, CA14 |
| PC11 | Steal from group | Ctrl+Shift+N or Alt+N moves units from their old group to the new group. | SC2, AOE DE | M | OPTIONAL | PC08 |
| PC12 | Subgroups and Tab cycling | A mixed selection has one subgroup per type. Tab changes the active subgroup. The command card shows the active subgroup. | SC2, WC3, SUP | M | COMMON | PC02, UI01, UI02 |
| PC13 | Building hotkeys | Select a production building or the main base with a key (SC2 Backspace, AOE H). | SC2, AOE, CNC (Home) | B | COMMON | PC01 |
| PC14 | Idle worker select | A key or button cycles through idle workers. | SC2, WC3, AOE, SUP | B | COMMON | UN11, UI15 |
| PC15 | Select all army | One key selects every own combat unit. | SC2, SUP, RA2 (Q / P) | B | COMMON | PC06 |
| PC16 | Inspect foreign entities | Select an enemy, ally, or neutral entity to read its info. No orders. | All6, FAC | M | CORE | PC01, FV04 |
| PC17 | Ownership gate | A player can order only own units. Order from another slot is rejected. | All6 | M | CORE | none |
| PC18 | Shared control | An ally can order your units when you allow it. | SC1, SC2, WC3, AOE DE (co-op) | B | OPTIONAL | PC17 |
| PC19 | Selection hygiene | Dead, transported, or converted units leave the selection and the groups. A group survives a reload. | All6 | M | CORE | PC08 |
| PC20 | Hover highlight | The entity under the cursor highlights before a click. | All6, FAC | M | CORE | IN01 |
| PC21 | Context cursor | The cursor shape shows the default order (attack, harvest, enter, repair, cannot move). | All6, FAC | B | CORE | IN01, OR22 |
| PC22 | Smart command | Right-click picks the default order from the target (move, attack, harvest, repair, load, garrison). | All6 | B | CORE | OR01, OR03, EC02 |
| PC23 | Selection visual | A circle, outline, or bracket on each selected entity. | All6, FAC | M | CORE | AN06 |

## A3. Orders (OR)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| OR01 | Move | Go to a point. | All6, FAC | M | CORE | PF01 |
| OR02 | Stop | Clear the order queue. Stand still. | All6 | M | CORE | OR11 |
| OR03 | Attack target | Attack one unit or building until it dies or leaves reach. | All6 | M | CORE | CB02 |
| OR04 | Attack-move | Move to a point. Engage enemies on the way. | SC1, SC2, WC3, SUP, AOE DE, CNC (TS/RA2 via modifier) | M | CORE | OR01, CB03 |
| OR05 | Force move | Move and ignore all enemies (CNC Alt-click, SC move). | All6 | M | CORE | OR01 |
| OR06 | Force attack | Attack the ground, an ally, or a neutral object. | CNC (Ctrl), SC2 (some units), SUP, AOE (attack ground) | B | COMMON | OR03 |
| OR07 | Patrol | Move back and forth between points. Engage enemies on the way. | SC1, SC2, WC3, AOE, SUP, RA2 (waypoint loop) | M | COMMON | OR04, OR11 |
| OR08 | Hold position | Do not move. Attack what is in range. | SC1, SC2, WC3, AOE (stand ground), SUP | M | CORE | CB03 |
| OR09 | Stances | Aggressive, defensive, stand ground, no attack, return fire, hold fire. | AOE, SUP, WC3, CNC (guard) | B | COMMON | CB03, CB05 |
| OR10 | Guard and follow | Stay near a unit or an area. Defend it. | CNC, AOE, SUP (assist), WC3 | M | COMMON | OR03 |
| OR11 | Order queue per unit | Each unit holds an ordered list of orders. | All6, FAC | M | CORE | none |
| OR12 | Shift-queued orders | Shift appends an order to the queue instead of replacing it. | All6 (RA2 waypoint mode) | M | CORE | OR11, IN03 |
| OR13 | Waypoint display | Show the queued orders as lines and markers. | SUP, SC2, RA2, AOE | M | COMMON | OR11, AN06 |
| OR14 | Edit queued orders | Drag or delete a queued waypoint. | SUP | M | OPTIONAL | OR13 |
| OR15 | Rally point | A production building sends new units to a point, a unit, or a resource. | All6, FAC (n/a) | B | CORE | PR05 |
| OR16 | Build order | A worker goes to a site and builds a structure. | SC1, SC2, AOE, WC3, SUP | B | CORE | CO04 |
| OR17 | Gather order | A worker or harvester gathers from a node. | All6 | B | CORE | EC02 |
| OR18 | Repair order | A unit repairs a building or a vehicle. | All6 except SC protoss and zerg | B | COMMON | CO11 |
| OR19 | Load and unload | Enter a transport or a building. Leave it. | All6 | M | COMMON | UN05, UN06 |
| OR20 | Ability order | Use an ability with no target, a point, a unit, or an area. | All6 | B | CORE | UN07 |
| OR21 | Autocast toggle | An ability fires on its own when its rule matches. | SC2, WC3 | B | OPTIONAL | OR20 |
| OR22 | Order validation feedback | A rejected order gives a reason ("Not enough minerals", "Cannot build there"). | All6, FAC | B | CORE | OR11 |
| OR23 | Order acknowledgement | A ground marker at the target and a voice line, at once on the client. | All6 | B | CORE | AU01, AN06, PE01 |
| OR24 | Group order fan-out | One command applies to N selected units. Each unit gets its own order. Each unit that cannot do it ignores it. | All6, FAC | M | CORE | PC02, OR11 |
| OR25 | Group destination spread | N units sent to one point do not all fight for that point. | All6 | M | CORE | OR24, GM01 |
| OR26 | Order state machine | A rule for what interrupts what: retaliation, auto-target, a new order, a queued order. | All6 | M | CORE | OR11, CB03, CB05 |
| OR27 | Deploy and transform | A unit changes mode (siege mode, MCV deploy, burrow). | SC1, SC2, CNC, WC3 | B | COMMON | OR20 |
| OR28 | Scatter | Units near a threat spread out. | CNC | M | OPTIONAL | GM04 |
| OR29 | Assist | A unit copies the work of another unit (engineer assist, factory assist). | SUP | M | OPTIONAL | OR10 |
| OR30 | Repeat queue | A factory or a patrol repeats its queue for ever. | SUP, FAC | M | OPTIONAL | OR11, PR01 |
| OR31 | Command rate limit | The server bounds orders per player per tick. It rejects floods. | All6 (implicit in netcode) | M | CORE | OR22 |

## A4. Group movement (GM)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| GM01 | Destination spreading | Each unit of a group gets a slot near the target. Units do not stack. | All6 | M | CORE | OR24 |
| GM02 | Formations | Shapes for a group (line, box, staggered, flank). | AOE, SUP, WC3, CNC (TS/RA2 formation) | B | COMMON | GM01 |
| GM03 | Speed matching | A group moves at the speed of the slowest unit. | AOE, WC3, SUP, RA2 | M | COMMON | GM02 |
| GM04 | Local avoidance | Moving units steer around each other. No overlap. | All6 | M | CORE | GM06 |
| GM05 | Flocking | Groups move as a flowing mass (boids-style steering). | SC2, SUP | M | COMMON | GM04 |
| GM06 | Unit collision size | Each unit has a radius or a footprint. Big units take more room. | All6, FAC | B | CORE | none |
| GM07 | Push idle friends | A moving unit pushes idle friendly units aside. | All6 | M | CORE | GM04 |
| GM08 | Air separation | Air units do not collide with ground units. They keep spacing among themselves. | SC1, SC2, WC3, SUP, CNC | M | COMMON | UN02 |
| GM09 | Arrival | A group stops without jitter when it reaches the target area. | All6 | M | CORE | GM01 |
| GM10 | Unreachable target | Move to the nearest reachable point. Do not freeze. | All6, FAC | M | CORE | PF04 |
| GM11 | Choke traffic | Units do not deadlock in a narrow pass. | All6 | M | CORE | GM04 |
| GM12 | Formation facing | Right-drag sets the line and the facing of a formation. | SUP, AOE DE | M | OPTIONAL | GM02 |
| GM13 | Worker collision exception | Gathering workers ignore each other in a mineral line. | SC2, SC1 | B | OPTIONAL | EC02, GM04 |

## A5. Camera (CA)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| CA01 | Classic RTS camera | Tilted top-down view (perspective or isometric). Fixed pitch. Pans on the map plane. | All6, FAC (top-down) | M | CORE | none |
| CA02 | Edge scroll | The camera pans when the cursor touches a screen edge. | All6, FAC | M | CORE | CA01, IN08 |
| CA03 | Key scroll | Arrow keys (or WASD) pan the camera. | All6, FAC | M | CORE | CA01, IN02 |
| CA04 | Drag pan | Middle-drag or a key-drag pans the camera. | SC2, WC3, SUP, AOE DE, FAC | M | COMMON | CA01 |
| CA05 | Zoom | Mouse wheel changes distance between limits. | SC2, WC3, SUP, AOE DE, FAC | B | COMMON | CA01 |
| CA06 | Strategic zoom | Zoom out to the whole map. Units become icons. | SUP | M | OPTIONAL | CA05, UI05 |
| CA07 | Rotation | Rotate the camera around the view centre. | WC3 (limited), SUP | M | OPTIONAL | CA01 |
| CA08 | Minimap click | Left-click the minimap to move the camera. Drag to keep moving. | All6, FAC | M | CORE | UI05 |
| CA09 | Minimap order | Right-click the minimap to order selected units. | All6 | M | CORE | UI05, OR24 |
| CA10 | Camera bookmarks | Save and recall camera positions by key. | SC2, AOE DE, WC3 (custom) | M | COMMON | CA01 |
| CA11 | Jump to last alert | One key moves the camera to the last event. | SC1, SC2, WC3, AOE, CNC | M | COMMON | UI16 |
| CA12 | Follow unit | The camera follows a selected unit. | SC2, AOE, SUP, WC3 | M | COMMON | CA01 |
| CA13 | Bounds clamp | The camera stays inside the playable area. | All6, FAC | B | CORE | MP09 |
| CA14 | Centre on selection | One key centres the camera on the selection. | All6 | M | CORE | CA01 |
| CA15 | Height-aware camera | The camera follows terrain height so the view does not clip hills. | WC3, SC2, SUP, AOE DE | M | CORE (3D) | CA01, MP01 |
| CA16 | Scroll speed setting | The player sets scroll and zoom speed. | All6, FAC | M | CORE | UI22 |
| CA17 | Second view | A second screen or picture-in-picture view. | SUP | M | OPTIONAL | CA01 |

## A6. User interface (UI)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| UI01 | Command card | A grid of order and production buttons for the selection. Buttons grey out with a reason. | All6 (CNC: sidebar plus right-click) | B | CORE | PC12, IN05 |
| UI02 | Selection panel | Icons of all selected units with health. Click narrows. Shift-click removes. | SC1, SC2, WC3, AOE, SUP, RA2 | M | CORE | PC02 |
| UI03 | Info panel and portrait | Single selection shows name, health, armour, weapon, kills, and a portrait. | All6, FAC | B | CORE | PC01 |
| UI04 | Resource bar | Shows each resource, supply or population, power. | All6, FAC | B | CORE | EC01 |
| UI05 | Minimap | Terrain, entities in player colours, fog, the camera frame. | All6, FAC | M | CORE | FV02 |
| UI07 | Minimap ping | Alt-click marks a point for allies. | SC2, WC3, AOE, SUP | M | COMMON | UI05, TM02 |
| UI08 | Minimap alert flash | An attack shows as a flash at its location. | All6 | M | CORE | UI05, UI16 |
| UI09 | Build menu | Structures by tech. Unavailable ones grey out with the missing requirement. | All6, FAC | B | CORE | PR07 |
| UI10 | Production queue display | Queue per building with progress. Click an item to cancel. | All6, FAC | B | CORE | PR01 |
| UI11 | Tooltips | Name, cost, hotkey, requirements, description. | All6, FAC | B | CORE | MD01 |
| UI12 | Health bars | Over entities. Modes: hover, selected, always. | All6, FAC | M | CORE | CB01 |
| UI13 | Status icons | Buffs, veterancy, upgrades, low power, no ammo. | All6 | B | COMMON | UN09 |
| UI14 | Fog rendering | Black for unexplored. Dim for explored but not visible. | All6, FAC | M | CORE | FV01, FV02 |
| UI15 | Idle worker button | A button with a count of idle workers. | WC3, AOE, SC2, SUP | B | COMMON | UN11 |
| UI16 | Event alerts | Voice and text: "base under attack", "construction complete", "unit ready", "insufficient funds", "low power", "research complete". | All6, FAC | B | CORE | AU02 |
| UI17 | Chat | All, allies, and private chat. In lobby and in game. | All6, FAC | M | CORE | IN09 |
| UI18 | Pause | A player pauses a match. Limits per player. | All6 | B | COMMON | MF14 |
| UI19 | Surrender and leave | Concede. Leave. | All6 | B | CORE | MF08 |
| UI20 | Game speed | Speed choice in the lobby or in single player. | SC1, SC2, AOE, WC3, SUP, CNC | B | COMMON | MF04 |
| UI21 | Game clock | Elapsed match time. | SC2, AOE, SUP, WC3 | M | COMMON | none |
| UI22 | In-game options menu | Keys, sound, graphics, gameplay toggles. | All6, FAC | M | CORE | none |
| UI23 | Cursor set | Mod-defined cursor images per context. | All6 | B | CORE | PC21 |
| UI24 | Rally display | Flag and line when a building is selected. | All6 | M | COMMON | OR15 |
| UI25 | Range display | Range ring while a turret is placed or selected. | SC2, CNC (RA2), SUP, AOE DE, FAC | B | COMMON | CO01 |
| UI26 | Building power indicator | A building shows low power or off. | CNC, SUP, FAC | B | COMMON | EC09 |
| UI27 | Mod UI layout | The mod defines panels, icons, skins, layout. | SC2, WC3, FAC | B | COMMON | MD08 |
| UI28 | Floating text | Income pop-ups, damage numbers. | WC3, AOE DE, SC2 (mods) | B | OPTIONAL | none |
| UI29 | Army and production overview | A tab of all own units and queues. | SUP, SC2 (observer), FAC | M | OPTIONAL | PR01 |
| UI30 | Localisation | All text comes from string tables. | All6, FAC | B | COMMON | MD13 |
| UI31 | Objectives panel | Mission and victory goals as text. | All6 (campaign), WC3, SC2 | B | COMMON | MF08 |

## A7. Construction (CO)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| CO01 | Placement ghost | A ghost of the structure follows the cursor, snapped to the grid. | All6, FAC | M | CORE | IN01, AN08 |
| CO02 | Footprint validity | Each footprint cell shows valid or invalid by colour. | All6, FAC | M | CORE | CO01, CO03 |
| CO03 | Placement rules | Slope, buildable terrain, no units or buildings in the way, base adjacency (CNC), creep or power field (SC), distance to resources. The server checks. | All6, FAC | B | CORE | MP01 |
| CO04 | Construction model | Worker-built (SC terran, AOE, WC3, SUP); placed after a sidebar build (CNC); warped or summoned (SC protoss); morphed from a worker (SC zerg). | All6 | B | CORE | OR16, PR03 |
| CO05 | Worker build queue | Shift-queue several structures for one worker. | SC1, SC2, AOE, WC3, SUP | M | COMMON | OR12, CO01 |
| CO06 | Multi-worker build | Several workers on one site build faster. | AOE, SUP, WC3 (human) | B | OPTIONAL | CO04 |
| CO07 | Under construction state | A site has partial health. It can be attacked. It shows progress. | All6 | B | CORE | CB01 |
| CO08 | Cancel and refund | Cancel a site. A refund follows the mod rule (SC 75 %, CNC full). | All6 | B | CORE | PR03 |
| CO09 | Wall drag | Drag a line of wall segments. | AOE, CNC, SUP | M | COMMON | CO01 |
| CO10 | Gates | A wall segment that opens for own units. | AOE, CNC (TS) | B | OPTIONAL | CO09, PF07 |
| CO11 | Repair | A worker or a self-repair button restores health for a cost. | All6 | B | COMMON | CB01 |
| CO12 | Sell and deconstruct | Remove a structure for a refund (CNC sell, SUP reclaim, FAC mine). | CNC, SUP, FAC | B | COMMON | CO08 |
| CO13 | Building rotation | Rotate a structure before placement. | FAC, AOE DE (some) | M | OPTIONAL (CORE for FAC) | CO01 |
| CO14 | Foundation and flatten | The terrain under a footprint becomes flat. | AOE (flat need), CNC TS, SUP | B | OPTIONAL | MP01 |
| CO15 | Upgrade in place | A structure turns into its next tier (keep to castle, hatchery to lair). | WC3, SC1, SC2 | B | COMMON | PR08 |
| CO16 | Grid overlay | Show the build grid during placement. | SC2, AOE DE, FAC | M | COMMON | CO01 |
| CO17 | Line drag placement | Drag a line of belts, pipes, or rails. | FAC | M | OPTIONAL | CO09 |
| CO18 | Blueprints | Copy, paste, and store a group of structures. | FAC | M | OPTIONAL | CO01 |
| CO19 | Capture | An engineer takes over a building. | CNC, AOE (monk converts), SUP | B | OPTIONAL | OR20 |
| CO20 | Destruction clears footprint | A destroyed structure frees its cells, leaves rubble or a husk. | All6, FAC | B | CORE | CB01 |
| CO21 | Placement vs units | Placement pushes own units off the footprint, or it is invalid. | All6 | M | CORE | CO03, GM07 |
| CO22 | Build radius | Placement only near own base or power (CNC adjacency, SC pylon field). | CNC, SC1, SC2 | B | COMMON | CO03 |

## A8. Production and technology (PR)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| PR01 | Production queue | A building holds a queue of units or research (SC 5, AOE 15). | All6, FAC | B | CORE | none |
| PR02 | Cancel and refund | Remove a queue item. Refund by mod rule. | All6, FAC | B | CORE | PR01, PR03 |
| PR03 | Payment policy | Charge when queued, reserve then charge, or charge over time. | SC (queued), CNC (over time), AOE (queued) | P | CORE | EC01 |
| PR04 | Parallel production | Each building has its own queue (SC, AOE) or one queue per category speeded by more factories (CNC). | All6 | B | CORE | PR01 |
| PR05 | Spawn and exit | A new unit appears at an exit point. A blocked exit waits or finds a free cell. | All6, FAC | M | CORE | PF04 |
| PR06 | Rally applied | A new unit takes the rally order. | All6 | M | CORE | OR15, PR05 |
| PR07 | Prerequisites | An item needs a building or a research first. | All6, FAC | B | CORE | MD05 |
| PR08 | Research and upgrades | A one-time or levelled improvement (weapons +1, ages). | All6, FAC | B | CORE | PR07, PR15 |
| PR09 | Ages and tiers | A global tier change unlocks a set (AOE ages, SUP T1 to T3). | AOE, SUP, WC3, SC (implicit) | B | COMMON | PR08 |
| PR10 | Unavailable reason | A greyed item names the missing requirement or resource. | All6, FAC | B | CORE | UI09 |
| PR11 | Repeat production | A queue repeats for ever. | SUP, FAC | M | OPTIONAL | PR01 |
| PR12 | Batch queue | Shift-click queues five. | SC2, AOE, WC3, CNC | M | COMMON | PR01 |
| PR13 | Multi-building production | Several selected buildings share one production command. The engine sends each item to the shortest queue. | SC2, AOE DE, SUP | M | COMMON | PR01, OR24 |
| PR14 | Supply block | Production waits at the population cap. | SC1, SC2, AOE, WC3, SUP | B | COMMON | EC08 |
| PR15 | Modifier application | An upgrade changes stats of present and future units. Stacking follows a rule. | All6, FAC | B | CORE | MD05 |
| PR16 | Power effect | Low power slows production or disables buildings. | CNC, SUP, FAC, SC (pylons) | B | COMMON | EC09 |
| PR17 | Asymmetric factions | Each faction has its own tree. | All6 | P | CORE | MD06 |
| PR18 | Recipes | Inputs become outputs over time in a machine. | FAC | B | OPTIONAL | EC01 |
| PR19 | Exclusive branches | Picking one tech locks another (WC3 hero choice, SC2 mutually exclusive). | WC3, SC2 (co-op), AOE (none) | B | OPTIONAL | PR07 |

## A9. Economy (EC)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| EC01 | Resource types | The mod defines resources (minerals and gas; food, wood, gold, stone; ore; gold and lumber; mass and energy). | All6, FAC | B | CORE | MD01 |
| EC02 | Gather cycle | Move to node, gather, carry, return to drop-off, repeat. | SC1, SC2, AOE, WC3, CNC; SUP uses extractors | B | CORE | OR17, PF01 |
| EC03 | Drop-off choice | A gatherer goes to the nearest valid drop-off. | SC1, SC2, AOE, WC3, CNC | M | CORE | EC02 |
| EC04 | Node depletion | A node has a finite amount. It disappears when empty. | All6, FAC | B | CORE | EC02 |
| EC05 | Auto-resume and seek | A gatherer resumes after a drop and finds a new node when its node is empty. | SC1, SC2, AOE, WC3, CNC | M | CORE | EC04 |
| EC06 | Node occupancy | A node allows a fixed number of gatherers at a time. | SC1, SC2, WC3 | B | COMMON | EC02 |
| EC07 | Generators | Buildings that make a resource with no gatherer (SUP mass extractor, AOE trade, power plant). | SUP, AOE, CNC, FAC | B | COMMON | EC01 |
| EC08 | Population cap | A limit on army size raised by supply buildings. | SC1, SC2, AOE, WC3, SUP | B | COMMON | EC01 |
| EC09 | Power | A balance (CNC), a field (SC pylons), or a network (SUP, FAC). | CNC, SC, SUP, FAC | B | COMMON | EC01 |
| EC10 | Insufficient funds | Reject the order, or pause it until funds exist. | All6, FAC | P | CORE | PR03 |
| EC11 | Tribute and transfer | Give resources to an ally. | AOE, SC2, WC3, SUP | B | COMMON | TM02 |
| EC12 | Market | Exchange one resource for another at a moving rate. | AOE | B | OPTIONAL | EC01 |
| EC13 | Storage caps | A maximum stock per resource. | SUP, FAC, WC3 (none) | B | OPTIONAL | EC01 |
| EC14 | Regrowing resources | A field spreads or regrows (Tiberium, RA ore). | CNC | B | OPTIONAL | EC04 |
| EC15 | Upkeep | Income drops as the army grows. | WC3 | P | OPTIONAL | EC08 |
| EC16 | Physical items and logistics | Items exist on belts, in inventories, in trains. | FAC | B | OPTIONAL (CORE for FAC) | EC01 |
| EC17 | Income rate display | Income per minute shown. | SUP, FAC | M | OPTIONAL | UI04 |

## A10. Combat (CB)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| CB01 | Health and death | Each damageable entity has health. At zero it dies. | All6, FAC | B | CORE | none |
| CB02 | Weapons | Range, cooldown, damage, minimum range, target filter (ground, air, buildings). | All6, FAC (turrets) | B | CORE | CB01 |
| CB03 | Auto-target | An idle or attack-moving unit engages enemies in its acquisition range. | All6, FAC | M | CORE | CB02, FV03 |
| CB04 | Target priority | A rule picks the target (threats before buildings, attackers first). | All6 | B | CORE | CB03 |
| CB05 | Retaliation | A hit unit returns fire. Nearby friends help. | All6 | B | CORE | CB03 |
| CB06 | Leash | A unit stops a chase and returns to its post. | WC3, AOE, SC2, SUP | B | COMMON | OR08 |
| CB07 | Damage and armour types | A matrix of damage type against armour type. | All6 (SC1 sizes, CNC warheads, AOE classes, WC3 table) | B | CORE | MD07 |
| CB08 | Armour value | Flat or percent reduction. Upgradable. | SC1, SC2, AOE, WC3 | B | COMMON | CB07 |
| CB09 | Splash damage | Area damage with falloff. Friendly fire by rule. | All6 | B | CORE | CB02 |
| CB10 | Projectiles | Instant hit, ballistic, homing. Travel time. Can miss a moving target. | All6, FAC | B | CORE | CB02 |
| CB11 | Line of fire | Terrain or obstacles block a shot. | SUP, SC2 (none), CNC TS | B | COMMON | MP01 |
| CB12 | High ground | Attacker on low ground has a miss chance, no vision, or less damage. | SC1, SC2, WC3, AOE | B | COMMON | FV09 |
| CB13 | Turrets and facing | Turn rate, turret arc, fire while moving. | CNC, SUP, SC2, FAC | B | COMMON | CB02 |
| CB14 | Multiple weapons | One unit has several weapons with their own targets. | SUP, CNC, SC2, WC3 | B | COMMON | CB02 |
| CB15 | Overkill control | Units spread fire to avoid wasted shots. | SC2 (some), SUP | M | OPTIONAL | CB04 |
| CB16 | Shields and regeneration | A second health pool, regeneration over time. | SC1, SC2, WC3, SUP | B | COMMON | CB01 |
| CB17 | Healing | A unit or building restores health of others. | SC1, SC2, AOE, WC3, CNC | B | COMMON | CB01 |
| CB18 | Wrecks and corpses | Death leaves a wreck (SUP reclaim) or a corpse (WC3 raise dead). | SUP, WC3, AOE, SC1 | B | COMMON | CB01 |
| CB19 | Crush | Vehicles kill infantry by driving over them. | CNC | B | OPTIONAL | GM06 |
| CB20 | Accuracy | Hit chance per shot. | SC1, AOE, WC3, CNC | B | OPTIONAL | CB10 |
| CB21 | Defensive buildings | Buildings with weapons. | All6, FAC | B | CORE | CB02 |
| CB22 | Ammunition | Limited shots, reload at a pad or depot. | RA2, CNC, FAC | B | OPTIONAL | CB02 |
| CB23 | Friendly fire | A rule decides if area damage hurts own units. | CNC, SUP, AOE, SC | P | COMMON | CB09 |
| CB24 | Kill credit | The killer gets credit for score and veterancy. | All6 | M | COMMON | UN09, MF12 |
| CB25 | Bonus damage | Damage against a tag (anti-armour, anti-building). | All6 | B | CORE | CB07 |

## A11. Fog of war and vision (FV)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| FV01 | Black shroud | Unexplored areas are black. | All6, FAC | M | CORE | none |
| FV02 | Fog of war | Explored but not seen: terrain visible, units hidden. | All6, FAC | M | CORE | FV03 |
| FV03 | Sight radius | Each entity sees a radius. | All6, FAC | B | CORE | none |
| FV04 | Server-side visibility | The server sends a client only what that player can see. Anti map hack. | Needed for authoritative play; SC2 and SUP lockstep lack it | M | CORE (authoritative) | FV03 |
| FV05 | Shared vision | Allies see what their allies see. | All6 | B | CORE | TM02 |
| FV06 | Detection | Cloaked or stealth units are visible only to detectors. | SC1, SC2, WC3, RA2, SUP | B | COMMON | FV03 |
| FV07 | Reveal | A temporary reveal of an area (scan, flare, radar). | SC1, SC2, WC3, CNC, SUP | B | COMMON | FV03 |
| FV08 | Radar gating | The minimap needs a building and power. | CNC | B | OPTIONAL | UI05, EC09 |
| FV09 | Terrain occlusion | Height and obstacles block sight. | SC1, SC2, WC3, SUP | M | COMMON | MP01 |
| FV10 | Last-seen structures | Enemy buildings stay as ghosts in fog as last seen. | All6 | M | CORE | FV02 |
| FV11 | Fog lobby options | Normal, explored, all visible. | AOE, WC3, SC2, SUP | P | COMMON | MF04 |
| FV12 | Shroud generator | A building re-shrouds an area for enemies. | CNC | B | OPTIONAL | FV01 |
| FV13 | Air vs ground sight | Air units see over cliffs. | SC2, WC3 | B | OPTIONAL | FV09 |

## A12. Units (UN)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| UN01 | Ground units and movement classes | Infantry, vehicles, each with a terrain class. | All6, FAC | B | CORE | PF03 |
| UN02 | Air units | Fly over terrain on their own layer. | SC1, SC2, CNC, WC3, SUP | B | COMMON | PF03 |
| UN03 | Naval units | Move on water. | CNC, AOE, WC3, SUP | B | COMMON | PF03 |
| UN04 | Amphibious and hover | Move on land and water. | CNC, SUP | B | OPTIONAL | UN03 |
| UN05 | Transports | Units carry units. | All6 | B | COMMON | OR19 |
| UN06 | Garrison | Units enter a building and change its behaviour. | AOE, RA2, WC3 (burrow), SC (bunker) | B | COMMON | OR19 |
| UN07 | Abilities | Energy or cooldown actions with targets. | All6 | B | CORE | MD04 |
| UN08 | Heroes | Unique units with levels, skills, and items. | WC3 | B | OPTIONAL | UN09 |
| UN09 | Veterancy | Units gain levels from kills. | CNC, SUP, WC3 | B | COMMON | CB24 |
| UN10 | Line upgrades | A research upgrades all units of a line in place. | AOE, SC2 | B | COMMON | PR15 |
| UN11 | Workers | Units that gather and build. | All6 (CNC: harvester plus MCV) | B | CORE | EC02, CO04 |
| UN12 | Cloak and burrow | Hidden units. | SC1, SC2, WC3, RA2, SUP | B | COMMON | FV06 |
| UN13 | Death and removal | Death animation, then removal, then a corpse timer. | All6, FAC | B | CORE | CB01 |
| UN14 | Neutral creeps and critters | Neutral hostile or passive units. | WC3, AOE (animals) | B | OPTIONAL | TM01 |
| UN15 | Capturable neutral objects | Relics, oil derricks, taverns. | AOE, RA2, WC3 | B | OPTIONAL | CO19 |
| UN16 | Conversion | A unit takes control of an enemy unit. | AOE, RA2 | B | OPTIONAL | OR20 |
| UN17 | Self-repair and regeneration | Health returns over time. | SC1, SC2, WC3, SUP | B | COMMON | CB16 |
| UN18 | Deployables | A unit turns into a building (MCV). | CNC, SC (zerg) | B | COMMON | OR27 |
| UN19 | Superweapons | Long-cooldown map-wide attacks. | CNC, SUP, SC (nuke) | B | COMMON | OR20 |
| UN20 | Per-type caps | A maximum count per type. | WC3, RA2, SUP, AOE (some) | B | COMMON | PR07 |
| UN21 | Spawners | Units or buildings that make units (carrier, mobile factory). | SC1, SC2, SUP | B | OPTIONAL | PR01 |
| UN22 | Air landing and refuel | Aircraft land at pads to rearm. | RA2, CNC, SUP | B | OPTIONAL | CB22 |

## A13. Pathfinding and navigation as the player sees it (PF)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| PF01 | Sensible path | A unit takes a short, plausible path to a reachable point. | All6, FAC | M | CORE | MP01 |
| PF02 | Repath on change | A new building or a wall changes the paths of units in motion. | All6, FAC | M | CORE | PF01 |
| PF03 | Movement layers | Land, water, air, hover; cliffs impassable. | All6 | B | CORE | MP01 |
| PF04 | No stuck units | A blocked unit recovers or gives up visibly. | All6, FAC | M | CORE | PF01 |
| PF05 | Instant start | A unit starts to move in the same tick it gets the order. No path delay is visible. | All6 | M | CORE | PE01 |
| PF06 | Clearance | Big units do not use gaps too small for them. | All6 | M | COMMON | GM06 |
| PF07 | Ally and gate passage | Own gates open. Ally gates by rule. | AOE, CNC TS | B | OPTIONAL | CO10 |
| PF08 | Terrain speed | Roads, slopes, and terrain types change speed. | SUP, CNC TS, FAC | B | OPTIONAL | MP01 |
| PF09 | Turn radius | Vehicles turn with a radius or a rate. | SUP, CNC, SC2 (turn rate) | B | COMMON | PF01 |
| PF10 | Mass pathing | Hundreds of units path at once with no hitch. | All6, FAC | M | CORE | PE03 |
| PF11 | Shared route display | Show the path a unit will take. | SUP | M | OPTIONAL | OR13 |

## A14. Match flow (MF)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| MF01 | Main menu | Skirmish, multiplayer, replays, options, mods. | All6, FAC | M | CORE | none |
| MF02 | Lobby | Create, join, slots, ready, team, colour, faction, start position. | All6, FAC | B | CORE | MD12 |
| MF03 | Map choice | List with preview and player count. | All6, FAC | B | CORE | MP05 |
| MF04 | Game settings | Starting resources, tech level, fog, speed, victory type. Server validates. | All6, FAC | B | CORE | MD12 |
| MF05 | AI in slots | A computer player in any slot, with a difficulty. | All6 | B | CORE | BT01 |
| MF06 | Start conditions | Starting units per faction. Fixed or random start positions. | All6, FAC | P | CORE | MP01 |
| MF07 | Load and countdown | All players load. The match starts at once for all. | All6, FAC | M | CORE | MP07 |
| MF08 | Victory and defeat | Rules: destroy all, destroy key buildings, regicide, wonder, relics, score, time. | All6, FAC (launch) | P | CORE | MD10 |
| MF09 | Defeated player observes | A defeated player can watch to the end. | SC2, AOE, WC3, SUP | B | COMMON | RO03 |
| MF10 | Team victory | Allies win together. | All6 | B | CORE | TM01 |
| MF11 | In-game diplomacy | Change alliance during play. | AOE, SUP, WC3 (melee option) | B | OPTIONAL | TM02 |
| MF12 | Score screen | Post-game statistics: units, resources, kills, graphs, APM. | All6 | B | CORE | none |
| MF13 | Matchmaking and ladder | Find opponents, rank. | SC2, AOE DE, WC3 | M | OPTIONAL | MF02 |
| MF14 | Disconnect handling | Wait dialog, drop vote, AI takeover, or abandon. | All6, FAC | B | CORE | none |
| MF15 | Reconnect | A dropped player rejoins the match. | AOE DE, SUP (FAF), SC2 (via replay) | M | COMMON | MF14 |
| MF16 | Campaign and scenarios | Scripted missions with objectives and dialogue. | All6 | B | COMMON | MP04 |
| MF17 | Handicap | Resource or health handicap per player. | SC2, WC3, AOE DE | P | OPTIONAL | MF04 |
| MF18 | Random faction and map | Random choices in lobby. | All6 | B | COMMON | MF02 |
| MF19 | Co-op vs AI | Players team up against AI. | All6 | B | COMMON | MF05 |

## A15. Replays and observers (RO)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| RO01 | Replay recording | Every match can be saved and played back. | SC1, SC2, WC3, AOE, SUP | M | COMMON | none |
| RO02 | Replay controls | Speed, pause, seek, per-player vision. | SC2, AOE DE, SUP, WC3 | M | COMMON | RO01 |
| RO03 | Live observers | Spectators join a match, with a delay to block cheating. | SC2, AOE, WC3, SUP | M | COMMON | FV04 |
| RO04 | Observer UI | Production, income, army value tabs. | SC2, AOE DE, SUP | B | OPTIONAL | RO03 |
| RO05 | Replay version safety | A replay states its engine and mod versions and refuses a mismatch. | SC2, AOE DE | M | COMMON | RO01 |

## A16. Saves (SV)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| SV01 | Single-player save and load | Save and load a skirmish or a mission. | All6, FAC | M | CORE | none |
| SV02 | Multiplayer save and load | Save a multiplayer match and resume with the same players. | AOE, WC3, SC1, CNC (RA2), FAC | M | COMMON | SV01 |
| SV03 | Autosave | Periodic saves. | FAC, AOE DE | M | OPTIONAL | SV01 |
| SV04 | Save version safety | A save names its engine and mod versions. | All with saves | M | CORE | SV01 |
| SV05 | Mod save data | Mod state is in the save. | WC3, FAC | B | CORE | SV01, MD14 |

## A17. Maps (MP)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| MP01 | Map format | Terrain, heights, textures, water, cliffs, resources, start positions, decoration. | All6, FAC | B | CORE | none |
| MP02 | Map editor | A tool to make maps. | SC1, SC2, WC3, AOE, SUP, CNC (fan), FAC | M | CORE (moddable engine) | MP01 |
| MP03 | Random map generator | Script-driven maps (AOE RMS, FAC). | AOE, FAC, CNC (RA2), SUP (FAF) | B | COMMON | MP01 |
| MP04 | Triggers and map scripts | Events, conditions, actions per map. | SC1, SC2, WC3, AOE, SUP, FAC | B | COMMON | MD14 |
| MP05 | Map metadata | Name, author, player count, size, preview image. | All6, FAC | M | CORE | MP01 |
| MP06 | Neutral objects | Trees, rocks, destructible obstacles. | All6, FAC | B | COMMON | MP01 |
| MP07 | Map distribution | A client downloads a missing map from the host. | SC1, WC3, AOE, SUP | M | CORE | MP01 |
| MP08 | Map validation | Start positions reachable. Resources placed. | SC2, AOE | M | COMMON | MP01 |
| MP09 | Playable bounds | Map size and the area units can enter. | All6 | B | CORE | MP01 |
| MP10 | Custom game modes | A map carries its own rules (WC3 tower defence, DotA). | SC1, SC2, WC3, AOE | B | COMMON | MP04 |

## A18. Modding surface (MD): what a modder edits

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| MD01 | Unit definition | Stats, cost, build time, prerequisites, model, icon, voice, hotkey, abilities. | All6, FAC | B | CORE | none |
| MD02 | Weapon and projectile definition | Damage, range, rate, projectile, effects. | All6, FAC | B | CORE | MD01 |
| MD03 | Building definition | Footprint, production list, power, placement rules. | All6, FAC | B | CORE | MD01 |
| MD04 | Ability definition | Target type, cost, cooldown, effect. | SC2, WC3, SUP, CNC | B | CORE | MD20 |
| MD05 | Upgrade and tech definition | Prerequisites, cost, modifiers. | All6, FAC | B | CORE | MD01 |
| MD06 | Faction definition | Roster, start units, tech tree. | All6 | B | CORE | MD01 |
| MD07 | Damage and armour tables | The type matrix. | All6 | B | CORE | MD02 |
| MD08 | UI definitions | Command card layout, icons, hotkeys, cursors. | SC2, WC3, FAC | B | CORE | UI01 |
| MD09 | Audio definitions | Voice sets, alerts, music. | All6, FAC | B | CORE | AU01 |
| MD10 | Victory definitions | Victory rules as mod content. | SC2, WC3, AOE, SUP | B | CORE | MF08 |
| MD11 | AI definitions | Build orders, roles, priorities for bots. | All6 | B | CORE | BT01 |
| MD12 | Lobby settings schema | Typed options the lobby shows and the server validates. | SC2, WC3, AOE, SUP, FAC | B | CORE | MF04 |
| MD13 | String tables | Localised text. | All6, FAC | B | COMMON | none |
| MD14 | Event scripting API | Mod code reacts to events (unit died, building done). | SC2, WC3, SUP (Lua), FAC (Lua) | B | COMMON | none |
| MD15 | Fast iteration | Hot reload or quick restart with changed data. | SC2, FAC | M | COMMON | none |
| MD16 | Data inheritance | A definition inherits from a parent and overrides fields. | SC2, SUP, FAC | M | COMMON | MD01 |
| MD17 | Asset pipeline | Import models, textures, animations, sounds. | All6, FAC | M | CORE | none |
| MD18 | Validation messages | Errors name the file, the field, and the cause. | SC2, FAC | M | CORE | none |
| MD19 | Mod layering | One small mod changes one unit of another mod. | SUP, FAC, SC2 | M | COMMON | MD16 |
| MD20 | Effect system | Data-driven effects (damage, apply buff, spawn, search area) composed in trees. | SC2, WC3, SUP | B | COMMON | MD02 |

## A19. Bots and computer opponents (BT)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| BT01 | Skirmish AI | Builds an economy, an army, attacks, defends. | All6 | B | CORE | MD11 |
| BT02 | Difficulty levels | Several strengths. | All6 | B | CORE | BT01 |
| BT03 | Same command path | AI uses the player command API. Cheats are explicit. | Varies | B | CORE | OR24 |
| BT04 | Ally AI cooperation | The AI answers pings and requests. | AOE, SC2 | B | OPTIONAL | UI07 |
| BT05 | Unit micro AI | Auto-retreat, focus fire, autocast. | SC2, WC3 | B | OPTIONAL | OR21 |

## A20. Teams and diplomacy (TM)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| TM01 | Teams | Teams set in the lobby. | All6, FAC | B | CORE | MF02 |
| TM02 | Alliance effects | No auto-target on allies, shared vision, allied victory. | All6 | B | CORE | TM01 |
| TM03 | Player colours | Each player has a distinct colour on units and minimap. | All6 | B | CORE | AN07 |

## A21. Audio (AU)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| AU01 | Unit responses | Voice lines on select and on order. | All6 | B | CORE | PC01, OR23 |
| AU02 | Alerts | Voice or sound for events. | All6, FAC | B | CORE | UI16 |
| AU03 | Positional effects | Weapon, impact, death sounds placed in space. | All6, FAC | M | CORE | none |
| AU04 | Music | Playlist or dynamic music. | All6, FAC | B | COMMON | none |
| AU05 | Ambient sound | Wind, water, birds per terrain. | WC3, SC2, AOE DE, FAC | B | OPTIONAL | MP01 |
| AU06 | Volume categories | Separate music, effects, voice, alerts. | All6, FAC | M | CORE | UI22 |
| AU07 | Sound limiting | A cap and priority so that 200 guns do not flood the mix. | All6 | M | CORE | AU03 |
| AU08 | Repeat-click lines | Special lines after many clicks. | SC1, SC2, WC3 | P | OPTIONAL | AU01 |

## A22. Animation and visual effects (AN)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| AN01 | Unit animations | Idle, move, attack, death, gather, build. | All6, FAC | B | CORE | MD17 |
| AN02 | Weapon effects | Muzzle flash, projectile, impact, explosion. | All6, FAC | B | CORE | CB10 |
| AN03 | Construction visuals | Progress shown on the structure. | All6, FAC | B | CORE | CO07 |
| AN04 | Damage states | Smoke and fire on hurt structures. | All6 | B | COMMON | CB01 |
| AN05 | Death visuals | Death animation, debris, corpse. | All6, FAC | B | CORE | UN13 |
| AN06 | Decals for orders | Selection circles, move and attack markers. | All6, FAC | M | CORE | none |
| AN07 | Team colour on models | Mask-based player colour. | All6 | M | CORE | none |
| AN08 | Tick interpolation | Smooth motion between simulation ticks. | All6 | M | CORE | none |
| AN09 | Terrain decals | Scorch marks, tracks. | SC2, SUP, CNC, FAC | B | OPTIONAL | MP01 |
| AN10 | Moddable particles | Mods define particle effects. | SC2, WC3, SUP, FAC | B | COMMON | MD17 |
| AN11 | Instancing and level of detail | Many units drawn cheaply. | SUP, SC2, AOE DE, FAC | M | CORE | PE02 |
| AN12 | Day and night | Light cycle with vision effect. | WC3, FAC | B | OPTIONAL | FV03 |
| AN13 | Weather | Rain, snow. | CNC (RA2 storm), FAC | B | OPTIONAL | none |

## A23. Accessibility and settings (AC)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| AC01 | Rebindable keys | Change every key. Presets. | SC2, AOE DE, SUP, FAC, WC3 (Reforged) | M | COMMON | IN02 |
| AC02 | Colour-blind support | Colour-blind palettes, team colour override (own green, enemy red). | SC2, AOE DE, FAC | B | COMMON | TM03 |
| AC03 | UI scale | Scale the interface. | SC2, AOE DE, FAC | M | COMMON | UI22 |
| AC04 | Subtitles and text size | Subtitles for voice. Large text. | SC2, AOE DE | M | OPTIONAL | AU01 |
| AC05 | Cursor confine | Keep the cursor in the window for edge scroll. | All6 (windowed), browser pointer lock | M | CORE | CA02, IN08 |
| AC06 | Graphics settings | Quality, resolution, effects. | All6, FAC | M | CORE | none |
| AC07 | Reduced motion | No screen shake, fewer flashes. | AOE DE, FAC | M | OPTIONAL | none |

## A24. Performance the player notices (PE)

| ID | Item | Definition | Games | Kind | Tier | Depends on |
| --- | --- | --- | --- | --- | --- | --- |
| PE01 | Order responsiveness | Click gives local feedback in the same frame. Units visibly react within about 100 to 250 ms online. | All6 | M | CORE | OR23, AN08 |
| PE02 | Stable frame rate | Frame rate holds with full armies on screen. | All6, FAC | M | CORE | AN11 |
| PE03 | Unit count | SC 200 supply per player, AOE 200 pop, SUP 1,000+ per player, FAC thousands of entities. | All6, FAC | M | CORE | PF10 |
| PE04 | Large selection speed | Box-selecting hundreds of units has no hitch. | SUP, AOE, FAC | M | CORE | PC02 |
| PE05 | Load time | Fast map and mod load. | All6, FAC | M | COMMON | none |
| PE06 | Late-game slowdown | The engine degrades gracefully (SUP sim speed). | SUP, FAC | M | COMMON | none |
| PE07 | Latency hiding | Local feedback hides network delay. | All6 | M | CORE | OR23 |

## A25. Counts for Part A

Counted with grep over the table rows of this file. See "What I ran". Totals are in the last
section.

---
# Part B: First look at the spec

Source: `RTS-Engine-Project-Specifications.md` version 0.7 (S:3), read once from line 1 to
line 809. "S:n" is a line in that file. Raw grep output for every term group:
`/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/review/inventory-engine-greps.txt`.

Method. I classified each of the 181 CORE items into one of three bins:

- NEVER: no spec line names the item. A grep hit that is a different sense of the word does
  not count. Example: "rally" hits only "literally" (S:366); "select" hits only library,
  server, and bot selection, never unit selection.
- WORD ONLY: the spec names the item as a word, usually in a table cell, with no behaviour,
  no rule, and no owner of the design.
- COVERED: the spec states a behaviour, a rule, or a decision for the item. I give the status.

Result: NEVER 77, WORD ONLY 61, COVERED 43. Total 181.

## B1. Headline

The spec plans the server, the package system, accounting, navigation scale, and rendering
scale. It does not plan the player's hands. The whole player-control layer is absent:
input, selection, control groups, hotkeys, camera controls, the order model, the game
interface, audio, and health and death.

- The Presentation module is one table row: "Bevy rendering, input, audio, UI, overlays,
  visual interpolation" (S:122). No section expands it.
- The Interface domain is one table row: "UI primitives and controlled state access" against
  "Resource displays, command panels, tooltips, screens" (S:164). No section expands it.
- No milestone M1 to M13 demonstrates player control (S:674 to S:688). M1 accepts "presents
  server results" (S:676). M13 checks "picking" only as a crowd-render correctness item
  (S:688, S:610).
- The first playable mod (S:724 to S:751) needs selection, orders to groups, a build menu,
  placement, health, and alerts. The spec names "Move, attack, attack-move, and automatic
  targeting" as a mod rule (S:747) and nothing that lets a player issue them.
- "health" has zero hits. The first loop has a turret, a tank, and a repair rule (S:742), but
  the spec never names the quantity that combat lowers and repair restores.
- Audio is one word (S:122). "sound", "voice", "music" have zero hits.

## B2. CORE items the spec never mentions (77)

| Area | Items |
| --- | --- |
| Input (5) | IN03 modifier semantics; IN04 double-click detection; IN05 hotkey per command button; IN08 browser input capture (right-click menu, Ctrl+1..9 tab switch, Ctrl+W, F5); IN09 text input mode for chat |
| Selection and control (13) | PC01 single select; PC02 box select; PC04 shift add/remove; PC05 type select (double-click, Ctrl-click); PC08 control group assign/recall; PC09 control group add; PC10 double-tap group centres camera; PC16 inspect foreign entities; PC19 selection hygiene; PC20 hover highlight; PC21 context cursor; PC22 smart right-click command; PC23 selection visual |
| Orders (7) | OR02 stop; OR05 force move; OR08 hold position; OR12 shift-queued orders; OR15 rally point; OR25 group destination spread; OR26 order state machine (what interrupts what) |
| Group movement (4) | GM01 destination spreading; GM07 push idle friends; GM09 arrival without jitter; GM10 unreachable target fallback |
| Camera (7) | CA02 edge scroll; CA03 key scroll; CA08 minimap click; CA09 minimap order; CA14 centre on selection; CA15 height-aware camera; CA16 scroll speed setting |
| Interface (11) | UI02 selection panel; UI03 info panel and portrait; UI08 minimap alert flash; UI09 build menu; UI10 production queue display; UI12 health bars; UI14 fog rendering; UI17 chat; UI19 surrender and leave; UI22 in-game options menu; UI23 cursor set |
| Construction (2) | CO01 placement ghost; CO20 destruction clears footprint |
| Production (2) | PR06 rally applied to new units; PR10 unavailable reason on greyed items |
| Economy (1) | EC05 gatherer auto-resume and auto-seek of a new node |
| Combat (4) | CB01 health and death; CB04 target priority; CB05 retaliation; CB25 bonus damage |
| Vision (2) | FV01 black shroud; FV10 last-seen structures (S:780 has "remembered past sightings" for bots only) |
| Units (1) | UN13 death and removal |
| Match flow (2) | MF01 main menu; MF12 score screen (S:272 "persist afterward for results" names a room, not a screen) |
| Maps (2) | MP05 map metadata; MP09 playable bounds |
| Modding (1) | MD09 audio definitions |
| Teams (1) | TM03 player colours (the only "color" hit is the utility overlay, S:531) |
| Audio (5) | AU01 unit responses; AU02 alerts; AU03 positional effects; AU06 volume categories; AU07 sound limiting |
| Animation (4) | AN03 construction visuals; AN05 death visuals; AN06 order and selection decals; AN07 team colour on models |
| Accessibility (2) | AC05 cursor confine (pointer lock); AC06 graphics settings |
| Performance (1) | PE04 large selection speed |

## B3. CORE items named only as a word (61)

| ID | Item | Where | Why it is word only |
| --- | --- | --- | --- |
| IN01 | Picking | S:610, S:688 | "Validate ... picking" for instanced crowds. No picking design. |
| IN02 | Action mapping | S:122; S:77, S:99 | "input" in a table; "keyboard and mouse first". No input model. |
| OR01, OR03, OR04 | Move, attack, attack-move | S:747 | A row of the default-mod rule table. No order model. |
| OR11 | Order queue | S:121, S:171 | "orders" in a table and in a tank example. No queue. |
| OR16 | Build order | S:140, S:447 | "construction or excavation orders"; terrain work orders. |
| OR17 | Gather order | S:140, S:743 | Composition word and mod rule. |
| OR20 | Ability order | S:160 | "Orders, targeting, timers, effects" in a table. |
| OR22 | Rejection feedback | S:237, S:782 | "authority validates"; "rejected-command reasons" is a proposed bot debug aid. No player feedback. |
| OR23 | Acknowledgement | S:243 | "immediate input acknowledgement" (Decided paragraph, baseline phrase). No content. |
| OR24 | Group order fan-out | S:342 | "Support group orders, batching" under **Proposed** platform controls. A cost item, not a player feature. |
| GM06 | Collision size | S:156, S:626, S:630 | "collision"; "actor-radius classes"; "mixed footprints". |
| CA01 | Classic camera | S:60 (D37), S:101, S:701 | "expected for the default game"; camera API is Open (S:701). |
| CA13 | Bounds clamp | S:101 | "motion constraints". |
| UI01 | Command card | S:164 | "command panels". |
| UI04 | Resource bar | S:164 | "Resource displays". |
| UI05 | Minimap | S:338 | "minimap data may still be required" (replication). |
| UI11 | Tooltips | S:164 | "tooltips". |
| UI16 | Event alerts | S:338, S:634 | "alerts" must be replicated. No alert list. |
| CO02 | Footprint validity display | S:436 | "Smooth rendering must not conceal ... placement rules from the player." |
| CO07 | Under construction | S:158, S:745 | "progress"; "progresses over time". |
| CO08 | Construction cancel | S:406 | Only production cancellation is named. |
| CO21 | Placement vs units | S:158 | "occupancy". |
| PR04 | Parallel production | S:410 | "without assuming identical queue semantics". |
| PR05 | Spawn and exit | S:142, S:406 | "unit-spawn output"; "output blockage". |
| PR17 | Asymmetric factions | S:163 | "Factions". |
| EC02 | Gather cycle | S:140, S:180, S:743 | Composition words and the mod rule. No cycle. |
| EC03 | Drop-off choice | S:743 | Mod rule: deliver to command center. |
| CB02 | Weapons | S:144, S:159 | Table cells. |
| CB03 | Auto-target | S:747 | Mod rule row. No acquisition rule. |
| CB07 | Damage and armour types | S:159 | Table cell. |
| CB09 | Splash | S:180 | "area damage". |
| CB10 | Projectiles | S:159 | "projectile support". |
| CB21 | Defensive buildings | S:144, S:746 | Turret composition; disabled by low power. |
| FV02 | Fog of war | S:162, S:634 | "fog-of-war mechanisms"; filtering order. |
| FV03 | Sight radius | S:162 | "Sight ranges". |
| FV05 | Shared vision | S:162 | "shared vision". |
| UN01 | Movement classes | S:156, S:470 | "movement categories"; traversal limits. |
| UN07 | Abilities | S:160 | Table row. |
| UN11 | Workers | S:140 | Composition row. |
| PF03 | Movement layers | S:156, S:113 | Categories; flying units "require a separate design". |
| PF05 | Instant start | S:630 | "explicit pending states" for path queues. No player rule. |
| MF03 | Map choice | S:290 | "map" as a settings example. |
| MF07 | Load and countdown | S:311 | Countdown is "suggestions only"; barrier is implementation. |
| MF10 | Team victory | S:748 | "player or team". |
| SV05 | Mod save data | S:714 | "dynamic mod-state serialization" is Open. |
| MP01 | Map format | S:179, S:213 | "maps" in definitions; "serializable map state". |
| MP02 | Map editor | S:447, S:720 | "map editing" edits; "Do not precommit to a large manual editor suite" (Decided). |
| MD01, MD03 | Unit, building definitions | S:179 | Word list. No field list. |
| MD02, MD07 | Weapon, damage tables | S:159 | Table cells. |
| MD04 | Ability definitions | S:160 | Table cell. |
| MD06, MD10 | Faction, victory definitions | S:163 | Table cell. |
| MD08 | UI definitions | S:164, S:179 | "UI data". |
| BT02 | Difficulty | S:776, S:780 | Open. |
| TM01 | Teams | S:163, S:290 | Table cell; settings example. |
| TM02 | Alliance effects | S:162 | "shared vision" only. |
| PE01 | Order responsiveness | S:243, S:560 | Baseline phrase; latency thresholds Open. |

## B4. CORE items the spec covers (43)

| ID | Item | Cite | Status |
| --- | --- | --- | --- |
| PC17 | Ownership gate | S:305, S:324 to S:328 | Decided |
| OR31 | Command rate limit | S:342 | Proposed |
| GM04 | Local avoidance | S:86 (D63), S:624 | Baseline |
| GM11 | Choke traffic | S:624, S:630, S:653 | Baseline (test case) |
| CO03 | Placement rules | S:158, S:460, S:470, S:745 | Decided principle; rules mod policy |
| CO04 | Construction model | S:158, S:745 | Decided (mod policy) |
| PR01 | Production queue | S:142, S:406, S:410 | Baseline |
| PR02 | Cancel and refund | S:406, S:707 | Open policy, must be explicit |
| PR03 | Payment policy | S:406, S:707 | Open policy (three options) |
| PR07 | Prerequisites | S:161, S:424 | Baseline |
| PR08 | Research | S:424 to S:428 | Baseline; policies Open |
| PR15 | Modifiers | S:426, S:428 | Stacking Open |
| EC01 | Resource types | S:394 to S:400 | Baseline |
| EC04 | Depletion | S:478 to S:480, S:542 | Geology only; surface ore fields not stated |
| EC10 | Insufficient funds | S:406 | Open policy |
| FV04 | Server-side visibility | S:338, S:496, S:634 | Baseline |
| PF01 | Sensible path | S:622 to S:630 | Baseline |
| PF02 | Repath on change | S:459, S:630 | Baseline |
| PF04 | No stuck units | S:630 | Baseline (terrain edits only) |
| PF10 | Mass pathing | S:566, S:624, S:653 | Baseline |
| MF02 | Lobby | S:286 to S:305 | Decided |
| MF04 | Game settings | S:290 | Baseline schema |
| MF05 | AI in slots | S:301, S:786 | Decided |
| MF06 | Start conditions | S:317, S:332, S:749 | Decided (mod policy) |
| MF08 | Victory | S:163, S:290, S:748 | Decided (mod policy) |
| MF14 | Disconnect | S:299, S:326 to S:332, S:786 | Decided |
| SV01 | Saves | S:64 (D41), S:253 | Decided: server saves only, no local save |
| SV04 | Save version safety | S:65 (D42), S:253 | Decided |
| MP07 | Map distribution | S:267, S:276 | Decided |
| MD05 | Tech definitions | S:426 | Baseline |
| MD11 | AI definitions | S:759 | Decided |
| MD12 | Settings schema | S:290 | Baseline |
| MD17 | Asset pipeline | S:225 to S:229 | Baseline |
| MD18 | Validation messages | S:209, S:229 | Baseline |
| BT01 | Skirmish AI | S:755 to S:776 | Decided |
| BT03 | Same command path | S:761, S:774, S:780 | Baseline |
| AN01 | Unit animations | S:594 to S:608 | Baseline |
| AN02 | Weapon effects | S:612 | Baseline (budgets) |
| AN08 | Interpolation | S:243, S:636 | Baseline |
| AN11 | Instancing and LOD | S:596 to S:610 | Baseline |
| PE02 | Frame rate | S:650, S:662 | Thresholds Open |
| PE03 | Unit count | S:52, S:80, S:578 to S:586 | Decided benchmarks |
| PE07 | Latency hiding | S:243, S:636 | Baseline |

## B5. Pattern

- Covered items cluster in networking, packages, accounting, navigation scale, rendering
  scale, and bots. These are the areas the spec author researched (S:590 to S:668).
- Never-mentioned items cluster in what a player touches every second: input, selection,
  groups, camera controls, interface, audio, health and death.
- A browser-first engine has one extra CORE risk with no line in the spec: the browser
  reserves Ctrl+1..9, Ctrl+W, and F5. RTS control groups use Ctrl+1..9 (PC08). IN08 needs a
  design before PC08 can work in the primary product (D02, S:25).
- Open status matters: S:701 lists the camera API as Open. That is the only player-control
  item that the spec lists as an open decision. Selection, orders, hotkeys, and interface are
  not even in the open-decision table (S:696 to S:714).

## What I ran

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| `rtk proxy grep -c '^\| [A-Z][A-Z][0-9][0-9] \|' inventory-engine.md` and a tier `awk` | Part A has N items split by tier | A count that differs from the table rows | 355 rows: CORE 181, COMMON 110, OPTIONAL 64 |
| 45 term-group greps, `rtk proxy grep -n -i -o -E <pattern> S` (patterns in the greps file) | Each B2 item has no spec line; each B3/B4 cite exists | A hit in the right sense for a B2 item | Output in `inventory-engine-greps.txt`; hits for B2 items are other senses only |
| `rtk proxy grep -n -i -E` for select, minimap, health, chat, hotkey, control group, rally, patrol, queue, surrender, score, sound, voice, shroud, death, menu, colou?r, `\bUI\b`, order | The player-control layer is absent | Any hit that names unit selection, hotkeys, chat, health, audio, or a menu | Zero hits for health, chat, hotkey, control group, patrol, surrender, score, sound, voice, shroud, death, menu; select, rally, colour hit other senses only |
| Read S:1 to S:809 | Every B3 and B4 cite says what the row claims | A cited line that does not hold the stated text | Each cite read in context |

Plain `grep` is filtered by the rtk hook in this environment. A first count with plain grep
returned 0 per tier. I re-ran every grep with `rtk proxy grep`.

## Totals

Part A: 355 items. CORE 181. COMMON 110. OPTIONAL 64.
Part B (CORE only): NEVER 77. WORD ONLY 61. COVERED 43.
