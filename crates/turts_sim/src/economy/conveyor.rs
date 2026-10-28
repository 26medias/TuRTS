//! Compact conveyor state and fixed-tick transport behavior.

use std::collections::BTreeMap;

use bevy_app::{App, FixedUpdate};
use bevy_ecs::{prelude::Resource, schedule::IntoScheduleConfigs};
use turts_mod_api::ContentId;

use super::gather::RateCarry;

/// Dense item index. Zero marks an empty belt slot.
pub(crate) type ItemIx = u16;

/// One ordered conveyor line and its compact item slots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BeltLine {
    pub(crate) def: ContentId,
    pub(crate) owner: u8,
    pub(crate) pieces: Vec<u32>,
    pub(crate) slots: Vec<ItemIx>,
    pub(crate) slots_per_piece: u8,
    pub(crate) slots_per_second: u64,
    pub(crate) carry: RateCarry,
}

/// Conveyor lines, indexed by their first piece and by each contained piece.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Conveyors {
    pub(crate) lines: BTreeMap<u32, BeltLine>,
    pub(crate) piece_line: BTreeMap<u32, (u32, u16)>,
}

#[cfg(test)]
mod item_table_tests {
    use super::{ItemIx, ItemTable};
    use turts_mod_api::ContentId;

    #[test]
    fn item_table_reserves_zero_and_maps_sorted_content_ids() {
        let alpha = ContentId::parse("probe:alpha").unwrap();
        let zeta = ContentId::parse("probe:zeta").unwrap();
        let table = ItemTable(vec![alpha.clone(), zeta.clone()]);

        assert_eq!(table.content(0), None);
        assert_eq!(table.content(1), Some(&alpha));
        assert_eq!(table.content(2), Some(&zeta));
        assert_eq!(table.content(3), None);
        assert_eq!(table.item_ix(&alpha), Some(1));
        assert_eq!(table.item_ix(&zeta), Some(2));
    }

    #[test]
    fn item_table_reverse_mapping_checks_u16_boundary() {
        assert_eq!(
            ItemTable::ix_for_position(usize::from(ItemIx::MAX) - 1),
            Some(ItemIx::MAX)
        );
        assert_eq!(ItemTable::ix_for_position(usize::from(ItemIx::MAX)), None);
    }
}

/// Sorted item ids used to translate compact belt slots to content ids.
/// Item index zero is reserved for an empty slot, so the first item is at vector index zero.
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ItemTable(pub(crate) Vec<ContentId>);

impl ItemTable {
    #[allow(dead_code)] // used from Task 3
    pub(crate) fn content(&self, item: ItemIx) -> Option<&ContentId> {
        (item != 0)
            .then(|| self.0.get(usize::from(item) - 1))
            .flatten()
    }

    #[allow(dead_code)] // used from Task 4
    pub(crate) fn item_ix(&self, item: &ContentId) -> Option<ItemIx> {
        let position = self.0.binary_search(item).ok()?;
        Self::ix_for_position(position)
    }

    fn ix_for_position(position: usize) -> Option<ItemIx> {
        ItemIx::try_from(position.checked_add(1)?).ok()
    }
}

/// Install the Transport phase system.
pub(crate) fn install(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        run_transport
            .in_set(crate::plugin::SimSet::Transport)
            .run_if(not_stopped),
    );
}

fn not_stopped(
    stopped: Option<bevy_ecs::prelude::Res<crate::economy::events::MatchStopped>>,
) -> bool {
    stopped.is_none()
}

/// Step every line in ascending key order. Each line owns one captured transaction per tick.
fn run_transport(world: &mut bevy_ecs::world::World) {
    if !world.contains_resource::<Conveyors>() {
        return;
    }
    let line_ids = world
        .resource::<Conveyors>()
        .lines
        .keys()
        .copied()
        .collect::<Vec<_>>();
    if line_ids.is_empty() {
        return;
    }
    let tick_hz = world.resource::<crate::SimClock>().tick_hz;
    for line_id in line_ids {
        let _ = super::transaction::transact(world, |tx| {
            tx.mark_m3();
            let mut line = tx.conveyor_line(line_id)?;
            let steps = line.carry.take_budget(line.slots_per_second, tick_hz)?;
            step_line(&mut line, steps);
            tx.stage_conveyor_line(line_id, line);
            Ok(())
        });
    }
}

/// Advance one line by the given number of literal downstream-first scans.
///
/// An item only moves to the right and can cross at most `slots.len() - 1` edges. After that
/// many scans, every item is either at the end or behind a fully occupied suffix, so every
/// later literal scan is a no-op. Bounding the loop at that point preserves the repeated-scan
/// result while allowing any `u64` budget to finish in time bounded by line length.
pub(crate) fn step_line(line: &mut BeltLine, steps: u64) {
    let useful_steps = steps.min(line.slots.len().saturating_sub(1) as u64);
    for _ in 0..useful_steps {
        for index in (0..line.slots.len().saturating_sub(1)).rev() {
            if line.slots[index] != 0 && line.slots[index + 1] == 0 {
                line.slots.swap(index, index + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(slots: Vec<ItemIx>, rate: u64) -> BeltLine {
        BeltLine {
            def: ContentId::parse("probe:belt").unwrap(),
            owner: 0,
            pieces: (0..slots.len().div_ceil(4)).map(|n| n as u32 + 1).collect(),
            slots,
            slots_per_piece: 4,
            slots_per_second: rate,
            carry: RateCarry::default(),
        }
    }

    fn literal(line: &mut BeltLine, steps: u64) {
        for _ in 0..steps {
            for index in (0..line.slots.len().saturating_sub(1)).rev() {
                if line.slots[index] != 0 && line.slots[index + 1] == 0 {
                    line.slots.swap(index, index + 1);
                }
            }
        }
    }

    #[test]
    fn step_line_moves_run_as_block() {
        let mut belt = line(vec![1, 2, 3, 0], 1);
        step_line(&mut belt, 1);
        assert_eq!(belt.slots, vec![0, 1, 2, 3]);
    }

    #[test]
    fn step_line_blocked_head_holds_run() {
        let mut belt = line(vec![1, 2, 3, 4], 1);
        step_line(&mut belt, 1);
        assert_eq!(belt.slots, vec![1, 2, 3, 4]);
    }

    #[test]
    fn last_slot_never_moves() {
        let mut belt = line(vec![0, 0, 0, 9], 1);
        step_line(&mut belt, 20);
        assert_eq!(belt.slots, vec![0, 0, 0, 9]);
    }

    #[test]
    fn transit_time_exact() {
        let mut belt = line(vec![1; 24], 4);
        belt.slots[1..].fill(0);
        for _ in 0..22 {
            step_line(&mut belt, 1);
        }
        assert_eq!(belt.slots[22], 1);
        assert_eq!(belt.slots[23], 0);
        step_line(&mut belt, 1);
        assert_eq!(belt.slots[22], 0);
        assert_eq!(belt.slots[23], 1);
        assert_eq!(belt.slots.iter().filter(|&&item| item != 0).count(), 1);
    }

    #[test]
    fn steps_per_tick_above_one() {
        let mut app = crate::headless_app(20);
        app.world_mut()
            .resource_mut::<Conveyors>()
            .lines
            .insert(1, line(vec![1, 0, 0, 0], 40));
        app.update();
        assert_eq!(
            app.world().resource::<Conveyors>().lines[&1].slots,
            vec![0, 0, 1, 0]
        );
    }

    #[test]
    fn rate_at_10_and_30_hz() {
        for (hz, ticks_to_move) in [(10, 3), (30, 8)] {
            let mut app = crate::headless_app(hz);
            app.world_mut()
                .resource_mut::<Conveyors>()
                .lines
                .insert(1, line(vec![1, 0], 4));
            for _ in 0..ticks_to_move - 1 {
                app.update();
                assert_eq!(
                    app.world().resource::<Conveyors>().lines[&1].slots,
                    vec![1, 0]
                );
            }
            app.update();
            let stepped = app.world().resource::<Conveyors>().lines.get(&1).unwrap();
            assert_eq!(stepped.slots, vec![0, 1]);
            assert_eq!(stepped.carry.remainder(), 2);
        }
    }

    #[test]
    fn no_loss_over_1000_ticks() {
        let mut belt = line(vec![1, 0, 2, 0, 3, 0, 4, 0], 40);
        let initial = belt.slots.iter().filter(|&&item| item != 0).count();
        for _ in 0..1000 {
            let steps = belt.carry.take_budget(40, 20).unwrap();
            step_line(&mut belt, steps);
        }
        assert_eq!(
            belt.slots.iter().filter(|&&item| item != 0).count(),
            initial
        );
        assert_eq!(belt.slots, vec![0, 0, 0, 0, 1, 2, 3, 4]);
    }

    #[test]
    fn line_clock_runs_when_empty() {
        let mut app = crate::headless_app(20);
        let belt = line(vec![0; 4], 4);
        app.world_mut()
            .resource_mut::<Conveyors>()
            .lines
            .insert(1, belt);
        app.world_mut()
            .resource_mut::<crate::economy::transaction::AuditTrail>()
            .enable();
        for _ in 0..3 {
            app.update();
        }
        let conveyors = app.world().resource::<Conveyors>();
        assert_eq!(conveyors.lines[&1].carry.remainder(), 12);
        assert_eq!(conveyors.lines[&1].slots, vec![0; 4]);
        assert_eq!(
            app.world()
                .resource::<crate::economy::transaction::AuditTrail>()
                .operations
                .len(),
            3
        );
    }

    #[test]
    fn items_keep_moving_after_split() {
        use crate::{
            economy::{
                EconomyOrders,
                combat::{Healths, Weapons},
                events::{
                    NextEventOrdinal, PendingEconomyEvents, RemoveCause, RemovedThisTick,
                    SpawnedThisTick,
                },
                extractor::Extractors,
                inserter::Inserters,
                ledger::EconomyLedger,
                production::ProductionQueues,
                research::{Disabled, ResearchState},
                transaction::{remove_unit, transact},
            },
            ids::IdAllocators,
            slots::SlotId,
            world::{Owner, Position, Tick, UnitId},
        };

        for (name, pieces, slots, remove_piece, expected) in [
            (
                "first",
                vec![1, 2, 3, 4, 5, 6],
                vec![1, 0, 2, 0, 3, 0],
                1,
                vec![(2, vec![0, 0, 2, 0, 3])],
            ),
            (
                "middle",
                vec![1, 2, 3, 4, 5, 6],
                vec![1, 0, 2, 0, 3, 0],
                3,
                vec![(1, vec![0, 1]), (4, vec![0, 0, 3])],
            ),
            (
                "last",
                vec![1, 2, 3, 4, 5, 6],
                vec![1, 0, 2, 0, 3, 0],
                6,
                vec![(1, vec![0, 1, 0, 2, 3])],
            ),
            ("only", vec![7], vec![1], 7, vec![]),
        ] {
            let mut app = crate::headless_app(20);
            {
                let world = app.world_mut();
                world.insert_resource(IdAllocators::default());
                world.insert_resource(Tick::default());
                world.insert_resource(ProductionQueues::default());
                world.insert_resource(EconomyOrders::default());
                world.insert_resource(EconomyLedger::default());
                world.insert_resource(Inserters::default());
                world.insert_resource(Extractors::default());
                world.insert_resource(Weapons::default());
                world.insert_resource(Healths::default());
                world.insert_resource(ResearchState::default());
                world.insert_resource(Disabled::default());
                world.insert_resource(RemovedThisTick::default());
                world.insert_resource(SpawnedThisTick::default());
                world.insert_resource(PendingEconomyEvents::default());
                world.insert_resource(NextEventOrdinal::default());
                let line = BeltLine {
                    pieces: pieces.clone(),
                    slots: slots.clone(),
                    slots_per_piece: 1,
                    slots_per_second: 20,
                    ..line(slots.clone(), 20)
                };
                let key = pieces[0];
                let mut conveyors = Conveyors::default();
                conveyors.lines.insert(key, line);
                conveyors.piece_line.extend(
                    pieces
                        .iter()
                        .enumerate()
                        .map(|(index, piece)| (*piece, (key, index as u16))),
                );
                world.insert_resource(conveyors);
                world.insert_resource(ItemTable(vec![
                    ContentId::parse("probe:item-1").unwrap(),
                    ContentId::parse("probe:item-2").unwrap(),
                    ContentId::parse("probe:item-3").unwrap(),
                ]));
                world.spawn((
                    UnitId(remove_piece),
                    Owner(SlotId(0)),
                    Position(bevy_math::Vec2::ZERO),
                ));
                transact(world, |tx| {
                    remove_unit(tx, remove_piece, RemoveCause::Removed)
                })
                .unwrap_or_else(|error| panic!("{name} removal failed: {error:?}"));
            }
            app.update();
            let conveyors = app.world().resource::<Conveyors>();
            let actual = conveyors
                .lines
                .iter()
                .map(|(key, line)| (*key, line.slots.clone()))
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "case: {name}");
            let expected_carry = {
                let mut carry = RateCarry::default();
                carry.take_budget(20, 20).unwrap();
                carry
            };
            assert!(
                conveyors
                    .lines
                    .values()
                    .all(|line| line.carry == expected_carry),
                "case: {name}"
            );
        }
    }

    #[test]
    fn optimized_step_matches_literal_reference() {
        for len in 0..=8 {
            for mask in 0..(1usize << len) {
                let slots = (0..len)
                    .map(|index| {
                        if mask & (1 << index) == 0 {
                            0
                        } else {
                            (index + 1) as u16
                        }
                    })
                    .collect::<Vec<_>>();
                for steps in 0..=12 {
                    let mut fast = line(slots.clone(), 1);
                    let mut slow = fast.clone();
                    step_line(&mut fast, steps);
                    literal(&mut slow, steps);
                    assert_eq!(
                        fast.slots, slow.slots,
                        "len={len}, mask={mask}, steps={steps}"
                    );
                }
            }
        }
    }

    #[test]
    fn extreme_budget_terminates_and_matches_saturation() {
        let mut belt = line(vec![1, 2, 3, 0, 0, 0], u64::MAX);
        step_line(&mut belt, u64::MAX);
        assert_eq!(belt.slots, vec![0, 0, 0, 1, 2, 3]);
    }

    #[test]
    fn empty_transport_phase_has_no_subject_to_capture() {
        let mut app = crate::headless_app(20);
        app.world_mut()
            .resource_mut::<crate::economy::transaction::AuditTrail>()
            .enable();
        app.update();
        assert!(app.world().resource::<Conveyors>().lines.is_empty());
        assert!(
            app.world()
                .resource::<crate::economy::transaction::AuditTrail>()
                .operations
                .is_empty()
        );
    }
}
