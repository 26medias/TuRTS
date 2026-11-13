VERDICT=REPAIRED
## Replace: `### 6.7 Match lifecycle, lobby, result (X11, X12, X13)`
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
---
## Replace row: `LOB1`
| LOB1 | Lobby start and claim boundary (D-B), package `standoff`. Run 1, headless clients A, B, C: A seats: A's first `lobby_states` entry has `you: 0`, seat 1 `Open`, start `Waiting`, and A has no `welcome`; server `--stats` `ticks` stays 0 for 3 s. B seats: both reports show two `Human` seats with different colours. Only A is ready for 3 s: start stays `Waiting`, no `welcome`. B ready: start `Countdown`; B's `lobby_move` in the countdown gets `match_not_started`. Both `welcome` records have `tick` 0, `start_tick` 1, and equal `teams` and `colours`; the first snapshot tick is 0 for both; the tick log starts at tick 1. After the start, C (new id) gets `no open slot`; B killed, then B with `--rejoin` gets slot 1 again. Run 2: B sends `unready` in the countdown: start goes back to `Waiting` and no `welcome` comes; B `ready` again: the match starts. Run 3: A and B seated, B killed: within 11 s A's lobby state shows seat 1 `Open` (the time is recorded as L), and C gets seat 1. Pixels, two windows: the words of L2 to L5, "Match starts in 3", "2", "1", and both windows show "Click to play" | the sim claims at `Hello` (run 3: C gets `no open slot`); the sim ticks in the lobby (the `welcome` ticks are above 0 and differ, as today: 76 and 171 in the gate 1 probe); a start with one ready (run 1); an order admitted before the start (run 1: `lobby_move` gets another result); `unready` does not stop the countdown (run 2) | headless, three clients; pixels, two windows |
---
## Replace row: `LOB2`
| LOB2 | Colour choice. Headless, two clients: at seating A holds colour 0 and B colour 1 (the lowest free index). B sends `set_colour` 0: B's `lobby_refusals` has `colour_taken`, and the last lobby state in both reports still has A 0 and B 1. A sends `set_colour` 2: both reports show A 2, and A's ready flag is cleared. The `welcome` `colours` of both clients equal the last lobby state. Pixels, two windows: L4 (the swatch of B's colour shows a cross and the tooltip "Player 2 has this colour"; a click on it changes nothing); after A picks colour 2, the units, rings and minimap marks of A have palette colour 2 in both windows | the server does not check the colour (headless: both seats hold 0 after B's `set_colour`; a pixels driver cannot reach this red, because the crossed swatch does nothing on click); the client takes the slot colour from the palette by slot index and not from `MatchStart.colours` (pixels: A's rings show palette colour 0); two seats get one default colour | headless, two clients; pixels, two windows |
---
## Replace row: `MAT1`
| MAT1 | Outcomes (X11, D-D), package `standoff` (limit 100 ticks at 20 Hz). Scripts, `--start-now --command-script`: (a) attack orders kill B's key unit at tick T: A `Victory`/`OpponentsDefeated`, B `Defeat`/`KeyUnitsLost`, `match_over_tick` T, and the tick log has no tick after T; (b) both key units die in one tick: `Draw` for both; (c) `connection: false` for both slots at tick 10: `Draw`/`Disconnected` at tick 110; at ticks 10 and 20: slot 0 `Defeat` at 110 and slot 1 `Victory` at 110; (d) slot 0 false at 10 and true at 60: no outcome at 110. Two headless clients: (e) B `surrender` at tick 40: both reports have the outcome with the reason `Surrendered` and the same `match_over_tick`, and B's key unit is alive at the end; (f) B killed: the input log has `connection: false` for slot 1 at tick D, at most 11 s after the kill, and B gets `Defeat`/`Disconnected` at D + 100; (g) B killed, then B with `--rejoin` before the limit: `connection: true`, and no outcome by D + 200; (h) the `--input-log` of (e) and of (f), replayed with `--start-now --command-script`, gives the same outcomes and `match_over_tick`; (i) `muster` (no victory data), one client killed: no outcome after 300 ticks; (j) after the end, a new id gets `The match is over`, and B (an owner that did not leave) gets `welcome` and the same `match_result`. A sim test with three slots: an order from a defeated slot gets `slot_defeated` | the client works out the outcome from the unit list ((e) and (f): `outcome` is null, because B's key unit is alive; in case (a) the two agree, so only (e) and (f) can show this red); the disconnect count is a wall-clock timer in the server ((h): the replay has no connections, so it gives no outcome); a defeat with no clause ((i), D28); a draw rule that gives `Victory` to the lower slot index ((b), (c)) | headless, two clients, and scripts |
---
## Replace row: `RES1`
| RES1 | Result (D-K). Pixels, two windows: within 1 s of the end tick both windows show the outcome and the reason line (7.6 words, for example "Player 2 surrendered"); within 3 s more, the result screen shows both slots, their outcomes and reasons, and the statistics; the oracle compares each number on the screen with the `match_result` of that client. Wire check (net 10), headless, two clients: in each report `stats_first_seen_tick` equals `match_over_tick`, and each client got exactly one `MatchResult`; an export unit test in `turts_server` builds the per-slot export at each tick of a scripted `standoff` match and finds no statistics before the end tick | the sim view gives the statistics at each tick (headless: `stats_first_seen_tick` is 1; the export test is red at tick 1); a result screen without the reason (pixels); a pixels driver alone cannot show the first red, because a client can hold a value and not show it | headless, two clients; pixels, two windows |
---
## Replace row: `SUR1`
| SUR1 | The menu item "Surrender" shows "Surrender and end the match? You will be defeated." Cancel: the server `--input-log` has no surrender envelope, and the match goes on for 5 s. Confirm: one surrender envelope; the server ends the match at the tick of that envelope; within 1 s both windows show the outcome (B "Defeat", A "Victory") with the reason, then the result screen. In each window, the first frame that shows the outcome has a replica tick at or after `match_over_tick` (frame log) | surrender with no confirmation (the envelope goes at the first click); the other window shows no result (`MatchResult` sent only to the slot that surrendered); the outcome shows before the server end tick (the client decides the outcome) | pixels, two windows |
---
## New row, after `SUR1`
| IMM1 | Default start policy `Immediate` (D-B). A package with no `exports.match` boots to Running: `muster` with no client has `--stats` `ticks` above 0 after 3 s, and a `Hello` gets `welcome` at once with `tick` above 0. The 31 ignored process tests of `crates/turts_server/tests`, hd1, T7 and the MC rows pass with no change to their commands. `standoff` with `--start-now` and a `--command-script` runs the script with no client. A `^0.2.3` package that writes `exports.match` is refused with "`exports.match` needs engine api 0.2.4; `^0.2.3` admits 0.2.3"; a `^0.2.3` package with an order of behaviour `Surrender` is refused with its row name | the default policy is a lobby (`muster` has 0 ticks; the `mc_group` tests time out); `--start-now` ignored (the `standoff` script run has 0 ticks); a version-table row missing (the `^0.2.3` package packs) | headless |
---
## New row, after `IMM1`
| LEAVE1 | One match for each process (D-C, D-K). Headless, two clients finish a `standoff` match, and each runs `leave`: the server exits with 0 within 2 s of the second `LeaveMatch`. Before that exit, a client that left gets `The match is over`, and an owner that did not leave gets `welcome` and `match_result`. With no `leave`, the server exits with 0 at `after_match_secs`. `scripts/mp1-serve.sh` starts a new process after exit 0; a new `Hello` gets a lobby seat, and the `welcome` of the next match has a different `match_id`. With a plant `server.ron` that names a missing `game_package`, the script stops with exit code 1 and does not start again | the server stays in Ended (no exit); a world reset in place (the same `match_id` after the restart); the script restarts after a non-zero exit; an owner that did not leave gets `The match is over` | headless and the script |
---
## Not done
- player 11 (separate instructions for the second and third match): section 13, not my section.
- rigour 9 (the "Changes" column of P8 leaves out MAT1, MF14 and step 19): section 2, not my section.
- net 6 (a visibility class for each record): the table of 6.8 is the technical-director's. Section 6.7.13 gives the class of each record of this section.
- net 5 (the token bucket first for every envelope): 6.8. Section 6.7.7 says that `match_not_started` costs a token, so it agrees.
- mod H1 (the full 0.2.4 table): section 6.10. Section 6.7.4 names the match fields, defaults, checks, texts and rows for that table.
- The 7.6 words for the lobby, the reasons and the result screen: the UX seat. Section 6.7 gives only the texts that D-K and D-C need.
- backend M7, browser part: whether a reload or a tab close sends a clean close is NOT TRACED. LOB1 run 3 and MAT1 (f) record L; the plan must add a browser measure of L before step 19 is tested.
- The bearer player id in the server stderr line (`session.rs:264-267`): D50 work, not MP1.
## Decisions for the CEO
1. `all_disconnected` default (S:327). Options: (a) `Continue`: the sim ticks, and the mod clause ends the match; (b) `Shutdown(after_secs)`: the process ends with no outcome. Recommendation: (a). Pause and save need M10.
2. The D23 automatic start `AllSeated`. Options: (a) build it in MP1 (one more LOB1 case); (b) reserve it for M9 with the creator start. Recommendation: (b): MP1 has no room creation, and `frontier` uses `AllSeatedReady`.
3. When Ended exits. Options: (a) when no session remains, or after `after_match_secs` (default 60); (b) only after `after_match_secs`. Recommendation: (a): two players who press Leave get the next match at once.
4. The disconnect notice L (10 s today, `join.rs:345`). Options: (a) keep 10 s; (b) lower it (it changes MC 10.5 and the rejoin window margin). Recommendation: (a) for MP1; it is a network-seat item.
