use semver::{Version, VersionReq};
use turts_mod_api::{Manifest, ModError, RELEASED_ENGINE_APIS};

use crate::archive::MANIFEST_PATH;

pub(crate) struct VersionTableRow {
    pub(crate) name: &'static str,
    pub(crate) present: fn(&Manifest) -> bool,
    pub(crate) since_api: &'static str,
}

pub(crate) const VERSION_TABLE: &[VersionTableRow] = &[];

pub(crate) fn check_version_table(
    manifest: &Manifest,
    rows: &[VersionTableRow],
) -> Result<(), ModError> {
    let requirement = VersionReq::parse(&manifest.engine_api).map_err(|error| {
        ModError::new(
            &manifest.id,
            "engine_api",
            format!(
                "engine api `{}` is not a version requirement: {error}",
                manifest.engine_api
            ),
        )
        .with_file(MANIFEST_PATH)
    })?;

    for row in rows {
        if !(row.present)(manifest) {
            continue;
        }
        let since_api = Version::parse(row.since_api).expect("version table API is valid");
        let admitted = RELEASED_ENGINE_APIS
            .iter()
            .rev()
            .map(|version| Version::parse(version).expect("released engine API is valid"))
            .find(|version| version < &since_api && requirement.matches(version));
        if let Some(version) = admitted {
            return Err(ModError::new(
                &manifest.id,
                "engine_api",
                format!(
                    "`{}` needs engine api {}; `{}` admits `{version}`",
                    row.name, row.since_api, manifest.engine_api
                ),
            )
            .with_file(MANIFEST_PATH));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{VersionTableRow, check_version_table};
    use turts_mod_api::Manifest;

    fn manifest(engine_api: &str) -> Manifest {
        let text = format!(
            r#"(id: "zeta", version: "0.1.0", kind: Game, engine_api: "{engine_api}", dependencies: {{}}, exports: (units: [], maps: ["maps/test.ron"], visuals: [], resources: [], nodes: [], production: []), context: Shared, capabilities: [], license: "test")"#
        );
        ron::from_str(&text).unwrap()
    }

    fn always_present(_: &Manifest) -> bool {
        true
    }

    fn row() -> VersionTableRow {
        VersionTableRow {
            name: "new field",
            present: always_present,
            since_api: "0.2.1",
        }
    }

    #[test]
    fn sc3_row_since_0_2_1_refuses_caret_0_2() {
        let error = check_version_table(&manifest("^0.2"), &[row()]).unwrap_err();
        assert_eq!(error.field_path, "engine_api");
        assert!(error.message.contains("new field"), "{error}");
        assert!(error.message.contains("0.2.1"), "{error}");
        assert!(error.message.contains("`^0.2` admits `0.2.0`"), "{error}");
    }

    #[test]
    fn sc3_row_since_0_2_1_accepts_caret_0_2_1() {
        assert!(check_version_table(&manifest("^0.2.1"), &[row()]).is_ok());
    }
}
