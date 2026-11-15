//! Bounded queries expose permitted facts and identical metadata for equivalent permissions.
use crate::{budget::BudgetUsage, types::*};
use serde::{Deserialize, Serialize};
use turts_mod_api::{Capability, ContentId, SlotRelation, WorldPos};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitObservation {
    pub unit: UnitId,
    pub owner_slot: u8,
    pub content: ContentId,
    pub position: WorldPos,
    pub radius: f32,
    pub capabilities: Vec<Capability>,
    pub health: Option<(u64, u64)>,
    pub site: bool,
    /// Present only for this bot's own units. Enemy orders/enable state are private.
    pub own: Option<OwnUnitObservation>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OwnActivity {
    Idle,
    Pending,
    Moving,
    Attacking,
    Holding,
    Gathering,
    Working,
    Blocked,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OwnUnitObservation {
    pub enabled: bool,
    pub activity: OwnActivity,
    pub current_order: Option<BotOrder>,
    pub queued_orders: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeObservation {
    pub node: NodeId,
    pub content: ContentId,
    /// Public item definition produced by this node, not private geology.
    pub item: ContentId,
    pub position: WorldPos,
    pub radius: f32,
    pub remaining: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitQuery {
    pub relation: SlotRelation,
    pub content: Vec<ContentId>,
    pub center: Option<WorldPos>,
    pub radius: Option<f32>,
}
impl UnitQuery {
    pub fn own(content: Vec<ContentId>) -> Self {
        Self {
            relation: SlotRelation::Same,
            content,
            center: None,
            radius: None,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeQuery {
    pub resources: Vec<ContentId>,
    pub center: Option<WorldPos>,
    pub radius: Option<f32>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlacementQuery {
    pub actor: UnitId,
    pub production: ContentId,
    pub point: WorldPos,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AffordabilityQuery {
    pub actor: UnitId,
    pub production: ContentId,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Query {
    Units(UnitQuery),
    Nodes(NodeQuery),
    LookupUnit(UnitId),
    Accounts(Vec<SpendKey>),
    Works,
    RememberedUnits(UnitQuery),
    RememberedNodes(NodeQuery),
    Placement(PlacementQuery),
    Affordability(AffordabilityQuery),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryCursor {
    pub basis: BasisId,
    pub token: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageState {
    Complete,
    More(QueryCursor),
    UnresolvedTie(QueryCursor),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Coverage {
    Complete,
    NotObserved,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Advice<T> {
    Known(T),
    NotObserved,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlacementAdvice {
    Allowed,
    Blocked,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostQuote {
    pub costs: Vec<AccountAmount>,
    pub affordable: bool,
}
// Keep the single lookup row inline. Boxing would allocate for each lookup and
// change the public query API solely to reduce this bounded enum's stack size.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryData {
    Units(Vec<UnitObservation>),
    Nodes(Vec<NodeObservation>),
    Lookup(Advice<UnitObservation>),
    Accounts(Vec<AccountObservation>),
    Works(Vec<WorkObservation>),
    RememberedUnits(RememberedRows<turts_mod_api::RememberedUnit>),
    RememberedNodes(RememberedRows<turts_mod_api::RememberedNode>),
    Placement(Advice<PlacementAdvice>),
    Affordability(Advice<CostQuote>),
}
/// Stale permitted facts. These are never live handles or a current-health query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RememberedRows<T> {
    pub rows: Vec<T>,
    pub retention_truncated: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryPage {
    pub basis: BasisId,
    pub completed_tick: u32,
    pub data: QueryData,
    pub state: PageState,
    /// Complete enumeration of permitted rows; placement/hidden lookup can instead be NotObserved.
    /// Complete never asserts that an unobserved region contains no other objects.
    pub coverage: Coverage,
    pub usage: BudgetUsage,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryError {
    Budget,
    ExpiredBasis,
    InvalidContinuation,
    InvalidQuery,
    NotObserved,
}
/// Every examined count and cursor describes permitted candidates only.
/// Private projection scans are accounted by the host, outside public metadata.
pub trait BotQueries {
    /// Retain this candidate basis across query calls and scheduled steps for this controller.
    fn pin_basis(&mut self, basis: BasisId) -> Result<(), QueryError>;
    /// Release only when the complete selection is consumed or explicitly abandoned.
    fn release_basis(&mut self, basis: BasisId) -> Result<(), QueryError>;
    fn query(
        &mut self,
        query: Query,
        continuation: Option<QueryCursor>,
        limit: u32,
    ) -> Result<QueryPage, QueryError>;
}
