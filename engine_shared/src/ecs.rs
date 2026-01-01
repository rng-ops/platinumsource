//! Entity/component system (minimal ECS).
//!
//! This is a deliberately small ECS suitable for deterministic simulation and
//! net replication. It is not archetype-based; instead it uses typed component
//! storages keyed by entity id.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use serde::{Deserialize, Serialize};

/// Opaque entity id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u64);

/// Simple world that can store typed components.
#[derive(Default)]
pub struct World {
    next_id: u64,
    storages: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl World {
    /// Creates a new entity.
    pub fn spawn(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Inserts/replaces a component for an entity.
    pub fn insert<T: 'static + Send + Sync>(&mut self, entity: EntityId, component: T) {
        let storage = self
            .storages
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(HashMap::<EntityId, T>::new()));

        let storage = storage
            .downcast_mut::<HashMap<EntityId, T>>()
            .expect("storage type mismatch");

        storage.insert(entity, component);
    }

    /// Gets a component reference.
    pub fn get<T: 'static + Send + Sync>(&self, entity: EntityId) -> Option<&T> {
        self.storages
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref::<HashMap<EntityId, T>>())
            .and_then(|storage| storage.get(&entity))
    }

    /// Gets a mutable component reference.
    pub fn get_mut<T: 'static + Send + Sync>(&mut self, entity: EntityId) -> Option<&mut T> {
        self.storages
            .get_mut(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_mut::<HashMap<EntityId, T>>())
            .and_then(|storage| storage.get_mut(&entity))
    }

    /// Iterates entities with a given component.
    pub fn iter<T: 'static + Send + Sync>(&self) -> impl Iterator<Item = (EntityId, &T)> {
        self.storages
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref::<HashMap<EntityId, T>>())
            .into_iter()
            .flat_map(|storage| storage.iter().map(|(k, v)| (*k, v)))
    }
}

/// Common component: position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

/// Common component: velocity.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecs_insert_and_get() {
        let mut world = World::default();
        let e = world.spawn();
        world.insert(e, Position { x: 1.0, y: 2.0, z: 3.0 });
        assert_eq!(world.get::<Position>(e).unwrap().x, 1.0);
    }
}
