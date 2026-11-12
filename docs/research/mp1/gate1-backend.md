# MP1 gate 1: backend lens

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=FAIL FINDINGS=10 HIGH=4 RUNS=9 NOT_CHECKED=5

This is the backend-engineer lens on MP1 design revision 3. The design gives the match lifecycle, the lobby and the result only four lines (design:176-184). The Player path depends on more than that. All the changes I ask for are text changes, but H1 needs a decision.

## Findings

**H1. The design has no server state machine, and it has no path to a next match.**
- The Player path needs a second match and a third match (design:706-710), and "Leave" says "reload … next match" (design:705).
- The code builds the world one time, at boot (boot.rs:38, main.rs:72). The match_id is also made one time (main.rs:78). Slots are never released (slots.rs:20,46-57).
- Probe 1: the sim ticked with no client (39 ticks after 3 s, 0 hellos). Client A was welcomed at tick 76, and client B at tick 171. So 171 ticks ran before the second player came.
- Add to 6.7:
    - Boot → Lobby, or Boot → Running when the policy is Immediate.
    - Lobby: the world is built, the sim does not tick, and seats are reservations.
    - Lobby → Countdown when the policy is satisfied.
    - Countdown → Lobby when a seat changes, a player leaves, or a player clears ready.
    - Countdown → Running: an atomic claim of all seats, then Welcome.
    - Running → Ended when every slot has an outcome (`MatchStopped` exists: plugin.rs:157-176).
    - Ended → exit 0 after `after_match_secs`.
- Define these events: the second player never comes; a player leaves during the countdown; all players disconnect (S:327 makes this server policy, so it is a `server.ron` field); a client connects after the end (the owner gets the result; other ids get "The match is over").
- **Needs decision.** Option (a): the process exits after Ended and a supervisor restarts it. This is the S:296 recommendation, it needs no reset code, and it gives a new match_id. Option (b): reset the world in the process. That means about 30 resources (plugin.rs:60-90), with a risk of stale state. I recommend (a).

**H2. The design does not define the claim boundary.**
- S:308 requires the exact claim boundary to be designed. S:350 says lobby edits need their own lifecycle semantics.
- Today the claim happens at Hello (session.rs:154).
- Add this text: "A lobby seat is a reservation that the room can release. The match start claims every seated slot in `Slots` in one step. From that tick D25 holds, and no leave, disconnect, defeat or admin action releases a claim. Under `Immediate`, the claim stays at Hello (late admission, S:337)."
- Add to LOB1: "after the start, a new id gets `no open slot`". Red build: the claim is released on leave.

**H3. The design does not give a default start policy, so the existing instruments would stop working.**
- 31 ignored process tests in `turts_server/tests` start a server with 0 or 1 client: mc_group (19 tests), unit_boot, det1, el1, command_script, and t4, t5, t6b and t8 (acceptance.rs:682-706). hd1, T7 and the MC rows do the same.
- In a lobby these tests never start. `--run-ticks` never fires, because the tick must advance (main.rs:139-142).
- Add this text: "A package with no `start_policy` gets `Immediate`. It has a version-table row, since 0.2.4."
- Add the server test hook `--start-now`. Without it, every `frontier` headless row that uses `--command-script` (DMG1, CON1, PWR1, PAY1, OBS1) has no client to press ready.

**H4. The disconnect rule and surrender must be sim inputs.**
- P8 (b) is a mechanism, not policy, only if two things are true:
    - The engine supplies only the connectivity facts. The rule is a victory clause that the mod declares, and the engine has no default. With no clause, nothing happens (D28).
    - Connect, disconnect and surrender go into the sim as envelopes at a fixed tick, and the command log records them (G9, invariant 15, RLY1). A server timer that uses wall-clock time breaks replay.
- No hook exists today: a grep for disconnect finds only the comment at session.rs:710.
- Define the cases:
    - Both slots reach N in the same tick: draw.
    - The slots reach N at different ticks: the first slot to reach N is defeated.

**M5. The wire must not carry another player's player_id.**
- The id is a bearer value: `judge_hello` gives a held slot to any Hello that has the id (session.rs:154-156). Today only Hello carries it (messages.rs:21). TD proposes a seat `Human(id)`.
- Add this text: "Until D50, a seat on the wire is {index, occupant Open|Human{label}|Bot{content}, ready, colour}."
- Lobby state: {revision, you, seats, settings, start}. `settings` is a map from content id to value. The server validates it against the mod schema (S:314). It is empty in MP1.

**M6. "Teams and colours at the welcome" (design:189) works only if Welcome is sent at the match start.**
- Define the message order:
    - With a lobby: Hello → lobby state → Welcome{slot, tick, match_id, colours} → Snapshot.
    - Under `Immediate`: Hello → Welcome, as today.
- The browser saves the player id only at `Joined` (turts_client_web/src/lib.rs:254-260). Move that to the lobby seat. If you do not, a reload in the lobby loses the seat.

**M7. The server detects a disconnect late, and the headless client cannot rejoin.**
- Probe 4: after a native client exits, the server refused the same id until +14.5 s and accepted it by +15.8 s (debug build). MC 10.5 assumed 10 s, with a 15 s client window.
- The native headless client has no rejoin retry (turts_client/src/main.rs:35-43). Probe 3: one Hello, a refusal, exit 1.
- Say that "disconnected" means the server session ended. Bound the timing of the LOB1 leave text and the 120 s count (step 19) on this lag.

**M8. The hooks and the instrument reds are missing.**
- Add `--actions` kinds instead of new flags: `ready`, `unready`, `set_colour`, `surrender` (actions.rs:20-46).
- Today the actions are timed from the Welcome tick (actions.rs:107-118). That tick does not exist in the lobby. Time lobby actions from the seat event or by wall clock.
- Add headless `--rejoin`.
- Add report fields: `lobby_states`, `outcome`, `match_over_tick`.
- Put N in a test package, not in an engine override.
- LOB2's red build "the client decides the colour" can be red only from a headless client that sends a colour the other slot holds. A pixels driver cannot click a crossed swatch.
- MAT1's red build "outcome from the unit list" can be red only in the surrender case and the disconnect case.

**L9. Persistence (6.12): lobby state in memory is fine, but some things must not happen.**
- No ownership outside `Slots`.
- No outcome or timers in server resources.
- No configuration changes after the start (S:308).
- Keep one serializable `MatchConfig` {lockfile hash, seats→slots, colours, settings, start tick}.

**L10. Hosting at D57: nothing to measure, if the sim does not tick in Lobby.**
- The victory and connectivity checks cost O(slots) for each tick.
- A restart costs about 1 s of boot (muster, debug, 42 MB resident). Record the boot-to-ready time for `frontier` in mp1.md.

## What I ran
Each row: what I ran | claim tested | result that would refute it | observed.
- `be-probe/probe.sh` | design:180, Q1 | tick 0 before a join | 39 ticks with no client; welcome ticks 76 and 171; C refused `no open slot`; exits A=0 B=0 C=1 A2=1 server=0
- `probe2.sh` | K27, MCD 10.5 | the same id refused after 20 s | the same id got slot 0 again; a new id got slot 1
- `probe3.sh` | MCD 10.5 retry | rejoin welcomed within 25 s | refused once, exit 1
- `probe4.sh` | MCD 10.5 "10 s" | welcomed within 2 s or less | refused until +14.5 s
- grep disconnect, server and sim | design:53 | a handler exists | comment only
- grep player_id, protocol | M5 | id in another message | messages.rs:21 only
- count of process tests | H3 | no test with 0 or 1 client | 31 ignored tests + hd1
- grep restart/next match in the design | H1 | a lifecycle text | only design:705-710
- `cargo build -p turts_server`: exit 0. `-p turts_client`: failed on alsa-sys pkg-config. That is the environment, not a red, so I used the mc-fix-hud2 debug client. Its crates tree has no diff from mp1 HEAD; the build-after-commit is shown by timestamps only.

All probe paths are under `/tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/be-probe/`. `git status --porcelain` is empty.

## Not checked
- How the browser closes its WebSocket: NOT TRACED.
- Release-build timings.
- Lightyear timeout internals.
- The pre-MP1 slice.
- Two headed windows on one GPU.

## What I changed
None.

## Open questions for the orchestrator
- H1 decision: (a) process per match with a restart, recommended; or (b) a reset in the process.
- Does the CEO accept `Immediate` as the default policy, and `--start-now` as a test hook?
