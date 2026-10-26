//! Content ids and package ids.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Namespace of a package. Allowed characters: `a-z`, `0-9`, `_`; at least one.
// M1 placeholder; format open (spec 15)
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PackageId(pub String);

impl PackageId {
    /// Check the id format and build the id.
    pub fn parse(s: &str) -> Result<Self, String> {
        if s.is_empty() {
            return Err("package id is empty".to_string());
        }
        if let Some(bad) = s
            .chars()
            .find(|c| !matches!(c, 'a'..='z' | '0'..='9' | '_'))
        {
            return Err(format!(
                "package id `{s}` has the character `{bad}`; allowed characters are a-z, 0-9 and _"
            ));
        }
        Ok(Self(s.to_string()))
    }
}

/// Stable content identifier: `<package>:<local>`.
// M1 placeholder; format open (spec 15)
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContentId {
    pub package: PackageId,
    pub local: String,
}

impl ContentId {
    /// Build an id from parts. This call does not check the parts.
    pub fn new(package: &PackageId, local: &str) -> Self {
        Self {
            package: package.clone(),
            local: local.to_string(),
        }
    }

    /// Split `<package>:<local>`. The text has exactly one `:`. Both parts are not empty.
    pub fn parse(s: &str) -> Result<Self, String> {
        let mut parts = s.split(':');
        let (package, local) = match (parts.next(), parts.next(), parts.next()) {
            (Some(package), Some(local), None) => (package, local),
            _ => return Err(format!("content id `{s}` must have exactly one `:`")),
        };
        if package.is_empty() || local.is_empty() {
            return Err(format!("content id `{s}` has an empty part"));
        }
        Ok(Self {
            package: PackageId(package.to_string()),
            local: local.to_string(),
        })
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.package.0, self.local)
    }
}

/// Dense runtime handle for a registered unit type. The simulation assigns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct UnitTypeId(pub u32);
