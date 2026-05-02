//! This is the main entrypoint crate. No real logic is contained here,
//! just engine init & tick looping

use anyhow::Context;
use tracing::error;

fn run() -> anyhow::Result<()> {
    tracing_log::LogTracer::init().context("init log_tracer")?;

    let mut engine = engine::Engine::init().context("engine init")?;
    engine.start().context("start engine")?;
    while engine.tick().context("engine tick")? {}

    if let Some(err) = engine.get_exit_error() {
        error!("{err:#}");
    }
    Ok(())
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
