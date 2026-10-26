//! Client visual validation and shape lookup.

use std::collections::{BTreeMap, HashMap};

use turts_mod_api::{ModError, Shape, VisualTarget};

use crate::archive::{MANIFEST_PATH, PackageArchive};

/// Shape for every authored unit and node in the supplied archives.
pub fn visuals_by_content(archives: &[PackageArchive]) -> Result<HashMap<String, Shape>, ModError> {
    let mut defined: BTreeMap<String, (String, &'static str)> = BTreeMap::new();
    for archive in archives {
        let package = archive.id();
        let catalog = archive.content()?;
        for id in catalog.units.keys() {
            defined.insert(id.to_string(), (package.0.clone(), "unit"));
        }
        for id in catalog.nodes.keys() {
            defined.insert(id.to_string(), (package.0.clone(), "node"));
        }
    }
    let mut found: HashMap<String, (String, Shape)> = HashMap::new();
    for archive in archives {
        let package = archive.manifest.id.as_str();
        for (file, visual) in archive.visuals()? {
            let (target, expected_kind) = match visual.target {
                VisualTarget::Unit(target) => (target, "unit"),
                VisualTarget::Node(target) => (target, "node"),
            };
            let error =
                |message: String| ModError::new(package, "target", message).with_file(&file);
            if !defined.contains_key(&target) {
                let allowed = defined
                    .iter()
                    .filter(|(_, (_, kind))| *kind == expected_kind)
                    .map(|(id, _)| id.clone())
                    .collect::<Vec<_>>();
                let error = error(format!(
                    "visual target `{target}` is not defined as a {expected_kind} by the archives"
                ));
                return Err(if allowed.len() <= 20 {
                    error.with_allowed(allowed)
                } else {
                    error
                });
            }
            if defined[&target].1 != expected_kind {
                return Err(error(format!(
                    "visual target `{target}` is a {}, but this visual declares a {expected_kind}",
                    defined[&target].1
                )));
            }
            if let Some((first, _)) = found.get(&target) {
                return Err(error(format!(
                    "target `{target}` already has a visual in `{first}`; a target has exactly one"
                )));
            }
            found.insert(target, (file, visual.shape));
        }
    }
    if let Some((target, (package, kind))) = defined.iter().find(|(id, _)| !found.contains_key(*id))
    {
        return Err(ModError::new(package, "exports.visuals", format!("{kind} `{target}` has no visual; add a file under visuals/ for it and list the file in exports.visuals")).with_file(MANIFEST_PATH));
    }
    Ok(found
        .into_iter()
        .map(|(id, (_, shape))| (id, shape))
        .collect())
}
