//! The world tooltip: the unit name and slot label over the hovered unit.

use super::*;

#[derive(Component)]
pub(crate) struct WorldTooltip(pub(crate) u32);

#[allow(clippy::too_many_arguments)]
pub(super) fn update_world_tooltip(
    commands: &mut Commands,
    hovered: Option<u32>,
    selected_node: Option<turts_net_protocol::NodeId>,
    started: Option<Instant>,
    now: Instant,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    presented: &HashMap<u32, &crate::present::PresentedUnit>,
    replica: &crate::replica::Replica,
    catalog: &turts_mod_api::ContentCatalog,
    assets: &VisualAssets,
    widget_assets: Option<&crate::widgets::assets::WidgetAssets>,
    labels: &mut Query<(Entity, &WorldTooltip, &mut Node, &mut Text, &mut TextFont)>,
) {
    let text = hovered
        .filter(|_| started.is_some_and(|time| now.duration_since(time).as_secs_f32() >= 0.5))
        .and_then(|id| {
            let shown = presented.get(&id)?;
            let unit = replica.units.get(&id)?;
            let name = ContentId::parse(&shown.content_id)
                .ok()
                .and_then(|content| catalog.units.get(&content))
                .map(|def| def.name.clone())?;
            let point = camera
                .world_to_viewport(camera_transform, world_to_scene(shown.pos) + Vec3::Y * 2.0)
                .ok()?;
            Some((id, tooltip_text(&name, unit.owner_slot), point))
        })
        .or_else(|| {
            let id = selected_node?;
            let node = replica.nodes.get(&id)?;
            let content = ContentId::parse(&node.content_id).ok()?;
            let name = &catalog.nodes.get(&content)?.name;
            let point = camera
                .world_to_viewport(camera_transform, world_to_scene(node.pos) + Vec3::Y * 2.0)
                .ok()?;
            Some((
                id.0,
                format!("{name} - {} remaining", node.remaining),
                point,
            ))
        });
    let font = assets
        .tooltip_font
        .as_deref()
        .and_then(|id| widget_assets.and_then(|assets| assets.font(id)))
        .unwrap_or_default();
    let mut matching_tooltip_exists = false;
    for (entity, tooltip, mut node, mut label, mut font_style) in labels.iter_mut() {
        if let Some((id, text, point)) = &text
            && tooltip.0 == *id
        {
            matching_tooltip_exists = true;
            *label = Text::new(text.clone());
            node.left = px(point.x + 12.0);
            node.top = px(point.y - 12.0);
            if font_style.font != FontSource::Handle(font.clone()) {
                font_style.font = FontSource::Handle(font.clone());
            }
            let size = f32::from(assets.tooltip_size);
            if !matches!(font_style.font_size, FontSize::Px(current) if current == size) {
                font_style.font_size = FontSize::Px(size);
            }
        } else {
            commands.entity(entity).despawn();
        }
    }
    if let Some((id, text, point)) = text
        && !matching_tooltip_exists
    {
        commands.spawn((
            WorldTooltip(id),
            Text::new(text),
            TextFont {
                font_size: FontSize::Px(f32::from(assets.tooltip_size)),
                font: FontSource::Handle(font),
                ..default()
            },
            TextColor(assets.tooltip_colour),
            BackgroundColor(assets.tooltip_background.with_alpha(0.92)),
            Node {
                position_type: PositionType::Absolute,
                left: px(point.x + 12.0),
                top: px(point.y - 12.0),
                padding: UiRect::axes(px(6), px(3)),
                ..default()
            },
            bevy::picking::Pickable::IGNORE,
        ));
    }
}

pub(super) fn tooltip_text(name: &str, owner_slot: u8) -> String {
    format!("{name} · Slot {}", owner_slot + 1)
}
