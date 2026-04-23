#![allow(missing_docs)]

/// -----------------------------------------------------------------------
/// Integration tests — Player lifecycle (requires test doubles / harness)
/// -----------------------------------------------------------------------
///
/// The tests below depend on `World`, `EventManager`, and the channel
/// infrastructure being available.  They are gated behind a feature flag so
/// they only run in environments that have the full engine linked.
use ::events::EventManager;
use platform::WindowId;
use world::{
    World,
    events::{PlayerUpdateEvent, PlayerUpdateType},
    player::{Player, PlayerRegion},
};

// A trivial `WindowId` value used across tests.
const WINDOW: WindowId = WindowId::from_raw(1);

fn setup() -> (World, EventManager) {
    let events = EventManager::new();
    let world = World::new(1, &events);
    (world, events)
}

// -----------------------------------------------------------------------
// Helpers / stubs
// -----------------------------------------------------------------------

/// Convenience constructor for a full-screen region.
fn full_screen() -> PlayerRegion {
    PlayerRegion::default()
}

/// Left-half region (horizontal split-screen, player 1).
fn left_half() -> PlayerRegion {
    PlayerRegion {
        offset: glam::vec2(0.0, 0.0),
        size: glam::vec2(0.5, 1.0),
    }
}

/// Right-half region (horizontal split-screen, player 2).
fn right_half() -> PlayerRegion {
    PlayerRegion {
        offset: glam::vec2(0.5, 0.0),
        size: glam::vec2(0.5, 1.0),
    }
}

// -------------------------------------------------------------------
// Spawn
// -------------------------------------------------------------------

#[test]
fn spawn_emits_spawned_event() {
    let (mut world, events) = setup();
    let listener = events.subscribe::<PlayerUpdateEvent>();

    let player = Player::spawn(1, &mut world, WINDOW, &events);

    events.dispatch_all();

    let evt = listener.try_consume().expect("expected a Spawned event");
    assert!(matches!(evt.update_type, PlayerUpdateType::Spawned));
    assert_eq!(evt.id, player.id());
    assert_eq!(evt.entity, player.entity());
    assert_eq!(evt.world, player.world());
    assert_eq!(evt.window, player.window());
}

#[test]
fn spawn_exactly_one_event() {
    let (mut world, events) = setup();
    let listener = events.subscribe::<PlayerUpdateEvent>();

    let _player = Player::spawn(2, &mut world, WINDOW, &events);

    events.dispatch_all();

    assert!(listener.try_consume().is_some());
    assert!(
        listener.try_consume().is_none(),
        "spawn must emit exactly one event"
    );
}

#[test]
fn spawn_default_region_is_full_screen() {
    let (mut world, events) = setup();
    let player = Player::spawn(3, &mut world, WINDOW, &events);
    assert_eq!(player.region().offset, glam::Vec2::ZERO);
    assert_eq!(player.region().size, glam::Vec2::ONE);
}

#[test]
fn spawn_creates_entity_in_world() {
    let (mut world, events) = setup();
    let player = Player::spawn(4, &mut world, WINDOW, &events);
    assert!(world.is_alive(player.entity()));
}

#[test]
fn spawn_attaches_transform_component() {
    let (mut world, events) = setup();
    let player = Player::spawn(5, &mut world, WINDOW, &events);
    let transform = world
        .get::<world::components::Transform>(player.entity())
        .expect("Transform should be attached on spawn");
    // Default location: y = 1000, z = 1000.
    assert!((transform.location.y - 1000.0).abs() < f32::EPSILON);
    assert!((transform.location.z - 1000.0).abs() < f32::EPSILON);
}

#[test]
fn spawn_attaches_camera_component() {
    let (mut world, events) = setup();
    let player = Player::spawn(6, &mut world, WINDOW, &events);
    let camera = world
        .get::<world::components::Camera>(player.entity())
        .expect("Camera should be attached on spawn");
    assert!((camera.fov - 45_f32.to_radians()).abs() < f32::EPSILON);
    assert!((camera.near_clip - 0.1).abs() < f32::EPSILON);
    assert!((camera.far_clip - 100_000.0).abs() < f32::EPSILON);
}

// -------------------------------------------------------------------
// set_region
// -------------------------------------------------------------------

#[test]
fn set_region_updates_stored_region() {
    let (mut world, events) = setup();
    let mut player = Player::spawn(10, &mut world, WINDOW, &events);
    let new_region = left_half();
    player.set_region(new_region.clone());
    assert_eq!(player.region().offset, new_region.offset);
    assert_eq!(player.region().size, new_region.size);
}

#[test]
fn set_region_emits_updated_event() {
    let (mut world, events) = setup();
    let listener = events.subscribe::<PlayerUpdateEvent>();
    let mut player = Player::spawn(11, &mut world, WINDOW, &events);

    events.dispatch_all();

    // Drain the Spawned event.
    let _ = listener.try_consume();

    player.set_region(right_half());

    events.dispatch_all();

    let evt = listener.try_consume().expect("expected an Updated event");
    assert!(
        matches!(evt.update_type, PlayerUpdateType::Updated),
        "expected Updated, got {:?}",
        evt.update_type
    );
    assert_eq!(evt.id, player.id());
}

#[test]
fn set_region_exactly_one_event_per_call() {
    let (mut world, events) = setup();
    let listener = events.subscribe::<PlayerUpdateEvent>();
    let mut player = Player::spawn(12, &mut world, WINDOW, &events);
    events.dispatch_all();
    let _ = listener.try_consume(); // drain Spawned

    player.set_region(left_half());
    events.dispatch_all();
    assert!(listener.try_consume().is_some());
    assert!(
        listener.try_consume().is_none(),
        "set_region must emit exactly one event"
    );
}

#[test]
fn set_region_multiple_calls_emit_multiple_events() {
    let (mut world, events) = setup();
    let listener = events.subscribe::<PlayerUpdateEvent>();
    let mut player = Player::spawn(13, &mut world, WINDOW, &events);
    events.dispatch_all();
    let _ = listener.try_consume(); // drain Spawned

    player.set_region(left_half());
    player.set_region(right_half());
    player.set_region(full_screen());

    let mut count = 0;
    events.dispatch_all();
    while listener.try_consume().is_some() {
        count += 1;
    }
    assert_eq!(count, 3, "expected 3 Updated events");
}

// -------------------------------------------------------------------
// Despawn
// -------------------------------------------------------------------

#[test]
fn despawn_emits_despawned_event() {
    let (mut world, events) = setup();
    let listener = events.subscribe::<PlayerUpdateEvent>();
    let player = Player::spawn(20, &mut world, WINDOW, &events);
    let entity = player.entity();
    events.dispatch_all();
    let _ = listener.try_consume(); // drain Spawned

    player.despawn(&mut world);

    events.dispatch_all();
    let evt = listener.try_consume().expect("expected a Despawned event");
    assert!(matches!(evt.update_type, PlayerUpdateType::Despawned));
    assert_eq!(evt.entity, entity);
}

#[test]
fn despawn_removes_entity_from_world() {
    let (mut world, events) = setup();
    let player = Player::spawn(21, &mut world, WINDOW, &events);
    let entity = player.entity();

    player.despawn(&mut world);

    assert!(
        !world.is_alive(entity),
        "entity should be gone after despawn"
    );
}

#[test]
fn despawn_exactly_one_event() {
    let (mut world, events) = setup();
    let listener = events.subscribe::<PlayerUpdateEvent>();
    let player = Player::spawn(22, &mut world, WINDOW, &events);
    events.dispatch_all();
    let _ = listener.try_consume(); // drain Spawned

    player.despawn(&mut world);

    events.dispatch_all();

    assert!(listener.try_consume().is_some());
    assert!(
        listener.try_consume().is_none(),
        "despawn must emit exactly one event"
    );
}

// -------------------------------------------------------------------
// Multiple players
// -------------------------------------------------------------------

#[test]
fn two_players_in_same_world_have_distinct_entities() {
    let (mut world, events) = setup();
    let p1 = Player::spawn(30, &mut world, WINDOW, &events);
    let p2 = Player::spawn(31, &mut world, WINDOW, &events);
    assert_ne!(p1.entity(), p2.entity());
}

#[test]
fn two_players_emit_independent_events() {
    let (mut world, events) = setup();
    let listener = events.subscribe::<PlayerUpdateEvent>();

    let _p1 = Player::spawn(40, &mut world, WINDOW, &events);
    let _p2 = Player::spawn(41, &mut world, WINDOW, &events);

    events.dispatch_all();

    let evt1 = listener.try_consume().unwrap();
    let evt2 = listener.try_consume().unwrap();
    assert_ne!(evt1.id, evt2.id);
    assert!(listener.try_consume().is_none());
}

#[test]
fn despawning_one_player_does_not_affect_other() {
    let (mut world, events) = setup();
    let p1 = Player::spawn(50, &mut world, WINDOW, &events);
    let p2 = Player::spawn(51, &mut world, WINDOW, &events);
    let p2_entity = p2.entity();

    p1.despawn(&mut world);

    assert!(world.is_alive(p2_entity), "p2 should still be alive");
}

// -------------------------------------------------------------------
// Accessor round-trips
// -------------------------------------------------------------------

#[test]
fn accessors_return_values_passed_to_spawn() {
    let (mut world, events) = setup();
    let id = 99_u32;
    let window = WINDOW;
    let player = Player::spawn(id, &mut world, window, &events);

    assert_eq!(player.id(), id);
    assert_eq!(player.window(), window);
    assert_eq!(player.world(), world.id());
}
