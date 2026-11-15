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
    let orders_by_unit = view
        .unit_orders
        .iter()
        .map(|order| (order.unit_id, order))
        .collect::<std::collections::BTreeMap<_, _>>();
    let health_by_unit = view
        .m3
        .raw_healths
        .iter()
        .map(|health| (health.unit, health.current))
        .collect::<std::collections::BTreeMap<_, _>>();
    let working = view
        .economy
        .queues
        .iter()
        .filter(|queue| !queue.jobs.is_empty())
        .map(|queue| queue.producer)
        .collect::<std::collections::BTreeSet<_>>();
    let own_ids = view
        .units
        .iter()
        .filter(|u| u.owner_slot == slot)
        .map(|u| u.unit_id)
        .collect::<std::collections::BTreeSet<_>>();
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
            let order = orders_by_unit.get(&unit.unit_id).copied();
            let activity = order.map_or(OwnActivity::Idle, |o| match o.activity {
                UnitOrderActivityView::Idle => OwnActivity::Idle,
                UnitOrderActivityView::Pending => OwnActivity::Pending,
                UnitOrderActivityView::Moving => OwnActivity::Moving,
                UnitOrderActivityView::Attacking => OwnActivity::Attacking,
                UnitOrderActivityView::Holding => OwnActivity::Holding,
                UnitOrderActivityView::Gathering => OwnActivity::Gathering,
                UnitOrderActivityView::Blocked => OwnActivity::Blocked,
            });
            let has_work = working.contains(&unit.unit_id)
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
            health_by_unit
                .get(&unit.unit_id)
                .map(|current| (*current, h.max))
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
    let owned = |id| own_ids.contains(&id);
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
    // Empty own accepted accounts are known zero, not unobserved endpoints.
    let mut known = accounts
        .iter()
        .map(|a| a.account.clone())
        .collect::<std::collections::BTreeSet<_>>();
    for unit in units.iter().filter(|u| u.owner_slot == slot) {
        let Some(definition) = catalog.units.get(&unit.content) else {
            continue;
        };
        for (definition, port) in [
            (definition.inventory.as_ref(), InventoryPort::Main),
            (definition.output_inventory.as_ref(), InventoryPort::Output),
        ] {
            let Some(definition) = definition else {
                continue;
            };
            for resource in &definition.accepts {
                let account = SpendKey {
                    endpoint: SourceEndpoint::Inventory {
                        unit: unit.unit,
                        port,
                    },
                    resource: resource.clone(),
                };
                if known.insert(account.clone()) {
                    accounts.push(AccountObservation {
                        account,
                        on_hand: 0,
                        reserved: 0,
                    });
                }
            }
        }
    }
    let exact_jobs = view
        .operating
        .as_ref()
        .into_iter()
        .flat_map(|op| op.jobs.iter())
        .map(|job| ((job.unit, job.job), job))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut works = Vec::new();
    for queue in view.economy.queues.iter().filter(|q| q.owner_slot == slot) {
        for job in &queue.jobs {
            let Some(recipe) = catalog.production.get(&job.recipe) else {
                continue;
            };
            let exact = exact_jobs.get(&(queue.producer, job.id)).copied();
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
    let mut pending_content = std::collections::BTreeMap::new();
    for (_, orders) in view
        .construction
        .held
        .iter()
        .filter(|(actor, _)| owned(**actor))
    {
        for order in orders {
            if let Some(recipe) = catalog.production.get(&order.recipe) {
                *pending_content
                    .entry(recipe.output_unit.clone())
                    .or_insert(0u64) += 1;
            }
        }
    }
    let supply = view
        .operating
        .as_ref()
        .into_iter()
        .flat_map(|op| op.supply.iter())
        .filter(|s| s.slot == slot)
        .map(|s| SupplyObservation {
            resource: s.resource.clone(),
            provided: s.provided,
            consumed: s.consumed,
        })
        .collect();
    let area = ObservationArea {
        min: WorldPos { x: 0.0, y: 0.0 },
        max: map_size,
    };
    let mut basis = ObservationBasis {
        id,
        tick: view.tick,
        slot,
        teams,
        units,
        nodes,
        accounts,
        account_index: Default::default(),
        unit_index: Default::default(),
        works,
        pending_content,
        supply,
        area: ObservedArea {
            min: area.min,
            max: area.max,
            complete: permissions.area_observed(slot, area),
        },
        catalog,
    };
    basis.index();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bots::observation::CandidateLease;
    struct Restricted;
    impl ObservationPermissions for Restricted {
        fn unit_visible(&self, _: u8, _: u32, _: u8) -> bool {
            false
        }
        fn node_visible(&self, _: u8, _: u32) -> bool {
            false
        }
        fn area_observed(&self, _: u8, _: ObservationArea) -> bool {
            false
        }
    }
    #[test]
    fn hidden_present_and_absent_worlds_have_identical_query_rows_and_metadata() {
        use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/base");
        let archive = PackageArchive::parse(&pack_dir(&root).unwrap()).unwrap();
        let mut app = turts_sim::headless_app(20);
        turts_pack::load_package(app.world_mut(), &archive).unwrap();
        turts_sim::initialize_completed_world(app.world_mut());
        let view = turts_sim::completed_world(app.world()).clone();
        let hidden = view
            .units
            .iter()
            .find(|u| u.owner_slot != 0)
            .expect("fixture has a foreign unit")
            .unit_id;
        let actor = view
            .units
            .iter()
            .find(|u| u.owner_slot == 0)
            .unwrap()
            .unit_id;
        let mut absent = view.clone();
        absent.units.retain(|u| u.owner_slot == 0);
        absent.nodes.clear();
        absent.economy.balances.retain(|a| a.slot == 0);
        absent.economy.inventories.retain(|a| a.owner_slot == 0);
        let catalog = Arc::new(archive.content().unwrap());
        let recipe = catalog.production.keys().next().unwrap().clone();
        let map = WorldPos { x: 200.0, y: 200.0 };
        let mut present = CandidateLease::new(
            project(&view, 0, 9, catalog.clone(), vec![0, 1], map, &Restricted).basis,
            0,
        );
        let mut absent = CandidateLease::new(
            project(&absent, 0, 9, catalog, vec![0, 1], map, &Restricted).basis,
            0,
        );
        let quota = crate::bots::operator_quota();
        let mut a = present.queries("tester", quota);
        let mut b = absent.queries("tester", quota);
        for query in [
            Query::Units(UnitQuery {
                relation: turts_mod_api::SlotRelation::Any,
                content: vec![],
                center: None,
                radius: None,
            }),
            Query::Nodes(NodeQuery {
                resources: vec![],
                center: None,
                radius: None,
            }),
            Query::LookupUnit(hidden),
            Query::Placement(PlacementQuery {
                actor,
                production: recipe.clone(),
                point: WorldPos { x: 20.0, y: 20.0 },
            }),
            Query::Affordability(AffordabilityQuery {
                actor: hidden,
                production: recipe,
            }),
        ] {
            assert_eq!(
                a.query(query.clone(), None, 64),
                b.query(query, None, 64),
                "private records cannot change a row, count, cursor, or coverage"
            );
        }
        let missing = a.query(Query::LookupUnit(u32::MAX), None, 1).unwrap();
        assert!(matches!(
            missing.data,
            QueryData::Lookup(Advice::NotObserved)
        ));
    }
}
