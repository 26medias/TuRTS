//! Stable seeds keyed by authored identity, independent of registration order.
use sha2::{Digest, Sha256};
use turts_mod_api::ContentId;

fn field(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}

/// Derive one controller/goal seed with versioned, unambiguous field boundaries.
pub fn derive_seed(
    match_seed: u64,
    profile: &ContentId,
    slot: u8,
    instance: &str,
    goal: Option<&str>,
) -> u64 {
    let mut hash = Sha256::new();
    field(&mut hash, b"turts.bot.seed.v1");
    field(&mut hash, &match_seed.to_le_bytes());
    field(&mut hash, profile.to_string().as_bytes());
    field(&mut hash, &[slot]);
    field(&mut hash, instance.as_bytes());
    // None and Some("") remain different identities.
    field(&mut hash, &[u8::from(goal.is_some())]);
    if let Some(goal) = goal {
        field(&mut hash, goal.as_bytes());
    }
    let bytes: [u8; 32] = hash.finalize().into();
    let mut prefix = [0; 8];
    prefix.copy_from_slice(&bytes[..8]);
    u64::from_le_bytes(prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_version_one_seed_encoding_has_a_fixed_reference_vector() {
        // Independently encoded with Python hashlib/struct using the documented field lengths.
        let profile = ContentId::parse("test:policy").unwrap();
        assert_eq!(
            derive_seed(7, &profile, 0, "economy", None),
            18_439_708_680_288_459_266
        );
    }

    #[test]
    fn field_boundaries_and_optional_goal_are_distinct() {
        let profile = ContentId::parse("test:policy").unwrap();
        assert_ne!(
            derive_seed(7, &profile, 0, "ab", Some("c")),
            derive_seed(7, &profile, 0, "a", Some("bc"))
        );
        assert_ne!(
            derive_seed(7, &profile, 0, "a", None),
            derive_seed(7, &profile, 0, "a", Some(""))
        );
        assert_ne!(
            derive_seed(7, &profile, 0, "a", None),
            derive_seed(7, &profile, 1, "a", None)
        );
    }

    #[test]
    fn authored_identity_survives_registration_order() {
        let profile = ContentId::parse("test:policy").unwrap();
        let first = ["economy", "army"].map(|key| (key, derive_seed(9, &profile, 0, key, None)));
        let reversed = ["army", "economy"].map(|key| (key, derive_seed(9, &profile, 0, key, None)));
        assert_eq!(first[0].1, reversed[1].1);
        assert_eq!(first[1].1, reversed[0].1);
    }
}
