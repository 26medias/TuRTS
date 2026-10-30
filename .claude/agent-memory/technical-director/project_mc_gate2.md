---
name: mc-gate2-plan-facts
description: Verified 2026-10-29 facts from the MC plan gate-2 review: turts_server has no lib target, scan hits for MC names, version-bump test traps, Cargo.lock member edges, debug.rs owns turts_debug and --out
metadata:
  type: project
---

Verified 2026-10-29 at f69badf (MC plan gate 2; report docs/research/mc/gate2-td.md):
- turts_server is binary only (no src/lib.rs). A dev-dependency from turts_pack gives "ignoring
  invalid dependency `turts_server` which is missing a lib target", then E0432. Server tests
  reach src modules with `#[path = "../src/x.rs"]` (tests/economy_export.rs:1, export_m3.rs:5,
  economy.rs:9).
- No-content scan (scan_rust, support/mod.rs:451) on real crates/: hits for `marker` (6),
  `gather` (17, module name), `enable` 2, `disable` 5, `remove` 37, `research` 115. Zero hits:
  move (keyword), stop, hold, attack, cancel, rush, trooper, post, dummy, ore, beacon, picket,
  pawn, decoy, order_gather. Generic exceptions are fixed: field, Item, Balance, log, credit
  (support/mod.rs:232-245). TokenCatalog.records is private; a scratch probe must make it pub.
- API bump traps: t2c_package_rejects.rs:57 asserts the engine version text "0.2.2". Tests that
  rewrite `engine_api: "^0.2"` (with the closing quote) become no-ops once a package says
  `^0.2.3`: t2c :52, :64 go red; e10_cedar (server tests/economy.rs:3182, in acceptance.sh:92)
  panics at expect_err. bad_packages.rs still passes (run).
- Cargo.lock lists member-to-member edges, dev-deps too (Cargo.lock:8596); `--locked` refuses a
  new path dependency with exit 101. Any new crate edge edits Cargo.lock.
- turts_debug = turts_client::debug::debug_json (client_web lib.rs:6); --out = report_json
  (debug.rs:212). Both read resources directly; a new debug key value means a debug.rs edit
  unless keys read typed resources.
- wire_reason is exhaustive in outbound.rs:55-90 and a second sim->wire match at export.rs:431.
- Frozen M2 stream JSON holds package ids only, no version or hash: a manifest version bump keeps it.
- Members glob `crates/*`: a new crate needs no root Cargo.toml edit. t7.sh and server tests pick free ports.
- Gate 1 of MC left 17 lens reports in docs/research/mc/ (budget line said 14 for both gates).

**Why:** MC execution and later plans will place tests and rename content; these traps recur.
**How to apply:** Re-check line numbers before citing; MC tasks move these files. Run the scan
probe for any new content id before a plan claims a name passes.

Re-check 1 (plan 9910798, design 89e1fa3 rev 8): rulings applied. Content ids are `order_<verb>`,
inputs `key_<verb>`, unit `beacon`; real scan 0 hits for all. Also hits: collect 256, build 8,
bindings 10, assets 12, produce 6. Server-code tests go in crates/turts_server/tests with #[path].
Debug keys read typed feeds in debug_feed.rs. MC critical path 55, total 209.
