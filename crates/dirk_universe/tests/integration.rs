//! Integration tests for the `universe` crate.

use dirk_universe::{Entity, EntityBuilder, Universe, World, WorldId, components::Component};

#[derive(Debug, serde::Serialize, serde::Deserialize, Component)]
struct Position(i32, i32);

#[derive(Debug, serde::Serialize, serde::Deserialize, Component)]
struct Hidden;

fn spawn_entity(universe: &mut Universe, world: WorldId, builder: EntityBuilder) -> Entity {
    let mut cmd = universe.handle().command_buffer();
    let entity = cmd.spawn(world, builder);
    cmd.submit();
    universe.tick(0.0);
    entity
}

#[test]
fn universe_public_api_supports_entity_lifecycle_across_worlds() {
    let mut universe = Universe::builder()
        .with_world(World::builder("overworld"))
        .with_world(World::builder("dungeon"))
        .build();
    universe.tick(0.0);

    let overworld = dirk_universe::WorldId::default();
    let dungeon = overworld + 1;

    let entity = spawn_entity(
        &mut universe,
        overworld,
        Entity::builder().with_component(Position(2, 3)),
    );

    assert!(universe.is_alive(entity));
    assert!(universe.is_in_world(overworld, entity));
    assert_eq!(universe.get_world(entity), Some(overworld));

    let mut cmd = universe.handle().command_buffer();
    cmd.send(entity, dungeon);
    cmd.submit();
    universe.tick(0.016);

    assert!(universe.is_in_world(dungeon, entity));
    assert_eq!(universe.get_world(entity), Some(dungeon));

    let mut cmd = universe.handle().command_buffer();
    cmd.despawn(entity);
    cmd.submit();
    universe.tick(0.016);

    assert!(!universe.is_alive(entity));
}

#[test]
fn buffered_spawns_are_applied_on_tick_and_components_are_readable() {
    let mut universe = Universe::builder().with_world(World::builder("w")).build();
    universe.tick(0.0);
    let world = dirk_universe::WorldId::default();

    let mut cmd = universe.handle().command_buffer();
    let e0 = cmd.spawn(world, Entity::builder().with_component(Position(1, 1)));
    let e1 = cmd.spawn(
        world,
        Entity::builder()
            .with_component(Position(9, 9))
            .with_component(Hidden),
    );
    cmd.submit();

    universe.tick(0.016);

    assert_eq!(universe.alive_count(), 2);

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
fn public_command_buffers_allocate_unique_handles() {
    let mut universe = Universe::builder().build();

    let mut first_cmd = universe.handle().command_buffer();
    let first_world = first_cmd.create_world(World::builder("first"));
    let first_entity = first_cmd.spawn(
        first_world,
        Entity::builder().with_component(Position(4, 8)),
    );

    let mut second_cmd = universe.handle().command_buffer();
    let second_world = second_cmd.create_world(World::builder("second"));
    let second_entity = second_cmd.spawn(
        second_world,
        Entity::builder().with_component(Position(16, 32)),
    );

    first_cmd.submit();
    second_cmd.submit();
    universe.tick(0.016);

    assert_ne!(first_world, second_world);
    assert_ne!(first_entity, second_entity);
    assert_eq!(universe.world(first_world).map(World::name), Some("first"));
    assert_eq!(
        universe.world(second_world).map(World::name),
        Some("second")
    );
    assert!(universe.is_in_world(first_world, first_entity));
    assert!(universe.is_in_world(second_world, second_entity));
    assert_eq!(
        universe
            .component::<Position>(first_entity)
            .map(|p| (p.0, p.1)),
        Some((4, 8))
    );
    assert_eq!(
        universe
            .component::<Position>(second_entity)
            .map(|p| (p.0, p.1)),
        Some((16, 32))
    );
}
