VERDICT=pass-with-changes BLOCKING=1 HIGH=0 RUNS=6

Gate 2, review-plan, engine lens (mod-runtime-engineer): T4 and T5 of
`docs/superpowers/plans/2026-10-29-qa0-instrument.md` (P). Commit `bfef807`. What I changed: none.

## Blocking

B1. Two "expected red" tests are compile errors, not reds. The rule at P:36 says that a compile
    error is not a red.
    - T4: P:296 and P:299. The test `released_list_ends_with_engine_version` names
      `RELEASED_ENGINE_APIS`. That constant is created only in step 4 (P:300). So at step 3,
      `cargo test -p turts_mod_api` does not compile. After step 4 the test is green. The test is
      never seen red.
    - T5: P:346 and P:350. The test `t2c_dependencies_parse_then_resolve_refuses` calls
      `resolve(..)`. That function is created only in step 3. So the whole `t2c_package_rejects`
      binary does not compile at step 2, and the A13 red is not seen. T6 has no mutation row for
      A13 (P:392-401), so A13 is never seen red.
    - Fix, using the same "signature first" pattern as T4 step 2 (P:298):
        - T4: add `RELEASED_ENGINE_APIS` with `0.2.2` as the last entry, but keep
          `ENGINE_API_VERSION = "0.2.1"` until step 4. The test then compiles and fails.
        - T5: first add `pub mod resolve;` and a stub `resolve` that returns `Ok(vec![root id])`.
          Then t2c fails at `parse` with "not supported at M1" (`manifest.rs:47`), which is a real
          red. The `resolve.rs` tests also compile, and they fail on the stub. That gives each
          rule a red before T6.

## Non-blocking

M1. P:319 changes `check_package` to a `match` on `kind`. The guard at
    `crates/turts_checks/tests/m2_public_path.rs:36-39` requires a call to `load_package` that
    is written inside the body of `check_package`. The guard uses a syn visitor
    (`m2_public_path.rs:173-197`), so a call in a `match` arm passes. A call through a helper
    function fails the guard. Name this constraint in T5, because the implementer runs
    Sonnet.
M2. P:284 gives the text of the version-table message, but it does not say which file the error
    names. SC4 (P:343) expects `zeta/manifest.ron: engine_api: ...`. So T4 must say that the
    error is `ModError::new(<manifest id>, "engine_api", ..).with_file(MANIFEST_PATH)`.
L1. T1 step 4 (P:122) expects the engine 0.2.1 exits (p1 0, p7 2). T1 and T4 run in the same
    wave. If T1 runs after T4 merges, p1 gives 2. The receipt must record its commit.

## Answers

1. Shared files: T4 writes only `crates/`. T1 writes `docs/qa/`, `tests/qa/` and `scripts/qa/`.
   They share no file. T4 and T5 share `manifest.rs`, `lib.rs`, `main.rs`, `cli.rs`,
   `bad_packages.rs`, `t2c` and `version_table.rs`, but they run one after the other (P:46). OK.
2. The `zeta` texts are correct. Display is `<package>/<file>: <field>: <message>`
   (`error.rs:42-60`). `validate_manifest` errors carry `with_file(MANIFEST_PATH)`
   (`manifest.rs:10-12`). Every edit target exists in `fixtures/zeta/manifest.ron`. The A10, A15
   and SC4 texts follow from these rules. The SC4 admitted version is 0.2.1: it is the highest
   released version below 0.2.2 that `^0.2` matches. A11 is not blocked by `maps/arena.ron`
   staying in the archive: `pack.rs` checks no export list, and `validate_m3_presence`
   (`content_m3_validation.rs:606-650`) reads only the listed maps. The hint probe gives
   `movenment`→`movement` (1), `helth`→`health` (1), `zzzzzzzz`→none. No other allowed name is
   within distance 2.
3. The 0.2.2 bump: every `^0.2.1` (`mods/forge`, `tests/archive.rs:114`, `content_m3.rs`)
   matches 0.2.2. The only assertion on the engine value is `t2c_package_rejects.rs:57`. There
   is no `0.2.1` in JSON, TOML or JS under `crates/`, `tests/` and `scripts/`.
4. Two-entry lockfile: traced, and no test breaks. `client_skips_server_only_packages`
   (`tests/acquire.rs:97-113`) has three distinct ids and calls only `packages_to_fetch`. The
   fake server (`rejected_exit.rs:125-129`) has one `acme` entry with `mod_id` `acme`. Also
   `unit_boot.rs:146` and `t2d_client_hash.rs:62-67` use one entry each.
5. Sonnet sufficiency: yes, after B1, M1 and M2.

## What I ran

| command | claim | refuted if | observed |
| --- | --- | --- | --- |
| python edit-distance probe on the allowed lists | P:283, P:292 hint texts | another name within 2, or none | one name each; `zzzzzzzz` none |
| `rtk proxy grep -rn -F 0.2.1` (rs, ron, js, sh, json, toml, md) | P:301 the existing packages still load | a pin other than `t2c:57` | none |
| `rtk proxy grep -rn "Lockfile \{\|mod_id"` + read the hits | Q4: no two-entry lockfile reaches `check_lockfile` | a same-id or a foreign-`mod_id` lockfile | none |
| read `error.rs:42-60`, zeta, `bad_packages.rs:35-76` | P:339-344 exact stderr | a format mismatch | match (M2 aside) |
| `rtk proxy grep function_has_call` + read | P:319 keeps the public-path guard | a guard on `check_package` | `m2_public_path.rs:36-39` (M1) |
| read `validate_m3_presence` | A11 with an unlisted map file | a check on unlisted files | none |

## NOT TRACED

Whether `tests/archive.rs` has a helper for the SC1 unit edit. The run time of the new tests.
Whether `scan_rust` (`no_content.rs:160`) reads the `#[cfg(test)]` modules in `src/`. The new
code holds no content ids, so the risk is low.

## Re-check (plan revision 1)

VERDICT=pass BLOCKING=0

- B1 closed.
    - T4: step 2 (P:305-306) adds `RELEASED_ENGINE_APIS` with 0.2.2 last and a stub
      `check_version_table`, and keeps the engine at 0.2.1. So `released_list_...` and
      `sc3_..._refuses` compile and fail at step 3 (P:308).
    - T5: step 2 (P:361) adds `pub mod resolve` with a stub. In step 3 (P:362), t2c panics on
      "not supported at M1", and the LR tests fail on the stub. T6 row 22 (P:418) gives A13 a
      mutation red: without `lr3_provided`, nothing else refuses an unprovided dependency.
- M1 closed (P:328). M2 closed (P:290). L1 closed (P:119, P:123). The p1 and p7 exits on 0.2.2
  agree with the T4 code: p7 fails on the Game map rule, because the kind rule is not there before T5.
- New compile-order defect: none. The stubs are not called until the implement step. The only
  result is a `dead_code` warning, which `cargo test` accepts. Clippy `-D warnings` runs only at
  the merge, after the stub is wired.
