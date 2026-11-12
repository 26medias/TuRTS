//! The effects of one delta (design 6.8): `Fired`, `Destroyed` and `Credited`. They ride the
//! unreliable state channel and are never replayed. The writer of the sim rows is `L-FX-WIRE`;
//! the sim rows are empty in lane 0.
//!
//! This file reads only `CommittedTick` fields (the scan `order_writers.rs`).
//!
//! `Fired` and `Destroyed` go to each client that may see the unit (every client in MP1).
//! `Credited` goes to its owner. Removal rule: an effect is an event, not state. A kill shot has
//! a `Fired` that names a unit that the same delta removes, and `Destroyed` names it by design,
//! so the removal rule of the state records does not apply; the client must not check an effect
//! against its unit table.

use turts_net_protocol::messages as wire;
use turts_sim::economy::events::EffectRow;
use turts_sim::{CommittedTick, SlotId};

pub(super) fn effects(batch: &CommittedTick, slot: SlotId) -> Vec<wire::Effect> {
    batch
        .effects
        .iter()
        .filter_map(|row| match row {
            EffectRow::Fired {
                unit,
                target,
                weapon,
            } => Some(wire::Effect::Fired {
                unit: wire::UnitId(*unit),
                target: wire::UnitId(*target),
                weapon: *weapon,
            }),
            EffectRow::Destroyed { unit } => Some(wire::Effect::Destroyed {
                unit: wire::UnitId(*unit),
            }),
            EffectRow::Credited {
                unit,
                owner_slot,
                resource,
                amount,
            } if *owner_slot == slot.0 => Some(wire::Effect::Credited {
                unit: wire::UnitId(*unit),
                resource: resource.to_string(),
                amount: *amount,
            }),
            EffectRow::Credited { .. } => None,
        })
        .collect()
}
