//! Print model metadata for package authors.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    for path in std::env::args().skip(1) {
        let bytes = std::fs::read(&path)?;
        let info = turts_mod_runtime::models::inspect_glb("inspect", &path, &bytes)?;
        println!(
            "{path}: bind {:?} {:?}; clips {:?}",
            info.min, info.max, info.clip_bounds
        );
    }
    Ok(())
}
