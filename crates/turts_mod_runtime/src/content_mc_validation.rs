//! Header-only API 0.2.3 mod validation. This module validates data and never loads Bevy.

use std::collections::{BTreeMap, BTreeSet};

use turts_mod_api::{
    AssetsDef, Behaviour, ButtonAction, ButtonIcon, ContentId, InterfaceDef, ModError, OrderDef,
    OrderRulesDef, PackageId, Primitive, Source, Value,
};

use crate::{
    archive::PackageArchive,
    asset_headers::{font_header, ogg_duration_seconds, png_header},
    content::error,
};

const MAX_DECODED_IMAGE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_IMAGE_DIMENSION: u32 = 2048;
const MAX_CURSOR_DIMENSION: u32 = 128;
const MAX_FONT_BYTES: usize = 8 * 1024 * 1024;
const MAX_SOUND_BYTES: usize = 32 * 1024 * 1024;
const MAX_SOUND_SECONDS: f64 = 30.0;
const MAX_TOTAL_SOUND_SECONDS: f64 = 300.0;

#[derive(Debug, Default)]
pub(crate) struct ParsedMcContent {
    pub(crate) orders: Vec<(String, OrderDef)>,
    pub(crate) order_rules: Vec<(String, OrderRulesDef)>,
    pub(crate) interfaces: Vec<(String, InterfaceDef)>,
    pub(crate) input_maps: Vec<(String, turts_mod_api::InputMapDef)>,
    pub(crate) assets: Vec<(String, AssetsDef)>,
    pub(crate) orders_present: bool,
    pub(crate) interface_present: bool,
    pub(crate) input_present: bool,
    pub(crate) assets_present: bool,
}

struct ImageInfo {
    width: u32,
    height: u32,
}

/// Parse and validate all API 0.2.3 content exported by one archive.
pub(crate) fn validate_mc(archive: &PackageArchive) -> Result<ParsedMcContent, ModError> {
    let orders = archive.orders()?;
    let order_rules = archive.order_rules()?;
    let interfaces = archive.interfaces()?;
    let input_maps = archive.input_maps()?;
    let assets = archive.assets()?;
    let parsed = ParsedMcContent {
        orders_present: !orders.is_empty() || !order_rules.is_empty(),
        interface_present: !interfaces.is_empty(),
        input_present: !input_maps.is_empty(),
        assets_present: !assets.is_empty(),
        orders,
        order_rules,
        interfaces,
        input_maps,
        assets,
    };
    let package = PackageId(archive.manifest.id.clone());
    let mut images = BTreeMap::<String, ImageInfo>::new();
    let mut cursors = BTreeMap::<String, String>::new();
    let mut fonts = BTreeSet::new();
    let mut sounds = BTreeSet::new();
    let mut decoded_image_bytes = 0u64;
    let mut total_sound_seconds = 0.0f64;
    let mut total_font_bytes = 0usize;
    let mut total_sound_bytes = 0usize;

    // Collect every image before resolving cursors so cross-file forward references work.
    for (file, assets) in &parsed.assets {
        for (index, image) in assets.images.iter().enumerate() {
            if images.contains_key(&image.id) {
                return Err(error(
                    &package,
                    file,
                    &format!("images[{index}].id"),
                    format!("duplicate image id `{}`", image.id),
                ));
            }
            let bytes = asset_file(
                archive,
                &package,
                file,
                &format!("images[{index}].path"),
                &image.path,
            )?;
            let header = png_header(bytes).ok_or_else(|| {
                error(
                    &package,
                    file,
                    &format!("images[{index}].path"),
                    "file is not a PNG with an IHDR header",
                )
            })?;
            check_dimensions(
                &package,
                file,
                &format!("images[{index}].path"),
                header.width,
                header.height,
            )?;
            let decoded = decoded_bytes(header.width, header.height).ok_or_else(|| {
                error(
                    &package,
                    file,
                    &format!("images[{index}].path"),
                    "decoded image size overflows",
                )
            })?;
            decoded_image_bytes = decoded_image_bytes.checked_add(decoded).ok_or_else(|| {
                error(
                    &package,
                    file,
                    "images",
                    "decoded image byte total overflows",
                )
            })?;
            images.insert(
                image.id.clone(),
                ImageInfo {
                    width: header.width,
                    height: header.height,
                },
            );
        }
    }

    for (file, assets) in &parsed.assets {
        for (index, font) in assets.fonts.iter().enumerate() {
            if !fonts.insert(font.id.as_str()) {
                return Err(error(
                    &package,
                    file,
                    &format!("fonts[{index}].id"),
                    format!("duplicate font id `{}`", font.id),
                ));
            }
            let bytes = asset_file(
                archive,
                &package,
                file,
                &format!("fonts[{index}].path"),
                &font.path,
            )?;
            if bytes.len() > MAX_FONT_BYTES {
                return Err(error(
                    &package,
                    file,
                    &format!("fonts[{index}].path"),
                    format!(
                        "font file is {} bytes; maximum is {MAX_FONT_BYTES} bytes",
                        bytes.len()
                    ),
                ));
            }
            total_font_bytes = total_font_bytes.saturating_add(bytes.len());
            if !font_header(bytes) {
                return Err(error(
                    &package,
                    file,
                    &format!("fonts[{index}].path"),
                    "font header must be TTF or OTF",
                ));
            }
        }
        for (index, sound) in assets.sounds.iter().enumerate() {
            if !sounds.insert(sound.id.as_str()) {
                return Err(error(
                    &package,
                    file,
                    &format!("sounds[{index}].id"),
                    format!("duplicate sound id `{}`", sound.id),
                ));
            }
            let bytes = asset_file(
                archive,
                &package,
                file,
                &format!("sounds[{index}].path"),
                &sound.path,
            )?;
            if bytes.len() > MAX_SOUND_BYTES {
                return Err(error(
                    &package,
                    file,
                    &format!("sounds[{index}].path"),
                    format!(
                        "sound file is {} bytes; maximum is {MAX_SOUND_BYTES} bytes",
                        bytes.len()
                    ),
                ));
            }
            total_sound_bytes = total_sound_bytes.saturating_add(bytes.len());
            let duration = ogg_duration_seconds(bytes).ok_or_else(|| error(&package, file, &format!("sounds[{index}].path"), "sound file must contain a valid OGG Vorbis identification header and final granule"))?;
            if duration > MAX_SOUND_SECONDS {
                return Err(error(
                    &package,
                    file,
                    &format!("sounds[{index}].path"),
                    format!(
                        "sound is {} seconds; maximum is 30 seconds",
                        whole_or_fraction(duration)
                    ),
                ));
            }
            total_sound_seconds += duration;
        }
        for (index, cursor) in assets.cursors.iter().enumerate() {
            if cursors
                .insert(cursor.id.clone(), cursor.image.clone())
                .is_some()
            {
                return Err(error(
                    &package,
                    file,
                    &format!("cursors[{index}].id"),
                    format!("duplicate cursor id `{}`", cursor.id),
                ));
            }
            let image = images.get(&cursor.image).ok_or_else(|| {
                error(
                    &package,
                    file,
                    &format!("cursors[{index}].image"),
                    format!("unknown image asset `{}`", cursor.image),
                )
            })?;
            if image.width > MAX_CURSOR_DIMENSION || image.height > MAX_CURSOR_DIMENSION {
                return Err(error(
                    &package,
                    file,
                    &format!("cursors[{index}].image"),
                    format!(
                        "cursor image is {} x {} px; maximum is 128 x 128 px",
                        image.width, image.height
                    ),
                ));
            }
            if u32::from(cursor.hotspot.0) >= image.width
                || u32::from(cursor.hotspot.1) >= image.height
            {
                return Err(error(
                    &package,
                    file,
                    &format!("cursors[{index}].hotspot"),
                    format!(
                        "hotspot ({}, {}) must be inside the {} x {} px image",
                        cursor.hotspot.0, cursor.hotspot.1, image.width, image.height
                    ),
                ));
            }
        }
    }
    if decoded_image_bytes > MAX_DECODED_IMAGE_BYTES {
        let file = parsed
            .assets
            .first()
            .map_or("manifest.ron", |(file, _)| file);
        return Err(error(
            &package,
            file,
            "images",
            format!(
                "decoded image bytes {decoded_image_bytes} exceed the {MAX_DECODED_IMAGE_BYTES} byte package limit"
            ),
        ));
    }
    if total_sound_seconds > MAX_TOTAL_SOUND_SECONDS {
        let file = parsed
            .assets
            .first()
            .map_or("manifest.ron", |(file, _)| file);
        return Err(error(
            &package,
            file,
            "sounds",
            format!(
                "total duration {} seconds exceeds the 300 second package limit",
                whole_or_fraction(total_sound_seconds)
            ),
        ));
    }
    if total_font_bytes > MAX_FONT_BYTES {
        let file = parsed
            .assets
            .first()
            .map_or("manifest.ron", |(file, _)| file);
        return Err(error(
            &package,
            file,
            "fonts",
            format!(
                "total font bytes {total_font_bytes} exceed the {MAX_FONT_BYTES} byte package limit"
            ),
        ));
    }
    if total_sound_bytes > MAX_SOUND_BYTES {
        let file = parsed
            .assets
            .first()
            .map_or("manifest.ron", |(file, _)| file);
        return Err(error(
            &package,
            file,
            "sounds",
            format!(
                "total sound bytes {total_sound_bytes} exceed the {MAX_SOUND_BYTES} byte package limit"
            ),
        ));
    }

    let mut actions = BTreeSet::new();
    for (file, input) in &parsed.input_maps {
        for (index, action) in input.actions.iter().enumerate() {
            if !actions.insert(action.name.as_str()) {
                return Err(error(
                    &package,
                    file,
                    &format!("actions[{index}].name"),
                    format!("duplicate action `{}`", action.name),
                ));
            }
            for (binding_index, binding) in action.default.iter().enumerate() {
                validate_binding(
                    &package,
                    file,
                    &format!("actions[{index}].default[{binding_index}]"),
                    binding,
                )?;
            }
        }
        for (index, engine_override) in input.engine_overrides.iter().enumerate() {
            if !is_engine_action(&engine_override.name) {
                return Err(error(
                    &package,
                    file,
                    &format!("engine_overrides[{index}].name"),
                    format!("unknown engine action `{}`", engine_override.name),
                ));
            }
            for (binding_index, binding) in engine_override.bindings.iter().enumerate() {
                validate_binding(
                    &package,
                    file,
                    &format!("engine_overrides[{index}].bindings[{binding_index}]"),
                    binding,
                )?;
            }
        }
    }

    let unit_ids: BTreeSet<ContentId> = archive
        .units()?
        .into_iter()
        .map(|(_, unit)| ContentId::new(&package, &unit.id))
        .collect();
    let mut other_content_ids = unit_ids.clone();
    for (_, resource) in archive.resources()? {
        other_content_ids.insert(ContentId::new(&package, &resource.id));
    }
    for (_, node) in archive.nodes()? {
        other_content_ids.insert(ContentId::new(&package, &node.id));
    }
    for (_, production) in archive.production()? {
        other_content_ids.insert(ContentId::new(&package, &production.id));
    }
    for (_, recipe) in archive.recipes()? {
        other_content_ids.insert(ContentId::new(&package, &recipe.id));
    }
    for (_, technology) in archive.technologies()? {
        other_content_ids.insert(ContentId::new(&package, &technology.id));
    }
    let balance_ids: BTreeSet<ContentId> = archive
        .resources()?
        .into_iter()
        .filter(|(_, resource)| resource.kind == turts_mod_api::ResourceKind::Balance)
        .map(|(_, resource)| ContentId::new(&package, &resource.id))
        .collect();

    let mut order_ids = BTreeMap::new();
    for (file, order) in &parsed.orders {
        validate_local_id(&package, file, "id", &order.id)?;
        let id = ContentId::new(&package, &order.id);
        if other_content_ids.contains(&id) {
            return Err(error(
                &package,
                file,
                "id",
                format!("order id `{id}` conflicts with another content kind"),
            ));
        }
        if let Some((first_file, _)) = order_ids.insert(id.clone(), (file.as_str(), order)) {
            return Err(error(
                &package,
                file,
                "id",
                format!("order `{id}` is also defined in `{first_file}`"),
            ));
        }
        if !images.contains_key(&order.icon) {
            return Err(error(
                &package,
                file,
                "icon",
                format!("unknown image asset `{}`", order.icon),
            ));
        }
        if let Some(cursor) = &order.cursor
            && !cursors.contains_key(cursor)
        {
            return Err(error(
                &package,
                file,
                "cursor",
                format!("unknown cursor asset `{cursor}`"),
            ));
        }
        if !actions.contains(order.hotkey.as_str()) {
            return Err(error(
                &package,
                file,
                "hotkey",
                format!("unknown input action `{}`", order.hotkey),
            ));
        }
        validate_order_target(&package, file, order)?;
    }

    let mut card_cells = BTreeMap::new();
    for (file, order) in &parsed.orders {
        if let Some(cell) = order.card {
            for unit in &order.units {
                let unit_id = checked_unit_reference(&package, file, "units", unit, &unit_ids)?;
                let key = (unit_id, cell.row, cell.column);
                if let Some((other_order, _)) =
                    card_cells.insert(key.clone(), (order.id.as_str(), file.as_str()))
                {
                    return Err(error(
                        &package,
                        file,
                        "card",
                        format!(
                            "card cell ({}, {}) collides for unit `{}` with order `{}`",
                            cell.row,
                            cell.column,
                            key.0,
                            ContentId::new(&package, other_order)
                        ),
                    ));
                }
            }
        }
        for unit in &order.units {
            checked_unit_reference(&package, file, "units", unit, &unit_ids)?;
        }
    }
    for (file, rules) in &parsed.order_rules {
        if rules.queue_depth == 0 {
            return Err(error(
                &package,
                file,
                "queue_depth",
                "must be between 1 and 255, got 0",
            ));
        }
        for (index, smart) in rules.smart.iter().enumerate() {
            let reference = checked_reference(
                &package,
                file,
                &format!("smart[{index}].order"),
                &smart.order,
            )?;
            if reference.package == package && !order_ids.contains_key(&reference) {
                return Err(error(
                    &package,
                    file,
                    &format!("smart[{index}].order"),
                    format!("unknown order `{reference}`"),
                ));
            }
            if let Some((_, order)) = order_ids.get(&reference) {
                validate_smart_rule(&package, file, index, smart, order)?;
            }
        }
    }

    let image_ids: BTreeSet<&str> = images.keys().map(String::as_str).collect();
    let font_ids: BTreeSet<&str> = fonts;
    let sound_ids: BTreeSet<&str> = sounds;
    let mut interface_ids = BTreeSet::new();
    for (file, interface) in &parsed.interfaces {
        validate_local_id(&package, file, "id", &interface.id)?;
        if !interface_ids.insert(interface.id.as_str()) {
            return Err(error(
                &package,
                file,
                "id",
                format!("duplicate interface id `{}`", interface.id),
            ));
        }
        for (context, cursor) in &interface.cursors {
            if !cursors.contains_key(cursor) {
                return Err(error(
                    &package,
                    file,
                    &format!("cursors.{context}"),
                    format!("unknown cursor asset `{cursor}`"),
                ));
            }
        }
        if let Some(font) = &interface.tooltip_style.font
            && !font_ids.contains(font.as_str())
        {
            return Err(error(
                &package,
                file,
                "tooltip_style.font",
                format!("unknown font asset `{font}`"),
            ));
        }
        if let Some(font) = &interface.world.tooltip.font
            && !font_ids.contains(font.as_str())
        {
            return Err(error(
                &package,
                file,
                "world.tooltip.font",
                format!("unknown font asset `{font}`"),
            ));
        }
        if interface.world.health_bar.width == 0 || interface.world.health_bar.height == 0 {
            return Err(error(
                &package,
                file,
                "world.health_bar",
                "width and height must be greater than zero",
            ));
        }
        let mut widget_ids = BTreeSet::new();
        for (index, widget) in interface.root.iter().enumerate() {
            validate_widget(
                &package,
                file,
                &format!("root[{index}]"),
                widget,
                &mut widget_ids,
                &image_ids,
                &font_ids,
                &sound_ids,
                &order_ids,
                &actions,
                &unit_ids,
                &balance_ids,
            )?;
        }
        let mut alert_ids = BTreeSet::new();
        for (index, alert) in interface.alerts.iter().enumerate() {
            validate_local_id(&package, file, &format!("alerts[{index}].id"), &alert.id)?;
            if !alert_ids.insert(alert.id.as_str()) {
                return Err(error(
                    &package,
                    file,
                    &format!("alerts[{index}].id"),
                    format!("duplicate alert id `{}`", alert.id),
                ));
            }
            if !alert.cooldown.is_finite() || alert.cooldown < 1.0 {
                return Err(error(
                    &package,
                    file,
                    &format!("alerts[{index}].cooldown"),
                    "must be finite and at least 1 second",
                ));
            }
            if !alert.coalesce_radius.is_finite() || alert.coalesce_radius < 0.0 {
                return Err(error(
                    &package,
                    file,
                    &format!("alerts[{index}].coalesce_radius"),
                    "must be finite and at least 0",
                ));
            }
            if let Some(seconds) = alert.trigger.waiting_seconds()
                && (!seconds.is_finite() || seconds <= 0.0)
            {
                return Err(error(
                    &package,
                    file,
                    &format!("alerts[{index}].trigger"),
                    "waiting duration must be finite and greater than 0 seconds",
                ));
            }
            if let Some(sound) = &alert.sound
                && !sound_ids.contains(sound.as_str())
            {
                return Err(error(
                    &package,
                    file,
                    &format!("alerts[{index}].sound"),
                    format!("unknown sound asset `{sound}`"),
                ));
            }
        }
        let mut guide_ids = BTreeSet::new();
        for (page_index, page) in interface.guide.iter().enumerate() {
            validate_local_id(&package, file, &format!("guide[{page_index}].id"), &page.id)?;
            if !guide_ids.insert(page.id.as_str()) {
                return Err(error(
                    &package,
                    file,
                    &format!("guide[{page_index}].id"),
                    format!("duplicate guide page id `{}`", page.id),
                ));
            }
            let mut page_widget_ids = BTreeSet::new();
            for (widget_index, widget) in page.widgets.iter().enumerate() {
                validate_widget(
                    &package,
                    file,
                    &format!("guide[{page_index}].widgets[{widget_index}]"),
                    widget,
                    &mut page_widget_ids,
                    &image_ids,
                    &font_ids,
                    &sound_ids,
                    &order_ids,
                    &actions,
                    &unit_ids,
                    &balance_ids,
                )?;
            }
        }
        for (id, presentation) in &interface.units {
            checked_unit_reference(&package, file, "units", id, &unit_ids)?;
            for (kind, asset) in [
                ("icon", &presentation.icon),
                ("portrait", &presentation.portrait),
            ] {
                if let Some(asset) = asset
                    && !image_ids.contains(asset.as_str())
                {
                    return Err(error(
                        &package,
                        file,
                        &format!("units.{id}.{kind}"),
                        format!("unknown image asset `{asset}`"),
                    ));
                }
            }
            for (kind, assets) in &presentation.responses {
                for asset in assets {
                    if !sound_ids.contains(asset.as_str()) {
                        return Err(error(
                            &package,
                            file,
                            &format!("units.{id}.responses.{kind}"),
                            format!("unknown sound asset `{asset}`"),
                        ));
                    }
                }
            }
        }
        for (index, unit) in interface.selection_priority.iter().enumerate() {
            checked_unit_reference(
                &package,
                file,
                &format!("selection_priority[{index}]"),
                unit,
                &unit_ids,
            )?;
        }
    }
    Ok(parsed)
}

/// Enforce aggregate decoded-image, font, and sound limits for the complete locked set.
pub fn check_locked_set_caps(archives: &[&PackageArchive]) -> Result<(), ModError> {
    let mut decoded_total = 0u64;
    let mut font_total = 0usize;
    let mut sound_bytes_total = 0usize;
    let mut sound_seconds_total = 0.0f64;
    let mut first = None;
    for archive in archives {
        let package = PackageId(archive.manifest.id.clone());
        let parsed = validate_mc(archive)?;
        for (file, assets) in parsed.assets {
            for (index, image) in assets.images.iter().enumerate() {
                let bytes = asset_file(
                    archive,
                    &package,
                    &file,
                    &format!("images[{index}].path"),
                    &image.path,
                )?;
                let header = png_header(bytes).ok_or_else(|| {
                    error(
                        &package,
                        &file,
                        &format!("images[{index}].path"),
                        "file is not a PNG with an IHDR header",
                    )
                })?;
                check_dimensions(
                    &package,
                    &file,
                    &format!("images[{index}].path"),
                    header.width,
                    header.height,
                )?;
                let decoded = decoded_bytes(header.width, header.height).ok_or_else(|| {
                    error(
                        &package,
                        &file,
                        &format!("images[{index}].path"),
                        "decoded image size overflows",
                    )
                })?;
                decoded_total = decoded_total.checked_add(decoded).ok_or_else(|| {
                    error(
                        &package,
                        &file,
                        "images",
                        "locked-set decoded image byte total overflows",
                    )
                })?;
                first.get_or_insert((package.clone(), file.clone()));
            }
            for (index, font) in assets.fonts.iter().enumerate() {
                let bytes = asset_file(
                    archive,
                    &package,
                    &file,
                    &format!("fonts[{index}].path"),
                    &font.path,
                )?;
                font_total = font_total.saturating_add(bytes.len());
                first.get_or_insert((package.clone(), file.clone()));
            }
            for (index, sound) in assets.sounds.iter().enumerate() {
                let bytes = asset_file(
                    archive,
                    &package,
                    &file,
                    &format!("sounds[{index}].path"),
                    &sound.path,
                )?;
                sound_bytes_total = sound_bytes_total.saturating_add(bytes.len());
                sound_seconds_total += ogg_duration_seconds(bytes).unwrap_or(0.0);
                first.get_or_insert((package.clone(), file.clone()));
            }
        }
    }
    let (package, file) =
        first.unwrap_or_else(|| (PackageId(String::new()), "manifest.ron".into()));
    if decoded_total > MAX_DECODED_IMAGE_BYTES {
        return Err(error(
            &package,
            &file,
            "images",
            format!(
                "decoded image bytes {decoded_total} exceed the {MAX_DECODED_IMAGE_BYTES} byte locked-set limit"
            ),
        ));
    }
    if font_total > MAX_FONT_BYTES {
        return Err(error(
            &package,
            &file,
            "fonts",
            format!(
                "total font bytes {font_total} exceed the {MAX_FONT_BYTES} byte locked-set limit"
            ),
        ));
    }
    if sound_bytes_total > MAX_SOUND_BYTES {
        return Err(error(
            &package,
            &file,
            "sounds",
            format!(
                "total sound bytes {sound_bytes_total} exceed the {MAX_SOUND_BYTES} byte locked-set limit"
            ),
        ));
    }
    if sound_seconds_total > MAX_TOTAL_SOUND_SECONDS {
        return Err(error(
            &package,
            &file,
            "sounds",
            format!(
                "total duration {} seconds exceeds the 300 second locked-set limit",
                whole_or_fraction(sound_seconds_total)
            ),
        ));
    }
    Ok(())
}

/// Human-readable advisory warnings; warnings never make package validation fail.
pub fn layout_warnings(archive: &PackageArchive) -> Result<Vec<String>, ModError> {
    let package = PackageId(archive.manifest.id.clone());
    let mut warnings = Vec::new();
    for (file, interface) in archive.interfaces()? {
        for (index, widget) in interface.root.iter().enumerate() {
            collect_layout_warnings(
                &package,
                &file,
                &format!("root[{index}]"),
                widget,
                &mut warnings,
            );
        }
    }
    Ok(warnings)
}

fn collect_layout_warnings(
    package: &PackageId,
    file: &str,
    path: &str,
    widget: &turts_mod_api::Widget,
    warnings: &mut Vec<String>,
) {
    match &widget.widget {
        Primitive::Text { size, .. } if *size < 11 => warnings.push(
            error(
                package,
                file,
                &format!("{path}.widget.size"),
                format!("text is {size} px; recommended minimum is 11 px"),
            )
            .to_string(),
        ),
        Primitive::Button { .. } => {
            if let (Some(width), Some(height)) = (px(widget.size.w), px(widget.size.h))
                && (width < 24.0 || height < 24.0)
            {
                warnings.push(
                    error(
                        package,
                        file,
                        &format!("{path}.size"),
                        format!(
                            "button is {} x {} px; recommended minimum is 24 x 24 px",
                            whole_or_fraction(width.into()),
                            whole_or_fraction(height.into())
                        ),
                    )
                    .to_string(),
                );
            }
        }
        Primitive::Box { children, .. } => {
            for (index, child) in children.iter().enumerate() {
                collect_layout_warnings(
                    package,
                    file,
                    &format!("{path}.children[{index}]"),
                    child,
                    warnings,
                );
            }
        }
        Primitive::List { item, .. } | Primitive::Grid { item, .. } => {
            collect_layout_warnings(package, file, &format!("{path}.item"), item, warnings)
        }
        _ => {}
    }
}

fn validate_widget(
    package: &PackageId,
    file: &str,
    path: &str,
    widget: &turts_mod_api::Widget,
    ids: &mut BTreeSet<String>,
    images: &BTreeSet<&str>,
    fonts: &BTreeSet<&str>,
    sounds: &BTreeSet<&str>,
    orders: &BTreeMap<ContentId, (&str, &OrderDef)>,
    actions: &BTreeSet<&str>,
    unit_ids: &BTreeSet<ContentId>,
    balance_ids: &BTreeSet<ContentId>,
) -> Result<(), ModError> {
    if !ids.insert(widget.id.clone()) {
        return Err(error(
            package,
            file,
            &format!("{path}.id"),
            format!("duplicate widget id `{}`", widget.id),
        ));
    }
    if let Some(value) = widget.tooltip.as_ref() {
        validate_value(
            package,
            file,
            &format!("{path}.tooltip"),
            value,
            balance_ids,
        )?;
    }
    match &widget.widget {
        Primitive::Box { fill, children, .. } => {
            if let Some(fill) = fill {
                validate_fill(package, file, &format!("{path}.widget.fill"), fill, images)?;
            }
            for (index, child) in children.iter().enumerate() {
                validate_widget(
                    package,
                    file,
                    &format!("{path}.widget.children[{index}]"),
                    child,
                    ids,
                    images,
                    fonts,
                    sounds,
                    orders,
                    actions,
                    unit_ids,
                    balance_ids,
                )?;
            }
        }
        Primitive::Image { image, .. } => validate_icon(
            package,
            file,
            &format!("{path}.widget.image"),
            image,
            images,
        )?,
        Primitive::Text { text, font, .. } => {
            if let Some(font) = font
                && !fonts.contains(font.as_str())
            {
                return Err(error(
                    package,
                    file,
                    &format!("{path}.widget.font"),
                    format!("unknown font asset `{font}`"),
                ));
            }
            validate_text_value(
                package,
                file,
                &format!("{path}.widget.text"),
                text,
                balance_ids,
            )?;
        }
        Primitive::Bar {
            value,
            max,
            fill,
            background,
            ..
        } => {
            validate_value(
                package,
                file,
                &format!("{path}.widget.value"),
                value,
                balance_ids,
            )?;
            validate_value(
                package,
                file,
                &format!("{path}.widget.max"),
                max,
                balance_ids,
            )?;
            validate_fill(package, file, &format!("{path}.widget.fill"), fill, images)?;
            validate_fill(
                package,
                file,
                &format!("{path}.widget.background"),
                background,
                images,
            )?;
        }
        Primitive::Button {
            icon,
            label,
            action,
            ..
        } => {
            if let Some(icon) = icon {
                validate_icon(package, file, &format!("{path}.widget.icon"), icon, images)?;
            }
            if let Some(label) = label {
                validate_text_value(
                    package,
                    file,
                    &format!("{path}.widget.label"),
                    label,
                    balance_ids,
                )?;
            }
            match action {
                ButtonAction::Order(id) => {
                    let reference =
                        checked_reference(package, file, &format!("{path}.widget.action"), id)?;
                    if reference.package == *package && !orders.contains_key(&reference) {
                        return Err(error(
                            package,
                            file,
                            &format!("{path}.widget.action"),
                            format!("unknown order `{reference}`"),
                        ));
                    }
                }
                ButtonAction::Input(name)
                    if !actions.contains(name.as_str()) && !is_engine_action(name) =>
                {
                    return Err(error(
                        package,
                        file,
                        &format!("{path}.widget.action"),
                        format!("unknown input action `{name}`"),
                    ));
                }
                _ => {}
            }
        }
        Primitive::List { source, item, .. } | Primitive::Grid { source, item, .. } => {
            validate_source(
                package,
                file,
                &format!("{path}.widget.source"),
                source,
                balance_ids,
            )?;
            validate_widget(
                package,
                file,
                &format!("{path}.item"),
                item,
                &mut BTreeSet::new(),
                images,
                fonts,
                sounds,
                orders,
                actions,
                unit_ids,
                balance_ids,
            )?;
        }
        Primitive::Tooltip { value } => validate_value(
            package,
            file,
            &format!("{path}.widget.value"),
            value,
            balance_ids,
        )?,
        Primitive::Cursor { cursors } => {
            for (context, (image, _)) in cursors {
                if !images.contains(image.as_str()) {
                    return Err(error(
                        package,
                        file,
                        &format!("{path}.widget.cursors.{context}"),
                        format!("unknown image asset `{image}`"),
                    ));
                }
            }
        }
        Primitive::MinimapView { background, .. } => {
            if let Some(background) = background {
                validate_fill(
                    package,
                    file,
                    &format!("{path}.widget.background"),
                    background,
                    images,
                )?;
            }
        }
    }
    Ok(())
}

fn validate_text_value(
    package: &PackageId,
    file: &str,
    field: &str,
    value: &turts_mod_api::TextValue,
    balance_ids: &BTreeSet<ContentId>,
) -> Result<(), ModError> {
    match value {
        turts_mod_api::TextValue::Bind(value) => {
            validate_value(package, file, field, value, balance_ids)
        }
        turts_mod_api::TextValue::Format(_, values) => {
            for value in values {
                validate_value(package, file, field, value, balance_ids)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_value(
    package: &PackageId,
    file: &str,
    field: &str,
    value: &Value,
    balance_ids: &BTreeSet<ContentId>,
) -> Result<(), ModError> {
    match value {
        Value::Balance(id) => {
            let reference = checked_reference(package, file, field, id)?;
            if reference.package == *package && !balance_ids.contains(&reference) {
                return Err(error(
                    package,
                    file,
                    field,
                    format!("unknown Balance resource `{reference}`"),
                ));
            }
        }
        Value::Format(_, values) => {
            for value in values {
                validate_value(package, file, field, value, balance_ids)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_source(
    package: &PackageId,
    file: &str,
    field: &str,
    source: &Source,
    balance_ids: &BTreeSet<ContentId>,
) -> Result<(), ModError> {
    if let Source::Balance(id) = source {
        let reference = checked_reference(package, file, field, id)?;
        if reference.package == *package && !balance_ids.contains(&reference) {
            return Err(error(
                package,
                file,
                field,
                format!("unknown Balance resource `{reference}`"),
            ));
        }
    }
    Ok(())
}

fn validate_fill(
    package: &PackageId,
    file: &str,
    field: &str,
    fill: &turts_mod_api::Fill,
    images: &BTreeSet<&str>,
) -> Result<(), ModError> {
    if let turts_mod_api::Fill::Image { id, .. } = fill
        && !images.contains(id.as_str())
    {
        return Err(error(
            package,
            file,
            field,
            format!("unknown image asset `{id}`"),
        ));
    }
    Ok(())
}

fn validate_icon(
    package: &PackageId,
    file: &str,
    field: &str,
    icon: &ButtonIcon,
    images: &BTreeSet<&str>,
) -> Result<(), ModError> {
    if let ButtonIcon::Asset(id) = icon
        && !images.contains(id.as_str())
    {
        return Err(error(
            package,
            file,
            field,
            format!("unknown image asset `{id}`"),
        ));
    }
    Ok(())
}

fn validate_order_target(
    package: &PackageId,
    file: &str,
    order: &OrderDef,
) -> Result<(), ModError> {
    use Behaviour as B;
    use turts_mod_api::TargetKind as T;
    let allowed = match order.behaviour {
        B::Move => matches!(order.target, T::Point | T::Unit | T::PointOrUnit),
        B::Stop | B::Hold | B::Remove | B::SetEnabled(_) => order.target == T::None,
        B::Attack => matches!(order.target, T::Point | T::Unit | T::PointOrUnit),
        B::Gather => order.target == T::Node,
        B::Produce => order.target == T::Content,
        B::CancelProduction => order.target == T::Job,
    };
    if !allowed {
        let expected = match order.behaviour {
            B::CancelProduction => "Job",
            B::Produce => "Content",
            B::Gather => "Node",
            B::Stop | B::Hold | B::Remove | B::SetEnabled(_) => "None",
            B::Move | B::Attack => "Point, Unit or PointOrUnit",
        };
        return Err(error(
            package,
            file,
            "target",
            format!(
                "{:?} requires {expected}, got {:?}",
                order.behaviour, order.target
            ),
        ));
    }
    if order.behaviour == B::Attack
        && order.engage == Some(turts_mod_api::EngagePolicy::StopAndFire)
        && !matches!(order.target, T::Point | T::PointOrUnit)
    {
        return Err(error(
            package,
            file,
            "engage",
            "StopAndFire is only valid for a point target",
        ));
    }
    if !matches!(order.behaviour, B::Move | B::Hold | B::Attack) && order.fire.is_some() {
        return Err(error(
            package,
            file,
            "fire",
            "fire policy is only valid for Move, Hold or Attack",
        ));
    }
    if order.behaviour != B::Attack && order.engage.is_some() {
        return Err(error(
            package,
            file,
            "engage",
            "engage policy is only valid for Attack",
        ));
    }
    Ok(())
}

fn validate_smart_rule(
    package: &PackageId,
    file: &str,
    index: usize,
    smart: &turts_mod_api::SmartOrder,
    order: &OrderDef,
) -> Result<(), ModError> {
    use Behaviour as B;
    use turts_mod_api::{SmartTarget as S, TargetKind as T};
    let valid = match smart.target {
        S::Ground | S::OwnUnit | S::ForeignUnit | S::Node => {
            smart.needs == turts_mod_api::Capability::Movement
                && order.behaviour == B::Move
                && matches!(order.target, T::Point | T::Unit | T::PointOrUnit)
        }
        S::ForeignUnitWeaponTarget => {
            smart.needs == turts_mod_api::Capability::Weapon
                && order.behaviour == B::Attack
                && matches!(order.target, T::Unit | T::PointOrUnit)
        }
        S::ResourceNodeAccepted => {
            smart.needs == turts_mod_api::Capability::Gatherer
                && order.behaviour == B::Gather
                && order.target == T::Node
        }
        S::Content => {
            matches!(
                smart.needs,
                turts_mod_api::Capability::Producer | turts_mod_api::Capability::Research
            ) && order.behaviour == B::Produce
                && order.target == T::Content
        }
    };
    if !valid {
        return Err(error(
            package,
            file,
            &format!("smart[{index}]"),
            format!(
                "target {:?} with capability {:?} is incompatible with order `{}` ({:?}, {:?})",
                smart.target, smart.needs, order.id, order.behaviour, order.target
            ),
        ));
    }
    Ok(())
}

fn validate_binding(
    package: &PackageId,
    file: &str,
    field: &str,
    binding: &turts_mod_api::Binding,
) -> Result<(), ModError> {
    let turts_mod_api::Binding::Key { code, mods } = binding else {
        return Ok(());
    };
    if !turts_mod_api::KNOWN_KEY_CODES.contains(&code.as_str()) {
        return Err(error(
            package,
            file,
            &format!("{field}.code"),
            format!("unknown physical key code `{code}`"),
        ));
    }
    let reserved =
        turts_mod_api::RESERVED_BROWSER_KEYS
            .iter()
            .any(|(reserved_code, reserved_mods)| {
                *reserved_code == code && *reserved_mods == mods.as_slice()
            });
    if reserved {
        let chord = mods
            .iter()
            .map(|modifier| format!("{modifier:?}"))
            .chain(std::iter::once(code.clone()))
            .collect::<Vec<_>>()
            .join("+");
        return Err(error(
            package,
            file,
            field,
            format!("{chord} is reserved by the browser"),
        ));
    }
    Ok(())
}

fn checked_reference(
    package: &PackageId,
    file: &str,
    field: &str,
    value: &str,
) -> Result<ContentId, ModError> {
    ContentId::parse(value).map_err(|message| error(package, file, field, message))
}

fn checked_unit_reference(
    package: &PackageId,
    file: &str,
    field: &str,
    value: &str,
    unit_ids: &BTreeSet<ContentId>,
) -> Result<ContentId, ModError> {
    let reference = checked_reference(package, file, field, value)?;
    if reference.package == *package && !unit_ids.contains(&reference) {
        return Err(error(
            package,
            file,
            field,
            format!("unknown unit `{reference}`"),
        ));
    }
    Ok(reference)
}

fn is_engine_action(name: &str) -> bool {
    matches!(
        name,
        "select"
            | "box_select"
            | "command"
            | "add_modifier"
            | "queue_modifier"
            | "type_modifier"
            | "zoom_in"
            | "zoom_out"
            | "centre_selection"
            | "cancel"
            | "menu"
            | "toggle_guide"
    ) || name
        .strip_prefix("group_recall_")
        .or_else(|| name.strip_prefix("group_assign_"))
        .or_else(|| name.strip_prefix("group_add_"))
        .is_some_and(|suffix| matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"))
        || matches!(
            name,
            "camera_pan_up" | "camera_pan_down" | "camera_pan_left" | "camera_pan_right"
        )
}

fn validate_local_id(
    package: &PackageId,
    file: &str,
    field: &str,
    id: &str,
) -> Result<(), ModError> {
    if id.is_empty() || id.contains(':') {
        return Err(error(
            package,
            file,
            field,
            "must be a nonempty local id without `:`",
        ));
    }
    Ok(())
}

fn asset_file<'a>(
    archive: &'a PackageArchive,
    package: &PackageId,
    file: &str,
    field: &str,
    path: &str,
) -> Result<&'a [u8], ModError> {
    archive.file(path).ok_or_else(|| {
        error(
            package,
            file,
            field,
            format!("asset file `{path}` is missing"),
        )
    })
}

fn check_dimensions(
    package: &PackageId,
    file: &str,
    field: &str,
    width: u32,
    height: u32,
) -> Result<(), ModError> {
    if width > MAX_IMAGE_DIMENSION {
        return Err(error(
            package,
            file,
            field,
            format!("PNG width {width} exceeds the 2048 pixel limit"),
        ));
    }
    if height > MAX_IMAGE_DIMENSION {
        return Err(error(
            package,
            file,
            field,
            format!("PNG height {height} exceeds the 2048 pixel limit"),
        ));
    }
    Ok(())
}

fn decoded_bytes(width: u32, height: u32) -> Option<u64> {
    u64::from(width)
        .checked_mul(u64::from(height))?
        .checked_mul(4)
}

fn whole_or_fraction(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

fn px(dimension: turts_mod_api::Dimension) -> Option<f32> {
    if let turts_mod_api::Dimension::Px(value) = dimension {
        Some(value)
    } else {
        None
    }
}
