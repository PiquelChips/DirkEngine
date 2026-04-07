//! This crate the engine's event system.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::mpsc::{self, Receiver, Sender},
};

/// The trait that should be implemented by every event type.
pub trait Event: Send + 'static {}

/// The event manager struct.
pub struct EventManager {
    producers: Vec<Box<dyn AnyProducer>>,
    subscribers: HashMap<TypeId, Subscriber>,
}

impl EventManager {
    pub fn new() -> Self {
        Self {
            producers: Vec::new(),
            subscribers: HashMap::new(),
        }
    }

    pub fn register<T: Event>(&mut self) -> Dispatcher<T> {
        let (sender, receiver) = mpsc::channel::<T>();

        self.producers.push(Box::new(TypedProducer {
            type_id: TypeId::of::<T>(),
            receiver: receiver,
        }));
        Dispatcher { sender }
    }
    pub fn subscribe<T: Event>(&mut self) -> Consumer<T> {
        let (sender, receiver) = mpsc::channel::<T>();

        self.subscribers.insert(
            TypeId::of::<T>(),
            Subscriber {
                sender: Box::new(sender),
            },
        );
        Consumer { receiver }
    }

/// The Type Erasure Trait
/// Allows the EventManager to forward pending events without knowing `T`.
trait AnyProducer: Send {
    fn forward_pending(&self, subscribers: &HashMap<TypeId, Subscriber>);
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
    fn forward_pending(&self, subscribers: &HashMap<TypeId, Subscriber>) {
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

/// This struct is created by the event manager.
/// It can consume events that are sent by the event manager.
pub struct Consumer<T: Event> {
    receiver: Receiver<T>,
}
