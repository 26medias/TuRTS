use crate::content_mc_validation::ParsedMcContent;
use crate::raw_presence::{ANY_KEY, RawDocuments, RawGroup};
use crate::variants::{self, VariantUse};
use semver::{Version, VersionReq};
use turts_mod_api::{Manifest, ModError, RELEASED_ENGINE_APIS};

use crate::archive::MANIFEST_PATH;

/// How a row finds out that a package uses its feature (design 6.10, "Version-table
/// detection"). This table is the one source of truth: a feature has one row, one text.
#[derive(Clone, Copy)]
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
    /// VAR and COMBO rows of API 0.2.4: the parsed value holds a new variant, or a combination
    /// of fields on one unit. `variants::collect` walks the parsed files once and lists every
    /// use by name; the row finds the use with its own name and reports the file and the field
    /// path where the value knows them. A `Written` field of a variant (`Button.font`) is
    /// listed there too.
    Variant(&'static str),
}

/// Where a row found its feature: the file and the field path. Both are empty for a row of the
/// parsed value that knows no place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Located {
    pub(crate) file: String,
    pub(crate) path: String,
}

impl Located {
    /// The text that a version message appends: empty for the manifest or for no place,
    /// ` (written in <file>: <path>)` otherwise (design 6.10, [mod L3]).
    pub(crate) fn written_in(&self) -> String {
        if self.file.is_empty() || self.file == MANIFEST_PATH {
            String::new()
        } else {
            format!(" (written in {}: {})", self.file, self.path)
        }
    }
}

/// Find the feature of `detect` in the package. `uses` is `variants::collect` of the parsed
/// value (empty while there is no parsed value).
pub(crate) fn locate(
    detect: &Detect,
    manifest: &Manifest,
    content: Option<&ParsedMcContent>,
    raw: &RawDocuments,
    uses: &[VariantUse],
) -> Option<Located> {
    match detect {
        Detect::Raw { group, path } => {
            let file = raw.written(*group, path)?;
            Some(Located {
                file: file.to_string(),
                // `*` (any key of a map) stays in the path: `units.*.role`.
                path: path.join("."),
            })
        }
        Detect::Parsed(present) => present(manifest, content?).then(|| Located {
            file: String::new(),
            path: String::new(),
        }),
        Detect::Variant(variant) => {
            uses.iter()
                .find(|used| used.name == *variant)
                .map(|used| Located {
                    file: used.file.clone(),
                    path: used.path.clone(),
                })
        }
    }
}

/// The uses of the variants in the parsed value, or none while there is no parsed value.
pub(crate) fn variant_uses(
    manifest: &Manifest,
    content: Option<&ParsedMcContent>,
) -> Vec<VariantUse> {
    content.map_or_else(Vec::new, |content| variants::collect(manifest, content))
}

#[derive(Clone, Copy)]
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

/// A RAW row of API 0.2.4.
const fn raw_0_2_4(
    name: &'static str,
    group: RawGroup,
    path: &'static [&'static str],
) -> VersionTableRow {
    raw(name, "0.2.4", group, path)
}

/// A VAR or COMBO row of API 0.2.4: the parsed value holds the use (`variants.rs`).
const fn variant_0_2_4(name: &'static str) -> VersionTableRow {
    VersionTableRow {
        name,
        detect: Detect::Variant(name),
        since_api: "0.2.4",
    }
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
    // API 0.2.4 (design 6.10, revision 6.2): one row for each new field, variant and
    // combination. A RAW row reads the source file (any value, also an empty one); a VAR or
    // COMBO row reads the parsed value (`Detect::Variant`, `variants.rs`). The order is the
    // order of the first refusal. The rows of the rules come before the rows of the orders,
    // and `target: ContentAtPoint` before `behaviour Construct`, so that a package whose rule
    // names a new order hears about the rule first. The edition rows T1 to T4 refuse nothing:
    // see `edition.rs`.
    // A1, A2, A3: the export groups.
    raw_0_2_4("exports.combat", RawGroup::Manifest, &["exports", "combat"]),
    raw_0_2_4("exports.match", RawGroup::Manifest, &["exports", "match"]),
    raw_0_2_4(
        "exports.visual_effects",
        RawGroup::Manifest,
        &["exports", "visual_effects"],
    ),
    // U1 to U8, K1 to K3, R1: the fields of a unit.
    raw_0_2_4("armour", RawGroup::Units, &["armour"]),
    raw_0_2_4(
        "weapon.damage_type",
        RawGroup::Units,
        &["weapon", "damage_type"],
    ),
    raw_0_2_4("acquire", RawGroup::Units, &["acquire"]),
    raw_0_2_4("provides", RawGroup::Units, &["provides"]),
    raw_0_2_4("consumes", RawGroup::Units, &["consumes"]),
    raw_0_2_4(
        "weapon.when_short",
        RawGroup::Units,
        &["weapon", "when_short"],
    ),
    raw_0_2_4(
        "producer.when_short",
        RawGroup::Units,
        &["producer", "when_short"],
    ),
    raw_0_2_4("income", RawGroup::Units, &["income"]),
    raw_0_2_4("spawn_order", RawGroup::Units, &["spawn_order"]),
    raw_0_2_4("constructor", RawGroup::Units, &["constructor"]),
    raw_0_2_4(
        "constructor.when_short",
        RawGroup::Units,
        &["constructor", "when_short"],
    ),
    raw_0_2_4("placement", RawGroup::Units, &["placement"]),
    raw_0_2_4("build_area", RawGroup::Units, &["build_area"]),
    raw_0_2_4("repairer", RawGroup::Units, &["repairer"]),
    // K5, O8, O10: the fields of a production and a technology.
    raw_0_2_4(
        "cancel_refunds.destroyed",
        RawGroup::Production,
        &["cancel_refunds", "destroyed"],
    ),
    raw_0_2_4(
        "technology cancel_refunds.destroyed",
        RawGroup::Technologies,
        &["cancel_refunds", "destroyed"],
    ),
    raw_0_2_4("hotkey", RawGroup::Production, &["hotkey"]),
    raw_0_2_4("card", RawGroup::Production, &["card"]),
    // M3: a map placement.
    raw_0_2_4(
        "start_order",
        RawGroup::Maps,
        &["placements", "start_order"],
    ),
    // O7: the rally list of the order rules.
    raw_0_2_4("rally", RawGroup::Orders, &["rally"]),
    // H1 to H3, H5, H6, H11, H12, H14, H16 to H22, H24: the fields of an interface file.
    raw_0_2_4("hints", RawGroup::Interface, &["hints"]),
    raw_0_2_4("hint_fallback", RawGroup::Interface, &["hint_fallback"]),
    raw_0_2_4("start_selection", RawGroup::Interface, &["start_selection"]),
    raw_0_2_4("states", RawGroup::Interface, &["states"]),
    raw_0_2_4(
        "alert_sound_gap_secs",
        RawGroup::Interface,
        &["alert_sound_gap_secs"],
    ),
    raw_0_2_4("effects_voices", RawGroup::Interface, &["effects_voices"]),
    raw_0_2_4("audible_range", RawGroup::Interface, &["audible_range"]),
    raw_0_2_4("alerts.units", RawGroup::Interface, &["alerts", "units"]),
    raw_0_2_4("alerts.kind", RawGroup::Interface, &["alerts", "kind"]),
    raw_0_2_4(
        "alerts.clear_after",
        RawGroup::Interface,
        &["alerts", "clear_after"],
    ),
    raw_0_2_4(
        "alerts.select_on_click",
        RawGroup::Interface,
        &["alerts", "select_on_click"],
    ),
    raw_0_2_4("alerts.focus", RawGroup::Interface, &["alerts", "focus"]),
    raw_0_2_4(
        "alerts.sound_when",
        RawGroup::Interface,
        &["alerts", "sound_when"],
    ),
    raw_0_2_4("role", RawGroup::Interface, &["units", ANY_KEY, "role"]),
    raw_0_2_4("cursors.place", RawGroup::Interface, &["cursors", "place"]),
    raw_0_2_4(
        "cursors.repair",
        RawGroup::Interface,
        &["cursors", "repair"],
    ),
    raw_0_2_4("cursors.rally", RawGroup::Interface, &["cursors", "rally"]),
    raw_0_2_4(
        "world.placement",
        RawGroup::Interface,
        &["world", "placement"],
    ),
    raw_0_2_4(
        "world.build_area_ring",
        RawGroup::Interface,
        &["world", "build_area_ring"],
    ),
    raw_0_2_4(
        "world.gap_ring",
        RawGroup::Interface,
        &["world", "gap_ring"],
    ),
    raw_0_2_4(
        "world.progress_bar",
        RawGroup::Interface,
        &["world", "progress_bar"],
    ),
    raw_0_2_4(
        "world.cargo_bar",
        RawGroup::Interface,
        &["world", "cargo_bar"],
    ),
    raw_0_2_4("world.rally", RawGroup::Interface, &["world", "rally"]),
    raw_0_2_4(
        "world.float_text",
        RawGroup::Interface,
        &["world", "float_text"],
    ),
    raw_0_2_4(
        "world.float_text.sound",
        RawGroup::Interface,
        &["world", "float_text", "sound"],
    ),
    raw_0_2_4(
        "world.unit_labels",
        RawGroup::Interface,
        &["world", "unit_labels"],
    ),
    raw_0_2_4(
        "world.health_bar.scale_to_footprint",
        RawGroup::Interface,
        &["world", "health_bar", "scale_to_footprint"],
    ),
    // V1 to V4: models, visuals and effects.
    raw_0_2_4("models", RawGroup::Assets, &["models"]),
    raw_0_2_4("model", RawGroup::Visuals, &["model"]),
    raw_0_2_4("effects", RawGroup::Visuals, &["effects"]),
    raw_0_2_4("effects.hit", RawGroup::Visuals, &["effects", "hit"]),
    raw_0_2_4(
        "effects.hit_reduced",
        RawGroup::Visuals,
        &["effects", "hit_reduced"],
    ),
    raw_0_2_4(
        "effects.damaged",
        RawGroup::Visuals,
        &["effects", "damaged"],
    ),
    raw_0_2_4(
        "visual effect sound",
        RawGroup::VisualEffects,
        &["effects", "sound"],
    ),
    // O6: the new smart targets and capabilities (the rules come first).
    variant_0_2_4("smart target OwnDamaged"),
    variant_0_2_4("needs: Constructor"),
    variant_0_2_4("needs: Repairer"),
    // O5, O1 to O4, O9: the new target and behaviours.
    variant_0_2_4("target: ContentAtPoint"),
    variant_0_2_4("behaviour Construct"),
    variant_0_2_4("behaviour CancelConstruction"),
    variant_0_2_4("behaviour SetRally"),
    variant_0_2_4("behaviour Repair"),
    variant_0_2_4("behaviour Surrender"),
    // S1, P1, L1.
    variant_0_2_4("kind: Supply"),
    variant_0_2_4("payment: ChargeOverTime"),
    variant_0_2_4("SlotRelation::Ally"),
    variant_0_2_4("SlotRelation::Enemy"),
    // COMBO rows K1, K4.
    variant_0_2_4("constructor beside producer"),
    variant_0_2_4("production with a static output"),
    // K7 and K8 (`health without removal`, `removal with a queue`) have no row yet: the two M3
    // rules are in force for every edition until `L0-API2` lifts them (`content_m3_validation.rs`
    // checks them after this table), so a row would refuse with the false advice "use 0.2.4".
    // `L0-API2` adds both rows with the lift. The uses are listed (`variants.rs`) and have lines
    // in `unbuilt.rs`.
    // H4: the new alert triggers.
    variant_0_2_4("AlertTrigger::ConstructionComplete"),
    variant_0_2_4("AlertTrigger::SupplyShort"),
    variant_0_2_4("AlertTrigger::SupplyRestored"),
    variant_0_2_4("AlertTrigger::InsufficientFunds"),
    variant_0_2_4("AlertTrigger::SlotDisconnected"),
    variant_0_2_4("AlertTrigger::SlotReconnected"),
    variant_0_2_4("AlertTrigger::GathererIdle"),
    // H7: the new values and sources.
    variant_0_2_4("Value::Supply"),
    variant_0_2_4("Value::ConstructionQueue"),
    variant_0_2_4("Value::Sites"),
    variant_0_2_4("Value::Notices"),
    variant_0_2_4("Value::SelectionTypes"),
    variant_0_2_4("Value::Connection"),
    variant_0_2_4("Value::Outcome"),
    variant_0_2_4("Value::OutcomeReason"),
    variant_0_2_4("Value::MatchStats"),
    variant_0_2_4("Value::MatchClock"),
    variant_0_2_4("Value::Hint"),
    variant_0_2_4("Source::ConstructionQueue"),
    variant_0_2_4("Source::Sites"),
    variant_0_2_4("Source::Notices"),
    variant_0_2_4("Source::SelectionTypes"),
    // H8, H9: the new primary and item fields.
    variant_0_2_4("PrimaryField::Activity"),
    variant_0_2_4("PrimaryField::Cargo"),
    variant_0_2_4("PrimaryField::Armour"),
    variant_0_2_4("ItemField::Role"),
    variant_0_2_4("ItemField::BuildTimeNow"),
    variant_0_2_4("ItemField::SupplyChange"),
    variant_0_2_4("ItemField::CausesShortage"),
    variant_0_2_4("ItemField::State"),
    variant_0_2_4("ItemField::Paid"),
    variant_0_2_4("ItemField::Refund"),
    variant_0_2_4("ItemField::Badge"),
    // H10: the new predicates.
    variant_0_2_4("Predicate::ItemHas"),
    variant_0_2_4("Predicate::PrimaryHasValue"),
    variant_0_2_4("Predicate::SupplyShort"),
    variant_0_2_4("Predicate::MatchOver"),
    variant_0_2_4("Predicate::OutcomeIs"),
    variant_0_2_4("Predicate::OutcomeReasonIs"),
    variant_0_2_4("Predicate::Placing"),
    variant_0_2_4("Predicate::HintShown"),
    // H13: `font` and `size` of a `Button`, a key marker on the typed value (`Written`).
    variant_0_2_4("Button.font"),
    variant_0_2_4("Button.size"),
    // H23: the engine input action `leave_match`.
    variant_0_2_4("engine action leave_match"),
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

impl Detect {
    /// True for a RAW detector on a file that only the client reads.
    pub(crate) fn is_presentation(&self) -> bool {
        matches!(self, Detect::Raw { group, .. } if group.is_presentation())
    }
}

/// The rows that run on the server path (`presentation` false) or on the client path (true).
pub(crate) fn rows_for(presentation: bool) -> Vec<VersionTableRow> {
    VERSION_TABLE
        .iter()
        .filter(|row| row.detect.is_presentation() == presentation)
        .copied()
        .collect()
}

/// The groups that the RAW rows of `rows` read.
pub(crate) fn raw_groups(rows: &[VersionTableRow]) -> impl Iterator<Item = RawGroup> + '_ {
    rows.iter().filter_map(|row| match row.detect {
        Detect::Raw { group, .. } => Some(group),
        Detect::Parsed(_) | Detect::Variant(_) => None,
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
        uses: &[VariantUse],
    ) -> Option<String> {
        locate(&self.detect, manifest, content, raw, uses).map(|located| located.written_in())
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

    let uses = variant_uses(manifest, content);
    for row in rows {
        let Some(written_in) = row.used(manifest, content, raw, &uses) else {
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
                "0.2.4" => ("^0.2.3", "0.2.3"),
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
        // 6 rows of 0.2.1 and 0.2.3, and `exports.combat`, `exports.match`,
        // `exports.visual_effects` of 0.2.4.
        assert_eq!(raw_rows, 9);
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

    /// Every RAW row of API 0.2.4 finds its path on its own: written with an empty value in the
    /// file of its group, a `^0.2.3` package is refused with the file and the path; at `^0.2.4`
    /// the table passes; with the field absent there is no refusal at all. A row that reads
    /// the wrong group or path is red here.
    #[test]
    fn every_raw_row_of_0_2_4_is_found_on_its_own() {
        use super::support::raw_for;
        let mut tested = 0;
        for row in VERSION_TABLE {
            let Detect::Raw { group, path } = row.detect else {
                continue;
            };
            if row.since_api != "0.2.4" {
                continue;
            }
            tested += 1;
            let rows = std::slice::from_ref(row);
            let (manifest, raw) = raw_for(row, "^0.2.3", true);
            let error = check_version_table(&manifest, None, &raw, rows).unwrap_err();
            let written_in = if group == RawGroup::Manifest {
                String::new()
            } else {
                format!(" (written in f.ron: {})", path.join("."))
            };
            assert_eq!(
                error.to_string(),
                format!(
                    "zeta/manifest.ron: engine_api: `{}` needs engine api 0.2.4; `^0.2.3` admits 0.2.3{written_in}",
                    row.name
                ),
                "row {}",
                row.name
            );
            let (manifest, raw) = raw_for(row, "^0.2.4", true);
            check_version_table(&manifest, None, &raw, rows)
                .unwrap_or_else(|error| panic!("row {} at ^0.2.4: {error}", row.name));
            let (manifest, raw) = raw_for(row, "^0.2.3", false);
            check_version_table(&manifest, None, &raw, rows)
                .unwrap_or_else(|error| panic!("row {}, field absent: {error}", row.name));
        }
        assert_eq!(tested, 57, "the number of RAW rows of 0.2.4 (design 6.10)");
    }

    /// Every VAR and COMBO row names a use that `variants.rs` can list: a name that is not in
    /// that file is a row that can never fire.
    #[test]
    fn every_variant_row_names_a_use_that_the_collector_lists() {
        let collector = include_str!("variants.rs");
        let mut tested = 0;
        for row in VERSION_TABLE {
            if let Detect::Variant(name) = row.detect {
                tested += 1;
                assert_eq!(name, row.name, "a row finds the use of its own name");
                assert!(
                    collector.contains(&format!("\"{name}\"")),
                    "`{name}` is not listed by variants.rs"
                );
                assert_eq!(row.since_api, "0.2.4");
            }
        }
        assert_eq!(tested, 59, "the number of VAR and COMBO rows of 0.2.4");
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

/// Documents for the tests of the RAW rows, here and in `unbuilt.rs`: a manifest that lists one
/// file of the group of a row, and a document with the path of the row written in it.
#[cfg(test)]
pub(crate) mod support {
    use super::{Detect, VersionTableRow};
    use crate::raw_presence::{RawDocuments, RawGroup};
    use turts_mod_api::Manifest;

    fn field(group: RawGroup) -> &'static str {
        match group {
            RawGroup::Manifest => "",
            RawGroup::Units => "units",
            RawGroup::Maps => "maps",
            RawGroup::Visuals => "visuals",
            RawGroup::Resources => "resources",
            RawGroup::Nodes => "nodes",
            RawGroup::Production => "production",
            RawGroup::Recipes => "recipes",
            RawGroup::Technologies => "technologies",
            RawGroup::Orders => "orders",
            RawGroup::Interface => "interface",
            RawGroup::Input => "input",
            RawGroup::Assets => "assets",
            RawGroup::Combat => "combat",
            RawGroup::Match => "match",
            RawGroup::VisualEffects => "visual_effects",
        }
    }

    /// A document with the path written, each value `None` (`a.b.c` is `(a: (b: (c: None)))`;
    /// the name `*` is a map entry).
    pub(crate) fn doc(path: &[&str]) -> String {
        match path {
            [] => "None".to_string(),
            [first, "*", rest @ ..] => format!("({first}: {{\"k\": {}}})", doc(rest)),
            [first, rest @ ..] => format!("({first}: {})", doc(rest)),
        }
    }

    /// The manifest text at `engine_api`, with `extra` written into `exports`, and the files of
    /// the group of `row` (`with_field` writes the path of the row in the file of the group).
    pub(crate) fn manifest_text(engine_api: &str, group: RawGroup, with_field: &[&str]) -> String {
        let mut lists = String::from(
            "units: [], maps: [], visuals: [], resources: [], nodes: [], production: []",
        );
        if group == RawGroup::Manifest {
            if !with_field.is_empty() {
                lists.push_str(&format!(", {}: []", with_field[1]));
            }
        } else {
            let name = field(group);
            // The group is one of the six required lists, or one that is added.
            let list = format!("{name}: [\"f.ron\"]");
            if lists.contains(&format!("{name}: []")) {
                lists = lists.replace(&format!("{name}: []"), &list);
            } else {
                lists.push_str(&format!(", {list}"));
            }
        }
        format!(
            r#"(id: "zeta", version: "0.1.0", kind: Game, engine_api: "{engine_api}", dependencies: {{}}, exports: ({lists}), context: Shared, capabilities: [], license: "test")"#
        )
    }

    /// The manifest and the raw documents of the group of `row`. `written` writes the path of the
    /// row; false writes an empty document (the control: the field is absent).
    pub(crate) fn raw_for(
        row: &VersionTableRow,
        engine_api: &str,
        written: bool,
    ) -> (Manifest, RawDocuments) {
        let Detect::Raw { group, path } = row.detect else {
            panic!("row {} is not a RAW row", row.name);
        };
        let with: &[&str] = if written { path } else { &[] };
        let text = manifest_text(engine_api, group, with);
        let manifest: Manifest = ron::from_str(&text).unwrap();
        let document = if group == RawGroup::Manifest {
            text.clone()
        } else if written {
            doc(path)
        } else {
            "()".to_string()
        };
        let raw = RawDocuments::read(&manifest, [group], |file| {
            let wanted = if group == RawGroup::Manifest {
                "manifest.ron"
            } else {
                "f.ron"
            };
            (file == wanted).then_some(document.as_bytes())
        })
        .unwrap();
        (manifest, raw)
    }
}
