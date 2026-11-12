//! The formula ratchet (L0-RULES review, finding 2; design section 9, "One copy of each
//! rule"). The charge, refund, damage, progress and placement rules live in `turts_rules`.
//! Nothing stops a later task from copying a formula into `turts_sim`, `turts_server`,
//! `turts_client` or `turts_interaction` in place of a call. This test counts three
//! fingerprints of such arithmetic in the production code of those crates, in each file, and
//! it is red when a count goes up or when a new file has a count above 0. A count that goes
//! down passes and prints the new number to record in `BASELINE`.
//!
//! The fingerprints (a count for each file, in this order):
//! 1. `u128_div`: a division (`/` or `/=`) whose operands mention `u128`: the wide
//!    multiply-divide of a charge, refund, damage, progress or health rule.
//! 2. `relation_path`: a path `SlotRelation::<variant>`: a copy of the relation match.
//! 3. `dist_sq`: the identifier `distance_squared` or `length_squared`: squared-distance
//!    placement arithmetic.
//!
//! The scan reads production code only: the files that the no-content scan reads (no `tests`
//! directory, no `#[cfg(test)]` item). Limits: the body of a macro is not parsed, and a `/`
//! counts only when `u128` is written inside the expression (a division of two variables that
//! hold `u128` values is not counted).

mod support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use syn::visit::Visit;

const NAMES: [&str; 3] = ["u128_div", "relation_path", "dist_sq"];

/// The roots that the ratchet reads, from the repository root.
const ROOTS: [&str; 4] = [
    "crates/turts_sim/src",
    "crates/turts_server/src",
    "crates/turts_client/src",
    "crates/turts_interaction/src",
];

/// The recorded counts: (file, `u128_div`, `relation_path`, `dist_sq`). A file that is not in
/// the table has 0 of each. Lower a number when a copy goes away. Raise one only with a reason
/// in the commit message.
const BASELINE: &[(&str, u32, u32, u32)] = &[
    ("crates/turts_client/src/camera.rs", 0, 0, 1),
    ("crates/turts_client/src/core_adapter.rs", 0, 3, 0),
    ("crates/turts_client/src/feedback.rs", 0, 0, 2),
    ("crates/turts_client/src/widgets/cursor.rs", 0, 6, 0),
    ("crates/turts_interaction/src/camera.rs", 0, 0, 1),
    ("crates/turts_sim/src/boot_m3.rs", 1, 0, 5),
    ("crates/turts_sim/src/economy/combat.rs", 0, 3, 4),
    ("crates/turts_sim/src/economy/gather.rs", 0, 0, 4),
    ("crates/turts_sim/src/economy/machine.rs", 1, 0, 0),
    ("crates/turts_sim/src/economy/production.rs", 1, 0, 0),
    ("crates/turts_sim/src/economy/research.rs", 1, 0, 0),
    ("crates/turts_sim/src/economy/transaction.rs", 1, 0, 0),
    ("crates/turts_sim/src/grid.rs", 0, 0, 2),
    ("crates/turts_sim/src/map.rs", 0, 0, 1),
    ("crates/turts_sim/src/movement/spread.rs", 0, 0, 2),
    ("crates/turts_sim/src/orders.rs", 0, 3, 1),
];

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[derive(Default)]
struct Counter {
    counts: [u32; 3],
}

struct MentionsU128(bool);
impl<'a> Visit<'a> for MentionsU128 {
    fn visit_ident(&mut self, ident: &'a syn::Ident) {
        if ident == "u128" {
            self.0 = true;
        }
    }
}

fn item_attrs(item: &syn::Item) -> &[syn::Attribute] {
    match item {
        syn::Item::Const(x) => &x.attrs,
        syn::Item::Enum(x) => &x.attrs,
        syn::Item::Fn(x) => &x.attrs,
        syn::Item::Impl(x) => &x.attrs,
        syn::Item::Mod(x) => &x.attrs,
        syn::Item::Static(x) => &x.attrs,
        syn::Item::Struct(x) => &x.attrs,
        syn::Item::Trait(x) => &x.attrs,
        syn::Item::Type(x) => &x.attrs,
        syn::Item::Use(x) => &x.attrs,
        _ => &[],
    }
}

impl<'a> Visit<'a> for Counter {
    fn visit_item(&mut self, item: &'a syn::Item) {
        if support::production_attrs(item_attrs(item)) {
            syn::visit::visit_item(self, item);
        }
    }
    fn visit_impl_item_fn(&mut self, item: &'a syn::ImplItemFn) {
        if support::production_attrs(&item.attrs) {
            syn::visit::visit_impl_item_fn(self, item);
        }
    }
    fn visit_expr(&mut self, expr: &'a syn::Expr) {
        if support::production_expr(expr) {
            syn::visit::visit_expr(self, expr);
        }
    }
    fn visit_expr_binary(&mut self, binary: &'a syn::ExprBinary) {
        if matches!(binary.op, syn::BinOp::Div(_) | syn::BinOp::DivAssign(_)) {
            let mut found = MentionsU128(false);
            found.visit_expr(&binary.left);
            found.visit_expr(&binary.right);
            if found.0 {
                self.counts[0] += 1;
            }
        }
        syn::visit::visit_expr_binary(self, binary);
    }
    fn visit_path(&mut self, path: &'a syn::Path) {
        // `SlotRelation::Other`, `turts_mod_api::SlotRelation::Same`: `SlotRelation` is not the
        // last segment.
        let last = path.segments.len().saturating_sub(1);
        if path
            .segments
            .iter()
            .take(last)
            .any(|s| s.ident == "SlotRelation")
        {
            self.counts[1] += 1;
        }
        syn::visit::visit_path(self, path);
    }
    fn visit_ident(&mut self, ident: &'a syn::Ident) {
        if ident == "distance_squared" || ident == "length_squared" {
            self.counts[2] += 1;
        }
    }
}

/// The three counts of one source text.
fn count_source(code: &str) -> [u32; 3] {
    let file = syn::parse_file(code).expect("source parses");
    let mut counter = Counter::default();
    counter.visit_file(&file);
    counter.counts
}

/// The counts of every production file under the roots; a file with no hit is left out.
fn count_tree(repo: &Path) -> BTreeMap<String, [u32; 3]> {
    let roots: Vec<PathBuf> = ROOTS.iter().map(|root| repo.join(root)).collect();
    let mut counts = BTreeMap::new();
    for file in support::production_sources(&roots).expect("production sources read") {
        let code = std::fs::read_to_string(&file).expect("source reads");
        let found = count_source(&code);
        if found != [0, 0, 0] {
            let relative = file
                .strip_prefix(repo)
                .expect("file is under the repository")
                .to_string_lossy()
                .replace('\\', "/");
            counts.insert(relative, found);
        }
    }
    counts
}

fn baseline() -> BTreeMap<String, [u32; 3]> {
    BASELINE
        .iter()
        .map(|(file, a, b, c)| ((*file).to_string(), [*a, *b, *c]))
        .collect()
}

/// Compare the counts with the record: (violations, improvements).
fn compare(
    recorded: &BTreeMap<String, [u32; 3]>,
    now: &BTreeMap<String, [u32; 3]>,
) -> (Vec<String>, Vec<String>) {
    let mut violations = Vec::new();
    let mut improvements = Vec::new();
    let files: std::collections::BTreeSet<&String> = recorded.keys().chain(now.keys()).collect();
    for file in files {
        let before = recorded.get(file).copied().unwrap_or([0; 3]);
        let after = now.get(file).copied().unwrap_or([0; 3]);
        for (i, name) in NAMES.iter().enumerate() {
            if after[i] > before[i] {
                let what = if recorded.contains_key(file) {
                    format!("the count went up from {} to {}", before[i], after[i])
                } else {
                    format!("a file with no record has a count of {}", after[i])
                };
                violations.push(format!(
                    "{file}: {name}: {what}: call turts_rules::<function> in place of a new copy, or raise the count with a reason in the commit"
                ));
            } else if after[i] < before[i] {
                improvements.push(format!(
                    "{file}: {name} went down from {} to {}: record {} in BASELINE",
                    before[i], after[i], after[i]
                ));
            }
        }
    }
    (violations, improvements)
}

#[test]
fn formula_copies_do_not_grow() {
    let (violations, improvements) = compare(&baseline(), &count_tree(&repo()));
    for line in &improvements {
        println!("formula ratchet: {line}");
    }
    assert!(
        violations.is_empty(),
        "formula ratchet:\n{}",
        violations.join("\n")
    );
}

#[test]
fn the_counter_finds_each_fingerprint_and_skips_test_code() {
    // Instrument check: each fingerprint is found in production code, and the same code in a
    // `#[cfg(test)]` module is not counted.
    let wide = "fn f(a: u64, b: u64) -> u64 { (u128::from(a) * u128::from(b) / 3) as u64 }";
    assert_eq!(count_source(wide), [1, 0, 0]);
    assert_eq!(
        count_source("fn f(a: u64) -> u64 { let mut x = 7u128; x /= u128::from(a); x as u64 }"),
        [1, 0, 0]
    );
    assert_eq!(
        count_source("fn f(a: u64, b: u64) -> u64 { a / b }"),
        [0, 0, 0]
    );
    assert_eq!(
        count_source(&format!("#[cfg(test)] mod tests {{ {wide} }}")),
        [0, 0, 0]
    );
    assert_eq!(
        count_source(&format!("struct S; impl S {{ #[cfg(test)] {wide} }}")),
        [0, 0, 0]
    );
    let relation = "fn h(r: SlotRelation) -> bool { match r { SlotRelation::Other => true, \
        turts_mod_api::SlotRelation::Same => false, _ => true } }";
    assert_eq!(count_source(relation), [0, 2, 0]);
    assert_eq!(
        count_source("use turts_mod_api::SlotRelation; fn k(_: SlotRelation) {}"),
        [0, 0, 0]
    );
    assert_eq!(
        count_source("fn d(a: V, b: V) -> f32 { a.distance_squared(b) + a.length_squared() }"),
        [0, 0, 2]
    );
}

#[test]
fn the_comparison_can_go_red() {
    // Instrument check: a count that goes up, a new file with a count, and a count that goes
    // down.
    let record = BTreeMap::from([
        ("a.rs".to_string(), [2, 0, 0]),
        ("b.rs".to_string(), [1, 0, 0]),
    ]);
    let same = record.clone();
    assert_eq!(compare(&record, &same), (vec![], vec![]));
    let mut up = record.clone();
    up.insert("a.rs".to_string(), [3, 0, 0]);
    let (violations, _) = compare(&record, &up);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].starts_with("a.rs: u128_div: the count went up from 2 to 3: call turts_rules::<function> in place of a new copy, or raise the count with a reason in the commit"));
    let mut new_file = record.clone();
    new_file.insert("c.rs".to_string(), [0, 1, 0]);
    let (violations, _) = compare(&record, &new_file);
    assert_eq!(violations.len(), 1);
    assert!(
        violations[0].starts_with("c.rs: relation_path: a file with no record has a count of 1")
    );
    let mut down = record.clone();
    down.remove("b.rs");
    let (violations, improvements) = compare(&record, &down);
    assert!(violations.is_empty());
    assert_eq!(
        improvements,
        ["b.rs: u128_div went down from 1 to 0: record 0 in BASELINE"]
    );
}
