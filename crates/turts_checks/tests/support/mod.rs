#![allow(dead_code)]
use proc_macro2::{Literal, TokenTree};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};
use syn::{Expr, Item, Lit, Macro, Stmt, visit::Visit};
use turts_mod_api::{
    ContentCatalog, ContentId, MapDef, ResourceKind, UnitDef, VisualDef, VisualTarget,
};
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
    /// A listed test-module file whose parent does not declare it as `#[cfg(test)] mod`.
    TestModule(PathBuf, PathBuf, String),
}
impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(p, e) => write!(f, "cannot read {}: {e}", p.display()),
            Self::Package(p, e) => write!(f, "invalid package at {}: {e}", p.display()),
            Self::Rust(p, e) => write!(f, "Rust parse failed at {}: {e}", p.display()),
            Self::TestModule(file, parent, e) => write!(
                f,
                "test-module file {} is skipped by the scan, but its parent {}: {e}",
                file.display(),
                parent.display()
            ),
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
            ("order-export", &a.manifest.exports.orders),
            ("interface-export", &a.manifest.exports.interface),
            ("input-export", &a.manifest.exports.input),
            ("asset-export", &a.manifest.exports.assets),
            ("recipe-export", &a.manifest.exports.recipes),
            ("technology-export", &a.manifest.exports.technologies),
            ("match-export", &a.manifest.exports.match_files),
            ("combat-export", &a.manifest.exports.combat),
            ("visual-effect-export", &a.manifest.exports.visual_effects),
            ("bot-export", &a.manifest.exports.bots),
            (
                "terrain-material-export",
                &a.manifest.exports.terrain_materials,
            ),
            (
                "terrain-movement-class-export",
                &a.manifest.exports.terrain_movement_classes,
            ),
            ("terrain-tool-export", &a.manifest.exports.terrain_tools),
            ("utility-export", &a.manifest.exports.utilities),
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
        let content = a.content().map_err(|e| pkgerr(&dir, e))?;
        validate_map_references(&dir, &a, &content)?;
        turts_mod_runtime::terrain::validate_terrain_catalog(
            &turts_mod_api::PackageId(p.clone()),
            &content,
        )
        .map_err(|error| pkgerr(&dir, error))?;
        for (file, map) in a.maps().map_err(|error| pkgerr(&dir, error))? {
            turts_mod_runtime::terrain::validate_terrain_map(
                &turts_mod_api::PackageId(p.clone()),
                &file,
                &map,
                &content,
            )
            .map_err(|error| pkgerr(&dir, error))?;
        }
        validate_visual_references(&dir, &a, &content)?;
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
        for (path, definition) in a.bots().map_err(|e| pkgerr(&dir, e))? {
            // Qualified profile IDs identify content. Local keys can be ordinary
            // framework terms, such as opening or opponent.
            add(
                &mut c,
                &p,
                "bot-profile",
                &path,
                "id",
                &format!("{p}:{}", definition.id),
            );
        }
        for (path, definition) in a.terrain_materials().map_err(|error| pkgerr(&dir, error))? {
            add_id(&mut c, &p, &path, "id", &definition.id, "terrain-material");
            add(
                &mut c,
                &p,
                "terrain-material-name",
                &path,
                "name",
                &definition.name,
            );
            for rate in definition
                .natural_outputs
                .iter()
                .chain(&definition.fill_inputs)
            {
                add_id(
                    &mut c,
                    &p,
                    &path,
                    "resource",
                    &rate.resource,
                    "resource-ref",
                );
            }
        }
        for (path, definition) in a
            .terrain_movement_classes()
            .map_err(|error| pkgerr(&dir, error))?
        {
            add_id(
                &mut c,
                &p,
                &path,
                "id",
                &definition.id,
                "terrain-movement-class",
            );
            add(
                &mut c,
                &p,
                "terrain-movement-class-name",
                &path,
                "name",
                &definition.name,
            );
        }
        for (path, definition) in a.terrain_tools().map_err(|error| pkgerr(&dir, error))? {
            add_id(&mut c, &p, &path, "id", &definition.id, "terrain-tool");
            add(
                &mut c,
                &p,
                "terrain-tool-name",
                &path,
                "name",
                &definition.name,
            );
            for material in &definition.allowed_materials {
                add_id(
                    &mut c,
                    &p,
                    &path,
                    "allowed_materials",
                    material,
                    "terrain-material-ref",
                );
            }
            if let turts_mod_api::TerrainOperation::Fill { material } = &definition.operation {
                add_id(
                    &mut c,
                    &p,
                    &path,
                    "operation.material",
                    material,
                    "terrain-material-ref",
                );
            }
        }
        for (path, d) in a.utilities().map_err(|e| pkgerr(&dir, e))? {
            add_id(&mut c, &p, &path, "id", &d.id, "utility");
            add(&mut c, &p, "utility-name", &path, "name", &d.name);
            add_id(&mut c, &p, &path, "carries", &d.carries, "resource-ref");
            for port in &d.ports {
                add_id(&mut c, &p, &path, "ports.unit", &port.unit, "unit-ref");
            }
            for cost in &d.segment_cost {
                add_id(
                    &mut c,
                    &p,
                    &path,
                    "segment_cost",
                    &cost.resource,
                    "resource-ref",
                );
            }
            let labels = &d.appearance.labels;
            for (field, text) in [
                ("powered", &labels.powered),
                ("short", &labels.short),
                ("unconnected", &labels.unconnected),
                ("exposed", &labels.exposed),
            ] {
                add(&mut c, &p, "utility-label", &path, field, text);
            }
        }
        for (path, d) in a.orders().map_err(|e| pkgerr(&dir, e))? {
            add_id(&mut c, &p, &path, "id", &d.id, "order");
            add(&mut c, &p, "order-name", &path, "name", &d.name);
            add(&mut c, &p, "image-asset-ref", &path, "icon", &d.icon);
            add(&mut c, &p, "input-action-ref", &path, "hotkey", &d.hotkey);
            if let Some(cursor) = &d.cursor {
                add(&mut c, &p, "cursor-asset-ref", &path, "cursor", cursor);
            }
            for unit in &d.units {
                add_id(&mut c, &p, &path, "units", unit, "unit-ref");
            }
        }
        for (path, d) in a.order_rules().map_err(|e| pkgerr(&dir, e))? {
            for smart in &d.smart {
                add_id(&mut c, &p, &path, "smart.order", &smart.order, "order-ref");
            }
        }
        for (path, d) in a.input_maps().map_err(|e| pkgerr(&dir, e))? {
            for action in &d.actions {
                add(&mut c, &p, "input-action", &path, "name", &action.name);
            }
        }
        for (path, d) in a.assets().map_err(|e| pkgerr(&dir, e))? {
            for image in &d.images {
                add_id(&mut c, &p, &path, "id", &image.id, "image-asset");
            }
            for font in &d.fonts {
                add_id(&mut c, &p, &path, "id", &font.id, "font-asset");
            }
            for sound in &d.sounds {
                add_id(&mut c, &p, &path, "id", &sound.id, "sound-asset");
            }
            for cursor in &d.cursors {
                add_id(&mut c, &p, &path, "id", &cursor.id, "cursor-asset");
                add(&mut c, &p, "image-asset-ref", &path, "image", &cursor.image);
            }
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
        for (path, d) in a.recipes().map_err(|e| pkgerr(&dir, e))? {
            add_id(&mut c, &p, &path, "id", &d.id, "recipe");
            add(&mut c, &p, "recipe-name", &path, "name", &d.name);
            for stack in d.inputs.iter().chain(&d.outputs) {
                add_id(&mut c, &p, &path, "item", &stack.item, "item-ref");
            }
        }
        for (path, d) in a.technologies().map_err(|e| pkgerr(&dir, e))? {
            add_id(&mut c, &p, &path, "id", &d.id, "technology");
            add(&mut c, &p, "technology-name", &path, "name", &d.name);
            for prerequisite in &d.prerequisites {
                add_id(
                    &mut c,
                    &p,
                    &path,
                    "prerequisites",
                    prerequisite,
                    "technology-ref",
                );
            }
            for cost in &d.costs {
                add_id(
                    &mut c,
                    &p,
                    &path,
                    "cost.resource",
                    &cost.resource,
                    "resource-ref",
                );
            }
            for effect in &d.effects {
                match effect {
                    turts_mod_api::EffectDef::WeaponModifier { unit, .. } => {
                        add_id(&mut c, &p, &path, "effects.unit", unit, "unit-ref");
                    }
                    turts_mod_api::EffectDef::ArmourModifier { unit, stat, .. } => {
                        add_id(&mut c, &p, &path, "effects.unit", unit, "unit-ref");
                        if let turts_mod_api::ArmourStat::FlatByType(damage_type) = stat {
                            add_id(
                                &mut c,
                                &p,
                                &path,
                                "effects.stat.FlatByType",
                                damage_type,
                                "damage-type-ref",
                            );
                        }
                    }
                }
            }
        }
        for (path, definition) in a.visual_effect_defs().map_err(|e| pkgerr(&dir, e))? {
            for effect in definition.effects {
                // Effect slots can use generic API words such as hit or release.
                // Qualified references identify package content without banning those words.
                add(
                    &mut c,
                    &p,
                    "visual-effect",
                    &path,
                    "effects.id",
                    &format!("{p}:{}", effect.id),
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
    Ok(c)
}
fn validate_map_references(
    dir: &Path,
    archive: &PackageArchive,
    content: &ContentCatalog,
) -> Result<(), ScanError> {
    let package = archive.manifest.id.as_str();
    for (file, map) in archive.maps().map_err(|e| pkgerr(dir, e))? {
        for (index, placement) in map.placements.iter().enumerate() {
            let id = ContentId::parse(&placement.unit)
                .map_err(|e| pkgerr(dir, format!("{file}: placements[{index}].unit: {e}")))?;
            let unit = content.units.get(&id).ok_or_else(|| {
                pkgerr(
                    dir,
                    format!("{file}: placements[{index}].unit: unknown unit `{id}` in `{package}`"),
                )
            })?;
            if !placement.contents.is_empty() && unit.inventory.is_none() {
                return Err(pkgerr(
                    dir,
                    format!("{file}: placements[{index}].contents: `{id}` has no inventory"),
                ));
            }
            for (item_index, stack) in placement.contents.iter().enumerate() {
                let item = ContentId::parse(&stack.item).map_err(|e| {
                    pkgerr(
                        dir,
                        format!("{file}: placements[{index}].contents[{item_index}].item: {e}"),
                    )
                })?;
                let resource = content.resources.get(&item).ok_or_else(|| pkgerr(dir, format!("{file}: placements[{index}].contents[{item_index}].item: unknown resource `{item}`")))?;
                if resource.kind != ResourceKind::Item {
                    return Err(pkgerr(
                        dir,
                        format!(
                            "{file}: placements[{index}].contents[{item_index}].item: `{item}` is not an item resource"
                        ),
                    ));
                }
                if !unit
                    .inventory
                    .as_ref()
                    .is_some_and(|inventory| inventory.accepts.contains(&item))
                {
                    return Err(pkgerr(
                        dir,
                        format!(
                            "{file}: placements[{index}].contents[{item_index}].item: `{id}` does not accept `{item}`"
                        ),
                    ));
                }
            }
        }
        for (index, placement) in map.nodes.iter().enumerate() {
            let id = ContentId::parse(&placement.node)
                .map_err(|e| pkgerr(dir, format!("{file}: nodes[{index}].node: {e}")))?;
            if !content.nodes.contains_key(&id) {
                return Err(pkgerr(
                    dir,
                    format!("{file}: nodes[{index}].node: unknown node `{id}` in `{package}`"),
                ));
            }
        }
        for (slot_index, seed) in map.slot_economy.iter().enumerate() {
            for (balance_index, amount) in seed.balances.iter().enumerate() {
                let id = ContentId::parse(&amount.balance).map_err(|e| pkgerr(dir, format!("{file}: slot_economy[{slot_index}].balances[{balance_index}].balance: {e}")))?;
                let resource = content.resources.get(&id).ok_or_else(|| pkgerr(dir, format!("{file}: slot_economy[{slot_index}].balances[{balance_index}].balance: unknown resource `{id}`")))?;
                if resource.kind != ResourceKind::Balance {
                    return Err(pkgerr(
                        dir,
                        format!(
                            "{file}: slot_economy[{slot_index}].balances[{balance_index}].balance: `{id}` is not a balance resource"
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}
fn validate_visual_references(
    dir: &Path,
    archive: &PackageArchive,
    content: &ContentCatalog,
) -> Result<(), ScanError> {
    let package = archive.manifest.id.as_str();
    for (file, visual) in archive.visuals().map_err(|e| pkgerr(dir, e))? {
        let (target, expected_kind, exists) = match &visual.target {
            VisualTarget::Unit(text) => {
                let id = ContentId::parse(text)
                    .map_err(|e| pkgerr(dir, format!("{file}: target: {e}")))?;
                (id.to_string(), "unit", content.units.contains_key(&id))
            }
            VisualTarget::Node(text) => {
                let id = ContentId::parse(text)
                    .map_err(|e| pkgerr(dir, format!("{file}: target: {e}")))?;
                (id.to_string(), "node", content.nodes.contains_key(&id))
            }
        };
        if !exists {
            return Err(pkgerr(
                dir,
                format!("{file}: visual target `{target}` is not a {expected_kind} in `{package}`"),
            ));
        }
    }
    Ok(())
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
    c.records.entry(t.into()).or_default().push(TokenRecord {
        token: t.into(),
        category: cat.into(),
        package: p.into(),
        definition_path: path.into(),
        field: field.into(),
    });
}
fn add_id(c: &mut TokenCatalog, p: &str, path: &str, field: &str, id: &str, cat: &str) {
    let (qualified, local) = match id.split_once(':') {
        Some((_, local)) => (id.to_owned(), local),
        None => (format!("{p}:{id}"), id),
    };
    add(c, p, cat, path, field, &qualified);
    add(c, p, cat, path, field, local);
}

fn add_unit(c: &mut TokenCatalog, p: &str, path: &str, d: &UnitDef) {
    add_id(c, p, path, "id", &d.id, "unit");
    add(c, p, "unit-name", path, "name", &d.name);
    if let Some(class) = &d.terrain_class {
        add_id(
            c,
            p,
            path,
            "terrain_class",
            class,
            "terrain-movement-class-ref",
        );
    }
    for tool in &d.terrain_tools {
        add_id(c, p, path, "terrain_tools", tool, "terrain-tool-ref");
    }
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
    let mut skipped = Vec::new();
    for r in roots {
        collect(r, &mut fslist, &mut skipped)?
    }
    let test_files = verified_test_modules(&fslist, &skipped)?;
    fslist.retain(|file| {
        !test_files.contains(&fs::canonicalize(file).unwrap_or_else(|_| file.clone()))
    });
    fslist.sort();
    let mut hits = Vec::new();
    for p in fslist {
        let s = fs::read_to_string(&p).map_err(|e| ScanError::Io(p.clone(), e))?;
        let f = syn::parse_file(&s).map_err(|e| ScanError::Rust(p.clone(), e.to_string()))?;
        if !production_attrs(&f.attrs) {
            continue;
        }
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
/// The production source files under `roots`, sorted: the files that `scan_rust` reads (no
/// `tests` directory, no test-only module file; real declarations and explicit paths
/// determine whether a source can be included in production).
pub fn production_sources(roots: &[PathBuf]) -> Result<Vec<PathBuf>, ScanError> {
    let mut fslist = Vec::new();
    let mut skipped = Vec::new();
    for r in roots {
        collect(r, &mut fslist, &mut skipped)?
    }
    let test_files = verified_test_modules(&fslist, &skipped)?;
    fslist.retain(|file| {
        !test_files.contains(&fs::canonicalize(file).unwrap_or_else(|_| file.clone()))
    });
    fslist.sort();
    let mut production = Vec::new();
    for file in fslist {
        let source =
            fs::read_to_string(&file).map_err(|error| ScanError::Io(file.clone(), error))?;
        let parsed = syn::parse_file(&source)
            .map_err(|error| ScanError::Rust(file.clone(), error.to_string()))?;
        if production_attrs(&parsed.attrs) {
            production.push(file);
        }
    }
    Ok(production)
}
/// Test modules in their own file. The parent file declares each one with `#[cfg(test)] mod`,
/// so the scan skips the file as it skips an inline `#[cfg(test)]` module.
/// Resolve real Rust module declarations once. A test inclusion cannot hide a
/// second production inclusion of the same source file.
#[derive(Clone)]
struct ModuleDeclaration {
    parent: PathBuf,
    name: String,
    production: bool,
    inline: bool,
}
fn verified_test_modules(
    files: &[PathBuf],
    conventional: &[PathBuf],
) -> Result<BTreeSet<PathBuf>, ScanError> {
    fn explicit_path(attributes: &[syn::Attribute]) -> Option<String> {
        attributes.iter().find_map(|attribute| {
            if attribute.path().is_ident("path")
                && let syn::Meta::NameValue(value) = &attribute.meta
                && let Expr::Lit(literal) = &value.value
                && let Lit::Str(path) = &literal.lit
            {
                Some(path.value())
            } else {
                None
            }
        })
    }
    fn declarations(
        items: &[Item],
        parent: &Path,
        default_dir: &Path,
        explicit_dir: &Path,
        production: bool,
        out: &mut BTreeMap<PathBuf, Vec<ModuleDeclaration>>,
    ) -> Result<(), ScanError> {
        for item in items {
            let Item::Mod(module) = item else {
                continue;
            };
            let production = production && production_attrs(&module.attrs);
            let name = normalize_ident(&module.ident);
            let explicit = explicit_path(&module.attrs).map(|path| explicit_dir.join(path));
            let target = explicit.clone().unwrap_or_else(|| {
                let flat = default_dir.join(format!("{name}.rs"));
                if flat.is_file() {
                    flat
                } else {
                    default_dir.join(&name).join("mod.rs")
                }
            });
            if target.is_file() {
                let target =
                    fs::canonicalize(&target).map_err(|error| ScanError::Io(target, error))?;
                out.entry(target).or_default().push(ModuleDeclaration {
                    parent: parent.into(),
                    name: name.clone(),
                    production,
                    inline: module.content.is_some(),
                });
            }
            if let Some((_, items)) = &module.content {
                let nested = explicit.unwrap_or_else(|| default_dir.join(&name));
                declarations(items, parent, &nested, &nested, production, out)?;
            }
        }
        Ok(())
    }
    let mut intrinsic = BTreeSet::new();
    let mut registry = BTreeMap::<PathBuf, Vec<ModuleDeclaration>>::new();
    let all = files
        .iter()
        .chain(conventional)
        .cloned()
        .collect::<BTreeSet<_>>();
    for file in all {
        let parent = fs::canonicalize(&file).map_err(|error| ScanError::Io(file.clone(), error))?;
        let source =
            fs::read_to_string(&parent).map_err(|error| ScanError::Io(parent.clone(), error))?;
        let parsed = syn::parse_file(&source)
            .map_err(|error| ScanError::Rust(parent.clone(), error.to_string()))?;
        let production = production_attrs(&parsed.attrs);
        if !production {
            intrinsic.insert(parent.clone());
        }
        let directory = parent.parent().unwrap();
        let default_dir = if parent
            .file_stem()
            .is_some_and(|stem| matches!(stem.to_str(), Some("lib" | "main" | "mod")))
        {
            directory.to_path_buf()
        } else {
            parent.with_extension("")
        };
        declarations(
            &parsed.items,
            &parent,
            &default_dir,
            directory,
            production,
            &mut registry,
        )?;
    }
    let mut only_test = intrinsic.clone();
    loop {
        let mut changed = false;
        for (target, entries) in &registry {
            let imports = entries
                .iter()
                .filter(|entry| !entry.inline)
                .collect::<Vec<_>>();
            if !imports.is_empty()
                && imports
                    .iter()
                    .all(|entry| !entry.production || only_test.contains(&entry.parent))
            {
                changed |= only_test.insert(target.clone());
            }
        }
        if !changed {
            break;
        }
    }
    for file in conventional {
        let canonical =
            fs::canonicalize(file).map_err(|error| ScanError::Io(file.clone(), error))?;
        let stem = file
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("");
        let fallback = file.parent().unwrap_or(Path::new("")).with_extension("rs");
        let entries = registry.get(&canonical).ok_or_else(|| {
            ScanError::TestModule(file.clone(), fallback, format!("declares no `mod {stem}`"))
        })?;
        for entry in entries {
            let why = if entry.inline {
                Some(format!("`mod {}` has an inline body", entry.name))
            } else if entry.production && !only_test.contains(&entry.parent) {
                Some(format!(
                    "`mod {}` is not gated by `#[cfg(test)]`",
                    entry.name
                ))
            } else {
                None
            };
            if let Some(why) = why {
                return Err(ScanError::TestModule(
                    file.clone(),
                    entry.parent.clone(),
                    why,
                ));
            }
        }
    }
    Ok(only_test)
}
fn collect(p: &Path, out: &mut Vec<PathBuf>, skipped: &mut Vec<PathBuf>) -> Result<(), ScanError> {
    if p.is_file() {
        if p.extension().is_some_and(|x| x == "rs")
            && !p.components().any(|x| x.as_os_str() == "tests")
        {
            let source = fs::read_to_string(p).map_err(|error| ScanError::Io(p.into(), error))?;
            let parsed = syn::parse_file(&source)
                .map_err(|error| ScanError::Rust(p.into(), error.to_string()))?;
            if !production_attrs(&parsed.attrs) {
                out.push(p.into());
                return Ok(());
            }
            let stem = p.file_stem().and_then(|stem| stem.to_str()).unwrap_or("");
            let contains_production = parsed
                .items
                .iter()
                .any(|item| production_attrs(item_attrs(item)));
            if contains_production && (stem == "tests" || stem.ends_with("_tests")) {
                skipped.push(p.into())
            } else {
                out.push(p.into())
            }
        }
        return Ok(());
    }
    for e in fs::read_dir(p).map_err(|e| ScanError::Io(p.into(), e))? {
        let q = e.map_err(|e| ScanError::Io(p.into(), e))?.path();
        if q.is_dir() && q.file_name().is_some_and(|x| x == "tests") {
            continue;
        }
        collect(&q, out, skipped)?
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
        // Bare names and display words are ambiguous ordinary Rust/game vocabulary.
        // Inspect only exact qualified IDs and explicit exported file selectors.
        if !(t.contains(':') || t.contains('/') || t.ends_with(".tar") || t.ends_with(".ron")) {
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
                TokenTree::Ident(_) => (),
                TokenTree::Literal(l) => self.literal(&l),
                TokenTree::Group(g) => self.stream(g.stream()),
                TokenTree::Punct(_) => (),
            }
        }
    }
    fn literal(&mut self, l: &Literal) {
        if let Ok(s) = syn::parse_str::<syn::LitStr>(&l.to_string()) {
            self.check(&s.value(), l.span())
        } else if let Ok(s) = syn::parse_str::<syn::LitByteStr>(&l.to_string())
            && let Ok(x) = std::str::from_utf8(&s.value())
        {
            self.check(x, l.span())
        }
    }
}
impl<'a> Visit<'a> for Scanner<'_> {
    fn visit_field(&mut self, field: &'a syn::Field) {
        if !production_cfg(&field.attrs) {
            return;
        }
        // A generic record member can share any mod's local content name.
        // Keep checking its attributes and type, not its field spelling.
        for attribute in &field.attrs {
            self.visit_attribute(attribute);
        }
        self.visit_visibility(&field.vis);
        self.visit_type(&field.ty);
    }
    fn visit_member(&mut self, _member: &'a syn::Member) {
        // Member access and struct initializers do not dispatch on content IDs.
        // Their value expressions are still visited by the ordinary traversal.
    }
    fn visit_item(&mut self, i: &'a Item) {
        if !production_cfg(item_attrs(i)) {
            return;
        }
        syn::visit::visit_item(self, i)
    }
    fn visit_stmt(&mut self, s: &'a Stmt) {
        let attrs = match s {
            Stmt::Local(x) => &x.attrs,
            Stmt::Item(_) | Stmt::Expr(_, _) => &[][..],
            Stmt::Macro(x) => &x.attrs,
        };
        if production_cfg(attrs) {
            syn::visit::visit_stmt(self, s);
        }
    }
    fn visit_expr(&mut self, e: &'a Expr) {
        if production_cfg(expr_attrs(e)) {
            syn::visit::visit_expr(self, e);
        }
    }
    fn visit_lit(&mut self, l: &'a Lit) {
        match l {
            Lit::Str(s) => self.check(&s.value(), s.span()),
            Lit::ByteStr(s) => {
                if let Ok(text) = std::str::from_utf8(&s.value()) {
                    self.check(text, s.span())
                }
            }
            _ => (),
        }
        syn::visit::visit_lit(self, l)
    }
    fn visit_macro(&mut self, m: &'a Macro) {
        self.visit_path(&m.path);
        self.stream(m.tokens.clone())
    }
}
fn normalize_ident(ident: &syn::Ident) -> String {
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
fn expr_attrs(expr: &Expr) -> &[syn::Attribute] {
    match expr {
        Expr::Array(x) => &x.attrs,
        Expr::Assign(x) => &x.attrs,
        Expr::Async(x) => &x.attrs,
        Expr::Await(x) => &x.attrs,
        Expr::Binary(x) => &x.attrs,
        Expr::Block(x) => &x.attrs,
        Expr::Break(x) => &x.attrs,
        Expr::Call(x) => &x.attrs,
        Expr::Cast(x) => &x.attrs,
        Expr::Closure(x) => &x.attrs,
        Expr::Const(x) => &x.attrs,
        Expr::Continue(x) => &x.attrs,
        Expr::Field(x) => &x.attrs,
        Expr::ForLoop(x) => &x.attrs,
        Expr::Group(x) => &x.attrs,
        Expr::If(x) => &x.attrs,
        Expr::Index(x) => &x.attrs,
        Expr::Infer(x) => &x.attrs,
        Expr::Let(x) => &x.attrs,
        Expr::Lit(x) => &x.attrs,
        Expr::Loop(x) => &x.attrs,
        Expr::Macro(x) => &x.attrs,
        Expr::Match(x) => &x.attrs,
        Expr::MethodCall(x) => &x.attrs,
        Expr::Paren(x) => &x.attrs,
        Expr::Path(x) => &x.attrs,
        Expr::Range(x) => &x.attrs,
        Expr::RawAddr(x) => &x.attrs,
        Expr::Reference(x) => &x.attrs,
        Expr::Repeat(x) => &x.attrs,
        Expr::Return(x) => &x.attrs,
        Expr::Struct(x) => &x.attrs,
        Expr::Try(x) => &x.attrs,
        Expr::TryBlock(x) => &x.attrs,
        Expr::Tuple(x) => &x.attrs,
        Expr::Unary(x) => &x.attrs,
        Expr::Unsafe(x) => &x.attrs,
        Expr::While(x) => &x.attrs,
        Expr::Yield(x) => &x.attrs,
        Expr::Verbatim(_) => &[],
        _ => &[],
    }
}
#[derive(Clone, Copy)]
struct CfgPossibilities {
    can_be_false: bool,
    can_be_true: bool,
}
fn production_cfg(attrs: &[syn::Attribute]) -> bool {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("cfg"))
        .all(|attr| {
            attr.parse_args::<syn::Meta>()
                .map(cfg_possibilities)
                .unwrap_or(CfgPossibilities {
                    can_be_false: true,
                    can_be_true: true,
                })
                .can_be_true
        })
}
pub fn production_attrs(attrs: &[syn::Attribute]) -> bool {
    production_cfg(attrs)
}
pub fn production_expr(expr: &Expr) -> bool {
    production_cfg(expr_attrs(expr))
}
fn cfg_possibilities(meta: syn::Meta) -> CfgPossibilities {
    use syn::Meta;
    match meta {
        Meta::Path(path) if path.is_ident("test") => CfgPossibilities {
            can_be_false: true,
            can_be_true: false,
        },
        Meta::Path(_) | Meta::NameValue(_) => CfgPossibilities {
            can_be_false: true,
            can_be_true: true,
        },
        Meta::List(list) if list.path.is_ident("not") => {
            let nested =
                list.parse_args::<Meta>()
                    .map(cfg_possibilities)
                    .unwrap_or(CfgPossibilities {
                        can_be_false: true,
                        can_be_true: true,
                    });
            CfgPossibilities {
                can_be_false: nested.can_be_true,
                can_be_true: nested.can_be_false,
            }
        }
        Meta::List(list) if list.path.is_ident("all") || list.path.is_ident("any") => {
            let all = list.path.is_ident("all");
            let args = list.parse_args_with(
                syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
            );
            let Ok(args) = args else {
                return CfgPossibilities {
                    can_be_false: true,
                    can_be_true: true,
                };
            };
            let values = args.into_iter().map(cfg_possibilities).collect::<Vec<_>>();
            if all {
                CfgPossibilities {
                    can_be_false: values.iter().any(|v| v.can_be_false),
                    can_be_true: values.iter().all(|v| v.can_be_true),
                }
            } else {
                CfgPossibilities {
                    can_be_false: values.iter().all(|v| v.can_be_false),
                    can_be_true: values.iter().any(|v| v.can_be_true),
                }
            }
        }
        Meta::List(_) => CfgPossibilities {
            can_be_false: true,
            can_be_true: true,
        },
    }
}
