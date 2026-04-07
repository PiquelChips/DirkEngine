//! This crate the engine's event system.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::mpsc::{self, Receiver, Sender},
};

mod tests;

/// The trait that should be implemented by every event type.
pub trait Event: Send + Clone + 'static {}

/// The event manager struct.
#[derive(Default)]
pub struct EventManager {
    producers: Vec<Box<dyn AnyProducer>>,
    subscribers: HashMap<TypeId, Vec<Subscriber>>,
}

impl EventManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<T: Event>(&mut self) -> Dispatcher<T> {
        let (sender, receiver) = mpsc::channel::<T>();

        self.producers.push(Box::new(TypedProducer {
            type_id: TypeId::of::<T>(),
            receiver,
        }));
        Dispatcher { sender }
    }
    pub fn subscribe<T: Event>(&mut self) -> Consumer<T> {
        let (sender, receiver) = mpsc::channel::<T>();

        let type_id = TypeId::of::<T>();

        let subscribers = self.subscribers.entry(type_id).or_default();

        subscribers.push(Subscriber {
            sender: Box::new(sender),
        });
        Consumer { receiver }
    }
    /// Drains all producers and forwards their pending events to matching subscribers.
    /// Call this once per frame / tick in your engine loop.
    pub fn dispatch_all(&self) {
        for producer in &self.producers {
            producer.forward_pending(&self.subscribers);
        }
    }
}

/// The Type Erasure Trait
/// Allows the EventManager to forward pending events without knowing `T`.
trait AnyProducer: Send {
    fn forward_pending(&self, subscribers: &HashMap<TypeId, Vec<Subscriber>>);
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
    fn forward_pending(&self, subscribers: &HashMap<TypeId, Vec<Subscriber>>) {
        let Some(subscribers) = subscribers.get(&self.type_id) else {
            return;
        };

        while let Ok(event) = self.receiver.try_recv() {
            subscribers.iter().for_each(|sub| {
                let Some(sender) = sub.sender.downcast_ref::<Sender<T>>() else {
                    return;
                };
                // Ignore send errors: it just means the Consumer was dropped.
                let _ = sender.send(event.clone());
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
}

impl<T: Event> Dispatcher<T> {
    /// Queues an event to be forwarded to subscribers.
    pub fn dispatch(&self, event: T) {
        // Ignore send errors: it just means the EventManager was dropped.
        let _ = self.sender.send(event);
    }
}

/// This struct is created by the event manager.
/// It can consume events that are sent by the event manager.
pub struct Consumer<T: Event> {
    receiver: Receiver<T>,
}

impl<T: Event> Consumer<T> {
    /// Returns the next pending event, or `None` if the queue is empty.
    pub fn try_consume(&self) -> Option<T> {
        self.receiver.try_recv().ok()
    }

    /// Returns an iterator that drains all currently pending events.
    pub fn consume_all(&self) -> impl Iterator<Item = T> + '_ {
        std::iter::from_fn(|| self.receiver.try_recv().ok())
    }
}
