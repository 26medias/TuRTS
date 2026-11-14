# Browser download compression

Status: implementation in progress.

## Player result

The browser downloads fewer bytes before the same game starts.
The decompressed WASM and JavaScript stay byte-identical to the tested minimap build.
Graphics, simulation, sound defaults, and match behavior do not change.

## Delivery contract

Build gzip sidecars for WASM, JavaScript, and CSS. Use deterministic gzip headers and compression level 6.
Keep the original files. Do not retain a sidecar that is larger than its source.
The prepare step records the compression script as an input and hashes the resulting files.

Both the native HTTP server and the local New skirmish host serve gzip when accepted and available.
Keep the original MIME type. Set Content-Encoding to gzip and Content-Length to the encoded byte count.
Vary static responses by Accept-Encoding. A request that excludes gzip gets the original bytes.
The local host also honors an explicit preference for identity and rejects a request that excludes all available encodings.
A missing or empty Accept-Encoding gets identity. Wildcard accepts gzip unless gzip has an explicit zero quality.
GET, HEAD, conditional requests, and query strings continue to work.
Restart routes retain their existing origin and token checks. Package archives and game wire messages are unchanged.

Negotiation follows [RFC 9110 section 12.5.3](https://www.rfc-editor.org/rfc/rfc9110.html#section-12.5.3).
The native server uses the pinned tower-http implementation. The local host tests its own negotiation.

## Acceptance

| Row | Acceptance |
| --- | --- |
| ZIP1 | Sidecars decompress to the original files. Repeated compression is deterministic. Incompressible input keeps identity only. |
| ZIP2 | Real local-host GET and HEAD negotiate gzip, fallback, q=0, wildcard, explicit identity, MIME, length, Vary, and conditional responses. Existing retry and rejection tests pass. |
| ZIP3 | Real native-server requests receive gzip and identity correctly. Decompressed WASM hashes equal the source. |
| ZIP4 | A fresh browser launched through the real launcher receives gzip and reaches ready models, map controls, and sound off. Record raw and transferred WASM bytes. |
| ZIP5 | Both cold first-click probes and the buttons probe pass on the final build. |

Run one independent code-running review after the build. Fix its findings before delivery.
Keep the minimap launcher until the final probes pass. Do not merge or push main.
