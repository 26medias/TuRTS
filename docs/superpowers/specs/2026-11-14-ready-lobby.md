# Ready lobby for human skirmishes

## Player result

Local human hosting and temporary Internet hosting open a two-seat lobby.
Each player sees who has joined, their colour, and both ready states.
Both players press Ready. A three-second countdown then starts one shared match.
A player can cancel readiness during the countdown. Leaving also cancels the countdown.
The computer game continues to start immediately.

## Foundation contract

Implement the existing Immediate, AllSeated, and AllSeatedReady start policies.
Without a MatchConfig, simulation tick stays zero and no economy or combat advances.
Before start, connections hold lobby seats. Permanent Slots claims happen once, at start, in seat order.
After start, claims remain permanent. Existing fast rejoin and duplicate-window rejection remain intact.
A disconnected lobby member releases only their unclaimed seat.
Ready and colour actions take their owner from the sending connection, never from a supplied player ID.
Reject stale revisions and invalid or occupied colours. A colour change clears that player's readiness.
Rate-limit lobby actions by real time because simulation time is stopped.
Reject pre-start gameplay commands with MatchNotStarted. They must never run after start.
Already accepted commands and admission budgets retain the existing reconnect rules.
Use server real time for countdowns. A cancelled countdown starts again from its full duration.

HostConfig gets an optional start_policy override. Otherwise use the package's match policy.
Human launchers set AllSeatedReady with three seconds. Computer mode keeps Immediate.
A computer player with a non-Immediate policy gets a clear startup error in this delivery.
Use existing LobbyState, LobbyAction, LobbyRefused, Welcome, and Snapshot layouts and registration order.
Advance protocol version to 6 because older clients do not implement the lobby handshake.
Release API 0.2.9 for newly supported package start policies. Older requirements cannot name these policies.
Keep rejected, unrelated mod capabilities rejected.
Colours in Welcome must apply to models, selection rings, and the minimap.

## Acceptance

| Row | Acceptance |
| --- | --- |
| LOB1 | One human waits in a visible lobby. Tick, economy, units, and health do not advance. Gameplay commands are rejected. |
| LOB2 | Two humans have distinct seats. Ready acts on its first click. Both see the same states and countdown. |
| LOB3 | Cancelling Ready or disconnecting during countdown prevents start. A replacement can take an unclaimed seat. |
| LOB4 | Stale actions, duplicate live identity, occupied colours, invalid colours, and action floods cannot bypass authority. |
| LOB5 | Start claims each seat once. Both clients receive one match ID and correct colours. Models, rings, and minimap agree. |
| LOB6 | An ordinary move works after start. Reload preserves the same slot, colour, army, and admitted order. |
| LOB7 | Immediate computer mode and AllSeated with zero countdown still work. API and protocol compatibility checks pass. |
| LOB8 | Real local and public two-browser checks pass. Final scripted cold probes pass. All owned test processes and profiles stop. |

Run one independent review that executes code after implementation. Fix its findings.
No main merge or push. Every test browser uses a fresh profile and --alsa-output-device=null.
Shared rematch, lobby bots, surrender, and disconnect defeat remain separate work.
