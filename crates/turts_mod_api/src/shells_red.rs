//! RED-PHASE SHELLS. Three empty top-level files and the key lists with no content, so that
//! the tests of task L0-API1 compile and fail on a parse error, not on a missing name. The
//! next commit replaces this file by the real types.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatDef {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MatchDef {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VisualEffectsDef {}
