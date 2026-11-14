//! Check effect references after every dependency's presentation assets have been merged.
use crate::acquire::PresentationContent;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Cursor,
};
use turts_mod_api::EffectLayerKind;
pub(crate) fn resolved_effects(content: &PresentationContent) -> Result<(), String> {
    for (target, refs) in &content.effect_visuals {
        for (field, reference) in [
            ("fire", refs.fire.as_ref()),
            ("hit", refs.hit.as_ref()),
            ("hit_reduced", refs.hit_reduced.as_ref()),
            ("death", refs.death.as_ref()),
            ("projectile", refs.projectile.as_ref()),
            ("work", refs.work.as_ref()),
            ("site", refs.site.as_ref()),
        ] {
            if let Some(reference) = reference
                && !content.effects.contains_key(reference)
            {
                return Err(format!(
                    "{target}: effects.{field}: unknown effect `{reference}`"
                ));
            }
        }
        if let Some(damaged) = &refs.damaged {
            for condition in damaged.as_slice() {
                if !content.effects.contains_key(&condition.effect) {
                    return Err(format!(
                        "{target}: effects.damaged: unknown effect `{}`",
                        condition.effect
                    ));
                }
            }
        }
    }
    let mut graph = BTreeMap::<&str, Vec<&str>>::new();
    for (id, effect) in &content.effects {
        if let Some(sound) = &effect.sound
            && !content.sounds.contains_key(sound)
        {
            return Err(format!("{id}: sound: unknown effect sound `{sound}`"));
        }
        for (index, layer) in effect.layers.iter().enumerate() {
            let field = format!("{id}: layers[{index}]");
            let image = |name: &str| -> Result<(), String> {
                let image = content
                    .images
                    .get(name)
                    .ok_or_else(|| format!("{field}.image: unknown effect image `{name}`"))?;
                let (width, height) = image::ImageReader::new(Cursor::new(&image.bytes))
                    .with_guessed_format()
                    .map_err(|e| format!("{field}.image: {e}"))?
                    .into_dimensions()
                    .map_err(|e| format!("{field}.image: {e}"))?;
                if width > 512 || height > 512 {
                    return Err(format!("{field}.image: {width}x{height} exceeds 512 px"));
                }
                Ok(())
            };
            let model = |name: &str| -> Result<(), String> {
                if content.models.contains_key(name) {
                    Ok(())
                } else {
                    Err(format!("{field}.model: unknown effect model `{name}`"))
                }
            };
            if let Some(name) = &layer.image {
                image(name)?;
            }
            match &layer.kind {
                EffectLayerKind::Projectile {
                    model: asset,
                    image: picture,
                    trail,
                    ..
                } => {
                    if let Some(name) = asset {
                        model(name)?;
                    }
                    if let Some(name) = picture {
                        image(name)?;
                    }
                    if let Some(name) = trail {
                        if !content.effects.contains_key(name) {
                            return Err(format!("{field}.trail: unknown effect `{name}`"));
                        }
                        graph.entry(id).or_default().push(name);
                    }
                }
                EffectLayerKind::Debris {
                    model: asset,
                    image: picture,
                    ..
                } => {
                    if let Some(name) = asset {
                        model(name)?;
                    }
                    if let Some(name) = picture {
                        image(name)?;
                    }
                }
                EffectLayerKind::Decal {
                    image: Some(name), ..
                } => image(name)?,
                _ => {}
            }
        }
    }
    fn visit<'a>(
        id: &'a str,
        graph: &BTreeMap<&'a str, Vec<&'a str>>,
        path: &mut BTreeSet<&'a str>,
        done: &mut BTreeSet<&'a str>,
    ) -> bool {
        if path.len() > 16 || path.contains(id) {
            return false;
        }
        if done.contains(id) {
            return true;
        }
        path.insert(id);
        if graph.get(id).is_some_and(|targets| {
            targets
                .iter()
                .any(|target| !visit(target, graph, path, done))
        }) {
            return false;
        }
        path.remove(id);
        done.insert(id);
        true
    }
    let mut done = BTreeSet::new();
    for id in graph.keys() {
        if !visit(id, &graph, &mut BTreeSet::new(), &mut done) {
            return Err(format!(
                "{id}: trail: effect references contain a cycle or exceed 16 levels"
            ));
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::acquire::PresentationImage;
    #[test]
    fn dependency_images_obey_the_same_effect_limit() {
        use image::ImageEncoder;
        let mut content = PresentationContent::default();
        let effect: turts_mod_api::VisualEffectsDef = ron::from_str(
            "(effects:[(id:\"hit\",layers:[(kind:Flash(),image:Some(\"library:large\"))])])",
        )
        .unwrap();
        content
            .effects
            .insert("game:hit".into(), effect.effects[0].clone());
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png)
            .write_image(&vec![255; 513 * 4], 513, 1, image::ExtendedColorType::Rgba8)
            .unwrap();
        content.images.insert(
            "library:large".into(),
            PresentationImage {
                bytes: png,
                nine_slice: None,
            },
        );
        let error = resolved_effects(&content).unwrap_err();
        assert!(error.contains("513x1 exceeds 512 px"), "{error}");
    }
}
