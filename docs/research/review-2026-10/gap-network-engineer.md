VERDICT=fail ITEMS=87 MISSING=36 PARTIAL=45 CONFLICT=0 RUNS=9 NOT_TRACED=5

Seat: network-engineer. Mode: review (gap audit). Areas: 4.4, 6.2, 6.3, 6.9, 6.10, 6.11, 13.7.
S = `RTS-Engine-Project-Specifications.md` v0.7. Code paths are relative to the repository root.
Term sweep output: `/tmp/claude-1000/-home-julien-Projects-TuRTS/0e0457bb-045c-4b6a-945f-1db6a0753ee2/scratchpad/review/net/terms.out`.
"Code-own" in a row means the code finding is mine (network-engineer-N rows). "M4b" means a control slice before M5; the orchestrator names it.

| id | item | verdict | evidence (S:line or terms searched) | mechanism/policy | milestone now | milestone proposed | depends on | M1-M3 change? | note |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| A-OR01 | Move | present | S:747 (16.2 Decided, mod); `messages.rs:55-58` | M | M1 | M1 | none | no | Built. |
| A-OR02 | Stop | missing | "stop" 0 hits; no variant in `messages.rs:54-78` | M | none | M4b | A-OR11 | yes: new order verb | Needs an order queue to clear. |
| A-OR03 | Attack target | partial | S:747 "attack" (16.2, mod policy); S:159; no wire verb | B | none | M4b | combat target query | yes: new verb | M3 turret is auto-only (`WeaponRecord.last_target`, `messages.rs:253`). |
| A-OR04 | Attack-move | partial | S:747 only | M | none | M4b | A-OR03 | yes | Not in any milestone row S:676-688. |
| A-OR05 | Force move | missing | "force", "attack move" searched | M | none | M4b | order modifiers | yes | Order modifier, not a new verb. |
| A-OR06 | Force attack | missing | "force", "attack ground" searched | B | none | M7 | A-OR03 | yes | Target union must hold a point. |
| A-OR07 | Patrol | missing | "patrol" 0 hits | M | none | M7 | A-OR11 | yes | |
| A-OR08 | Hold position | missing | "hold" hits S:197, 209, 382 are noise | M | none | M4b | A-OR11 | yes | |
| A-OR09 | Stances | missing | "stance" 0 hits | B | none | M7 | combat | yes | Owner-private stance record on the wire. |
| A-OR10 | Guard and follow | missing | "guard", "follow" searched | M | none | M7 | target = unit | yes | |
| A-OR11 | Order queue per unit | partial | S:171 "orders" in server representation; S:342 "persistent intentions" (Proposed) | M | none | M4b | none | yes: `MoveTarget(Vec2)` is one target, `crates/turts_sim/src/world.rs:31-33` | Sim seat owns the queue; wire must carry it. |
| A-OR12 | Shift-queued orders | missing | "shift", "waypoint", "queue" (S:342 is commands per tick) | M | none | M4b | A-OR11 | yes: no queue-mode field in `PlayerCommand`, `messages.rs:49-52` | Add replace/append mode to the envelope. |
| A-OR13 | Waypoint display | missing | "waypoint" 0 hits | M | none | M4b | A-OR11, network-engineer-14 | yes | Needs owner-private order-queue replication. |
| A-OR14 | Edit queued orders | missing | "waypoint" 0 | M | none | later | A-OR13 | no | OPTIONAL. |
| A-OR15 | Rally point | missing | "rally" 0 hits (S:366 "literally") | B | none | M4b | production output | yes: no verb, no record | Server state, owner-private; not selection state. |
| A-OR16 | Build order | partial | S:745, S:158 | B | none | M5-M7 | construction | yes | M4 draft excludes player construction (M4 design:22). |
| A-OR17 | Gather order | present | S:744; `messages.rs:59-62` | B | M2 | M2 | none | yes for groups: single `unit_id` | See network-engineer-6. |
| A-OR18 | Repair order | partial | S:742 (16.2) | B | none | M7 | construction | yes | |
| A-OR19 | Load and unload | missing | "garrison", "load", "unit transport" searched | M | none | later | target = unit | yes | |
| A-OR20 | Ability order | partial | S:160 (4.3 Decided); S:358 envelope (Baseline) | B | none | M4b | network-engineer-4 | yes | Mod abilities need a generic envelope. |
| A-OR21 | Autocast toggle | missing | "autocast" 0 | B | none | later | A-OR20 | no | |
| A-OR22 | Order validation feedback | partial | S:782 bots only (Proposed); `messages.rs:92-122` closed enum | B | M1 | M4b | network-engineer-5 | yes | Mod reasons cannot be expressed. |
| A-OR23 | Order acknowledgement | partial | S:243 "immediate input acknowledgement" (Decided para, Baseline item); `crates/turts_server/src/session.rs:254-259` | B | M1 | M4b | none | no for wire | Server ack is one round trip. Local marker is client-only and unspecified. |
| A-OR24 | Group order fan-out | partial | S:342 "Support group orders" (Proposed) | M | M1 | M4 | none | yes: all-or-nothing, `crates/turts_sim/src/command.rs:278-283` | See network-engineer-6. |
| A-OR27 | Deploy and transform | partial | S:160 "transformations" (mod column) | B | none | later | A-OR20 | no | |
| A-OR30 | Repeat queue | missing | "repeat" searched | M | none | later | A-OR11 | no | |
| A-OR31 | Command rate limit | partial | S:342 budgets (Proposed); S:551 invariant 13 | M | none | M7 | none | yes: only 256 units per command, `command.rs:19` | See network-engineer-12. |
| A-PC17 | Ownership gate | present | S:305; `command.rs:278-280` | M | M1 | M1 | none | no | |
| A-PC18 | Shared control | partial | S:786 temporary control of a disconnected player is mod policy | B | none | M9 | network-engineer-8 | yes: owner == slot is hard-coded, `command.rs:278` | Authorization must become a policy query. |
| A-PC19 | Selection hygiene | missing | "selection" hits are library selection | M | none | M4 | network-engineer-2 | no | Client-only, but needs death vs left-vision on the wire. |
| A-FV01 | Black shroud | missing | "shroud", "explored" 0 hits | M | none | M5 | network-engineer-10 | yes | |
| A-FV02 | Fog of war | partial | S:162 (Decided), S:634 (Baseline) | M | M5 (M1 design:46) | M5 | A-FV03 | yes: no filter, network-engineer-1 | S:680 M5 row names deposits only, not unit fog. |
| A-FV03 | Sight radius | partial | S:162 "Sight ranges" (mod) | B | none | M5 | mod schema | yes: no sight field in the package schema | NOT TRACED in `turts_mod_api`. |
| A-FV04 | Server-side visibility | partial | S:634, S:368, S:496 | M | M5 | M5 | A-FV03 | yes: `crates/turts_server/src/export.rs:28,104-118` send all units | Design is Baseline; code is open-map. |
| A-FV05 | Shared vision | partial | S:162 (mod) | B | none | M5 | network-engineer-8 | yes | No team on the wire. |
| A-FV06 | Detection | partial | S:162, S:618 | B | none | M7 | A-FV04 | yes | Visibility must be per-entity, not per-cell only. |
| A-FV07 | Reveal | partial | S:35 (D12, deposits) | B | none | M5 | A-FV04 | yes | Units not named. |
| A-FV08 | Radar gating | missing | "radar" S:618 is a Factorio reference | B | none | later | minimap | no | |
| A-FV09 | Terrain occlusion | partial | S:461 (9.4), S:618 | M | none | M5 | M4 heightfield | yes | |
| A-FV10 | Last-seen structures | partial | S:780 bot memory only (Baseline recommendation) | M | none | M5 | network-engineer-2 | yes: no last-seen record | Server-held memory per player; ghosts are not live units. |
| A-FV11 | Fog lobby options | partial | S:290 settings schema | P | none | M9 | A-FV04 | no | |
| A-FV12 | Shroud generator | missing | "shroud" 0 | B | none | later | A-FV01 | no | |
| A-FV13 | Air vs ground sight | missing | S:114 flying needs own design | B | none | later | A-FV09 | no | |
| A-RO01 | Replay recording | partial | S:253 optional, secondary; S:364 retention open; D41 | M | none | after M11 | spectator feed | no | No lockstep (S:243), so a replay is a state recording. |
| A-RO02 | Replay controls | partial | S:364 "seek ... remain open" | M | none | after M11 | A-RO01 | no | Checkpoints give seek. |
| A-RO03 | Live observers with delay | present | S:362-368 (Baseline); D30, D31 | M | M11 | M11 | A-FV04 | no | No spectator session type in code (grep: 0 files). |
| A-RO04 | Observer UI | missing | "observer" 0 hits | B | none | after M11 | A-RO03 | no | |
| A-RO05 | Replay version safety | partial | S:253 exact versions for saves only | M | none | after M11 | A-RO01 | no | |
| A-MF09 | Defeated player observes | missing | "defeat", "observer" 0; S:328 viewers are non-players | B | none | M11 | A-RO03 | no | A slot session must switch to a viewer projection. |
| A-MF14 | Disconnect handling | partial | S:298-303, S:330, S:786 | B | M10 | M10 | none | no | Mod policy; engine hooks. |
| A-MF15 | Reconnect | wrong-milestone | S:253, S:328 (Decided); M10 S:685 | M | M10 | before M7 | identity D50 | yes: returning player refused, `session.rs:32,103-104`; `crates/turts_sim/src/slots.rs:46-48` | Every multi-client test after M5 needs it. |
| A-TM02 | Alliance effects | partial | S:163 (mod), S:251 team scope | B | none | M5 | network-engineer-8 | yes | |
| A-PE01 | Order responsiveness | partial | S:243 | M | M1 | M4b | A-OR23 | no | No latency target (budgets open, S:704). |
| A-PE07 | Latency hiding | partial | S:243, S:636 no whole-world prediction | M | M1 | M4b | none | no | Interpolation exists (`crates/turts_client/src/interp.rs`). |
| A-IN09 | Text input mode | missing | "chat" 0 hits | M | none | M9 | B-69 | no | Client-only. |
| B-6 | Group order | partial | S:342 (Proposed); `messages.rs:55-58` | M | M4 (K1) | M4 | none | yes: network-engineer-6 | |
| B-8 | Attack-move | partial | S:747 | M | none | M4b | A-OR03 | yes | |
| B-9 | Stop and hold | missing | "stop" 0, "hold" noise | M | none | M4b | A-OR11 | yes | |
| B-10 | Shift order queue | missing | "shift", "waypoint" 0 | M | none | M4b | A-OR11 | yes | |
| B-17 | Refused-order feedback | partial | S:782 bots; S:243 | B | M1 | M4b | network-engineer-5 | yes | |
| B-20 | Rally point | missing | "rally" 0 | B | none | M4b | production | yes | |
| B-26 | Fog of war | partial | S:162, S:634 | M | M5 | M5 | A-FV04 | yes | |
| B-27 | Last-seen snapshot | partial | S:780 bots | M | none | M5 | A-FV10 | yes | |
| B-29 | Alerts | partial | S:338, S:634 name the data | B | none | M7 | network-engineer-9 | yes | No event message class. |
| B-30 | Voice on order | missing | "sound", "voice" 0 | B | none | presentation | A-OR23 | no | Client keys it off local input, not the ack. |
| B-34 | Idle-worker button | missing | "idle" S:618, S:655 are factory sleep | B | none | M4b | network-engineer-16 | yes | |
| B-37 | Cancel production with refund | present | S:406, S:707; `messages.rs:67-70` | B | M2 | M2 | none | no | |
| B-38 | Pause, menu, surrender | missing | "surrender" 0; "pause" S:303 is server policy | B | none | M9 | match lifecycle | yes | No leave or surrender message. |
| B-46 | Radar blips | missing | "blip" 0 | B | none | later | A-FV04 | yes | Needs a "known, not identified" record. |
| B-47 | Line of sight by height | partial | S:461 | M | none | M5 | M4 heightfield | yes | |
| B-69 | Ping, chat, team view | missing | "chat", "ping" 0 | B | none | M9 | network-engineer-8 | yes | Reliable, rate-limited, scoped message. |
| B-71 | Replays and observers | partial | S:253, S:362 | M | M11 | M11 + after | A-RO01, A-RO03 | no | |
| B-76 | Pause | missing | "pause" S:303 only | B | none | M9 | lifecycle | yes | |
| network-engineer-1 | Unit interest filter by visibility | partial (code contradicts S:634) | `export.rs:28` `units: view.units.iter().map(unit_snapshot)`; `export.rs:104-118` spawned, removed, positions unfiltered | M | M5 | M5 | A-FV03 | yes: `export.rs` unit, node, health paths | Every client gets every unit: a map hack. Owner filter covers economy only. |
| network-engineer-2 | Death vs left-vision vs entered-vision | missing | `WorldDelta.removed: Vec<UnitId>`, `messages.rs:284`; `export.rs:105-110` | M | none | M5 | network-engineer-1 | yes: protocol | One "removed" list cannot drive death effects, ghosts, and selection hygiene. |
| network-engineer-3 | Health and node state public to all slots | partial | M3 design:472 marks public health a proposal; `export.rs:94,174` health, `export.rs:29,119` nodes | M | M3 | M5 | network-engineer-1 | yes | Leaks hidden enemy health and node depletion. |
| network-engineer-4 | Closed command enum vs mod order envelope | partial (code contradicts S:358) | `messages.rs:54-78` six engine variants; S:358 (Baseline) | M | M1-M3 | M4 | none | yes: M4 adds `Excavate` as a seventh closed variant | Need order type = content id, target union, queue mode, bounded args. |
| network-engineer-5 | Closed reject reasons | partial | `messages.rs:92-122`; S:358 | B | M1-M3 | M4b | network-engineer-4 | yes | Mod rules ("no power") need a content-id reason. |
| network-engineer-6 | Group order is all-or-nothing | partial (code contradicts A-OR24) | `command.rs:278-283` rejects the whole `MoveTo` on one `NotOwner` or `NotMobile`; Gather, Produce, Remove, SetEnabled take one id, `messages.rs:59-77` | M | M1 | M4 | none | yes | M4 K1 sends all box-selected ids (M4 design:264-266). One turret in the box rejects the order. |
| network-engineer-7 | Per-unit outcome of a group order | missing | `CommandApplied { seq, tick, job_id }`, `messages.rs:85-89` | M | none | M4 | network-engineer-6 | yes | Client cannot tell which units took the order. |
| network-engineer-8 | Team and alliance on the wire | missing | `Welcome { slot_id, tick, tick_hz }`, `messages.rs:26-30`; grep "team_id\|alliance": 0 files | B | none | M5 | match rules | yes | Shared vision, allied chat, shared control need it. |
| network-engineer-9 | Transient event class | missing | No event message in `messages.rs`; S:251 mod events, S:364 presentation events, S:338 alerts | B | none | M7 | network-engineer-1 | yes | Alerts, shots, deaths, construction done; visibility-filtered. |
| network-engineer-10 | Explored and shroud data | missing | "explored", "shroud" 0; no record in `messages.rs` | M | none | M5 | M4 chunk format | yes | Per-player, chunked, versioned like terrain (S:340). |
| network-engineer-11 | Per-recipient delta build | partial | `crates/turts_server/src/outbound.rs:108-120` builds and serializes one delta per session | M | M1 | M7 | network-engineer-1 | yes | Public parts repeat per client; group by interest set before M7 and M11. |
| network-engineer-12 | Per-slot command budget | missing | grep "budget\|limit\|flood" in session.rs, command.rs, plugin.rs: 0 relevant hits | M | none | M7 | none | yes | S:342 is Proposed; AI APM (D29) makes it CORE. |
| network-engineer-13 | Identity and session replacement | partial | `Hello.player_id: u64` self-declared, `messages.rs:21`; D50, D52 Decided | M | M10 | before M7 | D50 | yes | No proof of key; no explicit replacement (D52). |
| network-engineer-14 | General order-state replication | partial | `OrderState` is gather-only, `economy.rs:52-58` | M | M2 | M4b | A-OR11 | yes | Owner-private queue record for waypoints and intent. |
| network-engineer-15 | Position encoding and priority | partial | S:634, S:638 (Baseline); `Pos { f32, f32 }`, `messages.rs:15-18`; 9,858.2 B/tick at 1,000 moving units, `docs/measurements/m1.md:136` | M | M1 | M7 | none | yes | About 9.86 B per unit per tick, no quantization, no priority. |
| network-engineer-16 | Idle state for every owned unit | missing | Only `GatherStage::Idle`, `economy.rs:96` | B | none | M4b | A-OR11 | yes | Idle-worker and idle-army keys need it; selection does not. |

## Conflicts with the goal

- No Decided item in section 2 blocks the goal.
- Tension, not conflict: D41 makes replays "secondary". Replays are COMMON (A-RO01). The status can delay them but does not forbid them.
- Consequence of D18 and S:243 (no lockstep): a replay cannot be a command log replayed on clients. It must be a state recording (checkpoints plus sequenced changes, S:364). This is compatible. It costs storage. A command log replayed on the same server build needs repeatable server ticks (S:239). That is a later decision.
- D28 and S:332 forbid engine disconnect rules and automatic AI takeover. A-MF14 stays mod policy. Compatible.
- Code contradicts a Baseline statement (verdict `partial`, not `conflict`; no Decided item blocks): network-engineer-1 (S:634), network-engineer-4 (S:358), network-engineer-6 (A-OR24 versus `command.rs:278-283`).

## Lens answer

Base to extend, with two parts redesigned.

Keep (base):
- Envelope framing: `PlayerCommand { seq, kind }`, `CommandAck`, `CommandApplied`, `CommandRejected` (`messages.rs:49-52,80-89,190-194`). Server acks on receipt (`session.rs:254-259`). This gives immediate acknowledgement and lets the client hold pending orders by `seq`.
- Channels: reliable ordered for orders, unreliable for state, with a `base_tick` chain and `ResyncRequest` (`crates/turts_net_lightyear/src/channels.rs`; `messages.rs:281-296`).
- The projection seam: `snapshot_for(view, slot)` and `delta_for(batch, view, slot)` (`export.rs:25,100`) already run once per recipient before any wire value.
- No Bevy types: `turts_net_protocol` depends only on `turts_mod_api` and `serde` (`crates/turts_net_protocol/Cargo.toml`). Variants append without tag change (test `messages.rs:327-336`, run passed).

Redesign:
1. Command kinds. Six closed engine variants (`messages.rs:54-78`) cannot carry mod abilities (S:358) or the RTS order union. Replace with one order envelope: order type as a content id, a target union (none, point, unit, node, area), a queue mode (replace, append), a unit list, and bounded mod arguments. Results need a per-unit outcome (network-engineer-6, -7). Do this at M4, because K4 already bumps protocol 3 to 4 and M4 adds `Excavate` as another closed variant.
2. Projection input. The projection key is `SlotId` and it filters economy only. Units, positions, nodes, and health go to all (`export.rs:28-29,94,104-119,174`). Fog needs a viewer-knowledge key (slot, team, viewer policy), enter and leave semantics distinct from death (network-engineer-2), last-seen records, and explored data. This rewrites the unit, node, and health parts of `export.rs` and changes `WorldDelta`.

Selection: client-only. It must stay off the wire. The server needs facts that selection-driven UI reads: an idle state per unit, rally-point records, order-queue records, alerts as events, and an authorization rule for shared control. These are server state, not selection.

M1 to M3 code that must change: `export.rs` (unit, node, health filters); `messages.rs` (command envelope, reject reasons, removed semantics, team in `Welcome`, event class); `command.rs:278-283` (per-unit fan-out); `world.rs:31-33` (one `MoveTarget` cannot hold a queue); `session.rs:32,103-104` and `slots.rs:46-48` (reconnect is refused).

## What I ran

| command | claim tested | result that would refute | observed |
| --- | --- | --- | --- |
| `python3 -I net/terms.py S` (52 terms) | Yardstick B "zero hits" list; my missing verdicts | Hits for stop, shift, waypoint, rally, shroud, chat, observer | All 0. "attack-move" 1 (S:747). "fog" 2 (S:162, S:634). |
| `rtk proxy grep -n -E "project\|owner_slot\|visib\|fog\|interest" crates/turts_server/src` | Brief: "per-slot owner-private projection" | A visibility filter on units | Only `owner_slot` economy filters; `export.rs:28` unfiltered. |
| `rtk proxy grep -rn CommandAck crates` | S:243 immediate acknowledgement is built | No sender on the server | Sent at `session.rs:242-259`; client receives `ingest.rs:868`. |
| `rtk proxy grep -rln -i "spectat\|chat\|team_id\|alliance\|visible_to\|fog" crates` | A spectator, chat, team, or fog path exists in code | Any file | 0 files. |
| `rtk proxy grep -n -i "budget\|limit\|flood\|per tick" session.rs command.rs plugin.rs` | A per-slot command budget exists (A-OR31) | A rate cap | Only test `queue_capacity` and `command.rs:270`. |
| `cargo test -p turts_net_protocol` | Variants append without tag change (`messages.rs:327-336`) | A failing test | 4 suites, 6 tests, all pass. |
| `rtk proxy grep` over M1-M4 designs for fog, reconnect, selection | Fog is placed at M5; reconnect at M10 | Other placement | M1 design:45-46; M4 design:22, 27, 264-266. |
| `rtk proxy grep "bytes\|per tick\|1000" docs/measurements/m1.md` | Cost of unfiltered positions | No figure | 10,396.5 B/tick wire, 9,858.2 payload, 1,000 units, 20 Hz (m1.md:135-136). |
| `sed` of `slots.rs`, `session.rs:85-130` | Reconnect restores the slot (S:328) | A path for a known `player_id` | `claim` returns `AlreadyClaimed`; reason "player already in match". |

## NOT TRACED

1. Whether `turts_mod_api` has any sight or vision field (A-FV03). Not searched.
2. Whether M4 drag-select can include non-mobile owned units in practice (network-engineer-6). It depends on the M4 hit filter, which is not built.
3. Lightyear time-sync or RTT facilities for order-latency display. Not read.
4. Browser support of WebTransport or WebRTC. Not fetched; I make no claim on it.
5. Per-client bytes with a visibility filter at the D57 medium workload. No probe; no filter exists to measure.
