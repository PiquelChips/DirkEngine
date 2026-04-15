//! This crate the engine's event system.
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

/// The trait that should be implemented by every event type.
pub trait Event: Send + Clone + 'static {
    fn debug(&self) -> String;
}

/// Private inner state, held behind the `Arc<Mutex<>>`.
#[derive(Default)]
struct EventManagerInner {
    producers: Vec<Box<dyn AnyProducer>>,
    subscribers: HashMap<TypeId, Vec<Subscriber>>,
}

/// The event manager.
///
/// Cheaply cloneable — all clones share the same underlying state.
/// Use [`EventManager::new`] to create one and pass it around freely.
#[derive(Clone, Default)]
pub struct EventManager {
    inner: Arc<Mutex<EventManagerInner>>,
}

impl EventManager {
    pub fn new() -> Self {
        Self::default()
    }
    /// Registers a new event type and returns a [`Dispatcher`] for it.
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
    /// Drains all producers and forwards their pending events to matching subscribers.
    /// Call this once per frame / tick in your engine loop.
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
/// Allows the EventManager to forward pending events without knowing `T`.
trait AnyProducer: Send {
    fn forward_pending(&self, subscribers: &mut HashMap<TypeId, Vec<Subscriber>>);
}

/// A producer is an object that will be queried for events.
/// On event collection, the event manager loops through every
/// producer and collects pending events.
///
/// This is a typed producer as it stores the actual type of the
/// event it is producing. It is then wrapped by the [AnyProducer]
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

/// This struct is created by the envent manager.
/// It allows dispatching of events to subscribers.
pub struct Dispatcher<T: Event> {
    sender: Sender<T>,
    manager: EventManager,
}

impl<T: Event> Dispatcher<T> {
    /// Queues an event to be forwarded to subscribers on the next [`EventManager::dispatch_all`].
    pub fn dispatch(&self, event: T) {
        trace!("dispatching event {}", event.debug());
        let _ = self.sender.send(event);
    }
}

impl<T: Event> Clone for Dispatcher<T> {
    fn clone(&self) -> Self {
        self.manager.register()
    }
}

impl<T: Event> std::fmt::Debug for Dispatcher<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dispatcher").finish_non_exhaustive()
    }
}

/// Cloning produces a fresh [`Consumer`] with its own independent subscription,
/// sharing the same underlying [`EventManager`].
pub struct Consumer<T: Event> {
    receiver: Receiver<T>,
    manager: EventManager,
}

impl<T: Event> Consumer<T> {
    /// Returns the next pending event, or `None` if the queue is empty.
    pub fn try_consume(&self) -> Option<T> {
        let res = self.receiver.try_recv().ok();
        if let Some(event) = res.clone() {
            trace!("consuming {}", event.debug())
        }
        res
    }

    /// Returns an iterator that drains all currently pending events.
    pub fn consume_all(&self) -> impl Iterator<Item = T> {
        std::iter::from_fn(|| self.try_consume())
    }
}

impl<T: Event> Clone for Consumer<T> {
    fn clone(&self) -> Self {
        self.manager.subscribe()
    }
}

impl<T: Event> std::fmt::Debug for Consumer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Consumer").finish_non_exhaustive()
    }
}
