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
    pub catalog: Arc<ContentCatalog>,
}
impl ObservationBasis {
    /// Only permitted rows enter these indexes. Their order cannot disclose rejected objects.
    pub fn index(&mut self) {
        self.units
            .sort_by(|a, b| physical(a.position, &a.content, b.position, &b.content));
        self.nodes
            .sort_by(|a, b| physical(a.position, &a.content, b.position, &b.content));
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
    fn team(&self, slot: u8) -> u8 {
        self.teams.get(usize::from(slot)).copied().unwrap_or(slot)
    }
    fn relation(&self, owner: u8, relation: SlotRelation) -> bool {
        match relation {
            SlotRelation::Same => owner == self.slot,
            SlotRelation::Ally => self.team(owner) == self.team(self.slot),
            SlotRelation::Other => owner != self.slot,
            SlotRelation::Enemy => self.team(owner) != self.team(self.slot),
            SlotRelation::Any => true,
        }
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
                .is_some_and(|p| p.buildings.contains(&recipe.output_unit))
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
    fn placement(&self, query: &PlacementQuery) -> Advice<PlacementAdvice> {
        // Partial observation must not consult an unseen collision or anchor.
        if !self.area.complete {
            return Advice::NotObserved;
        }
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
        let mut coverage = if basis.area.complete {
            Coverage::Complete
        } else {
            Coverage::NotObserved
        };
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
                (QueryData::Placement(basis.placement(request)), 1, 1)
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
            if !definition.units.contains(&unit.content.to_string()) {
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
                            && role.units.iter().any(|c| c == &unit.content.to_string())
                    })
                    .count() as u64;
                let pending = self
                    .current
                    .works
                    .iter()
                    .filter(|work| {
                        !work.state.terminal()
                            && work
                                .content
                                .as_ref()
                                .is_some_and(|c| role.units.iter().any(|r| r == &c.to_string()))
                    })
                    .count() as u64;
                let held = self
                    .current
                    .pending_content
                    .iter()
                    .filter(|(content, _)| role.units.iter().any(|r| r == &content.to_string()))
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
                let live = self
                    .current
                    .supply
                    .iter()
                    .find(|s| s.resource == resource)
                    .map_or(0, |s| s.provided.saturating_sub(s.consumed));
                let pending = self
                    .current
                    .works
                    .iter()
                    .filter(|work| !work.state.terminal())
                    .filter_map(|work| work.content.as_ref())
                    .chain(
                        self.current
                            .pending_content
                            .iter()
                            .flat_map(|(content, count)| {
                                std::iter::repeat_n(content, *count as usize)
                            }),
                    )
                    .filter_map(|content| self.current.catalog.units.get(content))
                    .flat_map(|unit| unit.provides.iter())
                    .filter(|s| s.resource == row.resource)
                    .try_fold(0u64, |sum, row| sum.checked_add(row.amount))
                    .ok_or(ProposalRejection::ArithmeticOverflow)?;
                let observed = live
                    .checked_add(pending)
                    .ok_or(ProposalRejection::ArithmeticOverflow)?;
                let cap = match goal {
                    Some(turts_mod_api::BotGoalAction::EnsureSupply {
                        resource: requested,
                        amount,
                        ..
                    }) if requested == &row.resource => Some(*amount),
                    _ => None,
                };
                fulfillment.push(FulfillmentClaim {
                    key: FulfillmentKey::Supply(resource),
                    observed,
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

fn role_matches(role: &turts_mod_api::BotRole, unit: &turts_mod_api::ResolvedUnit) -> bool {
    role.units.iter().any(|id| id == &unit.id.to_string())
        && role.requires.iter().all(|cap| match cap {
            turts_mod_api::Capability::Movement => unit.movement.is_some(),
            turts_mod_api::Capability::Weapon => unit.weapon.is_some(),
            turts_mod_api::Capability::Gatherer => unit.gatherer.is_some(),
            turts_mod_api::Capability::Producer => unit.producer.is_some(),
            turts_mod_api::Capability::Research => unit.research.is_some(),
            turts_mod_api::Capability::Constructor => unit.constructor.is_some(),
            turts_mod_api::Capability::Repairer => unit.repairer.is_some(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn empty_catalog() -> ContentCatalog {
        ContentCatalog {
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
            area: ObservedArea {
                min: WorldPos { x: 0.0, y: 0.0 },
                max: WorldPos { x: 20.0, y: 20.0 },
                complete: true,
            },
            catalog: Arc::new(empty_catalog()),
        }
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
