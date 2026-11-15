//! Optional host receipts for ordinary grouped command admission.
//!
//! These records identify work. They never authorize or apply a gameplay change.
//! Hosts install tracking only when they need exact per-member work identities.

use std::collections::{BTreeMap, BTreeSet};

use crate::economy::views::{ReservationLineView, reservation_line_view};
use bevy_ecs::prelude::Resource;
use bevy_ecs::world::World;
use serde::{Deserialize, Serialize};
use turts_mod_api::ContentId;

use crate::{JobId, SlotId};

/// A command member, independent of network connections and bot implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CommandMember {
    pub slot: u8,
    pub sequence: u32,
    pub unit: u32,
}

/// Exact admission facts, retained even if later work finishes or is destroyed in this tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedWork {
    pub unit: u32,
    pub job: JobId,
    pub recipe: ContentId,
    pub construction: bool,
    pub done: u64,
    pub total: u64,
    pub payment: Option<turts_mod_api::PaymentPolicy>,
    pub paid: Vec<ReservationLineView>,
    pub reserved: Vec<ReservationLineView>,
}

/// The final exact work state, captured in the transaction that ends it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalWorkState {
    Completed,
    Cancelled,
    Destroyed,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalWork {
    pub unit: u32,
    pub job: u64,
    pub recipe: ContentId,
    pub construction: bool,
    pub done: u64,
    pub total: u64,
    pub payment: Option<turts_mod_api::PaymentPolicy>,
    pub paid: Vec<ReservationLineView>,
    pub reserved: Vec<ReservationLineView>,
    pub state: TerminalWorkState,
}

/// Exact job identities returned by successful ordinary order application.
/// The host must bound watched requests and drain them after the completed tick.
#[derive(Resource, Default)]
pub struct CommandWorkReceipts {
    watched: BTreeSet<(u8, u32)>,
    jobs: BTreeMap<CommandMember, AdmittedWork>,
    tracked_jobs: BTreeSet<u64>,
    terminals: Vec<TerminalWork>,
}

impl CommandWorkReceipts {
    /// Register an admitted envelope. This grants no command permission.
    pub fn watch(&mut self, slot: SlotId, sequence: u32) {
        self.watched.insert((slot.0, sequence));
    }

    pub(crate) fn member(&self, slot: SlotId, sequence: u32, unit: u32) -> Option<CommandMember> {
        self.watched
            .contains(&(slot.0, sequence))
            .then_some(CommandMember {
                slot: slot.0,
                sequence,
                unit,
            })
    }

    pub(crate) fn record(&mut self, slot: SlotId, sequence: u32, unit: u32, work: AdmittedWork) {
        self.tracked_jobs.insert(work.job.0);
        self.jobs.insert(
            CommandMember {
                slot: slot.0,
                sequence,
                unit,
            },
            work,
        );
    }

    pub(crate) fn tracks(&self, job: u64) -> bool {
        self.tracked_jobs.contains(&job)
    }
    pub(crate) fn finish(&mut self, work: TerminalWork) {
        if self.tracked_jobs.remove(&work.job) {
            self.terminals.push(work);
        }
    }
    pub(crate) fn drain_terminals(&mut self) -> Vec<TerminalWork> {
        std::mem::take(&mut self.terminals)
    }
    pub fn forget_job(&mut self, job: u64) {
        self.tracked_jobs.remove(&job);
    }

    /// Consume only this envelope's job hints. Per-member acceptance/refusal still
    /// comes from the ordinary CommandResult, including envelope-level failures.
    pub fn take(&mut self, slot: SlotId, sequence: u32) -> BTreeMap<u32, AdmittedWork> {
        self.watched.remove(&(slot.0, sequence));
        let keys = self
            .jobs
            .keys()
            .filter(|key| key.slot == slot.0 && key.sequence == sequence)
            .copied()
            .collect::<Vec<_>>();
        keys.into_iter()
            .filter_map(|key| self.jobs.remove(&key).map(|job| (key.unit, job)))
            .collect()
    }
}

/// Read the successful transaction's own data before another command or work phase can remove it.
pub(crate) fn capture_admitted_work(
    world: &mut World,
    slot: SlotId,
    sequence: u32,
    unit: u32,
    job: JobId,
) {
    capture_work(world, slot, sequence, unit, job, false);
}

pub(crate) fn capture_origin_work(world: &mut World, origin: CommandMember, job: JobId) {
    capture_work(
        world,
        SlotId(origin.slot),
        origin.sequence,
        origin.unit,
        job,
        true,
    );
}

fn capture_work(
    world: &mut World,
    slot: SlotId,
    sequence: u32,
    unit: u32,
    job: JobId,
    origin: bool,
) {
    if !world.contains_resource::<CommandWorkReceipts>() {
        return;
    }
    if !origin
        && world
            .get_resource::<CommandWorkReceipts>()
            .and_then(|receipts| receipts.member(slot, sequence, unit))
            .is_none()
    {
        return;
    }
    let site = world
        .get_resource::<crate::economy::construction::ConstructionState>()
        .and_then(|state| state.sites.iter().find(|(_, site)| site.job == job))
        .map(|(id, site)| AdmittedWork {
            unit: *id,
            job,
            recipe: site.recipe.clone(),
            construction: true,
            done: site.done,
            total: site.total,
            payment: world
                .get_resource::<crate::bootstrap::InstalledCatalog>()
                .and_then(|catalog| catalog.0.production.get(&site.recipe))
                .map(|recipe| recipe.payment),
            paid: site.charged_rows(),
            reserved: vec![],
        });
    let work = site.or_else(|| {
        world
            .get_resource::<crate::economy::production::ProductionQueues>()
            .and_then(|queues| queues.0.get(&unit))
            .and_then(|queue| queue.iter().find(|item| item.id == job))
            .map(|item| AdmittedWork {
                unit,
                job,
                recipe: item.recipe.clone(),
                construction: false,
                done: item.elapsed_ticks,
                total: item.total,
                payment: world
                    .get_resource::<crate::bootstrap::InstalledCatalog>()
                    .and_then(|catalog| {
                        catalog
                            .0
                            .production
                            .get(&item.recipe)
                            .map(|recipe| recipe.payment)
                            .or_else(|| {
                                catalog
                                    .0
                                    .technologies
                                    .get(&item.recipe)
                                    .map(|recipe| recipe.payment)
                            })
                    }),
                paid: item
                    .charged
                    .iter()
                    .cloned()
                    .map(reservation_line_view)
                    .collect(),
                reserved: item
                    .reservation
                    .as_ref()
                    .map(|reservation| {
                        reservation
                            .lines
                            .iter()
                            .cloned()
                            .map(reservation_line_view)
                            .collect()
                    })
                    .unwrap_or_default(),
            })
    });
    if let Some(work) = work {
        world
            .resource_mut::<CommandWorkReceipts>()
            .record(slot, sequence, unit, work);
    }
}
