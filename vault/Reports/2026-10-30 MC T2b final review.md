---
type: report
date: 2026-10-30
tags: [turts, mc, review]
source: codex
---

PASS on 64cd4006681efedefa1ee678b5468a07f28b5f5d. Open findings: 0 High, 0 Medium.

Reviewed the complete Task 2b diff against 363d02d as mod-runtime-engineer with qa-lead review of the registry migration. No source changes remain. Compiler and plant ownership is handed back idle to root.

Independent runtime proof:
- cargo test -p turts_client --lib --locked: 21 passed, actual RC 0. Covers real CLI parsing/auto_move/private sender capture, first Move with non-Move first and two Move definitions, literal empty/static/foreign subject preservation, empty-envelope debug rejection, partial acceptance, shared group preservation and atomic sequence exhaustion.
- cargo test -p turts_net_protocol --test wire4 --locked: 4 passed, actual RC 0, including 4096 maximum-width UnitId varints.
- cargo test --release -p turts_server --test economy e8_commands_are_authoritative_and_acked --locked -- --ignored --exact --nocapture --test-threads=1: 1 passed, actual RC 0. Rechecks C19 original malformed syntax, owner/capability precedence and admission/audit-route semantics through real server/client transport.
- Independent plant changes only messages.rs MAX_ORDER_UNITS 4096 to 2048. Exact sender test with literal 4097 subjects has baseline RC 0, compiled semantic red RC 101 at commands.rs:525 (4 envelopes versus pinned 3), then restored RC 0. cap-original.rs and receipt.json retain source bytes, hashes, exact commands, environment and actual return codes. Restoration is byte-identical and git status is empty.

Repaired findings:
- C18 Medium repaired: debug.rs:217 handles envelope-only empty rejection; debug.rs:417 and :434 cover rejection keys and partial acceptance. Independent client suite is green; builder semantic red 101 and green 0 receipts were read directly.
- C19 High repaired: session.rs:309 and :340 select Produce by Behaviour and preserve unit existence/ownership, capability, then reference syntax before domain command/audit. Exact malformed fixture bodies are restored. Independent E8 process is green.
- C20 High repaired: refutations.json:669 retains PendingOrderCommands in both original and mutated install; :711 retains the original stable failure phrase. All four sequential E8 anchors occur once. The same delayed-Ack semantic defect yields the actual ReliableChannel MessageId assertion for seq 1; native phase RCs are 0/101/0 with exact restored session hash. No guard was relaxed and no plant retired.
- C21 route repaired: orders.rs:31 allows a valid empty Move/Point to reach the existing simulation EmptyCommand audit boundary; other empty fan-outs are locally refused. Real E8 and builder compiled red/green receipts confirm this route.
- C22 Medium repaired: commands.rs:217 preserves existing nonzero gesture group; :500 fixes 4097 input and 4096/1/1 chunk expectations in the real sender pipeline; :551 proves exhaustion emits no partial split. Independent cap plant proves this test detects cap drift.

All 14 final gate receipts are actual RC 0 on the committed candidate: G-RUST 5, G-QA 3, G-ACC 1, G-WEB 3 (Trunk, T7 headed Vulkan, M3 browser), G-FROZEN 2. Earlier interrupted acceptance and failed pre-repair invocations remain failures; they are not counted as green. All 18 retained F11 artifact hashes were checked. Frozen stream SHA remains bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b; expected Forge/stream values did not change.

All 10 required selected G-REF cases passed. I replayed unchanged require_phase_result against every raw phase log and actual RC, checked all 69 stdout hashes, checked all provenance source hashes against committed git blobs, and verified equal baseline/restored source hashes. Five native cycles are 0/101/0; five browser cycles are 0/1/0. Total 59 zero, 5 native semantic 101, 5 browser semantic 1. E2 authored-label failure was traced to the unchanged exact visible-text oracle, with infrastructure/timeout counts zero. gate-and-refutation-audit.json records this audit.

Protocol and scope: protocol 4 replaces old wire types and registrations; catalog Behaviour drives orders, universal 4096 cap precedes fan-out, match_id is created once and reused for Welcome/stats, owner-only unit_orders remains empty, and to_wire intentionally copies the first outcome to all subjects until Task 4. No future Task 4 simulation state was added. Existing MoveTo still caps 256 at turts_sim/src/command.rs:19,267; root explicitly assigned replacement to Task 4. This is a Task 2b protocol/adapter PASS and does not claim 4096-unit movement is implemented now.

Transport trace independently checked installed Lightyear 0.31: messages/send.rs:138-169 serializes whole logical messages; serde/registry.rs:184-189 and postcard_utils.rs:23-32 use growing Writer(BytesMut), writer.rs:54-56,83-88. OrderedReliable enters send_reliable.rs:95-112 fragmentation; fragments/send.rs:88-99 assigns chunks/count; fragments/receive.rs:63-84,136-146 reassembles with checked size arithmetic; receive.rs:304-308 yields complete messages in order. No competing logical cap below 4096*5 plus header was found. Full source paths are in builder/run30/lightyear-trace.txt.


## Task2b complete; Wave3 pushed; Wave4 registered

Task2b candidate64cd4006681efedefa1ee678b5468a07f28b5f5d merged6d2c46e7ca2f320b7d19103ca8563c9343a331cc. RUN30/31 complete,0High/0Medium. Compiled semantic reds/greens for protocol3Hello, real CLI first-Move hook, empty rejection keys, Produce owner-before-syntax, empty-Move audit route and split-group preservation. Final14gate receipts0:886passed/0failed/18ignored, QA, complete acceptance,18F11artifact hashes,Trunk,T7(138frames),Forge controls,frozen. Selected10GREF allpassed;69actual commandRCs=59zero,5native101,5browser1. Root and independent QA replayed original guards against raw logs/stdout hashes and exact64cd400git blobs; source restored. Independent client21/21,wire4 4/4,E8process1/1 all0; cap4096->2048 plant compiled namedred101 then exactrestoregreen0. C17-C23 record repairs/setup; no frozen expected/scanner/content changes. Legacy Move256 replacement remainsTask4; no full4096movement claim. Full review [[Reports/2026-10-30 MC T2b final review]].

Root postmerge Rust/QA/frozen/web all12commands0; source code identical64cd400,886/0/18,T7(138frames). Solecache moved by rename and all4workspaceprofiles invalidated, no stale workspace binaries. Evidence main target/mc-evidence/after-t2b, t2b-root-gate-audit.json, t2b-refutations-run30/root-guard-audit.json and t2b-review/run31. Clean merged T2b lane removed after provenance/re-gate, branch retained; protected main disk audit untouched.

Wave3 pushed6d2c46e fast-forward; CI41157502932 read queued (https://github.com/26medias/TuRTS/actions/runs/41157502932), draftPR2 updated and stays unmerged. Wave2 terminalcheck/agentsgreen, known browsertimeout retained. Mainc3f09d0 unchanged. C1-C23 available for orchestrator wave review. C24 sets single-cache Wave4 with Task4 compiler first and Task9 UX external test drafts. RUN32/33 registered before dispatch, total33/170.

## Changed paths

```text
crates/turts_client/src/commands.rs
crates/turts_client/src/debug.rs
crates/turts_client/src/economy_ui.rs
crates/turts_client/src/ingest.rs
crates/turts_client/src/input.rs
crates/turts_client/src/join.rs
crates/turts_client/src/lib.rs
crates/turts_client/src/live_view.rs
crates/turts_client/src/scenario.rs
crates/turts_client/tests/commands.rs
crates/turts_client/tests/economy_replica.rs
crates/turts_client/tests/economy_ui.rs
crates/turts_client/tests/ingest.rs
crates/turts_client/tests/replica.rs
crates/turts_client/tests/replica_m3.rs
crates/turts_client/tests/scenario.rs
crates/turts_client/tests/t8_resync_logic.rs
crates/turts_client_web/src/lib.rs
crates/turts_net_lightyear/src/bin/spike_client.rs
crates/turts_net_lightyear/src/bin/spike_server.rs
crates/turts_net_lightyear/src/channels.rs
crates/turts_net_lightyear/src/protocol_plugin.rs
crates/turts_net_lightyear/tests/registers.rs
crates/turts_net_protocol/src/economy.rs
crates/turts_net_protocol/src/lib.rs
crates/turts_net_protocol/src/messages.rs
crates/turts_net_protocol/tests/protocol4.rs
crates/turts_net_protocol/tests/wire.rs
crates/turts_net_protocol/tests/wire4.rs
crates/turts_server/src/export.rs
crates/turts_server/src/main.rs
crates/turts_server/src/orders.rs
crates/turts_server/src/outbound.rs
crates/turts_server/src/session.rs
crates/turts_server/src/stats.rs
crates/turts_server/tests/acceptance.rs
crates/turts_server/tests/economy.rs
crates/turts_server/tests/support/wire_sender.rs
tests/m2/refutations.json
```
