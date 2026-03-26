//! This crate has all the platform level functionnality. It should be the
//! only place where any kind of platform dependent code or #[cfg()] attributes
//! should be used. This allows us to create a central platform API for eaiser
//! development.
//!
//! The DirkEngine's platform API is build on the winit crate.

use winit::event_loop::{EventLoop, run_on_demand::EventLoopExtRunOnDemand};

mod errors;
mod handler;
mod window;
pub use errors::PlatformError;
pub use window::Window;

use errors::Result;
use handler::PlatformHandler;

/// The main Platform struct that is initialized by the engine.
pub struct Platform {
    handler: PlatformHandler,
    event_loop: EventLoop,
}

impl Platform {
    pub fn init() -> Self {
        Self {
            handler: handler::PlatformHandler::default(),
            event_loop: EventLoop::new().expect("failed to create empty winit event loop"),
        }
    }
    pub fn tick(&mut self, _delta_time: f32) -> Result<()> {
        // TODO: maybe listen on a separate thread in the future
        Ok(self.event_loop.run_app_on_demand(&mut self.handler)?)
    }
    pub fn main_window(&self) -> &Window {
        self.handler.main_window()
    }
}
