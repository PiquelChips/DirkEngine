#![doc = include_str!("../README.md")]

use parking_lot::Mutex;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{
        Arc,
        mpsc::{self, Receiver, Sender},
    },
};
use tracing::trace;
mod tests;

/// The marker trait that every event type must implement.
///
/// In practice you will almost never implement this by hand — use the
/// `#[derive(Event)]` proc-macro instead, which also gives you the
/// `#[event("…")]` format-string attribute.
///
/// # Requirements
///
/// | Bound | Reason |
/// |-------|--------|
/// | [`Send`] | Events may be queued from background threads |
/// | [`Clone`] | Each subscriber receives its own independent copy |
/// | `'static` | Events are stored inside type-erased trait objects |
///
/// # Manual Implementation
///
/// ```rust
/// use events::Event;
///
/// #[derive(Clone)]
/// struct MyEvent { value: i32 }
///
/// impl Event for MyEvent {
///     fn debug(&self) -> String {
///         format!("MyEvent({})", self.value)
///     }
/// }
/// ```
pub trait Event: Send + Clone + 'static {
    /// Returns a human-readable description of this event instance, used by
    /// the internal tracing instrumentation.
    ///
    /// When you use `#[derive(Event)]` this is generated automatically from the
    /// optional `#[event("…")]` format string (or falls back to `{self:?}`).
    fn debug(&self) -> String;
}

#[doc(hidden)]
pub use macros::Event;

/// Private inner state, held behind the `Arc<Mutex<>>`.
#[derive(Default)]
struct EventManagerInner {
    producers: Vec<Box<dyn AnyProducer>>,
    subscribers: HashMap<TypeId, Vec<Subscriber>>,
}

/// The central event bus.
///
/// `EventManager` owns the internal channel infrastructure and is responsible
/// for forwarding buffered events to the right subscribers on every call to
/// [`dispatch_all`].
///
/// # Cloning
///
/// `EventManager` is **cheaply cloneable** — all clones share the same
/// underlying state through an `Arc<Mutex<…>>`. Clone it freely and pass it
/// into every system that needs to produce or consume events.
///
/// ```rust
/// use events::EventManager;
///
/// let mgr_a = EventManager::new();
/// let mgr_b = mgr_a.clone(); // same bus, different handle
/// ```
///
/// # Frame Loop Integration
///
/// ```rust
/// use events::EventManager;
///
/// let mgr = EventManager::new();
///
/// loop {
///     // … collect input, run systems …
///
///     // Forward all queued events to subscribers.
///     mgr.dispatch_all();
///
///     // … render …
///     # break; // stop the doc-test loop
/// }
/// ```
///
/// [`dispatch_all`]: EventManager::dispatch_all
#[derive(Clone, Default)]
pub struct EventManager {
    inner: Arc<Mutex<EventManagerInner>>,
}

impl EventManager {
    /// Creates a new, empty event manager.
    ///
    /// Equivalent to [`EventManager::default`].
    ///
    /// ```rust
    /// use events::EventManager;
    ///
    /// let mgr = EventManager::new();
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new event type and returns a [`Dispatcher`] for it.
    ///
    /// Every call to `register` creates an **independent** producer channel.
    /// Multiple dispatchers for the same event type are fully supported — their
    /// events are all forwarded on the next [`dispatch_all`].
    ///
    /// # Type Inference
    ///
    /// The event type can be inferred from context or specified with the
    /// turbofish syntax:
    ///
    /// ```rust
    /// use events::{EventManager, Dispatcher, Event};
    ///
    /// #[derive(Debug, Clone, Event)]
    /// struct MyEvent;
    ///
    /// let mgr = EventManager::new();
    ///
    /// // Turbofish syntax:
    /// let d1 = mgr.register::<MyEvent>();
    ///
    /// // Type annotation on the binding:
    /// let d2: Dispatcher<MyEvent> = mgr.register();
    /// ```
    ///
    /// [`dispatch_all`]: EventManager::dispatch_all
    #[must_use]
    pub fn register<T: Event>(&self) -> Dispatcher<T> {
        let (sender, receiver) = mpsc::channel::<T>();
        self.inner.lock().producers.push(Box::new(TypedProducer {
            type_id: TypeId::of::<T>(),
            receiver,
        }));
        Dispatcher {
            sender,
            manager: self.clone(),
        }
    }

    /// Subscribes to an event type and returns a [`Consumer`] for it.
    ///
    /// Every call to `subscribe` creates an **independent** subscription. Each
    /// consumer receives its own clone of every event dispatched for that type,
    /// regardless of how many other consumers exist.
    ///
    /// A consumer can be created before or after a dispatcher is registered for
    /// the same type.
    ///
    /// # Type Inference
    ///
    /// ```rust
    /// use events::{EventManager, Consumer, Event};
    ///
    /// #[derive(Debug, Clone, Event)]
    /// struct MyEvent;
    ///
    /// let mgr = EventManager::new();
    ///
    /// // Turbofish syntax:
    /// let c1 = mgr.subscribe::<MyEvent>();
    ///
    /// // Type annotation on the binding:
    /// let c2: Consumer<MyEvent> = mgr.subscribe();
    /// ```
    #[must_use]
    pub fn subscribe<T: Event>(&self) -> Consumer<T> {
        let (sender, receiver) = mpsc::channel::<T>();
        let type_id = TypeId::of::<T>();
        self.inner
            .lock()
            .subscribers
            .entry(type_id)
            .or_default()
            .push(Subscriber {
                sender: Box::new(sender),
            });
        Consumer {
            receiver,
            manager: self.clone(),
        }
    }

    /// Drains all registered producers and forwards their pending events to
    /// every matching subscriber.
    ///
    /// **Call this exactly once per frame / tick** from your main engine loop.
    /// Events queued via [`Dispatcher::dispatch`] are invisible to consumers
    /// until this method is called.
    ///
    /// Dropped consumers are silently pruned during this call — no panic, no
    /// memory leak.
    ///
    /// # Behaviour Summary
    ///
    /// * Iterates over all registered producers in registration order.
    /// * For each producer, drains all pending events from its internal channel.
    /// * Clones each event and sends it to every live subscriber of that type.
    /// * Removes dead subscribers (those whose [`Consumer`] has been dropped).
    ///
    /// # Example — Two-Frame Simulation
    ///
    /// ```rust
    /// use events::{EventManager, Event};
    ///
    /// #[derive(Debug, Clone, Event)]
    /// struct TickEvent(u32);
    ///
    /// let mgr = EventManager::new();
    /// let dispatcher = mgr.register::<TickEvent>();
    /// let consumer   = mgr.subscribe::<TickEvent>();
    ///
    /// // Frame 1
    /// dispatcher.dispatch(TickEvent(1));
    /// mgr.dispatch_all();
    /// assert_eq!(consumer.try_consume().unwrap().0, 1);
    ///
    /// // Frame 2
    /// dispatcher.dispatch(TickEvent(2));
    /// mgr.dispatch_all();
    /// assert_eq!(consumer.try_consume().unwrap().0, 2);
    /// ```
    pub fn dispatch_all(&self) {
        let mut inner = self.inner.lock();
        let EventManagerInner {
            producers,
            subscribers,
        } = &mut *inner;
        for producer in producers.iter() {
            producer.forward_pending(subscribers);
        }
    }
}

/// The Type Erasure Trait.
/// Allows the `EventManager` to forward pending events without knowing `T`.
trait AnyProducer: Send {
    fn forward_pending(&self, subscribers: &mut HashMap<TypeId, Vec<Subscriber>>);
}

/// A producer is an object that will be queried for events.
/// On event collection, the event manager loops through every
/// producer and collects pending events.
///
/// This is a typed producer as it stores the actual type of the
/// event it is producing. It is then wrapped by the [`AnyProducer`]
/// trait to hide the event type from the event manager.
struct TypedProducer<T: Event> {
    type_id: TypeId,
    receiver: Receiver<T>,
}

impl<T: Event> AnyProducer for TypedProducer<T> {
    fn forward_pending(&self, subscribers: &mut HashMap<TypeId, Vec<Subscriber>>) {
        let Some(subscribers) = subscribers.get_mut(&self.type_id) else {
            return;
        };
        while let Ok(event) = self.receiver.try_recv() {
            subscribers.retain(|sub| {
                let Some(sender) = sub.sender.downcast_ref::<Sender<T>>() else {
                    return true;
                };
                sender.send(event.clone()).is_ok()
            });
        }
    }
}

/// A subscriber is a sender for events.
/// On event dispatching, will send the events through the
/// channels of every subscriber.
struct Subscriber {
    sender: Box<dyn Any + Send>,
}

/// Queues events to be forwarded to subscribers on the next
/// [`EventManager::dispatch_all`].
///
/// Created by [`EventManager::register`]. Cheaply shareable — pass it by clone
/// into any number of systems.
///
/// # Cloning
///
/// Cloning a `Dispatcher` registers a **new, independent producer** with the
/// same [`EventManager`]. This means the clone and the original each have their
/// own internal channel, but both sets of events are delivered to all
/// subscribers on the next `dispatch_all`.
///
/// ```rust
/// use events::{EventManager, Event};
///
/// #[derive(Debug, Clone, Event)]
/// struct Hit(u32);
///
/// let mgr = EventManager::new();
/// let d1  = mgr.register::<Hit>();
/// let d2  = d1.clone(); // independent producer
/// let c   = mgr.subscribe::<Hit>();
///
/// d1.dispatch(Hit(1));
/// d2.dispatch(Hit(2));
/// mgr.dispatch_all();
///
/// let hits: Vec<u32> = c.consume_all().map(|h| h.0).collect();
/// assert_eq!(hits.len(), 2);
/// ```
pub struct Dispatcher<T: Event> {
    sender: Sender<T>,
    manager: EventManager,
}

impl<T: Event> Dispatcher<T> {
    /// Queues `event` to be forwarded to all subscribers on the next call to
    /// [`EventManager::dispatch_all`].
    ///
    /// This method is non-blocking and returns immediately. The event is not
    /// visible to consumers until `dispatch_all` is called.
    ///
    /// # Example
    ///
    /// ```rust
    /// use events::{EventManager, Event};
    ///
    /// #[derive(Debug, Clone, Event)]
    /// #[event("enemy spawned at ({x}, {y})")]
    /// struct EnemySpawned { x: f32, y: f32 }
    ///
    /// let mgr = EventManager::new();
    /// let dispatcher = mgr.register::<EnemySpawned>();
    /// let consumer   = mgr.subscribe::<EnemySpawned>();
    ///
    /// // Queued — not yet visible.
    /// dispatcher.dispatch(EnemySpawned { x: 10.0, y: 20.0 });
    /// assert!(consumer.try_consume().is_none());
    ///
    /// // Now forwarded.
    /// mgr.dispatch_all();
    /// assert!(consumer.try_consume().is_some());
    /// ```
    pub fn dispatch(&self, event: T) {
        trace!("dispatching event {}", event.debug());
        let _ = self.sender.send(event);
    }
}

impl<T: Event> Clone for Dispatcher<T> {
    /// Creates a **new, independent** dispatcher registered with the same
    /// [`EventManager`]. See the [type-level docs](Dispatcher) for details.
    fn clone(&self) -> Self {
        self.manager.register()
    }
}

impl<T: Event> std::fmt::Debug for Dispatcher<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dispatcher").finish_non_exhaustive()
    }
}

/// Receives events forwarded by [`EventManager::dispatch_all`].
///
/// Created by [`EventManager::subscribe`]. Each `Consumer` holds an independent
/// subscription — it is **not** shared with other consumers of the same type.
///
/// # Cloning
///
/// Cloning a `Consumer` creates a **fresh subscription** backed by the same
/// [`EventManager`]. The clone starts empty and receives events dispatched
/// *after* it was created; it does **not** inherit any events already buffered
/// in the original.
///
/// ```rust
/// use events::{EventManager, Event};
///
/// #[derive(Debug, Clone, Event)]
/// struct Signal;
///
/// let mgr = EventManager::new();
/// let d   = mgr.register::<Signal>();
/// let c1  = mgr.subscribe::<Signal>();
/// let c2  = c1.clone(); // fresh, independent subscription
///
/// d.dispatch(Signal);
/// mgr.dispatch_all();
///
/// assert_eq!(c1.consume_all().count(), 1);
/// assert_eq!(c2.consume_all().count(), 1); // independent — also gets the event
/// ```
///
/// # Dropping
///
/// When a `Consumer` is dropped its subscription is automatically removed on
/// the next [`EventManager::dispatch_all`]. No explicit unsubscribe call is
/// needed.
pub struct Consumer<T: Event> {
    receiver: Receiver<T>,
    manager: EventManager,
}

impl<T: Event> Consumer<T> {
    /// Returns the **next** pending event, or `None` if the queue is currently
    /// empty.
    ///
    /// This is non-blocking. Use [`consume_all`] if you want to drain every
    /// event that arrived this tick.
    ///
    /// # Example
    ///
    /// ```rust
    /// use events::{EventManager, Event};
    ///
    /// #[derive(Debug, Clone, Event)]
    /// struct Counter(u32);
    ///
    /// let mgr = EventManager::new();
    /// let d   = mgr.register::<Counter>();
    /// let c   = mgr.subscribe::<Counter>();
    ///
    /// d.dispatch(Counter(1));
    /// d.dispatch(Counter(2));
    /// mgr.dispatch_all();
    ///
    /// assert_eq!(c.try_consume().unwrap().0, 1);
    /// assert_eq!(c.try_consume().unwrap().0, 2);
    /// assert!(c.try_consume().is_none()); // queue is now empty
    /// ```
    ///
    /// [`consume_all`]: Consumer::consume_all
    pub fn try_consume(&self) -> Option<T> {
        let res = self.receiver.try_recv().ok();
        if let Some(event) = res.clone() {
            trace!("consuming {}", event.debug());
        }
        res
    }

    /// Returns a lazy iterator that **drains all currently pending events**.
    ///
    /// The iterator calls [`try_consume`] repeatedly and stops as soon as the
    /// queue is empty. Collect it into a `Vec` or drive it with a `for` loop.
    ///
    /// # Example — Collect into a Vec
    ///
    /// ```rust
    /// use events::{EventManager, Event};
    ///
    /// #[derive(Debug, Clone, Event)]
    /// struct Score(u32);
    ///
    /// let mgr = EventManager::new();
    /// let d   = mgr.register::<Score>();
    /// let c   = mgr.subscribe::<Score>();
    ///
    /// for i in 0..5 { d.dispatch(Score(i)); }
    /// mgr.dispatch_all();
    ///
    /// let scores: Vec<u32> = c.consume_all().map(|s| s.0).collect();
    /// assert_eq!(scores, vec![0, 1, 2, 3, 4]);
    /// ```
    ///
    /// # Example — `for` Loop
    ///
    /// ```rust
    /// use events::{EventManager, Event};
    ///
    /// #[derive(Debug, Clone, Event)]
    /// #[event("damage={0}")]
    /// struct DamageEvent(u32);
    ///
    /// let mgr = EventManager::new();
    /// let d   = mgr.register::<DamageEvent>();
    /// let c   = mgr.subscribe::<DamageEvent>();
    ///
    /// d.dispatch(DamageEvent(10));
    /// d.dispatch(DamageEvent(5));
    /// mgr.dispatch_all();
    ///
    /// let mut total_damage = 0u32;
    /// for event in c.consume_all() {
    ///     total_damage += event.0;
    /// }
    /// assert_eq!(total_damage, 15);
    /// ```
    ///
    /// [`try_consume`]: Consumer::try_consume
    pub fn consume_all(&self) -> impl Iterator<Item = T> {
        std::iter::from_fn(|| self.try_consume())
    }
}

impl<T: Event> Clone for Consumer<T> {
    /// Creates a **fresh, independent subscription** with the same
    /// [`EventManager`]. See the [type-level docs](Consumer) for details.
    fn clone(&self) -> Self {
        self.manager.subscribe()
    }
}

impl<T: Event> std::fmt::Debug for Consumer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Consumer").finish_non_exhaustive()
    }
}

/// An event to request to the engine to exit.
///
/// This event is used by various engine systems.
/// It can be used by the platform to signal to the engine that the windows
/// have all been closed. It is also used when users manually exit the engine.
#[derive(Debug, Clone, Event)]
#[event("App exit requested: {0}")]
pub struct AppExit(pub String);
