#![cfg(test)]

use super::*;
use macros::Event;
use std::thread;
use std::time::Duration;

// -------------------------------------------------------------------------
// Test event types
// -------------------------------------------------------------------------

#[derive(Event, Clone, Debug, PartialEq)]
struct KeyEvent {
    key: String,
    pressed: bool,
}

#[derive(Event, Clone, Debug, PartialEq)]
struct MouseEvent {
    x: f32,
    y: f32,
}

#[derive(Event, Clone, Debug, PartialEq)]
struct WindowResizeEvent {
    width: u32,
    height: u32,
}

// A zero-sized event used to test that marker-like events work.
#[derive(Event, Clone, Debug, PartialEq)]
struct TickEvent;

// -------------------------------------------------------------------------
// Unit tests — Dispatcher & Consumer basics
// -------------------------------------------------------------------------

#[test]
fn dispatched_event_is_received_by_consumer() {
    let mut manager = EventManager::new();
    let dispatcher = manager.register::<KeyEvent>();
    let consumer = manager.subscribe::<KeyEvent>();

    dispatcher.dispatch(KeyEvent {
        key: "A".into(),
        pressed: true,
    });
    manager.dispatch_all();

    let received = consumer.try_consume();
    assert_eq!(
        received,
        Some(KeyEvent {
            key: "A".into(),
            pressed: true
        })
    );
}

#[test]
fn try_consume_returns_none_when_queue_is_empty() {
    let mut manager = EventManager::new();
    let _dispatcher = manager.register::<KeyEvent>();
    let consumer = manager.subscribe::<KeyEvent>();

    // No dispatch — queue must be empty.
    manager.dispatch_all();
    assert_eq!(consumer.try_consume(), None);
}

#[test]
fn try_consume_returns_none_before_dispatch_all() {
    let mut manager = EventManager::new();
    let dispatcher = manager.register::<KeyEvent>();
    let consumer = manager.subscribe::<KeyEvent>();

    // Event queued but dispatch_all not called yet.
    dispatcher.dispatch(KeyEvent {
        key: "B".into(),
        pressed: false,
    });
    assert_eq!(consumer.try_consume(), None);
}

#[test]
fn multiple_events_are_forwarded_in_order() {
    let mut manager = EventManager::new();
    let dispatcher = manager.register::<KeyEvent>();
    let consumer = manager.subscribe::<KeyEvent>();

    let events = vec![
        KeyEvent {
            key: "A".into(),
            pressed: true,
        },
        KeyEvent {
            key: "B".into(),
            pressed: true,
        },
        KeyEvent {
            key: "A".into(),
            pressed: false,
        },
    ];

    for e in &events {
        dispatcher.dispatch(e.clone());
    }
    manager.dispatch_all();

    let received: Vec<KeyEvent> = consumer.consume_all().collect();
    assert_eq!(received, events);
}

#[test]
fn consume_all_drains_the_queue_completely() {
    let mut manager = EventManager::new();
    let dispatcher = manager.register::<TickEvent>();
    let consumer = manager.subscribe::<TickEvent>();

    for _ in 0..5 {
        dispatcher.dispatch(TickEvent);
    }
    manager.dispatch_all();

    let count = consumer.consume_all().count();
    assert_eq!(count, 5);

    // A second call on the same frame should yield nothing.
    assert_eq!(consumer.consume_all().count(), 0);
}

// -------------------------------------------------------------------------
// Unit tests — multiple subscribers
// -------------------------------------------------------------------------

#[test]
fn event_is_cloned_to_all_subscribers() {
    let mut manager = EventManager::new();
    let dispatcher = manager.register::<MouseEvent>();
    let consumer_a = manager.subscribe::<MouseEvent>();
    let consumer_b = manager.subscribe::<MouseEvent>();

    dispatcher.dispatch(MouseEvent { x: 1.0, y: 2.0 });
    manager.dispatch_all();

    assert_eq!(
        consumer_a.try_consume(),
        Some(MouseEvent { x: 1.0, y: 2.0 })
    );
    assert_eq!(
        consumer_b.try_consume(),
        Some(MouseEvent { x: 1.0, y: 2.0 })
    );
}

#[test]
fn multiple_events_reach_all_subscribers() {
    let mut manager = EventManager::new();
    let dispatcher = manager.register::<TickEvent>();
    let consumer_a = manager.subscribe::<TickEvent>();
    let consumer_b = manager.subscribe::<TickEvent>();
    let consumer_c = manager.subscribe::<TickEvent>();

    for _ in 0..3 {
        dispatcher.dispatch(TickEvent);
    }
    manager.dispatch_all();

    for consumer in [&consumer_a, &consumer_b, &consumer_c] {
        assert_eq!(consumer.consume_all().count(), 3);
    }
}

// -------------------------------------------------------------------------
// Unit tests — type isolation
// -------------------------------------------------------------------------

#[test]
fn different_event_types_are_isolated() {
    let mut manager = EventManager::new();
    let key_dispatcher = manager.register::<KeyEvent>();
    let mouse_dispatcher = manager.register::<MouseEvent>();

    let key_consumer = manager.subscribe::<KeyEvent>();
    let mouse_consumer = manager.subscribe::<MouseEvent>();

    key_dispatcher.dispatch(KeyEvent {
        key: "Space".into(),
        pressed: true,
    });
    mouse_dispatcher.dispatch(MouseEvent { x: 0.5, y: 0.5 });
    manager.dispatch_all();

    // Key consumer receives only key events.
    assert!(key_consumer.try_consume().is_some());
    assert!(key_consumer.try_consume().is_none()); // No mouse event leaked in.

    // Mouse consumer receives only mouse events.
    assert!(mouse_consumer.try_consume().is_some());
    assert!(mouse_consumer.try_consume().is_none());
}

#[test]
fn subscriber_without_matching_producer_never_receives_events() {
    let mut manager = EventManager::new();
    // Register a producer for KeyEvent, but subscribe to MouseEvent instead.
    let _key_dispatcher = manager.register::<KeyEvent>();
    let mouse_consumer = manager.subscribe::<MouseEvent>();

    _key_dispatcher.dispatch(KeyEvent {
        key: "X".into(),
        pressed: true,
    });
    manager.dispatch_all();

    assert_eq!(mouse_consumer.try_consume(), None);
}

// -------------------------------------------------------------------------
// Unit tests — multiple dispatchers for the same type
// -------------------------------------------------------------------------

#[test]
fn two_dispatchers_for_same_type_both_reach_subscriber() {
    let mut manager = EventManager::new();
    let dispatcher_a = manager.register::<TickEvent>();
    let dispatcher_b = manager.register::<TickEvent>();
    let consumer = manager.subscribe::<TickEvent>();

    dispatcher_a.dispatch(TickEvent);
    dispatcher_b.dispatch(TickEvent);
    manager.dispatch_all();

    assert_eq!(consumer.consume_all().count(), 2);
}

// -------------------------------------------------------------------------
// Unit tests — dropped handles
// -------------------------------------------------------------------------

#[test]
fn dispatch_after_consumer_dropped_does_not_panic() {
    let mut manager = EventManager::new();
    let dispatcher = manager.register::<TickEvent>();
    {
        let _consumer = manager.subscribe::<TickEvent>();
        // Consumer dropped at end of this scope.
    }

    // Should silently discard the event instead of panicking.
    dispatcher.dispatch(TickEvent);
    manager.dispatch_all(); // Must not panic.
}

#[test]
fn dispatch_after_manager_dropped_does_not_panic() {
    let dispatcher = {
        let mut manager = EventManager::new();
        let d = manager.register::<TickEvent>();
        let _consumer = manager.subscribe::<TickEvent>();
        d
        // manager (and therefore the receiver) dropped here.
    };

    // Sending to a disconnected channel should be silently ignored.
    dispatcher.dispatch(TickEvent); // Must not panic.
}

// -------------------------------------------------------------------------
// Unit tests — zero-sized / marker events
// -------------------------------------------------------------------------

#[test]
fn zero_sized_event_round_trips_correctly() {
    let mut manager = EventManager::new();
    let dispatcher = manager.register::<TickEvent>();
    let consumer = manager.subscribe::<TickEvent>();

    dispatcher.dispatch(TickEvent);
    manager.dispatch_all();

    assert_eq!(consumer.try_consume(), Some(TickEvent));
}

// -------------------------------------------------------------------------
// Unit tests — idempotency across frames
// -------------------------------------------------------------------------

#[test]
fn events_do_not_carry_over_between_frames() {
    let mut manager = EventManager::new();
    let dispatcher = manager.register::<TickEvent>();
    let consumer = manager.subscribe::<TickEvent>();

    // Frame 1: dispatch one event and consume it.
    dispatcher.dispatch(TickEvent);
    manager.dispatch_all();
    let _ = consumer.try_consume();

    // Frame 2: no new dispatch.
    manager.dispatch_all();
    assert_eq!(
        consumer.try_consume(),
        None,
        "Stale event leaked into next frame"
    );
}

#[test]
fn dispatch_all_is_idempotent_when_queue_is_empty() {
    let mut manager = EventManager::new();
    let _dispatcher = manager.register::<TickEvent>();
    let consumer = manager.subscribe::<TickEvent>();

    // Calling dispatch_all with nothing queued must be a no-op.
    manager.dispatch_all();
    manager.dispatch_all();
    assert_eq!(consumer.try_consume(), None);
}

// -------------------------------------------------------------------------
// Integration tests
// -------------------------------------------------------------------------

/// Simulates a three-frame engine loop where events from multiple systems
/// are dispatched and consumed each frame.
#[test]
fn integration_multi_frame_engine_loop() {
    let mut manager = EventManager::new();
    let key_dispatcher = manager.register::<KeyEvent>();
    let mouse_dispatcher = manager.register::<MouseEvent>();
    let key_consumer = manager.subscribe::<KeyEvent>();
    let mouse_consumer = manager.subscribe::<MouseEvent>();

    // --- Frame 1 ---
    key_dispatcher.dispatch(KeyEvent {
        key: "W".into(),
        pressed: true,
    });
    mouse_dispatcher.dispatch(MouseEvent { x: 10.0, y: 20.0 });
    manager.dispatch_all();

    assert_eq!(key_consumer.consume_all().count(), 1);
    assert_eq!(mouse_consumer.consume_all().count(), 1);

    // --- Frame 2 ---
    // No events dispatched; both queues should be empty.
    manager.dispatch_all();
    assert_eq!(key_consumer.try_consume(), None);
    assert_eq!(mouse_consumer.try_consume(), None);

    // --- Frame 3 ---
    key_dispatcher.dispatch(KeyEvent {
        key: "W".into(),
        pressed: false,
    });
    mouse_dispatcher.dispatch(MouseEvent { x: 15.0, y: 25.0 });
    mouse_dispatcher.dispatch(MouseEvent { x: 16.0, y: 26.0 });
    manager.dispatch_all();

    assert_eq!(key_consumer.consume_all().count(), 1);
    assert_eq!(mouse_consumer.consume_all().count(), 2);
}

/// Two independent subsystems subscribe to the same event type.
/// Both must receive every event, independently.
#[test]
fn integration_two_systems_react_to_same_event() {
    let mut manager = EventManager::new();
    let dispatcher = manager.register::<WindowResizeEvent>();

    // "Renderer" and "UI system" both care about resize events.
    let renderer = manager.subscribe::<WindowResizeEvent>();
    let ui_system = manager.subscribe::<WindowResizeEvent>();

    dispatcher.dispatch(WindowResizeEvent {
        width: 1920,
        height: 1080,
    });
    manager.dispatch_all();

    let renderer_events: Vec<_> = renderer.consume_all().collect();
    let ui_events: Vec<_> = ui_system.consume_all().collect();

    assert_eq!(renderer_events.len(), 1);
    assert_eq!(ui_events.len(), 1);
    assert_eq!(
        renderer_events[0],
        WindowResizeEvent {
            width: 1920,
            height: 1080
        }
    );
    assert_eq!(ui_events[0], renderer_events[0]);
}

/// Verifies that the Dispatcher can safely be sent to another thread and
/// that events produced off-thread are forwarded correctly on the next
/// call to dispatch_all (which runs on the main thread).
#[test]
fn integration_dispatcher_usable_from_another_thread() {
    let mut manager = EventManager::new();
    let dispatcher = manager.register::<KeyEvent>();
    let consumer = manager.subscribe::<KeyEvent>();

    let handle = thread::spawn(move || {
        dispatcher.dispatch(KeyEvent {
            key: "Enter".into(),
            pressed: true,
        });
    });

    handle.join().expect("thread panicked");

    // Give the channel time to settle (it's synchronous, so this is
    // just belt-and-braces).
    thread::sleep(Duration::from_millis(10));

    manager.dispatch_all();
    assert_eq!(
        consumer.try_consume(),
        Some(KeyEvent {
            key: "Enter".into(),
            pressed: true
        })
    );
}

/// Stress test: large volume of events across multiple types in one frame.
#[test]
fn integration_high_volume_single_frame() {
    const N: usize = 10_000;

    let mut manager = EventManager::new();
    let tick_dispatcher = manager.register::<TickEvent>();
    let key_dispatcher = manager.register::<KeyEvent>();

    let tick_consumer_a = manager.subscribe::<TickEvent>();
    let tick_consumer_b = manager.subscribe::<TickEvent>();
    let key_consumer = manager.subscribe::<KeyEvent>();

    for _ in 0..N {
        tick_dispatcher.dispatch(TickEvent);
    }
    for i in 0..N {
        key_dispatcher.dispatch(KeyEvent {
            key: format!("key_{i}"),
            pressed: i % 2 == 0,
        });
    }

    manager.dispatch_all();

    assert_eq!(tick_consumer_a.consume_all().count(), N);
    assert_eq!(tick_consumer_b.consume_all().count(), N);
    assert_eq!(key_consumer.consume_all().count(), N);
}
