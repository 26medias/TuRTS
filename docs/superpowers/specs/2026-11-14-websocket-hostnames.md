# WebSocket hostnames

The browser must accept a WebSocket hostname, including a trusted HTTPS host's `wss` endpoint.
The existing numeric-only token parser blocks this before the browser can connect.

Parse the endpoint as a WebSocket URL. Keep the original URL for the transport.
Accept default ports 80 and 443, explicit ports, IPv4, IPv6, paths, and queries.
Reject invalid URLs, other schemes, credentials, fragments, and port zero before connecting.
Native clients continue to resolve hostnames for their netcode token.
Browser clients let the browser resolve DNS. Use a loopback placeholder only inside the netcode token for hostnames.
The placeholder does not change the URL that the browser dials.
The server already disables token address checks for different public and bind addresses.
Do not change that setting, token keys, wire schema, player identity, or slot authority.

| Row | Check |
| --- | --- |
| DNS1 | Native executable tests cover default ports, numeric addresses, IPv6, invalid inputs, and the browser token helper. |
| DNS2 | A real WASM client joins a local server through `localhost`, loads models, and completes an ordinary order. |
| DNS3 | Numeric-address cold probes still pass. |

Run one independent review that executes code after the build. Fix its findings.
This change alone does not establish public hosting, trusted TLS, or Internet play.
