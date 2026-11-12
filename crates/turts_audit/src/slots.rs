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
        for (name, state) in [("start", step.before), ("end", step.after)] {
            if state.claims != self.claims {
                return Err(format!(
                    "slots: the slot claims at the {name} of {label} are {:?}, the Claim records give {:?}",
                    state.claims, self.claims
                ));
            }
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
