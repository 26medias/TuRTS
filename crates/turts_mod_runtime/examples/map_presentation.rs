//! Export the selected map's presentation as a RON field fragment for render workloads.
use turts_mod_runtime::{archive::PackageArchive, pack::pack_dir};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let package = std::env::args_os()
        .nth(1)
        .ok_or("usage: map_presentation PACKAGE_DIRECTORY")?;
    let archive = PackageArchive::parse(&pack_dir(std::path::Path::new(&package))?)?;
    archive.content()?;
    let maps = archive.maps()?;
    let (_, map) = maps.first().ok_or("the package has no map")?;
    if map.ground.is_some() {
        println!("ground: {},", ron::ser::to_string(&map.ground)?);
    }
    if map.environment.is_some() {
        println!("environment: {},", ron::ser::to_string(&map.environment)?);
    }
    Ok(())
}
