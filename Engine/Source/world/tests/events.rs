use ::events::EventManager;
use platform::WindowId;
use world::events::{PlayerUpdateEvent, PlayerUpdateType};
use world::player::{Player, PlayerRegion};
use world::*;

const WINDOW: WindowId = WindowId::from_raw(1);

fn setup() -> (crate::World, EventManager) {
    let events = EventManager::new();
    (world::World::new(1, &events), events)
}

// -------------------------------------------------------------------
// from_player — field correctness
// -------------------------------------------------------------------

#[test]
fn from_player_captures_correct_id() {
    let (mut world, events) = setup();
    let player = Player::spawn(42, &mut world, WINDOW, &events);
    let evt = PlayerUpdateEvent::from_player(&player, PlayerUpdateType::Spawned);
    assert_eq!(evt.id, player.id());
}

#[test]
fn from_player_captures_correct_world() {
    let (mut world, events) = setup();
    let player = Player::spawn(1, &mut world, WINDOW, &events);
    let evt = PlayerUpdateEvent::from_player(&player, PlayerUpdateType::Spawned);
    assert_eq!(evt.world, player.world());
}

#[test]
fn from_player_captures_correct_entity() {
    let (mut world, events) = setup();
    let player = Player::spawn(2, &mut world, WINDOW, &events);
    let evt = PlayerUpdateEvent::from_player(&player, PlayerUpdateType::Spawned);
    assert_eq!(evt.entity, player.entity());
}

#[test]
fn from_player_captures_correct_window() {
    let (mut world, events) = setup();
    let player = Player::spawn(3, &mut world, WINDOW, &events);
    let evt = PlayerUpdateEvent::from_player(&player, PlayerUpdateType::Updated);
    assert_eq!(evt.window, WINDOW);
}

#[test]
fn from_player_snapshots_region_at_call_time() {
    let (mut world, events) = setup();
    let mut player = Player::spawn(4, &mut world, WINDOW, &events);

    // Change the region, then build the event.
    let new_region = PlayerRegion {
        offset: glam::vec2(0.5, 0.0),
        size: glam::vec2(0.5, 1.0),
    };
    player.set_region(new_region.clone());

    let evt = PlayerUpdateEvent::from_player(&player, PlayerUpdateType::Updated);
    assert_eq!(evt.region.offset, new_region.offset);
    assert_eq!(evt.region.size, new_region.size);
}

#[test]
fn from_player_snapshot_is_independent_of_later_mutation() {
    let (mut world, events) = setup();
    let mut player = Player::spawn(5, &mut world, WINDOW, &events);

    // Take a snapshot with the default (full-screen) region.
    let evt = PlayerUpdateEvent::from_player(&player, PlayerUpdateType::Updated);
    let original_offset = evt.region.offset;

    // Mutate the player's region after the snapshot.
    player.set_region(PlayerRegion {
        offset: glam::vec2(0.5, 0.0),
        size: glam::vec2(0.5, 1.0),
    });

    // The already-built event must be unaffected.
    assert_eq!(evt.region.offset, original_offset);
}

#[test]
fn from_player_stores_given_update_type() {
    let (mut world, events) = setup();
    let player = Player::spawn(6, &mut world, WINDOW, &events);

    for variant in [
        PlayerUpdateType::Spawned,
        PlayerUpdateType::Updated,
        PlayerUpdateType::Despawned,
    ] {
        let evt = PlayerUpdateEvent::from_player(&player, variant.clone());
        assert_eq!(evt.update_type, variant);
    }
}

// -------------------------------------------------------------------
// Event clone
// -------------------------------------------------------------------

#[test]
fn player_update_event_clone_has_same_fields() {
    let (mut world, events) = setup();
    let player = Player::spawn(7, &mut world, WINDOW, &events);
    let evt = PlayerUpdateEvent::from_player(&player, PlayerUpdateType::Spawned);
    let cloned = evt.clone();

    assert_eq!(cloned.id, evt.id);
    assert_eq!(cloned.world, evt.world);
    assert_eq!(cloned.entity, evt.entity);
    assert_eq!(cloned.window, evt.window);
    assert_eq!(cloned.region.offset, evt.region.offset);
    assert_eq!(cloned.region.size, evt.region.size);
    assert_eq!(cloned.update_type, evt.update_type);
}

// -------------------------------------------------------------------
// Integration — events received through the dispatcher
// -------------------------------------------------------------------

#[test]
fn spawned_event_received_by_subscriber_has_correct_update_type() {
    let (mut world, events) = setup();
    let listener = events.subscribe::<PlayerUpdateEvent>();

    let _player = Player::spawn(8, &mut world, WINDOW, &events);

    events.dispatch_all();
    let evt = listener.try_consume().expect("expected event");
    assert_eq!(evt.update_type, PlayerUpdateType::Spawned);
}

#[test]
fn updated_event_received_by_subscriber_has_correct_region() {
    let (mut world, events) = setup();
    let listener = events.subscribe::<PlayerUpdateEvent>();
    let mut player = Player::spawn(9, &mut world, WINDOW, &events);
    events.dispatch_all();
    let _ = listener.try_consume(); // drain Spawned

    let new_region = PlayerRegion {
        offset: glam::vec2(0.0, 0.0),
        size: glam::vec2(0.5, 1.0),
    };
    player.set_region(new_region.clone());

    events.dispatch_all();
    let evt = listener.try_consume().expect("expected Updated event");
    assert_eq!(evt.update_type, PlayerUpdateType::Updated);
    assert_eq!(evt.region.offset, new_region.offset);
    assert_eq!(evt.region.size, new_region.size);
}

#[test]
fn despawned_event_received_by_subscriber_has_correct_update_type() {
    let (mut world, events) = setup();
    let listener = events.subscribe::<PlayerUpdateEvent>();
    let player = Player::spawn(10, &mut world, WINDOW, &events);
    events.dispatch_all();
    let _ = listener.try_consume(); // drain Spawned

    player.despawn(&mut world);

    events.dispatch_all();
    let evt = listener.try_consume().expect("expected Despawned event");
    assert_eq!(evt.update_type, PlayerUpdateType::Despawned);
}
