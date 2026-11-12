# MP1 gate 1: network lens

Source: subagent hand-back, MP1 brainstorm input, 2026-11-11. Agent output, not reviewed; claims are leads until verified.

VERDICT=PASS-WITH-FINDINGS FINDINGS=12 HIGH=4 RUNS=9 NOT_CHECKED=5

The design is sound in direction: the server holds authority, effects are cosmetic, and every record goes through the per-slot export. But four wire and lifecycle facts are wrong or missing, and each costs a rewrite if it is found late. Probe output: /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/netprobe-output.txt (source: /tmp/claude-1000/-home-julien-Projects-TuRTS/bf418660-b272-4bb3-8628-8d9ef2677d1d/scratchpad/netprobe/src/main.rs).

## Findings

**High**

1. **The lobby conflicts with the slot claim and the handshake (design 6.7:180, 7.6:267; row LOB1:364 "a leave reopens the slot").**
    - Today the sim claims the slot at `Hello` (`session.rs:152`, `slots.rs:46-56`) and logs the claim (`session.rs:261`).
    - The client fails if `Welcome` does not come within 15 s (`join.rs:471`).
    - Spec S:350 allows lobby edits only before the match starts.
    - Text change: "The room holds lobby seats, and a player can release a seat before start. `Welcome` comes at room admission. Lobby records follow on the reliable channel. At start, the room gives the sim a `MatchConfig`; the sim claims the slots (D25) and logs the claims. A `MatchStart{start_tick, teams, colours}` record follows. The sim does not advance before `start_tick`. An order before it gets `match_not_started`, and an order from a defeated slot gets `slot_defeated`."
    - Add to LOB1: both clients get the same `start_tick`, and an order sent during the countdown is refused. Red build: the sim claims at `Hello`.

2. **The disconnect defeat is outside the command log (decision P8:44; S:267 and S:392, G9).**
    - Connectivity is a server fact. A server timer that defeats a slot breaks replay.
    - Text change: "The room writes `SlotConnection{slot, connected}` into the command log at the tick of the change. The sim counts the 120 s in ticks."
    - Add to MAT1: a replay gives the same defeat. Red build: a wall-clock timer.

3. **Step 19 of the Player path cannot pass (design :708-710: "B closes its window", then "B reloads").**
    - The browser keeps the player id in sessionStorage (`turts_client_web/src/lib.rs:239-251`).
    - MDN: "Closing the tab/window ends the session and clears the data in `sessionStorage`."
    - A new window gets a random id, and the server answers `NoOpenSlot`.
    - Needs decision:
        - (a) Step 19 says "B reloads the page" (same tab). Recommended.
        - (b) Keep the id in localStorage. Then a second tab becomes a D52 session replacement.

4. **"`unit_orders` as changes only" has a stale-order trap and no row (design 6.8:189).**
    - Today the delta carries the full owner order table every tick (`export.rs:190-195`).
    - The sim gives no record for a unit without `UnitOrders` (`views.rs:558-568`, `return None`). The client clears such a unit only because each delta replaces the whole table (`replica.rs:523-525`).
    - Measured: the full table is 2,181 to 4,281 B per tick, 64 to 83 % of the delta (44 to 86 KB/s per client at 20 Hz). This is an MP1 task, because the protocol step happens now.
    - Text change: "A delta carries the complete record of each unit whose record changed, plus `orders_cleared: Vec<UnitId>` for each record that left the view. The Snapshot keeps the full table. The replica merges in chain order only (`replica.rs:237-239`)."
    - New row ORDW1: a plain move ends on a unit with no `UnitOrders`; the delta that clears it is dropped (T8 hook), then a resync. The client must show no current order. Red build: no clear list.

**Medium**

5. **Refusals skip the token bucket.**
    - `production_precheck` and `BadSequence` refuse before `admit` (`session.rs:386-440`). Each refusal writes admission evidence and sends a reliable `OrderResult`, and costs no token.
    - Text change: "The bucket runs first for every envelope." No new bucket class is needed.
    - Placement checks are cheap: about 0.3 M circle checks per second at 640 placements/s (estimate, NOT measured).
    - The real grief is free sites: at progress 0 the charge is 0 (6.4:157). Needs decision: a cap on placed sites per slot, as mod data. The inputs disagree: GD:141 says 1; 7.3:252 and PRD1 say a queue of 5.

6. **No record has a visibility class (grep).** Add this table:

    | Record | Who gets it |
    | --- | --- |
    | Site existence and footprint | Public |
    | Site progress and charge | Owner |
    | Power | Owner |
    | Rally | Owner (order record, `export.rs:96-100`) |
    | Match state, outcomes, connection | Public |
    | Statistics | Public, after the end only |
    | Lobby | Room members |
    | Enemy health | Public in MP1 (`export.rs:94,180`) |

    - The filter goes in one place: a per-viewer visible-unit set in export, used for units, positions, healths, sites and effects.
    - `turts_rules::placement` takes the same visibility predicate, so a refusal reason never names a hidden blocker (6.3:152 names target queries only).
    - The wire `Fired` drops `damage`, `ammo` and `applied` (`events.rs:85-92`), and it leaves out any id that the viewer may not see.

7. **Effects can come in a burst after a lost delta.**
    - Effects ride the unreliable channel (`protocol_plugin.rs:20-24`). After a resync, up to 256 kept deltas (`replica.rs:60-64`) are applied in one commit (`replica.rs:150-190`).
    - Nothing depends on `Destroyed`: the death alert uses the removal and the health trace (`feedback.rs:639-651`).
    - Text change: "Effects are never replayed. The client drops an effect older than the presentation tick minus K."
    - Measured cost: `Fired` is 20 B with a weapon content id and 6 B with a weapon index. 60 units firing at 1 Hz cost 1,200 or 360 B/s, which is small. Use the index.

8. **Engine orders have no source.**
    - `apply_order` takes a slot and a seq (`orders.rs:2610-2614`), and results are routed by slot and seq (`outbound.rs:24-36`). A grep finds no `OrderSource` in the sim.
    - Text change: "An Engine order has no seq, costs no token and gives no `OrderResult`. A refusal shows in `last_failure`. The log records the source Engine."
    - Engine orders should skip admission and the bucket: they come from an already admitted `SetRally`.
    - The client needs no order id: the follow order shows in `unit_orders.current`.

9. **Row CON2 promises more than the client can know.**
    - The client sees positions that are 1 tick plus latency old (`m1.md:8`). It does not see a site that the other player placed in the same tick.
    - Text change: "For a scene with no motion and no new site between the client frame and the server tick, the client verdict from its replica equals the server verdict for 1,000 points. Red build: the client leaves out a blocker class, or uses another gap."
    - On a mismatch, the player sees: the pending site, then the refusal with its reason at the pointer, the error sound, and no credits change.

10. **Row RES1 cannot go red in pixels.** A client can hold the statistics and not show them. Add a wire check: no statistics in any message before the end tick.

**Low**

11. **No client prediction exists** (grep: no hits; progress comes from replicated values, `sources.rs:563`). Text change: "The replica is the only source of credits, progress and power (D18). A bar never goes past the last replicated value." The pending site is the one predicted object, and it lasts until the reliable `OrderResult`.

12. **The second and third matches of the Player path (:706-710).**
    - Slots and buckets live per process (`session.rs:48`), and `judge_hello` does not check whether the match is over (`session.rs:140-161`).
    - Text change: "Each match runs in a new server process. After the end, a claimed player gets `Welcome` plus the result; a new id gets `JoinRejected` 'The match is over'."

## What I ran

| Command | Claim tested | Result that would refute | Observed |
| --- | --- | --- | --- |
| netprobe (postcard), exit 0 | design 6.8:189, 191 | `unit_orders` under 20 % of the delta, or `Fired` above it | 64-83 % for `unit_orders`; `Fired` ≤1.4 % |
| grep design for reveal, hidden, visib, fog | fog readiness | a visibility class for each record | only lines :152, :183, :191 |
| read `views.rs:558-568` | `replica.rs:523-525` | every owned unit has a record | `return None` |
| read `session.rs:386-440` | TD:247 | the bucket comes before the precheck | the precheck comes first |
| WebFetch MDN sessionStorage | Player path :708 | the data survives a close | it is cleared |
| grep `OrderSource` and `Engine` in `turts_sim` | design 6.5:168 | a source field exists | none |
| grep predict and extrapol in the client | Q5 | a hit | none |
| read `judge_hello` | Q6 | a match-state check | none |
| `git status --porcelain` | the tree is clean | output | empty, exit 0 |

The MAT1 red is reachable only through the surrender and disconnect cases.

## Not checked

- No live two-client run (no client binary is built).
- The lightyear per-frame receive cap: NOT TRACED.
- Whether the client has the map bounds: NOT TRACED.
- The cost of the placement check: an estimate only, not measured.
- Spectators: not in MP1.

## What I changed

none

## Open questions for the orchestrator

- The CEO must decide on finding 3 (which storage keeps the player id) and finding 5 (the cap on placed sites).
