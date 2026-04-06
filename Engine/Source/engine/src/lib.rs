use std::{collections::HashMap, f32::consts::PI, ffi::CString, str::FromStr, time::Instant};

use anyhow::Context;
use log::info;
use world::{World, WorldId};

/// This is the main struct that holds global engine state.
pub struct Engine {
    // order is important as renderer should be dropped before platform
    renderer: renderer::Renderer,
    platform: platform::Platform,

    next_world_id: WorldId,
    worlds: HashMap<WorldId, World>,

    is_requesting_exit: bool,
    exit_error: Option<anyhow::Error>,
    last_tick: Instant,
}

impl Engine {
    pub fn init() -> anyhow::Result<Self> {
        let logger = logging::Logger::new(true, true, true);
        logging::init(logger);

        let version = utils::Version::from_str(env!("CARGO_PKG_VERSION"))?;
        let name = "DirkEngine";

        let platform = platform::Platform::init().context("platform init")?;
        let renderer = renderer::Renderer::init(
            renderer::RendererCreateInfo {
                engine_name: CString::from_str(name)?,
                engine_version: version,
                app_name: CString::from_str(name)?,
                app_version: version,
            },
            platform.main_window(),
        )
        .context("renderer init")?;

        /* A rough idea of the flow of the C++ Engine
         *
         * Intialize Main Engine Objects:
         * - EventManager
         * - World
         *
         * ImGui:
         * - Configure ImGui
         * - Init ImGui platform
         * - Init ImGui for renderer
         *
         * Create main viewport
         */
        let mut engine = Self {
            platform,
            renderer,

            next_world_id: 0,
            worlds: HashMap::new(),

            is_requesting_exit: false,
            exit_error: None,
            last_tick: Instant::now(),
        };

        // TODO: this should be initialized when needed, not now
        engine
            .renderer
            .upload_model(resource_manager::ResourceManager::load_model("Shrek")?)?;
        engine
            .renderer
            .upload_model(resource_manager::ResourceManager::load_model("Duck")?)?;

        let world_id = engine.create_world()?;

        // THIS IS JUST TEMPORARY FOR TESTING
        {
            use world::components;
            let world = engine.worlds.get_mut(&world_id).unwrap();

            let player = world.spawn();
            world.insert(
                player,
                components::Transform {
                    location: glam::vec3(0., 1000., 1000.),
                    rotation: glam::vec3(0., PI / 2., PI / 2.),
                    scale: glam::Vec3::splat(1.),
                },
            );
            world.insert(
                player,
                components::Camera {
                    fov: (45_f32).to_radians(),
                    near_clip: 0.1,
                    far_clip: 100000.,
                    width: 100.,
                    height: 100.,
                },
            );

            let shrek = world.spawn();
            world.insert(
                shrek,
                components::Transform {
                    location: glam::Vec3::ZERO,
                    rotation: glam::Vec3::ZERO,
                    scale: glam::Vec3::splat(1.),
                },
            );
            world.insert(
                shrek,
                components::Renderable {
                    model: "Shrek".to_string(),
                },
            );

            let duck = world.spawn();
            world.insert(
                duck,
                components::Transform {
                    location: glam::vec3(100., 0., 0.),
                    rotation: glam::Vec3::ZERO,
                    scale: glam::Vec3::splat(1.),
                },
            );
            world.insert(
                shrek,
                components::Renderable {
                    model: "Duck".to_string(),
                },
            );

            info!("world: {:?}", world);
            // TODO: see engine::create_world
            engine.renderer.create_scene(world)?;
        }

        Ok(engine)
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
         */

        self.render().context("tick: render")?;

        Ok(self.is_requesting_exit())
    }
    pub fn render(&mut self) -> anyhow::Result<()> {
        self.renderer.render()?;
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
    pub fn shutdown(&mut self) -> anyhow::Result<()> {
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

    fn create_world(&mut self) -> anyhow::Result<WorldId> {
        let id = self.next_world_id;
        self.next_world_id += 1;

        let world = World::new(id);
        // TODO: have the world submitted here, once not having camera doesn't panic.
        // self.renderer
        //     .create_scene(&world)
        //     .context("create renderer scene")?;
        self.worlds.insert(id, world);
        Ok(id)
    }
    #[allow(unused)]
    fn destroy_world(&mut self, id: WorldId) {
        // TODO: delete renderer scene
        self.worlds.remove(&id);
    }
}
