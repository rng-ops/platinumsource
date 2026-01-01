//! Physics abstraction.
//!
//! Placeholder for a deterministic physics step.

use crate::{ecs::World, math::Vec3};

/// Physics parameters.
#[derive(Debug, Clone, Copy)]
pub struct PhysicsConfig {
    pub gravity: Vec3,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravity: Vec3::new(0.0, 0.0, -9.81),
        }
    }
}

/// Physics stepper trait.
pub trait PhysicsBackend: Send + Sync {
    fn step(&mut self, world: &mut World, dt_sec: f32);
}

/// No-op physics.
#[derive(Default)]
pub struct NullPhysics;

impl PhysicsBackend for NullPhysics {
    fn step(&mut self, _world: &mut World, _dt_sec: f32) {}
}
