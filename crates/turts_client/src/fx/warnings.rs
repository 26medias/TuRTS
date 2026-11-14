//! An attack warning points at its place without capturing player input.
use super::*;
use bevy::window::PrimaryWindow;
#[derive(Resource, Default)]
pub struct WarningPulse {
    pub position: Option<Vec3>,
    pub started: f64,
    pub until: f64,
    entity: Option<Entity>,
}
/// Intersect a direction with the inset viewport edge.
pub fn edge_point(direction: Vec2, rect: Rect) -> Vec2 {
    let half = (rect.size() * 0.5 - Vec2::splat(10.)).max(Vec2::ONE);
    let direction = direction.try_normalize().unwrap_or(Vec2::Y);
    let scale =
        (half.x / direction.x.abs().max(0.0001)).min(half.y / direction.y.abs().max(0.0001));
    rect.center() + direction * scale
}
#[allow(clippy::too_many_arguments)]
pub fn update_warning_pulse(
    mut commands: Commands,
    time: Res<Time>,
    frame: Option<Res<crate::feedback::FrameFeedback>>,
    acquired: Option<Res<AcquiredPackages>>,
    mut pulse: ResMut<WarningPulse>,
    cameras: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut nodes: Query<(&mut Node, &mut BackgroundColor, &mut Visibility)>,
) {
    let now = time.elapsed_secs_f64();
    if let (Some(frame), Some(acquired)) = (frame, acquired)
        && let Some(interface) = acquired.0.presentation.interface.as_ref()
    {
        for alert in &frame.alerts {
            let rule = alert
                .get("id")
                .and_then(|id| id.as_str())
                .and_then(|id| interface.alerts.iter().find(|rule| rule.id == id));
            if !rule.is_some_and(|rule| {
                rule.offscreen_only
                    && matches!(rule.trigger, turts_mod_api::AlertTrigger::OwnUnitDamaged)
            }) {
                continue;
            }
            if let (Some(x), Some(y)) = (
                alert.pointer("/position/x").and_then(|x| x.as_f64()),
                alert.pointer("/position/y").and_then(|y| y.as_f64()),
            ) {
                pulse.position = Some(Vec3::new(x as f32, 0.8, y as f32));
                pulse.started = now;
                pulse.until = now + 2.;
                break;
            }
        }
    }
    let Some(position) = pulse.position else {
        return;
    };
    let Some((camera, transform)) = cameras.iter().find(|(camera, _)| camera.is_active) else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let viewport = camera.logical_viewport_rect().unwrap_or(Rect::from_corners(
        Vec2::ZERO,
        Vec2::new(window.width(), window.height()),
    ));
    let direction = camera
        .world_to_viewport(transform, position)
        .ok()
        .map(|p| p - viewport.center())
        .unwrap_or_else(|| {
            let delta = position - transform.translation();
            Vec2::new(
                delta.dot(transform.rotation() * Vec3::X),
                -delta.dot(transform.rotation() * Vec3::Y),
            )
        });
    let point = edge_point(direction, viewport);
    let vertical = (point.x - viewport.center().x).abs() > (viewport.size().x * 0.5 - 12.);
    let size = if vertical {
        Vec2::new(7., 64.)
    } else {
        Vec2::new(64., 7.)
    };
    let node = Node {
        position_type: PositionType::Absolute,
        left: Val::Px(point.x - size.x * 0.5),
        top: Val::Px(point.y - size.y * 0.5),
        width: Val::Px(size.x),
        height: Val::Px(size.y),
        ..default()
    };
    let colour = BackgroundColor(Color::srgba(
        1.,
        0.22,
        0.04,
        0.4 + 0.55 * ((now - pulse.started) as f32 * 12.).sin().abs(),
    ));
    let visibility = if now < pulse.until {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    if let Some(entity) = pulse.entity {
        if let Ok((mut current, mut background, mut shown)) = nodes.get_mut(entity) {
            *current = node;
            *background = colour;
            if *shown != visibility {
                *shown = visibility;
            }
        }
    } else {
        pulse.entity = Some(
            commands
                .spawn((
                    node,
                    colour,
                    visibility,
                    ZIndex(200),
                    bevy::ui::FocusPolicy::Pass,
                    bevy::picking::Pickable::IGNORE,
                ))
                .id(),
        );
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn direction_stays_on_the_correct_viewport_edge() {
        let rect = Rect::from_corners(Vec2::new(40., 20.), Vec2::new(1040., 720.));
        assert_eq!(edge_point(Vec2::X, rect), Vec2::new(1030., 370.));
        assert_eq!(edge_point(Vec2::NEG_Y, rect), Vec2::new(540., 30.));
        let diagonal = edge_point(Vec2::ONE, rect);
        assert!(rect.contains(diagonal));
        assert_eq!(diagonal.y, 710.);
    }
}
