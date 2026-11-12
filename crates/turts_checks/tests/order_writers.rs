//! L0-WIRE: the order writer scan (MP1 ruling R19, design 6.8) and the export-writer scan.
//!
//! 1. The only production write of `UnitOrders` outside `crates/turts_sim/src/orders/writer.rs`
//!    is none. A syn scan sees names only. It looks for: `get_mut::<UnitOrders>()`,
//!    `remove::<UnitOrders>()`, an `insert` or `spawn` of a `UnitOrders` value (a struct literal,
//!    a path call such as `UnitOrders::default()`, or a binding of that type), a field write or a
//!    mutating method on a `UnitOrders` binding, and a helper that takes `&mut UnitOrders` or
//!    `&mut Order`. It cannot see a write through an inferred `get_mut()`, an `insert(var)` of an
//!    inferred type, or a removal that must send `orders_cleared`: the `test-probes` check
//!    `check_order_changes` (`crates/turts_sim/src/orders/writer.rs`) covers those.
//! 2. The export writers `crates/turts_server/src/export_*.rs` read only `CommittedTick` fields:
//!    no `Query`, no `World`, no resource access (regate-performance M5, rigour L3).
//!
//! Each scan has fixtures that must be red: a scan that finds nothing in a fixture is a failure.

mod support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use syn::visit::Visit;
use syn::{Expr, Pat, Type};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const WRITER_FILE: &str = "crates/turts_sim/src/orders/writer.rs";

fn last_segment(path: &syn::Path) -> String {
    path.segments
        .last()
        .map(|segment| segment.ident.to_string())
        .unwrap_or_default()
}

fn names_unit_orders(path: &syn::Path) -> bool {
    path.segments
        .iter()
        .any(|segment| segment.ident == "UnitOrders")
}

fn type_names_unit_orders_mut(ty: &Type) -> bool {
    match ty {
        Type::Reference(reference) => {
            reference.mutability.is_some() && type_is_orders_type(&reference.elem)
                || type_names_unit_orders_mut(&reference.elem)
        }
        Type::Path(path) => path.path.segments.iter().any(|segment| {
            // `Mut<UnitOrders>`, `Query<&mut UnitOrders>`, `ResMut<..>` of the type.
            let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
                return false;
            };
            args.args.iter().any(|arg| match arg {
                syn::GenericArgument::Type(inner) => {
                    (segment.ident == "Mut" && type_is_orders_type(inner))
                        || type_names_unit_orders_mut(inner)
                }
                _ => false,
            })
        }),
        Type::Tuple(tuple) => tuple.elems.iter().any(type_names_unit_orders_mut),
        _ => false,
    }
}

/// `UnitOrders`, or a reference to it. A plain `Order` is not a binding of the component.
fn type_is_unit_orders(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => last_segment(&path.path) == "UnitOrders",
        Type::Reference(reference) => type_is_unit_orders(&reference.elem),
        Type::Paren(paren) => type_is_unit_orders(&paren.elem),
        _ => false,
    }
}

fn type_is_orders_type(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => {
            let last = last_segment(&path.path);
            last == "UnitOrders" || last == "Order"
        }
        Type::Paren(paren) => type_is_orders_type(&paren.elem),
        _ => false,
    }
}

/// True when the expression builds a `UnitOrders` value.
fn builds_unit_orders(expr: &Expr, bindings: &BTreeSet<String>) -> bool {
    match expr {
        Expr::Struct(literal) => names_unit_orders(&literal.path),
        Expr::Call(call) => {
            matches!(&*call.func, Expr::Path(path) if names_unit_orders(&path.path))
        }
        Expr::Path(path) => path
            .path
            .get_ident()
            .is_some_and(|ident| bindings.contains(&ident.to_string())),
        Expr::Tuple(tuple) => tuple
            .elems
            .iter()
            .any(|element| builds_unit_orders(element, bindings)),
        Expr::Paren(paren) => builds_unit_orders(&paren.expr, bindings),
        Expr::Reference(reference) => builds_unit_orders(&reference.expr, bindings),
        Expr::MethodCall(call) => {
            call.method == "clone" && builds_unit_orders(&call.receiver, bindings)
        }
        _ => false,
    }
}

/// The root identifier of a field or index chain: `orders.queue[0].x` gives `orders`.
fn root_ident(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Path(path) => path.path.get_ident().map(ToString::to_string),
        Expr::Field(field) => root_ident(&field.base),
        Expr::Index(index) => root_ident(&index.expr),
        Expr::Unary(unary) => root_ident(&unary.expr),
        Expr::Paren(paren) => root_ident(&paren.expr),
        Expr::MethodCall(call) => root_ident(&call.receiver),
        _ => None,
    }
}

#[derive(Default)]
struct WriterScan {
    hits: Vec<String>,
    /// Local names bound to a `UnitOrders` value or to a mutable reference to one.
    bindings: BTreeSet<String>,
}

impl WriterScan {
    fn hit(&mut self, what: impl Into<String>) {
        self.hits.push(what.into());
    }

    fn turbofish_names_unit_orders(
        &self,
        args: &Option<syn::AngleBracketedGenericArguments>,
    ) -> bool {
        args.as_ref().is_some_and(|args| {
            args.args.iter().any(|arg| {
                matches!(arg, syn::GenericArgument::Type(Type::Path(path)) if names_unit_orders(&path.path))
            })
        })
    }

    fn bind(&mut self, pat: &Pat, ty: Option<&Type>, init: Option<&Expr>) {
        let Pat::Ident(ident) = pat else { return };
        let name = ident.ident.to_string();
        let typed = ty.is_some_and(type_is_unit_orders);
        let built = init.is_some_and(|expr| {
            builds_unit_orders(expr, &self.bindings) || init_gets_unit_orders(expr)
        });
        if typed || built {
            self.bindings.insert(name);
        }
    }
}

/// `x.get_mut::<UnitOrders>()` anywhere in the initialiser (also behind `.unwrap()`).
fn init_gets_unit_orders(expr: &Expr) -> bool {
    match expr {
        Expr::MethodCall(call) => {
            (call.method == "get_mut"
                && call.turbofish.as_ref().is_some_and(|args| {
                    args.args.iter().any(|arg| {
                        matches!(arg, syn::GenericArgument::Type(Type::Path(path)) if names_unit_orders(&path.path))
                    })
                }))
                || init_gets_unit_orders(&call.receiver)
        }
        Expr::Paren(paren) => init_gets_unit_orders(&paren.expr),
        _ => false,
    }
}

impl<'ast> Visit<'ast> for WriterScan {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let attrs = match item {
            syn::Item::Fn(x) => &x.attrs,
            syn::Item::Mod(x) => &x.attrs,
            syn::Item::Impl(x) => &x.attrs,
            syn::Item::Const(x) => &x.attrs,
            syn::Item::Static(x) => &x.attrs,
            _ => &[][..],
        };
        if support::production_attrs(attrs) {
            syn::visit::visit_item(self, item);
        }
    }
    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        if support::production_attrs(&item.attrs) {
            syn::visit::visit_impl_item_fn(self, item);
        }
    }
    fn visit_stmt(&mut self, stmt: &'ast syn::Stmt) {
        if let syn::Stmt::Local(local) = stmt {
            let (pat, ty) = match &local.pat {
                Pat::Type(typed) => (&*typed.pat, Some(&*typed.ty)),
                other => (other, None),
            };
            let init = local.init.as_ref().map(|init| &*init.expr);
            self.bind(pat, ty, init);
        }
        syn::visit::visit_stmt(self, stmt);
    }
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if support::production_expr(expr) {
            syn::visit::visit_expr(self, expr);
        }
    }
    fn visit_signature(&mut self, signature: &'ast syn::Signature) {
        for input in &signature.inputs {
            if let syn::FnArg::Typed(typed) = input {
                if type_names_unit_orders_mut(&typed.ty) {
                    self.hit(format!(
                        "`fn {}` takes a mutable `UnitOrders` or `Order`",
                        signature.ident
                    ));
                }
                self.bind(&typed.pat, Some(&typed.ty), None);
            }
        }
        syn::visit::visit_signature(self, signature);
    }
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        let method = call.method.to_string();
        if (method == "get_mut" || method == "remove")
            && self.turbofish_names_unit_orders(&call.turbofish)
        {
            self.hit(format!("`{method}::<UnitOrders>()`"));
        }
        if matches!(
            method.as_str(),
            "insert" | "insert_if_new" | "spawn" | "insert_bundle"
        ) && call
            .args
            .iter()
            .any(|arg| builds_unit_orders(arg, &self.bindings))
        {
            self.hit(format!(
                "`{method}` of a `UnitOrders` value (line {})",
                call.method.span().start().line
            ));
        }
        // A mutating call on a binding of the type: `orders.queue.push_back(..)`.
        if matches!(
            method.as_str(),
            "push_back"
                | "push_front"
                | "pop_front"
                | "pop_back"
                | "clear"
                | "take"
                | "insert"
                | "remove"
                | "retain"
                | "drain"
                | "append"
                | "truncate"
        ) && root_ident(&call.receiver).is_some_and(|root| self.bindings.contains(&root))
            && matches!(&*call.receiver, Expr::Field(_))
        {
            self.hit(format!(
                "`.{method}()` on a field of a `UnitOrders` binding"
            ));
        }
        syn::visit::visit_expr_method_call(self, call);
    }
    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        // `world.spawn((.., UnitOrders::default()))` is a method call; a free `spawn(..)` is not
        // used. A path call with a `UnitOrders` bundle argument is checked here as well.
        if let Expr::Path(path) = &*call.func
            && matches!(last_segment(&path.path).as_str(), "insert" | "spawn")
            && call
                .args
                .iter()
                .any(|arg| builds_unit_orders(arg, &self.bindings))
        {
            self.hit("a call that inserts a `UnitOrders` value");
        }
        syn::visit::visit_expr_call(self, call);
    }
    fn visit_expr_assign(&mut self, assign: &'ast syn::ExprAssign) {
        if root_ident(&assign.left).is_some_and(|root| self.bindings.contains(&root)) {
            self.hit("an assignment through a `UnitOrders` binding");
        }
        syn::visit::visit_expr_assign(self, assign);
    }
    fn visit_expr_binary(&mut self, binary: &'ast syn::ExprBinary) {
        // Compound assignment (`x.field += 1`) is a binary expression in syn 2.
        let compound = matches!(
            binary.op,
            syn::BinOp::AddAssign(_)
                | syn::BinOp::SubAssign(_)
                | syn::BinOp::MulAssign(_)
                | syn::BinOp::BitOrAssign(_)
        );
        if compound && root_ident(&binary.left).is_some_and(|root| self.bindings.contains(&root)) {
            self.hit("a compound assignment through a `UnitOrders` binding");
        }
        syn::visit::visit_expr_binary(self, binary);
    }
}

/// The write patterns found in one source text.
fn writer_hits(source: &str) -> Vec<String> {
    let file = syn::parse_file(source).expect("the source parses");
    let mut scan = WriterScan::default();
    // Each function has its own bindings.
    for item in &file.items {
        scan_item(&mut scan, item);
    }
    scan.hits
}

fn scan_item(scan: &mut WriterScan, item: &syn::Item) {
    match item {
        syn::Item::Fn(function) if support::production_attrs(&function.attrs) => {
            scan.bindings.clear();
            scan.visit_item_fn(function);
        }
        syn::Item::Impl(imp) if support::production_attrs(&imp.attrs) => {
            for member in &imp.items {
                if let syn::ImplItem::Fn(function) = member
                    && support::production_attrs(&function.attrs)
                {
                    scan.bindings.clear();
                    scan.visit_impl_item_fn(function);
                }
            }
        }
        syn::Item::Mod(module) if support::production_attrs(&module.attrs) => {
            if let Some((_, items)) = &module.content {
                for inner in items {
                    scan_item(scan, inner);
                }
            }
        }
        _ => {}
    }
}

fn sim_sources() -> Vec<PathBuf> {
    let root = repo();
    support::production_sources(&[root.join("crates/turts_sim/src")]).expect("sources read")
}

fn relative(path: &Path) -> String {
    let root = repo().canonicalize().expect("repo root");
    path.canonicalize()
        .expect("source path")
        .strip_prefix(root)
        .expect("under the repo")
        .to_string_lossy()
        .replace('\\', "/")
}

#[test]
fn no_production_write_of_unit_orders_outside_the_writer() {
    let sources = sim_sources();
    assert!(
        sources.len() > 20,
        "the scan must read the sim sources: it read {}",
        sources.len()
    );
    assert!(
        sources.iter().any(|p| relative(p) == WRITER_FILE),
        "the writer file is part of the scan roots"
    );
    let mut found = Vec::new();
    for path in &sources {
        let name = relative(path);
        if name == WRITER_FILE {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("read");
        for hit in writer_hits(&text) {
            found.push(format!("{name}: {hit}"));
        }
    }
    assert!(
        found.is_empty(),
        "a write of `UnitOrders` outside `orders/writer.rs` (route it through `set_unit_orders`, \
         `edit_unit_orders` or `clear_unit_orders`):\n{}",
        found.join("\n")
    );
}

#[test]
fn the_writer_file_itself_is_scanned_and_holds_the_writes() {
    // The scan would stay green if it could not see a write at all. The writer file has the
    // writes by design, so the scan must find them there.
    let text = std::fs::read_to_string(repo().join(WRITER_FILE)).expect("writer file");
    let hits = writer_hits(&text);
    assert!(
        hits.iter().any(|hit| hit.contains("get_mut::<UnitOrders>")),
        "the scan sees `get_mut::<UnitOrders>()` in the writer: {hits:?}"
    );
    assert!(
        hits.iter().any(|hit| hit.contains("remove::<UnitOrders>")),
        "the scan sees `remove::<UnitOrders>()` in the writer: {hits:?}"
    );
    assert!(
        hits.iter().any(|hit| hit.contains("insert")),
        "the scan sees an `insert` of a `UnitOrders` value in the writer: {hits:?}"
    );
}

#[test]
fn every_write_pattern_has_a_fixture_that_the_scan_finds() {
    let fixtures: [(&str, &str); 9] = [
        (
            "get_mut",
            "fn f(world: &mut World, e: Entity) { world.entity_mut(e).get_mut::<UnitOrders>().unwrap().current = None; }",
        ),
        (
            "get_mut with a path",
            "fn f(world: &mut World, e: Entity) { let _ = world.entity_mut(e).get_mut::<crate::orders::UnitOrders>(); }",
        ),
        (
            "remove",
            "fn f(world: &mut World, e: Entity) { world.entity_mut(e).remove::<UnitOrders>(); }",
        ),
        (
            "insert of a literal",
            "fn f(world: &mut World, e: Entity) { world.entity_mut(e).insert(UnitOrders { current: None, ..Default::default() }); }",
        ),
        (
            "insert of a call",
            "fn f(world: &mut World, e: Entity) { world.entity_mut(e).insert(UnitOrders::default()); }",
        ),
        (
            "insert of a binding",
            "fn f(world: &mut World, e: Entity) { let fresh = UnitOrders::default(); world.entity_mut(e).insert(fresh); }",
        ),
        (
            "field write on a binding",
            "fn f(world: &mut World, e: Entity) { let mut orders = world.entity_mut(e).get_mut::<UnitOrders>().unwrap(); orders.current = None; }",
        ),
        (
            "a helper that takes `&mut UnitOrders`",
            "fn helper(orders: &mut UnitOrders) { let _ = orders; }",
        ),
        (
            "a helper that takes `&mut Order`",
            "fn helper(order: &mut Order) { let _ = order; }",
        ),
    ];
    for (name, source) in fixtures {
        assert!(
            !writer_hits(source).is_empty(),
            "the scan must find the write pattern `{name}`: {source}"
        );
    }
}

#[test]
fn clean_code_and_test_code_give_no_hit() {
    let clean = "
        fn read(world: &World, e: Entity) -> bool { world.entity(e).get::<UnitOrders>().is_some() }
        fn route(world: &mut World, e: Entity) { set_unit_orders(world, e, UnitOrders::default()); edit_unit_orders(world, e, |o| o.queue.clear()); }
        #[cfg(test)]
        mod tests { fn f(world: &mut World, e: Entity) { world.entity_mut(e).insert(UnitOrders::default()); } }
        #[cfg(test)]
        fn only_in_tests(world: &mut World, e: Entity) { world.entity_mut(e).remove::<UnitOrders>(); }
    ";
    assert_eq!(writer_hits(clean), Vec::<String>::new());
}

#[test]
fn the_inferred_write_is_not_seen_by_the_scan() {
    // The limit of the scan, stated as a test: a write through an inferred `get_mut()` is left
    // to the `test-probes` check. If a future scan sees it, this test fails and the limit text in
    // the plan changes.
    let source = "fn f(world: &mut World, e: Entity) { let mut m = world.entity_mut(e); let o = m.get_mut(); apply(o); }";
    assert!(
        writer_hits(source).is_empty(),
        "an inferred `get_mut()` has no name to match"
    );
    // The same limit for an alias of the type (plant (10) of `l0-wire.json` writes the stop
    // through one; the scan stays green and the `test-probes` check panics).
    let alias = "fn f(world: &mut World, e: Entity) { type Alias = UnitOrders; world.entity_mut(e).insert(Alias::default()); }";
    assert!(
        writer_hits(alias).is_empty(),
        "an alias of the type has no name to match"
    );
}

// ---- 2. The export writers ------------------------------------------------------------------

#[derive(Default)]
struct ExportScan {
    hits: Vec<String>,
}

impl<'ast> Visit<'ast> for ExportScan {
    fn visit_item(&mut self, item: &'ast syn::Item) {
        let attrs = match item {
            syn::Item::Fn(x) => &x.attrs,
            syn::Item::Mod(x) => &x.attrs,
            syn::Item::Impl(x) => &x.attrs,
            _ => &[][..],
        };
        if support::production_attrs(attrs) {
            syn::visit::visit_item(self, item);
        }
    }
    fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
        let name = segment.ident.to_string();
        if matches!(
            name.as_str(),
            "Query" | "World" | "Res" | "ResMut" | "Commands" | "iter_entities" | "SystemParam"
        ) {
            self.hits.push(format!("the type or name `{name}`"));
        }
        syn::visit::visit_path_segment(self, segment);
    }
    fn visit_expr_method_call(&mut self, call: &'ast syn::ExprMethodCall) {
        let name = call.method.to_string();
        if matches!(
            name.as_str(),
            "query"
                | "query_filtered"
                | "iter_entities"
                | "get_resource"
                | "resource"
                | "get_entity"
                | "entity"
        ) {
            self.hits.push(format!("the call `.{name}()`"));
        }
        syn::visit::visit_expr_method_call(self, call);
    }
}

fn export_hits(source: &str) -> Vec<String> {
    let file = syn::parse_file(source).expect("the source parses");
    let mut scan = ExportScan::default();
    scan.visit_file(&file);
    scan.hits
}

fn export_writer_files() -> Vec<PathBuf> {
    let dir = repo().join("crates/turts_server/src");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("server src")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("export_") && name.ends_with(".rs"))
        })
        .collect();
    files.sort();
    files
}

#[test]
fn the_export_writers_read_only_committed_tick_fields() {
    let files = export_writer_files();
    let names: Vec<String> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    for expected in [
        "export_sites.rs",
        "export_supply.rs",
        "export_jobs.rs",
        "export_effects.rs",
        "export_match.rs",
    ] {
        assert!(
            names.iter().any(|n| n == expected),
            "{expected} exists (the scan would be empty otherwise): {names:?}"
        );
    }
    let mut found = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).expect("read");
        for hit in export_hits(&text) {
            found.push(format!("{}: {hit}", file.display()));
        }
    }
    assert!(
        found.is_empty(),
        "an export writer reads sim state other than `CommittedTick` fields:\n{}",
        found.join("\n")
    );
}

#[test]
fn every_export_read_pattern_has_a_fixture_that_the_scan_finds() {
    let fixtures: [(&str, &str); 4] = [
        (
            "a Query of a sim component",
            "use bevy_ecs::prelude::*; fn f(q: Query<&Health>) { let _ = q; }",
        ),
        (
            "a World parameter",
            "fn f(world: &World) { let _ = world; }",
        ),
        (
            "a resource read",
            "fn f(world: &World) { let _ = world.resource::<Tick>(); }",
        ),
        (
            "iter_entities",
            "fn f(world: &World) { for _e in world.iter_entities() {} }",
        ),
    ];
    for (name, source) in fixtures {
        assert!(
            !export_hits(source).is_empty(),
            "the export scan must find `{name}`: {source}"
        );
    }
    assert!(
        export_hits("pub fn rows(batch: &CommittedTick) -> usize { batch.sites_completed.len() }")
            .is_empty()
    );
}
