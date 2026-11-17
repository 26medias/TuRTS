//! Protocol 11 owner-only underground utility view (M6).
//!
//! A viewer receives only its own slot's junctions, segments, networks and port units.
//! No record carries a surface or cover value, a foreign row, a foreign count or a foreign
//! id. The view rides the existing atomic state transaction next to terrain rows.
use serde::{Deserialize, Serialize};

use crate::{ContentRef, Pos, UnitId};

/// One end of an own segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UtilityEndRecord {
    /// An own junction id (counted per slot).
    Node(u32),
    /// The port of an own completed unit.
    Port(UnitId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UtilitySegmentState {
    Conducting,
    /// Exposed under policy `Operate`: still conducts.
    Exposed,
    /// Exposed under policy `Disconnect`: does not conduct.
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtilityNodeRecord {
    pub id: u32,
    pub utility: ContentRef,
    pub pos: Pos,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtilitySegmentRecord {
    pub id: u32,
    pub utility: ContentRef,
    pub a: UtilityEndRecord,
    pub b: UtilityEndRecord,
    /// Fixed elevation in terrain elevation steps above the datum.
    pub elevation: i32,
    /// The depth that was requested at placement.
    pub depth: u32,
    pub state: UtilitySegmentState,
}

/// One connected network of one utility type. A port unit with no conducting segment is a
/// network of its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtilityNetworkRecord {
    pub utility: ContentRef,
    /// Own completed port units in this network, ascending.
    pub units: Vec<UnitId>,
    /// Own junctions in this network, ascending.
    pub nodes: Vec<u32>,
    pub provided: u64,
    pub consumed: u64,
    pub short: bool,
}

/// The complete owner view. In a delta, `Some` replaces the whole view.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UtilityView {
    pub nodes: Vec<UtilityNodeRecord>,
    pub segments: Vec<UtilitySegmentRecord>,
    pub networks: Vec<UtilityNetworkRecord>,
}
