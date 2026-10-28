# M3 ready to test — 2026-10-28

All20 implementation entries, checkpoints C1–C4 and final technical/player/mod-author reviews are accepted after the final repair. Ready for local Forge playtesting. Hosted CI, push and merge to main remain pending approval.

Code source: `07936e8b31f0f8f9c8d5155b5e99adc49301c950` on `codex/m3-integration`; final closeout edits are documentation only. Original checkout remains on main. All75 approved agent runs used; no further agents dispatched.

## Try Forge

[Open the local playtest](http://127.0.0.1:7140/?server=http%3A%2F%2F127.0.0.1%3A7140&ws=ws%3A%2F%2F127.0.0.1%3A7141). First tab owns the automatic factory; second tab owns drones. Follow [[M3 Playtest Guide]]. Two permanent client slots: closing a tab does not free one. Simulation runs before joining, so restart for a fresh demonstration:

```sh
/home/julien/Projects/TuRTS/target/m3-playtest/20261028-07936e8/restart.sh
```

The helper validates its owned server before restarting it. Preview uses loopback7140/7141; existing7000/7001 and the user's browser are untouched. No test audit/tick logging is enabled in its configuration. Setup consumed no player slot. Provenance: `target/m3-playtest/20261028-07936e8/playtest-manifest.json`.

## Acceptance

Current code:763 passing tests,0 failures,18 ignored; fmt, strict Clippy, wasm, boundary and frozen M2 stream passed. Fresh formal17, packer19, optimized actual Forge browser7 and direct guard bypass/red/restore proof passed. Technical scoped recheck: PASS0 findings. Required player guide corrections and mod-author status correction are closed. Root confirmed final acceptance process PIDs2789700/2789699/2789790 no longer exist; intentional preview PID2799276 responds HTTP200 with the expected owned executable.

C4 executed on frozen dae6739: native33, formal17, M2 browser8, joint positive15, state-triggered private research F11, Forge7 and T7. Final07936e8 is an affected-domain amendment; unchanged valid-domain historical evidence is explicitly reused, not relabeled as a fresh final-source run. Full provenance/limits: [[M3 Acceptance Evidence]]. Artifacts: `target/m3-evidence/final/fix/` and `target/m3-evidence/final/technical/recheck.md`.

## Remaining integration

README/vault closeout prepared. Board section7 requires user approval before pushing `codex/m3-integration`, then hosted CI green and a separate approval before merging main. Neither push nor merge has occurred. Private orchestration workspace and evidence retained pending integration; no broad cache deletion performed at handoff. All33 decisions and their costs are retained in [[M3 Orchestrator Rulings]].

Superseded desktop preview: human feedback required a UI amendment. Active run is target/m3-playtest/20261028-1354e33/ with named Factory/Drones tabs, object labels and research feedback. Use [[Reports/2026-10-28 M3 human playtest repair]] for current765/0/18 evidence and reset helper; earlier07936e8 paths above are historical.
