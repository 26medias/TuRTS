//! Movement writer scan (pre-MP1 task A1b; design section 4 item 4, second bullet).
//!
//! The replay validator found a unit that moves with no order. The ledger audit has no movement
//! model, so this static scan takes that cover: only the files of the pinned table write
//! `Position`, `MoveTarget`, `UnitOrders`, the movement progress (`MoveProgress`,
//! `PlainMoveProgress`) and the movement pools (`MovementPool`, `MovementPools`), and each
//! (entry, kind, type) has the pinned number of writes (A1c: a new writer in an allowed file is
//! red, plant: a `get_mut::<Position>` write in `economy/combat.rs`).
//!
//! A write is the kind of write of the shared visitor (`support/writers.rs`: `&mut T`, `ResMut<T>`
//! `Mut<T>` and `NonSendMut<T>` (arm added in A1d), a method with the turbofish `T` and a writing
//! name, a value of `T` that the code makes). The methods here are `get_mut`, `remove`, `take`,
//! `resource_mut`, `get_resource_mut`, `insert_resource` and `init_resource`.
//!
//! Known gap: a value of `T` that a file receives and inserts with no `&mut T`, no `ResMut<T>`,
//! no turbofish and no constructor is not seen. Code review must catch it.

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use support::writers::{Write, production_writes as scan_production, scan_file as scan_one};

const WRITTEN: [&str; 7] = [
    "Position",
    "MoveTarget",
    "UnitOrders",
    "MoveProgress",
    "PlainMoveProgress",
    "MovementPool",
    "MovementPools",
];
const WRITE_METHODS: [&str; 7] = [
    "get_mut",
    "remove",
    "take",
    "resource_mut",
    "get_resource_mut",
    "insert_resource",
    "init_resource",
];

/// The production writes of movement state, pinned as a multiset. An entry is a file, or a
/// directory when the path ends with `/` (it holds every file below it). For each entry the table
/// gives each kind of write and each type with the number of writes that the scan finds. A new
/// write in an allowed file changes a count, so the test is red. A write in a file with no entry is
/// red too. A task that adds, moves out of the entry, or deletes a write updates the count here
/// and names the line in its report. The text is the reason that the entry holds the writes.
/// (kind, type, count) of one entry of the table.
type Pin = (&'static str, &'static str, usize);

const PINNED: &[(&str, &str, &[Pin])] = &[
    (
        "crates/turts_sim/src/movement/",
        "the movement step and its state",
        &[
            ("construct", "MoveTarget", 1),
            ("construct", "MovementPool", 2),
            ("construct", "PlainMoveProgress", 1),
            ("construct", "Position", 1),
            ("method", "MoveProgress", 1),
            ("method", "MoveTarget", 1),
            ("method", "MovementPools", 1),
            ("method", "PlainMoveProgress", 1),
            ("method", "Position", 1),
        ],
    ),
    (
        "crates/turts_sim/src/orders/",
        "order dispatch writes orders, targets, progress and pools",
        &[
            ("construct", "MoveProgress", 1),
            ("construct", "MoveTarget", 9),
            ("construct", "PlainMoveProgress", 1),
            ("construct", "UnitOrders", 11),
            ("method", "MoveProgress", 20),
            ("method", "MoveTarget", 17),
            ("method", "MovementPools", 1),
            ("method", "PlainMoveProgress", 24),
            ("method", "UnitOrders", 23),
        ],
    ),
    (
        "crates/turts_sim/src/economy/gather.rs",
        "a gather trip sets and clears its target and its progress",
        &[
            ("construct", "MoveTarget", 1),
            ("method", "MoveTarget", 6),
            ("method", "PlainMoveProgress", 1),
        ],
    ),
    (
        "crates/turts_sim/src/economy/transaction.rs",
        "the spawn of a produced unit",
        &[("construct", "Position", 1), ("construct", "UnitOrders", 1)],
    ),
    (
        "crates/turts_sim/src/economy/combat.rs",
        "a target in range stops the approach",
        &[("method", "MoveTarget", 1)],
    ),
    (
        "crates/turts_sim/src/economy/views.rs",
        "the order view compares with the default value, a read",
        &[("construct", "UnitOrders", 1)],
    ),
    (
        "crates/turts_sim/src/bootstrap.rs",
        "the spawn of package placements and nodes at install",
        &[("construct", "Position", 3), ("construct", "UnitOrders", 2)],
    ),
    (
        "crates/turts_sim/src/map.rs",
        "the spawn of map placements",
        &[("construct", "Position", 1), ("construct", "UnitOrders", 1)],
    ),
    (
        "crates/turts_sim/src/command.rs",
        "the legacy Gather and MoveTo commands",
        &[
            ("construct", "MoveTarget", 1),
            ("construct", "UnitOrders", 1),
            ("method", "PlainMoveProgress", 1),
        ],
    ),
    (
        "crates/turts_sim/src/lib.rs",
        "the spawn of diagnostic units",
        &[("construct", "Position", 1), ("construct", "UnitOrders", 1)],
    ),
    (
        "crates/turts_sim/src/plugin.rs",
        "the plugin makes the empty pool resource",
        &[("method", "MovementPools", 1)],
    ),
    (
        "crates/turts_server/src/synthetic.rs",
        "the targets of the --synthetic-units load units",
        &[("construct", "MoveTarget", 2)],
    ),
];

fn scan_file(path: &std::path::Path, label: &str) -> Vec<Write> {
    scan_one(path, label, &WRITTEN, &WRITE_METHODS)
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The production writes of the crates that can write the simulation world.
fn production_writes() -> Vec<Write> {
    scan_production(&repo(), &WRITTEN, &WRITE_METHODS)
}

/// The scan finds each kind of write in the planted fixture, and no read and no test code.
#[test]
fn scan_finds_every_planted_write() {
    let fixture = repo().join("crates/turts_checks/tests/fixtures/movement_writers/planted.rs");
    let writes = scan_file(&fixture, "planted.rs");
    let found: Vec<_> = writes
        .iter()
        .map(|write| (write.line, write.kind, write.type_name.as_str()))
        .collect();
    assert_eq!(
        found,
        vec![
            (5, "construct", "Position"),
            (9, "construct", "MoveTarget"),
            (13, "method", "UnitOrders"),
            (19, "method", "MoveProgress"),
            (22, "&mut", "Position"),
            (29, "method", "MovementPools"),
            (33, "construct", "MoveProgress"),
            (43, "mut-param", "MovementPools"),
            (47, "mut-param", "Position"),
            (51, "mut-param", "MovementPools"),
        ],
        "the scan misses a planted write, or finds a read or test code"
    );
}

type Key = (String, String, String);

/// The entry of the table that holds `file`, or the file itself when no entry holds it.
fn entry_of(file: &str) -> String {
    PINNED
        .iter()
        .find(|(entry, _, _)| {
            if entry.ends_with('/') {
                file.starts_with(entry)
            } else {
                file == *entry
            }
        })
        .map_or_else(|| file.to_owned(), |(entry, _, _)| (*entry).to_owned())
}

/// The multiset of the production writes: (entry, kind, type) and its count.
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

/// The pinned table as a multiset.
fn pinned() -> BTreeMap<Key, usize> {
    let mut pinned = BTreeMap::new();
    for (entry, _why, entries) in PINNED {
        for (kind, type_name, count) in *entries {
            let key = (
                (*entry).to_owned(),
                (*kind).to_owned(),
                (*type_name).to_owned(),
            );
            let first = pinned.insert(key, *count).is_none();
            assert!(
                first,
                "a pinned entry is listed twice: {entry} {kind} {type_name}"
            );
        }
    }
    pinned
}

/// Every production write of movement state is pinned by file, kind, type and count. A new write
/// in an allowed file is red: the count changes.
#[test]
fn movement_state_writers_match_the_pinned_multiset() {
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
                "({:?}, {:?}, {:?}, {found}) pinned {pin}, found {found} at {lines:?}",
                key.0, key.1, key.2
            ));
        }
    }
    assert!(
        differences.is_empty(),
        "movement state writers differ from the pinned table in {}; a new writer is a decision, not an edit of the count:\n{}",
        file!(),
        differences.join("\n")
    );
}
