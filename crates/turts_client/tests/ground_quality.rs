use bevy::{asset::RenderAssetUsages, prelude::*};
use turts_client::{
    acquire::build_acquired,
    join::{AcquiredPackages, Phase},
    models::{GraphicsSettings, ModelState, Quality, sync_ground_quality},
    render::setup_scene,
};
#[test]
fn low_quality_never_uploads_an_oversized_ground_texture() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
    let bytes = turts_mod_runtime::pack::pack_dir(&root).unwrap();
    let archive = turts_mod_runtime::archive::PackageArchive::parse(&bytes).unwrap();
    let acquired = build_acquired(&[archive], String::new()).unwrap();
    let mut app = App::new();
    app.insert_resource(Phase::Joined { slot: 0 })
        .insert_resource(AcquiredPackages(acquired))
        .insert_resource(Assets::<Mesh>::default())
        .insert_resource(Assets::<StandardMaterial>::default())
        .insert_resource(Assets::<Image>::default())
        .insert_resource(GraphicsSettings {
            quality: Quality::Low,
        })
        .insert_resource(ModelState::default())
        .add_systems(Update, (setup_scene, sync_ground_quality).chain());
    app.update();
    let images = app.world().resource::<Assets<Image>>();
    let sizes: Vec<_> = images
        .iter()
        .map(|(_, i)| (i.width(), i.height(), i.asset_usage))
        .collect();
    println!("Ground image allocations: {sizes:?}");
    assert!(
        images.iter().any(|(_, image)| image.width() == 1024
            && image.asset_usage == RenderAssetUsages::MAIN_WORLD),
        "Keep the authored source on the CPU, without upscaling it"
    );
    assert!(
        images
            .iter()
            .filter(|(_, i)| i.asset_usage.contains(RenderAssetUsages::RENDER_WORLD))
            .all(|(_, i)| i.width() <= 512 && i.height() <= 512),
        "Low must keep all GPU ground images within512px: {sizes:?}"
    );
}
