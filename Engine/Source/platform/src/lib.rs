//! This crate has all the platform level functionnality. It should be the
//! only place where any kind of platform dependent code or #[cfg()] attributes
//! should be used. This allows us to create a central platform API for eaiser
//! development.
//!
//! The DirkEngine's platform API is build on the winit crate.

use std::time::Duration;

use log::info;
use winit::event_loop::{
    EventLoop,
    pump_events::{EventLoopExtPumpEvents, PumpStatus},
};

mod errors;
mod handler;
mod window;
pub use errors::Error;
pub use window::Window;

use errors::Result;
use handler::PlatformHandler;

/// The main Platform struct that is initialized by the engine.
pub struct Platform {
    handler: PlatformHandler,
    event_loop: EventLoop,
}

impl Platform {
    pub fn init() -> Result<Self> {
        let mut platform = Self {
            handler: PlatformHandler::default(),
            event_loop: EventLoop::new().expect("failed to create winit event loop"),
        };

        // Pump until `can_create_surfaces` fires and the main window exists.
        // Each call returns quickly; the OS dispatches the startup events
        // within the first few iterations.
        while !platform.handler.is_initialized() {
            match platform
                .event_loop
                .pump_app_events(Some(Duration::ZERO), &mut platform.handler)
            {
                PumpStatus::Exit(code) => return Err(Error::AppExited(code)),
                PumpStatus::Continue => {}
            }
        }

        info!("initialized platform");
        Ok(platform)
    }
    /// Process all pending OS events without blocking. Returns `Ok(true)`
    /// when the application has requested to exit (e.g. last window closed).
    pub fn tick(&mut self, _delta_time: f32) -> Result<bool> {
        match self
            .event_loop
            .pump_app_events(Some(Duration::ZERO), &mut self.handler)
        {
            PumpStatus::Exit(code) => {
                info!("Event loop exited with code {code}");
                Ok(true)
            }
            PumpStatus::Continue => Ok(false),
        }
    }

    pub fn main_window(&self) -> &Window {
        self.handler.main_window()
    }
}

impl Drop for Platform {
    fn drop(&mut self) {
        info!("Shutting down platform");
        self.handler.shutdown();
        // One final pump so winit can process the window destruction
        // events before we tear everything down.
        self.event_loop
            .pump_app_events(Some(Duration::ZERO), &mut self.handler);
    }
}
