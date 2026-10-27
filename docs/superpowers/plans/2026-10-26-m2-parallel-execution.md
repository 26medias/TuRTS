# M2 parallel execution amendment

Approved scope and gameplay remain the revision-2 plan/spec. Tasks 1–2 are reviewed at `9e383896`. The user's explicit parallel-worktree instruction overrides sequential dispatch and the single-implementer rule only for isolated ownership below. Sol coordinates/reasons/reviews; Luna implements. This amendment needs no additional approval gate.

## Start three lanes now

Canonical integration/vault: `/home/julien/.codex/worktrees/ee40/TuRTS`. Pin named base `m2-parallel-base` to `9e383896`; create three native isolated worktrees/branches from it: `m2-sim`, `m2-data`, `m2-wire`. Native tooling first, git-worktree fallback. Root plus three implementers fills four slots; reviews temporarily replace idle implementers. Never dispatch a fourth child.

**A — simulation:** execute original Task 3 using its existing brief/context, then Tasks 4→5→6→7→8 in order, with review between original tasks. Own exactly those tasks' simulation files, `turts_pack/tests/common/`, their named economy tests, and accounting evidence. First deliverable: real atomic transfer/reservation operations and immutable query tests, independently executable on Task 2 boot. No network dependency. Task 6's positive contention tests precede live network-economy integration; Task 7 immediately follows before production completion claims.

**B — authored packages:** prepare the final base/haul/acme and exact ten-file Cedar data from approved §8/companion. Own `mods/{base,haul,acme,cedar}/` and new `crates/turts_pack/tests/economy_package_boot.rs` only. First deliverable: pack actual directories, parse archives, boot both slots and assert exact catalog, seeds, geometry, capabilities, zero-account behavior and acme absence of economy. No synthetic catalog injection or no-op economic success. Keep changes in its branch until the coordinator migrates old base-dependent M1 expectations. This is Task 9/14 preparation; positive loops, independent accounting, variants and E1–E10 evidence remain required later.

**C — portable wire preparation:** own new `crates/turts_net_protocol/src/staged_v2.rs`, its submodules if needed, `tests/staged_v2.rs`, and the single `pub mod staged_v2;` addition in protocol `lib.rs`. First deliverable: complete approved protocol-2 DTOs, closed reason keys and postcard roundtrips for every field/variant, while protocol 1 remains selected. No server/client consumer edits, live registration or version bump. Task 10 remains incomplete. After reviewed C1, prepare nonselected registration with exact ten-message order/channel tests in new `turts_net_lightyear/src/staged_v2.rs` plus module declaration; current plugin unchanged. Do not invent a second selected protocol.

## Freeze interfaces before dispatch

Copy these references into each brief; deviation returns to coordinator before dependent work:

* Simulation: original Shared type decisions and Task 3 signatures/context verbatim. Internal `ContentId`/simulation ids stay distinct from wire strings/newtypes; private reservation-claim tombstones remain authoritative. Task 4 owns `WorldView`, `CommittedTick`, `Applied`, command signatures and completed accessors exactly as planned.
* Wire: spec §7 exact `PlayerCommandKind`, Snapshot/WorldDelta, Node/Balance/Inventory/Order/Queue/JobState and EconomyState/Changes fields; `NodeId(u32)`, `JobId(u64)`, qualified `ContentRef=String`. Inventory wire capacity comes from definitions, never an added field. Retained handshake types reuse existing definitions where shape is unchanged. Staged names live under `staged_v2`; activation relocates/reexports once and deletes staging. No compatibility defaults.
* Commands/UI: original `PendingCommands(VecDeque<PlayerCommandKind>)`, checked `CommandSequence::allocate`, `Selection(Option<u32>)`, `UiAction`, and `panel_model(&Replica,&ClientContent,&Selection,&CommandResults)->PanelModel` remain final interfaces. No lane invents substitute gameplay records.

## Dependencies and integration

```text
base → A3 → A4 → A5 → A6 → A7 → A8
base → Bboot ───────────────────→ T9 integration
base → C1 DTO → C2 registration → T10 atomic activation
A6 + A8 + T9 + C2 → T10 → T11 export → T12 replica
C1 → UI presentation preparation; T12 → T13 bindings
T9 + A8 → T14; T11–14 → T15 → T16 → whole-branch gates
```

After Bboot, reuse that slot for shared UI presentation preparation: new `economy_ui.rs`/tests only, data-derived formatting, actual Node/Text/action linkage and rendering snapshot helpers; fixture inputs are explicit immutable records, not fake Replica/network implementations. Coordinator owns client feature/dependency changes. Wait for C1 before typed actions; preserve final adapter signature for T13. If this separation needs replacement Replica APIs or extensive shared edits, stop preparation and switch the slot to Task 14 data variants/scan fixtures; integrate UI after T12. Likewise if staged registration requires live transport changes, stop at DTOs and defer registration to atomic T10.

Coordinator alone owns shared activation paths: root/client Cargo manifests/lock, server session/outbound, client lib/input/ingest/replica/commands/render, web bindings, migrated M1 tests/scripts. Merge reviewed A checkpoints first; integrate B with M1 migration after A8; merge staged C; perform complete T10 activation atomically, then T11/T12/T13. No half-consumer API merge.

## Checks and reports

Each lane records named tests, semantic red→green, changed paths, interface deviations and commit/hash checkpoint. Prep marks no original task complete. Use hashed `--lib`/named integration targets without child processes with the shared Cargo cache; forbid concurrent un-hashed binary builds. If any target launches binaries, give that lane its own target directory or defer it. Root serializes workspace/wasm checks and individual server/client builds, process acceptance and browser checks in canonical worktree after integration. No full release Bevy rebuild per preparation lane. Retain task review and final E1–E10 validation.


## R6 — Post-Task-4 parallel simulation split

Coordinator accepted Sol's recommendation in `/tmp/post-task4-parallel-brief.md` under the user's existing parallel-worktree authorization. This changes execution routing only; approved gameplay and design scope do not change. The full assessment is retained at `.superpowers/sdd/2026-10-26-m2-economy/post-task4-parallel-brief.md`.

After Task 4 passes review, branch two isolated worktrees from that exact reviewed Task 4 commit: the existing simulation lane A continues gathering Task 5; a new production lane implements Task 6, then Task 7 only after Task 6 review. Do not create the production worktree until the exact Task 4 approved baseline is known. Task 8 waits until reviewed Tasks 5–7 are integrated. Task 7 merges only after Tasks 5 and 6 are integrated together and combined positive timing/atomicity tests pass.

Freeze existing interfaces: `assign_gather`, `interrupt_order`, `run_gather`; `queue_production`, `run_production`, `cancel_production`, `try_output`; Task 4 command/event/view types and `SimSet` ordering; Task 3 reservation/charge/refund semantics and claim tombstones. No new public mutation API, alternate ledger/allocator/domain records, or transaction architecture. Report any necessary signature change before dependent work.

Gather lane owns `economy/gather.rs`, `economy_gather.rs`, gathering order/carry types, extraction/Store/Convert operations/tests, and `tests/common/mod.rs::scenario_with`. Production lane owns `economy/production.rs`, `economy_production.rs`, `economy_completion.rs`, payment/FIFO/output/cancellation, reservation lifecycle and Task 7 allocator/spawn changes. Both may edit planned shared `command.rs`, `plugin.rs`, and `economy/{mod,transaction,views,events}.rs` only within disjoint semantic ownership: gather arms/helpers/projections versus production arms/helpers/projections. Coordinator alone composes shared-file hunks, especially Transaction/WriteSet/commit, event ordinal allocation, combined dispatch, fixed scheduling and immutable collection. Each lane reports common-file hunks and assumptions; resolve conflicts by composing validated typed writes. Do not refactor transaction infrastructure just to avoid conflicts. If semantics conflict, pause the shared change and resolve centrally while independent work continues.

Review Task 6 independently before Task 7 starts. Integrate reviewed Task 5 and Task 6 in either arrival order, then rebase/review Task 7 against their union before merging; an earlier Task 7 completion is held. At combined integration, rerun ordering/gather/production/completion and retain a positive witness that gathering conversion income is available to same-tick producer start while commands use tick-start stock; check reserved cargo remains unavailable, combined events are ordered/atomic, and newly spawned capable units begin empty and orderless. No separate accounting-preparation lane; Task 8 validates finished behavior. This split adds one deliberate shared-file conflict/review pass and combined tests.

Earlier schedule text for Tasks 5–7 is superseded only where it conflicts with this R6 routing. All other ownership, interface, check and integration rules remain in force.
