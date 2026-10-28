//! Manifest validation.

use turts_mod_api::{ENGINE_API_VERSION, Manifest, ModError, PackageId};

use crate::archive::{MANIFEST_PATH, is_safe_path};

/// Check a parsed manifest. The first problem found is the error.
// M1 placeholder; format open (spec 15): version and engine_api use semver.
pub fn validate_manifest(m: &Manifest) -> Result<(), ModError> {
    let fail = |field: &str, message: String| {
        ModError::new(&m.id, field, message).with_file(MANIFEST_PATH)
    };

    PackageId::parse(&m.id).map_err(|message| fail("id", message))?;

    semver::Version::parse(&m.version).map_err(|error| {
        fail(
            "version",
            format!("version `{}` is not a semantic version: {error}", m.version),
        )
    })?;

    let requirement = semver::VersionReq::parse(&m.engine_api).map_err(|error| {
        fail(
            "engine_api",
            format!(
                "engine api `{}` is not a version requirement: {error}",
                m.engine_api
            ),
        )
    })?;
    let engine = semver::Version::parse(ENGINE_API_VERSION)
        .expect("ENGINE_API_VERSION is a semantic version");
    if !requirement.matches(&engine) {
        return Err(fail(
            "engine_api",
            format!(
                "engine api `{}` does not accept engine `{ENGINE_API_VERSION}`",
                m.engine_api
            ),
        ));
    }

    if !m.dependencies.0.is_empty() {
        return Err(fail(
            "dependencies",
            "dependencies are not supported at M1".to_string(),
        ));
    }

    if m.exports.maps.len() != 1 {
        return Err(fail(
            "exports.maps",
            format!(
                "a package exports exactly one map at M1, found {}",
                m.exports.maps.len()
            ),
        ));
    }

    let exports = [
        ("exports.units", &m.exports.units),
        ("exports.maps", &m.exports.maps),
        ("exports.visuals", &m.exports.visuals),
        ("exports.resources", &m.exports.resources),
        ("exports.nodes", &m.exports.nodes),
        ("exports.production", &m.exports.production),
        ("exports.recipes", &m.exports.recipes),
        ("exports.technologies", &m.exports.technologies),
    ];
    for (field, paths) in exports {
        for path in paths {
            if !is_safe_path(path) || !path.ends_with(".ron") {
                return Err(fail(
                    field,
                    format!(
                        "export path `{path}` must be relative, stay inside the archive, and end with .ron"
                    ),
                ));
            }
        }
    }
    Ok(())
}
