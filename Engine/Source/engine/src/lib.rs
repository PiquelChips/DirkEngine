//! The [`Engine`] crate. The engine holds all the state & manages
//! all the systems for the engine to run properly.

use std::{collections::HashMap, ffi::CString, str::FromStr, time::Instant};

use anyhow::Context;
use tracing::info;
use universe::{Entity, Universe, World, WorldId};
use world::player::{Player, PlayerId};

use logging::Logger;

/// This is the main struct that holds global engine state.
pub struct Engine {
    exit_consumer: events::Consumer<events::AppExit>,
    event_manager: events::EventManager,

    #[allow(unused)]
    asset_registry: assets::AssetRegistry,

    renderer: renderer::Renderer,
    platform: platform::Platform,
    universe: universe::Universe,

    next_player_id: PlayerId,
    players: HashMap<PlayerId, Player>,

    is_requesting_exit: bool,
    exit_error: Option<anyhow::Error>,
    last_tick: Instant,

    #[allow(unused)]
    logger: Logger,
}

impl Engine {
    /// Constructs and initialises the gine
    ///
    /// # Errors
    ///
    /// Errors can be returned during platform & renderer initialisation
    pub fn init() -> anyhow::Result<Self> {
        #[cfg(editor)]
        info!("starting editor");

        info!("initialising engine");

        let logger = logging::Logger::new()
            .write_fs(true)
            .max_level(logging::LogLevel::Debug)
            .init()
            .context("initialising logger")?;

        let event_manager = events::EventManager::new();
        let asset_registry =
            assets::AssetRegistry::init(&event_manager).context("initialising asset registry")?;

        let version = utils::Version::from_str(env!("CARGO_PKG_VERSION"))?;
        let name = "DirkEngine";

        let platform = platform::Platform::init(&event_manager).context("platform init")?;
        let mut renderer = renderer::Renderer::init(
            &renderer::RendererCreateInfo {
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
            .with_other(world::universe_builder(&event_manager))
            .with_other(renderer.universe_builder())
            .build();

        info!("engine initialised");
        Ok(Self {
            exit_consumer: event_manager.subscribe(),
            event_manager,
            logger,
            asset_registry,

            platform,
            renderer,
            universe,

            next_player_id: 0,
            players: HashMap::new(),

            is_requesting_exit: false,
            exit_error: None,
            last_tick: Instant::now(),
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

        self.spawn_player(world_id);

        // we tick the engine a few times before entering proper
        // game loop & rendering cycles. This allows the event manager
        // to fire off its events & allows systems to process the first
        // few volleys before rendering gets involved
        for _ in 0..5 {
            self.tick_inner().context("pre-start ticking")?;
        }

        Ok(())
    }
    /// Ticks the engine. This is the master function that calls
    /// every other system's tick function.
    ///
    /// # Errors
    ///
    /// Errors can occure if the various ticking systems have errors.
    /// For now, only rendering can return an error.
    pub fn tick(&mut self) -> bool {
        if !match self.tick_inner() {
            Ok(exit) => exit,
            Err(err) => {
                self.exit_error = Some(err.context("engine tick"));
                false
            }
        } {
            return false;
        }
        match self.render() {
            Ok(()) => true,
            Err(err) => {
                self.exit_error = Some(err.context("rendering"));
                false
            }
        }
    }

    fn tick_inner(&mut self) -> anyhow::Result<bool> {
        let delta_time = self.capture_delta_time();
        self.event_manager.dispatch_all();

        // TODO: renders too fast and semaphores have problem.
        // remove when rendering takes longer
        std::thread::sleep(std::time::Duration::from_millis(100));

        self.process_events();
        if self.is_requesting_exit() {
            return Ok(false);
        }

        self.platform.tick(delta_time);
        self.universe.tick(delta_time);

        self.renderer
            .tick(delta_time, self.platform.windows_mut())
            .context("renderer")?;

        self.players
            .values_mut()
            .for_each(|player| player.tick(&mut self.universe));

        Ok(!self.is_requesting_exit())
    }

    fn render(&mut self) -> anyhow::Result<()> {
        Ok(self.renderer.render()?)
    }

    fn process_events(&mut self) {
        if let Some(events::AppExit(msg)) = self.exit_consumer.try_consume() {
            info!("App exit requested: {msg}");
            self.exit(None);
        }
    }
    /// Returns if the engine is planning to exit. i.e., if the engine
    /// will shutdown at the next tick.
    pub fn is_requesting_exit(&self) -> bool {
        self.is_requesting_exit || self.exit_error.is_some()
    }
    /// Specify `err` to exit with an error.
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

    fn spawn_player(&mut self, world: WorldId) -> PlayerId {
        let id = self.next_player_id;
        self.next_player_id += 1;

        let player = Player::spawn(
            id,
            &mut self.universe,
            world,
            self.platform.main_window().id(),
            &self.event_manager,
        );

        self.players.insert(id, player);
        id
    }
    #[allow(unused)]
    fn kill_player(&mut self, id: PlayerId) {
        let Some(player) = self.players.remove(&id) else {
            return;
        };
        player.despawn(&mut self.universe);
    }

    fn create_test_world(&mut self) -> WorldId {
        use world::components::{Renderable, Transform};

        let duck_model =
            assets::AssetHandle::from_raw("models/Duck/Duck.dirkasset", assets::AssetType::Model);
        let shrek_model =
            assets::AssetHandle::from_raw("models/Shrek/Shrek.dirkasset", assets::AssetType::Model);

        let shrek_builder = Entity::builder()
            .with_component(Transform {
                location: glam::Vec3::ZERO,
                rotation: glam::Vec3::ZERO,
                scale: glam::Vec3::splat(1.),
            })
            .with_component(Renderable { model: shrek_model });

        let duck_builder = Entity::builder()
            .with_component(Transform {
                location: glam::vec3(100., 0., 0.),
                rotation: glam::Vec3::ZERO,
                scale: glam::Vec3::splat(1.),
            })
            .with_component(Renderable { model: duck_model });

        let world_builder = World::builder("test world");

        let world = self.universe.create_world(world_builder);
        // TODO: this allows world_created event to fire before spawn, command buffers should fix
        self.universe.spawn(world, shrek_builder);
        self.universe.spawn(world, duck_builder);
        world
    }
}
