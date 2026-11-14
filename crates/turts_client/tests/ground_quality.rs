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
        .init_resource::<Assets<turts_client::ground::GroundMaterial>>()
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

#[test]
fn textured_overlay_keeps_patches_and_restores_authored_pixels_across_quality_changes() {
    use turts_client::ground::GroundMaterial;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
    let archive = turts_mod_runtime::archive::PackageArchive::parse(
        &turts_mod_runtime::pack::pack_dir(&root).unwrap(),
    )
    .unwrap();
    let mut acquired = build_acquired(&[archive], String::new()).unwrap();
    acquired
        .presentation
        .ground
        .as_mut()
        .unwrap()
        .overlay
        .as_mut()
        .unwrap()
        .base_colour = Some("skirmish:ground_grass".into());
    let mut app = App::new();
    app.insert_resource(Phase::Joined { slot: 0 })
        .insert_resource(AcquiredPackages(acquired))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<GroundMaterial>>()
        .init_resource::<GraphicsSettings>()
        .init_resource::<ModelState>()
        .add_systems(
            Update,
            (setup_scene, turts_client::ground::sync_quality).chain(),
        );
    for (quality, width) in [
        (Quality::High, 1024),
        (Quality::Low, 512),
        (Quality::High, 1024),
    ] {
        app.world_mut().resource_mut::<GraphicsSettings>().quality = quality;
        app.update();
        let handle = app
            .world_mut()
            .query::<&MeshMaterial3d<GroundMaterial>>()
            .single(app.world())
            .unwrap();
        let material = app
            .world()
            .resource::<Assets<GroundMaterial>>()
            .get(&handle.0)
            .unwrap();
        let image = app
            .world()
            .resource::<Assets<Image>>()
            .get(material.extension.texture.as_ref().unwrap())
            .unwrap();
        assert_eq!(image.width(), width);
        assert_eq!(image.height(), width);
        assert!(image.asset_usage.contains(RenderAssetUsages::RENDER_WORLD));
        assert_eq!(material.base.perceptual_roughness, 1.0);
    }
}

#[test]
fn absent_overlay_uses_the_original_standard_material() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mods/skirmish");
    let archive = turts_mod_runtime::archive::PackageArchive::parse(
        &turts_mod_runtime::pack::pack_dir(&root).unwrap(),
    )
    .unwrap();
    let mut acquired = build_acquired(&[archive], String::new()).unwrap();
    acquired.presentation.ground.as_mut().unwrap().overlay = None;
    let mut app = App::new();
    app.insert_resource(Phase::Joined { slot: 0 })
        .insert_resource(AcquiredPackages(acquired))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<turts_client::ground::GroundMaterial>>()
        .init_resource::<GraphicsSettings>()
        .init_resource::<ModelState>()
        .add_systems(Update, setup_scene);
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&MeshMaterial3d<StandardMaterial>>()
            .iter(app.world())
            .count(),
        1
    );
    assert_eq!(
        app.world_mut()
            .query::<&MeshMaterial3d<turts_client::ground::GroundMaterial>>()
            .iter(app.world())
            .count(),
        0
    );
}
