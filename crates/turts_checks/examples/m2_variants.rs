use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};
use turts_mod_api::{
    BalanceAmount, ContentId, Cost, DropOffEffect, GathererDef, InventoryDef, ItemStack, Manifest,
    MapDef, NodeDef, NodePlacement, PaymentPolicy, Placement, ProducerDef, ProductionDef,
    ResourceDef, UnitDef, VisualDef, VisualTarget,
};
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir, visuals::visuals_by_content};

type IdKey = (String, String, String);
type NameKey = (String, String, String, String);
#[derive(Default)]
struct Plan {
    packages: BTreeMap<String, String>,
    ids: BTreeMap<IdKey, String>,
    names: BTreeMap<NameKey, String>,
    old_names: BTreeMap<NameKey, String>,
    paths: BTreeMap<(String, String), String>,
}
#[derive(Serialize)]
struct Mapping {
    version: u32,
    seed: i64,
    mode: String,
    payment: String,
    packages: BTreeMap<String, PackageRecord>,
    definitions: Vec<DefinitionRecord>,
    names: Vec<NameRecord>,
    paths: Vec<PathRecord>,
    unchanged_assets: Vec<AssetRecord>,
}
#[derive(Serialize)]
struct PackageRecord {
    old: String,
    new: String,
    old_package_path: String,
    new_package_path: String,
    old_archive_path: String,
    new_archive_path: String,
    old_versioned_archive_path: String,
    new_versioned_archive_path: String,
}
#[derive(Serialize)]
struct DefinitionRecord {
    package: String,
    category: String,
    definition_path: String,
    old_local_id: String,
    new_local_id: String,
    old_full_id: Option<String>,
    new_full_id: Option<String>,
}
#[derive(Serialize)]
struct NameRecord {
    package: String,
    category: String,
    definition_path: String,
    field: String,
    old: String,
    new: String,
}
#[derive(Serialize)]
struct PathRecord {
    package: String,
    category: String,
    old_export_path: String,
    new_export_path: String,
    old_package_path: String,
    new_package_path: String,
    old_archive_path: String,
    new_archive_path: String,
}
#[derive(Serialize)]
struct AssetRecord {
    package: String,
    new_package: String,
    old_path: String,
    new_path: String,
    unchanged: bool,
}
struct Input {
    archive: PackageArchive,
    files: BTreeMap<String, Vec<u8>>,
}
struct Args {
    source: PathBuf,
    out: PathBuf,
    seed: i64,
    mode: String,
    payment: String,
}
fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("m2_variants: {e}");
            ExitCode::FAILURE
        }
    }
}
fn run(raw: Vec<String>) -> Result<(), String> {
    let a = args(&raw)?;
    if a.mode != "rename" {
        return Err(format!(
            "unsupported mode `{}`; only rename is supported",
            a.mode
        ));
    }
    let policy = match a.payment.as_str() {
        "charge" => PaymentPolicy::ChargeOnQueue,
        "reserve" => PaymentPolicy::ReserveThenCharge,
        _ => return Err("payment must be charge or reserve".into()),
    };
    let source = a
        .source
        .canonicalize()
        .map_err(|e| format!("source: {e}"))?;
    if !source.is_dir() {
        return Err("source is not a directory".into());
    }
    let out_lex = abs_lex(&a.out)?;
    if fs::symlink_metadata(&out_lex).is_ok() {
        return Err("output already exists".into());
    }
    let out_parent = out_lex
        .parent()
        .ok_or("output has no parent")?
        .canonicalize()
        .map_err(|e| format!("output parent: {e}"))?;
    let out = out_parent.join(out_lex.file_name().ok_or("output has no filename")?);
    if out.exists() {
        return Err("output already exists".into());
    }
    if overlaps(&source, &out) {
        return Err("source and output overlap".into());
    }
    let parent = out.parent().ok_or("output has no parent")?;
    if !parent.is_dir() {
        return Err("output parent must already exist".into());
    }
    let stage = parent.join(format!(
        ".{}.m2-stage-{}",
        out.file_name().and_then(|s| s.to_str()).unwrap_or("out"),
        std::process::id()
    ));
    if stage.exists() {
        return Err("staging path already exists".into());
    }
    let r = build(&source, &stage, a.seed, policy, &a.payment);
    if r.is_err() {
        let _ = fs::remove_dir_all(&stage);
    }
    r?;
    fs::rename(&stage, &out).map_err(|e| {
        let _ = fs::remove_dir_all(&stage);
        format!("publish output: {e}")
    })
}
fn args(xs: &[String]) -> Result<Args, String> {
    let mut m = BTreeMap::new();
    let mut i = 0;
    while i < xs.len() {
        let k = xs[i].as_str();
        if !matches!(k, "--source" | "--out" | "--seed" | "--mode" | "--payment") {
            return Err(format!("unknown argument {k}"));
        }
        let v = xs
            .get(i + 1)
            .ok_or_else(|| format!("missing value for {k}"))?;
        if m.insert(k, v.as_str()).is_some() {
            return Err(format!("duplicate argument {k}"));
        }
        i += 2
    }
    let get = |k| {
        m.get(k)
            .copied()
            .ok_or_else(|| format!("missing required argument {k}"))
    };
    Ok(Args {
        source: get("--source")?.into(),
        out: get("--out")?.into(),
        seed: get("--seed")?
            .parse()
            .map_err(|_| "seed must be an integer")?,
        mode: get("--mode")?.into(),
        payment: get("--payment")?.into(),
    })
}
fn abs_lex(p: &Path) -> Result<PathBuf, String> {
    let p = if p.is_absolute() {
        p.to_owned()
    } else {
        env::current_dir().map_err(|e| e.to_string())?.join(p)
    };
    let mut o = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                o.pop();
            }
            x => o.push(x.as_os_str()),
        }
    }
    Ok(o)
}
fn overlaps(a: &Path, b: &Path) -> bool {
    a.starts_with(b) || b.starts_with(a)
}
fn build(
    source: &Path,
    stage: &Path,
    seed: i64,
    policy: PaymentPolicy,
    payment: &str,
) -> Result<(), String> {
    let inputs = discover(source)?;
    if inputs.is_empty() {
        return Err("empty package source".into());
    }
    let mut pkgset = BTreeSet::new();
    for x in &inputs {
        let m = &x.archive.manifest;
        if !pkgset.insert(m.id.clone()) {
            return Err(format!("duplicate package identity {}", m.id));
        }
        if !m.dependencies.0.is_empty() {
            return Err(format!(
                "dependency-bearing package {} is unsupported",
                m.id
            ));
        }
        x.archive
            .content()
            .map_err(|e| format!("{} content: {e}", m.id))?;
        x.archive.units().map_err(|e| e.to_string())?;
        x.archive.maps().map_err(|e| e.to_string())?;
        x.archive.visuals().map_err(|e| e.to_string())?;
        x.archive.resources().map_err(|e| e.to_string())?;
        x.archive.nodes().map_err(|e| e.to_string())?;
        for (_, p) in x.archive.production().map_err(|e| e.to_string())? {
            if p.payment != policy {
                return Err(format!(
                    "{} recipe {} payment policy mismatches requested {payment}",
                    m.id, p.id
                ));
            }
        }
        if x.files
            .keys()
            .any(|p| p.ends_with(".ron") && p != "manifest.ron" && !exported(m, p))
        {
            return Err(format!("{} has unexported RON", m.id));
        }
    }
    let source_archives = inputs
        .iter()
        .map(|input| input.archive.clone())
        .collect::<Vec<_>>();
    visuals_by_content(&source_archives)
        .map_err(|e| format!("source visual validation failed: {e}"))?;
    let plan = make_plan(&inputs, seed)?;
    fs::create_dir(stage).map_err(|e| format!("stage: {e}"))?;
    let mut defs = vec![];
    let mut names = vec![];
    let mut paths = vec![];
    let mut assets = vec![];
    let mut built = vec![];
    let mut pkgs = BTreeMap::new();
    for x in &inputs {
        let old = &x.archive.manifest.id;
        let new = &plan.packages[old];
        pkgs.insert(
            old.clone(),
            PackageRecord {
                old: old.clone(),
                new: new.clone(),
                old_package_path: format!("mods/{old}"),
                new_package_path: format!("mods/{new}"),
                old_archive_path: format!("{old}.tar"),
                new_archive_path: format!("{new}.tar"),
                old_versioned_archive_path: format!("{old}-{}.tar", x.archive.manifest.version),
                new_versioned_archive_path: format!("{new}-{}.tar", x.archive.manifest.version),
            },
        );
        let dest = stage.join(new);
        fs::create_dir(&dest).map_err(|e| e.to_string())?;
        let manifest = rename_manifest(&x.archive.manifest, old, &plan)?;
        let mb = ron::ser::to_string_pretty(
            &ManifestOutput::from(&manifest),
            ron::ser::PrettyConfig::default(),
        )
        .map_err(|e| e.to_string())?
        .into_bytes();
        fs::write(dest.join("manifest.ron"), &mb).map_err(|e| e.to_string())?;
        let mut output_paths = BTreeSet::from(["manifest.ron".to_owned()]);
        for (p, b) in &x.files {
            if p == "manifest.ron" {
                continue;
            }
            if exported(&x.archive.manifest, p) {
                let np = plan
                    .paths
                    .get(&(old.clone(), p.clone()))
                    .ok_or("missing path mapping")?;
                if !output_paths.insert(np.clone()) {
                    return Err(format!("generated path collision `{np}`"));
                }
                let bytes = transform_export(x, p, &plan, &mut defs, &mut names)?;
                let target = dest.join(np);
                if let Some(par) = target.parent() {
                    fs::create_dir_all(par).map_err(|e| e.to_string())?
                }
                fs::write(&target, &bytes).map_err(|e| e.to_string())?;
                paths.push(PathRecord {
                    package: old.clone(),
                    category: category(&x.archive.manifest, p).unwrap_or("unknown").into(),
                    old_export_path: p.clone(),
                    new_export_path: np.clone(),
                    old_package_path: format!("mods/{old}/{p}"),
                    new_package_path: format!("mods/{new}/{np}"),
                    old_archive_path: format!("{old}/{p}"),
                    new_archive_path: format!("{new}/{np}"),
                })
            } else {
                if !output_paths.insert(p.clone()) {
                    return Err(format!("generated asset path collision `{p}`"));
                }
                assets.push(AssetRecord {
                    package: old.clone(),
                    new_package: new.clone(),
                    old_path: p.clone(),
                    new_path: p.clone(),
                    unchanged: true,
                });
                let target = dest.join(p);
                if let Some(par) = target.parent() {
                    fs::create_dir_all(par).map_err(|e| e.to_string())?
                }
                fs::write(target, b).map_err(|e| e.to_string())?
            }
        }
        let packed = pack_dir(&dest).map_err(|e| e.to_string())?;
        let parsed =
            PackageArchive::parse(&packed).map_err(|e| format!("generated archive parse: {e}"))?;
        validate(&parsed)?;
        built.push(parsed)
    }
    visuals_by_content(&built).map_err(|e| format!("visual validation: {e}"))?;
    defs.sort_by(|a, b| {
        (&a.package, &a.category, &a.definition_path).cmp(&(
            &b.package,
            &b.category,
            &b.definition_path,
        ))
    });
    names.sort_by(|a, b| {
        (&a.package, &a.category, &a.definition_path, &a.field).cmp(&(
            &b.package,
            &b.category,
            &b.definition_path,
            &b.field,
        ))
    });
    paths.sort_by(|a, b| (&a.package, &a.old_export_path).cmp(&(&b.package, &b.old_export_path)));
    assets.sort_by(|a, b| (&a.package, &a.old_path).cmp(&(&b.package, &b.old_path)));
    let map = Mapping {
        version: 1,
        seed,
        mode: "rename".into(),
        payment: payment.into(),
        packages: pkgs,
        definitions: defs,
        names,
        paths,
        unchanged_assets: assets,
    };
    let mut json = serde_json::to_vec_pretty(&map).map_err(|e| e.to_string())?;
    json.push(b'\n');
    fs::write(stage.join("mapping.json"), json).map_err(|e| e.to_string())?;
    Ok(())
}
fn discover(s: &Path) -> Result<Vec<Input>, String> {
    let dirs = if s.join("manifest.ron").is_file() {
        vec![s.to_owned()]
    } else {
        let mut d = vec![];
        for e in fs::read_dir(s).map_err(|e| e.to_string())? {
            let e = e.map_err(|e| e.to_string())?;
            let t = e.file_type().map_err(|e| e.to_string())?;
            if t.is_symlink() {
                return Err("symlink input unsupported".into());
            }
            if t.is_dir() {
                if !e.path().join("manifest.ron").is_file() {
                    return Err(format!("non-package source child {}", e.path().display()));
                }
                d.push(e.path())
            } else {
                return Err("source must contain package directories".into());
            }
        }
        d.sort();
        d
    };
    let mut out = vec![];
    for dir in dirs {
        let mut files = BTreeMap::new();
        collect(&dir, &dir, &mut files)?;
        let bytes = pack_dir(&dir).map_err(|e| e.to_string())?;
        let archive = PackageArchive::parse(&bytes).map_err(|e| e.to_string())?;
        out.push(Input { archive, files })
    }
    out.sort_by(|a, b| a.archive.manifest.id.cmp(&b.archive.manifest.id));
    Ok(out)
}
fn collect(root: &Path, d: &Path, out: &mut BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    for e in fs::read_dir(d).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        let p = e.path();
        let t = e.file_type().map_err(|e| e.to_string())?;
        if t.is_symlink() {
            return Err(format!("symlink input unsupported: {}", p.display()));
        }
        if t.is_dir() {
            collect(root, &p, out)?;
            continue;
        }
        if !t.is_file() {
            return Err(format!("non-regular input: {}", p.display()));
        }
        let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if n == "Cargo.toml" || p.extension().is_some_and(|e| e == "rs") {
            return Err(format!("native input unsupported: {}", p.display()));
        }
        let rel = p
            .strip_prefix(root)
            .map_err(|e| e.to_string())?
            .to_str()
            .ok_or("non-UTF8 path")?
            .replace('\\', "/");
        out.insert(rel, fs::read(p).map_err(|e| e.to_string())?);
    }
    Ok(())
}
fn exported(m: &Manifest, p: &str) -> bool {
    m.exports
        .units
        .iter()
        .chain(&m.exports.maps)
        .chain(&m.exports.visuals)
        .chain(&m.exports.resources)
        .chain(&m.exports.nodes)
        .chain(&m.exports.production)
        .any(|x| x == p)
}
fn category(m: &Manifest, p: &str) -> Option<&'static str> {
    [
        (&m.exports.units, "units"),
        (&m.exports.maps, "maps"),
        (&m.exports.visuals, "visuals"),
        (&m.exports.resources, "resources"),
        (&m.exports.nodes, "nodes"),
        (&m.exports.production, "production"),
    ]
    .into_iter()
    .find_map(|(xs, c)| xs.iter().any(|x| x == p).then_some(c))
}
fn hash(seed: i64, p: &str, c: &str, x: &str, f: &str) -> String {
    let mut h = 0xcbf29ce484222325u64;
    for b in seed
        .to_be_bytes()
        .into_iter()
        .chain([p, c, x, f].into_iter().flat_map(|s| {
            (s.len() as u64)
                .to_be_bytes()
                .into_iter()
                .chain(s.as_bytes().iter().copied())
        }))
    {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x100000001b3)
    }
    format!("{h:016x}")
}
fn make_plan(xs: &[Input], seed: i64) -> Result<Plan, String> {
    let mut z = Plan::default();
    let mut package_ids = BTreeSet::new();
    let mut output_ids = BTreeSet::new();
    let mut output_names = BTreeSet::new();
    for x in xs {
        let p = x.archive.manifest.id.as_str();
        let np = format!("p{}", hash(seed, p, "package", p, "id"));
        if !package_ids.insert(np.clone()) {
            return Err("package collision".into());
        }
        z.packages.insert(p.into(), np);
        let groups: Vec<(&str, Vec<(String, String)>)> = vec![
            (
                "units",
                x.archive
                    .units()
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|(p, v)| (p, v.id))
                    .collect(),
            ),
            (
                "resources",
                x.archive
                    .resources()
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|(p, v)| (p, v.id))
                    .collect(),
            ),
            (
                "nodes",
                x.archive
                    .nodes()
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|(p, v)| (p, v.id))
                    .collect(),
            ),
            (
                "production",
                x.archive
                    .production()
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|(p, v)| (p, v.id))
                    .collect(),
            ),
            (
                "maps",
                x.archive
                    .maps()
                    .map_err(|e| e.to_string())?
                    .into_iter()
                    .map(|(p, v)| (p, v.id))
                    .collect(),
            ),
        ];
        for (c, rows) in groups {
            for (path, local) in rows {
                let n = format!("v_{}", hash(seed, p, c, &path, &local));
                if !output_ids.insert((c.to_string(), z.packages[p].clone(), n.clone())) {
                    return Err("definition collision".into());
                }
                z.ids.insert((p.into(), c.into(), local), n.clone());
                let np = Path::new(&path)
                    .parent()
                    .unwrap_or(Path::new(""))
                    .join(format!("{n}.ron"))
                    .to_string_lossy()
                    .replace('\\', "/");
                if z.paths.values().any(|old| old == &np) {
                    return Err("path mapping collision".into());
                }
                z.paths.insert((p.into(), path), np);
            }
        }
        for path in &x.archive.manifest.exports.visuals {
            let n = format!("v_{}.ron", hash(seed, p, "visuals", path, "path"));
            let np = Path::new(path)
                .parent()
                .unwrap_or(Path::new(""))
                .join(n)
                .to_string_lossy()
                .replace('\\', "/");
            if z.paths.values().any(|old| old == &np) {
                return Err("path mapping collision".into());
            }
            z.paths.insert((p.into(), path.clone()), np);
        }
        for (path, v) in x.archive.units().map_err(|e| e.to_string())? {
            put_name(
                &mut z,
                &mut output_names,
                seed,
                p,
                "units",
                &path,
                "name",
                &v.name,
            )?;
        }
        for (path, v) in x.archive.resources().map_err(|e| e.to_string())? {
            put_name(
                &mut z,
                &mut output_names,
                seed,
                p,
                "resources",
                &path,
                "name",
                &v.name,
            )?;
            put_name(
                &mut z,
                &mut output_names,
                seed,
                p,
                "resources",
                &path,
                "unit_name",
                &v.unit_name,
            )?;
        }
        for (path, v) in x.archive.nodes().map_err(|e| e.to_string())? {
            put_name(
                &mut z,
                &mut output_names,
                seed,
                p,
                "nodes",
                &path,
                "name",
                &v.name,
            )?;
        }
        for (path, v) in x.archive.production().map_err(|e| e.to_string())? {
            put_name(
                &mut z,
                &mut output_names,
                seed,
                p,
                "production",
                &path,
                "name",
                &v.name,
            )?;
        }
    }
    Ok(z)
}
fn put_name(
    z: &mut Plan,
    seen: &mut BTreeSet<String>,
    seed: i64,
    p: &str,
    c: &str,
    path: &str,
    field: &str,
    old: &str,
) -> Result<(), String> {
    let n = format!("Name_{}", hash(seed, p, c, path, field));
    if !seen.insert(format!("{c}:{n}")) {
        return Err("name mapping collision".into());
    }
    z.names
        .insert((p.into(), c.into(), path.into(), field.into()), n);
    z.old_names
        .insert((p.into(), c.into(), path.into(), field.into()), old.into());
    Ok(())
}
fn ref_id(z: &Plan, c: &str, v: &str) -> Result<String, String> {
    let id = ContentId::parse(v).map_err(|e| e.to_string())?;
    let p = z
        .packages
        .get(&id.package.0)
        .ok_or_else(|| format!("dangling package ref {v}"))?;
    let l = z
        .ids
        .get(&(id.package.0, c.into(), id.local.clone()))
        .ok_or_else(|| format!("dangling {c} ref {v}"))?;
    Ok(format!("{p}:{l}"))
}
fn local(z: &Plan, p: &str, c: &str, v: &str) -> Result<String, String> {
    z.ids
        .get(&(p.into(), c.into(), v.into()))
        .cloned()
        .ok_or_else(|| format!("missing {c} id mapping {p}:{v}"))
}
fn name(z: &Plan, p: &str, c: &str, path: &str, f: &str) -> Result<String, String> {
    z.names
        .get(&(p.into(), c.into(), path.into(), f.into()))
        .cloned()
        .ok_or_else(|| format!("missing name mapping {p}/{path}/{f}"))
}
fn newpath(z: &Plan, p: &str, path: &str) -> Result<String, String> {
    z.paths
        .get(&(p.into(), path.into()))
        .cloned()
        .ok_or_else(|| "missing path mapping".into())
}
#[derive(Serialize)]
struct ManifestOutput<'a> {
    id: &'a str,
    version: &'a str,
    kind: turts_mod_api::PackageKind,
    engine_api: &'a str,
    dependencies: &'a BTreeMap<String, String>,
    exports: &'a turts_mod_api::Exports,
    context: turts_mod_api::ExecutionContext,
    capabilities: &'a Vec<String>,
    license: &'a str,
}
impl<'a> From<&'a Manifest> for ManifestOutput<'a> {
    fn from(m: &'a Manifest) -> Self {
        Self {
            id: &m.id,
            version: &m.version,
            kind: m.kind,
            engine_api: &m.engine_api,
            dependencies: &m.dependencies.0,
            exports: &m.exports,
            context: m.context,
            capabilities: &m.capabilities,
            license: &m.license,
        }
    }
}
fn rename_manifest(m: &Manifest, p: &str, z: &Plan) -> Result<Manifest, String> {
    let mut o = m.clone();
    o.id = z.packages[p].clone();
    for x in o
        .exports
        .units
        .iter_mut()
        .chain(&mut o.exports.maps)
        .chain(&mut o.exports.visuals)
        .chain(&mut o.exports.resources)
        .chain(&mut o.exports.nodes)
        .chain(&mut o.exports.production)
    {
        *x = newpath(z, p, x)?
    }
    Ok(o)
}
fn transform_export(
    x: &Input,
    path: &str,
    z: &Plan,
    defs: &mut Vec<DefinitionRecord>,
    names: &mut Vec<NameRecord>,
) -> Result<Vec<u8>, String> {
    let p = x.archive.manifest.id.as_str();
    macro_rules! simple {
        ($method:ident,$cat:literal,$fn:ident) => {
            for (f, mut v) in x.archive.$method().map_err(|e| e.to_string())? {
                if f == path {
                    let old = v.id.clone();
                    let new = local(z, p, $cat, &old)?;
                    let old_name = v.name.clone();
                    $fn(&mut v, p, path, z)?;
                    defs.push(defrec(p, $cat, path, &old, &new, Some(&z.packages[p])));
                    names.push(NameRecord {
                        package: p.into(),
                        category: $cat.into(),
                        definition_path: path.into(),
                        field: "name".into(),
                        old: old_name,
                        new: name(z, p, $cat, path, "name")?,
                    });
                    return ronbytes(&v);
                }
            }
        };
    }
    simple!(units, "units", unit);
    for (f, mut v) in x.archive.resources().map_err(|e| e.to_string())? {
        if f == path {
            let old = v.id.clone();
            let new = local(z, p, "resources", &old)?;
            let old_name = v.name.clone();
            let old_unit_name = v.unit_name.clone();
            resource(&mut v, p, path, z)?;
            defs.push(defrec(
                p,
                "resources",
                path,
                &old,
                &new,
                Some(&z.packages[p]),
            ));
            names.push(NameRecord {
                package: p.into(),
                category: "resources".into(),
                definition_path: path.into(),
                field: "name".into(),
                old: old_name,
                new: name(z, p, "resources", path, "name")?,
            });
            names.push(NameRecord {
                package: p.into(),
                category: "resources".into(),
                definition_path: path.into(),
                field: "unit_name".into(),
                old: old_unit_name,
                new: name(z, p, "resources", path, "unit_name")?,
            });
            return ronbytes(&v);
        }
    }
    simple!(nodes, "nodes", node);
    simple!(production, "production", production);
    for (f, mut v) in x.archive.maps().map_err(|e| e.to_string())? {
        if f == path {
            let old = v.id.clone();
            let new = local(z, p, "maps", &old)?;
            mapdef(&mut v, p, z)?;
            defs.push(defrec(p, "maps", path, &old, &new, None));
            return ronbytes(&v);
        }
    }
    for (f, mut v) in x.archive.visuals().map_err(|e| e.to_string())? {
        if f == path {
            visual(&mut v, z)?;
            return ronbytes(&v);
        }
    }
    Err(format!("missing export {path}"))
}
#[cfg(test)]
fn transformed_files(
    x: &Input,
    z: &Plan,
    defs: &mut Vec<DefinitionRecord>,
    names: &mut Vec<NameRecord>,
) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let p = &x.archive.manifest.id;
    let manifest = rename_manifest(&x.archive.manifest, p, z)?;
    let mut out = BTreeMap::new();
    out.insert(
        "manifest.ron".into(),
        ronbytes(&ManifestOutput::from(&manifest))?,
    );
    for (path, bytes) in &x.files {
        if path == "manifest.ron" {
            continue;
        }
        if exported(&x.archive.manifest, path) {
            out.insert(
                newpath(z, p, path)?,
                transform_export(x, path, z, defs, names)?,
            );
        } else {
            out.insert(path.clone(), bytes.clone());
        }
    }
    Ok(out)
}
#[cfg(test)]
fn inverse_plan(forward: &Plan) -> Result<Plan, String> {
    let mut r = Plan::default();
    for (old, new) in &forward.packages {
        r.packages.insert(new.clone(), old.clone());
    }
    for ((p, c, old), new) in &forward.ids {
        let np = &forward.packages[p];
        r.ids
            .insert((np.clone(), c.clone(), new.clone()), old.clone());
    }
    for ((p, c, path, field), new) in &forward.names {
        let np = &forward.packages[p];
        let npath = forward
            .paths
            .get(&(p.clone(), path.clone()))
            .ok_or("missing forward path")?;
        let key = (np.clone(), c.clone(), npath.clone(), field.clone());
        r.names.insert(
            key.clone(),
            forward
                .old_names
                .get(&(p.clone(), c.clone(), path.clone(), field.clone()))
                .ok_or("missing original name")?
                .clone(),
        );
        r.old_names.insert(key, new.clone());
    }
    for ((p, path), new) in &forward.paths {
        let np = &forward.packages[p];
        r.paths.insert((np.clone(), new.clone()), path.clone());
    }
    Ok(r)
}
fn defrec(
    p: &str,
    c: &str,
    path: &str,
    old: &str,
    new: &str,
    new_package: Option<&str>,
) -> DefinitionRecord {
    DefinitionRecord {
        package: p.into(),
        category: c.into(),
        definition_path: path.into(),
        old_local_id: old.into(),
        new_local_id: new.into(),
        old_full_id: new_package.map(|_| format!("{p}:{old}")),
        new_full_id: new_package.map(|np| format!("{np}:{new}")),
    }
}
fn ronbytes<T: serde::Serialize>(v: &T) -> Result<Vec<u8>, String> {
    ron::ser::to_string_pretty(v, ron::ser::PrettyConfig::default())
        .map(String::into_bytes)
        .map_err(|e| e.to_string())
}
fn unit(v: &mut UnitDef, p: &str, path: &str, z: &Plan) -> Result<(), String> {
    v.id = local(z, p, "units", &v.id)?;
    v.name = name(z, p, "units", path, "name")?;
    if let Some(InventoryDef { accepts, .. }) = &mut v.inventory {
        for r in accepts {
            *r = ref_id(z, "resources", r)?
        }
    }
    if let Some(GathererDef { items, .. }) = &mut v.gatherer {
        for r in items {
            *r = ref_id(z, "resources", r)?
        }
    }
    if let Some(d) = &mut v.drop_off {
        for r in &mut d.rules {
            r.item = ref_id(z, "resources", &r.item)?;
            if let DropOffEffect::Convert { balance, .. } = &mut r.effect {
                *balance = ref_id(z, "resources", balance)?
            }
        }
    }
    if let Some(ProducerDef { recipes, .. }) = &mut v.producer {
        for r in recipes {
            *r = ref_id(z, "production", r)?
        }
    }
    Ok(())
}
fn resource(v: &mut ResourceDef, p: &str, path: &str, z: &Plan) -> Result<(), String> {
    v.id = local(z, p, "resources", &v.id)?;
    v.name = name(z, p, "resources", path, "name")?;
    v.unit_name = name(z, p, "resources", path, "unit_name")?;
    Ok(())
}
fn node(v: &mut NodeDef, p: &str, path: &str, z: &Plan) -> Result<(), String> {
    v.id = local(z, p, "nodes", &v.id)?;
    v.name = name(z, p, "nodes", path, "name")?;
    v.item = ref_id(z, "resources", &v.item)?;
    Ok(())
}
fn production(v: &mut ProductionDef, p: &str, path: &str, z: &Plan) -> Result<(), String> {
    v.id = local(z, p, "production", &v.id)?;
    v.name = name(z, p, "production", path, "name")?;
    v.output_unit = ref_id(z, "units", &v.output_unit)?;
    for Cost { resource, .. } in &mut v.costs {
        *resource = ref_id(z, "resources", resource)?
    }
    Ok(())
}
fn mapdef(v: &mut MapDef, p: &str, z: &Plan) -> Result<(), String> {
    v.id = local(z, p, "maps", &v.id)?;
    for Placement { unit, contents, .. } in &mut v.placements {
        *unit = ref_id(z, "units", unit)?;
        for ItemStack { item, .. } in contents {
            *item = ref_id(z, "resources", item)?
        }
    }
    for NodePlacement { node, .. } in &mut v.nodes {
        *node = ref_id(z, "nodes", node)?
    }
    for BalanceAmount { balance, .. } in v.slot_economy.iter_mut().flat_map(|e| &mut e.balances) {
        *balance = ref_id(z, "resources", balance)?
    }
    Ok(())
}
fn visual(v: &mut VisualDef, z: &Plan) -> Result<(), String> {
    match &mut v.target {
        VisualTarget::Unit(x) => *x = ref_id(z, "units", x)?,
        VisualTarget::Node(x) => *x = ref_id(z, "nodes", x)?,
    }
    Ok(())
}
fn validate(a: &PackageArchive) -> Result<(), String> {
    let catalog = a
        .content()
        .map_err(|e| format!("content references: {e}"))?;
    for (file, map) in a.maps().map_err(|e| e.to_string())? {
        for (index, placement) in map.placements.iter().enumerate() {
            let unit_id = ContentId::parse(&placement.unit)
                .map_err(|e| format!("{file}: placements[{index}].unit: {e}"))?;
            let unit = catalog
                .units
                .get(&unit_id)
                .ok_or_else(|| format!("{file}: dangling unit reference `{unit_id}`"))?;
            if !placement.contents.is_empty() && unit.inventory.is_none() {
                return Err(format!("{file}: placed unit `{unit_id}` has no inventory"));
            }
            for (item_index, stack) in placement.contents.iter().enumerate() {
                let item = ContentId::parse(&stack.item).map_err(|e| {
                    format!("{file}: placements[{index}].contents[{item_index}].item: {e}")
                })?;
                let resource = catalog
                    .resources
                    .get(&item)
                    .ok_or_else(|| format!("{file}: dangling resource reference `{item}`"))?;
                if resource.kind != turts_mod_api::ResourceKind::Item {
                    return Err(format!("{file}: `{item}` is not an item resource"));
                }
                if !unit
                    .inventory
                    .as_ref()
                    .is_some_and(|inventory| inventory.accepts.contains(&item))
                {
                    return Err(format!("{file}: unit `{unit_id}` does not accept `{item}`"));
                }
            }
        }
        for (index, node) in map.nodes.iter().enumerate() {
            let id = ContentId::parse(&node.node)
                .map_err(|e| format!("{file}: nodes[{index}].node: {e}"))?;
            if !catalog.nodes.contains_key(&id) {
                return Err(format!("{file}: dangling node reference `{id}`"));
            }
        }
        for (slot_index, slot) in map.slot_economy.iter().enumerate() {
            for (balance_index, amount) in slot.balances.iter().enumerate() {
                let id = ContentId::parse(&amount.balance).map_err(|e| {
                    format!(
                        "{file}: slot_economy[{slot_index}].balances[{balance_index}].balance: {e}"
                    )
                })?;
                let resource = catalog
                    .resources
                    .get(&id)
                    .ok_or_else(|| format!("{file}: dangling balance reference `{id}`"))?;
                if resource.kind != turts_mod_api::ResourceKind::Balance {
                    return Err(format!("{file}: `{id}` is not a balance resource"));
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_name_hash_changes_with_seed() {
        assert_ne!(
            hash(7, "a", "units", "u", "name"),
            hash(8, "a", "units", "u", "name")
        );
        assert_eq!(
            hash(7, "a", "units", "u", "name"),
            hash(7, "a", "units", "u", "name")
        );
    }

    #[test]
    fn all_six_typed_exports_restore_after_inverse_rename() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar");
        let temp = env::temp_dir().join(format!("m2-variant-roundtrip-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        let stage = temp.join("stage");
        build(
            &root,
            &stage,
            123,
            PaymentPolicy::ReserveThenCharge,
            "reserve",
        )
        .unwrap();
        let original = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
        let original_input = discover(&root).unwrap().remove(0);
        let forward = make_plan(std::slice::from_ref(&original_input), 123).unwrap();
        let renamed_id = forward.packages["cedar"].clone();
        let renamed = discover(&stage.join(renamed_id)).unwrap().remove(0);
        let reverse = inverse_plan(&forward).unwrap();
        let mut defs = vec![];
        let mut names = vec![];
        let files = transformed_files(&renamed, &reverse, &mut defs, &mut names).unwrap();
        let bytes = turts_mod_runtime::pack::pack_files(&files).unwrap();
        let restored = PackageArchive::parse(&bytes).unwrap();
        assert_eq!(restored.manifest, original.manifest);
        assert_eq!(restored.units().unwrap(), original.units().unwrap());
        assert_eq!(restored.resources().unwrap(), original.resources().unwrap());
        assert_eq!(restored.nodes().unwrap(), original.nodes().unwrap());
        assert_eq!(
            restored.production().unwrap(),
            original.production().unwrap()
        );
        assert_eq!(restored.maps().unwrap(), original.maps().unwrap());
        assert_eq!(restored.visuals().unwrap(), original.visuals().unwrap());
        assert_eq!(restored.content().unwrap(), original.content().unwrap());
        fs::remove_dir_all(temp).unwrap();
    }
}
