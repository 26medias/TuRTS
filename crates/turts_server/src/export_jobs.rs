//! The production job records of one delta (design 6.8): the same form as `SiteProgress`, owner
//! only, sent on change of rate, job state or `held`. The writer of the sim rows is `L-PAY`; the
//! sim rows are empty in lane 0.
//!
//! This file reads only `CommittedTick` fields (the scan `order_writers.rs`).
//!
//! Removal rule: a producer that the delta removes has no job record in the delta. The
//! client drops the queue of a removed producer when it applies `removed`.

use turts_net_protocol::messages as wire;
use turts_sim::economy::events::{RateRecordRow, RecordStateRow};
use turts_sim::{CommittedTick, SlotId};

pub(super) fn rate_record(row: &RateRecordRow) -> wire::RateRecord {
    wire::RateRecord {
        done: row.done,
        total: row.total,
        per_tick: row.per_tick,
        at_tick: row.at_tick,
        state: match &row.state {
            RecordStateRow::Running => wire::RecordState::Running,
            RecordStateRow::Slowed => wire::RecordState::Slowed,
            RecordStateRow::WaitingForFunds(resource) => {
                wire::RecordState::WaitingForFunds(resource.to_string())
            }
            RecordStateRow::OutputBlocked => wire::RecordState::OutputBlocked,
            RecordStateRow::Held => wire::RecordState::Held,
        },
        charged: row.charged.iter().map(super::cost).collect(),
        held: row.held,
        held_since_tick: row.held_since_tick,
    }
}

/// The job progress rows of the owner. A removed producer has no row.
pub(super) fn job_progress(batch: &CommittedTick, slot: SlotId) -> Vec<wire::JobProgress> {
    batch
        .job_progress
        .iter()
        .filter(|row| row.owner_slot == slot.0 && batch.removed.binary_search(&row.unit).is_err())
        .filter_map(|row| {
            Some(wire::JobProgress {
                producer: wire::UnitId(row.unit),
                job: wire::JobId(row.job?),
                progress: rate_record(row),
            })
        })
        .collect()
}

/// The job progress record of every job in the queues, at `tick` (design 6.8, 6.4). The sim
/// view holds the elapsed ticks of a job; the wire holds `done` of `total` at `at_tick` and the
/// rate: 1 tick for each tick while the job runs, 0 while it waits or its output is blocked.
/// `L-PAY` replaces the source with `CommittedTick::job_progress` (rates other than 1, the
/// held and starved states); the record shape does not change.
pub(super) fn queue_job_records(
    queues: &[turts_sim::economy::views::QueueView],
    tick: u32,
) -> Vec<wire::JobProgress> {
    use turts_sim::economy::production::JobStage;
    queues
        .iter()
        .flat_map(|queue| {
            queue.jobs.iter().map(move |job| wire::JobProgress {
                producer: wire::UnitId(queue.producer),
                job: wire::JobId(job.id),
                progress: wire::RateRecord {
                    done: job.elapsed_ticks,
                    total: job.duration_ticks,
                    per_tick: u64::from(job.stage == JobStage::Running),
                    at_tick: tick,
                    state: match job.stage {
                        JobStage::OutputBlocked => wire::RecordState::OutputBlocked,
                        JobStage::Queued | JobStage::Running => wire::RecordState::Running,
                    },
                    charged: Vec::new(),
                    held: false,
                    held_since_tick: 0,
                },
            })
        })
        .collect()
}
