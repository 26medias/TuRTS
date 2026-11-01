//! Accepted package images and fonts used by the retained widget tree.

use std::collections::{BTreeMap, BTreeSet};

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::text::FontCx;

use crate::{acquire::PresentationContent, join::AcquiredPackages};

/// Handles and metadata for assets referenced by accepted interface content.
#[derive(Resource, Default)]
pub struct WidgetAssets {
    /// Increments whenever a successfully decoded package set replaces these handles.
    pub revision: u64,
    /// Lockfile whose presentation payloads produced these handles.
    pub source_lockfile_hash: String,
    /// Package-qualified image handles retained for UI drawing.
    pub images: BTreeMap<String, Handle<Image>>,
    /// Independently decoded image handles retained for cursor CPU-side reads.
    pub cursor_images: BTreeMap<(String, (u16, u16)), Handle<Image>>,
    /// Package-qualified font handles.
    pub fonts: BTreeMap<String, Handle<Font>>,
    /// Authored nine-slice margins keyed by package-qualified image id.
    pub nine_slices: BTreeMap<String, (u16, u16, u16, u16)>,
}

impl WidgetAssets {
    /// Find a normal UI image by its package-qualified id.
    pub fn image(&self, id: &str) -> Option<Handle<Image>> {
        self.images.get(id).cloned()
    }

    /// Find separately decoded cursor pixels by image id and hotspot. The operating system
    /// caches custom cursors by handle without including the hotspot in its key.
    pub fn cursor_image_for(&self, id: &str, hotspot: (u16, u16)) -> Option<Handle<Image>> {
        self.cursor_images.get(&(id.to_owned(), hotspot)).cloned()
    }

    /// Find a font by its package-qualified id.
    pub fn font(&self, id: &str) -> Option<Handle<Font>> {
        self.fonts.get(id).cloned()
    }

    /// Find authored nine-slice margins by their package-qualified image id.
    pub fn nine_slice(&self, id: &str) -> Option<(u16, u16, u16, u16)> {
        self.nine_slices.get(id).copied()
    }

    /// Whether handles belong to the accepted package set, or no decoded presentation assets
    /// are needed for that set.
    pub fn matches_accepted_packages(
        &self,
        lockfile_hash: &str,
        presentation: &PresentationContent,
    ) -> bool {
        (presentation.images.is_empty() && presentation.fonts.is_empty())
            || self.source_lockfile_hash == lockfile_hash
    }
}

/// Decode accepted presentation assets and replace the retained widget asset resource.
///
/// This system runs only when accepted packages change. Invalid payloads are logged and leave
/// the previous resource and its handles intact; the public loader returns the detailed error.
pub fn load_widget_assets(
    acquired: Option<Res<AcquiredPackages>>,
    mut images: ResMut<Assets<Image>>,
    mut fonts: ResMut<Assets<Font>>,
    existing: Option<ResMut<WidgetAssets>>,
    phase: Option<ResMut<crate::join::Phase>>,
    mut commands: Commands,
) {
    let Some(acquired) = acquired else {
        return;
    };
    if existing.is_some() && !acquired.is_changed() {
        return;
    }

    let previous = existing.as_deref();
    match load_presentation_assets(&acquired.0.presentation, &mut images, &mut fonts, previous) {
        Ok(mut loaded) => {
            loaded.source_lockfile_hash = acquired.0.lockfile_hash.clone();
            if let Some(mut existing) = existing {
                *existing = loaded;
            } else {
                commands.insert_resource(loaded);
            }
        }
        Err(error) => {
            if let Some(mut phase) = phase {
                *phase = crate::join::Phase::Failed(error.clone());
            }
            error!("failed to load accepted widget assets: {error}");
        }
    }
}

/// Decode presentation assets into Bevy assets, independently retaining cursor pixels.
///
/// All payloads are decoded and validated before any new handles are allocated. On success,
/// handles owned by `previous` are removed so replaced packages do not leave stale assets behind.
pub fn load_presentation_assets(
    presentation: &PresentationContent,
    images: &mut Assets<Image>,
    fonts: &mut Assets<Font>,
    previous: Option<&WidgetAssets>,
) -> Result<WidgetAssets, String> {
    let mut decoded_images = BTreeMap::new();
    let mut nine_slices = BTreeMap::new();
    for (id, image) in &presentation.images {
        let decoded = decode_image(id, &image.bytes, RenderAssetUsages::RENDER_WORLD)?;
        decoded_images.insert(id.clone(), decoded);
        if let Some(margins) = image.nine_slice {
            nine_slices.insert(id.clone(), margins);
        }
    }

    let cursor_declarations = cursor_declarations(presentation);
    let mut decoded_cursors = BTreeMap::new();
    for (id, hotspot) in cursor_declarations {
        let source = presentation
            .images
            .get(&id)
            .ok_or_else(|| format!("cursor references missing image asset {id:?}"))?;
        let decoded = decode_image(
            &id,
            &source.bytes,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )?;
        let decoded = normalize_cursor_image(decoded).ok_or_else(|| {
            format!("cursor image {id:?} cannot be normalized to CPU-backed RGBA8")
        })?;
        decoded_cursors.insert((id, hotspot), decoded);
    }

    let decoded_fonts = presentation
        .fonts
        .iter()
        .map(|(id, data)| decode_font(id, data).map(|font| (id.clone(), font)))
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    let loaded = WidgetAssets {
        revision: previous.map_or(1, |assets| assets.revision.wrapping_add(1)),
        source_lockfile_hash: previous
            .map_or_else(String::new, |assets| assets.source_lockfile_hash.clone()),
        images: decoded_images
            .into_iter()
            .map(|(id, image)| (id, images.add(image)))
            .collect(),
        cursor_images: decoded_cursors
            .into_iter()
            .map(|(id, image)| (id, images.add(image)))
            .collect(),
        fonts: decoded_fonts
            .into_iter()
            .map(|(id, font)| (id, fonts.add(font)))
            .collect(),
        nine_slices,
    };

    if let Some(previous) = previous {
        for handle in previous
            .images
            .values()
            .chain(previous.cursor_images.values())
        {
            images.remove(handle.id());
        }
        for handle in previous.fonts.values() {
            fonts.remove(handle.id());
        }
    }

    Ok(loaded)
}

fn normalize_cursor_image(image: Image) -> Option<Image> {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    if image.texture_descriptor.format == TextureFormat::Rgba8UnormSrgb {
        return Some(image);
    }
    if image.texture_descriptor.format == TextureFormat::Rgba16Unorm {
        let pixels = image.data.as_deref()?;
        let mut rgba8 = Vec::with_capacity(pixels.len() / 2);
        for channel in pixels.as_chunks::<2>().0 {
            let sample = u16::from_le_bytes([channel[0], channel[1]]);
            rgba8.push(((u32::from(sample) + 128) / 257) as u8);
        }
        let size = Extent3d {
            width: image.texture_descriptor.size.width,
            height: image.texture_descriptor.size.height,
            depth_or_array_layers: image.texture_descriptor.size.depth_or_array_layers,
        };
        return Some(Image::new(
            size,
            TextureDimension::D2,
            rgba8,
            TextureFormat::Rgba8UnormSrgb,
            image.asset_usage,
        ));
    }
    image.convert(TextureFormat::Rgba8UnormSrgb)
}

fn cursor_declarations(presentation: &PresentationContent) -> BTreeSet<(String, (u16, u16))> {
    let mut declarations = presentation
        .cursor_assets
        .values()
        .cloned()
        .chain(presentation.cursors.values().cloned())
        .collect::<BTreeSet<_>>();
    if let (Some(interface), Some(package)) =
        (&presentation.interface, &presentation.interface_package)
    {
        fn visit(
            widgets: &[turts_mod_api::Widget],
            package: &str,
            declarations: &mut BTreeSet<(String, (u16, u16))>,
        ) {
            for widget in widgets {
                if let turts_mod_api::Primitive::Cursor { cursors } = &widget.widget {
                    declarations.extend(
                        cursors
                            .values()
                            .map(|(image, hotspot)| (qualify(package, image), *hotspot)),
                    );
                }
                if let turts_mod_api::Primitive::Box { children, .. } = &widget.widget {
                    visit(children, package, declarations);
                }
                match &widget.widget {
                    turts_mod_api::Primitive::List { item, .. }
                    | turts_mod_api::Primitive::Grid { item, .. } => {
                        visit(std::slice::from_ref(item.as_ref()), package, declarations)
                    }
                    _ => {}
                }
            }
        }
        visit(&interface.root, package, &mut declarations);
        for page in &interface.guide {
            visit(&page.widgets, package, &mut declarations);
        }
    }
    declarations
}

fn qualify(package: &str, asset: &str) -> String {
    if asset.contains(':') {
        asset.to_owned()
    } else {
        format!("{package}:{asset}")
    }
}

/// Find a direct cursor primitive mapping, qualified in the archive that authored it.
pub fn primitive_cursor(
    presentation: &PresentationContent,
    context: &str,
) -> Option<(String, (u16, u16))> {
    fn visit(
        widgets: &[turts_mod_api::Widget],
        package: &str,
        context: &str,
    ) -> Option<(String, (u16, u16))> {
        for widget in widgets {
            if let turts_mod_api::Primitive::Cursor { cursors } = &widget.widget
                && let Some((image, hotspot)) = cursors.get(context)
            {
                return Some((qualify(package, image), *hotspot));
            }
            let found = match &widget.widget {
                turts_mod_api::Primitive::Box { children, .. } => visit(children, package, context),
                turts_mod_api::Primitive::List { item, .. }
                | turts_mod_api::Primitive::Grid { item, .. } => {
                    visit(std::slice::from_ref(item.as_ref()), package, context)
                }
                _ => None,
            };
            if found.is_some() {
                return found;
            }
        }
        None
    }
    let package = presentation.interface_package.as_deref()?;
    let interface = presentation.interface.as_ref()?;
    visit(&interface.root, package, context).or_else(|| {
        interface
            .guide
            .iter()
            .find_map(|page| visit(&page.widgets, package, context))
    })
}

fn decode_image(id: &str, bytes: &[u8], usage: RenderAssetUsages) -> Result<Image, String> {
    // Package image declarations currently use PNG payloads. Bevy performs the complete PNG
    // decode, including CRC and pixel-data validation, from the accepted bytes.
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(format!("image asset {id:?} is not a PNG"));
    }
    Image::from_buffer(
        bytes,
        bevy::image::ImageType::Format(bevy::image::ImageFormat::Png),
        bevy::image::CompressedImageFormats::empty(),
        true,
        bevy::image::ImageSampler::Default,
        usage,
    )
    .map_err(|error| format!("image asset {id:?} could not be decoded: {error}"))
}

fn decode_font(id: &str, bytes: &[u8]) -> Result<Font, String> {
    if bytes.is_empty() {
        return Err(format!("font asset {id:?} is empty"));
    }

    // Registering the bytes with Bevy's font engine parses the font tables. Font::from_bytes
    // alone only wraps bytes, so it is not sufficient validation for accepted package data.
    let mut font_cx = FontCx::default();
    let font = Font::from_bytes(bytes.to_vec());
    if font_cx
        .collection
        .register_fonts(font.data.clone(), None)
        .is_empty()
    {
        return Err(format!("font asset {id:?} contains no parseable font face"));
    }
    Ok(font)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        acquire::{Acquired, PresentationImage},
        content::ClientContent,
    };

    const ONE_PIXEL_PNG: &[u8] =
        include_bytes!("../../../../mods/muster/assets/images/cursor_move.png");
    const RGBA16_PNG: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 16,
        6, 0, 0, 0, 79, 133, 24, 202, 0, 0, 0, 17, 73, 68, 65, 84, 120, 156, 99, 16, 96, 80, 96,
        48, 96, 248, 255, 31, 0, 5, 6, 2, 95, 59, 92, 14, 122, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66,
        96, 130,
    ];

    fn presentation() -> PresentationContent {
        let mut presentation = PresentationContent::default();
        presentation.images.insert(
            "muster:icon".into(),
            PresentationImage {
                bytes: ONE_PIXEL_PNG.to_vec(),
                nine_slice: Some((1, 2, 3, 4)),
            },
        );
        presentation
    }

    fn acquired(presentation: PresentationContent) -> AcquiredPackages {
        AcquiredPackages(Acquired {
            lockfile_hash: "test-lock".into(),
            visuals: Default::default(),
            map_size: (1.0, 1.0),
            unit_names: Default::default(),
            content: ClientContent::default(),
            presentation,
        })
    }

    #[test]
    fn decoded_png_retains_rgba_dimensions_usage_and_nine_slice() {
        let mut source = presentation();
        source
            .cursor_assets
            .insert("muster:cursor_move".into(), ("muster:icon".into(), (0, 0)));
        let mut images = Assets::<Image>::default();
        let mut fonts = Assets::<Font>::default();
        let assets = load_presentation_assets(&source, &mut images, &mut fonts, None).unwrap();

        let image = images
            .get(assets.image("muster:icon").unwrap().id())
            .unwrap();
        assert_eq!(image.texture_descriptor.size.width, 32);
        assert_eq!(image.texture_descriptor.size.height, 32);
        assert_eq!(image.data.as_deref().unwrap().len(), 32 * 32 * 4);
        assert_eq!(image.asset_usage, RenderAssetUsages::RENDER_WORLD);
        assert_eq!(assets.nine_slice("muster:icon"), Some((1, 2, 3, 4)));
    }

    #[test]
    fn cursor_image_is_separately_decoded_with_main_world_retention() {
        let mut source = presentation();
        source
            .cursor_assets
            .insert("muster:cursor_move".into(), ("muster:icon".into(), (0, 0)));
        let mut images = Assets::<Image>::default();
        let mut fonts = Assets::<Font>::default();
        let assets = load_presentation_assets(&source, &mut images, &mut fonts, None).unwrap();

        let image_handle = assets.image("muster:icon").unwrap();
        let cursor_handle = assets.cursor_image_for("muster:icon", (0, 0)).unwrap();
        assert_ne!(image_handle.id(), cursor_handle.id());
        assert_eq!(
            images.get(image_handle.id()).unwrap().asset_usage,
            RenderAssetUsages::RENDER_WORLD
        );
        assert_eq!(
            images.get(cursor_handle.id()).unwrap().asset_usage,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
        );
    }

    #[test]
    fn cursor_handles_are_unique_and_stable_per_image_and_hotspot() {
        let mut source = presentation();
        source
            .cursor_assets
            .insert("muster:cursor_first".into(), ("muster:icon".into(), (0, 0)));
        source.cursor_assets.insert(
            "muster:cursor_second".into(),
            ("muster:icon".into(), (3, 4)),
        );
        let mut images = Assets::<Image>::default();
        let mut fonts = Assets::<Font>::default();
        let first = load_presentation_assets(&source, &mut images, &mut fonts, None).unwrap();
        let first_handle = first.cursor_image_for("muster:icon", (0, 0)).unwrap();
        let second_handle = first.cursor_image_for("muster:icon", (3, 4)).unwrap();
        assert_ne!(
            first_handle.id(),
            second_handle.id(),
            "Bevy/Winit cursor cache must distinguish hotspot variants by image handle"
        );
        assert_eq!(
            first_handle.id(),
            first.cursor_image_for("muster:icon", (0, 0)).unwrap().id(),
            "the same authored pair is stable across frames"
        );

        let replacement =
            load_presentation_assets(&source, &mut images, &mut fonts, Some(&first)).unwrap();
        assert_ne!(
            first_handle.id(),
            replacement
                .cursor_image_for("muster:icon", (0, 0))
                .unwrap()
                .id(),
            "package replacement refreshes owned handles"
        );
        assert_ne!(
            second_handle.id(),
            replacement
                .cursor_image_for("muster:icon", (3, 4))
                .unwrap()
                .id()
        );
    }

    #[test]
    fn nested_cursor_primitive_images_receive_cpu_retained_handles() {
        let mut source = presentation();
        let mut cursors = BTreeMap::new();
        cursors.insert("default".into(), ("cursor_move".into(), (3, 4)));
        let cursor_widget = turts_mod_api::Widget {
            id: "cursor-map".into(),
            at: None,
            size: turts_mod_api::Size {
                w: turts_mod_api::Dimension::Fill,
                h: turts_mod_api::Dimension::Fill,
            },
            layer: 0,
            visible: Vec::new(),
            tooltip: None,
            widget: turts_mod_api::Primitive::Cursor { cursors },
        };
        source.interface =
            Some(ron::from_str(include_str!("../../../../mods/muster/interface/hud.ron")).unwrap());
        source.interface_package = Some("muster".into());
        source.interface.as_mut().unwrap().root.push(cursor_widget);
        source.images.insert(
            "muster:cursor_move".into(),
            PresentationImage {
                bytes: ONE_PIXEL_PNG.to_vec(),
                nine_slice: None,
            },
        );
        assert_eq!(
            primitive_cursor(&source, "default"),
            Some(("muster:cursor_move".into(), (3, 4)))
        );

        let mut images = Assets::<Image>::default();
        let mut fonts = Assets::<Font>::default();
        let assets = load_presentation_assets(&source, &mut images, &mut fonts, None).unwrap();
        let cursor = images
            .get(
                assets
                    .cursor_image_for("muster:cursor_move", (3, 4))
                    .unwrap()
                    .id(),
            )
            .unwrap();
        assert!(
            cursor.data.is_some(),
            "primitive-only mappings retain CPU-readable pixels"
        );
        assert_eq!(
            cursor.asset_usage,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
        );
    }

    #[test]
    fn cursor_only_decode_normalizes_16_bit_png_to_cpu_cursor_rgba8() {
        let mut source = PresentationContent::default();
        source.images.insert(
            "muster:wide".into(),
            PresentationImage {
                bytes: RGBA16_PNG.to_vec(),
                nine_slice: None,
            },
        );
        source
            .cursor_assets
            .insert("muster:cursor_wide".into(), ("muster:wide".into(), (0, 0)));
        let mut images = Assets::<Image>::default();
        let mut fonts = Assets::<Font>::default();
        let assets = load_presentation_assets(&source, &mut images, &mut fonts, None).unwrap();
        assert_eq!(
            images
                .get(assets.image("muster:wide").unwrap().id())
                .unwrap()
                .texture_descriptor
                .format,
            bevy::render::render_resource::TextureFormat::Rgba16Unorm,
        );
        let cursor = images
            .get(assets.cursor_image_for("muster:wide", (0, 0)).unwrap().id())
            .unwrap();
        assert_eq!(
            cursor.texture_descriptor.format,
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb
        );
        assert_eq!(cursor.data.as_deref().unwrap().len(), 4);
        assert_eq!(
            cursor.asset_usage,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD
        );
    }

    #[test]
    fn package_qualified_ids_keep_same_local_name_separate() {
        let mut source = presentation();
        source.images.insert(
            "other:icon".into(),
            PresentationImage {
                bytes: ONE_PIXEL_PNG.to_vec(),
                nine_slice: Some((5, 6, 7, 8)),
            },
        );
        let mut images = Assets::<Image>::default();
        let mut fonts = Assets::<Font>::default();
        let assets = load_presentation_assets(&source, &mut images, &mut fonts, None).unwrap();

        assert_ne!(
            assets.image("muster:icon").unwrap().id(),
            assets.image("other:icon").unwrap().id()
        );
        assert_eq!(assets.nine_slice("muster:icon"), Some((1, 2, 3, 4)));
        assert_eq!(assets.nine_slice("other:icon"), Some((5, 6, 7, 8)));
        assert!(assets.image("icon").is_none());
    }

    #[test]
    fn font_engine_accepts_real_font_and_rejects_invalid_bytes() {
        let font = include_bytes!("../../../../mods/muster/assets/fonts/Roboto-Regular.ttf");
        assert!(decode_font("muster:font", font).is_ok());
        assert!(decode_font("muster:bad", b"not a font").is_err());
    }

    #[test]
    fn failed_replacement_keeps_previous_assets_live() {
        let source = presentation();
        let mut images = Assets::<Image>::default();
        let mut fonts = Assets::<Font>::default();
        let old = load_presentation_assets(&source, &mut images, &mut fonts, None).unwrap();
        let old_handle = old.image("muster:icon").unwrap();
        let invalid = PresentationContent::default();
        let mut invalid = invalid;
        invalid.images.insert(
            "muster:broken".into(),
            PresentationImage {
                bytes: b"not an image".to_vec(),
                nine_slice: None,
            },
        );

        assert!(load_presentation_assets(&invalid, &mut images, &mut fonts, Some(&old)).is_err());
        assert!(images.get(old_handle.id()).is_some());
    }

    #[test]
    fn successful_replacement_removes_stale_handles_and_metadata() {
        let source = presentation();
        let mut images = Assets::<Image>::default();
        let mut fonts = Assets::<Font>::default();
        let old = load_presentation_assets(&source, &mut images, &mut fonts, None).unwrap();
        let stale = old.image("muster:icon").unwrap();
        let mut replacement = PresentationContent::default();
        replacement.images.insert(
            "other:new_icon".into(),
            PresentationImage {
                bytes: ONE_PIXEL_PNG.to_vec(),
                nine_slice: None,
            },
        );

        let loaded =
            load_presentation_assets(&replacement, &mut images, &mut fonts, Some(&old)).unwrap();
        assert!(images.get(stale.id()).is_none());
        assert!(loaded.image("muster:icon").is_none());
        assert!(loaded.nine_slice("muster:icon").is_none());
        assert!(loaded.image("other:new_icon").is_some());
    }

    #[test]
    fn failed_asset_revision_cannot_be_used_with_a_different_accepted_package_set() {
        let mut presentation = PresentationContent::default();
        presentation.images.insert(
            "new:icon".into(),
            PresentationImage {
                bytes: ONE_PIXEL_PNG.to_vec(),
                nine_slice: None,
            },
        );
        let old = WidgetAssets {
            source_lockfile_hash: "old-lock".into(),
            ..Default::default()
        };
        assert!(!old.matches_accepted_packages("new-lock", &presentation));
        assert!(old.matches_accepted_packages("old-lock", &presentation));
        assert!(old.matches_accepted_packages("new-lock", &PresentationContent::default()));
    }

    #[test]
    fn system_reuses_handles_until_accepted_packages_are_replaced() {
        let mut app = App::new();
        app.insert_resource(acquired(presentation()))
            .insert_resource(Assets::<Image>::default())
            .insert_resource(Assets::<Font>::default())
            .init_resource::<WidgetAssets>()
            .add_systems(Update, load_widget_assets);

        app.update();
        let first = app
            .world()
            .resource::<WidgetAssets>()
            .image("muster:icon")
            .unwrap();
        app.update();
        let second = app
            .world()
            .resource::<WidgetAssets>()
            .image("muster:icon")
            .unwrap();
        assert_eq!(first.id(), second.id());

        let mut replacement = PresentationContent::default();
        replacement.images.insert(
            "other:new_icon".into(),
            PresentationImage {
                bytes: ONE_PIXEL_PNG.to_vec(),
                nine_slice: None,
            },
        );
        app.world_mut()
            .resource_mut::<AcquiredPackages>()
            .0
            .presentation = replacement;
        app.update();

        assert!(
            app.world()
                .resource::<Assets<Image>>()
                .get(first.id())
                .is_none()
        );
        let loaded = app.world().resource::<WidgetAssets>();
        assert!(loaded.image("muster:icon").is_none());
        assert!(loaded.image("other:new_icon").is_some());
    }
}
