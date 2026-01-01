//! Interpolation.
//!
//! The server sends discrete snapshots at tick boundaries.
//! The client renders at its own rate and interpolates entity states.

use std::collections::VecDeque;

use engine_shared::{math::Vec3, net::{EntityState, Snapshot}};

/// Buffered snapshot history for interpolation.
#[derive(Default)]
pub struct SnapshotBuffer {
    history: VecDeque<Snapshot>,
    max: usize,
}

impl SnapshotBuffer {
    pub fn new(max: usize) -> Self {
        Self {
            history: VecDeque::new(),
            max,
        }
    }

    pub fn push(&mut self, snap: Snapshot) {
        self.history.push_back(snap);
        while self.history.len() > self.max {
            self.history.pop_front();
        }
    }

    /// Returns the number of buffered snapshots.
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Returns true if no snapshots are buffered.
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Gets an interpolated position for an entity given a fractional alpha.
    ///
    /// `alpha` should be in $[0,1]$ where 0 = older snapshot, 1 = newer.
    pub fn interp_entity(&self, entity: engine_shared::ecs::EntityId, alpha: f32) -> Option<Vec3> {
        if self.history.len() < 2 {
            return None;
        }
        let a = &self.history[self.history.len() - 2];
        let b = &self.history[self.history.len() - 1];

        let pa = a.entities.iter().find(|e| e.id == entity).map(|e| e.position);
        let pb = b.entities.iter().find(|e| e.id == entity).map(|e| e.position);
        match (pa, pb) {
            (Some(pa), Some(pb)) => Some(pa.lerp(pb, alpha)),
            _ => None,
        }
    }

    pub fn last_snapshot(&self) -> Option<&Snapshot> {
        self.history.back()
    }
}

/// Convenience: find entity state in a snapshot.
pub fn find_entity<'a>(snap: &'a Snapshot, id: engine_shared::ecs::EntityId) -> Option<&'a EntityState> {
    snap.entities.iter().find(|e| e.id == id)
}
