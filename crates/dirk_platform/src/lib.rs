#![doc = include_str!("../README.md")]

use std::{collections::HashMap, time::Duration};

use tracing::info;
use winit::event_loop::{
    EventLoop,
    pump_events::{EventLoopExtPumpEvents, PumpStatus},
};

mod errors;
mod event;
mod handler;
mod window;
pub use errors::Error;
pub use event::*;
pub use window::Window;
pub use winit::{
    event::{ButtonSource, MouseButton},
    keyboard::{KeyCode, PhysicalKey},
    window::WindowId,
};

use errors::Result;
use handler::PlatformHandler;

/// The main Platform struct that is initialized by the engine.
pub struct Platform {
    handler: PlatformHandler,
    event_loop: EventLoop,

    exit_dispatcher: dirk_events::Dispatcher<dirk_events::AppExit>,
    window_consumer: dirk_events::Consumer<WindowEvent>,
}

impl Platform {
    /// Initialise the platform wrapper. Essentially just creates
    /// the handler for [winit] platform events.
    ///
    /// # Errors
    ///
    /// Returns an error if the [winit] App exited while trying
    /// to start it.
    pub fn init(events: &dirk_events::EventManager) -> Result<Self> {
        let mut platform = Self {
            handler: PlatformHandler::new(events),
            event_loop: EventLoop::new()?,
            exit_dispatcher: events.register(),
            window_consumer: events.subscribe(),
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
    /// Process pending OS events without blocking.
    /// Returns the events that occurred this tick for the engine to handle.
    pub fn tick(&mut self, _delta_time: f64) {
        match self
            .event_loop
            .pump_app_events(Some(Duration::ZERO), &mut self.handler)
        {
            PumpStatus::Exit(code) => {
                // Treat a forced OS exit like a window close.
                self.exit_dispatcher.dispatch(dirk_events::AppExit(format!(
                    "Event loop exited with code {code}"
                )));
                return;
            }
            PumpStatus::Continue => {}
        }

        if self.handler.windows.is_empty() {
            self.exit_dispatcher.dispatch(dirk_events::AppExit(
                "all platform windows have been closed".into(),
            ));
        }

        self.window_consumer.consume_all().for_each(|event| {
            if let Some(window) = self.handler.windows.get_mut(event.id()) {
                window.handle_event(&event);
            }
        });
    }

    /// Returns a reference to the main window. The main window is just the
    /// first window ever created.
    pub fn main_window(&self) -> &Window {
        self.handler.main_window()
    }

    /// Returns a reference to the `HashMap` of all the windows currently
    /// owned by the engine.
    pub fn windows(&self) -> &HashMap<WindowId, Window> {
        &self.handler.windows
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
