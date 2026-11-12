// Planted fixture for `movement_writers.rs`: each function holds one kind of write that the scan
// must find. The file is not compiled; the scan parses it with syn.

fn spawn_with_position(world: &mut World) {
    world.spawn((UnitId(9), Position(Vec2::ZERO)));
}

fn insert_target(world: &mut World, entity: Entity) {
    world.entity_mut(entity).insert(MoveTarget(Vec2::ONE));
}

fn mutate_orders(world: &mut World, entity: Entity) {
    if let Some(mut orders) = world.get_mut::<UnitOrders>(entity) {
        orders.queue.clear();
    }
}

fn remove_progress(world: &mut World, entity: Entity) {
    world.entity_mut(entity).remove::<MoveProgress>();
}

fn query_positions(mut positions: Query<&mut Position>) {
    for mut position in &mut positions {
        position.0 = Vec2::ZERO;
    }
}

fn clear_pools(world: &mut World) {
    world.resource_mut::<MovementPools>().0.clear();
}

fn build_progress() -> MoveProgress {
    MoveProgress {
        pool_id: 1,
        ..Default::default()
    }
}

fn read_only(world: &World, entity: Entity) -> Option<Vec2> {
    world.get::<Position>(entity).map(|position| position.0)
}

fn pools_param(mut pools: ResMut<MovementPools>) {
    pools.0.clear();
}

fn component_handle(world: &mut World, entity: Entity) -> Option<Mut<Position>> {
    world.get_entity_mut(entity).ok()?.get_mut()
}

fn non_send_param(mut pools: NonSendMut<MovementPools>) {
    pools.0.clear();
}

fn read_param(pools: Res<MovementPools>) -> usize {
    pools.0.len()
}

#[cfg(test)]
mod tests {
    fn test_write(world: &mut World) {
        world.spawn(Position(Vec2::ZERO));
    }
}
