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

#[test]
fn authored_fog_without_overlay_uses_masked_ground_from_the_selected_root_map() {
    use turts_client::acquire::{VerifiedRootMod, build_acquired_with_root};
    use turts_client::ground::GroundMaterial;
    let load = |name: &str| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../mods")
            .join(name);
        turts_mod_runtime::archive::PackageArchive::parse(
            &turts_mod_runtime::pack::pack_dir(&path).unwrap(),
        )
        .unwrap()
    };
    let fog = load("visibility_probe");
    let flat = load("skirmish");
    let root = |archive: &turts_mod_runtime::archive::PackageArchive| VerifiedRootMod {
        id: archive.manifest.id.clone(),
        version: archive.manifest.version.to_string(),
    };
    let flat_root = root(&flat);
    let fog_root = root(&fog);
    let archives = [fog, flat];
    let selected_flat =
        build_acquired_with_root(&archives, String::new(), Some(flat_root)).unwrap();
    assert!(
        !selected_flat.presentation.requires_fog,
        "an unrelated fog package must not change an AllVisible root map"
    );
    let acquired = build_acquired_with_root(&archives, String::new(), Some(fog_root)).unwrap();
    assert!(acquired.presentation.requires_fog);
    assert!(
        acquired.presentation.ground.is_none(),
        "the actual fog fixture must exercise the untextured, no-overlay path"
    );
    let mut app = App::new();
    app.insert_resource(Phase::Joined { slot: 0 })
        .insert_resource(AcquiredPackages(acquired))
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<StandardMaterial>>()
        .init_resource::<Assets<GroundMaterial>>()
        .init_resource::<GraphicsSettings>()
        .init_resource::<ModelState>()
        .add_systems(Update, setup_scene);
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&MeshMaterial3d<GroundMaterial>>()
            .iter(app.world())
            .count(),
        1
    );
    assert_eq!(
        app.world_mut()
            .query::<&MeshMaterial3d<StandardMaterial>>()
            .iter(app.world())
            .count(),
        0
    );
}
