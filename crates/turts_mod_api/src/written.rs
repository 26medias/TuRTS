//! A presence marker for a field inside an enum variant (design 6.10, "Version-table
//! detection", kind "field inside an enum variant").
//!
//! The version table must see that a package WRITES a new field, also with its default value
//! (`font: None`): an older engine refuses the file, because the variant struct has
//! `deny_unknown_fields`. The raw key walk cannot do this inside a variant (`ron::Value` drops
//! the variant name, so a field and a map key look the same), and the parsed value loses the
//! difference between a field that is absent and a field that is written with its default.
//! `Written<T>` keeps the difference in the typed value: use it as
//! `#[serde(default)] pub font: Written<Option<String>>`.
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

    #[test]
    fn the_flag_is_not_serialised() {
        let (font, _) = button(r#"Button(label: "x", font: Some("mono"))"#);
        assert_eq!(ron::to_string(&font).unwrap(), r#"Some("mono")"#);
    }
}
