# M3 acceptance evidence

This index tracks the original F1–F11 requirements. A green task or unit test does not imply final milestone acceptance. The final process/browser run, registered mutation sweep, C4 checkpoint and milestone judges remain required. Evidence paths below are relative to the original repository root.

| Requirement | Current evidence | Remaining proof |
| --- | --- | --- |
| F1: content independence | Final workspace no-content tests and M2 E1/E2 native mutation cases; evidence under `target/m3-evidence/t16-builder` | Final reviewer and C4 checkpoint |
| F2: unchanged M2 behavior | Frozen stream SHA `bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b`; final native process acceptance passes | Final 15-case M2 browser refresh after the exact-selection harness repair |
| F3: portable package and boot | Final `acceptance.sh`: Forge/Cedar public package boot and T5 pass; log in `target/m3-evidence/t16-builder/acceptance/final-20261028T20` | Final reviewer and C4 checkpoint |
| F4: literal timing | Final workspace timing/fixed-point suites pass; native workspace log in `target/m3-evidence/t16-builder/gates/native-workspace-test.log` | Final reviewer and C4 checkpoint |
| F5: C1–C17 conservation | All 17 scenario tests and final C1–C17 baseline/red/restore sweep pass; `target/m3-evidence/t16-builder/refutations/final-native` | Final reviewer and C4 checkpoint |
| F6: fixed point | Final workspace fixed-point suite passes, including ten versus five shots | Final reviewer and C4 checkpoint |
| F7: research changes ten shots to five | F11 audit records ten 10-point shots on A and five 20-point shots on B; retained in `target/m3-evidence/t16-builder/acceptance/final-20261028T20/f11-evidence` | Final reviewer and C4 checkpoint |
| F8: wire privacy and lifecycle | Final F11 reports prove opponent has no owner research; M2 two-owner lifecycle acceptance passes | Final reviewer and C4 checkpoint |
| F9: 17 registered faults | M3 C1–C17 all pass baseline green, causal semantic red, exact restore green; native M2 33/33 also passes. Browser E2 timeout attempt is retained and uncredited; repaired browser sweep is running. | Complete the serial M2 browser refutation retry, then R1 and C4 |
| F10: real browser panels and commands | Seven-stage actual browser run: Produce tick1169, SetEnabled1177, Remove1194; evidence `target/m3-evidence/t16-builder/browser/run-20261028T185908Z-mVY7sY` | Final reviewer and C4 checkpoint |
| F11: two clients driven by replica state | Final F11 passes: server research tick1361, opponent MoveTo applied tick1363, both initial health100 and complete health traces; evidence `target/m3-evidence/t16-builder/acceptance/final-20261028T20/f11-evidence` | Final reviewer and C4 checkpoint |

Canonical T14 V0 source: `0e288550863a7180479571fd372c9cd868afbd47`. T14 review: `target/m3-evidence/t14-review/recovery-report.md`. Accepted T15a union gates: `target/m3-evidence/t15a-v0/summary.json`. Live board: [[M3 Execution Board]].

C1 uses the observable three-layer capacity fault defined in [[Decisions/2026-10-28 M3 C1 observable capacity fault]]. A single inner-guard deletion stays green because other defenses remain; it is not credited as a successful red. Health trace research ordering follows [[Decisions/2026-10-28 M3 coordinator signal contract]].

Industrial queue correction accepted on `88d5a3a`: canonical 750/0/17, all eight gates green. Evidence: `target/m3-evidence/t15a-queues/root-acceptance.md`. The preceding 742-test T14 run remains historical evidence.

C3 accepted on `2dcdf49`, including source-form correction and precise red-proof reconciliation. T15b accepted on merged `e4c591f`: canonical758/0/17, all eight gates; M2 browsers15/15 and seven-stage Forge rendered control proof. F11 and final registered-sweep/process/CI evidence remain Task16. See root acceptance files under `target/m3-evidence`.
