//! The engine module. The engine holds all the state & manages
//! all the systems for the engine to run properly.

use std::{collections::HashMap, ffi::CString, path::PathBuf, str::FromStr, time::Instant};

use anyhow::Context;
use dirk_threads::WorkerPool;
use dirk_universe::{Entity, Universe, World, WorldId};
use tracing::info;

/// This state is returned by [`Engine::tick`].
/// It holds why the engine is exiting.
pub enum ExitState {
    /// The engine is not exiting. The engine should never exit with this state.
    Running,
    /// The engine exit has been requested by a system.
    Requested,
    /// An error occured in the [`Engine`], it is exiting.
    Error(anyhow::Error),
}

impl ExitState {
    /// Returns if this exit state is exiting
    #[must_use]
    pub fn exiting(&self) -> bool {
        match self {
            Self::Running => false,
            Self::Requested | Self::Error(_) => true,
        }
    }
}

/// This is the main struct that holds global engine state.
pub struct Engine {
    exit_consumer: dirk_events::Consumer<dirk_events::AppExit>,
    exit_dispatcher: dirk_events::Dispatcher<dirk_events::Exiting>,
    frame_dispatcher: dirk_events::Dispatcher<dirk_events::BeginFrame>,
    event_manager: dirk_events::EventManager,

    /// This is a thread pool use by various engine systems for async tasks.
    #[allow(unused)]
    workers: WorkerPool,
    #[allow(unused)]
    logger: piquel_log::Logger,

    frame: u64,
    last_tick: Instant,

    renderer: dirk_renderer::Renderer,
    platform: dirk_platform::Platform,
    universe: dirk_universe::Universe,
    players: dirk_player::PlayerManager,
    #[allow(unused)]
    asset_registry: dirk_assets::AssetRegistry,

    exit_state: ExitState,
}

impl Engine {
    /// Constructs and initialises the gine
    ///
    /// # Errors
    ///
    /// Errors can be returned during platform & renderer initialisation
    pub fn init() -> anyhow::Result<Self> {
        #[cfg(feature = "editor")]
        info!("starting editor");

        info!("initialising engine");

        let logger = piquel_log::Logger::new()
            .with_max_level(piquel_log::LogLevel::Debug)
            .with_log_bridge(true)
            .with_file(piquel_log::FileConfig::new(
                PathBuf::from(std::env!("SAVED_PATH")).join("logs"),
            ));

        logger.init().context("init logger")?;

        let workers = WorkerPool::new("dirk-workers");

        let event_manager = dirk_events::EventManager::new(workers.clone());
        let asset_registry = dirk_assets::AssetRegistry::init(&event_manager)
            .context("initialising asset registry")?;

        let version = dirk_utils::Version::from_str(env!("CARGO_PKG_VERSION"))?;
        let name = "DirkEngine";

        let platform = dirk_platform::Platform::init(&event_manager).context("platform init")?;
        let mut renderer = dirk_renderer::Renderer::init(
            &dirk_renderer::RendererCreateInfo {
                engine_name: CString::from_str(name)?,
                engine_version: version,
                app_name: CString::from_str(name)?,
                app_version: version,
            },
            platform.main_window(),
            &event_manager,
        )
        .context("renderer init")?;

        let universe = Universe::builder()
            .with_other(dirk_world::universe_builder(&event_manager))
            .with_other(renderer.universe_builder())
            .build();

        let players = dirk_player::PlayerManager::new(&event_manager);

        info!("engine initialised");
        Ok(Self {
            exit_consumer: event_manager.subscribe(),
            exit_dispatcher: event_manager.register(),
            frame_dispatcher: event_manager.register(),
            event_manager,
            logger,
            asset_registry,
            workers,

            frame: 0,
            last_tick: Instant::now(),

            platform,
            renderer,
            universe,

            players,

            exit_state: ExitState::Running,
        })
    }
    /// Will start the main game/editor. This should be called
    /// once right after init.
    ///
    /// # Errors
    ///
    /// None for now, will be one if an error occurs when creating the world.
    pub fn start(&mut self) -> anyhow::Result<()> {
        // setup the world
        info!("starting engine");
        let world_id = self.create_test_world();

        let player = self.players.new_player(self.platform.main_window().id());
        self.universe
            .spawn_entity(world_id, Entity::builder().with_component(player));

        Ok(())
    }
    /// Ticks the engine. This is the master function that calls
    /// every other system's tick function.
    ///
    /// Returns the current exit state after advancing one frame, unless an
    /// exit has already been requested.
    pub fn tick(&mut self) -> &ExitState {
        if self.is_requesting_exit() {
            return &self.exit_state;
        }

        self.frame += 1;
        if let Err(err) = self.tick_inner() {
            self.exit(Some(err.context("engine tick")));
        }

        &self.exit_state
    }

    fn tick_inner(&mut self) -> anyhow::Result<()> {
        self.frame_dispatcher
            .dispatch(dirk_events::BeginFrame(self.frame));

        let delta_time = self.capture_delta_time();

        // TODO: renders too fast and semaphores have problem.
        // remove when rendering takes longer
        std::thread::sleep(std::time::Duration::from_millis(100));

        self.process_events();
        if self.is_requesting_exit() {
            return Ok(());
        }

        self.platform.tick(delta_time);
        if self.platform.windows().is_empty() {
            self.exit(None);
            return Ok(());
        }

        self.process_events();
        if self.is_requesting_exit() {
            return Ok(());
        }

        self.universe.tick(delta_time);
        self.asset_registry.tick();

        self.renderer
            .tick(delta_time, self.platform.windows())
            .context("renderer")?;

        self.players.tick();

        self.renderer.render().context("rendering")?;
        Ok(())
    }

    fn process_events(&mut self) {
        if let Some(dirk_events::AppExit(msg)) = self.exit_consumer.try_consume() {
            info!("App exit requested: {msg}");
            self.exit(None);
        }
    }
    /// Returns if the engine is planning to exit. i.e., if the engine
    /// will shutdown at the next tick.
    pub fn is_requesting_exit(&self) -> bool {
        self.exit_state.exiting()
    }
    /// Specify `err` to exit with an error.
    pub fn exit(&mut self, err: Option<anyhow::Error>) {
        if matches!(self.exit_state, ExitState::Error(_)) {
            return;
        }

        self.exit_state = match err {
            Some(err) => ExitState::Error(err),
            None => ExitState::Requested,
        };

        if self.exit_state.exiting() {
            self.exit_dispatcher.dispatch(dirk_events::Exiting);
        }
    }
    /// Returns the exit error
    pub fn exit_state(&self) -> &ExitState {
        &self.exit_state
    }
    /// Returns the time in seconds since last tick. This consumes the delta time.
    fn capture_delta_time(&mut self) -> f64 {
        let current_time = Instant::now();
        let delta = current_time.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = current_time;
        delta
    }

    fn create_test_world(&mut self) -> WorldId {
        use dirk_world::components::{Renderable, Transform};

        let duck_model = dirk_assets::AssetHandle::from_raw(
            "models/Duck/Duck.dirkasset",
            dirk_assets::AssetType::Model,
        );
        let shrek_model = dirk_assets::AssetHandle::from_raw(
            "models/Shrek/Shrek.dirkasset",
            dirk_assets::AssetType::Model,
        );

        let shrek_builder = Entity::builder()
            .with_component(Transform {
                location: glam::Vec3::ZERO,
                rotation: glam::Vec3::ZERO,
                scale: glam::Vec3::splat(1.),
            })
            .with_component(Renderable::new(shrek_model));

        let duck_builder = Entity::builder()
            .with_component(Transform {
                location: glam::vec3(100., 0., 0.),
                rotation: glam::Vec3::ZERO,
                scale: glam::Vec3::splat(1.),
            })
            .with_component(Renderable::new(duck_model));

        let world_builder = World::builder("test world")
            .with_entity(shrek_builder)
            .with_entity(duck_builder);

        let world = self.universe.create_world(world_builder);
        world.expect("world creation shouldn't fail")
    }
}
