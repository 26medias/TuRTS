//! Byte permits follow packets through the frontend queue and socket write.
use bytes::Bytes;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

/// Maximum retained payload and packet bookkeeping bytes per socket sender.
pub const MAX_WEBSOCKET_SEND_QUEUE_BYTES: usize = 4 * 1024 * 1024;
// Charge empty messages too, including queue/Bytes/permit bookkeeping.
const ENTRY_OVERHEAD: usize = 64;

#[derive(Debug, Clone)]
pub(crate) struct SendBudget {
    used: Arc<AtomicUsize>,
    limit: usize,
}
impl Default for SendBudget {
    fn default() -> Self {
        Self {
            used: Arc::new(AtomicUsize::new(0)),
            limit: MAX_WEBSOCKET_SEND_QUEUE_BYTES,
        }
    }
}
impl SendBudget {
    pub(crate) fn used(&self) -> usize {
        self.used.load(Ordering::Relaxed)
    }

    pub(crate) fn packet(&self, payload: Bytes) -> Option<QueuedPacket> {
        let charge = payload.len().checked_add(ENTRY_OVERHEAD)?;
        let mut used = self.used.load(Ordering::Relaxed);
        loop {
            let next = used.checked_add(charge).filter(|next| *next <= self.limit)?;
            match self.used.compare_exchange_weak(used, next, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(actual) => used = actual,
            }
        }
        Some(QueuedPacket {
            payload,
            permit: BytePermit {
                budget: self.clone(),
                charge,
            },
        })
    }
}
#[derive(Debug)]
pub(crate) struct QueuedPacket {
    pub(crate) payload: Bytes,
    pub(crate) permit: BytePermit,
}
#[derive(Debug)]
pub(crate) struct BytePermit {
    budget: SendBudget,
    charge: usize,
}
impl Drop for BytePermit {
    fn drop(&mut self) {
        let old = self.budget.used.fetch_sub(self.charge, Ordering::Relaxed);
        debug_assert!(old >= self.charge);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queued_and_active_packets_keep_credit_until_their_payload_is_released() {
        let budget = SendBudget {
            limit: 2 * (ENTRY_OVERHEAD + 10),
            ..Default::default()
        };
        let queued = budget.packet(Bytes::from(vec![0; 10])).unwrap();
        let active = budget.packet(Bytes::from(vec![0; 10])).unwrap();
        assert!(budget.packet(Bytes::new()).is_none());
        let QueuedPacket { payload, permit } = active;
        assert_eq!(budget.used(), 2 * (ENTRY_OVERHEAD + 10));
        drop(payload);
        drop(permit);
        assert_eq!(budget.used(), ENTRY_OVERHEAD + 10);
        drop(queued);
        assert_eq!(budget.used(), 0);
    }
    #[test]
    fn dropping_a_closed_queue_returns_all_packet_credit() {
        let budget = SendBudget::default();
        let (tx, rx) = futures::channel::mpsc::unbounded();
        tx.unbounded_send(budget.packet(Bytes::from_static(b"payload")).unwrap())
            .unwrap();
        assert!(budget.used() > 0);
        drop(rx);
        assert_eq!(budget.used(), 0);
        assert!(
            tx.unbounded_send(budget.packet(Bytes::new()).unwrap())
                .is_err()
        );
        assert_eq!(budget.used(), 0);
    }
    #[test]
    fn concurrent_reservations_never_exceed_the_shared_byte_limit() {
        let budget = SendBudget {
            limit: 4 * ENTRY_OVERHEAD,
            ..Default::default()
        };
        let barrier = Arc::new(std::sync::Barrier::new(16));
        let threads: Vec<_> = (0..16)
            .map(|_| {
                let budget = budget.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let packet = budget.packet(Bytes::new());
                    barrier.wait();
                    assert!(budget.used() <= budget.limit);
                    packet
                })
            })
            .collect();
        let packets: Vec<_> = threads
            .into_iter()
            .filter_map(|thread| thread.join().unwrap())
            .collect();
        assert_eq!(packets.len(), 4);
        drop(packets);
        assert_eq!(budget.used(), 0);
    }
}
