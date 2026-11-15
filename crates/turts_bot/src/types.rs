//! Portable identity, work, accounting, and proposal records.
use serde::{Deserialize, Serialize};
use turts_mod_api::{ContentId, PaymentPolicy, WorldPos};

pub type UnitId = u32;
pub type NodeId = u32;
pub type BasisId = u64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct BotActor {
    pub slot: u8,
    pub profile: ContentId,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskKey {
    pub controller: String,
    pub goal: String,
    pub instance: String,
}
impl TaskKey {
    pub fn new(
        controller: impl Into<String>,
        goal: impl Into<String>,
        instance: impl Into<String>,
    ) -> Self {
        Self {
            controller: controller.into(),
            goal: goal.into(),
            instance: instance.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RequestKey {
    pub actor: BotActor,
    pub sequence: u32,
    pub unit: UnitId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Watermark {
    pub completed_tick: u32,
    pub revision: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum InventoryPort {
    Main,
    Output,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SourceEndpoint {
    SlotBalance(u8),
    Inventory { unit: UnitId, port: InventoryPort },
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SpendKey {
    pub endpoint: SourceEndpoint,
    pub resource: ContentId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountAmount {
    pub account: SpendKey,
    pub amount: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountObservation {
    pub account: SpendKey,
    pub on_hand: u64,
    pub reserved: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum WorkKind {
    Production,
    Construction,
    Research,
    Order,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WorkIdentity {
    pub unit: UnitId,
    pub id: u64,
    pub kind: WorkKind,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkState {
    Queued,
    Running,
    WaitingForFunds(ContentId),
    Paused,
    OutputBlocked,
    Finished,
    Failed,
}
impl WorkState {
    pub fn terminal(&self) -> bool {
        matches!(self, Self::Finished | Self::Failed)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkObservation {
    pub identity: WorkIdentity,
    /// Original actor, including a constructor removed while its autonomous site survives.
    pub actor_unit: UnitId,
    pub content: Option<ContentId>,
    pub payment: Option<PaymentPolicy>,
    pub done: u64,
    pub total: u64,
    pub paid: Vec<AccountAmount>,
    pub reserved: Vec<AccountAmount>,
    pub state: WorkState,
}

/// Exact effects of this request included in the published account basis.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedRequestEffect {
    pub request: RequestKey,
    pub effect_watermark: Watermark,
    pub refused: bool,
    /// An accepted held construction request exists, but its actual site/job is not placed yet.
    #[serde(default)]
    pub pending_placement: bool,
    /// Cumulative paid and current reservation rows, each at this same basis.
    pub paid: Vec<AccountAmount>,
    pub reserved: Vec<AccountAmount>,
    pub work: Option<WorkObservation>,
    pub terminal: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconcileBatch {
    pub watermark: Watermark,
    pub accounts: Vec<AccountObservation>,
    pub included: Vec<ObservedRequestEffect>,
    /// Explicit proof that these pending requests have no effects in this basis yet.
    #[serde(default)]
    pub unapplied: Vec<RequestKey>,
    /// False forbids both account advancement and new spending decisions.
    pub complete: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorResult {
    pub request: RequestKey,
    pub effect_watermark: Watermark,
    pub accepted: bool,
    pub work: Option<WorkIdentity>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BotTarget {
    None,
    Point(WorldPos),
    Unit(UnitId),
    Node(NodeId),
    Job(u64),
    Content(ContentId),
    ContentAtPoint { content: ContentId, point: WorldPos },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BotQueueMode {
    Replace,
    Append,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BotOrder {
    pub content: ContentId,
    pub target: BotTarget,
    pub queue: BotQueueMode,
    pub group: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proposal {
    /// The retained permitted candidate basis that produced this intention.
    pub source_basis: BasisId,
    pub task: TaskKey,
    pub actors: Vec<UnitId>,
    pub order: BotOrder,
    pub priority: u32,
    pub goal_order: u32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifiedMember {
    pub unit: UnitId,
    pub anchor: WorldPos,
    pub costs: Vec<AccountAmount>,
    pub payment: Option<PaymentPolicy>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerifiedProposal {
    pub members: Vec<VerifiedMember>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalRejection {
    Budget,
    UnknownActor,
    NotObserved,
    InvalidOrder,
    InvalidTask,
    InvalidCosts,
    InvalidAnchor,
    DuplicateActor,
    DuplicateTask,
    ConflictingAssignment,
    InsufficientResources(SpendKey),
    EqualRank,
    ArithmeticOverflow,
    AccountingUnresolved,
    SequenceExhausted,
}

/// A typed read-only verification port over the permitted immutable basis.
/// Implementations derive anchors/costs from facts, never from controller estimates.
pub trait ProposalVerifier {
    fn verify(
        &self,
        basis: BasisId,
        proposal: &Proposal,
    ) -> Result<VerifiedProposal, ProposalRejection>;
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmittedCommand {
    pub actor: BotActor,
    pub sequence: u32,
    pub task: TaskKey,
    pub units: Vec<UnitId>,
    pub order: BotOrder,
}
