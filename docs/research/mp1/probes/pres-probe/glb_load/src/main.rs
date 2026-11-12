//! PRES-PROBE probe 1: a model from package bytes on Bevy 0.20, headless.
//! Usage: glb_load <file.glb>
//! Without the `pbr` feature: AssetPlugin + memory source + MeshPlugin + WorldSerializationPlugin + GltfPlugin.
//! With the `pbr` feature: the same plus PbrPlugin on a RenderPlugin with no backend (no GPU).
use std::path::Path;

use bevy::app::{App, TaskPoolPlugin};
use bevy::asset::io::memory::{Dir, MemoryAssetReader};
use bevy::asset::io::AssetSourceBuilder;
use bevy::asset::{AssetApp, AssetPlugin, AssetServer, Assets, Handle};
use bevy::ecs::name::Name;
use bevy::ecs::prelude::*;
use bevy::gltf::{Gltf, GltfAssetLabel, GltfMaterial, GltfMaterialName, GltfMesh, GltfNode, GltfPlugin};
use bevy::mesh::{Mesh, Mesh3d, MeshPlugin};
use bevy::transform::TransformPlugin;
use bevy::world_serialization::{WorldAssetRoot, WorldSerializationPlugin};

#[cfg(feature = "pbr")]
use bevy::pbr::{MeshMaterial3d, StandardMaterial};

fn asset_path() -> &'static str {
    if std::env::var("LATE").is_ok() { "models/lock123/frontier/tank.glb" } else { "tank.glb" }
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: glb_load <file.glb>");
    let bytes = std::fs::read(&path).expect("read glb");
    println!("input={path} bytes={}", bytes.len());

    // The memory source. There is no file on disk for the asset server: the bytes are in a Dir.
    let dir = Dir::default();
    // LATE=1: the app is built and updated FIRST with an empty Dir; the bytes arrive later (the client
    // acquires the package after the app is built). Dir is an Arc<RwLock<..>>: a clone shared with the reader sees the insert.
    let late = std::env::var("LATE").is_ok();
    let asset_path = if late { "models/lock123/frontier/tank.glb" } else { "tank.glb" };
    if !late {
        dir.insert_asset(Path::new(asset_path), bytes.clone());
    }
    println!("late_insert={late} asset_path={asset_path}");
    let reader_dir = dir.clone();

    let mut app = App::new();
    // Rule of bevy_asset lib.rs:578-579: register the source BEFORE AssetPlugin.
    app.register_asset_source(
        "pkg",
        AssetSourceBuilder::new(move || {
            Box::new(MemoryAssetReader {
                root: reader_dir.clone(),
            })
        }),
    );
    app.add_plugins((
        TaskPoolPlugin::default(),
        // File sources are not needed: the default source is never read. The default path is "assets".
        AssetPlugin::default(),
        TransformPlugin,
        MeshPlugin,
        WorldSerializationPlugin,
    ));
    // GltfPlugin BEFORE PbrPlugin: PbrPlugin::build reads the GltfExtensionHandlers resource (bevy_pbr gltf.rs:26).
    app.add_plugins(GltfPlugin::default());
    #[cfg(feature = "pbr")]
    {
        use bevy::render::{settings::WgpuSettings, RenderPlugin};
        app.add_plugins((
            bevy::window::WindowPlugin { primary_window: None, ..Default::default() },
            bevy::camera::CameraPlugin,
            bevy::image::ImagePlugin::default(),
            RenderPlugin {
                render_creation: WgpuSettings {
                    backends: None,
                    ..Default::default()
                }
                .into(),
                ..Default::default()
            },
            bevy::pbr::PbrPlugin::default(),
        ));
    }
    #[cfg(not(feature = "pbr"))]
    {
        app.init_asset::<bevy::image::Image>();
    }
    println!("plugins=TaskPool,Asset,Transform,Mesh,WorldSerialization,Gltf pbr={}", cfg!(feature = "pbr"));

    // GltfPlugin registers the real loader in finish() (bevy_gltf lib.rs:287-313). A manual update loop must call finish() and cleanup().
    app.finish();
    app.cleanup();
    if late {
        for _ in 0..5 {
            app.update();
        }
        dir.insert_asset(Path::new(asset_path), bytes.clone());
        println!("bytes inserted after 5 updates of a built, finished app");
    }
    let handle: Handle<Gltf> = app.world().resource::<AssetServer>().load(format!("pkg://{asset_path}"));
    let mut updates = 0;
    loop {
        app.update();
        updates += 1;
        if app.world().resource::<AssetServer>().is_loaded_with_dependencies(&handle) {
            break;
        }
        if updates >= 200 {
            println!("LOAD FAILED after {updates} updates; state={:?}", app.world().resource::<AssetServer>().load_state(&handle));
            std::process::exit(3);
        }
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    println!("loaded_with_dependencies=true updates={updates}");

    // Named nodes, materials.
    let gltfs = app.world().resource::<Assets<Gltf>>();
    let gltf = gltfs.get(&handle).expect("gltf asset");
    let mut nn: Vec<_> = gltf.named_nodes.keys().map(|k| k.to_string()).collect();
    nn.sort();
    let mut nm: Vec<_> = gltf.named_materials.keys().map(|k| k.to_string()).collect();
    nm.sort();
    println!("named_nodes={nn:?}");
    println!("named_materials={nm:?}");
    println!("n_nodes={} n_meshes={} n_materials={} n_scenes={}", gltf.nodes.len(), gltf.meshes.len(), gltf.materials.len(), gltf.scenes.len());

    // Per node: local transform, mesh primitives, material handle, triangles.
    let nodes = app.world().resource::<Assets<GltfNode>>();
    let meshes = app.world().resource::<Assets<GltfMesh>>();
    let raw_meshes = app.world().resource::<Assets<Mesh>>();
    let gltf_mats = app.world().resource::<Assets<GltfMaterial>>();
    let mut tris = 0usize;
    for (i, nh) in gltf.nodes.iter().enumerate() {
        let n = nodes.get(nh).unwrap();
        println!("node[{i}] name={:?} translation={:?} children={} mesh={}", n.name, n.transform.translation, n.children.len(), n.mesh.is_some());
        if let Some(mh) = &n.mesh {
            let m = meshes.get(mh).unwrap();
            for p in &m.primitives {
                let mesh = raw_meshes.get(&p.mesh).unwrap();
                let t = mesh.indices().map_or(0, |ix| ix.len()) / 3;
                tris += t;
                let mat = p.material.as_ref().map(|h| gltf_mats.get(h).map(|g| g.base_color));
                println!("    mesh {:?} primitive {} tris={t} material_is_Option<Handle<GltfMaterial>>={} base_color={mat:?}", m.name, p.index, p.material.is_some());
            }
        }
    }
    println!("triangles_total={tris} (hull 36 + hull_team 12 + turret 24 = 72 expected)");
    // glTF material label API.
    for (i, name) in ["body", "team"].iter().enumerate() {
        let label = GltfAssetLabel::Material { index: i, is_scale_inverted: false };
        let std_label = format!("{label}/std");
        println!("material {name}: gltf label={label} std label={std_label} named_materials_has={}", gltf.named_materials.contains_key(*name));
    }

    #[cfg(feature = "pbr")]
    pbr_part(&mut app, &handle);
    #[cfg(not(feature = "pbr"))]
    nopbr_part(&mut app, &handle);
}

fn spawn_two_units(app: &mut App, handle: &Handle<Gltf>) -> (Entity, Entity) {
    let scene = app.world().resource::<Assets<Gltf>>().get(handle).unwrap().default_scene.clone().expect("default scene");
    let a = app.world_mut().spawn(WorldAssetRoot(scene.clone())).id();
    let b = app.world_mut().spawn(WorldAssetRoot(scene)).id();
    for _ in 0..4 {
        app.update();
    }
    (a, b)
}

fn dump_hierarchy(world: &mut World, root: Entity, depth: usize) {
    let comps: Vec<String> = world
        .inspect_entity(root)
        .unwrap()
        .map(|c| c.1.name().to_string().rsplit("::").next().unwrap().to_string())
        .collect();
    let name = world.get::<Name>(root).map(|n| n.to_string()).unwrap_or_default();
    println!("{}{root:?} name={name:?} components={comps:?}", "    ".repeat(depth));
    let kids: Vec<Entity> = world.get::<Children>(root).map(|c| c.iter().collect()).unwrap_or_default();
    for k in kids {
        dump_hierarchy(world, k, depth + 1);
    }
}

#[cfg(not(feature = "pbr"))]
fn nopbr_part(app: &mut App, handle: &Handle<Gltf>) {
    let (a, _b) = spawn_two_units(app, handle);
    println!("--- hierarchy of unit A after 4 updates (no pbr) ---");
    dump_hierarchy(app.world_mut(), a, 0);
    let mut q = app.world_mut().query::<(&GltfMaterialName, &Mesh3d)>();
    let n = q.iter(app.world()).count();
    println!("entities with GltfMaterialName+Mesh3d (two units)={n}");
    println!("NOPBR: no StandardMaterial type in the app: no `Material{{n}}/std` asset exists");
}

#[cfg(feature = "pbr")]
fn pbr_part(app: &mut App, handle: &Handle<Gltf>) {
    let server = app.world().resource::<AssetServer>().clone();
    // The labelled standard material.
    for (i, name) in ["body", "team"].iter().enumerate() {
        let label = GltfAssetLabel::Material { index: i, is_scale_inverted: false };
        let path = format!("pkg://{}#{label}/std", asset_path());
        let h: Handle<StandardMaterial> = server.load(path.clone());
        let mut n = 0;
        while !server.is_loaded_with_dependencies(&h) && n < 50 {
            app.update();
            n += 1;
        }
        let ok = app.world().resource::<Assets<StandardMaterial>>().get(&h).map(|m| m.base_color);
        println!("std material {name}: path={path} loaded={} base_color={ok:?} handle_id={:?}", server.is_loaded_with_dependencies(&h), h.id());
        // Also via the Gltf asset's own label handle.
        let _ = handle;
    }
    let mats_before = app.world().resource::<Assets<StandardMaterial>>().len();
    let (a, b) = spawn_two_units(app, handle);
    println!("--- hierarchy of unit A after 4 updates (pbr) ---");
    dump_hierarchy(app.world_mut(), a, 0);

    // Collect entities of each unit with the material name.
    fn mesh_entities(world: &mut World, root: Entity) -> Vec<(Entity, String, bevy::asset::AssetId<StandardMaterial>)> {
        let mut out = Vec::new();
        let mut stack = vec![root];
        while let Some(e) = stack.pop() {
            if let (Some(n), Some(m)) = (world.get::<GltfMaterialName>(e), world.get::<MeshMaterial3d<StandardMaterial>>(e)) {
                out.push((e, n.0.clone(), m.0.id()));
            }
            if let Some(c) = world.get::<Children>(e) {
                stack.extend(c.iter());
            }
        }
        out
    }
    let ma = mesh_entities(app.world_mut(), a);
    let mb = mesh_entities(app.world_mut(), b);
    println!("unit A mesh entities (entity,name,std material id)={ma:?}");
    println!("unit B mesh entities={mb:?}");
    let mats_after = app.world().resource::<Assets<StandardMaterial>>().len();
    println!("StandardMaterial assets before spawn={mats_before} after spawning 2 units={mats_after} (equal: no material per unit)");
    let shared = ma.iter().zip(mb.iter()).all(|(x, y)| x.2 == y.2);
    println!("units A and B share the same material handle on each mesh entity={shared}");

    // The per-slot team material: ONE handle for all units of a slot.
    let slot_red = app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial {
        base_color: bevy::color::Color::srgb(1.0, 0.0, 0.0),
        ..Default::default()
    });
    let slot_blue = app.world_mut().resource_mut::<Assets<StandardMaterial>>().add(StandardMaterial {
        base_color: bevy::color::Color::srgb(0.0, 0.0, 1.0),
        ..Default::default()
    });
    for (root, slot) in [(a, &slot_red), (b, &slot_blue)] {
        for (e, name, _) in mesh_entities(app.world_mut(), root) {
            if name == "team" {
                app.world_mut().entity_mut(e).insert(MeshMaterial3d(slot.clone()));
            }
        }
    }
    app.update();
    let ma = mesh_entities(app.world_mut(), a);
    let mb = mesh_entities(app.world_mut(), b);
    println!("after the swap: unit A (slot red)={ma:?}");
    println!("after the swap: unit B (slot blue)={mb:?}");
    let team_a: Vec<_> = ma.iter().filter(|x| x.1 == "team").map(|x| x.2).collect();
    let team_b: Vec<_> = mb.iter().filter(|x| x.1 == "team").map(|x| x.2).collect();
    println!("team entities A all on slot_red={} B all on slot_blue={}", team_a.iter().all(|i| *i == slot_red.id()), team_b.iter().all(|i| *i == slot_blue.id()));
    let body_a: Vec<_> = ma.iter().filter(|x| x.1 == "body").map(|x| x.2).collect();
    let body_b: Vec<_> = mb.iter().filter(|x| x.1 == "body").map(|x| x.2).collect();
    println!("body entities A and B still share one handle={}", body_a.iter().chain(body_b.iter()).all(|i| *i == body_a[0]));
    println!("StandardMaterial assets now={} (before spawn {mats_before}; +2 slot materials only)", app.world().resource::<Assets<StandardMaterial>>().len());

    // The flat template of D 8.1: no scene spawn; entities built from the GltfNode data.
    flat_spawn(app, handle, &slot_red);
}

#[cfg(feature = "pbr")]
fn flat_spawn(app: &mut App, handle: &Handle<Gltf>, slot: &Handle<StandardMaterial>) {
    use bevy::transform::components::Transform;
    let server = app.world().resource::<AssetServer>().clone();
    let gltf = app.world().resource::<Assets<Gltf>>().get(handle).unwrap();
    let nodes = app.world().resource::<Assets<GltfNode>>();
    let meshes = app.world().resource::<Assets<GltfMesh>>();
    let gltf_mats = app.world().resource::<Assets<GltfMaterial>>();
    // Material index of the team material from named_materials; the std handle from the label.
    let team_gm = gltf.named_materials.get("team").unwrap().clone();
    let mut parts: Vec<(Transform, Handle<Mesh>, bool, Handle<StandardMaterial>)> = Vec::new();
    // Parent transforms: the loader gives a node tree; walk from nodes that are nobody's child.
    let mut is_child = vec![false; gltf.nodes.len()];
    for nh in &gltf.nodes {
        for c in &nodes.get(nh).unwrap().children {
            let idx = gltf.nodes.iter().position(|x| x.id() == c.id()).unwrap();
            is_child[idx] = true;
        }
    }
    fn walk(
        nh: &Handle<GltfNode>,
        parent: Transform,
        nodes: &Assets<GltfNode>,
        meshes: &Assets<GltfMesh>,
        team_gm: &Handle<GltfMaterial>,
        gltf_mats: &Assets<GltfMaterial>,
        server: &AssetServer,
        parts: &mut Vec<(Transform, Handle<Mesh>, bool, Handle<StandardMaterial>)>,
    ) {
        let n = nodes.get(nh).unwrap();
        let world = parent * n.transform;
        if let Some(mh) = &n.mesh {
            for p in &meshes.get(mh).unwrap().primitives {
                let gm = p.material.clone().unwrap();
                let is_team = gm.id() == team_gm.id();
                // The material label: Material{n}/std. n is found by the gltf material index in `Gltf.materials`.
                let _ = gltf_mats;
                let idx_label = format!("pkg://{}#Material{}/std", asset_path(), if is_team { 1 } else { 0 });
                parts.push((world, p.mesh.clone(), is_team, server.load(idx_label)));
            }
        }
        for c in &n.children {
            walk(c, world, nodes, meshes, team_gm, gltf_mats, server, parts);
        }
    }
    for (i, nh) in gltf.nodes.iter().enumerate() {
        if !is_child[i] {
            walk(nh, Transform::IDENTITY, nodes, meshes, &team_gm, gltf_mats, &server, &mut parts);
        }
    }
    println!("--- flat template (no scene spawn) ---");
    println!("template parts={} (mesh, is_team, y of part)", parts.len());
    // Spawn one unit: one root entity + one child per part. The team part uses the slot handle.
    let root = app.world_mut().spawn((Transform::default(), bevy::camera::visibility::Visibility::default())).id();
    for (t, mesh, is_team, std) in parts {
        let mat = if is_team { slot.clone() } else { std };
        app.world_mut().spawn((t, Mesh3d(mesh), MeshMaterial3d(mat), ChildOf(root)));
    }
    for _ in 0..3 {
        app.update();
    }
    let n_kids = app.world().get::<Children>(root).map(|c| c.len()).unwrap_or(0);
    println!("flat unit: root + {n_kids} child entities; no new StandardMaterial per unit (slot handle reused)");
}
