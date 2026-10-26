//! Package tooling: the load path of a package, shared by the server and `turts-pack`, and the
//! checks that `turts-pack` runs before it writes an archive.
//!
//! The server rules (speed, radius, owner slot, position, one map) live in `turts_sim`, which
//! `turts_mod_runtime` and the client must not depend on. This crate depends on both, so the
//! packer can run the same code that the server runs. A package that `turts-pack` accepts is
//! a package that the server boots and the client accepts.

use std::path::Path;

use bevy_ecs::world::World;
use turts_mod_api::{ModError, UnitRegistry};
use turts_mod_runtime::archive::{MANIFEST_PATH, PackageArchive};
use turts_mod_runtime::pack::pack_dir;
use turts_mod_runtime::visuals::visuals_by_unit;
use turts_sim::{DEFAULT_TICK_HZ, UnitTypes, headless_app, load_map};

/// The server path of a package: register its unit types, then load its map into `world`.
/// `world` needs the `SimPlugin` resources. An error carries the file it comes from.
///
/// The server calls this at boot. There is no other path from a package to the world.
pub fn load_package(world: &mut World, archive: &PackageArchive) -> Result<(), ModError> {
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
    load_map(world, &id, map).map_err(|error| error.with_file(file))
}

/// Check a parsed package on both paths. The server path loads it into a throwaway headless
/// world with [`load_package`]. The client path checks the visuals
/// (`turts_mod_runtime::visuals::visuals_by_unit`). The first problem is the error.
pub fn check_package(archive: &PackageArchive) -> Result<(), ModError> {
    let mut app = headless_app(DEFAULT_TICK_HZ);
    load_package(app.world_mut(), archive)?;
    visuals_by_unit(std::slice::from_ref(archive))?;
    Ok(())
}

/// Pack the directory `dir` and check the archive as a server and a client read it. The
/// bytes are returned only when the package passes every check, so a caller never writes an
/// archive for a bad package.
pub fn pack_checked(dir: &Path) -> Result<Vec<u8>, ModError> {
    let bytes = pack_dir(dir)?;
    check_package(&PackageArchive::parse(&bytes)?)?;
    Ok(bytes)
}
