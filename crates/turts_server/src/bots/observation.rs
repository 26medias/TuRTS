//! Immutable permitted facts. Controllers cannot reach the authoritative world through this port.
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use turts_bot::*;
use turts_mod_api::{
    Behaviour, BotQuota, ContentCatalog, ContentId, CostSource, SlotRelation, TargetKind, WorldPos,
};

/// Coverage is independent of which private objects exist outside it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ObservedArea {
    pub min: WorldPos,
    pub max: WorldPos,
    pub complete: bool,
}

#[derive(Clone)]
pub(crate) struct ObservationBasis {
    pub terrain: Option<Arc<turts_sim::terrain::TerrainPhase>>,
    pub id: BasisId,
    pub tick: u32,
    pub slot: u8,
    pub teams: Vec<u8>,
    pub units: Vec<UnitObservation>,
    pub nodes: Vec<NodeObservation>,
    pub accounts: Vec<AccountObservation>,
    pub account_index: BTreeMap<SpendKey, usize>,
    pub unit_index: BTreeMap<UnitId, usize>,
    pub works: Vec<WorkObservation>,
    pub pending_content: BTreeMap<ContentId, u64>,
    pub supply: Vec<SupplyObservation>,
    pub area: ObservedArea,
    pub knowledge: Option<Arc<turts_mod_api::KnowledgeSnapshot>>,
    pub catalog: Arc<ContentCatalog>,
}

fn terrain_cell_count(
    grid: turts_rules::terrain::geometry::Grid,
    cells: turts_mod_api::TerrainRect,
) -> Result<usize, QueryError> {
    let width = cells
        .max
        .x
        .checked_sub(cells.min.x)
        .ok_or(QueryError::InvalidQuery)?;
    let height = cells
        .max
        .y
        .checked_sub(cells.min.y)
        .ok_or(QueryError::InvalidQuery)?;
    let count = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or(QueryError::InvalidQuery)?;
    if width == 0
        || height == 0
        || cells.max.x > grid.columns
        || cells.max.y > grid.rows
        || count > 256
    {
        return Err(QueryError::InvalidQuery);
    }
    usize::try_from(count).map_err(|_| QueryError::InvalidQuery)
}

impl ObservationBasis {
    /// Only permitted rows enter these indexes. Their order cannot disclose rejected objects.
    pub fn index(&mut self) {
        self.units
            .sort_by(|a, b| physical(a.position, &a.content, b.position, &b.content));
        self.nodes
            .sort_by(|a, b| physical(a.position, &a.content, b.position, &b.content));
        if let Some(knowledge) = &mut self.knowledge {
            let knowledge = Arc::make_mut(knowledge);
            knowledge
                .remembered_units
                .sort_by(|a, b| physical(a.position, &a.content, b.position, &b.content));
            knowledge
                .remembered_nodes
                .sort_by(|a, b| physical(a.position, &a.content, b.position, &b.content));
        }
        self.accounts.sort_by(|a, b| a.account.cmp(&b.account));
        self.works.sort_by(|a, b| a.identity.cmp(&b.identity));
        self.unit_index = self
            .units
            .iter()
            .enumerate()
            .map(|(i, u)| (u.unit, i))
            .collect();
        self.account_index = self
            .accounts
            .iter()
            .enumerate()
            .map(|(i, a)| (a.account.clone(), i))
            .collect();
    }
    fn own_unit(&self, id: UnitId) -> Option<&UnitObservation> {
        self.unit_index
            .get(&id)
            .and_then(|index| self.units.get(*index))
            .filter(|u| u.owner_slot == self.slot && u.own.is_some())
    }
    fn relation(&self, owner: u8, relation: SlotRelation) -> bool {
        turts_sim::rules_adapter::holds(
            relation,
            self.slot,
            owner,
            turts_sim::rules_adapter::Teams::new(&self.teams),
        )
    }
    fn quote(&self, actor: UnitId, production: &ContentId) -> Option<CostQuote> {
        let unit = self.own_unit(actor)?;
        let definition = self.catalog.units.get(&unit.content)?;
        let recipe = self.catalog.production.get(production)?;
        if !definition
            .producer
            .as_ref()
            .is_some_and(|p| p.recipes.contains(production))
            && !definition
                .constructor
                .as_ref()
                .is_some_and(|p| p.buildings.contains(production))
        {
            return None;
        }
        let mut sums = BTreeMap::<SpendKey, u64>::new();
        for cost in &recipe.costs {
            let endpoint = match cost.source {
                CostSource::SlotBalance => SourceEndpoint::SlotBalance(self.slot),
                CostSource::ProducerInventory => SourceEndpoint::Inventory {
                    unit: actor,
                    port: InventoryPort::Main,
                },
            };
            let account = SpendKey {
                endpoint,
                resource: cost.resource.clone(),
            };
            let sum = sums.entry(account).or_default();
            *sum = sum.checked_add(cost.amount)?;
        }
        let mut affordable = true;
        for (key, amount) in &sums {
            let account = self
                .account_index
                .get(key)
                .and_then(|index| self.accounts.get(*index))?;
            affordable &= account
                .on_hand
                .checked_sub(account.reserved)
                .is_some_and(|n| n >= *amount);
        }
        Some(CostQuote {
            costs: sums
                .into_iter()
                .map(|(account, amount)| AccountAmount { account, amount })
                .collect(),
            affordable,
        })
    }
    fn area_observed(&self, min: WorldPos, max: WorldPos) -> bool {
        if self.area.complete {
            return true;
        }
        let Some(knowledge) = self.knowledge.as_ref() else {
            return false;
        };
        let grid = knowledge.grid;
        turts_rules::visibility::Grid {
            width: grid.size.x,
            height: grid.size.y,
            cell_size: grid.cell_size,
            columns: grid.columns,
            rows: grid.rows,
        }
        .rectangle(
            turts_rules::visibility::Point { x: min.x, y: min.y },
            turts_rules::visibility::Point { x: max.x, y: max.y },
            |cell| {
                knowledge
                    .observed
                    .get(cell as usize / 8)
                    .is_some_and(|byte| byte & (1 << (cell % 8)) != 0)
            },
        )
    }
    fn placement(&self, query: &PlacementQuery) -> Advice<PlacementAdvice> {
        let Some(actor) = self.own_unit(query.actor) else {
            return Advice::NotObserved;
        };
        let Some(source) = self.catalog.units.get(&actor.content) else {
            return Advice::NotObserved;
        };
        let Some(recipe) = self.catalog.production.get(&query.production) else {
            return Advice::NotObserved;
        };
        let Some(output) = self.catalog.units.get(&recipe.output_unit) else {
            return Advice::NotObserved;
        };
        if !source
            .constructor
            .as_ref()
            .is_some_and(|c| c.buildings.contains(&query.production))
        {
            return Advice::Known(PlacementAdvice::Blocked);
        }
        let p = query.point;
        if !p.x.is_finite() || !p.y.is_finite() {
            return Advice::Known(PlacementAdvice::Blocked);
        }
        let Some(config) = output.placement else {
            return Advice::Known(PlacementAdvice::Blocked);
        };
        use turts_rules::placement as rules;
        let circle = |position: WorldPos, radius| rules::Circle {
            centre: rules::Point {
                x: position.x,
                y: position.y,
            },
            radius,
        };
        let radius = |definition: &turts_mod_api::ResolvedUnit| match definition.footprint {
            turts_mod_api::Footprint::Circle { radius } => radius,
        };
        let output_radius = radius(output);
        if p.x - output_radius < 0.0
            || p.y - output_radius < 0.0
            || p.x + output_radius > self.area.max.x
            || p.y + output_radius > self.area.max.y
        {
            return Advice::Known(PlacementAdvice::Blocked);
        }
        // Public maxima determine the region. Hidden object presence never affects its size.
        let max_unit_radius = self
            .catalog
            .units
            .values()
            .map(radius)
            .fold(0.0_f32, f32::max);
        let max_static_gap = self
            .catalog
            .units
            .values()
            .filter_map(|unit| unit.placement.map(|placement| placement.gap))
            .fold(0.0_f32, f32::max);
        let max_node_radius = self
            .catalog
            .nodes
            .values()
            .map(|node| node.radius)
            .fold(0.0_f32, f32::max);
        let Some(region) = turts_rules::visibility_advice::required_region(
            rules::Point { x: p.x, y: p.y },
            output_radius,
            config.gap,
            config.node_clearance,
            turts_rules::visibility_advice::PublicPlacementMaxima {
                unit_radius: max_unit_radius,
                static_gap: max_static_gap,
                node_radius: max_node_radius,
            },
            rules::Point {
                x: self.area.max.x,
                y: self.area.max.y,
            },
        ) else {
            return Advice::NotObserved;
        };
        if !self.area_observed(
            WorldPos {
                x: region.min.x,
                y: region.min.y,
            },
            WorldPos {
                x: region.max.x,
                y: region.max.y,
            },
        ) {
            return Advice::NotObserved;
        }
        let sources = self
            .units
            .iter()
            .filter(|unit| unit.owner_slot == self.slot && !unit.site)
            .filter_map(|unit| {
                Some(rules::BuildSource {
                    footprint: circle(unit.position, unit.radius),
                    build_area: self.catalog.units.get(&unit.content)?.build_area?,
                })
            })
            .collect::<Vec<_>>();
        let obstacles = self
            .units
            .iter()
            .map(|unit| {
                let definition = self.catalog.units.get(&unit.content);
                rules::Obstacle {
                    id: unit.unit,
                    footprint: circle(unit.position, unit.radius),
                    kind: if !unit.site && definition.is_some_and(|d| d.movement.is_some()) {
                        rules::ObstacleKind::Mobile
                    } else {
                        rules::ObstacleKind::Static {
                            gap: definition.and_then(|d| d.placement).map_or(0.0, |p| p.gap),
                        }
                    },
                }
            })
            .collect::<Vec<_>>();
        let nodes = self
            .nodes
            .iter()
            .filter(|node| node.remaining > 0)
            .map(|node| circle(node.position, node.radius))
            .collect::<Vec<_>>();
        let verdict = rules::check(
            &rules::Site {
                footprint: circle(p, radius(output)),
                gap: config.gap,
                node_clearance: config.node_clearance,
                in_build_area: config.in_build_area,
                queue: None,
            },
            &rules::View {
                map_width: self.area.max.x,
                map_height: self.area.max.y,
                build_sources: &sources,
                obstacles: &obstacles,
                nodes: &nodes,
            },
            |_| true,
        );
        Advice::Known(if verdict == rules::Verdict::Valid {
            PlacementAdvice::Allowed
        } else {
            PlacementAdvice::Blocked
        })
    }
}
fn physical(a: WorldPos, ac: &ContentId, b: WorldPos, bc: &ContentId) -> Ordering {
    a.x.total_cmp(&b.x)
        .then(a.y.total_cmp(&b.y))
        .then(ac.cmp(bc))
}
fn distance_squared(a: WorldPos, b: WorldPos) -> f64 {
    (f64::from(a.x) - f64::from(b.x)).powi(2) + (f64::from(a.y) - f64::from(b.y)).powi(2)
}
fn within(p: WorldPos, center: Option<WorldPos>, radius: Option<f32>) -> bool {
    match (center, radius) {
        (Some(c), Some(r)) => distance_squared(p, c) <= f64::from(r).powi(2),
        _ => true,
    }
}

#[derive(Clone)]
struct Continuation {
    query: Query,
    offset: usize,
    controller: String,
}
/// One candidate basis per bot. Pins belong to controller identities, not query kinds.
pub(crate) struct CandidateLease {
    basis: Arc<ObservationBasis>,
    pins: BTreeSet<String>,
    born_step: u64,
    cursors: BTreeMap<u64, Continuation>,
    next_cursor: u64,
}
impl CandidateLease {
    pub fn new(mut basis: ObservationBasis, step: u64) -> Self {
        basis.index();
        Self {
            basis: Arc::new(basis),
            pins: BTreeSet::new(),
            born_step: step,
            cursors: BTreeMap::new(),
            next_cursor: 0,
        }
    }
    pub fn basis(&self) -> &ObservationBasis {
        &self.basis
    }
    pub fn expired(&self, step: u64) -> bool {
        step.saturating_sub(self.born_step) >= 8
    }
    pub fn retained(&self) -> bool {
        !self.pins.is_empty()
    }
    pub fn queries(&mut self, controller: &str, quota: BotQuota) -> PermittedQueries<'_> {
        PermittedQueries {
            lease: self,
            controller: controller.into(),
            budget: DecisionBudget::new(quota),
        }
    }
}
pub(crate) struct PermittedQueries<'a> {
    lease: &'a mut CandidateLease,
    controller: String,
    budget: DecisionBudget,
}
impl BotQueries for PermittedQueries<'_> {
    fn pin_basis(&mut self, basis: BasisId) -> Result<(), QueryError> {
        if self.lease.basis.id != basis {
            return Err(QueryError::ExpiredBasis);
        }
        self.lease.pins.insert(self.controller.clone());
        Ok(())
    }
    fn release_basis(&mut self, basis: BasisId) -> Result<(), QueryError> {
        if self.lease.basis.id != basis {
            return Err(QueryError::ExpiredBasis);
        }
        self.lease.pins.remove(&self.controller);
        Ok(())
    }
    fn query(
        &mut self,
        query: Query,
        continuation: Option<QueryCursor>,
        limit: u32,
    ) -> Result<QueryPage, QueryError> {
        if limit == 0 || limit > 64 {
            return Err(QueryError::InvalidQuery);
        }
        let offset = if let Some(cursor) = continuation {
            if cursor.basis != self.lease.basis.id {
                return Err(QueryError::ExpiredBasis);
            }
            let state = self
                .lease
                .cursors
                .get(&cursor.token)
                .ok_or(QueryError::InvalidContinuation)?;
            if state.query != query || state.controller != self.controller {
                return Err(QueryError::InvalidContinuation);
            }
            state.offset
        } else {
            0
        };
        let remaining = self.budget.remaining();
        if remaining.queries == 0 || remaining.rows == 0 || remaining.candidates == 0 {
            return Err(QueryError::Budget);
        }
        let basis = &self.lease.basis;
        let max_rows =
            usize::try_from(limit.min(remaining.rows)).map_err(|_| QueryError::InvalidQuery)?;
        let max_scan = remaining.candidates as usize;
        let mut examined = 0usize;
        let mut end = offset;
        let mut tied = false;
        // This enumerates permitted rows completely. It never asserts hidden-world absence.
        let mut coverage = Coverage::Complete;
        let (data, total, returned) = match &query {
            Query::Units(filter) => {
                validate_radius(filter.center, filter.radius)?;
                let mut rows = Vec::new();
                for u in basis.units.iter().skip(offset) {
                    if examined >= max_scan || rows.len() >= max_rows {
                        break;
                    }
                    examined += 1;
                    end += 1;
                    if basis.relation(u.owner_slot, filter.relation)
                        && (filter.content.is_empty() || filter.content.contains(&u.content))
                        && within(u.position, filter.center, filter.radius)
                    {
                        rows.push(u.clone())
                    }
                }
                if end < basis.units.len() && end > 0 {
                    let a = &basis.units[end - 1];
                    let b = &basis.units[end];
                    tied =
                        physical(a.position, &a.content, b.position, &b.content) == Ordering::Equal;
                }
                let n = rows.len();
                (QueryData::Units(rows), basis.units.len(), n)
            }
            Query::Nodes(filter) => {
                validate_radius(filter.center, filter.radius)?;
                let mut rows = Vec::new();
                for n in basis.nodes.iter().skip(offset) {
                    if examined >= max_scan || rows.len() >= max_rows {
                        break;
                    }
                    examined += 1;
                    end += 1;
                    if (filter.resources.is_empty() || filter.resources.contains(&n.item))
                        && within(n.position, filter.center, filter.radius)
                    {
                        rows.push(n.clone())
                    }
                }
                if end < basis.nodes.len() && end > 0 {
                    let a = &basis.nodes[end - 1];
                    let b = &basis.nodes[end];
                    tied =
                        physical(a.position, &a.content, b.position, &b.content) == Ordering::Equal;
                }
                let n = rows.len();
                (QueryData::Nodes(rows), basis.nodes.len(), n)
            }
            Query::LookupUnit(id) => {
                if offset != 0 {
                    return Err(QueryError::InvalidContinuation);
                }
                // Index lookup costs are independent of absent or hidden subjects.
                examined = 1;
                end = 1;
                let row = basis
                    .unit_index
                    .get(id)
                    .and_then(|index| basis.units.get(*index))
                    .cloned();
                let data = if let Some(row) = row {
                    Advice::Known(row)
                } else {
                    coverage = Coverage::NotObserved;
                    Advice::NotObserved
                };
                (QueryData::Lookup(data), 1, 1)
            }
            Query::Accounts(keys) => {
                if keys.len() > 64 {
                    return Err(QueryError::InvalidQuery);
                }
                let mut rows = Vec::new();
                for key in keys.iter().skip(offset) {
                    if examined >= max_scan || rows.len() >= max_rows {
                        break;
                    }
                    examined += 1;
                    end += 1;
                    if let Some(row) = basis
                        .account_index
                        .get(key)
                        .and_then(|index| basis.accounts.get(*index))
                    {
                        rows.push(row.clone())
                    } else {
                        coverage = Coverage::NotObserved
                    }
                }
                let n = rows.len();
                (QueryData::Accounts(rows), keys.len(), n)
            }
            Query::Works => {
                let n = max_rows
                    .min(max_scan)
                    .min(basis.works.len().saturating_sub(offset));
                examined = n;
                end = offset + n;
                (
                    QueryData::Works(basis.works[offset..end].to_vec()),
                    basis.works.len(),
                    n,
                )
            }
            Query::RememberedUnits(filter) => {
                validate_radius(filter.center, filter.radius)?;
                let source = basis
                    .knowledge
                    .as_ref()
                    .map_or(&[][..], |knowledge| knowledge.remembered_units.as_slice());
                let mut rows = vec![];
                for row in source.iter().skip(offset) {
                    if examined >= max_scan || rows.len() >= max_rows {
                        break;
                    }
                    examined += 1;
                    end += 1;
                    if basis.relation(row.owner, filter.relation)
                        && (filter.content.is_empty() || filter.content.contains(&row.content))
                        && within(row.position, filter.center, filter.radius)
                    {
                        rows.push(row.clone());
                    }
                }
                if end < source.len() && end > 0 {
                    tied = physical(
                        source[end - 1].position,
                        &source[end - 1].content,
                        source[end].position,
                        &source[end].content,
                    ) == Ordering::Equal;
                }
                let count = rows.len();
                (
                    QueryData::RememberedUnits(RememberedRows {
                        rows,
                        retention_truncated: basis
                            .knowledge
                            .as_ref()
                            .is_some_and(|knowledge| knowledge.retention_truncated),
                    }),
                    source.len(),
                    count,
                )
            }
            Query::RememberedNodes(filter) => {
                validate_radius(filter.center, filter.radius)?;
                let source = basis
                    .knowledge
                    .as_ref()
                    .map_or(&[][..], |knowledge| knowledge.remembered_nodes.as_slice());
                let mut rows = vec![];
                for row in source.iter().skip(offset) {
                    if examined >= max_scan || rows.len() >= max_rows {
                        break;
                    }
                    examined += 1;
                    end += 1;
                    if (filter.resources.is_empty()
                        || basis
                            .catalog
                            .nodes
                            .get(&row.content)
                            .is_some_and(|definition| filter.resources.contains(&definition.item)))
                        && within(row.position, filter.center, filter.radius)
                    {
                        rows.push(row.clone());
                    }
                }
                if end < source.len() && end > 0 {
                    tied = physical(
                        source[end - 1].position,
                        &source[end - 1].content,
                        source[end].position,
                        &source[end].content,
                    ) == Ordering::Equal;
                }
                let count = rows.len();
                (
                    QueryData::RememberedNodes(RememberedRows {
                        rows,
                        retention_truncated: basis
                            .knowledge
                            .as_ref()
                            .is_some_and(|knowledge| knowledge.retention_truncated),
                    }),
                    source.len(),
                    count,
                )
            }
            Query::Placement(request) => {
                if offset != 0 {
                    return Err(QueryError::InvalidContinuation);
                }
                examined = basis
                    .units
                    .len()
                    .checked_add(basis.nodes.len())
                    .and_then(|n| n.checked_add(1))
                    .ok_or(QueryError::Budget)?;
                if examined > max_scan {
                    return Err(QueryError::Budget);
                }
                end = 1;
                let advice = basis.placement(request);
                coverage = if matches!(advice, Advice::Known(_)) {
                    Coverage::Complete
                } else {
                    Coverage::NotObserved
                };
                (QueryData::Placement(advice), 1, 1)
            }
            Query::Affordability(request) => {
                if offset != 0 {
                    return Err(QueryError::InvalidContinuation);
                }
                examined = basis
                    .accounts
                    .len()
                    .checked_add(1)
                    .ok_or(QueryError::Budget)?;
                if examined > max_scan {
                    return Err(QueryError::Budget);
                }
                end = 1;
                let advice = basis
                    .quote(request.actor, &request.production)
                    .map_or(Advice::NotObserved, Advice::Known);
                (QueryData::Affordability(advice), 1, 1)
            }
            Query::Terrain(request) => {
                let terrain = basis.terrain.as_ref().ok_or(QueryError::InvalidQuery)?;
                let total = terrain_cell_count(terrain.grid(), request.cells)?;
                if offset > total {
                    return Err(QueryError::InvalidContinuation);
                }
                let width = request.cells.max.x - request.cells.min.x;
                let mut rows = Vec::new();
                while end < total && rows.len() < max_rows && examined < max_scan {
                    let index = u32::try_from(end).map_err(|_| QueryError::InvalidQuery)?;
                    let cell = turts_mod_api::TerrainCell {
                        x: request.cells.min.x + index % width,
                        y: request.cells.min.y + index / width,
                    };
                    let knowledge = if let Some(surface) = terrain.surface(basis.slot, cell) {
                        let fact = TerrainSurfaceObservation {
                            elevation: surface.elevation,
                            material: surface.material,
                            provenance: surface.provenance,
                            observed_tick: surface.last_seen_tick,
                        };
                        if surface.current {
                            TerrainCellKnowledge::Current(fact)
                        } else {
                            coverage = Coverage::NotObserved;
                            TerrainCellKnowledge::Remembered(fact)
                        }
                    } else {
                        coverage = Coverage::NotObserved;
                        TerrainCellKnowledge::Unknown
                    };
                    rows.push(TerrainCellObservation { cell, knowledge });
                    end += 1;
                    examined += 1;
                }
                let returned = rows.len();
                (QueryData::Terrain(rows), total, returned)
            }
        };
        let usage = BudgetUsage {
            queries: 1,
            candidates: u32::try_from(examined).map_err(|_| QueryError::Budget)?,
            rows: u32::try_from(returned).map_err(|_| QueryError::Budget)?,
            proposals: 0,
        };
        let state = if end < total {
            if self.lease.cursors.len() >= 1024 {
                return Err(QueryError::Budget);
            }
            let token = self
                .lease
                .next_cursor
                .checked_add(1)
                .ok_or(QueryError::Budget)?;
            self.lease.next_cursor = token;
            self.lease.cursors.insert(
                token,
                Continuation {
                    query: query.clone(),
                    offset: end,
                    controller: self.controller.clone(),
                },
            );
            let cursor = QueryCursor {
                basis: basis.id,
                token,
            };
            if tied {
                PageState::UnresolvedTie(cursor)
            } else {
                PageState::More(cursor)
            }
        } else {
            PageState::Complete
        };
        self.budget.charge(usage).map_err(|_| QueryError::Budget)?;
        Ok(QueryPage {
            basis: basis.id,
            completed_tick: basis.tick,
            data,
            state,
            coverage,
            usage,
        })
    }
}
fn validate_radius(center: Option<WorldPos>, radius: Option<f32>) -> Result<(), QueryError> {
    if center.is_some_and(|p| !p.x.is_finite() || !p.y.is_finite())
        || radius.is_some_and(|r| !r.is_finite() || r < 0.0)
        || radius.is_some() != center.is_some()
    {
        return Err(QueryError::InvalidQuery);
    }
    Ok(())
}

pub(crate) struct ObservationVerifier<'a> {
    pub candidate: &'a CandidateLease,
    pub current: &'a ObservationBasis,
    pub profile: &'a turts_mod_api::BotDef,
}
impl ProposalVerifier for ObservationVerifier<'_> {
    fn verify(
        &self,
        basis: BasisId,
        proposal: &Proposal,
    ) -> Result<VerifiedProposal, ProposalRejection> {
        if basis != self.current.id || proposal.source_basis != self.candidate.basis.id {
            return Err(ProposalRejection::NotObserved);
        }
        let definition = self
            .current
            .catalog
            .orders
            .get(&proposal.order.content)
            .ok_or(ProposalRejection::InvalidOrder)?;
        let target_valid = matches!(
            (&proposal.order.target, definition.target),
            (BotTarget::None, TargetKind::None)
                | (
                    BotTarget::Point(_),
                    TargetKind::Point | TargetKind::PointOrUnit
                )
                | (
                    BotTarget::Unit(_),
                    TargetKind::Unit | TargetKind::PointOrUnit
                )
                | (BotTarget::Node(_), TargetKind::Node)
                | (BotTarget::Job(_), TargetKind::Job)
                | (BotTarget::Content(_), TargetKind::Content)
                | (BotTarget::ContentAtPoint { .. }, TargetKind::ContentAtPoint)
                | (
                    BotTarget::TerrainWork { .. },
                    TargetKind::TerrainDesignation
                )
        );
        if !target_valid {
            return Err(ProposalRejection::InvalidOrder);
        }
        let mut members = Vec::new();
        for id in &proposal.actors {
            let unit = self
                .current
                .own_unit(*id)
                .ok_or(ProposalRejection::NotObserved)?;
            if !definition.units.is_empty() && !definition.units.contains(&unit.content.to_string())
            {
                return Err(ProposalRejection::InvalidOrder);
            }
            let production = match (&proposal.order.target, definition.behaviour) {
                (BotTarget::Content(content), Behaviour::Produce) => Some(content),
                (BotTarget::ContentAtPoint { content, .. }, Behaviour::Construct) => Some(content),
                _ => None,
            };
            let (costs, payment) = if let Some(content) = production {
                let quote = self
                    .current
                    .quote(*id, content)
                    .ok_or(ProposalRejection::NotObserved)?;
                let recipe = self
                    .current
                    .catalog
                    .production
                    .get(content)
                    .ok_or(ProposalRejection::InvalidOrder)?;
                (quote.costs, Some(recipe.payment))
            } else {
                (Vec::new(), None)
            };
            match &proposal.order.target {
                BotTarget::Unit(target)
                    if !self.current.units.iter().any(|u| u.unit == *target) =>
                {
                    return Err(ProposalRejection::NotObserved);
                }
                BotTarget::Node(target)
                    if !self.current.nodes.iter().any(|u| u.node == *target) =>
                {
                    return Err(ProposalRejection::NotObserved);
                }
                BotTarget::Point(p) | BotTarget::ContentAtPoint { point: p, .. }
                    if !p.x.is_finite() || !p.y.is_finite() =>
                {
                    return Err(ProposalRejection::InvalidOrder);
                }
                _ => {}
            }
            if let BotTarget::TerrainWork {
                tool,
                cells,
                target_elevation,
            } = &proposal.order.target
            {
                let terrain = self
                    .current
                    .terrain
                    .as_ref()
                    .ok_or(ProposalRejection::InvalidOrder)?;
                terrain_cell_count(terrain.grid(), *cells)
                    .map_err(|_| ProposalRejection::InvalidOrder)?;
                let grid = terrain.grid();
                let actor = self
                    .current
                    .catalog
                    .units
                    .get(&unit.content)
                    .ok_or(ProposalRejection::InvalidOrder)?;
                if definition.behaviour != Behaviour::TerrainWork
                    || !self.current.catalog.terrain_tools.contains_key(tool)
                    || !actor.terrain_tools.contains(tool)
                    || *target_elevation < grid.floor
                    || *target_elevation > grid.ceiling
                {
                    return Err(ProposalRejection::InvalidOrder);
                }
                // Future prism costs depend on authorized exposure. No speculative
                // buried-material quote or advance credit enters this proposal.
            }
            members.push(VerifiedMember {
                unit: *id,
                anchor: unit.position,
                costs,
                payment,
            });
        }
        let mut fulfillment = Vec::new();
        let production = match (&proposal.order.target, definition.behaviour) {
            (BotTarget::Content(content), Behaviour::Produce)
            | (BotTarget::ContentAtPoint { content, .. }, Behaviour::Construct) => Some(content),
            _ => None,
        };
        if let Some(recipe) = production.and_then(|id| self.current.catalog.production.get(id)) {
            let output = self
                .current
                .catalog
                .units
                .get(&recipe.output_unit)
                .ok_or(ProposalRejection::InvalidOrder)?;
            let goal = self
                .profile
                .knowledge
                .opening
                .iter()
                .chain(&self.profile.knowledge.recovery)
                .find(|goal| goal.key == proposal.task.goal)
                .map(|goal| &goal.action);
            for role in &self.profile.knowledge.roles {
                if !role_matches(role, output) {
                    continue;
                }
                let completed = self
                    .current
                    .units
                    .iter()
                    .filter(|unit| {
                        unit.owner_slot == self.current.slot
                            && !unit.site
                            && role
                                .units
                                .iter()
                                .any(|c| super::content_ref_matches(c, &unit.content))
                    })
                    .count() as u64;
                let pending = self
                    .current
                    .works
                    .iter()
                    .filter(|work| {
                        !work.state.terminal()
                            && work.content.as_ref().is_some_and(|c| {
                                role.units.iter().any(|r| super::content_ref_matches(r, c))
                            })
                    })
                    .count() as u64;
                let held = self
                    .current
                    .pending_content
                    .iter()
                    .filter(|(content, _)| {
                        role.units
                            .iter()
                            .any(|r| super::content_ref_matches(r, content))
                    })
                    .try_fold(0u64, |total, (_, count)| total.checked_add(*count))
                    .ok_or(ProposalRejection::ArithmeticOverflow)?;
                let observed = completed
                    .checked_add(pending)
                    .and_then(|n| n.checked_add(held))
                    .ok_or(ProposalRejection::ArithmeticOverflow)?;
                let cap = match goal {
                    Some(turts_mod_api::BotGoalAction::EnsureUnits {
                        role: requested,
                        count,
                        ..
                    }) if requested == &role.key => Some(u64::from(*count)),
                    _ => None,
                }
                .or_else(|| {
                    self.profile
                        .knowledge
                        .production
                        .iter()
                        .find(|target| target.key == proposal.task.goal && target.role == role.key)
                        .map(|t| u64::from(t.count))
                });
                fulfillment.push(FulfillmentClaim {
                    key: FulfillmentKey::Role(role.key.clone()),
                    observed,
                    cap,
                    contribution: 1,
                });
            }
            for row in &output.provides {
                let resource = ContentId::parse(&row.resource)
                    .map_err(|_| ProposalRejection::InvalidFulfillment)?;
                let (provided, consumed) = supply_totals(self.current, &resource)?;
                let cap = match goal {
                    Some(turts_mod_api::BotGoalAction::EnsureSupply {
                        resource: requested,
                        amount,
                        ..
                    }) if requested == &row.resource => {
                        let threshold = amount
                            .checked_add(consumed)
                            .ok_or(ProposalRejection::ArithmeticOverflow)?;
                        Some(provider_cap(threshold, row.amount)?)
                    }
                    _ => None,
                };
                fulfillment.push(FulfillmentClaim {
                    key: FulfillmentKey::Supply(resource),
                    observed: provided,
                    cap,
                    contribution: row.amount,
                });
            }
        }
        Ok(VerifiedProposal {
            members,
            fulfillment,
        })
    }
}

fn provider_cap(threshold: u64, contribution: u64) -> Result<u64, ProposalRejection> {
    threshold
        .checked_add(
            contribution
                .checked_sub(1)
                .ok_or(ProposalRejection::InvalidFulfillment)?,
        )
        .ok_or(ProposalRejection::ArithmeticOverflow)
}
fn supply_totals(
    basis: &ObservationBasis,
    resource: &ContentId,
) -> Result<(u64, u64), ProposalRejection> {
    let live = basis.supply.iter().find(|row| row.resource == *resource);
    let mut provided = live.map_or(0, |row| row.provided);
    let mut consumed = live.map_or(0, |row| row.consumed);
    let pending = basis
        .works
        .iter()
        .filter(|work| !work.state.terminal())
        .filter_map(|work| work.content.as_ref().map(|content| (content, 1)))
        .chain(
            basis
                .pending_content
                .iter()
                .map(|(content, count)| (content, *count)),
        );
    for (content, count) in pending {
        let Some(definition) = basis.catalog.units.get(content) else {
            continue;
        };
        for (rows, total) in [
            (&definition.provides, &mut provided),
            (&definition.consumes, &mut consumed),
        ] {
            for row in rows
                .iter()
                .filter(|row| row.resource == resource.to_string())
            {
                *total = row
                    .amount
                    .checked_mul(count)
                    .and_then(|amount| total.checked_add(amount))
                    .ok_or(ProposalRejection::ArithmeticOverflow)?;
            }
        }
    }
    Ok((provided, consumed))
}

fn role_matches(role: &turts_mod_api::BotRole, unit: &turts_mod_api::ResolvedUnit) -> bool {
    role.units
        .iter()
        .any(|id| super::content_ref_matches(id, &unit.id))
        && role.requires.iter().all(|cap| match cap {
            turts_mod_api::Capability::Movement => unit.movement.is_some(),
            turts_mod_api::Capability::Weapon => unit.weapon.is_some(),
            turts_mod_api::Capability::Gatherer => unit.gatherer.is_some(),
            turts_mod_api::Capability::Producer => unit.producer.is_some(),
            turts_mod_api::Capability::Research => unit.research.is_some(),
            turts_mod_api::Capability::Constructor => unit.constructor.is_some(),
            turts_mod_api::Capability::Repairer => unit.repairer.is_some(),
            turts_mod_api::Capability::TerrainWorker => !unit.terrain_tools.is_empty(),
            turts_mod_api::Capability::CargoUnloader => unit.terrain_cargo.is_some(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn content_matching_preserves_exact_display_equality_without_formatting_in_scans() {
        for package in ["fixture", "fixture-long", "", "with:colon"] {
            for local in ["unit", "unit-long", "", "with:colon"] {
                let id = ContentId {
                    package: turts_mod_api::PackageId(package.into()),
                    local: local.into(),
                };
                let printed = id.to_string();
                for candidate in [
                    printed.clone(),
                    format!("{printed}x"),
                    format!("x{printed}"),
                    local.into(),
                    package.into(),
                    "fixture:unit".into(),
                ] {
                    assert_eq!(
                        super::super::content_ref_matches(&candidate, &id),
                        candidate == printed
                    );
                }
            }
        }
    }
    #[test]
    fn role_counts_keep_owner_site_capability_and_pending_work_rules() {
        let mut current = basis();
        for unit in &mut current.units {
            unit.capabilities.push(turts_mod_api::Capability::Weapon);
        }
        current.units[1].site = true;
        current.units[2].owner_slot = 1;
        let content = current.units[0].content.clone();
        for (id, state, output) in [
            (1, WorkState::Running, Some(content.clone())),
            (
                2,
                WorkState::WaitingForFunds(content.clone()),
                Some(content.clone()),
            ),
            (3, WorkState::OutputBlocked, Some(content.clone())),
            (4, WorkState::Finished, Some(content.clone())),
            (5, WorkState::Failed, Some(content.clone())),
            (6, WorkState::Running, None),
            (
                7,
                WorkState::Running,
                Some(ContentId::parse("foreign:unit").unwrap()),
            ),
        ] {
            current.works.push(WorkObservation {
                identity: WorkIdentity {
                    unit: 1,
                    id,
                    kind: WorkKind::Production,
                },
                actor_unit: 1,
                content: output,
                payment: None,
                done: 1,
                total: 2,
                paid: vec![],
                reserved: vec![],
                state,
            });
        }
        current.pending_content.insert(content, 2);
        current
            .pending_content
            .insert(ContentId::parse("foreign:unit").unwrap(), 100);
        let profile = turts_mod_api::BotDef {
            id: "policy".into(),
            name: "Policy".into(),
            decision_ms: 1000,
            start_when: turts_mod_api::BotStartWhen::MatchRunning,
            knowledge: turts_mod_api::BotKnowledge {
                roles: vec![
                    turts_mod_api::BotRole {
                        key: "armed".into(),
                        units: vec!["fixture:unit".into()],
                        requires: vec![turts_mod_api::Capability::Weapon],
                    },
                    turts_mod_api::BotRole {
                        key: "worker".into(),
                        units: vec!["fixture:unit".into()],
                        requires: vec![turts_mod_api::Capability::Gatherer],
                    },
                    turts_mod_api::BotRole {
                        key: "foreign".into(),
                        units: vec!["foreign:unit".into()],
                        requires: vec![],
                    },
                ],
                ..Default::default()
            },
            controllers: vec![],
        };
        assert_eq!(
            super::super::role_counts(&current, &profile),
            vec![
                RoleCount {
                    role: "armed".into(),
                    completed: 1,
                    pending: 5
                },
                RoleCount {
                    role: "worker".into(),
                    completed: 0,
                    pending: 5
                },
                RoleCount {
                    role: "foreign".into(),
                    completed: 0,
                    pending: 101
                },
            ]
        );
        current.units.reverse();
        current.works.reverse();
        assert_eq!(
            super::super::role_counts(&current, &profile)[0],
            RoleCount {
                role: "armed".into(),
                completed: 1,
                pending: 5
            }
        );
    }
    #[test]
    fn role_count_groups_keep_mixed_capabilities_and_duplicate_binding_membership() {
        use turts_mod_api::{BotDef, BotKnowledge, BotRole, BotStartWhen, Capability};
        let mut current = basis();
        for (unit, capabilities) in current.units.iter_mut().zip([
            vec![Capability::Weapon],
            vec![Capability::Gatherer],
            vec![Capability::Weapon, Capability::Gatherer],
        ]) {
            unit.capabilities = capabilities;
        }
        let mut repeated = current.units[0].clone();
        repeated.unit = 4;
        current.units.push(repeated);
        let profile = BotDef {
            id: "grouped".into(),
            name: "Grouped".into(),
            decision_ms: 1000,
            start_when: BotStartWhen::MatchRunning,
            knowledge: BotKnowledge {
                roles: [
                    ("any", vec![]),
                    ("armed", vec![Capability::Weapon]),
                    ("worker", vec![Capability::Gatherer]),
                    ("both", vec![Capability::Weapon, Capability::Gatherer]),
                ]
                .into_iter()
                .map(|(key, requires)| BotRole {
                    key: key.into(),
                    units: vec![
                        "fixture:unit".into(),
                        "fixture:unit".into(),
                        "unknown:unit".into(),
                        "fixture:unit-extra".into(),
                        "bad".into(),
                    ],
                    requires,
                })
                .chain([BotRole {
                    key: "empty".into(),
                    units: vec![],
                    requires: vec![],
                }])
                .collect(),
                ..Default::default()
            },
            controllers: vec![],
        };
        let expected = [
            ("any", 4),
            ("armed", 3),
            ("worker", 2),
            ("both", 1),
            ("empty", 0),
        ]
        .into_iter()
        .map(|(role, completed)| RoleCount {
            role: role.into(),
            completed,
            pending: 0,
        })
        .collect::<Vec<_>>();
        assert_eq!(super::super::role_counts(&current, &profile), expected);
        current.units.reverse();
        for unit in &mut current.units {
            unit.capabilities.reverse();
        }
        assert_eq!(super::super::role_counts(&current, &profile), expected);
    }
    #[test]
    fn independent_review_wildcard_orders_remain_valid_for_bot_proposals() {
        use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/cedar");
        let archive = PackageArchive::parse(&pack_dir(&path).unwrap()).unwrap();
        let catalog = archive.content().unwrap();
        let stop = ContentId::parse("cedar:order_stop").unwrap();
        let mut order = catalog.orders[&stop].clone();
        order.units.clear(); // The ordinary order API defines this as all capable units.
        let mut current = basis();
        Arc::make_mut(&mut current.catalog)
            .orders
            .insert(stop.clone(), order);
        current.index();
        let candidate = CandidateLease::new(current.clone(), 0);
        let profile = turts_mod_api::BotDef {
            id: "policy".into(),
            name: "Policy".into(),
            decision_ms: 1000,
            start_when: turts_mod_api::BotStartWhen::MatchRunning,
            knowledge: Default::default(),
            controllers: vec![],
        };
        let proposal = Proposal {
            source_basis: current.id,
            task: TaskKey::new("custom", "stop", "one"),
            actors: vec![1],
            order: BotOrder {
                content: stop,
                target: BotTarget::None,
                queue: BotQueueMode::Replace,
                group: 0,
            },
            priority: 1,
            goal_order: 0,
        };
        let verified = ObservationVerifier {
            candidate: &candidate,
            current: &current,
            profile: &profile,
        }
        .verify(current.id, &proposal)
        .expect("a global ordinary order must remain available through the public bot API");
        assert_eq!(verified.members.len(), 1);
        assert_eq!(verified.members[0].unit, 1);
    }
    fn empty_catalog() -> ContentCatalog {
        ContentCatalog {
            terrain_materials: Default::default(),
            terrain_movement_classes: Default::default(),
            terrain_tools: Default::default(),
            bots: BTreeMap::new(),
            combat: None,
            resources: BTreeMap::new(),
            units: BTreeMap::new(),
            nodes: BTreeMap::new(),
            production: BTreeMap::new(),
            recipes: BTreeMap::new(),
            technologies: BTreeMap::new(),
            orders: BTreeMap::new(),
            order_queue_depths: BTreeMap::new(),
            source_files: BTreeMap::new(),
        }
    }
    fn basis() -> ObservationBasis {
        let content = ContentId::parse("fixture:unit").unwrap();
        let units = (1..=3)
            .map(|unit| UnitObservation {
                ground_elevation: None,
                unit,
                owner_slot: 0,
                content: content.clone(),
                position: WorldPos {
                    x: unit as f32,
                    y: 2.0,
                },
                radius: 0.5,
                capabilities: Vec::new(),
                health: None,
                site: false,
                own: Some(OwnUnitObservation {
                    enabled: true,
                    activity: OwnActivity::Idle,
                    current_order: None,
                    queued_orders: 0,
                }),
            })
            .collect();
        ObservationBasis {
            terrain: None,
            id: 7,
            tick: 5,
            slot: 0,
            teams: vec![0, 1],
            units,
            nodes: vec![],
            accounts: vec![],
            account_index: BTreeMap::new(),
            unit_index: BTreeMap::new(),
            works: vec![],
            pending_content: BTreeMap::new(),
            supply: vec![],
            knowledge: None,
            area: ObservedArea {
                min: WorldPos { x: 0.0, y: 0.0 },
                max: WorldPos { x: 20.0, y: 20.0 },
                complete: true,
            },
            catalog: Arc::new(empty_catalog()),
        }
    }
    #[test]
    fn remembered_queries_are_separate_bounded_and_pinned_to_the_candidate_basis() {
        use turts_mod_api::{
            KnowledgeBasis, KnowledgeSnapshot, ObservationPhase, RememberedUnit, VisibilityGrid,
        };
        let mut current = basis();
        let mut knowledge = Arc::new(KnowledgeSnapshot {
            basis: KnowledgeBasis {
                match_id: 1,
                map: ContentId::parse("fixture:map").unwrap(),
                tick: current.tick,
                phase: ObservationPhase::Completed,
            },
            viewer: 0,
            grid: VisibilityGrid {
                size: WorldPos { x: 20.0, y: 20.0 },
                cell_size: 1.0,
                columns: 20,
                rows: 20,
            },
            observed: vec![0; 50],
            explored: vec![255; 50],
            remembered_units: [(8, 8.0), (7, 7.0)]
                .into_iter()
                .map(|(unit, x)| RememberedUnit {
                    ground_elevation: None,
                    unit,
                    content: ContentId::parse("fixture:enemy").unwrap(),
                    owner: 1,
                    position: WorldPos { x, y: 4.0 },
                    facing: None,
                    site: false,
                    last_seen_tick: 2,
                })
                .collect(),
            remembered_nodes: vec![],
            retention_truncated: true,
        });
        current.knowledge = Some(knowledge.clone());
        current.area.complete = false;
        let mut lease = CandidateLease::new(current, 0);
        Arc::make_mut(&mut knowledge).remembered_units[0].position.x = 19.0;
        let mut q = lease.queries(
            "memory",
            BotQuota {
                queries: 4,
                candidates: 4,
                rows: 4,
                proposals: 0,
            },
        );
        let first = q
            .query(
                Query::RememberedUnits(UnitQuery {
                    relation: turts_mod_api::SlotRelation::Any,
                    content: vec![],
                    center: None,
                    radius: None,
                }),
                None,
                1,
            )
            .unwrap();
        assert_eq!(first.coverage, Coverage::Complete);
        assert_eq!(first.usage.candidates, 1);
        assert_eq!(first.completed_tick, 5);
        let QueryData::RememberedUnits(first_rows) = first.data else {
            panic!()
        };
        assert!(first_rows.retention_truncated);
        assert_eq!(first_rows.rows[0].unit, 7);
        assert_eq!(first_rows.rows[0].last_seen_tick, 2);
        let PageState::More(cursor) = first.state else {
            panic!()
        };
        let second = q
            .query(
                Query::RememberedUnits(UnitQuery {
                    relation: turts_mod_api::SlotRelation::Any,
                    content: vec![],
                    center: None,
                    radius: None,
                }),
                Some(cursor),
                1,
            )
            .unwrap();
        let QueryData::RememberedUnits(second_rows) = second.data else {
            panic!()
        };
        assert_eq!(
            second_rows.rows[0].position.x, 8.0,
            "a newer memory cannot change a pinned lease"
        );
        assert_eq!(second.state, PageState::Complete);
        let lookup = q.query(Query::LookupUnit(7), None, 1).unwrap();
        assert_eq!(lookup.data, QueryData::Lookup(Advice::NotObserved));
        assert_eq!(lookup.coverage, Coverage::NotObserved);
    }
    #[test]
    fn a_supply_goal_accepts_one_indivisible_provider_crossing_the_target() {
        assert_eq!(provider_cap(1, 5), Ok(5));
        assert_eq!(provider_cap(7, 3), Ok(9));
        assert_eq!(
            provider_cap(u64::MAX, 2),
            Err(ProposalRejection::ArithmeticOverflow)
        );
        assert_eq!(
            provider_cap(1, 0),
            Err(ProposalRejection::InvalidFulfillment)
        );
    }
    #[test]
    fn a_candidate_pin_and_cursor_survive_other_query_kinds_and_new_steps() {
        let mut lease = CandidateLease::new(basis(), 0);
        let query = Query::Units(UnitQuery::own(vec![]));
        let cursor = {
            let mut q = lease.queries(
                "opening",
                BotQuota {
                    queries: 2,
                    candidates: 4,
                    rows: 2,
                    proposals: 0,
                },
            );
            q.pin_basis(7).unwrap();
            let page = q.query(query.clone(), None, 1).unwrap();
            let PageState::More(cursor) = page.state else {
                panic!("first page must be partial")
            };
            assert!(matches!(
                q.query(Query::Works, None, 1).unwrap().state,
                PageState::Complete
            ));
            cursor
        };
        assert!(lease.retained());
        assert!(!lease.expired(7));
        let mut q = lease.queries(
            "opening",
            BotQuota {
                queries: 2,
                candidates: 4,
                rows: 2,
                proposals: 0,
            },
        );
        let page = q.query(query, Some(cursor), 1).unwrap();
        let QueryData::Units(rows) = page.data else {
            panic!("unit page")
        };
        assert_eq!(
            rows[0].unit, 2,
            "switching query kind must not restart the earlier scan"
        );
        q.release_basis(7).unwrap();
        assert!(!lease.retained());
        assert!(lease.expired(8));
        let mut q = lease.queries("opening", operator_test_quota());
        assert_eq!(q.pin_basis(6), Err(QueryError::ExpiredBasis));
    }
    fn operator_test_quota() -> BotQuota {
        BotQuota {
            queries: 8,
            candidates: 32,
            rows: 16,
            proposals: 0,
        }
    }
    #[test]
    fn filtered_candidates_count_and_exact_tie_pages_remain_unresolved() {
        let mut facts = basis();
        facts.units[1].position = facts.units[0].position;
        let mut lease = CandidateLease::new(facts, 0);
        let mut queries = lease.queries("selection", operator_test_quota());
        let first = queries
            .query(Query::Units(UnitQuery::own(vec![])), None, 1)
            .unwrap();
        assert!(matches!(first.state, PageState::UnresolvedTie(_)));
        let mut query = UnitQuery::own(vec![ContentId::parse("fixture:missing").unwrap()]);
        query.center = None;
        let empty = queries.query(Query::Units(query), None, 1).unwrap();
        assert_eq!(
            empty.usage.candidates, 3,
            "rejected permitted candidates still consume work"
        );
        assert_eq!(empty.usage.rows, 0);
        assert_eq!(empty.state, PageState::Complete);
    }
    #[test]
    fn quota_exhaustion_does_not_produce_a_complete_prefix() {
        let mut lease = CandidateLease::new(basis(), 0);
        let mut query = lease.queries(
            "limited",
            BotQuota {
                queries: 2,
                candidates: 1,
                rows: 1,
                proposals: 0,
            },
        );
        let kind = Query::Units(UnitQuery::own(vec![]));
        let first = query.query(kind.clone(), None, 64).unwrap();
        let PageState::More(cursor) = first.state else {
            panic!("quota-limited result is incomplete")
        };
        assert_eq!(query.query(kind, Some(cursor), 64), Err(QueryError::Budget));
    }
}
