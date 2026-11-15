//! Bulk-copy chunk bytes without changing postcard's length-plus-u8 wire format.
use serde::{Deserializer, Serializer, de};
use std::fmt;

pub fn serialize<S: Serializer>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_bytes(bytes)
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
    struct ChunkBytes;
    impl<'de> de::Visitor<'de> for ChunkBytes {
        type Value = Vec<u8>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("at most 1024 state chunk bytes")
        }

        fn visit_bytes<E: de::Error>(self, bytes: &[u8]) -> Result<Self::Value, E> {
            if bytes.len() > super::MAX_STATE_CHUNK_BYTES {
                return Err(E::custom("state chunk exceeds 1024 bytes"));
            }
            Ok(bytes.to_vec())
        }

        fn visit_byte_buf<E: de::Error>(self, bytes: Vec<u8>) -> Result<Self::Value, E> {
            if bytes.len() > super::MAX_STATE_CHUNK_BYTES {
                return Err(E::custom("state chunk exceeds 1024 bytes"));
            }
            Ok(bytes)
        }

        fn visit_seq<A: de::SeqAccess<'de>>(
            self,
            mut sequence: A,
        ) -> Result<Self::Value, A::Error> {
            let length = sequence.size_hint().unwrap_or(0);
            if length > super::MAX_STATE_CHUNK_BYTES {
                return Err(de::Error::custom("state chunk exceeds 1024 bytes"));
            }
            let mut bytes = Vec::new();
            bytes.try_reserve_exact(length).map_err(de::Error::custom)?;
            while let Some(byte) = sequence.next_element::<u8>()? {
                if bytes.len() == super::MAX_STATE_CHUNK_BYTES {
                    return Err(de::Error::custom("state chunk exceeds 1024 bytes"));
                }
                bytes.push(byte);
            }
            Ok(bytes)
        }
    }
    deserializer.deserialize_byte_buf(ChunkBytes)
}

#[cfg(test)]
mod tests {
    use crate::{StateTransactionChunk, StateTransactionStamp};
    use serde::Serialize;
    use turts_mod_api::{ContentId, KnowledgeBasis, ObservationPhase};

    #[derive(Serialize)]
    struct LegacyChunk {
        stamp: StateTransactionStamp,
        index: u32,
        bytes: Vec<u8>,
    }

    fn chunk(length: usize) -> StateTransactionChunk {
        StateTransactionChunk {
            stamp: StateTransactionStamp {
                basis: KnowledgeBasis {
                    match_id: 19,
                    map: ContentId::parse("fixture:map").unwrap(),
                    tick: 42,
                    phase: ObservationPhase::Completed,
                },
                transaction_id: 17,
                parent_transaction_id: Some(16),
            },
            index: 127,
            bytes: (0..length).map(|index| index as u8).collect(),
        }
    }

    #[test]
    fn bulk_bytes_preserve_legacy_postcard_at_every_chunk_length() {
        for length in 0..=super::super::MAX_STATE_CHUNK_BYTES {
            let value = chunk(length);
            let legacy = LegacyChunk {
                stamp: value.stamp.clone(),
                index: value.index,
                bytes: value.bytes.clone(),
            };
            let old_bytes = postcard::to_allocvec(&legacy).unwrap();
            assert_eq!(postcard::to_allocvec(&value).unwrap(), old_bytes);
            assert_eq!(
                postcard::from_bytes::<StateTransactionChunk>(&old_bytes).unwrap(),
                value
            );
        }
    }

    #[test]
    fn json_arrays_still_round_trip_and_oversized_chunks_refuse() {
        let value = chunk(128);
        assert_eq!(
            serde_json::from_str::<StateTransactionChunk>(&serde_json::to_string(&value).unwrap())
                .unwrap(),
            value
        );
        let oversized = chunk(super::super::MAX_STATE_CHUNK_BYTES + 1);
        let legacy = LegacyChunk {
            stamp: oversized.stamp,
            index: oversized.index,
            bytes: oversized.bytes,
        };
        assert!(
            postcard::from_bytes::<StateTransactionChunk>(&postcard::to_allocvec(&legacy).unwrap())
                .is_err()
        );
        assert!(
            serde_json::from_str::<StateTransactionChunk>(&serde_json::to_string(&legacy).unwrap())
                .is_err()
        );
    }
}
