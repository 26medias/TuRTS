use std::collections::{BTreeMap, BTreeSet};

use semver::{Version, VersionReq};
use turts_mod_api::{Manifest, ModError, PackageId, PackageKind};

use crate::archive::MANIFEST_PATH;

/// Resolve a game and its available libraries in deterministic dependency order.
pub fn resolve(root: &Manifest, available: &[Manifest]) -> Result<Vec<PackageId>, ModError> {
    lr7_root_kind(root)?;
    lr1_fields(root)?;
    lr2_self(root)?;
    for package in available {
        lr1_fields(package)?;
        lr2_self(package)?;
    }
    let packages = lr3_provided(root, available)?;
    lr4_version(root, &packages)?;
    lr5_library_only(root, &packages)?;
    let order = lr6_acyclic(root, &packages)?;
    lr8_reachable(root, available, &order)?;
    Ok(order.into_iter().map(PackageId).collect())
}

/// Apply the package-local dependency checks during manifest validation and resolution.
pub(crate) fn validate_dependency_fields(manifest: &Manifest) -> Result<(), ModError> {
    lr1_fields(manifest)?;
    lr2_self(manifest)
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

fn lr1_fields(manifest: &Manifest) -> Result<(), ModError> {
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
    Ok(())
}

fn lr2_self(manifest: &Manifest) -> Result<(), ModError> {
    if manifest.dependencies.0.contains_key(&manifest.id) {
        return Err(fail(
            manifest,
            format!("package `{}` depends on itself", manifest.id),
        ));
    }
    Ok(())
}

fn lr3_provided<'a>(
    root: &Manifest,
    available: &'a [Manifest],
) -> Result<BTreeMap<String, &'a Manifest>, ModError> {
    let mut selected = BTreeMap::new();
    let dependency_ids: BTreeSet<_> = root
        .dependencies
        .0
        .keys()
        .cloned()
        .chain(
            available
                .iter()
                .flat_map(|package| package.dependencies.0.keys().cloned()),
        )
        .collect();

    for id in dependency_ids {
        let count = available.iter().filter(|package| package.id == id).count();
        if count == 0 {
            let requirement = find_requirement(root, available, &id).unwrap_or_default();
            return Err(fail(
                root,
                format!("dependency `{id}` `{requirement}` is not provided"),
            ));
        }
        if count > 1 {
            return Err(fail(
                root,
                format!("dependency `{id}` is provided {count} times"),
            ));
        }
        let package = available
            .iter()
            .find(|package| package.id == id)
            .expect("count is one");
        selected.insert(id, package);
    }

    for (id, requirement) in &root.dependencies.0 {
        if !available.iter().any(|package| package.id == *id) {
            return Err(fail(
                root,
                format!("dependency `{id}` `{requirement}` is not provided"),
            ));
        }
    }
    Ok(selected)
}

fn find_requirement<'a>(
    root: &'a Manifest,
    available: &'a [Manifest],
    id: &str,
) -> Option<&'a str> {
    root.dependencies.0.get(id).map(String::as_str).or_else(|| {
        available
            .iter()
            .find_map(|package| package.dependencies.0.get(id).map(String::as_str))
    })
}

fn lr4_version(root: &Manifest, packages: &BTreeMap<String, &Manifest>) -> Result<(), ModError> {
    let mut manifests = packages.clone();
    manifests.insert(root.id.clone(), root);
    for manifest in manifests.values() {
        for (id, requirement) in &manifest.dependencies.0 {
            let dependency = packages.get(id).expect("LR3 proved dependency is provided");
            let version = Version::parse(&dependency.version).map_err(|error| {
                fail(
                    manifest,
                    format!(
                        "dependency `{id}` has invalid version `{}`: {error}",
                        dependency.version
                    ),
                )
            })?;
            let requirement = VersionReq::parse(requirement).expect("LR1 checked requirements");
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

fn lr5_library_only(
    root: &Manifest,
    packages: &BTreeMap<String, &Manifest>,
) -> Result<(), ModError> {
    let mut manifests = packages.clone();
    manifests.insert(root.id.clone(), root);
    for manifest in manifests.values() {
        for id in manifest.dependencies.0.keys() {
            let dependency = packages.get(id).expect("LR3 proved dependency is provided");
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

fn lr6_acyclic(
    root: &Manifest,
    packages: &BTreeMap<String, &Manifest>,
) -> Result<Vec<String>, ModError> {
    fn visit(
        id: &str,
        manifest: &Manifest,
        packages: &BTreeMap<String, &Manifest>,
        temporary: &mut Vec<String>,
        permanent: &mut BTreeSet<String>,
        order: &mut Vec<String>,
    ) -> Result<(), ModError> {
        if permanent.contains(id) {
            return Ok(());
        }
        if let Some(start) = temporary.iter().position(|item| item == id) {
            let mut cycle = temporary[start..].to_vec();
            cycle.push(id.to_string());
            return Err(fail(
                manifest,
                format!("dependency cycle: {}", cycle.join(" -> ")),
            ));
        }
        temporary.push(id.to_string());
        for dependency_id in manifest.dependencies.0.keys() {
            let dependency = packages
                .get(dependency_id)
                .expect("LR3 proved dependency is provided");
            visit(
                dependency_id,
                dependency,
                packages,
                temporary,
                permanent,
                order,
            )?;
        }
        temporary.pop();
        permanent.insert(id.to_string());
        order.push(id.to_string());
        Ok(())
    }

    let mut order = Vec::new();
    visit(
        &root.id,
        root,
        packages,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut order,
    )?;
    Ok(order)
}

fn lr8_reachable(
    root: &Manifest,
    available: &[Manifest],
    order: &[String],
) -> Result<(), ModError> {
    let reachable: BTreeSet<_> = order.iter().cloned().collect();
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
