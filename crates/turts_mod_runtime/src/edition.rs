//! The edition of a package (design 6.10, "Version-table detection", kind ED).
//!
//! The edition of the root package is the lowest released API that its `engine_api`
//! requirement admits. An ED row selects a behaviour for the edition 0.2.4 and refuses
//! nothing: a package with `engine_api: "^0.2.3"` has the edition 0.2.3 and keeps the 0.2.3
//! behaviour; a package with `"^0.2.4"` has the edition 0.2.4.

use semver::{Version, VersionReq};
use turts_mod_api::RELEASED_ENGINE_APIS;

/// The edition rows T1 to T4 of design 6.10, as names. `L0-API2` (T2, T3, T4: the validation of
/// each) and `L-OBS` (T1: the approach point) read the edition through `edition` and use
/// the name in their messages.
#[allow(
    dead_code,
    reason = "the validation of L0-API2 and L-OBS read these names"
)]
pub(crate) const EDITION_ROWS: [(&str, &str); 4] = [
    (
        "T1",
        "the approach point of a receiver or a node (design 6.9)",
    ),
    ("T2", "unknown `reasons`, `cursors` and `states` keys"),
    ("T3", "weapon damage 0"),
    (
        "T4",
        "`{unit}` and `{slot}` in an alert text, and `{slot}` in a `Format` text",
    ),
];

/// The first edition that has the behaviours of API 0.2.4.
pub(crate) fn first_0_2_4() -> Version {
    Version::new(0, 2, 4)
}

/// The lowest released API that `engine_api` admits. `None` when the requirement does not
/// parse or admits no released API (the manifest check reports both).
pub(crate) fn edition(engine_api: &str) -> Option<Version> {
    let requirement = VersionReq::parse(engine_api).ok()?;
    RELEASED_ENGINE_APIS
        .iter()
        .filter_map(|version| Version::parse(version).ok())
        .find(|version| requirement.matches(version))
}

/// True when the package has the edition 0.2.4 or a later one.
pub(crate) fn is_edition_0_2_4(engine_api: &str) -> bool {
    edition(engine_api).is_some_and(|edition| edition >= first_0_2_4())
}

#[cfg(test)]
mod tests {
    use super::{EDITION_ROWS, edition, is_edition_0_2_4};
    use semver::Version;

    #[test]
    fn the_edition_is_the_lowest_released_api_that_the_requirement_admits() {
        assert_eq!(edition("^0.2.3"), Some(Version::new(0, 2, 3)));
        assert_eq!(edition("^0.2.4"), Some(Version::new(0, 2, 4)));
        assert_eq!(edition("^0.2"), Some(Version::new(0, 2, 0)));
        assert_eq!(edition(">=0.2.3, <0.2.4"), Some(Version::new(0, 2, 3)));
        assert_eq!(edition("^0.3"), None, "no released API is admitted");
        assert_eq!(edition("not a requirement"), None);
    }

    #[test]
    fn a_requirement_that_admits_0_2_3_is_not_the_edition_0_2_4() {
        assert!(!is_edition_0_2_4("^0.2.3"));
        assert!(!is_edition_0_2_4("^0.2"));
        assert!(is_edition_0_2_4("^0.2.4"));
        assert!(is_edition_0_2_4(">=0.2.4"));
    }

    #[test]
    fn the_four_edition_rows_are_named() {
        let ids: Vec<_> = EDITION_ROWS.iter().map(|(id, _)| *id).collect();
        assert_eq!(ids, ["T1", "T2", "T3", "T4"]);
    }
}
