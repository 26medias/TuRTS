use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};
use turts_mod_api::{
    BalanceAmount, ContentId, Cost, CostSource, DropOffEffect, GathererDef, InventoryDef,
    ItemStack, Manifest, MapDef, NodeDef, NodePlacement, PaymentPolicy, Placement, ProducerDef,
    ProductionDef, ResourceDef, ResourceKind, SlotEconomyInit, UnitDef, VisualDef, VisualTarget,
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
    semantic_changes: Vec<SemanticRecord>,
}
#[derive(Serialize)]
struct SemanticRecord {
    package: String,
    definition_path: String,
    field: String,
    before: serde_json::Value,
    after: serde_json::Value,
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
    if !matches!(a.mode.as_str(), "rename" | "balance" | "physical") {
        return Err(format!("unsupported mode `{}`", a.mode));
    }
    let policy = match a.payment.as_str() {
        "charge" => PaymentPolicy::ChargeOnQueue,
        "reserve" => PaymentPolicy::ReserveThenCharge,
        _ => return Err("payment must be charge or reserve".into()),
    };
    let source_argument = abs_lex(&a.source)?;
    let source_metadata =
        fs::symlink_metadata(&source_argument).map_err(|e| format!("source: {e}"))?;
    if source_metadata.file_type().is_symlink() {
        return Err("symlink source root is unsupported".into());
    }
    let source = source_argument
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
    let r = build(&source, &stage, a.seed, policy, &a.payment, &a.mode);
    if r.is_err() {
        let _ = fs::remove_dir_all(&stage);
    }
    r?;
    if let Err(error) = publish_without_replace(&stage, &out) {
        let _ = fs::remove_dir_all(&stage);
        return Err(error);
    }
    Ok(())
}

// `create_dir` atomically claims the final path and fails if another actor created it.
// The stage is already fully validated; its entries move into the claimed directory.
// This prevents replacement but does not provide crash-atomic visibility of the whole tree.
fn publish_without_replace(stage: &Path, output: &Path) -> Result<(), String> {
    fs::create_dir(output).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            format!("output {} already exists", output.display())
        } else {
            format!("cannot claim output {}: {error}", output.display())
        }
    })?;
    let result = (|| {
        for entry in fs::read_dir(stage).map_err(|e| format!("cannot read validated stage: {e}"))? {
            let entry = entry.map_err(|e| format!("cannot read validated stage entry: {e}"))?;
            let destination = output.join(entry.file_name());
            fs::rename(entry.path(), &destination)
                .map_err(|e| format!("cannot publish {}: {e}", destination.display()))?;
        }
        fs::remove_dir(stage).map_err(|e| format!("cannot remove empty stage: {e}"))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(output);
    }
    result
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
    mode: &str,
) -> Result<(), String> {
    let mut inputs = discover(source)?;
    if inputs.is_empty() {
        return Err("empty package source".into());
    }
    let mut semantic_changes = vec![];
    if mode != "rename" {
        apply_role_transform(&mut inputs, mode, seed, policy, &mut semantic_changes)?;
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
            if mode == "rename" && p.payment != policy {
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
    semantic_changes.sort_by(|a, b| {
        (&a.package, &a.definition_path, &a.field).cmp(&(&b.package, &b.definition_path, &b.field))
    });
    let map = Mapping {
        version: if mode == "rename" { 1 } else { 2 },
        seed,
        mode: mode.into(),
        payment: payment.into(),
        packages: pkgs,
        definitions: defs,
        names,
        paths,
        unchanged_assets: assets,
        semantic_changes,
    };
    let mut json = serde_json::to_vec_pretty(&map).map_err(|e| e.to_string())?;
    json.push(b'\n');
    fs::write(stage.join("mapping.json"), json).map_err(|e| e.to_string())?;
    Ok(())
}
fn semantic_record<T: serde::Serialize, U: serde::Serialize>(
    package: &str,
    path: &str,
    field: &str,
    before: &T,
    after: &U,
) -> SemanticRecord {
    SemanticRecord {
        package: package.into(),
        definition_path: path.into(),
        field: field.into(),
        before: serde_json::to_value(before).unwrap_or(serde_json::Value::Null),
        after: serde_json::to_value(after).unwrap_or(serde_json::Value::Null),
    }
}
fn write_def<T: serde::Serialize>(
    files: &mut BTreeMap<String, Vec<u8>>,
    path: &str,
    value: &T,
) -> Result<(), String> {
    files.insert(path.into(), ronbytes(value)?);
    Ok(())
}
fn apply_role_transform(
    inputs: &mut [Input],
    mode: &str,
    seed: i64,
    payment: PaymentPolicy,
    changes: &mut Vec<SemanticRecord>,
) -> Result<(), String> {
    for input in inputs.iter() {
        let manifest = &input.archive.manifest;
        if input.files.keys().any(|path| {
            path.ends_with(".ron") && path != "manifest.ron" && !exported(manifest, path)
        }) {
            return Err(format!("{} has unexported RON", manifest.id));
        }
    }
    for input in inputs {
        let package = input.archive.manifest.id.clone();
        let resources = input.archive.resources().map_err(|e| e.to_string())?;
        let recipes = input.archive.production().map_err(|e| e.to_string())?;
        if recipes.is_empty()
            && resources.is_empty()
            && input.archive.nodes().map_err(|e| e.to_string())?.is_empty()
        {
            changes.push(SemanticRecord {
                package: package.clone(),
                definition_path: "<package>".into(),
                field: "role_transform_noop".into(),
                before: serde_json::json!("structurally_empty_economy"),
                after: serde_json::json!(mode),
            });
            continue;
        }
        if recipes.is_empty() {
            return Err(format!("{package}: unsupported economy without recipes"));
        }
        if resources
            .iter()
            .filter(|(_, r)| r.kind == ResourceKind::Item)
            .count()
            != 1
            || resources.len() > 2
            || resources
                .iter()
                .filter(|(_, r)| r.kind == ResourceKind::Balance)
                .count()
                > 1
        {
            return Err(format!("{package}: unsupported resource endpoints"));
        }
        let (item_path, item_def) = resources
            .iter()
            .find(|(_, r)| r.kind == ResourceKind::Item)
            .ok_or("missing Item endpoint")?;
        let item = format!("{package}:{}", item_def.id);
        let balance_pair = resources
            .iter()
            .find(|(_, r)| r.kind == ResourceKind::Balance);
        let balance = balance_pair.map(|(_, r)| format!("{package}:{}", r.id));
        let balance_path = balance_pair.map(|(p, _)| p.clone());
        let all_units = input.archive.units().map_err(|e| e.to_string())?;
        let receivers = all_units
            .iter()
            .filter(|(_, u)| u.producer.is_some())
            .collect::<Vec<_>>();
        if receivers.len() != 1 {
            return Err(format!(
                "{package}: expected exactly one producer definition"
            ));
        }
        let receiver_path = receivers[0].0.clone();
        let receiver_id = format!("{package}:{}", receivers[0].1.id);
        let mut gatherers = all_units
            .iter()
            .filter(|(_, u)| u.gatherer.is_some())
            .collect::<Vec<_>>();
        if gatherers.len() != 1 {
            return Err(format!(
                "{package}: expected exactly one gathering definition"
            ));
        }
        let (_, gatherer) = gatherers
            .pop()
            .ok_or_else(|| format!("{package}: missing gathering definition"))?;
        if gatherer.gatherer.as_ref().unwrap().items != vec![item.clone()]
            || !gatherer
                .inventory
                .as_ref()
                .is_some_and(|i| i.accepts == vec![item.clone()])
            || all_units
                .iter()
                .any(|(path, u)| path != &receiver_path && u.drop_off.is_some())
        {
            return Err(format!(
                "{package}: unsupported gathering item/filter shape"
            ));
        }
        let node_defs = input.archive.nodes().map_err(|e| e.to_string())?;
        if node_defs.is_empty() || node_defs.iter().any(|(_, n)| n.item != item) {
            return Err(format!("{package}: unsupported node Item endpoints"));
        }
        let receiver = &receivers[0].1;
        let drop = receiver
            .drop_off
            .as_ref()
            .ok_or_else(|| format!("{package}: producer has no receiver rule"))?;
        if drop.rules.len() != 1 || drop.rules[0].item != item {
            return Err(format!("{package}: unsupported receiver rules"));
        }
        let (source_resource, source_kind) = {
            let mut shape = None;
            for (_, recipe) in &recipes {
                if recipe.costs.len() != 1 {
                    return Err(format!("{package}: each recipe must have exactly one cost"));
                }
                let cost = &recipe.costs[0];
                let kind = match cost.source {
                    CostSource::ProducerInventory if cost.resource == item => "item",
                    CostSource::SlotBalance => {
                        let b = balance.as_deref().ok_or_else(|| {
                            format!("{package}: SlotBalance cost has no Balance definition")
                        })?;
                        if cost.resource != b {
                            return Err(format!("{package}: unsupported cost endpoint"));
                        }
                        "balance"
                    }
                    _ => return Err(format!("{package}: unsupported cost source/resource shape")),
                };
                if shape
                    .as_ref()
                    .is_some_and(|(resource, prior_kind): &(String, &str)| {
                        resource != &cost.resource || *prior_kind != kind
                    })
                {
                    return Err(format!("{package}: mixed recipe cost endpoints"));
                }
                shape = Some((cost.resource.clone(), kind));
            }
            shape.ok_or_else(|| format!("{package}: no recipes to transform"))?
        };
        let source_is_balance = source_kind == "balance";
        if (source_is_balance && source_resource != balance.as_deref().unwrap_or_default())
            || (!source_is_balance && source_resource != item)
        {
            return Err(format!(
                "{package}: cost endpoint does not match inferred economic role"
            ));
        }
        let producer = receiver
            .producer
            .as_ref()
            .ok_or_else(|| format!("{package}: producer definition disappeared"))?;
        let enabled = producer.recipes.iter().cloned().collect::<BTreeSet<_>>();
        let recipe_ids = recipes
            .iter()
            .map(|(_, r)| format!("{package}:{}", r.id))
            .collect::<BTreeSet<_>>();
        if enabled != recipe_ids {
            return Err(format!(
                "{package}: producer recipe set does not exactly enable the package recipes"
            ));
        }
        match &drop.rules[0].effect {
            DropOffEffect::Store if !source_is_balance => {}
            DropOffEffect::Convert {
                balance: rule_balance,
                per_item: 1,
            } if source_is_balance && Some(rule_balance) == balance.as_ref() => {}
            _ => {
                return Err(format!(
                    "{package}: unsupported receiver effect for cost endpoint"
                ))
            }
        }
        if payment == PaymentPolicy::ReserveThenCharge
            && recipes.iter().any(|(_, r)| {
                r.cancel_refunds.queued.numerator != 1 || r.cancel_refunds.queued.denominator != 1
            })
        {
            return Err(format!(
                "{package}: reserve requires queued refund exactly 1/1"
            ));
        }
        let mut maps = input.archive.maps().map_err(|e| e.to_string())?;
        if maps.is_empty() {
            return Err(format!(
                "{package}: role transform requires at least one map"
            ));
        }
        let mut map_seeds: BTreeMap<String, BTreeMap<u8, u64>> = BTreeMap::new();
        let mut capacity = 0u64;
        for (map_path, map) in &maps {
            let owners = map
                .placements
                .iter()
                .map(|p| p.owner_slot)
                .collect::<BTreeSet<_>>();
            if map.slot_economy.iter().any(|e| !owners.contains(&e.slot)) {
                return Err(format!(
                    "{package}/{map_path}: slot economy has no represented owner"
                ));
            }
            let mut owner_seeds = BTreeMap::new();
            for owner in owners {
                let owner_producers = map
                    .placements
                    .iter()
                    .filter(|p| p.owner_slot == owner && p.unit == receiver_id)
                    .collect::<Vec<_>>();
                if owner_producers.len() != 1 {
                    return Err(format!("{package}/{map_path}: expected exactly one producer placement for owner {owner}"));
                }
                let endpoint = owner_producers[0];
                for placed in map
                    .placements
                    .iter()
                    .filter(|p| p.owner_slot == owner && p.unit != receiver_id)
                {
                    if !placed.contents.is_empty() {
                        return Err(format!(
                            "{package}/{map_path}: stock outside producer endpoint"
                        ));
                    }
                }
                if endpoint.contents.len() > 1 || endpoint.contents.iter().any(|s| s.item != item) {
                    return Err(format!(
                        "{package}/{map_path}: unsupported producer inventory stock"
                    ));
                }
                let item_seed = endpoint.contents.first().map_or(0, |s| s.amount);
                let slot = map.slot_economy.iter().find(|e| e.slot == owner);
                let balance_seed = if let Some(slot) = slot {
                    if slot
                        .balances
                        .iter()
                        .any(|b| Some(&b.balance) != balance.as_ref())
                    {
                        return Err(format!(
                            "{package}/{map_path}: stock outside declared Balance endpoint"
                        ));
                    }
                    if slot.balances.len() > 1 {
                        return Err(format!("{package}/{map_path}: ambiguous Balance stock"));
                    }
                    slot.balances.first().map_or(0, |b| b.amount)
                } else {
                    0
                };
                if (source_is_balance && item_seed > 0) || (!source_is_balance && balance_seed > 0)
                {
                    return Err(format!(
                        "{package}/{map_path}: stock outside the selected economic endpoint"
                    ));
                }
                let source_seed = if source_is_balance {
                    balance_seed
                } else {
                    item_seed
                };
                owner_seeds.insert(owner, source_seed);
                if mode == "physical" {
                    let node_total = map
                        .nodes
                        .iter()
                        .try_fold(0u64, |sum, n| sum.checked_add(n.remaining).ok_or(()))
                        .map_err(|_| format!("{package}/{map_path}: node capacity overflow"))?;
                    let required = source_seed.checked_add(node_total).ok_or_else(|| {
                        format!("{package}/{map_path}: inventory capacity overflow")
                    })?;
                    capacity = capacity.max(required);
                }
            }
            map_seeds.insert(map_path.clone(), owner_seeds);
        }
        if mode == "physical" && capacity == 0 {
            return Err(format!("{package}: derived physical capacity is zero"));
        }
        let mut target_balance = balance.clone();
        let mut target_balance_path = balance_path.clone();
        let mut added_resource = None;
        if mode == "balance" && target_balance.is_none() {
            let local = format!(
                "balance_{}",
                hash(seed, &package, "role", item_path, "balance")
            );
            if resources.iter().any(|(_, r)| r.id == local) {
                return Err(format!("{package}: generated Balance identity collision"));
            }
            target_balance = Some(format!("{package}:{local}"));
            target_balance_path = Some(format!("resources/{local}.ron"));
            added_resource = Some(ResourceDef {
                id: local,
                name: format!(
                    "Balance {}",
                    hash(seed, &package, "role", item_path, "name")
                ),
                unit_name: format!("unit {}", hash(seed, &package, "role", item_path, "unit")),
                kind: ResourceKind::Balance,
            });
        }
        let target_balance = target_balance;
        let target_balance_path = target_balance_path;
        if let Some(def) = added_resource {
            let path = target_balance_path
                .as_ref()
                .ok_or_else(|| format!("{package}: missing generated Balance path"))?;
            if input.files.contains_key(path) {
                return Err(format!(
                    "{package}: generated Balance path already exists `{path}`"
                ));
            }
            let before = serde_json::Value::Null;
            input.files.insert(path.clone(), ronbytes(&def)?);
            let mut manifest = input.archive.manifest.clone();
            manifest.exports.resources.push(path.clone());
            manifest.exports.resources.sort();
            write_def(
                &mut input.files,
                "manifest.ron",
                &ManifestOutput::from(&manifest),
            )?;
            changes.push(SemanticRecord {
                package: package.clone(),
                definition_path: path.clone(),
                field: "added_definition".into(),
                before,
                after: serde_json::to_value(&def).map_err(|error| error.to_string())?,
            });
            changes.push(semantic_record(
                &package,
                "manifest.ron",
                "exports.resources",
                &input.archive.manifest.exports.resources,
                &manifest.exports.resources,
            ));
        }
        let target_item_source = if mode == "physical" {
            CostSource::ProducerInventory
        } else {
            CostSource::SlotBalance
        };
        let mut after_units = all_units.clone();
        for (path, unit) in &mut after_units {
            let before = unit.clone();
            if path == &receiver_path {
                if mode == "physical" {
                    let max_seed = map_seeds
                        .values()
                        .flat_map(|m| m.values())
                        .copied()
                        .max()
                        .unwrap_or(0);
                    let inv = unit.inventory.get_or_insert(InventoryDef {
                        capacity,
                        accepts: vec![item.clone()],
                    });
                    if inv.accepts != vec![item.clone()] || inv.capacity < max_seed {
                        inv.accepts = vec![item.clone()];
                        inv.capacity = capacity;
                    }
                }
                if let Some(drop) = &mut unit.drop_off {
                    drop.rules[0].effect = if mode == "physical" {
                        DropOffEffect::Store
                    } else {
                        DropOffEffect::Convert {
                            balance: target_balance
                                .clone()
                                .ok_or_else(|| format!("{package}: missing target Balance"))?,
                            per_item: 1,
                        }
                    };
                }
            }
            if mode == "physical" && path == &receiver_path {
                if let Some(inv) = &mut unit.inventory {
                    if inv.accepts != vec![item.clone()] {
                        inv.accepts = vec![item.clone()];
                        inv.capacity = capacity;
                    }
                }
            }
            if unit.inventory != before.inventory {
                changes.push(semantic_record(
                    &package,
                    path,
                    "inventory",
                    &before.inventory,
                    &unit.inventory,
                ));
            }
            if unit.drop_off != before.drop_off {
                changes.push(semantic_record(
                    &package,
                    path,
                    "drop_off",
                    &before.drop_off,
                    &unit.drop_off,
                ));
            }
            if *unit != before {
                write_def(&mut input.files, path, unit)?;
            }
        }
        let mut after_recipes = recipes.clone();
        for (path, recipe) in &mut after_recipes {
            let before = recipe.clone();
            let cost = &mut recipe.costs[0];
            cost.resource = if mode == "physical" {
                item.clone()
            } else {
                target_balance.clone().ok_or("missing target Balance")?
            };
            cost.source = target_item_source;
            recipe.payment = payment;
            if recipe.costs != before.costs {
                changes.push(semantic_record(
                    &package,
                    path,
                    "costs",
                    &before.costs,
                    &recipe.costs,
                ));
            }
            if recipe.payment != before.payment {
                changes.push(semantic_record(
                    &package,
                    path,
                    "payment",
                    &before.payment,
                    &recipe.payment,
                ));
            }
            if *recipe != before {
                write_def(&mut input.files, path, recipe)?;
            }
        }
        for (map_path, map) in &mut maps {
            let before = map.clone();
            let owner_values = map_seeds
                .get(map_path)
                .ok_or_else(|| format!("{package}/{map_path}: missing derived seed map"))?;
            for placement in &mut map.placements {
                if placement.unit != receiver_id {
                    continue;
                }
                let seed_amount = owner_values
                    .get(&placement.owner_slot)
                    .copied()
                    .unwrap_or(0);
                if mode == "balance" && !source_is_balance {
                    placement.contents.clear();
                }
                if mode == "physical" && source_is_balance {
                    placement.contents = if seed_amount == 0 {
                        vec![]
                    } else {
                        vec![ItemStack {
                            item: item.clone(),
                            amount: seed_amount,
                        }]
                    };
                }
            }
            for owner in owner_values.keys().copied() {
                if mode == "physical" && balance.is_none() {
                    continue;
                }
                if (mode == "balance" && !source_is_balance)
                    || (mode == "physical" && source_is_balance)
                {
                    let amount = *owner_values
                        .get(&owner)
                        .ok_or_else(|| format!("{package}/{map_path}: missing owner seed"))?;
                    changes.push(semantic_record(
                        &package,
                        map_path,
                        &format!("seed_transfer.owner_{owner}"),
                        &serde_json::json!({
                            "endpoint": if source_is_balance { "slot_balance" } else { "producer_inventory" },
                            "amount": amount,
                        }),
                        &serde_json::json!({
                            "endpoint": if mode == "balance" { "slot_balance" } else { "producer_inventory" },
                            "amount": amount,
                        }),
                    ));
                }
                let entry =
                    if let Some(index) = map.slot_economy.iter().position(|e| e.slot == owner) {
                        &mut map.slot_economy[index]
                    } else {
                        map.slot_economy.push(SlotEconomyInit {
                            slot: owner,
                            balances: vec![],
                        });
                        map.slot_economy.last_mut().ok_or_else(|| {
                            format!("{package}/{map_path}: failed to add slot economy")
                        })?
                    };
                if mode == "physical" {
                    if let Some(b) = balance.as_ref() {
                        entry.balances.retain(|x| &x.balance != b);
                        entry.balances.push(BalanceAmount {
                            balance: b.clone(),
                            amount: 0,
                        });
                    }
                } else if let Some(b) = target_balance.as_ref() {
                    let amount = *owner_values
                        .get(&owner)
                        .ok_or_else(|| format!("{package}/{map_path}: missing owner seed"))?;
                    if let Some(found) = entry.balances.iter_mut().find(|x| &x.balance == b) {
                        found.amount = amount;
                    } else {
                        entry.balances.push(BalanceAmount {
                            balance: b.clone(),
                            amount,
                        });
                    }
                }
            }
            if map.placements != before.placements {
                changes.push(semantic_record(
                    &package,
                    map_path,
                    "placements",
                    &before.placements,
                    &map.placements,
                ));
            }
            if map.slot_economy != before.slot_economy {
                changes.push(semantic_record(
                    &package,
                    map_path,
                    "slot_economy",
                    &before.slot_economy,
                    &map.slot_economy,
                ));
            }
            if *map != before {
                write_def(&mut input.files, map_path, map)?;
            }
        }
        if mode == "physical" {
            for (path, unit) in &after_units {
                if path == &receiver_path {
                    continue;
                }
                if unit.inventory.as_ref().is_some_and(|i| {
                    i.accepts.iter().any(|x| x == &item) && unit.gatherer.is_some()
                }) {
                    changes.push(semantic_record(
                        &package,
                        path,
                        "gathering_items_preserved",
                        &unit.inventory,
                        &unit.inventory,
                    ));
                }
            }
            changes.push(semantic_record(
                &package,
                &receiver_path,
                "inventory.capacity_requirement",
                &serde_json::Value::Null,
                &serde_json::json!({"derived_maximum": capacity, "method": "owner_seed_plus_all_map_node_remaining"}),
            ));
        }
        for (map_path, map) in maps {
            if input.files.contains_key(&map_path) {
                write_def(&mut input.files, &map_path, &map)?;
            }
        }
        let packed =
            turts_mod_runtime::pack::pack_files(&input.files).map_err(|e| e.to_string())?;
        input.archive = PackageArchive::parse(&packed)
            .map_err(|e| format!("role transform staged parse: {e}"))?;
        let _ = balance_path;
    }
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
    fn role_modes_conserve_seed_derive_capacity_and_validate_added_balance_references() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods");
        let mut physical = discover(&root.join("base")).unwrap();
        let mut physical_changes = vec![];
        apply_role_transform(
            &mut physical,
            "physical",
            17,
            PaymentPolicy::ChargeOnQueue,
            &mut physical_changes,
        )
        .unwrap();
        let archive = &physical[0].archive;
        let receiver = archive
            .units()
            .unwrap()
            .into_iter()
            .find(|(_, u)| u.producer.is_some())
            .unwrap()
            .1;
        assert_eq!(receiver.inventory.unwrap().capacity, 2500);
        assert_eq!(
            receiver.drop_off.unwrap().rules[0].effect,
            DropOffEffect::Store
        );
        let recipes = archive.production().unwrap();
        assert!(recipes
            .iter()
            .all(|(_, r)| r.costs[0].source == CostSource::ProducerInventory
                && r.costs[0].resource.ends_with(":ore")));
        let map = archive.maps().unwrap().remove(0).1;
        assert_eq!(
            map.placements
                .iter()
                .filter(|p| p.unit.ends_with(":command_center"))
                .map(|p| p.contents.iter().map(|x| x.amount).sum::<u64>())
                .sum::<u64>(),
            1000
        );
        assert!(map
            .slot_economy
            .iter()
            .all(|e| e.balances.len() == 1 && e.balances[0].amount == 0));
        assert!(visuals_by_content(std::slice::from_ref(archive)).is_ok());

        let mut item_only = discover(&root.join("cedar")).unwrap();
        let mut physical_changes = vec![];
        apply_role_transform(
            &mut item_only,
            "physical",
            19,
            PaymentPolicy::ChargeOnQueue,
            &mut physical_changes,
        )
        .unwrap();
        let item_only_archive = &item_only[0].archive;
        assert!(!item_only_archive
            .resources()
            .unwrap()
            .iter()
            .any(|(_, r)| r.kind == ResourceKind::Balance));
        assert!(item_only_archive
            .maps()
            .unwrap()
            .iter()
            .all(|(_, map)| map.slot_economy.is_empty()));

        let mut balance = discover(&root.join("cedar")).unwrap();
        let mut changes = vec![];
        apply_role_transform(
            &mut balance,
            "balance",
            18,
            PaymentPolicy::ReserveThenCharge,
            &mut changes,
        )
        .unwrap();
        let archive = &balance[0].archive;
        assert_eq!(
            archive
                .resources()
                .unwrap()
                .iter()
                .filter(|(_, r)| r.kind == ResourceKind::Balance)
                .count(),
            1
        );
        assert!(archive
            .production()
            .unwrap()
            .iter()
            .all(|(_, r)| r.costs[0].source == CostSource::SlotBalance
                && r.payment == PaymentPolicy::ReserveThenCharge));
        let map = archive.maps().unwrap().remove(0).1;
        assert!(map
            .placements
            .iter()
            .filter(|p| p.unit.ends_with(":depot"))
            .all(|p| p.contents.is_empty()));
        assert!(map
            .slot_economy
            .iter()
            .all(|e| e.balances.len() == 1 && e.balances[0].amount == 100));
        assert!(visuals_by_content(std::slice::from_ref(archive)).is_ok());

        let added = archive
            .manifest
            .exports
            .resources
            .iter()
            .find(|p| p.contains("balance_"))
            .unwrap()
            .clone();
        let mut manifest = archive.manifest.clone();
        manifest.exports.resources.retain(|p| p != &added);
        write_def(
            &mut balance[0].files,
            "manifest.ron",
            &ManifestOutput::from(&manifest),
        )
        .unwrap();
        let packed = turts_mod_runtime::pack::pack_files(&balance[0].files).unwrap();
        let missing_export = PackageArchive::parse(&packed).unwrap();
        assert!(
            validate(&missing_export).is_err(),
            "public reference validation must reject a missing generated Balance export"
        );
    }

    #[test]
    fn physical_mode_retains_unused_balance_definition_and_zero_accounts() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar");
        let mut inputs = discover(&root).unwrap();
        let mut manifest = inputs[0].archive.manifest.clone();
        manifest
            .exports
            .resources
            .push("resources/reserve.ron".into());
        manifest.exports.resources.sort();
        write_def(
            &mut inputs[0].files,
            "resources/reserve.ron",
            &ResourceDef {
                id: "reserve".into(),
                name: "Reserve".into(),
                unit_name: "unit".into(),
                kind: ResourceKind::Balance,
            },
        )
        .unwrap();
        write_def(
            &mut inputs[0].files,
            "manifest.ron",
            &ManifestOutput::from(&manifest),
        )
        .unwrap();
        inputs[0].archive =
            PackageArchive::parse(&turts_mod_runtime::pack::pack_files(&inputs[0].files).unwrap())
                .unwrap();
        let mut changes = vec![];
        apply_role_transform(
            &mut inputs,
            "physical",
            31,
            PaymentPolicy::ChargeOnQueue,
            &mut changes,
        )
        .unwrap();
        let archive = &inputs[0].archive;
        assert!(archive
            .resources()
            .unwrap()
            .iter()
            .any(|(_, r)| r.id == "reserve" && r.kind == ResourceKind::Balance));
        let map = archive.maps().unwrap().remove(0).1;
        assert!(map.slot_economy.iter().all(|slot| slot.balances
            == vec![BalanceAmount {
                balance: "cedar:reserve".into(),
                amount: 0
            }]));
        assert!(archive
            .production()
            .unwrap()
            .iter()
            .all(|(_, recipe)| recipe.costs[0].resource.ends_with(":wood")
                && recipe.costs[0].source == CostSource::ProducerInventory));
        assert!(validate(archive).is_ok());
    }

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
    fn concurrent_empty_output_is_not_replaced_during_publication() {
        use std::os::unix::fs::MetadataExt;
        use std::time::{Duration, Instant};

        let temp = env::temp_dir().join(format!("m2-variant-publish-race-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();
        let source = temp.join("source");
        copy_tree(
            &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar"),
            &source,
        );
        fs::create_dir_all(source.join("assets")).unwrap();
        fs::write(
            source.join("assets/large.bin"),
            vec![0x5a; 32 * 1024 * 1024],
        )
        .unwrap();
        let output = temp.join("out");
        let stage = temp.join(format!(".out.m2-stage-{}", std::process::id()));
        let args = vec![
            "--source".into(),
            source.to_string_lossy().into_owned(),
            "--out".into(),
            output.to_string_lossy().into_owned(),
            "--seed".into(),
            "17".into(),
            "--mode".into(),
            "rename".into(),
            "--payment".into(),
            "reserve".into(),
        ];
        let worker = std::thread::spawn(move || run(args));
        let deadline = Instant::now() + Duration::from_secs(30);
        while !stage.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(
            stage.exists(),
            "generator did not reach its staging directory"
        );
        fs::create_dir(&output).unwrap();
        let original_inode = fs::metadata(&output).unwrap().ino();
        let result = worker.join().unwrap();
        let final_inode = fs::metadata(&output).unwrap().ino();
        assert_eq!(
            final_inode, original_inode,
            "publication replaced the concurrently created output directory"
        );
        assert!(
            result.is_err(),
            "publication must reject the concurrently created output"
        );
        assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
        fs::remove_dir_all(temp).unwrap();
    }

    fn copy_tree(source: &Path, destination: &Path) {
        fs::create_dir_all(destination).unwrap();
        for entry in fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let target = destination.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_tree(&entry.path(), &target);
            } else {
                fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    #[test]
    fn inverse_rename_restores_role_transformed_intermediate() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar");
        let mut inputs = discover(&root).unwrap();
        let mut changes = vec![];
        apply_role_transform(
            &mut inputs,
            "balance",
            123,
            PaymentPolicy::ChargeOnQueue,
            &mut changes,
        )
        .unwrap();
        let intermediate = inputs[0].archive.clone();
        let forward = make_plan(&inputs, 123).unwrap();
        let mut defs = vec![];
        let mut names = vec![];
        let renamed_files = transformed_files(&inputs[0], &forward, &mut defs, &mut names).unwrap();
        let renamed =
            PackageArchive::parse(&turts_mod_runtime::pack::pack_files(&renamed_files).unwrap())
                .unwrap();
        let reverse = inverse_plan(&forward).unwrap();
        let renamed_input = Input {
            archive: renamed,
            files: renamed_files,
        };
        let mut reverse_defs = vec![];
        let mut reverse_names = vec![];
        let restored_files = transformed_files(
            &renamed_input,
            &reverse,
            &mut reverse_defs,
            &mut reverse_names,
        )
        .unwrap();
        let restored =
            PackageArchive::parse(&turts_mod_runtime::pack::pack_files(&restored_files).unwrap())
                .unwrap();
        assert_eq!(restored.manifest, intermediate.manifest);
        assert_eq!(restored.units().unwrap(), intermediate.units().unwrap());
        assert_eq!(
            restored.resources().unwrap(),
            intermediate.resources().unwrap()
        );
        assert_eq!(
            restored.production().unwrap(),
            intermediate.production().unwrap()
        );
        assert_eq!(restored.maps().unwrap(), intermediate.maps().unwrap());
        assert_eq!(restored.content().unwrap(), intermediate.content().unwrap());
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
            "rename",
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
