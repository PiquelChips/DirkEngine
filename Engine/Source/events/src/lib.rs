//! This crate the engine's event system.

use std::{
    any::TypeId,
    collections::HashMap,
    sync::mpsc::{self, Receiver, Sender},
};

/// The trait that should be implemented by every event type.
pub trait Event {}

/// The event manager struct.
pub struct EventManager {
    producers: Vec<Producer>,
    subscribers: HashMap<TypeId, Subscriber>,
}

impl EventManager {
    pub fn new() -> Self {
        Self {
            producers: Vec::new(),
            subscribers: HashMap::new(),
        }
    }

    pub fn register<T: Event>(&self) -> Dispatcher<T> {
        let (sender, receiver) = mpsc::channel::<T>();

        self.producers.push(Producer { receiver });
        Dispatcher { sender }
    }
    pub fn subscribe<T: Event>(&self) -> Consumer<T> {
        let (sender, receiver) = mpsc::channel::<T>();

        self.subscribers
            .insert(TypeId::of::<T>(), Subscriber { sender });
        Consumer { receiver }
    }
}

/// A producer is an object that will be queried for events.
/// On event collection, the event manager loops through every
/// producer and collects pending events.
struct Producer {
    receiver: Receiver,
}

/// A subscriber is a sender for events.
/// On event dispatching, will send the events through the
/// channels of every subscriber.
struct Subscriber {
    sender: Sender,
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
