//! Integration tests for the `universe` crate.

use dirk_universe::{Entity, Universe, World, components::Component};

#[derive(Debug, serde::Serialize, serde::Deserialize, Component)]
struct Position(i32, i32);

#[derive(Debug, serde::Serialize, serde::Deserialize, Component)]
struct Hidden;

#[test]
fn universe_public_api_supports_entity_lifecycle_across_worlds() {
    let mut universe = Universe::builder()
        .with_world(World::builder("overworld"))
        .with_world(World::builder("dungeon"))
        .build();
    universe.tick(0.0);

    let overworld = dirk_universe::WorldId::default();
    let dungeon = overworld + 1;

    let entity = universe
        .spawn_entity(overworld, Entity::builder().with_component(Position(2, 3)))
        .expect("entity should spawn");

    assert!(universe.is_alive(entity));
    assert!(universe.is_in_world(overworld, entity));
    assert_eq!(universe.get_world(entity), Some(overworld));

    let mut cmd = dirk_universe::CommandBuffer::new();
    cmd.send(entity, dungeon);
    universe.submit_buffer(cmd);
    universe.tick(0.016);

    assert!(universe.is_in_world(dungeon, entity));
    assert_eq!(universe.get_world(entity), Some(dungeon));

    let mut cmd = dirk_universe::CommandBuffer::new();
    cmd.despawn(entity);
    universe.submit_buffer(cmd);
    universe.tick(0.016);

    assert!(!universe.is_alive(entity));
}

#[test]
fn buffered_spawns_are_applied_on_tick_and_components_are_readable() {
    let mut universe = Universe::builder().with_world(World::builder("w")).build();
    universe.tick(0.0);
    let world = dirk_universe::WorldId::default();

    let mut cmd = dirk_universe::CommandBuffer::new();
    cmd.spawn(world, Entity::builder().with_component(Position(1, 1)));
    cmd.spawn(
        world,
        Entity::builder()
            .with_component(Position(9, 9))
            .with_component(Hidden),
    );
    universe.submit_buffer(cmd);

    universe.tick(0.016);

    assert_eq!(universe.alive_count(), 2);

    let e0 = Entity::default();
    let e1 = e0 + 1;

    assert_eq!(
        universe.component::<Position>(e0).map(|p| (p.0, p.1)),
        Some((1, 1))
    );
    assert_eq!(universe.component::<Hidden>(e0).map(|_| true), None);

    assert_eq!(
        universe.component::<Position>(e1).map(|p| (p.0, p.1)),
        Some((9, 9))
    );
    assert_eq!(universe.component::<Hidden>(e1).map(|_| true), Some(true));
}

#[test]
fn allocator_handles_can_be_used_in_command_buffers() {
    let mut universe = Universe::builder().build();
    let allocator = universe.allocator();
    let world = allocator.allocate_world();
    let entity = allocator.allocate_entity();

    let mut cmd = dirk_universe::CommandBuffer::new();
    cmd.create_allocated_world(world, World::builder("allocated"));
    cmd.spawn_allocated(
        world,
        entity,
        Entity::builder().with_component(Position(4, 8)),
    );
    universe.submit_buffer(cmd);

    universe.tick(0.016);

    assert_eq!(universe.world(world).map(World::name), Some("allocated"));
    assert!(universe.is_in_world(world, entity));
    assert_eq!(
        universe.component::<Position>(entity).map(|p| (p.0, p.1)),
        Some((4, 8))
    );
}
