//! Helpers shared by the integration tests. Test code may name the example packages.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use turts_mod_runtime::pack::pack_files;

/// The directory of one package under the repository's `mods` directory.
pub fn package_dir(name: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../mods")).join(name)
}

/// Every file under `dir`, as `relative path -> bytes`.
pub fn read_files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).expect("read dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let relative = path.strip_prefix(root).expect("under root");
                let key = relative.to_str().expect("utf-8 path").replace('\\', "/");
                out.insert(key, fs::read(&path).expect("read file"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    as_edition_0_2_3(&mut out);
    out
}

/// The shipped packages place the sound switch of API 0.2.4, row H25, so they need `^0.2.4`. The
/// tests of the older editions use a shipped package as the 0.2.3 baseline and edit its manifest
/// and its files by text. This gives them that baseline: the two switch widgets are removed and the
/// requirement is `^0.2.3` again. A package that has no switch is not changed. The tests of the
/// shipped form read the files by hand (`the_shipped_packages_place_the_sound_switch...` in
/// `val1.rs`).
pub fn as_edition_0_2_3(files: &mut BTreeMap<String, Vec<u8>>) {
    let Some(hud) = files.get("interface/hud.ron") else {
        return;
    };
    let text = String::from_utf8(hud.clone()).expect("utf-8 hud");
    let Some(start) = text.find("        (id: \"sound_switch_off\"") else {
        return;
    };
    let on = text[start..]
        .find("        (id: \"sound_switch_on\"")
        .expect("both switch widgets")
        + start;
    let end = text[on..]
        .find("        ])),\n")
        .expect("the end of the switch")
        + on
        + "        ])),\n".len();
    let stripped = format!("{}{}", &text[..start], &text[end..]);
    files.insert("interface/hud.ron".into(), stripped.into_bytes());
    let manifest = String::from_utf8(files["manifest.ron"].clone()).expect("utf-8 manifest");
    files.insert(
        "manifest.ron".into(),
        manifest
            .replace("engine_api: \"^0.2.4\"", "engine_api: \"^0.2.3\"")
            .into_bytes(),
    );
}

/// The `acme` package with one text edit. Panics when the edit changes nothing, so a
/// case can never pass because its edit missed.
pub fn acme_with(path: &str, from: &str, to: &str) -> BTreeMap<String, Vec<u8>> {
    let mut files = read_files(&package_dir("acme"));
    let text = String::from_utf8(files[path].clone()).expect("utf-8 file");
    assert!(text.contains(from), "edit target `{from}` is not in {path}");
    files.insert(path.to_string(), text.replacen(from, to, 1).into_bytes());
    files
}

/// The `acme` package with one text edit, packed in memory.
pub fn acme_archive_with(path: &str, from: &str, to: &str) -> Vec<u8> {
    pack_files(&acme_with(path, from, to)).expect("pack in memory")
}

/// The unchanged `acme` package, packed in memory.
pub fn acme_archive() -> Vec<u8> {
    pack_files(&read_files(&package_dir("acme"))).expect("pack in memory")
}

/// Build a ustar archive entry by entry. The name goes into the header unchecked, so a
/// test can write names that `tar::Header::set_path` refuses.
pub fn raw_tar(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    for (name, data) in entries {
        let mut header = tar::Header::new_ustar();
        let ustar = header.as_ustar_mut().expect("ustar header");
        ustar.name[..name.len()].copy_from_slice(name.as_bytes());
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        builder.append(&header, *data).expect("append entry");
    }
    builder.into_inner().expect("finish archive")
}

/// Copy a directory tree.
pub fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create dir");
    for entry in fs::read_dir(from).expect("read dir") {
        let entry = entry.expect("dir entry");
        let target = to.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy file");
        }
    }
}

/// A new empty directory under the test target directory.
pub fn fresh_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create dir");
    dir
}
