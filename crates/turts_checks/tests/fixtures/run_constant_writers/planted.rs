// Planted fixture for `run_constant_writers.rs`: each function holds one kind of write of a run
// constant that the scan must find. The file is not compiled; the scan parses it with syn.

fn grow_bounds(world: &mut World) {
    if let Some(mut bounds) = world.get_resource_mut::<MapBounds>() {
        bounds.size.y += 1.0;
    }
}

fn bounds_param(mut bounds: ResMut<MapBounds>) {
    bounds.size.y += 1.0;
}

fn durations_scope(world: &mut World) {
    world.resource_scope::<ProductionDurations, _>(|_, mut durations| {
        durations.0.clear();
    });
}

fn types_handle(types: Mut<UnitTypes>) -> usize {
    types.len()
}

fn drop_types(world: &mut World) {
    world.remove_resource::<UnitTypes>();
}

fn drop_clock(world: &mut World) {
    world.remove_resource::<SimClock>();
}

fn make_bounds() -> MapBounds {
    MapBounds {
        size: Vec2::ONE,
    }
}

fn read_only(world: &World) -> usize {
    world.resource::<ProductionDurations>().0.len() + world.get_resource::<MapBounds>().is_some() as usize
}

#[cfg(test)]
mod tests {
    fn test_write(world: &mut World) {
        world.remove_resource::<MapBounds>();
    }
}
