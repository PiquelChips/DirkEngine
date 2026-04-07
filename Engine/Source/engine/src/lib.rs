use std::time::Instant;

use anyhow::Context;
use platform::PlatformEvent;

/// This is the main struct that holds global engine state.
pub struct Engine {
    event_manager: events::EventManager,
    platform: platform::Platform,
    is_requesting_exit: bool,
    exit_error: Option<anyhow::Error>,
    last_tick: Instant,
}

impl Engine {
    pub fn init() -> anyhow::Result<Self> {
        let logger = logging::Logger::new(true, true, true);
        logging::init(logger);

        let mut event_manager = events::EventManager::new();
        let platform = platform::Platform::init(&mut event_manager).context("platform init")?;

        /* A rough idea of the flow of the C++ Engine
         *
         * Intialize Main Engine Objects:
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
            event_manager,
            exit_error: None,
            last_tick: Instant::now(),
        })
    }
    pub fn tick(&mut self) -> anyhow::Result<bool> {
        let delta_time = self.capture_delta_time();

        if self.is_requesting_exit() {
            return Ok(false);
        }

        self.event_manager.dispatch_all();

        self.platform.tick(delta_time);

        if self.is_requesting_exit() {
            return Ok(false);
        }

        /*
         * World Tick
         * Main Viewport tick
         * Render
         */
        self.render().context("tick: render")?;
        Ok(true)
    }

    pub fn render(&self) -> anyhow::Result<()> {
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
    pub fn shutdown(&self) -> anyhow::Result<()> {
        /*
         * Shutdown ImGui (renderer then platform)
         *
         * logger.cleanup():
         * - Should cleanup and close all the log files
         */
        Ok(())
    }
    pub fn is_requesting_exit(&self) -> bool {
        self.is_requesting_exit || self.exit_error.is_some()
    }
    /// Specify [err] to exit with an error.
    pub fn exit(&mut self, err: Option<anyhow::Error>) {
        self.is_requesting_exit = true;
        self.exit_error = err;
    }
    /// Returns the exit error
    pub fn get_exit_error(&self) -> &Option<anyhow::Error> {
        &self.exit_error
    }
    /// Returns the time in seconds since last tick. This consumes the delta time.
    fn capture_delta_time(&mut self) -> f32 {
        let current_time = Instant::now();
        let delta = current_time.duration_since(self.last_tick).as_secs_f32();
        self.last_tick = current_time;
        delta
    }
}
