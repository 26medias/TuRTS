VERDICT=fail ITEMS=51 MISSING=14 PARTIAL=29 CONFLICT=2 RUNS=10 NOT_TRACED=6

# Gap audit: backend-engineer (spec 6.4, 6.6, 6.7, 6.8, 6.12, 6.13; match flow, saves, replays, reconnect, profile)

Spec = `RTS-Engine-Project-Specifications.md` v0.7, cited S:line. Other counts: present 5, wrong-milestone 1.
Status key in notes: [D] Decided, [B] Baseline, [O] Open, [P] Proposed.

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A-MF01 | Main menu (skirmish, multiplayer, replays, options, mods) | partial | Server list and saved servers S:257, S:270 [D]/[B]. "menu" 0 hits. | engine (client shell) | none (M8 covers index only, S:683) | M8 | index, profile store (backend-10) | yes: client takes only `--server`/`--ws` (CLAUDE.md options); no browser. Screen NOT TRACED | Add a menu and server browser to M8 acceptance. |
| A-MF02 | Lobby (create, join, slots, ready, team, colour, faction, start position) | partial | Rooms S:288-292 [D]/[B]; admin table S:294-305 [D]; slots S:317-332 [D]. Team only as schema example S:290 [B]. Ready "suggestion only" S:311. Colour, faction pick: 0 hits. | both | M9, M10 (S:684-685) | M9 | backend-2, backend-3 | NOT TRACED (no room code exists) | The spec designs ownership, not a roster a player sees. |
| A-MF03 | Map choice with preview and player count | conflict | "map" only as schema example S:290 [B]. Server lockfile frozen for server lifetime S:259 [B]. Map metadata/preview: 0 hits. | both | none | M9 | MP05, map packages in lockfile | NOT TRACED | Frozen per-server lockfile [B] blocks a creator who brings a new map (SC1/WC3 host map). Needs decision. |
| A-MF04 | Game settings, server validates | present | Settings schema, client-generated controls, server validates S:290 [B]; frozen per match S:284 [B]. | both | M9 | M9 | MD12 | NOT TRACED | Room-level only; see backend-2 for per-slot scope. |
| A-MF05 | AI in slots | present | Creator assigns mod bots to unclaimed lobby slots S:301 [D], S:780, S:786. Difficulty open S:780 [O]. | both | none named | M9 | BT01 | no | Bot slot identity is implementation detail S:786. |
| A-MF06 | Start conditions (start units, fixed/random positions) | partial | Slot independent of spawn S:317 [D]; spawn rules rejected as engine S:332 [D]; "what joining creates" is mod S:330; first mod S:749 [D]. Hook list: none. | policy (engine hook) | M10 | M9 | backend-1 (start hook) | yes: start assets are static map data (`turts_mod_api/src/schema.rs:449` `SlotEconomyInit`) | Mod needs a "match start, per slot" hook. |
| A-MF07 | Load barrier and countdown | partial | Loading barrier is implementation S:311; countdown suggestion S:311; claim boundary open S:284, S:714 [O]. | engine | M9 | M9 | backend-3 | NOT TRACED | |
| A-MF08 | Victory and defeat | partial | Engine "lifecycle, event hooks" S:163 [D]; mod victory S:163; first mod rule S:748 [D]. "defeat", "eliminat", "match end", "game over": 0 player hits. | both | none | M9 | backend-1 | yes: no outcome type; M3 defers victory (`docs/superpowers/specs/2026-10-27-m3-industrial-design.md:55`) | Rule exists; no engine outcome state. |
| A-MF09 | Defeated player observes | missing | Searched defeat, eliminat, observe. Spectating claims no slot S:328 [D]; spectator permission S:292. | both | none | M11 | backend-1, RO03 | no | Defeated slot stays claimed (D25); its view policy is undefined. |
| A-MF10 | Team victory | partial | "last player or team" S:748 [D]; teams engine S:163. No team relation in slot contract S:317-332. | both | none | M9 | A-TM01 | yes: see A-TM02 | |
| A-MF11 | In-game diplomacy | partial | Diplomacy mod policy S:163; during-play rule change needs mod support S:300 [D]. | both | none | M9 | A-TM02 | no | |
| A-MF12 | Score screen | missing | "score" 0 hits; "statistics" S:751 is balance data. Room persists "for results" S:272 [B]. | both | none | M9 | backend-1, backend-8, CB24 | NOT TRACED | Engine counts and stores; mod picks stats and screen (S:164). |
| A-MF13 | Matchmaking and ladder | missing | Searched matchmak, ladder, rank, rating: 0 hits. | engine | none | after M12 | D50 identity, index | no | Identity is linkable but not unique S:386; rating per identity is weak. |
| A-MF14 | Disconnect handling (wait, drop vote, takeover) | partial | Ownership kept S:299, S:326 [D]; entity behaviour mod S:330; no engine AI takeover S:332 [D]; temporary control mod S:786. Wait dialog, drop vote: 0 hits. | both | M10 | M10 | UI18 pause, connectivity hook | NOT TRACED | Mod needs a "slot connectivity changed" hook and match pause. |
| A-MF15 | Reconnect | present | Restores claimed slot S:253 [D], S:328 [D]; M10 S:685. Catch-up is implementation S:253. | engine | M10 | M10 | backend-4 | yes: `crates/turts_server/src/session.rs:103-105` rejects a returning `player_id` with `REASON_ALREADY_IN_MATCH` | Correct for M1 scope; M10 must route it to the slot. |
| A-MF16 | Campaign and scenarios | conflict | No standalone solo mode, no local saves D41 S:64 [D], S:253 [D]. Server-hosted bot matches allowed S:253. | both | none | after M12 | D41, MP04 | no | Offline campaign blocked by D41. Server-hosted campaign possible. |
| A-MF17 | Handicap | missing | "handicap" 0 hits. | policy | none | M9 | backend-2 | no | Needs per-slot settings. |
| A-MF18 | Random faction and map | partial | Seeded random streams S:213 [D]. Lobby random choice: 0 hits. | both | none | M9 | backend-2 | no | Server resolves the random choice at start. |
| A-MF19 | Co-op vs AI | partial | Bots in slots S:301; teams undesigned (A-TM01). | both | none | M9 | A-TM01, BT01 | no | |
| A-UI18 | Pause | partial | Only "server-configured pause/save/shutdown" when everyone disconnects S:303 [D]. Player pause: 0 hits. | both | none | M9 | fixed ticks S:237 | NOT TRACED | Engine suspends a match tick; mod/room sets who and how often. |
| A-UI19 | Surrender and leave | missing | Searched surrender, resign, concede, forfeit: 0 hits. Leave keeps ownership S:326 [D]. | both | none | M10 | backend-1 | no | Concede is an outcome, not a connectivity change. |
| A-UI20 | Game speed | missing | "speed" hits are movement, research, belts (S:156, S:290, S:414). Tick rate open S:110 [O]. | both | none | M9 | tick rate decision | yes: `tick_hz` is server-wide (`crates/turts_server/src/config.rs:72`), not per room | Speed changes CPU per room (D58). |
| A-RO01 | Replay recording | partial | Optional, secondary S:253, D41 S:64; retention open S:364 [O]. | engine | none | after M11 | spectator feed S:364 | no | Not deterministic (S:243): a replay is a state stream, not a command log. |
| A-RO02 | Replay controls | partial | Seek open S:364 [O]. | engine | none | after M11 | A-RO01 | no | |
| A-RO05 | Replay version safety | partial | Save holds exact lockfile S:253 [B]; replay not stated. | engine | none | after M11 | A-RO01 | no | |
| A-SV01 | Save and load a skirmish | partial | Server saves S:253 [D]; contract S:253 [B]. Save trigger, load flow, save list: 0 hits. | engine | none (S:694 "additional") | M10 | backend-5 | yes: no persistence; no SQLite crate (grep of all Cargo.toml: 0 hits) | D41 allows only server saves. |
| A-SV02 | Multiplayer save and load, same players | partial | Saves bound to identity S:386 [D]; ownership kept on restore S:303. Load-to-room flow: none. | engine | none | M10 | backend-4, backend-5 | yes: as A-SV01 | |
| A-SV03 | Autosave | partial | Policy on all-disconnect S:303; crash recovery is implementation S:253. | engine | none | M10 | backend-5 | yes: as A-SV01 | |
| A-SV04 | Save version safety | present | Exact lockfile S:253 [B]; D42 S:65 [D]. Engine-version handling "still need" S:253. | engine | none | M10 | backend-5 | no | |
| A-SV05 | Mod save data | partial | Dynamic mod-state serialization open S:714 [O]. | both | none | M10 | script runtime S:705 [O] | NOT TRACED | |
| A-TM01 | Teams set in lobby | partial | Engine "teams" S:163 [D]; teams as schema example S:290 [B]; team-scoped discovery S:251, S:547. No team model. | both | none | M5 then M9 | backend-2 | yes: no team type in `turts_sim` (grep) | M5 (S:680) needs team scope by invariant 9 (S:547), but no milestone builds teams. |
| A-TM02 | Alliance effects | partial | Shared vision mod S:162; team victory S:748. No relation query designed. | both | none | M9 | A-TM01 | yes: target filter is slot equality only (`crates/turts_sim/src/economy/combat.rs:182-186`) | `Other` will hit allies. |
| A-TM03 | Player colours | missing | "colou?r" 1 hit, S:531 (overlay). | both | none | M9 | backend-2, AN07 | yes: one material for all units (`crates/turts_client/src/render.rs:405`) | |
| A-AC01 | Rebindable keys (persisted) | missing | Searched key bind, rebind, hotkey, keymap: 0 hits. | engine (client shell) | none | M8 | backend-10 | NOT TRACED | Mod declares actions and defaults; client stores bindings. |
| B-36 | Win and lose screen with reason | partial | Rule S:748 [D]; screens are mod S:164. Outcome reason: 0 hits. | both | none | M9 | backend-1 | NOT TRACED | |
| B-38 | Pause or menu, surrender | missing | As A-UI18, A-UI19. | both | none | M9/M10 | backend-1 | no | |
| B-70 | Score screen with graphs | missing | As A-MF12. | both | none | M9 | backend-8 | no | Graphs need time series per slot. |
| B-71 | Replays and observer mode | partial | Spectators [D] S:362; replays secondary S:253. | engine | M11 (viewers) | M11 + later | A-RO01 | no | |
| B-76 | Pause (single and by agreement) | partial | As A-UI18. | both | none | M9 | | no | |
| B-77 | Rebindable hotkeys in game | missing | As A-AC01. | engine | none | M8 | backend-10 | NOT TRACED | |
| B-78 | Custom game modes, victory by mod | partial | Policy S:163, S:290 [B]. No outcome mechanism. | both | none | M9 | backend-1 | no | |
| backend-1 | Match lifecycle states and per-slot outcome (lobby, loading, running, paused, ended; playing, defeated, won, drawn) | missing | "lifecycle" only as table cells S:163, S:265, S:272, S:374; no state list. Outcome: 0 hits. | engine (mod sets outcome) | none | M9 | slot contract S:326 | yes: none built | Outcome is a slot property beside connectivity; it must not change ownership (D25). |
| backend-2 | Per-slot settings scope and authority (team, colour, faction, handicap, ready) | missing | Schema is room-level S:290; creator edits rules S:300. Per-slot value or owner edit: 0 hits. | engine (mod declares fields) | none | M9 | MD12 | NOT TRACED | Who edits a slot's value: owner, creator, or both. |
| backend-3 | Pre-start occupancy: claim or reservation; slot switch in lobby | partial | "Lobby edits before match initialization require their own lifecycle semantics" S:326 [D]; claim boundary open S:284, S:714 [O]. | engine | M9/M10 | M9 | backend-2 | NOT TRACED | Needs decision: claimed is terminal, so a lobby team switch needs a pre-claim state. |
| backend-4 | Cryptographic identity has no milestone | wrong-milestone | 6.13 [D] S:382-388; no row in S:676-688 names identity. | engine | none | M10 or earlier | auth component [O] S:714 | yes: client picks an unauthenticated `u64` (`crates/turts_net_protocol/src/messages.rs:21`; random default `crates/turts_client/src/options.rs:17`) | Reconnect (M10) on an unauthenticated id lets any client take a slot. Identity must land with or before M10. |
| backend-5 | Saves and restart have no milestone; SQLite absent | partial | Only "additional proposed validation" S:694; D40 S:63 [D]. | engine | none | M10 | backend-6 | yes: no SQLite dependency in any Cargo.toml | Long matches (M12) need restart safety. |
| backend-6 | Room owns slots and outlives simulation | present | Room owns slots S:265 [B]; separate lifecycles S:272 [B]. | engine | M9 | M9 | | yes: `Slots` is a Bevy `Resource` in the simulation (`crates/turts_sim/src/slots.rs:22-25`) | Results and rematch need slots outside the match world. |
| backend-7 | Rematch semantics | partial | "persist afterward for results or rematches" S:272 [B]. | engine | none | M9 | backend-1, backend-3 | no | A rematch is a new match with new slots; re-seat flow is undefined. |
| backend-8 | Results record: content, retention, access | missing | Searched results, statistic, history: only S:272. | engine | none | M9 | backend-1, D40 | no | Bind to identity (S:386); scope reads to participants by default. |
| backend-9 | Display-name ambiguity in lobby | partial | Names are labels S:382 [D]. Fingerprint display: 0 hits. | engine (client shell) | none | M9 | backend-4 | no | Two "Bob" entries need a key fingerprint. |
| backend-10 | Player profile store (name, identity, saved servers, bindings, settings) | partial | Saved servers S:270 [B]; local identity S:384 [D]; browser storage not durable S:384. | engine (client shell) | none | M8 | | NOT TRACED | One store; identity export covers keys only. |

## Conflicts with the goal (Decided and Baseline items; not reopened)

- D41 (S:64) [D], S:253 [D]: no standalone solo mode and no local saves. Offline skirmish and an offline campaign
    (all six games) cannot exist. Every bot match needs a server and costs hosting.
- S:259 [B]: one frozen lockfile per server lifetime. A room creator cannot bring a map that is not in that set
    (SC1, WC3 custom maps). Baseline, so it can be refined; needs decision.
- D50 (S:73) [D] with S:386: no central account. A global ladder (A-MF13, OPTIONAL) has no unique-player basis.
    Tension, not a block.
- D25, D28, S:332: no conflict. A mod can give a disconnected ally's assets to a teammate (S:786); the engine never
    reassigns the slot.

## Lens answer

Spec 6.6 to 6.8 design ownership and admission in depth: atomic claims, terminal states, admin transfer, password,
start policy. They do not describe a lobby a player recognises. Start is designed (S:309). Team and map appear only
as examples in a room-level schema (S:290). Ready is a suggestion (S:311). Colour, faction pick, handicap: 0 hits.
No design says who edits a per-slot value or how a player changes team before start (S:326 defers it). The match end
is absent: the first-mod rule exists (S:748), but no engine outcome state, no match-end transition, no defeat screen,
no score, no results record. S:272 names "results" once.

Engine: lifecycle states, per-slot outcome, per-slot settings and authority, pause and speed of the tick, team
relation, stat counters, results store, save/restart, identity. Mod: victory rule, which settings exist, team and
colour choices, start assets, what concede does, all screens (S:164).

Milestone changes. M8: main menu, server browser, profile store, key-binding store. M9: per-slot settings, roster,
teams, colours, map pick, load barrier, pause, speed, lifecycle and outcome, results, score data, rematch, room
outside the simulation. M10: identity before reconnect, concede, disconnect hooks, saves and restart on SQLite.
M11: defeated players observe; replays later. Teams also block M5 (S:547). A playable match against a bot (S:694)
has no milestone and comes long after the CEO needs one.

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| `rtk proxy grep -c -i -E` over S for 25 terms | Yardstick: S has no surrender, score, menu, chat, handicap, keybind, match end | Any player-flow hit | 0 hits each; score/stats 1 hit S:751 (balance data); colour 1 hit S:531 (overlay) |
| `rtk proxy grep -n -i -E "lifecycle|hook"` S | S:163 engine supplies match lifecycle hooks | A hook or state list | Table cells only: S:123, 155, 163, 265, 272, 309, 374, 684 |
| Read `crates/turts_sim/src/slots.rs` | S:265 Room owns slots | Slots outside the simulation | `Slots` is a simulation `Resource` (lines 22-25) |
| `rtk proxy grep` `claim` in `turts_server/src` | S:253 reconnect restores the slot | A restore path | `session.rs:103-105` rejects a returning id |
| `rtk proxy grep` sqlite/ed25519/signature in all Cargo.toml | D40 SQLite and 6.13 identity exist | Any dependency | 0 hits |
| `rtk proxy grep` `SlotRelation::` in `turts_sim` | S:163 teams are an engine mechanism | A team relation | Slot equality only, `combat.rs:182-186` |
| `rtk proxy grep` colour in `turts_client/src` | TM03 player colour exists | Per-slot colour | One material, `render.rs:405` |
| `rtk proxy grep` tick/speed in `turts_server/src` | Per-room speed possible | Per-room tick setting | Server-wide `tick_hz`, `config.rs:72` |
| `rtk proxy grep` victory/team in M3 design | M3 left match end out on purpose | No deferral | Deferred, m3 design:55 |
| `python3 -I` count of verdict column | Line-1 counts match the table | Mismatch | 51 rows: partial 29, missing 14, present 5, conflict 2, wrong-milestone 1; matches line 1 |

## NOT TRACED

1. Client main menu and server browser absence beyond `--help` options.
2. Whether M1 design text covers lobby or room behaviour (not read).
3. Yardstick per-game facts (inventory authors' claims).
4. Lightyear support for session replacement and reconnect.
5. Mod-state serialization effort (A-SV05); owner is the mod-runtime seat.
6. Pause behaviour in the current server tick loop (A-UI18).
