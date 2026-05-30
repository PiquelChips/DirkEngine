//! This is the main entrypoint crate. No real logic is contained here,
//! just engine init & tick looping

use anyhow::Context;
use dirkengine::engine::ExitState;
use tracing::error;

fn run() -> anyhow::Result<()> {
    let mut engine = dirkengine::engine::Engine::init().context("engine init")?;
    engine.start().context("start engine")?;
    while matches!(engine.tick(), ExitState::Running) {}

    match engine.exit_state() {
        ExitState::Running => unreachable!("engine loop only stops once exit is requested"),
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
