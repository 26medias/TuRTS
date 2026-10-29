use std::collections::{BTreeMap, BTreeSet, VecDeque};

use semver::{Version, VersionReq};
use turts_mod_api::{Manifest, ModError, PackageId, PackageKind};

use crate::archive::MANIFEST_PATH;

type PackageIndex<'a> = BTreeMap<String, &'a Manifest>;

/// Resolve a game and its available libraries in deterministic dependency order.
pub fn resolve(root: &Manifest, available: &[Manifest]) -> Result<Vec<PackageId>, ModError> {
    let index = index_packages(root, available);
    let order = dependency_order(root, &index);
    let reachable = reachable_packages(root, &index);

    lr7_root_kind(root)?;
    lr1_fields(root, available)?;
    lr2_self(root, available)?;
    lr3_provided(root, available)?;
    lr4_version(&index)?;
    lr5_library_only(&index)?;
    lr6_acyclic(root, &index, &order)?;
    lr8_reachable(root, available, &reachable)?;
    Ok(order.into_iter().map(PackageId).collect())
}

/// Apply the package-local dependency checks during manifest validation and resolution.
pub(crate) fn validate_dependency_fields(manifest: &Manifest) -> Result<(), ModError> {
    lr1_fields(manifest, &[])?;
    lr2_self(manifest, &[])
}

fn index_packages<'a>(root: &'a Manifest, available: &'a [Manifest]) -> PackageIndex<'a> {
    let mut index = BTreeMap::new();
    for package in available {
        index.entry(package.id.clone()).or_insert(package);
    }
    index.insert(root.id.clone(), root);
    index
}

fn fail(manifest: &Manifest, message: String) -> ModError {
    ModError::new(&manifest.id, "dependencies", message).with_file(MANIFEST_PATH)
}

fn lr7_root_kind(root: &Manifest) -> Result<(), ModError> {
    if root.kind == PackageKind::Library {
        return Err(
            ModError::new(&root.id, "kind", "a library cannot be the root of a match")
                .with_file(MANIFEST_PATH),
        );
    }
    Ok(())
}

fn lr1_fields(root: &Manifest, available: &[Manifest]) -> Result<(), ModError> {
    for manifest in std::iter::once(root).chain(available) {
        for (id, requirement) in &manifest.dependencies.0 {
            PackageId::parse(id).map_err(|message| fail(manifest, message))?;
            VersionReq::parse(requirement).map_err(|error| {
                fail(
                    manifest,
                    format!(
                        "dependency `{id}` has invalid version requirement `{requirement}`: {error}"
                    ),
                )
            })?;
        }
    }
    Ok(())
}

fn lr2_self(root: &Manifest, available: &[Manifest]) -> Result<(), ModError> {
    for manifest in std::iter::once(root).chain(available) {
        if manifest.dependencies.0.contains_key(&manifest.id) {
            return Err(fail(
                manifest,
                format!("package `{}` depends on itself", manifest.id),
            ));
        }
    }
    Ok(())
}

fn lr3_provided(root: &Manifest, available: &[Manifest]) -> Result<(), ModError> {
    for manifest in std::iter::once(root).chain(available) {
        for (id, requirement) in &manifest.dependencies.0 {
            let count = available.iter().filter(|package| package.id == *id).count();
            if count == 0 {
                return Err(fail(
                    manifest,
                    format!("dependency `{id}` `{requirement}` is not provided"),
                ));
            }
            if count > 1 {
                return Err(fail(
                    manifest,
                    format!("dependency `{id}` is provided {count} times"),
                ));
            }
        }
    }
    Ok(())
}

fn lr4_version(index: &PackageIndex<'_>) -> Result<(), ModError> {
    for manifest in index.values() {
        for (id, requirement) in &manifest.dependencies.0 {
            let Some(dependency) = index.get(id) else {
                continue;
            };
            let Ok(version) = Version::parse(&dependency.version) else {
                return Err(fail(
                    manifest,
                    format!(
                        "dependency `{id}` has invalid version `{}`",
                        dependency.version
                    ),
                ));
            };
            let Ok(requirement) = VersionReq::parse(requirement) else {
                continue;
            };
            if !requirement.matches(&version) {
                return Err(fail(
                    manifest,
                    format!(
                        "dependency `{id}` version `{}` does not match `{requirement}`",
                        dependency.version
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn lr5_library_only(index: &PackageIndex<'_>) -> Result<(), ModError> {
    for manifest in index.values() {
        for id in manifest.dependencies.0.keys() {
            let Some(dependency) = index.get(id) else {
                continue;
            };
            if dependency.kind != PackageKind::Library {
                return Err(fail(
                    manifest,
                    format!("dependency `{id}` is a Game; only libraries can be dependencies"),
                ));
            }
        }
    }
    Ok(())
}

fn dependency_order(root: &Manifest, index: &PackageIndex<'_>) -> Vec<String> {
    let mut packages = index.clone();
    packages.insert(root.id.clone(), root);
    let mut unresolved: BTreeMap<String, BTreeSet<String>> = packages
        .iter()
        .map(|(id, manifest)| {
            let dependencies = manifest
                .dependencies
                .0
                .keys()
                .filter(|dependency| packages.contains_key(*dependency))
                .cloned()
                .collect();
            (id.clone(), dependencies)
        })
        .collect();
    let mut ready: BTreeSet<String> = unresolved
        .iter()
        .filter(|(_, dependencies)| dependencies.is_empty())
        .map(|(id, _)| id.clone())
        .collect();
    let mut order = Vec::with_capacity(unresolved.len());

    while let Some(id) = ready.pop_first() {
        order.push(id.clone());
        for (dependent, dependencies) in &mut unresolved {
            if dependencies.remove(&id) && dependencies.is_empty() {
                ready.insert(dependent.clone());
            }
        }
    }
    order
}

fn lr6_acyclic(
    root: &Manifest,
    index: &PackageIndex<'_>,
    order: &[String],
) -> Result<(), ModError> {
    let mut packages = index.clone();
    packages.insert(root.id.clone(), root);
    if order.len() == packages.len() {
        return Ok(());
    }
    let cycle = find_cycle(&packages).unwrap_or_else(|| vec!["unknown".to_string()]);
    Err(fail(
        root,
        format!("dependency cycle: {}", cycle.join(" -> ")),
    ))
}

fn find_cycle(packages: &PackageIndex<'_>) -> Option<Vec<String>> {
    fn visit(
        id: &str,
        packages: &PackageIndex<'_>,
        active: &mut Vec<String>,
        complete: &mut BTreeSet<String>,
    ) -> Option<Vec<String>> {
        if let Some(start) = active.iter().position(|current| current == id) {
            let mut cycle = active[start..].to_vec();
            cycle.push(id.to_string());
            return Some(cycle);
        }
        if complete.contains(id) {
            return None;
        }
        active.push(id.to_string());
        if let Some(manifest) = packages.get(id) {
            for dependency in manifest.dependencies.0.keys() {
                if packages.contains_key(dependency)
                    && let Some(cycle) = visit(dependency, packages, active, complete)
                {
                    return Some(cycle);
                }
            }
        }
        active.pop();
        complete.insert(id.to_string());
        None
    }

    let mut active = Vec::new();
    let mut complete = BTreeSet::new();
    for id in packages.keys() {
        if let Some(cycle) = visit(id, packages, &mut active, &mut complete) {
            return Some(cycle);
        }
    }
    None
}

fn reachable_packages(root: &Manifest, index: &PackageIndex<'_>) -> BTreeSet<String> {
    let mut reachable = BTreeSet::new();
    let mut pending = VecDeque::from([root.id.clone()]);
    while let Some(id) = pending.pop_front() {
        if !reachable.insert(id.clone()) {
            continue;
        }
        let manifest = if id == root.id {
            Some(root)
        } else {
            index.get(&id).copied()
        };
        if let Some(manifest) = manifest {
            pending.extend(manifest.dependencies.0.keys().cloned());
        }
    }
    reachable
}

fn lr8_reachable(
    root: &Manifest,
    available: &[Manifest],
    reachable: &BTreeSet<String>,
) -> Result<(), ModError> {
    for package in available {
        if !reachable.contains(&package.id) {
            return Err(fail(
                root,
                format!(
                    "provided package `{}` is not reachable from the root",
                    package.id
                ),
            ));
        }
    }
    Ok(())
}
