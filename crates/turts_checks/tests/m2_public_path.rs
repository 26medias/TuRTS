mod support;

use proc_macro2::TokenTree;
use std::{
    fs,
    path::{Path, PathBuf},
};
use syn::{Expr, ExprCall, ExprMethodCall, Item, Stmt, UseTree, visit::Visit};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn source(p: &Path) -> String {
    fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

#[test]
fn public_loading_path_has_no_native_hooks() {
    let root = repo();
    assert_no_native_mod_files(&root.join("mods")).unwrap();
    let boot = source(&root.join("crates/turts_server/src/boot.rs"));
    let pack_main = source(&root.join("crates/turts_pack/src/main.rs"));
    let pack_lib = source(&root.join("crates/turts_pack/src/lib.rs"));
    assert!(
        function_has_call(&boot, "boot", "turts_pack::load_package"),
        "server boot must call the public turts_pack::load_package path"
    );
    assert!(
        function_has_call(&pack_main, "run", "turts_pack::pack_checked"),
        "CLI run must call the public turts_pack::pack_checked path"
    );
    assert!(
        function_has_call(&pack_lib, "pack_checked", "check_package"),
        "pack_checked must call local check_package"
    );
    assert!(
        function_has_call(&pack_lib, "check_package", "load_package"),
        "check_package must call local load_package"
    );
    for (name, text) in [("server boot", boot), ("pack CLI", pack_main)] {
        let bypass = forbidden_bypasses(&text);
        assert!(
            bypass.is_empty(),
            "{name} bypasses public loader: {bypass:?}"
        );
    }

    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("m2_public_path");
    let mods = scratch.join("mods/rogue");
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&mods).unwrap();
    fs::write(mods.join("native.rs"), "pub fn hook() {}\n").unwrap();
    fs::write(mods.join("Cargo.toml"), "[package]\nname='rogue'\n").unwrap();
    assert!(
        assert_no_native_mod_files(&scratch.join("mods")).is_err(),
        "scratch native hook must be detected"
    );
    let bypass = "use turts_sim::install_package as publish; fn f() { publish(); }";
    assert!(
        !forbidden_bypasses(bypass).is_empty(),
        "aliased bypass must be detected"
    );
    assert!(
        !forbidden_bypasses("fn f(a: &Archive) { let _ = a.content(); }").is_empty(),
        "direct catalog methods must be detected"
    );
    assert!(
        !forbidden_bypasses(
            "use turts_sim::UnitRegistry as Registry; fn f() { let _ = Registry::new(); }"
        )
        .is_empty(),
        "aliased registry construction must be detected"
    );
    assert!(
        !forbidden_bypasses("fn boot() { wrapper!(turts_sim::install_package()); }").is_empty(),
        "macro-body direct bypass must be detected"
    );
    assert!(
        !forbidden_bypasses(
            "use turts_sim::install_package as install; fn boot() { wrapper!(install()); }"
        )
        .is_empty(),
        "macro-body imported alias bypass must be detected"
    );
    assert!(
        assert_no_native_mod_files(&scratch.join("missing_mods")).is_err(),
        "missing mod root must fail closed"
    );
    let not_dir = scratch.join("not_a_directory");
    fs::write(&not_dir, "file").unwrap();
    assert!(
        assert_no_native_mod_files(&not_dir).is_err(),
        "non-directory traversal root must fail closed"
    );
}

#[test]
fn public_loader_guard_requires_actual_expected_production_calls() {
    assert!(function_has_call(
        "fn boot() { turts_pack::load_package(); }",
        "boot",
        "turts_pack::load_package"
    ));
    assert!(function_has_call(
        "use turts_pack::load_package as load; fn boot() { load(); }",
        "boot",
        "turts_pack::load_package"
    ));
    assert!(function_has_call(
        "use turts_pack as package; fn boot() { package::load_package(); }",
        "boot",
        "turts_pack::load_package"
    ));
    for code in [
        "fn boot() { let _ = turts_pack::load_package; }",
        "fn boot() { other::load_package(); }",
        "#[cfg(test)] fn boot() { turts_pack::load_package(); }",
        "fn boot() { #[cfg(test)] { turts_pack::load_package(); } }",
    ] {
        assert!(
            !function_has_call(code, "boot", "turts_pack::load_package"),
            "accepted non-production or wrong path: {code}"
        );
    }
    assert!(function_has_call(
        "#[cfg(not(test))] fn boot() { turts_pack::load_package(); }",
        "boot",
        "turts_pack::load_package"
    ));
    assert!(!function_has_call(
        "fn pack_checked() { other::check_package(); }",
        "pack_checked",
        "check_package"
    ));
    assert!(!function_has_call(
        "fn check_package() { other::load_package(); }",
        "check_package",
        "load_package"
    ));
}

fn assert_no_native_mod_files(root: &Path) -> Result<(), String> {
    fn walk(path: &Path, hits: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let child = entry.path();
            let kind = entry.file_type()?;
            if kind.is_dir() {
                walk(&child, hits)?;
            } else if child.extension().is_some_and(|x| x == "rs")
                || child.file_name().is_some_and(|x| x == "Cargo.toml")
            {
                hits.push(child);
            }
        }
        Ok(())
    }
    if !root.is_dir() {
        return Err(format!(
            "{} is not a readable mod directory",
            root.display()
        ));
    }
    let mut hits = Vec::new();
    walk(root, &mut hits).map_err(|e| format!("cannot read {}: {e}", root.display()))?;
    if hits.is_empty() {
        Ok(())
    } else {
        Err(format!("native files under mods: {hits:?}"))
    }
}

fn function_has_call(code: &str, function: &str, expected: &str) -> bool {
    let Ok(file) = syn::parse_file(code) else {
        return false;
    };
    let mut calls = ProductionCalls::default();
    calls.visit_file(&file);
    calls.scoped_calls.iter().any(|(scope, call)| {
        if scope != function {
            return false;
        }
        if call == expected {
            return true;
        }
        calls.imports.iter().any(|(import_scope, alias, path)| {
            if !(import_scope.is_empty() || import_scope == function) {
                return false;
            }
            if alias == call && path == expected {
                return true;
            }
            call.strip_prefix(&format!("{alias}::"))
                .is_some_and(|suffix| format!("{path}::{suffix}") == expected)
        })
    })
}

#[derive(Default)]
struct ProductionCalls {
    scoped_calls: Vec<(String, String)>,
    imports: Vec<(String, String, String)>,
    current: String,
}
impl<'ast> Visit<'ast> for ProductionCalls {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if !support::production_attrs(&item.attrs) {
            return;
        }
        let old = std::mem::replace(&mut self.current, item.sig.ident.to_string());
        syn::visit::visit_item_fn(self, item);
        self.current = old;
    }
    fn visit_item(&mut self, item: &'ast Item) {
        if !support::production_attrs(item_attrs(item)) {
            return;
        }
        if let Item::Use(import) = item {
            let mut imports = Vec::new();
            flatten_use(&import.tree, String::new(), &mut imports);
            self.imports.extend(
                imports
                    .into_iter()
                    .map(|(alias, path)| (self.current.clone(), alias, path)),
            );
        }
        syn::visit::visit_item(self, item);
    }
    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        let attrs = stmt_attrs(stmt);
        if support::production_attrs(attrs) {
            syn::visit::visit_stmt(self, stmt);
        }
    }
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if support::production_expr(expr) {
            syn::visit::visit_expr(self, expr);
        }
    }
    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(path) = &*call.func {
            let name = path
                .path
                .segments
                .iter()
                .map(|segment| normalized_ident(&segment.ident))
                .collect::<Vec<_>>()
                .join("::");
            self.scoped_calls.push((self.current.clone(), name));
        }
        syn::visit::visit_expr_call(self, call);
    }
}

#[derive(Default)]
struct BypassTokens {
    paths: Vec<(String, String)>,
    methods: Vec<(String, String)>,
    macro_tokens: Vec<(String, String)>,
    imports: Vec<(String, String, String)>,
    current: String,
}
impl<'ast> Visit<'ast> for BypassTokens {
    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        if !support::production_attrs(&item.attrs) {
            return;
        }
        let old = std::mem::replace(&mut self.current, item.sig.ident.to_string());
        syn::visit::visit_item_fn(self, item);
        self.current = old;
    }
    fn visit_item(&mut self, item: &'ast Item) {
        if !support::production_attrs(item_attrs(item)) {
            return;
        }
        if let Item::Use(import) = item {
            let mut imports = Vec::new();
            flatten_use(&import.tree, String::new(), &mut imports);
            self.imports.extend(
                imports
                    .into_iter()
                    .map(|(alias, path)| (self.current.clone(), alias, path)),
            );
        }
        syn::visit::visit_item(self, item);
    }
    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        if support::production_attrs(stmt_attrs(stmt)) {
            syn::visit::visit_stmt(self, stmt);
        }
    }
    fn visit_expr(&mut self, expr: &'ast Expr) {
        if support::production_expr(expr) {
            syn::visit::visit_expr(self, expr);
        }
    }
    fn visit_expr_path(&mut self, path: &'ast syn::ExprPath) {
        self.paths.push((
            self.current.clone(),
            path.path
                .segments
                .iter()
                .map(|segment| normalized_ident(&segment.ident))
                .collect::<Vec<_>>()
                .join("::"),
        ));
        syn::visit::visit_expr_path(self, path);
    }
    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        self.methods
            .push((self.current.clone(), normalized_ident(&call.method)));
        syn::visit::visit_expr_method_call(self, call);
    }
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.macro_tokens.extend(
            mac.path
                .segments
                .iter()
                .map(|segment| (self.current.clone(), normalized_ident(&segment.ident))),
        );
        collect_macro_idents(mac.tokens.clone(), &self.current, &mut self.macro_tokens);
    }
}

fn collect_macro_idents(
    stream: proc_macro2::TokenStream,
    scope: &str,
    out: &mut Vec<(String, String)>,
) {
    for token in stream {
        match token {
            TokenTree::Ident(ident) => out.push((scope.to_owned(), normalized_ident(&ident))),
            TokenTree::Group(group) => collect_macro_idents(group.stream(), scope, out),
            TokenTree::Literal(_) | TokenTree::Punct(_) => (),
        }
    }
}
fn normalized_ident(ident: &syn::Ident) -> String {
    let spelling = ident.to_string();
    spelling.strip_prefix("r#").unwrap_or(&spelling).to_owned()
}
fn item_attrs(item: &Item) -> &[syn::Attribute] {
    match item {
        Item::Const(x) => &x.attrs,
        Item::Enum(x) => &x.attrs,
        Item::ExternCrate(x) => &x.attrs,
        Item::Fn(x) => &x.attrs,
        Item::ForeignMod(x) => &x.attrs,
        Item::Impl(x) => &x.attrs,
        Item::Macro(x) => &x.attrs,
        Item::Mod(x) => &x.attrs,
        Item::Static(x) => &x.attrs,
        Item::Struct(x) => &x.attrs,
        Item::Trait(x) => &x.attrs,
        Item::TraitAlias(x) => &x.attrs,
        Item::Type(x) => &x.attrs,
        Item::Union(x) => &x.attrs,
        Item::Use(x) => &x.attrs,
        Item::Verbatim(_) => &[],
        _ => &[],
    }
}
fn stmt_attrs(stmt: &Stmt) -> &[syn::Attribute] {
    match stmt {
        Stmt::Local(local) => &local.attrs,
        Stmt::Item(item) => item_attrs(item),
        Stmt::Macro(mac) => &mac.attrs,
        Stmt::Expr(_, _) => &[],
    }
}
fn flatten_use(tree: &UseTree, prefix: String, out: &mut Vec<(String, String)>) {
    match tree {
        UseTree::Path(path) => flatten_use(
            &path.tree,
            if prefix.is_empty() {
                normalized_ident(&path.ident)
            } else {
                format!("{prefix}::{}", normalized_ident(&path.ident))
            },
            out,
        ),
        UseTree::Name(name) => {
            let full = if prefix.is_empty() {
                normalized_ident(&name.ident)
            } else {
                format!("{prefix}::{}", normalized_ident(&name.ident))
            };
            out.push((normalized_ident(&name.ident), full));
        }
        UseTree::Rename(rename) => {
            let full = if prefix.is_empty() {
                normalized_ident(&rename.ident)
            } else {
                format!("{prefix}::{}", normalized_ident(&rename.ident))
            };
            out.push((normalized_ident(&rename.rename), full));
        }
        UseTree::Group(group) => {
            for item in &group.items {
                flatten_use(item, prefix.clone(), out);
            }
        }
        UseTree::Glob(_) => (),
    }
}
fn forbidden_bypasses(code: &str) -> Vec<String> {
    let Ok(file) = syn::parse_file(code) else {
        return vec!["Rust parse failure".into()];
    };
    let mut scan = BypassTokens::default();
    scan.visit_file(&file);
    let forbidden = [
        "install_package",
        "stage_package",
        "load_map",
        "UnitRegistry",
        "content",
        "maps",
        "units",
        "resources",
        "nodes",
        "production",
    ];
    let mut hits = Vec::new();
    for (scope, path) in &scan.paths {
        if path.split("::").any(|part| forbidden.contains(&part)) {
            hits.push(path.clone());
        }
        for (import_scope, alias, full) in &scan.imports {
            let target = full.rsplit("::").next().unwrap_or(full);
            if forbidden.contains(&target)
                && alias == path.split("::").next().unwrap_or(path)
                && (import_scope.is_empty() || import_scope == scope)
            {
                hits.push(format!("{alias} ({full})"));
            }
        }
    }
    for (_, method) in &scan.methods {
        if forbidden.contains(&method.as_str()) {
            hits.push(method.clone());
        }
    }
    for (scope, token) in &scan.macro_tokens {
        if forbidden.contains(&token.as_str()) {
            hits.push(format!("macro token `{token}`"));
        }
        for (import_scope, alias, full) in &scan.imports {
            let target = full.rsplit("::").next().unwrap_or(full);
            if forbidden.contains(&target)
                && alias == token
                && (import_scope.is_empty() || import_scope == scope)
            {
                hits.push(format!("macro alias {alias} ({full})"));
            }
        }
    }
    hits.sort();
    hits.dedup();
    hits
}
