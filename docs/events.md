# Events — Usage Guide

The `events` crate provides a lightweight, type-safe, **channel-backed pub/sub
event bus** built for game-engine frame loops. Events are buffered between
frames and forwarded to all subscribers in a single, explicit flush.

---

## Table of Contents

1. [Core Types](#1-core-types)
2. [Defining an Event](#2-defining-an-event)
   - 2.1 [Structs](#21-structs)
   - 2.2 [Enums](#22-enums)
   - 2.3 [Partial Field References](#23-partial-field-references)
   - 2.4 [No Attribute — Debug Fallback](#24-no-attribute--debug-fallback)
3. [Wiring Up the Bus](#3-wiring-up-the-bus)
4. [The Frame Loop](#4-the-frame-loop)
5. [Reading Events](#5-reading-events)
6. [Fan-Out: Multiple Subscribers](#6-fan-out-multiple-subscribers)
7. [Multiple Dispatchers for the Same Type](#7-multiple-dispatchers-for-the-same-type)
8. [Sharing the Manager Across Systems](#8-sharing-the-manager-across-systems)
9. [Cloning Dispatchers and Consumers](#9-cloning-dispatchers-and-consumers)
10. [Thread Safety](#10-thread-safety)
11. [Dropped Consumers](#11-dropped-consumers)
12. [Complete Example — Mini Engine](#12-complete-example--mini-engine)
13. [Behaviour Reference](#13-behaviour-reference)

---

## 1. Core Types

| Type | Created by | Role |
|------|-----------|------|
| `EventManager` | `EventManager::new()` | Central bus; call `dispatch_all()` once per frame |
| `Dispatcher<T>` | `mgr.register::<T>()` | Queues events of type `T` |
| `Consumer<T>` | `mgr.subscribe::<T>()` | Reads events of type `T` that arrived this tick |

All three are cheaply cloneable and share the same underlying state.

---

## 2. Defining an Event

Add `#[derive(Debug, Clone, Event)]` to any type. The `Event` derive macro
handles everything. An optional `#[event("…")]` attribute controls the
string returned by `Event::debug()`, which appears in tracing logs.

### 2.1 Structs

**Named fields** — reference fields by name inside the format string:

```rust
use events::Event;

#[derive(Debug, Clone, Event)]
#[event("entity {id} moved to ({x:.1}, {y:.1})")]
struct EntityMoved {
    id: u64,
    x: f32,
    y: f32,
}
```

**Tuple (unnamed) fields** — reference fields by zero-based index:

```rust
use events::Event;

#[derive(Debug, Clone, Event)]
#[event("key pressed: code={0}")]
struct KeyPressed(u32);

#[derive(Debug, Clone, Event)]
#[event("error {0}: {1}")]
struct ErrorEvent(u32, String);
```

**Unit struct** — no fields; a static message is enough:

```rust
use events::Event;

#[derive(Debug, Clone, Event)]
#[event("application shutdown requested")]
struct ShutdownRequested;
```

### 2.2 Enums

Annotate **each variant** individually. Variants can mix unit, named-field, and
unnamed-field forms freely:

```rust
use events::Event;

#[derive(Debug, Clone, Event)]
enum NetworkEvent {
    #[event("connected to {host}:{port}")]
    Connected { host: String, port: u16 },

    #[event("disconnected")]
    Disconnected,

    #[event("packet received: {0} bytes")]
    PacketReceived(usize),
}
```

```rust
# use events::Event;
# #[derive(Debug, Clone, Event)]
# enum NetworkEvent {
#     #[event("connected to {host}:{port}")]
#     Connected { host: String, port: u16 },
#     #[event("disconnected")]
#     Disconnected,
#     #[event("packet received: {0} bytes")]
#     PacketReceived(usize),
# }
assert_eq!(
    NetworkEvent::Connected { host: "game.server.io".into(), port: 7777 }.debug(),
    "connected to game.server.io:7777",
);
assert_eq!(NetworkEvent::PacketReceived(1024).debug(), "packet received: 1024 bytes");
```

### 2.3 Partial Field References

You don't have to reference every field — unreferenced ones are silently ignored
by the macro:

```rust
use events::Event;

#[derive(Debug, Clone, Event)]
#[event("x={x}")]           // y and z are present but not shown
struct Position3D { x: f32, y: f32, z: f32 }
```

This works identically for unnamed fields:

```rust
use events::Event;

#[derive(Debug, Clone, Event)]
#[event("first={0}")]       // fields 1 and 2 are ignored
struct Triple(i32, i32, i32);
```

### 2.4 No Attribute — Debug Fallback

Omitting `#[event("…")]` makes `debug()` fall back to `format!("{self:?}")`,
so `#[derive(Debug)]` must also be present:

```rust
use events::Event;

#[derive(Debug, Clone, Event)]   // no #[event(...)] attribute
struct RawPacket { id: u32, payload: Vec<u8> }

// debug() returns something like: RawPacket { id: 1, payload: [...] }
```

---

## 3. Wiring Up the Bus

```rust
use events::{EventManager, Event};

#[derive(Debug, Clone, Event)]
#[event("player scored {points} points")]
struct PlayerScored { points: u32 }

// Create the shared bus — one per application.
let mgr = EventManager::new();

// Producer side: call register() once per system that will emit this event.
let dispatcher = mgr.register::<PlayerScored>();

// Consumer side: call subscribe() once per system that will react to this event.
let consumer = mgr.subscribe::<PlayerScored>();
```

Subscriptions and registrations can happen in **any order** and at any time,
including after the loop has started.

---

## 4. The Frame Loop

Call `dispatch_all()` **exactly once per frame** to move buffered events to
subscribers:

```rust
# use events::EventManager;
# let mgr = EventManager::new();
loop {
    // --- Update phase: systems queue events via their Dispatcher ---
    // (dispatcher.dispatch(…) calls happen inside system update functions)

    // --- Dispatch phase: flush all queued events to consumers ---
    mgr.dispatch_all();

    // --- Read phase: systems read events via their Consumer ---
    // (consumer.consume_all() / consumer.try_consume() calls happen here)

    # break;
}
```

Events queued **before** `dispatch_all` are invisible to consumers. Events
queued **after** `dispatch_all` (i.e. during the read phase or next frame's
update phase) wait until the *next* `dispatch_all`.

---

## 5. Reading Events

### `try_consume` — one at a time

```rust
# use events::EventManager;
# use events::Event;
# #[derive(Debug, Clone, Event)] struct MyEvent(u32);
# let mgr = EventManager::new();
# let dispatcher = mgr.register::<MyEvent>();
# let consumer = mgr.subscribe::<MyEvent>();
# dispatcher.dispatch(MyEvent(1));
# mgr.dispatch_all();
// Returns Some(event) or None — never blocks.
while let Some(event) = consumer.try_consume() {
    println!("got: {}", event.debug());
}
```

### `consume_all` — drain everything

```rust
# use events::EventManager;
# use events::Event;
# #[derive(Debug, Clone, Event)] struct DamageEvent(u32);
# let mgr = EventManager::new();
# let dispatcher = mgr.register::<DamageEvent>();
# let consumer = mgr.subscribe::<DamageEvent>();
# for i in 0..3 { dispatcher.dispatch(DamageEvent(i * 5)); }
# mgr.dispatch_all();
let total_damage: u32 = consumer.consume_all().map(|e| e.0).sum();
println!("total damage this frame: {}", total_damage);
```

Both methods are **non-blocking**.

---

## 6. Fan-Out: Multiple Subscribers

Every active `Consumer` receives an **independent clone** of each event — there
is no competition between consumers:

```rust
use events::{EventManager, Event};

#[derive(Debug, Clone, Event)]
#[event("collision: entity {a} hit entity {b}")]
struct CollisionEvent { a: u64, b: u64 }

let mgr = EventManager::new();
let dispatcher = mgr.register::<CollisionEvent>();

// Physics system consumer
let physics_consumer = mgr.subscribe::<CollisionEvent>();
// Audio system consumer
let audio_consumer = mgr.subscribe::<CollisionEvent>();
// Scoring system consumer
let score_consumer = mgr.subscribe::<CollisionEvent>();

dispatcher.dispatch(CollisionEvent { a: 1, b: 2 });
mgr.dispatch_all();

// All three receive the event independently.
assert_eq!(physics_consumer.consume_all().count(), 1);
assert_eq!(audio_consumer.consume_all().count(), 1);
assert_eq!(score_consumer.consume_all().count(), 1);
```

---

## 7. Multiple Dispatchers for the Same Type

Different systems can each hold their own `Dispatcher` for the same event type.
All their events are collected on `dispatch_all` and delivered to every
subscriber:

```rust
use events::{EventManager, Event};

#[derive(Debug, Clone, Event)]
#[event("damage: {0} hp")]
struct DamageEvent(u32);

let mgr = EventManager::new();

// Each system registers independently.
let melee_dispatcher      = mgr.register::<DamageEvent>();
let projectile_dispatcher = mgr.register::<DamageEvent>();
let explosion_dispatcher  = mgr.register::<DamageEvent>();

let health_consumer = mgr.subscribe::<DamageEvent>();

melee_dispatcher.dispatch(DamageEvent(10));
projectile_dispatcher.dispatch(DamageEvent(25));
explosion_dispatcher.dispatch(DamageEvent(50));
mgr.dispatch_all();

let total: u32 = health_consumer.consume_all().map(|e| e.0).sum();
assert_eq!(total, 85);
```

---

## 8. Sharing the Manager Across Systems

`EventManager` is an `Arc`-backed handle — cloning it is `O(1)` and all clones
share the same internal state:

```rust
use events::EventManager;

let mgr = EventManager::new();

// Hand out clones to every system at startup.
let input_mgr   = mgr.clone();
let render_mgr  = mgr.clone();
let physics_mgr = mgr.clone();
let audio_mgr   = mgr.clone();

// Each system calls register() or subscribe() on its local clone.
// dispatch_all() can be called on any clone — it affects the shared bus.
mgr.dispatch_all(); // equivalent to calling it on any of the clones above
```

---

## 9. Cloning Dispatchers and Consumers

### Cloning a `Dispatcher`

Produces a **new, independent producer** registered on the same bus. Useful when
the same system struct needs to be cloned (e.g. passed to worker threads):

```rust
# use events::EventManager;
# use events::Event;
# #[derive(Debug, Clone, Event)] struct Ping;
# let mgr = EventManager::new();
let d1 = mgr.register::<Ping>();
let d2 = d1.clone(); // independent channel, same bus

// Both dispatchers contribute events to the same set of consumers.
```

### Cloning a `Consumer`

Produces a **fresh subscription** — it does **not** share the original's
internal queue. Events dispatched after cloning arrive in both independently:

```rust
# use events::EventManager;
# use events::Event;
# #[derive(Debug, Clone, Event)] struct Ping;
# let mgr = EventManager::new();
# let d = mgr.register::<Ping>();
let c1 = mgr.subscribe::<Ping>();
let c2 = c1.clone(); // independent subscription, starts empty

d.dispatch(Ping);
mgr.dispatch_all();

assert_eq!(c1.consume_all().count(), 1);
assert_eq!(c2.consume_all().count(), 1); // both got it
```

---

## 10. Thread Safety

`Dispatcher<T>` is `Send`, so it can be moved into background threads:

```rust
use events::{EventManager, Event};
use std::thread;

#[derive(Debug, Clone, Event)]
#[event("asset loaded: {0}")]
struct AssetLoaded(String);

let mgr = EventManager::new();
let dispatcher = mgr.register::<AssetLoaded>();
let consumer   = mgr.subscribe::<AssetLoaded>();

// Move the dispatcher into a worker thread.
let handle = thread::spawn(move || {
    dispatcher.dispatch(AssetLoaded("player.mesh".into()));
    dispatcher.dispatch(AssetLoaded("terrain.png".into()));
});

handle.join().unwrap();

// Back on the main thread, flush and read.
mgr.dispatch_all();
assert_eq!(consumer.consume_all().count(), 2);
```

> **Note:** `dispatch_all` acquires the internal mutex. It should be called
> from a single thread (typically the main loop thread) to avoid contention.

---

## 11. Dropped Consumers

No unsubscribe call is necessary. When a `Consumer` is dropped its channel
receiver closes. On the next `dispatch_all` the dead entry is silently pruned
from the subscriber list:

```rust
# use events::EventManager;
# use events::Event;
# #[derive(Debug, Clone, Event)] struct Tick;
# let mgr = EventManager::new();
# let dispatcher = mgr.register::<Tick>();
let alive = mgr.subscribe::<Tick>();

{
    let _transient = mgr.subscribe::<Tick>(); // dropped at end of block
}

// Safe — the dead subscription is pruned without a panic.
dispatcher.dispatch(Tick);
mgr.dispatch_all();

assert_eq!(alive.consume_all().count(), 1);
```

---

## 12. Complete Example — Mini Engine

```rust
use events::{Consumer, Dispatcher, Event, EventManager};

// ── Event types ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Event)]
#[event("key={0}")]
struct KeyPressed(u32);

#[derive(Debug, Clone, Event)]
#[event("window resized to {width}×{height} px")]
struct WindowResized { width: u32, height: u32 }

#[derive(Debug, Clone, Event)]
#[event("shutdown requested")]
struct ShutdownRequested;

// ── System stubs ───────────────────────────────────────────────────────────

struct InputSystem {
    dispatcher: Dispatcher<KeyPressed>,
    shutdown:   Dispatcher<ShutdownRequested>,
}

impl InputSystem {
    fn update(&self, frame: u32) {
        self.dispatcher.dispatch(KeyPressed(65 + frame));
        if frame == 2 {
            self.shutdown.dispatch(ShutdownRequested);
        }
    }
}

struct RenderSystem {
    win_consumer: Consumer<WindowResized>,
}

impl RenderSystem {
    fn update(&self) {
        for event in self.win_consumer.consume_all() {
            println!("render: adapting to {}", event.debug());
        }
    }
}

// ── Engine loop ────────────────────────────────────────────────────────────

fn main() {
    let mgr = EventManager::new();

    let input = InputSystem {
        dispatcher: mgr.register::<KeyPressed>(),
        shutdown:   mgr.register::<ShutdownRequested>(),
    };
    let render = RenderSystem {
        win_consumer: mgr.subscribe::<WindowResized>(),
    };

    let key_consumer      = mgr.subscribe::<KeyPressed>();
    let shutdown_consumer = mgr.subscribe::<ShutdownRequested>();
    let win_dispatcher    = mgr.register::<WindowResized>();

    for frame in 0u32..10 {
        // --- Update ---
        input.update(frame);
        if frame == 1 {
            win_dispatcher.dispatch(WindowResized { width: 1920, height: 1080 });
        }

        // --- Flush ---
        mgr.dispatch_all();

        // --- Read ---
        render.update();
        for key in key_consumer.consume_all() {
            println!("input: {}", key.debug());
        }
        if shutdown_consumer.try_consume().is_some() {
            println!("shutting down after frame {}", frame);
            break;
        }
    }
}
```

---

## 13. Behaviour Reference

| Scenario | Result |
|----------|--------|
| `dispatch` before `dispatch_all` | Event is buffered; consumers see nothing yet |
| `dispatch_all` with no subscribers | No-op; no panic |
| `dispatch_all` with no events queued | No-op; no panic |
| `dispatch_all` called repeatedly per frame | Each call drains only what was queued since the last call |
| Consumer dropped before `dispatch_all` | Pruned silently on next `dispatch_all` |
| Multiple consumers, same type | Each receives an independent clone of every event |
| Multiple dispatchers, same type | All events from all dispatchers reach all consumers |
| Clone a `Dispatcher` | New independent producer on the same bus |
| Clone a `Consumer` | Fresh independent subscription on the same bus |
| `Dispatcher` sent to another thread | Fully supported (`Send` bound) |
