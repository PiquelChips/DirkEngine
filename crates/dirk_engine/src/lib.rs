//! This module holds all the traits & struct for the engine.

use dirk_events::EventManager;
use dirk_threads::WorkerPool;
use parking_lot::{RwLock, RwLockReadGuard};
use tracing::info;

pub mod errors;
use errors::{Error, Result};

pub mod events;
pub mod subsystem;

use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

/// The main engine object.
///
/// Create an [`Engine`] with the [`EngineBuilder`] to specifiy modules &
/// subsystems that you want in the engine.
pub struct Engine {
    logger: piquel_log::Logger,
    events: dirk_events::EventManager,
    workers: WorkerPool,

    /// The global shared engine state
    state: Arc<RwLock<State>>,
    /// A cheap handle to the engine & its state
    handle: EngineHandle,
}

impl Engine {
    /// Creates a new engine with all the base primitives it needs.
    pub fn new() -> Result<Self> {
        let logger = piquel_log::Logger::new()
            .with_max_level(piquel_log::LogLevel::Debug)
            .with_log_bridge(true)
            .with_file(piquel_log::FileConfig::new(
                PathBuf::from(std::env!("SAVED_PATH")).join("logs"),
            ));

        logger.init()?;

        #[cfg(feature = "editor")]
        info!("starting editor");

        info!("initialising engine");

        let workers = WorkerPool::new("dirk-workers");
        let events = dirk_events::EventManager::new(workers.clone());

        let state = Arc::new(RwLock::new(State {
            state: EngineState::Initializing,
            frame: 0,
        }));

        let handle = EngineHandle {
            state: state.clone(),
            workers: workers.clone(),
            events: events.clone(),
        };

        Ok(Self {
            logger,
            events,
            workers,
            state,
            handle,
        })
    }

    /// Actually initialises & runs the engine. This is a blocking loop that
    /// will not exit until exit is requested or an error occurs.
    pub fn run(mut self) -> Result<()> {
        self.start().map_err(|err| Error::StartFailed(err.into()))?;
        self.state.write().state = EngineState::Running;

        while !self.exiting() {
            if let Err(err) = self.tick() {
                self.state.write().state = EngineState::Error(err);
            }
            self.state.write().frame += 1;
        }

        Ok(())
    }

    fn start(&mut self) -> Result<()> {
        self.state.write().state = EngineState::Starting;

        // TODO: start engine systems
        // TODO: create worlds

        Ok(())
    }
    fn tick(&mut self) -> Result<()> {
        // TODO: broadcast new frame
        // TODO: calculate delta_time
        // TODO: process engine events
        // TODO: return if exiting
        // TODO: tick systems
        Ok(())
    }

    fn request_exit(&self, msg: impl Into<String>) {
        self.state.write().state = EngineState::ExitRequested(msg.into());
    }
    fn exiting(&self) -> bool {
        matches!(
            self.state.read().state,
            EngineState::ExitRequested(_) | EngineState::Error(_)
        )
    }
}

impl Drop for Engine {
    fn drop(&mut self) {}
}

/// A simple enum representing the current state of the engine.
pub enum EngineState {
    /// The engine has just been initialised. All core systems have been
    /// loaded and started. No external game systems are ready.
    Initializing,
    /// Game systems are starting to be created.
    /// Worlds are starting to be built/loaded.
    Starting,
    /// The engine is in its normal tick loop.
    Running,
    /// A system has requested that the engine exit.
    ExitRequested(String),
    /// An error has occured in the engine.
    Error(Error),
}

/// A cheap handle for systems to communicate with the main engine object.
pub struct EngineHandle {
    // TODO: see about storing just a `ReadLock`.
    state: Arc<RwLock<State>>,
    events: dirk_events::EventManager,
    workers: WorkerPool,
}

impl EngineHandle {
    fn state(&self) -> RwLockReadGuard<State> {
        self.state.read()
    }

    pub fn events(&self) -> &EventManager {
        &self.events
    }
    pub fn workers(&self) -> &WorkerPool {
        &self.workers
    }

    pub fn exit(&self, msg: impl Into<String>) {
        // TODO: send the request to the actual engine.
        info!("engine exit requested: {}", msg.into());
    }

    pub fn frame(&self) -> u64 {
        self.state().frame()
    }
}

struct State {
    frame: u64,
    state: EngineState,
}

impl State {
    fn frame(&self) -> u64 {
        self.frame
    }
}
