//! Shared bounded atomic-state framing. A staged payload is never an activated view.
use crate::visibility::*;
use turts_mod_api::{ContentId, ObservationPhase};

#[path = "visibility_decode.rs"]
mod decode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateTransactionError {
    FrameLimit,
    MemoryLimit,
    InvalidStamp,
    InvalidShape,
    PendingTransaction,
    ConflictingDuplicate,
    MissingChunks,
    MalformedPayload,
}
#[derive(Debug, Clone, PartialEq)]
pub enum StateTransactionFrame {
    Begin(StateTransactionBegin),
    Chunk(StateTransactionChunk),
    End(StateTransactionEnd),
}
impl StateTransactionFrame {
    pub fn encoded_bytes(&self) -> Result<usize, StateTransactionError> {
        let size = match self {
            Self::Begin(value) => postcard::experimental::serialized_size(value),
            Self::Chunk(value) => postcard::experimental::serialized_size(value),
            Self::End(value) => postcard::experimental::serialized_size(value),
        };
        size.map_err(|_| StateTransactionError::MalformedPayload)
    }
}
fn capacity(
    stamp: &StateTransactionStamp,
    frame_limit: usize,
) -> Result<usize, StateTransactionError> {
    // Reserve the largest index and vector-length varints. Account transport headers in caller's limit.
    let overhead = postcard::experimental::serialized_size(&StateTransactionChunk {
        stamp: stamp.clone(),
        index: u32::MAX,
        bytes: Vec::new(),
    })
    .map_err(|_| StateTransactionError::InvalidShape)?;
    frame_limit
        .min(MAX_STATE_CHUNK_BYTES)
        .checked_sub(overhead + 9)
        .filter(|size| *size > 0)
        .ok_or(StateTransactionError::FrameLimit)
}
fn shape(
    begin: &StateTransactionBegin,
    frame_limit: usize,
) -> Result<(usize, usize), StateTransactionError> {
    if begin.stamp.transaction_id == 0
        || begin.stamp.basis.phase != ObservationPhase::Completed
        || matches!(begin.kind, StateTransactionKind::Snapshot)
            != begin.stamp.parent_transaction_id.is_none()
    {
        return Err(StateTransactionError::InvalidStamp);
    }
    let bytes =
        usize::try_from(begin.encoded_bytes).map_err(|_| StateTransactionError::MemoryLimit)?;
    let per_chunk = capacity(&begin.stamp, frame_limit)?;
    let count = bytes.div_ceil(per_chunk);
    if bytes == 0 || u32::try_from(count).ok() != Some(begin.chunk_count) {
        return Err(StateTransactionError::InvalidShape);
    }
    let metadata = count.div_ceil(8).saturating_add(4096);
    if bytes
        .checked_add(metadata)
        .is_none_or(|total| total as u64 > MAX_PENDING_STATE_BYTES)
    {
        return Err(StateTransactionError::MemoryLimit);
    }
    let control = postcard::experimental::serialized_size(begin)
        .map_err(|_| StateTransactionError::InvalidShape)?;
    if control > frame_limit.min(MAX_STATE_CHUNK_BYTES) {
        return Err(StateTransactionError::FrameLimit);
    }
    Ok((bytes, per_chunk))
}
fn payload_valid(stamp: &StateTransactionStamp, payload: &StateTransactionPayload) -> bool {
    let (tick, knowledge, kind) = match payload {
        StateTransactionPayload::Snapshot {
            state, knowledge, ..
        } => (
            state.tick,
            knowledge.as_ref().map(|k| &k.basis),
            StateTransactionKind::Snapshot,
        ),
        StateTransactionPayload::Patch {
            state, knowledge, ..
        } => (
            state.tick,
            knowledge.as_ref().map(|k| &k.basis),
            StateTransactionKind::Patch,
        ),
    };
    tick == stamp.basis.tick
        && knowledge.is_none_or(|basis| basis == &stamp.basis)
        && (kind == StateTransactionKind::Snapshot) == stamp.parent_transaction_id.is_none()
}
/// Owns one encoded payload. The transport must include queued copies in its own byte budget.
pub struct EncodedStateTransaction {
    begin: StateTransactionBegin,
    bytes: Vec<u8>,
    per_chunk: usize,
    cursor: u32,
    started: bool,
    ended: bool,
}
impl EncodedStateTransaction {
    pub fn begin(&self) -> &StateTransactionBegin {
        &self.begin
    }
    pub fn retained_bytes(&self) -> usize {
        self.bytes.capacity()
    }
    /// Produces at most one bounded frame. Call only when the transport has queue headroom.
    pub fn next_frame(&mut self) -> Option<StateTransactionFrame> {
        if !self.started {
            self.started = true;
            return Some(StateTransactionFrame::Begin(self.begin.clone()));
        }
        if self.cursor < self.begin.chunk_count {
            let index = self.cursor;
            self.cursor += 1;
            let offset = index as usize * self.per_chunk;
            let end = (offset + self.per_chunk).min(self.bytes.len());
            return Some(StateTransactionFrame::Chunk(StateTransactionChunk {
                stamp: self.begin.stamp.clone(),
                index,
                bytes: self.bytes[offset..end].to_vec(),
            }));
        }
        if !self.ended {
            self.ended = true;
            return Some(StateTransactionFrame::End(StateTransactionEnd {
                stamp: self.begin.stamp.clone(),
            }));
        }
        None
    }
    pub fn fully_queued(&self) -> bool {
        self.ended
    }
}
pub fn encode_transaction(
    stamp: StateTransactionStamp,
    payload: &StateTransactionPayload,
    frame_limit: usize,
) -> Result<EncodedStateTransaction, StateTransactionError> {
    if !payload_valid(&stamp, payload) {
        return Err(StateTransactionError::InvalidStamp);
    }
    encode_serialized(
        stamp,
        if matches!(payload, StateTransactionPayload::Snapshot { .. }) {
            StateTransactionKind::Snapshot
        } else {
            StateTransactionKind::Patch
        },
        payload,
        frame_limit,
    )
}
#[derive(serde::Serialize)]
enum SnapshotReference<'a> {
    Snapshot {
        state: &'a crate::Snapshot,
        knowledge: Option<&'a turts_mod_api::KnowledgeSnapshot>,
        event_units: &'a [crate::EventUnitObservation],
    },
}
pub fn encoded_snapshot_size(
    state: &crate::Snapshot,
    knowledge: Option<&turts_mod_api::KnowledgeSnapshot>,
) -> Result<usize, StateTransactionError> {
    postcard::experimental::serialized_size(&SnapshotReference::Snapshot {
        state,
        knowledge,
        event_units: &[],
    })
    .map_err(|_| StateTransactionError::MalformedPayload)
}
/// Snapshot encoding borrows the already-permitted baseline. It never clones its full tables.
pub fn encode_snapshot(
    stamp: StateTransactionStamp,
    state: &crate::Snapshot,
    knowledge: Option<&turts_mod_api::KnowledgeSnapshot>,
    frame_limit: usize,
) -> Result<EncodedStateTransaction, StateTransactionError> {
    if stamp.parent_transaction_id.is_some()
        || state.tick != stamp.basis.tick
        || knowledge.is_some_and(|knowledge| knowledge.basis != stamp.basis)
    {
        return Err(StateTransactionError::InvalidStamp);
    }
    encode_serialized(
        stamp,
        StateTransactionKind::Snapshot,
        &SnapshotReference::Snapshot {
            state,
            knowledge,
            event_units: &[],
        },
        frame_limit,
    )
}
fn encode_serialized(
    stamp: StateTransactionStamp,
    kind: StateTransactionKind,
    payload: &impl serde::Serialize,
    frame_limit: usize,
) -> Result<EncodedStateTransaction, StateTransactionError> {
    let size = postcard::experimental::serialized_size(payload)
        .map_err(|_| StateTransactionError::MalformedPayload)?;
    let per_chunk = capacity(&stamp, frame_limit)?;
    let begin = StateTransactionBegin {
        kind,
        stamp,
        encoded_bytes: size as u64,
        chunk_count: u32::try_from(size.div_ceil(per_chunk))
            .map_err(|_| StateTransactionError::MemoryLimit)?,
    };
    shape(&begin, frame_limit)?;
    let bytes =
        postcard::to_allocvec(payload).map_err(|_| StateTransactionError::MalformedPayload)?;
    Ok(EncodedStateTransaction {
        begin,
        bytes,
        per_chunk,
        cursor: 0,
        started: false,
        ended: false,
    })
}

struct Pending {
    begin: StateTransactionBegin,
    bytes: Vec<u8>,
    received: Vec<u8>,
    chunks: u32,
    per_chunk: usize,
    started_ms: u64,
}
/// Returned once, only after exact assembly and bounded decoding. The caller validates game references.
pub struct StagedStateTransaction {
    pub stamp: StateTransactionStamp,
    pub payload: StateTransactionPayload,
}
/// Holds only one incomplete transaction. Activation is a separate client operation.
pub struct StateTransactionAssembler {
    match_id: u64,
    map: ContentId,
    frame_limit: usize,
    highest_seen: u64,
    active: Option<StateTransactionStamp>,
    staged: Option<StateTransactionStamp>,
    pending: Option<Pending>,
}
impl StateTransactionAssembler {
    pub fn new(match_id: u64, map: ContentId, frame_limit: usize) -> Self {
        Self {
            match_id,
            map,
            frame_limit,
            highest_seen: 0,
            active: None,
            staged: None,
            pending: None,
        }
    }
    pub fn active(&self) -> Option<&StateTransactionStamp> {
        self.active.as_ref()
    }
    /// Delayed frames from an already completed or cancelled stream cannot restart a basis.
    pub fn is_obsolete(&self, stamp: &StateTransactionStamp) -> bool {
        stamp.basis.match_id == self.match_id
            && stamp.basis.map == self.map
            && (stamp.transaction_id < self.highest_seen
                || stamp.transaction_id == self.highest_seen
                    && self
                        .pending
                        .as_ref()
                        .is_none_or(|pending| pending.begin.stamp != *stamp))
    }
    pub fn retained_bytes(&self) -> usize {
        self.pending
            .as_ref()
            .map_or(0, |p| p.bytes.capacity() + p.received.capacity())
    }
    pub fn begin(
        &mut self,
        begin: StateTransactionBegin,
        now_ms: u64,
    ) -> Result<(), StateTransactionError> {
        if self.pending.as_ref().is_some_and(|p| p.begin == begin) {
            return Ok(());
        }
        if self.pending.is_some() || self.staged.is_some() {
            return Err(StateTransactionError::PendingTransaction);
        }
        let stamp = &begin.stamp;
        if stamp.basis.match_id != self.match_id
            || stamp.basis.map != self.map
            || stamp.transaction_id <= self.highest_seen
            || stamp.parent_transaction_id.is_some_and(|parent| {
                self.active.as_ref().is_none_or(|active| {
                    active.transaction_id != parent || stamp.basis.tick < active.basis.tick
                })
            })
            || self
                .active
                .as_ref()
                .is_some_and(|active| stamp.basis.tick < active.basis.tick)
        {
            return Err(StateTransactionError::InvalidStamp);
        }
        let (bytes, per_chunk) = shape(&begin, self.frame_limit)?;
        self.highest_seen = stamp.transaction_id;
        let received = vec![0; (begin.chunk_count as usize).div_ceil(8)];
        self.pending = Some(Pending {
            begin,
            bytes: vec![0; bytes],
            received,
            chunks: 0,
            per_chunk,
            started_ms: now_ms,
        });
        Ok(())
    }
    pub fn chunk(&mut self, chunk: &StateTransactionChunk) -> Result<(), StateTransactionError> {
        let pending = self
            .pending
            .as_mut()
            .ok_or(StateTransactionError::InvalidStamp)?;
        if chunk.stamp != pending.begin.stamp {
            return Err(StateTransactionError::InvalidStamp);
        }
        if chunk.index >= pending.begin.chunk_count
            || postcard::experimental::serialized_size(chunk)
                .map_err(|_| StateTransactionError::InvalidShape)?
                > self.frame_limit.min(MAX_STATE_CHUNK_BYTES)
        {
            return Err(StateTransactionError::InvalidShape);
        }
        let offset = (chunk.index as usize)
            .checked_mul(pending.per_chunk)
            .ok_or(StateTransactionError::InvalidShape)?;
        let expected = pending.per_chunk.min(pending.bytes.len() - offset);
        if chunk.bytes.len() != expected {
            return Err(StateTransactionError::InvalidShape);
        }
        let byte = chunk.index as usize / 8;
        let bit = 1 << (chunk.index % 8);
        if pending.received[byte] & bit != 0 {
            return if pending.bytes[offset..offset + expected] == chunk.bytes {
                Ok(())
            } else {
                Err(StateTransactionError::ConflictingDuplicate)
            };
        }
        pending.bytes[offset..offset + expected].copy_from_slice(&chunk.bytes);
        pending.received[byte] |= bit;
        pending.chunks += 1;
        Ok(())
    }
    pub fn end(
        &mut self,
        end: &StateTransactionEnd,
    ) -> Result<Option<StagedStateTransaction>, StateTransactionError> {
        if self.staged.as_ref() == Some(&end.stamp) || self.active.as_ref() == Some(&end.stamp) {
            return Ok(None);
        }
        let pending = self
            .pending
            .as_ref()
            .ok_or(StateTransactionError::InvalidStamp)?;
        if pending.begin.stamp != end.stamp {
            return Err(StateTransactionError::InvalidStamp);
        }
        if pending.chunks != pending.begin.chunk_count {
            return Err(StateTransactionError::MissingChunks);
        }
        let pending = self.pending.take().unwrap();
        let remaining = MAX_PENDING_STATE_BYTES as usize
            - pending.bytes.capacity()
            - pending.received.capacity();
        let payload = decode::payload(&pending.bytes, remaining)?;
        if !payload_valid(&end.stamp, &payload) {
            return Err(StateTransactionError::InvalidStamp);
        }
        self.staged = Some(end.stamp.clone());
        Ok(Some(StagedStateTransaction {
            stamp: end.stamp.clone(),
            payload,
        }))
    }
    /// Call after semantic validation and atomic installation. Only then may an acknowledgement leave.
    pub fn activate(
        &mut self,
        stamp: &StateTransactionStamp,
    ) -> Result<StateTransactionAck, StateTransactionError> {
        if self.staged.as_ref() != Some(stamp) {
            return Err(StateTransactionError::InvalidStamp);
        }
        self.active = self.staged.take();
        Ok(StateTransactionAck {
            stamp: stamp.clone(),
        })
    }
    pub fn cancel(&mut self, stamp: &StateTransactionStamp) {
        // Control cancellation can overtake a state-channel Begin. Retire its ID before receipt.
        if stamp.basis.match_id != self.match_id || stamp.basis.map != self.map {
            return;
        }
        self.highest_seen = self.highest_seen.max(stamp.transaction_id);
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| &pending.begin.stamp == stamp)
        {
            self.pending = None;
        }
        if self.staged.as_ref() == Some(stamp) {
            self.staged = None;
        }
    }
    pub fn expire(&mut self, now_ms: u64) -> Option<StateTransactionStamp> {
        if self.pending.as_ref().is_some_and(|pending| {
            now_ms.saturating_sub(pending.started_ms) >= STATE_TRANSACTION_TIMEOUT_MS
        }) {
            return self.pending.take().map(|pending| pending.begin.stamp);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Pos, Snapshot, UnitId, UnitSnapshot, WorldDelta};
    use turts_mod_api::KnowledgeBasis;
    fn stamp(id: u64, parent: Option<u64>) -> StateTransactionStamp {
        StateTransactionStamp {
            basis: KnowledgeBasis {
                match_id: 7,
                map: ContentId::parse("fixture:map").unwrap(),
                tick: id as u32,
                phase: ObservationPhase::Completed,
            },
            transaction_id: id,
            parent_transaction_id: parent,
        }
    }
    fn payload(id: u64, large: bool) -> StateTransactionPayload {
        let mut state = Snapshot {
            tick: id as u32,
            ..Default::default()
        };
        if large {
            state.units = (1..=200)
                .map(|unit| UnitSnapshot {
                    unit_id: UnitId(unit),
                    content_id: "fixture:ordinary_unit".into(),
                    owner_slot: crate::SlotId(0),
                    facing: None,
                    pos: Pos {
                        x: unit as f32,
                        y: 3.0,
                    },
                    site: false,
                })
                .collect();
        }
        StateTransactionPayload::Snapshot {
            state: Box::new(state),
            knowledge: None,
            event_units: vec![],
        }
    }
    fn assembler() -> StateTransactionAssembler {
        StateTransactionAssembler::new(7, ContentId::parse("fixture:map").unwrap(), 768)
    }
    fn receive(
        assembler: &mut StateTransactionAssembler,
        encoded: &mut EncodedStateTransaction,
    ) -> StagedStateTransaction {
        let StateTransactionFrame::Begin(begin) = encoded.next_frame().unwrap() else {
            panic!()
        };
        assembler.begin(begin, 0).unwrap();
        let mut chunks = vec![];
        let mut end = None;
        while let Some(frame) = encoded.next_frame() {
            assert!(frame.encoded_bytes().unwrap() <= 768);
            match frame {
                StateTransactionFrame::Chunk(chunk) => chunks.push(chunk),
                StateTransactionFrame::End(value) => end = Some(value),
                _ => panic!(),
            }
        }
        assert!(assembler.end(end.as_ref().unwrap()).is_err());
        chunks.reverse();
        for chunk in chunks {
            assembler.chunk(&chunk).unwrap();
            assembler.chunk(&chunk).unwrap();
        }
        assert!(assembler.active().is_none());
        assembler.end(&end.unwrap()).unwrap().unwrap()
    }
    #[test]
    fn borrowed_snapshot_encoding_preserves_the_portable_payload_exactly() {
        let original = payload(1, true);
        let StateTransactionPayload::Snapshot {
            state, knowledge, ..
        } = &original
        else {
            panic!()
        };
        let borrowed = encode_snapshot(stamp(1, None), state, knowledge.as_ref(), 768).unwrap();
        let normal = encode_transaction(stamp(1, None), &original, 768).unwrap();
        assert_eq!(borrowed.bytes, normal.bytes);
        assert_eq!(
            encoded_snapshot_size(state, knowledge.as_ref()).unwrap(),
            normal.bytes.len()
        );
    }
    #[test]
    fn reordered_chunks_and_identical_duplicates_stage_once_and_require_activation() {
        let expected = payload(1, true);
        let mut encoded = encode_transaction(stamp(1, None), &expected, 768).unwrap();
        let mut assembler = assembler();
        let staged = receive(&mut assembler, &mut encoded);
        assert_eq!(staged.payload, expected);
        assert!(assembler.active().is_none());
        assert_eq!(assembler.retained_bytes(), 0);
        let ack = assembler.activate(&staged.stamp).unwrap();
        assert_eq!(ack.stamp, stamp(1, None));
        assert_eq!(assembler.active(), Some(&ack.stamp));
        assert!(
            assembler
                .end(&StateTransactionEnd { stamp: ack.stamp })
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn conflicting_duplicate_and_gaps_cannot_activate_and_cancel_retires_stamp() {
        let mut encoded = encode_transaction(stamp(1, None), &payload(1, true), 768).unwrap();
        let mut assembler = assembler();
        let begin = encoded.begin().clone();
        assembler.begin(begin.clone(), 0).unwrap();
        encoded.next_frame();
        let StateTransactionFrame::Chunk(mut chunk) = encoded.next_frame().unwrap() else {
            panic!()
        };
        assembler.chunk(&chunk).unwrap();
        chunk.bytes[0] ^= 1;
        assert_eq!(
            assembler.chunk(&chunk),
            Err(StateTransactionError::ConflictingDuplicate)
        );
        assert!(matches!(
            assembler.end(&StateTransactionEnd {
                stamp: stamp(1, None)
            }),
            Err(StateTransactionError::MissingChunks)
        ));
        assembler.cancel(&stamp(1, None));
        assert_eq!(assembler.retained_bytes(), 0);
        assert_eq!(
            assembler.begin(begin, 1),
            Err(StateTransactionError::InvalidStamp)
        );
        assert_eq!(
            assembler.chunk(&chunk),
            Err(StateTransactionError::InvalidStamp)
        );
    }
    #[test]
    fn cancellation_before_begin_retires_only_the_matching_context() {
        let mut assembler = assembler();
        let encoded = encode_transaction(stamp(1, None), &payload(1, false), 768).unwrap();
        let mut unrelated = stamp(99, None);
        unrelated.basis.match_id = 8;
        assembler.cancel(&unrelated);
        assembler.cancel(&stamp(1, None));
        assert_eq!(
            assembler.begin(encoded.begin().clone(), 0),
            Err(StateTransactionError::InvalidStamp)
        );
        let encoded = encode_transaction(stamp(2, None), &payload(2, false), 768).unwrap();
        assert_eq!(assembler.begin(encoded.begin().clone(), 1), Ok(()));
    }
    #[test]
    fn patch_parent_and_context_are_checked_and_timeout_releases_the_buffer() {
        let mut assembler = assembler();
        let mut base = encode_transaction(stamp(1, None), &payload(1, true), 768).unwrap();
        let staged = receive(&mut assembler, &mut base);
        assembler.activate(&staged.stamp).unwrap();
        let patch = StateTransactionPayload::Patch {
            state: Box::new(WorldDelta {
                tick: 2,
                base_tick: 1,
                ..Default::default()
            }),
            knowledge: None,
            event_units: vec![],
        };
        let encoded = encode_transaction(stamp(2, Some(1)), &patch, 768).unwrap();
        let mut wrong = encoded.begin().clone();
        wrong.stamp.parent_transaction_id = Some(90);
        assert_eq!(
            assembler.begin(wrong, 0),
            Err(StateTransactionError::InvalidStamp)
        );
        let mut wrong = encoded.begin().clone();
        wrong.stamp.basis.match_id = 8;
        assert_eq!(
            assembler.begin(wrong, 0),
            Err(StateTransactionError::InvalidStamp)
        );
        assembler.begin(encoded.begin().clone(), 10).unwrap();
        assert!(assembler.expire(5009).is_none());
        assert_eq!(assembler.expire(5010), Some(stamp(2, Some(1))));
        assert_eq!(assembler.retained_bytes(), 0);
        assert_eq!(assembler.active(), Some(&stamp(1, None)));
    }
    #[test]
    fn an_unframeable_identity_fails_closed_without_creating_staging() {
        let mut large = stamp(1, None);
        large.basis.map.local = "large_identity".repeat(100);
        assert!(matches!(
            encode_transaction(large.clone(), &payload(1, false), 768),
            Err(StateTransactionError::FrameLimit)
        ));
        let mut assembler = StateTransactionAssembler::new(7, large.basis.map.clone(), 768);
        let begin = StateTransactionBegin {
            stamp: large,
            kind: StateTransactionKind::Snapshot,
            encoded_bytes: 1,
            chunk_count: 1,
        };
        assert_eq!(
            assembler.begin(begin, 0),
            Err(StateTransactionError::FrameLimit)
        );
        assert_eq!(assembler.retained_bytes(), 0);
    }
    #[test]
    fn excessive_declared_sizes_and_counts_fail_before_staging_and_decoding_is_bounded() {
        let mut assembler = assembler();
        let encoded = encode_transaction(stamp(1, None), &payload(1, false), 768).unwrap();
        let mut begin = encoded.begin().clone();
        begin.encoded_bytes = MAX_PENDING_STATE_BYTES + 1;
        let cap = capacity(&begin.stamp, 768).unwrap();
        begin.chunk_count = u32::try_from((begin.encoded_bytes as usize).div_ceil(cap)).unwrap();
        assert_eq!(
            assembler.begin(begin, 0),
            Err(StateTransactionError::MemoryLimit)
        );
        let mut begin = encoded.begin().clone();
        begin.chunk_count = u32::MAX;
        assert_eq!(
            assembler.begin(begin, 0),
            Err(StateTransactionError::InvalidShape)
        );
        assert_eq!(assembler.retained_bytes(), 0);
        let bytes = postcard::to_allocvec(&payload(1, true)).unwrap();
        assert!(matches!(
            decode::payload(&bytes, 32),
            Err(StateTransactionError::MemoryLimit)
        ));
        assert_eq!(
            decode::payload(&bytes, MAX_PENDING_STATE_BYTES as usize).unwrap(),
            payload(1, true)
        );
    }
}
