//! Slots domain: slot claims and unit ownership (D25, spec 6.8).
//!
//! Rules:
//! - slots: a `Claim` record names a configured slot that is open.
//! - slots: a player claims at most one slot.
//! - slots: a `Claim` record is at the clock of the log.
//! - slots: the logged slot claims equal the claims of the `Claim` records, in every snapshot; so a
//!   claimed slot is never given to another player and never opens again.
//! - slots: a unit keeps its owner slot for as long as it exists.
//! - slots: an `Admission` record holds its ordinal, the wire format and the raw wire bytes, at
//!   the clock of its snapshot.

use std::collections::BTreeMap;

use crate::{LedgerAudit, Step};

pub(crate) struct Slots {
    claims: Vec<Option<u64>>,
}

impl Slots {
    pub(crate) fn new(initial: &LedgerAudit) -> Self {
        Self {
            claims: initial.claims.clone(),
        }
    }

    pub(crate) fn claim(
        &mut self,
        tick: u32,
        player_id: u64,
        slot: u8,
        clock: u32,
    ) -> Result<(), String> {
        if tick != clock {
            return Err(format!(
                "slots: the claim of player {player_id} has a discontinuous tick {tick}; the log clock is {clock}"
            ));
        }
        if let Some(held) = self
            .claims
            .iter()
            .position(|claim| *claim == Some(player_id))
        {
            return Err(format!(
                "slots: player {player_id} claims slot {slot} and already holds slot {held}"
            ));
        }
        match self.claims.get_mut(usize::from(slot)) {
            None => Err(format!(
                "slots: player {player_id} claims slot {slot}, which is not configured"
            )),
            Some(Some(holder)) => Err(format!(
                "slots: player {player_id} claims slot {slot}, which player {holder} holds"
            )),
            Some(open) => {
                *open = Some(player_id);
                Ok(())
            }
        }
    }

    pub(crate) fn check(&mut self, step: &Step<'_>) -> Result<(), String> {
        let label = step.label();
        // Each snapshot is the end of exactly one step. The start of a gap can be a snapshot from
        // before a `Claim` record, so only the end is compared with the Claim records.
        if step.after.claims != self.claims {
            return Err(format!(
                "slots: the slot claims at the end of {label} are {:?}, the Claim records give {:?}",
                step.after.claims, self.claims
            ));
        }
        let owners: BTreeMap<u32, u8> = step
            .before
            .raw_units
            .iter()
            .map(|unit| (unit.unit_id, unit.owner_slot))
            .collect();
        for unit in &step.after.raw_units {
            if let Some(owner) = owners.get(&unit.unit_id)
                && *owner != unit.owner_slot
            {
                return Err(format!(
                    "slots: unit {} changed its owner slot from {owner} to {} in {label}",
                    unit.unit_id, unit.owner_slot
                ));
            }
        }
        Ok(())
    }
}

pub(crate) fn check_admission(
    ordinal: u64,
    wire_format: &str,
    raw_envelope: &[u8],
    tick: u32,
    clock: u32,
) -> Result<(), String> {
    if ordinal == 0 || wire_format != "postcard" || raw_envelope.is_empty() {
        return Err("slots: admission evidence is missing its ordinal or raw wire bytes".into());
    }
    if clock != tick {
        return Err(format!(
            "slots: wire admission tick {tick} differs from its snapshot clock {clock}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{EconomyLogRecord, validate_ledger};

    fn initial() -> turts_sim::AuditState {
        let mut app = turts_sim::headless_app(10);
        turts_sim::audit_state(app.world_mut())
    }

    fn claim_then_tick(claimed: Option<u64>) -> Vec<EconomyLogRecord> {
        let before = initial();
        let mut state = before.clone();
        state.economy.tick += 1;
        state.claims[0] = claimed;
        vec![
            EconomyLogRecord::Initial(before.clone()),
            EconomyLogRecord::Claim {
                tick: 0,
                player_id: 7,
                slot: 0,
            },
            EconomyLogRecord::Tick {
                tick: 1,
                before,
                state,
                events: Vec::new(),
            },
        ]
    }

    /// A Claim record comes after the snapshot that the next step starts from (the E8 and E9
    /// process logs). The snapshot after the Claim holds the claim.
    #[test]
    fn a_claim_between_snapshots_is_accepted() {
        assert_eq!(validate_ledger(&claim_then_tick(Some(7))), Ok(()));
    }

    #[test]
    fn a_snapshot_without_the_claim_is_rejected() {
        let error = validate_ledger(&claim_then_tick(None)).unwrap_err();
        assert!(
            error.starts_with("slots: the slot claims at the end of the end of tick 1"),
            "{error}"
        );
    }
}
