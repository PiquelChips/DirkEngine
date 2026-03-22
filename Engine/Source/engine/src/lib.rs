use std::time::Instant;

use anyhow::Context;

use crate::errors::{InitResult, RenderResult, ShutdownResult};

mod errors;

/// This is the main struct that holds global engine state.
pub struct Engine {
    platform: platform::Platform,
    is_requesting_exit: bool,
    last_tick: Instant,
}

impl Engine {
    pub fn init() -> InitResult<Self> {
        let logger = logging::Logger::new(true, true, true);
        logging::init(logger);

        let platform = platform::Platform::init();

        /* A rough idea of the flow of the C++ Engine
         *
         * Intialize Main Engine Objects:
         * - EventManager
         * - Renderer
         * - World
         *
         * ImGui:
         * - Configure ImGui
         * - Init ImGui platform
         * - Init ImGui for renderer
         *
         * Create main viewport
         */
        Ok(Self {
            is_requesting_exit: false,
            platform,
            last_tick: Instant::now(),
        })
    }
    /// Engine tick.
    /// Returns if the engine should continue ticking.
    pub fn tick(&mut self) -> anyhow::Result<bool> {
        if self.is_requesting_exit() {
            return Ok(false);
        }

        let delta_time = self.capture_delta_time();

        self.platform.tick(delta_time).context("ticking platform")?;
        if self.is_requesting_exit() {
            return Ok(false);
        }

        /*
         * EventManager dispatch events
         *
         * World Tick
         * Main Viewport tick
         * Render
         */
        Ok(self.is_requesting_exit())
    }
    pub fn render(&self) -> RenderResult<()> {
        /* Renderer::render
         *
         * ImGui:
         * - update delta time
         * - Renderer begin frame
         * - ImGui::NewFrame()
         * - engine renderImGui
         * - ImGui::Render()
         * - Renderer render ImGui
         */
        Ok(())
    }
    pub fn shutdown(&self) -> ShutdownResult<()> {
        /*
         * Shutdown ImGui (renderer then platform)
         *
         * logger.cleanup():
         * - Should cleanup and close all the log files
         */
        Ok(())
    }
    pub fn is_requesting_exit(&self) -> bool {
        self.is_requesting_exit
    }
    /// Specify [err] to exit with an error.
    /// TODO: have the engine exit with the specified error
    pub fn exit(&mut self, _err: Option<anyhow::Error>) {
        self.is_requesting_exit = true
    }
    /// Returns the time in seconds since last tick. This consumes the delta time.
    fn capture_delta_time(&mut self) -> f32 {
        let current_time = Instant::now();
        let delta = current_time.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = current_time;
        delta
    }
}
