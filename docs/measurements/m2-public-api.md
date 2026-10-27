# M2 public authoring and economy evidence

Baseline: `1f004e6ad363a5f94f47cb52e4bf98ce5c95aa08`. Task14 source remains isolated in `codex/m2-public-proof`. The Task14 data path is generated files → `pack_checked` → `PackageArchive::parse` → public `load_package`; tests do not inject a native world or catalog. Independently authored Cedar remains exactly ten companion files.

## Focused checks

| Command | Result |
| --- | --- |
| `CARGO_TARGET_DIR=/home/julien/Projects/TuRTS/target/m2-public-proof CARGO_BUILD_JOBS=2 cargo test -p turts_pack --test economy_accounting -- --test-threads=1` | 42 passed |
| `CARGO_TARGET_DIR=/home/julien/Projects/TuRTS/target/m2-public-proof CARGO_BUILD_JOBS=2 cargo test -p turts_pack --test economy_variants -- --test-threads=1` | 6 passed |
| `CARGO_TARGET_DIR=/home/julien/Projects/TuRTS/target/m2-public-proof CARGO_BUILD_JOBS=2 cargo test -p turts_checks --test no_content` | 4 passed |
| `CARGO_TARGET_DIR=/home/julien/Projects/TuRTS/target/m2-public-proof CARGO_BUILD_JOBS=2 cargo test -p turts_checks --test m2_public_path` | 2 passed |
| `python3 tests/m2/test_variants.py` | 26 passed |
| `bash scripts/m2-refutations.sh --case all` | 18 semantic RED cases; all source hashes restored and selected tests GREEN |

The variant test exercises `{base, haul, cedar} × {balance, physical} × {charge, reserve}`. Each combination loads through the public package path, validates declarations, and runs the independent accounting oracle on command, automatic operation, and tick boundaries. Each trace must positively extract, deliver, charge, refund, cancel, and complete before any equality assertion. Balance variants require Balance and Item declarations, SlotBalance costs, and a positive Convert sink. Physical variants require item costs, ProducerInventory costs, Store sinks, and no funded or reserved balance accounts. Unused zero-valued balance declarations retained by a source package do not count as funded accounts.

For all six physical combinations, the test adds an empty second depot by changing the public map file. Production at B fails with no state change. The test then performs `inventory_transfer` from A to B and observes it as `ExpectedOperation::InventoryTransfer`; the exact recipe quantity moves, A remains at its post-transfer stock, and B alone consumes the local cost when production starts. The rejected command and transfer both pass through the independent oracle.

The rename comparison only compares a package with its pure renamed form. It inversely maps package/local/full content IDs, names, paths, and dense unit type handles resolved through each frozen type table. It retains game unit, node, and job IDs; allocator cursors; ticks; event ordinals; stage; amounts; owners; positions; outcomes; and all other fields. E2 and E3 refutations below make this equality and policy binding fail when their implementation is broken.

The public E10 check corrupts Cedar's worker inventory reference and requires the loader error to identify `units/worker.ron` and `inventory.accepts[0]`. The separate Acme check proves its economy remains empty. A source scan also proves the mod directory contains no Rust or Cargo files and that the package API calls `install_package` through the public loader.

## Mutation evidence

The harness copies the repository to `/home/julien/Projects/TuRTS/target/m2-public-proof/refutation-scratch`, uses the separate `refutation-build` target, requires an exact-once anchor, a selected test count above zero, a semantic assertion in the nonzero test result, and a restored source hash plus GREEN rerun. Full command: `bash scripts/m2-refutations.sh --case all`.

| Case | Planted semantic failure | Restored source SHA-256 |
| --- | --- | --- |
| E1-derived-scan | inline comment must not hide code | `efc62ccdf5c04c6f5d292fca5421ec2dfbdd1d0d0b93d5767dce6cc356f1550d` |
| E2-rename-canonicalizer | base pure rename trace equality assertion | `247091d95bac819cfcb9310ef6a1d2d565e3d2198bedde81cea690d2eb707ccf` |
| E3-policy-binding | policy becomes ChargeOnQueue | `54eec4bfb44e4dcfc1290965287e686afcfb8c52fce7dc696292800f380fa3c0` |
| E4-local-capacity | transfer rejection assertion | `0a2ea70eb6d6f62541ad649a5ad216eb32a3838976c136346d047eafe98c5c61` |
| E5-conversion-cargo-separation | independent gather transition projection | `50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160` |
| E6-reservation-omission | requested recipe/ledger projection | `2f6188e51826cbdd3a596e6a958fed6630713a6b5895b4f8ad822cbd6e22f8ee` |
| E6-mixed-cost-partial-commit | partial debit rollback assertion | `0a2ea70eb6d6f62541ad649a5ad216eb32a3838976c136346d047eafe98c5c61` |
| E6-node-replenishment | independent gather projection | `50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160` |
| E6-arithmetic-overflow | refund overflow rejection | `0a2ea70eb6d6f62541ad649a5ad216eb32a3838976c136346d047eafe98c5c61` |
| E6-carried-item-loss | interrupted move preserves carried cargo | `82816bc71257d9d680d1d9b7ec8ae90df1bef66ebb3c57389681001c2b163d` |
| E6-duplicate-conversion | independent gather transition projection | `50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160` |
| E6-duplicate-refund | refund destination delta from charged lines | `0a2ea70eb6d6f62541ad649a5ad216eb32a3838976c136346d047eafe98c5c61` |
| E6-repeated-spawn | exact frozen recipe and whole-state completion | `0a2ea70eb6d6f62541ad649a5ad216eb32a3838976c136346d047eafe98c5c61` |
| E6-capacity-bypass | transfer capacity rejection assertion | `0a2ea70eb6d6f62541ad649a5ad216eb32a3838976c136346d047eafe98c5c61` |
| E6-self-store | independent gather transition projection | `50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160` |
| E6-no-op-economy | independently predicted gather owner/endpoints/item/amount | `50607582f93b334421b2e1d5b29f8694895a555418c4d56535925cb79d5e9160` |
| E6-event-only-spawn | exact frozen recipe and whole-state completion | `0a2ea70eb6d6f62541ad649a5ad216eb32a3838976c136346d047eafe98c5c61` |
| E10-public-error-provenance | Cedar package/file/field diagnostic | `b294f64e4a82bc85135fa260fd2492ae0430790c6e33d4ba2daea932bdca711d` |

E8 transport mutation and E9 UI stale-state mutation depend on the real transport and UI paths scheduled for Tasks 15 and 16. They are intentionally deferred; no stand-in plant is registered here. The suite does not run workspace-wide, wasm, server, or browser gates; root owns those serialized gates.

## Scoped helper change

`crates/turts_pack/tests/common/accounting.rs` changes `advance_accounted_tick` to return the `OperationAudit` records and `TickAudit` after its existing oracle observations pass. This lets the trace serialize the already-validated operations. It does not change the oracle's expected operations, modeled state, validation order, or acceptance conditions. The existing 42-test accounting suite and six variant tests pass with this signature.
