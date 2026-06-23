#![cfg(test)]

use std::any::TypeId;

use crate::{CommandBuffer, Entity, Universe, World, components::Component, query::Query};

#[derive(Debug, serde::Serialize, serde::Deserialize, Component)]
struct Health(u32);

#[derive(Debug, serde::Serialize, serde::Deserialize, Component)]
struct Mana(u32);

#[test]
fn allocator_clones_share_world_and_entity_sequences() {
    let allocator = crate::Allocator::new();
    let clone = allocator.clone();

    assert_eq!(allocator.allocate_world().raw(), 0);
    assert_eq!(clone.allocate_world().raw(), 1);
    assert_eq!(allocator.allocate_entity().raw(), 0);
    assert_eq!(clone.allocate_entity().raw(), 1);
}

#[test]
fn builder_creates_worlds_and_initial_entities() {
    let mut universe = Universe::builder()
        .with_world(World::builder("alpha").with_entity(Entity::builder()))
        .with_world(World::builder("beta").with_entity(Entity::builder()))
        .build();
    universe.tick(0.0);

    assert_eq!(universe.alive_count(), 2);
    assert_eq!(
        universe.world(crate::WorldId::default()).map(World::name),
        Some("alpha")
    );
    assert_eq!(
        universe
            .world(crate::WorldId::default() + 1)
            .map(World::name),
        Some("beta")
    );
}

#[test]
fn spawn_entity_in_missing_world_returns_none() {
    let mut universe = Universe::builder().build();
    universe.tick(0.0);
    let missing = crate::WorldId::default() + 99;

    let spawned = universe.spawn_entity(missing, Entity::builder());

    assert!(spawned.is_none());
    assert_eq!(universe.alive_count(), 0);
}

#[test]
fn query_filters_by_components_and_world_membership() {
    let mut universe = Universe::builder()
        .with_world(World::builder("a"))
        .with_world(World::builder("b"))
        .build();
    universe.tick(0.0);

    let world_a = crate::WorldId::default();
    let world_b = world_a + 1;

    let e1 = universe
        .spawn_entity(world_a, Entity::builder().with_component(Health(10)))
        .expect("world exists");
    let e2 = universe
        .spawn_entity(
            world_b,
            Entity::builder()
                .with_component(Health(20))
                .with_component(Mana(5)),
        )
        .expect("world exists");

    let q = Query::empty()
        .with_component::<Health>()
        .without_component::<Mana>()
        .with_world(world_a)
        .without_world(world_b);

    assert!(q.matches(&universe, e1));
    assert!(!q.matches(&universe, e2));
}

#[test]
fn component_getter_returns_expected_values() {
    let mut universe = Universe::builder().with_world(World::builder("w")).build();
    universe.tick(0.0);
    let world = crate::WorldId::default();

    let e = universe
        .spawn_entity(world, Entity::builder().with_component(Health(123)))
        .expect("world exists");

    assert_eq!(universe.component::<Health>(e).map(|h| h.0), Some(123));
    assert_eq!(universe.component::<Mana>(e).map(|m| m.0), None);
}

#[test]
fn worlds_returns_all_live_worlds() {
    let mut universe = Universe::builder()
        .with_world(World::builder("alpha"))
        .with_world(World::builder("beta"))
        .build();
    universe.tick(0.0);

    let mut worlds: Vec<_> = universe
        .worlds()
        .map(|world| (world.id().raw(), world.name().to_owned()))
        .collect();
    worlds.sort_by_key(|(id, _)| *id);

    assert_eq!(
        worlds,
        vec![(0, "alpha".to_owned()), (1, "beta".to_owned())]
    );
}

#[test]
fn entities_returns_live_entity_world_pairs() {
    let mut universe = Universe::builder()
        .with_world(World::builder("alpha"))
        .with_world(World::builder("beta"))
        .build();
    universe.tick(0.0);

    let first_world = crate::WorldId::default();
    let second_world = first_world + 1;
    let first = universe
        .spawn_entity(first_world, Entity::builder())
        .expect("world exists");
    let second = universe
        .spawn_entity(second_world, Entity::builder())
        .expect("world exists");

    let mut entities: Vec<_> = universe
        .entities()
        .map(|(entity, world)| (entity.raw(), world.raw()))
        .collect();
    entities.sort_by_key(|(entity, _)| *entity);

    assert_eq!(
        entities,
        vec![
            (first.raw(), first_world.raw()),
            (second.raw(), second_world.raw())
        ]
    );
}

#[test]
fn entities_in_world_filters_correctly() {
    let mut universe = Universe::builder()
        .with_world(World::builder("alpha"))
        .with_world(World::builder("beta"))
        .build();
    universe.tick(0.0);

    let first_world = crate::WorldId::default();
    let second_world = first_world + 1;
    let first = universe
        .spawn_entity(first_world, Entity::builder())
        .expect("world exists");
    let _second = universe
        .spawn_entity(second_world, Entity::builder())
        .expect("world exists");

    let entities: Vec<_> = universe
        .entities_in_world(first_world)
        .map(Entity::raw)
        .collect();

    assert_eq!(entities, vec![first.raw()]);
}

#[test]
fn component_infos_exposes_type_name_and_debug_value() {
    let mut universe = Universe::builder().with_world(World::builder("w")).build();
    universe.tick(0.0);
    let world = crate::WorldId::default();
    let entity = universe
        .spawn_entity(world, Entity::builder().with_component(Health(77)))
        .expect("world exists");

    let infos: Vec<_> = universe.component_infos(entity).collect();

    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].type_id, TypeId::of::<Health>());
    assert_eq!(infos[0].type_name, std::any::type_name::<Health>());
    assert_eq!(format!("{:?}", infos[0].debug), "Health(77)");
}

#[test]
fn inspection_helpers_update_after_despawn_and_world_destruction() {
    let mut universe = Universe::builder()
        .with_world(World::builder("alpha"))
        .with_world(World::builder("beta"))
        .build();
    universe.tick(0.0);

    let first_world = crate::WorldId::default();
    let second_world = first_world + 1;
    let despawned = universe
        .spawn_entity(first_world, Entity::builder().with_component(Health(10)))
        .expect("world exists");
    let destroyed = universe
        .spawn_entity(second_world, Entity::builder().with_component(Mana(20)))
        .expect("world exists");

    let mut command_buffer = CommandBuffer::new();
    command_buffer.despawn(despawned);
    command_buffer.destroy_world(second_world);
    universe.submit_buffer(command_buffer);
    universe.tick(0.016);

    assert_eq!(universe.component_infos(despawned).count(), 0);
    assert_eq!(universe.component_infos(destroyed).count(), 0);
    assert!(!universe.entities().any(|(entity, _)| entity == despawned));
    assert!(!universe.entities().any(|(entity, _)| entity == destroyed));
    assert!(!universe.worlds().any(|world| world.id() == second_world));
}
