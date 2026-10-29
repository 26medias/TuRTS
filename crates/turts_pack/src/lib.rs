//! Package tooling: the only public path from a package archive into a simulation world.

use std::path::Path;

use bevy_ecs::world::World;
use turts_mod_api::{ModError, PackageKind, SourceDef};
use turts_mod_runtime::archive::{MANIFEST_PATH, PackageArchive};
use turts_mod_runtime::pack::pack_dir;
use turts_mod_runtime::visuals::visuals_by_content;
use turts_sim::{DEFAULT_TICK_HZ, SimClock, headless_app, install_package, stage_package};

/// Validate the complete portable catalog and map, then install them atomically.
pub fn load_package(world: &mut World, archive: &PackageArchive) -> Result<(), ModError> {
    turts_mod_runtime::resolve::resolve(&archive.manifest, &[])?;
    let catalog = archive.content()?;
    let maps = archive.maps()?;
    let [(file, map)] = maps.as_slice() else {
        return Err(ModError::new(
            &archive.manifest.id,
            "exports.maps",
            format!("a package exports exactly one map, found {}", maps.len()),
        )
        .with_file(MANIFEST_PATH));
    };
    let tick_hz = world
        .get_resource::<SimClock>()
        .map_or(DEFAULT_TICK_HZ, |clock| clock.tick_hz);
    let staged = stage_package(
        world,
        catalog,
        SourceDef {
            file: file.clone(),
            value: map.clone(),
        },
        tick_hz,
    )?;
    install_package(world, staged);
    Ok(())
}

/// Check the package on the server path and client visual path.
pub fn check_package(archive: &PackageArchive) -> Result<(), ModError> {
    match archive.manifest.kind {
        PackageKind::Game => {
            let mut app = headless_app(DEFAULT_TICK_HZ);
            load_package(app.world_mut(), archive)?;
            visuals_by_content(std::slice::from_ref(archive))?;
            Ok(())
        }
        PackageKind::Library => {
            archive.content()?;
            visuals_by_content(std::slice::from_ref(archive))?;
            Ok(())
        }
    }
}

/// Pack and validate a package before returning archive bytes.
pub fn pack_checked(dir: &Path) -> Result<Vec<u8>, ModError> {
    let bytes = pack_dir(dir)?;
    check_package(&PackageArchive::parse(&bytes)?)?;
    Ok(bytes)
}
