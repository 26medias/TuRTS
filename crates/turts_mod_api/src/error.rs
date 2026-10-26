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

impl fmt::Display for ModError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{}: {}: {}",
            self.package,
            self.file.as_deref().unwrap_or("-"),
            self.field_path,
            self.message
        )
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
            "pkg/units/one.ron: speed: must be greater than 0"
        );
        assert_eq!(e.allowed, Some(vec!["Circle".to_string()]));
    }
}
