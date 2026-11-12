//! Presence of a field in the source file (design 6.10, "Version-table detection", kind RAW).
//!
//! A typed value loses the difference between a field that the file does not write and a field
//! that the file writes with its default (`orders: []`, `armour: None`). The version table
//! needs that difference: a package that writes a new field uses the new API, also with an
//! empty value. This module parses a file a second time as `ron::Value` and walks its keys.
//!
//! What `ron::Value` keeps and what it drops (ron 0.12.2, the workspace version):
//! - It keeps the keys of a struct. A key with an empty value (`[]`, `None`) stays.
//! - It drops the name of an enum variant, and a struct field and a map key have the same
//!   `Map` form. So a key walk cannot say whether a key is a field of a variant or a key of a
//!   map. A RAW row therefore names a path from the root of one file. A field inside an enum
//!   variant is detected on the parsed value (`Detect::Parsed` in `version_table.rs`).

use ron::Value;
use turts_mod_api::{Manifest, ModError};

use crate::archive::MANIFEST_PATH;
use crate::parse::parse_ron;

/// The files in which a RAW row looks for its path: the manifest, or the files of one export
/// group of the manifest. A later task adds a RAW row for a group (for example `Units`) and
/// uses the variant then.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code, reason = "the RAW rows of API 0.2.4 use the other groups")]
pub(crate) enum RawGroup {
    Manifest,
    Units,
    Maps,
    Visuals,
    Resources,
    Nodes,
    Production,
    Recipes,
    Technologies,
    Orders,
    Interface,
    Input,
    Assets,
    /// The files of `exports.combat` (API 0.2.4).
    Combat,
    /// The files of `exports.match` (API 0.2.4).
    Match,
    /// The files of `exports.visual_effects` (API 0.2.4).
    VisualEffects,
}

impl RawGroup {
    /// True for the groups whose files only the client reads: the visuals and the visual effects.
    /// The server path (`PackageArchive::content`) never reads them (the server does not read
    /// presentation files, M1 verify), so their rows run on the client path
    /// (`PackageArchive::visuals`).
    pub(crate) fn is_presentation(self) -> bool {
        matches!(self, Self::Visuals | Self::VisualEffects)
    }

    /// The archive paths of the files of this group.
    fn files(self, manifest: &Manifest) -> Vec<&str> {
        let exports = &manifest.exports;
        let list = match self {
            Self::Manifest => return vec![MANIFEST_PATH],
            Self::Units => &exports.units,
            Self::Maps => &exports.maps,
            Self::Visuals => &exports.visuals,
            Self::Resources => &exports.resources,
            Self::Nodes => &exports.nodes,
            Self::Production => &exports.production,
            Self::Recipes => &exports.recipes,
            Self::Technologies => &exports.technologies,
            Self::Orders => &exports.orders,
            Self::Interface => &exports.interface,
            Self::Input => &exports.input,
            Self::Assets => &exports.assets,
            Self::Combat => &exports.combat,
            Self::Match => &exports.match_files,
            Self::VisualEffects => &exports.visual_effects,
        };
        list.iter().map(String::as_str).collect()
    }
}

/// The raw documents that the RAW rows of the version table read.
#[derive(Debug, Default)]
pub(crate) struct RawDocuments {
    documents: Vec<(RawGroup, String, Value)>,
}

impl RawDocuments {
    /// Parse every file of each group in `groups` once. `file` gives the bytes of an archive
    /// path. A file that is missing or is not UTF-8 text is skipped: the typed read of the same
    /// file reports it with its own error. A file that is not RON is an error.
    pub(crate) fn read<'a>(
        manifest: &Manifest,
        groups: impl IntoIterator<Item = RawGroup>,
        file: impl Fn(&str) -> Option<&'a [u8]>,
    ) -> Result<Self, ModError> {
        let mut documents = Vec::new();
        let mut done = Vec::new();
        for group in groups {
            if done.contains(&group) {
                continue;
            }
            done.push(group);
            for path in group.files(manifest) {
                let Some(text) = file(path).and_then(|bytes| std::str::from_utf8(bytes).ok())
                else {
                    continue;
                };
                let value: Value = parse_ron(&manifest.id, path, text)?;
                documents.push((group, path.to_string(), value));
            }
        }
        Ok(Self { documents })
    }

    /// The first file of `group` in which `path`, a list of field names from the root of the
    /// file, is written. `None` when no file of the group writes it.
    pub(crate) fn written(&self, group: RawGroup, path: &[&str]) -> Option<&str> {
        self.documents
            .iter()
            .find(|(g, _, value)| *g == group && has_path(value, path))
            .map(|(_, file, _)| file.as_str())
    }
}

/// The name in a path that stands for any key of a map (`units.*.role`: the `role` of each
/// entry of the `units` map of an interface file).
pub(crate) const ANY_KEY: &str = "*";

/// Walk `path` from the root of `value`. A struct or a map is a `Map` of string keys; an
/// `Option` that wraps a struct is looked through; a list is looked through to its elements
/// (`placements[].facing` is the path `["placements", "facing"]`): the rest of the path must hold
/// in at least one element. A list look-through is one step, not a deep walk: the next name is
/// always a key of the value that the path has reached. The name `*` matches any key of a map
/// (a map from ids to structs). The last name must be a key: its value can be empty (`[]`,
/// `None`, `()`).
fn has_path(value: &Value, path: &[&str]) -> bool {
    let Some((first, rest)) = path.split_first() else {
        return true;
    };
    match value {
        Value::Option(Some(inner)) => has_path(inner, path),
        Value::Seq(items) => items.iter().any(|item| has_path(item, path)),
        Value::Map(map) => map.iter().any(|(key, child)| {
            if *first == ANY_KEY {
                return has_path(child, rest);
            }
            matches!(key, Value::String(name) if name == first) && has_path(child, rest)
        }),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{RawDocuments, RawGroup, has_path};
    use ron::Value;
    use turts_mod_api::Manifest;

    fn parse(text: &str) -> Value {
        ron::from_str(text).unwrap()
    }

    #[test]
    fn a_key_with_an_empty_value_is_written() {
        let value = parse("(id: \"x\", orders: [], armour: None, exports: (orders: []))");
        for path in [&["orders"][..], &["armour"], &["exports", "orders"]] {
            assert!(has_path(&value, path), "{path:?}");
        }
    }

    #[test]
    fn an_absent_key_is_not_written() {
        let value = parse("(id: \"x\", exports: (units: []))");
        assert!(!has_path(&value, &["orders"]));
        assert!(!has_path(&value, &["exports", "orders"]));
    }

    /// The path starts at the root: the same name deeper down is another field.
    #[test]
    fn the_path_starts_at_the_root_of_the_file() {
        let value = parse("(exports: (orders: []))");
        assert!(!has_path(&value, &["orders"]));
        assert!(has_path(&value, &["exports", "orders"]));
    }

    /// The probe of the gate: a `Text` widget writes `font` and `size`, and `ron::Value` shows
    /// them as map keys. A path from the root does not see them; this is why a field inside an
    /// enum variant is not a RAW row.
    #[test]
    fn a_field_inside_a_variant_is_not_a_root_path() {
        let value = parse("(root: [(widget: Text(font: None, size: 12))])");
        assert!(!has_path(&value, &["font"]));
        assert!(!has_path(&value, &["size"]));
        assert!(has_path(&value, &["root"]));
    }

    #[test]
    fn a_key_through_an_option_is_found() {
        let value = parse("(weapon: Some((damage: 1)))");
        assert!(has_path(&value, &["weapon", "damage"]));
        assert!(!has_path(&value, &["weapon", "range"]));
    }

    #[test]
    fn an_empty_path_is_always_written_and_a_string_value_has_no_keys() {
        assert!(has_path(&parse("()"), &[]));
        assert!(!has_path(&parse("\"orders\""), &["orders"]));
        assert!(!has_path(&parse("[\"orders\"]"), &["orders"]));
    }

    /// A path looks through a list: `placements[].facing`, `alerts[].units`.
    #[test]
    fn a_path_looks_through_a_list_of_structs() {
        let value = parse(
            "(placements: [(unit: \"a\"), (unit: \"b\", facing: None)], alerts: [(id: \"x\", units: [])])",
        );
        assert!(has_path(&value, &["placements", "facing"]));
        assert!(has_path(&value, &["alerts", "units"]));
        assert!(!has_path(&value, &["placements", "owner_slot"]));
        assert!(!has_path(
            &parse("(placements: [])"),
            &["placements", "facing"]
        ));
    }

    #[test]
    fn a_path_looks_through_a_list_inside_an_option() {
        let value = parse("(maybe: Some([(a: 1), (b: 2)]), plain: Some((list: [(c: ())])))");
        assert!(has_path(&value, &["maybe", "b"]));
        assert!(has_path(&value, &["plain", "list", "c"]));
        assert!(!has_path(&value, &["maybe", "c"]));
    }

    /// A list look-through is not a deep walk: the next name must be a key of the element
    /// itself, and a key that is deeper down in the element does not count.
    #[test]
    fn a_list_look_through_is_not_a_deep_walk() {
        let value = parse("(items: [(inner: (facing: None))], other: [[(facing: None)]])");
        assert!(!has_path(&value, &["items", "facing"]));
        assert!(has_path(&value, &["items", "inner", "facing"]));
        assert!(!has_path(&value, &["facing"]));
        // A list of lists is looked through at each level.
        assert!(has_path(&value, &["other", "facing"]));
    }

    /// `*` matches any key of a map: the `role` of each entry of `units`.
    #[test]
    fn a_wildcard_looks_through_the_values_of_a_map() {
        let value = parse(r#"(units: {"a:b": (role: None), "a:c": (icon: None)}, role: None)"#);
        assert!(has_path(&value, &["units", "*", "role"]));
        assert!(!has_path(&value, &["units", "*", "portrait"]));
        // The wildcard is one step, and it needs a map: a list is looked through as before.
        assert!(!has_path(&parse("(units: {})"), &["units", "*", "role"]));
        assert!(!has_path(&parse(r#"(units: "x")"#), &["units", "*"]));
    }

    /// A list of strings or numbers holds no keys.
    #[test]
    fn a_list_of_plain_values_holds_no_key() {
        assert!(!has_path(
            &parse("(names: [\"facing\", \"belts\"])"),
            &["names", "facing"]
        ));
    }

    fn manifest() -> Manifest {
        ron::from_str(
            r#"(id: "zeta", version: "0.1.0", kind: Game, engine_api: "^0.2.3", dependencies: {}, exports: (units: ["units/a.ron", "units/b.ron"], maps: [], visuals: [], resources: [], nodes: [], production: []), context: Shared, capabilities: [], license: "test")"#,
        )
        .unwrap()
    }

    #[test]
    fn a_path_is_looked_up_in_every_file_of_the_group_only() {
        let a: &[u8] = b"(id: \"a\")";
        let b: &[u8] = b"(id: \"b\", armour: None)";
        let files = |path: &str| match path {
            "units/a.ron" => Some(a),
            "units/b.ron" => Some(b),
            _ => None,
        };
        let documents = RawDocuments::read(&manifest(), [RawGroup::Units], files).unwrap();
        assert_eq!(
            documents.written(RawGroup::Units, &["armour"]),
            Some("units/b.ron")
        );
        assert_eq!(documents.written(RawGroup::Units, &["acquire"]), None);
        // A group that nobody asked to read holds no document.
        assert_eq!(documents.written(RawGroup::Maps, &["armour"]), None);
    }

    #[test]
    fn a_file_that_is_not_ron_is_an_error_with_its_file_name() {
        let bad: &[u8] = b"(id: ";
        let files = |path: &str| (path == "units/a.ron").then_some(bad);
        let error = RawDocuments::read(&manifest(), [RawGroup::Units], files).unwrap_err();
        assert!(
            error.to_string().starts_with("zeta/units/a.ron:"),
            "{error}"
        );
    }
}
