//! Pack-time bounds and asset references for declarative visual effects.
use crate::archive::PackageArchive;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Cursor,
};
use turts_mod_api::{EffectCost, EffectLayerKind as Kind, ModError, VisualEffectDef};

pub(crate) fn validate(archive: &PackageArchive) -> Result<(), ModError> {
    let package = archive.id().0;
    let fail = |file: &str, field: &str, message: String| {
        ModError::new(&package, field, message).with_file(file)
    };
    let mut images = BTreeMap::new();
    let mut models = BTreeSet::new();
    for (_, assets) in archive.assets()? {
        for image in assets.images {
            images.insert(format!("{package}:{}", image.id), image.path);
        }
        for model in assets.models {
            models.insert(format!("{package}:{}", model.id));
        }
    }
    let qualified = |name: &str| {
        if name.contains(':') {
            name.to_owned()
        } else {
            format!("{package}:{name}")
        }
    };
    let external = |name: &str| {
        name.split_once(':').is_some_and(|(owner, _)| {
            owner != package && archive.manifest.dependencies.0.contains_key(owner)
        })
    };
    let image_check = |file: &str, field: &str, image: &str| -> Result<(), ModError> {
        if external(image) {
            return Ok(());
        }
        let key = qualified(image);
        let path = images
            .get(&key)
            .ok_or_else(|| fail(file, field, format!("unknown effect image `{image}`")))?;
        let bytes = archive
            .file(path)
            .ok_or_else(|| fail(file, field, format!("missing effect image `{image}`")))?;
        let reader = image::ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|e| fail(file, field, e.to_string()))?;
        let (w, h) = reader
            .into_dimensions()
            .map_err(|e| fail(file, field, e.to_string()))?;
        if w > 512 || h > 512 {
            return Err(fail(
                file,
                field,
                format!("effect image {w}x{h} exceeds 512 px"),
            ));
        }
        Ok(())
    };
    let model_check = |file: &str, field: &str, model: &str| -> Result<(), ModError> {
        if !models.contains(&qualified(model)) && !external(model) {
            return Err(fail(file, field, format!("unknown effect model `{model}`")));
        }
        Ok(())
    };
    let files = archive.visual_effect_defs()?;
    let definitions: BTreeMap<String, &VisualEffectDef> = files
        .iter()
        .flat_map(|(_, f)| f.effects.iter())
        .map(|e| (qualified(&e.id), e))
        .collect();
    let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (file, group) in &files {
        for effect in &group.effects {
            let field = format!("effects.{}", effect.id);
            turts_mod_api::ContentId::parse(&qualified(&effect.id))
                .map_err(|message| fail(file, &format!("{field}.id"), message))?;
            let invalid = |message: &str| fail(file, &field, message.into());
            if effect.layers.len() > 8 {
                return Err(invalid("effect has more than 8 layers"));
            }
            if effect.layers.is_empty() {
                if !effect.size.is_finite() || effect.size <= 0. || effect.ms > 10_000 {
                    return Err(invalid(
                        "legacy effect size must be finite, positive; duration must be at most 10000 ms",
                    ));
                }
                continue;
            }
            let mut peak = 0_u32;
            let mut longest = 0_u32;
            for (index, layer) in effect.layers.iter().enumerate() {
                let path = format!("{field}.layers[{index}]");
                let error = |message: &str| fail(file, &path, message.into());
                let scalar = |value: f32, positive: bool| {
                    value.is_finite() && value >= 0. && (!positive || value > 0.)
                };
                let colour = |value: &str| {
                    value.strip_prefix('#').is_some_and(|s| {
                        matches!(s.len(), 6 | 8) && s.bytes().all(|b| b.is_ascii_hexdigit())
                    })
                };
                if !colour(&layer.colour) || layer.end_colour.as_deref().is_some_and(|c| !colour(c))
                {
                    return Err(error("colours must use #RRGGBB or #RRGGBBAA"));
                }
                if !scalar(layer.size, true)
                    || layer.end_size.is_some_and(|v| !scalar(v, false))
                    || !layer.offset.iter().all(|v| v.is_finite())
                {
                    return Err(error(
                        "size and offsets must be finite and valid numeric ranges",
                    ));
                }
                if layer.delay_ms > 10_000
                    || (!layer.looping
                        && !matches!(layer.kind, Kind::Decal { .. })
                        && layer.ms > 10_000)
                {
                    return Err(error(
                        "a non-looping layer and its delay must be at most 10000 ms",
                    ));
                }
                if let Some(image) = &layer.image {
                    image_check(file, &path, image)?;
                }
                let count = match &layer.kind {
                    Kind::Flash { radius, .. } => {
                        if !scalar(*radius, true) {
                            return Err(error("flash radius must be finite, positive"));
                        }
                        1
                    }
                    Kind::Sparks {
                        count,
                        speed,
                        spread_deg,
                        gravity,
                    } => {
                        if !scalar(*speed, false)
                            || !scalar(*gravity, false)
                            || !spread_deg.is_finite()
                            || !(0.0..=360.0).contains(spread_deg)
                        {
                            return Err(error(
                                "spark speed, gravity and spread must be finite; spread must be in 0..360",
                            ));
                        }
                        u32::from(*count)
                    }
                    Kind::Burst {
                        radius_start,
                        radius_end,
                    } => {
                        if !scalar(*radius_start, false) || !scalar(*radius_end, true) {
                            return Err(error("burst radii must be finite and nonnegative"));
                        }
                        1
                    }
                    Kind::Smoke { rate, rise, spread } => {
                        if !scalar(*rate, true) || !scalar(*rise, false) || !scalar(*spread, false)
                        {
                            return Err(error(
                                "smoke rate, rise and spread must be finite and nonnegative",
                            ));
                        }
                        (*rate * layer.ms as f32 / 1000.).ceil() as u32
                    }
                    Kind::Fire { rate, height } => {
                        if !scalar(*rate, true) || !scalar(*height, true) {
                            return Err(error("fire rate and height must be finite, positive"));
                        }
                        (*rate * layer.ms as f32 / 1000.).ceil() as u32
                    }
                    Kind::Debris {
                        count,
                        speed,
                        model,
                        image,
                    } => {
                        if !scalar(*speed, false) {
                            return Err(error("debris speed must be finite and nonnegative"));
                        }
                        if let Some(model) = model {
                            model_check(file, &path, model)?;
                        }
                        if let Some(image) = image {
                            image_check(file, &path, image)?;
                        }
                        u32::from(*count)
                    }
                    Kind::Trail { ms, .. } if *ms > 10_000 => {
                        return Err(error("trail duration must be at most 10000 ms"));
                    }
                    Kind::Tracer { width } | Kind::Trail { width, .. } => {
                        if !scalar(*width, true) {
                            return Err(error("line width must be finite, positive"));
                        }
                        1
                    }
                    Kind::Projectile {
                        speed,
                        arc_height,
                        model,
                        image,
                        trail,
                    } => {
                        if !speed.is_finite() || *speed <= 0. || !scalar(*arc_height, false) {
                            return Err(error(
                                "projectile speed must be finite and positive; arc height must be finite and nonnegative",
                            ));
                        }
                        if let Some(model) = model {
                            model_check(file, &path, model)?;
                        }
                        if let Some(image) = image {
                            image_check(file, &path, image)?;
                        }
                        if let Some(trail) = trail {
                            let target = qualified(trail);
                            if !definitions.contains_key(&target) && !external(trail) {
                                return Err(error("unknown projectile trail effect"));
                            }
                            graph.entry(qualified(&effect.id)).or_default().push(target);
                        }
                        1
                    }
                    Kind::Decal { image, size, ms } => {
                        if !scalar(*size, true) || *ms > 120_000 {
                            return Err(error(
                                "decal size must be finite and positive, and duration at most 120000 ms",
                            ));
                        }
                        if let Some(image) = image {
                            image_check(file, &path, image)?;
                        }
                        0
                    }
                    Kind::Shake { strength, ms } => {
                        if !scalar(*strength, false) || *ms > 10_000 {
                            return Err(error(
                                "shake strength must be finite and positive and duration at most 10000 ms",
                            ));
                        }
                        0
                    }
                };
                if count > 64 {
                    return Err(error("a layer has more than 64 particles"));
                }
                peak = peak.saturating_add(count);
                if !matches!(layer.kind, Kind::Decal { .. } | Kind::Shake { .. }) {
                    longest = longest.max(layer.ms);
                }
            }
            let (count, duration) = match effect.cost {
                EffectCost::Small => (16, 1000),
                EffectCost::Medium => (64, 3000),
                EffectCost::Large => (512, 10_000),
            };
            if peak > count || longest > duration {
                return Err(invalid(
                    "declared effect cost is too small for its particle count or lifetime",
                ));
            }
        }
    }
    fn visit(
        node: &str,
        graph: &BTreeMap<String, Vec<String>>,
        path: &mut BTreeSet<String>,
        done: &mut BTreeSet<String>,
        depth: usize,
    ) -> bool {
        if depth > 16 || path.contains(node) {
            return false;
        }
        if done.contains(node) {
            return true;
        }
        path.insert(node.into());
        if graph.get(node).is_some_and(|children| {
            children
                .iter()
                .any(|child| !visit(child, graph, path, done, depth + 1))
        }) {
            return false;
        }
        path.remove(node);
        done.insert(node.into());
        true
    }
    let mut done = BTreeSet::new();
    for node in graph.keys() {
        if !visit(node, &graph, &mut BTreeSet::new(), &mut done, 0) {
            return Err(fail(
                "visual_effects",
                "trail",
                "effect references contain a cycle or exceed 16 levels".into(),
            ));
        }
    }
    Ok(())
}
