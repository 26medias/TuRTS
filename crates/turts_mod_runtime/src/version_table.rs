use crate::content_mc_validation::ParsedMcContent;
use crate::raw_presence::{RawDocuments, RawGroup};
use semver::{Version, VersionReq};
use turts_mod_api::{Manifest, ModError, RELEASED_ENGINE_APIS};

use crate::archive::MANIFEST_PATH;

/// How a row finds out that a package uses its feature (design 6.10, "Version-table
/// detection"). This table is the one source of truth: a feature has one row, one text.
pub(crate) enum Detect {
    /// RAW: the field is written in the source file, also with an empty or default value
    /// (`orders: []`, `armour: None`). `path` is a list of field names from the root of a file
    /// of one group; a list on the path is looked through (`["placements", "facing"]`). The
    /// files are read as `ron::Value`, which drops the name of an enum variant: do not use RAW
    /// for a field inside an enum variant.
    Raw {
        group: RawGroup,
        path: &'static [&'static str],
    },
    /// The parsed value. Use it for a VAR row (the parsed value holds the new variant, and no
    /// default makes a new variant), for a COMBO row (a combination of written fields on one
    /// parsed unit), and for a field that every file writes, so that a key walk cannot tell its
    /// use. For a field inside an enum variant, the field has the type `turts_mod_api::Written`:
    /// the function reads `is_written()`, which is true when the key is written, also with
    /// `None`. A plain `Option` field would accept `font: None` that an older engine refuses.
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

/// A RAW row.
const fn raw(
    name: &'static str,
    since_api: &'static str,
    group: RawGroup,
    path: &'static [&'static str],
) -> VersionTableRow {
    VersionTableRow {
        name,
        detect: Detect::Raw { group, path },
        since_api,
    }
}

/// A RAW row for an M3 field of a unit file (API 0.2.1).
const fn unit_field(name: &'static str, field: &'static [&'static str]) -> VersionTableRow {
    raw(name, "0.2.1", RawGroup::Units, field)
}

/// Add one row for each new feature of a released API. A later API adds its rows here with the
/// detection kind that its design row names, and nowhere else. The order is the order of the
/// first refusal when one package has several problems.
pub(crate) const VERSION_TABLE: &[VersionTableRow] = &[
    // API 0.2.2.
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
    // API 0.2.3.
    raw(
        "exports.orders",
        "0.2.3",
        RawGroup::Manifest,
        &["exports", "orders"],
    ),
    raw(
        "exports.interface",
        "0.2.3",
        RawGroup::Manifest,
        &["exports", "interface"],
    ),
    raw(
        "exports.input",
        "0.2.3",
        RawGroup::Manifest,
        &["exports", "input"],
    ),
    raw(
        "exports.assets",
        "0.2.3",
        RawGroup::Manifest,
        &["exports", "assets"],
    ),
    VersionTableRow {
        name: "weapon with movement",
        detect: Detect::Parsed(mobile_weapon_present),
        since_api: "0.2.3",
    },
    // API 0.2.1 (M3), last in the order: a field that the file writes, with any value. The meaning is the same as the
    // check that these rows replace: `exports.recipes` and `exports.technologies`, the nine unit
    // fields, the map field `belts`, and `facing` in a placement. They come after the rows of the later APIs so
    // that the first refusal of a package with several problems is the one that it was before.
    raw(
        "exports.recipes",
        "0.2.1",
        RawGroup::Manifest,
        &["exports", "recipes"],
    ),
    raw(
        "exports.technologies",
        "0.2.1",
        RawGroup::Manifest,
        &["exports", "technologies"],
    ),
    unit_field("unit.output_inventory", &["output_inventory"]),
    unit_field("unit.extractor", &["extractor"]),
    unit_field("unit.conveyor", &["conveyor"]),
    unit_field("unit.inserter", &["inserter"]),
    unit_field("unit.machine", &["machine"]),
    unit_field("unit.research", &["research"]),
    unit_field("unit.health", &["health"]),
    unit_field("unit.weapon", &["weapon"]),
    unit_field("unit.removal", &["removal"]),
    raw("map.belts", "0.2.1", RawGroup::Maps, &["belts"]),
    raw(
        "map.placements.facing",
        "0.2.1",
        RawGroup::Maps,
        &["placements", "facing"],
    ),
];

/// The groups that the RAW rows of `rows` read.
pub(crate) fn raw_groups(rows: &[VersionTableRow]) -> impl Iterator<Item = RawGroup> + '_ {
    rows.iter().filter_map(|row| match row.detect {
        Detect::Raw { group, .. } => Some(group),
        Detect::Parsed(_) => None,
    })
}

impl VersionTableRow {
    /// `Some(where)` when the package uses the feature of this row. `where` is the text that
    /// the message appends: empty, or ` (written in <file>: <path>)` for a RAW row on a file
    /// other than the manifest (design 6.10, [mod L3]). A parsed row is skipped while there is
    /// no parsed value yet (`content` is `None`).
    fn used(
        &self,
        manifest: &Manifest,
        content: Option<&ParsedMcContent>,
        raw: &RawDocuments,
    ) -> Option<String> {
        match self.detect {
            Detect::Raw { group, path } => {
                let file = raw.written(group, path)?;
                Some(if group == RawGroup::Manifest {
                    String::new()
                } else {
                    format!(" (written in {file}: {})", path.join("."))
                })
            }
            Detect::Parsed(present) => present(manifest, content?).then(String::new),
        }
    }
}

/// Refuse the first row that the package uses and that its `engine_api` requirement admits an
/// older API for. The error is in the form of today: file `manifest.ron`, field `engine_api`.
/// Call it first with `content = None`: the RAW rows need only the raw documents, so a modder
/// hears about the version before a content cross-check fails for the same reason. Call it again
/// with the parsed value for the parsed rows.
pub(crate) fn check_version_table(
    manifest: &Manifest,
    content: Option<&ParsedMcContent>,
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
        let Some(written_in) = row.used(manifest, content, raw) else {
            continue;
        };
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
                    "`{}` needs engine api {}; `{}` admits {version}{written_in}",
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
    use crate::raw_presence::{RawDocuments, RawGroup};
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
            Some(&ParsedMcContent::default()),
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

    /// Every RAW row on the manifest finds an explicit empty value: the version table reads the
    /// file, not the parsed value. (The rows on unit and map files are in
    /// `tests/raw_presence.rs`, which packs files.)
    #[test]
    fn every_manifest_raw_row_refuses_an_explicit_empty_value() {
        let mut raw_rows = 0;
        for row in VERSION_TABLE {
            let Detect::Raw {
                group: RawGroup::Manifest,
                path,
            } = row.detect
            else {
                continue;
            };
            raw_rows += 1;
            let [exports, field] = path else {
                panic!("row {}: the test knows the exports paths only", row.name);
            };
            assert_eq!(*exports, "exports", "row {}", row.name);
            let (requirement, admits) = match row.since_api {
                "0.2.1" => ("^0.2", "0.2.0"),
                "0.2.3" => ("^0.2.2", "0.2.2"),
                other => panic!("row {}: the test has no requirement for {other}", row.name),
            };
            let text = manifest_text_with_export(requirement, field)
                .replace("[\"new/content.ron\"]", "[]");
            let error = check(&text, VERSION_TABLE).unwrap_err();
            assert_eq!(
                error.to_string(),
                format!(
                    "zeta/manifest.ron: engine_api: `{}` needs engine api {}; `{requirement}` admits {admits}",
                    row.name, row.since_api
                )
            );
        }
        assert_eq!(raw_rows, 6);
    }

    /// A row on a `Written` field of an enum variant fires when the key is written, also with
    /// `None`, and stays quiet when the key is absent (design 6.10, a field inside a variant).
    #[test]
    fn a_written_marker_row_fires_on_an_explicit_none() {
        use turts_mod_api::Written;

        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        #[allow(dead_code)]
        enum Scratch {
            Button {
                #[serde(default)]
                font: Written<Option<String>>,
            },
        }
        fn font_written(text: &str) -> bool {
            let Scratch::Button { font } = ron::from_str(text).unwrap();
            font.is_written()
        }
        fn written_none(_: &Manifest, _: &ParsedMcContent) -> bool {
            font_written("Button(font: None)")
        }
        fn absent(_: &Manifest, _: &ParsedMcContent) -> bool {
            font_written("Button()")
        }
        let row = |detect| VersionTableRow {
            name: "Button.font",
            detect: Detect::Parsed(detect),
            since_api: "0.2.1",
        };
        let text = manifest_text("^0.2");
        assert_eq!(
            check(&text, &[row(written_none)]).unwrap_err().to_string(),
            "zeta/manifest.ron: engine_api: `Button.font` needs engine api 0.2.1; `^0.2` admits 0.2.0"
        );
        assert!(check(&text, &[row(absent)]).is_ok());
    }

    /// With no parsed value yet, a parsed row is skipped and a RAW row still fires. With the
    /// parsed value, the first row of the table order wins.
    #[test]
    fn raw_rows_run_before_the_parsed_value_exists() {
        let text =
            manifest_text_with_export("^0.2", "orders").replace("[\"new/content.ron\"]", "[]");
        let manifest: Manifest = ron::from_str(&text).unwrap();
        let orders = super::raw(
            "exports.orders",
            "0.2.3",
            RawGroup::Manifest,
            &["exports", "orders"],
        );
        let rows = [row(), orders];
        let raw = raw_of(&text, &rows);
        let without = check_version_table(&manifest, None, &raw, &rows).unwrap_err();
        assert_eq!(
            without.to_string(),
            "zeta/manifest.ron: engine_api: `exports.orders` needs engine api 0.2.3; `^0.2` admits 0.2.2"
        );
        let with = check_version_table(&manifest, Some(&ParsedMcContent::default()), &raw, &rows)
            .unwrap_err();
        assert!(with.to_string().contains("`new field`"), "{with}");
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
