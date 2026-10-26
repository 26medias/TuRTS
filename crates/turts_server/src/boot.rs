//! Boot: load the package, build the world.
//!
//! The server reads a package only through `turts_mod_runtime` and registers it only through
//! `UnitRegistry` and `load_map`. The server never reads the visuals of a package.

use bevy::ecs::world::World;
use turts_mod_api::{ENGINE_API_VERSION, Lockfile, ModError, UnitRegistry};
use turts_mod_runtime::archive::{MANIFEST_PATH, PackageArchive};
use turts_mod_runtime::hash::sha256_hex;
use turts_mod_runtime::lock::write_lockfile;
use turts_net_protocol::{Advertisement, NET_PROTOCOL_VERSION};
use turts_sim::{UnitTypes, load_map};

use crate::config::HostConfig;

/// Everything that the HTTP endpoints serve, built once at boot.
#[derive(Debug)]
pub struct Booted {
    pub archive: PackageArchive,
    /// The exact bytes of the archive. The package endpoint serves these bytes, so the hash
    /// in the lockfile matches what a client downloads.
    pub package_bytes: Vec<u8>,
    /// The advertisement as JSON.
    pub advertisement: Vec<u8>,
    /// The lockfile as JSON.
    pub lockfile: Vec<u8>,
    /// Hash of the lockfile bytes. Clients send it in `Hello`.
    pub lockfile_hash: String,
    /// The last part of the package URL.
    pub package_file_name: String,
}

/// Parse the archive, register its unit types, load its map, and build the lockfile and the
/// advertisement. An error in a file of the package carries that file in `ModError::file`.
pub fn boot(world: &mut World, cfg: &HostConfig, archive_bytes: &[u8]) -> Result<Booted, ModError> {
    let archive = PackageArchive::parse(archive_bytes)?;
    let id = archive.id();
    for (file, def) in archive.units()? {
        world
            .resource_mut::<UnitTypes>()
            .register_unit_type(&id, def)
            .map_err(|error| error.with_file(&file))?;
    }
    let maps = archive.maps()?;
    let [(file, map)] = maps.as_slice() else {
        return Err(ModError::new(
            &id.0,
            "exports.maps",
            format!(
                "a package exports exactly one map at M1, found {}",
                maps.len()
            ),
        )
        .with_file(MANIFEST_PATH));
    };
    load_map(world, &id, map).map_err(|error| error.with_file(file))?;

    let public_url = cfg.public_url.trim_end_matches('/');
    // M1 placeholder; format open (spec 15): the archive is a ustar file named <id>-<version>.tar.
    let package_file_name = format!("{}-{}.tar", archive.manifest.id, archive.manifest.version);
    let entry = archive.lock_entry(vec![format!("{public_url}/packages/{package_file_name}")]);
    let lockfile = write_lockfile(&Lockfile {
        packages: vec![entry],
    });
    let lockfile_hash = sha256_hex(&lockfile);
    let advertisement = Advertisement {
        mod_id: archive.manifest.id.clone(),
        mod_version: archive.manifest.version.clone(),
        engine_version: ENGINE_API_VERSION.to_string(),
        protocol_version: NET_PROTOCOL_VERSION,
        lockfile_url: format!("{public_url}/lockfile.json"),
        lockfile_hash: lockfile_hash.clone(),
    };
    Ok(Booted {
        archive,
        package_bytes: archive_bytes.to_vec(),
        advertisement: serde_json::to_vec(&advertisement)
            .expect("an advertisement serializes to JSON"),
        lockfile,
        lockfile_hash,
        package_file_name,
    })
}
