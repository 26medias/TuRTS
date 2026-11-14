# Independent review: browser HTTP compression

Reviewed implementation: `9b889655`. Verified fixes: `113e86f0`. Baseline: `6b6460d9`.
Worktree: `/home/julien/Projects/TuRTS-compression-review`.

Verdict: PASS for the HTTP delivery change. Both findings are fixed and independently verified. No blocking finding remains.
No browser or compiler was started by the reviewer.

## Findings and verified fixes

1. **Static errors can claim gzip for plain HTML.**
   `scripts/skirmish_host.py:150-166` sets the encoding before opening the selected sidecar.
   If that file is unreadable, the base handler returns 404 HTML. The new header hook labels that HTML as gzip.
   A real HTTP test reproduces this with `app.wasm.gz` set to mode 000 under UID 1000.
   Clear the selected encoding for error responses, while retaining it for successful files and valid 304 responses.
   Fixed at `113e86f0`: the host clears the selected encoding before sending an error.
   The real unreadable-file case now returns 404 HTML without Content-Encoding.
   Red evidence: `review-independent.log`. Green evidence: `review-independent-green.log`.
   Case: `test_unreadable_selected_sidecar_does_not_label_an_error_body_as_gzip`.

2. **The final launcher guard accepts zero models after a timeout.**
   `tests/browser/compression-launcher.js` waits for a nonempty model array, but its final guard only calls `every()`.
   An empty model array passes that predicate. Add a nonzero model count to the final rejection condition.
   Fixed at `113e86f0`: the final guard now requires a nonzero model count.
   The actual source predicate accepts a ready scene and rejects zero models, an unready model, no units, and no map.
   Red evidence: `review-model-guard.log`. Green evidence: `review-model-guard-green.log`.

## Executed checks

Evidence paths are relative to `target/mc-evidence/compression/`.

| Check | Result | Evidence |
| --- | --- | --- |
| Authored compression and local HTTP tests | Three tests pass, including deterministic sidecars, decoded bytes, negotiation, HEAD, MIME, Vary, query, and 304. | `review-local.log` |
| Existing restart security and process cleanup | Pass. Wrong origin/token are refused. A valid restart replaces and reaps the server. | `review-retry.log` |
| Real native server against prepared WASM | Pass. Gzip GET/HEAD and raw identity/q=0 responses preserve the decoded program hash. | `review-native.log`; `review-native/result.json` |
| Actual prepared artifact integrity | Every original dist file and the package archive match the minimap build. Both sidecars decode exactly. | `review-artifacts.json` |
| Independent local HTTP edges | All five tests pass after the fix. Includes escaped file names/query, repeated Accept-Encoding headers, explicit gzip q=0, invalid quality values, conditional requests, and an unreadable sidecar. | `review-independent-green.log` |
| Independent native HTTP edges | Seven cases pass: absent/empty encoding, gzip, Chrome’s usual gzip/deflate/br/zstd header, explicit q=0 over wildcard, conditional HEAD, and unreadable sidecar. The native error has no false encoding. | `review-native-edges.json`; `review-native-edges.log` |

The prepared WASM is 59,686,193 raw bytes and 14,821,363 gzip bytes.
Its decoded SHA-256 is `042888f3f7db814f144758ff0ee2756a2d9d91562560137e901aaaf4db0eb939`.
JavaScript is 194,386 raw bytes and 29,467 gzip bytes.

The fix changes only the local-host error path, launcher test guard, and retained regression tests.
No Rust engine source, game content, or Cargo dependency changes follow the initial review.
The actual artifact checks above remain applicable.

## Source assessment

The native change enables the pinned tower-http static gzip support.
Advertisement, lockfile, package bytes, game wire messages, and simulation are unchanged.
The local host keeps the original MIME type and encoded length. It varies static responses by Accept-Encoding.
Its quality handling covers explicit identity, zero quality, wildcard exclusion, and unavailable sidecars.
The compressor preserves source files and removes a stale sidecar when compression has no benefit.
The prepare path records the new compressor and local host as inputs. It hashes sidecars as dist artifacts.
The restart origin and token checks remain unchanged.

## Limits

I did not run a browser, cold player probes, or the actual launcher. The author runs those checks.
I did not repeat the 37 prepare tests or compile server Clippy. The author records those checks separately.
This review does not claim a local startup-time speedup. It verifies fewer transferred bytes and identical decoded files.
