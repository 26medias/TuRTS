#![allow(dead_code)]
use proc_macro2::{Literal, TokenTree};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use syn::{Item, Lit, Macro, visit::Visit};
use turts_mod_api::{MapDef, UnitDef, VisualDef};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenRecord {
    pub token: String,
    pub category: String,
    pub package: String,
    pub definition_path: String,
    pub field: String,
}
#[derive(Debug, Clone, Default)]
pub struct TokenCatalog {
    records: BTreeMap<String, Vec<TokenRecord>>,
    exceptions: BTreeMap<String, GenericException>,
}
#[derive(Debug, Clone)]
pub struct GenericException {
    pub category: String,
    pub reason: String,
    pub definition_provenance: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub token: String,
    pub category: String,
    pub package: String,
    pub definition_path: String,
    pub field: String,
    pub source: PathBuf,
    pub line: usize,
    pub column: usize,
}
#[derive(Debug)]
pub enum ScanError {
    Io(PathBuf, std::io::Error),
    Package(PathBuf, String),
    Rust(PathBuf, String),
}
impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(p, e) => write!(f, "cannot read {}: {e}", p.display()),
            Self::Package(p, e) => write!(f, "invalid package at {}: {e}", p.display()),
            Self::Rust(p, e) => write!(f, "Rust parse failed at {}: {e}", p.display()),
        }
    }
}
impl std::error::Error for ScanError {}
fn pkgerr(p: &Path, e: impl ToString) -> ScanError {
    ScanError::Package(p.into(), e.to_string())
}

pub fn derive_tokens(root: &Path) -> Result<TokenCatalog, ScanError> {
    let mut dirs = fs::read_dir(root)
        .map_err(|e| ScanError::Io(root.into(), e))?
        .map(|e| {
            e.map(|e| e.path())
                .map_err(|e| ScanError::Io(root.into(), e))
        })
        .collect::<Result<Vec<_>, _>>()?;
    dirs.sort();
    let mut c = TokenCatalog::default();
    let mut count = 0;
    for dir in dirs.into_iter().filter(|p| p.is_dir()) {
        count += 1;
        let pname = dir
            .file_name()
            .and_then(|x| x.to_str())
            .unwrap_or("")
            .to_owned();
        let bytes = pack_dir(&dir).map_err(|e| pkgerr(&dir, e))?;
        let a = PackageArchive::parse(&bytes).map_err(|e| pkgerr(&dir, e))?;
        let p = a.manifest.id.clone();
        let mut exports = BTreeSet::from(["manifest.ron".to_string()]);
        for (kind, xs) in [
            ("unit-export", &a.manifest.exports.units),
            ("map-export", &a.manifest.exports.maps),
            ("visual-export", &a.manifest.exports.visuals),
            ("resource-export", &a.manifest.exports.resources),
            ("node-export", &a.manifest.exports.nodes),
            ("production-export", &a.manifest.exports.production),
        ] {
            for path in xs {
                exports.insert(path.clone());
                add(&mut c, &p, kind, path, "export-path", path);
                add(
                    &mut c,
                    &p,
                    "package-path",
                    path,
                    "package-file",
                    &format!("mods/{pname}/{path}"),
                );
                add(
                    &mut c,
                    &p,
                    "archive-path",
                    path,
                    "archive-file",
                    &format!("{pname}/{path}"),
                );
            }
        }
        stray_ron(&dir, &dir, &exports)?;
        a.content().map_err(|e| pkgerr(&dir, e))?;
        add(&mut c, &p, "package", &pname, "manifest.id", &p);
        add(
            &mut c,
            &p,
            "package-path",
            &pname,
            "directory",
            &format!("mods/{pname}"),
        );
        add(
            &mut c,
            &p,
            "archive-path",
            &pname,
            "archive",
            &format!("{pname}.tar"),
        );
        add(
            &mut c,
            &p,
            "archive-path",
            &pname,
            "archive",
            &format!("{}-{}.tar", p, a.manifest.version),
        );
        for (path, d) in a.units().map_err(|e| pkgerr(&dir, e))? {
            add_unit(&mut c, &p, &path, &d);
        }
        for (path, d) in a.resources().map_err(|e| pkgerr(&dir, e))? {
            add_id(&mut c, &p, &path, "id", &d.id, "resource");
            add(&mut c, &p, "resource-name", &path, "name", &d.name);
            add(&mut c, &p, "unit-name", &path, "unit_name", &d.unit_name);
            add(
                &mut c,
                &p,
                "resource-kind",
                &path,
                "kind",
                &format!("{:?}", d.kind),
            );
        }
        for (path, d) in a.nodes().map_err(|e| pkgerr(&dir, e))? {
            add_id(&mut c, &p, &path, "id", &d.id, "node");
            add(&mut c, &p, "node-name", &path, "name", &d.name);
            add_id(&mut c, &p, &path, "item", &d.item, "item-ref");
        }
        for (path, d) in a.production().map_err(|e| pkgerr(&dir, e))? {
            add_id(&mut c, &p, &path, "id", &d.id, "production");
            add(&mut c, &p, "production-name", &path, "name", &d.name);
            add_id(&mut c, &p, &path, "output_unit", &d.output_unit, "unit-ref");
            for x in &d.costs {
                add_id(
                    &mut c,
                    &p,
                    &path,
                    "cost.resource",
                    &x.resource,
                    "resource-ref",
                );
            }
        }
        for (path, d) in a.maps().map_err(|e| pkgerr(&dir, e))? {
            add_map(&mut c, &p, &path, &d);
        }
        for (path, d) in a.visuals().map_err(|e| pkgerr(&dir, e))? {
            add_visual(&mut c, &p, &path, &d);
        }
    }
    if count == 0 || c.records.is_empty() {
        return Err(pkgerr(root, "empty package discovery or token catalog"));
    }
    // Keep only known generic spellings that occur in the parsed data. Full ids and paths are never exempt.
    for (word, reason) in [
        ("field", "map display name is an ordinary source identifier"),
        ("Item", "resource kind is a common Rust type word"),
        ("Balance", "resource kind is a common Rust type word"),
        ("log", "resource unit name is a common source identifier"),
    ] {
        if let Some(records) = c.records.get(word) {
            let r = &records[0];
            c.exceptions.insert(
                word.into(),
                GenericException {
                    category: "generic-word".into(),
                    reason: reason.into(),
                    definition_provenance: format!(
                        "{}:{}:{}:{}",
                        r.package, r.definition_path, r.field, r.token
                    ),
                },
            );
        }
    }
    Ok(c)
}
fn stray_ron(dir: &Path, base: &Path, ex: &BTreeSet<String>) -> Result<(), ScanError> {
    for e in fs::read_dir(dir).map_err(|e| ScanError::Io(dir.into(), e))? {
        let p = e.map_err(|e| ScanError::Io(dir.into(), e))?.path();
        if p.is_dir() {
            stray_ron(&p, base, ex)?
        } else if p.extension().is_some_and(|x| x == "ron") {
            let r = p
                .strip_prefix(base)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            if !ex.contains(&r) {
                return Err(pkgerr(&p, format!("unexported RON definition {r}")));
            }
        }
    }
    Ok(())
}
fn add(c: &mut TokenCatalog, p: &str, cat: &str, path: &str, field: &str, t: &str) {
    if t.is_empty() {
        return;
    }
    let r = TokenRecord {
        token: t.into(),
        category: cat.into(),
        package: p.into(),
        definition_path: path.into(),
        field: field.into(),
    };
    c.records.entry(t.into()).or_default().push(r.clone());
    if let Some((pkg, local)) = t.split_once(':') {
        if pkg == p {
            c.records
                .entry(local.into())
                .or_default()
                .push(TokenRecord {
                    token: local.into(),
                    ..r
                });
        }
    }
}
fn add_id(c: &mut TokenCatalog, p: &str, path: &str, field: &str, id: &str, cat: &str) {
    add(c, p, cat, path, field, id);
    if let Some((_, l)) = id.split_once(':') {
        add(c, p, cat, path, field, l)
    }
}
fn add_unit(c: &mut TokenCatalog, p: &str, path: &str, d: &UnitDef) {
    add_id(c, p, path, "id", &d.id, "unit");
    add(c, p, "unit-name", path, "name", &d.name)
}
fn add_map(c: &mut TokenCatalog, p: &str, path: &str, d: &MapDef) {
    add_id(c, p, path, "id", &d.id, "map");
    for x in &d.placements {
        add_id(c, p, path, "placements.unit", &x.unit, "unit-ref");
        for i in &x.contents {
            add_id(c, p, path, "placements.contents.item", &i.item, "item-ref")
        }
    }
    for x in &d.nodes {
        add_id(c, p, path, "nodes.node", &x.node, "node-ref")
    }
    for x in &d.slot_economy {
        for b in &x.balances {
            add_id(
                c,
                p,
                path,
                "slot_economy.balance",
                &b.balance,
                "resource-ref",
            )
        }
    }
}
fn add_visual(c: &mut TokenCatalog, p: &str, path: &str, d: &VisualDef) {
    match &d.target {
        turts_mod_api::VisualTarget::Unit(i) => {
            add_id(c, p, path, "target.unit", i, "visual-unit-ref")
        }
        turts_mod_api::VisualTarget::Node(i) => {
            add_id(c, p, path, "target.node", i, "visual-node-ref")
        }
    }
}

pub fn scan_rust(roots: &[PathBuf], c: &TokenCatalog) -> Result<Vec<Hit>, ScanError> {
    let mut fslist = Vec::new();
    for r in roots {
        collect(r, &mut fslist)?
    }
    fslist.sort();
    let mut hits = Vec::new();
    for p in fslist {
        let s = fs::read_to_string(&p).map_err(|e| ScanError::Io(p.clone(), e))?;
        let f = syn::parse_file(&s).map_err(|e| ScanError::Rust(p.clone(), e.to_string()))?;
        let mut v = Scanner {
            catalog: c,
            path: &p,
            hits: Vec::new(),
        };
        v.visit_file(&f);
        hits.extend(v.hits)
    }
    hits.sort_by(|a, b| {
        (&a.source, a.line, a.column, &a.token).cmp(&(&b.source, b.line, b.column, &b.token))
    });
    Ok(hits)
}
fn collect(p: &Path, out: &mut Vec<PathBuf>) -> Result<(), ScanError> {
    if p.is_file() {
        if p.extension().is_some_and(|x| x == "rs")
            && !p.components().any(|x| x.as_os_str() == "tests")
        {
            out.push(p.into())
        }
        return Ok(());
    }
    for e in fs::read_dir(p).map_err(|e| ScanError::Io(p.into(), e))? {
        let q = e.map_err(|e| ScanError::Io(p.into(), e))?.path();
        if q.is_dir() && q.file_name().is_some_and(|x| x == "tests") {
            continue;
        }
        collect(&q, out)?
    }
    Ok(())
}
struct Scanner<'a> {
    catalog: &'a TokenCatalog,
    path: &'a Path,
    hits: Vec<Hit>,
}
impl Scanner<'_> {
    fn check(&mut self, t: &str, s: proc_macro2::Span) {
        if self.catalog.exceptions.contains_key(t) {
            return;
        }
        if let Some(rs) = self.catalog.records.get(t) {
            let p = s.start();
            for r in rs {
                self.hits.push(Hit {
                    token: t.into(),
                    category: r.category.clone(),
                    package: r.package.clone(),
                    definition_path: r.definition_path.clone(),
                    field: r.field.clone(),
                    source: self.path.into(),
                    line: p.line,
                    column: p.column,
                })
            }
        }
    }
    fn stream(&mut self, s: proc_macro2::TokenStream) {
        for t in s {
            match t {
                TokenTree::Ident(i) => self.check(&i.to_string(), i.span()),
                TokenTree::Literal(l) => self.literal(&l),
                TokenTree::Group(g) => self.stream(g.stream()),
                TokenTree::Punct(_) => (),
            }
        }
    }
    fn literal(&mut self, l: &Literal) {
        if let Ok(s) = syn::parse_str::<syn::LitStr>(&l.to_string()) {
            self.check(&s.value(), l.span())
        } else if let Ok(s) = syn::parse_str::<syn::LitByteStr>(&l.to_string()) {
            if let Ok(x) = std::str::from_utf8(&s.value()) {
                self.check(x, l.span())
            }
        }
    }
}
impl<'a> Visit<'a> for Scanner<'_> {
    fn visit_item(&mut self, i: &'a Item) {
        if is_cfg_test(i) {
            return;
        }
        syn::visit::visit_item(self, i)
    }
    fn visit_ident(&mut self, i: &'a syn::Ident) {
        self.check(&i.to_string(), i.span())
    }
    fn visit_lit(&mut self, l: &'a Lit) {
        if let Lit::Str(s) = l {
            self.check(&s.value(), s.span())
        }
        syn::visit::visit_lit(self, l)
    }
    fn visit_macro(&mut self, m: &'a Macro) {
        self.stream(m.tokens.clone())
    }
}
fn is_cfg_test(i: &Item) -> bool {
    let attrs = match i {
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
        Item::Verbatim(_) => return false,
        _ => return false,
    };
    attrs
        .iter()
        .any(|a| a.path().is_ident("cfg") && a.parse_args::<syn::Meta>().is_ok_and(meta_has_test))
}
fn meta_has_test(meta: syn::Meta) -> bool {
    match meta {
        syn::Meta::Path(path) => path.is_ident("test"),
        syn::Meta::List(list) => list
            .parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            )
            .is_ok_and(|nested| nested.into_iter().any(meta_has_test)),
        syn::Meta::NameValue(_) => false,
    }
}
