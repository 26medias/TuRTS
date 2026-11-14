# Fast rejoin after a page reload

A page reload currently waits about ten seconds for its old session to expire.
The public HTTPS/WSS probe recorded eight connection attempts for each reload.
The old WebSocket had closed, but its application Session still blocked the same player.

When Lightyear marks a connection Disconnected, retire its application Session and pending snapshot.
Drop only that connection's received commands that have not reached fixed-tick admission.
Keep permanent slot ownership, units, accepted gameplay orders, and the slot's rate-limit bucket.
Do not shorten transport timeouts or allow a new Hello to take over a live connection.
Ignore Hello, command, and resync traffic from disconnected connections.
Do not change the wire schema, token keys, or mod API.

| Row | Acceptance |
| --- | --- |
| RJ1 | A closed transport loses its application session before timeout. The same player immediately recovers the same slot. |
| RJ2 | Another live window with that player identity is still rejected. Other players and their queued commands are unchanged. |
| RJ3 | Rejoin retains the admission bucket and drops only unadmitted commands from the closed connection. |
| RJ4 | The real public HTTPS/WSS reload succeeds without the previous repeated duplicate-window refusals. Both clients can still issue orders. |
| RJ5 | Final scripted cold player probes pass. |

Run one independent review that executes code after implementation. Fix its findings.
