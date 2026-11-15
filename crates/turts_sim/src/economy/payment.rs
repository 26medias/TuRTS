//! Exact proposed payment rows. Callers stage debits with their progress transaction.

use super::ledger::ReservationLine;
use crate::RejectReason;

#[derive(Debug, Clone)]
pub(crate) struct ProgressCharge {
    /// Positive differences, retaining authored order for funding diagnostics.
    pub(crate) debits: Vec<ReservationLine>,
    /// Exact cumulative paid amounts at the proposed progress.
    pub(crate) charged: Vec<ReservationLine>,
}

pub(crate) fn progress_charges(
    costs: &[ReservationLine],
    previous: u64,
    next: u64,
    total: u64,
) -> Result<ProgressCharge, RejectReason> {
    if total == 0 || previous > next || next > total {
        return Err(RejectReason::IntegrityViolation);
    }
    let mut proposal = ProgressCharge {
        debits: Vec::with_capacity(costs.len()),
        charged: Vec::with_capacity(costs.len()),
    };
    for cost in costs {
        let paid = turts_rules::charge::charged_at(cost.amount, next, total);
        let difference = turts_rules::charge::charge_difference(cost.amount, previous, next, total);
        if difference != 0 {
            proposal.debits.push(ReservationLine {
                amount: difference,
                ..cost.clone()
            });
        }
        if paid != 0 {
            proposal.charged.push(ReservationLine {
                amount: paid,
                ..cost.clone()
            });
        }
    }
    Ok(proposal)
}
