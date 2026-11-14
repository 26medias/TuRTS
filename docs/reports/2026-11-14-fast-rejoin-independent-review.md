# Independent fast rejoin review

Verdict: PASS. No findings.
Reviewed commit: `e0de418b8a9862b94a283dbc6e1ac8f746d62027`.
Review checkout: `/home/julien/Projects/TuRTS-fast-rejoin-review`.
The author checkout was not changed.

## Executed checks

```sh
/tmp/codex-mp1-cargo test -p turts_server --bin turts_server independent_closed -- --nocapture
/tmp/codex-mp1-cargo test -p turts_server --bin turts_server
```

The independent checks pass 2 tests, exit 0. See `independent.log`.
The complete server suite passes 109 tests, exit 0. See `server-full.log`.
That total includes the 107 product tests and 2 independent tests.
The review uses the existing shared native cache. It creates no lane build cache.

The independent tests exercise two additional cases:

- A Hello travels through the real Lightyear message transport. The receiver closes after decoding and before `receive_hello`. It creates no session or slot claim.
- An accepted session closes while admitted order metadata and unit orders exist. Those orders, the unit, and permanent ownership remain intact.

The independent test patch is `independent-tests.patch`.
It changes only this detached review checkout.

## Authority and lifecycle conclusions

`Disconnected` comes from the transport lifecycle. A new Hello does not force another live connection to close.
The observer removes only that connection's application Session and pending snapshot.
It drops only unadmitted entries with that connection entity.
It does not change Slots, the admission bucket, pending admitted orders, or simulation entities.
The receive queries exclude disconnected connections.
The durable slot claim remains bound to the same player.
The executed product regression confirms immediate same-player rejoin and rejection of a real second live window.
It also confirms that another player's queued command survives and the admission bucket does not refill on rejoin.
No wire schema, token key, transport timeout, or mod API changed.

## Audit test correction

The positive audit case remains intact.
The old negative labels automatic work as Other, which this validator can permit.
The replacement uses Command without a command record. The validator explicitly rejects that state.
The full suite executes this corrected negative and passes.
The baseline failure log was inspected at `../audit-baseline.log`; this review did not rerun that baseline command.
No audit product code changed. This test correction does not weaken audit validation.

## Limits

This review did not launch a browser or repeat the public tunnel test.
The root agent owns the final HTTPS/WSS reload and cold player probes.
Those delivery checks remain required. No further independent review pass is needed unless implementation changes materially.
