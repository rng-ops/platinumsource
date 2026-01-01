//! Input handling.
//!
//! In a real engine this would integrate with windowing, raw mouse/keyboard,
//! action bindings, and per-frame sampling. This scaffold focuses on producing
//! deterministic per-tick `PlayerCommand` messages.

use engine_shared::{
    math::Vec3,
    net::{ClientId, PlayerCommand},
};

/// User input state at a moment in time.
#[derive(Debug, Clone, Copy, Default)]
pub struct InputState {
    pub forward: f32,
    pub right: f32,
    pub up: f32,
}

impl InputState {
    pub fn wish_vector(self) -> Vec3 {
        Vec3::new(self.forward, self.right, self.up)
    }
}

/// Turns sampled input into a `PlayerCommand` for a tick.
pub fn build_command(client_id: ClientId, tick: u32, input: InputState) -> PlayerCommand {
    PlayerCommand {
        client_id,
        tick,
        wish: input.wish_vector(),
    }
}
