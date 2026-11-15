//! Game-independent stock policies. All names and thresholds come from BotKnowledge.
mod squads;
use crate::*;
use std::sync::Arc;
use turts_mod_api::{
    BotControllerDef, BotDef, BotGoalAction, BotRecipeBinding, BotRecipeKind, BotValue, ContentId,
    SlotRelation, WorldPos,
};

#[derive(Clone, Copy)]
enum Policy {
    Opening,
    Economy,
    Production,
    Squads,
}
struct StockFactory(Policy);
/// Register the four portable stock factories. No profile is activated by registration.
pub fn register_stock(registry: &mut ControllerRegistry) -> Result<(), FactoryError> {
    for (name, policy) in [
        ("opening", Policy::Opening),
        ("economy", Policy::Economy),
        ("production", Policy::Production),
        ("squads", Policy::Squads),
    ] {
        registry.register(
            ContentId::parse(&format!("engine:{name}")).expect("stock factory identifier"),
            Arc::new(StockFactory(policy)),
        )?;
    }
    Ok(())
}
impl ControllerFactory for StockFactory {
    fn validate(&self, _: &BotDef, instance: &BotControllerDef) -> Result<(), FactoryError> {
        if instance.config != BotValue::Null {
            return Err(FactoryError::new(
                "config",
                "stock controllers take their configuration from knowledge; use Null",
            ));
        }
        Ok(())
    }
    fn create(&self, init: ControllerInit<'_>) -> Result<Box<dyn BotController>, FactoryError> {
        self.validate(init.profile, init.instance)?;
        Ok(Box::new(StockController {
            key: init.instance.key.clone(),
            policy: self.0,
            scans: Vec::new(),
            basis: None,
            placement_steps: std::collections::BTreeMap::new(),
        }))
    }
}

struct Scan {
    query: Query,
    cursor: Option<QueryCursor>,
    units: Vec<UnitObservation>,
    nodes: Vec<NodeObservation>,
    complete: Option<QueryData>,
}
struct StockController {
    key: String,
    policy: Policy,
    scans: Vec<Scan>,
    basis: Option<BasisId>,
    placement_steps: std::collections::BTreeMap<(u32, String), usize>,
}
impl StockController {
    /// Never select a prefix. Incomplete pages retain their exact immutable-basis cursor.
    fn collect(&mut self, query: Query, queries: &mut dyn BotQueries) -> Option<QueryData> {
        let index = self
            .scans
            .iter()
            .position(|s| s.query == query)
            .unwrap_or_else(|| {
                self.scans.push(Scan {
                    query: query.clone(),
                    cursor: None,
                    units: Vec::new(),
                    nodes: Vec::new(),
                    complete: None,
                });
                self.scans.len() - 1
            });
        loop {
            let scan = &mut self.scans[index];
            if let Some(data) = &scan.complete {
                return Some(data.clone());
            }
            let page = match queries.query(query.clone(), scan.cursor.clone(), 64) {
                Ok(page) => page,
                Err(QueryError::Budget) => return None,
                Err(_) => {
                    self.scans.remove(index);
                    return None;
                }
            };
            if page.coverage != Coverage::Complete {
                self.scans.remove(index);
                return None;
            }
            match page.data {
                QueryData::Units(mut rows) => scan.units.append(&mut rows),
                QueryData::Nodes(mut rows) => scan.nodes.append(&mut rows),
                data => {
                    scan.complete = Some(data.clone());
                    return Some(data);
                }
            }
            match page.state {
                PageState::Complete => {
                    let data = if matches!(query, Query::Nodes(_)) {
                        QueryData::Nodes(std::mem::take(&mut scan.nodes))
                    } else {
                        QueryData::Units(std::mem::take(&mut scan.units))
                    };
                    scan.complete = Some(data.clone());
                    return Some(data);
                }
                PageState::More(cursor) | PageState::UnresolvedTie(cursor) => {
                    scan.cursor = Some(cursor)
                }
            }
        }
    }
    fn units(
        &mut self,
        frame: &BotFrame<'_>,
        role: &str,
        queries: &mut dyn BotQueries,
    ) -> Option<Vec<UnitObservation>> {
        let role = frame
            .profile
            .knowledge
            .roles
            .iter()
            .find(|r| r.key == role)?;
        let content = role
            .units
            .iter()
            .map(|s| ContentId::parse(s).ok())
            .collect::<Option<Vec<_>>>()?;
        match self.collect(Query::Units(UnitQuery::own(content)), queries)? {
            QueryData::Units(rows) => Some(rows.into_iter().filter(|u| !u.site).collect()),
            _ => None,
        }
    }
    fn order(&self, frame: &BotFrame<'_>, binding: &str, target: BotTarget) -> Option<BotOrder> {
        let binding = frame
            .profile
            .knowledge
            .orders
            .iter()
            .find(|o| o.key == binding)?;
        Some(BotOrder {
            content: ContentId::parse(&binding.order).ok()?,
            target,
            queue: BotQueueMode::Replace,
            group: 0,
        })
    }
    // Keep the authored goal/rank/priority and output ports explicit at this boundary.
    #[allow(clippy::too_many_arguments)]
    fn propose(
        &self,
        frame: &BotFrame<'_>,
        goal: &str,
        rank: usize,
        priority: u32,
        actor: &UnitObservation,
        order: BotOrder,
        out: &mut dyn ProposalSink,
    ) {
        let _ = out.propose(Proposal {
            source_basis: self.basis.unwrap_or(frame.basis),
            task: TaskKey::new(&self.key, goal, "desired"),
            actors: vec![actor.unit],
            order,
            priority,
            goal_order: rank as u32,
        });
    }
    // The query and proposal ports have distinct budgets and must remain explicit.
    #[allow(clippy::too_many_arguments)]
    fn build(
        &mut self,
        frame: &BotFrame<'_>,
        goal: &str,
        rank: usize,
        priority: u32,
        recipe_key: &str,
        queries: &mut dyn BotQueries,
        out: &mut dyn ProposalSink,
    ) {
        let Some(recipe) = frame
            .profile
            .knowledge
            .recipes
            .iter()
            .find(|r| r.key == recipe_key)
            .cloned()
        else {
            return;
        };
        let Some(actors) = self.units(frame, &recipe.actor_role, queries) else {
            return;
        };
        // Physical order is semantic. Exact coincident sources acquire neither action.
        let Some(actor) = unique_first(actors.into_iter().filter(idle).collect()) else {
            return;
        };
        let Ok(content) = ContentId::parse(&recipe.recipe) else {
            return;
        };
        let target = match recipe.kind {
            BotRecipeKind::Produce => BotTarget::Content(content),
            BotRecipeKind::Construct => {
                let Some(point) = self.placement(&actor, &recipe, queries) else {
                    return;
                };
                BotTarget::ContentAtPoint { content, point }
            }
        };
        if let Some(order) = self.order(frame, &recipe.order, target) {
            self.propose(frame, goal, rank, priority, &actor, order, out);
        }
    }
    fn placement(
        &mut self,
        actor: &UnitObservation,
        recipe: &BotRecipeBinding,
        queries: &mut dyn BotQueries,
    ) -> Option<WorldPos> {
        let production = ContentId::parse(&recipe.recipe).ok()?;
        // This is a geometric search, not game knowledge. Public advice checks footprint,
        // authored build areas, nodes, and permitted occupancy before an ordinary request.
        let key = (actor.unit, recipe.key.clone());
        let mut step = self.placement_steps.get(&key).copied().unwrap_or(0);
        let offsets = [
            (1., 0.),
            (0., 1.),
            (-1., 0.),
            (0., -1.),
            (1., 1.),
            (-1., 1.),
            (-1., -1.),
            (1., -1.),
        ];
        for _ in 0..32 {
            let ring = step / 8 + 1;
            let (x, y) = offsets[step % 8];
            let distance = actor.radius + 3.0 * ring as f32;
            let point = WorldPos {
                x: actor.position.x + x * distance,
                y: actor.position.y + y * distance,
            };
            let query = Query::Placement(PlacementQuery {
                actor: actor.unit,
                production: production.clone(),
                point,
            });
            match self.collect(query, queries) {
                Some(QueryData::Placement(Advice::Known(PlacementAdvice::Allowed))) => {
                    self.placement_steps.remove(&key);
                    return Some(point);
                }
                None => {
                    self.placement_steps.insert(key, step);
                    return None;
                }
                _ => {
                    step = (step + 1) % 32;
                    self.placement_steps.insert(key.clone(), step);
                }
            }
        }
        None
    }
    fn goal(
        &mut self,
        frame: &BotFrame<'_>,
        goal: &turts_mod_api::BotGoal,
        rank: usize,
        queries: &mut dyn BotQueries,
        out: &mut dyn ProposalSink,
    ) {
        match &goal.action {
            BotGoalAction::EnsureUnits {
                role,
                count,
                recipe,
            } => {
                let have = frame
                    .role_counts
                    .iter()
                    .find(|r| &r.role == role)
                    .map(|r| u64::from(r.completed) + u64::from(r.pending))
                    .unwrap_or(0);
                if have < u64::from(*count) {
                    self.build(frame, &goal.key, rank, goal.priority, recipe, queries, out);
                }
            }
            BotGoalAction::EnsureSupply {
                resource,
                amount,
                provider_role,
                recipe,
            } => {
                let Ok(resource) = ContentId::parse(resource) else {
                    return;
                };
                let Some(supply) = frame.supply.iter().find(|s| s.resource == resource) else {
                    return;
                };
                if frame
                    .role_counts
                    .iter()
                    .any(|r| &r.role == provider_role && r.pending > 0)
                {
                    return;
                }
                if supply.provided.saturating_sub(supply.consumed) < *amount {
                    self.build(frame, &goal.key, rank, goal.priority, recipe, queries, out);
                }
            }
            BotGoalAction::Gather {
                role,
                resource,
                order,
            } => {
                let Some(units) = self.units(frame, role, queries) else {
                    return;
                };
                let Ok(resource) = ContentId::parse(resource) else {
                    return;
                };
                let Some(QueryData::Nodes(nodes)) = self.collect(
                    Query::Nodes(NodeQuery {
                        resources: vec![resource],
                        center: None,
                        radius: None,
                    }),
                    queries,
                ) else {
                    return;
                };
                let units = ranked_sources(
                    units.into_iter().filter(idle).collect(),
                    frame.quota.proposals as usize,
                );
                for (i, actor) in units.iter().enumerate() {
                    let mut nodes = nodes.iter().filter(|n| n.remaining > 0).collect::<Vec<_>>();
                    nodes.sort_by(|a, b| {
                        distance(actor.position, a.position)
                            .total_cmp(&distance(actor.position, b.position))
                    });
                    let Some(node) = nodes.first() else { continue };
                    if nodes.get(1).is_some_and(|n| {
                        distance(actor.position, n.position)
                            == distance(actor.position, node.position)
                    }) {
                        continue;
                    }
                    if let Some(order) = self.order(frame, order, BotTarget::Node(node.node)) {
                        let _ = out.propose(Proposal {
                            source_basis: self.basis.unwrap_or(frame.basis),
                            task: TaskKey::new(
                                &self.key,
                                &goal.key,
                                format!("actor/{}", actor.unit),
                            ),
                            actors: vec![actor.unit],
                            order,
                            priority: goal.priority,
                            goal_order: rank as u32,
                        });
                    }
                    if i + 1 >= frame.quota.proposals as usize {
                        break;
                    }
                }
            }
        }
    }
}
impl BotController for StockController {
    fn step(
        &mut self,
        frame: &BotFrame<'_>,
        queries: &mut dyn BotQueries,
        out: &mut dyn ProposalSink,
    ) {
        let basis = self.basis.unwrap_or(frame.basis);
        if queries.pin_basis(basis).is_err() {
            self.scans.clear();
            self.placement_steps.clear();
            self.basis = None;
            return;
        }
        self.basis = Some(basis);
        match self.policy {
            Policy::Opening => {
                for (rank, goal) in frame.profile.knowledge.opening.clone().iter().enumerate() {
                    self.goal(frame, goal, rank, queries, out);
                }
            }
            Policy::Economy => {
                for (rank, goal) in frame.profile.knowledge.recovery.clone().iter().enumerate() {
                    self.goal(frame, goal, rank, queries, out);
                }
            }
            Policy::Production => {
                for (rank, target) in frame
                    .profile
                    .knowledge
                    .production
                    .clone()
                    .iter()
                    .enumerate()
                {
                    self.goal(
                        frame,
                        &turts_mod_api::BotGoal {
                            key: target.key.clone(),
                            priority: target.priority,
                            action: BotGoalAction::EnsureUnits {
                                role: target.role.clone(),
                                count: target.count,
                                recipe: target.recipe.clone(),
                            },
                        },
                        rank,
                        queries,
                        out,
                    );
                }
            }
            Policy::Squads => self.squads(frame, queries, out),
        }
        if self.scans.iter().all(|s| s.complete.is_some()) {
            let _ = queries.release_basis(basis);
            self.scans.clear();
            self.basis = None;
        }
    }
}
fn distance(a: WorldPos, b: WorldPos) -> f32 {
    (a.x - b.x).powi(2) + (a.y - b.y).powi(2)
}
fn unique_first(mut rows: Vec<UnitObservation>) -> Option<UnitObservation> {
    rows.sort_by(|a, b| {
        a.position
            .x
            .total_cmp(&b.position.x)
            .then_with(|| a.position.y.total_cmp(&b.position.y))
            .then_with(|| a.content.cmp(&b.content))
    });
    let first = rows.first()?;
    if rows
        .get(1)
        .is_some_and(|other| other.position == first.position)
    {
        return None;
    }
    rows.into_iter().next()
}

fn idle(unit: &UnitObservation) -> bool {
    unit.own.as_ref().is_some_and(|own| {
        own.enabled
            && own.queued_orders == 0
            && matches!(own.activity, OwnActivity::Idle | OwnActivity::Blocked)
    })
}

#[cfg(test)]
mod tests;

/// Select only resolved physical ranks. Coincident sources are refused as a group,
/// including at the quota boundary. Identity never selects a prefix within a tie.
fn ranked_sources(mut rows: Vec<UnitObservation>, limit: usize) -> Vec<UnitObservation> {
    rows.sort_by(|a, b| {
        a.position
            .x
            .total_cmp(&b.position.x)
            .then_with(|| a.position.y.total_cmp(&b.position.y))
    });
    let mut selected = Vec::new();
    let mut start = 0;
    while start < rows.len() && selected.len() < limit {
        let mut end = start + 1;
        while end < rows.len() && rows[end].position == rows[start].position {
            end += 1;
        }
        if end == start + 1 {
            selected.push(rows[start].clone());
        }
        start = end;
    }
    selected
}
