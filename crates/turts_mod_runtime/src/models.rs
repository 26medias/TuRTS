//! Embedded glTF package contract. This module has no renderer or simulation dependency.
use crate::archive::PackageArchive;
use std::{collections::BTreeMap, io::Cursor};
use turts_mod_api::{Footprint, ModError, ModelDef, VisualTarget};

/// Loading limits. These are safety limits, not quality presets.
pub struct ModelLimits {
    pub file_bytes: usize,
    pub package_bytes: usize,
    pub triangles: usize,
    pub image_side: u32,
    pub materials: usize,
    pub joints: usize,
}
pub const MODEL_LIMITS: ModelLimits = ModelLimits {
    file_bytes: 32 * 1024 * 1024,
    package_bytes: 512 * 1024 * 1024,
    triangles: 100_000,
    image_side: 4096,
    materials: 8,
    joints: 128,
};
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub triangles: usize,
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub materials: Vec<String>,
    pub nodes: Vec<String>,
    pub warnings: Vec<String>,
    pub clips: Vec<String>,
    pub clip_bounds: BTreeMap<String, ([f32; 3], [f32; 3])>,
}
fn error(package: &str, file: &str, field: &str, message: impl Into<String>) -> ModError {
    ModError::new(package, field, message).with_file(file)
}

/// Parse exactly the glTF revision used by Bevy. External resources are never accepted.
pub fn inspect_glb(package: &str, file: &str, bytes: &[u8]) -> Result<ModelInfo, ModError> {
    let fail = |field, message: String| error(package, file, field, message);
    if bytes.len() > MODEL_LIMITS.file_bytes {
        return Err(fail("bytes", "model exceeds 32 MiB".into()));
    }
    if bytes.get(..4) != Some(b"glTF") || bytes.get(4..8) != Some(&2u32.to_le_bytes()) {
        return Err(fail("version", "expected glTF 2.0 binary (.glb)".into()));
    }
    let glb =
        gltf::Gltf::from_slice(bytes).map_err(|e| fail("model", format!("invalid GLB: {e}")))?;
    if let Some(extension) = glb.extensions_required().next() {
        return Err(fail(
            "extensionsRequired",
            format!("unsupported required extension `{extension}`"),
        ));
    }
    for buffer in glb.buffers() {
        if matches!(buffer.source(), gltf::buffer::Source::Uri(_)) {
            return Err(fail(
                "buffers.uri",
                "all model data must be embedded; external uri is refused".into(),
            ));
        }
    }
    let blob = glb.blob.as_deref().unwrap_or(&[]);
    if glb.buffers().any(|b| b.length() > blob.len()) {
        return Err(fail(
            "buffers.byteLength",
            "embedded buffer is shorter than its declared length".into(),
        ));
    }
    let mut warnings = Vec::new();
    for image in glb.images() {
        let gltf::image::Source::View { view, mime_type } = image.source() else {
            return Err(fail(
                "images.uri",
                "all images must be embedded; external uri is refused".into(),
            ));
        };
        if !matches!(mime_type, "image/png" | "image/jpeg") {
            return Err(fail(
                "images.mimeType",
                format!("stage 1 accepts PNG and JPEG, got `{mime_type}`"),
            ));
        }
        let data = blob
            .get(view.offset()..view.offset().saturating_add(view.length()))
            .ok_or_else(|| {
                fail(
                    "images.bufferView",
                    "image is outside the binary buffer".into(),
                )
            })?;
        let reader = image::ImageReader::new(Cursor::new(data))
            .with_guessed_format()
            .map_err(|e| fail("images", e.to_string()))?;
        let format = reader.format();
        if !matches!(
            format,
            Some(image::ImageFormat::Png | image::ImageFormat::Jpeg)
        ) {
            return Err(fail("images", "image bytes must be PNG or JPEG".into()));
        }
        let (w, h) = reader
            .into_dimensions()
            .map_err(|e| fail("images", e.to_string()))?;
        if w > MODEL_LIMITS.image_side || h > MODEL_LIMITS.image_side {
            return Err(fail(
                "images.size",
                format!("image {w}x{h} exceeds 4096 px on each side"),
            ));
        }
        if !w.is_power_of_two() || !h.is_power_of_two() {
            warnings.push(format!(
                "{package}/{file}: images.size: image {w}x{h} is not a power of two"
            ));
        }
    }
    if glb.materials().len() > MODEL_LIMITS.materials {
        return Err(fail("materials", "model has more than 8 materials".into()));
    }
    for material in glb.materials() {
        if material.alpha_mode() == gltf::material::AlphaMode::Blend {
            return Err(fail(
                "materials.alphaMode",
                "BLEND is not supported in stage 1; use OPAQUE or MASK".into(),
            ));
        }
    }
    for skin in glb.skins() {
        if skin.joints().len() > MODEL_LIMITS.joints {
            return Err(fail("skins.joints", "skin has more than 128 joints".into()));
        }
    }
    if glb.animations().len() > 64 {
        return Err(fail("animations", "model has more than 64 clips".into()));
    }
    for animation in glb.animations() {
        for channel in animation.channels() {
            let reader = channel.reader(|_| Some(blob));
            let inputs = reader
                .read_inputs()
                .ok_or_else(|| fail("animations", "unreadable clip times".into()))?;
            if inputs
                .into_iter()
                .any(|t| !t.is_finite() || !(0.0..=60.0).contains(&t))
            {
                return Err(fail(
                    "animations.duration",
                    "clip times must be finite and between 0 and 60 s".into(),
                ));
            }
        }
    }
    if glb.cameras().len() > 0
        || glb.nodes().any(|n| {
            n.extensions()
                .is_some_and(|e| e.contains_key("KHR_lights_punctual"))
        })
    {
        warnings.push(format!(
            "{package}/{file}: cameras/lights: embedded cameras and lights are ignored"
        ));
    }
    // Validate every node, including nodes outside the selected scene. Rig construction uses them.
    let children: Vec<Vec<usize>> = glb
        .nodes()
        .map(|n| n.children().map(|c| c.index()).collect())
        .collect();
    let mut parents = vec![0_u32; children.len()];
    for list in &children {
        for child in list {
            parents[*child] += 1;
            if parents[*child] > 1 {
                return Err(fail("nodes", "a node has multiple parents".into()));
            }
        }
    }
    fn visit(
        node: usize,
        children: &[Vec<usize>],
        colour: &mut [u8],
        depth: usize,
    ) -> Result<(), &'static str> {
        if depth > 128 {
            return Err("model node hierarchy exceeds 128 levels");
        }
        if colour[node] == 1 {
            return Err("model node hierarchy contains a cycle");
        }
        if colour[node] == 2 {
            return Ok(());
        }
        colour[node] = 1;
        for child in &children[node] {
            visit(*child, children, colour, depth + 1)?;
        }
        colour[node] = 2;
        Ok(())
    }
    let mut colour = vec![0; children.len()];
    for node in 0..children.len() {
        visit(node, &children, &mut colour, 0).map_err(|e| fail("nodes", e.into()))?;
    }
    let mut triangles = 0usize;
    for mesh in glb.meshes() {
        for primitive in mesh.primitives() {
            let count = primitive
                .indices()
                .map(|a| a.count())
                .or_else(|| primitive.get(&gltf::Semantic::Positions).map(|a| a.count()))
                .unwrap_or(0);
            triangles = triangles.saturating_add(match primitive.mode() {
                gltf::mesh::Mode::Triangles => count / 3,
                gltf::mesh::Mode::TriangleStrip | gltf::mesh::Mode::TriangleFan => {
                    count.saturating_sub(2)
                }
                _ => 0,
            });
        }
    }
    if triangles > MODEL_LIMITS.triangles {
        return Err(fail(
            "triangles",
            format!("model has {triangles} triangles; limit is 100000"),
        ));
    }
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    let scenes: Vec<_> = glb
        .default_scene()
        .into_iter()
        .chain(if glb.default_scene().is_none() {
            glb.scenes().collect()
        } else {
            Vec::new()
        })
        .collect();
    for scene in scenes {
        for node in scene.nodes() {
            bounds(node, identity(), blob, &mut min, &mut max, 0).map_err(|e| fail("bounds", e))?;
        }
    }
    if !min.into_iter().chain(max).all(f32::is_finite) {
        return Err(fail(
            "bounds",
            "model has no finite visible geometry".into(),
        ));
    }
    let clip_bounds = glb
        .animations()
        .filter_map(|animation| {
            let name = animation.name()?.to_owned();
            pose_bounds(&glb, blob, &animation).map(|bounds| (name, bounds))
        })
        .collect();
    Ok(ModelInfo {
        clip_bounds,
        clips: glb
            .animations()
            .filter_map(|a| a.name().map(str::to_owned))
            .collect(),
        triangles,
        min,
        max,
        materials: glb
            .materials()
            .filter_map(|m| m.name().map(str::to_owned))
            .collect(),
        nodes: glb
            .nodes()
            .filter_map(|n| n.name().map(str::to_owned))
            .collect(),
        warnings,
    })
}
fn identity() -> [[f32; 4]; 4] {
    [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., 1.],
    ]
}
fn multiply(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut c = [[0.; 4]; 4];
    for col in 0..4 {
        for row in 0..4 {
            c[col][row] = (0..4).map(|k| a[k][row] * b[col][k]).sum();
        }
    }
    c
}
fn bounds(
    node: gltf::Node<'_>,
    parent: [[f32; 4]; 4],
    blob: &[u8],
    min: &mut [f32; 3],
    max: &mut [f32; 3],
    depth: usize,
) -> Result<(), String> {
    if depth > 128 {
        return Err("model node hierarchy exceeds 128 levels".into());
    }
    let transform = multiply(parent, node.transform().matrix());
    if let Some(mesh) = node.mesh() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|_| Some(blob));
            let positions = reader
                .read_positions()
                .ok_or("mesh needs readable POSITION data")?;
            for p in positions {
                for axis in 0..3 {
                    let v = transform[0][axis] * p[0]
                        + transform[1][axis] * p[1]
                        + transform[2][axis] * p[2]
                        + transform[3][axis];
                    if !v.is_finite() {
                        return Err("non-finite model position".into());
                    }
                    min[axis] = min[axis].min(v);
                    max[axis] = max[axis].max(v);
                }
            }
        }
    }
    for child in node.children() {
        bounds(child, transform, blob, min, max, depth + 1)?;
    }
    Ok(())
}
/// Bounds of the first authored frame, including skinning. A wide bind pose is not an idle body.
fn pose_bounds(
    glb: &gltf::Gltf,
    blob: &[u8],
    animation: &gltf::Animation<'_>,
) -> Option<([f32; 3], [f32; 3])> {
    let mut local: Vec<_> = glb
        .nodes()
        .map(|node| node.transform().decomposed())
        .collect();
    for channel in animation.channels() {
        let reader = channel.reader(|_| Some(blob));
        let node = channel.target().node().index();
        // Cubic outputs store [incoming tangent, value, outgoing tangent] for each key.
        let key = usize::from(
            channel.sampler().interpolation() == gltf::animation::Interpolation::CubicSpline,
        );
        match reader.read_outputs()? {
            gltf::animation::util::ReadOutputs::Translations(mut values) => {
                local[node].0 = values.nth(key)?
            }
            gltf::animation::util::ReadOutputs::Rotations(values) => {
                local[node].1 = values.into_f32().nth(key)?
            }
            gltf::animation::util::ReadOutputs::Scales(mut values) => {
                local[node].2 = values.nth(key)?
            }
            _ => {}
        }
    }
    let local: Vec<_> = local
        .into_iter()
        .map(|(translation, rotation, scale)| {
            gltf::scene::Transform::Decomposed {
                translation,
                rotation,
                scale,
            }
            .matrix()
        })
        .collect();
    let mut world = vec![identity(); local.len()];
    fn walk(
        node: gltf::Node<'_>,
        parent: [[f32; 4]; 4],
        local: &[[[f32; 4]; 4]],
        world: &mut [[[f32; 4]; 4]],
    ) {
        let matrix = multiply(parent, local[node.index()]);
        world[node.index()] = matrix;
        for child in node.children() {
            walk(child, matrix, local, world);
        }
    }
    for node in glb.nodes().filter(|node| {
        !glb.nodes()
            .any(|p| p.children().any(|c| c.index() == node.index()))
    }) {
        walk(node, identity(), &local, &mut world);
    }
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    let transform = |matrix: [[f32; 4]; 4], p: [f32; 3]| -> [f32; 3] {
        std::array::from_fn(|axis| {
            matrix[0][axis] * p[0]
                + matrix[1][axis] * p[1]
                + matrix[2][axis] * p[2]
                + matrix[3][axis]
        })
    };
    for node in glb.nodes() {
        let Some(mesh) = node.mesh() else { continue };
        let skin = node.skin().map(|skin| {
            let inverse: Vec<_> = skin
                .reader(|_| Some(blob))
                .read_inverse_bind_matrices()
                .map(Iterator::collect)
                .unwrap_or_else(|| vec![identity(); skin.joints().len()]);
            skin.joints()
                .zip(inverse)
                .map(|(joint, inverse)| multiply(world[joint.index()], inverse))
                .collect::<Vec<_>>()
        });
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|_| Some(blob));
            let positions: Vec<_> = reader.read_positions()?.collect();
            let joints = reader
                .read_joints(0)
                .map(|v| v.into_u16().collect::<Vec<_>>());
            let weights = reader
                .read_weights(0)
                .map(|v| v.into_f32().collect::<Vec<_>>());
            for (i, p) in positions.into_iter().enumerate() {
                let point =
                    if let (Some(skin), Some(joints), Some(weights)) = (&skin, &joints, &weights) {
                        let mut point = [0.; 3];
                        for k in 0..4 {
                            let v = transform(*skin.get(joints.get(i)?[k] as usize)?, p);
                            for axis in 0..3 {
                                point[axis] += v[axis] * weights.get(i)?[k];
                            }
                        }
                        point
                    } else {
                        transform(world[node.index()], p)
                    };
                for axis in 0..3 {
                    if !point[axis].is_finite() {
                        return None;
                    }
                    min[axis] = min[axis].min(point[axis]);
                    max[axis] = max[axis].max(point[axis]);
                }
            }
        }
    }
    min.into_iter()
        .chain(max)
        .all(f32::is_finite)
        .then_some((min, max))
}
/// Use the authored idle pose for a skinned body's fit and picking bounds.
pub fn visual_bounds(info: &ModelInfo, model: &ModelDef) -> ([f32; 3], [f32; 3]) {
    model
        .animations
        .as_ref()
        .and_then(|a| a.idle.as_ref())
        .and_then(|idle| info.clip_bounds.get(idle.clip()))
        .copied()
        .unwrap_or((info.min, info.max))
}

fn check_name(
    package: &str,
    file: &str,
    field: &str,
    name: &Option<String>,
    names: &[String],
) -> Result<(), ModError> {
    if let Some(name) = name
        && !names.contains(name)
    {
        let hint = names
            .iter()
            .min_by_key(|n| edit_distance(name, n))
            .map_or(String::new(), |n| format!("; did you mean `{n}`?"));
        return Err(error(
            package,
            file,
            field,
            format!("unknown name `{name}`{hint}; names: {}", names.join(", ")),
        ));
    }
    Ok(())
}
fn edit_distance(a: &str, b: &str) -> usize {
    let mut row: Vec<_> = (0..=b.chars().count()).collect();
    for (i, ac) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, bc) in b.chars().enumerate() {
            let old = row[j + 1];
            row[j + 1] = (row[j] + 1).min(old + 1).min(prev + usize::from(ac != bc));
            prev = old;
        }
    }
    row[b.chars().count()]
}

/// Model checks are client/pack checks. The server never needs visual geometry.
/// Reject extended terrain whose texture coordinates cannot be represented by the renderer.
pub(crate) fn validate_environment_ground(
    package: &str,
    file: &str,
    map: &turts_mod_api::MapDef,
) -> Result<(), ModError> {
    let (Some(environment), Some(ground)) = (&map.environment, &map.ground) else {
        return Ok(());
    };
    if !ground.tile_size.is_finite() || ground.tile_size <= 0.0 {
        return Err(error(
            package,
            file,
            "ground.tile_size",
            "tile size must be finite and positive",
        ));
    }
    let margin = environment.ground_margin;
    let coordinates = [
        margin / ground.tile_size,
        (map.size.x + 2.0 * margin) / ground.tile_size,
        (map.size.y + 2.0 * margin) / ground.tile_size,
    ];
    if coordinates.into_iter().any(|value| !value.is_finite()) {
        return Err(error(
            package,
            file,
            "environment.ground_margin",
            "extended ground UV range must be finite for ground.tile_size",
        ));
    }
    Ok(())
}

/// Shared pack/content validation. Presentation never enters simulation state.
pub(crate) fn validate_ground_overlay(
    package: &str,
    file: &str,
    map: &turts_mod_api::MapDef,
) -> Result<(), ModError> {
    let Some(layer) = map.ground.as_ref().and_then(|g| g.overlay.as_ref()) else {
        return Ok(());
    };
    let fail = |field: &str, message: &str| {
        error(package, file, &format!("ground.overlay.{field}"), message)
    };
    if layer.colour.len() != 7
        || !layer.colour.starts_with('#')
        || !layer.colour.as_bytes()[1..]
            .iter()
            .all(u8::is_ascii_hexdigit)
    {
        return Err(fail("colour", "must use opaque #RRGGBB"));
    }
    if !layer.tile_size.is_finite() || layer.tile_size <= 0.0 {
        return Err(fail("tile_size", "must be finite and positive"));
    }
    let margin = map.environment.as_ref().map_or(0.0, |e| e.ground_margin);
    if [margin, map.size.x + margin, map.size.y + margin]
        .iter()
        .any(|x| !(x / layer.tile_size).is_finite())
    {
        return Err(fail(
            "tile_size",
            "world texture coordinates must be finite",
        ));
    }
    if !layer.roughness.is_finite() || !(0.0..=1.0).contains(&layer.roughness) {
        return Err(fail("roughness", "must be finite and between 0 and 1"));
    }
    if layer.patches.len() > 16 {
        return Err(fail("patches", "at most 16 patches are supported"));
    }
    for (i, patch) in layer.patches.iter().enumerate() {
        let field = |name: &str, message: &str| fail(&format!("patches[{i}].{name}"), message);
        if [patch.centre.x, patch.centre.y]
            .iter()
            .any(|x| !x.is_finite() || x.abs() > 10000.0)
        {
            return Err(field(
                "centre",
                "must be finite and within 10000 world units",
            ));
        }
        if [patch.size.x, patch.size.y]
            .iter()
            .any(|x| !x.is_finite() || *x <= 0.0 || *x > 10000.0)
        {
            return Err(field("size", "must be finite, positive, and at most 10000"));
        }
        if !patch.rotation_deg.is_finite() || !(-360.0..=360.0).contains(&patch.rotation_deg) {
            return Err(field(
                "rotation_deg",
                "must be finite and between -360 and 360",
            ));
        }
        if !patch.feather.is_finite()
            || patch.feather < 0.00001
            || patch.feather > patch.size.x.min(patch.size.y) * 0.5
        {
            return Err(field(
                "feather",
                "must be at least 0.00001 and no larger than half the smaller size",
            ));
        }
        if !patch.opacity.is_finite() || !(0.0..=1.0).contains(&patch.opacity) {
            return Err(field("opacity", "must be finite and between 0 and 1"));
        }
    }
    Ok(())
}

pub fn validate_models(archive: &PackageArchive) -> Result<Vec<String>, ModError> {
    let package = &archive.id().0;
    let mut images = std::collections::BTreeSet::new();
    let mut models = BTreeMap::new();
    let mut total = 0usize;
    for (file, assets) in archive.assets()? {
        for image in assets.images {
            images.insert(format!("{package}:{}", image.id));
        }
        for asset in assets.models {
            turts_mod_api::ContentId::parse(&format!("{package}:{}", asset.id))
                .map_err(|e| error(package, &file, "models.id", e))?;
            if models.contains_key(&asset.id) {
                return Err(error(
                    package,
                    &file,
                    "models.id",
                    format!("duplicate model asset `{}`", asset.id),
                ));
            }
            if !asset.path.ends_with(".glb") {
                return Err(error(
                    package,
                    &file,
                    "models.path",
                    "model files must use .glb",
                ));
            }
            let bytes = archive.file(&asset.path).ok_or_else(|| {
                error(
                    package,
                    &file,
                    "models.path",
                    format!("missing model `{}`", asset.path),
                )
            })?;
            total = total.saturating_add(bytes.len());
            if total > MODEL_LIMITS.package_bytes {
                return Err(error(
                    package,
                    &file,
                    "models.bytes",
                    "all models exceed 512 MiB",
                ));
            }
            models.insert(asset.id.clone(), inspect_glb(package, &asset.path, bytes)?);
        }
    }
    for (file, map) in archive.maps()? {
        validate_environment_ground(package, &file, &map)?;
        validate_ground_overlay(package, &file, &map)?;
        if let Some(ground) = map.ground {
            if !ground.tile_size.is_finite() || ground.tile_size <= 0.0 {
                return Err(error(
                    package,
                    &file,
                    "ground.tile_size",
                    "tile size must be finite and positive",
                ));
            }
            for image in std::iter::once(&ground.base_colour)
                .chain(ground.normal.iter())
                .chain(
                    ground
                        .overlay
                        .iter()
                        .flat_map(|layer| layer.base_colour.iter()),
                )
            {
                if !images.contains(image) {
                    return Err(error(
                        package,
                        &file,
                        "ground",
                        format!("unknown ground image `{image}`"),
                    ));
                }
            }
        }
    }
    let content = archive.content()?;
    let mut warnings = Vec::new();
    for (file, visual) in archive.visuals_unchecked()? {
        let Some(model) = visual.model else { continue };
        let radius = match &visual.target {
            VisualTarget::Unit(id) => content
                .units
                .iter()
                .find(|(k, _)| k.to_string() == *id)
                .map(|(_, u)| match u.footprint {
                    Footprint::Circle { radius } => radius,
                }),
            VisualTarget::Node(id) => content
                .nodes
                .iter()
                .find(|(k, _)| k.to_string() == *id)
                .map(|(_, n)| n.radius),
        }
        .ok_or_else(|| {
            error(
                package,
                &file,
                "target",
                "model target does not exist in this package",
            )
        })?;
        validate_visual_model(package, &file, &model, radius, &models, &mut warnings)?;
    }
    for info in models.values() {
        warnings.extend(info.warnings.iter().cloned());
    }
    Ok(warnings)
}
fn validate_visual_model(
    package: &str,
    file: &str,
    model: &ModelDef,
    radius: f32,
    models: &BTreeMap<String, ModelInfo>,
    warnings: &mut Vec<String>,
) -> Result<(), ModError> {
    if !model.scale.is_finite() || model.scale <= 0.0 {
        return Err(error(
            package,
            file,
            "model.scale",
            "scale must be finite and positive",
        ));
    }
    let mut threshold = 1.0;
    let mut triangles = usize::MAX;
    for (level, asset, below) in std::iter::once((0, model.asset.as_str(), None)).chain(
        model
            .lods
            .iter()
            .enumerate()
            .map(|(i, l)| (i + 1, l.asset.as_str(), Some(l.below_screen_height))),
    ) {
        let local = asset.strip_prefix(&format!("{package}:")).unwrap_or(asset);
        let info = models.get(local).ok_or_else(|| {
            error(
                package,
                file,
                "model.asset",
                format!("unknown model asset `{asset}`"),
            )
        })?;
        if let Some(below) = below {
            if !below.is_finite() || below <= 0.0 || below >= threshold {
                return Err(error(
                    package,
                    file,
                    "model.lods.below_screen_height",
                    "LOD thresholds must be positive, below 1, and strictly decrease",
                ));
            }
            threshold = below;
        }
        if info.triangles > triangles {
            return Err(error(
                package,
                file,
                "model.lods",
                "a LOD has more triangles than its preceding level",
            ));
        }
        triangles = info.triangles;
        check_name(
            package,
            file,
            "model.team_material",
            &model.team_material,
            &info.materials,
        )?;
        check_name(
            package,
            file,
            "model.turret_node",
            &model.turret_node,
            &info.nodes,
        )?;
        check_name(
            package,
            file,
            "model.muzzle_node",
            &model.muzzle_node,
            &info.nodes,
        )?;
        // A static far LOD has no clips. Animated LODs must honour the same mappings.
        if (level == 0 || !info.clips.is_empty())
            && let Some(animations) = &model.animations
        {
            for (action, clip) in [
                ("idle", &animations.idle),
                ("move", &animations.r#move),
                ("attack", &animations.attack),
                ("death", &animations.death),
                ("work", &animations.work),
                ("hit", &animations.hit),
            ] {
                let Some(clip) = clip else { continue };
                check_name(
                    package,
                    file,
                    &format!("model.animations.{action}.clip"),
                    &Some(clip.clip().to_owned()),
                    &info.clips,
                )?;
                if let Some(settings) = clip.settings() {
                    check_name(
                        package,
                        file,
                        "model.animations.hold",
                        &settings.hold,
                        &info.clips,
                    )?;
                    if settings.stride.is_some_and(|v| !v.is_finite() || v <= 0.0)
                        || settings
                            .fire_at
                            .is_some_and(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
                        || !settings.yaw_offset_deg.is_finite()
                    {
                        return Err(error(
                            package,
                            file,
                            "model.animations",
                            "stride must be positive; fire_at must be in 0..1; all timing and yaw values must be finite",
                        ));
                    }
                }
            }
        }
        let (min, max) = visual_bounds(info, model);
        let x = min[0].abs().max(max[0].abs()) * model.scale;
        let z = min[2].abs().max(max[2].abs()) * model.scale;
        let ratio = x.hypot(z) / radius;
        if !(0.5..=1.5).contains(&ratio) {
            return Err(error(
                package,
                file,
                "model.bounds",
                format!(
                    "LOD {level} scaled XZ radius is {ratio:.3} times the footprint radius; expected 0.5 to 1.5"
                ),
            ));
        }
        if ratio > 1.2 {
            warnings.push(format!("{package}/{file}: model.bounds: LOD {level} radius is {ratio:.3} times the footprint"));
        }
        if (min[1] * model.scale).abs() > 0.05 {
            return Err(error(
                package,
                file,
                "model.pivot",
                format!("LOD {level} lowest scaled point must be within 0.05 of ground"),
            ));
        }
        if level == 0 && info.triangles > 20_000 && model.lods.is_empty() {
            warnings.push(format!(
                "{package}/{file}: model.lods: model above 20000 triangles has no LOD"
            ));
        }
    }
    Ok(())
}
