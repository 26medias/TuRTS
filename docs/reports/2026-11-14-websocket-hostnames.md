# WebSocket hostname delivery

Status: DNS1–DNS3 pass. Foundation review complete. Its one finding is fixed.
Client: `2df27a06`. Combined hosting code: `b047ccf0`. Main is unchanged.

| Row | Result | Evidence under `target/mc-evidence/websocket-hostnames/` |
| --- | --- | --- |
| DNS1 | PASS. Browser token parsing, native endpoints, invalid inputs, and transport equivalence. | `native-final.log`; `review/edge-green.log`; `clippy-final.log` |
| DNS2 | PASS. Real WASM joins `WS://localhost` and finishes an authoritative move. | `browser-uppercase-final/result.json`; `browser-uppercase-final/arrival.png` |
| DNS3 | PASS. Both first-click probes and the buttons probe pass on the final client. | `cold-final-1280/`; `cold-final-1024/`; `cold-final-buttons/` |

The final full native client suite passes 346 tests with one existing ignored test.
Evidence: `client-native-final.log`.
The ingestion suite passes 23 tests. The final targeted parser tests and strict Clippy also pass.
The independent reviewer ran code and found an uppercase-scheme mismatch.
The exact failing regression is retained in the product tests. It now passes.
The old WASM fails the lowercase hostname check. The first implementation also fails the reviewer's uppercase browser check.
Both failures remain in the evidence folder.

The browser resolves DNS through its WebSocket URL. Its token placeholder does not replace that URL.
The server's address-check setting, keys, wire records, player identity, and slot authority are unchanged.
A real public HTTPS and WSS check also passes. See the shared host report.

Final cold receipt: `target/mc-evidence/hostname-shared-final-cold-checks-v2/receipt.json`.
Command and wrapper exit 0. Source stayed unchanged.
The earlier wrapper without `v2` failed before starting a browser because its temporary command named a nonexistent script.
No other-hardware or new 400-unit FPS test was done. Rendering code and content are unchanged.
