#![cfg(test)]

use crate::{Entity, Universe, World, components::Component, query::Query};

#[derive(Debug, serde::Serialize, serde::Deserialize, Component)]
struct Health(u32);

#[derive(Debug, serde::Serialize, serde::Deserialize, Component)]
struct Mana(u32);

#[test]
fn builder_creates_worlds_and_initial_entities() {
    let mut universe = Universe::builder()
        .with_world(World::builder("alpha").with_entity(Entity::builder()))
        .with_world(World::builder("beta").with_entity(Entity::builder()))
        .build();
    universe.tick(0.0);

    assert_eq!(universe.alive_count(), 2);
    assert_eq!(
        universe.world(crate::WorldId::default()).map(|w| w.name()),
        Some("alpha")
    );
    assert_eq!(
        universe
            .world(crate::WorldId::default() + 1)
            .map(|w| w.name()),
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
