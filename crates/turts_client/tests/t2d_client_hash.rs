//! Task 8a-1, T2d client half: bytes that do not match their hash are refused before any parse.
//! Test code may name the example packages.

use std::path::PathBuf;

use turts_client::acquire::{accept_package, check_lockfile};
use turts_mod_api::{ExecutionContext, Lockfile};
use turts_mod_runtime::archive::PackageArchive;
use turts_mod_runtime::hash::sha256_hex;
use turts_mod_runtime::lock::{read_lockfile, write_lockfile};
use turts_mod_runtime::pack::pack_dir;

fn acme_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods/acme"))
}

#[test]
fn t2d_package_one_byte_off_is_refused_before_parse() {
    let archive = pack_dir(&acme_dir()).unwrap();
    let entry = PackageArchive::parse(&archive)
        .unwrap()
        .lock_entry(vec!["http://h/acme.tar".to_string()]);

    // Byte 520 is inside the content of manifest.ron: the ustar header is bytes 0..512 and
    // `pack_orders_manifest_first` holds. The flip breaks the manifest text.
    let mut flipped = archive.clone();
    flipped[520] ^= 1;
    assert_ne!(flipped, archive);
    assert!(
        PackageArchive::parse(&flipped).is_err(),
        "the flip must break the parse, so a skipped hash check shows as a parse error"
    );

    let error = accept_package(&flipped, &entry).unwrap_err();
    assert!(error.starts_with("acme/-: hash: "), "{error}");
    assert!(error.contains(&entry.hash), "{error}");
    assert!(error.contains(&sha256_hex(&flipped)), "{error}");

    let accepted = accept_package(&archive, &entry).expect("the unchanged archive is accepted");
    assert_eq!(accepted.manifest.id, "acme");
    assert_eq!(accepted.hash, entry.hash);
}

#[test]
fn t2d_package_with_matching_hash_but_other_lock_fields_is_refused() {
    // The hash passes; the lock entry still has to describe the archive (context here).
    let archive = pack_dir(&acme_dir()).unwrap();
    let mut entry = PackageArchive::parse(&archive)
        .unwrap()
        .lock_entry(Vec::new());
    entry.context = ExecutionContext::Server;
    let error = accept_package(&archive, &entry).unwrap_err();
    assert!(error.contains("lock.context"), "{error}");
}

#[test]
fn t2d_lockfile_bytes_off_are_refused() {
    let archive = pack_dir(&acme_dir()).unwrap();
    let entry = PackageArchive::parse(&archive)
        .unwrap()
        .lock_entry(vec!["http://h/acme.tar".to_string()]);
    let lock = Lockfile {
        packages: vec![entry],
    };
    let bytes = write_lockfile(&lock);
    let hash = sha256_hex(&bytes);
    assert_eq!(check_lockfile(&bytes, &hash).unwrap(), lock);

    // Change one letter of a URL. The JSON stays valid, so only the hash check can refuse it.
    let at = bytes
        .windows(8)
        .position(|window| window == b"acme.tar")
        .expect("the url is in the lockfile");
    let mut flipped = bytes.clone();
    flipped[at] ^= 1;
    assert!(
        read_lockfile(&flipped).is_ok(),
        "the flip must leave valid JSON"
    );

    let error = check_lockfile(&flipped, &hash).unwrap_err();
    assert!(error.contains("hash"), "{error}");
    assert!(
        error.contains(&hash) && error.contains(&sha256_hex(&flipped)),
        "{error}"
    );
}
