//! Reproducible package packing.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use turts_mod_api::ModError;

use crate::archive::{MANIFEST_PATH, is_safe_path};

/// Pack a package directory into a reproducible archive.
///
/// The format is ustar. `manifest.ron` is the first entry, so its content starts at byte 512.
/// The other entries follow in path order. Every entry has mode 0o644, owner 0, modification
/// time 0, and is a regular file. Two calls on the same tree give identical bytes.
// M1 placeholder; format open (spec 15)
pub fn pack_dir(dir: &Path) -> Result<Vec<u8>, ModError> {
    let mut files = BTreeMap::new();
    collect(dir, dir, &mut files)?;
    pack_files(&files)
}

/// Pack files given as `relative path -> bytes`, with the same layout as `pack_dir`.
pub fn pack_files(files: &BTreeMap<String, Vec<u8>>) -> Result<Vec<u8>, ModError> {
    let mut builder = tar::Builder::new(Vec::new());
    let manifest = files.get_key_value(MANIFEST_PATH);
    let rest = files
        .iter()
        .filter(|(path, _)| path.as_str() != MANIFEST_PATH);
    for (path, data) in manifest.into_iter().chain(rest) {
        if !is_safe_path(path) {
            return Err(pack_error(format!("unsafe path `{path}`")));
        }
        let mut header = tar::Header::new_ustar();
        header
            .set_path(path)
            .map_err(|error| pack_error(format!("cannot store `{path}`: {error}")))?;
        header.set_size(data.len() as u64);
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_entry_type(tar::EntryType::Regular);
        header.set_cksum();
        builder
            .append(&header, data.as_slice())
            .map_err(|error| pack_error(format!("cannot store `{path}`: {error}")))?;
    }
    builder
        .into_inner()
        .map_err(|error| pack_error(format!("cannot finish the archive: {error}")))
}

fn pack_error(message: String) -> ModError {
    ModError::new("", "pack", message)
}

/// Read every regular file under `dir` into `files`, keyed by the `/` separated path
/// relative to `root`.
fn collect(root: &Path, dir: &Path, files: &mut BTreeMap<String, Vec<u8>>) -> Result<(), ModError> {
    let unreadable = |path: &Path, error: std::io::Error| {
        pack_error(format!("cannot read `{}`: {error}", path.display()))
    };
    for entry in fs::read_dir(dir).map_err(|error| unreadable(dir, error))? {
        let entry = entry.map_err(|error| unreadable(dir, error))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|error| unreadable(&path, error))?;
        if kind.is_dir() {
            collect(root, &path, files)?;
        } else if kind.is_file() {
            let relative = path.strip_prefix(root).expect("an entry is under its root");
            let parts: Option<Vec<&str>> = relative.iter().map(|part| part.to_str()).collect();
            let name = parts
                .ok_or_else(|| pack_error(format!("`{}` is not a UTF-8 path", path.display())))?
                .join("/");
            let data = fs::read(&path).map_err(|error| unreadable(&path, error))?;
            files.insert(name, data);
        } else {
            return Err(pack_error(format!(
                "`{}` is not a regular file or a directory",
                path.display()
            )));
        }
    }
    Ok(())
}
