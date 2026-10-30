use crate::content_mc_validation::ParsedMcContent;
use semver::{Version, VersionReq};
use turts_mod_api::{Manifest, ModError, RELEASED_ENGINE_APIS};

use crate::archive::MANIFEST_PATH;

pub(crate) struct VersionTableRow {
    pub(crate) name: &'static str,
    pub(crate) present: fn(&Manifest, &ParsedMcContent) -> bool,
    pub(crate) since_api: &'static str,
}

fn has_dependencies(manifest: &Manifest, _: &ParsedMcContent) -> bool {
    !manifest.dependencies.0.is_empty()
}

fn is_library(manifest: &Manifest, _: &ParsedMcContent) -> bool {
    manifest.kind == turts_mod_api::PackageKind::Library
}

pub(crate) const VERSION_TABLE: &[VersionTableRow] = &[
    VersionTableRow {
        name: "dependencies",
        present: has_dependencies,
        since_api: "0.2.2",
    },
    VersionTableRow {
        name: "kind: Library",
        present: is_library,
        since_api: "0.2.2",
    },
    VersionTableRow {
        name: "exports.orders",
        present: orders_present,
        since_api: "0.2.3",
    },
    VersionTableRow {
        name: "exports.interface",
        present: interface_present,
        since_api: "0.2.3",
    },
    VersionTableRow {
        name: "exports.input",
        present: input_present,
        since_api: "0.2.3",
    },
    VersionTableRow {
        name: "exports.assets",
        present: assets_present,
        since_api: "0.2.3",
    },
];

fn orders_present(_: &Manifest, content: &ParsedMcContent) -> bool {
    content.orders_present
}
fn interface_present(_: &Manifest, content: &ParsedMcContent) -> bool {
    content.interface_present
}
fn input_present(_: &Manifest, content: &ParsedMcContent) -> bool {
    content.input_present
}
fn assets_present(_: &Manifest, content: &ParsedMcContent) -> bool {
    content.assets_present
}

pub(crate) fn check_version_table(
    manifest: &Manifest,
    content: &ParsedMcContent,
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
        if !(row.present)(manifest, content) {
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
                    "`{}` needs engine api {}; `{}` admits {version}",
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
    use super::{VERSION_TABLE, VersionTableRow, check_version_table};
    use crate::content_mc_validation::ParsedMcContent;
    use turts_mod_api::Manifest;

    fn manifest(engine_api: &str) -> Manifest {
        let text = format!(
            r#"(id: "zeta", version: "0.1.0", kind: Game, engine_api: "{engine_api}", dependencies: {{}}, exports: (units: [], maps: ["maps/test.ron"], visuals: [], resources: [], nodes: [], production: []), context: Shared, capabilities: [], license: "test")"#
        );
        ron::from_str(&text).unwrap()
    }

    fn always_present(_: &Manifest, _: &ParsedMcContent) -> bool {
        true
    }

    fn row() -> VersionTableRow {
        VersionTableRow {
            name: "new field",
            present: always_present,
            since_api: "0.2.1",
        }
    }

    fn manifest_with_export(engine_api: &str, field: &str) -> Manifest {
        let text = format!(
            r#"(id: "zeta", version: "0.1.0", kind: Game, engine_api: "{engine_api}", dependencies: {{}}, exports: (units: [], maps: ["maps/test.ron"], visuals: [], resources: [], nodes: [], production: [], {field}: ["new/content.ron"]), context: Shared, capabilities: [], license: "test")"#
        );
        ron::from_str(&text).unwrap()
    }

    #[test]
    fn sc4_new_export_groups_require_api_0_2_3() {
        for field in ["orders", "interface", "input", "assets"] {
            let old = manifest_with_export("^0.2.2", field);
            let content = match field {
                "orders" => ParsedMcContent {
                    orders_present: true,
                    ..Default::default()
                },
                "interface" => ParsedMcContent {
                    interface_present: true,
                    ..Default::default()
                },
                "input" => ParsedMcContent {
                    input_present: true,
                    ..Default::default()
                },
                "assets" => ParsedMcContent {
                    assets_present: true,
                    ..Default::default()
                },
                _ => unreachable!(),
            };
            assert!(
                check_version_table(&old, &content, VERSION_TABLE).is_err(),
                "{field} exports accepted engine_api ^0.2.2"
            );
            let current = manifest_with_export("^0.2.3", field);
            assert!(
                check_version_table(&current, &content, VERSION_TABLE).is_ok(),
                "{field} exports rejected engine_api ^0.2.3"
            );
        }
    }

    #[test]
    fn sc3_row_since_0_2_1_refuses_caret_0_2() {
        let error = check_version_table(&manifest("^0.2"), &ParsedMcContent::default(), &[row()])
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "zeta/manifest.ron: engine_api: `new field` needs engine api 0.2.1; `^0.2` admits 0.2.0"
        );
    }

    #[test]
    fn sc3_row_since_0_2_1_accepts_caret_0_2_1() {
        assert!(
            check_version_table(&manifest("^0.2.1"), &ParsedMcContent::default(), &[row()]).is_ok()
        );
    }
}
