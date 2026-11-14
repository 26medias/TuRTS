# P4 environment independent review

Reviewed commit: `e95e628a0af7c665828015e913e4a300e26c96b2`.
Review checkout: `/home/julien/Projects/TuRTS-p4-environment-review` (detached).
Author checkout was not modified. No browser was started. No main merge or push was done.

## Finding: P2 — reject ground values that create non-finite UVs

`crates/turts_mod_runtime/src/models.rs:550` accepts every finite positive `ground.tile_size`.
The new environment validation accepts `ground_margin: 10000.0`.
These checks accept a 64 by 48 map with `tile_size: 1e-35` and that margin.
Both `PackageArchive::content()` and `validate_models()` succeed on a real package with these values.

`crates/turts_client/src/render.rs:386` then builds non-finite mesh UVs.
The actual `setup_scene` mesh contains `[-inf, -inf]` in the independent native test.
The previous 64 by 48 extent divided by this tile size is finite.
Thus the new extension can invalidate a previously representable texture scale.
This violates the contract that the ground keeps its world scale and origin for accepted data.
The shipped skirmish tile size and margin are safe.

Reject a ground tile size when the extended extent or resulting UV range cannot be represented as finite `f32` values.
Return a field error before the client accepts the package.
An additional lower bound is possible, but a derived-range check preserves more valid inputs.
The independent validation test currently demonstrates acceptance. Change its expectation to rejection when retaining it.
The mesh test currently fails. A validator fix must test rejection before building this invalid mesh; do not keep a test that bypasses validation and still expects it to succeed.

Evidence:
- `independent-validation.log`: accepted by both real validators; 1 passed, exit 0.
- `independent-mesh.log`: actual scene generation; fractional-margin phase check passed; finite-UV check failed, Cargo exit 101.
- `independent-tests.patch`: exact independent test sources.

## Code-running checks

All commands ran in the detached review checkout with the existing shared cache.
No compilation ran during the author's GPU measurement interval.

| Command | Result | Evidence |
|---|---|---|
| `/tmp/codex-mp1-cargo test -p turts_mod_runtime --test environment --test lobby --test models` | 18 passed | `runtime.log` |
| `/tmp/codex-mp1-cargo test -p turts_client --lib environment::tests::` | 4 passed | `client.log` |
| `/tmp/codex-mp1-cargo test -p turts_pack --test environment` | 1 passed | `authority.log` |
| `/tmp/codex-mp1-cargo test -p turts_mod_runtime --test environment independent_extreme -- --nocapture` | 1 passed; confirms invalid derived range is accepted | `independent-validation.log` |
| `/tmp/codex-mp1-cargo test -p turts_client --lib environment::tests::independent_ -- --nocapture` | 1 passed, 1 failed; confirmed finding above | `independent-mesh.log` |

The authority test compares exact audited movement and harvesting results after 2000 ticks.
The independent phase test samples the actual mesh at fixed world positions before and after a fractional 3.25-unit margin.
This avoids the shipped integer texture-repeat margin masking an origin error.

I also ran `git diff --check`, `bash -n scripts/web-preview.sh`, Python AST parsing of `scripts/qa/effects/generate_benchmark.py`, and `node --check` on `tests/browser/environment-acceptance.js` and `tests/browser/effects-measure.js`. All returned 0.

## Reviewed behavior

I read the whole feature spec and the environment API document.
I reviewed the additive API 0.2.10 fields, serde defaults, raw version floor, release list, finite scalar and colour validation, root-map acquisition, camera systems, ground generation, preview script, and benchmark changes.
I found no other blocking issue in this review.

Old maps preserve absent environment data and existing camera settings in the native checks.
The verified-root test prevents an earlier extra package map from replacing the accepted game map.
Ground, dimensions, and environment use the same selected map.
The authored environment applies one sample, adds or removes FXAA and fog at runtime, and leaves the accepted map dimensions unchanged.
The boundary uses the original dimensions. The ground remains centred on the original map.
Production movement code and protocol 6 are unchanged.
No additional process records or gates were added.

## Limits and disposition

This is the single independent review. The author can fix the finding and run retained regressions without another review pass.
I did not run a browser, GPU measurements, or the final cold-player probes.
The author owns those checks on the final build.
I did not rerun all historical repository gates.
Native tests prove the tested components and exact simulation comparison. They do not prove final browser appearance or frame rate.
The only review checkout modifications are the independent tests preserved in the patch.
No spec scope change is needed. The numeric validation should be clarified to require finite derived ground UVs.
