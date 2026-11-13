//! Run-constant writer scan (pre-MP1 slice, task A1d; the A1d ruling, review finding M1).
//!
//! The economy log does not hold four `AuditState` fields: `bounds`, `resources`,
//! `compiled_durations`, `unit_types` (`turts_sim/src/economy/ledger_view.rs`,
//! `LEDGER_DROPPED_FIELDS`). They must not change in a run. The test `assert_run_constants`
//! (`turts_server/tests/economy.rs`) reads them from a live world, but only in sim-code tests. A
//! server system that changed one of them was seen by no default test (review plant SRV-B: a
//! `get_resource_mut::<MapBounds>()` in `admit_player_commands`). So this static scan takes that
//! cover for the production code of `turts_sim`, `turts_server` and `turts_pack`:
//! - the resources that hold the fields are `MapBounds` (`bounds`), `ProductionDurations`
//!   (`compiled_durations`) and `UnitTypes` (`unit_types`). Each write of them is a hit of the
//!   shared visitor (`support/writers.rs`): `&mut T`, `ResMut<T>`, `Mut<T>`, `NonSendMut<T>`, a
//!   method with the turbofish `T` and a writing name, a value of `T` that the code makes;
//! - the presence of a resource (`AuditResourcePresence`, `economy/views.rs`) is a field too, so a
//!   `remove_resource::<T>()` of any type of that struct is a hit (kind `remove_resource`). A test
//!   reads `views.rs` and keeps the type list equal to the struct.
//!
//! Each (entry, kind, type) has the pinned number of writes, with a reason. A new write is a
//! decision, not an edit of a count: it needs the sentence that says why the field is still
//! constant in a run (it is set at boot, or it is restored by a rollback).
//!
//! Methods beyond the `WRITE_METHODS` of `movement_writers.rs`: `remove_resource` (A1d ruling),
//! `resource_scope`, `get_resource_or_init` and `get_resource_or_insert_with` (added here: each
//! gives a `Mut<T>` of a resource; the last two are in `movement_writers.rs` too).
//!
//! Known gap: a value that a file receives and inserts with no `&mut T`, no `ResMut<T>`, no
//! turbofish and no constructor is not seen. Code review must catch it. Example: the three
//! `world.insert_resource(staged.types)`, `(staged.bounds)` and `(staged.durations)` of
//! `install_package` (`turts_sim/src/bootstrap.rs`) are seen only through the constructors in the
//! staging of the same file.
//!
//! Not pinned, and checked: the `remove_resource` calls of the rollback of a failed transaction
//! (`economy/transaction.rs`) and the `insert_resource` of the boot tests (`boot_m3.rs`) are in
//! `#[cfg(test)]` modules, so the scan does not read them (the scan found 0 there).

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use support::writers::{Write, production_writes as scan_production, scan_file as scan_one};

/// The resources that hold the four fields that the log does not hold (`bounds`,
/// `compiled_durations`, `unit_types`).
const RUN_CONSTANTS: [&str; 3] = ["MapBounds", "ProductionDurations", "UnitTypes"];
const WRITE_METHODS: [&str; 11] = [
    "get_mut",
    "remove",
    "take",
    "resource_mut",
    "get_resource_mut",
    "insert_resource",
    "init_resource",
    "resource_scope",
    "remove_resource",
    "get_resource_or_init",
    "get_resource_or_insert_with",
];
/// The type of each field of `AuditResourcePresence` (`economy/views.rs`): a resource that can
/// be present or absent. A `remove_resource::<T>()` of one of them changes the `resources` field.
const PRESENCE: [&str; 14] = [
    "MapBounds",
    "InstalledCatalog",
    "ProductionDurations",
    "EconomyLedger",
    "SurfaceNodes",
    "EconomyOrders",
    "ProductionQueues",
    "UnitTypes",
    "Slots",
    "SimClock",
    "IdAllocators",
    "Tick",
    "PendingEconomyEvents",
    "NextEventOrdinal",
];

/// (kind, type, count) of one entry of the table.
type Pin = (&'static str, &'static str, usize);

/// The production writes of the run constants, pinned as a multiset. An entry is a file. A write
/// in a file with no entry is red. A changed count is red.
const PINNED: &[(&str, &str, &[Pin])] = &[
    (
        "crates/turts_sim/src/map.rs",
        "`load_map` (direct map loading) inserts the map bounds at boot, before the first tick",
        &[("construct", "MapBounds", 1)],
    ),
    (
        "crates/turts_sim/src/bootstrap.rs",
        "the package staging builds the map bounds, the duration table and the type registry (`UnitTypes::from_catalog`); `install_package` inserts them at boot, before the first tick",
        &[
            ("construct", "MapBounds", 1),
            ("construct", "ProductionDurations", 1),
            ("construct", "UnitTypes", 1),
        ],
    ),
    (
        "crates/turts_sim/src/plugin.rs",
        "the plugin makes the empty type registry at app build (`init_resource`)",
        &[("method", "UnitTypes", 1)],
    ),
];

type Key = (String, String, String);

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The scan of a file: the writes of the run constants and the `remove_resource` of the types of
/// `AuditResourcePresence` (kind `remove_resource`).
fn scan_file(path: &Path, label: &str) -> Vec<Write> {
    let mut writes = scan_one(path, label, &RUN_CONSTANTS, &WRITE_METHODS);
    writes.extend(presence_removals(scan_one(
        path,
        label,
        &PRESENCE,
        &["remove_resource"],
    )));
    writes.sort();
    writes
}

/// The `remove_resource::<T>()` hits of the second scan, as kind `remove_resource`. A run constant
/// is in the first scan already (kind `method`), and the second scan sees the other kinds of hit
/// too, so only the method hits of the other presence types stay.
fn presence_removals(hits: Vec<Write>) -> Vec<Write> {
    hits.into_iter()
        .filter(|write| {
            write.kind == "method" && !RUN_CONSTANTS.contains(&write.type_name.as_str())
        })
        .map(|mut write| {
            write.kind = "remove_resource";
            write
        })
        .collect()
}

fn production_writes() -> Vec<Write> {
    let repo = repo();
    let mut writes = scan_production(&repo, &RUN_CONSTANTS, &WRITE_METHODS);
    writes.extend(presence_removals(scan_production(
        &repo,
        &PRESENCE,
        &["remove_resource"],
    )));
    writes.sort();
    writes
}

/// The scan finds each kind of write in the planted fixture, and no read and no test code.
#[test]
fn scan_finds_every_planted_write() {
    let fixture = repo().join("crates/turts_checks/tests/fixtures/run_constant_writers/planted.rs");
    let found: Vec<_> = scan_file(&fixture, "planted.rs")
        .iter()
        .map(|write| (write.line, write.kind, write.type_name.clone()))
        .collect();
    let expected: Vec<(usize, &str, String)> = vec![
        (5, "method", "MapBounds".into()),
        (10, "mut-param", "MapBounds".into()),
        (15, "method", "ProductionDurations".into()),
        (20, "mut-param", "UnitTypes".into()),
        (25, "method", "UnitTypes".into()),
        (29, "remove_resource", "SimClock".into()),
        (33, "construct", "MapBounds".into()),
        (43, "method", "UnitTypes".into()),
        (47, "construct", "MapBounds".into()),
        (47, "method", "MapBounds".into()),
        (50, "&mut", "MapBounds".into()),
    ];
    assert_eq!(
        found, expected,
        "the scan misses a planted write, or finds a read or test code"
    );
}

/// The type list `PRESENCE` is the type list of `AuditResourcePresence`.
#[test]
fn presence_types_are_the_fields_of_audit_resource_presence() {
    let views = std::fs::read_to_string(repo().join("crates/turts_sim/src/economy/views.rs"))
        .expect("read views.rs");
    let start = views
        .find("let resources = AuditResourcePresence {")
        .expect("the presence struct is built in views.rs");
    let block = &views[start..start + views[start..].find("};").expect("end of the block")];
    let mut types = BTreeSet::new();
    for part in block.split("::<").skip(1) {
        let name = part.split('>').next().unwrap().rsplit("::").next().unwrap();
        types.insert(name.to_owned());
    }
    // `catalog` is read through a local (`catalog_present`): its type is the installed catalog.
    types.insert("InstalledCatalog".to_owned());
    assert_eq!(
        types,
        PRESENCE.iter().map(|name| (*name).to_owned()).collect(),
        "PRESENCE differs from the fields of AuditResourcePresence in views.rs"
    );
}

fn entry_of(file: &str) -> String {
    PINNED
        .iter()
        .find(|(entry, _, _)| file == *entry)
        .map_or_else(|| file.to_owned(), |(entry, _, _)| (*entry).to_owned())
}

fn counts(writes: &[Write]) -> BTreeMap<Key, usize> {
    let mut counts = BTreeMap::new();
    for write in writes {
        let key = (
            entry_of(&write.file),
            write.kind.to_owned(),
            write.type_name.clone(),
        );
        *counts.entry(key).or_default() += 1;
    }
    counts
}

fn pinned() -> BTreeMap<Key, usize> {
    let mut pinned = BTreeMap::new();
    for (entry, why, entries) in PINNED {
        assert!(!why.is_empty(), "a pinned entry needs a reason: {entry}");
        for (kind, type_name, count) in *entries {
            let key = (
                (*entry).to_owned(),
                (*kind).to_owned(),
                (*type_name).to_owned(),
            );
            assert!(
                pinned.insert(key, *count).is_none(),
                "a pinned entry is listed twice: {entry} {kind} {type_name}"
            );
        }
    }
    pinned
}

/// Every production write of a run constant is pinned by file, kind, type and count.
#[test]
fn run_constant_writers_match_the_pinned_multiset() {
    let writes = production_writes();
    let actual = counts(&writes);
    let expected = pinned();
    let mut differences = Vec::new();
    for key in actual
        .keys()
        .chain(expected.keys())
        .collect::<BTreeSet<_>>()
    {
        let found = actual.get(key).copied().unwrap_or(0);
        let pin = expected.get(key).copied().unwrap_or(0);
        if found != pin {
            let lines: Vec<String> = writes
                .iter()
                .filter(|write| {
                    entry_of(&write.file) == key.0
                        && write.kind == key.1
                        && write.type_name == key.2
                })
                .map(|write| format!("{}:{}", write.file, write.line))
                .collect();
            differences.push(format!(
                "({:?}, {:?}, {:?}) pinned {pin}, found {found} at {lines:?}",
                key.0, key.1, key.2
            ));
        }
    }
    assert!(
        differences.is_empty(),
        "run-constant writers differ from the pinned table in {}; a new writer is a decision, not an edit of the count:\n{}",
        file!(),
        differences.join("\n")
    );
}
