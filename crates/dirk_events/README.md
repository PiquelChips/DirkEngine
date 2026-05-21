# `events` — Engine Event System

A lightweight, type-safe, **channel-backed** pub/sub event bus designed for
use inside a game-engine loop.

## Core Concepts

| Term | Type | Role |
|------|------|------|
| **Event** | any `T: [events::Event]` | A value that travels through the bus |
| **Dispatcher** | [`Dispatcher<T>`] | Queues events for the next tick |
| **Consumer** | [`Consumer<T>`] | Reads events that were forwarded this tick |
| **Event Manager** | [`EventManager`] | Wires everything together; call [`dispatch_all`] once per frame |

## Quick Start

```rust
use dirk_events::{Event, EventManager};

// 1. Define your event type.
#[derive(Debug, Clone, Event)]
#[event("player scored {points} points")]
struct PlayerScored { points: u32 }

// 2. Create the shared manager.
let mgr = EventManager::new();

// 3. Obtain a dispatcher (producer side) and a consumer (subscriber side).
let dispatcher = mgr.register::<PlayerScored>();
let mut consumer   = mgr.subscribe::<PlayerScored>();

// 4. Queue an event from anywhere that holds the dispatcher.
dispatcher.dispatch(PlayerScored { points: 42 });

// 5. Once per frame / tick: forward queued events to all subscribers.
mgr.dispatch_all();

// 6. Read events on the consumer side.
for event in consumer.consume_all() {
    println!("score update: {}", event.debug()); // "player scored 42 points"
}
```

## Lifecycle & Delivery Guarantees

```text
 dispatcher.dispatch(e)
       │
       ▼
 [internal channel buffer]   ← events accumulate here between ticks
       │
 mgr.dispatch_all()          ← call exactly once per frame
       │
       ├──▶ consumer A
       ├──▶ consumer B
       └──▶ consumer C       ← every live consumer receives a clone
```

* **Buffered until `dispatch_all`** — events queued before `dispatch_all`
  are invisible to consumers. Call `dispatch_all` once per frame.
* **Fan-out** — every active [`Consumer`] for a given type receives its own
  independent clone of each event.
* **Type-isolated** — consumers only receive events of the exact type they
  subscribed to; other event types are never delivered to them.
* **Dropped-consumer pruning** — if a [`Consumer`] is dropped, its entry is
  silently removed on the next `dispatch_all`; no panic, no leak.

## Using the `#[derive(Event)]` Macro

The [`macros::Event`] derive macro implements the [`Event`] trait for you and
lets you customise the string returned by [`Event::debug`] via an optional
`#[event("…")]` attribute.

### No attribute — falls back to `{self:?}`

```rust
use dirk_events::Event;

#[derive(Debug, Clone, Event)]
struct RawEvent { id: u32 }

assert!(RawEvent { id: 7 }.debug().contains("RawEvent"));
```

### Named-field struct — reference fields by name

```rust
use dirk_events::Event;

#[derive(Debug, Clone, Event)]
#[event("moved to ({x}, {y})")]
struct EntityMoved { x: f32, y: f32 }

assert_eq!(EntityMoved { x: 1.0, y: 2.5 }.debug(), "moved to (1, 2.5)");
```

### Tuple struct — reference fields by zero-based index

```rust
use dirk_events::Event;

#[derive(Debug, Clone, Event)]
#[event("key pressed: code={0}")]
struct KeyPressed(u32);

assert_eq!(KeyPressed(65).debug(), "key pressed: code=65");
```

### Enum — annotate each variant individually

```rust
use dirk_events::Event;

#[derive(Debug, Clone, Event)]
enum NetworkEvent {
    #[event("connected to {host}:{port}")]
    Connected { host: String, port: u16 },

    #[event("disconnected")]
    Disconnected,

    #[event("packet received: {0} bytes")]
    PacketReceived(usize),
}

assert_eq!(
    NetworkEvent::Connected { host: "example.com".into(), port: 443 }.debug(),
    "connected to example.com:443",
);
assert_eq!(NetworkEvent::Disconnected.debug(), "disconnected");
assert_eq!(NetworkEvent::PacketReceived(512).debug(), "packet received: 512 bytes");
```

### Partial field references

You do **not** have to reference every field. Unreferenced fields are simply
ignored; the macro handles the binding silently.

```rust
use dirk_events::Event;

#[derive(Debug, Clone, Event)]
#[event("x only: {x}")]
struct Position { x: i32, y: i32, z: i32 }

assert_eq!(Position { x: 5, y: 99, z: 0 }.debug(), "x only: 5");
```

## Sharing the Manager Across Systems

[`EventManager`] is cheaply cloneable — every clone shares the **same**
underlying state via `Arc<Mutex<…>>`. Pass it by value (or clone it freely)
into as many systems as you like:

```rust
use dirk_events::EventManager;

let mgr = EventManager::new();

let input_system_mgr  = mgr.clone();
let render_system_mgr = mgr.clone();

// All three handles point to the same bus.
```

## Multiple Dispatchers for the Same Type

Several systems can independently produce events of the same type. All of
their events are delivered to all subscribers in the next `dispatch_all`.

```rust
use dirk_events::{EventManager, Event};

#[derive(Debug, Clone, Event)]
struct DamageEvent(u32);

let mgr = EventManager::new();
let d1 = mgr.register::<DamageEvent>(); // melee system
let d2 = mgr.register::<DamageEvent>(); // projectile system
let mut consumer = mgr.subscribe::<DamageEvent>();

d1.dispatch(DamageEvent(10));
d2.dispatch(DamageEvent(25));
mgr.dispatch_all();

let total: u32 = consumer.consume_all().map(|e| e.0).sum();
assert_eq!(total, 35);
```

## Cloning Dispatchers and Consumers

Cloning a [`Dispatcher`] registers a **new, independent producer** with the
same manager. Cloning a [`Consumer`] creates a **fresh, independent
subscription** — it does not share the receiver of the original.

```rust
use dirk_events::{EventManager, Event};

#[derive(Debug, Clone, Event)]
struct Ping;

let mgr  = EventManager::new();
let d1   = mgr.register::<Ping>();
let d2   = d1.clone(); // independent dispatcher
let mut c1   = mgr.subscribe::<Ping>();
let mut c2   = c1.clone(); // independent consumer — its own subscription

d1.dispatch(Ping);
d2.dispatch(Ping);
mgr.dispatch_all();

assert_eq!(c1.consume_all().count(), 2); // receives from both dispatchers
assert_eq!(c2.consume_all().count(), 2);
```

## Thread Safety

[`Dispatcher`] is [`Send`], so it can be moved into background threads to
produce events off the main thread. [`EventManager::dispatch_all`] is
typically called from the main game-loop thread.

```rust
use dirk_events::{EventManager, Event};
use std::thread;

#[derive(Debug, Clone, Event)]
struct WorkDone(u32);

let mgr = EventManager::new();
let dispatcher = mgr.register::<WorkDone>();
let mut consumer   = mgr.subscribe::<WorkDone>();

thread::spawn(move || {
    dispatcher.dispatch(WorkDone(1));
    dispatcher.dispatch(WorkDone(2));
}).join().unwrap();

mgr.dispatch_all();
assert_eq!(consumer.consume_all().count(), 2);
```

[`dispatch_all`]: EventManager::dispatch_all
