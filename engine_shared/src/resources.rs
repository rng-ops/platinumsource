//! Resource management system.
//!
//! This provides a `ResourceManager` and simple typed handles.
//! In a real engine you'd integrate hot-reload, async loading, streaming, etc.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

/// Typed resource handle.
#[derive(Debug, Clone)]
pub struct Handle<T> {
    id: u64,
    _phantom: std::marker::PhantomData<T>,
}

/// In-memory resource manager.
#[derive(Default)]
pub struct ResourceManager {
    next_id: u64,
    by_type: HashMap<TypeId, HashMap<u64, Arc<dyn Any + Send + Sync>>>,
}

impl ResourceManager {
    /// Inserts a resource and returns a handle.
    pub fn insert<T: 'static + Send + Sync>(&mut self, value: T) -> Handle<T> {
        let id = self.next_id;
        self.next_id += 1;
        let map = self.by_type.entry(TypeId::of::<T>()).or_default();
        map.insert(id, Arc::new(value));
        Handle {
            id,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Gets a resource by handle.
    pub fn get<T: 'static + Send + Sync>(&self, h: &Handle<T>) -> Option<Arc<T>> {
        self.by_type
            .get(&TypeId::of::<T>())
            .and_then(|map| map.get(&h.id))
            .and_then(|arc_any| arc_any.clone().downcast::<T>().ok())
    }
}
