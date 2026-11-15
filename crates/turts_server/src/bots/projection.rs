//! Host-only projection. Permission checks run before public rows or query indexes exist.
use super::observation::{ObservationBasis, ObservedArea};
use crate::permissions::{ObservationArea, ObservationPermissions, unit_permitted};
use std::sync::Arc;
use turts_bot::*;
use turts_mod_api::{Capability, ContentCatalog, ContentId, CostSource, WorldPos};
use turts_sim::{
    WorldView,
    economy::{
        production::JobStage,
        views::{CostView, ReservationEndpointView, UnitOrderActivityView, UnitOrderTargetView},
    },
};

pub(crate) struct Projection {
    pub basis: ObservationBasis,
    /// Private host work is charged separately. It never enters QueryPage metadata.
    pub private_scanned: usize,
}
pub(crate) fn project(
    view: &WorldView,
    slot: u8,
    id: BasisId,
    catalog: Arc<ContentCatalog>,
    teams: Vec<u8>,
    map_size: WorldPos,
    permissions: &dyn ObservationPermissions,
) -> Projection {
    let mut units = Vec::new();
    for unit in &view.units {
        if !unit_permitted(permissions, slot, unit.unit_id, unit.owner_slot) {
            continue;
        }
        let Ok(content) = ContentId::parse(&unit.content_id) else {
            continue;
        };
        let Some(def) = catalog.units.get(&content) else {
            continue;
        };
        let own = if unit.owner_slot == slot {
            let order = view.unit_orders.iter().find(|o| o.unit_id == unit.unit_id);
            let activity = order.map_or(OwnActivity::Idle, |o| match o.activity {
                UnitOrderActivityView::Idle => OwnActivity::Idle,
                UnitOrderActivityView::Pending => OwnActivity::Pending,
                UnitOrderActivityView::Moving => OwnActivity::Moving,
                UnitOrderActivityView::Attacking => OwnActivity::Attacking,
                UnitOrderActivityView::Holding => OwnActivity::Holding,
                UnitOrderActivityView::Gathering => OwnActivity::Gathering,
                UnitOrderActivityView::Blocked => OwnActivity::Blocked,
            });
            let has_work = view
                .economy
                .queues
                .iter()
                .any(|q| q.producer == unit.unit_id && !q.jobs.is_empty())
                || view
                    .construction
                    .held
                    .get(&unit.unit_id)
                    .is_some_and(|q| !q.is_empty());
            Some(OwnUnitObservation {
                enabled: !view.m3.disabled.contains(&unit.unit_id),
                activity: if has_work && activity == OwnActivity::Idle {
                    OwnActivity::Working
                } else {
                    activity
                },
                current_order: order.and_then(|o| o.current.as_ref()).and_then(|o| {
                    Some(BotOrder {
                        content: ContentId::parse(&o.order).ok()?,
                        target: target(&o.target),
                        queue: BotQueueMode::Replace,
                        group: 0,
                    })
                }),
                queued_orders: order
                    .map_or(0, |o| u32::try_from(o.queued.len()).unwrap_or(u32::MAX)),
            })
        } else {
            None
        };
        let radius = match def.footprint {
            turts_mod_api::Footprint::Circle { radius } => radius,
        };
        let capabilities = [
            (def.movement.is_some(), Capability::Movement),
            (def.weapon.is_some(), Capability::Weapon),
            (def.gatherer.is_some(), Capability::Gatherer),
            (def.producer.is_some(), Capability::Producer),
            (def.research.is_some(), Capability::Research),
            (def.constructor.is_some(), Capability::Constructor),
            (def.repairer.is_some(), Capability::Repairer),
        ]
        .into_iter()
        .filter_map(|(yes, c)| yes.then_some(c))
        .collect();
        let health = def.health.as_ref().and_then(|h| {
            view.m3
                .raw_healths
                .iter()
                .find(|h| h.unit == unit.unit_id)
                .map(|raw| (raw.current, h.max))
        });
        units.push(UnitObservation {
            unit: unit.unit_id,
            owner_slot: unit.owner_slot,
            content,
            position: WorldPos {
                x: unit.pos.x,
                y: unit.pos.y,
            },
            radius,
            capabilities,
            health,
            site: view.construction.sites.contains_key(&unit.unit_id),
            own,
        });
    }
    let nodes = view
        .nodes
        .iter()
        .filter(|n| permissions.node_visible(slot, n.id.0))
        .filter_map(|n| {
            let def = catalog.nodes.get(&n.content_id)?;
            Some(NodeObservation {
                node: n.id.0,
                content: n.content_id.clone(),
                item: def.item.clone(),
                position: n.pos,
                radius: def.radius,
                remaining: n.remaining,
            })
        })
        .collect();
    let owned = |id| {
        view.units
            .iter()
            .any(|u| u.unit_id == id && u.owner_slot == slot)
    };
    let mut accounts = view
        .economy
        .balances
        .iter()
        .filter(|b| b.slot == slot)
        .map(|b| AccountObservation {
            account: SpendKey {
                endpoint: SourceEndpoint::SlotBalance(slot),
                resource: b.resource.clone(),
            },
            on_hand: b.on_hand,
            reserved: b.reserved,
        })
        .collect::<Vec<_>>();
    for inventory in view
        .economy
        .inventories
        .iter()
        .filter(|i| i.owner_slot == slot)
    {
        accounts.extend(inventory.items.iter().map(|item| AccountObservation {
            account: SpendKey {
                endpoint: SourceEndpoint::Inventory {
                    unit: inventory.unit,
                    port: InventoryPort::Main,
                },
                resource: item.resource.clone(),
            },
            on_hand: item.on_hand,
            reserved: item.reserved,
        }));
    }
    for inventory in view
        .m3
        .raw_output_inventories
        .iter()
        .filter(|i| owned(i.unit))
    {
        accounts.extend(
            inventory
                .items
                .iter()
                .map(|(resource, item)| AccountObservation {
                    account: SpendKey {
                        endpoint: SourceEndpoint::Inventory {
                            unit: inventory.unit,
                            port: InventoryPort::Output,
                        },
                        resource: resource.clone(),
                    },
                    on_hand: item.on_hand,
                    reserved: item.reserved,
                }),
        );
    }
    let mut works = Vec::new();
    for queue in view.economy.queues.iter().filter(|q| q.owner_slot == slot) {
        for job in &queue.jobs {
            let Some(recipe) = catalog.production.get(&job.recipe) else {
                continue;
            };
            let exact = view.operating.as_ref().and_then(|op| {
                op.jobs
                    .iter()
                    .find(|j| j.unit == queue.producer && j.job == job.id)
            });
            let funding = exact.and_then(|e| e.funding.as_ref());
            let mut state = match job.stage {
                JobStage::Queued => WorkState::Queued,
                JobStage::Running => WorkState::Running,
                JobStage::OutputBlocked => WorkState::OutputBlocked,
            };
            if let Some(resource) = funding.and_then(|f| f.waiting_for_funds.as_ref()) {
                state = WorkState::WaitingForFunds(resource.clone())
            } else if view.m3.disabled.contains(&queue.producer) || exact.is_some_and(|e| e.held) {
                state = WorkState::Paused
            }
            works.push(WorkObservation {
                identity: WorkIdentity {
                    unit: queue.producer,
                    id: job.id,
                    kind: WorkKind::Production,
                },
                actor_unit: queue.producer,
                content: Some(recipe.output_unit.clone()),
                payment: Some(recipe.payment),
                done: exact.map_or(job.elapsed_ticks, |j| j.done),
                total: exact.map_or(job.duration_ticks, |j| j.total),
                paid: costs(slot, queue.producer, &job.charged),
                reserved: costs(slot, queue.producer, &job.reserved),
                state,
            });
        }
    }
    for (id, site) in view
        .construction
        .sites
        .iter()
        .filter(|(_, s)| s.owner_slot == slot)
    {
        let Some(recipe) = catalog.production.get(&site.recipe) else {
            continue;
        };
        let state = if let Some(resource) = site
            .funding
            .as_ref()
            .and_then(|f| f.waiting_for_funds.as_ref())
        {
            WorkState::WaitingForFunds(resource.clone())
        } else if view.m3.disabled.contains(&site.constructor) {
            WorkState::Paused
        } else {
            WorkState::Running
        };
        let paid = site
            .charged_rows()
            .into_iter()
            .map(|line| AccountAmount {
                account: SpendKey {
                    endpoint: match line.endpoint {
                        ReservationEndpointView::Slot(slot) => SourceEndpoint::SlotBalance(slot),
                        ReservationEndpointView::Inventory(unit) => SourceEndpoint::Inventory {
                            unit,
                            port: InventoryPort::Main,
                        },
                        ReservationEndpointView::Output(unit) => SourceEndpoint::Inventory {
                            unit,
                            port: InventoryPort::Output,
                        },
                    },
                    resource: line.resource,
                },
                amount: line.amount,
            })
            .collect();
        works.push(WorkObservation {
            identity: WorkIdentity {
                unit: *id,
                id: site.job.0,
                kind: WorkKind::Construction,
            },
            actor_unit: site.constructor,
            content: Some(recipe.output_unit.clone()),
            payment: Some(recipe.payment),
            done: site.done,
            total: site.total,
            paid,
            reserved: Vec::new(),
            state,
        });
    }
    let area = ObservationArea {
        min: WorldPos { x: 0.0, y: 0.0 },
        max: map_size,
    };
    let basis = ObservationBasis {
        id,
        tick: view.tick,
        slot,
        teams,
        units,
        nodes,
        accounts,
        works,
        area: ObservedArea {
            min: area.min,
            max: area.max,
            complete: permissions.area_observed(slot, area),
        },
        catalog,
    };
    Projection {
        basis,
        private_scanned: view.units.len()
            + view.nodes.len()
            + view.economy.balances.len()
            + view.economy.inventories.len()
            + view.economy.queues.len()
            + view.construction.sites.len(),
    }
}
fn costs(slot: u8, actor: u32, rows: &[CostView]) -> Vec<AccountAmount> {
    rows.iter()
        .map(|row| AccountAmount {
            account: SpendKey {
                endpoint: match row.source {
                    CostSource::SlotBalance => SourceEndpoint::SlotBalance(slot),
                    CostSource::ProducerInventory => SourceEndpoint::Inventory {
                        unit: actor,
                        port: InventoryPort::Main,
                    },
                },
                resource: row.resource.clone(),
            },
            amount: row.amount,
        })
        .collect()
}
fn target(value: &UnitOrderTargetView) -> BotTarget {
    match value {
        UnitOrderTargetView::None => BotTarget::None,
        UnitOrderTargetView::Point(p) => BotTarget::Point(*p),
        UnitOrderTargetView::Unit(id) => BotTarget::Unit(*id),
        UnitOrderTargetView::Node(id) => BotTarget::Node(id.0),
        UnitOrderTargetView::Job(id) => BotTarget::Job(id.0),
        UnitOrderTargetView::Content(content) => BotTarget::Content(content.clone()),
        UnitOrderTargetView::ContentAtPoint { content, point } => BotTarget::ContentAtPoint {
            content: content.clone(),
            point: *point,
        },
    }
}
