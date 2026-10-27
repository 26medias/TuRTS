use std::{
    fs,
    path::{Path, PathBuf},
};
use syn::{Expr, ExprCall, ExprMethodCall, Item, UseTree, visit::Visit};

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
        function_has_call(&boot, "boot", "load_package"),
        "server boot must call turts_pack::load_package"
    );
    assert!(
        function_has_call(&pack_main, "run", "pack_checked"),
        "CLI run must call pack_checked"
    );
    assert!(
        function_has_call(&pack_lib, "pack_checked", "check_package"),
        "pack_checked must call check_package"
    );
    assert!(
        function_has_call(&pack_lib, "check_package", "load_package"),
        "check_package must call load_package"
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
}

fn assert_no_native_mod_files(root: &Path) -> Result<(), Vec<PathBuf>> {
    fn walk(p: &Path, bad: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(p) else { return };
        for e in entries.flatten() {
            let q = e.path();
            if q.is_dir() {
                walk(&q, bad)
            } else if q.extension().is_some_and(|x| x == "rs")
                || q.file_name().is_some_and(|x| x == "Cargo.toml")
            {
                bad.push(q)
            }
        }
    }
    let mut bad = Vec::new();
    walk(root, &mut bad);
    if bad.is_empty() { Ok(()) } else { Err(bad) }
}
fn function_has_call(code: &str, function: &str, target: &str) -> bool {
    let Ok(file) = syn::parse_file(code) else {
        return false;
    };
    let mut v = Calls::default();
    v.visit_file(&file);
    v.scoped_calls.iter().any(|(scope, call)| {
        scope == function
            && (call == target
                || call.ends_with(&format!("::{target}"))
                || v.imports
                    .iter()
                    .any(|(name, full)| name == call && full.ends_with(&format!("::{target}"))))
    })
}

#[derive(Default)]
struct Calls {
    calls: Vec<String>,
    scoped_calls: Vec<(String, String)>,
    imports: Vec<(String, String)>,
    current: String,
}
impl<'a> Visit<'a> for Calls {
    fn visit_expr_call(&mut self, n: &'a ExprCall) {
        if let Expr::Path(p) = &*n.func {
            let call = p
                .path
                .segments
                .iter()
                .map(|x| x.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            self.calls.push(call.clone());
            self.scoped_calls.push((self.current.clone(), call));
        }
        syn::visit::visit_expr_call(self, n)
    }
    fn visit_item_fn(&mut self, n: &'a syn::ItemFn) {
        let old = std::mem::replace(&mut self.current, n.sig.ident.to_string());
        syn::visit::visit_item_fn(self, n);
        self.current = old;
    }
    fn visit_expr_path(&mut self, n: &'a syn::ExprPath) {
        let path = n
            .path
            .segments
            .iter()
            .map(|x| x.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        self.calls.push(path.clone());
        self.scoped_calls.push((self.current.clone(), path));
        syn::visit::visit_expr_path(self, n)
    }
    fn visit_expr_method_call(&mut self, n: &'a ExprMethodCall) {
        let call = n.method.to_string();
        self.calls.push(call.clone());
        self.scoped_calls.push((self.current.clone(), call));
        syn::visit::visit_expr_method_call(self, n)
    }
    fn visit_item(&mut self, n: &'a Item) {
        if let Item::Use(u) = n {
            flatten_use(&u.tree, String::new(), &mut self.imports);
        }
        syn::visit::visit_item(self, n)
    }
}
fn flatten_use(t: &UseTree, prefix: String, out: &mut Vec<(String, String)>) {
    match t {
        UseTree::Path(p) => flatten_use(
            &p.tree,
            if prefix.is_empty() {
                p.ident.to_string()
            } else {
                format!("{prefix}::{}", p.ident)
            },
            out,
        ),
        UseTree::Name(n) => {
            let full = if prefix.is_empty() {
                n.ident.to_string()
            } else {
                format!("{prefix}::{}", n.ident)
            };
            out.push((n.ident.to_string(), full));
        }
        UseTree::Rename(r) => {
            let full = if prefix.is_empty() {
                r.ident.to_string()
            } else {
                format!("{prefix}::{}", r.ident)
            };
            out.push((r.rename.to_string(), full));
        }
        UseTree::Group(g) => {
            for x in &g.items {
                flatten_use(x, prefix.clone(), out)
            }
        }
        UseTree::Glob(_) => (),
    }
}
fn forbidden_bypasses(code: &str) -> Vec<String> {
    let Ok(file) = syn::parse_file(code) else {
        return vec!["Rust parse failure".into()];
    };
    let mut v = Calls::default();
    v.visit_file(&file);
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
    let mut bad = Vec::new();
    for call in &v.calls {
        if call.split("::").any(|segment| forbidden.contains(&segment)) {
            bad.push(call.clone())
        }
    }
    for (alias, full) in &v.imports {
        let target = full.rsplit("::").next().unwrap_or(full);
        if forbidden.contains(&target)
            && v.calls
                .iter()
                .any(|c| c.split("::").any(|part| part == alias))
        {
            bad.push(format!("{alias} ({full})"))
        }
    }
    bad.sort();
    bad.dedup();
    bad
}
