//! The client path check: visuals and units of a package match one to one.

use std::collections::{BTreeMap, HashMap};

use turts_mod_api::{ContentId, ModError, Shape};

use crate::archive::{MANIFEST_PATH, PackageArchive};

/// A message lists the defined unit ids only when there are this many or fewer.
const MAX_LISTED_UNITS: usize = 20;

/// The visual shape of each unit of `archives`, by full content id.
///
/// Every unit has exactly one visual, and every visual names a unit that a unit file of
/// `archives` defines (design 4.4). The client calls this when it accepts packages, and
/// `turts-pack` calls it before it writes an archive, so a modder sees the error before a
/// server starts. An error names the file and the field:
///
/// - A visual of an unknown unit: the visual file, field `unit`, with the defined unit ids
///   as the allowed names.
/// - A second visual for one unit: the second visual file, field `unit`.
/// - A unit without a visual: `manifest.ron`, field `exports.visuals`.
///
/// A visual file that does not parse is an error of that file (`PackageArchive::visuals`).
pub fn visuals_by_unit(archives: &[PackageArchive]) -> Result<HashMap<String, Shape>, ModError> {
    // Full content id -> the package that defines the unit.
    let mut defined: BTreeMap<String, String> = BTreeMap::new();
    for archive in archives {
        let package = archive.id();
        for (_, unit) in archive.units()? {
            defined.insert(
                ContentId::new(&package, &unit.id).to_string(),
                package.0.clone(),
            );
        }
    }

    // Full content id -> the file of its visual and the shape.
    let mut found: HashMap<String, (String, Shape)> = HashMap::new();
    for archive in archives {
        let package = archive.manifest.id.as_str();
        for (file, visual) in archive.visuals()? {
            let error = |message: String| ModError::new(package, "unit", message).with_file(&file);
            if !defined.contains_key(&visual.unit) {
                let error = error(format!(
                    "the visual is for unit `{}`, which no unit file of the package defines",
                    visual.unit
                ));
                return Err(if defined.len() <= MAX_LISTED_UNITS {
                    error.with_allowed(defined.keys().cloned().collect())
                } else {
                    error
                });
            }
            if let Some((first, _)) = found.get(&visual.unit) {
                return Err(error(format!(
                    "unit `{}` already has a visual in `{first}`; a unit has exactly one",
                    visual.unit
                )));
            }
            found.insert(visual.unit, (file, visual.shape));
        }
    }

    if let Some((unit, package)) = defined.iter().find(|(unit, _)| !found.contains_key(*unit)) {
        return Err(ModError::new(
            package,
            "exports.visuals",
            format!(
                "unit `{unit}` has no visual; add a file under visuals/ for it and list the \
                 file in exports.visuals"
            ),
        )
        .with_file(MANIFEST_PATH));
    }
    Ok(found
        .into_iter()
        .map(|(unit, (_, shape))| (unit, shape))
        .collect())
}
