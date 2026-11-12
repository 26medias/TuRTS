use crate::content_mc_validation::ParsedMcContent;
use crate::raw_presence::{RawDocuments, RawGroup};
use semver::{Version, VersionReq};
use turts_mod_api::{Manifest, ModError, RELEASED_ENGINE_APIS};

use crate::archive::MANIFEST_PATH;

/// How a row finds out that a package uses its feature (design 6.10, "Version-table
/// detection").
pub(crate) enum Detect {
    /// RAW: the field is written in the source file, also with an empty or default value
    /// (`orders: []`, `armour: None`). `path` is a list of field names from the root of a file
    /// of one group. The files are read as `ron::Value`, which drops the name of an enum
    /// variant: do not use RAW for a field inside an enum variant.
    Raw {
        group: RawGroup,
        path: &'static [&'static str],
    },
    /// The parsed value. Use it for a VAR row (the parsed value holds the new variant, and no
    /// default makes a new variant), for a COMBO row (a combination of written fields on one
    /// parsed unit), for a field inside an enum variant (a default never writes it), and for a
    /// field that every file writes, so that a key walk cannot tell its use.
    Parsed(fn(&Manifest, &ParsedMcContent) -> bool),
}

pub(crate) struct VersionTableRow {
    pub(crate) name: &'static str,
    pub(crate) detect: Detect,
    pub(crate) since_api: &'static str,
}

/// `dependencies` is a required field of the manifest: every manifest writes it, also `{}`.
/// A key walk would refuse every package below 0.2.2, so the row reads the parsed value.
fn has_dependencies(manifest: &Manifest, _: &ParsedMcContent) -> bool {
    !manifest.dependencies.0.is_empty()
}

/// VAR.
fn is_library(manifest: &Manifest, _: &ParsedMcContent) -> bool {
    manifest.kind == turts_mod_api::PackageKind::Library
}

/// COMBO: a unit with a weapon and movement.
fn mobile_weapon_present(_: &Manifest, content: &ParsedMcContent) -> bool {
    content.mobile_weapon_present
}

/// A RAW row for a field of `exports` in `manifest.ron`.
const fn export_group(name: &'static str, path: &'static [&'static str]) -> VersionTableRow {
    VersionTableRow {
        name,
        detect: Detect::Raw {
            group: RawGroup::Manifest,
            path,
        },
        since_api: "0.2.3",
    }
}

/// Add one row for each new feature of a released API. A later API adds its rows here with the
/// detection kind that its design row names, and nowhere else.
pub(crate) const VERSION_TABLE: &[VersionTableRow] = &[
    VersionTableRow {
        name: "dependencies",
        detect: Detect::Parsed(has_dependencies),
        since_api: "0.2.2",
    },
    VersionTableRow {
        name: "kind: Library",
        detect: Detect::Parsed(is_library),
        since_api: "0.2.2",
    },
    export_group("exports.orders", &["exports", "orders"]),
    export_group("exports.interface", &["exports", "interface"]),
    export_group("exports.input", &["exports", "input"]),
    export_group("exports.assets", &["exports", "assets"]),
    VersionTableRow {
        name: "weapon with movement",
        detect: Detect::Parsed(mobile_weapon_present),
        since_api: "0.2.3",
    },
];

/// The groups that the RAW rows of `rows` read.
pub(crate) fn raw_groups(rows: &[VersionTableRow]) -> impl Iterator<Item = RawGroup> + '_ {
    rows.iter().filter_map(|row| match row.detect {
        Detect::Raw { group, .. } => Some(group),
        Detect::Parsed(_) => None,
    })
}

impl VersionTableRow {
    fn is_present(
        &self,
        manifest: &Manifest,
        content: &ParsedMcContent,
        raw: &RawDocuments,
    ) -> bool {
        match self.detect {
            Detect::Raw { group, path } => raw.written(group, path),
            Detect::Parsed(present) => present(manifest, content),
        }
    }
}

pub(crate) fn check_version_table(
    manifest: &Manifest,
    content: &ParsedMcContent,
    raw: &RawDocuments,
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
        if !row.is_present(manifest, content, raw) {
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
    use super::{Detect, VERSION_TABLE, VersionTableRow, check_version_table, raw_groups};
    use crate::content_mc_validation::ParsedMcContent;
    use crate::raw_presence::RawDocuments;
    use turts_mod_api::{Manifest, ModError};

    fn manifest_text(engine_api: &str) -> String {
        format!(
            r#"(id: "zeta", version: "0.1.0", kind: Game, engine_api: "{engine_api}", dependencies: {{}}, exports: (units: [], maps: ["maps/test.ron"], visuals: [], resources: [], nodes: [], production: []), context: Shared, capabilities: [], license: "test")"#
        )
    }

    fn always_present(_: &Manifest, _: &ParsedMcContent) -> bool {
        true
    }

    fn row() -> VersionTableRow {
        VersionTableRow {
            name: "new field",
            detect: Detect::Parsed(always_present),
            since_api: "0.2.1",
        }
    }

    fn manifest_text_with_export(engine_api: &str, field: &str) -> String {
        format!(
            r#"(id: "zeta", version: "0.1.0", kind: Game, engine_api: "{engine_api}", dependencies: {{}}, exports: (units: [], maps: ["maps/test.ron"], visuals: [], resources: [], nodes: [], production: [], {field}: ["new/content.ron"]), context: Shared, capabilities: [], license: "test")"#
        )
    }

    /// The raw documents of a manifest text for the RAW rows of `rows`.
    fn raw_of(text: &str, rows: &[VersionTableRow]) -> RawDocuments {
        let manifest: Manifest = ron::from_str(text).unwrap();
        RawDocuments::read(&manifest, raw_groups(rows), |path| {
            (path == "manifest.ron").then_some(text.as_bytes())
        })
        .unwrap()
    }

    fn check(text: &str, rows: &[VersionTableRow]) -> Result<(), ModError> {
        let manifest: Manifest = ron::from_str(text).unwrap();
        check_version_table(
            &manifest,
            &ParsedMcContent::default(),
            &raw_of(text, rows),
            rows,
        )
    }

    #[test]
    fn sc4_new_export_groups_require_api_0_2_3() {
        for field in ["orders", "interface", "input", "assets"] {
            let old = manifest_text_with_export("^0.2.2", field);
            assert!(
                check(&old, VERSION_TABLE).is_err(),
                "{field} exports accepted engine_api ^0.2.2"
            );
            let current = manifest_text_with_export("^0.2.3", field);
            assert!(
                check(&current, VERSION_TABLE).is_ok(),
                "{field} exports rejected engine_api ^0.2.3"
            );
        }
    }

    /// Every RAW row of the table finds an explicit empty value: the version table reads the
    /// file, not the parsed value.
    #[test]
    fn every_raw_row_refuses_an_explicit_empty_value() {
        let mut raw_rows = 0;
        for row in VERSION_TABLE {
            let Detect::Raw { path, .. } = row.detect else {
                continue;
            };
            raw_rows += 1;
            let [exports, field] = path else {
                panic!("row {}: the test knows the exports paths only", row.name);
            };
            assert_eq!(*exports, "exports", "row {}", row.name);
            let text =
                manifest_text_with_export("^0.2.2", field).replace("[\"new/content.ron\"]", "[]");
            let error = check(&text, VERSION_TABLE).unwrap_err();
            assert_eq!(
                error.to_string(),
                format!(
                    "zeta/manifest.ron: engine_api: `{}` needs engine api {}; `^0.2.2` admits 0.2.2",
                    row.name, row.since_api
                )
            );
        }
        assert_eq!(raw_rows, 4);
    }

    #[test]
    fn sc3_row_since_0_2_1_refuses_caret_0_2() {
        let rows = [row()];
        let error = check(&manifest_text("^0.2"), &rows).unwrap_err();
        assert_eq!(
            error.to_string(),
            "zeta/manifest.ron: engine_api: `new field` needs engine api 0.2.1; `^0.2` admits 0.2.0"
        );
    }

    #[test]
    fn sc3_row_since_0_2_1_accepts_caret_0_2_1() {
        assert!(check(&manifest_text("^0.2.1"), &[row()]).is_ok());
    }
}
