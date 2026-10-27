use crate::{
    economy::{
        EconomyOrders, SurfaceNodes,
        ledger::EconomyLedger,
        production::{ProductionJob, ProductionQueues},
    },
    ids::{IdAllocators, NodeId},
    slots::Slots,
    world::{MapBounds, MoveTarget, Owner, Position, UnitId, UnitType},
};
use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};
use turts_mod_api::{ContentId, CostSource, PaymentPolicy, ResourceKind, WorldPos};

mod world_pos_bits {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use turts_mod_api::WorldPos;
    pub fn serialize<S: Serializer>(p: &WorldPos, s: S) -> Result<S::Ok, S::Error> {
        [p.x.to_bits(), p.y.to_bits()].serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<WorldPos, D::Error> {
        let [x, y] = <[u32; 2]>::deserialize(d)?;
        Ok(WorldPos {
            x: f32::from_bits(x),
            y: f32::from_bits(y),
        })
    }
}
mod optional_world_pos_bits {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use turts_mod_api::WorldPos;
    pub fn serialize<S: Serializer>(p: &Option<WorldPos>, s: S) -> Result<S::Ok, S::Error> {
        p.as_ref()
            .map(|p| [p.x.to_bits(), p.y.to_bits()])
            .serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<WorldPos>, D::Error> {
        Ok(Option::<[u32; 2]>::deserialize(d)?.map(|[x, y]| WorldPos {
            x: f32::from_bits(x),
            y: f32::from_bits(y),
        }))
    }
}
mod optional_producer_bits {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use turts_mod_api::{ContentId, WorldPos};
    type ProducerBits = Option<(Vec<ContentId>, u16, Vec<[u32; 2]>)>;
    type ProducerValue = Option<(Vec<ContentId>, u16, Vec<WorldPos>)>;
    pub fn serialize<S: Serializer>(v: &ProducerValue, s: S) -> Result<S::Ok, S::Error> {
        let bits = v.as_ref().map(|(a, b, c)| {
            (
                a,
                b,
                c.iter()
                    .map(|p| [p.x.to_bits(), p.y.to_bits()])
                    .collect::<Vec<_>>(),
            )
        });
        bits.serialize(s)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<ProducerValue, D::Error> {
        Ok(ProducerBits::deserialize(d)?.map(|(a, b, c)| {
            (
                a,
                b,
                c.into_iter()
                    .map(|[x, y]| WorldPos {
                        x: f32::from_bits(x),
                        y: f32::from_bits(y),
                    })
                    .collect(),
            )
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceView {
    pub slot: u8,
    pub resource: ContentId,
    pub on_hand: u64,
    pub reserved: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemView {
    pub resource: ContentId,
    pub on_hand: u64,
    pub reserved: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryView {
    pub unit: u32,
    pub owner_slot: u8,
    pub capacity: u64,
    pub items: Vec<ItemView>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderView {
    pub unit: u32,
    pub owner_slot: u8,
    pub node: Option<NodeId>,
    pub receiver: Option<u32>,
    pub stage: super::gather::GatherStage,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CostView {
    pub source: CostSource,
    pub resource: ContentId,
    pub amount: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobView {
    pub id: u64,
    pub recipe: ContentId,
    pub stage: crate::economy::production::JobStage,
    pub elapsed_ticks: u64,
    pub duration_ticks: u64,
    pub charged: Vec<CostView>,
    pub reserved: Vec<CostView>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueView {
    pub producer: u32,
    pub owner_slot: u8,
    pub jobs: Vec<JobView>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeView {
    pub id: NodeId,
    pub content_id: ContentId,
    #[serde(with = "world_pos_bits")]
    pub pos: WorldPos,
    pub remaining: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitTypeView {
    pub type_id: u32,
    pub content_id: String,
    pub movement_speed: Option<f32>,
    pub radius: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitView {
    pub unit_id: u32,
    pub content_id: String,
    pub owner_slot: u8,
    #[serde(with = "world_pos_bits")]
    pub pos: WorldPos,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReservationEndpointView {
    Slot(u8),
    Inventory(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReservationLineView {
    pub endpoint: ReservationEndpointView,
    pub resource: ContentId,
    pub amount: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReservationClaimView {
    Active {
        map_job_id: u64,
        token_job_id: u64,
        lines: Vec<ReservationLineView>,
    },
    Closed {
        map_job_id: u64,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditResourcePresence {
    pub bounds: bool,
    pub catalog: bool,
    pub durations: bool,
    pub ledger: bool,
    pub nodes: bool,
    pub orders: bool,
    pub queues: bool,
    pub unit_types: bool,
    pub slots: bool,
    pub clock: bool,
    pub allocators: bool,
    pub tick: bool,
    pub pending_events: bool,
    pub event_ordinal: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawUnitAudit {
    pub unit_id: u32,
    pub unit_type: u32,
    pub owner_slot: u8,
    #[serde(with = "world_pos_bits")]
    pub pos: WorldPos,
    #[serde(with = "optional_world_pos_bits")]
    pub move_target: Option<WorldPos>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawInventoryAudit {
    pub unit: u32,
    pub capacity: u64,
    pub accepted_items: Vec<ContentId>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawOrderAudit {
    pub unit: u32,
    pub node: Option<NodeId>,
    pub deposit: Option<NodeId>,
    pub receiver: Option<u32>,
    pub stage: super::gather::GatherStage,
    pub rate_carry: u16,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawReservationAudit {
    pub job_id: u64,
    pub lines: Vec<ReservationLineView>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawJobAudit {
    pub id: u64,
    pub recipe: ContentId,
    pub stage: crate::economy::production::JobStage,
    pub elapsed_ticks: u64,
    pub duration_ticks: u64,
    pub charged: Vec<ReservationLineView>,
    pub reservation: Option<RawReservationAudit>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawQueueAudit {
    pub producer: u32,
    pub jobs: Vec<RawJobAudit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurationAudit {
    pub recipe: ContentId,
    pub ticks: u64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogAudit {
    pub resource_kinds: Vec<(ContentId, ResourceKind)>,
    pub nodes: Vec<CatalogNodeAudit>,
    pub units: Vec<CatalogUnitAudit>,
    pub recipes: Vec<CatalogRecipeAudit>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogNodeAudit {
    pub id: ContentId,
    pub item: ContentId,
    pub radius: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogUnitAudit {
    pub id: ContentId,
    pub radius: f32,
    pub movement_speed: Option<f32>,
    pub inventory: Option<(u64, Vec<ContentId>)>,
    pub gatherer: Option<CatalogGathererAudit>,
    pub drop_off: Vec<CatalogDropOffAudit>,
    #[serde(with = "optional_producer_bits")]
    pub producer: Option<(Vec<ContentId>, u16, Vec<WorldPos>)>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogGathererAudit {
    pub items: Vec<ContentId>,
    pub gather_per_second: u64,
    pub unload_per_second: u64,
    pub interaction_range: f32,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogDropOffAudit {
    pub item: ContentId,
    pub effect: CatalogDropOffEffectAudit,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CatalogDropOffEffectAudit {
    Store,
    Convert { balance: ContentId, per_item: u64 },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogRecipeAudit {
    pub id: ContentId,
    pub output_unit: ContentId,
    pub duration_ms: u64,
    pub costs: Vec<CostView>,
    pub payment: PaymentPolicy,
    pub refunds: turts_mod_api::CancelRefunds,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditState {
    pub tick_hz: Option<u16>,
    #[serde(with = "optional_world_pos_bits")]
    pub bounds: Option<WorldPos>,
    pub resources: AuditResourcePresence,
    pub raw_units: Vec<RawUnitAudit>,
    pub raw_inventories: Vec<RawInventoryAudit>,
    pub raw_orders: Vec<RawOrderAudit>,
    pub raw_queues: Vec<RawQueueAudit>,
    pub compiled_durations: Vec<DurationAudit>,
    pub next_event_ordinal: Option<u64>,
    pub pending_events: Vec<super::events::EconomyEvent>,
    pub catalog: Option<CatalogAudit>,
    pub ledger_claims: Vec<ReservationClaimView>,
    pub unit_types: Vec<UnitTypeView>,
    pub units: Vec<UnitView>,
    pub nodes: Vec<NodeView>,
    pub economy: WorldView,
    pub allocators: IdAllocators,
    pub claims: Vec<Option<u64>>,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EconomyStateView {
    pub balances: Vec<BalanceView>,
    pub inventories: Vec<InventoryView>,
    pub orders: Vec<OrderView>,
    pub queues: Vec<QueueView>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EconomyChangesView {
    pub balances: Vec<BalanceView>,
    pub inventories: Vec<InventoryView>,
    pub orders: Vec<OrderView>,
    pub queues: Vec<QueueView>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct WorldView {
    pub tick: u32,
    pub units: Vec<crate::world::UnitState>,
    pub nodes: Vec<NodeView>,
    pub economy: EconomyStateView,
}

impl std::ops::Deref for WorldView {
    type Target = EconomyStateView;
    fn deref(&self) -> &Self::Target {
        &self.economy
    }
}

impl std::ops::DerefMut for WorldView {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.economy
    }
}

pub fn world_view(world: &mut World) -> WorldView {
    let economy = world
        .get_resource::<EconomyLedger>()
        .cloned()
        .unwrap_or_default();
    let orders = world
        .get_resource::<EconomyOrders>()
        .cloned()
        .unwrap_or_default();
    let queues = world
        .get_resource::<ProductionQueues>()
        .cloned()
        .unwrap_or_default();
    let owners = owner_map(world);
    let tick = world
        .get_resource::<crate::world::Tick>()
        .map_or(0, |tick| tick.0);
    let units = crate::world::snapshot(world);
    let mut view = WorldView {
        tick,
        units,
        ..WorldView::default()
    };
    for ((slot, resource), account) in economy.balances {
        view.balances.push(BalanceView {
            slot,
            resource,
            on_hand: account.on_hand,
            reserved: account.reserved,
        });
    }
    for (unit, inventory) in economy.inventories {
        let owner_slot = owners.get(&unit).copied().unwrap_or_default();
        view.inventories.push(InventoryView {
            unit,
            owner_slot,
            capacity: inventory.capacity,
            items: inventory
                .items
                .into_iter()
                .map(|(resource, account)| ItemView {
                    resource,
                    on_hand: account.on_hand,
                    reserved: account.reserved,
                })
                .collect(),
        });
    }
    for (unit, order) in orders.0 {
        view.orders.push(OrderView {
            unit,
            owner_slot: owners.get(&unit).copied().unwrap_or_default(),
            node: order.node,
            receiver: order.receiver,
            stage: order.stage,
        });
    }
    for (producer, jobs) in queues.0 {
        view.queues.push(QueueView {
            producer,
            owner_slot: owners.get(&producer).copied().unwrap_or_default(),
            jobs: jobs.into_iter().map(job_view).collect(),
        });
    }
    view.nodes = node_views(world);
    view
}

/// Return absolute records that differ from the previous completed economy snapshot.
pub(crate) fn economy_changes(
    previous: &EconomyStateView,
    current: &EconomyStateView,
) -> EconomyChangesView {
    fn replacements<T: Clone + PartialEq, K: Ord>(
        previous: &[T],
        current: &[T],
        key: impl Fn(&T) -> K,
    ) -> Vec<T> {
        let previous: std::collections::BTreeMap<_, _> = previous
            .iter()
            .map(|record| (key(record), record))
            .collect();
        current
            .iter()
            .filter(|record| match previous.get(&key(record)) {
                Some(old) => *old != *record,
                None => true,
            })
            .cloned()
            .collect()
    }

    let mut changes = EconomyChangesView {
        balances: replacements(&previous.balances, &current.balances, |v| {
            (v.slot, v.resource.clone())
        }),
        inventories: replacements(&previous.inventories, &current.inventories, |v| v.unit),
        orders: replacements(&previous.orders, &current.orders, |v| v.unit),
        queues: replacements(&previous.queues, &current.queues, |v| v.producer),
    };
    changes
        .balances
        .sort_by_key(|record| (record.slot, record.resource.clone()));
    changes.inventories.sort_by_key(|record| record.unit);
    changes.orders.sort_by_key(|record| record.unit);
    changes.queues.sort_by_key(|record| record.producer);
    changes
}
fn job_view(job: ProductionJob) -> JobView {
    JobView {
        id: job.id.0,
        recipe: job.recipe,
        stage: job.stage,
        elapsed_ticks: job.elapsed_ticks,
        duration_ticks: job.duration_ticks,
        charged: job
            .charged
            .into_iter()
            .map(|line| CostView {
                source: match line.endpoint {
                    crate::economy::ledger::Endpoint::Slot(_) => CostSource::SlotBalance,
                    crate::economy::ledger::Endpoint::Inventory(_) => CostSource::ProducerInventory,
                },
                resource: line.resource,
                amount: line.amount,
            })
            .collect(),
        reserved: job.reservation.map_or_else(Vec::new, |r| {
            r.lines
                .into_iter()
                .map(|line| CostView {
                    source: match line.endpoint {
                        crate::economy::ledger::Endpoint::Slot(_) => CostSource::SlotBalance,
                        crate::economy::ledger::Endpoint::Inventory(_) => {
                            CostSource::ProducerInventory
                        }
                    },
                    resource: line.resource,
                    amount: line.amount,
                })
                .collect()
        }),
    }
}
fn owner_map(world: &mut World) -> std::collections::BTreeMap<u32, u8> {
    let mut result = std::collections::BTreeMap::new();
    let mut q = world.query::<(&UnitId, &Owner)>();
    for (unit, owner) in q.iter(world) {
        result.insert(unit.0, owner.0.0);
    }
    result
}
fn node_views(world: &World) -> Vec<NodeView> {
    world
        .get_resource::<SurfaceNodes>()
        .map_or_else(Vec::new, |nodes| {
            nodes
                .0
                .iter()
                .map(|n| NodeView {
                    id: n.id,
                    content_id: n.content_id.clone(),
                    pos: WorldPos {
                        x: n.position.0.x,
                        y: n.position.0.y,
                    },
                    remaining: n.remaining,
                })
                .collect()
        })
}
fn reservation_line_view(line: crate::economy::ledger::ReservationLine) -> ReservationLineView {
    ReservationLineView {
        endpoint: match line.endpoint {
            crate::economy::ledger::Endpoint::Slot(slot) => ReservationEndpointView::Slot(slot),
            crate::economy::ledger::Endpoint::Inventory(unit) => {
                ReservationEndpointView::Inventory(unit)
            }
        },
        resource: line.resource,
        amount: line.amount,
    }
}

fn raw_queue_view((producer, jobs): (u32, Vec<ProductionJob>)) -> RawQueueAudit {
    RawQueueAudit {
        producer,
        jobs: jobs
            .into_iter()
            .map(|job| RawJobAudit {
                id: job.id.0,
                recipe: job.recipe,
                stage: job.stage,
                elapsed_ticks: job.elapsed_ticks,
                duration_ticks: job.duration_ticks,
                charged: job.charged.into_iter().map(reservation_line_view).collect(),
                reservation: job.reservation.map(|reservation| RawReservationAudit {
                    job_id: reservation.job_id.0,
                    lines: reservation
                        .lines
                        .into_iter()
                        .map(reservation_line_view)
                        .collect(),
                }),
            })
            .collect(),
    }
}

pub fn audit_state(world: &mut World) -> AuditState {
    let mut q = world.query::<(&UnitId, &UnitType, &Owner, &Position)>();
    let types = world.get_resource::<crate::UnitTypes>();
    let mut units: Vec<_> = q
        .iter(world)
        .map(|(id, kind, owner, pos)| UnitView {
            unit_id: id.0,
            content_id: types
                .and_then(|types| types.get(kind.0))
                .map_or_else(String::new, |t| t.content_id.clone()),
            owner_slot: owner.0.0,
            pos: WorldPos {
                x: pos.0.x,
                y: pos.0.y,
            },
        })
        .collect();
    units.sort_by_key(|unit| unit.unit_id);
    let nodes = node_views(world);
    let unit_types = world
        .get_resource::<crate::UnitTypes>()
        .map_or_else(Vec::new, |types| {
            types
                .entries()
                .iter()
                .enumerate()
                .map(|(type_id, entry)| UnitTypeView {
                    type_id: type_id as u32,
                    content_id: entry.content_id.clone(),
                    movement_speed: entry.movement.as_ref().map(|movement| movement.speed),
                    radius: entry.radius,
                })
                .collect()
        });
    let economy = world_view(world);
    let allocators = *world
        .get_resource::<IdAllocators>()
        .unwrap_or(&IdAllocators {
            next_unit: 1,
            next_node: 1,
            next_job: 1,
        });
    let claims = world
        .get_resource::<Slots>()
        .map_or_else(Vec::new, Slots::owners);
    let ledger_claims = world
        .get_resource::<EconomyLedger>()
        .map_or_else(Vec::new, |ledger| {
            ledger
                .reservation_claims
                .iter()
                .map(|(map_job_id, claim)| match claim {
                    crate::economy::ledger::ReservationClaim::Active(token) => {
                        ReservationClaimView::Active {
                            map_job_id: map_job_id.0,
                            token_job_id: token.job_id.0,
                            lines: token
                                .lines
                                .iter()
                                .map(|line| ReservationLineView {
                                    endpoint: match line.endpoint {
                                        crate::economy::ledger::Endpoint::Slot(slot) => {
                                            ReservationEndpointView::Slot(slot)
                                        }
                                        crate::economy::ledger::Endpoint::Inventory(unit) => {
                                            ReservationEndpointView::Inventory(unit)
                                        }
                                    },
                                    resource: line.resource.clone(),
                                    amount: line.amount,
                                })
                                .collect(),
                        }
                    }
                    crate::economy::ledger::ReservationClaim::Closed => {
                        ReservationClaimView::Closed {
                            map_job_id: map_job_id.0,
                        }
                    }
                })
                .collect()
        });
    let catalog_present = world.contains_resource::<crate::bootstrap::InstalledCatalog>();
    let catalog = world
        .get_resource::<crate::bootstrap::InstalledCatalog>()
        .map(|installed| {
            let c = &installed.0;
            CatalogAudit {
                resource_kinds: c
                    .resources
                    .iter()
                    .map(|(id, d)| (id.clone(), d.kind))
                    .collect(),
                nodes: c
                    .nodes
                    .iter()
                    .map(|(id, d)| CatalogNodeAudit {
                        id: id.clone(),
                        item: d.item.clone(),
                        radius: d.radius,
                    })
                    .collect(),
                units: c
                    .units
                    .iter()
                    .map(|(id, d)| CatalogUnitAudit {
                        id: id.clone(),
                        radius: match d.footprint {
                            turts_mod_api::Footprint::Circle { radius } => radius,
                        },
                        movement_speed: d.movement.as_ref().map(|m| m.speed),
                        inventory: d
                            .inventory
                            .as_ref()
                            .map(|i| (i.capacity, i.accepts.clone())),
                        gatherer: d.gatherer.as_ref().map(|g| CatalogGathererAudit {
                            items: g.items.clone(),
                            gather_per_second: g.gather_per_second,
                            unload_per_second: g.unload_per_second,
                            interaction_range: g.interaction_range,
                        }),
                        drop_off: d.drop_off.as_ref().map_or_else(Vec::new, |d| {
                            d.rules
                                .iter()
                                .map(|r| CatalogDropOffAudit {
                                    item: r.item.clone(),
                                    effect: match &r.effect {
                                        turts_mod_api::ResolvedDropOffEffect::Store => {
                                            CatalogDropOffEffectAudit::Store
                                        }
                                        turts_mod_api::ResolvedDropOffEffect::Convert {
                                            balance,
                                            per_item,
                                        } => CatalogDropOffEffectAudit::Convert {
                                            balance: balance.clone(),
                                            per_item: *per_item,
                                        },
                                    },
                                })
                                .collect()
                        }),
                        producer: d.producer.as_ref().map(|p| {
                            (p.recipes.clone(), p.queue_capacity, p.spawn_offsets.clone())
                        }),
                    })
                    .collect(),
                recipes: c
                    .production
                    .iter()
                    .map(|(id, r)| CatalogRecipeAudit {
                        id: id.clone(),
                        output_unit: r.output_unit.clone(),
                        duration_ms: r.duration_ms,
                        costs: r
                            .costs
                            .iter()
                            .map(|c| CostView {
                                source: c.source,
                                resource: c.resource.clone(),
                                amount: c.amount,
                            })
                            .collect(),
                        payment: r.payment,
                        refunds: r.cancel_refunds,
                    })
                    .collect(),
            }
        });
    let mut raw_units = Vec::new();
    let mut raw_query =
        world.query::<(&UnitId, &UnitType, &Owner, &Position, Option<&MoveTarget>)>();
    raw_units.extend(
        raw_query
            .iter(world)
            .map(|(id, kind, owner, pos, target)| RawUnitAudit {
                unit_id: id.0,
                unit_type: kind.0.0,
                owner_slot: owner.0.0,
                pos: WorldPos {
                    x: pos.0.x,
                    y: pos.0.y,
                },
                move_target: target.map(|t| WorldPos { x: t.0.x, y: t.0.y }),
            }),
    );
    raw_units.sort_by_key(|u| u.unit_id);
    let raw_inventories = world
        .get_resource::<EconomyLedger>()
        .map_or_else(Vec::new, |ledger| {
            ledger
                .inventories
                .iter()
                .map(|(unit, inv)| RawInventoryAudit {
                    unit: *unit,
                    capacity: inv.capacity,
                    accepted_items: inv.accepted_items.iter().cloned().collect(),
                })
                .collect()
        });
    let raw_orders = world
        .get_resource::<EconomyOrders>()
        .map_or_else(Vec::new, |orders| {
            orders
                .0
                .iter()
                .map(|(unit, o)| RawOrderAudit {
                    unit: *unit,
                    node: o.node,
                    deposit: o.deposit,
                    receiver: o.receiver,
                    stage: o.stage,
                    rate_carry: o.carry.remainder(),
                })
                .collect()
        });
    let compiled_durations = world
        .get_resource::<crate::bootstrap::ProductionDurations>()
        .map_or_else(Vec::new, |d| {
            d.0.iter()
                .map(|(recipe, ticks)| DurationAudit {
                    recipe: recipe.clone(),
                    ticks: *ticks,
                })
                .collect()
        });
    let resources = AuditResourcePresence {
        bounds: world.contains_resource::<MapBounds>(),
        catalog: catalog_present,
        durations: world.contains_resource::<crate::bootstrap::ProductionDurations>(),
        ledger: world.contains_resource::<EconomyLedger>(),
        nodes: world.contains_resource::<SurfaceNodes>(),
        orders: world.contains_resource::<EconomyOrders>(),
        queues: world.contains_resource::<ProductionQueues>(),
        unit_types: world.contains_resource::<crate::registry::UnitTypes>(),
        slots: world.contains_resource::<Slots>(),
        clock: world.contains_resource::<crate::SimClock>(),
        allocators: world.contains_resource::<IdAllocators>(),
        tick: world.contains_resource::<crate::world::Tick>(),
        pending_events: world.contains_resource::<super::events::PendingEconomyEvents>(),
        event_ordinal: world.contains_resource::<super::events::NextEventOrdinal>(),
    };
    AuditState {
        tick_hz: world.get_resource::<crate::SimClock>().map(|c| c.tick_hz),
        next_event_ordinal: world
            .get_resource::<super::events::NextEventOrdinal>()
            .and_then(|n| n.0),
        pending_events: world
            .get_resource::<super::events::PendingEconomyEvents>()
            .map_or_else(Vec::new, |e| e.0.clone()),
        bounds: world.get_resource::<MapBounds>().map(|b| WorldPos {
            x: b.size.x,
            y: b.size.y,
        }),
        resources,
        raw_units,
        raw_inventories,
        raw_orders,
        raw_queues: world
            .get_resource::<ProductionQueues>()
            .map_or_else(Vec::new, |queues| {
                queues
                    .0
                    .iter()
                    .map(|(producer, jobs)| raw_queue_view((*producer, jobs.clone())))
                    .collect()
            }),
        compiled_durations,
        catalog,
        ledger_claims,
        unit_types,
        units,
        nodes,
        economy,
        allocators,
        claims,
    }
}

/// Return an owned balance snapshot. The shared World borrow cannot expose writable ledger state.
pub fn balance_view(
    world: &World,
    slot: crate::SlotId,
    resource: &ContentId,
) -> Result<BalanceView, crate::RejectReason> {
    let ledger = world
        .get_resource::<EconomyLedger>()
        .ok_or(crate::RejectReason::UnknownBalance)?;
    let account = ledger
        .balances
        .get(&(slot.0, resource.clone()))
        .ok_or(crate::RejectReason::UnknownBalance)?;
    Ok(BalanceView {
        slot: slot.0,
        resource: resource.clone(),
        on_hand: account.on_hand,
        reserved: account.reserved,
    })
}

/// Return an owned inventory snapshot after checking the owner's slot first.
pub fn inventory_view(
    world: &World,
    slot: crate::SlotId,
    unit_id: u32,
) -> Result<InventoryView, crate::RejectReason> {
    let owner = world
        .iter_entities()
        .find_map(|entity| {
            (entity.get::<UnitId>().is_some_and(|unit| unit.0 == unit_id))
                .then(|| entity.get::<Owner>().map(|owner| owner.0))?
        })
        .ok_or(crate::RejectReason::UnknownInventory)?;
    if owner != slot {
        return Err(crate::RejectReason::NotOwner);
    }
    let inventory = world
        .get_resource::<EconomyLedger>()
        .and_then(|ledger| ledger.inventories.get(&unit_id))
        .ok_or(crate::RejectReason::UnknownInventory)?;
    Ok(InventoryView {
        unit: unit_id,
        owner_slot: owner.0,
        capacity: inventory.capacity,
        items: inventory
            .items
            .iter()
            .map(|(resource, account)| ItemView {
                resource: resource.clone(),
                on_hand: account.on_hand,
                reserved: account.reserved,
            })
            .collect(),
    })
}

/// Return an owned surface node snapshot.
pub fn node_view(world: &World, id: NodeId) -> Result<NodeView, crate::RejectReason> {
    world
        .get_resource::<SurfaceNodes>()
        .and_then(|nodes| nodes.0.iter().find(|node| node.id == id))
        .map(|node| NodeView {
            id: node.id,
            content_id: node.content_id.clone(),
            pos: WorldPos {
                x: node.position.0.x,
                y: node.position.0.y,
            },
            remaining: node.remaining,
        })
        .ok_or(crate::RejectReason::UnknownNode)
}

/// Return a producer queue snapshot after checking the producer's owner.
pub fn queue_view(
    world: &World,
    slot: crate::SlotId,
    unit_id: u32,
) -> Result<QueueView, crate::RejectReason> {
    let owner = unit_owner(world, unit_id)?;
    if owner != slot {
        return Err(crate::RejectReason::NotOwner);
    }
    let jobs = world
        .get_resource::<ProductionQueues>()
        .and_then(|queues| queues.0.get(&unit_id))
        .ok_or(crate::RejectReason::UnknownUnit)?;
    Ok(QueueView {
        producer: unit_id,
        owner_slot: owner.0,
        jobs: jobs.iter().cloned().map(job_view).collect(),
    })
}

/// Return an order snapshot after checking the unit's owner.
pub fn order_view(
    world: &World,
    slot: crate::SlotId,
    unit_id: u32,
) -> Result<OrderView, crate::RejectReason> {
    let owner = unit_owner(world, unit_id)?;
    if owner != slot {
        return Err(crate::RejectReason::NotOwner);
    }
    let order = world
        .get_resource::<EconomyOrders>()
        .and_then(|orders| orders.0.get(&unit_id))
        .ok_or(crate::RejectReason::UnknownUnit)?;
    Ok(OrderView {
        unit: unit_id,
        owner_slot: owner.0,
        node: order.node,
        receiver: order.receiver,
        stage: order.stage,
    })
}

fn unit_owner(world: &World, unit_id: u32) -> Result<crate::SlotId, crate::RejectReason> {
    world
        .iter_entities()
        .find_map(|entity| {
            (entity.get::<UnitId>().is_some_and(|unit| unit.0 == unit_id))
                .then(|| entity.get::<Owner>().map(|owner| owner.0))?
        })
        .ok_or(crate::RejectReason::UnknownUnit)
}
