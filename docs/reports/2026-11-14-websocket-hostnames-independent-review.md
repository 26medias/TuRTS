# Independent WebSocket hostname review

Reviewed commit: `7b9d194eab3d17bc17473b3efc6dad12d4e5eee3`.
Review checkout: `/home/julien/Projects/TuRTS-hostname-review`.
The author checkout was not changed.

## Finding

**P2: accepted uppercase schemes dial a different endpoint.**

`parse_websocket_url` accepts `WS://127.0.0.1:7001/path?room=two` as a valid URL.
The later `ws_url` helper recognizes only lowercase scheme prefixes.
It changes the transport address to `ws://WS://127.0.0.1:7001/path?room=two`.
The transport then dials host `ws`, not the validated numeric address.
`WSS://` has the same problem.

Location: `crates/turts_client/src/join.rs:239`, used by `spawn_client` at line 446.
Make scheme handling consistent between validation and transport construction.
Retain the independent regression when fixing it.

Evidence:

- `edge-red.log`: the independent endpoint-equivalence test fails, exit 101.
- `independent-test.patch`: the exact added regression. It modifies only this review checkout.
- `browser-uppercase/result.json`: the actual prepared WASM fails to join `WS://localhost:45451/`, exit 1.
- That browser used null sound and a fresh profile. The profile was removed.

## Executed checks

From the review checkout:

```sh
/tmp/codex-mp1-cargo test -p turts_client --lib websocket_address_tests -- --nocapture
/tmp/codex-mp1-cargo test -p turts_client --test ingest
/tmp/codex-mp1-cargo test -p turts_client --lib independent_hostname_review -- --nocapture
```

The first command passes 3 tests, exit 0. See `native.log`.
The second passes 23 tests, exit 0. See `ingest.log`.
The third fails the named endpoint mismatch, exit 101. See `edge-red.log`.

The browser command was:

```sh
env CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \
  xvfb-run -a -s '-screen 0 1280x720x24' \
  node /home/julien/Projects/TuRTS/target/mc-evidence/websocket-hostnames/review/uppercase-browser.js \
  /home/julien/Projects/TuRTS/target/mc-evidence/skirmish-hostname-prepared \
  /home/julien/Projects/TuRTS/target/mc-evidence/websocket-hostnames/review/browser-uppercase
```

The browser driver is the original hostname driver with `WS://` instead of `ws://`.
Its copied source is retained beside this report.

## Other review conclusions

The browser token placeholder is separate from the transport URL.
Pinned `lightyear_websocket` uses `WebSocketTarget::Url` directly when `from_url` is used.
The token address sets `PeerAddr`; it does not replace that URL.
The server already has `server_addr_check: false` before this change.
The diff changes no server authority, keys, wire records, identity, or slot rules.

IPv4, IPv6, default ports, explicit ports, invalid schemes, credentials, fragments, and zero ports have executed coverage.
The root's lowercase hostname browser evidence was inspected. It passes a real authoritative move to approximately `(18, 30)`.
This review did not independently run a public WSS or TLS connection.
Numeric cold probes are part of the delivery checks, not repeated in this review.

This is the single requested review. One finding needs a fix and execution of its regression.
No other findings were found.

## Finding fixed

Commit `2df27a06` makes validation and transport scheme handling consistent.
The retained independent regression passes. See `review/edge-green.log`.
The final real browser passes `WS://localhost` and completes its movement order.
See `browser-uppercase-final/result.json`. No new review pass was started.
