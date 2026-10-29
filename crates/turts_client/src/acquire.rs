//! Package acquisition: the checks between the bytes on the wire and the data the client uses.
//!
//! These are pure functions. The phase machine of the client calls them in this order:
//! `check_advertisement`, `check_lockfile`, `packages_to_fetch`, `accept_package` for each
//! package, `build_acquired` (design 4.4 and 5).

use std::collections::HashMap;
use std::collections::HashSet;

use crate::content::ClientContent;
use turts_mod_api::{ContentId, ENGINE_API_VERSION, ExecutionContext, LockEntry, Lockfile, Shape};
use turts_mod_runtime::archive::PackageArchive;
use turts_mod_runtime::hash::verify_hash;
use turts_mod_runtime::lock::read_lockfile;
use turts_net_protocol::{Advertisement, NET_PROTOCOL_VERSION};

/// What the client keeps after it has accepted the server's packages.
#[derive(Clone, Debug, PartialEq)]
pub struct Acquired {
    /// The hash that `Hello` carries.
    pub lockfile_hash: String,
    /// The visual shape of each unit, by full content id.
    pub visuals: HashMap<String, Shape>,
    /// The size of the map on the ground plane.
    pub map_size: (f32, f32),
    /// The display name of each unit, by full content id.
    pub unit_names: HashMap<String, String>,
    pub content: ClientContent,
}

/// Read the advertisement and check that the client can talk to this server: the same wire
/// protocol version and the same engine API version.
pub fn check_advertisement(bytes: &[u8]) -> Result<Advertisement, String> {
    let ad: Advertisement =
        serde_json::from_slice(bytes).map_err(|error| format!("advertisement: {error}"))?;
    if ad.protocol_version != NET_PROTOCOL_VERSION {
        return Err(format!(
            "advertisement: the server speaks protocol version {} but this client speaks {NET_PROTOCOL_VERSION}",
            ad.protocol_version
        ));
    }
    if ad.engine_version != ENGINE_API_VERSION {
        return Err(format!(
            "advertisement: the server runs engine version {} but this client runs {ENGINE_API_VERSION}",
            ad.engine_version
        ));
    }
    Ok(ad)
}

/// Read the lockfile. The bytes are checked against the advertised hash first, and are not
/// parsed when the hash differs (design 5).
pub fn check_lockfile(
    bytes: &[u8],
    advertised_hash: &str,
    advertised_mod_id: &str,
) -> Result<Lockfile, String> {
    verify_hash(bytes, advertised_hash)
        .map_err(|error| format!("lockfile hash mismatch: {}", error.message))?;
    let lockfile = read_lockfile(bytes).map_err(|error| error.to_string())?;
    let mut seen = HashSet::new();
    for entry in &lockfile.packages {
        if !seen.insert(entry.id.as_str()) {
            return Err(format!("lockfile: package `{}` has two entries", entry.id));
        }
    }
    if !seen.contains(advertised_mod_id) {
        return Err(format!(
            "lockfile: the advertised package `{advertised_mod_id}` has no entry"
        ));
    }
    Ok(lockfile)
}

/// The packages the client downloads: context `Client` or `Shared`. A `Server` package is never
/// requested (spec 6.5).
pub fn packages_to_fetch(lock: &Lockfile) -> Vec<&LockEntry> {
    lock.packages
        .iter()
        .filter(|entry| entry.context != ExecutionContext::Server)
        .collect()
}

/// Accept downloaded package bytes. The bytes are checked against the lock entry hash before
/// they are parsed. Then the parsed package must match the rest of the entry.
pub fn accept_package(bytes: &[u8], entry: &LockEntry) -> Result<PackageArchive, String> {
    verify_hash(bytes, &entry.hash).map_err(|mut error| {
        error.package = entry.id.clone();
        error.to_string()
    })?;
    let archive = PackageArchive::parse(bytes).map_err(|error| error.to_string())?;
    archive
        .check_against_lock(entry)
        .map_err(|error| error.to_string())?;
    Ok(archive)
}

/// Build what the client keeps from the accepted packages. Every unit has exactly one visual,
/// and every visual names a defined unit, else this is an error that names the file and the
/// field (design 4.4; `turts_mod_runtime::visuals::visuals_by_content`). The map is the first map
/// of the first package that has one.
pub fn build_acquired(
    archives: &[PackageArchive],
    lockfile_hash: String,
) -> Result<Acquired, String> {
    let content = ClientContent::from_archives(archives).map_err(|error| error.to_string())?;
    let visuals = content.shapes.clone();
    let mut unit_names = HashMap::new();
    let mut map_size = None;
    for archive in archives {
        let package = archive.id();
        for (_, unit) in archive.units().map_err(|error| error.to_string())? {
            unit_names.insert(ContentId::new(&package, &unit.id).to_string(), unit.name);
        }
        if map_size.is_none() {
            let maps = archive.maps().map_err(|error| error.to_string())?;
            map_size = maps.first().map(|(_, map)| (map.size.x, map.size.y));
        }
    }
    let map_size = map_size.ok_or_else(|| "the packages have no map".to_string())?;
    Ok(Acquired {
        lockfile_hash,
        visuals,
        map_size,
        unit_names,
        content,
    })
}
