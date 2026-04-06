use std::time::Instant;

use anyhow::Context;

/// This is the main struct that holds global engine state.
pub struct Engine {
    platform: platform::Platform,
    is_requesting_exit: bool,
    exit_error: Option<anyhow::Error>,
    last_tick: Instant,
}

impl Engine {
    pub fn init() -> anyhow::Result<Self> {
        let logger = logging::Logger::new(true, true, true);
        logging::init(logger);

        let platform = platform::Platform::init().context("platform init")?;

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
            exit_error: None,
            last_tick: Instant::now(),
        })
    }
    pub fn tick(&mut self) -> anyhow::Result<bool> {
        if self.is_requesting_exit() {
            return Ok(false);
        }

        let delta_time = self.capture_delta_time();
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Process platform events and react to each one.
        let events = self.platform.tick().context("ticking platform")?;
        for event in events {
            match event {
                PlatformEvent::WindowCloseRequested { .. } => {
                    self.exit(None);
                    return Ok(false);
                }
                PlatformEvent::WindowResized { id, width, height } => {
                    self.renderer
                        .resize_window(id.into_raw(), width, height)
                        .context("resizing window")?;
                }
                PlatformEvent::WindowFocusChanged { .. } => { /* pause/unpause logic */ }
                PlatformEvent::WindowOccluded { .. } => { /* skip rendering */ }
            }
        }

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
