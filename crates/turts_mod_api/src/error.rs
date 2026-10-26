//! Structured ModError.

use std::fmt;

use serde::{Deserialize, Serialize};

/// A package or registration error that names where the problem is.
/// No Rust type name appears in `message`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModError {
    pub package: String,
    /// The file in the archive. `register_unit_type` leaves it `None`; the loader fills it.
    pub file: Option<String>,
    pub field_path: String,
    pub message: String,
    /// The values the field accepts, when the set is small.
    pub allowed: Option<Vec<String>>,
}

impl ModError {
    pub fn new(package: &str, field_path: &str, message: impl Into<String>) -> Self {
        Self {
            package: package.to_string(),
            file: None,
            field_path: field_path.to_string(),
            message: message.into(),
            allowed: None,
        }
    }

    pub fn with_file(mut self, file: &str) -> Self {
        self.file = Some(file.to_string());
        self
    }

    pub fn with_allowed(mut self, allowed: Vec<String>) -> Self {
        self.allowed = Some(allowed);
        self
    }
}

/// The text is `<package>/<file>: <field path>: <message>`, then ` (allowed: a, b)` when the
/// error lists the accepted names. The package and its slash are left out when the package is
/// empty (the manifest is parsed before the package id is known). The field path is left out
/// when it is the file itself (a parse error), so the file appears once.
impl fmt::Display for ModError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file = self.file.as_deref().unwrap_or("-");
        if self.package.is_empty() {
            write!(f, "{file}")?;
        } else {
            write!(f, "{}/{file}", self.package)?;
        }
        if self.file.as_deref() != Some(self.field_path.as_str()) {
            write!(f, ": {}", self.field_path)?;
        }
        write!(f, ": {}", self.message)?;
        if let Some(allowed) = &self.allowed {
            write!(f, " (allowed: {})", allowed.join(", "))?;
        }
        Ok(())
    }
}

impl std::error::Error for ModError {}

#[cfg(test)]
mod tests {
    use super::ModError;

    #[test]
    fn display_names_package_file_and_field() {
        let e = ModError::new("pkg", "speed", "must be greater than 0");
        assert_eq!(e.to_string(), "pkg/-: speed: must be greater than 0");
        let e = e
            .with_file("units/one.ron")
            .with_allowed(vec!["Circle".to_string()]);
        assert_eq!(
            e.to_string(),
            "pkg/units/one.ron: speed: must be greater than 0 (allowed: Circle)"
        );
        assert_eq!(e.allowed, Some(vec!["Circle".to_string()]));
    }

    #[test]
    fn display_prints_the_allowed_names() {
        let e = ModError::new("pkg", "units/one.ron", "unknown variant `Square`")
            .with_file("units/one.ron")
            .with_allowed(vec!["Circle".to_string(), "Ring".to_string()]);
        assert_eq!(
            e.to_string(),
            "pkg/units/one.ron: unknown variant `Square` (allowed: Circle, Ring)"
        );
    }

    #[test]
    fn display_prints_the_file_once() {
        // A parse error has the file as its field path. The file is not printed twice.
        let e = ModError::new("pkg", "units/one.ron", "unknown field `sped`")
            .with_file("units/one.ron");
        assert_eq!(e.to_string(), "pkg/units/one.ron: unknown field `sped`");
        // A different field path stays in the text.
        let e = ModError::new("pkg", "speed", "too small").with_file("units/one.ron");
        assert_eq!(e.to_string(), "pkg/units/one.ron: speed: too small");
    }

    #[test]
    fn display_leaves_out_an_empty_package() {
        // The manifest is parsed before the package id is known.
        let e = ModError::new("", "manifest.ron", "unknown field `hash`").with_file("manifest.ron");
        assert_eq!(e.to_string(), "manifest.ron: unknown field `hash`");
        let e = ModError::new("", "lockfile", "bad json");
        assert_eq!(e.to_string(), "-: lockfile: bad json");
    }
}
