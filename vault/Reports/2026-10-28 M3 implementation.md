---
type: report
date: 2026-10-28
tags: [turts, m3, report]
---

# M3 implementation

Fourteen of20 task entries are accepted: T0, T1, T2, T3a, T3, T4, T5, T6, T7, T8, T9, T10, T11 and T12. Public Forge boot and shared spawn-row installation pass review; independent checkpoint C1 passed. Extraction, transport, inserters, machines, research and combat passed; Remove/SetEnabled commands passed; views/audit is under repair after independent review, before C2. Exact status: [[M3 Execution Board]]; evidence: [[Work Log/2026-10-28]].

Last canonical gates: 673 passed, 0 failed, 17 ignored; fmt, strict Clippy, workspace check, wasm, boundary and frozen M2 stream passed. The complete industrial chain and forge demonstration are still pending.

Research stays private under the user's approved F11 coordinator design. No push or main merge has occurred.

C1 refuted all88 added tests and found one masked capacity fixture. Test-only repair `32d1495` uses valid capacity20 against input21; the isolated aggregate-capacity fault now fails. Fresh gates remain585/0/17 with wasm and the frozen stream green. Evidence and exact limitations: `target/m3-evidence/c1/root-acceptance.md`.

T13a candidate `0ea4ab1` passes 684 workspace tests, strict checks and wasm in its isolated lane. Independent R1 found a blocking publication defect: removing a disabled unit produces a false enabled=true delta. The same builder is adding a real removal regression and preparing the fix; the same reviewer will recheck. T13a is not accepted. Checked weapon damage remains an explicit T13b wire-contract decision. Execution count:45.

T13a R1 repair: public scheduled Forge regression reproduced phantom enabled=true on actual removal (red log /tmp/m3-t13a-r1-removal-red.log). Root applied hash-verified plugin patch 7edeee5f2c7cfc23dd364e0e797640d18b11112b40e567251ec4f86538ffd266 to candidate lane; enabled rows now exclude drained removed IDs. Luna resumed for green regression, required gates and commit. Not accepted yet;14/20 remains. Execution count46.
