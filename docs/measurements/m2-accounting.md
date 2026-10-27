# M2 committed-operation accounting proof

## Scope and independent model

The integration oracle lives in `crates/turts_pack/tests/common/accounting.rs`. It freezes the submitted `Command`, prior `AuditState`, and immutable installed catalog; `FrozenJob` stores the recipe, stage, duration, charged lines, and reservation lines derived at queue time. Production start/advance/output, cancellation refunds, node extraction, unload conversion/Store, and trusted transfers are checked against that prior model. It never reads operation counters or events to choose an expected amount or endpoint. Events are checked after independent state prediction, and tick publication checks the independently built pending-event list and cursor.

The default-off `AuditTrail` records owned command, operation, and tick snapshots. Nested ledger operations are suppressed under the logical command/gather/production operation, and gather capture occurs outside each per-unit resource scope. Disabled capture executes the direct path. Invalid diagnostic ID exhaustion invalidates the trail and runs gameplay normally. `AuditState` serializes complete raw unit/order/inventory-filter data, catalog, durations, resource presence, event cursor, clock, and stable unit-type IDs; IEEE float bit codecs preserve non-finite command targets and raw positions. R11 enables only Bevy Math's `serialize` feature and uses the minimal lock delta; it adds no renderer feature.

## Verification

Historical checkpoint (28-test oracle): `cargo test -p turts_pack --test economy_accounting --locked` — 28 passed, 0 failed. Its hashes and mutation evidence below are historical only, not final corrected-model proof. Strict lint at that checkpoint: `cargo clippy -p turts_sim -p turts_pack --tests --locked -- -D warnings` — passed.

Earlier economy pack targets run sequentially in the Task 8 target directory:

| Target | Result |
| --- | --- |
| `economy_boot` | 15 passed |
| `economy_transactions` | 5 passed |
| `economy_gather` | 21 passed |
| `economy_ordering` | 7 passed after the approved full-snapshot fixture correction |

The first expanded run exposed an existing ordering assertion that compared state from before two accepted MoveTo commands to state after those commands and a rejected command. With coordinator approval, `crates/turts_pack/tests/economy_ordering.rs` now compares the full state immediately before the rejected command to its post-rejection state and checks each earlier MoveTo boundary independently.

The accounting cases use deterministic loader-backed fixtures, fixed sequences, and explicit rates (10, 20, 30 Hz). No randomized sequence was used, so there is no PRNG seed to report. All positive journeys remain within 2,000 ticks.

## Semantic fault plants

Plants ran in a minimal isolated source copy under `/tmp/task8-mutation/src` with a separate target `/tmp/task8-mut-target`. The scratch baseline passed all 28 accounting tests. Each plant was applied to scratch product code only, its named test failed semantically, the exact original product source hash was restored, and that targeted test passed again. Full command, assertion, original/mutated/restored hashes, and RED/GREEN outcomes are in `/tmp/task8-mutation/evidence.txt`.

| Fault | Detecting focused test | RED then restored GREEN |
| --- | --- | --- |
| Omit reservation availability | `e6_charge_and_reserve_policies_compete_for_the_same_last_stock` | Yes |
| Commit first mixed cost before validating second | `e6_mixed_cost_rejection_keeps_every_endpoint_unchanged` | Yes |
| Replenish a depleted node | `e6_partial_store_unloads_and_exhausts_the_seeded_node` | Yes |
| Allow inventory refund over capacity | `e6_running_cancel_refund_rejects_when_inventory_endpoint_is_full` | Yes |
| Discard cargo during Store | `e6_partial_store_unloads_and_exhausts_the_seeded_node` | Yes |
| Duplicate conversion credit | `e6_gather_conversion_and_interruptions_conserve_through_delivery` | Yes |
| Duplicate cancellation refund | `e6_cancel_refund_matrix_covers_both_policies_and_all_stages` | Yes |
| Duplicate output spawn | `e6_reserve_then_charge_tracks_claim_start_and_output` | Yes |
| Ignore transfer destination capacity | `e6_transfer_rejections_cover_filter_capacity_and_access` | Yes |
| Select self as Store receiver | `e6_self_store_is_never_selected_as_its_own_receiver` | Yes |

## Covered E6/E7/E8 paths

The accounting target covers both charge policies competing for the same final stock, mixed-cost rollback, transfer success and filter/capacity/access rejections, Store/Convert partial unload and depletion, no/full/overflow receiver waiting, cargo-preserving interruption/resume, Store self-receiver exclusion, queued/running/output-blocked cancellation under both policies, full-refund rejection, production completion and 10/30 Hz timing, rejected-command control, exact command-boundary presence, slot-claim neutrality, raw-state JSON roundtrip, and non-finite rejected command serialization. It also contains semantic refutations for poisoned job costs, recipe switch at start, wrong cancel target, removal of unrelated queued jobs, hidden inventory filter writes, stationary teleport, tick skip, repeated movement, forged tick cursor, and metadata writes.

## Exact fault-plant log

All ten plants used this command shape, with the exact test target from the table above:

```text
CARGO_TARGET_DIR=/tmp/task8-mut-target CARGO_BUILD_JOBS=2 cargo test -p turts_pack --test economy_accounting <test-name> --locked --manifest-path /tmp/task8-mutation/src/Cargo.toml -- --exact --nocapture
```

The scenarios are deterministic and use the exact loader fixture edits and input sequences in those named tests; no random generator or seed is involved. For each plant the targeted command failed with the listed semantic test assertion, source files were restored to the original hash, and the same targeted command passed.

| Plant | Semantic red assertion and seeded input |
| --- | --- |
| Omit reservation availability | Second producer policy must reject after the first accepted job consumes or reserves the sole 100-token stock (`second_result.outcome.is_err()`). Two ordered cases: ChargeOnQueue then ReserveThenCharge, and reverse. |
| Mixed-cost partial debit | Modified train recipe charges 100 token then 500 timber; yard has 10 timber. The rejected command must leave `before == after` and preserve token balance. |
| Replenish depleted node | Worker gathers from node 1 seeded with 25 timber for 1,000 ticks; oracle and final `remaining == 0` assertion reject replenishment. |
| Allow overflow refund | Hire consumes 50 timber, cargo fills the 50-capacity yard, then running cancellation must return `RefundDestinationFull` without state mutation. |
| Discard cargo | Same 25-timber Store/depletion journey; oracle state/event equation rejects the missing source cargo debit. |
| Duplicate conversion | 1,400-tick gather/MoveTo interruption/resume scenario; conversion event and slot balance must match catalog per-item multiplier 2 (credited tokens equal twice converted timber). |
| Duplicate refund | Five policy/stage cases across queued, running, and output-blocked jobs; exact catalog refund event and inventory/balance delta are checked. |
| Duplicate spawn | ReserveThenCharge `probe:hire` completes by tick 121; exact output UnitId, one spawned unit, queue removal, allocator, catalog spawn candidate, and full state must match. |
| Ignore capacity | Three transfer rejections include a one-slot full destination; trusted transfer must return an error and keep the whole state unchanged. |
| Self Store | Worker fixture gains a Store rule and gathers for 900 ticks; each tick asserts its selected receiver is not `Some(3)`. |


```text
Task8 isolated mutation evidence; scratch root=/tmp/task8-mutation/src; target=/tmp/task8-mut-target

## reservation availability omitted
targeted_test=e6_charge_and_reserve_policies_compete_for_the_same_last_stock
semantic_red=True
restored_green=True
assertion=thread 'e6_charge_and_reserve_policies_compete_for_the_same_last_stock' (1325693) panicked at crates/turts_pack/tests/economy_accounting.rs:1940:9: | test e6_charge_and_reserve_policies_compete_for_the_same_last_stock ... FAILED | test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 27 filtered out; finished in 0.01s
original_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160
mutated_hashes=crates/turts_sim/src/economy/transaction.rs:56596006cb37795c4e6fb7fa01bf611a51837fb6c770e2b35e855897b28e3c15,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160
restored_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160

## mixed cost first debit commits before second validation
targeted_test=e6_mixed_cost_rejection_keeps_every_endpoint_unchanged
semantic_red=True
restored_green=True
assertion=thread 'e6_mixed_cost_rejection_keeps_every_endpoint_unchanged' (1326755) panicked at crates/turts_pack/tests/economy_accounting.rs:1376:10: | test e6_mixed_cost_rejection_keeps_every_endpoint_unchanged ... FAILED | test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 27 filtered out; finished in 0.01s
original_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160
mutated_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:b529a2351171f474c92763e2f57f0c5ce4659ecab99bf66b82bd69f0631e1af2,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160
restored_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160

## depleted node replenished
targeted_test=e6_partial_store_unloads_and_exhausts_the_seeded_node
semantic_red=True
restored_green=True
assertion=thread 'e6_partial_store_unloads_and_exhausts_the_seeded_node' (1327823) panicked at crates/turts_pack/tests/economy_accounting.rs:50:33: | test e6_partial_store_unloads_and_exhausts_the_seeded_node ... FAILED | test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 27 filtered out; finished in 0.06s
original_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160
mutated_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:33627cc4a47c8c2008406e984e145218817b9400b63b3311bcc7eb656531857c
restored_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160

## inventory refund overflow allowed
targeted_test=e6_running_cancel_refund_rejects_when_inventory_endpoint_is_full
semantic_red=True
restored_green=True
assertion=thread 'e6_running_cancel_refund_rejects_when_inventory_endpoint_is_full' (1328883) panicked at crates/turts_pack/tests/economy_accounting.rs:1664:5: | test e6_running_cancel_refund_rejects_when_inventory_endpoint_is_full ... FAILED | test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 27 filtered out; finished in 0.03s
original_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160
mutated_hashes=crates/turts_sim/src/economy/transaction.rs:9169ea4cf8091e1a8bad11f499f4a886249569989673e9ed28e1c05f8d5abe97,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160
restored_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160

## Store discards source cargo debit
targeted_test=e6_partial_store_unloads_and_exhausts_the_seeded_node
semantic_red=True
restored_green=True
assertion=thread 'e6_partial_store_unloads_and_exhausts_the_seeded_node' (1329953) panicked at crates/turts_pack/tests/economy_accounting.rs:50:33: | test e6_partial_store_unloads_and_exhausts_the_seeded_node ... FAILED | test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 27 filtered out; finished in 0.13s
original_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160
mutated_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:521d03c72fe736cc8c8d527c4ce9d720c94776900410fd316b11877717dc7db7
restored_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160

## conversion credited twice
targeted_test=e6_gather_conversion_and_interruptions_conserve_through_delivery
semantic_red=True
restored_green=True
assertion=thread 'e6_gather_conversion_and_interruptions_conserve_through_delivery' (1331063) panicked at crates/turts_pack/tests/economy_accounting.rs:50:33: | test e6_gather_conversion_and_interruptions_conserve_through_delivery ... FAILED | test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 27 filtered out; finished in 0.15s
original_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160
mutated_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:ee089ae84e35d2df0b40dbecde8252e55e49ce47906bf5f7b138313ab92b9a80
restored_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160

## cancellation refund credited twice
targeted_test=e6_cancel_refund_matrix_covers_both_policies_and_all_stages
semantic_red=True
restored_green=True
assertion=thread 'e6_cancel_refund_matrix_covers_both_policies_and_all_stages' (1332133) panicked at crates/turts_pack/tests/economy_accounting.rs:1766:14: | test e6_cancel_refund_matrix_covers_both_policies_and_all_stages ... FAILED | test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 27 filtered out; finished in 0.01s
original_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160
mutated_hashes=crates/turts_sim/src/economy/transaction.rs:679c646dd8449cb1ad7fbc1e59dbe43cffd17a814a5ea00934ef3f2d33fe7181,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160
restored_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160

## output entity spawned twice
targeted_test=e6_reserve_then_charge_tracks_claim_start_and_output
semantic_red=True
restored_green=True
assertion=thread 'e6_reserve_then_charge_tracks_claim_start_and_output' (1333205) panicked at crates/turts_pack/tests/economy_accounting.rs:127:37: | test e6_reserve_then_charge_tracks_claim_start_and_output ... FAILED | test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 27 filtered out; finished in 0.10s
original_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160
mutated_hashes=crates/turts_sim/src/economy/transaction.rs:9ff28ab41b92148da12ac05c89b94c64502e15e29904c067c63e3db855eed43a,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160
restored_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160

## transfer destination capacity ignored
targeted_test=e6_transfer_rejections_cover_filter_capacity_and_access
semantic_red=True
restored_green=True
assertion=thread 'e6_transfer_rejections_cover_filter_capacity_and_access' (1334261) panicked at crates/turts_pack/tests/economy_accounting.rs:1491:9: | test e6_transfer_rejections_cover_filter_capacity_and_access ... FAILED | test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 27 filtered out; finished in 0.01s
original_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160
mutated_hashes=crates/turts_sim/src/economy/transaction.rs:b5dec8eaa099a5aa0622dc566637277e34ffbb2c1c338c6c6144bae42542f5dd,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160
restored_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160

## self Store receiver allowed
targeted_test=e6_self_store_is_never_selected_as_its_own_receiver
semantic_red=True
restored_green=True
assertion=thread 'e6_self_store_is_never_selected_as_its_own_receiver' (1335335) panicked at crates/turts_pack/tests/economy_accounting.rs:50:33: | test e6_self_store_is_never_selected_as_its_own_receiver ... FAILED | test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 27 filtered out; finished in 0.14s
original_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160
mutated_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:a10f2caf0e9d830cd49086b83f00831e4041999b7e5d0db130db34c6eb26f833
restored_hashes=crates/turts_sim/src/economy/transaction.rs:9a7d097a05230db6b491de981bfe6ae2e27ed35c2c591dfb87f96ae9b3b86a5f,crates/turts_sim/src/economy/production.rs:2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee,crates/turts_sim/src/economy/gather.rs:50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160

```

## Final corrected oracle and capture proof

This final evidence supersedes the earlier 28-test and pre-redraft fault-plant checkpoint above. It uses integrated raw queue capture commit `bdd89fb145d92019b6553bdcce3ca30fe2200373`, final oracle hashes `crates/turts_pack/tests/common/accounting.rs` SHA256 `f28b275bf1e3625939eaf3173036fc23d29e5b79442b80166667eced35ee8b7e` and `crates/turts_pack/tests/economy_accounting.rs` SHA256 `9b8f86e6a771edb14bb0bd0d2d713251f18fdb8daf79c79df6f6913be4a25fdf`. The corrected `economy_accounting` target passed 40 tests; the eight economy integration targets passed 124 tests total and the raw audit capture target passed 5 tests. Strict Clippy passed.

The final isolated scratch baseline at `/tmp/task8-mutation/src` passed all 40 accounting tests using `/tmp/task8-mut-target`. The ten required semantic product faults are evidenced across two baselines: seven retained families on the corrected 39-test baseline, then three exact families on the refreshed 40-test baseline. Each selected exactly one test, failed semantically (`0 passed; 1 failed`), and passed after exact source restoration (`1 passed; 0 failed`). The corrected reservation mutation removes both reserve and charge availability checks; an earlier ineffective single-check attempt is excluded. The former refund-capacity and Store-debit substitutes are EXTRA evidence, not required families. The broader final fault record is at `/tmp/task8-mutation/final-evidence.txt`; exact final proofs for the mixed-cost partial commit, arithmetic overflow, and carried-item loss are at `/tmp/task8-mutation/final-three-exact.txt`.

Additional oracle-model mutants for rejecting legal self-Convert and miscomputing partial conversion headroom were each detected and restored GREEN. A raw queue projection mutant that adopted observed after-state was also detected. Final oracle model and test hashes are listed above so review can distinguish this proof from the older checkpoint. The five sim capture tests cover boundary fidelity and resource presence; pack accounting tests roundtrip active raw queue/reservation snapshots and operation payloads.

### Durable final required-family evidence

These are evidence-baseline hashes for the retained mutation runs, not claims about files changed by ongoing work after those runs. Restored engine source hashes recorded in the final logs are: transaction.rs `0a2ea70eb6d6f62541ad649a5ad216eb32a3838976c136346d047eafe98c5c61`; command.rs `82816bc71257d9d680d1b1d9b7ec8ae90df1bef66ebb3c57389681001c2b163d`; production.rs `2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee`; gather.rs `50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160`. The corrected helper SHA was `f28b275bf1e3625939eaf3173036fc23d29e5b79442b80166667eced35ee8b7e` for both groups.

Seven retained families ran on the corrected 39-test source baseline `1b2b1b78a028d6dce6eb02ea3e6fe91c4aa90f48593b71dd89e5e4ddbe0b9de8`. The three exact families ran on the refreshed 40-test source baseline `9b8f86e6a771edb14bb0bd0d2d713251f18fdb8daf79c79df6f6913be4a25fdf` (scratch test SHA `05ae3d9bd9ea1a19b6205419e9c1d995ec2f406f1c0c12b89e9baf1bad906192`; reviewed scratch-to-HEAD difference was formatting only). The helper was unchanged between these baselines.

| Required family | Evidence baseline and retained logs |
| --- | --- |
| Omit reservation availability | Earlier seven; `reservation_final_RED.log` / `reservation_final_GREEN.log` |
| Replenish depleted node | Earlier seven; `depleted_node_replenished_RED.log` / `depleted_node_replenished_GREEN.log` |
| Duplicate conversion credit | Earlier seven; `conversion_credited_twice_RED.log` / `conversion_credited_twice_GREEN.log` |
| Duplicate cancellation refund | Earlier seven; `cancellation_refund_credited_twice_RED.log` / `cancellation_refund_credited_twice_GREEN.log` |
| Repeat output spawn | Earlier seven; `output_job_retained_and_spawned_again_RED.log` / `output_job_retained_and_spawned_again_GREEN.log` |
| Ignore transfer capacity | Earlier seven; `transfer_destination_capacity_bypassed_RED.log` / `transfer_destination_capacity_bypassed_GREEN.log` |
| Allow self-Store | Earlier seven; `self_Store_receiver_allowed_RED.log` / `self_Store_receiver_allowed_GREEN.log` |
| Mixed-cost partial commit | Exact three; `final-three-exact.txt:3-13` |
| Refund arithmetic overflow | Exact three; `final-three-exact.txt:15-25` |
| Discard carried items | Exact three; `final-three-exact.txt:27-37` |

The genuine reservation RED excerpt is `/tmp/task8-mutation/reservation_final_RED.log:6-18`:

```text
assertion failed: second_result.outcome.is_err()
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 38 filtered out
```

Its restored GREEN is `/tmp/task8-mutation/reservation_final_GREEN.log:6-9`:

```text
test e6_charge_and_reserve_policies_compete_for_the_same_last_stock ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 38 filtered out
```

The former excerpt at `/tmp/task8-mutation/final-evidence.txt:12-33` was an ineffective single-check run that passed and is not RED evidence.

Full family logs are under `/tmp/task8-mutation/` using the stems above. Source-hash mapping and the baseline distinction are documented in `.superpowers/sdd/2026-10-26-m2-economy/task-8-capture-evidence-rereview-1.md`.

## Exact semantic family closures from final review

The three labels that previously stood in for related faults now have exact, separate final plants. These run against the refreshed 40-test scratch baseline, and each failure reports the violated state directly:

| Required fault | Focused test | Mutated state | Result |
| --- | --- | --- | --- |
| First mixed-cost debit persists after later rejection | `e6_mixed_cost_rejection_keeps_every_endpoint_unchanged` | Debit the first token cost into staged writes, reject the later underfunded timber cost, and commit staged writes on error. Assertion shows token balance `200 → 100`; restored test passes. | Semantic RED / restored GREEN |
| Cancellation refund arithmetic wraps | `e6_running_cancel_refund_rejects_on_balance_u64_overflow` | Valid gather/Convert trace brings slot token balance to `u64::MAX`; mutant uses wrapping addition during the running cancellation refund. Result is `Ok` instead of `RefundDestinationOverflow`; restored test passes. This is separate from inventory-capacity rejection. | Semantic RED / restored GREEN |
| MoveTo interruption loses carried items | `e6_gather_conversion_and_interruptions_conserve_through_delivery` | Mutant clears the carrier inventory while applying MoveTo. Assertion shows cargo `8 → 0`; restored test passes. This is separate from a Store debit fault. | Semantic RED / restored GREEN |

Per-plant mutated and restored hashes, exit codes, and assertion outputs are in `/tmp/task8-mutation/final-three-exact.txt`. The prior capacity and Store debit mutations remain additional evidence and are not counted as substitutes for these exact families.

## Fix 2: gather wakeups after trusted transfers

Sol's `task-8-oracle-rereview-1.md` isolated one remaining stage×cargo dispatch defect. The predictor now dispatches all positive available cargo from `MovingToReceiver` and any `Waiting` reason through receiver selection; empty `NoDeposit` remains terminal; empty `MovingToReceiver` and nonterminal waiting states use the independent deposit-resume path shared with empty `Unloading`.

Two loader-backed regressions cover accepted transfer plus truthful automatic transition and the suppressed no-op counterpart:

| Test | Accounted setup and expected wakeup | Previous oracle RED | Fixed oracle GREEN |
| --- | --- | --- | --- |
| `review_empty_receiver_stage_resumes_after_transfer_removes_cargo` | Full worker cargo, accepted Gather, transfer all 10 cargo from worker 3 to yard 1; next Gather resumes the live deposit with `MovingToNode`. | Truthful result rejected as `unsupported modeled gather stage`; suppressed control is also rejected. | Both controls pass their expectations. |
| `review_stocked_no_deposit_wait_selects_receiver_after_transfer` | Deplete finite nodes to empty `Waiting(NoDeposit)`, transfer 1 timber from yard 1 to worker 3; next Gather selects receiver 1 and enters `MovingToReceiver`. | Truthful result rejected; suppressed unchanged/empty-event operation accepted (`truthful=Err`, `suppressed=Ok`). | Truthful transition passes and suppressed wakeup rejects. |

The exact old-oracle RED and fixed-oracle GREEN logs are preserved under `/tmp/task8-mutation/{empty_receiver_old_oracle_RED,stocked_nodeposit_old_oracle_RED,empty_receiver_fixed_GREEN,stocked_nodeposit_fixed_GREEN}.log`. Final fix2 hashes: accounting helper SHA256 `7a4404b37cfe1f99ed47f22d19f25e1f74cbb8b18a7f6eb68df8aed05099ef66`; accounting tests SHA256 `3fe443deb0902a3af7f4e930123333c44665cc85576806f09a0bd2b596145c40`.

Fix2 verification: `economy_accounting` 42 passed; `economy_gather` 21 passed; `economy_transactions` 5 passed; strict Clippy, formatting, and diff checks passed. Since the stage projection can affect depletion/cargo paths, the depleted-node replenishment and MoveTo cargo-loss plants were rerun: each produced semantic RED and restored GREEN. No other required fault plants were repeated.
