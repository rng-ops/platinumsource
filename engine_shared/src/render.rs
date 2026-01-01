//! Rendering abstraction.
//!
//! This crate intentionally does not depend on a graphics backend.
//! Define traits that a renderer implementation would satisfy.

use crate::math::{Mat4, Vec3};

/// A minimal rendering API.
pub trait RenderBackend: Send + Sync {
    fn begin_frame(&mut self);
    fn draw_debug_point(&mut self, position: Vec3);
    fn set_view_proj(&mut self, view_proj: Mat4);
    fn end_frame(&mut self);
}

/// A no-op renderer useful for headless tests.
#[derive(Default)]
pub struct NullRenderer;

impl RenderBackend for NullRenderer {
    fn begin_frame(&mut self) {}
    fn draw_debug_point(&mut self, _position: Vec3) {}
    fn set_view_proj(&mut self, _view_proj: Mat4) {}
    fn end_frame(&mut self) {}
}
