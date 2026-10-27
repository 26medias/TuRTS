use std::collections::{BTreeMap, BTreeSet};

use bevy_ecs::world::World;
use turts_mod_api::ContentId;

use crate::{
    Owner, RejectReason, SlotId, UnitId,
    economy::{
        events::{EconomyEvent, EconomyEventKind, NextEventOrdinal, PendingEconomyEvents},
        ledger::{
            Account, EconomyLedger, Endpoint, Inventory, Reservation, ReservationClaim,
            ReservationLine,
        },
        production::ProductionQueues,
    },
    ids::{IdAllocators, JobId},
    slots::Slots,
};

#[derive(Default)]
struct WriteSet {
    accounts: BTreeMap<(Endpoint, ContentId), Account>,
    claims: BTreeMap<JobId, ReservationClaim>,
    queues: Option<ProductionQueues>,
    next_job_id: Option<u64>,
    events: Vec<(u8, EconomyEventKind)>,
    next_event_ordinal: Option<u64>,
}

pub(crate) struct Transaction<'w> {
    world: &'w mut World,
    writes: WriteSet,
}

pub(crate) fn transact<R>(
    world: &mut World,
    run: impl FnOnce(&mut Transaction<'_>) -> Result<R, RejectReason>,
) -> Result<R, RejectReason> {
    let mut tx = Transaction {
        world,
        writes: WriteSet::default(),
    };
    let result = run(&mut tx)?;
    tx.commit();
    Ok(result)
}

impl Transaction<'_> {
    pub(crate) fn queues(&self) -> ProductionQueues {
        self.writes.queues.clone().unwrap_or_else(|| {
            self.world
                .get_resource::<ProductionQueues>()
                .cloned()
                .unwrap_or_default()
        })
    }

    pub(crate) fn stage_queues(&mut self, queues: ProductionQueues) {
        self.writes.queues = Some(queues);
    }

    pub(crate) fn allocate_job_id(&mut self) -> Result<JobId, RejectReason> {
        let next = self.writes.next_job_id.unwrap_or_else(|| {
            self.world
                .get_resource::<IdAllocators>()
                .map_or(1, |ids| ids.next_job)
        });
        self.writes.next_job_id = Some(next.checked_add(1).ok_or(RejectReason::IdExhausted)?);
        Ok(JobId(next))
    }

    pub(crate) fn stage_event(
        &mut self,
        owner_slot: u8,
        kind: EconomyEventKind,
    ) -> Result<(), RejectReason> {
        let ordinal = match self.writes.next_event_ordinal {
            Some(value) => value,
            None => self
                .world
                .get_resource::<NextEventOrdinal>()
                .and_then(|next| next.0)
                .ok_or(RejectReason::ArithmeticOverflow)?,
        };
        self.writes.next_event_ordinal = Some(
            ordinal
                .checked_add(1)
                .ok_or(RejectReason::ArithmeticOverflow)?,
        );
        self.writes.events.push((owner_slot, kind));
        Ok(())
    }
    pub(crate) fn account(&self, endpoint: Endpoint, resource: &ContentId) -> Account {
        if let Some(account) = self.writes.accounts.get(&(endpoint, resource.clone())) {
            return *account;
        }
        let Some(ledger) = self.world.get_resource::<EconomyLedger>() else {
            return Account::default();
        };
        match endpoint {
            Endpoint::Slot(slot) => ledger
                .balances
                .get(&(slot, resource.clone()))
                .copied()
                .unwrap_or_default(),
            Endpoint::Inventory(unit) => ledger
                .inventories
                .get(&unit)
                .and_then(|i| i.items.get(resource))
                .copied()
                .unwrap_or_default(),
        }
    }

    pub(crate) fn inventory(&self, unit: u32) -> Result<&Inventory, RejectReason> {
        self.world
            .get_resource::<EconomyLedger>()
            .and_then(|ledger| ledger.inventories.get(&unit))
            .ok_or(RejectReason::UnknownInventory)
    }

    pub(crate) fn inventory_used(&self, unit: u32) -> Result<u128, RejectReason> {
        let inventory = self.inventory(unit)?;
        let resources = inventory
            .items
            .keys()
            .cloned()
            .chain(
                self.writes
                    .accounts
                    .keys()
                    .filter(|(endpoint, _)| *endpoint == Endpoint::Inventory(unit))
                    .map(|(_, resource)| resource.clone()),
            )
            .collect::<BTreeSet<_>>();
        resources.into_iter().try_fold(0_u128, |total, resource| {
            total
                .checked_add(u128::from(
                    self.account(Endpoint::Inventory(unit), &resource).on_hand,
                ))
                .ok_or(RejectReason::ArithmeticOverflow)
        })
    }

    pub(crate) fn validate_endpoint(
        &self,
        endpoint: Endpoint,
        resource: &ContentId,
    ) -> Result<(), RejectReason> {
        match endpoint {
            Endpoint::Slot(slot) => {
                if !self
                    .world
                    .get_resource::<Slots>()
                    .is_some_and(|slots| slots.contains(SlotId(slot)))
                {
                    return Err(RejectReason::UnknownBalance);
                }
                let exists = self
                    .world
                    .get_resource::<EconomyLedger>()
                    .is_some_and(|ledger| ledger.balances.contains_key(&(slot, resource.clone())));
                if !exists {
                    return Err(RejectReason::UnknownBalance);
                }
            }
            Endpoint::Inventory(unit) => {
                let inventory = self.inventory(unit)?;
                if !inventory.accepted_items.contains(resource) {
                    return Err(RejectReason::IncompatibleItem);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn stage_account(
        &mut self,
        endpoint: Endpoint,
        resource: ContentId,
        account: Account,
    ) {
        self.writes.accounts.insert((endpoint, resource), account);
    }

    fn commit(self) {
        let Transaction { world, writes } = self;
        if world.get_resource::<EconomyLedger>().is_none() {
            world.insert_resource(EconomyLedger::default());
        }
        {
            let mut ledger = world.resource_mut::<EconomyLedger>();
            for ((endpoint, resource), account) in writes.accounts {
                match endpoint {
                    Endpoint::Slot(slot) => {
                        ledger.balances.insert((slot, resource), account);
                    }
                    Endpoint::Inventory(unit) => {
                        if let Some(inventory) = ledger.inventories.get_mut(&unit) {
                            inventory.items.insert(resource, account);
                        }
                    }
                }
            }
            ledger.reservation_claims.extend(writes.claims);
        }
        if let Some(queues) = writes.queues {
            world.insert_resource(queues);
        }
        if let Some(next_job_id) = writes.next_job_id {
            world.resource_mut::<IdAllocators>().next_job = next_job_id;
        }
        if let Some(next_event_ordinal) = writes.next_event_ordinal {
            let tick = world.resource::<crate::world::Tick>().0;
            let first = world
                .resource::<NextEventOrdinal>()
                .0
                .expect("event ordinal was preflighted before transaction commit");
            {
                let mut pending = world.resource_mut::<PendingEconomyEvents>();
                for (offset, (owner_slot, kind)) in writes.events.into_iter().enumerate() {
                    pending.0.push(EconomyEvent {
                        tick,
                        ordinal: first + offset as u64,
                        owner_slot,
                        kind,
                    });
                }
            }
            world.resource_mut::<NextEventOrdinal>().0 = Some(next_event_ordinal);
        }
    }
}

/// Transfer items between inventories owned by the caller after checking filters and capacity.
pub fn inventory_transfer(
    world: &mut World,
    slot: SlotId,
    from: u32,
    to: u32,
    item: ContentId,
    amount: u64,
) -> Result<(), RejectReason> {
    if amount == 0 {
        return Err(RejectReason::InvalidAmount);
    }
    let from_owner = inventory_owner(world, from)?;
    if from_owner != slot {
        return Err(RejectReason::NotOwner);
    }
    let to_owner = inventory_owner(world, to)?;
    if to_owner != slot {
        return Err(RejectReason::NotOwner);
    }
    if from == to {
        return Err(RejectReason::InvalidAmount);
    }
    transact(world, |tx| {
        tx.validate_endpoint(Endpoint::Inventory(from), &item)?;
        tx.validate_endpoint(Endpoint::Inventory(to), &item)?;
        let source = tx.account(Endpoint::Inventory(from), &item);
        let available = source
            .on_hand
            .checked_sub(source.reserved)
            .ok_or(RejectReason::ArithmeticOverflow)?;
        if available < amount {
            return Err(RejectReason::InsufficientItems);
        }
        let destination = tx.account(Endpoint::Inventory(to), &item);
        let used = tx.inventory_used(to)?;
        let new_used = used
            .checked_add(u128::from(amount))
            .ok_or(RejectReason::ArithmeticOverflow)?;
        if new_used > u128::from(tx.inventory(to)?.capacity) {
            return Err(RejectReason::RefundDestinationFull);
        }
        let new_source = source
            .on_hand
            .checked_sub(amount)
            .ok_or(RejectReason::ArithmeticOverflow)?;
        let new_destination = destination
            .on_hand
            .checked_add(amount)
            .ok_or(RejectReason::ArithmeticOverflow)?;
        tx.stage_account(
            Endpoint::Inventory(from),
            item.clone(),
            Account {
                on_hand: new_source,
                ..source
            },
        );
        tx.stage_account(
            Endpoint::Inventory(to),
            item,
            Account {
                on_hand: new_destination,
                ..destination
            },
        );
        Ok(())
    })
}

fn inventory_owner(world: &World, id: u32) -> Result<SlotId, RejectReason> {
    let owner = world
        .iter_entities()
        .find_map(|entity| {
            (entity.get::<UnitId>().is_some_and(|unit| unit.0 == id))
                .then(|| entity.get::<Owner>().map(|owner| owner.0))?
        })
        .ok_or(RejectReason::UnknownInventory)?;
    Ok(owner)
}

// Used by later transaction callers; these remain internal to the simulation crate.
#[allow(dead_code)]
pub(crate) fn reserve_costs(
    tx: &mut Transaction<'_>,
    job_id: JobId,
    lines: &[ReservationLine],
) -> Result<Reservation, RejectReason> {
    let canonical = validate_lines(lines)?;
    if tx
        .world
        .get_resource::<EconomyLedger>()
        .is_some_and(|ledger| ledger.reservation_claims.contains_key(&job_id))
        || tx.writes.claims.contains_key(&job_id)
    {
        return Err(RejectReason::UnknownJob);
    }
    let mut updates = BTreeMap::new();
    for line in &canonical {
        tx.validate_endpoint(line.endpoint, &line.resource)?;
        let key = (line.endpoint, line.resource.clone());
        let old = tx.account(line.endpoint, &line.resource);
        if old
            .on_hand
            .checked_sub(old.reserved)
            .ok_or(RejectReason::ArithmeticOverflow)?
            < line.amount
        {
            return Err(match line.endpoint {
                Endpoint::Slot(_) => RejectReason::InsufficientBalance,
                Endpoint::Inventory(_) => RejectReason::InsufficientItems,
            });
        }
        let reserved = old
            .reserved
            .checked_add(line.amount)
            .ok_or(RejectReason::ArithmeticOverflow)?;
        if reserved > old.on_hand {
            return Err(RejectReason::InsufficientItems);
        }
        updates.insert(key, Account { reserved, ..old });
    }
    for ((endpoint, resource), account) in updates {
        tx.stage_account(endpoint, resource, account);
    }
    let reservation = Reservation {
        job_id,
        lines: canonical,
    };
    tx.writes
        .claims
        .insert(job_id, ReservationClaim::Active(reservation.clone()));
    Ok(reservation)
}

// Used by later transaction callers; these remain internal to the simulation crate.
#[allow(dead_code)]
pub(crate) fn charge_costs(
    tx: &mut Transaction<'_>,
    lines: &[ReservationLine],
) -> Result<(), RejectReason> {
    let canonical = validate_lines(lines)?;
    let mut updates = BTreeMap::new();
    for line in canonical {
        tx.validate_endpoint(line.endpoint, &line.resource)?;
        let old = tx.account(line.endpoint, &line.resource);
        if old
            .on_hand
            .checked_sub(old.reserved)
            .ok_or(RejectReason::ArithmeticOverflow)?
            < line.amount
        {
            return Err(match line.endpoint {
                Endpoint::Slot(_) => RejectReason::InsufficientBalance,
                Endpoint::Inventory(_) => RejectReason::InsufficientItems,
            });
        }
        updates.insert(
            (line.endpoint, line.resource),
            Account {
                on_hand: old
                    .on_hand
                    .checked_sub(line.amount)
                    .ok_or(RejectReason::ArithmeticOverflow)?,
                ..old
            },
        );
    }
    for ((endpoint, resource), account) in updates {
        tx.stage_account(endpoint, resource, account);
    }
    Ok(())
}

// Used by later transaction callers; these remain internal to the simulation crate.
#[allow(dead_code)]
pub(crate) fn consume_reservation(
    tx: &mut Transaction<'_>,
    reservation: &Reservation,
) -> Result<(), RejectReason> {
    close_reservation(tx, reservation, true)
}
// Used by later transaction callers; these remain internal to the simulation crate.
#[allow(dead_code)]
pub(crate) fn release_reservation(
    tx: &mut Transaction<'_>,
    reservation: &Reservation,
) -> Result<(), RejectReason> {
    close_reservation(tx, reservation, false)
}
fn close_reservation(
    tx: &mut Transaction<'_>,
    reservation: &Reservation,
    consume: bool,
) -> Result<(), RejectReason> {
    let canonical = validate_lines(&reservation.lines)?;
    let claim = tx
        .writes
        .claims
        .get(&reservation.job_id)
        .or_else(|| {
            tx.world
                .get_resource::<EconomyLedger>()
                .and_then(|ledger| ledger.reservation_claims.get(&reservation.job_id))
        })
        .ok_or(RejectReason::UnknownJob)?;
    if claim
        != &ReservationClaim::Active(Reservation {
            job_id: reservation.job_id,
            lines: canonical.clone(),
        })
    {
        return Err(RejectReason::UnknownJob);
    }
    let mut updates = BTreeMap::new();
    for line in canonical {
        let old = tx.account(line.endpoint, &line.resource);
        let reserved = old
            .reserved
            .checked_sub(line.amount)
            .ok_or(RejectReason::ArithmeticOverflow)?;
        let on_hand = if consume {
            old.on_hand
                .checked_sub(line.amount)
                .ok_or(RejectReason::ArithmeticOverflow)?
        } else {
            old.on_hand
        };
        updates.insert(
            (line.endpoint, line.resource),
            Account { on_hand, reserved },
        );
    }
    for ((endpoint, resource), account) in updates {
        tx.stage_account(endpoint, resource, account);
    }
    tx.writes
        .claims
        .insert(reservation.job_id, ReservationClaim::Closed);
    Ok(())
}

// Used by later transaction callers; these remain internal to the simulation crate.
#[allow(dead_code)]
pub(crate) fn refund_charged(
    tx: &mut Transaction<'_>,
    lines: &[ReservationLine],
    numerator: u64,
    denominator: u64,
) -> Result<(), RejectReason> {
    if denominator == 0 || numerator > denominator {
        return Err(RejectReason::InvalidAmount);
    }
    let canonical = validate_lines(lines)?;
    let mut updates = BTreeMap::new();
    let mut inventory_credits = BTreeMap::<u32, u128>::new();
    for line in canonical {
        tx.validate_endpoint(line.endpoint, &line.resource)?;
        let refund = u64::try_from(
            u128::from(line.amount)
                .checked_mul(u128::from(numerator))
                .ok_or(RejectReason::ArithmeticOverflow)?
                / u128::from(denominator),
        )
        .map_err(|_| RejectReason::ArithmeticOverflow)?;
        let old = tx.account(line.endpoint, &line.resource);
        let on_hand = old
            .on_hand
            .checked_add(refund)
            .ok_or(RejectReason::RefundDestinationOverflow)?;
        if let Endpoint::Inventory(unit) = line.endpoint {
            let total = inventory_credits.entry(unit).or_default();
            *total = total
                .checked_add(u128::from(refund))
                .ok_or(RejectReason::ArithmeticOverflow)?;
        }
        updates.insert((line.endpoint, line.resource), Account { on_hand, ..old });
    }
    for (unit, credit) in inventory_credits {
        let used = tx.inventory_used(unit)?;
        if used
            .checked_add(credit)
            .ok_or(RejectReason::ArithmeticOverflow)?
            > u128::from(tx.inventory(unit)?.capacity)
        {
            return Err(RejectReason::RefundDestinationFull);
        }
    }
    for ((endpoint, resource), account) in updates {
        tx.stage_account(endpoint, resource, account);
    }
    Ok(())
}

fn validate_lines(lines: &[ReservationLine]) -> Result<Vec<ReservationLine>, RejectReason> {
    let mut sorted = lines.to_vec();
    if sorted.iter().any(|line| line.amount == 0) {
        return Err(RejectReason::InvalidAmount);
    }
    sorted.sort_by(|a, b| (a.endpoint, &a.resource).cmp(&(b.endpoint, &b.resource)));
    if sorted
        .windows(2)
        .any(|pair| pair[0].endpoint == pair[1].endpoint && pair[0].resource == pair[1].resource)
    {
        return Err(RejectReason::InvalidAmount);
    }
    Ok(sorted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::slots::Slots;

    pub(super) fn seeded_world() -> World {
        let mut world = World::new();
        world.insert_resource(Slots::new(1));
        world.insert_resource(crate::registry::UnitTypes::default());
        let mut ledger = EconomyLedger::default();
        ledger.balances.insert(
            (0, ContentId::parse("probe:ore").unwrap()),
            Account {
                on_hand: 10,
                reserved: 0,
            },
        );
        ledger.balances.insert(
            (0, ContentId::parse("probe:wood").unwrap()),
            Account {
                on_hand: 1,
                reserved: 0,
            },
        );
        world.insert_resource(ledger);
        world
    }

    pub(super) fn line(resource: &str, amount: u64) -> ReservationLine {
        ReservationLine {
            endpoint: Endpoint::Slot(0),
            resource: ContentId::parse(resource).unwrap(),
            amount,
        }
    }

    #[test]
    fn mixed_cost_reservation_rolls_back_all_lines() {
        let mut world = seeded_world();
        let before = world.resource::<EconomyLedger>().clone();
        let result = transact(&mut world, |tx| {
            reserve_costs(tx, JobId(1), &[line("probe:ore", 5), line("probe:wood", 2)])
        });
        assert_eq!(result, Err(RejectReason::InsufficientBalance));
        assert_eq!(*world.resource::<EconomyLedger>(), before);
    }

    #[test]
    fn mixed_resources_on_one_endpoint_are_valid() {
        let mut world = seeded_world();
        let reservation = transact(&mut world, |tx| {
            reserve_costs(tx, JobId(1), &[line("probe:wood", 1), line("probe:ore", 4)])
        })
        .unwrap();
        assert_eq!(reservation.lines.len(), 2);
        assert_eq!(
            world.resource::<EconomyLedger>().balances
                [&(0, ContentId::parse("probe:ore").unwrap())]
                .reserved,
            4
        );
        assert_eq!(
            world.resource::<EconomyLedger>().balances
                [&(0, ContentId::parse("probe:wood").unwrap())]
                .reserved,
            1
        );
    }

    #[test]
    fn reservation_contention_and_stale_replay_are_rejected() {
        let mut world = seeded_world();
        let first = transact(&mut world, |tx| {
            reserve_costs(tx, JobId(1), &[line("probe:ore", 8)])
        })
        .unwrap();
        let before_contender = world.resource::<EconomyLedger>().clone();
        assert_eq!(
            transact(&mut world, |tx| reserve_costs(
                tx,
                JobId(2),
                &[line("probe:ore", 8)]
            )),
            Err(RejectReason::InsufficientBalance)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before_contender);
        transact(&mut world, |tx| release_reservation(tx, &first)).unwrap();
        let before_replay = world.resource::<EconomyLedger>().clone();
        let cross_job = Reservation {
            job_id: JobId(2),
            lines: first.lines.clone(),
        };
        assert_eq!(
            transact(&mut world, |tx| consume_reservation(tx, &cross_job)),
            Err(RejectReason::UnknownJob)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before_replay);
        assert_eq!(
            transact(&mut world, |tx| release_reservation(tx, &first)),
            Err(RejectReason::UnknownJob)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before_replay);
        assert_eq!(
            transact(&mut world, |tx| reserve_costs(
                tx,
                JobId(1),
                &[line("probe:ore", 1)]
            )),
            Err(RejectReason::UnknownJob)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before_replay);
    }

    #[test]
    fn checked_max_math_and_duplicate_lines_never_commit() {
        let mut world = seeded_world();
        let before = world.resource::<EconomyLedger>().clone();
        assert_eq!(
            transact(&mut world, |tx| reserve_costs(
                tx,
                JobId(1),
                &[line("probe:ore", 1), line("probe:ore", 1)]
            )),
            Err(RejectReason::InvalidAmount)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before);
        assert_eq!(
            transact(&mut world, |tx| charge_costs(
                tx,
                &[line("probe:ore", u64::MAX)]
            )),
            Err(RejectReason::InsufficientBalance)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before);
    }

    #[test]
    fn charge_costs_preflights_all_resources_before_debiting() {
        let mut world = seeded_world();
        transact(&mut world, |tx| {
            charge_costs(tx, &[line("probe:ore", 3), line("probe:wood", 1)])
        })
        .unwrap();
        assert_eq!(
            world.resource::<EconomyLedger>().balances
                [&(0, ContentId::parse("probe:ore").unwrap())]
                .on_hand,
            7
        );
        assert_eq!(
            world.resource::<EconomyLedger>().balances
                [&(0, ContentId::parse("probe:wood").unwrap())]
                .on_hand,
            0
        );
    }

    #[test]
    fn reservation_consumption_is_atomic_and_closes_claim() {
        let mut world = seeded_world();
        let token = transact(&mut world, |tx| {
            reserve_costs(tx, JobId(44), &[line("probe:ore", 6)])
        })
        .unwrap();
        transact(&mut world, |tx| consume_reservation(tx, &token)).unwrap();
        let account = world.resource::<EconomyLedger>().balances
            [&(0, ContentId::parse("probe:ore").unwrap())];
        assert_eq!(
            account,
            Account {
                on_hand: 4,
                reserved: 0
            }
        );
        assert_eq!(
            world.resource::<EconomyLedger>().reservation_claims[&JobId(44)],
            ReservationClaim::Closed
        );
        assert_eq!(
            transact(&mut world, |tx| consume_reservation(tx, &token)),
            Err(RejectReason::UnknownJob)
        );
    }

    #[test]
    fn audit_distinguishes_reservation_claim_payloads() {
        let mut world = seeded_world();
        let token = transact(&mut world, |tx| {
            reserve_costs(tx, JobId(7), &[line("probe:ore", 3)])
        })
        .unwrap();
        let before = crate::economy::views::audit_state(&mut world);
        assert_eq!(
            before.ledger_claims,
            vec![crate::economy::views::ReservationClaimView::Active {
                map_job_id: 7,
                token_job_id: 7,
                lines: vec![crate::economy::views::ReservationLineView {
                    endpoint: crate::economy::views::ReservationEndpointView::Slot(0),
                    resource: ContentId::parse("probe:ore").unwrap(),
                    amount: 3,
                }],
            }],
        );
        let mut changed = token;
        changed.lines[0].amount = 2;
        world
            .resource_mut::<EconomyLedger>()
            .reservation_claims
            .insert(JobId(7), ReservationClaim::Active(changed));
        let after = crate::economy::views::audit_state(&mut world);
        assert_ne!(
            after, before,
            "audit must include authoritative reservation lines"
        );
    }

    #[test]
    fn failed_outer_transaction_preserves_complete_claim_audit() {
        let mut world = seeded_world();
        transact(&mut world, |tx| {
            reserve_costs(tx, JobId(7), &[line("probe:ore", 3)])
        })
        .unwrap();
        let before = crate::economy::views::audit_state(&mut world);
        let result: Result<(), RejectReason> = transact(&mut world, |tx| {
            reserve_costs(tx, JobId(8), &[line("probe:wood", 1)])?;
            Err(RejectReason::UnknownJob)
        });
        assert_eq!(result, Err(RejectReason::UnknownJob));
        assert_eq!(crate::economy::views::audit_state(&mut world), before);
    }

    #[test]
    fn refund_uses_checked_u128_and_leaves_overflow_unchanged() {
        let mut world = seeded_world();
        world.resource_mut::<EconomyLedger>().balances.insert(
            (0, ContentId::parse("probe:ore").unwrap()),
            Account {
                on_hand: u64::MAX,
                reserved: 0,
            },
        );
        let before = world.resource::<EconomyLedger>().clone();
        let result = transact(&mut world, |tx| {
            refund_charged(tx, &[line("probe:ore", u64::MAX)], 2, 1)
        });
        assert_eq!(result, Err(RejectReason::InvalidAmount));
        assert_eq!(*world.resource::<EconomyLedger>(), before);
        let result = transact(&mut world, |tx| {
            refund_charged(tx, &[line("probe:ore", u64::MAX)], 1, 1)
        });
        assert_eq!(result, Err(RejectReason::RefundDestinationOverflow));
        assert_eq!(*world.resource::<EconomyLedger>(), before);
    }
}

#[cfg(test)]
mod inventory_tests {
    use super::tests::{line, seeded_world};
    use super::*;
    use crate::{Owner, UnitId, economy::ledger::Inventory, slots::Slots};
    use std::collections::{BTreeMap, BTreeSet};

    pub(super) fn inventory_world() -> World {
        let timber = ContentId::parse("probe:timber").unwrap();
        let mut world = World::new();
        world.insert_resource(Slots::new(1));
        world.spawn((UnitId(1), Owner(SlotId(0))));
        world.spawn((UnitId(2), Owner(SlotId(0))));
        let mut ledger = EconomyLedger::default();
        let mut source = Inventory {
            capacity: 20,
            accepted_items: BTreeSet::from([timber.clone()]),
            items: BTreeMap::new(),
        };
        source.items.insert(
            timber.clone(),
            Account {
                on_hand: 10,
                reserved: 4,
            },
        );
        let destination = Inventory {
            capacity: 20,
            accepted_items: BTreeSet::from([timber.clone()]),
            items: BTreeMap::new(),
        };
        ledger.inventories.insert(1, source);
        ledger.inventories.insert(2, destination);
        world.insert_resource(ledger);
        world
    }

    #[test]
    fn transfer_cannot_spend_reserved_inventory_stock() {
        let mut world = inventory_world();
        let same_endpoint = inventory_transfer(
            &mut world,
            SlotId(0),
            1,
            1,
            ContentId::parse("probe:timber").unwrap(),
            1,
        );
        assert_eq!(same_endpoint, Err(RejectReason::InvalidAmount));
        let before = world.resource::<EconomyLedger>().clone();
        assert_eq!(
            inventory_transfer(
                &mut world,
                SlotId(0),
                1,
                2,
                ContentId::parse("probe:timber").unwrap(),
                7
            ),
            Err(RejectReason::InsufficientItems)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before);
        inventory_transfer(
            &mut world,
            SlotId(0),
            1,
            2,
            ContentId::parse("probe:timber").unwrap(),
            6,
        )
        .unwrap();
        assert_eq!(
            world.resource::<EconomyLedger>().inventories[&1].items
                [&ContentId::parse("probe:timber").unwrap()],
            Account {
                on_hand: 4,
                reserved: 4
            }
        );
    }

    #[test]
    fn destination_capacity_counts_all_resources_and_rejects_overflowing_u64_sum() {
        let mut world = inventory_world();
        let timber = ContentId::parse("probe:timber").unwrap();
        let wood = ContentId::parse("probe:wood").unwrap();
        {
            let mut ledger = world.resource_mut::<EconomyLedger>();
            let destination = ledger.inventories.get_mut(&2).unwrap();
            destination.capacity = u64::MAX;
            destination.accepted_items.insert(wood.clone());
            destination.items.insert(
                wood,
                Account {
                    on_hand: u64::MAX,
                    reserved: 0,
                },
            );
        }
        let before = world.resource::<EconomyLedger>().clone();
        assert_eq!(
            inventory_transfer(&mut world, SlotId(0), 1, 2, timber, 1),
            Err(RejectReason::RefundDestinationFull)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before);
    }

    #[test]
    fn refund_can_credit_u64_max_after_checked_u128_multiplication() {
        let mut world = seeded_world();
        world.resource_mut::<EconomyLedger>().balances.insert(
            (0, ContentId::parse("probe:ore").unwrap()),
            Account::default(),
        );
        transact(&mut world, |tx| {
            refund_charged(tx, &[line("probe:ore", u64::MAX)], u64::MAX, u64::MAX)
        })
        .unwrap();
        assert_eq!(
            world.resource::<EconomyLedger>().balances
                [&(0, ContentId::parse("probe:ore").unwrap())]
                .on_hand,
            u64::MAX
        );
    }
}

#[cfg(test)]
mod refund_tests {
    use super::inventory_tests::inventory_world;
    use super::*;

    #[test]
    fn refund_preflights_combined_inventory_credit_across_resources() {
        let mut world = inventory_world();
        let timber = ContentId::parse("probe:timber").unwrap();
        let wood = ContentId::parse("probe:wood").unwrap();
        {
            let mut ledger = world.resource_mut::<EconomyLedger>();
            let destination = ledger.inventories.get_mut(&2).unwrap();
            destination.capacity = 10;
            destination.accepted_items.insert(wood.clone());
            destination.items.insert(
                wood.clone(),
                Account {
                    on_hand: 4,
                    reserved: 0,
                },
            );
        }
        let before = world.resource::<EconomyLedger>().clone();
        let lines = [
            ReservationLine {
                endpoint: Endpoint::Inventory(2),
                resource: timber,
                amount: 4,
            },
            ReservationLine {
                endpoint: Endpoint::Inventory(2),
                resource: wood,
                amount: 4,
            },
        ];
        assert_eq!(
            transact(&mut world, |tx| refund_charged(tx, &lines, 1, 1)),
            Err(RejectReason::RefundDestinationFull)
        );
        assert_eq!(*world.resource::<EconomyLedger>(), before);
    }
}

#[cfg(test)]
mod staged_refund_tests {
    use super::inventory_tests::inventory_world;
    use super::*;

    fn prepare_empty_tight_inventory(world: &mut World) {
        let wood = ContentId::parse("probe:wood").unwrap();
        let mut ledger = world.resource_mut::<EconomyLedger>();
        let destination = ledger.inventories.get_mut(&2).unwrap();
        destination.capacity = 5;
        destination.accepted_items.insert(wood);
    }

    #[test]
    fn second_refund_to_new_same_resource_rejects_and_outer_transaction_rolls_back() {
        let mut world = inventory_world();
        prepare_empty_tight_inventory(&mut world);
        let before = world.resource::<EconomyLedger>().clone();
        let line = ReservationLine {
            endpoint: Endpoint::Inventory(2),
            resource: ContentId::parse("probe:timber").unwrap(),
            amount: 4,
        };
        let result = transact(&mut world, |tx| {
            refund_charged(tx, std::slice::from_ref(&line), 1, 1)?;
            refund_charged(tx, std::slice::from_ref(&line), 1, 1)?;
            Ok(())
        });
        assert_eq!(result, Err(RejectReason::RefundDestinationFull));
        assert_eq!(*world.resource::<EconomyLedger>(), before);
    }

    #[test]
    fn second_refund_to_different_new_resource_rejects_but_first_can_commit() {
        let mut world = inventory_world();
        prepare_empty_tight_inventory(&mut world);
        let timber = ReservationLine {
            endpoint: Endpoint::Inventory(2),
            resource: ContentId::parse("probe:timber").unwrap(),
            amount: 4,
        };
        let wood = ReservationLine {
            endpoint: Endpoint::Inventory(2),
            resource: ContentId::parse("probe:wood").unwrap(),
            amount: 4,
        };
        transact(&mut world, |tx| {
            refund_charged(tx, std::slice::from_ref(&timber), 1, 1)?;
            assert_eq!(
                refund_charged(tx, std::slice::from_ref(&wood), 1, 1),
                Err(RejectReason::RefundDestinationFull)
            );
            Ok(())
        })
        .unwrap();
        let inventory = &world.resource::<EconomyLedger>().inventories[&2];
        assert_eq!(inventory.items[&timber.resource].on_hand, 4);
        assert!(!inventory.items.contains_key(&wood.resource));
    }
}
