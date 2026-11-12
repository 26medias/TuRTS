//! `UNBUILT_FEATURES`: the 0.2.4 features whose engine behaviour no lane has merged yet (plan
//! 5.3; design 6.8, "One mod API step"). The packer refuses a package that uses one, with the
//! text `<package>/<file>: <field path>: `<feature>` is not built in this engine build; leave it
//! out until an engine build supports it`. The text names no plan task: a modder cannot act on
//! one, so the owner stays in a comment on each line (orchestrator ruling M1 on the review of
//! `L0-API1`; design 6.8, "One mod API step", gives the first half of the text). So the packages
//! `frontier` and `standoff` cannot pack before their mechanisms exist, and a half-built feature
//! can never run.
//!
//! Each feature lane deletes its own lines when it merges, and only those lines (the owner is
//! in each line). A line with no detector of its own finds its feature in the version table by
//! its name (`version_table::VERSION_TABLE`): one detector for one feature. The check runs
//! after the version table (a `^0.2.3` package hears about the version first), in two phases
//! like the table: the RAW lines on the raw documents first, then the lines that read the
//! parsed value. The lines apply to the edition 0.2.4 only (`edition.rs`): a package whose
//! requirement admits an older API keeps the behaviour of that API, so a lifted rule (`health
//! without removal`) and a placeholder in a text (`{unit}`) are not refused for it.

use crate::content_mc_validation::ParsedMcContent;
use crate::edition::is_edition_0_2_4;
use crate::raw_presence::RawDocuments;
use crate::version_table::{Detect, VersionTableRow, locate, variant_uses};
use turts_mod_api::{Manifest, ModError};

use crate::archive::MANIFEST_PATH;

/// One line: a feature that is not built, and the task that builds it.
#[derive(Clone, Copy)]
pub(crate) struct UnbuiltLine {
    /// The name in the message. For a line with no `detect` it is the name of a row of the
    /// version table.
    pub(crate) feature: &'static str,
    /// The task of plan 5.3 that removes the line. It is for the code and the tests only: the
    /// message never prints it.
    pub(crate) owner: &'static str,
    /// A detector of its own, for a feature that has no row in the version table (a stricter
    /// use of a variant that a row already covers).
    pub(crate) detect: Option<Detect>,
}

const fn line(feature: &'static str, owner: &'static str) -> UnbuiltLine {
    UnbuiltLine {
        feature,
        owner,
        detect: None,
    }
}

/// A line that reads a use by its own name.
const fn use_line(feature: &'static str, owner: &'static str) -> UnbuiltLine {
    UnbuiltLine {
        feature,
        owner,
        detect: Some(Detect::Variant(feature)),
    }
}

/// The lines of plan 5.3, in its order. A task deletes only the lines of its own name.
pub(crate) const UNBUILT_FEATURES: &[UnbuiltLine] = &[
    // L-DMG: `exports.combat` (A1), `armour` (U1), `weapon.damage_type` (U2).
    line("exports.combat", "L-DMG"),
    line("armour", "L-DMG"),
    line("weapon.damage_type", "L-DMG"),
    // L-TGT: `acquire` (U4).
    line("acquire", "L-TGT"),
    // L-PWR: `kind: Supply` (S1), `provides`, `consumes` (U5), `*.when_short` (U6).
    line("kind: Supply", "L-PWR"),
    line("provides", "L-PWR"),
    line("consumes", "L-PWR"),
    line("weapon.when_short", "L-PWR"),
    line("producer.when_short", "L-PWR"),
    line("constructor.when_short", "L-PWR"),
    // L-PAY: `payment: ChargeOverTime` (P1).
    line("payment: ChargeOverTime", "L-PAY"),
    // L-INC: `income` (U7).
    line("income", "L-INC"),
    // L-CON1: `constructor` (K1), `placement` (K2), `build_area` (K3), production with a static
    // output (K4), `ContentAtPoint` (O5), `Construct` (O1), `needs: Constructor` (O6). The target
    // comes first, as in the version table.
    line("constructor", "L-CON1"),
    line("placement", "L-CON1"),
    line("build_area", "L-CON1"),
    line("production with a static output", "L-CON1"),
    line("target: ContentAtPoint", "L-CON1"),
    line("behaviour Construct", "L-CON1"),
    line("needs: Constructor", "L-CON1"),
    // L-DES: `health without removal` (K7), `removal with a queue` (K8),
    // `cancel_refunds.destroyed` (K5).
    use_line("health without removal", "L-DES"),
    use_line("removal with a queue", "L-DES"),
    line("cancel_refunds.destroyed", "L-DES"),
    line("technology cancel_refunds.destroyed", "L-DES"),
    // L-STO: `start_order` (M3), `spawn_order` (U8) with `ground_rally: Ignore`.
    line("start_order", "L-STO"),
    line("spawn_order", "L-STO"),
    // L-MATCH-SRV1: `exports.match` (A2) with `AllSeated(countdown_secs: 0)` and `KeyUnitsLost`.
    line("exports.match", "L-MATCH-SRV1"),
    // L-LOBBY2-SRV: `AllSeated` above 0, `AllSeatedReady` (M2).
    use_line("AllSeated with countdown_secs above 0", "L-LOBBY2-SRV"),
    use_line("AllSeatedReady", "L-LOBBY2-SRV"),
    // L-MATCH-SIM2: `DisconnectedFor` (M1), `Surrender` (O9).
    use_line("DisconnectedFor", "L-MATCH-SIM2"),
    line("behaviour Surrender", "L-MATCH-SIM2"),
    // L-CON3: `CancelConstruction` (O2).
    line("behaviour CancelConstruction", "L-CON3"),
    // L-RLY: `SetRally` (O3), `rally` (O7), `ground_rally: ApplyFirst` (U8), smart `Ground` with
    // `needs: Producer` (O6).
    line("behaviour SetRally", "L-RLY"),
    line("rally", "L-RLY"),
    use_line("ground_rally: ApplyFirst", "L-RLY"),
    use_line("smart Ground with needs: Producer", "L-RLY"),
    // L-REP: `repairer` (R1), `Repair` (O4), `OwnDamaged`, `needs: Repairer` (O6).
    line("repairer", "L-REP"),
    line("behaviour Repair", "L-REP"),
    line("smart target OwnDamaged", "L-REP"),
    line("needs: Repairer", "L-REP"),
    // UI-HINT: `hints`, `hint_fallback` (H14), `start_selection` (H18), `Value::Hint`,
    // `Predicate::HintShown`.
    line("hints", "UI-HINT"),
    line("hint_fallback", "UI-HINT"),
    line("start_selection", "UI-HINT"),
    line("Value::Hint", "UI-HINT"),
    line("Predicate::HintShown", "UI-HINT"),
    // UI-ALERT: `alerts[].kind`, `clear_after` (H6), `units` (H5), `select_on_click`, `{unit}`
    // (H17), `focus` (H22), `sound_when` (H21), `alert_sound_gap_secs` (H20), the triggers
    // `ConstructionComplete`, `SupplyShort`, `SupplyRestored`, `InsufficientFunds`,
    // `GathererIdle` (H4).
    line("alerts.kind", "UI-ALERT"),
    line("alerts.clear_after", "UI-ALERT"),
    line("alerts.units", "UI-ALERT"),
    line("alerts.select_on_click", "UI-ALERT"),
    use_line("placeholder {unit}", "UI-ALERT"),
    line("alerts.focus", "UI-ALERT"),
    line("alerts.sound_when", "UI-ALERT"),
    line("alert_sound_gap_secs", "UI-ALERT"),
    line("AlertTrigger::ConstructionComplete", "UI-ALERT"),
    line("AlertTrigger::SupplyShort", "UI-ALERT"),
    line("AlertTrigger::SupplyRestored", "UI-ALERT"),
    line("AlertTrigger::InsufficientFunds", "UI-ALERT"),
    line("AlertTrigger::GathererIdle", "UI-ALERT"),
    // UI2-CONN: the triggers `SlotDisconnected`, `SlotReconnected` (H4), `Value::Connection`.
    line("AlertTrigger::SlotDisconnected", "UI2-CONN"),
    line("AlertTrigger::SlotReconnected", "UI2-CONN"),
    line("Value::Connection", "UI2-CONN"),
    // UI-PLACE: `world.placement`, `world.build_area_ring`, `world.gap_ring`,
    // `world.progress_bar` (H11), the cursor context `place` (H2), `Predicate::Placing`.
    line("world.placement", "UI-PLACE"),
    line("world.build_area_ring", "UI-PLACE"),
    line("world.gap_ring", "UI-PLACE"),
    line("world.progress_bar", "UI-PLACE"),
    line("cursors.place", "UI-PLACE"),
    line("Predicate::Placing", "UI-PLACE"),
    // UI-LABEL: `world.unit_labels` (H24, errata E10, E22).
    line("world.unit_labels", "UI-LABEL"),
    // UI-HUD: `role` (H19), `hotkey` (O8), `card` (O10), `Value::Supply`, `ConstructionQueue`,
    // `Sites`, `SelectionTypes`, `Notices`, `Predicate::SupplyShort`, `ItemHas`, the item fields
    // `Role`, `State`, `Paid`, `Refund`, `Badge`.
    line("role", "UI-HUD"),
    line("hotkey", "UI-HUD"),
    line("card", "UI-HUD"),
    line("Value::Supply", "UI-HUD"),
    line("Value::ConstructionQueue", "UI-HUD"),
    line("Value::Sites", "UI-HUD"),
    line("Value::SelectionTypes", "UI-HUD"),
    line("Value::Notices", "UI-HUD"),
    line("Source::ConstructionQueue", "UI-HUD"),
    line("Source::Sites", "UI-HUD"),
    line("Source::SelectionTypes", "UI-HUD"),
    line("Source::Notices", "UI-HUD"),
    line("Predicate::SupplyShort", "UI-HUD"),
    line("Predicate::ItemHas", "UI-HUD"),
    line("ItemField::Role", "UI-HUD"),
    line("ItemField::State", "UI-HUD"),
    line("ItemField::Paid", "UI-HUD"),
    line("ItemField::Refund", "UI-HUD"),
    line("ItemField::Badge", "UI-HUD"),
    // UI-RESULT: `Value::Outcome`, `OutcomeReason`, `MatchClock`, `Predicate::MatchOver`,
    // `OutcomeIs`, `OutcomeReasonIs`, `{slot}` (H17, T4), `leave_match` (H23).
    line("Value::Outcome", "UI-RESULT"),
    line("Value::OutcomeReason", "UI-RESULT"),
    line("Value::MatchClock", "UI-RESULT"),
    line("Predicate::MatchOver", "UI-RESULT"),
    line("Predicate::OutcomeIs", "UI-RESULT"),
    line("Predicate::OutcomeReasonIs", "UI-RESULT"),
    use_line("placeholder {slot}", "UI-RESULT"),
    line("engine action leave_match", "UI-RESULT"),
    // UI2-RESULT: `Value::MatchStats` (H7).
    line("Value::MatchStats", "UI2-RESULT"),
    // UI2-HUD: the item fields `BuildTimeNow`, `SupplyChange`, `CausesShortage`, the primary
    // fields `Activity`, `Cargo`, `Armour` (H8, H9), `PrimaryHasValue` (H10), `world.cargo_bar`,
    // `world.health_bar.scale_to_footprint` (H12), `world.rally` (H11), the cursor contexts
    // `repair`, `rally` (H2).
    line("ItemField::BuildTimeNow", "UI2-HUD"),
    line("ItemField::SupplyChange", "UI2-HUD"),
    line("ItemField::CausesShortage", "UI2-HUD"),
    line("PrimaryField::Activity", "UI2-HUD"),
    line("PrimaryField::Cargo", "UI2-HUD"),
    line("PrimaryField::Armour", "UI2-HUD"),
    line("Predicate::PrimaryHasValue", "UI2-HUD"),
    line("world.cargo_bar", "UI2-HUD"),
    line("world.health_bar.scale_to_footprint", "UI2-HUD"),
    line("world.rally", "UI2-HUD"),
    line("cursors.repair", "UI2-HUD"),
    line("cursors.rally", "UI2-HUD"),
    // L-FX-PRES: `exports.visual_effects` (A3), the visual `effects` with `fire` and `death`
    // (V3, V4), and `world.float_text` (H11). L3 of the review of `L0-API1`: no plan task reads the
    // `float_text` style. Plan 5.3 gives only its `sound` to `FX2`, and the block of `L-FX-PRES`
    // (plan "Task L-FX-PRES") renders the "+N" float (X14, R14) but names no style. So the line
    // goes to `L-FX-PRES`, the task that draws the float, until the orchestrator gives the style to
    // another task; the task that reads the style deletes the line.
    line("exports.visual_effects", "L-FX-PRES"),
    line("effects", "L-FX-PRES"),
    line("world.float_text", "L-FX-PRES"),
    // FX2: `hit`, `hit_reduced`, `damaged` (V4), the effect `sound` (V3), the `world.float_text`
    // sound (H11), `effects_voices`, `audible_range` (H16).
    line("effects.hit", "FX2"),
    line("effects.hit_reduced", "FX2"),
    line("effects.damaged", "FX2"),
    line("visual effect sound", "FX2"),
    line("world.float_text.sound", "FX2"),
    line("effects_voices", "FX2"),
    line("audible_range", "FX2"),
    // UI2-MC: `Button.font`, `Button.size` (H13).
    line("Button.font", "UI2-MC"),
    line("Button.size", "UI2-MC"),
    // PRES-MODELS: `models` (V1), `model` (V2).
    line("models", "PRES-MODELS"),
    line("model", "PRES-MODELS"),
];

/// The lines that run on the server path (`presentation` false) or on the client path (true): a
/// line is a client line when its detector reads a file that only the client reads.
pub(crate) fn lines_for(presentation: bool, table: &[VersionTableRow]) -> Vec<UnbuiltLine> {
    UNBUILT_FEATURES
        .iter()
        .filter(|line| {
            let detect = line.detect.as_ref().or_else(|| {
                table
                    .iter()
                    .find(|row| row.name == line.feature)
                    .map(|row| &row.detect)
            });
            detect.is_some_and(Detect::is_presentation) == presentation
        })
        .copied()
        .collect()
}

/// Refuse the first line whose feature the package uses. Call it after the version table, first
/// with `content = None` (the RAW lines need only the raw documents), then with the parsed value.
pub(crate) fn check_unbuilt(
    manifest: &Manifest,
    content: Option<&ParsedMcContent>,
    raw: &RawDocuments,
    lines: &[UnbuiltLine],
    table: &[VersionTableRow],
) -> Result<(), ModError> {
    if !is_edition_0_2_4(&manifest.engine_api) {
        return Ok(());
    }
    let uses = variant_uses(manifest, content);
    for line in lines {
        let detect = match &line.detect {
            Some(detect) => detect,
            None => match table.iter().find(|row| row.name == line.feature) {
                Some(row) => &row.detect,
                None => panic!(
                    "UNBUILT line `{}` has no detector and no row of the version table",
                    line.feature
                ),
            },
        };
        if let Some(found) = locate(detect, manifest, content, raw, &uses) {
            let file = if found.file.is_empty() {
                MANIFEST_PATH
            } else {
                found.file.as_str()
            };
            return Err(ModError::new(
                &manifest.id,
                &found.path,
                format!(
                    "`{}` is not built in this engine build; leave it out until an engine build supports it",
                    line.feature
                ),
            )
            .with_file(file));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{UNBUILT_FEATURES, check_unbuilt};
    use crate::raw_presence::RawDocuments;
    use crate::version_table::{Detect, VERSION_TABLE, support::raw_for};
    use std::collections::{BTreeMap, BTreeSet};

    /// Every line finds a detector: its own, or the row of the version table with its name.
    #[test]
    fn every_line_has_a_detector() {
        for line in UNBUILT_FEATURES {
            let own = line.detect.is_some();
            let row = VERSION_TABLE.iter().any(|row| row.name == line.feature);
            assert!(own || row, "`{}` has no detector", line.feature);
            assert!(!(own && row), "`{}` has two detectors", line.feature);
        }
        let names: BTreeSet<_> = UNBUILT_FEATURES.iter().map(|line| line.feature).collect();
        assert_eq!(
            names.len(),
            UNBUILT_FEATURES.len(),
            "a feature has one line"
        );
    }

    /// The lines of each task, counted against plan 5.3 (the reviewer counts them with
    /// `grep -c`). A task deletes its own lines and nothing else; a count that is not in this
    /// table is a line that no task owns.
    #[test]
    fn the_lines_of_each_task_are_the_lines_of_plan_5_3() {
        let want: BTreeMap<&str, usize> = BTreeMap::from([
            ("L-DMG", 3),
            ("L-TGT", 1),
            ("L-PWR", 6),
            ("L-PAY", 1),
            ("L-INC", 1),
            ("L-CON1", 7),
            ("L-DES", 4),
            ("L-STO", 2),
            ("L-MATCH-SRV1", 1),
            ("L-LOBBY2-SRV", 2),
            ("L-MATCH-SIM2", 2),
            ("L-CON3", 1),
            ("L-RLY", 4),
            ("L-REP", 4),
            ("UI-HINT", 5),
            ("UI-ALERT", 13),
            ("UI2-CONN", 3),
            ("UI-PLACE", 6),
            ("UI-LABEL", 1),
            ("UI-HUD", 19),
            ("UI-RESULT", 8),
            ("UI2-RESULT", 1),
            ("UI2-HUD", 12),
            ("L-FX-PRES", 3),
            ("FX2", 7),
            ("UI2-MC", 2),
            ("PRES-MODELS", 2),
        ]);
        let mut got: BTreeMap<&str, usize> = BTreeMap::new();
        for line in UNBUILT_FEATURES {
            *got.entry(line.owner).or_default() += 1;
        }
        assert_eq!(got, want);
        assert_eq!(UNBUILT_FEATURES.len(), 121);
    }

    /// Each row of 0.2.4 either has an `UNBUILT` line, or is in this list of the rows that the
    /// plan does not hold back (built in lane 0, or shadowed by another line). A new row
    /// has to be put in one of the two.
    #[test]
    fn the_rows_with_no_line_are_the_rows_that_lane_0_builds() {
        let with_line: BTreeSet<_> = UNBUILT_FEATURES.iter().map(|line| line.feature).collect();
        let without: BTreeSet<_> = VERSION_TABLE
            .iter()
            .filter(|row| row.since_api == "0.2.4" && !with_line.contains(row.name))
            .map(|row| row.name)
            .collect();
        let want = BTreeSet::from([
            // H3: the `states` texts are data, read by nobody yet.
            "states",
            // L1: `Ally` and `Enemy` are served by `turts_rules::relation` in this task.
            "SlotRelation::Ally",
            "SlotRelation::Enemy",
            // K1: the combination needs `constructor`, whose line comes first.
            "constructor beside producer",
        ]);
        assert_eq!(without, want);
    }

    /// Every RAW line refuses a `^0.2.4` package that writes its path, with the file, the path and
    /// the owner, and refuses nothing for the edition 0.2.3 (the version table speaks first).
    #[test]
    fn every_raw_line_refuses_with_its_file_path_and_owner() {
        let mut tested = 0;
        for line in UNBUILT_FEATURES {
            let Some(row) = VERSION_TABLE.iter().find(|row| row.name == line.feature) else {
                continue;
            };
            let Detect::Raw { path, group } = row.detect else {
                continue;
            };
            tested += 1;
            let (manifest, raw) = raw_for(row, "^0.2.4", true);
            let error = check_unbuilt(&manifest, None, &raw, UNBUILT_FEATURES, VERSION_TABLE)
                .expect_err(line.feature);
            let file = if group == crate::raw_presence::RawGroup::Manifest {
                "manifest.ron"
            } else {
                "f.ron"
            };
            // The first line that the document trips can be an earlier line with the same path
            // (`constructor` before `constructor.when_short`): the text names the line.
            let text = error.to_string();
            let first = text.split('`').nth(1).unwrap_or_else(|| panic!("{text}"));
            assert_eq!(
                text,
                format!(
                    "zeta/{file}: {}: `{first}` is not built in this engine build; leave it out until an engine build supports it",
                    if first == line.feature {
                        path.join(".")
                    } else {
                        // The path of the shadowing line: found in the table.
                        match VERSION_TABLE
                            .iter()
                            .find(|r| r.name == first)
                            .unwrap()
                            .detect
                        {
                            Detect::Raw { path, .. } => path.join("."),
                            _ => panic!("{text}"),
                        }
                    }
                ),
                "line {}",
                line.feature
            );
            // The edition 0.2.3 is not refused: the version table speaks first.
            let (manifest, raw) = raw_for(row, "^0.2.3", true);
            check_unbuilt(&manifest, None, &raw, UNBUILT_FEATURES, VERSION_TABLE)
                .unwrap_or_else(|error| panic!("{}: {error}", line.feature));
            // With the field absent there is no refusal.
            let (manifest, raw) = raw_for(row, "^0.2.4", false);
            check_unbuilt(&manifest, None, &raw, UNBUILT_FEATURES, VERSION_TABLE)
                .unwrap_or_else(|error| panic!("{}, absent: {error}", line.feature));
        }
        assert_eq!(tested, 56, "the RAW lines");
        let _ = RawDocuments::default();
    }

    /// The lines of the lines of other lanes stay when one lane deletes its own: removing the
    /// lines of one owner lets its features pass and keeps the others refused.
    #[test]
    fn a_lane_removes_only_its_own_lines() {
        let row = VERSION_TABLE
            .iter()
            .find(|row| row.name == "armour")
            .unwrap();
        let (manifest, raw) = raw_for(row, "^0.2.4", true);
        let without_dmg: Vec<_> = UNBUILT_FEATURES
            .iter()
            .filter(|line| line.owner != "L-DMG")
            .map(|line| super::UnbuiltLine {
                feature: line.feature,
                owner: line.owner,
                detect: None,
            })
            .filter(|line| VERSION_TABLE.iter().any(|row| row.name == line.feature))
            .collect();
        check_unbuilt(&manifest, None, &raw, &without_dmg, VERSION_TABLE)
            .expect("`armour` passes when L-DMG deleted its lines");
        assert!(check_unbuilt(&manifest, None, &raw, UNBUILT_FEATURES, VERSION_TABLE).is_err());
    }
}
