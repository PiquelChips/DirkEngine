use std::{collections::HashMap, ffi::CString, str::FromStr, time::Instant};

use anyhow::Context;
use player::Player;
use tracing::info;
use world::{World, WorldId};

use logging::Logger;

/// This is the main struct that holds global engine state.
pub struct Engine {
    exit_consumer: events::Consumer<platform::AppExit>,
    event_manager: events::EventManager,
    input_manager: player::input::InputManager,

    renderer: renderer::Renderer,
    platform: platform::Platform,

    next_world_id: WorldId,
    worlds: HashMap<WorldId, World>,
    players: Vec<Player>,

    is_requesting_exit: bool,
    exit_error: Option<anyhow::Error>,
    last_tick: Instant,

    #[allow(unused)]
    logger: Logger,
}

impl Engine {
    /// Constructs and initialises the gine
    pub fn init() -> anyhow::Result<Self> {
        info!("initialising engine");
        let logger = logging::Logger::new()
            .write_fs(true)
            .max_level(logging::LogLevel::Debug)
            .init()
            .context("initialising logger")?;

        let mut event_manager = events::EventManager::new();
        let exit_consumer = event_manager.subscribe();

        let input_manager = player::input::InputManager::init(&mut event_manager);

        let version = utils::Version::from_str(env!("CARGO_PKG_VERSION"))?;
        let name = "DirkEngine";

        let platform = platform::Platform::init(&mut event_manager).context("platform init")?;
        let renderer = renderer::Renderer::init(
            renderer::RendererCreateInfo {
                engine_name: CString::from_str(name)?,
                engine_version: version,
                app_name: CString::from_str(name)?,
                app_version: version,
            },
            platform.main_window(),
            event_manager.clone(),
        )
        .context("renderer init")?;

        info!("engine initialised");
        Ok(Self {
            event_manager,
            exit_consumer,
            logger,

            platform,
            renderer,
            input_manager,

            next_world_id: 0,
            worlds: HashMap::new(),
            players: Vec::new(),

            is_requesting_exit: false,
            exit_error: None,
            last_tick: Instant::now(),
        })
    }
    /// Will start the main game/editor. This should be called
    /// once right after init.
    pub fn start(&mut self) -> anyhow::Result<()> {
        info!("starting engine");
        let world_id = self.create_test_world();
        let world = self.worlds.get_mut(&world_id).unwrap();

        self.players
            .push(Player::spawn(world, self.platform.main_window().id()));

        Ok(())
    }
    /// Ticks the engine. This is the master function that calls
    /// every other system's tick function.
    pub fn tick(&mut self) -> anyhow::Result<bool> {
        let delta_time = self.capture_delta_time();
        self.event_manager.dispatch_all();

        // TODO: renders too fast and semaphores have problem.
        // remove when rendering takes longer
        std::thread::sleep(std::time::Duration::from_millis(100));

        self.process_events();
        if self.is_requesting_exit() {
            return Ok(false);
        }

        self.input_manager.tick(delta_time, &self.players);

        self.platform.tick(delta_time);
        self.renderer
            .tick(delta_time, &self.worlds, self.platform.windows_mut())
            .context("renderer")?;

        self.render().context("rendering")?;
        Ok(!self.is_requesting_exit())
    }
    pub fn render(&mut self) -> anyhow::Result<()> {
        for player in &self.players {
            self.renderer
                .render(*player.window(), *player.world(), *player.entity())?;
        }
        Ok(())
    }

    pub fn shutdown(&self) -> anyhow::Result<()> {
        info!("engine shutting down");
        Ok(())
    }
    fn process_events(&mut self) {
        if let Some(platform::AppExit(_)) = self.exit_consumer.try_consume() {
            self.exit(None);
        }
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

    fn create_world(&mut self) -> WorldId {
        let id = self.next_world_id;
        self.next_world_id += 1;

        let world = World::new(id, &mut self.event_manager);
        self.worlds.insert(id, world);
        id
    }
    #[allow(unused)]
    fn destroy_world(&mut self, id: WorldId) {
        self.worlds.remove(&id);
    }

    fn create_test_world(&mut self) -> WorldId {
        let world_id = self.create_world();
        use world::components::*;
        let world = self.worlds.get_mut(&world_id).unwrap();

        let shrek = world.spawn();
        world.insert(
            shrek,
            Transform {
                location: glam::Vec3::ZERO,
                rotation: glam::Vec3::ZERO,
                scale: glam::Vec3::splat(1.),
            },
        );
        world.insert(
            shrek,
            Renderable {
                model: "Shrek".to_string(),
            },
        );

        let duck = world.spawn();
        world.insert(
            duck,
            Transform {
                location: glam::vec3(100., 0., 0.),
                rotation: glam::Vec3::ZERO,
                scale: glam::Vec3::splat(1.),
            },
        );
        world.insert(
            duck,
            Renderable {
                model: "Duck".to_string(),
            },
        );
        world_id
    }
}
