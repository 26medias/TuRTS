//! The formula ratchet (L0-RULES review, finding 2; design section 9, "One copy of each
//! rule"). The charge, refund, damage, progress and placement rules live in `turts_rules`.
//! Nothing stops a later task from copying a formula into `turts_sim`, `turts_server`,
//! `turts_client` or `turts_interaction` in place of a call. This test counts five
//! fingerprints of such arithmetic in the production code of those crates, in each file, and
//! it is red when a count differs from the record: a count that goes up (a new copy), a new
//! file with a count, and a count that goes down. The count of the record is exact. A count that
//! goes down is red too (it names the number to record in `BASELINE`), because the output of a
//! pass is captured and a silent headroom would let a later copy in.
//!
//! The fingerprints (a count for each file, in this order):
//! 1. `u128_div`: a division (`/` or `/=`) whose operands mention `u128`: the wide
//!    multiply-divide of a charge, refund, damage, progress or health rule.
//! 2. `relation_path`: a path `SlotRelation::<variant>`: a copy of the relation match.
//! 3. `dist_sq`: the identifier `distance_squared` or `length_squared`: squared-distance
//!    placement arithmetic.
//! 4. `wide_let_div`: a division whose operands name a binding that is wide: a `let` with the
//!    type `u128` (`let w: u128 = cost.into();`) or with an initializer that mentions `u128`
//!    (`let w = u128::from(cost);`), or a parameter of the type `u128`, in the same function.
//!    A copy such as `w * n / d` has no `u128` inside the division, so fingerprint 1 does not
//!    see it. A division that fingerprint 1 counts is not counted here again.
//! 5. `same_name_fn`: a free function outside `turts_rules` whose name is the name of a public
//!    free function of `turts_rules`, unless its body calls `turts_rules::<path>::<that name>`
//!    (a thin wrapper, such as `power::weapon_enabled`). A copy of a rule in `u64` with
//!    `checked_mul` and `/` carries no `u128` at all; it is caught when it has the name of the
//!    rule that it copies.
//!
//! The scan reads production code only: the files that the no-content scan reads (no `tests`
//! directory, no `#[cfg(test)]` item). Limits: the body of a macro is not parsed; a division
//! of two variables that hold `u128` values is counted only when a `let` or a parameter of the
//! same function says `u128` (not a field, not a call result); a copy under another name that
//! uses `u64` and `checked_mul` only is not counted (review it by eye); a method is not a free
//! function, so fingerprint 5 does not see a method with the name of a rule.

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use syn::visit::Visit;

const NAMES: [&str; 5] = [
    "u128_div",
    "relation_path",
    "dist_sq",
    "wide_let_div",
    "same_name_fn",
];

/// The five counts of one file, in the order of `NAMES`.
type Counts = [u32; 5];

/// The roots that the ratchet reads, from the repository root.
const ROOTS: [&str; 4] = [
    "crates/turts_sim/src",
    "crates/turts_server/src",
    "crates/turts_client/src",
    "crates/turts_interaction/src",
];

/// The recorded counts: (file, `[u128_div, relation_path, dist_sq, wide_let_div, same_name_fn]`).
/// A file that is not in the table has 0 of each. A count that differs from the record, up or
/// down, is red. Lower a number when a copy goes away (the failure text names the number).
/// Raise one only with a reason in the commit message.
const BASELINE: &[(&str, Counts)] = &[
    ("crates/turts_client/src/camera.rs", [0, 0, 1, 0, 0]),
    ("crates/turts_client/src/feedback.rs", [0, 0, 2, 0, 0]),
    // L0-API1: the one mapping of the schema filter to the rule relation, an exhaustive match of
    // five paths. The five copies of the relation match (core_adapter 3, cursor 6, combat 3,
    // run 3: 15 paths) are gone; they call `turts_rules::relation` through this mapping.
    ("crates/turts_client/src/relation.rs", [0, 5, 0, 0, 0]),
    ("crates/turts_interaction/src/camera.rs", [0, 0, 1, 0, 0]),
    ("crates/turts_sim/src/boot_m3.rs", [1, 0, 5, 0, 0]),
    // The range rule `within_weapon_range` (1) and the distance that orders the targets (1). Before
    // the repair of the approach of an Attack order (combat-fixes) the file had 4 copies of the rule.
    ("crates/turts_sim/src/economy/combat.rs", [0, 0, 2, 0, 0]),
    // `wide_let_div` 1: `RateCarry::take_budget` (`numerator / hz`), the per-second budget of a
    // gatherer for one tick. It is a rate rule of the gather loop, not a rule of `turts_rules`;
    // fingerprint 4 found it when it was added (L0-SIM) and the record keeps it.
    ("crates/turts_sim/src/economy/gather.rs", [0, 0, 4, 1, 0]),
    ("crates/turts_sim/src/economy/machine.rs", [1, 0, 0, 0, 0]),
    (
        "crates/turts_sim/src/economy/production.rs",
        [1, 0, 0, 0, 0],
    ),
    ("crates/turts_sim/src/economy/research.rs", [1, 0, 0, 0, 0]),
    (
        "crates/turts_sim/src/economy/transaction.rs",
        [1, 0, 0, 0, 0],
    ),
    ("crates/turts_sim/src/grid.rs", [0, 0, 2, 0, 0]),
    ("crates/turts_sim/src/map.rs", [0, 0, 1, 0, 0]),
    ("crates/turts_sim/src/movement/spread.rs", [0, 0, 2, 0, 0]),
    // L0-API1: the same mapping in the simulation (the client cannot depend on it).
    ("crates/turts_sim/src/rules_adapter.rs", [0, 5, 0, 0, 0]),
];

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

struct Counter<'n> {
    counts: Counts,
    /// The names of the public free functions of `turts_rules`.
    rule_names: &'n BTreeSet<String>,
    /// For each enclosing function: the names of its wide bindings.
    wide: Vec<BTreeSet<String>>,
}

impl<'n> Counter<'n> {
    fn new(rule_names: &'n BTreeSet<String>) -> Self {
        Self {
            counts: [0; 5],
            rule_names,
            wide: Vec::new(),
        }
    }
}

struct MentionsU128(bool);
impl<'a> Visit<'a> for MentionsU128 {
    fn visit_ident(&mut self, ident: &'a syn::Ident) {
        if ident == "u128" {
            self.0 = true;
        }
    }
}

fn mentions_u128(expr: &syn::Expr) -> bool {
    let mut found = MentionsU128(false);
    found.visit_expr(expr);
    found.0
}

/// Collects the wide bindings of one function body: a `let` with the type `u128` or with an
/// initializer that mentions `u128`. It does not enter a nested function item.
#[derive(Default)]
struct WideLets(BTreeSet<String>);
impl<'a> Visit<'a> for WideLets {
    fn visit_item_fn(&mut self, _: &'a syn::ItemFn) {}
    fn visit_local(&mut self, local: &'a syn::Local) {
        let typed_wide = matches!(
            &local.pat,
            syn::Pat::Type(typed) if matches!(&*typed.ty, syn::Type::Path(path) if path.path.is_ident("u128"))
        );
        let init_wide = local
            .init
            .as_ref()
            .is_some_and(|init| mentions_u128(&init.expr));
        if typed_wide || init_wide {
            self.0.extend(bound_names(&local.pat));
        }
        syn::visit::visit_local(self, local);
    }
}

/// The identifiers that a pattern binds.
fn bound_names(pat: &syn::Pat) -> Vec<String> {
    match pat {
        syn::Pat::Ident(ident) => vec![ident.ident.to_string()],
        syn::Pat::Type(typed) => bound_names(&typed.pat),
        syn::Pat::Reference(reference) => bound_names(&reference.pat),
        syn::Pat::Paren(paren) => bound_names(&paren.pat),
        syn::Pat::Tuple(tuple) => tuple.elems.iter().flat_map(bound_names).collect(),
        _ => Vec::new(),
    }
}

/// True when an expression names one of `names`.
struct NamesAny<'s>(&'s BTreeSet<String>, bool);
impl<'a> Visit<'a> for NamesAny<'_> {
    fn visit_ident(&mut self, ident: &'a syn::Ident) {
        if self.0.contains(&ident.to_string()) {
            self.1 = true;
        }
    }
}

/// True when a body calls `turts_rules::<...>::<name>`.
struct CallsRule<'s>(&'s str, bool);
impl<'a> Visit<'a> for CallsRule<'_> {
    fn visit_expr_call(&mut self, call: &'a syn::ExprCall) {
        if let syn::Expr::Path(path) = &*call.func {
            let segments = &path.path.segments;
            if segments.first().is_some_and(|s| s.ident == "turts_rules")
                && segments.last().is_some_and(|s| s.ident == self.0)
            {
                self.1 = true;
            }
        }
        syn::visit::visit_expr_call(self, call);
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

/// The wide bindings of a function: its `u128` parameters and its wide `let`s.
fn wide_names(sig: &syn::Signature, block: &syn::Block) -> BTreeSet<String> {
    let mut found = WideLets::default();
    found.visit_block(block);
    for input in &sig.inputs {
        if let syn::FnArg::Typed(typed) = input
            && matches!(&*typed.ty, syn::Type::Path(path) if path.path.is_ident("u128"))
        {
            found.0.extend(bound_names(&typed.pat));
        }
    }
    found.0
}

impl<'a> Visit<'a> for Counter<'_> {
    fn visit_item(&mut self, item: &'a syn::Item) {
        if support::production_attrs(item_attrs(item)) {
            syn::visit::visit_item(self, item);
        }
    }
    fn visit_item_fn(&mut self, item: &'a syn::ItemFn) {
        // Fingerprint 5: a function with the name of a public rule, and no call of that rule.
        let name = item.sig.ident.to_string();
        if self.rule_names.contains(&name) {
            let mut calls = CallsRule(&name, false);
            calls.visit_block(&item.block);
            if !calls.1 {
                self.counts[4] += 1;
            }
        }
        self.wide.push(wide_names(&item.sig, &item.block));
        syn::visit::visit_item_fn(self, item);
        self.wide.pop();
    }
    fn visit_impl_item_fn(&mut self, item: &'a syn::ImplItemFn) {
        if support::production_attrs(&item.attrs) {
            self.wide.push(wide_names(&item.sig, &item.block));
            syn::visit::visit_impl_item_fn(self, item);
            self.wide.pop();
        }
    }
    fn visit_expr(&mut self, expr: &'a syn::Expr) {
        if support::production_expr(expr) {
            syn::visit::visit_expr(self, expr);
        }
    }
    fn visit_expr_binary(&mut self, binary: &'a syn::ExprBinary) {
        if matches!(binary.op, syn::BinOp::Div(_) | syn::BinOp::DivAssign(_)) {
            if mentions_u128(&binary.left) || mentions_u128(&binary.right) {
                self.counts[0] += 1;
            } else if let Some(wide) = self.wide.last() {
                let mut named = NamesAny(wide, false);
                named.visit_expr(&binary.left);
                named.visit_expr(&binary.right);
                if named.1 {
                    self.counts[3] += 1;
                }
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

/// The names of the public free functions of `turts_rules`: every `pub fn` that is not a method
/// and not in `#[cfg(test)]` code, in the files of `crates/turts_rules/src`.
fn rule_names(repo: &Path) -> BTreeSet<String> {
    struct Names(BTreeSet<String>);
    impl<'a> Visit<'a> for Names {
        fn visit_item(&mut self, item: &'a syn::Item) {
            if support::production_attrs(item_attrs(item)) {
                syn::visit::visit_item(self, item);
            }
        }
        fn visit_item_fn(&mut self, item: &'a syn::ItemFn) {
            if matches!(item.vis, syn::Visibility::Public(_)) {
                self.0.insert(item.sig.ident.to_string());
            }
            // Do not enter the body: a nested function is not public.
        }
        fn visit_impl_item_fn(&mut self, _: &'a syn::ImplItemFn) {}
    }
    let root = repo.join("crates/turts_rules/src");
    let mut names = Names(BTreeSet::new());
    for file in support::production_sources(&[root]).expect("turts_rules sources read") {
        let code = std::fs::read_to_string(&file).expect("source reads");
        names.visit_file(&syn::parse_file(&code).expect("source parses"));
    }
    names.0
}

/// The counts of one source text, with the names of the public rule functions.
fn count_source(code: &str, rules: &BTreeSet<String>) -> Counts {
    let file = syn::parse_file(code).expect("source parses");
    let mut counter = Counter::new(rules);
    counter.visit_file(&file);
    counter.counts
}

/// The counts of every production file under the roots; a file with no hit is left out.
fn count_tree(repo: &Path) -> BTreeMap<String, Counts> {
    let roots: Vec<PathBuf> = ROOTS.iter().map(|root| repo.join(root)).collect();
    let rules = rule_names(repo);
    let mut counts = BTreeMap::new();
    for file in support::production_sources(&roots).expect("production sources read") {
        let code = std::fs::read_to_string(&file).expect("source reads");
        let found = count_source(&code, &rules);
        if found != [0; 5] {
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

fn baseline() -> BTreeMap<String, Counts> {
    BASELINE
        .iter()
        .map(|(file, counts)| ((*file).to_string(), *counts))
        .collect()
}

/// Compare the counts with the record. The comparison is exact: every difference is a
/// violation. A count above the record is a new copy; a count below it must be recorded, so
/// that the record never keeps headroom for a later copy.
fn compare(recorded: &BTreeMap<String, Counts>, now: &BTreeMap<String, Counts>) -> Vec<String> {
    let mut violations = Vec::new();
    let files: BTreeSet<&String> = recorded.keys().chain(now.keys()).collect();
    for file in files {
        let before = recorded.get(file).copied().unwrap_or([0; 5]);
        let after = now.get(file).copied().unwrap_or([0; 5]);
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
                violations.push(format!(
                    "{file}: {name} went down from {} to {}: record {} in BASELINE",
                    before[i], after[i], after[i]
                ));
            }
        }
    }
    violations
}

#[test]
fn formula_copies_do_not_grow_and_the_record_is_exact() {
    let violations = compare(&baseline(), &count_tree(&repo()));
    assert!(
        violations.is_empty(),
        "formula ratchet:\n{}",
        violations.join("\n")
    );
}

/// A fixture set of rule names for the tests of the counter.
fn fixture_rules() -> BTreeSet<String> {
    ["charged_at", "refund_for", "weapon_enabled"]
        .into_iter()
        .map(String::from)
        .collect()
}

#[test]
fn the_counter_finds_each_fingerprint_and_skips_test_code() {
    // Instrument check: each fingerprint is found in production code, and the same code in a
    // `#[cfg(test)]` module is not counted.
    let rules = fixture_rules();
    let count = |code: &str| count_source(code, &rules);
    let wide = "fn f(a: u64, b: u64) -> u64 { (u128::from(a) * u128::from(b) / 3) as u64 }";
    assert_eq!(count(wide), [1, 0, 0, 0, 0]);
    assert_eq!(
        count("fn f(a: u64) -> u64 { let mut x = 7u128; x /= u128::from(a); x as u64 }"),
        [1, 0, 0, 0, 0]
    );
    assert_eq!(count("fn f(a: u64, b: u64) -> u64 { a / b }"), [0; 5]);
    assert_eq!(
        count(&format!("#[cfg(test)] mod tests {{ {wide} }}")),
        [0; 5]
    );
    assert_eq!(
        count(&format!("struct S; impl S {{ #[cfg(test)] {wide} }}")),
        [0; 5]
    );
    let relation = "fn h(r: SlotRelation) -> bool { match r { SlotRelation::Other => true, \
        turts_mod_api::SlotRelation::Same => false, _ => true } }";
    assert_eq!(count(relation), [0, 2, 0, 0, 0]);
    assert_eq!(
        count("use turts_mod_api::SlotRelation; fn k(_: SlotRelation) {}"),
        [0; 5]
    );
    assert_eq!(
        count("fn d(a: V, b: V) -> f32 { a.distance_squared(b) + a.length_squared() }"),
        [0, 0, 2, 0, 0]
    );
}

#[test]
fn a_division_of_let_bound_u128_values_is_counted() {
    // The missed copy of the L0-RULES re-check: no `u128` appears inside the division, because
    // the operands are bindings that a `let` typed `u128`.
    let rules = fixture_rules();
    let typed = "fn charge(cost: u64, n: u64, d: u64) -> u64 { \
        let w: u128 = cost.into(); let n: u128 = n.into(); let d: u128 = d.into(); \
        (w * n / d) as u64 }";
    assert_eq!(count_source(typed, &rules), [0, 0, 0, 1, 0]);
    // An initializer that names `u128`, and a `u128` parameter, bind a wide value too.
    let initialised = "fn charge(cost: u64, n: u128, d: u128) -> u128 { \
        let w = u128::from(cost); w * n / d }";
    assert_eq!(count_source(initialised, &rules), [0, 0, 0, 1, 0]);
    let compound = "fn charge(cost: u64, d: u64) -> u64 { \
        let mut w: u128 = cost.into(); w /= u128::from(d); w as u64 }";
    // `u128` is inside the division: fingerprint 1 counts it, fingerprint 4 does not count it
    // again.
    assert_eq!(count_source(compound, &rules), [1, 0, 0, 0, 0]);
    // A division of `u64` values is not counted, and the same names in another function are
    // not wide there.
    let narrow = "fn a(w: u64, d: u64) -> u64 { w / d } \
        fn b(cost: u64) -> u128 { let w: u128 = cost.into(); w } \
        fn c(w: u64, d: u64) -> u64 { w / d }";
    assert_eq!(count_source(narrow, &rules), [0; 5]);
    // Test code is skipped.
    assert_eq!(
        count_source(&format!("#[cfg(test)] mod tests {{ {typed} }}"), &rules),
        [0; 5]
    );
}

#[test]
fn a_free_function_with_the_name_of_a_rule_is_counted_unless_it_calls_the_rule() {
    let rules = fixture_rules();
    // The missed copy of the L0-RULES re-check: a formula in `u64` with `checked_mul` and `/`.
    // It has the name of the rule that it copies.
    let copy = "fn charged_at(cost: u64, done: u64, total: u64) -> u64 { \
        cost.checked_mul(done).map_or(u64::MAX, |product| product / total) }";
    assert_eq!(count_source(copy, &rules), [0, 0, 0, 0, 1]);
    // A thin wrapper that calls the rule of the same name is the sanctioned form
    // (`power::weapon_enabled`).
    let wrapper = "fn weapon_enabled(world: &World, unit: u32) -> bool { \
        turts_rules::supply::weapon_enabled(None, false) }";
    assert_eq!(count_source(wrapper, &rules), [0; 5]);
    // A wrapper of another rule does not excuse the name.
    let wrong = "fn weapon_enabled(world: &World, unit: u32) -> bool { \
        turts_rules::supply::is_short(0, 0) }";
    assert_eq!(count_source(wrong, &rules), [0, 0, 0, 0, 1]);
    // A function with another name, a method with the name of a rule, and a copy in test code
    // are not counted.
    assert_eq!(count_source("fn charge_now() {}", &rules), [0; 5]);
    assert_eq!(
        count_source("struct S; impl S { fn charged_at(&self) {} }", &rules),
        [0; 5]
    );
    assert_eq!(
        count_source(&format!("#[cfg(test)] mod tests {{ {copy} }}"), &rules),
        [0; 5]
    );
}

#[test]
fn the_rule_names_come_from_the_public_functions_of_turts_rules() {
    let names = rule_names(&repo());
    for expected in [
        "charged_at",
        "refund_for",
        "value_at",
        "weapon_enabled",
        "rate_factor",
        "rate_step",
    ] {
        assert!(names.contains(expected), "{expected} is not in {names:?}");
    }
    // A private helper and a method are not rule names.
    assert!(!names.contains("mul_div_u64"), "{names:?}");
    assert!(!names.contains("numerator"), "{names:?}");
}

#[test]
fn the_comparison_can_go_red() {
    // Instrument check: a count that goes up, a new file with a count, and a count that goes
    // down are each a violation, and an equal record is not.
    let record = BTreeMap::from([
        ("a.rs".to_string(), [2, 0, 0, 0, 0]),
        ("b.rs".to_string(), [1, 0, 0, 0, 0]),
    ]);
    let same = record.clone();
    assert_eq!(compare(&record, &same), Vec::<String>::new());
    let mut up = record.clone();
    up.insert("a.rs".to_string(), [3, 0, 0, 0, 0]);
    let violations = compare(&record, &up);
    assert_eq!(violations.len(), 1);
    assert!(violations[0].starts_with("a.rs: u128_div: the count went up from 2 to 3: call turts_rules::<function> in place of a new copy, or raise the count with a reason in the commit"));
    let mut new_file = record.clone();
    new_file.insert("c.rs".to_string(), [0, 1, 0, 0, 0]);
    let violations = compare(&record, &new_file);
    assert_eq!(violations.len(), 1);
    assert!(
        violations[0].starts_with("c.rs: relation_path: a file with no record has a count of 1")
    );
    let mut new_wide = record.clone();
    new_wide.insert("d.rs".to_string(), [0, 0, 0, 1, 1]);
    assert_eq!(compare(&record, &new_wide).len(), 2);
}

#[test]
fn a_count_that_goes_down_is_red_and_names_the_number_to_record() {
    // The binding note of the L0-RULES re-check: the pass output is captured, so a count that
    // goes down must fail, or the record would keep headroom for a later copy.
    let record = BTreeMap::from([
        ("a.rs".to_string(), [2, 0, 0, 0, 0]),
        ("b.rs".to_string(), [1, 0, 0, 0, 0]),
    ]);
    let mut down = record.clone();
    down.remove("b.rs");
    assert_eq!(
        compare(&record, &down),
        ["b.rs: u128_div went down from 1 to 0: record 0 in BASELINE"]
    );
    let mut lower = record.clone();
    lower.insert("a.rs".to_string(), [1, 0, 0, 0, 0]);
    assert_eq!(
        compare(&record, &lower),
        ["a.rs: u128_div went down from 2 to 1: record 1 in BASELINE"]
    );
}
