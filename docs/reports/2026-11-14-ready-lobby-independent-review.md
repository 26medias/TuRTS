# Ready lobby independent foundation review

Reviewed commit: `b76f4565`.
Review checkout: `/home/julien/Projects/TuRTS-ready-lobby-review` (detached).
Author checkout was not changed. No browser was started. No main merge or push was done.

Verdict: fix two findings before delivery. The review is complete. Root owns the fixes and final player checks.

## Findings

### R1: preserve Unready while Ready is in flight (high)

`crates/turts_client/src/lobby.rs:123-127` clears desired state when it matches the last server snapshot. That snapshot can precede an in-flight opposite action.

Reproduction: start with server ready=false. Send Ready. Before its echo, click Unready. The next send pass sees desired=false and server=false and deletes the cancellation. When the Ready echo arrives, the client sends no Unready. The match can start against the player's last choice. This violates the cancellation behavior in LOB3.

Independent executable regression: `lobby::tests::independent_review_unready_survives_inflight_ready_echo`. It uses the real send_intent system, a current RejoinController, and a typed Lightyear sender. It expects SetReady(false) on revision 2 after the pending Ready echo. It fails with last_sent=None. Exit 101. Evidence: `client-independent.log`.

Fix: track pending acknowledgements separately from the latest local choice. Do not treat a stale matching snapshot as acknowledgement of a newer choice. Retain a regression for this ordering.

### R2: reject invalid palette values before offering them (medium)

`crates/turts_mod_runtime/src/mp1_validation.rs:1320-1325` counts distinct strings without parsing them. `crates/turts_server/src/main.rs:111-123` also checks only count and string uniqueness. An API 0.2.9 Ready package with ["invalid-red", "invalid-blue"] passes content validation and passes these boot checks. Lobby swatches fall back to white (`lobby.rs:309-312`). Models also fall back to white (`render.rs:321-333`). The accepted package therefore cannot supply the distinct visible colours promised by docs/mod-api/lobby.md and LOB5.

Independent executable regression: `independent_review_ready_palette_rejects_invalid_colours`. It packs a real skirmish mod with the malformed palette, reparses its archive, and calls content(). It expects a validation error and fails. Exit 101. Evidence: `api-independent.log`. The two existing API floor tests in that same run pass.

Fix: validate slot_colours values with the existing colour validator. Ensure any distinctness check applies to the palette actually selected by the engine.

## Executed checks

All commands use `/tmp/codex-mp1-cargo` and the existing shared native target. The appended reviewer tests are in `independent-tests.patch`.

| Command | Result | Evidence |
| --- | --- | --- |
| test -p turts_client independent_review_unready -- --nocapture | Exit 101, R1 reproduced | client-independent.log |
| test -p turts_mod_runtime --test lobby -- --nocapture | Exit 101, two existing checks pass, R2 reproduced | api-independent.log |
| test -p turts_server independent_review_prestart -- --nocapture | Exit 0, one independent check passes | server-independent.log |
| test -p turts_server --bin turts_server independent_review_first_real_hello -- --nocapture | Exit 0, one independent check passes | first-hello.log |
| test -p turts_server --bin turts_server | Exit 0, 113 checks pass | server-suite.log |
| test -p turts_net_protocol | Exit 0, 14 checks pass | protocol-suite.log |
| test -p turts_client lobby::tests:: -- --skip independent_review | Exit 0, two checks pass | client-existing.log |

The pre-start check puts gameplay and the final Ready action in the same reliable packet. The server starts the match, but refuses that command with MatchNotStarted and leaves both staging and command queues empty.

The first-Hello check relays an actual typed Hello and captures the returned LobbyState after MessageSystems::Receive. The first member receives seat 0 and both seat records. Initial test instrumentation read the receiver after the frame ended; Lightyear had already cleared it. Moving capture into PreUpdate corrected that test. No product change was needed. An initial --lib server command was invalid because this package only has a binary target; the recorded suite uses --bin turts_server.

## Contract inspection and limits

Inspected tick-zero gating, pre-start admission order, transient lobby seats, durable post-start slots, reconnect handling, revision rejection, real-time budgets, countdown cancellation, single start, Welcome colours, client intent flow, model/ring/minimap palette mapping, API floors, and shared wire registration order. Native server and protocol checks pass outside the two findings above.

This review did not run WASM, local/public browsers, cold-play probes, GPU/performance checks, or a visual colour inspection. Root owns those checks. No claim about those checks follows from this review.

The detached review checkout contains only uncommitted independent test instrumentation. It uses no new lane build cache.

## Author closeout

Both findings are fixed in `18c4c31a`. The retained regressions pass.
Evidence: `target/mc-evidence/ready-lobby/client-lobby-final.log` and `native-fixes.log`.
Browser checks also found and fixed lazy receiver handling and colour-button presentation.
The final client passes 354 tests, with one existing ignored test.
No second review loop was started.
