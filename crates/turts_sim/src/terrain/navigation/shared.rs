//! Shared reverse fields. Actor and field handles are lookup identities only.
#[path = "waypoint.rs"]
mod waypoint;
use super::super::traffic::{DestinationRegion, SCRATCH_BYTES as TRAFFIC_BYTES, TrafficState};
use super::geometry::{
    Certificate, GEOMETRY_BUDGET, Planning, TerrainRead, bits, certify, certify_with_refusal, point,
};
use super::storage::{Frontier, Index, PAGE_BYTES, QueueRow, Sparse, Vertex, index_bytes, pages};
use crate::movement::{MovementFrame, MovementFrameUnit};
use bevy_ecs::{prelude::Resource, world::World};
use bevy_math::Vec2;
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    mem::size_of,
};
use turts_rules::terrain::geometry::Traversal;
const VIEWERS: usize = turts_mod_api::SLOT_COUNT as usize;
const MAX_FIELDS: usize = 32 / VIEWERS;
const MAX_MEMBERS: usize = 4096 / VIEWERS;
const MAX_BYTES: usize = (64 * 1024 * 1024 - TRAFFIC_BYTES - 1024) / VIEWERS;
// Includes ranking buffers, exact-predicate scratch, and small index rehash overlap.
const SCRATCH_BYTES: usize = 8 * 1024 * 1024;
const _: () =
    assert!(4096 * (size_of::<(u32, u32, f32, Traversal)>() + size_of::<u32>()) <= SCRATCH_BYTES);
const MAX_ENTRIES: usize = 4096 / VIEWERS;
const MAX_EXPANSIONS: usize = 128 / VIEWERS;
const MAX_EDGES: usize = 1024 / VIEWERS;
const MAX_RANK_COMPARISONS: usize = 262_144 / VIEWERS;
const MAX_RETIRED_PAGES: usize = 16 / VIEWERS;
const REGION_HALF_WIDTH: u32 = 16;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NavigationDiagnostics {
    pub searches: u64,
    pub cache_hits: u64,
    /// Exact direct-query results reused after local lease validation.
    pub direct_query_cache_hits: u64,
    /// Direct queries that actually invoke the geometric certificate.
    pub direct_query_cache_misses: u64,
    /// Fresh direct queries put off to a later tick by the per-tick work limit.
    pub direct_query_deferrals: u64,
    /// Geometry operations of the fresh direct queries of this tick.
    pub direct_query_work_this_tick: usize,
    pub invalidations: u64,
    pub solver_retries: u64,
    pub budget_waits: u64,
    pub resident_fields: usize,
    pub resident_bytes: usize,
    pub retired_bytes: usize,
    pub retired_pages: usize,
    pub memberships: usize,
    pub pending: usize,
    pub expansions_this_tick: usize,
    pub edge_checks_this_tick: usize,
    pub admission_entries_this_tick: usize,
    pub invalidation_entries_this_tick: usize,
    pub retired_pages_this_tick: usize,
    pub ranking_comparisons_this_tick: usize,
    pub frame_rows_this_tick: usize,
    pub body_index_queries_this_tick: u64,
    pub body_index_rank_comparisons_this_tick: u64,
    pub body_index_tail_entries_this_tick: u64,
    pub body_index_word_ops_this_tick: u64,
    pub body_index_build_comparisons_this_tick: u64,
    pub body_index_build_prefix_words_this_tick: u64,
    pub body_index_build_mask_words_this_tick: u64,
    pub body_envelope_filters_this_tick: u64,
    pub body_index_rebuilds_this_tick: u64,
    pub body_index_updates_this_tick: u64,
    pub body_index_update_entries_this_tick: u64,
    pub body_support_queries_this_tick: u64,
    /// Start-of-tick overlap scans of clipped rows that carry an escape.
    pub escape_keep_scans_this_tick: u64,
    /// Packed stop (operator decision 2026-11-22).
    pub packed_stop_visits_this_tick: u64,
    pub packed_stops: u64,
    pub packed_steps: u64,
    pub body_support_pairs_this_tick: u64,
    pub body_support_failures_this_tick: u64,
    pub body_support_accepted_members_this_tick: u64,
    /// Complete permitted neighbor discoveries admitted to the velocity model.
    pub model_attempts_this_tick: u64,
    /// Model work debited from the existing shared physical pair allowance.
    pub model_operations_this_tick: u64,
    pub model_fallbacks_this_tick: u64,
    pub model_view_refusals_this_tick: u64,
    /// Per-actor alignment attempts; aggregate contributions do not count here.
    pub coherence_attempts_this_tick: u64,
    /// Completed validated alignment proposals, before final physical commit.
    pub coherence_advice_this_tick: u64,
    pub coherence_members_this_tick: u64,
    pub coherence_operations_this_tick: u64,
    pub body_pair_checks: u64,
    pub body_pair_checks_this_tick: usize,
    pub neighbor_visits: u64,
    pub neighbor_overflows: u64,
    pub neighbor_visits_this_tick: usize,
    pub neighbor_overflows_this_tick: usize,
    pub neighbor_bucket_visits: u64,
    pub neighbor_bucket_visits_this_tick: usize,
    pub traffic_index_queries: u64,
    pub traffic_index_queries_this_tick: u64,
    pub traffic_index_rank_comparisons: u64,
    pub traffic_index_rank_comparisons_this_tick: u64,
    pub traffic_index_tail_entries: u64,
    pub traffic_index_tail_entries_this_tick: u64,
    pub traffic_index_word_ops: u64,
    pub traffic_index_word_ops_this_tick: u64,
    pub traffic_index_build_comparisons: u64,
    pub traffic_index_build_comparisons_this_tick: u64,
    pub traffic_index_build_prefix_words: u64,
    pub traffic_index_build_prefix_words_this_tick: u64,
    pub traffic_index_build_mask_words: u64,
    pub traffic_index_build_mask_words_this_tick: u64,
    pub traffic_index_query_rank_bound: u64,
    pub traffic_index_query_tail_bound: u64,
    pub traffic_index_query_word_bound: u64,
    pub local_detour_filter_rows_this_tick: usize,
    pub local_detour_scanned_rows_this_tick: usize,
    pub local_detour_generator_comparisons_this_tick: usize,
    pub local_detour_candidates_this_tick: usize,
    pub local_detour_body_checks_this_tick: usize,
    pub local_detour_terrain_calls_this_tick: usize,
    pub traffic_members: usize,
    pub traffic_gates: usize,
    pub traffic_capacity_waits: u64,
    pub max_pending_age_ticks: u64,
    pub stale_activations: u64,
    /// Blocked-recovery accounting (design 2026-11-18-bounded-recovery). Work values are
    /// the larger owner total of all solver passes in the tick; bytes are counted, not heap.
    pub recovery_work_this_tick: u64,
    pub recovery_actual_work_this_tick: u64,
    pub recovery_candidates_this_tick: u64,
    pub recovery_visibility_checks_this_tick: u64,
    pub recovery_digests_this_tick: u64,
    pub recovery_invalidations_this_tick: u64,
    pub recovery_relaxed_commits_this_tick: u64,
    pub recovery_pool_work_this_tick: u64,
    pub recovery_spare_slot_work_this_tick: u64,
    /// Recovery work that no owner allowance pays (design 2026-11-19-recovery-actual-work
    /// section 7): the neighbourhood reads of the base grid, their cells and bodies, and
    /// the owner view builds. Sums of all solver passes in the tick.
    pub recovery_physical_reads_this_tick: u64,
    pub recovery_physical_work_this_tick: u64,
    pub recovery_view_work_this_tick: u64,
    pub recovery_jobs: u64,
    pub recovery_max_age: u64,
    pub recovery_retained_bytes: u64,
    pub recovery_phase_bytes: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavigationMemberState {
    Searching,
    Ready,
    Blocked,
    Pending,
    Direct,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavigationDestinationRegionAudit {
    pub goal: [f32; 2],
    pub primary_min: [f32; 2],
    pub primary_max: [f32; 2],
    pub cohort_radius: f32,
    pub actor_radius: f32,
    pub epoch: u64,
    pub inline_bytes: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct NavigationMemberAudit {
    pub unit: u32,
    pub viewer: u8,
    pub order_epoch: u64,
    pub field: Option<u64>,
    pub field_revision: u64,
    pub endpoint: [f32; 2],
    pub destination_region: Option<NavigationDestinationRegionAudit>,
    pub waypoint: Option<[f32; 2]>,
    pub waypoint_group: Option<crate::movement::MoveProgress>,
    pub waypoint_plain: Option<crate::movement::PlainMoveProgress>,
    pub dependencies: Vec<(usize, u64)>,
    pub state: NavigationMemberState,
}
#[derive(Clone, Debug, PartialEq)]
pub struct NavigationAudit {
    pub members: Vec<NavigationMemberAudit>,
    pub diagnostics: NavigationDiagnostics,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Base {
    viewer: u8,
    radius: u32,
    profile: Traversal,
    rx: u32,
    ry: u32,
}
#[derive(Clone, Copy, Default)]
struct Node {
    generation: u32,
    component: bool,
    component_cost: u64,
    component_parent: Vertex,
    component_parent_set: bool,
    cost: u64,
    parent: Vertex,
    parent_set: bool,
    closed: bool,
}
#[derive(Clone, Copy, Default)]
struct LeaseRow {
    generation: u32,
    revision: u64,
}
#[derive(Default)]
struct Leases {
    data: Sparse<LeaseRow>,
    keys: Frontier,
    generation: u32,
}
impl Leases {
    fn begin(&mut self) -> bool {
        let Some(next) = self.generation.checked_add(1) else {
            return false;
        };
        self.generation = next;
        self.keys.clear();
        true
    }
    fn bytes(&self) -> usize {
        self.data.bytes() + self.keys.bytes()
    }
    fn allocated_pages(&self) -> usize {
        self.data.allocated_pages() + self.keys.allocated_pages()
    }
    fn merge(&mut self, rows: &BTreeMap<usize, u64>, limit: usize) -> bool {
        for (&key, &revision) in rows {
            let key = key as u64;
            let exists = self
                .data
                .get(key)
                .is_some_and(|row| row.generation == self.generation);
            if exists {
                if self
                    .data
                    .get(key)
                    .is_none_or(|row| row.revision != revision)
                {
                    return false;
                }
                continue;
            }
            if !self
                .data
                .ensure(key, limit.saturating_sub(self.keys.bytes()))
                || !self.keys.push(
                    QueueRow {
                        cost: key,
                        vertex: Vertex::default(),
                    },
                    limit.saturating_sub(self.data.bytes()),
                )
            {
                return false;
            }
            if let Some(row) = self.data.get_mut(key) {
                *row = LeaseRow {
                    generation: self.generation,
                    revision,
                };
            }
        }
        true
    }
    fn at(&self, index: usize) -> Option<(usize, u64)> {
        let key = self.keys.at(index)?.cost;
        let row = self.data.get(key)?;
        (row.generation == self.generation).then_some((key as usize, row.revision))
    }
    fn rows(&self) -> Vec<(usize, u64)> {
        (0..self.keys.len())
            .filter_map(|index| self.at(index))
            .collect()
    }
    fn retire(&mut self, remaining: &mut usize) {
        self.data.retire(remaining);
        self.keys.retire(remaining);
    }
}
#[derive(Clone, Copy)]
struct Expansion {
    row: QueueRow,
    direction: u8,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Component,
    Reverse,
    Pending,
    Exhausted,
    Blocked,
}
struct Field {
    id: u64,
    base: Base,
    seed: Vertex,
    component_min: Vertex,
    origin: Vec2,
    accepted_tick: u32,
    revision: u64,
    generation: u32,
    refs: usize,
    stage: Stage,
    nodes: Sparse<Node>,
    frontier: Frontier,
    leases: Leases,
    expanding: Option<Expansion>,
    valid_epoch: u64,
    validation_epoch: u64,
    validation_cursor: usize,
    resource_fault: bool,
    seed_checked: bool,
    last_service: u32,
    last_reachable: u32,
    demand: bool,
    unresolved_edges: bool,
}
fn width(terrain: &impl TerrainRead) -> f64 {
    f64::from(terrain.grid().cell_width) / f64::from(terrain.grid().length_scale)
}
fn vertex_point(terrain: &impl TerrainRead, vertex: Vertex) -> Vec2 {
    let scale = width(terrain) * 0.5;
    Vec2::new(
        (f64::from(vertex.x) * scale) as f32,
        (f64::from(vertex.y) * scale) as f32,
    )
}
fn vertex_index(terrain: &impl TerrainRead, vertex: Vertex) -> Option<u64> {
    let columns = u64::from(terrain.grid().columns)
        .checked_mul(2)?
        .checked_add(1)?;
    if u64::from(vertex.x) >= columns || u64::from(vertex.y) > u64::from(terrain.grid().rows) * 2 {
        return None;
    }
    u64::from(vertex.y)
        .checked_mul(columns)?
        .checked_add(u64::from(vertex.x))
}
fn neighbor(vertex: Vertex, direction: u8) -> Option<Vertex> {
    const STEPS: [(i64, i64); 8] = [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];
    let (x, y) = STEPS[usize::from(direction)];
    Some(Vertex {
        x: u32::try_from(i64::from(vertex.x) + x).ok()?,
        y: u32::try_from(i64::from(vertex.y) + y).ok()?,
    })
}
impl Field {
    fn bytes(&self) -> usize {
        size_of::<Self>() + self.nodes.bytes() + self.frontier.bytes() + self.leases.bytes()
    }
    fn allocated_pages(&self) -> usize {
        1 + self.nodes.allocated_pages()
            + self.frontier.allocated_pages()
            + self.leases.allocated_pages()
    }
    fn node(&self, terrain: &impl TerrainRead, vertex: Vertex) -> Option<&Node> {
        let row = self.nodes.get(vertex_index(terrain, vertex)?)?;
        (row.generation == self.generation).then_some(row)
    }
    fn within_terminal(&self, vertex: Vertex, terrain: &impl TerrainRead) -> bool {
        vertex.x >= self.base.rx
            && vertex.y >= self.base.ry
            && vertex.x
                <= self
                    .base
                    .rx
                    .saturating_add(REGION_HALF_WIDTH)
                    .min(terrain.grid().columns.saturating_mul(2))
            && vertex.y
                <= self
                    .base
                    .ry
                    .saturating_add(REGION_HALF_WIDTH)
                    .min(terrain.grid().rows.saturating_mul(2))
    }
    fn fresh(id: u64, base: Base, seed: Vertex, origin: Vec2, accepted_tick: u32) -> Self {
        Self {
            id,
            base,
            seed,
            component_min: seed,
            origin,
            accepted_tick,
            revision: 1,
            generation: 0,
            refs: 0,
            stage: Stage::Component,
            nodes: Sparse::default(),
            frontier: Frontier::default(),
            leases: Leases::default(),
            expanding: None,
            valid_epoch: 0,
            validation_epoch: 0,
            validation_cursor: 0,
            resource_fault: false,
            seed_checked: false,
            last_service: 0,
            last_reachable: accepted_tick,
            demand: true,
            unresolved_edges: false,
        }
    }
    fn reset(&mut self, terrain: &impl TerrainRead, limit: usize) -> bool {
        let Some(generation) = self.generation.checked_add(1) else {
            return false;
        };
        self.generation = generation;
        self.frontier.clear();
        self.leases.begin();
        self.expanding = None;
        self.stage = Stage::Component;
        self.component_min = self.seed;
        self.resource_fault = false;
        self.unresolved_edges = false;
        self.seed_checked = false;
        let Some(index) = vertex_index(terrain, self.seed) else {
            self.stage = Stage::Blocked;
            return true;
        };
        let allowance =
            limit.saturating_sub(size_of::<Self>() + self.frontier.bytes() + self.leases.bytes());
        if !self.nodes.ensure(index, allowance) {
            self.resource_fault = true;
            return false;
        }
        if let Some(row) = self.nodes.get_mut(index) {
            *row = Node {
                generation,
                component: true,
                cost: u64::MAX,
                ..Default::default()
            };
        }
        let available =
            limit.saturating_sub(size_of::<Self>() + self.nodes.bytes() + self.leases.bytes());
        if !self.frontier.push(
            QueueRow {
                cost: 0,
                vertex: self.seed,
            },
            available,
        ) {
            self.resource_fault = true;
            return false;
        }
        self.valid_epoch = terrain.epoch();
        self.validation_epoch = terrain.epoch();
        self.validation_cursor = 0;
        true
    }
    fn validate(
        &mut self,
        terrain: &impl TerrainRead,
        diagnostics: &mut NavigationDiagnostics,
        limit: usize,
    ) -> bool {
        if self.resource_fault {
            return self.reset(terrain, limit);
        }
        if self.valid_epoch == terrain.epoch() {
            return true;
        }
        if self.validation_epoch != terrain.epoch() {
            self.validation_epoch = terrain.epoch();
            self.validation_cursor = 0;
        }
        while self.validation_cursor < self.leases.keys.len()
            && diagnostics.invalidation_entries_this_tick < MAX_ENTRIES
        {
            diagnostics.invalidation_entries_this_tick += 1;
            let Some((key, revision)) = self.leases.at(self.validation_cursor) else {
                self.resource_fault = true;
                return false;
            };
            self.validation_cursor += 1;
            if !terrain.lease_matches(key, revision) {
                diagnostics.invalidations = diagnostics.invalidations.saturating_add(1);
                let Some(revision) = self.revision.checked_add(1) else {
                    self.resource_fault = true;
                    return false;
                };
                self.revision = revision;
                return self.reset(terrain, limit);
            }
        }
        if self.validation_cursor == self.leases.keys.len() {
            self.valid_epoch = terrain.epoch();
        }
        self.valid_epoch == terrain.epoch() && !self.resource_fault
    }
    fn query(
        &mut self,
        terrain: &impl TerrainRead,
        from: Vertex,
        to: Vertex,
        diagnostics: &mut NavigationDiagnostics,
        limit: usize,
    ) -> Option<Certificate> {
        if diagnostics.edge_checks_this_tick >= MAX_EDGES {
            return None;
        }
        diagnostics.edge_checks_this_tick += 1;
        let mut lease = BTreeMap::new();
        let result = certify(
            terrain,
            vertex_point(terrain, from),
            vertex_point(terrain, to),
            f32::from_bits(self.base.radius),
            self.base.profile,
            &mut lease,
        );
        let available =
            limit.saturating_sub(size_of::<Self>() + self.nodes.bytes() + self.frontier.bytes());
        if !self.leases.merge(&lease, available) {
            self.resource_fault = true;
            return None;
        }
        Some(result)
    }
    /// One resumable edge at a time. Even stale heap pops consume expansion budget.
    fn advance(
        &mut self,
        terrain: &impl TerrainRead,
        diagnostics: &mut NavigationDiagnostics,
        limit: usize,
    ) -> bool {
        if self.resource_fault
            || self.valid_epoch != terrain.epoch()
            || matches!(
                self.stage,
                Stage::Blocked | Stage::Exhausted | Stage::Pending
            )
        {
            return false;
        }
        if !self.seed_checked {
            match self.query(terrain, self.seed, self.seed, diagnostics, limit) {
                Some(Certificate::Clear) => self.seed_checked = true,
                Some(Certificate::Blocked) => {
                    self.stage = Stage::Blocked;
                    return true;
                }
                _ => return false,
            }
        }
        if self.expanding.is_none() {
            if diagnostics.expansions_this_tick >= MAX_EXPANSIONS {
                return false;
            }
            let Some(row) = self.frontier.pop() else {
                if self.stage == Stage::Component {
                    self.stage = Stage::Reverse;
                    let Some(index) = vertex_index(terrain, self.seed) else {
                        return false;
                    };
                    if let Some(node) = self.nodes.get_mut(index) {
                        node.cost = 0;
                        node.closed = false;
                        node.parent_set = false;
                    }
                    let available = limit.saturating_sub(
                        size_of::<Self>() + self.nodes.bytes() + self.leases.bytes(),
                    );
                    if !self.frontier.push(
                        QueueRow {
                            cost: 0,
                            vertex: self.seed,
                        },
                        available,
                    ) {
                        self.resource_fault = true;
                    }
                } else {
                    self.stage = if self.unresolved_edges {
                        Stage::Pending
                    } else {
                        Stage::Exhausted
                    };
                }
                return true;
            };
            diagnostics.expansions_this_tick += 1;
            let Some(node) = self.node(terrain, row.vertex) else {
                return true;
            };
            if self.stage == Stage::Reverse && (node.closed || node.cost != row.cost) {
                return true;
            }
            if self.stage == Stage::Component && node.component_cost != row.cost {
                return true;
            }
            if self.stage == Stage::Reverse
                && let Some(index) = vertex_index(terrain, row.vertex)
                && let Some(node) = self.nodes.get_mut(index)
            {
                node.closed = true;
            }
            self.expanding = Some(Expansion { row, direction: 0 });
        }
        let Some(current) = self.expanding else {
            return false;
        };
        if current.direction == 8 {
            self.expanding = None;
            return true;
        }
        let needed = if self.stage == Stage::Component { 2 } else { 1 };
        if diagnostics.edge_checks_this_tick + needed > MAX_EDGES {
            return false;
        }
        let next = neighbor(current.row.vertex, current.direction);
        if let Some(expanding) = self.expanding.as_mut() {
            expanding.direction += 1;
        }
        let Some(next) = next.filter(|vertex| vertex_index(terrain, *vertex).is_some()) else {
            return true;
        };
        if self.stage == Stage::Component && !self.within_terminal(next, terrain) {
            return true;
        }
        let cost = current.row.cost.saturating_add(
            if next.x != current.row.vertex.x && next.y != current.row.vertex.y {
                14
            } else {
                10
            },
        );
        if self.node(terrain, next).is_some_and(|node| {
            if self.stage == Stage::Component {
                node.component && node.component_cost <= cost
            } else {
                node.cost <= cost
            }
        }) {
            return true;
        }
        let Some(first) = self.query(terrain, next, current.row.vertex, diagnostics, limit) else {
            return false;
        };
        if first == Certificate::Pending {
            // Retrying the identical fixed-budget query forever must not starve
            // other edges. This edge grants no movement and remains unresolved
            // for this geometry epoch; it never proves the graph exhausted.
            self.unresolved_edges = true;
            return true;
        }
        if first != Certificate::Clear {
            return true;
        }
        if self.stage == Stage::Component {
            let Some(second) = self.query(terrain, current.row.vertex, next, diagnostics, limit)
            else {
                return false;
            };
            if second != Certificate::Clear {
                if second == Certificate::Pending {
                    self.unresolved_edges = true;
                }
                return true;
            }
        }
        let Some(index) = vertex_index(terrain, next) else {
            return true;
        };
        let available =
            limit.saturating_sub(size_of::<Self>() + self.frontier.bytes() + self.leases.bytes());
        if !self.nodes.ensure(index, available) {
            self.resource_fault = true;
            return false;
        }
        let mut node = self.node(terrain, next).copied().unwrap_or(Node {
            generation: self.generation,
            cost: u64::MAX,
            ..Default::default()
        });
        if self.stage == Stage::Component {
            node.component = true;
            node.component_cost = cost;
            node.component_parent = current.row.vertex;
            node.component_parent_set = true;
            self.component_min = self.component_min.min(next);
        } else {
            node.cost = cost;
            node.parent = current.row.vertex;
            node.parent_set = true;
            node.closed = false;
        }
        if let Some(slot) = self.nodes.get_mut(index) {
            *slot = node;
        }
        let available =
            limit.saturating_sub(size_of::<Self>() + self.nodes.bytes() + self.leases.bytes());
        if !self
            .frontier
            .push(QueueRow { cost, vertex: next }, available)
        {
            self.resource_fault = true;
            return false;
        }
        true
    }
    fn retire(&mut self, remaining: &mut usize) {
        self.nodes.retire(remaining);
        self.frontier.retire(remaining);
        self.leases.retire(remaining);
    }
}

/// One immutable geometric query. The containing member supplies the match
/// lifetime; all remaining numeric/authority inputs are captured without rounding.
#[derive(Clone, Copy, PartialEq, Eq)]
struct DirectQueryKey {
    from: [u32; 2],
    to: [u32; 2],
    radius: u32,
    profile: Traversal,
    viewer: u8,
    order_epoch: u64,
}
#[derive(Clone, Copy)]
struct DirectQueryMemo {
    key: DirectQueryKey,
    result: Certificate,
}
/// Geometry operations that the fresh direct queries of the members of one order
/// with no clear direct line can start in one tick. The limit is in operations, never in time, so it is deterministic.
/// The last admitted query can finish above it by one certificate at most.
const DIRECT_QUERY_WORK: usize = 4 * GEOMETRY_BUDGET;
/// Orders with their own work account in one tick. Further orders share the last row.
const DIRECT_ORDER_ROWS: usize = 8;
/// The limited direct-query work of one order in one tick.
#[derive(Clone, Copy, Default)]
struct DirectOrderAccount {
    used: bool,
    order: u64,
    work: usize,
    /// One or more members of the order were put off in this tick.
    put_off: bool,
}
/// One admitted geometry refusal of the direct line. It holds until the unit
/// leaves the cell of the refused start, or any other input changes. A refused
/// long segment is not swept again on each tick from a slightly different pose.
#[derive(Clone, Copy, PartialEq, Eq)]
struct DirectRefusalMemo {
    from_cell: [u32; 2],
    to: [u32; 2],
    radius: u32,
    profile: Traversal,
    viewer: u8,
    order_epoch: u64,
    terrain_epoch: u64,
}
// Inline storage is charged by index_bytes/member admission, with no new heap.
const _: () = assert!(size_of::<DirectQueryMemo>() + size_of::<DirectRefusalMemo>() <= 128);
#[derive(Clone, Copy)]
struct ParkingScope {
    region: DestinationRegion,
    epoch: u64,
    radius: u32,
    kind: u32,
    profile: Traversal,
}
impl ParkingScope {
    fn matches(self, unit: &MovementFrameUnit, profile: Traversal, epoch: u64) -> bool {
        self.epoch == epoch
            && self.radius == unit.radius.to_bits()
            && self.kind == unit.unit_type
            && self.profile == profile
    }
}
struct Member {
    motion: Option<waypoint::WaypointMotion>,
    parking_scope: Option<ParkingScope>,
    viewer: u8,
    endpoint: Vec2,
    arrival_slot: Option<Vec2>,
    held_leg: HeldLeg,
    radius: u32,
    profile: Traversal,
    epoch: u64,
    accepted: u32,
    serviced: u32,
    seen: u32,
    live_target: bool,
    slot: usize,
    cancelled: bool,
    owner_fence: bool,
    owner_reported: bool,
    owner_released: bool,
    field: Option<u64>,
    field_revision: u64,
    seed: Option<Vertex>,
    connector: Option<Vec2>,
    connector_cursor: usize,
    direct: Vec<(usize, u64)>,
    direct_query: Option<DirectQueryMemo>,
    direct_refusal: Option<DirectRefusalMemo>,
    /// The last tick in which this member ran a limited fresh direct query.
    direct_turn: Option<u32>,
    contact: Vec<(usize, u64)>,
    contact_epoch: u64,
    contact_validation_epoch: u64,
    contact_cursor: usize,
    contact_invalidated: bool,
    valid_epoch: u64,
    validation_epoch: u64,
    validation_cursor: usize,
    state: NavigationMemberState,
}
/// Clock of consecutive held ticks on the direct final leg of one accepted
/// order. Only the terrain layer writes it, and only for a direct row. A tick
/// that is not held on the final leg clears it, so it cannot start expired.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct HeldLeg {
    ticks: u16,
    best: f32,
}
impl HeldLeg {
    /// `held` is the distance to the order endpoint on a held final-leg tick.
    /// Returns true when the body made no approach of a quarter radius for
    /// `limit` consecutive held ticks.
    fn observe(&mut self, held: Option<f32>, radius: f32, limit: u16) -> bool {
        let Some(distance) = held.filter(|d| d.is_finite()) else {
            *self = Self::default();
            return false;
        };
        if self.ticks == 0 || distance < self.best - 0.25 * radius {
            self.best = distance;
            self.ticks = 1;
        } else {
            self.ticks = self.ticks.saturating_add(1);
        }
        if self.ticks >= limit.max(1) {
            *self = Self::default();
            return true;
        }
        false
    }
}
impl Member {
    fn stamp(viewer: u8, accepted: u32, epoch: u64, slot: usize) -> Self {
        Self {
            motion: None,
            parking_scope: None,
            viewer,
            endpoint: Vec2::ZERO,
            arrival_slot: None,
            held_leg: HeldLeg::default(),
            radius: 0,
            profile: Traversal {
                max_step: 0,
                max_grade: turts_rules::terrain::Ratio {
                    numerator: 0,
                    denominator: 1,
                },
            },
            epoch,
            accepted,
            serviced: 0,
            seen: accepted,
            live_target: false,
            slot,
            cancelled: true,
            owner_fence: false,
            owner_reported: false,
            owner_released: false,
            field: None,
            field_revision: 1,
            seed: None,
            connector: None,
            connector_cursor: 0,
            direct: Vec::new(),
            direct_query: None,
            direct_refusal: None,
            direct_turn: None,
            contact: Vec::new(),
            contact_epoch: 0,
            contact_validation_epoch: 0,
            contact_cursor: 0,
            contact_invalidated: false,
            valid_epoch: 0,
            validation_epoch: 0,
            validation_cursor: 0,
            state: NavigationMemberState::Pending,
        }
    }
    fn bytes(&self) -> usize {
        (self.direct.capacity() + self.contact.capacity()) * size_of::<(usize, u64)>()
            + self
                .motion
                .as_ref()
                .map_or(0, waypoint::WaypointMotion::bytes)
    }
}
/// All handles and generations are local to this viewer within the match resource. They never order service.
#[derive(Default)]
#[allow(
    clippy::vec_box,
    reason = "Fields retire independently under a bounded cleanup budget."
)]
struct ViewerNavigation {
    viewer: u8,
    members: Index<u32, Member>,
    member_ids: Vec<u32>,
    fields: Vec<Box<Field>>,
    retired: Vec<Box<Field>>,
    retired_members: Vec<Vec<(usize, u64)>>,
    next_epoch: u64,
    next_field: u64,
    tick: Option<u32>,
    cleanup_cursor: usize,
    owner_changes: Vec<(u32, u8)>,
    pub(super) diagnostics: NavigationDiagnostics,
    small_retired_bytes: usize,
    /// Work of the limited fresh direct queries of this tick, for each order.
    /// The last row collects every further order. Inline: no heap.
    direct_order_work: [DirectOrderAccount; DIRECT_ORDER_ROWS],
    /// The accounts of the tick before: who was put off decides who yields now.
    direct_order_before: [DirectOrderAccount; DIRECT_ORDER_ROWS],
    member_buffer_bytes: usize,
    cancellation_overflow: bool,
}
impl ViewerNavigation {
    fn resident_bytes(&self) -> usize {
        SCRATCH_BYTES
            + size_of::<Self>()
            + index_bytes(&self.members)
            + self.member_ids.capacity() * size_of::<u32>()
            + (self.fields.capacity() + self.retired.capacity()) * size_of::<Box<Field>>()
            + self.member_buffer_bytes
            + self.fields.iter().map(|field| field.bytes()).sum::<usize>()
            + self.owner_changes.capacity() * size_of::<(u32, u8)>()
            // Reserve the bounded delayed-buffer directory before any cache can grow.
            + 2*MAX_MEMBERS*size_of::<Vec<(usize,u64)>>()
    }
    fn retired_bytes(&self) -> usize {
        self.retired
            .iter()
            .map(|field| field.bytes())
            .sum::<usize>()
            + self
                .retired_members
                .iter()
                .map(|buffer| buffer.capacity() * size_of::<(usize, u64)>())
                .sum::<usize>()
    }
    fn available(&self) -> usize {
        MAX_BYTES.saturating_sub(self.resident_bytes() + self.retired_bytes())
    }
    fn retiring(&self) -> bool {
        !self.retired.is_empty() || !self.retired_members.is_empty()
    }
    fn field_limit(&self, index: usize) -> usize {
        self.fields[index].bytes() + if self.retiring() { 0 } else { self.available() }
    }
    fn retire_member_buffer(&mut self, buffer: Vec<(usize, u64)>) {
        let bytes = buffer.capacity() * size_of::<(usize, u64)>();
        self.member_buffer_bytes = self.member_buffer_bytes.saturating_sub(bytes);
        let remaining =
            MAX_RETIRED_PAGES.saturating_sub(self.diagnostics.retired_pages_this_tick) * PAGE_BYTES;
        if self.small_retired_bytes.saturating_add(bytes) <= remaining {
            self.small_retired_bytes += bytes;
            drop(buffer);
        } else if bytes > 0 {
            self.retired_members.push(buffer);
        }
    }
    fn begin(&mut self, tick: u32) {
        if self.tick == Some(tick) {
            return;
        }
        let consecutive = self
            .tick
            .is_some_and(|last| last.checked_add(1) == Some(tick));
        self.tick = Some(tick);
        self.small_retired_bytes = 0;
        // Accounts of a tick that is not the tick before give no turn order.
        self.direct_order_before = if consecutive {
            self.direct_order_work
        } else {
            Default::default()
        };
        self.direct_order_work = Default::default();
        self.diagnostics.expansions_this_tick = 0;
        self.diagnostics.edge_checks_this_tick = 0;
        self.diagnostics.direct_query_work_this_tick = 0;
        self.diagnostics.admission_entries_this_tick = 0;
        self.diagnostics.invalidation_entries_this_tick = 0;
        self.diagnostics.retired_pages_this_tick = 0;
        self.diagnostics.ranking_comparisons_this_tick = 0;
        self.diagnostics.frame_rows_this_tick = 0;
        self.diagnostics.neighbor_visits_this_tick = 0;
        self.diagnostics.neighbor_overflows_this_tick = 0;
        // At most 32 active fields can be detached before retirement blocks new fields.
        let mut pages_left = MAX_RETIRED_PAGES;
        for field in &mut self.retired {
            field.retire(&mut pages_left);
            if pages_left == 0 {
                break;
            }
        }
        let mut index = 0;
        while index < self.retired.len() && pages_left > 0 {
            if self.retired[index].allocated_pages() == 1 {
                self.retired.swap_remove(index);
                pages_left -= 1;
            } else {
                index += 1;
            }
        }
        while pages_left > 0 && !self.retired_members.is_empty() {
            self.retired_members.pop();
            pages_left -= 1;
        }
        self.diagnostics.retired_pages_this_tick = MAX_RETIRED_PAGES - pages_left;
    }
    fn detach(&mut self, handle: Option<u64>) {
        if let Some(field) = handle.and_then(|id| self.fields.iter_mut().find(|f| f.id == id)) {
            field.refs = field.refs.saturating_sub(1);
        }
    }
    fn cancel(&mut self, unit: u32, tick: u32, viewer: Option<u8>) {
        self.begin(tick);
        // Overflow invalidates the whole cohort with one flag. It cannot leave an
        // unvisited actor using an old result, nor select retained winners by handle.
        if self.diagnostics.invalidation_entries_this_tick >= MAX_ENTRIES {
            self.cancellation_overflow = true;
            return;
        }
        self.diagnostics.invalidation_entries_this_tick += 1;
        // Cancellation is immediate and O(1). Reclamation is separately budgeted.
        self.next_epoch = self.next_epoch.saturating_add(1);
        if let Some(member) = self.members.get_mut(&unit) {
            member.epoch = self.next_epoch;
            member.accepted = tick;
            member.cancelled = true;
            member.arrival_slot = None;
            member.held_leg = HeldLeg::default();
            member.parking_scope = None;
            member.owner_fence = false;
            member.owner_reported = false;
            member.owner_released = false;
            if let Some(viewer) = viewer {
                member.viewer = viewer;
            }
            member.direct_query = None;
            member.direct_refusal = None;
            let buffer = std::mem::take(&mut member.direct);
            let contact = std::mem::take(&mut member.contact);
            member.contact_invalidated = false;
            let motion_bytes = member.motion.take().map_or(0, |m| m.bytes());
            self.member_buffer_bytes = self.member_buffer_bytes.saturating_sub(motion_bytes);
            self.retire_member_buffer(buffer);
            self.retire_member_buffer(contact);
        } else if let Some(viewer) = viewer {
            let growth = 3
                * (index_bytes(&self.members) + self.member_ids.capacity() * size_of::<u32>())
                + size_of::<Member>() * 8;
            if self.members.len() >= MAX_MEMBERS || growth > self.available() {
                self.cancellation_overflow = true;
                return;
            }
            let member = Member::stamp(viewer, tick, self.next_epoch, self.member_ids.len());
            self.member_ids.push(unit);
            self.members.insert(unit, member);
        }
    }

    fn reclaim(&mut self, unit: u32) -> bool {
        if let Some(member) = self.members.remove(&unit) {
            self.member_buffer_bytes = self.member_buffer_bytes.saturating_sub(
                member
                    .motion
                    .as_ref()
                    .map_or(0, waypoint::WaypointMotion::bytes),
            );
            self.detach(member.field);
            self.member_ids.swap_remove(member.slot);
            if let Some(&moved) = self.member_ids.get(member.slot)
                && let Some(row) = self.members.get_mut(&moved)
            {
                row.slot = member.slot;
            }
            self.retire_member_buffer(member.direct);
            self.retire_member_buffer(member.contact);
        }
        true
    }
    fn retire_unused(&mut self) {
        let mut i = 0;
        while i < self.fields.len() {
            if self.fields[i].refs == 0 {
                self.retired.push(self.fields.swap_remove(i));
            } else {
                i += 1;
            }
        }
    }
    fn evict_unproductive(&mut self, tick: u32) {
        let key = |field: &Field| {
            (
                field.last_reachable,
                field.accepted_tick,
                field.base.rx,
                field.base.ry,
                field.seed,
                field.base.radius,
                field.base.profile.max_step,
                field.base.profile.max_grade.numerator,
                field.base.profile.max_grade.denominator,
            )
        };
        let candidate = self
            .fields
            .iter()
            .filter(|field| {
                (matches!(
                    field.stage,
                    Stage::Blocked | Stage::Exhausted | Stage::Pending
                ) || field.resource_fault)
                    && tick.saturating_sub(field.last_reachable) >= 32
            })
            .map(|field| key(field))
            .min();
        if let Some(candidate) = candidate {
            // Indistinguishable physical requests retire together. Vector/handle order
            // cannot decide which viewer keeps capacity. Admission waits for all pages.
            let mut i = 0;
            while i < self.fields.len() {
                if key(&self.fields[i]) == candidate
                    && (matches!(
                        self.fields[i].stage,
                        Stage::Blocked | Stage::Exhausted | Stage::Pending
                    ) || self.fields[i].resource_fault)
                    && tick.saturating_sub(self.fields[i].last_reachable) >= 32
                {
                    self.retired.push(self.fields.swap_remove(i));
                } else {
                    i += 1;
                }
            }
        }
    }
    fn update_diagnostics(&mut self) {
        self.diagnostics.resident_fields = self.fields.len();
        self.diagnostics.resident_bytes = self.resident_bytes();
        self.diagnostics.retired_bytes = self.retired_bytes();
        self.diagnostics.retired_pages = self
            .retired
            .iter()
            .map(|f| f.allocated_pages())
            .sum::<usize>()
            + self.retired_members.len();
        self.diagnostics.retired_pages_this_tick += pages(self.small_retired_bytes);
        self.small_retired_bytes = 0;
        self.diagnostics.memberships = self
            .members
            .values()
            .filter(|m| !m.cancelled && !m.owner_fence)
            .count();
        self.diagnostics.pending = self
            .members
            .values()
            .filter(|m| {
                !m.cancelled
                    && !m.owner_fence
                    && matches!(
                        m.state,
                        NavigationMemberState::Pending | NavigationMemberState::Searching
                    )
            })
            .count();
    }
    fn validate_contact(&mut self, member: &mut Member, terrain: &impl TerrainRead) {
        if member.contact.is_empty() || member.contact_epoch == terrain.epoch() {
            return;
        }
        if member.contact_validation_epoch != terrain.epoch() {
            member.contact_validation_epoch = terrain.epoch();
            member.contact_cursor = 0;
        }
        while member.contact_cursor < member.contact.len()
            && self.diagnostics.invalidation_entries_this_tick < MAX_ENTRIES
        {
            self.diagnostics.invalidation_entries_this_tick += 1;
            let (key, revision) = member.contact[member.contact_cursor];
            member.contact_cursor += 1;
            if !terrain.lease_matches(key, revision) {
                member.contact.clear();
                member.contact_invalidated = true;
                member.contact_epoch = terrain.epoch();
                member.contact_cursor = 0;
                self.diagnostics.invalidations = self.diagnostics.invalidations.saturating_add(1);
                return;
            }
        }
        if member.contact_cursor == member.contact.len() {
            member.contact_epoch = terrain.epoch();
        }
    }
    fn validate_member(&mut self, member: &mut Member, terrain: &impl TerrainRead) -> bool {
        if member.valid_epoch == terrain.epoch() {
            return true;
        }
        if member.validation_epoch != terrain.epoch() {
            member.validation_epoch = terrain.epoch();
            member.validation_cursor = 0;
        }
        while member.validation_cursor < member.direct.len()
            && self.diagnostics.invalidation_entries_this_tick < MAX_ENTRIES
        {
            self.diagnostics.invalidation_entries_this_tick += 1;
            let (key, revision) = member.direct[member.validation_cursor];
            member.validation_cursor += 1;
            if !terrain.lease_matches(key, revision) {
                member.direct.clear();
                member.direct_query = None;
                member.direct_refusal = None;
                member.seed = None;
                member.connector = None;
                member.connector_cursor = 0;
                member.valid_epoch = terrain.epoch();
                member.field_revision = member.field_revision.saturating_add(1);
                member.state = NavigationMemberState::Pending;
                self.diagnostics.invalidations = self.diagnostics.invalidations.saturating_add(1);
                return true;
            }
        }
        if member.validation_cursor == member.direct.len() {
            member.valid_epoch = terrain.epoch();
        }
        member.valid_epoch == terrain.epoch()
    }
    fn member_query(
        &mut self,
        member: &mut Member,
        terrain: &impl TerrainRead,
        from: Vec2,
        to: Vec2,
    ) -> Certificate {
        self.member_query_with_refusal(member, terrain, from, to).0
    }
    /// As `member_query`. The flag is true only for a geometry refusal whose
    /// complete lease was admitted. Quota, revision, capacity and memory
    /// refusals are transient and are never flagged.
    fn member_query_with_refusal(
        &mut self,
        member: &mut Member,
        terrain: &impl TerrainRead,
        from: Vec2,
        to: Vec2,
    ) -> (Certificate, bool, usize) {
        const TRANSIENT: (Certificate, bool, usize) = (Certificate::Pending, false, 0);
        if self.diagnostics.edge_checks_this_tick >= MAX_EDGES {
            return TRANSIENT;
        }
        self.diagnostics.edge_checks_this_tick += 1;
        let mut lease = BTreeMap::new();
        let result = certify_with_refusal(
            terrain,
            from,
            to,
            f32::from_bits(member.radius),
            member.profile,
            &mut lease,
        );
        // A transient refusal after the geometry still reports the work that it used.
        let spent = (Certificate::Pending, false, result.2);
        // A member owns at most one 64KiB page of dependency rows. Reserve before growth;
        // rejected partial certificates can never activate guidance without their leases.
        let mut additions = 0;
        for (&key, &revision) in &lease {
            match member.direct.binary_search_by_key(&key, |row| row.0) {
                Ok(i) if member.direct[i].1 != revision => return spent,
                Ok(_) => {}
                Err(_) => additions += 1,
            }
        }
        let length = member.direct.len() + additions;
        if length > 4096 {
            return spent;
        }
        let extra = length.saturating_sub(member.direct.capacity()) * size_of::<(usize, u64)>();
        if extra > self.available() || (extra > 0 && self.retiring()) {
            return spent;
        }
        member
            .direct
            .reserve_exact(length.saturating_sub(member.direct.len()));
        for (key, revision) in lease {
            if let Err(i) = member.direct.binary_search_by_key(&key, |row| row.0) {
                member.direct.insert(i, (key, revision));
            }
        }
        member.valid_epoch = terrain.epoch();
        result
    }
    fn direct_query(
        &mut self,
        member: &mut Member,
        terrain: &impl TerrainRead,
        from: Vec2,
        to: Vec2,
    ) -> Certificate {
        // A cache hit consumes the same scheduling quota as a fresh query. No
        // stale result can bypass a suspended local dependency validation.
        if self.diagnostics.edge_checks_this_tick >= MAX_EDGES
            || !self.validate_member(member, terrain)
        {
            return Certificate::Pending;
        }
        let key = DirectQueryKey {
            from: from.to_array().map(f32::to_bits),
            to: to.to_array().map(f32::to_bits),
            radius: member.radius,
            profile: member.profile,
            viewer: member.viewer,
            order_epoch: member.epoch,
        };
        if let Some(memo) = member.direct_query.filter(|memo| memo.key == key) {
            self.diagnostics.edge_checks_this_tick += 1;
            self.diagnostics.direct_query_cache_hits =
                self.diagnostics.direct_query_cache_hits.saturating_add(1);
            return memo.result;
        }
        let refusal =
            terrain
                .grid()
                .point_to_cell(point(from))
                .ok()
                .map(|cell| DirectRefusalMemo {
                    from_cell: [cell.x, cell.y],
                    to: key.to,
                    radius: key.radius,
                    profile: key.profile,
                    viewer: key.viewer,
                    order_epoch: key.order_epoch,
                    terrain_epoch: terrain.epoch(),
                });
        if refusal.is_some() && member.direct_refusal == refusal {
            // The caller reads only `== Clear`. A hit takes the same edge
            // quota as a fresh query, so the service of other members is equal.
            self.diagnostics.edge_checks_this_tick += 1;
            self.diagnostics.direct_query_cache_hits =
                self.diagnostics.direct_query_cache_hits.saturating_add(1);
            return Certificate::Pending;
        }
        // A member whose last direct line was clear is always served: its route must
        // not change because other members used the work of this tick. The limit
        // applies to members with no clear line (a new order, or an earlier refusal).
        let limited = !member
            .direct_query
            .is_some_and(|memo| memo.result == Certificate::Clear);
        // Each order has its own account: a large order cannot put off the direct
        // query of a unit of another order. The members of one group order have the
        // same acceptance tick and the same destination goal; a member with no
        // destination region is alone with its endpoint. An equal key of two orders
        // only makes them share one account.
        let goal = member
            .parking_scope
            .map_or(member.endpoint, |scope| scope.region.goal);
        let order = (u64::from(member.accepted) << 32)
            ^ u64::from(goal.x.to_bits())
            ^ (u64::from(goal.y.to_bits()) << 16);
        let account = self
            .direct_order_work
            .iter()
            .position(|row| !row.used || row.order == order)
            .unwrap_or(DIRECT_ORDER_ROWS - 1);
        if limited {
            // Turns: when members of this order were put off in the tick before, a
            // member that ran then yields now. So the limit reaches every member in
            // a bounded number of ticks and no member is put off without end.
            let yields = member.direct_turn.is_some()
                && member.direct_turn.and_then(|turn| turn.checked_add(1)) == self.tick
                && self
                    .direct_order_before
                    .iter()
                    .any(|row| row.used && row.order == order && row.put_off);
            let row = &mut self.direct_order_work[account];
            (row.used, row.order) = (true, order);
            if yields || row.work >= DIRECT_QUERY_WORK {
                // This member runs no geometry now and asks again on the next tick.
                row.put_off = true;
                self.diagnostics.edge_checks_this_tick += 1;
                self.diagnostics.direct_query_deferrals =
                    self.diagnostics.direct_query_deferrals.saturating_add(1);
                return Certificate::Pending;
            }
            member.direct_turn = self.tick;
        }
        self.diagnostics.direct_query_cache_misses =
            self.diagnostics.direct_query_cache_misses.saturating_add(1);
        let (result, geometry_refusal, work) =
            self.member_query_with_refusal(member, terrain, from, to);
        if limited {
            self.diagnostics.direct_query_work_this_tick += work;
            self.direct_order_work[account].work += work;
        }
        // member_query returns Clear/Blocked only after admitting its complete
        // local lease. Those results are retained for the exact key. An admitted
        // geometry refusal is retained for the start cell. Unknown terrain and
        // every resource refusal are never retained: the next tick asks again.
        member.direct_query = match result {
            Certificate::Clear | Certificate::Blocked => Some(DirectQueryMemo { key, result }),
            Certificate::Pending => None,
        };
        member.direct_refusal = refusal.filter(|_| geometry_refusal);
        result
    }
    fn guide(
        &mut self,
        member: &mut Member,
        unit: &MovementFrameUnit,
        terrain: &Planning<'_>,
    ) -> Option<Vec2> {
        self.validate_contact(member, terrain);
        if !self.validate_member(member, terrain) {
            member.state = NavigationMemberState::Pending;
            return None;
        }
        let endpoint = member.endpoint;
        if self.direct_query(member, terrain, unit.position, endpoint) == Certificate::Clear {
            member.state = NavigationMemberState::Direct;
            self.diagnostics.cache_hits = self.diagnostics.cache_hits.saturating_add(1);
            return Some(member.endpoint);
        }
        if member.seed.is_none() {
            let candidates = connectors(terrain, endpoint);
            // Persist the finite connector cursor. An expensive failed connector cannot
            // monopolize all service or restart ahead of other actors on every tick.
            for _ in 0..3 {
                if member.connector_cursor >= 90 {
                    member.state = NavigationMemberState::Pending;
                    break;
                }
                let cursor = member.connector_cursor;
                let Some(&vertex) = candidates.get(cursor % 9) else {
                    member.state = NavigationMemberState::Blocked;
                    break;
                };
                if self.diagnostics.edge_checks_this_tick + 2 > MAX_EDGES {
                    break;
                }
                member.connector_cursor += 1;
                if cursor < 9 {
                    if self.member_query(member, terrain, vertex_point(terrain, vertex), endpoint)
                        == Certificate::Clear
                    {
                        member.seed = Some(vertex);
                        break;
                    }
                } else {
                    // Bounded quarter-cell connector refinement. It changes neither the
                    // graph radius nor the actual destination. Every leg is exact certified.
                    let offset = (cursor / 9) - 1;
                    let quarter = width(terrain) * 0.25;
                    let via = Vec2::new(
                        (f64::from(endpoint.x) + (offset % 3) as f64 * quarter - quarter) as f32,
                        (f64::from(endpoint.y) + (offset / 3) as f64 * quarter - quarter) as f32,
                    );
                    if self.member_query(member, terrain, vertex_point(terrain, vertex), via)
                        == Certificate::Clear
                        && self.member_query(member, terrain, via, endpoint) == Certificate::Clear
                    {
                        member.seed = Some(vertex);
                        member.connector = Some(via);
                        break;
                    }
                }
            }
        }
        let seed = member.seed?;
        let base = Base {
            viewer: member.viewer,
            radius: member.radius,
            profile: member.profile,
            rx: seed.x / REGION_HALF_WIDTH * REGION_HALF_WIDTH,
            ry: seed.y / REGION_HALF_WIDTH * REGION_HALF_WIDTH,
        };
        if let Some(id) = member.field
            && self.fields.iter().all(|f| f.id != id || f.base != base)
        {
            self.detach(member.field.take());
        }
        if member.field.is_none() {
            // Pending terminal components are shared provisionally. No actor uses them until
            // connectivity to its exact endpoint connector has been certified.
            let existing = self.fields.iter().position(|field| {
                field.base == base
                    && (field.stage == Stage::Component
                        || field.node(terrain, seed).is_some_and(|n| n.component))
            });
            let index = if let Some(index) = existing {
                index
            } else {
                if (self.fields.len() >= MAX_FIELDS || self.available() < 5 * PAGE_BYTES)
                    && !self.retiring()
                {
                    // A proved blocked/exhausted request, or a memory-stalled field, cannot
                    // monopolize capacity forever. Retire an old unproductive field using
                    // physical destination geometry; actor/field handles do not break ties.
                    let tick = self.tick.unwrap_or(0);
                    self.evict_unproductive(tick);
                }
                if self.fields.len() >= MAX_FIELDS
                    || self.retiring()
                    || self.available() < 5 * PAGE_BYTES
                {
                    member.state = NavigationMemberState::Pending;
                    return None;
                }
                self.next_field = self.next_field.checked_add(1)?;
                let mut field = Box::new(Field::fresh(
                    self.next_field
                        .checked_mul(VIEWERS as u64)?
                        .checked_add(u64::from(self.viewer))?,
                    base,
                    seed,
                    member.endpoint,
                    member.accepted,
                ));
                if !field.reset(terrain, self.available()) {
                    member.state = NavigationMemberState::Pending;
                    return None;
                }
                self.fields.push(field);
                self.diagnostics.searches = self.diagnostics.searches.saturating_add(1);
                self.fields.len() - 1
            };
            self.fields[index].refs += 1;
            member.field = Some(self.fields[index].id);
        }
        let index = self
            .fields
            .iter()
            .position(|f| Some(f.id) == member.field)?;
        let field = &self.fields[index];
        member.field_revision = field.revision;
        member.state = NavigationMemberState::Searching;
        if field.valid_epoch != terrain.epoch()
            || field.resource_fault
            || field.stage == Stage::Component
        {
            self.fields[index].demand = true;
            return None;
        }
        if field.stage == Stage::Blocked {
            member.state = NavigationMemberState::Blocked;
            return None;
        }
        if !field.node(terrain, seed).is_some_and(|n| n.component) {
            self.detach(member.field.take());
            return None;
        }
        // The field's reverse tree leads to its root. Within the connected terminal
        // component, follow the same certified bidirectional tree toward this member's seed.
        let candidates = connectors(terrain, unit.position);
        let mut options = Vec::with_capacity(9);
        for vertex in candidates {
            let Some(node) = self.fields[index].node(terrain, vertex) else {
                continue;
            };
            if !node.closed {
                continue;
            }
            let mut cursor = seed;
            let mut child = seed;
            for _ in 0..=289 {
                if cursor == vertex {
                    break;
                }
                let Some(row) = self.fields[index].node(terrain, cursor) else {
                    break;
                };
                if !row.component_parent_set {
                    break;
                }
                child = cursor;
                cursor = row.component_parent;
            }
            let target = if cursor == vertex {
                if vertex == seed {
                    member.connector.unwrap_or(member.endpoint)
                } else {
                    vertex_point(terrain, child)
                }
            } else if node.parent_set {
                vertex_point(terrain, node.parent)
            } else {
                vertex_point(terrain, vertex)
            };
            options.push((node.cost, vertex, target));
        }
        options.sort_by_key(|(cost, vertex, _)| (*cost, *vertex));
        for (_, _, target) in options {
            if target != unit.position
                && self.member_query(member, terrain, unit.position, target) == Certificate::Clear
            {
                self.fields[index].last_reachable = self.tick.unwrap_or(0);
                member.state = NavigationMemberState::Ready;
                return Some(target);
            }
        }
        if self.fields[index].stage == Stage::Exhausted {
            member.state = NavigationMemberState::Blocked;
        } else if self.fields[index].stage == Stage::Pending {
            member.state = NavigationMemberState::Pending;
        } else {
            self.fields[index].demand = true;
        }
        None
    }
    fn advance_fields(&mut self, phase: &super::super::TerrainPhase, tick: u32) {
        self.advance_fields_with(tick, |viewer| Planning { phase, viewer });
    }
    fn advance_fields_with<T: TerrainRead>(
        &mut self,
        tick: u32,
        mut planning: impl FnMut(u8) -> T,
    ) {
        // Bounded 32-entry physical ordering. A serviced field goes behind waiting older
        // fields on the following round, independent of handles or membership unit IDs.
        let mut order = (0..self.fields.len()).collect::<Vec<_>>();
        order.sort_by(|&a, &b| {
            let a = &self.fields[a];
            let b = &self.fields[b];
            (
                a.last_service,
                a.accepted_tick,
                a.base.rx,
                a.base.ry,
                a.seed,
                a.base.radius,
            )
                .cmp(&(
                    b.last_service,
                    b.accepted_tick,
                    b.base.rx,
                    b.base.ry,
                    b.seed,
                    b.base.radius,
                ))
                .then_with(|| a.origin.x.total_cmp(&b.origin.x))
                .then_with(|| a.origin.y.total_cmp(&b.origin.y))
        });
        loop {
            let mut progressed = false;
            for &index in &order {
                if self.diagnostics.edge_checks_this_tick + 3 > MAX_EDGES / 2 {
                    break;
                }
                if !self.fields[index].demand && self.fields[index].stage != Stage::Component {
                    continue;
                }
                let limit = self.field_limit(index);
                let terrain = planning(self.fields[index].base.viewer);
                let valid = self.fields[index].validate(&terrain, &mut self.diagnostics, limit);
                if valid && self.fields[index].advance(&terrain, &mut self.diagnostics, limit) {
                    self.fields[index].last_service = tick;
                    progressed = true;
                }
            }
            if !progressed
                || self.diagnostics.expansions_this_tick >= MAX_EXPANSIONS
                || self.diagnostics.edge_checks_this_tick >= MAX_EDGES / 2
            {
                break;
            }
        }
        for field in &mut self.fields {
            field.demand = false;
        }
    }
}
fn connectors(terrain: &impl TerrainRead, point: Vec2) -> Vec<Vertex> {
    if !point.is_finite() {
        return Vec::new();
    }
    let scale = width(terrain) * 0.5;
    let x = (f64::from(point.x) / scale).round();
    let y = (f64::from(point.y) / scale).round();
    if x < 0.0 || y < 0.0 || x > f64::from(u32::MAX - 1) || y > f64::from(u32::MAX - 1) {
        return Vec::new();
    }
    let centre = Vertex {
        x: x as u32,
        y: y as u32,
    };
    let mut result = Vec::with_capacity(9);
    result.push(centre);
    for direction in 0..8 {
        if let Some(v) = neighbor(centre, direction).filter(|v| vertex_index(terrain, *v).is_some())
        {
            result.push(v);
        }
    }
    result.sort_by(|&a, &b| {
        let distance = |v| {
            let p = vertex_point(terrain, v);
            let dx = f64::from(p.x) - f64::from(point.x);
            let dy = f64::from(p.y) - f64::from(point.y);
            dx * dx + dy * dy
        };
        distance(a).total_cmp(&distance(b)).then_with(|| a.cmp(&b))
    });
    result
}

/// Outward-rounded float bounds enclose the exact widened-f32 offset products.
/// No actor pose is snapped, and every included cell is read only through Planning.
fn contact_dependencies(
    terrain: &impl TerrainRead,
    from: Vec2,
    to: Vec2,
    axis: Vec2,
    radius: f32,
) -> Option<BTreeMap<usize, u64>> {
    contact_dependencies_with_probe(terrain, from, to, axis, radius, radius)
}
fn contact_dependencies_with_probe(
    terrain: &impl TerrainRead,
    from: Vec2,
    to: Vec2,
    axis: Vec2,
    radius: f32,
    probe_radius: f32,
) -> Option<BTreeMap<usize, u64>> {
    use turts_rules::terrain::geometry::{Cell, Point};
    if !from.is_finite()
        || !to.is_finite()
        || !axis.is_finite()
        || !radius.is_finite()
        || radius <= 0.0
        || !probe_radius.is_finite()
        || probe_radius <= 0.
    {
        return None;
    }
    let r = f64::from(radius);
    let reach = 2.0 * f64::from(probe_radius);
    let rx = (f64::from(axis.y).abs() * reach + r).next_up();
    let ry = (f64::from(axis.x).abs() * reach + r).next_up();
    let min = terrain
        .grid()
        .point_to_cell(Point {
            x: (f64::from(from.x.min(to.x)) - rx).next_down(),
            y: (f64::from(from.y.min(to.y)) - ry).next_down(),
        })
        .ok()?;
    let max = terrain
        .grid()
        .point_to_cell(Point {
            x: (f64::from(from.x.max(to.x)) + rx).next_up(),
            y: (f64::from(from.y.max(to.y)) + ry).next_up(),
        })
        .ok()?;
    let area = u64::from(max.x - min.x + 1) * u64::from(max.y - min.y + 1);
    if area > 4096 {
        return None;
    }
    let mut rows = BTreeMap::new();
    for y in min.y..=max.y {
        for x in min.x..=max.x {
            let cell = Cell { x, y };
            if !terrain.rectangle(cell, cell, &mut rows) {
                return None;
            }
        }
    }
    Some(rows)
}
// A heap row's frame index is a lookup address, never a priority tie-breaker.
#[derive(Clone, Copy)]
struct Ranked {
    key: [u64; 12],
    index: usize,
}
impl PartialEq for Ranked {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl Eq for Ranked {}
impl PartialOrd for Ranked {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Ranked {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}
fn float_key(value: f32) -> u64 {
    let bits = bits(value);
    u64::from(if bits >> 31 != 0 {
        !bits
    } else {
        bits ^ (1 << 31)
    })
}
impl ViewerNavigation {
    pub(super) fn completed(&mut self, unit: u32) {
        if let Some(member) = self.members.get_mut(&unit) {
            member.parking_scope = None;
            member.cancelled = true;
        }
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "The planning boundary keeps phase inputs explicit."
    )]
    pub(super) fn propose(
        &mut self,
        world: &World,
        frame: &MovementFrame,
        phase: &super::super::TerrainPhase,
        profiles: &[Option<Traversal>],
        candidate: &mut MovementFrame,
        waiting: &mut BTreeSet<u32>,
        fenced: &BTreeSet<u32>,
    ) {
        let tick = world.get_resource::<crate::Tick>().map_or(0, |tick| tick.0);
        self.begin(tick);
        let mut ranked = std::collections::BinaryHeap::<Ranked>::with_capacity(MAX_ENTRIES + 1);
        let mut cutoff: Option<[u64; 12]> = None;
        let mut ranking_overflow = false;
        for (index, unit) in frame.units.iter().enumerate() {
            self.diagnostics.frame_rows_this_tick += 1;
            if unit.owner_slot != self.viewer {
                continue;
            }
            if fenced.contains(&unit.unit_id) {
                candidate.units[index].speed = None;
                candidate.units[index].target = None;
                waiting.insert(unit.unit_id);
                continue;
            }
            if let Some(member) = self.members.get_mut(&unit.unit_id) {
                member.seen = tick;
                member.live_target = unit.target.is_some();
                if self.cancellation_overflow {
                    member.cancelled = true;
                    member.parking_scope = None;
                }
                if unit.target.is_none() {
                    member.cancelled = true;
                    member.parking_scope = None;
                }
            }
            let Some(goal) = super::requested_target(unit) else {
                continue;
            };
            if unit.speed.is_none() {
                continue;
            }
            candidate.units[index].target = None;
            candidate.units[index].speed = None;
            waiting.insert(unit.unit_id);
            if self
                .members
                .get(&unit.unit_id)
                .is_some_and(|m| m.owner_fence)
            {
                continue;
            }
            let Some(profile) = profiles[index] else {
                continue;
            };
            if let Some(member) = self.members.get_mut(&unit.unit_id)
                && member.parking_scope.is_some_and(|scope| {
                    !scope.matches(unit, profile, member.epoch)
                        || super::public_move_goal(world, unit.unit_id, unit.owner_slot)
                            != Some(scope.region.goal)
                })
            {
                member.parking_scope = None;
            }
            let member = self.members.get(&unit.unit_id);
            let priority = member.map_or(1, |m| match m.state {
                NavigationMemberState::Direct
                | NavigationMemberState::Ready
                | NavigationMemberState::Searching => 0,
                NavigationMemberState::Pending => 1,
                NavigationMemberState::Blocked => 2,
            });
            let key = [
                priority,
                u64::from(member.map_or(tick, |m| m.serviced)),
                u64::from(member.map_or(tick, |m| m.accepted)),
                float_key(unit.position.x),
                float_key(unit.position.y),
                float_key(goal.x),
                float_key(goal.y),
                u64::from(unit.radius.to_bits()),
                u64::from(profile.max_step),
                profile.max_grade.numerator,
                profile.max_grade.denominator,
                0,
            ];
            // Heap comparisons are bounded above by 2*ceil(log2(4097))+2 per visit.
            // Reserve that upper bound before doing any heap operation. Overflow admits
            // nobody this tick, so authoritative frame storage order cannot choose winners.
            if self.diagnostics.ranking_comparisons_this_tick + 40 > MAX_RANK_COMPARISONS - 32_768 {
                ranking_overflow = true;
                continue;
            }
            self.diagnostics.ranking_comparisons_this_tick += 40;
            let row = Ranked { key, index };
            if ranked.len() < MAX_ENTRIES {
                ranked.push(row);
            } else if ranked.peek().is_some_and(|last| key < last.key) {
                if let Some(removed) = ranked.pop() {
                    cutoff = Some(cutoff.map_or(removed.key, |old| old.min(removed.key)));
                }
                ranked.push(row);
            } else {
                cutoff = Some(cutoff.map_or(key, |old| old.min(key)));
            }
        }
        self.cancellation_overflow = false;
        // Cancellation/death reclamation scans a bounded rotating cursor. Its order only
        // reclaims memory; all new allocations stop while deferred field retirement exists.
        let mut examined = 0;
        while !self.member_ids.is_empty()
            && examined < 256
            && self.diagnostics.invalidation_entries_this_tick < MAX_ENTRIES
        {
            self.cleanup_cursor %= self.member_ids.len();
            let unit = self.member_ids[self.cleanup_cursor];
            self.diagnostics.invalidation_entries_this_tick += 1;
            examined += 1;
            let dead = self.members.get(&unit).is_some_and(|m| {
                (!m.owner_fence || m.owner_reported)
                    && (m.seen != tick || (m.cancelled && !m.live_target))
            });
            if dead && self.reclaim(unit) {
            } else {
                self.cleanup_cursor += 1;
            }
        }
        self.retire_unused();
        self.advance_fields(phase, tick);
        if ranking_overflow {
            self.diagnostics.budget_waits = self.diagnostics.budget_waits.saturating_add(1);
            self.update_diagnostics();
            return;
        }
        let comparisons = std::cell::Cell::new(0usize);
        let mut ranked = ranked.into_vec();
        ranked.sort_by(|a, b| {
            comparisons.set(comparisons.get() + 1);
            a.key.cmp(&b.key)
        });
        self.diagnostics.ranking_comparisons_this_tick += comparisons.get();
        if self.diagnostics.ranking_comparisons_this_tick > MAX_RANK_COMPARISONS {
            // The pre-sort reserve leaves ample room for 4096 rows; keep refusal explicit.
            // No hidden work is credited as completed. This remains an explicit hold.
            self.update_diagnostics();
            return;
        }
        let mut new_rows = Vec::new();
        for row in ranked {
            if cutoff.is_some_and(|cutoff| row.key == cutoff) {
                continue;
            }
            if self.diagnostics.admission_entries_this_tick >= MAX_ENTRIES
                || self.diagnostics.edge_checks_this_tick >= MAX_EDGES
            {
                break;
            }
            self.diagnostics.admission_entries_this_tick += 1;
            let unit = &frame.units[row.index];
            let Some(profile) = profiles[row.index] else {
                continue;
            };
            let Some(goal) = unit.target else {
                continue;
            };
            let retained_scope = self.members.get(&unit.unit_id).and_then(|member| {
                member.parking_scope.filter(|scope| {
                    scope.matches(unit, profile, member.epoch)
                        && super::public_move_goal(world, unit.unit_id, unit.owner_slot)
                            == Some(scope.region.goal)
                })
            });
            let stamp = self
                .members
                .get(&unit.unit_id)
                .filter(|member| member.cancelled || retained_scope.is_some())
                .map(|member| (member.epoch, member.accepted));
            let changed = self.members.get(&unit.unit_id).is_some_and(|m| {
                m.cancelled
                    || m.endpoint != goal
                    || m.radius != unit.radius.to_bits()
                    || m.profile != profile
            });
            if changed {
                if let Some(member) = self.members.get_mut(&unit.unit_id) {
                    member.cancelled = true;
                }
                if !self.reclaim(unit.unit_id) {
                    continue;
                }
            }
            if !self.members.contains_key(&unit.unit_id) {
                // The conservative reserve includes a simultaneous hash reallocation and
                // the member vector. Never allocate a coordinate-sized dense table.
                let growth = 3
                    * (index_bytes(&self.members) + self.member_ids.capacity() * size_of::<u32>())
                    + size_of::<Member>() * 8
                    + PAGE_BYTES;
                if self.members.len() >= MAX_MEMBERS || self.retiring() || growth > self.available()
                {
                    continue;
                }
                let (epoch, accepted) = stamp.unwrap_or_else(|| {
                    self.next_epoch = self.next_epoch.saturating_add(1);
                    (self.next_epoch, tick)
                });
                let mut member =
                    Member::stamp(unit.owner_slot, accepted, epoch, self.member_ids.len());
                member.endpoint = goal;
                member.radius = unit.radius.to_bits();
                member.profile = profile;
                member.parking_scope = retained_scope;
                member.cancelled = false;
                member.live_target = true;
                member.seen = tick;
                member.valid_epoch = phase.planning_epoch(unit.owner_slot);
                self.member_ids.push(unit.unit_id);
                self.members.insert(unit.unit_id, member);
                new_rows.push(row.index);
            }
            let Some(mut member) = self.members.remove(&unit.unit_id) else {
                continue;
            };
            member.serviced = tick;
            let old_bytes = member.bytes();
            let terrain = Planning {
                phase,
                viewer: unit.owner_slot,
            };
            let waypoint = self.guide(&mut member, unit, &terrain);
            if matches!(
                member.state,
                NavigationMemberState::Pending | NavigationMemberState::Searching
            ) {
                self.diagnostics.max_pending_age_ticks = self
                    .diagnostics
                    .max_pending_age_ticks
                    .max(u64::from(tick.saturating_sub(member.accepted)));
            }
            self.member_buffer_bytes =
                self.member_buffer_bytes.saturating_sub(old_bytes) + member.bytes();
            self.members.insert(unit.unit_id, member);
            if let Some(point) = waypoint {
                candidate.units[row.index] = unit.clone();
                candidate.units[row.index].target = Some(point);
                if point != goal {
                    candidate.units[row.index].progress = None;
                    candidate.units[row.index].plain_progress = None;
                }
                waiting.remove(&unit.unit_id);
            }
        }
        if self.retiring() {
            // A later retirement in this batch cannot leave the first enumerated new
            // actors moving while their peers wait for the same reclamation boundary.
            for index in new_rows {
                candidate.units[index].speed = None;
                candidate.units[index].target = None;
                waiting.insert(frame.units[index].unit_id);
            }
        }
        self.update_diagnostics();
    }
}
impl ViewerNavigation {
    fn audit(&self) -> NavigationAudit {
        let mut members = self
            .members
            .iter()
            .filter(|(_, m)| !m.cancelled && !m.owner_fence)
            .map(|(&unit, m)| {
                let field = m
                    .field
                    .and_then(|id| self.fields.iter().find(|f| f.id == id));
                let mut dependencies = m.direct.clone();
                if let Some(field) = field {
                    dependencies.extend(field.leases.rows());
                    dependencies.sort_unstable();
                    dependencies.dedup();
                }
                NavigationMemberAudit {
                    unit,
                    viewer: m.viewer,
                    order_epoch: m.epoch,
                    field: field.map(|f| f.id),
                    field_revision: field.map_or(m.field_revision, |f| f.revision),
                    endpoint: m.endpoint.to_array(),
                    destination_region: m.parking_scope.map(|s| NavigationDestinationRegionAudit {
                        goal: s.region.goal.to_array(),
                        primary_min: s.region.min.to_array(),
                        primary_max: s.region.max.to_array(),
                        cohort_radius: s.region.cohort_radius,
                        actor_radius: f32::from_bits(s.radius),
                        epoch: s.epoch,
                        inline_bytes: size_of::<Option<ParkingScope>>(),
                    }),
                    waypoint: m.motion.as_ref().map(|v| v.target.to_array()),
                    waypoint_group: m.motion.as_ref().and_then(|v| v.group.clone()),
                    waypoint_plain: m.motion.as_ref().and_then(|v| v.plain_progress()),
                    dependencies,
                    state: if m.field.is_some() && field.is_none() {
                        NavigationMemberState::Pending
                    } else {
                        m.state
                    },
                }
            })
            .collect::<Vec<_>>();
        // Presentation order is irrelevant to service. Consumers normalize opaque actors.
        members.sort_by_key(|m| m.unit);
        NavigationAudit {
            members,
            diagnostics: self.diagnostics,
        }
    }
}

fn exhausted_approach(
    member: &Member,
    field: &Field,
    terrain: &impl TerrainRead,
    from: Vec2,
    endpoint: Vec2,
) -> bool {
    member.live_target && member.viewer == field.base.viewer
        && !member.cancelled && !member.owner_fence && !member.owner_released
        && member.endpoint == endpoint && member.state == NavigationMemberState::Blocked
        && member.valid_epoch == terrain.epoch() && field.valid_epoch == terrain.epoch()
        && member.field_revision == field.revision && field.stage == Stage::Exhausted
        && !field.resource_fault && !field.unresolved_edges && field.seed_checked
        // Blocked can also follow an unfinished connector certificate. Only the
        // complete absence of reachable connector nodes proves this refusal.
        && connectors(terrain, from).iter().all(|&vertex| {
            !field.node(terrain, vertex).is_some_and(|node| node.closed)
        })
}

/// Publicly partition scarce routing resources. Neither private work nor cache pressure
/// in one configured viewer can donate work, memory, or admission priority to another.
#[derive(Resource)]
pub(crate) struct TerrainNavigation {
    viewers: [ViewerNavigation; VIEWERS],
    pub(super) diagnostics: NavigationDiagnostics,
    traffic_bytes: usize,
}
impl Default for TerrainNavigation {
    fn default() -> Self {
        Self {
            viewers: std::array::from_fn(|viewer| ViewerNavigation {
                viewer: viewer as u8,
                ..Default::default()
            }),
            diagnostics: NavigationDiagnostics::default(),
            traffic_bytes: 0,
        }
    }
}
impl TerrainNavigation {
    /// A completed permitted search with no reachable start connector can reject
    /// a work approach. Pending/uncertain geometry and resource pressure cannot.
    pub(crate) fn approach_blocked(
        &self,
        unit: u32,
        viewer: u8,
        from: Vec2,
        endpoint: Vec2,
        phase: &super::super::TerrainPhase,
    ) -> bool {
        let Some(book) = self.viewers.get(usize::from(viewer)) else {
            return false;
        };
        let Some(member) = book.members.get(&unit) else {
            return false;
        };
        let field = member
            .field
            .and_then(|id| book.fields.iter().find(|field| field.id == id));
        let Some(field) = field else {
            return false;
        };
        !book.retiring()
            && member.viewer == viewer
            && exhausted_approach(member, field, &Planning { phase, viewer }, from, endpoint)
    }
    /// A local solver refusal is recoverable while this terrain order retains
    /// authority. Flat physical escape and retired/fenced orders do not qualify.
    pub(crate) fn continues_order(&self, unit: u32, viewer: u8) -> bool {
        self.viewers
            .get(usize::from(viewer))
            .and_then(|book| book.members.get(&unit))
            .is_some_and(|member| {
                member.live_target
                    && !member.cancelled
                    && !member.owner_fence
                    && !member.owner_released
            })
    }
    pub(super) fn contact_ready(
        &self,
        unit: u32,
        viewer: u8,
        phase: &super::super::TerrainPhase,
    ) -> bool {
        self.allows_traffic(unit, viewer)
            && self
                .viewers
                .get(usize::from(viewer))
                .and_then(|book| book.members.get(&unit))
                .is_some_and(|member| {
                    member.contact.is_empty()
                        || member.contact_epoch == phase.planning_epoch(viewer)
                })
    }
    pub(super) fn invalidate_contact_history(
        &mut self,
        frame: &MovementFrame,
        traffic: &mut TrafficState,
    ) {
        for unit in &frame.units {
            let Some(book) = self.viewers.get_mut(usize::from(unit.owner_slot)) else {
                continue;
            };
            let Some(member) = book.members.get_mut(&unit.unit_id) else {
                continue;
            };
            if member.contact_invalidated {
                traffic.invalidate_contact(unit);
                member.contact_invalidated = false;
            }
        }
    }
    pub(super) fn contact_departed(&mut self, unit: u32, viewer: u8) {
        let Some(book) = self.viewers.get_mut(usize::from(viewer)) else {
            return;
        };
        let Some(member) = book.members.get_mut(&unit) else {
            return;
        };
        let buffer = std::mem::take(&mut member.contact);
        book.retire_member_buffer(buffer);
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "The immutable phase, actor authority and exact segment define a contact witness"
    )]
    pub(super) fn observe_contact(
        &mut self,
        phase: &super::super::TerrainPhase,
        unit: &MovementFrameUnit,
        from: Vec2,
        to: Vec2,
        axis: Vec2,
        profile: Traversal,
    ) -> turts_rules::terrain::geometry::DualSupportContact {
        self.observe_public_contact(phase, unit, from, to, axis, unit.radius, profile)
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "The owned observer leases a current public footprint/probe witness"
    )]
    pub(super) fn observe_public_contact(
        &mut self,
        phase: &super::super::TerrainPhase,
        unit: &MovementFrameUnit,
        from: Vec2,
        to: Vec2,
        axis: Vec2,
        probe_radius: f32,
        profile: Traversal,
    ) -> turts_rules::terrain::geometry::DualSupportContact {
        use turts_rules::terrain::geometry::DualSupportContact;
        if !self.contact_ready(unit.unit_id, unit.owner_slot, phase) {
            return DualSupportContact::Unresolved;
        }
        let observed = if probe_radius == unit.radius {
            super::geometry::permitted_entrance_contact(
                phase,
                unit.owner_slot,
                from,
                to,
                axis,
                unit.radius,
                profile,
            )
        } else {
            super::geometry::permitted_public_contact(
                phase,
                unit.owner_slot,
                from,
                to,
                axis,
                unit.radius,
                probe_radius,
                profile,
            )
        };
        if !matches!(
            observed,
            DualSupportContact::Inside | DualSupportContact::Transition { .. }
        ) {
            return observed;
        }
        let terrain = Planning {
            phase,
            viewer: unit.owner_slot,
        };
        let rows = if probe_radius == unit.radius {
            contact_dependencies(&terrain, from, to, axis, unit.radius)
        } else {
            contact_dependencies_with_probe(&terrain, from, to, axis, unit.radius, probe_radius)
        };
        let Some(rows) = rows else {
            return DualSupportContact::Unresolved;
        };
        let book = &mut self.viewers[usize::from(unit.owner_slot)];
        let Some(mut member) = book.members.remove(&unit.unit_id) else {
            return DualSupportContact::Unresolved;
        };
        let prior = member.bytes();
        let mut added = 0;
        let mut coherent = true;
        for (&key, &revision) in &rows {
            match member.contact.binary_search_by_key(&key, |row| row.0) {
                Ok(index) => coherent &= member.contact[index].1 == revision,
                Err(_) => added += 1,
            }
        }
        let length = member.contact.len() + added;
        let extra = length.saturating_sub(member.contact.capacity()) * size_of::<(usize, u64)>();
        let allowed = coherent
            && length <= 4096
            && extra <= book.available()
            && (!book.retiring() || extra == 0);
        if allowed {
            member
                .contact
                .reserve_exact(length.saturating_sub(member.contact.len()));
            for (key, revision) in rows {
                if let Err(index) = member.contact.binary_search_by_key(&key, |row| row.0) {
                    member.contact.insert(index, (key, revision));
                }
            }
            member.contact_epoch = phase.planning_epoch(unit.owner_slot);
            member.contact_validation_epoch = member.contact_epoch;
            member.contact_cursor = 0;
        }
        book.member_buffer_bytes = book.member_buffer_bytes.saturating_sub(prior) + member.bytes();
        book.members.insert(unit.unit_id, member);
        if allowed {
            observed
        } else {
            DualSupportContact::Unresolved
        }
    }
    pub(super) fn public_throat(
        &mut self,
        phase: &super::super::TerrainPhase,
        unit: &MovementFrameUnit,
        goal: Vec2,
        radius: f32,
        profile: Traversal,
        charge: &mut (impl FnMut() -> bool + ?Sized),
    ) -> Option<super::super::traffic::public_passage::Throat> {
        if !self.contact_ready(unit.unit_id, unit.owner_slot, phase) {
            return None;
        }
        let terrain = Planning {
            phase,
            viewer: unit.owner_slot,
        };
        let mut rows = BTreeMap::new();
        let witness = super::geometry::public_throat(
            &terrain,
            unit.position,
            goal,
            radius,
            profile,
            &mut rows,
            charge,
        )?;
        let book = &mut self.viewers[usize::from(unit.owner_slot)];
        let mut member = book.members.remove(&unit.unit_id)?;
        let prior = member.bytes();
        let mut added = 0;
        let mut coherent = true;
        for (&key, &revision) in &rows {
            match member.contact.binary_search_by_key(&key, |row| row.0) {
                Ok(index) => coherent &= member.contact[index].1 == revision,
                Err(_) => added += 1,
            }
        }
        let length = member.contact.len() + added;
        let extra = length.saturating_sub(member.contact.capacity()) * size_of::<(usize, u64)>();
        let allowed = coherent
            && length <= 4096
            && extra <= book.available()
            && (!book.retiring() || extra == 0);
        if allowed {
            member
                .contact
                .reserve_exact(length.saturating_sub(member.contact.len()));
            for (key, revision) in rows {
                if let Err(index) = member.contact.binary_search_by_key(&key, |row| row.0) {
                    member.contact.insert(index, (key, revision));
                }
            }
            member.contact_epoch = phase.planning_epoch(unit.owner_slot);
            member.contact_validation_epoch = member.contact_epoch;
            member.contact_cursor = 0;
        }
        book.member_buffer_bytes = book.member_buffer_bytes.saturating_sub(prior) + member.bytes();
        book.members.insert(unit.unit_id, member);
        allowed.then_some(witness)
    }
    pub(crate) fn allows_traffic(&self, unit: u32, viewer: u8) -> bool {
        self.viewers
            .get(usize::from(viewer))
            .and_then(|book| book.members.get(&unit))
            .is_some_and(|member| {
                member.viewer == viewer
                    && !member.cancelled
                    && !member.owner_fence
                    && !member.owner_released
            })
    }
    fn refresh(&mut self) {
        let mut d = NavigationDiagnostics::default();
        for viewer in &self.viewers {
            let s = viewer.diagnostics;
            d.searches = d.searches.saturating_add(s.searches);
            d.cache_hits = d.cache_hits.saturating_add(s.cache_hits);
            d.direct_query_cache_hits = d
                .direct_query_cache_hits
                .saturating_add(s.direct_query_cache_hits);
            d.direct_query_cache_misses = d
                .direct_query_cache_misses
                .saturating_add(s.direct_query_cache_misses);
            d.invalidations = d.invalidations.saturating_add(s.invalidations);
            d.budget_waits = d.budget_waits.saturating_add(s.budget_waits);
            d.resident_fields += s.resident_fields;
            d.resident_bytes += s.resident_bytes;
            d.retired_bytes += s.retired_bytes;
            d.retired_pages += s.retired_pages;
            d.memberships += s.memberships;
            d.pending += s.pending;
            d.expansions_this_tick += s.expansions_this_tick;
            d.edge_checks_this_tick += s.edge_checks_this_tick;
            d.direct_query_work_this_tick += s.direct_query_work_this_tick;
            d.direct_query_deferrals = d
                .direct_query_deferrals
                .saturating_add(s.direct_query_deferrals);
            d.admission_entries_this_tick += s.admission_entries_this_tick;
            d.invalidation_entries_this_tick += s.invalidation_entries_this_tick;
            d.retired_pages_this_tick += s.retired_pages_this_tick;
            d.ranking_comparisons_this_tick += s.ranking_comparisons_this_tick;
            d.frame_rows_this_tick += s.frame_rows_this_tick;
            d.max_pending_age_ticks = d.max_pending_age_ticks.max(s.max_pending_age_ticks);
            d.stale_activations = d.stale_activations.saturating_add(s.stale_activations);
        }
        d.resident_bytes += 1024 + self.traffic_bytes;
        d.solver_retries = self.diagnostics.solver_retries;
        d.body_index_queries_this_tick = self.diagnostics.body_index_queries_this_tick;
        d.body_index_rank_comparisons_this_tick =
            self.diagnostics.body_index_rank_comparisons_this_tick;
        d.body_index_tail_entries_this_tick = self.diagnostics.body_index_tail_entries_this_tick;
        d.body_index_word_ops_this_tick = self.diagnostics.body_index_word_ops_this_tick;
        d.body_index_build_comparisons_this_tick =
            self.diagnostics.body_index_build_comparisons_this_tick;
        d.body_index_build_prefix_words_this_tick =
            self.diagnostics.body_index_build_prefix_words_this_tick;
        d.body_index_build_mask_words_this_tick =
            self.diagnostics.body_index_build_mask_words_this_tick;
        d.body_envelope_filters_this_tick = self.diagnostics.body_envelope_filters_this_tick;
        d.body_index_rebuilds_this_tick = self.diagnostics.body_index_rebuilds_this_tick;
        d.body_index_updates_this_tick = self.diagnostics.body_index_updates_this_tick;
        d.body_index_update_entries_this_tick =
            self.diagnostics.body_index_update_entries_this_tick;
        d.body_support_queries_this_tick = self.diagnostics.body_support_queries_this_tick;
        d.escape_keep_scans_this_tick = self.diagnostics.escape_keep_scans_this_tick;
        d.packed_stop_visits_this_tick = self.diagnostics.packed_stop_visits_this_tick;
        d.packed_stops = self.diagnostics.packed_stops;
        d.packed_steps = self.diagnostics.packed_steps;
        d.body_support_pairs_this_tick = self.diagnostics.body_support_pairs_this_tick;
        d.body_support_failures_this_tick = self.diagnostics.body_support_failures_this_tick;
        d.body_support_accepted_members_this_tick =
            self.diagnostics.body_support_accepted_members_this_tick;
        d.model_attempts_this_tick = self.diagnostics.model_attempts_this_tick;
        d.model_operations_this_tick = self.diagnostics.model_operations_this_tick;
        d.model_fallbacks_this_tick = self.diagnostics.model_fallbacks_this_tick;
        d.model_view_refusals_this_tick = self.diagnostics.model_view_refusals_this_tick;
        d.coherence_attempts_this_tick = self.diagnostics.coherence_attempts_this_tick;
        d.coherence_advice_this_tick = self.diagnostics.coherence_advice_this_tick;
        d.coherence_members_this_tick = self.diagnostics.coherence_members_this_tick;
        d.coherence_operations_this_tick = self.diagnostics.coherence_operations_this_tick;
        d.body_pair_checks = self.diagnostics.body_pair_checks;
        d.body_pair_checks_this_tick = self.diagnostics.body_pair_checks_this_tick;
        d.neighbor_visits = self.diagnostics.neighbor_visits;
        d.neighbor_overflows = self.diagnostics.neighbor_overflows;
        d.neighbor_visits_this_tick = self.diagnostics.neighbor_visits_this_tick;
        d.neighbor_overflows_this_tick = self.diagnostics.neighbor_overflows_this_tick;
        d.neighbor_bucket_visits = self.diagnostics.neighbor_bucket_visits;
        d.neighbor_bucket_visits_this_tick = self.diagnostics.neighbor_bucket_visits_this_tick;
        d.traffic_index_queries = self.diagnostics.traffic_index_queries;
        d.traffic_index_queries_this_tick = self.diagnostics.traffic_index_queries_this_tick;
        d.traffic_index_rank_comparisons = self.diagnostics.traffic_index_rank_comparisons;
        d.traffic_index_rank_comparisons_this_tick =
            self.diagnostics.traffic_index_rank_comparisons_this_tick;
        d.traffic_index_tail_entries = self.diagnostics.traffic_index_tail_entries;
        d.traffic_index_tail_entries_this_tick =
            self.diagnostics.traffic_index_tail_entries_this_tick;
        d.traffic_index_word_ops = self.diagnostics.traffic_index_word_ops;
        d.traffic_index_word_ops_this_tick = self.diagnostics.traffic_index_word_ops_this_tick;
        d.traffic_index_build_comparisons = self.diagnostics.traffic_index_build_comparisons;
        d.traffic_index_build_comparisons_this_tick =
            self.diagnostics.traffic_index_build_comparisons_this_tick;
        d.traffic_index_build_prefix_words = self.diagnostics.traffic_index_build_prefix_words;
        d.traffic_index_build_prefix_words_this_tick =
            self.diagnostics.traffic_index_build_prefix_words_this_tick;
        d.traffic_index_build_mask_words = self.diagnostics.traffic_index_build_mask_words;
        d.traffic_index_build_mask_words_this_tick =
            self.diagnostics.traffic_index_build_mask_words_this_tick;
        d.traffic_index_query_rank_bound = self.diagnostics.traffic_index_query_rank_bound;
        d.traffic_index_query_tail_bound = self.diagnostics.traffic_index_query_tail_bound;
        d.traffic_index_query_word_bound = self.diagnostics.traffic_index_query_word_bound;
        d.local_detour_filter_rows_this_tick = self.diagnostics.local_detour_filter_rows_this_tick;
        d.local_detour_scanned_rows_this_tick =
            self.diagnostics.local_detour_scanned_rows_this_tick;
        d.local_detour_generator_comparisons_this_tick = self
            .diagnostics
            .local_detour_generator_comparisons_this_tick;
        d.local_detour_candidates_this_tick = self.diagnostics.local_detour_candidates_this_tick;
        d.local_detour_body_checks_this_tick = self.diagnostics.local_detour_body_checks_this_tick;
        d.local_detour_terrain_calls_this_tick =
            self.diagnostics.local_detour_terrain_calls_this_tick;
        d.traffic_members = self.diagnostics.traffic_members;
        d.traffic_gates = self.diagnostics.traffic_gates;
        d.traffic_capacity_waits = self.diagnostics.traffic_capacity_waits;
        self.diagnostics = d;
    }
    pub(super) fn record_traffic(&mut self, traffic: &TrafficState) {
        self.traffic_bytes = traffic
            .retained_bytes()
            .saturating_add(traffic.scratch_bytes());
        self.diagnostics.traffic_index_queries = traffic.index_work_total.queries;
        self.diagnostics.traffic_index_queries_this_tick = traffic.index_work.queries;
        self.diagnostics.traffic_index_rank_comparisons = traffic.index_work_total.rank_comparisons;
        self.diagnostics.traffic_index_rank_comparisons_this_tick =
            traffic.index_work.rank_comparisons;
        self.diagnostics.traffic_index_tail_entries = traffic.index_work_total.tail_entries;
        self.diagnostics.traffic_index_tail_entries_this_tick = traffic.index_work.tail_entries;
        self.diagnostics.traffic_index_word_ops = traffic.index_work_total.word_ops;
        self.diagnostics.traffic_index_word_ops_this_tick = traffic.index_work.word_ops;
        self.diagnostics.traffic_index_build_comparisons =
            traffic.index_work_total.build_comparisons;
        self.diagnostics.traffic_index_build_comparisons_this_tick =
            traffic.index_work.build_comparisons;
        self.diagnostics.traffic_index_build_prefix_words =
            traffic.index_work_total.build_prefix_words;
        self.diagnostics.traffic_index_build_prefix_words_this_tick =
            traffic.index_work.build_prefix_words;
        self.diagnostics.traffic_index_build_mask_words = traffic.index_work_total.build_mask_words;
        self.diagnostics.traffic_index_build_mask_words_this_tick =
            traffic.index_work.build_mask_words;
        let (rank, tail, word) = TrafficState::index_query_bounds();
        self.diagnostics.traffic_index_query_rank_bound = rank;
        self.diagnostics.traffic_index_query_tail_bound = tail;
        self.diagnostics.traffic_index_query_word_bound = word;
        let (scanned_rows, generator_comparisons, candidates, body_checks, terrain_calls) =
            traffic.detour_counters();
        self.diagnostics.local_detour_filter_rows_this_tick = traffic.detour_filter_rows;
        self.diagnostics.local_detour_scanned_rows_this_tick = scanned_rows;
        self.diagnostics
            .local_detour_generator_comparisons_this_tick = generator_comparisons;
        self.diagnostics.local_detour_candidates_this_tick = candidates;
        self.diagnostics.local_detour_body_checks_this_tick = body_checks;
        self.diagnostics.local_detour_terrain_calls_this_tick = terrain_calls;
        self.diagnostics.body_pair_checks = traffic.body_pair_checks;
        self.diagnostics.body_pair_checks_this_tick = traffic.tick_body_pair_checks;
        let (bodies, buckets) = traffic.tick_neighbor_work();
        let overflows = traffic
            .neighbor_overflows
            .checked_sub(self.diagnostics.neighbor_overflows)
            .unwrap_or(traffic.neighbor_overflows);
        self.diagnostics.neighbor_visits = traffic.neighbor_visits;
        self.diagnostics.neighbor_overflows = traffic.neighbor_overflows;
        self.diagnostics.neighbor_bucket_visits = traffic.neighbor_bucket_visits;
        self.diagnostics.neighbor_visits_this_tick = bodies;
        self.diagnostics.neighbor_bucket_visits_this_tick = buckets;
        self.diagnostics.neighbor_overflows_this_tick =
            usize::try_from(overflows).unwrap_or(usize::MAX);
        self.diagnostics.traffic_members = traffic.retained_members();
        self.diagnostics.traffic_gates = traffic.retained_gates();
        self.diagnostics.traffic_capacity_waits = self
            .diagnostics
            .traffic_capacity_waits
            .saturating_add(u64::from(traffic.capacity_blocked));
        self.refresh();
    }
    pub(super) fn record_body_index(&mut self, budget: &crate::movement::sweep::SweepBudget) {
        self.diagnostics.body_support_queries_this_tick = budget.support_queries;
        self.diagnostics.escape_keep_scans_this_tick = budget.escape_keep_scans;
        self.diagnostics.packed_stop_visits_this_tick = budget.packed_stop_visits;
        self.diagnostics.packed_stops = self
            .diagnostics
            .packed_stops
            .saturating_add(budget.packed_stops);
        self.diagnostics.packed_steps = self
            .diagnostics
            .packed_steps
            .saturating_add(budget.packed_steps);
        self.diagnostics.body_support_pairs_this_tick = budget.support_pairs;
        self.diagnostics.body_support_failures_this_tick = budget.support_failures;
        self.diagnostics.body_support_accepted_members_this_tick = budget.support_accepted_members;
        self.diagnostics.model_attempts_this_tick = budget.model_attempts;
        self.diagnostics.model_operations_this_tick = budget.model_operations;
        self.diagnostics.model_fallbacks_this_tick = budget.model_fallbacks;
        self.diagnostics.model_view_refusals_this_tick = budget.model_view_refusals;
        self.diagnostics.coherence_attempts_this_tick = budget.coherence_attempts;
        self.diagnostics.coherence_advice_this_tick = budget.coherence_advice;
        self.diagnostics.coherence_members_this_tick = budget.coherence_members;
        self.diagnostics.coherence_operations_this_tick = budget.coherence_operations;
        self.diagnostics.body_index_queries_this_tick = budget.index_work.queries;
        self.diagnostics.body_index_rank_comparisons_this_tick = budget.index_work.rank_comparisons;
        self.diagnostics.body_index_tail_entries_this_tick = budget.index_work.tail_entries;
        self.diagnostics.body_index_word_ops_this_tick = budget.index_work.word_ops;
        self.diagnostics.body_index_build_comparisons_this_tick =
            budget.index_work.build_comparisons;
        self.diagnostics.body_index_build_prefix_words_this_tick =
            budget.index_work.build_prefix_words;
        self.diagnostics.body_index_build_mask_words_this_tick = budget.index_work.build_mask_words;
        self.diagnostics.body_envelope_filters_this_tick = budget.envelope_filters;
        self.diagnostics.body_index_rebuilds_this_tick = budget.index_rebuilds;
        self.diagnostics.body_index_updates_this_tick = budget.index_updates;
        self.diagnostics.body_index_update_entries_this_tick = budget.index_update_entries;
        let recovery = &budget.recovery;
        self.diagnostics.recovery_work_this_tick = recovery.peak_owner_work() as u64;
        self.diagnostics.recovery_actual_work_this_tick = recovery.peak_owner_actual() as u64;
        self.diagnostics.recovery_candidates_this_tick = recovery.candidates;
        self.diagnostics.recovery_visibility_checks_this_tick = recovery.visibility_checks;
        self.diagnostics.recovery_digests_this_tick = recovery.digests;
        self.diagnostics.recovery_invalidations_this_tick = recovery.invalidations;
        self.diagnostics.recovery_relaxed_commits_this_tick = recovery.relaxed_commits;
        self.diagnostics.recovery_pool_work_this_tick = recovery.pool_work;
        self.diagnostics.recovery_spare_slot_work_this_tick = recovery.spare_slot_work;
        self.diagnostics.recovery_physical_reads_this_tick = recovery.physical_reads;
        self.diagnostics.recovery_physical_work_this_tick = recovery.physical_work;
        self.diagnostics.recovery_view_work_this_tick = recovery.view_work;
        self.diagnostics.recovery_jobs = recovery.jobs;
        self.diagnostics.recovery_max_age = u64::from(recovery.max_age);
        self.diagnostics.recovery_retained_bytes = recovery.retained_bytes as u64;
        self.diagnostics.recovery_phase_bytes = recovery.phase_bytes as u64;
    }
    pub(super) fn parking_region(
        &self,
        unit: &MovementFrameUnit,
        goal: Vec2,
        profile: Traversal,
    ) -> Option<DestinationRegion> {
        let member = self
            .viewers
            .get(usize::from(unit.owner_slot))?
            .members
            .get(&unit.unit_id)?;
        if member.cancelled
            || member.owner_fence
            || member.owner_released
            || !member.live_target
            || member.viewer != unit.owner_slot
        {
            return None;
        }
        let scope = member.parking_scope?;
        (scope.matches(unit, profile, member.epoch) && scope.region.goal == goal)
            .then_some(scope.region)
    }
    pub(super) fn record_dispatch_scopes(
        &mut self,
        owner: u8,
        region: DestinationRegion,
        shapes: &[(u32, u32, f32, Traversal)],
        tick: u32,
    ) {
        let Some(book) = self.viewers.get_mut(usize::from(owner)) else {
            return;
        };
        book.begin(tick);
        let prior_cancellation_overflow = book.cancellation_overflow;
        if shapes.len() > MAX_MEMBERS || prior_cancellation_overflow {
            return;
        }
        let mut created = Vec::with_capacity(shapes.len());
        let mut complete = true;
        for &(id, _, _, _) in shapes {
            if !book.members.contains_key(&id) {
                book.cancel(id, tick, Some(owner));
                if !book.members.contains_key(&id) {
                    complete = false;
                    break;
                }
                created.push(id);
            }
        }
        complete &= !book.cancellation_overflow
            && shapes.iter().all(|&(id, _, _, _)| {
                book.members.get(&id).is_some_and(|m| {
                    m.cancelled && !m.owner_fence && !m.owner_released && m.viewer == owner
                })
            });
        if !complete {
            // No partial cohort can gain destination authority under admission
            // pressure. Reverse reclamation leaves every old member slot intact.
            for id in created.into_iter().rev() {
                book.reclaim(id);
            }
            // This synchronous optional batch alone raised any new overflow.
            // Keep genuine prior cancellation and every consumed work charge;
            // a scope admission miss must not cancel unrelated accepted orders.
            book.cancellation_overflow = prior_cancellation_overflow;
            return;
        }
        for &(id, kind, radius, profile) in shapes {
            let member = book.members.get_mut(&id).unwrap();
            member.parking_scope = Some(ParkingScope {
                region,
                epoch: member.epoch,
                radius: radius.to_bits(),
                kind,
                profile,
            });
        }
    }
    pub(super) fn record_arrival_slot(&mut self, unit: u32, viewer: u8, slot: Vec2) {
        if let Some(member) = self
            .viewers
            .get_mut(usize::from(viewer))
            .and_then(|book| book.members.get_mut(&unit))
            .filter(|member| !member.cancelled && !member.owner_fence && !member.owner_released)
        {
            member.arrival_slot = Some(slot);
        }
    }
    /// Arrive first, keep the point. A clear body on the direct final leg
    /// that current bodies hold back stops where it stands; see `HeldLeg`.
    pub(super) fn held_final_leg(
        &mut self,
        unit: u32,
        viewer: u8,
        held: Option<f32>,
        radius: f32,
        limit: u16,
    ) -> bool {
        self.viewers
            .get_mut(usize::from(viewer))
            .and_then(|book| book.members.get_mut(&unit))
            .filter(|member| !member.cancelled && !member.owner_fence && !member.owner_released)
            .is_some_and(|member| member.held_leg.observe(held, radius, limit))
    }
    pub(super) fn arrival_slot(&self, unit: u32, viewer: u8) -> Option<Vec2> {
        self.viewers
            .get(usize::from(viewer))
            .and_then(|book| book.members.get(&unit))
            .filter(|member| !member.cancelled && !member.owner_fence && !member.owner_released)
            .and_then(|member| member.arrival_slot)
    }
    pub(super) fn completed(&mut self, unit: u32) {
        for viewer in &mut self.viewers {
            viewer.completed(unit);
        }
    }
    pub(super) fn finish_tick(&mut self) {
        for viewer in &mut self.viewers {
            viewer.update_diagnostics();
        }
        self.refresh();
    }
    pub(super) fn propose(
        &mut self,
        world: &World,
        frame: &MovementFrame,
        phase: &super::super::TerrainPhase,
        profiles: &[Option<Traversal>],
        candidate: &mut MovementFrame,
        waiting: &mut BTreeSet<u32>,
    ) {
        let tick = world.get_resource::<crate::Tick>().map_or(0, |tick| tick.0);
        let mut fenced = BTreeSet::new();
        for (index, unit) in frame.units.iter().enumerate() {
            if usize::from(unit.owner_slot) >= VIEWERS && unit.target.is_some() {
                candidate.units[index].speed = None;
                candidate.units[index].target = None;
                waiting.insert(unit.unit_id);
            }
        }
        // The authoritative frame is read for ownership before any viewer can admit a
        // retained intent. This check cannot learn or export geometry from either viewer.
        for viewer in &mut self.viewers {
            viewer.begin(tick);
            for unit in &frame.units {
                let Some(member) = viewer.members.get_mut(&unit.unit_id) else {
                    continue;
                };
                if member.viewer != unit.owner_slot && !member.owner_released {
                    member.owner_fence = true;
                    member.parking_scope = None;
                    let motion_bytes = member.motion.take().map_or(0, |m| m.bytes());
                    viewer.member_buffer_bytes =
                        viewer.member_buffer_bytes.saturating_sub(motion_bytes);
                    fenced.insert(unit.unit_id);
                    if !member.owner_reported && viewer.owner_changes.len() < MAX_MEMBERS {
                        viewer.owner_changes.push((unit.unit_id, member.viewer));
                        member.owner_reported = true;
                    }
                }
            }
        }
        for viewer in &mut self.viewers {
            viewer.propose(world, frame, phase, profiles, candidate, waiting, &fenced);
        }
        self.refresh();
    }
}
pub fn privileged_diagnostics(world: &World) -> NavigationDiagnostics {
    world
        .get_resource::<TerrainNavigation>()
        .map_or_else(NavigationDiagnostics::default, |n| n.diagnostics)
}
pub fn privileged_navigation_audit(world: &World) -> NavigationAudit {
    let Some(n) = world.get_resource::<TerrainNavigation>() else {
        return NavigationAudit {
            members: Vec::new(),
            diagnostics: Default::default(),
        };
    };
    let mut members = n
        .viewers
        .iter()
        .flat_map(|viewer| viewer.audit().members)
        .collect::<Vec<_>>();
    members.sort_by_key(|m| m.unit);
    NavigationAudit {
        members,
        diagnostics: n.diagnostics,
    }
}
pub(crate) fn order_changed(world: &mut World, unit: u32) {
    change_order(world, unit, true);
}
/// Actual deferred dispatch retires an old identity. Missing members are admitted
/// only by the following complete cohort batch, never by member enumeration.
pub(crate) fn dispatch_order_changed(world: &mut World, unit: u32) {
    change_order(world, unit, false);
}
fn change_order(world: &mut World, unit: u32, create: bool) {
    // Equal coordinates do not preserve a retired order's private route intent.
    if let Some(mut traffic) = world.get_resource_mut::<TrafficState>() {
        traffic.forget(unit);
    }
    if !world.contains_resource::<TerrainNavigation>() {
        if !create || !world.contains_resource::<super::super::TerrainState>() {
            return;
        }
        world.init_resource::<TerrainNavigation>();
    }
    let tick = world.get_resource::<crate::Tick>().map_or(0, |tick| tick.0);
    let owner = crate::unit_entity(world, unit)
        .and_then(|entity| world.get::<crate::Owner>(entity))
        .map(|owner| owner.0.0);
    if let Some(mut navigation) = world.get_resource_mut::<TerrainNavigation>() {
        for viewer in &mut navigation.viewers {
            if Some(viewer.viewer) == owner {
                if create || viewer.members.contains_key(&unit) {
                    viewer.cancel(unit, tick, owner);
                }
            } else if let Some(member) = viewer.members.get_mut(&unit) {
                member.cancelled = true;
                member.owner_released = true;
                member.parking_scope = None;
                member.owner_fence = false;
            }
        }
    }
}
pub(crate) fn take_owner_changes(world: &mut World) -> Vec<(u32, u8)> {
    let Some(mut n) = world.get_resource_mut::<TerrainNavigation>() else {
        return Vec::new();
    };
    let mut records = Vec::new();
    for viewer in &mut n.viewers {
        records.append(&mut viewer.owner_changes);
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use turts_rules::terrain::{
        Ratio,
        geometry::{Cell, Grid, Surface},
    };
    #[derive(Clone)]
    struct Terrain {
        grid: Grid,
        heights: Vec<i32>,
        revisions: Vec<u64>,
        epoch: u64,
    }
    impl Terrain {
        fn flat() -> Self {
            Self {
                grid: Grid::checked(8, 8, 1, 1, 1, -8, 8).unwrap(),
                heights: vec![0; 64],
                revisions: vec![0; 64],
                epoch: 0,
            }
        }
        fn index(&self, cell: Cell) -> Option<usize> {
            (cell.x < 8 && cell.y < 8).then_some((cell.y * 8 + cell.x) as usize)
        }
        fn edit(&mut self, cell: Cell, height: i32) {
            let i = self.index(cell).unwrap();
            self.heights[i] = height;
            self.revisions[i] += 1;
            self.epoch += 1;
        }
    }
    impl Surface for Terrain {
        fn elevation(&self, cell: Cell) -> Option<i32> {
            self.heights.get(self.index(cell)?).copied()
        }
    }
    impl TerrainRead for Terrain {
        fn grid(&self) -> Grid {
            self.grid
        }
        fn epoch(&self) -> u64 {
            self.epoch
        }
        fn lease_matches(&self, key: usize, revision: u64) -> bool {
            self.revisions.get(key) == Some(&revision)
        }
        fn valid_lease(&self, lease: &BTreeMap<usize, u64>) -> bool {
            lease.iter().all(|(&k, &v)| self.lease_matches(k, v))
        }
        fn rectangle(&self, min: Cell, max: Cell, lease: &mut BTreeMap<usize, u64>) -> bool {
            let Some(height) = self.elevation(min) else {
                return false;
            };
            let mut flat = true;
            for y in min.y..=max.y {
                for x in min.x..=max.x {
                    let Some(i) = self.index(Cell { x, y }) else {
                        return false;
                    };
                    lease.insert(i, self.revisions[i]);
                    flat &= self.heights[i] == height;
                }
            }
            flat
        }
    }
    fn traversal() -> Traversal {
        Traversal {
            max_step: 0,
            max_grade: Ratio {
                numerator: 0,
                denominator: 1,
            },
        }
    }
    fn field(terrain: &Terrain, radius: f32) -> Field {
        let mut field = Field::fresh(
            1,
            Base {
                viewer: 0,
                radius: radius.to_bits(),
                profile: traversal(),
                rx: 0,
                ry: 0,
            },
            Vertex { x: 13, y: 8 },
            Vec2::new(6.5, 4.0),
            1,
        );
        assert!(field.reset(terrain, 8 * 1024 * 1024));
        field
    }
    fn run(field: &mut Field, terrain: &Terrain, ticks: usize) -> NavigationDiagnostics {
        let mut total = NavigationDiagnostics::default();
        for _ in 0..ticks {
            let mut d = NavigationDiagnostics::default();
            for _ in 0..4096 {
                if !field.advance(terrain, &mut d, 8 * 1024 * 1024) {
                    break;
                }
            }
            assert!(d.expansions_this_tick <= 128);
            assert!(d.edge_checks_this_tick <= 1024);
            total.expansions_this_tick += d.expansions_this_tick;
            if field.stage == Stage::Exhausted {
                break;
            }
        }
        total
    }
    #[test]
    fn approach_refusal_requires_complete_current_permitted_graph() {
        let mut terrain = Terrain::flat();
        for y in 0..8 {
            for x in 4..8 {
                terrain.edit(Cell { x, y }, 2);
            }
        }
        let mut field = field(&terrain, 0.4);
        run(&mut field, &terrain, 100);
        assert!(field.stage == Stage::Exhausted);
        let mut member = Member::stamp(0, 0, 1, 0);
        member.cancelled = false;
        member.live_target = true;
        member.endpoint = Vec2::new(6.5, 4.0);
        member.valid_epoch = terrain.epoch();
        member.field_revision = field.revision;
        member.state = NavigationMemberState::Blocked;
        let from = Vec2::new(1.5, 4.0);
        let endpoint = member.endpoint;
        assert!(exhausted_approach(
            &member, &field, &terrain, from, endpoint
        ));
        // The optional direct cache can be absent. The completed permitted graph
        // is the positive proof; the returned fact only rejects this approach.
        assert!(member.direct_query.is_none());
        assert!(!exhausted_approach(
            &member,
            &field,
            &terrain,
            Vec2::new(5.5, 4.0),
            endpoint
        ));
        assert!(!exhausted_approach(
            &member,
            &field,
            &terrain,
            from,
            endpoint + Vec2::X
        ));
        for state in [
            NavigationMemberState::Pending,
            NavigationMemberState::Searching,
            NavigationMemberState::Ready,
        ] {
            member.state = state;
            assert!(!exhausted_approach(
                &member, &field, &terrain, from, endpoint
            ));
        }
        member.state = NavigationMemberState::Blocked;
        for mode in 0..4 {
            member.cancelled = mode == 0;
            member.owner_fence = mode == 1;
            member.owner_released = mode == 2;
            member.live_target = mode != 3;
            assert!(!exhausted_approach(
                &member, &field, &terrain, from, endpoint
            ));
        }
        member.cancelled = false;
        member.owner_fence = false;
        member.owner_released = false;
        member.live_target = true;
        member.viewer = 1;
        assert!(!exhausted_approach(
            &member, &field, &terrain, from, endpoint
        ));
        member.viewer = 0;
        field.resource_fault = true;
        assert!(!exhausted_approach(
            &member, &field, &terrain, from, endpoint
        ));
        field.resource_fault = false;
        field.unresolved_edges = true;
        assert!(!exhausted_approach(
            &member, &field, &terrain, from, endpoint
        ));
        field.unresolved_edges = false;
        field.stage = Stage::Pending;
        assert!(!exhausted_approach(
            &member, &field, &terrain, from, endpoint
        ));
        field.stage = Stage::Exhausted;
        field.revision += 1;
        assert!(!exhausted_approach(
            &member, &field, &terrain, from, endpoint
        ));
        field.revision -= 1;
        terrain.edit(Cell { x: 4, y: 4 }, 0);
        assert!(!exhausted_approach(
            &member, &field, &terrain, from, endpoint
        ));
        member.valid_epoch = terrain.epoch();
        assert!(
            !exhausted_approach(&member, &field, &terrain, from, endpoint),
            "field leases must also be current"
        );
    }
    #[test]
    fn reverse_field_retains_exact_half_cell_tangent_connections() {
        let mut t = Terrain::flat();
        for y in 0..8 {
            if !(3..5).contains(&y) {
                for x in 3..5 {
                    t.edit(Cell { x, y }, 4);
                }
            }
        }
        let mut exact = field(&t, 1.0);
        run(&mut exact, &t, 100);
        let approach = Vertex { x: 3, y: 8 };
        assert!(exact.node(&t, approach).is_some_and(|n| n.closed));
        let mut larger = field(&t, f32::from_bits(1.0f32.to_bits() + 1));
        run(&mut larger, &t, 100);
        assert!(!larger.node(&t, approach).is_some_and(|n| n.closed));
    }
    #[test]
    fn direct_dependencies_survive_distant_edit_and_reject_local_edit() {
        let mut t = Terrain::flat();
        let mut lease = BTreeMap::new();
        assert_eq!(
            certify(
                &t,
                Vec2::new(1.5, 1.5),
                Vec2::new(5.5, 1.5),
                0.4,
                traversal(),
                &mut lease
            ),
            Certificate::Clear
        );
        t.edit(Cell { x: 7, y: 7 }, 4);
        assert!(t.valid_lease(&lease));
        t.edit(Cell { x: 3, y: 1 }, 4);
        assert!(!t.valid_lease(&lease));
    }
    #[test]
    fn field_reset_certifies_seed_again_before_any_expansion() {
        let mut t = Terrain::flat();
        let mut f = field(&t, 0.4);
        run(&mut f, &t, 100);
        let old = f.revision;
        t.edit(Cell { x: 6, y: 4 }, 4);
        t.edit(Cell { x: 7, y: 4 }, 0);
        let mut d = NavigationDiagnostics::default();
        assert!(f.validate(&t, &mut d, 8 * 1024 * 1024));
        assert_eq!(f.revision, old + 1);
        // A radius crossing a four-step edge cannot reuse its previously legal terminal.
        f.seed = Vertex { x: 12, y: 8 };
        assert!(f.reset(&t, 8 * 1024 * 1024));
        assert!(f.advance(&t, &mut d, 8 * 1024 * 1024));
        assert!(f.stage == Stage::Blocked);
        assert_eq!(d.expansions_this_tick, 0);
    }
    #[test]
    fn exact_radius_and_viewer_are_part_of_shared_field_key() {
        let t = Terrain::flat();
        let a = field(&t, 1.0);
        let b = field(&t, f32::from_bits(1.0f32.to_bits() + 1));
        assert!(a.base != b.base);
        let mut other = a.base;
        other.viewer = 1;
        assert!(other != a.base);
        assert_eq!(
            connectors(&t, Vec2::new(-0.0, 4.0)),
            connectors(&t, Vec2::new(0.0, 4.0))
        );
    }
    #[test]
    fn held_leg_does_not_complete_far_from_an_unoccupied_goal() {
        let mut clock = HeldLeg::default();
        let goal = Vec2::new(10., 10.);
        let actor = MovementFrameUnit {
            unit_id: 7,
            owner_slot: 0,
            unit_type: 0,
            position: Vec2::new(30., 10.),
            radius: 1.,
            speed: Some(1.),
            target: Some(goal),
            progress: None,
            plain_progress: None,
            physical_escape: None,
        };
        let mut frame = crate::movement::MovementFrame {
            recovery: Default::default(),
            bounds: Vec2::splat(64.),
            tick_hz: 20,
            units: vec![actor.clone()],
            pools: Default::default(),
        };
        let mut expired = 0;
        // The direct route is temporarily blocked, but the destination is free.
        for _ in 0..120 {
            let timed_out = clock.observe(Some(20.), 1.0, 60);
            expired += usize::from(timed_out);
            assert!(
                !(timed_out
                    && super::super::held_position_is_nearest(
                        &frame,
                        &actor,
                        actor.position,
                        goal
                    ))
            );
        }
        assert_eq!(expired, 2, "the held-leg timeout branch must be exercised");

        // An own settled body at the goal proves a lower distance bound. A far
        // pose does not meet it, but a collision-safe tangent pose does.
        let mut holder = actor.clone();
        holder.unit_id = 9;
        holder.position = goal;
        holder.target = None;
        holder.speed = None;
        frame.units.push(holder);
        assert!(!super::super::held_position_is_nearest(
            &frame,
            &actor,
            actor.position,
            goal
        ));
        assert!(super::super::held_position_is_nearest(
            &frame,
            &actor,
            Vec2::new(12., 10.),
            goal,
        ));
        frame.units[1].target = Some(Vec2::new(20., 10.));
        assert!(!super::super::held_position_is_nearest(
            &frame,
            &actor,
            Vec2::new(12., 10.),
            goal,
        ));
        frame.units[1].target = None;
        frame.units[1].owner_slot = 1;
        assert!(!super::super::held_position_is_nearest(
            &frame,
            &actor,
            Vec2::new(12., 10.),
            goal,
        ));
    }
    #[test]
    fn held_leg_clock_counts_only_consecutive_held_ticks_without_real_approach() {
        let mut clock = HeldLeg::default();
        // Fifty-nine held ticks at one distance do not end the move.
        for _ in 0..59 {
            assert!(!clock.observe(Some(10.), 1.1, 60));
        }
        // Millimetre creep is not an approach; the sixtieth tick ends it.
        assert!(clock.observe(Some(9.999), 1.1, 60));
        assert_eq!(clock, HeldLeg::default());
        // A tick that is not held on the final leg clears the clock.
        for _ in 0..59 {
            assert!(!clock.observe(Some(10.), 1.1, 60));
        }
        assert!(!clock.observe(None, 1.1, 60));
        for _ in 0..59 {
            assert!(!clock.observe(Some(10.), 1.1, 60));
        }
        // An approach of more than a quarter radius restarts it.
        assert!(!clock.observe(Some(9.7), 1.1, 60));
        for _ in 0..58 {
            assert!(!clock.observe(Some(9.7), 1.1, 60));
        }
        assert!(clock.observe(Some(9.7), 1.1, 60));
        // A retreat does not restart it, and a bad distance clears it.
        assert!(!clock.observe(Some(5.), 1.1, 3));
        assert!(!clock.observe(Some(6.), 1.1, 3));
        assert!(clock.observe(Some(7.), 1.1, 3));
        assert!(!clock.observe(Some(f32::NAN), 1.1, 1));
    }
    #[test]
    fn cancellation_changes_identity_even_for_identical_endpoint() {
        let mut n = ViewerNavigation::default();
        n.begin(5);
        n.next_epoch = 1;
        n.member_ids.push(99);
        n.members.insert(
            99,
            Member {
                motion: None,
                parking_scope: None,
                viewer: 2,
                endpoint: Vec2::new(4.0, 4.0),
                arrival_slot: Some(Vec2::new(3.5, 4.0)),
                held_leg: HeldLeg::default(),
                radius: 0.4f32.to_bits(),
                profile: traversal(),
                epoch: 1,
                accepted: 1,
                serviced: 1,
                seen: 5,
                live_target: true,
                slot: 0,
                cancelled: false,
                owner_fence: false,
                owner_reported: false,
                owner_released: false,
                field: None,
                field_revision: 1,
                seed: None,
                connector: None,
                connector_cursor: 0,
                direct: Vec::new(),
                direct_query: None,
                direct_refusal: None,
                direct_turn: None,
                contact: Vec::new(),
                contact_epoch: 0,
                contact_validation_epoch: 0,
                contact_cursor: 0,
                contact_invalidated: false,
                valid_epoch: 0,
                validation_epoch: 0,
                validation_cursor: 0,
                state: NavigationMemberState::Direct,
            },
        );
        n.cancel(99, 5, None);
        assert!(n.audit().members.is_empty());
        assert_eq!(n.members[&99].epoch, 2);
        assert!(n.members[&99].cancelled);
        assert_eq!(n.members[&99].arrival_slot, None);
    }
    #[test]
    fn retirement_counts_each_page_and_includes_deferred_storage() {
        let t = Terrain::flat();
        let mut f = field(&t, 0.4);
        run(&mut f, &t, 100);
        let mut n = ViewerNavigation::default();
        n.retired.push(Box::new(f));
        let before = n.retired_bytes();
        assert!(before > PAGE_BYTES);
        n.begin(1);
        n.update_diagnostics();
        assert!(n.diagnostics.retired_pages_this_tick <= 16);
        assert!(n.retired_bytes() < before);
        assert!(n.diagnostics.resident_bytes + n.diagnostics.retired_bytes <= MAX_BYTES);
    }
    #[test]
    fn capacity_eviction_yields_blocked_fields_without_handle_tie_breaks() {
        let t = Terrain::flat();
        let mut n = ViewerNavigation::default();
        for id in [900, 2] {
            let mut f = field(&t, 0.4);
            f.id = id;
            f.stage = Stage::Exhausted;
            f.last_reachable = 1;
            n.fields.push(Box::new(f));
        }
        let mut live = field(&t, 0.4);
        live.id = 77;
        live.stage = Stage::Reverse;
        live.last_reachable = 1;
        n.fields.push(Box::new(live));
        n.evict_unproductive(40);
        assert_eq!(n.fields.len(), 1);
        assert_eq!(n.fields[0].id, 77);
        assert_eq!(n.retired.len(), 2);
        assert!(n.retired_bytes() > 0);
    }
    #[test]
    fn pending_stamp_keeps_accepted_tick_before_admission_and_releases_owner_fence() {
        let mut n = ViewerNavigation::default();
        n.cancel(31, 0, Some(0));
        assert_eq!(n.members[&31].accepted, 0);
        assert_eq!(n.members[&31].epoch, 1);
        n.members.get_mut(&31).unwrap().owner_fence = true;
        n.cancel(31, 17, Some(0));
        assert_eq!(n.members[&31].viewer, 0);
        assert_eq!(n.members[&31].accepted, 17);
        assert_eq!(n.members[&31].epoch, 2);
        assert!(!n.members[&31].owner_fence);
    }
    #[test]
    fn private_search_pressure_cannot_borrow_protected_memory_or_work_quota() {
        let t = Terrain::flat();
        let mut a = TerrainNavigation::default();
        let mut b = TerrainNavigation::default();
        let protected_field = |id| {
            let mut f = field(&t, 0.4);
            f.id = id;
            f.base.viewer = 0;
            Box::new(f)
        };
        a.viewers[0].fields.push(protected_field(2));
        b.viewers[0].fields.push(protected_field(2));
        for navigation in [&mut a, &mut b] {
            let mut member = Member::stamp(0, 0, 1, 0);
            member.cancelled = false;
            member.endpoint = Vec2::new(6.5, 4.0);
            member.radius = 0.4f32.to_bits();
            member.field = Some(2);
            member.state = NavigationMemberState::Searching;
            navigation.viewers[0].member_ids.push(10);
            navigation.viewers[0].members.insert(10, member);
            navigation.viewers[0].fields[0].refs = 1;
        }
        // Only the second world's foreign viewer owns extra retained private searches.
        for id in 0..MAX_FIELDS {
            let mut f = field(&t, 0.4);
            f.id = (id as u64 + 1) * VIEWERS as u64 + 1;
            f.base.viewer = 1;
            b.viewers[1].fields.push(Box::new(f));
        }
        assert_eq!(a.viewers[0].available(), b.viewers[0].available());
        // Drive the actual bounded Field predicate against equal protected geometry.
        // Each viewer has independent counters; unused foreign work is never transferred.
        for tick in 1..=4 {
            for navigation in [&mut a, &mut b] {
                for viewer in &mut navigation.viewers {
                    viewer.begin(tick);
                }
                for viewer in &mut navigation.viewers {
                    viewer.advance_fields_with(tick, |_| t.clone());
                    viewer.update_diagnostics();
                }
                navigation.refresh();
                assert!(navigation.diagnostics.expansions_this_tick <= 128);
                assert!(navigation.diagnostics.edge_checks_this_tick <= 1024);
                assert!(
                    navigation.diagnostics.resident_bytes + navigation.diagnostics.retired_bytes
                        <= 64 * 1024 * 1024
                );
            }
            let left = &a.viewers[0];
            let right = &b.viewers[0];
            assert_eq!(left.fields[0].leases.rows(), right.fields[0].leases.rows());
            assert_eq!(
                left.fields[0].frontier.len(),
                right.fields[0].frontier.len()
            );
            assert_eq!(
                left.diagnostics.expansions_this_tick,
                right.diagnostics.expansions_this_tick
            );
            assert_eq!(
                left.diagnostics.edge_checks_this_tick,
                right.diagnostics.edge_checks_this_tick
            );
            assert_eq!(left.available(), right.available());
            assert_eq!(left.audit().members, right.audit().members);
        }
    }
    #[test]
    fn traffic_accepts_admitted_waiters_and_refuses_stamps_and_owner_fences() {
        let mut n = TerrainNavigation::default();
        let member = Member::stamp(0, 0, 1, 0);
        n.viewers[0].members.insert(11, member);
        assert!(!n.allows_traffic(11, 0));
        let member = n.viewers[0].members.get_mut(&11).unwrap();
        member.cancelled = false;
        member.state = NavigationMemberState::Searching;
        assert!(n.allows_traffic(11, 0));
        assert!(!n.allows_traffic(11, 1));
        n.viewers[0].members.get_mut(&11).unwrap().owner_fence = true;
        assert!(!n.allows_traffic(11, 0));
    }
    #[test]
    fn member_retirement_pressure_holds_new_admission_until_whole_backlog_drains() {
        let mut n = ViewerNavigation::default();
        n.begin(1);
        for _ in 0..MAX_RETIRED_PAGES + 3 {
            let mut buffer = Vec::with_capacity(4096);
            buffer.resize(4096, (0, 0));
            n.member_buffer_bytes += buffer.capacity() * size_of::<(usize, u64)>();
            n.retire_member_buffer(buffer);
        }
        assert!(n.retiring());
        assert_eq!(n.retired_members.len(), 3);
        n.update_diagnostics();
        assert_eq!(n.diagnostics.retired_pages_this_tick, MAX_RETIRED_PAGES);
        let mut f = field(&Terrain::flat(), 0.4);
        f.refs = 1;
        n.fields.push(Box::new(f));
        assert_eq!(n.field_limit(0), n.fields[0].bytes());
        n.begin(2);
        assert!(!n.retiring());
        assert_eq!(n.member_buffer_bytes, 0);
    }
    #[test]
    fn contact_leases_validate_locally_without_flushing_route_dependencies() {
        let mut terrain = Terrain::flat();
        let mut book = ViewerNavigation::default();
        let rows = contact_dependencies(
            &terrain,
            Vec2::new(4.0, 4.0),
            Vec2::new(4.1, 4.0),
            Vec2::X,
            0.4,
        )
        .unwrap();
        let mut member = Member::stamp(0, 0, 1, 0);
        member.contact = rows.into_iter().collect();
        member.contact_epoch = terrain.epoch;
        member.direct = vec![(63, 0)];
        let prior = member.contact.clone();
        terrain.edit(Cell { x: 0, y: 0 }, 4);
        book.validate_contact(&mut member, &terrain);
        assert_eq!(member.contact, prior);
        assert!(!member.contact_invalidated);
        assert_eq!(member.contact_epoch, terrain.epoch);
        terrain.edit(Cell { x: 3, y: 3 }, 4);
        book.validate_contact(&mut member, &terrain);
        assert!(member.contact.is_empty());
        assert!(member.contact_invalidated);
        assert_eq!(member.direct, vec![(63, 0)]);
    }
    #[test]
    fn deferred_contact_validation_preserves_history_until_local_leases_are_checked() {
        let mut terrain = Terrain::flat();
        terrain.epoch = 1;
        let mut member = Member::stamp(0, 0, 1, 0);
        member.contact = (0..64).map(|key| (key, 0)).collect();
        let prior = member.contact.clone();
        let mut book = ViewerNavigation::default();
        book.diagnostics.invalidation_entries_this_tick = MAX_ENTRIES - 1;
        book.validate_contact(&mut member, &terrain);
        assert_eq!(member.contact_cursor, 1);
        assert_eq!(member.contact, prior);
        assert_eq!(member.contact_epoch, 0);
        assert!(!member.contact_invalidated);
        book.diagnostics.invalidation_entries_this_tick = 0;
        book.validate_contact(&mut member, &terrain);
        assert_eq!(member.contact_epoch, 1);
        assert_eq!(member.contact, prior);
        assert!(!member.contact_invalidated);
    }
    #[test]
    fn contact_dependency_bounds_cover_both_flanks_without_signed_zero_differences() {
        let terrain = Terrain::flat();
        let a = contact_dependencies(
            &terrain,
            Vec2::new(4.0, 4.0),
            Vec2::new(4.1, 4.0),
            Vec2::new(1.0, 0.0),
            0.4,
        )
        .unwrap();
        let b = contact_dependencies(
            &terrain,
            Vec2::new(4.0, 4.0),
            Vec2::new(4.1, 4.0),
            Vec2::new(1.0, -0.0),
            0.4,
        )
        .unwrap();
        assert_eq!(a, b);
        assert!(a.contains_key(&(2 * 8 + 3)));
        assert!(a.contains_key(&(5 * 8 + 4)));
        assert!(!a.contains_key(&0));
    }
    #[test]
    fn unresolved_edge_does_not_starve_other_certified_routes_or_prove_no_route() {
        struct WithUnknown {
            terrain: Terrain,
            unknown: Option<Cell>,
        }
        impl Surface for WithUnknown {
            fn elevation(&self, cell: Cell) -> Option<i32> {
                if self.unknown == Some(cell) {
                    None
                } else {
                    self.terrain.elevation(cell)
                }
            }
        }
        impl TerrainRead for WithUnknown {
            fn grid(&self) -> Grid {
                self.terrain.grid()
            }
            fn epoch(&self) -> u64 {
                self.terrain.epoch()
            }
            fn lease_matches(&self, key: usize, revision: u64) -> bool {
                self.terrain.lease_matches(key, revision)
            }
            fn valid_lease(&self, lease: &BTreeMap<usize, u64>) -> bool {
                self.terrain.valid_lease(lease)
            }
            fn rectangle(&self, min: Cell, max: Cell, lease: &mut BTreeMap<usize, u64>) -> bool {
                let clear = self.terrain.rectangle(min, max, lease);
                clear
                    && !self.unknown.is_some_and(|cell| {
                        cell.x >= min.x && cell.x <= max.x && cell.y >= min.y && cell.y <= max.y
                    })
            }
        }
        let mut terrain = WithUnknown {
            terrain: Terrain::flat(),
            unknown: Some(Cell { x: 5, y: 3 }),
        };
        let mut f = field(&terrain.terrain, 0.4);
        for _ in 0..100 {
            let mut diagnostics = NavigationDiagnostics::default();
            for _ in 0..4096 {
                if !f.advance(&terrain, &mut diagnostics, 8 * 1024 * 1024) {
                    break;
                }
            }
            assert!(diagnostics.expansions_this_tick <= MAX_EXPANSIONS);
            assert!(diagnostics.edge_checks_this_tick <= MAX_EDGES);
        }
        assert!(
            f.node(&terrain, Vertex { x: 13, y: 10 })
                .is_some_and(|node| node.closed),
            "an unresolved side edge cannot starve an entirely known direct path"
        );
        assert!(
            !f.node(&terrain, Vertex { x: 11, y: 7 })
                .is_some_and(|node| node.closed),
            "unknown-footprint vertices must never become certified through omission"
        );
        assert!(f.unresolved_edges);
        assert_eq!(f.stage as u8, Stage::Pending as u8);
        terrain.unknown = None;
        terrain.terrain.edit(Cell { x: 5, y: 3 }, 0);
        let mut diagnostics = NavigationDiagnostics::default();
        assert!(f.validate(&terrain, &mut diagnostics, 8 * 1024 * 1024));
        assert!(
            !f.unresolved_edges,
            "a relevant new permitted epoch must retry unresolved edges"
        );
        for _ in 0..100 {
            let mut diagnostics = NavigationDiagnostics::default();
            for _ in 0..4096 {
                if !f.advance(&terrain, &mut diagnostics, 8 * 1024 * 1024) {
                    break;
                }
            }
        }
        assert!(
            f.node(&terrain, Vertex { x: 11, y: 7 })
                .is_some_and(|node| node.closed)
        );
        assert_eq!(f.stage as u8, Stage::Exhausted as u8);
    }
}

#[cfg(test)]
pub(crate) fn debug_field_status(world: &World) -> Vec<(u64, u8, bool, bool, u64, u32)> {
    world
        .get_resource::<TerrainNavigation>()
        .into_iter()
        .flat_map(|navigation| navigation.viewers.iter())
        .flat_map(|viewer| viewer.fields.iter())
        .map(|field| {
            (
                field.id,
                field.stage as u8,
                field.resource_fault,
                field.demand,
                field.valid_epoch,
                field.last_service,
            )
        })
        .collect()
}

/// Privileged diagnostic state. Never include this output in a player projection.
pub fn privileged_contact_debug(world: &World) -> String {
    let Some(n) = world.get_resource::<TerrainNavigation>() else {
        return String::from("no navigation");
    };
    let Some(phase) = super::super::phase(world, turts_mod_api::ObservationPhase::Start) else {
        return String::from("no start phase");
    };
    let rows = n
        .viewers
        .iter()
        .flat_map(|book| {
            book.members.iter().map(|(&id, m)| {
                (
                    id,
                    m.viewer,
                    n.contact_ready(id, m.viewer, &phase),
                    m.contact_epoch,
                    phase.planning_epoch(m.viewer),
                    m.contact.len(),
                    m.contact_cursor,
                    m.contact_invalidated,
                    m.cancelled,
                    m.owner_fence,
                )
            })
        })
        .collect::<Vec<_>>();
    format!(
        "unit/viewer/ready/contact_epoch/phase_epoch/leases/cursor/invalid/cancelled/fence={rows:?}"
    )
}

#[cfg(test)]
#[path = "direct_memo_tests.rs"]
mod direct_memo_tests;

#[cfg(test)]
#[path = "continuation_tests.rs"]
mod continuation_tests;
