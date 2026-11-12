//! A presence marker for a field inside an enum variant (design 6.10, "Version-table
//! detection", kind "field inside an enum variant").
//!
//! The version table must see that a package WRITES a new field, also with its default value
//! (`font: None`): an older engine refuses the file, because the variant struct has
//! `deny_unknown_fields`. The raw key walk cannot do this inside a variant (`ron::Value` drops
//! the variant name, so a field and a map key look the same), and the parsed value loses the
//! difference between a field that is absent and a field that is written with its default.
//! `Written<T>` keeps the difference in the typed value: use it as
//! `#[serde(default, skip_serializing_if = "Written::is_unwritten")] pub font:
//! Written<Option<String>>`. The `skip_serializing_if` is part of the contract: a value
//! that was not written is left out on the way back, so the written text parses to the same
//! value (a flag cannot be part of the serialised value). With it, the flag survives a write
//! and a read: absent stays absent, `font: None` stays `font: None`.
//!
//! **Equality.** The derived `PartialEq` compares the value AND the flag. A test that
//! compares two parsed files then sees a key that one of them wrote and the other did not.
//! `same_value` compares the value only.
//!
//! A field that is NOT in a variant uses a RAW row (`turts_mod_runtime`, `raw_presence`), not
//! this type. Do not use `Written` with `#[serde(flatten)]`, `#[serde(untagged)]` or
//! `#[serde(skip_deserializing)]`: the data is then buffered, or the field is not read, and
//! the marker is not reliable.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A value and a flag: did the source file write the key?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Written<T> {
    value: T,
    written: bool,
}

impl<T> Written<T> {
    /// The value, or the default of `T` when the key is absent.
    pub fn value(&self) -> &T {
        &self.value
    }

    /// `true` when the source file wrote the key, also with the default value.
    pub fn is_written(&self) -> bool {
        self.written
    }

    /// The value, without the flag.
    pub fn into_inner(self) -> T {
        self.value
    }

    /// `true` when the source file did not write the key. Use it as the `skip_serializing_if`
    /// of the field.
    pub fn is_unwritten(&self) -> bool {
        !self.written
    }

    /// A value as if the source file wrote the key (a flag of `true`).
    pub fn from_source(value: T) -> Self {
        Self {
            value,
            written: true,
        }
    }

    /// Equality of the two values, with no regard to the flag.
    pub fn same_value(&self, other: &Self) -> bool
    where
        T: PartialEq,
    {
        self.value == other.value
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Written<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(|value| Self {
            value,
            written: true,
        })
    }
}

/// The flag is not written back: the value serialises as `T` does.
impl<T: Serialize> Serialize for Written<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::Written;
    use serde::{Deserialize, Serialize};

    /// A scratch enum like the widget `Primitive`: a struct variant with `deny_unknown_fields`.
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    enum Scratch {
        Button {
            label: String,
            #[serde(default)]
            font: Written<Option<String>>,
            #[serde(default)]
            size: Written<Option<u16>>,
        },
        Text {
            font: Option<String>,
        },
    }

    fn button(text: &str) -> (Written<Option<String>>, Written<Option<u16>>) {
        match ron::from_str::<Scratch>(text).unwrap() {
            Scratch::Button { font, size, .. } => (font, size),
            other => panic!("not a button: {other:?}"),
        }
    }

    #[test]
    fn an_absent_key_is_not_written() {
        let (font, size) = button(r#"Button(label: "x")"#);
        assert!(!font.is_written());
        assert!(!size.is_written());
        assert_eq!(font.value(), &None);
    }

    /// The hole that the type closes: the parsed value of `font: None` equals the parsed value of
    /// an absent `font`, but the key is written.
    #[test]
    fn a_key_written_with_its_default_is_written() {
        let (font, size) = button(r#"Button(label: "x", font: None)"#);
        assert!(font.is_written());
        assert_eq!(font.value(), &None);
        assert!(!size.is_written(), "only the key that is written");
    }

    #[test]
    fn a_key_written_with_a_value_is_written_and_keeps_the_value() {
        let (font, size) = button(r#"Button(label: "x", font: Some("mono"), size: Some(12))"#);
        assert!(font.is_written());
        assert_eq!(font.value().as_deref(), Some("mono"));
        assert!(size.is_written());
        assert_eq!(size.into_inner(), Some(12));
    }

    /// The marker changes no other variant: a `Text` that writes `font` is a normal field.
    #[test]
    fn the_marker_does_not_touch_other_variants() {
        let text: Scratch = ron::from_str("Text(font: None)").unwrap();
        assert_eq!(text, Scratch::Text { font: None });
    }

    /// The scratch enum with the `skip_serializing_if` of the contract.
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    enum Kept {
        Button {
            label: String,
            #[serde(default, skip_serializing_if = "Written::is_unwritten")]
            font: Written<Option<String>>,
            #[serde(default, skip_serializing_if = "Written::is_unwritten")]
            size: Written<Option<u16>>,
        },
    }

    /// L0-RAW re-check: the flag survives a write and a read, in each of its three states.
    #[test]
    fn the_flag_survives_a_round_trip() {
        for text in [
            r#"Button(label: "x")"#,
            r#"Button(label: "x", font: None)"#,
            r#"Button(label: "x", size: None, font: Some("mono"))"#,
            r#"Button(label: "x", font: None, size: None)"#,
        ] {
            let first: Kept = ron::from_str(text).unwrap();
            let written = ron::to_string(&first).unwrap();
            let again: Kept = ron::from_str(&written).unwrap();
            assert_eq!(first, again, "{text} -> {written}");
            // The written text names exactly the keys that the source named.
            let Kept::Button { font, size, .. } = &again;
            assert_eq!(
                font.is_written(),
                text.contains("font:"),
                "{text} -> {written}"
            );
            assert_eq!(
                size.is_written(),
                text.contains("size:"),
                "{text} -> {written}"
            );
        }
    }

    /// An unwritten value is left out; the old derive would write `font: None` and the read
    /// would then set the flag.
    #[test]
    fn an_unwritten_value_is_left_out_of_the_text() {
        let button: Kept = ron::from_str(r#"Button(label: "x")"#).unwrap();
        assert_eq!(ron::to_string(&button).unwrap(), r#"Button(label:"x")"#);
        let button: Kept = ron::from_str(r#"Button(label: "x", font: None)"#).unwrap();
        assert_eq!(
            ron::to_string(&button).unwrap(),
            r#"Button(label:"x",font:None)"#
        );
    }

    /// The decision on equality: the flag is part of it, `same_value` ignores it.
    #[test]
    fn equality_compares_the_flag_and_same_value_ignores_it() {
        let absent: Written<Option<u16>> = Written::default();
        let none = Written::from_source(None);
        assert!(absent.is_unwritten() && !none.is_unwritten());
        assert_ne!(absent, none, "the key that was written is a difference");
        assert!(absent.same_value(&none));
    }

    #[test]
    fn the_flag_is_not_serialised() {
        let (font, _) = button(r#"Button(label: "x", font: Some("mono"))"#);
        assert_eq!(ron::to_string(&font).unwrap(), r#"Some("mono")"#);
    }
}
