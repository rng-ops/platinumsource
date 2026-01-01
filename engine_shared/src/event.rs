//! Event and messaging system.
//!
//! This is a small typed event bus.
//! - Client: use for input/events, prediction reconciliation, UI.
//! - Server: use for gameplay events, networking notifications.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

/// Typed event bus.
#[derive(Default)]
pub struct EventBus {
    queues: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl EventBus {
    /// Pushes an event into the queue.
    pub fn push<E: 'static + Send + Sync>(&mut self, e: E) {
        let q = self
            .queues
            .entry(TypeId::of::<E>())
            .or_insert_with(|| Box::new(Vec::<E>::new()));
        let q = q.downcast_mut::<Vec<E>>().expect("queue type mismatch");
        q.push(e);
    }

    /// Drains all queued events of a type.
    pub fn drain<E: 'static + Send + Sync>(&mut self) -> Vec<E> {
        self.queues
            .remove(&TypeId::of::<E>())
            .and_then(|boxed| boxed.downcast::<Vec<E>>().ok())
            .map(|boxed| *boxed)
            .unwrap_or_default()
    }
}
