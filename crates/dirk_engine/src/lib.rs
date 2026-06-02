//! Core front-facing engine API.
//!
//! This crate owns the engine runtime primitives and composition API. Optional
//! engine features such as rendering, assets, players, and editor tooling should
//! be registered through [`EnginePlugin`] implementations instead of being
//! hard-wired into [`Engine`].

use std::{
    any::TypeId,
    collections::HashMap,
    mem,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    time::Instant,
};

use dirk_events::EventManager;
use dirk_threads::WorkerPool;
use dirk_universe::{Universe, UniverseBuilder};
use parking_lot::RwLock;
use tracing::{error, info};

pub mod errors;
pub mod events;
pub mod subsystem;

use errors::{Error, Result};
pub use subsystem::{EnginePlugin, Subsystem};

type SubsystemFactory =
    Box<dyn FnOnce(&EngineHandle) -> anyhow::Result<Box<dyn Subsystem>> + 'static>;

/// Builds an [`Engine`] from core configuration and plugin registrations.
pub struct EngineBuilder {
    app_name: String,
    worker_name: String,
    log_level: piquel_log::LogLevel,
    /// Store factories in `HashMap` keyed by the typeId of the [`Subsystem`].
    /// This avoid duplicate subsystems.
    subsystem_factories: HashMap<TypeId, SubsystemFactory>,
    universe_builder: UniverseBuilder,
    plugin_names: Vec<&'static str>,
}

impl EngineBuilder {
    /// Creates an empty engine builder with default core configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            app_name: "DirkEngine".to_owned(),
            worker_name: "dirk-workers".to_owned(),
            log_level: piquel_log::LogLevel::Debug,
            subsystem_factories: HashMap::new(),
            universe_builder: Universe::builder(),
            plugin_names: Vec::new(),
        }
    }

    /// Sets the application name used for diagnostics.
    #[must_use]
    pub fn with_app_name(mut self, app_name: impl Into<String>) -> Self {
        self.app_name = app_name.into();
        self
    }

    /// Sets the worker thread name prefix.
    #[must_use]
    pub fn with_worker_name(mut self, worker_name: impl Into<String>) -> Self {
        self.worker_name = worker_name.into();
        self
    }

    /// Sets the maximum log level configured by the engine.
    #[must_use]
    pub fn with_log_level(mut self, level: piquel_log::LogLevel) -> Self {
        self.log_level = level;
        self
    }

    /// Registers a plugin with the builder.
    ///
    /// # Errors
    ///
    /// Returns an error if the plugin fails to register its build-time pieces.
    pub fn with_plugin<P>(mut self, plugin: P) -> anyhow::Result<Self>
    where
        P: EnginePlugin,
    {
        let name = plugin.name();
        plugin.build(&mut self)?;
        self.plugin_names.push(name);
        drop(plugin);
        Ok(self)
    }

    /// Registers a plugin with the builder.
    ///
    /// # Errors
    ///
    /// Returns an error if the plugin fails to register its build-time pieces.
    pub fn add_plugin<P>(&mut self, plugin: &P) -> anyhow::Result<&mut Self>
    where
        P: EnginePlugin,
    {
        plugin.build(self)?;
        self.plugin_names.push(plugin.name());
        Ok(self)
    }

    /// Adds a runtime subsystem factory.
    ///
    /// The factory runs during [`EngineBuilder::build`], after core engine
    /// services have been created.
    pub fn add_subsystem<F, S>(&mut self, factory: F) -> &mut Self
    where
        F: FnOnce(&EngineHandle) -> anyhow::Result<S> + 'static,
        S: Subsystem + 'static,
    {
        self.subsystem_factories.insert(
            TypeId::of::<S>(),
            Box::new(move |context| Ok(Box::new(factory(context)?) as Box<dyn Subsystem>)),
        );
        self
    }

    /// Extends the engine ECS builder with another prepared ECS builder.
    pub fn extend_universe(&mut self, builder: UniverseBuilder) -> &mut Self {
        let universe_builder = mem::take(&mut self.universe_builder);
        self.universe_builder = universe_builder.with_other(builder);
        self
    }

    /// Builds a new engine.
    ///
    /// # Errors
    ///
    /// Returns an error if logging or subsystem initialization fails.
    pub fn build(self) -> Result<Engine> {
        let logger = piquel_log::Logger::new()
            .with_max_level(self.log_level)
            .with_log_bridge(true)
            .with_file(piquel_log::FileConfig::new(
                PathBuf::from(std::env!("SAVED_PATH")).join("logs"),
            ));

        logger.init()?;

        #[cfg(feature = "editor")]
        info!("starting editor");

        info!(app = self.app_name, "initialising engine");

        let workers = WorkerPool::new(&self.worker_name);
        let events = EventManager::new(workers.clone());
        let state = Arc::new(EngineState::new());
        let (commands, command_receiver) = mpsc::channel();
        let handle = EngineHandle {
            state: Arc::clone(&state),
            events: events.clone(),
            workers: workers.clone(),
            commands,
        };

        let mut subsystems = Vec::with_capacity(self.subsystem_factories.len());
        for (_, factory) in self.subsystem_factories {
            subsystems.push(factory(&handle).map_err(Error::SubsystemFailedInit)?);
        }

        let universe = self.universe_builder.build();

        Ok(Engine {
            logger,
            workers,
            universe,
            subsystems,
            state,
            handle,
            command_receiver,
            frame_dispatcher: events.register(),
            exiting_dispatcher: events.register(),
            events,
            last_tick: Instant::now(),
            started: false,
            shutdown: false,
            plugin_names: self.plugin_names,
        })
    }
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The main engine object.
///
/// Create an engine with [`Engine::builder`] to register plugins and
/// subsystems before the runtime starts.
pub struct Engine {
    logger: piquel_log::Logger,
    events: EventManager,
    workers: WorkerPool,
    universe: Universe,
    subsystems: Vec<Box<dyn Subsystem>>,
    state: Arc<EngineState>,
    handle: EngineHandle,
    command_receiver: Receiver<EngineCommand>,
    frame_dispatcher: dirk_events::Dispatcher<events::BeginFrame>,
    exiting_dispatcher: dirk_events::Dispatcher<events::Exiting>,
    last_tick: Instant,
    started: bool,
    shutdown: bool,
    plugin_names: Vec<&'static str>,
}

impl Engine {
    /// Returns a new engine builder.
    #[must_use]
    pub fn builder() -> EngineBuilder {
        EngineBuilder::new()
    }

    /// Creates a new engine with default builder configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the engine cannot be built.
    pub fn new() -> Result<Self> {
        Self::builder().build()
    }

    /// Returns a cheap handle to the engine.
    #[must_use]
    pub fn handle(&self) -> EngineHandle {
        self.handle.clone()
    }

    /// Returns the shared event manager.
    #[must_use]
    pub fn events(&self) -> &EventManager {
        &self.events
    }

    /// Returns the worker pool.
    #[must_use]
    pub fn workers(&self) -> &WorkerPool {
        &self.workers
    }

    /// Returns the engine ECS.
    #[must_use]
    pub fn universe(&self) -> &Universe {
        &self.universe
    }

    /// Returns mutable access to the engine ECS.
    #[must_use]
    pub fn universe_mut(&mut self) -> &mut Universe {
        &mut self.universe
    }

    /// Returns the names of registered plugins.
    #[must_use]
    pub fn plugin_names(&self) -> &[&'static str] {
        &self.plugin_names
    }

    /// Starts all subsystems.
    ///
    /// This is called automatically by [`Engine::run`] and [`Engine::tick`].
    ///
    /// # Errors
    ///
    /// Returns an error if a subsystem fails to start.
    pub fn start(&mut self) -> Result<()> {
        if self.started {
            return Ok(());
        }

        self.state.set_status(EngineStatus::Starting);

        for subsystem in &mut self.subsystems {
            subsystem
                .start(&self.handle, &mut self.universe)
                .map_err(|source| Error::SubsystemFailedTick {
                    name: subsystem.name(),
                    source,
                })?;
        }

        self.last_tick = Instant::now();
        self.started = true;
        self.state.set_status(EngineStatus::Running);
        Ok(())
    }

    /// Runs the engine until exit is requested or an error occurs.
    ///
    /// # Errors
    ///
    /// Returns an error when startup, ticking, or shutdown fails.
    pub fn run(mut self) -> Result<()> {
        self.start().map_err(|err| Error::StartFailed(err.into()))?;

        while !self.is_exiting() {
            self.tick().map_err(|err| Error::TickFailed(err.into()))?;
        }

        self.shutdown()
            .map_err(|err| Error::ShutdownFailed(err.into()))
    }

    /// Advances the engine by one tick.
    ///
    /// # Errors
    ///
    /// Returns an error if a subsystem tick fails.
    pub fn tick(&mut self) -> Result<EngineStatus> {
        if !self.started {
            self.start()?;
        }

        self.process_commands();
        if self.is_exiting() {
            return Ok(self.status());
        }

        let frame = self.state.increment_frame();
        self.frame_dispatcher.dispatch(events::BeginFrame(frame));

        let delta_time = self.capture_delta_time();

        for index in 0..self.subsystems.len() {
            let subsystem = &mut self.subsystems[index];
            let name = subsystem.name();
            subsystem
                .tick(delta_time, &self.handle, &mut self.universe)
                .map_err(|source| Error::SubsystemFailedTick { name, source })?;

            self.process_commands();
            if self.is_exiting() {
                break;
            }
        }

        self.universe.tick(delta_time);
        self.process_commands();
        Ok(self.status())
    }

    /// Requests engine exit without an error.
    pub fn exit(&mut self) {
        self.request_exit(None);
    }

    /// Requests engine exit with an error.
    pub fn exit_with_error(&mut self, error: anyhow::Error) {
        self.request_exit(Some(error));
    }

    /// Returns whether the engine is exiting or has exited.
    #[must_use]
    pub fn is_exiting(&self) -> bool {
        self.status().is_exiting()
    }

    /// Returns the current lightweight engine status.
    #[must_use]
    pub fn status(&self) -> EngineStatus {
        self.state.status()
    }

    /// Returns the current frame number.
    #[must_use]
    pub fn frame(&self) -> u64 {
        self.state.frame()
    }

    /// Shuts down the engine subsystems.
    ///
    /// Shutdown happens in reverse subsystem registration order.
    ///
    /// # Errors
    ///
    /// Returns an error if a subsystem fails to shut down.
    pub fn shutdown(&mut self) -> Result<()> {
        if self.shutdown {
            return Ok(());
        }

        if !self.is_exiting() {
            self.request_exit(None);
        }

        while let Some(mut subsystem) = self.subsystems.pop() {
            subsystem
                .shutdown(&self.handle, &mut self.universe)
                .map_err(|source| Error::SubsystemFailedTick {
                    name: subsystem.name(),
                    source,
                })?;
        }

        self.shutdown = true;
        self.state.set_status(EngineStatus::Exited);
        Ok(())
    }

    fn request_exit(&mut self, error: Option<anyhow::Error>) {
        if self.is_exiting() {
            return;
        }

        match error {
            Some(error) => self.state.set_error(error),
            None => self.state.set_status(EngineStatus::ExitRequested),
        }

        self.exiting_dispatcher.dispatch(events::Exiting);
    }

    fn process_commands(&mut self) {
        while let Ok(command) = self.command_receiver.try_recv() {
            match command {
                EngineCommand::RequestExit(error) => self.request_exit(error),
            }
        }
    }

    fn capture_delta_time(&mut self) -> f64 {
        let current_time = Instant::now();
        let delta_time = current_time.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = current_time;
        delta_time
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if !self.shutdown
            && let Err(err) = self.shutdown()
        {
            error!("engine shutdown failed: {err:#}");
        }

        let _ = &self.logger;
    }
}

/// A lightweight copyable engine lifecycle status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EngineStatus {
    /// The engine has been built but has not started subsystems.
    Initializing,
    /// The engine is starting subsystems.
    Starting,
    /// The engine is in its normal tick loop.
    Running,
    /// A system has requested that the engine exit.
    ExitRequested,
    /// An error occurred in the engine.
    Error,
    /// The engine has shut down.
    Exited,
}

impl EngineStatus {
    /// Returns whether this status means the engine should stop ticking.
    #[must_use]
    pub fn is_exiting(self) -> bool {
        matches!(self, Self::ExitRequested | Self::Error | Self::Exited)
    }
}

/// A cheap handle for systems to communicate with the main engine object.
#[derive(Clone)]
pub struct EngineHandle {
    state: Arc<EngineState>,
    events: EventManager,
    workers: WorkerPool,
    commands: Sender<EngineCommand>,
}

impl EngineHandle {
    /// Returns the shared event manager.
    #[must_use]
    pub fn events(&self) -> &EventManager {
        &self.events
    }

    /// Returns the worker pool.
    #[must_use]
    pub fn workers(&self) -> &WorkerPool {
        &self.workers
    }

    /// Requests engine exit without an error.
    pub fn exit(&self) {
        self.request_exit(None);
    }

    /// Requests engine exit with an error.
    pub fn exit_with_error(&self, error: anyhow::Error) {
        self.request_exit(Some(error));
    }

    /// Returns the current frame number.
    #[must_use]
    pub fn frame(&self) -> u64 {
        self.state.frame()
    }

    /// Returns the current lightweight engine status.
    #[must_use]
    pub fn status(&self) -> EngineStatus {
        self.state.status()
    }

    fn request_exit(&self, error: Option<anyhow::Error>) {
        if let Err(err) = self.commands.send(EngineCommand::RequestExit(error)) {
            error!("engine exit request failed: {err}");
        }
    }
}

enum EngineCommand {
    RequestExit(Option<anyhow::Error>),
}

struct EngineState {
    frame: AtomicU64,
    status: RwLock<EngineStatus>,
    error: RwLock<Option<anyhow::Error>>,
}

impl EngineState {
    fn new() -> Self {
        Self {
            frame: AtomicU64::new(0),
            status: RwLock::new(EngineStatus::Initializing),
            error: RwLock::new(None),
        }
    }

    fn frame(&self) -> u64 {
        self.frame.load(Ordering::Relaxed)
    }

    fn increment_frame(&self) -> u64 {
        self.frame.fetch_add(1, Ordering::Relaxed) + 1
    }

    fn status(&self) -> EngineStatus {
        *self.status.read()
    }

    fn set_error(&self, err: anyhow::Error) {
        *self.error.write() = Some(err);
        *self.status.write() = EngineStatus::Error;
    }

    fn set_status(&self, status: EngineStatus) {
        debug_assert!(
            !matches!(status, EngineStatus::Error),
            "use EngineStatus::set_error to set an error status"
        );
        *self.status.write() = status;
    }
}
