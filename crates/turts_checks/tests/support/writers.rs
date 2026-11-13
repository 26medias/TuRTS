//! The writer scan visitor, shared by `movement_writers.rs` and `run_constant_writers.rs`
//! (pre-MP1 slice, task A1d; before that it was private to `movement_writers.rs`).
//!
//! A write is, in production code (the scanner production filter: no `#[cfg(test)]` item, no
//! `tests` directory, no listed test module file):
//! - kind `&mut`: a `&mut T` type (a query, a parameter, a field);
//! - kind `mut-param`: a `ResMut<T>`, `Mut<T>` or `NonSendMut<T>` type (a system parameter, a
//!   query item, a returned handle). Added in A1d: the scan had no arm for them, so a system
//!   parameter `ResMut<MovementPools>` was not seen;
//! - kind `method`: a method call with the turbofish `T` and a name of the method list;
//! - kind `construct`: a value of `T` that the code makes: `T(..)`, `T { .. }` or a call of a
//!   `T::` function (spawn and insert bundles hold such values).
//!
//! Known gap (A1d re-check, `docs/research/mp1/rereview-slice-a1d.md` section 3). The scan is
//! syntactic and sees a type only by its written name. It does NOT see:
//! - a value of `T` that a file receives and inserts with no `&mut T`, no `ResMut<T>`, no
//!   turbofish and no constructor;
//! - a copy that is changed and inserted again: `let mut b = *world.resource::<T>();` then
//!   `world.insert_resource(b)`, also through `Commands` (`commands.insert_resource(next)`). This
//!   is the natural form for a `Copy` type such as `MapBounds`;
//! - a type alias (`type A = T;` then `get_resource_mut::<A>()`);
//! - a rename of the import (`use ... T as A;` then `ResMut<A>`);
//! - a write in the body of a `macro_rules!` macro (the scan does not expand macros).
//!
//! Today code review is the only cover for these forms. A runtime guard (a system after boot that
//! reads the Bevy change ticks of the run constants and the presence set) would see every form;
//! it is an OPEN item, owner: technical-director decision. It is not built.

use std::path::Path;

use super::{collect, production_attrs, production_expr};
use syn::visit::Visit;

/// The wrappers of a mutable borrow of a resource or a component (bevy `ResMut`, `Mut`,
/// `NonSendMut`).
pub const MUT_WRAPPERS: [&str; 3] = ["ResMut", "Mut", "NonSendMut"];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Write {
    pub file: String,
    pub line: usize,
    pub kind: &'static str,
    pub type_name: String,
}

pub struct Scan<'a> {
    pub file: &'a str,
    /// The type names that the scan looks for (a path segment equal to one of them).
    pub written: &'a [&'a str],
    /// The method names whose turbofish type is a write.
    pub methods: &'a [&'a str],
    pub writes: Vec<Write>,
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

    fn written(&self, path: &syn::Path) -> Option<String> {
        path.segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .find(|name| self.written.contains(&name.as_str()))
    }

    fn written_type(&self, ty: &syn::Type) -> Option<String> {
        match ty {
            syn::Type::Path(path) => self.written(&path.path),
            _ => None,
        }
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
            && let Some(name) = self.written_type(&node.elem)
        {
            self.hit(node.and_token.span, "&mut", &name);
        }
        syn::visit::visit_type_reference(self, node);
    }

    fn visit_type_path(&mut self, node: &'ast syn::TypePath) {
        if let Some(last) = node.path.segments.last()
            && MUT_WRAPPERS.contains(&last.ident.to_string().as_str())
            && let syn::PathArguments::AngleBracketed(arguments) = &last.arguments
        {
            for argument in &arguments.args {
                if let syn::GenericArgument::Type(ty) = argument
                    && let Some(name) = self.written_type(ty)
                {
                    self.hit(last.ident.span(), "mut-param", &name);
                }
            }
        }
        syn::visit::visit_type_path(self, node);
    }

    fn visit_expr(&mut self, node: &'ast syn::Expr) {
        if !production_expr(node) {
            return;
        }
        match node {
            syn::Expr::MethodCall(call)
                if self.methods.contains(&call.method.to_string().as_str()) =>
            {
                let turbofish = call.turbofish.iter().flat_map(|fish| fish.args.iter());
                for argument in turbofish {
                    if let syn::GenericArgument::Type(ty) = argument
                        && let Some(name) = self.written_type(ty)
                    {
                        self.hit(call.method.span(), "method", &name);
                    }
                }
            }
            syn::Expr::Call(call) => {
                if let syn::Expr::Path(path) = call.func.as_ref()
                    && let Some(name) = self.written(&path.path)
                {
                    self.hit(path.path.segments[0].ident.span(), "construct", &name);
                }
            }
            syn::Expr::Struct(literal) => {
                if let Some(name) = self.written(&literal.path) {
                    self.hit(literal.path.segments[0].ident.span(), "construct", &name);
                }
            }
            _ => {}
        }
        syn::visit::visit_expr(self, node);
    }
}

/// The writes of one file.
pub fn scan_file(path: &Path, label: &str, written: &[&str], methods: &[&str]) -> Vec<Write> {
    let source = std::fs::read_to_string(path).expect("read a scanned file");
    let file = syn::parse_file(&source).expect("parse a scanned file");
    let mut scan = Scan {
        file: label,
        written,
        methods,
        writes: Vec::new(),
    };
    scan.visit_file(&file);
    scan.writes
}

/// The production writes of the three crates that can write the simulation world
/// (`turts_sim`, `turts_server`, `turts_pack`).
pub fn production_writes(repo: &Path, written: &[&str], methods: &[&str]) -> Vec<Write> {
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
            .strip_prefix(repo)
            .expect("a scanned file is in the repository")
            .to_string_lossy()
            .replace('\\', "/");
        let label = label.trim_start_matches("./").to_owned();
        writes.extend(scan_file(&file, &label, written, methods));
    }
    writes
}
