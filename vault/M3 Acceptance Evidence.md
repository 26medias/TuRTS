# M3 acceptance evidence

This index tracks the original F1–F11 requirements. A green task or unit test does not imply final milestone acceptance. The final process/browser run, registered mutation sweep, C4 checkpoint and milestone judges remain required. Evidence paths below are relative to the original repository root.

| Requirement | Current evidence | Remaining proof |
| --- | --- | --- |
| F1: content independence | T5 E1 scanner and three content plants accepted; T14 renamed industrial operation trace reviewed | C3 validation of the merged trace and final gate run |
| F2: unchanged M2 behavior | Frozen T0 stream SHA `bcbf72c2206251f635a7c7f5b6c8f573d69a5fc05a5bd77716e98ebfb08e370b`; canonical gates through T15a | Final native process and browser regression suites |
| F3: portable package and boot | T2 validation, T4 public boot, T5 Forge package accepted | Final acceptance against the full union |
| F4: literal timing | Accepted phase tests; T14 timing suite reviewed | C3 and final gates |
| F5: C1–C17 conservation | T14 repaired suite reviewed, 32 focused tests green | C3 and registered fault sweep |
| F6: fixed point | T14 fixed-point suite reviewed | C3 and final gates |
| F7: research changes ten shots to five | T11 atomic combat tests and T14 industrial scenarios reviewed | Final process trace of both drones |
| F8: wire privacy and lifecycle | T13b accepted, including authoritative belt ownership and health-zero removal; T15a atomic ingest accepted | C3 and final process verification |
| F9: 17 registered faults | Prior independent fault evidence retained | T16 registry/runner and full baseline-red-restored sweep |
| F10: real browser panels and commands | T15b implementation saved; browser gates in progress | Actual Produce, SetEnabled and Remove receipts/state changes, R1 and C4 |
| F11: two clients driven by replica state | Coordinator interface frozen before T15b; owner-only research rule preserved | T16 real server/client test, ordering audit and public health traces |

Canonical T14 V0 source: `0e288550863a7180479571fd372c9cd868afbd47`. T14 review: `target/m3-evidence/t14-review/recovery-report.md`. Accepted T15a union gates: `target/m3-evidence/t15a-v0/summary.json`. Live board: [[M3 Execution Board]].

C1 uses the observable three-layer capacity fault defined in [[Decisions/2026-10-28 M3 C1 observable capacity fault]]. A single inner-guard deletion stays green because other defenses remain; it is not credited as a successful red. Health trace research ordering follows [[Decisions/2026-10-28 M3 coordinator signal contract]].
