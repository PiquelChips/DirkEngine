//! This is the main entrypoint crate. No real logic is contained here,
//! just engine init & tick looping

use anyhow::Context;
use dirkengine::engine::ExitState;
use tracing::error;

fn run() -> anyhow::Result<()> {
    tracing_log::LogTracer::init().context("init log_tracer")?;

    let mut engine = dirkengine::engine::Engine::init().context("engine init")?;
    engine.start().context("start engine")?;
    while engine.tick() {}

    match engine.exit_state() {
        ExitState::Running => panic!(""),
        ExitState::Requested => Ok(()),
        ExitState::Error(err) => {
            error!("{err:#}");
            Ok(())
        }
    }
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(err) => {
            error!("{err:#}");
            panic!("Error: {err:#}");
        }
    }
}
