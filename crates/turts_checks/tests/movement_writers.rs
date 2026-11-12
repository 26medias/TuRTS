//! Movement writer scan (pre-MP1 task A1b; design section 4 item 4, second bullet).
//!
//! The replay validator found a unit that moves with no order. The ledger audit has no movement
//! model, so this static scan takes that cover: only the files of the allow list write
//! `Position`, `MoveTarget`, `UnitOrders`, the movement progress (`MoveProgress`,
//! `PlainMoveProgress`) and the movement pools (`MovementPool`, `MovementPools`).
//!
//! A write is, in production code (the scanner production filter: no `#[cfg(test)]` item, no
//! `tests` directory, no listed test module file):
//! - a `&mut T` type (a query, a parameter, a field);
//! - a method call with the turbofish `T` and a writing name (`get_mut`, `remove`, `take`,
//!   `resource_mut`, `get_resource_mut`, `insert_resource`, `init_resource`);
//! - a value of `T` that the code makes: `T(..)`, `T { .. }` or a call of a `T::` function
//!   (spawn and insert bundles hold such values).
//!
//! Known gap: a value of `T` that a file receives and inserts with no `&mut T`, no turbofish and
//! no constructor is not seen. Code review must catch it.

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use support::{collect, production_attrs, production_expr};
use syn::visit::Visit;

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

/// The files that may write movement state, and why (the lines are the writes of the A1b scan).
/// A path that ends with `/` allows every file below it. Each entry must hold at least one write,
/// so the list stays exact.
const ALLOWED: [(&str, &str); 12] = [
    (
        "crates/turts_sim/src/movement/",
        "the movement step and its state",
    ),
    (
        "crates/turts_sim/src/orders/",
        "order dispatch writes orders, targets, progress and pools",
    ),
    (
        "crates/turts_sim/src/economy/gather.rs",
        "a gather trip sets its target (:199, :602, :816, :1406-1432)",
    ),
    (
        "crates/turts_sim/src/economy/transaction.rs",
        "the spawn of a produced unit (:699, :702)",
    ),
    (
        "crates/turts_sim/src/economy/combat.rs",
        "a target in range stops the approach (:216)",
    ),
    (
        "crates/turts_sim/src/economy/views.rs",
        "the order view compares with the default value, a read (:1130)",
    ),
    (
        "crates/turts_sim/src/bootstrap.rs",
        "the spawn of package placements and nodes at install (:362, :459-472)",
    ),
    (
        "crates/turts_sim/src/map.rs",
        "the spawn of map placements (:163, :166)",
    ),
    (
        "crates/turts_sim/src/command.rs",
        "the legacy Gather and MoveTo commands (:245, :494, :495)",
    ),
    (
        "crates/turts_sim/src/lib.rs",
        "the spawn of diagnostic units (:119, :122)",
    ),
    (
        "crates/turts_sim/src/plugin.rs",
        "the plugin makes the empty pool resource (:77)",
    ),
    (
        "crates/turts_server/src/synthetic.rs",
        "the targets of the --synthetic-units load units (:72, :92)",
    ),
];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Write {
    file: String,
    line: usize,
    kind: &'static str,
    type_name: String,
}

struct Scan<'a> {
    file: &'a str,
    writes: Vec<Write>,
}

impl Scan<'_> {
    fn hit(&mut self, span: proc_macro2::Span, kind: &'static str, type_name: &str) {
        self.writes.push(Write {
            file: self.file.to_owned(),
            line: span.start().line,
            kind,
            type_name: type_name.to_owned(),
        });
    }
}

fn written(path: &syn::Path) -> Option<String> {
    path.segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .find(|name| WRITTEN.contains(&name.as_str()))
}

fn written_type(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(path) => written(&path.path),
        _ => None,
    }
}

impl<'ast> Visit<'ast> for Scan<'_> {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if production_attrs(&node.attrs) {
            syn::visit::visit_item_mod(self, node);
        }
    }

    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        if production_attrs(&node.attrs) {
            syn::visit::visit_item_fn(self, node);
        }
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if production_attrs(&node.attrs) {
            syn::visit::visit_item_impl(self, node);
        }
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if production_attrs(&node.attrs) {
            syn::visit::visit_impl_item_fn(self, node);
        }
    }

    fn visit_type_reference(&mut self, node: &'ast syn::TypeReference) {
        if node.mutability.is_some()
            && let Some(name) = written_type(&node.elem)
        {
            self.hit(node.and_token.span, "&mut", &name);
        }
        syn::visit::visit_type_reference(self, node);
    }

    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        if !production_expr(node) {
            return;
        }
        match node {
            syn::Expr::MethodCall(call)
                if WRITE_METHODS.contains(&call.method.to_string().as_str()) =>
            {
                let turbofish = call.turbofish.iter().flat_map(|fish| fish.args.iter());
                for argument in turbofish {
                    if let syn::GenericArgument::Type(ty) = argument
                        && let Some(name) = written_type(ty)
                    {
                        self.hit(call.method.span(), "method", &name);
                    }
                }
            }
            syn::Expr::Call(call) => {
                if let syn::Expr::Path(path) = call.func.as_ref()
                    && let Some(name) = written(&path.path)
                {
                    self.hit(path.path.segments[0].ident.span(), "construct", &name);
                }
            }
            syn::Expr::Struct(literal) => {
                if let Some(name) = written(&literal.path) {
                    self.hit(literal.path.segments[0].ident.span(), "construct", &name);
                }
            }
            _ => {}
        }
        syn::visit::visit_expr(self, node);
    }
}

fn scan_file(path: &Path, label: &str) -> Vec<Write> {
    let source = std::fs::read_to_string(path).expect("read a scanned file");
    let file = syn::parse_file(&source).expect("parse a scanned file");
    let mut scan = Scan {
        file: label,
        writes: Vec::new(),
    };
    scan.visit_file(&file);
    scan.writes
}

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The production writes of the crates that can write the simulation world.
fn production_writes() -> Vec<Write> {
    let repo = repo();
    let mut files = Vec::new();
    let mut skipped = Vec::new();
    for root in [
        "crates/turts_sim/src",
        "crates/turts_server/src",
        "crates/turts_pack/src",
    ] {
        collect(&repo.join(root), &mut files, &mut skipped).expect("collect the source files");
    }
    files.sort();
    let mut writes = Vec::new();
    for file in files {
        let label = file
            .strip_prefix(&repo)
            .expect("a scanned file is in the repository")
            .to_string_lossy()
            .replace('\\', "/");
        let label = label.trim_start_matches("./").to_owned();
        writes.extend(scan_file(&file, &label));
    }
    writes
}

fn allowed(file: &str) -> Option<&'static str> {
    ALLOWED
        .iter()
        .find(|(entry, _)| {
            if entry.ends_with('/') {
                file.starts_with(entry)
            } else {
                file == *entry
            }
        })
        .map(|(entry, _)| *entry)
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
        ],
        "the scan misses a planted write, or finds a read or test code"
    );
}

/// Every production write of movement state is in a file of the allow list, and each entry of
/// the list holds a write.
#[test]
fn movement_state_writers_are_in_the_allow_list() {
    let writes = production_writes();
    let outside: Vec<_> = writes
        .iter()
        .filter(|write| allowed(&write.file).is_none())
        .collect();
    assert!(
        outside.is_empty(),
        "movement state is written outside the allow list: {outside:#?}"
    );
    let mut used: BTreeMap<&str, BTreeSet<usize>> = BTreeMap::new();
    for write in &writes {
        if let Some(entry) = allowed(&write.file) {
            used.entry(entry).or_default().insert(write.line);
        }
    }
    for (entry, why) in ALLOWED {
        assert!(
            used.contains_key(entry),
            "the allow list entry {entry} ({why}) holds no write; remove it"
        );
    }
}
