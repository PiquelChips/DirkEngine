#![doc = include_str!("../README.md")]

use parking_lot::RwLock;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tracing::trace;

use dirk_threads::WorkerPool;

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
/// use dirk_events::Event;
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
pub use dirk_proc::Event;

/// The central event bus.
///
/// `EventManager` owns the internal channel infrastructure and is responsible
/// for routing dispatched events to the right subscribers on background worker
/// threads.
///
/// # Cloning
///
/// `EventManager` is **cheaply cloneable** — all clones share the same
/// underlying state through an `Arc<Mutex<…>>`. Clone it freely and pass it
/// into every system that needs to produce or consume events.
///
/// ```rust
/// use dirk_events::EventManager;
///
/// let mgr_a = EventManager::new();
/// let mgr_b = mgr_a.clone(); // same bus, different handle
/// ```
///
/// # Frame Loop Integration
///
/// ```rust
/// use dirk_events::EventManager;
///
/// let mgr = EventManager::new();
///
/// loop {
///     // … collect input, run systems …
///
///     // Wait for events dispatched so far to finish routing.
///     mgr.dispatch_all();
///
///     // … render …
///     # break; // stop the doc-test loop
/// }
/// ```
///
/// [`dispatch_all`]: EventManager::dispatch_all
#[derive(Clone)]
pub struct EventManager {
    /// We only store the producers to be able to sync them
    /// with [`dispatch_all`].
    ///
    /// [`dispatch_all`]: EventManager::dispatch_all
    producers: Arc<RwLock<Vec<Box<dyn AnyProducer>>>>,
    subscribers: Arc<RwLock<HashMap<TypeId, Vec<Subscriber>>>>, // TODO: see about better HashMap where we can lock just the value instead of entire thing
    workers: WorkerPool,
}

impl EventManager {
    /// Creates a new, empty event manager.
    ///
    /// Equivalent to [`EventManager::default`].
    ///
    /// ```rust
    /// use dirk_events::EventManager;
    ///
    /// let mgr = EventManager::new();
    /// ```
    #[must_use]
    pub fn new(workers: WorkerPool) -> Self {
        Self {
            producers: Arc::default(),
            subscribers: Arc::default(),
            workers,
        }
    }

    /// Registers a new event type and returns a [`Dispatcher`] for it.
    ///
    /// Every call to `register` creates an **independent** producer channel and
    /// a matching background routing task. Multiple dispatchers for the same
    /// event type are fully supported.
    ///
    /// # Type Inference
    ///
    /// The event type can be inferred from context or specified with the
    /// turbofish syntax:
    ///
    /// ```rust
    /// use dirk_events::{EventManager, Dispatcher, Event};
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
        let (sender, receiver) = mpsc::unbounded_channel::<ProducerMessage<T>>();
        self.producers.write().push(Box::new(TypedProducer {
            sender: sender.clone(),
        }));
        self.spawn_router(receiver);
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
    /// use dirk_events::{EventManager, Consumer, Event};
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
        let (sender, receiver) = mpsc::unbounded_channel::<T>();
        let type_id = TypeId::of::<T>();
        self.subscribers
            .write()
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

    /// Waits until all events queued before this call have been forwarded to
    /// their subscribers.
    ///
    /// This remains useful as a frame barrier for systems that want to observe
    /// a quiescent event state, but delivery itself happens immediately on
    /// background worker threads.
    ///
    /// Dropped consumers are silently pruned during this call — no panic, no
    /// memory leak.
    ///
    /// # Behaviour Summary
    ///
    /// * Iterates over all registered producers.
    /// * Inserts a barrier into each producer queue.
    /// * Waits until each producer has routed every event queued before that barrier.
    /// * Dead subscribers (those whose [`Consumer`] has been dropped) are pruned
    ///   during routing.
    ///
    /// # Example — Two-Frame Simulation
    ///
    /// ```rust
    /// use dirk_events::{EventManager, Event};
    ///
    /// #[derive(Debug, Clone, Event)]
    /// struct TickEvent(u32);
    ///
    /// let mgr = EventManager::new();
    /// let mut dispatcher = mgr.register::<TickEvent>();
    /// let mut consumer   = mgr.subscribe::<TickEvent>();
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
        let producers = self.producers.read();
        for producer in producers.iter() {
            producer.sync();
        }
    }

    fn spawn_router<T: Event>(&self, mut receiver: UnboundedReceiver<ProducerMessage<T>>) {
        let subscribers = Arc::clone(&self.subscribers);
        self.workers.spawn(async move {
            while let Some(message) = receiver.recv().await {
                match message {
                    ProducerMessage::Event(event) => {
                        let mut subscribers = subscribers.write();
                        let Some(listeners) = subscribers.get_mut(&TypeId::of::<T>()) else {
                            continue;
                        };

                        listeners.retain(|sub| {
                            let Some(sender) = sub.sender.downcast_ref::<UnboundedSender<T>>()
                            else {
                                // TODO: do we really want to keep what isn't being downcasted?
                                return true;
                            };
                            sender.send(event.clone()).is_ok()
                        });
                    }
                    ProducerMessage::Barrier(barrier) => {
                        let _ = barrier.send(());
                    }
                }
            }
        });
    }
}

/// The Type Erasure Trait.
/// Allows the `EventManager` to forward pending events without knowing `T`.
trait AnyProducer: Send + Sync {
    fn sync(&self);
}

/// A producer is an object that will be queried for events.
/// On event collection, the event manager loops through every
/// producer and collects pending events.
///
/// This is a typed producer as it stores the actual type of the
/// event it is producing. It is then wrapped by the [`AnyProducer`]
/// trait to hide the event type from the event manager.
struct TypedProducer<T: Event> {
    sender: UnboundedSender<ProducerMessage<T>>,
}

impl<T: Event> AnyProducer for TypedProducer<T> {
    fn sync(&self) {
        let (tx, rx) = std::sync::mpsc::channel();
        if self.sender.send(ProducerMessage::Barrier(tx)).is_ok() {
            let _ = rx.recv();
        }
    }
}

enum ProducerMessage<T: Event> {
    Event(T),
    Barrier(std::sync::mpsc::Sender<()>),
}

/// A subscriber is a sender for events.
/// On event dispatching, will send the events through the
/// channels of every subscriber.
struct Subscriber {
    sender: Box<dyn Any + Send + Sync>,
}

/// Queues events to be forwarded to subscribers by a background worker task.
///
/// Created by [`EventManager::register`]. Cheaply shareable — pass it by clone
/// into any number of systems.
///
/// # Cloning
///
/// Cloning a `Dispatcher` registers a **new, independent producer** with the
/// same [`EventManager`]. This means the clone and the original each have their
/// own internal routing channel, but both sets of events are delivered to all
/// subscribers.
///
/// ```rust
/// use dirk_events::{EventManager, Event};
///
/// #[derive(Debug, Clone, Event)]
/// struct Hit(u32);
///
/// let mgr = EventManager::new();
/// let d1  = mgr.register::<Hit>();
/// let d2  = d1.clone(); // independent producer
/// let mut c   = mgr.subscribe::<Hit>();
///
/// d1.dispatch(Hit(1));
/// d2.dispatch(Hit(2));
/// mgr.dispatch_all();
///
/// let hits: Vec<u32> = c.consume_all().map(|h| h.0).collect();
/// assert_eq!(hits.len(), 2);
/// ```
pub struct Dispatcher<T: Event> {
    sender: UnboundedSender<ProducerMessage<T>>,
    manager: EventManager,
}

impl<T: Event> Dispatcher<T> {
    /// Queues `event` to be forwarded to all subscribers as soon as a worker
    /// thread can route it.
    ///
    /// This method is non-blocking and returns immediately. The event is not
    /// routed on the caller thread.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dirk_events::{EventManager, Event};
    ///
    /// #[derive(Debug, Clone, Event)]
    /// #[event("enemy spawned at ({x}, {y})")]
    /// struct EnemySpawned { x: f32, y: f32 }
    ///
    /// let mgr = EventManager::new();
    /// let dispatcher = mgr.register::<EnemySpawned>();
    /// let mut consumer   = mgr.subscribe::<EnemySpawned>();
    ///
    /// // Routed asynchronously.
    /// dispatcher.dispatch(EnemySpawned { x: 10.0, y: 20.0 });
    /// mgr.dispatch_all();
    /// assert!(consumer.try_consume().is_some());
    /// ```
    pub fn dispatch(&self, event: T) {
        trace!("dispatching event {}", event.debug());
        let _ = self.sender.send(ProducerMessage::Event(event));
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

/// Receives events routed by background worker tasks.
///
/// Created by [`EventManager::subscribe`]. Each `Consumer` holds an independent
/// subscription — it is **not** shared with other consumers of the same type.
///
/// # Cloning
///
/// Cloning a `Consumer` creates a **fresh subscription** backed by the same
/// [`EventManager`]. The clone starts empty and receives events dispatched
/// *after* it was created; it does **not** inherit any events already queued
/// in the original.
///
/// ```rust
/// use dirk_events::{EventManager, Event};
///
/// #[derive(Debug, Clone, Event)]
/// struct Signal;
///
/// let mgr = EventManager::new();
/// let d   = mgr.register::<Signal>();
/// let mut c1  = mgr.subscribe::<Signal>();
/// let mut c2  = c1.clone(); // fresh, independent subscription
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
/// When a `Consumer` is dropped its subscription is automatically removed the
/// next time a worker attempts to route an event to it. No explicit
/// unsubscribe call is needed.
pub struct Consumer<T: Event> {
    receiver: UnboundedReceiver<T>,
    manager: EventManager,
}

impl<T: Event> Consumer<T> {
    /// Returns the **next** pending event, or `None` if the queue is currently
    /// empty.
    ///
    /// This is non-blocking. Use [`consume_all`] if you want to drain every
    /// event that arrived so far.
    ///
    /// # Example
    ///
    /// ```rust
    /// use dirk_events::{EventManager, Event};
    ///
    /// #[derive(Debug, Clone, Event)]
    /// struct Counter(u32);
    ///
    /// let mgr = EventManager::new();
    /// let d   = mgr.register::<Counter>();
    /// let mut c   = mgr.subscribe::<Counter>();
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
    pub fn try_consume(&mut self) -> Option<T> {
        let res = self.receiver.try_recv().ok();
        if let Some(event) = res.clone() {
            trace!("consuming {}", event.debug());
        }
        res
    }

    /// Async consumption function. Returns a future that resolved to the next
    /// event that is dispatched to this [`Consumer`].
    pub async fn consume(&mut self) -> Option<T> {
        let res = self.receiver.recv().await;
        if let Some(event) = res.clone() {
            trace!("consuming {}", event.debug());
        }
        res
    }

    /// Blocks the current thread until the next event arrives, or all
    /// dispatchers for this subscription are dropped.
    pub fn consume_blocking(&mut self) -> Option<T> {
        let res = self.receiver.blocking_recv();
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
    /// use dirk_events::{EventManager, Event};
    ///
    /// #[derive(Debug, Clone, Event)]
    /// struct Score(u32);
    ///
    /// let mgr = EventManager::new();
    /// let d   = mgr.register::<Score>();
    /// let mut c   = mgr.subscribe::<Score>();
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
    /// use dirk_events::{EventManager, Event};
    ///
    /// #[derive(Debug, Clone, Event)]
    /// #[event("damage={0}")]
    /// struct DamageEvent(u32);
    ///
    /// let mgr = EventManager::new();
    /// let d   = mgr.register::<DamageEvent>();
    /// let mut c   = mgr.subscribe::<DamageEvent>();
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
    pub fn consume_all(&mut self) -> impl Iterator<Item = T> {
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

/// An event run at the beginning of every tick.
///
/// This event contains the frame number.
/// Used for thread synchronization.
#[derive(Debug, Clone, Event)]
#[event("Begin frame number {0}")]
pub struct BeginFrame(pub u64);
