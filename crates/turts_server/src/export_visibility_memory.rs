//! Retained heap accounting for the permitted wire baseline, including nested capacities.
use turts_net_protocol as w;
fn vec_bytes<T>(rows: &Vec<T>, nested: impl Fn(&T) -> usize) -> usize {
    rows.capacity()
        .saturating_mul(std::mem::size_of::<T>())
        .saturating_add(rows.iter().map(nested).fold(0usize, usize::saturating_add))
}
fn strings(rows: &Vec<String>) -> usize {
    vec_bytes(rows, String::capacity)
}
fn costs(rows: &Vec<turts_mod_api::Cost>) -> usize {
    vec_bytes(rows, |row| row.resource.capacity())
}
fn rate(row: &w::RateRecord) -> usize {
    costs(&row.charged).saturating_add(match &row.state {
        w::RecordState::WaitingForFunds(resource) => resource.capacity(),
        _ => 0,
    })
}
fn order(row: &w::OrderSummary) -> usize {
    row.order.capacity().saturating_add(match &row.target {
        w::OrderTarget::Content(content) | w::OrderTarget::ContentAtPoint { content, .. } => {
            content.capacity()
        }
        _ => 0,
    })
}
pub(super) fn baseline(value: &super::PermittedBaseline) -> usize {
    let s = &value.state;
    let mut total = std::mem::size_of_val(value);
    let fields = [
        vec_bytes(&s.units, |row| row.content_id.capacity()),
        vec_bytes(&s.nodes, |row| row.content_id.capacity()),
        vec_bytes(&s.economy.balances, |row| row.resource.capacity()),
        vec_bytes(&s.economy.inventories, |row| {
            vec_bytes(&row.items, |item| item.resource.capacity())
        }),
        vec_bytes(&s.economy.orders, |_| 0),
        vec_bytes(&s.economy.queues, |row| {
            vec_bytes(&row.jobs, |job| {
                job.recipe
                    .capacity()
                    .saturating_add(costs(&job.charged))
                    .saturating_add(costs(&job.reserved))
            })
        }),
        vec_bytes(&s.belts, |row| {
            vec_bytes(&row.slots, |slot| slot.as_ref().map_or(0, String::capacity))
        }),
        vec_bytes(&s.inserters, |_| 0),
        vec_bytes(&s.extractors, |_| 0),
        vec_bytes(&s.weapons, |_| 0),
        vec_bytes(&s.healths, |_| 0),
        vec_bytes(&s.research, |row| {
            strings(&row.armour_order).saturating_add(strings(&row.completed))
        }),
        vec_bytes(&s.unit_orders, |row| {
            row.current
                .as_ref()
                .map_or(0, order)
                .saturating_add(vec_bytes(&row.queued, order))
                .saturating_add(
                    row.last_failure
                        .as_ref()
                        .map_or(0, |failure| failure.order.capacity()),
                )
        }),
        vec_bytes(&s.records.repairs, |row| match &row.state {
            w::RepairState::WaitingForFunds(resource) => resource.capacity(),
            _ => 0,
        }),
        vec_bytes(&s.records.sites, |row| {
            row.production
                .capacity()
                .saturating_add(rate(&row.progress))
        }),
        vec_bytes(&s.records.jobs, |row| rate(&row.progress)),
        vec_bytes(&s.records.construction_queues, |row| {
            vec_bytes(&row.orders, |order| order.content.capacity())
        }),
        vec_bytes(&s.records.supply, |row| row.resource.capacity()),
        s.records.match_state.as_ref().map_or(0, |row| {
            vec_bytes(&row.connections, |_| 0).saturating_add(vec_bytes(&row.outcomes, |_| 0))
        }),
    ];
    for bytes in fields {
        total = total.saturating_add(bytes);
    }
    if let Some(k) = &value.knowledge {
        total = total
            .saturating_add(k.basis.map.package.0.capacity())
            .saturating_add(k.basis.map.local.capacity())
            .saturating_add(k.observed.capacity())
            .saturating_add(k.explored.capacity())
            .saturating_add(vec_bytes(&k.remembered_units, |row| {
                row.content
                    .package
                    .0
                    .capacity()
                    .saturating_add(row.content.local.capacity())
            }))
            .saturating_add(vec_bytes(&k.remembered_nodes, |row| {
                row.content
                    .package
                    .0
                    .capacity()
                    .saturating_add(row.content.local.capacity())
            }));
    }
    total
}
