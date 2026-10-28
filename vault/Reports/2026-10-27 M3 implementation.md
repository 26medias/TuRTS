---
type: report
date: 2026-10-27
tags: [turts, m3, report]
---

# M3 implementation

Work is active on `codex/m3-integration`. T0 and T1 are accepted (2/20); T2 and T3a run in parallel. See [[M3 Execution Board]] for the task ledger and [[Work Log/2026-10-27]] for exact commits, tests and evidence.

The M2 operation stream is frozen. API0.2.1 schema, package readers, resolved data and source-file diagnostics pass review and canonical gates: 504 passed,0failed,17ignored, plus fmt/strictClippy/check/wasm/boundary. Independent defects and refutations include extra captured operations, wrong event kind with unchanged counts, missing serde defaults, and old-reader serializer incompatibility. The serializer repair keeps old `^0.2` packages loadable by the actual baseline0.2.0 parser while preserving nonempty M3 fields.

Research remains private. The user resolved F11's contradictory opponent trigger by authorizing a test coordinator to release the move after the owner client's replica confirms completion.

No industrial simulation, forge demo, process/browser acceptance, push or main merge is claimed. Build caches are removed after task acceptance; evidence is retained. The board requires separate approval before the final branch push and main merge.
