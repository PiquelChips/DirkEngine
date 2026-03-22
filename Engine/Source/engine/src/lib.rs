use crate::errors::{InitResult, RenderResult, ShutdownResult, TickResult};

mod errors;

/// This is the main struct that holds global engine state.
pub struct Engine {
    is_requesting_exit: bool,
}

impl Engine {
    pub fn init() -> InitResult<Self> {
        let logger = logging::Logger::new(true, true, true);
        logging::init(logger);

        /* A rough idea of the flow of the C++ Engine
         *
         * Intialize Main Engine Objects:
         * - EventManager
         * - Renderer
         * - Platform
         * - World
         *
         * ImGui:
         * - Configure ImGui
         * - Init ImGui platform
         * - Init ImGui for renderer
         *
         * Create main viewport
         *
         * Set `last_tick` to time::now()
         */
        Ok(Self {
            is_requesting_exit: false,
        })
    }
    /// Engine tick.
    /// Returns if the engine should continue ticking.
    pub fn tick(&self) -> TickResult<bool> {
        if self.is_requesting_exit() {
            return Ok(false);
        }

        /*
         * deltaTime = captureDeltaTime();
         *
         * Platform tick
         * if is_requesting_exit
         *     return false;
         *
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
    pub fn exit(&mut self) {
        self.is_requesting_exit = true
    }
}
