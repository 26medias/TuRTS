VERDICT=pass-with-changes BLOCKING=1 HIGH=3 RUNS=8

Gate 1, engine lens (mod-runtime-engineer), packages, manifests, schema. Artefact:
`docs/superpowers/specs/2026-10-29-qa0-instrument-design.md` (D). Commit `f56125d`.
What I changed: none (repository). Scratch files only: `qa0/modrt-probe/`, `qa0/runs-modrt-o2.txt`, this file.

## Blocking

B1. D:341 (section 7, client call site) contradicts S:302 (6.5, Baseline) and the code.
    D:341 says client acquire calls `resolve` "over the locked set". S:302: "The client never
    resolves versions itself ... The client does not download server-only packages." The code
    does the same: `packages_to_fetch` drops `ExecutionContext::Server` entries
    (`crates/turts_client/src/acquire.rs:60-65`). Concrete failure: a game (Shared) that
    depends on a Server-context library (an AI library, D48, MP2 S:819) has a lockfile with two
    entries; the client fetches one; `resolve` over the fetched set fails with L3 "not provided"
    and L8 is undefined on that set. The client also has no dependency graph: `LockEntry` holds
    the seven S:231 fields and no `dependencies` or `kind` (`crates/turts_mod_api/src/lock.rs:15-24`).
    At QA0 no such lockfile exists (the server refuses with L3, K5), so the defect is latent and
    goes live at MP2. Fix: remove the client call site. Replace it with a lockfile check that
    needs no graph: one entry per id (D35, S:64) and the advertised `mod_id` is an entry
    (`acquire.rs:32-56`). Keep `resolve` on the server and in `turts-pack` only. Update D:5
    section 7 list, A15, and risk 10.1 item 5 ("three paths" becomes two plus the lockfile check).

## High (non-blocking)

H1. D:30, D:293 K1 benefit is not true on the real types. Probe `qa0/modrt-probe/src/main.rs`
    builds O2 for the real `UnitDef` fields. Result (`qa0/runs-modrt-o2.txt`): O2 CAN reach
    parity: typo, duplicate (with a seen-set), missing required `movement` (`schema.rs:331-340`),
    missing `name`, typo in a block: all the same error codes as the derive; 20 of 20 unit files in
    `mods/` and `crates/` parse equal; 20 of 20 round trip through `ron::ser::to_string_pretty` (the
    `m2_variants.rs:1560-1563` path). But one new field still needs four edits: the typed struct
    field (consumers such as `m2_variants.rs:1565-1590` and `turts_sim` read typed fields), a match
    arm, a table row, and an entry in a second `&'static [&str]` list (serde `unknown_field` needs
    it). The author probe hides this: it stores values in `BTreeMap<String, Cap>` and adds a `Cap`
    variant and a match arm per field (`qa0/schema-probe/src/main.rs:21-24,48-55`), so "accepted
    with no struct change" (D:30) is not true. The version table works with O1 too (D:292 says so).
    My call: build O1 + version table + did-you-mean hint. Cost: about 1 task, zero hand-written
    deserializers. O2 costs about 2 tasks plus about 120 lines per type that a reviewer must keep in
    sync, for no extra power. K1 stays a CEO call: needs decision, evidence above.

H2. D:316 (S4) tests a shape that no real package can reach, and D:304 is not defined.
    `ENGINE_API_VERSION` is `0.2.1` (`crates/turts_mod_api/src/lib.rs:33`). Probe V lines: `^0.3`
    is refused at `manifest.rs:34` on engine 0.2.1; on engine 0.3.0, `^0.2` and `^0.2.1` are
    refused at `manifest.rs:34` before any field gate runs. So a `since_api` of 0.3.0 is never
    reachable through a package: the refusal comes from the engine check, and "remove the
    `since_api` check" does not turn a package-level S4 red. Only a patch `since` inside the current
    0.x minor is reachable (the M3 case: `content_m3_validation.rs:511-520`, legacy 0.2.0 at :511). Also semver has no
    "admits a version below X" query (D:305); the probe tests one version (patch minus 1).
    Fix: S4 on a package: row `since` 0.2.1, `^0.2` refused, `^0.2.1` accepted (probe shows both).
    Define the query. Name the location: `PackageArchive::content()` next to `validate_m3`
    (`archive.rs:132-133`); the server (`turts_pack/src/lib.rs:14`) and the client
    (`turts_client/src/content.rs:39`) both run it, so the two paths agree.

H3. D:342, D:347, D:393 (A13) break the pack-equals-boot invariant. `turts-pack game --lib ai_core`
    exits 0, but the server boot of the same game fails L3 (K5, D:339-340). The code states the
    invariant: "a package that it packs is a package that boots" (`turts_server/src/boot.rs:4-6`)
    and "checks the package the way a server and a client check it" (`turts_pack/src/main.rs:22-23`).
    Fix: either cut `--lib` (cut line item 3 already allows it) or keep it and change both texts and
    `--help` to say that no server at this engine version serves dependencies. State the choice.

## Non-blocking

N1. D:292 "API minor bump (M3 K4)" is wrong: M3 K4 is a patch bump to 0.2.1
    (`docs/superpowers/specs/2026-10-27-m3-industrial-design.md:30`). A minor bump (0.3.0) refuses
    every `^0.2` manifest in `mods/` (probe V lines). Write "patch bump".
N2. D (whole) is silent on `ENGINE_API_VERSION` and the lockfile hash (grep: only D:283, D:292).
    QA0 makes new manifests valid (Library with zero maps; non-empty `dependencies`). Under the M3
    K4 precedent a package that needs these declares a requirement that excludes 0.2.1. Needs a
    statement: bump to 0.2.2 with a gate, or no bump because no server serves libraries yet.
    Lockfile: no change at QA0 (`LockEntry` unchanged, one entry, `boot.rs:43-46`), so the hash of
    an existing package set does not change. Say so.
N3. D:265-270 K3 is consistent with S:229 (Decided) and S:231, and touches every reader of `kind`
    and `capabilities`: `validate_manifest` runs inside `PackageArchive::parse`
    (`archive.rs:42`), its only call site (grep), so server, client, `turts-pack` and
    `m2_variants` all get the rule. The client never calls `load_package`; a Library root fails on
    the client only by accident ("the packages have no map", `acquire.rs:103`). Record that. Also
    record the coupling: `build_acquired` takes "the first map of the first package that has one"
    (`acquire.rs:83-84,98-100`); with dependencies-first order, a later K3 relaxation (map
    libraries) picks the library map. Any relaxation must change `acquire.rs` too.
N4. D:322 `resolve` returns `Vec<PackageId>`. The MP2 load path needs ordered archives, and
    `HostConfig` has no `deny_unknown_fields` (`turts_server/src/config.rs:65-75`), so a `libraries:`
    key in `server.ron` is silently ignored today. Add `deny_unknown_fields` or note it for MP2.

## Undecided (state the status)

K1 to K6: CEO calls, open. Library execution context with definitions (a Server-context library
with units has visuals that no client fetches, `visuals.rs:61-63`): library format, excluded
(D:58), open. Version scheme: semver is the M1 placeholder (`manifest.rs:8`), open (S 15).

## Lens answers

1. O2 rejects typo, duplicate (only with a seen-set) and missing required; parity with the server
   and client paths holds because both parse through one `parse_ron::<UnitDef>`
   (`archive.rs:61-62,190-207`). `m2_variants` needs derived `Serialize` and typed fields. I build O1.
2. K3 holds (N3). Unknown capability: covered at parse for all readers. Library rule: fine.
3. No: server refuses, `turts-pack --lib` accepts (H3), client resolve is wrong (B1).
4. Design is silent (N2).
5. After QA0 a modder cannot add an unread field: error with the allowed names
   (`parse.rs:20-22,36`) plus the S1 hint. A typo gives a clear error. Consistent with D:287.

## What I ran

| command | claim tested | refuted if | observed |
| --- | --- | --- | --- |
| `cargo run` in `qa0/modrt-probe` (real `turts_mod_api`, ron 0.12.2, serde 1.0.229) | D:293 O2 keeps typo, duplicate, required fields | any E1-E6 differs from derive | all equal; naive O2 accepts duplicate |
| same, golden part | D:315 S3 is reachable for O2 | a unit file differs | 20/20 equal, 20/20 round trip |
| same, V part | D:316 S4 reaches `since_api` 0.3.0 | `^0.3` passes on 0.2.1, or `^0.2.1` passes on 0.3.0 | both refused at the engine check |
| read `qa0/schema-probe/src/main.rs` | D:30 "no struct change" | no per-field code | `Cap` variant and match arm per field |
| `rtk proxy grep -n -E "validate_manifest"` crates | D:76 fix reaches all readers | a second call site | one: `archive.rs:42` |
| `rtk proxy grep` client content path | the gate in `content()` covers the client | client skips `content()` | `content.rs:39` |
| `rtk proxy grep` S:302 vs D:341 | D:341 client resolve is allowed | S has no "never resolves" | S:302 forbids it |
| `rtk proxy grep` D for `ENGINE_API_VERSION`, lockfile hash; M3 K4 wording | D states the version effect | a hit | none; K4 is a patch bump |

Also: `rtk proxy grep` of `mods/` and fixtures for non-empty `capabilities` or `kind: Library`:
none, so the capability refusal breaks no shipped package.

## NOT TRACED

Resolver behaviour (no code). The `turts-pack --lib` argument parser. `turts_sim` reads of
`UnitDef` fields (counted literal sites only). The O2 effect on `Exports` (probe covers `UnitDef`
only). CI time of the probe-equivalent tests. Whether AI libraries (D48) will be Server context:
the spec does not say; B1 holds for any Server-context dependency.

## Re-check (revision 1)

VERDICT=pass-with-changes BLOCKING=1

Closed:
- B1 closed. D:492-498 removes the client `resolve` and puts the lockfile checks in `check_lockfile`
  (one entry per id, advertised `mod_id` present). A16 (D:609-610) can go red: today
  `acquire.rs:52-56` checks only the hash and the parse.
- H1 closed. K1 (D:39) and D:424 recommend O1 plus the version table plus the hint. SC2 (D:460)
  rests on the ron derive (`runs-modrt-o2.txt` E2, E3).
- H2 closed. D:438-447 defines the query over a finite released list and puts the check in
  `content()` after `validate_m3` (`archive.rs:132-133`). SC4 (D:462) is a reachable package case.
- H3 closed. `--lib` is cut (D:489-491).
- Versions closed. D:523-535: 0.2.1 to 0.2.2. My grep of `0.2.1` in crates, mods, scripts and
  tests/browser: the other hits are `^0.2.1` requirements, and they match 0.2.2. The only assertion on
  the engine value is `t2c_package_rejects.rs:57`, as D:531 says. `LockEntry` is unchanged.

New defect (blocking):
- R1. D:489 says that `turts-pack` calls `resolve(root, [])` with no condition. LR7 (D:481) refuses a
  Library root. So the Library path cannot exit 0. A11 (D:595), row 2 of D:405 and SC4 (D:462, which
  expects the `engine_api` field, not LR7) fail on a build that follows D:489. Fix: call `resolve`
  on the Game path only. State what the Library path does with a non-empty `dependencies` value at
  QA0 (LR3, or accept it after LR1 and LR2).

Non-blocking:
- R2. `check_lockfile` has no `mod_id` today (`acquire.rs:52`). Its signature must change, and that
  touches `join.rs:168` and five test calls (`tests/acquire.rs:80,85,94`, `t2d_client_hash.rs:67,81`).
  The plan must list these files.

Runs: grep of D for `resolve(root`, `LR7` and `library path` (claim: the turts-pack call is limited to
Game; refuted if a condition is found; none found). Grep of `check_lockfile(` callers (6 sites). Grep of
`0.2.1` literals (1 assertion).

## Re-check 2 (revision 2)

VERDICT=pass BLOCKING=0

- R1 closed. D:449-453 and D:561-567: `resolve` runs only as step 1 of `load_package`. The Library
  path of `check_package` runs `content()` and `visuals_by_content`, with no `resolve`. SC4 (D:536),
  A11 (D:681-682) and D:474-475 can exit 0. `content()` does not read maps or kind (grep of
  `content.rs` and `content_m3.rs`), so a Library with zero maps passes it.
- R2 closed. D:579-582 lists the six sites. They match my grep of `check_lockfile(`. The
  advertisement is in `acquisition.advertisement` at that point (`join.rs:155`, field `join.rs:76`).
- A12 with the merged kind check: it goes red. With LR7 removed, a Library with zero maps, no
  dependencies and `^0.2.2` passes `resolve`. It passes `content()` (0.2.2 is the lowest released
  version that `^0.2.2` admits). Then `lib.rs:16-23` gives "a package exports exactly one map,
  found 0", which is not the asserted text. The entry is real: `files` is private (`archive.rs:28`).
- New defect: none found. Every existing `load_package` caller (about 120 test sites, `boot.rs:38`,
  `turts_pack/src/lib.rs:43`) passes a Game with `dependencies: {}`. A step-1 `resolve(root, [])`
  passes for all of them.
