use super::*;
#[test]
fn raised_hover_body_uses_live_support_and_does_not_invent_missing_contacts() {
    let shape = turts_mod_api::Shape::Cuboid {
        x: 0.6,
        y: 0.6,
        z: 1.0,
    };
    let ray = Ray3d::new(Vec3::new(0.0, 3.5, 5.0), Dir3::NEG_Z);
    assert!(
        body_hit(ray, Vec2::ZERO, &shape, 0.0, Some(0.0)).is_none(),
        "the old ground-zero box must miss this elevated ray"
    );
    let hit = body_hit(ray, Vec2::ZERO, &shape, 0.0, Some(3.0)).expect("ray crosses raised body");
    assert!((hit - 4.7).abs() < 1e-5);
    assert!(
        body_hit(ray, Vec2::ZERO, &shape, 0.0, None).is_none(),
        "missing live anchor cannot grant a body hit"
    );
    let flat = Ray3d::new(Vec3::new(0.0, 0.5, 5.0), Dir3::NEG_Z);
    assert_eq!(
        body_hit(flat, Vec2::ZERO, &shape, 0.0, Some(0.0)),
        turts_interaction::picking::shape_hit(
            flat.origin,
            *flat.direction,
            Vec2::ZERO,
            &shape,
            0.0
        )
    );
}
#[test]
fn raised_hover_footprint_intersects_its_contact_plane_and_uses_physical_ties() {
    let ray = Ray3d::new(
        Vec3::new(0.0, 5.0, 5.0),
        Dir3::new(Vec3::new(0.0, -1.0, -1.0)).unwrap(),
    );
    let position = Vec2::new(0.0, 3.0);
    assert!(footprint_hit(ray, position, 0.35, Some(3.0)).is_some());
    assert!(footprint_hit(ray, position, 0.35, Some(0.0)).is_none());
    assert!(footprint_hit(ray, position, 0.35, None).is_none());
    let flat = Vec2::ZERO;
    assert_eq!(
        footprint_hit(ray, flat, 0.35, Some(0.0)),
        ray.intersect_plane(Vec3::ZERO, InfinitePlane3d::new(Dir3::Y))
    );
    let left = (u32::MAX, 2.0, Vec2::new(1.0, 4.0));
    let right = (1, 2.0, Vec2::new(3.0, 4.0));
    assert!(physical_hit_order(&left, &right).is_lt());
    assert!(physical_hit_order(&(1, left.1, left.2), &(u32::MAX, right.1, right.2)).is_lt());
}
