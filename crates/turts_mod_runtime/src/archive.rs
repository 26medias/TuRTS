//! Package archive read.

use std::collections::BTreeMap;
use std::io::Read;

use serde::de::DeserializeOwned;
use turts_mod_api::{LockEntry, Manifest, MapDef, ModError, PackageId, UnitDef, VisualDef};

use crate::hash::sha256_hex;
use crate::manifest::validate_manifest;
use crate::parse::parse_ron;

/// Archive path of the manifest.
pub const MANIFEST_PATH: &str = "manifest.ron";

/// A package archive, read in memory. Nothing is written to disk.
#[derive(Debug, Clone, PartialEq)]
pub struct PackageArchive {
    /// Hash of the exact archive bytes.
    pub hash: String,
    /// Length of the archive in bytes.
    pub size: u64,
    pub manifest: Manifest,
    files: BTreeMap<String, Vec<u8>>,
}

impl PackageArchive {
    /// Read an archive and check its manifest. An unsafe path (absolute, `..`, empty or `.`
    /// part), a repeated path, an entry that is not a regular file, or a missing
    /// `manifest.ron` is an error with the field path `archive`.
    pub fn parse(bytes: &[u8]) -> Result<Self, ModError> {
        let files = read_entries(bytes)?;
        let manifest_bytes = files
            .get(MANIFEST_PATH)
            .ok_or_else(|| archive_error(format!("the archive has no {MANIFEST_PATH}")))?;
        let text = utf8(manifest_bytes, "", MANIFEST_PATH)?;
        let manifest: Manifest = parse_ron("", MANIFEST_PATH, text)?;
        validate_manifest(&manifest)?;
        Ok(Self {
            hash: sha256_hex(bytes),
            size: bytes.len() as u64,
            manifest,
            files,
        })
    }

    pub fn id(&self) -> PackageId {
        PackageId(self.manifest.id.clone())
    }

    /// The bytes of one file of the archive.
    pub fn file(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(Vec::as_slice)
    }

    /// The exported unit definitions as `(archive path, definition)`.
    pub fn units(&self) -> Result<Vec<(String, UnitDef)>, ModError> {
        self.parse_exports("exports.units", &self.manifest.exports.units)
    }

    /// The exported map definitions as `(archive path, definition)`.
    pub fn maps(&self) -> Result<Vec<(String, MapDef)>, ModError> {
        self.parse_exports("exports.maps", &self.manifest.exports.maps)
    }

    /// The exported visual definitions as `(archive path, definition)`. Only the client
    /// calls this.
    pub fn visuals(&self) -> Result<Vec<(String, VisualDef)>, ModError> {
        self.parse_exports("exports.visuals", &self.manifest.exports.visuals)
    }

    /// The lock entry that describes this archive. The caller gives the download URLs.
    pub fn lock_entry(&self, urls: Vec<String>) -> LockEntry {
        LockEntry {
            id: self.manifest.id.clone(),
            version: self.manifest.version.clone(),
            hash: self.hash.clone(),
            size: self.size,
            urls,
            context: self.manifest.context,
            engine_api: self.manifest.engine_api.clone(),
        }
    }

    /// Check a lock entry against this archive. The first mismatch is the error, with the
    /// field path `lock.<field>`. The archive cannot say where it was downloaded from, so
    /// `urls` is the one lock field that this check does not compare.
    pub fn check_against_lock(&self, entry: &LockEntry) -> Result<(), ModError> {
        let own = self.lock_entry(Vec::new());
        let fields = [
            ("id", own.id, entry.id.clone()),
            ("version", own.version, entry.version.clone()),
            ("hash", own.hash, entry.hash.clone()),
            ("size", own.size.to_string(), entry.size.to_string()),
            (
                "context",
                format!("{:?}", own.context),
                format!("{:?}", entry.context),
            ),
            ("engine_api", own.engine_api, entry.engine_api.clone()),
        ];
        for (field, archive, lock) in fields {
            if archive != lock {
                return Err(ModError::new(
                    &self.manifest.id,
                    &format!("lock.{field}"),
                    format!("the archive has {field} `{archive}` but the lock entry has `{lock}`"),
                ));
            }
        }
        Ok(())
    }

    fn parse_exports<T: DeserializeOwned>(
        &self,
        field: &str,
        paths: &[String],
    ) -> Result<Vec<(String, T)>, ModError> {
        let package = self.manifest.id.as_str();
        let mut out = Vec::with_capacity(paths.len());
        for path in paths {
            let bytes = self.file(path).ok_or_else(|| {
                ModError::new(
                    package,
                    field,
                    format!("exported file `{path}` is not in the archive"),
                )
                .with_file(MANIFEST_PATH)
            })?;
            let text = utf8(bytes, package, path)?;
            out.push((path.clone(), parse_ron(package, path, text)?));
        }
        Ok(out)
    }
}

/// Whether an archive path is relative, has no empty, `.` or `..` part, and has no backslash.
pub(crate) fn is_safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains(['\\', '\0'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn archive_error(message: String) -> ModError {
    ModError::new("", "archive", message)
}

fn utf8<'a>(bytes: &'a [u8], package: &str, file: &str) -> Result<&'a str, ModError> {
    std::str::from_utf8(bytes)
        .map_err(|_| ModError::new(package, file, "the file is not valid UTF-8").with_file(file))
}

/// Read every regular file of the tar into memory. Directory entries are skipped.
fn read_entries(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>, ModError> {
    // The tar error text can echo bytes of the archive, so the message does not carry it.
    let unreadable =
        |_: std::io::Error| archive_error("the bytes are not a valid tar archive".to_string());
    let mut files = BTreeMap::new();
    let mut archive = tar::Archive::new(bytes);
    for entry in archive.entries().map_err(unreadable)? {
        let mut entry = entry.map_err(unreadable)?;
        let raw = entry.path_bytes().into_owned();
        let name = String::from_utf8_lossy(&raw).into_owned();
        let kind = entry.header().entry_type();
        let path = match std::str::from_utf8(&raw) {
            Ok(text) if kind.is_dir() => text.strip_suffix('/').unwrap_or(text),
            Ok(text) => text,
            Err(_) => "",
        };
        if !is_safe_path(path) {
            return Err(archive_error(format!("unsafe path `{name}`")));
        }
        if kind.is_dir() {
            continue;
        }
        if !kind.is_file() {
            return Err(archive_error(format!(
                "entry `{name}` is not a regular file"
            )));
        }
        let mut data = Vec::new();
        entry.read_to_end(&mut data).map_err(unreadable)?;
        if files.insert(path.to_string(), data).is_some() {
            return Err(archive_error(format!("duplicate path `{name}`")));
        }
    }
    Ok(files)
}
