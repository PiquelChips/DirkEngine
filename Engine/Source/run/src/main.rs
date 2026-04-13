use anyhow::Context;
use tracing::error;

fn run() -> anyhow::Result<()> {
    tracing_log::LogTracer::init().context("init log_tracer")?;

    let mut engine = engine::Engine::init().context("engine init")?;
    engine.start().context("start engine")?;
    while engine.tick().context("engine tick")? {}
    engine.shutdown().context("engine shutdown")?;

    if let Some(err) = engine.get_exit_error() {
        error!("{err:#}");
    }
    Ok(())
}

fn main() {
    match run() {
        Ok(_) => {}
        Err(err) => {
            error!("{err:#}");
            panic!("Error: {err:#}");
        }
    }
}
