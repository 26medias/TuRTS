# Human lobby start policies

API 0.2.9 implements the package match start policies:

```ron
(start: AllSeatedReady(countdown_secs: 3), open_sites_per_slot: Some(4))
```

`Immediate` starts at boot. `AllSeated` waits for every human seat.
`AllSeatedReady` also waits for each human to press Ready.
Countdown is 0 to 30 seconds. Cancelling readiness, changing colour, or leaving cancels it.
A later valid start condition begins a full new countdown.
Newly implemented lobby policies require an engine API requirement that excludes versions below 0.2.9.
Existing Immediate packages retain their previous API requirements.
A lobby needs at least two distinct colours in the interface's `slot_colours` palette.
The server supports 2 to 255 palette entries. Colours stay unique among occupied seats.
A colour change clears that player's readiness.

The server stops simulation at tick zero while waiting. It claims permanent player slots only at start.
A disconnected lobby member releases their unclaimed seat.
After start, ownership stays permanent and a same-identity reload recovers the original slot and colour.
Pre-start gameplay orders get `MatchNotStarted`. They do not run after start.
Readiness and colours come from the sending connection. Lobby records contain no private player ID.

Host operators can set an optional `start_policy` in `server.ron` to override the package default:

```ron
start_policy: AllSeatedReady(countdown_secs: 3),
```

Human skirmish launchers use this override. The computer game keeps Immediate.
A computer player with a non-Immediate policy is refused in this delivery.
Lobby bots, surrender, disconnect defeat, and shared rematch remain separate work.

Protocol 6 activates the existing lobby handshake. Message layouts and registration order are unchanged.
Older clients are refused by protocol version. Each accepted start sends Welcome and a full Snapshot.
Welcome's colour indices map player slots to the interface palette for models, rings, and the minimap.
