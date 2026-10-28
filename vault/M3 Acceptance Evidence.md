# M3 acceptance evidence

This index tracks the original F1–F11 requirements. A green task or unit test does not imply final milestone acceptance. The final process/browser run, registered mutation sweep, C4 checkpoint and milestone judges remain required. Evidence paths below are relative to the original repository root.

| Requirement | Current evidence | Remaining proof |
| --- | --- | --- |
| F1: content independence | T5 E1 scanner and three content plants accepted; T14 renamed industrial operation trace reviewed | C3 accepted; final frozen-union gate run remains |
| F2: unchanged M2 behavior | Frozen T0 stream SHA `bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b`; canonical gates through T15b; native M2 fault sweep33/33 passed | Final native process and browser regression suites |
| F3: portable package and boot | T2 validation, T4 public boot, T5 Forge package accepted | Final acceptance against the full union |
| F4: literal timing | Accepted phase tests; T14 timing suite reviewed | Final frozen-union gates |
| F5: C1–C17 conservation | T14 repaired suite reviewed, 32 focused tests green | T16 final runner review and C4 sweep |
| F6: fixed point | T14 fixed-point suite reviewed | Final frozen-union gates |
| F7: research changes ten shots to five | T11 atomic combat tests and T14 industrial scenarios reviewed | Actual F11 passed; final accepted-source replay and C4 remain |
| F8: wire privacy and lifecycle | T13b accepted, including authoritative belt ownership and health-zero removal; T15a atomic ingest and industrial queue correction accepted | C3 accepted; final process verification and C4 remain |
| F9: 17 registered faults | T16 builder all17 baseline/red/restored passed; prior independent evidence retained | Final runner interruption repairs, R1 and independent C4 |
| F10: real browser panels and commands | T15b accepted after Sol R1; seven-stage real browser proof with Acked/Applied and state changes | Final Task16 source and C4 |
| F11: two clients driven by replica state | Actual server/two-client F11 passed; root verified owner privacy, completion1361 before Move1363 and both health traces | Final runner/source acceptance and C4 |

Canonical T14 V0 source: `0e288550863a7180479571fd372c9cd868afbd47`. T14 review: `target/m3-evidence/t14-review/recovery-report.md`. Accepted T15a union gates: `target/m3-evidence/t15a-v0/summary.json`. Live board: [[M3 Execution Board]].

C1 uses the observable three-layer capacity fault defined in [[Decisions/2026-10-28 M3 C1 observable capacity fault]]. A single inner-guard deletion stays green because other defenses remain; it is not credited as a successful red. Health trace research ordering follows [[Decisions/2026-10-28 M3 coordinator signal contract]].

Industrial queue correction accepted on `88d5a3a`: canonical 750/0/17, all eight gates green. Evidence: `target/m3-evidence/t15a-queues/root-acceptance.md`. The preceding 742-test T14 run remains historical evidence.

C3 accepted on `2dcdf49`, including source-form correction and precise red-proof reconciliation. T15b accepted on merged `e4c591f`: canonical758/0/17, all eight gates; M2 browsers15/15 and seven-stage Forge rendered control proof. Task16 has passing F11, browser and mutation proofs, but final runner/source gates and review remain outstanding. See root acceptance files under `target/m3-evidence`.

Task16 current proof locations: `target/m3-evidence/t16-builder/f11/run-20261028-fde2d92/f11-summary.json`, `target/m3-evidence/t16-root-f11-preflight.json`, `target/m3-evidence/t16-builder/browser/run-20261028T185908Z-mVY7sY`, `target/m3-evidence/t16-root-m2-native-preflight.json`. Native33 manifests are retained in `target/m3-worktrees/t16/target/m2-refutations/20261028T190611.193595Z-2170093-ea4f701a`. These are builder/root-preflight results, not completed C4 acceptance.
