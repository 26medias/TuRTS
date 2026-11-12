# MP1: Playable skirmish for two people: design

Date: 2026-11-12. Revision 4 (repair pass in progress: section 6.7 and the lifecycle rows are repaired; the other parts follow). Status: DRAFT. Gate 1 ran on revision 3 with six lenses: 23 High findings (docs/research/mp1/gate1-*.md). Revision 2: section 6.9 from probe MP1-P1. Revision 3: the Yardstick coverage and Player path sections (qa-lead; the completeness gate passes), 12 new acceptance rows, section 16.
Author: the orchestrator, from five seat inputs under `docs/research/mp1/` (producer, game-designer,
technical-director, ux-designer, bevy-rendering-engineer). The inputs are agent output. This design takes
their conclusions; gate 1 checks each claim that the design depends on.

Short references: `S:n` is `RTS-Engine-Project-Specifications.md` line n. `GD`, `TD`, `UX`, `RE`, `PR` are the
five input files (`input-game-designer.md`, `input-technical-director.md`, `input-ux-designer.md`,
`input-bevy-rendering-engineer.md`, `input-producer.md`). `MCD` is the MC design
(`2026-10-29-mc-player-control-design.md`, revision 16).

## 1. Goal and exit

MP1 is the first real game on the engine. Roadmap row (S:723): two people finish a match with mouse and
keyboard; both build, power and fight; both see the result screen. The mod is the first playable mod of
specification section 16.2: one faction, three infantry types, one resource and a harvester, one tank, one
turret, a command centre, a power station (S:782-805). The ground is flat (G15). There is no bot, no fog of
war, and no terrain.

MP1 is done when:

1. Every acceptance row of section 12 is green, each with its recorded red.
2. Two people play one full match to the result screen and one person signs off (section 13).
3. The technical-director audit of the whole MP1 change finds no structural defect that is open.
4. The gates and the hosted CI are green in the form that the CEO decides for MC (decision note
   `vault/Decisions/2026-11-11 MC hosted CI without a GPU.md`).
5. `docs/measurements/mp1.md` records the numbers of section 12 (recorded, not judged).

## 2. Decisions for the CEO before gate 1 closes

Each line has the team recommendation. The design below assumes the recommendation; a different choice
changes the named sections.

| Id | Question | Options | Recommendation | Changes |
| --- | --- | --- | --- | --- |
| P1 | Real models in MP1? | (a) simple shapes; (b) one static model and one texture set for each unit and building, loaded from the mod package, with the shape as fallback; (c) wait for M13 | (b), with a cap of 9 models; the engine side is 5 to 7 runs because the model loader is already in the web build (RE part 1) | 8, 14 |
| P2 | Lobby | (a) two slots, ready only, team = slot; (b) team and colour choice | (a) for the exit; colour choice is cheap and stays; team choice moves to the milestone that needs teams (PR 5c) | 7.6 |
| P3 | Scope cuts to MP2 | engineer repair, rally point, cancel of construction, charge over time | Keep all four in MP1. They are not on the critical path (PR part 1), the engineer has no function without repair, and S:798 is Decided. Cut them only if the schedule slips, in this order: rally, cancel of construction, repair | 4, 6 |
| P4 | Payment | (a) pay at the order; (b) pay over time while the job runs | (b) (GD part 2, S:723, S:434); it is what makes "On hold: no credits" a readable state | 6.4 |
| P5 | Sound | none; a minimal set (fire, hit, death, built, low power, ready, error); voices | the minimal set | 8.4 |
| P6 | Names, tone, art direction | open in S:807 | Faction: the guard force of a frontier mining company (working name "Lantern Company"), a mirror match over a claim. Tone: grounded near future, no gore. Art: low-poly, clear top-down silhouettes, team colour on large surfaces. Plain function names for units | 5 |
| P7 | The two people of the play test | two people; or the CEO and one more person | two people, one of them not the CEO, so one tester is blind | 13 |
| P8 | A lost connection | (a) the match waits for ever; (b) a slot that stays disconnected for N seconds is defeated | (b) with N = 120 s as mod data (UX part 6); without it a two-player match cannot end | 6.7 |
| P9 | Units and buildings in the way | (a) a free drop-off point in the engine and a minimum gap between buildings as a mod rule; (b) bring the M4 route step into MP1 | (a): the probe shows single buildings pass and every gap of 2.0 gives 0 blocked units; (b) moves the first match later | 6.9 |

## 3. What exists and what MP1 adds

MC gave the engine: orders as content ids in an open envelope, group movement with slot pools, the HUD
widget set with mod layout, the client shell, control groups, key bindings with a settings menu, alerts,
and the pixels-only test harness.

The engine has none of these today (verified by the seats with grep; gate 1 checks again): construction,
power, armour and damage types, teams, a match end, a lobby, a result screen, a rally point, repair,
a fire event on the wire, a unit heading on the wire, per-entry keys in the build menu, an action scope for
keys (MCD:312 says actions have a scope; `crates/turts_mod_api/src/input.rs:10-14` has none).

Two M3 validation rules remain (`crates/turts_mod_runtime/src/content_m3_validation.rs:374-376`,
`:420-424`): "a unit with health requires removal" and "a unit with a queue cannot be removed". MC already
lifted "weapons must be static". S:809 says "three"; that text is stale and the specification update after
MC corrects it.

## 4. Scope

Types: E engine mechanism for all mods; C mod content; UI client; I test instrument.

| Id | Item | Type |
| --- | --- | --- |
| X1 | Lift the two M3 rules; destruction of a producer with an explicit policy for its queued jobs | E |
| X2 | Damage types and armour classes as mod data; automatic targeting for idle and hold units with an acquire range and a leash | E |
| X4 | Construction: a `Construct` order with content and a point, a site check, a site entity that owns its progress and can be attacked, a footprint that blocks | E |
| X4b | Cancel of construction with refund | E |
| X5 | Placement mode and ghost | UI + E (interaction core) |
| X6 | Power: supply and demand for each slot, a shortage rule for each capability | E + C + UI |
| X7 | Charge over time, with the stage "waiting for funds" | E |
| X8 | Repair: a `repairer` capability and a `Repair` behaviour, charged over time | E + C |
| X9 | Rally: `SetRally` on a producer; the engine sends the follow order through the normal order path | E + UI |
| X10 | Relations: `Ally` and `Enemy` beside the present slot filter; MP1 uses two teams of one | E |
| X11 | Match lifecycle: a declarative victory rule as mod data, an outcome for each slot, surrender, the disconnect rule | E + C |
| X12 | Lobby for two slots | UI + E (server room) |
| X13 | Result screen, owned by the mod, from engine sources | UI |
| X14 | Effects: fire, hit, death, building damage; named by content id | UI + E (wire event) |
| X15 | The mod `frontier`: 8 types, ore, the map "Twin Fields", HUD layout, guide, numbers | C |
| X16 | Models (decision P1) | E + C |
| X17 | Sound set (decision P5) | C |
| X18 | MC items handed to MP1: Button `font`, "Unbound", Controls groups and keyboard navigation, the alert list defects, a value predicate for labels, the grouped failure notice (O14), the no-GPU page | UI |
| X19 | One protocol step (4 to 5) and one mod API step (0.2.3 to 0.2.4) for all of MP1 | E |
| X20 | Instruments: a headless two-client match, acceptance rows, fault plants, a pixels-only two-player driver | I |
| X21 | The play test | I |
| X22 | Final regression and CI | I |
| X23 | Architecture audit before the merge to main | I |

Not in MP1: a bot (MP2), fog of war (M5), terrain (M4), routes round obstacles beyond section 6.9,
skeletal animation (M13), more than two slots, rooms (M9), scripting (MS).

## 5. The mod `frontier` (content)

The content is in a new package `frontier`, not in `base`, because the frozen M2 stream test uses `base`
(`crates/turts_pack/tests/m3_m2_stream.rs:350-366`). All ids below passed the game-designer's scan against
engine identifiers (`light` and `shell` collide and are not used).

Roster, statistics, the damage table, the economy numbers, the placement rules, the power rule and the map
are in GD parts 1 to 6 and are not repeated here. They are proposals to measure: MP1 adds a headless duel
scenario for each balance ratio and a harvester timing, and the numbers stay Open until measured (S:807).
The ratios that carry the balance (GD part 1):

- Fights last seconds and are readable: a rifleman kills a rifleman in 9 s.
- At equal cost, riflemen lose to a tank; three launchers kill a tank; riflemen kill launchers.
- One turret beats one tank and loses to two; a turret outranges a tank.
- One harvester earns about 330 credits a minute and pays for itself in about 2.4 minutes.
- One power station supplies the command centre and two turrets; the third turret needs a second station.

Power, in the words the player gets: "When your base uses more power than it makes, everything builds at
half speed and your turrets switch off."

Victory: at the end of a tick a team is alive if one of its slots has a command centre with health above 0.
One team alive: it wins. No team alive: a draw for every team that was alive at the start of the tick.

No private path: every rule above is data in the package. No content id of `frontier` appears in engine
code; the no-content scan checks it (row NC1).

## 6. Engine mechanisms

Each mechanism is exposed to all mods through the public mod API (S 4.3, 5.2). The owner crate and the
decision that keeps later milestones cheap are from TD part B.

### 6.1 Construction (X4, X4b, X5)

- The server checks the site and owns the progress. The client ghost uses the same pure function for its
  colour only; a difference gives a refusal with a reason for each unit (D65).
- The site is an entity. It owns its progress and its job, so several sites can progress, a worker unit can
  build in a later mod, and a site can be attacked. Progress inside the producer's queue is refused: it would
  be a private shape for this one mod.
- Placement rules are data: inside the map; within a build distance of an own completed building that
  extends the area; no overlap with a footprint, with a gap; a clearance from a resource node. Footprints
  stay circles (`crates/turts_mod_api/src/schema.rs:28-30`).
- The rule "at most one work capability" (`content_m3_validation.rs:418`) gets an exemption row, because the
  command centre produces units and constructs.

### 6.2 Power (X6)

A resource kind for a capacity with supply and demand, `scope: Slot` explicit so that M6 can add networks.
Each capability names what a shortage does to it (a production rate ratio, weapon disabled). No "power"
check is hard-coded in production or combat code.

### 6.3 Damage, armour, targeting (X2)

- A damage type on the weapon, an armour class on the unit, and a table type x class of integer ratios.
  Formula: `max(min, floor(damage x ratio) - flat)`. Integer rationals only (invariant 15).
- Idle acquisition: a weapon has `acquire {range, leash}`. Today an idle unit fires only inside its weapon
  range (`crates/turts_sim/src/combat.rs:297-339`), so an outranged idle unit dies in place.
- Target queries take a visibility predicate that is always true in MP1. M5 (fog) and MP2 (bots) use the
  same query. A UnitId-to-Entity index replaces the world scans (`orders.rs:3056,3085`).

### 6.4 Payment, cancel, destruction (X7, X4b, X1)

- `ChargeOverTime`: the total charge at a progress p is `floor(cost x p)`; each tick charges the difference.
  A tick that cannot pay does not advance; the balance never goes below 0. Refund on cancel is a ratio of what
  the job charged; the ratio is mod data.
- Destruction goes through the one death path (`transaction_m3.rs:354`, called from `combat.rs:405`), which
  cancels the jobs of the destroyed unit with an `on_destroy` ratio and releases its claims.
- Every charge and refund is a ledger event, so `turts_audit` sees it. No MP1 behaviour adds a branch to a
  validator (TD part E1).

### 6.5 Repair and rally (X8, X9)

- Repair writes health only inside a transaction and charges as a ledger event.
- Rally: on spawn the engine sends the follow order through `apply_order` with the source `Engine`. A replay
  of a command log derives it again (G9). A fake player envelope is refused.

### 6.6 Relations and teams (X10)

`SlotRelation` (`schema.rs:245-249`) gains `Ally` and `Enemy`; `Other` keeps its meaning. Export takes a viewer
group, so M5 can share vision inside a team.

### 6.7 Match lifecycle, lobby, result (X11, X12, X13)

This section is the server side of the match: the room, the lobby, the claim, the start, connectivity, the
outcome, the result and the end of the process. The screen words are in section 7.6 (UX seat). The wire
records are in the table of section 6.8 (technical-director); section 6.7.13 names each record that this
section needs, with its fields. This section applies the decisions D-B, D-C, D-D, D-H and D-K of the gate.

Spec status that applies here: D23, D24, D25, D26, D28 and D56 are Decided (S:52-57, S:85). The worker
process for each match is a recommendation (S:296). Readiness and a countdown are suggestions only (S:335).
The claim boundary needs design (S:308). D70 (who owns the result screen) is Proposed (S:99).

#### 6.7.1 Terms

- Player slot: the D25 slot, one entry of `Slots` (`crates/turts_sim/src/slots.rs:22-25`).
- Seat: the reservation of one player-slot index in the lobby, before the start. A seat is not a claim. After
  the start, this design uses "player slot" only [rigour 11].
- Claim: a player id recorded as the owner of a player slot in `Slots`. A claim is terminal for the match.
- Connection: one transport link (`ClientOf`) that sent an accepted `Hello`.
- Disconnected: the server ended the connection (a transport timeout, a kick, or a clean close). The server
  knows nothing else about a player who is gone (section 6.7.8).
- Room: the lobby and lifecycle state machine in the new file `crates/turts_server/src/room.rs`. In MP1 each
  process has one room and one match.
- Start step, start tick, end tick: defined in sections 6.7.3 and 6.7.9.

#### 6.7.2 One match for each process (decision D-C)

The server runs one match for each process: Boot, Lobby, Countdown, Running, Ended, then exit with code 0.
A supervisor starts the next process (section 6.7.10). There is no world reset in place [backend H1].

Reasons from the code: the world is built one time, at boot (`crates/turts_server/src/main.rs:72`,
`boot.rs:38`). The match id is made one time (`main.rs:78`). `Slots` has no release (`slots.rs:46-57`).
About 30 resources make the sim state (`crates/turts_sim/src/plugin.rs:60-90`); a reset in place would have
to clear each of them. A new process gives a new match id and an empty `Slots` for no code. This is the S:296
recommendation; S:296 also asks for separate room and simulation lifecycles, and section 6.7.5 keeps them
separate inside the one process.

#### 6.7.3 The claim boundary (decision D-B)

The rule, in one sentence that M10 keeps: a lobby seat is a reservation that the room can release; the start
step claims every seated slot in `Slots` in one step; from the start tick, D25 holds, and no leave,
disconnect, defeat, surrender or administration action releases or reassigns a claim; under the start policy
`Immediate`, the claim happens at the accepted `Hello`, as today (late admission, S:337) [backend H2]
[rigour 3] [net 1].

Facts that the rule depends on:

- Today the claim happens at `Hello` (`crates/turts_server/src/session.rs:152-159`), and the economy log
  records it (`session.rs:261-263`). Under `Immediate` this does not change.
- The start step is one exclusive system. It calls `Slots::claim` for the seats in index order. All slots are
  open at that time, so seat i gets slot i; the step asserts this and exits with code 1 on a difference (it
  cannot happen in a lobby process).
- Two `Hello` messages in one frame get two different seats, or one refusal, because one system judges them in
  order (`session.rs:162-176`, `:215-237`). The seat table has the same rule.
- The row LOB1 checks the boundary: after the start, a new id gets `no open slot`; in the lobby, a leave
  releases the seat.

#### 6.7.4 The start policy (mod data)

The mod selects the start policy (S:333, D23). The engine supplies the variants. Readiness is not mandatory
(S:335), because `Immediate` exists.

| Variant | Behaviour | MP1 |
| --- | --- | --- |
| `Immediate` (default when the field is absent) | Boot goes to Running. Claims happen at `Hello`, first open slot. This is the behaviour of every package today | built |
| `AllSeatedReady(countdown_secs)` | The countdown starts when every seat has a human and every seat is ready. `countdown_secs` is 0 to 30 | built; `frontier` uses 3 |
| `AllSeated(countdown_secs)` | The D23 automatic start: the countdown starts when every seat has a human | reserved, not built (decision 2 below) |
| a creator start | Needs a room creator (M9) | not in MP1 |

`Immediate` keeps the 31 ignored process tests of `crates/turts_server/tests` (31 lines with `#[ignore`),
hd1, T7 and the MC rows working with no change to their commands: their packages write no match data, so
they boot to Running and claim at `Hello` [backend H3]. Row IMM1 checks this.

The server test hook `--start-now` starts the match at boot as if the policy were `Immediate`, for any
package. Every `frontier` headless row that uses `--command-script` with no client (DMG1, CON1, PWR1, PAY1,
OBS1, MAT1 script cases) uses it [backend H3].

The match data, for the 0.2.4 table of section 6.10 (the field list there is the authority; these are the
names that this section depends on) [mod H1, the match part]:

- Export group `exports.match`, at most one file, for example `match/match.ron`. Version-table row
  `exports.match`, since 0.2.4. Presence: the key `match` is written in `exports` in the raw manifest (D-I).
  A `^0.2.3` package that writes it gets the existing text (`crates/turts_mod_runtime/src/version_table.rs:101-108`):
  "`exports.match` needs engine api 0.2.4; `^0.2.3` admits 0.2.3".
- `MatchDef { start: StartPolicy, victory: Option<VictoryRule> }`. Absent `start` gives `Immediate`.
  Absent `victory` gives no outcome, ever (D28) [mod L11].
- `VictoryRule::LastTeamStanding { defeat: Vec<DefeatClause> }`. The trait behind it lets MS add a script
  form later.
- `DefeatClause::KeyUnitsLost { units: Vec<ContentRef> }` and `DefeatClause::DisconnectedFor { secs: u32 }`.
- The order behaviour `Surrender` (a new `Behaviour` variant, `crates/turts_mod_api/src/orders.rs:32-42`):
  version-table row "behaviour Surrender", since 0.2.4, presence from the raw order file. Orders exist since
  0.2.3, so `exports.match` does not cover this row.
- The colour palette is the existing `slot_colours` list of the interface (`crates/turts_mod_api/src/interface.rs:14`).
  A colour on the wire and in the room is an index into that list. The server reads only the length of the
  list, at boot. NOT TRACED: whether `turts_pack::load_package` parses the interface file today.

Pack-time checks and their texts:

- `exports.match: at most one file, found 2`
- `match/match.ron: start: countdown_secs must be 0 to 30, found 45`
- `match/match.ron: start: AllSeatedReady needs 2 slot colours in the interface, found 0` (one colour for
  each player slot; a package with a lobby policy and fewer colours is refused)
- `match/match.ron: victory.defeat[0]: KeyUnitsLost needs at least one unit`
- `match/match.ron: victory.defeat[0].units[0]: unknown unit `frontier:comand_center`; did you mean `frontier:command_center`?`
- `match/match.ron: victory.defeat[1]: DisconnectedFor secs must be above 0`
- `match/match.ron: victory.defeat: DisconnectedFor is written 2 times; at most 1`

#### 6.7.5 The server states

The room lifecycle and the simulation lifecycle are separate (S:296). The room owns the state. The sim gets
one input from the room before the start: the sim `MatchConfig` (section 6.7.11). Every sim system runs in a
`SimSet` (`plugin.rs:31-44`); each set gets the run condition "the sim `MatchConfig` exists". Without it, the
tick does not advance and no sim system runs. The server fixed systems that read the tick also wait for it:
the economy log capture (`main.rs:117`, `:120`), `exit_after_ticks` (`main.rs:139-142`), the tick log and the
tick count of `--stats`. So `--run-ticks N` counts sim ticks only; in the lobby it does not count.

Today the sim ticks with no client: 39 ticks before any `Hello`, and two clients got `Welcome` at ticks 76 and
171 (gate 1 probe, `scratchpad/be-probe/probe.out`) [backend H1].

| State | What runs | Leaves the state when |
| --- | --- | --- |
| Boot | `server.ron` read; package loaded; world built (`boot.rs:38`, `main.rs:104`); match id made (`main.rs:78`); HTTP and WebSocket bound | the boot ends: to Lobby if the policy is a lobby policy; to Running if the policy is `Immediate` or `--start-now` is set. A boot error exits with code 1 (`main.rs:36-38`) |
| Lobby | the world exists and does not tick (tick 0); seats; lobby actions; HTTP | the start policy holds: to Countdown |
| Countdown | as Lobby; the room counts `countdown_secs x tick_hz` fixed steps of the server clock. The sim still does not tick | any accepted seat change (a leave, a colour change, `unready`): to Lobby. The count reaches 0: the start step, then Running |
| Running | the sim ticks from the start tick; orders; connectivity inputs; the outcome check once for each tick | the outcome check gives the end (every slot has an outcome, section 6.7.8): to Ended |
| Ended | the sim is stopped (`MatchStopped`, reason `MatchOver`); the result is served | see section 6.7.10: exit with code 0 |

The start step (Countdown to Running), in one exclusive system, in this order:

1. Build the `MatchConfig` from the seats (section 6.7.11) and freeze it.
2. Claim each seat in `Slots` (section 6.7.3). Write each claim to the economy log at tick 0.
3. Put a `Session { slot }` on each seated connection and send `Welcome`, then the `Snapshot` of tick 0
   (section 6.7.7).
4. Insert the sim `MatchConfig`. The next fixed step advances the sim to tick 1. The start tick is 1.

The receive systems (`Hello`, connection ends, lobby actions) run before the room transition, and the room
transition runs before the start step, in one frame (`session.rs:123-133` is the present receive chain). So a
seat whose connection ended in that frame is released first, and the room goes back to Lobby before the start
step can claim it.

Each event of the lens list, defined:

- The second player never comes. The room stays in Lobby with no time limit. The waiting player sees the
  status "Waiting for a second player" (7.6). The engine has no minimum-player rule, no auto-fill and no
  deadline (S:333, S:337, D23). The operator stops the process.
- A player leaves in the lobby. The seat goes to `Open`, the revision goes up, and the other seats keep their
  ready flags. The other player sees the leave text of 7.6 ("Player 2 left. Waiting for a player to join.").
  The text comes when the server notices the leave, at most about 10 s after the loss (section 6.7.8).
- A player leaves, or clears ready, during the countdown. Countdown to Lobby. The countdown starts again from
  its full length when the policy holds again.
- A seat change during the countdown. Every accepted change of a seat (its occupant, its colour, its ready
  flag) ends the countdown, because a colour change clears the ready flag of that seat (L5). MP1 has no action
  that moves a player to another seat.
- All players disconnect during the match. The new `server.ron` field `all_disconnected` applies (S:327):
  `Continue` (default) or `Shutdown(after_secs)`. `Continue`: the sim keeps ticking; the mod clause
  `DisconnectedFor`, if any, gives the outcome. `Shutdown(after_secs)`: when no claimed slot has a connection
  for `after_secs` seconds of wall time, the server logs `turts_server: all players disconnected; shutdown`
  and exits with code 0, with no outcome. This timer ends the process; it changes no sim state, so a replay
  is not affected. Pause and save need M10 and are not MP1 values. Slot ownership never changes (S:327)
  [backend H1].
- The match ended, and a client connects. The owner of a claimed slot that did not send `LeaveMatch` gets
  `Welcome`, the `Snapshot` of the end tick and the `MatchResult`. Any other id, and an owner that sent
  `LeaveMatch`, gets `JoinRejected` with the reason `The match is over` [backend H1] [net 12].
- Both slots reach the disconnect limit in one tick: a draw for both. At different ticks: the first slot is
  defeated at its tick, and the other slot wins at that tick (section 6.7.8) [backend H4].

What a connecting client gets in each state. The protocol and lockfile checks come first in every state, as
today (`session.rs:146-151`).

| State | New id | Id with a seat or claim and no live connection | Id with a live connection |
| --- | --- | --- | --- |
| Lobby | the lowest open seat, then `LobbyState`; no open seat: `no open slot` | not possible: a seat is released when its connection ends | `This game is open in another window` (`session.rs:157`) |
| Countdown | `no open slot` (every seat is full under `AllSeatedReady`) | not possible, as above | as above |
| Running, after a lobby start | `no open slot` | `Welcome`, `Snapshot`, then deltas; `SlotConnection { connected: true }` at the next tick | as above |
| Running, `Immediate` | the first open slot is claimed: `Welcome` (late admission, as today); none open: `no open slot` | as above | as above |
| Ended | `The match is over` | owner that did not leave: `Welcome`, `Snapshot`, `MatchResult`; owner that left: `The match is over` | as above |

#### 6.7.6 Lobby rules

- The seat count is the player-slot count (2 in MP1, `slots.rs:7`).
- A new seat gets the lowest palette index that no seat holds. So the two seats start with different colours
  (L3).
- The seat label is `Player N`, with N = seat index + 1. The server makes it. MP1 has no name input. A label is
  not an identity (S:410).
- Lobby actions (client to server): `SetReady(bool)` and `SetColour(index)`. The server checks each one against
  its current state. Refusals:
  - `colour_taken`: another seat holds the index. Nothing changes.
  - `colour_unknown`: the index is outside the palette.
  - `lobby_changed`: a `SetReady(true)` names a revision older than the last change of an occupant or a
    colour. So a player cannot ready on a state that they did not see. Two ready presses in the same instant
    both pass, because a ready change does not make an older ready stale.
  - `not_in_lobby`: any lobby action after the start (S:308 freezes the room rules for the running match).
- An accepted colour change clears the ready flag of that seat (L5). A colour change to the colour that the
  seat already holds changes nothing.
- The room has no creator and no administrator in MP1. The D56 rows for a creator (S:322-327) start at M9.
  No MP1 action can reassign a seat of another player.

#### 6.7.7 Message order

With a lobby policy:

1. Client: `Hello`. Server: a seat and `LobbyState { you: seat }` to the new member, and `LobbyState` (next
   revision) to every member. Or `JoinRejected`.
2. In the lobby: `LobbyAction` from a member; `LobbyState` to every member after each accepted change;
   `LobbyRefused` to the sender after a refused action.
3. At the start step, in one frame: `Welcome { slot_id, tick: 0, tick_hz, match_id, start }`, with
   `start = MatchStart { start_tick: 1, teams, colours }`, then the `Snapshot` of tick 0. Teams and colours come
   with `Welcome` (section 6.8) because `Welcome` now comes at the start [backend M6].
4. Deltas from tick 1. `MatchState` changes ride in the deltas.
5. At the end tick: the delta of the end tick, with the outcomes, then one `MatchResult` (reliable).
6. Client: `LeaveMatch`. Server: a kick after `KICK_DELAY` (`session.rs:30`).

Under `Immediate`: `Hello`, then `Welcome { tick: the current tick, start: MatchStart { start_tick: 1, teams,
colours: the default colour of each slot } }`, then the `Snapshot`, as today (`session.rs:268-283`).

A reconnect in Running: `Hello`, `Welcome { tick: the current tick }`, `Snapshot`, deltas.

Orders:

- A seated connection has no `Session` before the start. Today the server drops the commands of such a
  connection with no answer (`session.rs:357-360`). MP1 answers each one with an `OrderResult` with the reason
  `match_not_started`. The refusal costs a token, as every envelope does (net 5).
- An order from a slot that has an outcome gets `slot_defeated` while the match runs (only possible with more
  than two teams; a sim test covers it, row MAT1).
- An order after the end tick gets `match_over`. Today the server skips such commands with no answer
  (`session.rs:386-388`).

The player id in the browser (decision D-H). Today the browser saves the player id in sessionStorage at
`Phase::Joined` (`crates/turts_client_web/src/lib.rs:259`). `Joined` comes from `Welcome`
(`crates/turts_client/src/join.rs:107-113`), and `Welcome` now comes at the start. So the client gets a new
phase `Seated { seat }` at the first `LobbyState` with `you` set, and the save moves to `Seated`. Also the
rejoin controller counts `Seated` as joined (`join.rs:479`, `crates/turts_client/src/rejoin.rs:306-311`).
Without both changes, a reload in the lobby loses the seat, and a reload with a stored id ends after 15 s in
the lobby with the text "This game is open in another window. Close it, then reload this page"
(`rejoin.rs:7-8`, `:338-341`) [backend M6]. Step 19 is a reload in the same tab; closing the tab clears
sessionStorage (net 3) [net 3].

The wire never carries the player id of another player. The id is a bearer value: `judge_hello` gives a held
slot to any `Hello` with that id (`session.rs:154-156`). Today only `Hello` carries it
(`crates/turts_net_protocol/src/messages.rs:20-24`). Lobby and result records name seats and slots with labels
only [backend M5].

#### 6.7.8 Connectivity, surrender and victory as sim inputs (decision D-D)

The engine supplies facts. The mod data decides what they do. With no clause, nothing happens (D28, S:57)
[backend H4] [net 2].

- The room writes `SlotConnection { slot, connected }` into the command log at the tick of the change: the
  first sim tick that starts after the server sees the change. It goes through the `CommandQueue`, at the
  same place in the fixed schedule as admitted orders and `--command-script` entries (`session.rs:117-122`,
  `crates/turts_server/src/command_script.rs:41-44`).
- `connected: false` is written when the connection of a claimed slot ends. `connected: true` is written when
  the owner reconnects. A claimed slot starts as connected, so the start writes no record. A slot that is never
  claimed has no record and is never counted as disconnected.
- "Disconnected" means that the server ended the session. The server notices a lost client at the netcode
  timeout, `client_timeout_secs: 10` (`crates/turts_client/src/join.rs:345`). Probe of this repair
  (`scratchpad/be-repair/lag.sh`, debug build, load 13): after a native client ended, a same-id `Hello` was
  refused until 9.1 s (clean exit) and 9.5 s (SIGKILL), and accepted at 9.8 s and 10.0 s. The gate 1 figure of
  14.5 s came from one-second tries that the server partly accepted (`scratchpad/be-probe/server4.log` has
  three `took slot 0` lines), so it is not a clean measure. NOT TRACED: whether a browser reload or a tab close
  sends a clean close, which would make the notice immediate [backend M7].
- What the lag bounds. Call it L (at most about 10 s):
  - The lobby leave text shows at most L after the real loss. LOB1 allows 11 s.
  - The disconnect count starts when the server notices. The defeat comes at most L + N after the real loss
    (130 s for `frontier`). The text "Player 2 lost connection. Waiting 2:00 for them to return." shows from the
    tick of `connected: false`, and the client counts down from the record tick, so both windows show the same
    count.
  - A reload before the server notices gets `This game is open in another window`. The browser retries every
    1 s for 15 s (`rejoin.rs:5-6`). 15 s is more than L + 1 s, so the reload gets its slot. If a later change
    makes L larger than 13 s, step 19 breaks. LOB1 and MAT1 record L.
- Surrender is an order: an envelope with a mod order of behaviour `Surrender`, no units and no target. It goes
  through the normal order path, the token bucket and the command log. It defeats the slot at the tick of the
  order with the reason `Surrendered`. A mod with no `Surrender` order cannot surrender (`unknown_order`).
- The disconnect rule is the mod clause `DisconnectedFor { secs }`. The sim counts it in ticks: `secs x tick_hz`
  from the sim `MatchConfig`. `frontier` writes 120 (decision P8 b). The test package writes 5 (section 6.7.12).
- The outcome check runs once for each tick, after all removals and after the death path, before `Collect`.
  At the end of tick t, for each slot with no outcome, a clause fires when:
  - `KeyUnitsLost`: the slot has no live, completed unit of a listed type. The sim keeps a count for each slot
    and changes it at spawn, at completion and at death. It does no world scan for each tick (perf H2).
  - `Surrender`: the slot surrendered in tick t.
  - `DisconnectedFor`: the last connection record of the slot is `false`, from tick d, and t - d is at least
    the limit.
  A slot with a fired clause is defeated in tick t. The reason is the first fired clause in the mod's list
  order. Then, with teams (team = slot in MP1, decision P2 a):
  - More than one team still has a slot with no outcome: the slots defeated in tick t get `Defeat`.
  - One team: the slots defeated in tick t get `Defeat`, and each slot of that team gets `Victory` with the
    reason `OpponentsDefeated`. The match ends at tick t.
  - No team: the slots defeated in tick t get `Draw` (the teams alive at the start of the tick, section 5).
    Slots defeated earlier keep `Defeat`. The match ends at tick t.
- A defeated player keeps the slot (D25) [rigour 12]. What happens to the units of a defeated or disconnected
  slot is mod policy (S:354). In MP1 a defeat of one of two teams ends the match.
- The end. The end tick is t. The sim inserts `MatchStopped { reason: MatchOver }` (a new `StopReason`,
  `crates/turts_sim/src/economy/events.rs:183-197`) after the `Collect` chain of tick t. So tick t is
  published, and no later tick runs (`plugin.rs:151`, `:157-160`).
- Replay. The outcome depends only on the command log, the initial state and the tick. A replay of the command
  log gives the same outcomes and the same end tick (G9, invariant 15, S:581). A server timer that counts wall
  time breaks this; row MAT1 has that red.

#### 6.7.9 The result and the statistics (decision D-K)

- Each slot gets an outcome (`Victory`, `Defeat`, `Draw`), a reason (`KeyUnitsLost`, `Surrendered`,
  `Disconnected`, `OpponentsDefeated`), and the tick of the outcome. The result screen names the reason of the
  result in words (7.6): for example "Player 2 surrendered", "Player 2 left", "Command centre destroyed"
  [player 9].
- No rematch button in MP1 (D-K). "Leave" says how to start the next match (section 6.7.10).
- The statistics for each slot: units trained, units lost, units destroyed, structures completed, structures
  lost, credits gathered, and the match time (end tick - start tick + 1, in ticks). The sim counts them from its
  events: `Destroyed` with the cause `Killed { by }` gives "units destroyed" to the owner of `by`
  (`events.rs:69-73`, `:100-103`). NOT TRACED: that the owner of `by` is known when the event is written (the
  killer can die in the same tick); the plan checks it. NOT TRACED: the name of the credit delivery event.
- The statistics are sim state. They go on the wire only in `MatchResult`, after the end tick, because before
  the end they leak enemy state. `MatchState` (public, in the deltas) carries outcomes and connections, and no
  statistics [net 10].
- The result screen reads only `MatchState` and `MatchResult`. It never works out a result from the unit list:
  under fog (M5) the unit list is partial (`docs/research/mp1/input-technical-director.md:214-216`). Whether the mod or the shell owns the screen
  follows D70, which is Proposed (S:99).

#### 6.7.10 Leave and the supervisor (decisions D-C and D-K)

- "Leave" sends `LeaveMatch` and shows the shell text: "The match is over. Reload this page to join the next
  match." The server marks the slot as left and kicks the connection after `KICK_DELAY`, so the server does not
  wait for the 10 s timeout.
- Ended ends with exit code 0 at the first of:
  - no connection with a `Session` remains;
  - `after_match_secs` of wall time after the end tick (a new `server.ron` field; default 60).
- A reload before the old process has ended reaches the old process. An owner that left, and any new id, get
  `JoinRejected` `The match is over`. The client shows: "The match is over. Reload this page in a moment to
  join the next match." The words can change in 7.6; the meaning stays.
- The supervisor for MP1 is the new script `scripts/mp1-serve.sh <server.ron> [turts_server options]`. It runs
  the release `turts_server` in a loop and does not build it:
  - exit 0: it prints `mp1-serve: match <n> ended; starting match <n+1>` and starts the server again at once;
  - any other exit: it prints `mp1-serve: turts_server exited with <code>; stopped` and exits with that code,
    so a crash does not loop;
  - SIGINT or SIGTERM: it stops the server and exits with 0.
  The play test (section 13) runs the server with this script. Each new process serves the page again from
  `static_dir`.
- The restart gap is the boot time: about 1 s for `muster` in a debug build, 42 MB resident (gate 1 backend
  L10). `docs/measurements/mp1.md` records the boot-to-Lobby time for `frontier` in release [backend L10]. A
  reload in the gap gets the existing fetch error, and a second reload works.

#### 6.7.11 Persistence and the limits that keep M9 and M10 cheap

MP1 keeps no data between processes (no SQLite, D40; no save, D41). The result exists only until the process
ends. These rules hold in MP1 so that a save (M10) and rooms (M9) need no rewrite [backend L9]:

- No ownership outside `Slots`. The room drops its seat table at the start step. After the start, every
  question "which player owns slot s" reads `Slots`.
- No outcome, connection state or match timer in a `turts_server` resource. They are sim state (the sim
  `MatchState`), made only from the command log and the tick, so a save holds them. The `Shutdown` timer and
  the `after_match_secs` timer are process policy, not match state, and a save does not hold them.
- No configuration change after the start (S:308). A lobby action after the start gets `not_in_lobby`.
- One serializable `MatchConfig`, frozen at the start step: `{ lockfile_hash, tick_hz, start_tick, slots:
  [{ slot, team, colour }], settings }`. It holds no player id. The sim gets the part without colour
  (`{ tick_hz, start_tick, teams, settings }`), because colour never enters the sim. An M10 save stores
  `MatchConfig`, `Slots`, the sim state and the lockfile. M10 decides whether a restore writes
  `connected: false` for each slot.
- `settings` is an empty list in MP1; the mod settings schema is M9 (MD12, S:314).
- MP1 adds no new output of a player id. The existing stderr line `turts_server: player <id> took slot <n>`
  (`session.rs:264-267`) stays until D50 replaces the bearer id.
- Hosting (D58): the sim does not tick in Lobby and Countdown, so a waiting server costs almost nothing. The
  outcome check costs O(slots) for each tick with the counts above [backend L10].

#### 6.7.12 Test hooks

In the style of the existing hooks (`crates/turts_client/src/actions.rs:20-46`, `:107-118`; the options in
`CLAUDE.md`) [backend M8]:

- Server options:
  - `--start-now`: start at boot as under `Immediate` (section 6.7.4).
  - `--input-log PATH`: write every sim input that the server admitted, with its tick, as one JSON array in the
    `--command-script` format. A replay is `turts_server <server.ron> --start-now --command-script PATH`. RLY1
    uses the same file (6.5).
  - `--command-script` gets a second entry kind: `{"tick": T, "slot": S, "connection": false}`. Surrender is an
    order envelope and needs no new kind.
  - `--stats` gets `room_state` and `match_over_tick`.
  - `server.ron` gets `all_disconnected` and `after_match_secs`. The help test reads the fields from
    `config.rs` (`crates/turts_server/tests/help.rs:1-3`), so both fields go into `--help`.
- Headless client:
  - `--actions` kinds: `ready`, `unready`, `set_colour` (new field `colour`, a palette index), `surrender`,
    `leave`, and the test-only `lobby_move` (one `MoveTo` envelope for the listed `units` to `world_point`,
    sent through the normal send path with no interaction core, to test `match_not_started`).
  - Timing. A match action keeps `tick`: ticks after the `Welcome` tick (`actions.rs:117-118`). There is no
    `Welcome` tick in the lobby, so a lobby action (`ready`, `unready`, `set_colour`, `lobby_move`) and `leave`
    use `on: { event, wait_ms }`: the action runs `wait_ms` after the first state in which `event` holds, after
    the previous action ran. Events: `seated` (`you` is set), `full` (every seat has a human), `countdown`,
    `waiting`, `result` (`MatchResult` received). An action with both `tick` and `on` is a parse error.
  - `--rejoin` (requires `--player-id`): the headless client uses the stored-identity rejoin controller. Today
    only a client with a window does (`crates/turts_client/src/main.rs:38-43`); a headless client sends one
    `Hello` and exits with 1 on a refusal (gate 1 probe 3).
  - New `--out` report fields: `lobby_states` (each `LobbyState` with its receive time in ms), `lobby_refusals`,
    `welcome` (`tick`, `start_tick`, `teams`, `colours`), `match_states` (each change, with its tick), `outcome`
    (the own slot: `outcome` and `reason`, or null), `match_result` (or null), `match_over_tick` (or null),
    `stats_first_seen_tick` (the tick of the first received record that holds statistics, or null).
- The disconnect limit for tests is mod data in a test package, not an engine override. The test package
  `mods/standoff` (Game, `^0.2.4`; its ids pass the NC1 scan, checked when it is written): two slots, one key
  unit and one armed unit for each slot, symmetric, so that one script tick of attack orders kills both key
  units in one tick; `start: AllSeatedReady(countdown_secs: 1)`; `defeat: [KeyUnitsLost(..),
  DisconnectedFor(secs: 5)]`; one `Surrender` order. At 20 Hz the limit is 100 ticks.

#### 6.7.13 Records for the wire table (section 6.8)

| Record | Direction, channel | Fields | Who gets it |
| --- | --- | --- | --- |
| `LobbyState` | server to client, reliable | `revision: u32`, `you: Option<u8>`, `seats: Vec<Seat>`, `settings: Vec<SettingEntry>` (empty in MP1), `start: Waiting \| Countdown { remaining_ms: u32 }` | room members |
| `Seat` | part of `LobbyState` | `index: u8`, `occupant: Open \| Human { label: String } \| Bot { content: String }`, `ready: bool`, `colour: u8` | room members |
| `LobbyAction` | client to server, reliable | `revision: u32`, `action: SetReady(bool) \| SetColour(u8)` | - |
| `LobbyRefused` | server to client, reliable | `revision: u32`, `reason: colour_taken \| colour_unknown \| lobby_changed \| not_in_lobby` | the sender |
| `Welcome` (changed) | server to client, reliable | today's fields (`messages.rs:26-31`) plus `start: MatchStart` | the slot owner |
| `MatchStart` | part of `Welcome` | `start_tick: u32`, `teams: Vec<u8>`, `colours: Vec<u8>` (index = slot) | the slot owner |
| `MatchState` | in the delta, on change | `connections: Vec<{ slot, connected: bool, since_tick: u32 }>`, `outcomes: Vec<Option<{ outcome, reason, tick }>>` | public |
| `MatchResult` | server to client, reliable, once | `end_tick: u32`, `slots: Vec<{ slot, label, outcome, reason, tick }>`, `stats: Vec<{ slot, units_trained, units_lost, units_destroyed, structures_completed, structures_lost, credits_gathered }>`, `duration_ticks: u32` | public, after the end tick only |
| `LeaveMatch` | client to server, reliable | none | - |
| `JoinRejected` (new reason) | server to client | `The match is over` | the sender |
| `CommandRejectReason` (new) | in `OrderResult` | `MatchNotStarted`, `SlotDefeated`, `MatchOver` | the sender |
| `SlotConnection` | sim input in the command log, not on the wire | `slot`, `connected` | - |

#### 6.7.14 Decisions for the CEO from this section

1. `all_disconnected` default: `Continue` (recommended: with the mod clause the match ends by itself) or
   `Shutdown(after_secs)`.
2. `AllSeated` (the D23 automatic start): build it in MP1 (one more LOB1 case) or keep it for M9 with the
   creator start (recommended: MP1 has no room creation, and `frontier` uses `AllSeatedReady`).
3. Ended: exit when no session remains, or after `after_match_secs` = 60 (recommended), or only after the time.
4. The disconnect notice L: keep the 10 s netcode timeout (recommended for MP1), or lower it (a network-seat
   change that also changes MC 10.5).

### 6.8 Wire and API (X19)

- One protocol step, 4 to 5: construction, power, effects, match state and lobby records; rally on the order
  record; teams and colours at the welcome; new refusal reasons; and `unit_orders` as changes only (MC audit
  debt, `export.rs:190-195`). Everything goes through the per-slot export (`export.rs:25,106`).
- `effects: [Fired{unit, target, weapon}, Destroyed{unit}]`, visible to each client that may see the units
  (all clients in MP1). The client derives a hit from the health drop. Effects are cosmetic; the state comes
  from health and removal.
- One mod API step, 0.2.3 to 0.2.4, additive, with one version-table row for each feature.
- Lane 0 (section 10) adds every schema field, table row and wire record first, with stub refusals, so that
  the feature lanes do not all edit the same schema and message files.

### 6.9 Units and buildings in the way (decision P9)

Probe MP1-P1 measured today's behaviour (`docs/research/mp1/probe-obstacles.md`; native, deterministic, the
same result in two runs). Result: PASSABLE WITH LIMITS.

- One building is passable. One unit and twenty units pass a static footprint of radius 2.5 at every offset;
  they arrive 1.00 to 1.22 times later than on a free line. A click on a building ends on a ring round it.
- A compact base traps units. With a gap between footprints that is smaller than a unit's diameter, 20 to
  50 % of tanks stay `Blocked` inside the base, and a lone tank also. With the 0.5 gap of the first content
  proposal, 7 of 20 infantry stay too. With every gap at least 2.0, 0 units are `Blocked`.
- A building near the drop-off stops the economy. A power station 1 to 2 from the command centre on the ore
  side gives 0 harvester trips: the harvester stops `Blocked` and its gather order is cleared
  (`gather.rs:1126-1149`, `orders.rs:785-819`).

MP1 takes two measures, and no route search:

1. Engine (small, generic, stays valid at M4): the approach point of a receiver or a node is the nearest
   ring point that no static footprint blocks. About 40 lines in one file with one test (estimate of the probe,
   NOT TRACED).
2. Mod placement rule, as data: the gap between two footprints is at least the largest mobile diameter plus
   0.2 (2.0 for the tank of the proposal). The cost: bases are wider, and a player cannot build a wall.

Recorded limit, as MC ruling O14: MP1 has no route round a group of obstacles. A route over the static
footprints is the first step of M4 navigation (the M4 design, `2026-10-28-m4-terrain-design.md:178-191`), not
of M4b. A `Blocked` unit in physical escape can walk up to 1.154 into a footprint (MC design 6.5 permits it);
a player can see it; it stays until M4b.

Decision P9 for the CEO: accept these two measures (recommended), or bring the M4 route step into MP1 (400 to
700 lines in 5 to 7 files, estimate of the probe; it moves the first match later).

## 7. Player interface

From UX. Engine widgets with mod layout (D67); every new item is an id, a number or a string.

### 7.1 The player path

The oracle list of the play test is the section `## Player path` at the end of this design (lobby lines L1 to
L5 and match steps 1 to 19). The testers never see it. The first three steps are the lesson of the CEO play of
MC on 2026-11-11: the goal is on the screen in one line, one hint line names the first action, and the screen
shows a reward when the action is done. Row PT3 is red when a tester does not see the site of step 3 within
60 s of "Click to play".

### 7.2 Construction

Start from the build menu of the command centre. A ghost follows the pointer. A valid place shows a solid
ring in the slot colour; an invalid place shows stripes, the `forbidden` cursor and the reason in words at
the pointer ("Something is in the way", "Cannot build on ore", "Too far from your base", "Outside the map").
Colour is never the only signal. Left click places; Shift and left click places and keeps the ghost; right
click or Esc leaves placement and never gives an order. No rotation (circles).

### 7.3 Production

Two queues on the command centre, structures and units, which run at the same time (named queues for each
producer: engine mechanism). Each entry shows a count badge and its progress. Words: "On hold: no credits",
"Slowed: low power", "Queue is full (5/5)".

### 7.4 Keys

The build menu needs one key for each entry (today one key for all: `sources.rs:391-393`). The key check is
global today, so each MP1 key is unique across all cards (UX part 2 gives the table). The action scope of
MCD:312 is not built in MP1; the MC design text is corrected to say so.

### 7.5 HUD sources

New sources, primary fields, predicates, world styles, cursor contexts and alert triggers are listed in UX
part 8. The power source is named `Supply`, because the same source gives the supply cap of a later mod.

### 7.6 Lobby and result screen

The shell owns the lobby (room lifecycle that M9 grows); its fields come from the mod settings schema
(S:314). The mod owns the result screen (D70). Words and states: UX part 6.

### 7.7 MC items

Button `font` and `size`; "Unbound"; Controls groups and keyboard navigation (search waits for M8); the alert
list clip marks and row pitch; a value predicate so that "x1 - HP" does not show for a unit with no health;
the grouped failure notice of ruling O14 ("3 of 8 units: Cannot reach that place", one sound, a click selects
the failed units); and a plain page when the browser gives no GPU adapter (the CEO met a panic with a console
message only, 2026-11-11).

## 8. Presentation

From RE.

### 8.1 Models (decision P1, option b)

- `models` in the package assets, checked like the other asset kinds: GLB version 2, no external reference,
  no skin, no animation, triangle and size caps, the named nodes present. The validator uses the same parser
  version as the client.
- The browser loads from the package bytes in memory (Bevy 0.20 `MemoryAssetReader`), with no fetch.
- The visual keeps its `shape` as the fallback while a model loads and as the content cut line.
- Team colour: a material slot named `team`, replaced at spawn by the slot material.
- The client builds one flat template for each model and spawns its own entities; it does not spawn scenes.
- The MC packages keep their shapes, so the MC pixel rows do not change.

### 8.2 Heading and aim

The client derives the yaw of a unit from its movement, with a turn rate; no wire change. A turret node aims
from the `Fired` event. Walking animation needs skinning and stays at M13; infantry gets a small bob.

### 8.3 Effects

Pooled unlit meshes for flash and spark, a line for a tracer, an expanding burst for a death, smoke at a
health threshold for a building; a cap of 256 instances; no compute shader, so WebGL2 stays possible. A mod
names each effect by content id; the server never reads them.

### 8.4 Sound

The audio engine exists. MP1 adds the minimal set of decision P5 as mod content.

### 8.5 Measurement first

The first presentation task measures the frame time at 400 units with primitives and with models, in the
browser on the real GPU (RE part 5: 10.7 to 43 ms is an estimate, not a measurement).

## 9. Architecture

- New crate `turts_rules`: pure placement, relation and damage rules, with no Bevy type, used by the sim and
  by the interaction core, so the client holds no second copy of a rule.
- The pre-MP1 slice (`2026-11-12-pre-mp1-slice-design.md`) lands first: `turts_audit`, one movement model,
  the neighbour grid, and the file splits.
- After the pure moves: a behaviour handler table in `turts_sim/src/orders/`, so a new behaviour is one new
  file and one table line; and `Core::step` of the interaction core split by input kind.
- The crate table after MP1 is TD part D.
- Refused in any MP1 plan (TD part E): a validator branch for an MP1 behaviour; a second movement path; a
  default-mod rule name in engine code; a frozen-value change other than the one approved re-freeze; client
  authority over placement or outcome.

## 10. Lanes and order

A dependency graph, not waves; a task starts when its inputs are merged (PR part 4). One worktree and one
build cache for each lane; reviewers in their own worktrees; fast checks for each task; the slow checks one
time for each merged tree.

- Lane 0: the schema, version table and wire freeze with stub refusals; `turts_rules`; the handler table.
- Critical path (39 runs with the slice done during the gates): X1, X2, X6, X15, the headless match, the
  two-player browser driver, the play test, the final regression, the audit.
- Up to 7 lanes in parallel in the first half. The tail is serial, so each feature lane owns its own pixel
  rows and the final browser driver holds only the full match.
- The GPU is the scarce resource: two headed clients for one match is tested in the first two days.

## 11. Budget

At the MC rate (PR part 3): the slice 11 to 18 runs; MP1 tasks 85 to 139; gates 20 to 35; total 116 to 192
(124 to 208 with models). Build time on the critical path 55 to 160 hours. MP2, coarse: 51 to 85 runs. These
replace the early estimate of 95 to 150 runs for MP1 and MP2 together.

## 12. Acceptance rows

Each row names what is checked, the build on which it must be red, and where it runs. Gate 1 (qa-lead)
checks that each red is reachable. The row list is a first draft; the plan fixes the commands.

| Row | Check | Red build | Runs |
| --- | --- | --- | --- |
| NC1 | No content id of `frontier` in engine code | a `frontier` id in a crate source | headless |
| VAL1 | The two M3 rules are lifted with their version-table rows; a package for API 0.2.3 with the new content is refused | a row missing | headless |
| DMG1 | Damage follows the type x class table with integer arithmetic; each balance ratio of section 5 holds in a headless duel | a float multiplier; a swapped table cell | headless |
| TGT1 | An idle unit acquires an enemy inside its acquire range and returns at its leash; hold never moves | acquisition off | headless |
| CON1 | A site is refused for each placement cause with its reason; a valid site appears, progresses, charges over time, and completes | the site check off; progress in the producer queue | headless |
| CON2 | Client ghost validity equals the server result for 1,000 seeded points | the client uses its own copy of a rule | headless |
| CON3 | Cancel refunds the charged amount by the ratio; a destroyed site refunds nothing; claims are released | refund of the full cost | headless |
| PWR1 | Supply and demand count completed buildings only; a shortage slows production by the ratio and disables turrets; recovery restores both | a hard-coded check; sites counted | headless |
| PAY1 | Charge over time: the total at completion equals the cost; the balance never goes below 0; a job that cannot pay waits and resumes | charge at the order | headless |
| REP1 | Repair raises health at the rate, charges for each point, stalls with no credits | a health write outside a transaction | headless |
| RLY1 | A new unit follows the rally through the normal order path; a replay of the command log gives the same result | a fake player envelope | headless |
| MAT1 | Outcomes (X11, D-D), package `standoff` (limit 100 ticks at 20 Hz). Scripts, `--start-now --command-script`: (a) attack orders kill B's key unit at tick T: A `Victory`/`OpponentsDefeated`, B `Defeat`/`KeyUnitsLost`, `match_over_tick` T, and the tick log has no tick after T; (b) both key units die in one tick: `Draw` for both; (c) `connection: false` for both slots at tick 10: `Draw`/`Disconnected` at tick 110; at ticks 10 and 20: slot 0 `Defeat` at 110 and slot 1 `Victory` at 110; (d) slot 0 false at 10 and true at 60: no outcome at 110. Two headless clients: (e) B `surrender` at tick 40: both reports have the outcome with the reason `Surrendered` and the same `match_over_tick`, and B's key unit is alive at the end; (f) B killed: the input log has `connection: false` for slot 1 at tick D, at most 11 s after the kill, and B gets `Defeat`/`Disconnected` at D + 100; (g) B killed, then B with `--rejoin` before the limit: `connection: true`, and no outcome by D + 200; (h) the `--input-log` of (e) and of (f), replayed with `--start-now --command-script`, gives the same outcomes and `match_over_tick`; (i) `muster` (no victory data), one client killed: no outcome after 300 ticks; (j) after the end, a new id gets `The match is over`, and B (an owner that did not leave) gets `welcome` and the same `match_result`. A sim test with three slots: an order from a defeated slot gets `slot_defeated` | the client works out the outcome from the unit list ((e) and (f): `outcome` is null, because B's key unit is alive; in case (a) the two agree, so only (e) and (f) can show this red); the disconnect count is a wall-clock timer in the server ((h): the replay has no connections, so it gives no outcome); a defeat with no clause ((i), D28); a draw rule that gives `Victory` to the lower slot index ((b), (c)) | headless, two clients, and scripts |
| LOB1 | Lobby start and claim boundary (D-B), package `standoff`. Run 1, headless clients A, B, C: A seats: A's first `lobby_states` entry has `you: 0`, seat 1 `Open`, start `Waiting`, and A has no `welcome`; server `--stats` `ticks` stays 0 for 3 s. B seats: both reports show two `Human` seats with different colours. Only A is ready for 3 s: start stays `Waiting`, no `welcome`. B ready: start `Countdown`; B's `lobby_move` in the countdown gets `match_not_started`. Both `welcome` records have `tick` 0, `start_tick` 1, and equal `teams` and `colours`; the first snapshot tick is 0 for both; the tick log starts at tick 1. After the start, C (new id) gets `no open slot`; B killed, then B with `--rejoin` gets slot 1 again. Run 2: B sends `unready` in the countdown: start goes back to `Waiting` and no `welcome` comes; B `ready` again: the match starts. Run 3: A and B seated, B killed: within 11 s A's lobby state shows seat 1 `Open` (the time is recorded as L), and C gets seat 1. Pixels, two windows: the words of L2 to L5, "Match starts in 3", "2", "1", and both windows show "Click to play" | the sim claims at `Hello` (run 3: C gets `no open slot`); the sim ticks in the lobby (the `welcome` ticks are above 0 and differ, as today: 76 and 171 in the gate 1 probe); a start with one ready (run 1); an order admitted before the start (run 1: `lobby_move` gets another result); `unready` does not stop the countdown (run 2) | headless, three clients; pixels, two windows |
| RES1 | Result (D-K). Pixels, two windows: within 1 s of the end tick both windows show the outcome and the reason line (7.6 words, for example "Player 2 surrendered"); within 3 s more, the result screen shows both slots, their outcomes and reasons, and the statistics; the oracle compares each number on the screen with the `match_result` of that client. Wire check (net 10), headless, two clients: in each report `stats_first_seen_tick` equals `match_over_tick`, and each client got exactly one `MatchResult`; an export unit test in `turts_server` builds the per-slot export at each tick of a scripted `standoff` match and finds no statistics before the end tick | the sim view gives the statistics at each tick (headless: `stats_first_seen_tick` is 1; the export test is red at tick 1); a result screen without the reason (pixels); a pixels driver alone cannot show the first red, because a client can hold a value and not show it | headless, two clients; pixels, two windows |
| FX1 | A `Fired` event reaches each client that may see the unit; an enemy turret aims; a death shows its effect | events to the owner only | headless and pixels |
| HUD5 | The power text, the low-power state, the queue states and the credits counter show the words of section 7 | a missing source | pixels |
| KEY1 | Each build entry has its own key; no MP1 key conflicts | one key for all entries | headless |
| OBS1 | A harvester completes 20 trips with a building 1 to 2 from the command centre on the ore side; with the minimum gap rule, 0 of 20 tanks are `Blocked` in a compact base; the site check refuses a smaller gap | the approach point ignores statics; the gap rule off | headless |
| MDL1 | A package with a model loads in the browser from memory; a damaged model is refused at pack time; the shape shows until the model is ready | a URL fetch; no refusal | headless and pixels |
| VIS3 | Frame time p50 and p95 at 400 units, primitives and models | recorded, not judged | pixels |
| MCX1 | The MC items of section 7.7, one check each | each defect put back | pixels |
| NOGPU1 | With no adapter the page shows the plain message | the panic | pixels |
| PT3 | The two-person play test | a tester cannot find an object, an action or a reason, or 60 s pass with no progress | people |
| REG2 | All MC rows, the fault sets, the frozen values after the one re-freeze | any regression | all |
| GOAL1 | In the first presented frame after "Click to play": the camera shows the own command centre, and it is selected; the top line says "Destroy the enemy command centre"; one hint line names one first action and its key. A hint line shows while its condition holds and goes when the action is done; then the next hint shows. A pixels-only driver that does only what the hint lines name places a Power Station and sees the next hint within 60 s of "Click to play" | The MC `forge` start: a guide page, no objective, no selected building (PT1 person run 1, build 8d6902c); or a hint that stays after its action is done | pixels, two windows |
| GHO1 | P, or a click on the build entry, starts placement. The ghost follows the pointer. Over a valid point: a solid ring in the slot colour. Over each invalid cause (a unit, ore, too far, outside the map): stripes, the `forbidden` cursor, and the reason text of section 7.2 at the pointer. Right click and Esc end placement: the server order log has no new line, and the credits do not change. Left click places: a pending site shows in the same frame; after the server result, a world progress bar grows on the site until it is complete. Shift and left click places and keeps the ghost | One ghost style for valid and invalid; no reason text; a right click in placement sends a move | pixels |
| PRD1 | The command centre runs one structure job and one unit job at the same time, each in its own named queue. A sixth job in a full queue of 5 is refused with `queue_full`. A new unit appears at an exit point of the producer. With every exit point blocked, the completed job waits, charges nothing more, and the unit appears when an exit point is free | One shared queue; a unit that appears inside a footprint or on another unit | headless |
| PAY2 | Cancel of production under charge over time: a cancel of the active job refunds what the job charged, multiplied by the mod ratio; a cancel of a waiting job refunds 0, because the job charged 0; the ledger (`turts_audit`) balances after each cancel | Refund of the full cost; a refund for a job that charged nothing | headless |
| DES1 | The one death path removes a destroyed unit that has health and a queue (the two M3 rules lifted, VAL1). It cancels the queued jobs with the `on_destroy` ratio, releases the claims, and the ledger balances. In the same tick the footprint is free: a site on the same point passes the site check, and a unit moves across the point | A second removal path; jobs left on a removed unit; the footprint kept after the death | headless |
| ALR2 | Forced states give one alert line each, with its sound, and a minimap ping where the state has a place: Construction complete, Unit ready, Low power, Not enough credits, Base under attack, Harvester idle, a lost connection. A state that persists (low power for 60 s; 0 credits for 60 s) gives one line and one sound; the alert comes again only after the state clears and returns. A click on an alert line centres the camera on its place | The alert repeats at each cooldown while the state persists (the rule form of `mods/forge/interface/hud.ron:130-136`); a click on the line does nothing | headless and pixels |
| HUD6 | The tooltip of a build entry shows the name, the cost with "paid while it builds", the build time, the power change, and the key. A disabled entry names its cause ("Queue is full (5/5)"). The info panel of one selected unit shows its armour class; for the harvester it also shows the load and the activity ("Harvesting", "Returning", "Waiting: field empty", "Idle") | A tooltip without the cost or the key; a disabled entry with no cause | pixels |
| SMT1 | MP1 smart rules and cursors. Command centre selected: a right click on the ground sets the rally (a flag and a line show; no unit moves); a right click on ore sets a gather rally. Engineer selected: over a damaged own building the cursor is `repair`, and a right click repairs it (the health bar rises). Harvester selected: over ore the cursor is `gather`. In placement the cursor is `place` or `forbidden`. The oracle reads `turts_debug.cursor` for each context | One cursor for all contexts; a right click on the ground with the command centre selected gives a refusal or a move | pixels |
| AUD2 | The sound set of decision P5 plays from the mod package: fire, hit, death, construction complete, low power, unit ready, error. A weapon sound is positional: a shot far from the camera look-at point has a lower gain than a near shot. With 40 units that fire at once, no more weapon sounds play at once than the mod cap (`turts_debug.audio`) | A missing sound id; equal gain near and far; no cap | pixels |
| LOB2 | Colour choice. Headless, two clients: at seating A holds colour 0 and B colour 1 (the lowest free index). B sends `set_colour` 0: B's `lobby_refusals` has `colour_taken`, and the last lobby state in both reports still has A 0 and B 1. A sends `set_colour` 2: both reports show A 2, and A's ready flag is cleared. The `welcome` `colours` of both clients equal the last lobby state. Pixels, two windows: L4 (the swatch of B's colour shows a cross and the tooltip "Player 2 has this colour"; a click on it changes nothing); after A picks colour 2, the units, rings and minimap marks of A have palette colour 2 in both windows | the server does not check the colour (headless: both seats hold 0 after B's `set_colour`; a pixels driver cannot reach this red, because the crossed swatch does nothing on click); the client takes the slot colour from the palette by slot index and not from `MatchStart.colours` (pixels: A's rings show palette colour 0); two seats get one default colour | headless, two clients; pixels, two windows |
| SUR1 | The menu item "Surrender" shows "Surrender and end the match? You will be defeated." Cancel: the server `--input-log` has no surrender envelope, and the match goes on for 5 s. Confirm: one surrender envelope; the server ends the match at the tick of that envelope; within 1 s both windows show the outcome (B "Defeat", A "Victory") with the reason, then the result screen. In each window, the first frame that shows the outcome has a replica tick at or after `match_over_tick` (frame log) | surrender with no confirmation (the envelope goes at the first click); the other window shows no result (`MatchResult` sent only to the slot that surrendered); the outcome shows before the server end tick (the client decides the outcome) | pixels, two windows |
| IMM1 | Default start policy `Immediate` (D-B). A package with no `exports.match` boots to Running: `muster` with no client has `--stats` `ticks` above 0 after 3 s, and a `Hello` gets `welcome` at once with `tick` above 0. The 31 ignored process tests of `crates/turts_server/tests`, hd1, T7 and the MC rows pass with no change to their commands. `standoff` with `--start-now` and a `--command-script` runs the script with no client. A `^0.2.3` package that writes `exports.match` is refused with "`exports.match` needs engine api 0.2.4; `^0.2.3` admits 0.2.3"; a `^0.2.3` package with an order of behaviour `Surrender` is refused with its row name | the default policy is a lobby (`muster` has 0 ticks; the `mc_group` tests time out); `--start-now` ignored (the `standoff` script run has 0 ticks); a version-table row missing (the `^0.2.3` package packs) | headless |
| LEAVE1 | One match for each process (D-C, D-K). Headless, two clients finish a `standoff` match, and each runs `leave`: the server exits with 0 within 2 s of the second `LeaveMatch`. Before that exit, a client that left gets `The match is over`, and an owner that did not leave gets `welcome` and `match_result`. With no `leave`, the server exits with 0 at `after_match_secs`. `scripts/mp1-serve.sh` starts a new process after exit 0; a new `Hello` gets a lobby seat, and the `welcome` of the next match has a different `match_id`. With a plant `server.ron` that names a missing `game_package`, the script stops with exit code 1 and does not start again | the server stays in Ended (no exit); a world reset in place (the same `match_id` after the restart); the script restarts after a non-zero exit; an owner that did not leave gets `The match is over` | headless and the script |
| TGT2 | An idle or holding unit that takes damage targets its attacker inside its leash; a unit with no weapon does not | retaliation off | headless |

## 13. Play test

The `player` agent plays through the harness with the goals only. Two people play in two windows or on two
machines; one person signs off. Goals text (UX part 9): "Play one match against the other person. Each of you
starts with a command centre, a harvester and some credits. Win by destroying the other command centre. Get
power, collect credits from ore, train an army, build a turret, and fight. Use only the mouse and the
keyboard. Say aloud when you do not know what to do next. Stop at the result screen, or after 40 minutes."

The three places where a first-time player most likely stops (UX part 9), each with its design answer: the
player does not find building (the command centre is selected at the start, with an objective line); a
silent stall from no credits or low power (the row states and alerts of section 7.3); a red ghost with no
cause (the reason at the pointer; right click and Esc cancel).

## 14. Risks and the early tests

| Risk | Early test, in the first two days |
| --- | --- |
| Units cannot pass buildings | Probe MP1-P1 (running) |
| The lobby and screens overrun, as the client shell did in MC (22 runs against 6) | A pixels-only stub: two clients reach the lobby, press ready, the match starts; red if the stub needs more than 4 runs |
| One GPU for a two-client match | Two headed clients for 10 minutes with combat; frame p95 and the wait for the GPU lock |
| The numbers do not end a match, or end it too fast | A scripted headless match with the draft numbers |
| Models cost too much frame time | The 400-unit measurement of section 8.5 |
| Art production has no natural end | The cap of decision P1 and the shape fallback |
| The re-freeze and the audit change have a long tail of moved fault plants | The slice gate runs the fault sets with the replay validator disabled |

## 15. Open points for gate 1

1. Section 6.9 has the probe result; the 40-line estimate of the engine measure is NOT TRACED.
2. The pre-MP1 slice gate is in progress; its findings can change section 9.
3. Two M2-era open decisions touch MP1 lightly and stay Open: the tick rate and the numeric budgets (S 15).
   MP1 records numbers and promises none.
4. The MP2 row asks for remembered sightings, but fog comes at M5 (PR part 3): a sequencing question for the
   MP2 design, not for MP1.

## 16. Cover that the yardstick does not ask for (from the coverage work)

The yardstick marks an objectives panel as COMMON, not CORE, so the completeness gate never asks for a visible
goal or a first action. That is the cause class of the CEO's failed MC play. MP1 covers it with its own rows:

- GOAL1 and PT3: the goal line, the first action hint, and the 60 s rule of section 7.1.
- ALR2: an alert for a state that lasts (low power, no credits, a harvester idle) shows and sounds one time when
  the state starts, and not again while it lasts. An event alert (unit ready, construction complete, under
  attack with its cooldown) sounds each time. The engine tells the start of a state from its repeat; a long
  cooldown in content is not the rule. MC row ALR1 stays green while a state fires again at each cooldown,
  which is how "it beeps non stop" passed MC.
- Refusals (OR22): a refused order names a cause that the player can act on, near the pointer. GHO1 covers
  placement; MCX1 covers the grouped notice; the same rule holds for every refused order.
- TGT2 (new): an idle or holding unit that takes damage targets its attacker inside its leash. Without it, with
  the proposed numbers, a rifleman (acquire range 7) under a turret (range 8) does not react. X2 takes this.
- Sound (AUD2): effect sounds need a small engine part (an effect names a sound; the engine plays it at the
  place of the effect, with a cap). Section 8.4 said "content only"; that was wrong. X14 takes the engine part.
- Lobby (decision P2 a): teams and team choice stay later; the starting credits setting is not in MP1.
- 57 MC rows are `done` and rest on REG2 alone. MC is not closed: its person play failed and Tasks 23 and 24
  are open. MP1 gate 2 checks again that each `done` path is green on the tree that MP1 starts from.

## Yardstick coverage

One row for each of the 181 CORE items of `docs/qa/yardstick-a.tsv`, in file order. `in` rows name a
row of section 12. `done:` rows name the evidence of an earlier milestone: for an MC item it is the MC
row file or test, and the note names the MC row. MC acceptance is not closed (PT1 person run 1 failed on
2026-11-11), so REG2 runs every MC row again, and a red MC row makes MP1 red. `later:` rows name a
milestone after MP1 in `docs/qa/milestones.tsv`; the note gives the reason and the MP1 part with its row,
if MP1 builds a part. Where a yardstick dependency is later than MP1, the item is `later:` even when MP1
builds its flat-ground part. The dispositions assume the recommendations of section 2 (P1 b, P2 a, P4 b,
P5 minimal set, P8 b).

| id | disposition | test | item | note |
| --- | --- | --- | --- | --- |
| IN01 | done:tests/browser/rows/SEL1.json | - | Screen-to-world picking | MC SEL1; REG2 runs it again |
| IN02 | done:tests/browser/rows/KM1.json | - | Action mapping layer | MC KM1; REG2 runs it again |
| IN03 | done:crates/turts_interaction/tests/km3.rs | - | Modifier semantics | MC KM3; REG2 runs it again |
| IN04 | done:crates/turts_interaction/tests/km3.rs | - | Double-click detection | MC KM3; REG2 runs it again |
| IN05 | in | KEY1 | Hotkey per command button | MC HUD2. MP1: one key for each build entry |
| IN08 | done:tests/browser/rows/KM2.json | - | Browser input capture | MC KM2; REG2 runs it again |
| IN09 | later:M9 | - | Text input mode | as MC |
| PC01 | done:tests/browser/rows/SEL1.json | - | Single select | MC SEL1; REG2 runs it again |
| PC02 | done:tests/browser/rows/SEL2.json | - | Box (drag) select | MC SEL2; REG2 runs it again |
| PC04 | done:tests/browser/rows/SEL2.json | - | Shift add and remove | MC SEL2; REG2 runs it again |
| PC05 | done:tests/browser/rows/SEL3.json | - | Type select on screen | MC SEL3; REG2 runs it again |
| PC08 | done:tests/browser/rows/SEL4.json | - | Control group assign and recall | MC SEL4; REG2 runs it again |
| PC09 | done:tests/browser/rows/SEL4.json | - | Control group add | MC SEL4; REG2 runs it again |
| PC10 | done:tests/browser/rows/SEL4.json | - | Group double-tap centres camera | MC SEL4; REG2 runs it again |
| PC16 | done:tests/browser/rows/SEL1.json | - | Inspect foreign entities | MC SEL1; REG2 runs it again |
| PC17 | done:tests/browser/rows/ORD2.json | - | Ownership gate | MC ORD2; REG2 runs it again |
| PC19 | done:tests/browser/rows/SEL4.json | - | Selection hygiene | MC SEL4; REG2 runs it again |
| PC20 | done:tests/browser/rows/SEL1.json | - | Hover highlight | MC SEL1; REG2 runs it again |
| PC21 | in | SMT1 | Context cursor | MC HUD4. MP1: place, repair, rally cursors |
| PC22 | in | SMT1 | Smart command | MC ORD5. MP1: rally and repair smart rules |
| PC23 | done:tests/browser/rows/VIS1.json | - | Selection visual | MC VIS1; REG2 runs it again |
| OR01 | done:tests/browser/rows/ORD5.json | - | Move | MC ORD5; REG2 runs it again |
| OR02 | done:tests/browser/rows/ORD3.json | - | Stop | MC ORD3; REG2 runs it again |
| OR03 | done:tests/browser/rows/ORD4.json | - | Attack target | MC ORD4; REG2 runs it again |
| OR04 | done:tests/browser/rows/ORD4.json | - | Attack-move | MC ORD4; REG2 runs it again |
| OR05 | done:tests/browser/rows/ORD4.json | - | Force move | MC ORD4; REG2 runs it again |
| OR08 | in | TGT1 | Hold position | MC ORD3. MP1: hold with idle acquisition |
| OR11 | done:tests/browser/rows/ORD3.json | - | Order queue per unit | MC ORD3; REG2 runs it again |
| OR12 | done:tests/browser/rows/ORD3.json | - | Shift-queued orders | MC ORD3; REG2 runs it again |
| OR15 | in | RLY1 | Rally point | MC later:MP1. Visible flag: SMT1 |
| OR16 | in | CON1 | Build order | MC later:MP1. The producer gets the order (C&C form); a worker that walks to the site: not in MP1 |
| OR17 | done:tests/browser/rows/ORD5.json | - | Gather order | MC ORD5; REG2 runs it again |
| OR20 | later:MB | - | Ability order | as MC |
| OR22 | in | MCX1 | Order validation feedback | MC ORD6. MP1: grouped failure notice (O14); also CON1 reasons |
| OR23 | done:tests/browser/rows/ORD7.json | - | Order acknowledgement | MC ORD7; REG2 runs it again |
| OR24 | done:tests/browser/rows/ORD2.json | - | Group order fan-out | MC ORD2; REG2 runs it again |
| OR25 | done:tests/browser/rows/GM40.json | - | Group destination spread | MC GM40; REG2 runs it again |
| OR26 | later:MP2 | - | Order state machine | MC later:MP1. Needs CB05 (later:MP2). MP1 part: TGT1 |
| OR31 | done:tests/browser/rows/ORD6.json | - | Command rate limit | MC ORD6; REG2 runs it again |
| GM01 | done:tests/browser/rows/GM40.json | - | Destination spreading | MC GM40; REG2 runs it again |
| GM04 | done:crates/turts_server/tests/mc_group.rs | - | Local avoidance | MC GMA; REG2 runs it again |
| GM06 | done:crates/turts_server/tests/mc_group.rs | - | Unit collision size | MC GMA; REG2 runs it again |
| GM07 | later:M4b | - | Push idle friends | as MC |
| GM09 | done:tests/browser/rows/GM40.json | - | Arrival | MC GM40; REG2 runs it again |
| GM10 | done:crates/turts_server/tests/mc_group.rs | - | Unreachable target | MC GMU; REG2 runs it again |
| GM11 | later:M4b | - | Choke traffic | as MC |
| CA01 | done:tests/browser/rows/CAM1.json | - | Classic RTS camera | MC CAM1; REG2 runs it again |
| CA02 | done:tests/browser/rows/CAM1.json | - | Edge scroll | MC CAM1; REG2 runs it again |
| CA03 | done:tests/browser/rows/CAM1.json | - | Key scroll | MC CAM1; REG2 runs it again |
| CA08 | done:tests/browser/rows/MM1.json | - | Minimap click | MC MM1; REG2 runs it again |
| CA09 | done:tests/browser/rows/MM1.json | - | Minimap order | MC MM1; REG2 runs it again |
| CA13 | done:tests/browser/rows/CAM1.json | - | Bounds clamp | MC CAM1; REG2 runs it again |
| CA14 | done:tests/browser/rows/CAM1.json | - | Centre on selection | MC CAM1; REG2 runs it again |
| CA15 | later:M4 | - | Height-aware camera | as MC |
| CA16 | done:tests/browser/rows/SH1.json | - | Scroll speed setting | MC SH1; REG2 runs it again |
| UI01 | in | KEY1 | Command card | MC HUD2. MP1: unique card cells and keys |
| UI02 | in | MCX1 | Selection panel | MC HUD2. MP1: value predicate, no 'x1 - HP' |
| UI03 | in | HUD6 | Info panel and portrait | MC HUD2. MP1: armour, load and activity |
| UI04 | in | HUD5 | Resource bar | MC HUD2. MP1: credits and power |
| UI05 | done:tests/browser/rows/MM1.json | - | Minimap | MC MM1; REG2 runs it again |
| UI08 | done:tests/browser/rows/MM1.json | - | Minimap alert flash | MC MM1; REG2 runs it again |
| UI09 | in | HUD6 | Build menu | MC HUD2. MP1: build menu of the command centre |
| UI10 | in | HUD5 | Production queue display | MC HUD2. MP1: two queues, badges, row states |
| UI11 | in | HUD6 | Tooltips | MC HUD2. MP1: cost, power change and key in the tooltip |
| UI12 | done:tests/browser/rows/VIS1.json | - | Health bars | MC VIS1; REG2 runs it again |
| UI14 | later:M5 | - | Fog rendering | as MC |
| UI16 | in | ALR2 | Event alerts | MC ALR1. MP1: new alerts; one alert for a state that persists |
| UI17 | later:M9 | - | Chat | as MC |
| UI19 | in | SUR1 | Surrender and leave | MC later:MP1. Headless: MAT1 |
| UI22 | in | MCX1 | In-game options menu | MC SH1. MP1: Controls groups, keyboard navigation, Unbound |
| UI23 | in | SMT1 | Cursor set | MC HUD4. MP1: new cursor contexts |
| CO01 | in | GHO1 | Placement ghost | MC later:MP1 |
| CO02 | later:M4 | - | Footprint validity | MC later:MP1. Needs CO03. MP1 part (flat ground): GHO1, CON2 |
| CO03 | later:M4 | - | Placement rules | MC later:MP1. Needs MP01 (map format, M4). MP1 part (flat ground): CON1, CON2 |
| CO04 | in | CON1 | Construction model | MC later:MP1 |
| CO07 | in | CON1 | Under construction state | MC later:MP1. Attack on a site: CON3 |
| CO08 | in | CON3 | Cancel and refund | MC later:MP1 |
| CO20 | in | DES1 | Destruction clears footprint | MC later:MP1 |
| CO21 | later:M4b | - | Placement vs units | MC later:MP1. Needs GM07 (push, M4b) and CO03. MP1 part: CON1 refuses a site over a unit |
| PR01 | done:crates/turts_pack/tests/economy_production.rs | - | Production queue | as MC |
| PR02 | in | PAY2 | Cancel and refund | MC done. MP1: refund of what the job charged |
| PR03 | in | PAY1 | Payment policy | MC done. MP1: charge over time (decision P4 b) |
| PR04 | in | PRD1 | Parallel production | MC later:MP1 |
| PR05 | in | PRD1 | Spawn and exit | MC later:MP1 |
| PR06 | in | RLY1 | Rally applied | MC later:MP1 |
| PR07 | done:crates/turts_pack/tests/forge_cases.rs | - | Prerequisites | as MC |
| PR08 | done:crates/turts_pack/tests/forge_cases.rs | - | Research and upgrades | as MC |
| PR10 | in | HUD6 | Unavailable reason | MC HUD2. MP1: queue full and no-credit causes |
| PR15 | done:crates/turts_pack/tests/forge_cases.rs | - | Modifier application | as MC |
| PR17 | later:MB | - | Asymmetric factions | as MC |
| EC01 | in | PWR1 | Resource types | MC done. MP1: a capacity resource kind for power |
| EC02 | in | OBS1 | Gather cycle | MC ORD5. MP1: free approach point at the receiver |
| EC03 | done:crates/turts_pack/tests/economy_gather.rs | - | Drop-off choice | MC later:MP1. M2 test nearest_receiver_wins_before_id_tie_break (:471) |
| EC04 | done:tests/browser/rows/ORD3.json | - | Node depletion | MC ORD3; REG2 runs it again |
| EC05 | done:crates/turts_pack/tests/economy_gather.rs | - | Auto-resume and seek | MC later:MP1. M2 test depleted_original_reselects_nearest_node_after_delivery (:656) |
| EC10 | in | PAY1 | Insufficient funds | MC done. MP1: the job waits for funds and resumes |
| CB01 | done:crates/turts_pack/tests/forge_cases.rs | - | Health and death | as MC |
| CB02 | done:crates/turts_pack/tests/forge_cases.rs | - | Weapons | as MC |
| CB03 | in | TGT1 | Auto-target | MC ORD4. MP1: acquire range and leash |
| CB04 | later:MP2 | - | Target priority | MC later:MP1. MP1 rule: ordered target, then the nearest enemy (TGT1); no threat priority |
| CB05 | later:MP2 | - | Retaliation | MC later:MP1. Not in X2; see the weak-cover list |
| CB07 | in | DMG1 | Damage and armour types | MC later:MP1 |
| CB09 | later:MB | - | Splash damage | as MC |
| CB10 | later:MB | - | Projectiles | as MC |
| CB21 | in | PWR1 | Defensive buildings | MC done. MP1: a turret is off in a power shortage |
| CB25 | later:MB | - | Bonus damage | as MC |
| FV01 | later:M5 | - | Black shroud | as MC |
| FV02 | later:M5 | - | Fog of war | as MC |
| FV03 | later:M5 | - | Sight radius | as MC |
| FV04 | later:M5 | - | Server-side visibility | as MC |
| FV05 | later:M5 | - | Shared vision | as MC |
| FV10 | later:M5 | - | Last-seen structures | as MC |
| UN01 | later:M4 | - | Ground units and movement classes | as MC |
| UN07 | later:MB | - | Abilities | as MC |
| UN11 | later:MB | - | Workers | MC later:MP1. No unit gathers and builds in 16.2 (S:798-800) |
| UN13 | in | DES1 | Death and removal | MC done. MP1: removal of a unit with health and a queue (X1) |
| PF01 | done:tests/browser/rows/GM40.json | - | Sensible path | MC GM40; REG2 runs it again |
| PF02 | later:M4 | - | Repath on change | as MC |
| PF03 | later:M4 | - | Movement layers | as MC |
| PF04 | in | OBS1 | No stuck units | MC GMU. MP1: 0 of 20 tanks Blocked in a compact base |
| PF05 | done:tests/browser/rows/ORD7.json | - | Instant start | MC ORD7; REG2 runs it again |
| PF10 | later:M7 | - | Mass pathing | as MC |
| MF01 | later:M8 | - | Main menu | as MC |
| MF02 | later:M9 | - | Lobby | MC later:MP1. Needs MD12 (later:M9). MP1 part: LOB1, LOB2 (two slots, colour, ready; team = slot, decision P2 a) |
| MF03 | later:M9 | - | Map choice | as MC |
| MF04 | later:M9 | - | Game settings | as MC |
| MF05 | later:MP2 | - | AI in slots | as MC |
| MF06 | later:M4 | - | Start conditions | MC later:MP1. Needs MP01 (M4). MP1 part: fixed starts of Twin Fields, MAT1 |
| MF07 | in | LOB1 | Load and countdown | MC later:MP1. Countdown, one start for both |
| MF08 | in | MAT1 | Victory and defeat | MC later:MP1 |
| MF10 | later:M5 | - | Team victory | MC later:MP1. Needs TM01 (later:M5) |
| MF12 | in | RES1 | Score screen | MC later:M9. MP1: units, buildings, credits, time; no graphs, no APM |
| MF14 | in | MAT1 | Disconnect handling | MC later:M10. MP1: defeat after 120 s disconnected (decision P8 b) |
| SV01 | later:M10 | - | Single-player save and load | as MC |
| SV04 | later:M10 | - | Save version safety | as MC |
| SV05 | later:M10 | - | Mod save data | as MC |
| MP01 | later:M4 | - | Map format | as MC |
| MP02 | later:MB | - | Map editor | as MC |
| MP05 | later:M9 | - | Map metadata | as MC |
| MP07 | done:crates/turts_server/tests/acceptance.rs | - | Map distribution | MC later:M8. The client downloads the locked package with its maps from the server since M1 (t2a_package_parity) |
| MP09 | done:crates/turts_server/tests/mc_group.rs | - | Playable bounds | MC GMU; REG2 runs it again |
| MD01 | in | VAL1 | Unit definition | MC MOD1. MP1: armour, power, placement, repairer, acquire fields |
| MD02 | in | DMG1 | Weapon and projectile definition | MC later:MB. MP1: damage type, acquire, effect ids; projectile travel stays MB (CB10) |
| MD03 | in | CON1 | Building definition | MC later:MP1. Power: PWR1 |
| MD04 | later:MB | - | Ability definition | as MC |
| MD05 | done:crates/turts_pack/tests/forge_cases.rs | - | Upgrade and tech definition | as MC |
| MD06 | later:MB | - | Faction definition | as MC |
| MD07 | in | DMG1 | Damage and armour tables | MC later:MP1 |
| MD08 | in | KEY1 | UI definitions | MC HUD3. MP1: entry hotkey and card fields |
| MD09 | done:tests/browser/rows/AUD1.json | - | Audio definitions | MC AUD1; REG2 runs it again |
| MD10 | in | MAT1 | Victory definitions | MC later:MP1 |
| MD11 | later:MP2 | - | AI definitions | as MC |
| MD12 | later:M9 | - | Lobby settings schema | as MC |
| MD17 | in | MDL1 | Asset pipeline | MC later:M13. MP1: static models and textures (decision P1 b); animation stays M13 |
| MD18 | done:crates/turts_pack/tests/bad_packages.rs | - | Validation messages | as MC |
| BT01 | later:MP2 | - | Skirmish AI | as MC |
| BT02 | later:M12 | - | Difficulty levels | as MC |
| BT03 | later:MP2 | - | Same command path | as MC |
| TM01 | later:M5 | - | Teams | MC later:MP1. Decision P2 a: team = slot; M5 needs teams (S:727) |
| TM02 | later:M5 | - | Alliance effects | MC later:MP1. Needs TM01. MP1 part: Enemy relation in TGT1 |
| TM03 | in | LOB2 | Player colours | MC VIS1. MP1: colour choice in the lobby |
| AU01 | done:tests/browser/rows/AUD1.json | - | Unit responses | MC AUD1; REG2 runs it again |
| AU02 | in | ALR2 | Alerts | MC ALR1. MP1: alert sounds once for each state |
| AU03 | in | AUD2 | Positional effects | MC later:MP1. Decision P5 sound set |
| AU06 | done:tests/browser/rows/AUD1.json | - | Volume categories | MC AUD1; REG2 runs it again |
| AU07 | in | AUD2 | Sound limiting | MC later:MP1 |
| AN01 | later:M13 | - | Unit animations | as MC |
| AN02 | later:MB | - | Weapon effects | MC later:MP1. Needs CB10 (projectiles, MB). MP1 part: FX1 |
| AN03 | in | GHO1 | Construction visuals | MC later:MP1 |
| AN05 | in | FX1 | Death visuals | MC later:MP1 |
| AN06 | done:tests/browser/rows/VIS1.json | - | Decals for orders | MC VIS1; REG2 runs it again |
| AN07 | in | MDL1 | Team colour on models | MC VIS1. MP1: material slot team on models (decision P1 b) |
| AN08 | done:crates/turts_client/tests/interp.rs | - | Tick interpolation | as MC |
| AN11 | later:M13 | - | Instancing and level of detail | as MC |
| AC05 | done:tests/browser/rows/CAM2.json | - | Cursor confine | MC CAM2; REG2 runs it again |
| AC06 | later:M13 | - | Graphics settings | as MC |
| PE01 | done:tests/browser/rows/ORD7.json | - | Order responsiveness | MC ORD7; REG2 runs it again |
| PE02 | later:M7 | - | Stable frame rate | as MC |
| PE03 | later:M7 | - | Unit count | as MC |
| PE04 | done:tests/browser/rows/SEL2.json | - | Large selection speed | MC SEL2; REG2 runs it again |
| PE07 | done:tests/browser/rows/ORD7.json | - | Latency hiding | MC ORD7; REG2 runs it again |

## Player path

This is the oracle list of PT3. The testers never see it: they get the goals text of section 13. The source is
UX part 1, corrected by PT1 person run 1 (2026-11-11: no goal, no first action, an alert sound that repeats).
It assumes decisions P2 (a), P4 (b), P5 and P8 (b). Setup: two browser windows A and B, or two machines;
mouse and keyboard only. The brackets name the rows of section 12 that check the step.

Lines L1 to L5 are the lobby. They come first, but their time depends on the second person, so they have no
clock. The clock of steps 1 to 3 starts at the click on "Click to play". Steps 1 to 3 are the first 60
seconds: a person with no knowledge of the game must do them from the screen alone. PT3 is red when a tester
does not see the site of step 3 within 60 s of that click.

- L1. A: the load screen shows each phase and a progress bar (MC SH1).
- L2. A: the lobby shows the goal line "Destroy the enemy command centre", the rows "Slot 1: You — Not ready"
  and "Slot 2: Open — waiting for a player", and the status line "Waiting for a second player" (LOB1).
- L3. B joins. Both windows show "Slot 2: Player 2 — Not ready". Each slot has a different default colour
  (LOB1, LOB2).
- L4. A picks the colour of B: the swatch shows a cross, and its tooltip says "Player 2 has this colour". A then
  picks a free colour, and both windows show it (LOB2).
- L5. Both press Ready. The rows show "Ready", then "Match starts in 3", "2", "1". A change by a player clears
  the ready state of that player. Both windows show "Click to play" (LOB1).

1. In the first frame after "Click to play": the camera shows the own command centre, and it is selected. The
   top line says "Destroy the enemy command centre". The HUD shows the credits and the power, with "Low power",
   because the command centre uses power and nothing makes it (GOAL1, HUD5).
2. One hint line names one first action and its key: "Build a Power Station: press P, then click near your
   command centre." The tester presses P or clicks the entry. The ghost follows the pointer. The tester
   left-clicks where the ring is solid (GOAL1, GHO1).
3. The reward shows at once: the site shows in the same frame, then a progress bar that grows, and the credits
   count down. When the station is complete, the alert "Construction complete" shows with its sound, the power
   text shows more power made than used, "Low power" goes, and the hint line names the next action (GOAL1,
   GHO1, ALR2, PWR1).
4. The tester hovers the Turret entry. The tooltip shows the name, the cost with "paid while it builds", the
   time, the power change and the key (HUD6).
5. The tester presses the Turret key. Over the harvester the ghost shows stripes and "Something is in the way";
   over ore, "Cannot build on ore"; far out, "Too far from your base". A right click ends placement: the ghost
   goes, and the credits do not change (GHO1).
6. The tester selects the Ore Hauler and right-clicks the ore field. The gather cursor shows, then the gather
   marker. The load in the info panel goes up. At the command centre, "+N" rises, and the credits go up by N
   (SMT1, HUD6, HUD5).
7. The tester selects the command centre and presses the Rifleman key three times. The entry shows the badge
   "3". The first row of the unit queue shows its progress and the seconds left. A structure job in the other
   queue progresses at the same time (HUD5, PRD1).
8. The tester right-clicks the ground with the command centre selected. A flag shows, with a line from the
   command centre. The new rifleman walks to the flag, and the alert "Unit ready" shows (SMT1, RLY1, ALR2).
9. The tester right-clicks the active job in the queue. The job goes, "+N" rises at the command centre, and the
   credits go up by N, the amount that the job paid. A cancel of a waiting job gives back nothing, because it
   paid nothing (PAY2, HUD5).
10. The tester builds turrets until the base uses more power than it makes. The power text shows "Low power"
    with a mark. The alert "Low power" shows once. The queue rows say "Slowed: low power". Each turret shows an
    offline mark. A second Power Station clears all of these (PWR1, HUD5, ALR2).
11. The credits reach 0 during a job. The row says "On hold: no credits", with a striped bar, and the alert "Not
    enough credits" shows once. The job continues after the next delivery (PAY1, HUD5, ALR2).
12. B selects units, presses A and left-clicks near the base of A. The attack cursor shows, then a cross marker at
    once. The units move, and they fire by themselves at enemies in range (TGT1, MC ORD4).
13. In the fight: muzzle flashes, tracers, impacts, and health bars on damaged units. A weapon sound near the
    camera is louder than a far one. A gets "Base under attack" with a minimap ping. A click on the alert line
    centres the camera on the base (FX1, AUD2, ALR2).
14. An enemy turret turns to its target and fires (FX1).
15. Units die with a death effect, and they leave the selection and their groups. A partly refused order shows
    "3 of 8 units: Cannot reach that place" with one error sound. A click on the line selects the failed units
    (FX1, MCX1).
16. The engineer of A right-clicks a damaged own building. The repair cursor and the repair effect show, the
    health bar rises, and the credits go down. At 0 credits the repair stops (SMT1, REP1).
17. A command centre is destroyed. Within 1 s, both windows show "Victory" or "Defeat". After 3 s, the result
    screen shows both slots, the outcomes and the statistics. "Leave" opens the shell screen "The match is over.
    Reload this page to join the next match." (MAT1, RES1).
18. In a second match: A opens the menu and selects "Surrender". The confirmation text shows. Cancel changes
    nothing. Confirm defeats A, and both windows show the result (SUR1).
19. In a third match: B closes its window. A shows "Player 2 lost connection. Waiting 2:00 for them to return."
    When B does not return within 2:00, B is defeated, and A shows the result. When B reloads within 2:00, B
    rejoins its slot and the match goes on (MAT1, ALR2, MC SLOT1).
