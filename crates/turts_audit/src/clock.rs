//! Clock domain: the simulation clock, the operation ids and the event ordinals.
//!
//! Rules:
//! - clock: each `Tick` record names the tick after the previous `Tick` record.
//! - clock: a `Tick` record starts at the clock before its tick and ends at its tick.
//! - clock: the clock of a record is never below the clock of the previous record, and never above
//!   the last published tick plus one.
//! - clock: an operation does not change the clock.
//! - clock: each operation id is the previous operation id plus one.
//! - clock: an operation appends exactly its own events to the pending events, each at the clock of
//!   the operation, with the ordinals that follow the pending ordinal.
//! - clock: the pending events and the event ordinal change only inside an operation or at the
//!   end of a tick.
//! - clock: a `Tick` record publishes exactly the events of the operations of its tick, in
//!   ordinal order, and resets the pending events and the ordinal.

use turts_sim::EconomyEvent;

use crate::{LedgerAudit, Step, StepKind};

pub(crate) struct Clock {
    last_tick: u32,
    clock: u32,
    last_operation: Option<u64>,
    tick_events: Vec<EconomyEvent>,
}

impl Clock {
    pub(crate) fn new(initial: &LedgerAudit) -> Self {
        Self {
            last_tick: initial.economy.tick,
            clock: initial.economy.tick,
            last_operation: None,
            tick_events: initial.pending_events.clone(),
        }
    }

    fn see_clock(&mut self, clock: u32, label: &str) -> Result<(), String> {
        let Some(limit) = self.last_tick.checked_add(1) else {
            return Err("clock: the tick counter overflows".into());
        };
        if clock < self.clock || clock > limit {
            return Err(format!(
                "clock: {label} is at clock {clock} after clock {} (last tick {})",
                self.clock, self.last_tick
            ));
        }
        self.clock = clock;
        Ok(())
    }

    pub(crate) fn check(&mut self, step: &Step<'_>) -> Result<(), String> {
        let label = step.label();
        match step.kind {
            StepKind::Operation(id) => {
                if step.before.economy.tick != step.after.economy.tick {
                    return Err(format!("clock: {label} changes the clock"));
                }
                self.see_clock(step.before.economy.tick, &label)?;
                if self
                    .last_operation
                    .is_some_and(|previous| previous.checked_add(1) != Some(id))
                {
                    return Err(format!(
                        "clock: operation id {id} follows operation id {}",
                        self.last_operation.unwrap_or_default()
                    ));
                }
                self.last_operation = Some(id);
                let clock = step.before.economy.tick;
                let mut expected = step.before.pending_events.clone();
                let mut ordinal = step.before.next_event_ordinal;
                for event in step.events {
                    if event.tick != clock {
                        return Err(format!(
                            "clock: {label} holds an event of tick {} at clock {clock}",
                            event.tick
                        ));
                    }
                    if Some(event.ordinal) != ordinal {
                        return Err(format!(
                            "clock: {label} holds event ordinal {} where the next ordinal is {ordinal:?}",
                            event.ordinal
                        ));
                    }
                    ordinal = ordinal.and_then(|value| value.checked_add(1));
                    expected.push(event.clone());
                }
                if step.after.pending_events != expected || step.after.next_event_ordinal != ordinal
                {
                    return Err(format!(
                        "clock: {label} pending events differ from its own events"
                    ));
                }
                self.tick_events.extend(step.events.iter().cloned());
            }
            StepKind::Gap => {
                self.see_clock(step.after.economy.tick, &label)?;
                if step.before.pending_events != step.after.pending_events
                    || step.before.next_event_ordinal != step.after.next_event_ordinal
                {
                    return Err(format!(
                        "clock: the pending events changed outside an operation at {label}"
                    ));
                }
            }
            StepKind::TickEnd(_) => {}
        }
        Ok(())
    }

    /// The `Tick` record: numbering, its two clocks, and the published events.
    pub(crate) fn tick(
        &mut self,
        tick: u32,
        before_clock: u32,
        state: &LedgerAudit,
        events: &[EconomyEvent],
    ) -> Result<(), String> {
        if self.last_tick.checked_add(1) != Some(tick) {
            return Err(format!(
                "clock: tick {tick} follows tick {}",
                self.last_tick
            ));
        }
        if before_clock.checked_add(1) != Some(tick) || state.economy.tick != tick {
            return Err(format!(
                "clock: tick {tick} record runs from clock {before_clock} to clock {}",
                state.economy.tick
            ));
        }
        self.see_clock(tick, &format!("tick {tick}"))?;
        let mut published = std::mem::take(&mut self.tick_events);
        published.sort_by_key(|event| event.ordinal);
        if events != published.as_slice() {
            return Err(format!(
                "clock: tick {tick} publishes {} events, its operations hold {}",
                events.len(),
                published.len()
            ));
        }
        if !state.pending_events.is_empty() || state.next_event_ordinal != Some(0) {
            return Err(format!(
                "clock: tick {tick} did not reset the pending events and the ordinal"
            ));
        }
        self.last_tick = tick;
        Ok(())
    }
}
