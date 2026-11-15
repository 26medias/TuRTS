//! Periodic balance income. Clocks start at admitted activation, not at observation.

use std::collections::BTreeMap;

use bevy_ecs::{prelude::Resource, world::World};
use turts_mod_api::{ContentId, IncomeDef};

use super::{
    combat::Healths,
    construction::ConstructionState,
    events::EconomyEventKind,
    ledger::{Account, Endpoint},
    transaction,
};
use crate::{Owner, Position, RejectReason, unit_index::unit_entity, world::Tick};

type IncomeAccount = (u8, ContentId);

/// Prepared before the creation transaction. Applying it cannot fail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IncomeActivation {
    owner_slot: u8,
    resource: ContentId,
    amount: u64,
    period_ticks: u64,
    activated_at: u32,
    next_due: u64,
}

#[derive(Resource, Debug, Default, PartialEq, Eq)]
struct IncomeState {
    clocks: BTreeMap<u32, IncomeActivation>,
    held: BTreeMap<IncomeAccount, RejectReason>,
}

/// Owner-only presentation can consume these transitions without invented command ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IncomeTransition {
    Held {
        owner_slot: u8,
        resource: ContentId,
        reason: RejectReason,
    },
    Restored {
        owner_slot: u8,
        resource: ContentId,
    },
}

#[derive(Resource, Debug, Default, PartialEq, Eq)]
pub(crate) struct IncomeTransitions(pub(crate) Vec<IncomeTransition>);

/// Package replacement discards all clocks and owner-display transitions.
pub(crate) fn clear(world: &mut World) {
    world.remove_resource::<IncomeState>();
    world.remove_resource::<IncomeTransitions>();
}

/// A copy of authoritative clock state for audits. It grants no mutation access.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IncomeClockAudit {
    pub(crate) unit: u32,
    pub(crate) owner_slot: u8,
    pub(crate) resource: ContentId,
    pub(crate) amount: u64,
    pub(crate) period_ticks: u64,
    pub(crate) activation_tick: u32,
    pub(crate) next_due: u64,
    pub(crate) held: Option<RejectReason>,
}

/// Unit-id order is only an audit serialization order. Payments remain atomic by account.
pub(crate) fn clock_audit(world: &World) -> Vec<IncomeClockAudit> {
    let Some(state) = world.get_resource::<IncomeState>() else {
        return Vec::new();
    };
    state
        .clocks
        .iter()
        .map(|(&unit, clock)| IncomeClockAudit {
            unit,
            owner_slot: clock.owner_slot,
            resource: clock.resource.clone(),
            amount: clock.amount,
            period_ticks: clock.period_ticks,
            activation_tick: clock.activated_at,
            next_due: clock.next_due,
            held: state
                .held
                .get(&(clock.owner_slot, clock.resource.clone()))
                .copied(),
        })
        .collect()
}

/// The caller checks resource kind, period conversion, and shared admission before commit.
pub(crate) fn prepare_activation(
    owner_slot: u8,
    definition: &IncomeDef,
    period_ticks: u64,
    activation_tick: u32,
) -> Result<IncomeActivation, RejectReason> {
    if definition.amount == 0 || period_ticks == 0 {
        return Err(RejectReason::InvalidAmount);
    }
    let resource =
        ContentId::parse(&definition.resource).map_err(|_| RejectReason::UnknownBalance)?;
    let next_due = u64::from(activation_tick)
        .checked_add(period_ticks)
        .ok_or(RejectReason::ArithmeticOverflow)?;
    Ok(IncomeActivation {
        owner_slot,
        resource,
        amount: definition.amount,
        period_ticks,
        activated_at: activation_tick,
        next_due,
    })
}

/// Commit hook for initial placement, normal spawn, site completion, and admitted revival.
pub(crate) fn on_activate(world: &mut World, unit: u32, activation: Option<IncomeActivation>) {
    on_remove(world, unit);
    let Some(activation) = activation else {
        return;
    };
    world.init_resource::<IncomeState>();
    world.init_resource::<IncomeTransitions>();
    world
        .resource_mut::<IncomeState>()
        .clocks
        .insert(unit, activation);
}

/// Commit hook for removal or loss of positive health. A later activation starts afresh.
pub(crate) fn on_remove(world: &mut World, unit: u32) {
    let Some(mut state) = world.get_resource_mut::<IncomeState>() else {
        return;
    };
    let Some(old) = state.clocks.remove(&unit) else {
        return;
    };
    let account = (old.owner_slot, old.resource);
    if !state.held.contains_key(&account) {
        return;
    }
    let still_active = state
        .clocks
        .values()
        .any(|clock| clock.owner_slot == account.0 && clock.resource == account.1);
    if !still_active && state.held.remove(&account).is_some() {
        world
            .resource_mut::<IncomeTransitions>()
            .0
            .push(IncomeTransition::Restored {
                owner_slot: account.0,
                resource: account.1,
            });
    }
}

fn is_active(world: &World, unit: u32, clock: &IncomeActivation) -> bool {
    let Some(entity) = unit_entity(world, unit) else {
        return false;
    };
    world
        .get::<Owner>(entity)
        .is_some_and(|owner| owner.0.0 == clock.owner_slot)
        && !world
            .get_resource::<ConstructionState>()
            .is_some_and(|state| state.sites.contains_key(&unit))
        && !world
            .get_resource::<Healths>()
            .is_some_and(|healths| healths.0.get(&unit) == Some(&0))
}

#[derive(Debug)]
struct DueIncome {
    unit: u32,
    position: bevy_math::Vec2,
    amount: u64,
    next_due: Option<u64>,
}

/// After commands, before per-tick payment. One account receives its whole batch or nothing.
pub(crate) fn run_income(world: &mut World) {
    let Some(state) = world.get_resource::<IncomeState>() else {
        return;
    };
    if state.clocks.is_empty() {
        return;
    }
    let inactive = state
        .clocks
        .iter()
        .filter_map(|(&unit, clock)| (!is_active(world, unit, clock)).then_some(unit))
        .collect::<Vec<_>>();
    for unit in inactive {
        transaction::capture_operation(world, |world| on_remove(world, unit));
    }

    let now = world.resource::<Tick>().0;
    let mut due = BTreeMap::<IncomeAccount, Vec<DueIncome>>::new();
    for (&unit, clock) in &world.resource::<IncomeState>().clocks {
        if u64::from(now) < clock.next_due {
            continue;
        }
        let position = unit_entity(world, unit)
            .and_then(|entity| world.get::<Position>(entity))
            .map_or(bevy_math::Vec2::ZERO, |position| position.0);
        due.entry((clock.owner_slot, clock.resource.clone()))
            .or_default()
            .push(DueIncome {
                unit,
                position,
                amount: clock.amount,
                next_due: u64::from(now).checked_add(clock.period_ticks),
            });
    }
    for ((owner_slot, resource), mut sources) in due {
        // All sources are paid together. Coordinates only order their presentation events.
        sources.sort_by(|left, right| {
            left.position
                .x
                .total_cmp(&right.position.x)
                .then_with(|| left.position.y.total_cmp(&right.position.y))
        });
        transaction::capture_operation(world, |world| {
            let result = credit_account(world, owner_slot, &resource, &sources);
            let key = (owner_slot, resource.clone());
            match result {
                Ok(()) => {
                    let mut state = world.resource_mut::<IncomeState>();
                    for source in sources {
                        if let Some(clock) = state.clocks.get_mut(&source.unit)
                            && let Some(next_due) = source.next_due
                        {
                            clock.next_due = next_due;
                        }
                    }
                    let restored = state.held.remove(&key).is_some();
                    if restored {
                        world.resource_mut::<IncomeTransitions>().0.push(
                            IncomeTransition::Restored {
                                owner_slot,
                                resource,
                            },
                        );
                    }
                }
                Err(reason) => {
                    let old = world.resource_mut::<IncomeState>().held.insert(key, reason);
                    if old != Some(reason) {
                        world
                            .resource_mut::<IncomeTransitions>()
                            .0
                            .push(IncomeTransition::Held {
                                owner_slot,
                                resource,
                                reason,
                            });
                    }
                }
            }
        });
    }
}

fn credit_account(
    world: &mut World,
    owner_slot: u8,
    resource: &ContentId,
    sources: &[DueIncome],
) -> Result<(), RejectReason> {
    let total = sources.iter().try_fold(0_u64, |total, source| {
        if source.next_due.is_none() {
            return Err(RejectReason::ArithmeticOverflow);
        }
        total
            .checked_add(source.amount)
            .ok_or(RejectReason::ArithmeticOverflow)
    })?;
    transaction::transact(world, |tx| {
        let endpoint = Endpoint::Slot(owner_slot);
        tx.validate_endpoint(endpoint, resource)?;
        let account = tx.account(endpoint, resource);
        let on_hand = account
            .on_hand
            .checked_add(total)
            .ok_or(RejectReason::ArithmeticOverflow)?;
        tx.stage_account(endpoint, resource.clone(), Account { on_hand, ..account });
        for source in sources {
            tx.stage_event(
                owner_slot,
                EconomyEventKind::IncomeCredited {
                    unit: source.unit,
                    balance: resource.clone(),
                    credited: source.amount,
                },
            )?;
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        SlotId, UnitId,
        economy::{
            events::{NextEventOrdinal, PendingEconomyEvents},
            ledger::EconomyLedger,
            research::Disabled,
        },
    };

    fn balance() -> ContentId {
        ContentId::parse("fixture:credits").unwrap()
    }

    fn fixture(hz: u16) -> bevy_app::App {
        let mut app = crate::headless_app(hz);
        for slot in 0..2 {
            app.world_mut()
                .resource_mut::<EconomyLedger>()
                .balances
                .insert((slot, balance()), Account::default());
        }
        app
    }

    fn actor(world: &mut World, id: u32, slot: u8, x: f32) {
        world.spawn((
            UnitId(id),
            Owner(SlotId(slot)),
            Position(bevy_math::Vec2::new(x, 4.0)),
        ));
        world.resource_mut::<Healths>().0.insert(id, 100);
    }

    fn activate(world: &mut World, unit: u32, slot: u8, amount: u64, period: u64, tick: u32) {
        let definition = IncomeDef {
            resource: balance().to_string(),
            amount,
            period_ms: 1_050,
        };
        let prepared = prepare_activation(slot, &definition, period, tick).unwrap();
        on_activate(world, unit, Some(prepared));
    }

    fn run_at(world: &mut World, tick: u32) {
        world.resource_mut::<Tick>().0 = tick;
        run_income(world);
    }

    fn cash(world: &World, slot: u8) -> u64 {
        world.resource::<EconomyLedger>().balances[&(slot, balance())].on_hand
    }

    fn events(world: &World) -> &[super::super::events::EconomyEvent] {
        &world.resource::<PendingEconomyEvents>().0
    }

    #[test]
    fn full_periods_at_two_tick_rates_start_from_explicit_activation() {
        for (hz, expected_ticks) in [(20, 21), (30, 32)] {
            let mut app = fixture(hz);
            let world = app.world_mut();
            actor(world, 1, 0, 2.0);
            let period = crate::boot_m3::try_inserter_duration_ticks(1_050, u32::from(hz)).unwrap();
            assert_eq!(period, expected_ticks);
            activate(world, 1, 0, 7, u64::from(period), 0);
            run_at(world, 0);
            assert_eq!(cash(world, 0), 0, "no activation payment");
            run_at(world, period - 1);
            assert_eq!(cash(world, 0), 0, "no partial-period payment at {hz} Hz");
            run_at(world, period);
            assert_eq!(cash(world, 0), 7);
            run_at(world, period * 2 - 1);
            assert_eq!(cash(world, 0), 7);
            run_at(world, period * 2);
            assert_eq!(cash(world, 0), 14);
            assert_eq!(world.resource::<IncomeState>().clocks[&1].activated_at, 0);
            assert_eq!(
                clock_audit(world),
                [IncomeClockAudit {
                    unit: 1,
                    owner_slot: 0,
                    resource: balance(),
                    amount: 7,
                    period_ticks: u64::from(period),
                    activation_tick: 0,
                    next_due: u64::from(period * 3),
                    held: None,
                }]
            );

            actor(world, 2, 1, 5.0);
            let birth = period * 2 + 7;
            activate(world, 2, 1, 9, u64::from(period), birth);
            // First observation occurs late. Its deadline still derives from activation.
            run_at(world, birth + period - 1);
            assert_eq!(cash(world, 1), 0);
            run_at(world, birth + period);
            assert_eq!(cash(world, 1), 9);
            assert_eq!(
                world.resource::<IncomeState>().clocks[&2].activated_at,
                birth
            );
        }
    }

    #[test]
    fn a_delayed_system_visit_never_backfills_missed_periods() {
        let mut app = fixture(20);
        let world = app.world_mut();
        actor(world, 1, 0, 2.0);
        activate(world, 1, 0, 7, 10, 0);
        run_at(world, 100);
        assert_eq!(cash(world, 0), 7);
        assert_eq!(world.resource::<IncomeState>().clocks[&1].next_due, 110);
        run_at(world, 109);
        assert_eq!(cash(world, 0), 7);
        run_at(world, 110);
        assert_eq!(cash(world, 0), 14);
    }

    #[test]
    fn a_full_account_holds_all_due_sources_and_other_accounts_continue() {
        for ids in [[1, 2], [2, 1]] {
            let mut app = fixture(20);
            let world = app.world_mut();
            actor(world, ids[0], 0, 2.0);
            actor(world, ids[1], 0, 5.0);
            actor(world, 3, 1, 9.0);
            actor(world, 4, 0, 12.0);
            activate(world, ids[0], 0, 4, 10, 0);
            activate(world, ids[1], 0, 6, 10, 0);
            activate(world, 3, 1, 9, 10, 0);
            let other_balance = ContentId::parse("fixture:other").unwrap();
            world
                .resource_mut::<EconomyLedger>()
                .balances
                .insert((0, other_balance.clone()), Account::default());
            let other_income = IncomeDef {
                resource: other_balance.to_string(),
                amount: 3,
                period_ms: 500,
            };
            on_activate(
                world,
                4,
                Some(prepare_activation(0, &other_income, 10, 0).unwrap()),
            );
            world
                .resource_mut::<EconomyLedger>()
                .balances
                .get_mut(&(0, balance()))
                .unwrap()
                .on_hand = u64::MAX - 7;
            run_at(world, 10);
            assert_eq!(
                cash(world, 0),
                u64::MAX - 7,
                "neither source receives a partial payment"
            );
            assert_eq!(cash(world, 1), 9);
            assert_eq!(
                world.resource::<EconomyLedger>().balances[&(0, other_balance)].on_hand,
                3
            );
            assert_eq!(events(world).len(), 2);
            assert_eq!(world.resource::<IncomeTransitions>().0.len(), 1);
            assert_eq!(world.resource::<IncomeState>().clocks[&ids[0]].next_due, 10);
            assert_eq!(world.resource::<IncomeState>().clocks[&ids[1]].next_due, 10);
            for tick in 11..=35 {
                run_at(world, tick);
            }
            assert_eq!(
                world.resource::<IncomeTransitions>().0.len(),
                1,
                "held state must not spam"
            );
            world
                .resource_mut::<EconomyLedger>()
                .balances
                .get_mut(&(0, balance()))
                .unwrap()
                .on_hand = 0;
            run_at(world, 36);
            assert_eq!(
                cash(world, 0),
                10,
                "one pending payment per source, no missed-period accrual"
            );
            assert_eq!(world.resource::<IncomeState>().clocks[&ids[0]].next_due, 46);
            assert_eq!(world.resource::<IncomeState>().clocks[&ids[1]].next_due, 46);
            assert!(matches!(
                world.resource::<IncomeTransitions>().0.last(),
                Some(IncomeTransition::Restored { owner_slot: 0, .. })
            ));
            run_at(world, 45);
            assert_eq!(cash(world, 0), 10);
            run_at(world, 46);
            assert_eq!(cash(world, 0), 20);
            let paid = events(world)
                .iter()
                .filter_map(|event| match event.kind {
                    EconomyEventKind::IncomeCredited { unit, credited, .. }
                        if event.owner_slot == 0 && event.tick == 36 =>
                    {
                        Some((unit, credited))
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(
                paid,
                [(ids[0], 4), (ids[1], 6)],
                "physical event order survives an identity swap"
            );
        }
    }

    #[test]
    fn credit_preserves_reserved_funds_and_uses_a_distinct_income_event() {
        let mut app = fixture(20);
        let world = app.world_mut();
        actor(world, 1, 0, 2.0);
        activate(world, 1, 0, 7, 2, 0);
        world.resource_mut::<EconomyLedger>().balances.insert(
            (0, balance()),
            Account {
                on_hand: 20,
                reserved: 12,
            },
        );
        world.resource_mut::<Disabled>().0.insert(1);
        run_at(world, 2);
        assert_eq!(
            world.resource::<EconomyLedger>().balances[&(0, balance())],
            Account {
                on_hand: 27,
                reserved: 12
            }
        );
        assert_eq!(events(world).len(), 1);
        assert!(
            matches!(&events(world)[0].kind, EconomyEventKind::IncomeCredited {unit:1, balance:resource, credited:7} if *resource == balance())
        );
        assert!(
            !events(world)
                .iter()
                .any(|event| matches!(event.kind, EconomyEventKind::Converted { .. })),
            "income is not a Gathered delivery"
        );
    }

    #[test]
    fn a_site_a_dead_unit_and_a_missing_unit_discard_their_clocks() {
        let mut app = fixture(20);
        let world = app.world_mut();
        for unit in 1..=3 {
            actor(world, unit, 0, unit as f32 * 3.0);
            activate(world, unit, 0, 7, 10, 0);
        }
        world.resource_mut::<ConstructionState>().sites.insert(
            1,
            super::super::construction::Site {
                constructor: 99,
                owner_slot: 0,
                recipe: balance(),
                job: crate::JobId(1),
                done: 0,
                total: 10,
                charged: Vec::new(),
            },
        );
        world.resource_mut::<Healths>().0.insert(2, 0);
        let gone = unit_entity(world, 3).unwrap();
        world.despawn(gone);
        run_at(world, 10);
        assert_eq!(cash(world, 0), 0);
        assert!(world.resource::<IncomeState>().clocks.is_empty());
        assert!(events(world).is_empty());

        world.resource_mut::<ConstructionState>().sites.remove(&1);
        activate(world, 1, 0, 7, 10, 10);
        world.resource_mut::<Healths>().0.insert(2, 100);
        // Health alone does not silently admit a revival or restart it at observation time.
        run_at(world, 19);
        assert_eq!(cash(world, 0), 0);
        assert!(!world.resource::<IncomeState>().clocks.contains_key(&2));
        activate(world, 2, 0, 7, 10, 19);
        run_at(world, 20);
        assert_eq!(cash(world, 0), 7);
        run_at(world, 28);
        assert_eq!(cash(world, 0), 7);
        run_at(world, 29);
        assert_eq!(cash(world, 0), 14);
    }

    #[test]
    fn removal_discards_a_partial_period_and_reuse_starts_a_new_one() {
        let mut app = fixture(20);
        let world = app.world_mut();
        actor(world, 1, 0, 2.0);
        activate(world, 1, 0, 7, 10, 0);
        run_at(world, 6);
        on_remove(world, 1);
        activate(world, 1, 0, 7, 10, 6);
        run_at(world, 10);
        assert_eq!(cash(world, 0), 0);
        run_at(world, 15);
        assert_eq!(cash(world, 0), 0);
        run_at(world, 16);
        assert_eq!(cash(world, 0), 7);
    }

    #[test]
    fn event_ordinal_overflow_rolls_back_the_whole_account_batch() {
        let mut app = fixture(20);
        let world = app.world_mut();
        for unit in 1..=2 {
            actor(world, unit, 0, unit as f32 * 3.0);
            activate(world, unit, 0, 4, 10, 0);
        }
        world.resource_mut::<NextEventOrdinal>().0 = Some(u64::MAX - 1);
        run_at(world, 10);
        assert_eq!(cash(world, 0), 0);
        assert!(events(world).is_empty());
        assert_eq!(world.resource::<NextEventOrdinal>().0, Some(u64::MAX - 1));
        assert_eq!(world.resource::<IncomeState>().clocks[&1].next_due, 10);
        world.resource_mut::<NextEventOrdinal>().0 = Some(0);
        run_at(world, 11);
        assert_eq!(cash(world, 0), 8);
        assert_eq!(events(world).len(), 2);
        assert_eq!(world.resource::<IncomeState>().clocks[&1].next_due, 21);
    }

    #[test]
    fn invalid_activation_payloads_fail_before_any_world_mutation() {
        let definition = IncomeDef {
            resource: balance().to_string(),
            amount: 7,
            period_ms: 1,
        };
        assert_eq!(
            prepare_activation(0, &definition, 0, 0),
            Err(RejectReason::InvalidAmount)
        );
        assert_eq!(
            prepare_activation(0, &definition, u64::MAX, 1),
            Err(RejectReason::ArithmeticOverflow)
        );
        let zero = IncomeDef {
            amount: 0,
            ..definition.clone()
        };
        assert_eq!(
            prepare_activation(0, &zero, 1, 0),
            Err(RejectReason::InvalidAmount)
        );
        let unknown = IncomeDef {
            resource: "not qualified".into(),
            ..definition
        };
        assert_eq!(
            prepare_activation(0, &unknown, 1, 0),
            Err(RejectReason::UnknownBalance)
        );
    }

    #[test]
    fn no_income_keeps_the_existing_world_and_audit_shape() {
        let mut app = fixture(20);
        let world = app.world_mut();
        let before = crate::audit_state(world);
        run_income(world);
        on_remove(world, 1);
        on_activate(world, 1, None);
        assert_eq!(crate::audit_state(world), before);
        assert!(!world.contains_resource::<IncomeState>());
        assert!(!world.contains_resource::<IncomeTransitions>());
    }

    #[test]
    fn repeated_income_runs_have_identical_clocks_accounts_and_events_at_every_tick() {
        let mut left = fixture(20);
        let mut right = fixture(20);
        for app in [&mut left, &mut right] {
            for unit in 1..=3 {
                actor(app.world_mut(), unit, (unit % 2) as u8, unit as f32 * 3.0);
                activate(
                    app.world_mut(),
                    unit,
                    (unit % 2) as u8,
                    u64::from(unit),
                    7,
                    0,
                );
            }
        }
        for tick in 0..=100 {
            for app in [&mut left, &mut right] {
                let world = app.world_mut();
                if tick == 20 {
                    world
                        .resource_mut::<EconomyLedger>()
                        .balances
                        .get_mut(&(0, balance()))
                        .unwrap()
                        .on_hand = u64::MAX;
                }
                if tick == 40 {
                    world
                        .resource_mut::<EconomyLedger>()
                        .balances
                        .get_mut(&(0, balance()))
                        .unwrap()
                        .on_hand = 0;
                }
                if tick == 60 {
                    on_remove(world, 1);
                }
                run_at(world, tick);
            }
            assert_eq!(
                left.world().resource::<IncomeState>(),
                right.world().resource::<IncomeState>(),
                "clocks at tick {tick}"
            );
            assert_eq!(
                left.world().resource::<EconomyLedger>(),
                right.world().resource::<EconomyLedger>(),
                "ledger at tick {tick}"
            );
            assert_eq!(
                events(left.world()),
                events(right.world()),
                "events at tick {tick}"
            );
            assert_eq!(
                left.world().resource::<IncomeTransitions>(),
                right.world().resource::<IncomeTransitions>(),
                "transitions at tick {tick}"
            );
            assert_eq!(
                clock_audit(left.world()),
                clock_audit(right.world()),
                "clock audit at tick {tick}"
            );
        }
    }
}
