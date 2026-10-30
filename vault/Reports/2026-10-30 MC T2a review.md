---
type: report
date: 2026-10-30
tags: [turts, mc, review]
ai-first: true
source: codex
---

# T2a independent review

Verdict: PASS for code review and scoped verification; full G-RUST remains pending with root before merge.

Reviewed commit: `e8f9e024c752cee12b492be8deb413beee9b767c`, branch `codex/mc-t2a`, base `f8b204e` in `/home/julien/Projects/TuRTS-mc-t2a`. HEAD verified before and after review. Exact diff contains only `messages.rs`, `tests/wire4.rs`, and `tests/reasons_m3.rs` (180 additions). No code findings or blockers.

## Contract review

- Plan task 2a lines 313–336 and design revision 10 sections 5.1, 5.4 reasons, and 5.5 reviewed. All nine public types match planned field names, types, field order, and enum variant order exactly: `OrderEnvelope`, `OrderTarget`, `QueueMode`, `UnitRefusal`, `OrderResult`, `Activity`, `OrderSummary`, `OrderFailure`, and `UnitOrderRecord` (`crates/turts_net_protocol/src/messages.rs:54–133`). A Python assertion compared every field and enum variant against independent literal plan contracts, exit 0.
- All six new record structs carry `#[serde(deny_unknown_fields)]` (`messages.rs:56`, `82`, `89`, `111`, `118`, `126`). The three enums use ordinary serde enum decoding.
- Eight reasons and exact `as_str` spellings append after `Internal` (`messages.rs:204–211`, `284–291`). Static comparison against the base proves all 29 old enum positions unchanged. `ALL` is the complete enum sequence of exactly 37 unique variants (`messages.rs:214–252`), with no omission or duplicate; validation exit 0.
- Protocol remains 3 (`crates/turts_net_protocol/src/lib.rs:13`). Existing `PlayerCommand`, `PlayerCommandKind`, `Welcome`, `Snapshot`, `WorldDelta`, `CommandApplied`, and `CommandRejected` definitions are byte-identical to base. Repository search finds new top-level types only in their declarations and new protocol tests; task 2b wiring has not started. No dependency or lockfile changes.
- `wire4.rs:9–29` round-trips all six targets in full envelopes through postcard; `wire4.rs:33–47` round-trips accepted/refused outcomes and a job ID; `wire4.rs:51–65` pins all eight names. `reasons_m3.rs:20–39` adds all eight to reason round-trip/membership coverage.

## Recorded builder evidence

Read `/home/julien/Projects/TuRTS/target/mc-evidence/t2a-builder/wire4-red-semantic.log`: compiled successfully; named `new_reason_names_are_stable` assertion failed with left `internal`, right `unknown_order`, and 2 other tests passed. Read `wire4-green.log`: 3/3 passed. Builder G-QA logs show yardstick inventory match, completeness 21/21 self-tests and 2 checked specs, and driver rule 46-case PASS. Logs themselves omit explicit exit-code markers; independent review reruns below record actual exit codes.

## Independent plant and restoration

Root confirmed builder idle before mutation; builder notified. Temporarily changed only `UnknownOrder.as_str()` from `unknown_order` to `internal` at `messages.rs:284`.

Command: `cargo test -p turts_net_protocol --test wire4 --locked new_reason_names_are_stable -- --exact`.

Result: exit 101, exactly `new_reason_names_are_stable` failed with left `internal`, right `unknown_order`; compilation succeeded. Evidence: `plant-red.log` beside this report. Restored the file using the committed HEAD blob in a Python `finally` block; byte equality with committed blob and empty `git status --porcelain=v1` verified immediately and after green tests. No fixes, staging, commits, merge, push, vault edits, or additional caches/copies.

## Independent verification

Every Rust invocation used `CARGO_TARGET_DIR=target/mc-t2a CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4`. Test binaries were built and run sequentially.

| Command | Exit | Result / evidence |
| --- | --- | --- |
| `cargo test -p turts_net_protocol --test wire4 --locked` | 0 | 3 passed; `wire4-green.log` |
| `cargo test -p turts_net_protocol --test reasons_m3 --locked` | 0 | 1 passed; `reasons-m3.log` |
| `cargo test -p turts_net_protocol --test protocol3 --locked` | 0 | 2 passed; `protocol3.log` |
| `cargo test -p turts_net_protocol --test wire --locked` | 0 | 2 passed; `wire.log` |
| `cargo test -p turts_net_protocol --lib --locked` | 0 | 1 passed; `lib.log` |
| `cargo fmt --all --check` | 0 | Formatting clean |
| `python3 scripts/qa/yardstick.py --check` | 0 | Inventory match; `yardstick.log` |
| `python3 scripts/qa/completeness.py --self-test` | 0 | 21/21 passed; `completeness.log` |
| `python3 scripts/qa/completeness.py --enforce` | 0 | 2 specs checked; `completeness.log` |
| `python3 scripts/qa/driver_rule.py --self-test` | 0 | 46 cases PASS; `driver-rule.log` |
| `python3 scripts/qa/driver_rule.py tests/browser` | 0 | Clean; `driver-rule.log` |
| `git diff --check` | 0 | Clean |

Pending: full G-RUST (workspace clippy, wasm check, workspace tests, boundary gate) is root-owned and intentionally not started while root holds the compiler slot for integration/task gates. Formatting already passed. No browser work applies to task 2a; Firefox is explicitly deferred by user, and K11(a) remains Chrome. This review does not establish full gate acceptance or authorize merge.
