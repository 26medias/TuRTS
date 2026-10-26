//! Reproducible packing. The `turts-pack` binary is tested in the `turts_pack` crate.

mod common;

use std::fs;

use common::{copy_dir, fresh_dir, package_dir, read_files};
use turts_mod_runtime::archive::PackageArchive;
use turts_mod_runtime::hash::sha256_hex;
use turts_mod_runtime::pack::{pack_dir, pack_files};

#[test]
fn pack_is_reproducible() {
    let dir = package_dir("base");
    let first = pack_dir(&dir).unwrap();
    let second = pack_dir(&dir).unwrap();
    assert_eq!(first, second);
    assert_eq!(sha256_hex(&first), sha256_hex(&second));
}

#[test]
fn pack_ignores_file_times() {
    // Same content, different modification time: same bytes.
    let from = package_dir("acme");
    let dir = fresh_dir("pack_ignores_file_times").join("acme");
    copy_dir(&from, &dir);
    let before = pack_dir(&dir).unwrap();
    let manifest = fs::File::options()
        .write(true)
        .open(dir.join("manifest.ron"))
        .unwrap();
    manifest
        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000))
        .unwrap();
    assert_eq!(pack_dir(&dir).unwrap(), before);
}

#[test]
fn pack_orders_manifest_first() {
    // (G2) T2d flips a byte at offset 520, inside the content of the first entry.
    let bytes = pack_dir(&package_dir("base")).unwrap();
    assert_eq!(&bytes[0..12], b"manifest.ron");
    let manifest = fs::read(package_dir("base").join("manifest.ron")).unwrap();
    assert_eq!(&bytes[512..512 + manifest.len()], manifest.as_slice());
}

#[test]
fn pack_puts_manifest_first_even_when_another_path_sorts_before_it() {
    let mut files = read_files(&package_dir("acme"));
    files.insert("a/early.ron".to_string(), b"()".to_vec());
    let bytes = pack_files(&files).unwrap();
    assert_eq!(&bytes[0..12], b"manifest.ron");
}

#[test]
fn pack_then_parse_round_trips() {
    let bytes = pack_dir(&package_dir("acme")).unwrap();
    let archive = PackageArchive::parse(&bytes).unwrap();
    assert_eq!(archive.manifest.id, "acme");
    assert_eq!(archive.units().unwrap()[0].1.speed, 6.0);
    assert_eq!(archive.hash, sha256_hex(&bytes));
    assert_eq!(archive.size, bytes.len() as u64);
}

#[test]
fn pack_dir_reports_a_missing_directory() {
    let dir = fresh_dir("pack_missing").join("nope");
    let error = pack_dir(&dir).unwrap_err();
    assert_eq!(error.field_path, "pack");
}
